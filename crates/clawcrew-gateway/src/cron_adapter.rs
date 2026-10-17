//! Gateway cron adapter: decouples `api.rs` from direct `clawcrew_runtime::cron` dependency.
#![allow(dead_code)]

use clawcrew_api::runtime_traits::CronRunner;
use clawcrew_config::schema::{Config, CronJobDecl, CronShellOutputFormat};
use clawcrew_infra::cron_types::{
    CronJob, CronJobPatch, CronRun, DeliveryConfig, JobType, ManualCronRunResult, Schedule,
    SessionTarget,
};

fn to_runtime_job_type(t: JobType) -> clawcrew_runtime::cron::JobType {
    match t {
        JobType::Shell => clawcrew_runtime::cron::JobType::Shell,
        JobType::Agent => clawcrew_runtime::cron::JobType::Agent,
    }
}

fn to_infra_job_type(t: clawcrew_runtime::cron::JobType) -> JobType {
    match t {
        clawcrew_runtime::cron::JobType::Shell => JobType::Shell,
        clawcrew_runtime::cron::JobType::Agent => JobType::Agent,
    }
}

fn to_runtime_session_target(t: SessionTarget) -> clawcrew_runtime::cron::SessionTarget {
    match t {
        SessionTarget::Isolated => clawcrew_runtime::cron::SessionTarget::Isolated,
        SessionTarget::Main => clawcrew_runtime::cron::SessionTarget::Main,
    }
}

fn to_infra_session_target(t: clawcrew_runtime::cron::SessionTarget) -> SessionTarget {
    match t {
        clawcrew_runtime::cron::SessionTarget::Isolated => SessionTarget::Isolated,
        clawcrew_runtime::cron::SessionTarget::Main => SessionTarget::Main,
    }
}

fn to_runtime_schedule(s: Schedule) -> clawcrew_runtime::cron::Schedule {
    match s {
        Schedule::Cron { expr, tz } => clawcrew_runtime::cron::Schedule::Cron { expr, tz },
        Schedule::At { at } => clawcrew_runtime::cron::Schedule::At { at },
        Schedule::Every { every_ms } => clawcrew_runtime::cron::Schedule::Every { every_ms },
    }
}

fn to_infra_schedule(s: clawcrew_runtime::cron::Schedule) -> Schedule {
    match s {
        clawcrew_runtime::cron::Schedule::Cron { expr, tz } => Schedule::Cron { expr, tz },
        clawcrew_runtime::cron::Schedule::At { at } => Schedule::At { at },
        clawcrew_runtime::cron::Schedule::Every { every_ms } => Schedule::Every { every_ms },
    }
}

fn to_runtime_delivery(d: DeliveryConfig) -> clawcrew_runtime::cron::DeliveryConfig {
    clawcrew_runtime::cron::DeliveryConfig {
        mode: d.mode,
        channel: d.channel,
        to: d.to,
        thread_id: d.thread_id,
        best_effort: d.best_effort,
    }
}

fn to_infra_delivery(d: clawcrew_runtime::cron::DeliveryConfig) -> DeliveryConfig {
    DeliveryConfig {
        mode: d.mode,
        channel: d.channel,
        to: d.to,
        thread_id: d.thread_id,
        best_effort: d.best_effort,
    }
}

fn to_infra_job(j: clawcrew_runtime::cron::CronJob) -> CronJob {
    CronJob {
        id: j.id,
        expression: j.expression,
        schedule: to_infra_schedule(j.schedule),
        command: j.command,
        prompt: j.prompt,
        name: j.name,
        job_type: to_infra_job_type(j.job_type),
        session_target: to_infra_session_target(j.session_target),
        model: j.model,
        agent_alias: j.agent_alias,
        enabled: j.enabled,
        delivery: to_infra_delivery(j.delivery),
        delete_after_run: j.delete_after_run,
        allowed_tools: j.allowed_tools,
        uses_memory: j.uses_memory,
        source: j.source,
        shell_output_format: j.shell_output_format.into(),
        created_at: j.created_at,
        next_run: j.next_run,
        last_run: j.last_run,
        last_status: j.last_status,
        last_output: j.last_output,
    }
}

fn to_infra_run(r: clawcrew_runtime::cron::CronRun) -> CronRun {
    CronRun {
        id: r.id,
        job_id: r.job_id,
        started_at: r.started_at,
        finished_at: r.finished_at,
        status: r.status,
        output: r.output,
        duration_ms: r.duration_ms,
    }
}

fn to_runtime_patch(p: CronJobPatch) -> clawcrew_runtime::cron::CronJobPatch {
    clawcrew_runtime::cron::CronJobPatch {
        schedule: p.schedule.map(to_runtime_schedule),
        command: p.command,
        prompt: p.prompt,
        name: p.name,
        enabled: p.enabled,
        delivery: p.delivery.map(to_runtime_delivery),
        model: p.model,
        session_target: p.session_target.map(to_runtime_session_target),
        delete_after_run: p.delete_after_run,
        allowed_tools: p.allowed_tools,
        uses_memory: p.uses_memory,
        shell_output_format: p.shell_output_format.map(Into::into),
    }
}

fn to_infra_manual_result(
    r: clawcrew_runtime::cron::scheduler::ManualCronRunResult,
) -> ManualCronRunResult {
    ManualCronRunResult {
        job_id: r.job_id,
        success: r.success,
        status: r.status,
        output: r.output,
        duration_ms: r.duration_ms,
        started_at: r.started_at,
        finished_at: r.finished_at,
    }
}

pub fn list_jobs(config: &Config) -> anyhow::Result<Vec<CronJob>> {
    let jobs = clawcrew_runtime::cron::list_jobs(config)?;
    Ok(jobs.into_iter().map(to_infra_job).collect())
}

pub fn get_job(config: &Config, id: &str) -> anyhow::Result<CronJob> {
    let job = clawcrew_runtime::cron::get_job(config, id)?;
    Ok(to_infra_job(job))
}

pub fn remove_job(config: &Config, id: &str) -> anyhow::Result<()> {
    clawcrew_runtime::cron::remove_job(config, id)
}

pub fn list_runs(config: &Config, id: &str, limit: usize) -> anyhow::Result<Vec<CronRun>> {
    let runs = clawcrew_runtime::cron::list_runs(config, id, limit)?;
    Ok(runs.into_iter().map(to_infra_run).collect())
}

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
    let job = clawcrew_runtime::cron::add_agent_job(
        config,
        agent_alias,
        name,
        to_runtime_schedule(schedule),
        prompt,
        to_runtime_session_target(session_target),
        model,
        delivery.map(to_runtime_delivery),
        delete_after_run,
        allowed_tools,
        uses_memory,
    )?;
    Ok(to_infra_job(job))
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
    let job = clawcrew_runtime::cron::add_shell_job_with_approval_and_format(
        config,
        agent_alias,
        name,
        to_runtime_schedule(schedule),
        command,
        delivery.map(to_runtime_delivery),
        approved,
        shell_output_format,
    )?;
    Ok(to_infra_job(job))
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
    let job = clawcrew_runtime::cron::add_shell_job_with_approval(
        config,
        agent_alias,
        name,
        to_runtime_schedule(schedule),
        command,
        delivery.map(to_runtime_delivery),
        approved,
    )?;
    Ok(to_infra_job(job))
}

pub fn update_shell_job_with_approval(
    config: &Config,
    agent_alias: &str,
    job_id: &str,
    patch: CronJobPatch,
    approved: bool,
) -> anyhow::Result<CronJob> {
    let job = clawcrew_runtime::cron::update_shell_job_with_approval(
        config,
        agent_alias,
        job_id,
        to_runtime_patch(patch),
        approved,
    )?;
    Ok(to_infra_job(job))
}

pub async fn run_manual_job(
    config: &Config,
    job: &CronJob,
    event_tx: &Option<tokio::sync::broadcast::Sender<serde_json::Value>>,
) -> ManualCronRunResult {
    let runtime_job: clawcrew_runtime::cron::CronJob = serde_json::from_value(
        serde_json::to_value(job).expect("serialize CronJob"),
    )
    .expect("deserialize to runtime CronJob");

    let res = clawcrew_runtime::cron::scheduler::run_manual_job(
        config,
        &runtime_job,
        clawcrew_runtime::cron::scheduler::CronDeliveryContext::GatewayManual,
        event_tx,
    )
    .await;
    to_infra_manual_result(res)
}

pub fn sync_declarative_jobs(
    config: &Config,
    decls: &std::collections::HashMap<String, CronJobDecl>,
) -> anyhow::Result<()> {
    clawcrew_runtime::cron::sync_declarative_jobs(config, decls)
}

#[cfg(test)]
pub fn register_delivery_fn(f: clawcrew_runtime::cron::scheduler::DeliveryFn) {
    clawcrew_runtime::cron::scheduler::register_delivery_fn(f);
}

pub struct GatewayCronRunner;

#[async_trait::async_trait]
impl CronRunner for GatewayCronRunner {
    async fn trigger_job(&self, _job_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
