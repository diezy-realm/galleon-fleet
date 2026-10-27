//! Cron state store: persistence for cron jobs in SQLite.

use crate::cron_schedule::{next_run_for_schedule, schedule_cron_expression, validate_schedule};
use crate::cron_types::{
    CronJob, CronJobPatch, CronRun, CronShellOutputFormat, DeliveryConfig, JobType, Schedule,
    SessionTarget, validate_delivery_config,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Lightweight representation of a cron job row for serialization and archival.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobRecord {
    pub id: String,
    pub expression: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<serde_json::Value>,
    pub job_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub session_target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<serde_json::Value>,
    pub delete_after_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<serde_json::Value>,
    pub created_at: String,
    pub next_run: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub uses_memory: bool,
    pub agent_alias: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_output_format: Option<String>,
}

pub struct CronStore {
    db_path: PathBuf,
}

impl CronStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("cron").join("jobs.db");
        Ok(Self { db_path })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn has_cron_jobs_table(conn: &Connection) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='cron_jobs'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false)
    }

    fn has_cron_runs_table(conn: &Connection) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='cron_runs'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false)
    }

    pub fn ensure_table(&self) -> Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cron dir: {}", parent.display()))?;
        }
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;
        conn.execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS cron_jobs (
                id               TEXT PRIMARY KEY,
                expression       TEXT NOT NULL,
                command          TEXT NOT NULL,
                schedule         TEXT,
                job_type         TEXT NOT NULL DEFAULT 'shell',
                prompt           TEXT,
                name             TEXT,
                session_target   TEXT NOT NULL DEFAULT 'isolated',
                model            TEXT,
                enabled          INTEGER NOT NULL DEFAULT 1,
                delivery         TEXT,
                delete_after_run INTEGER NOT NULL DEFAULT 0,
                allowed_tools    TEXT,
                created_at       TEXT NOT NULL,
                next_run         TEXT NOT NULL,
                last_run         TEXT,
                last_status      TEXT,
                last_output      TEXT,
                source           TEXT DEFAULT 'imperative',
                uses_memory      INTEGER NOT NULL DEFAULT 1,
                agent_alias      TEXT NOT NULL DEFAULT '',
                shell_output_format TEXT NOT NULL DEFAULT 'wrapped'
             );
             CREATE TABLE IF NOT EXISTS cron_runs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id      TEXT NOT NULL,
                started_at  TEXT NOT NULL,
                finished_at TEXT NOT NULL,
                status      TEXT NOT NULL,
                output      TEXT,
                duration_ms INTEGER,
                FOREIGN KEY (job_id) REFERENCES cron_jobs(id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_cron_runs_job_id ON cron_runs(job_id);
             CREATE INDEX IF NOT EXISTS idx_cron_runs_started_at ON cron_runs(started_at);
             CREATE INDEX IF NOT EXISTS idx_cron_runs_job_started ON cron_runs(job_id, started_at);",
        )
        .context("Failed to ensure cron_jobs and cron_runs tables")?;
        Ok(())
    }

    pub fn add_job(&self, agent_alias: &str, expression: &str, command: &str) -> Result<String> {
        self.ensure_table()?;
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO cron_jobs (id, expression, command, created_at, next_run, agent_alias)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, expression, command, now, now, agent_alias],
        )
        .context("Failed to insert cron job")?;
        Ok(id)
    }

    pub fn list_jobs(&self) -> Result<Vec<CronJob>> {
        if !self.db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;

        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        if !Self::has_cron_jobs_table(&conn) {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            "SELECT id, expression, command, schedule, job_type, prompt, name, session_target, model,
                    enabled, delivery, delete_after_run, created_at, next_run, last_run, last_status, last_output,
                    allowed_tools, source, uses_memory, agent_alias, shell_output_format
             FROM cron_jobs ORDER BY next_run ASC",
        )?;

        let rows = stmt.query_map([], map_cron_job_row)?;
        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
    }

    pub fn get_job(&self, job_id: &str) -> Result<CronJob> {
        if !self.db_path.exists() {
            anyhow::bail!("Cron job '{job_id}' not found");
        }

        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;

        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        if !Self::has_cron_jobs_table(&conn) {
            anyhow::bail!("Cron job '{job_id}' not found");
        }

        let mut stmt = conn.prepare(
            "SELECT id, expression, command, schedule, job_type, prompt, name, session_target, model,
                    enabled, delivery, delete_after_run, created_at, next_run, last_run, last_status, last_output,
                    allowed_tools, source, uses_memory, agent_alias, shell_output_format
             FROM cron_jobs WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![job_id])?;
        if let Some(row) = rows.next()? {
            Ok(map_cron_job_row(row)?)
        } else {
            anyhow::bail!("Cron job '{job_id}' not found")
        }
    }

    pub fn remove_job(&self, id: &str) -> Result<()> {
        if !self.db_path.exists() {
            anyhow::bail!("Cron job '{id}' not found");
        }

        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;

        conn.execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )?;

        if !Self::has_cron_jobs_table(&conn) {
            anyhow::bail!("Cron job '{id}' not found");
        }

        let changed = conn
            .execute("DELETE FROM cron_jobs WHERE id = ?1", params![id])
            .context("Failed to delete cron job")?;

        if changed == 0 {
            anyhow::bail!("Cron job '{id}' not found");
        }

        Ok(())
    }

    pub fn list_runs(&self, job_id: &str, limit: usize) -> Result<Vec<CronRun>> {
        if !self.db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;

        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        if !Self::has_cron_runs_table(&conn) {
            return Ok(Vec::new());
        }

        let lim = i64::try_from(limit.max(1)).context("Run history limit overflow")?;
        let mut stmt = conn.prepare(
            "SELECT id, job_id, started_at, finished_at, status, output, duration_ms
             FROM cron_runs
             WHERE job_id = ?1
             ORDER BY started_at DESC, id DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![job_id, lim], map_cron_run_row)?;
        let mut runs = Vec::new();
        for row in rows {
            runs.push(row?);
        }
        Ok(runs)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_agent_job(
        &self,
        agent_alias: &str,
        name: Option<String>,
        schedule: Schedule,
        prompt: &str,
        session_target: SessionTarget,
        model: Option<String>,
        delivery: Option<DeliveryConfig>,
        delete_after_run: bool,
        allowed_tools: Option<Vec<String>>,
        uses_memory: bool,
    ) -> Result<CronJob> {
        let now = Utc::now();
        validate_schedule(&schedule, now)?;
        validate_delivery_config(delivery.as_ref())?;
        let next_run = next_run_for_schedule(&schedule, now)?;
        let id = uuid::Uuid::new_v4().to_string();
        let expression = schedule_cron_expression(&schedule).unwrap_or_default();
        let schedule_json = serde_json::to_string(&schedule)?;
        let delivery = delivery.unwrap_or_default();
        let agent_alias = agent_alias.trim();
        if agent_alias.is_empty() {
            anyhow::bail!("agent_alias is required; cron jobs must name an owning agent");
        }

        self.ensure_table()?;
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        let allowed_tools_json = allowed_tools
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        conn.execute(
            "INSERT INTO cron_jobs (
                id, expression, command, schedule, job_type, prompt, name, session_target, model,
                enabled, delivery, delete_after_run, allowed_tools, agent_alias, created_at, next_run,
                uses_memory
             ) VALUES (?1, ?2, '', ?3, 'agent', ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id,
                expression,
                schedule_json,
                prompt,
                name,
                session_target.as_str(),
                model,
                serde_json::to_string(&delivery)?,
                if delete_after_run { 1 } else { 0 },
                allowed_tools_json,
                agent_alias,
                now.to_rfc3339(),
                next_run.to_rfc3339(),
                if uses_memory { 1 } else { 0 },
            ],
        )
        .context("Failed to insert cron agent job")?;

        self.get_job(&id)
    }

    pub fn add_shell_job(
        &self,
        agent_alias: &str,
        name: Option<String>,
        schedule: Schedule,
        command: &str,
        delivery: Option<DeliveryConfig>,
        shell_output_format: CronShellOutputFormat,
    ) -> Result<CronJob> {
        let now = Utc::now();
        validate_schedule(&schedule, now)?;
        validate_delivery_config(delivery.as_ref())?;
        let next_run = next_run_for_schedule(&schedule, now)?;
        let id = uuid::Uuid::new_v4().to_string();
        let expression = schedule_cron_expression(&schedule).unwrap_or_default();
        let schedule_json = serde_json::to_string(&schedule)?;
        let delivery = delivery.unwrap_or_default();

        let delete_after_run = matches!(schedule, Schedule::At { .. });
        let agent_alias = agent_alias.trim();
        if agent_alias.is_empty() {
            anyhow::bail!("agent_alias is required; cron jobs must name an owning agent");
        }

        let shell_output_format_str = match shell_output_format {
            CronShellOutputFormat::Wrapped => "wrapped",
            CronShellOutputFormat::Raw => "raw",
        };

        self.ensure_table()?;
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        conn.execute(
            "INSERT INTO cron_jobs (
                id, expression, command, schedule, job_type, prompt, name, session_target, model,
                enabled, delivery, delete_after_run, allowed_tools, agent_alias, created_at, next_run,
                uses_memory, shell_output_format
             ) VALUES (?1, ?2, ?3, ?4, 'shell', NULL, ?5, 'isolated', NULL, 1, ?6, ?7, NULL, ?8, ?9, ?10, 1, ?11)",
            params![
                id,
                expression,
                command,
                schedule_json,
                name,
                serde_json::to_string(&delivery)?,
                if delete_after_run { 1 } else { 0 },
                agent_alias,
                now.to_rfc3339(),
                next_run.to_rfc3339(),
                shell_output_format_str,
            ],
        )
        .context("Failed to insert cron shell job")?;

        self.get_job(&id)
    }

    pub fn update_job(&self, job_id: &str, patch: CronJobPatch) -> Result<CronJob> {
        let mut job = self.get_job(job_id)?;
        let mut schedule_changed = false;

        if let Some(schedule) = patch.schedule {
            validate_schedule(&schedule, Utc::now())?;
            job.schedule = schedule;
            job.expression = schedule_cron_expression(&job.schedule).unwrap_or_default();
            schedule_changed = true;
        }
        if let Some(command) = patch.command {
            job.command = command;
        }
        if let Some(prompt) = patch.prompt {
            job.prompt = Some(prompt);
        }
        if let Some(name) = patch.name {
            job.name = Some(name);
        }
        if let Some(enabled) = patch.enabled {
            job.enabled = enabled;
        }
        if let Some(delivery) = patch.delivery {
            if job.source == "declarative" {
                anyhow::bail!(
                    "Cron job '{job_id}': delivery is owned by [cron.{job_id}].delivery in \
                     config.toml for a declarative job, not the database. Edit the config \
                     and restart the daemon."
                );
            }
            validate_delivery_config(Some(&delivery))?;
            job.delivery = delivery;
        }
        if let Some(model) = patch.model {
            job.model = Some(model);
        }
        if let Some(target) = patch.session_target {
            job.session_target = target;
        }
        if let Some(delete_after_run) = patch.delete_after_run {
            job.delete_after_run = delete_after_run;
        }
        if let Some(allowed_tools) = patch.allowed_tools {
            if allowed_tools.is_empty() {
                job.allowed_tools = None;
            } else {
                job.allowed_tools = Some(allowed_tools);
            }
        }
        if let Some(uses_memory) = patch.uses_memory {
            job.uses_memory = uses_memory;
        }
        if let Some(shell_output_format) = patch.shell_output_format {
            if job.source == "declarative" {
                anyhow::bail!(
                    "Cron job '{job_id}': shell_output_format is owned by config.toml for a \
                     declarative job, not the database"
                );
            }
            if job.job_type != JobType::Shell {
                let job_type: &str = job.job_type.into();
                anyhow::bail!(
                    "Cron job '{job_id}': shell_output_format is shell-only and cannot be set on a '{job_type}' job"
                );
            }
            job.shell_output_format = shell_output_format;
        }

        if schedule_changed {
            job.next_run = next_run_for_schedule(&job.schedule, Utc::now())?;
        }

        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        let allowed_tools_json = job
            .allowed_tools
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        let shell_output_format_str = match job.shell_output_format {
            CronShellOutputFormat::Wrapped => "wrapped",
            CronShellOutputFormat::Raw => "raw",
        };

        let job_type_str: &str = job.job_type.clone().into();

        if job.source == "declarative" {
            conn.execute(
                "UPDATE cron_jobs
                 SET expression = ?1, command = ?2, schedule = ?3, job_type = ?4, prompt = ?5, name = ?6,
                     session_target = ?7, model = ?8, enabled = ?9, delivery = ?10, delete_after_run = ?11,
                     allowed_tools = ?12, next_run = ?13, uses_memory = ?14
                 WHERE id = ?15",
                params![
                    job.expression,
                    job.command,
                    serde_json::to_string(&job.schedule)?,
                    job_type_str,
                    job.prompt,
                    job.name,
                    job.session_target.as_str(),
                    job.model,
                    if job.enabled { 1 } else { 0 },
                    serde_json::to_string(&job.delivery)?,
                    if job.delete_after_run { 1 } else { 0 },
                    allowed_tools_json,
                    job.next_run.to_rfc3339(),
                    if job.uses_memory { 1 } else { 0 },
                    job_id,
                ],
            )
            .context("Failed to update declarative cron job")?;
        } else {
            conn.execute(
                "UPDATE cron_jobs
                 SET expression = ?1, command = ?2, schedule = ?3, job_type = ?4, prompt = ?5, name = ?6,
                     session_target = ?7, model = ?8, enabled = ?9, delivery = ?10, delete_after_run = ?11,
                     allowed_tools = ?12, next_run = ?13, uses_memory = ?14, shell_output_format = ?15
                 WHERE id = ?16",
                params![
                    job.expression,
                    job.command,
                    serde_json::to_string(&job.schedule)?,
                    job_type_str,
                    job.prompt,
                    job.name,
                    job.session_target.as_str(),
                    job.model,
                    if job.enabled { 1 } else { 0 },
                    serde_json::to_string(&job.delivery)?,
                    if job.delete_after_run { 1 } else { 0 },
                    allowed_tools_json,
                    job.next_run.to_rfc3339(),
                    if job.uses_memory { 1 } else { 0 },
                    shell_output_format_str,
                    job_id,
                ],
            )
            .context("Failed to update imperative cron job")?;
        }

        self.get_job(job_id)
    }

    pub fn record_manual_run(
        &self,
        job_id: &str,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        status: &str,
        output: Option<&str>,
        duration_ms: i64,
    ) -> Result<()> {
        self.ensure_table()?;
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        let started_at_str = started_at.to_rfc3339();
        let finished_at_str = finished_at.to_rfc3339();

        conn.execute(
            "INSERT INTO cron_runs (job_id, started_at, finished_at, status, output, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                job_id,
                started_at_str,
                finished_at_str,
                status,
                output,
                duration_ms
            ],
        )
        .context("Failed to insert cron run")?;

        conn.execute(
            "UPDATE cron_jobs
             SET last_run = ?1, last_status = ?2, last_output = ?3
             WHERE id = ?4",
            params![started_at_str, status, output, job_id],
        )
        .context("Failed to update last run on cron job")?;

        Ok(())
    }

    pub fn list_jobs_by_agent(&self, agent_alias: &str) -> Result<Vec<CronJobRecord>> {
        if !self.db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;

        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        if !Self::has_cron_jobs_table(&conn) {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            "SELECT id, expression, command, schedule, job_type, prompt, name, session_target, model,
                    enabled, delivery, delete_after_run, created_at, next_run, last_run, last_status, last_output,
                    allowed_tools, source, uses_memory, agent_alias, shell_output_format
             FROM cron_jobs WHERE agent_alias = ?1 ORDER BY next_run ASC",
        )?;

        let rows = stmt.query_map(params![agent_alias], |row| {
            let schedule_raw: Option<String> = row.get(3).ok();
            let schedule = schedule_raw
                .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s)));
            let delivery_raw: Option<String> = row.get(10).ok();
            let delivery = delivery_raw
                .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s)));
            let allowed_tools_raw: Option<String> = row.get(17).ok();
            let allowed_tools = allowed_tools_raw
                .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s)));

            Ok(CronJobRecord {
                id: row.get(0)?,
                expression: row.get(1)?,
                command: row.get(2)?,
                schedule,
                job_type: row
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| "shell".to_string()),
                prompt: row.get(5).ok(),
                name: row.get(6).ok(),
                session_target: row
                    .get::<_, Option<String>>(7)?
                    .unwrap_or_else(|| "isolated".to_string()),
                model: row.get(8).ok(),
                enabled: row
                    .get::<_, Option<i64>>(9)?
                    .map(|v| v != 0)
                    .unwrap_or(true),
                delivery,
                delete_after_run: row
                    .get::<_, Option<i64>>(11)?
                    .map(|v| v != 0)
                    .unwrap_or(false),
                created_at: row.get::<_, Option<String>>(12)?.unwrap_or_default(),
                next_run: row.get::<_, Option<String>>(13)?.unwrap_or_default(),
                last_run: row.get(14).ok(),
                last_status: row.get(15).ok(),
                last_output: row.get(16).ok(),
                allowed_tools,
                source: row.get(18).ok(),
                uses_memory: row
                    .get::<_, Option<i64>>(19)?
                    .map(|v| v != 0)
                    .unwrap_or(true),
                agent_alias: row.get::<_, Option<String>>(20)?.unwrap_or_default(),
                shell_output_format: row.get(21).ok(),
            })
        })?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
    }

    pub fn remove_jobs_by_agent(&self, agent_alias: &str) -> Result<usize> {
        if !self.db_path.exists() {
            return Ok(0);
        }

        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;

        conn.execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )?;

        if !Self::has_cron_jobs_table(&conn) {
            return Ok(0);
        }

        let changed = conn
            .execute(
                "DELETE FROM cron_jobs WHERE agent_alias = ?1",
                params![agent_alias],
            )
            .context("Failed to delete cron jobs for agent")?;

        Ok(changed)
    }

    pub fn rename_jobs_by_agent(&self, from: &str, to: &str) -> Result<usize> {
        if !self.db_path.exists() {
            return Ok(0);
        }

        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open cron DB: {}", self.db_path.display()))?;

        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        if !Self::has_cron_jobs_table(&conn) {
            return Ok(0);
        }

        let changed = conn
            .execute(
                "UPDATE cron_jobs SET agent_alias = ?2 WHERE agent_alias = ?1",
                params![from, to],
            )
            .context("Failed to rename cron job owner")?;

        Ok(changed)
    }
}

fn map_cron_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CronJob> {
    let expression: String = row.get(1)?;
    let schedule_raw: Option<String> = row.get(3)?;
    let schedule = if let Some(raw) = schedule_raw.as_deref().filter(|s| !s.trim().is_empty()) {
        serde_json::from_str(raw).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
        })?
    } else {
        Schedule::Cron {
            expr: expression.clone(),
            tz: None,
        }
    };

    let delivery_raw: Option<String> = row.get(10)?;
    let delivery = if let Some(raw) = delivery_raw.as_deref().filter(|s| !s.trim().is_empty()) {
        serde_json::from_str(raw).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e))
        })?
    } else {
        DeliveryConfig::default()
    };

    let allowed_tools_raw: Option<String> = row.get(17)?;
    let allowed_tools = if let Some(raw) = allowed_tools_raw
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        serde_json::from_str(raw).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(17, rusqlite::types::Type::Text, Box::new(e))
        })?
    } else {
        None
    };

    let jt_str: String = row.get(4)?;
    let job_type = JobType::try_from(jt_str.as_str()).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, e.into())
    })?;

    let st_str: Option<String> = row.get(7)?;
    let session_target = match st_str.as_deref() {
        Some(s) => SessionTarget::parse(s),
        None => SessionTarget::Isolated,
    };

    let fmt_raw: Option<String> = row.get(21)?;
    let shell_output_format = match fmt_raw.as_deref() {
        Some("raw") => CronShellOutputFormat::Raw,
        _ => CronShellOutputFormat::Wrapped,
    };

    let created_at_raw: String = row.get(12)?;
    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let next_run_raw: String = row.get(13)?;
    let next_run = chrono::DateTime::parse_from_rfc3339(&next_run_raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let last_run_raw: Option<String> = row.get(14)?;
    let last_run = last_run_raw.and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    });

    let uses_memory: Option<i64> = row.get(19)?;
    let agent_alias: Option<String> = row.get(20)?;
    let source: Option<String> = row.get(18)?;

    Ok(CronJob {
        id: row.get(0)?,
        expression,
        schedule,
        command: row.get(2)?,
        job_type,
        prompt: row.get(5)?,
        name: row.get(6)?,
        session_target,
        model: row.get(8)?,
        agent_alias: agent_alias
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        enabled: row
            .get::<_, Option<i64>>(9)?
            .map(|v| v != 0)
            .unwrap_or(true),
        delivery,
        delete_after_run: row
            .get::<_, Option<i64>>(11)?
            .map(|v| v != 0)
            .unwrap_or(false),
        allowed_tools,
        uses_memory: uses_memory != Some(0),
        source: source.unwrap_or_else(|| "imperative".to_string()),
        shell_output_format,
        created_at,
        next_run,
        last_run,
        last_status: row.get(15)?,
        last_output: row.get(16)?,
    })
}

fn map_cron_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CronRun> {
    let sa_str: String = row.get(2)?;
    let started_at = chrono::DateTime::parse_from_rfc3339(&sa_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let fa_str: String = row.get(3)?;
    let finished_at = chrono::DateTime::parse_from_rfc3339(&fa_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Ok(CronRun {
        id: row.get(0)?,
        job_id: row.get(1)?,
        started_at,
        finished_at,
        status: row.get(4)?,
        output: row.get(5)?,
        duration_ms: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_db(temp_dir: &TempDir) -> CronStore {
        let data_dir = temp_dir.path();
        let cron_dir = data_dir.join("cron");
        std::fs::create_dir_all(&cron_dir).unwrap();
        let db_path = cron_dir.join("jobs.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE cron_jobs (
                id               TEXT PRIMARY KEY,
                expression       TEXT NOT NULL,
                command          TEXT NOT NULL,
                schedule         TEXT,
                job_type         TEXT NOT NULL DEFAULT 'shell',
                prompt           TEXT,
                name             TEXT,
                session_target   TEXT NOT NULL DEFAULT 'isolated',
                model            TEXT,
                enabled          INTEGER NOT NULL DEFAULT 1,
                delivery         TEXT,
                delete_after_run INTEGER NOT NULL DEFAULT 0,
                allowed_tools    TEXT,
                created_at       TEXT NOT NULL,
                next_run         TEXT NOT NULL,
                last_run         TEXT,
                last_status      TEXT,
                last_output      TEXT,
                source           TEXT DEFAULT 'imperative',
                uses_memory      INTEGER NOT NULL DEFAULT 1,
                agent_alias      TEXT NOT NULL DEFAULT '',
                shell_output_format TEXT NOT NULL DEFAULT 'wrapped'
             );",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO cron_jobs (id, expression, command, created_at, next_run, agent_alias)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "job-1",
                "* * * * *",
                "echo test",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:01:00Z",
                "test-agent"
            ],
        )
        .unwrap();

        CronStore::new(data_dir).unwrap()
    }

    #[test]
    fn test_empty_db_path() {
        let temp = TempDir::new().unwrap();
        let store = CronStore::new(temp.path()).unwrap();
        assert_eq!(store.list_jobs_by_agent("any").unwrap().len(), 0);
        assert_eq!(store.remove_jobs_by_agent("any").unwrap(), 0);
        assert_eq!(store.rename_jobs_by_agent("from", "to").unwrap(), 0);
        assert_eq!(store.list_jobs().unwrap().len(), 0);
    }

    #[test]
    fn test_list_rename_remove_jobs() {
        let temp = TempDir::new().unwrap();
        let store = create_test_db(&temp);

        // List
        let jobs = store.list_jobs_by_agent("test-agent").unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "job-1");
        assert_eq!(jobs[0].agent_alias, "test-agent");

        let all_jobs = store.list_jobs().unwrap();
        assert_eq!(all_jobs.len(), 1);
        assert_eq!(all_jobs[0].id, "job-1");

        let job = store.get_job("job-1").unwrap();
        assert_eq!(job.id, "job-1");
        assert_eq!(job.command, "echo test");

        // Rename
        let renamed = store
            .rename_jobs_by_agent("test-agent", "new-agent")
            .unwrap();
        assert_eq!(renamed, 1);
        assert_eq!(store.list_jobs_by_agent("test-agent").unwrap().len(), 0);
        assert_eq!(store.list_jobs_by_agent("new-agent").unwrap().len(), 1);

        // Remove
        let removed = store.remove_jobs_by_agent("new-agent").unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.list_jobs_by_agent("new-agent").unwrap().len(), 0);
    }
}
