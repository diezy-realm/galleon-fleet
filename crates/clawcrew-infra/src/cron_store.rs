//! Cron state store: persistence for cron jobs in SQLite.

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};

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

    fn has_cron_jobs_table(conn: &Connection) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='cron_jobs'",
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
            "PRAGMA foreign_keys = ON;
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
             );",
        )
        .context("Failed to ensure cron_jobs table")?;
        Ok(())
    }

    pub fn add_job(
        &self,
        agent_alias: &str,
        expression: &str,
        command: &str,
    ) -> Result<String> {
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
            let schedule = schedule_raw.map(|s| {
                serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s))
            });
            let delivery_raw: Option<String> = row.get(10).ok();
            let delivery = delivery_raw.map(|s| {
                serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s))
            });
            let allowed_tools_raw: Option<String> = row.get(17).ok();
            let allowed_tools = allowed_tools_raw.map(|s| {
                serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s))
            });

            Ok(CronJobRecord {
                id: row.get(0)?,
                expression: row.get(1)?,
                command: row.get(2)?,
                schedule,
                job_type: row.get::<_, Option<String>>(4)?.unwrap_or_else(|| "shell".to_string()),
                prompt: row.get(5).ok(),
                name: row.get(6).ok(),
                session_target: row.get::<_, Option<String>>(7)?.unwrap_or_else(|| "isolated".to_string()),
                model: row.get(8).ok(),
                enabled: row.get::<_, Option<i64>>(9)?.map(|v| v != 0).unwrap_or(true),
                delivery,
                delete_after_run: row.get::<_, Option<i64>>(11)?.map(|v| v != 0).unwrap_or(false),
                created_at: row.get::<_, Option<String>>(12)?.unwrap_or_default(),
                next_run: row.get::<_, Option<String>>(13)?.unwrap_or_default(),
                last_run: row.get(14).ok(),
                last_status: row.get(15).ok(),
                last_output: row.get(16).ok(),
                allowed_tools,
                source: row.get(18).ok(),
                uses_memory: row.get::<_, Option<i64>>(19)?.map(|v| v != 0).unwrap_or(true),
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

        let changed = conn.execute(
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

        let changed = conn.execute(
            "UPDATE cron_jobs SET agent_alias = ?2 WHERE agent_alias = ?1",
            params![from, to],
        )
        .context("Failed to rename cron job owner")?;

        Ok(changed)
    }
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
            params!["job-1", "* * * * *", "echo test", "2026-01-01T00:00:00Z", "2026-01-01T00:01:00Z", "test-agent"],
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

        // Rename
        let renamed = store.rename_jobs_by_agent("test-agent", "new-agent").unwrap();
        assert_eq!(renamed, 1);
        assert_eq!(store.list_jobs_by_agent("test-agent").unwrap().len(), 0);
        assert_eq!(store.list_jobs_by_agent("new-agent").unwrap().len(), 1);

        // Remove
        let removed = store.remove_jobs_by_agent("new-agent").unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.list_jobs_by_agent("new-agent").unwrap().len(), 0);
    }
}
