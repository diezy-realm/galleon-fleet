//! Gateway cron adapter: fully decoupled from `galleon-runtime`.
//!
//! Uses leaf crates `galleon-infra`, `galleon-config`, and `galleon-api`
//! for all cron state persistence, validation, and execution.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::OnceLock;

use anyhow::Context;
use galleon_api::runtime_traits::CronRunner;
use galleon_config::schema::{Config, CronJobDecl, CronScheduleDecl, CronShellOutputFormat};
use galleon_infra::cron_schedule::{next_run_for_schedule, schedule_cron_expression};
use galleon_infra::cron_store::CronStore;
use galleon_infra::cron_types::{
    CronJob, CronJobPatch, CronRun, DeliveryConfig, JobType, ManualCronRunResult, Schedule,
    SessionTarget, validate_delivery_config,
};
use rusqlite::{Connection, params};

pub type DeliveryFn = Box<
    dyn Fn(
            Config,
            String,
            String,
            Option<String>,
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>>
        + Send
        + Sync,
>;

static DELIVERY_FN: OnceLock<DeliveryFn> = OnceLock::new();

pub fn register_delivery_fn(f: DeliveryFn) {
    let _ = DELIVERY_FN.set(f);
}

pub fn list_jobs(config: &Config) -> anyhow::Result<Vec<CronJob>> {
    let store = CronStore::new(&config.data_dir)?;
    let mut jobs = store.list_jobs()?;
    for job in &mut jobs {
        if job.source == "declarative" {
            if let Some(decl) = config.cron.get(&job.id) {
                job.shell_output_format = decl.shell_output_format.clone().into();
            }
        }
    }
    Ok(jobs)
}

pub fn get_job(config: &Config, id: &str) -> anyhow::Result<CronJob> {
    let store = CronStore::new(&config.data_dir)?;
    let mut job = store.get_job(id)?;
    if job.source == "declarative" {
        if let Some(decl) = config.cron.get(&job.id) {
            job.shell_output_format = decl.shell_output_format.clone().into();
        }
    }
    Ok(job)
}

pub fn remove_job(config: &Config, id: &str) -> anyhow::Result<()> {
    let store = CronStore::new(&config.data_dir)?;
    store.remove_job(id)
}

pub fn list_runs(config: &Config, id: &str, limit: usize) -> anyhow::Result<Vec<CronRun>> {
    let store = CronStore::new(&config.data_dir)?;
    store.list_runs(id, limit)
}

#[allow(clippy::too_many_arguments)]
pub fn add_agent_job(
    config: &Config,
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
) -> anyhow::Result<CronJob> {
    let store = CronStore::new(&config.data_dir)?;
    store.add_agent_job(
        agent_alias,
        name,
        schedule,
        prompt,
        session_target,
        model,
        delivery,
        delete_after_run,
        allowed_tools,
        uses_memory,
    )
}

pub fn add_shell_job_with_approval_and_format(
    config: &Config,
    agent_alias: &str,
    name: Option<String>,
    schedule: Schedule,
    command: &str,
    delivery: Option<DeliveryConfig>,
    approved: bool,
    shell_output_format: CronShellOutputFormat,
) -> anyhow::Result<CronJob> {
    let security = galleon_config::policy::SecurityPolicy::for_agent(config, agent_alias)?;
    #[cfg(windows)]
    let dialect = galleon_api::runtime_traits::ShellDialect::WindowsCmd;
    #[cfg(not(windows))]
    let dialect = galleon_api::runtime_traits::ShellDialect::Posix;
    security
        .validate_command_execution_for_shell(command, approved, dialect)
        .map_err(|reason| anyhow::Error::msg(format!("blocked by security policy: {reason}")))?;
    validate_delivery_config(delivery.as_ref())?;
    let store = CronStore::new(&config.data_dir)?;
    store.add_shell_job(
        agent_alias,
        name,
        schedule,
        command,
        delivery,
        shell_output_format.into(),
    )
}

pub fn add_shell_job_with_approval(
    config: &Config,
    agent_alias: &str,
    name: Option<String>,
    schedule: Schedule,
    command: &str,
    delivery: Option<DeliveryConfig>,
    approved: bool,
) -> anyhow::Result<CronJob> {
    add_shell_job_with_approval_and_format(
        config,
        agent_alias,
        name,
        schedule,
        command,
        delivery,
        approved,
        CronShellOutputFormat::default(),
    )
}

pub fn update_shell_job_with_approval(
    config: &Config,
    agent_alias: &str,
    job_id: &str,
    mut patch: CronJobPatch,
    approved: bool,
) -> anyhow::Result<CronJob> {
    let store = CronStore::new(&config.data_dir)?;
    let existing = store.get_job(job_id)?;
    if existing.job_type == JobType::Agent && patch.command.is_some() {
        patch.prompt = patch.command.take().or(patch.prompt);
    }
    if let Some(command) = patch.command.as_deref() {
        let security = galleon_config::policy::SecurityPolicy::for_agent(config, agent_alias)?;
        #[cfg(windows)]
        let dialect = galleon_api::runtime_traits::ShellDialect::WindowsCmd;
        #[cfg(not(windows))]
        let dialect = galleon_api::runtime_traits::ShellDialect::Posix;
        security
            .validate_command_execution_for_shell(command, approved, dialect)
            .map_err(|reason| {
                anyhow::Error::msg(format!("blocked by security policy: {reason}"))
            })?;
    }
    store.update_job(job_id, patch)
}

pub async fn run_manual_job(
    config: &Config,
    job: &CronJob,
    event_tx: &Option<tokio::sync::broadcast::Sender<serde_json::Value>>,
) -> ManualCronRunResult {
    let started_at = chrono::Utc::now();
    let timeout = std::time::Duration::from_secs(300);

    let (mut success, mut output) = match job.job_type {
        JobType::Shell => {
            #[cfg(windows)]
            let mut cmd = {
                let mut cmd = tokio::process::Command::new("cmd");
                cmd.args(["/C", &job.command]);
                cmd
            };
            #[cfg(not(windows))]
            let mut cmd = {
                let mut cmd = tokio::process::Command::new("sh");
                cmd.args(["-c", &job.command]);
                cmd
            };
            cmd.current_dir(&config.data_dir);
            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true);

            match cmd.spawn() {
                Ok(child) => match tokio::time::timeout(timeout, child.wait_with_output()).await {
                    Ok(Ok(out)) => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        let combined = match job.shell_output_format {
                            galleon_infra::cron_types::CronShellOutputFormat::Raw
                                if out.status.success() =>
                            {
                                stdout.trim().to_string()
                            }
                            _ => format!(
                                "status={}\nstdout:\n{}\nstderr:\n{}",
                                out.status,
                                stdout.trim(),
                                stderr.trim()
                            ),
                        };
                        (out.status.success(), combined)
                    }
                    Ok(Err(e)) => (false, format!("spawn error: {e}")),
                    Err(_) => (
                        false,
                        format!("job timed out after {}s", timeout.as_secs_f64()),
                    ),
                },
                Err(e) => (false, format!("spawn error: {e}")),
            }
        }
        JobType::Agent => (
            false,
            "agent cron job execution requires runtime".to_string(),
        ),
    };

    let mut status = if success { "ok" } else { "error" }.to_string();

    if job.delivery.mode == "announce" {
        if let Some(f) = DELIVERY_FN.get() {
            let res = f(
                config.clone(),
                job.delivery.channel.clone().unwrap_or_default(),
                job.delivery.to.clone().unwrap_or_default(),
                job.delivery.thread_id.clone(),
                output.clone(),
            )
            .await;
            if let Err(e) = res {
                let delivery_error = e.to_string();
                if job.delivery.best_effort {
                    if success {
                        status = "degraded".to_string();
                    }
                } else {
                    success = false;
                    status = "error".to_string();
                }
                if output.trim().is_empty() {
                    output = format!("delivery failed: {delivery_error}");
                } else {
                    output.push_str("\n\ndelivery failed: ");
                    output.push_str(&delivery_error);
                }
            }
        }
    }

    let finished_at = chrono::Utc::now();
    let duration_ms = (finished_at - started_at).num_milliseconds();

    if let Ok(store) = CronStore::new(&config.data_dir) {
        let _ = store.record_manual_run(
            &job.id,
            started_at,
            finished_at,
            &status,
            Some(&output),
            duration_ms,
        );
    }

    if let Some(tx) = event_tx {
        let _ = tx.send(serde_json::json!({
            "type": "cron_result",
            "job_id": job.id,
            "success": success,
            "output": &output,
            "manual": true,
            "timestamp": finished_at.to_rfc3339(),
        }));
    }

    ManualCronRunResult {
        job_id: job.id.clone(),
        success,
        status,
        output,
        duration_ms,
        started_at,
        finished_at,
    }
}

pub fn sync_declarative_jobs(
    config: &Config,
    decls: &HashMap<String, CronJobDecl>,
) -> anyhow::Result<()> {
    let store = CronStore::new(&config.data_dir)?;
    store.ensure_table()?;
    let conn = Connection::open(store.db_path())
        .with_context(|| format!("Failed to open cron DB: {}", store.db_path().display()))?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

    if decls.is_empty() {
        conn.execute("DELETE FROM cron_jobs WHERE source = 'declarative'", [])?;
        return Ok(());
    }

    let config_ids: std::collections::HashSet<&str> = decls.keys().map(String::as_str).collect();

    {
        let mut stmt = conn.prepare("SELECT id FROM cron_jobs WHERE source = 'declarative'")?;
        let db_ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        for db_id in db_ids {
            if !config_ids.contains(db_id.as_str()) {
                conn.execute("DELETE FROM cron_jobs WHERE id = ?1", params![db_id])?;
            }
        }
    }

    let now = chrono::Utc::now();
    for (id, decl) in decls {
        let schedule = match &decl.schedule {
            CronScheduleDecl::Cron { expr, tz } => Schedule::Cron {
                expr: expr.clone(),
                tz: tz.clone(),
            },
            CronScheduleDecl::At { at } => {
                let dt = chrono::DateTime::parse_from_rfc3339(at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .map_err(|e| {
                        anyhow::Error::msg(format!(
                            "Invalid RFC 3339 timestamp in declarative job '{id}': {e}"
                        ))
                    })?;
                Schedule::At { at: dt }
            }
            CronScheduleDecl::Every { every_ms } => Schedule::Every {
                every_ms: *every_ms,
            },
        };
        let expression = schedule_cron_expression(&schedule).unwrap_or_default();
        let schedule_json = serde_json::to_string(&schedule)?;
        let job_type = &decl.job_type;
        let session_target = match decl.session_target.as_deref() {
            Some(raw) => SessionTarget::try_parse(raw).unwrap_or(SessionTarget::Isolated),
            None => SessionTarget::Isolated,
        };
        let delivery = match &decl.delivery {
            Some(d) => DeliveryConfig {
                mode: d.mode.clone(),
                channel: d.channel.clone(),
                to: d.to.clone(),
                thread_id: d.thread_id.clone(),
                best_effort: d.best_effort,
            },
            None => DeliveryConfig::default(),
        };
        let delivery_json = serde_json::to_string(&delivery)?;
        let allowed_tools_json = decl
            .allowed_tools
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let command = decl.command.as_deref().unwrap_or("");
        let delete_after_run = matches!(decl.schedule, CronScheduleDecl::At { .. });

        let exists: bool = conn
            .prepare("SELECT COUNT(*) FROM cron_jobs WHERE id = ?1")?
            .query_row(params![id], |row| row.get::<_, i64>(0))
            .map(|c| c > 0)
            .unwrap_or(false);

        if exists {
            let next_run = next_run_for_schedule(&schedule, now)?;
            conn.execute(
                "UPDATE cron_jobs
                 SET expression = ?1, command = ?2, schedule = ?3, job_type = ?4,
                     prompt = ?5, name = ?6, session_target = ?7, model = ?8,
                     enabled = ?9, delivery = ?10, delete_after_run = ?11,
                     allowed_tools = ?12, source = 'declarative', next_run = ?13,
                     uses_memory = ?14
                 WHERE id = ?15",
                params![
                    expression,
                    command,
                    schedule_json,
                    job_type,
                    decl.prompt,
                    decl.name,
                    session_target.as_str(),
                    decl.model,
                    if decl.enabled { 1 } else { 0 },
                    delivery_json,
                    if delete_after_run { 1 } else { 0 },
                    allowed_tools_json,
                    next_run.to_rfc3339(),
                    if decl.uses_memory { 1 } else { 0 },
                    id,
                ],
            )?;
        } else {
            let next_run = next_run_for_schedule(&schedule, now)?;
            conn.execute(
                "INSERT INTO cron_jobs (
                    id, expression, command, schedule, job_type, prompt, name,
                    session_target, model, enabled, delivery, delete_after_run,
                    allowed_tools, created_at, next_run, source, uses_memory,
                    agent_alias
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 'declarative', ?16, '')",
                params![
                    id,
                    expression,
                    command,
                    schedule_json,
                    job_type,
                    decl.prompt,
                    decl.name,
                    session_target.as_str(),
                    decl.model,
                    if decl.enabled { 1 } else { 0 },
                    delivery_json,
                    if delete_after_run { 1 } else { 0 },
                    allowed_tools_json,
                    now.to_rfc3339(),
                    next_run.to_rfc3339(),
                    if decl.uses_memory { 1 } else { 0 },
                ],
            )?;
        }
    }

    Ok(())
}

pub struct GatewayCronRunner;

#[async_trait::async_trait]
impl CronRunner for GatewayCronRunner {
    async fn trigger_job(&self, _job_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
