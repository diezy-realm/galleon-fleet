//! Cron REST API handlers.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::Deserialize;

use super::require_auth;
use crate::AppState;

#[derive(Deserialize)]
pub struct CronRunsQuery {
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct CronAddBody {
    /// Configured agent alias the cron job will run as. Required —
    /// there is no default agent.
    pub agent: String,
    pub name: Option<String>,
    pub schedule: String,
    pub tz: Option<String>,
    pub command: Option<String>,
    pub job_type: Option<String>,
    pub prompt: Option<String>,
    pub delivery: Option<clawcrew_infra::cron_types::DeliveryConfig>,
    /// Agent session context: `"isolated"` (default) or `"main"`. For agent jobs,
    /// a present value that is not one of those two names is rejected; omitted
    /// keeps the isolated default. Same contract as the `cron_add` tool. Shell
    /// jobs ignore this field.
    pub session_target: Option<String>,
    pub model: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub delete_after_run: Option<bool>,
    /// If false, disable memory recall for this agent cron job (default: true).
    pub uses_memory: Option<bool>,
    /// Shell output format for shell-type cron jobs. `"wrapped"` (default) or
    /// `"raw"`. Only meaningful when `job_type` is `"shell"` (or inferred as
    /// shell when no prompt is given). Rejected for agent jobs.
    #[serde(default)]
    pub shell_output_format: Option<clawcrew_config::schema::CronShellOutputFormat>,
}

#[derive(Deserialize)]
pub struct CronPatchBody {
    /// Configured agent alias whose risk profile gates the new shell
    /// command. Only consulted when `command` is being patched; optional
    /// otherwise (e.g. a pure schedule/name change or an enable/disable
    /// toggle), so non-command patches need not supply it.
    #[serde(default)]
    pub agent: String,
    pub name: Option<String>,
    pub schedule: Option<String>,
    pub tz: Option<String>,
    pub clear_tz: Option<bool>,
    pub command: Option<String>,
    pub prompt: Option<String>,
    /// Toggle the job on/off without deleting it (pause/resume). `None` leaves
    /// the current state unchanged.
    pub enabled: Option<bool>,
    /// If false, disable memory recall for this agent cron job (default: true).
    pub uses_memory: Option<bool>,
    /// Shell output format override. `"wrapped"` (default) or `"raw"`.
    /// Only applied to shell-type jobs.
    #[serde(default)]
    pub shell_output_format: Option<clawcrew_config::schema::CronShellOutputFormat>,
}

enum CronTimezonePatch {
    Preserve,
    Set(String),
    Clear,
}

pub(crate) fn bad_request(message: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": message.into() })),
    )
}

fn normalize_optional_timezone(
    tz: Option<String>,
) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    match tz {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Err(bad_request(
                    "tz must be a non-empty IANA timezone; use clear_tz=true to clear it",
                ))
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        None => Ok(None),
    }
}

fn parse_timezone_patch(
    tz: Option<String>,
    clear_tz: Option<bool>,
) -> Result<CronTimezonePatch, (StatusCode, Json<serde_json::Value>)> {
    let tz = normalize_optional_timezone(tz)?;
    let clear_tz = clear_tz.unwrap_or(false);

    if clear_tz && tz.is_some() {
        return Err(bad_request("Provide either tz or clear_tz=true, not both"));
    }

    if clear_tz {
        Ok(CronTimezonePatch::Clear)
    } else if let Some(tz) = tz {
        Ok(CronTimezonePatch::Set(tz))
    } else {
        Ok(CronTimezonePatch::Preserve)
    }
}

fn cron_schedule_from_api(
    expr: String,
    tz: Option<String>,
) -> Result<clawcrew_infra::cron_types::Schedule, (StatusCode, Json<serde_json::Value>)> {
    let schedule = clawcrew_infra::cron_types::Schedule::Cron { expr, tz };
    clawcrew_infra::cron_schedule::validate_schedule(&schedule, chrono::Utc::now())
        .map_err(|e| bad_request(format!("Invalid cron schedule: {e}")))?;
    Ok(schedule)
}

/// GET /api/cron — list cron jobs
pub async fn handle_api_cron_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    match crate::cron_adapter::list_jobs(&config) {
        Ok(jobs) => Json(serde_json::json!({"jobs": jobs})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to list cron jobs: {e}")})),
        )
            .into_response(),
    }
}

/// POST /api/cron — add a new cron job
pub async fn handle_api_cron_add(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CronAddBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let CronAddBody {
        agent: agent_alias,
        name,
        schedule,
        tz,
        command,
        job_type,
        prompt,
        delivery,
        session_target,
        model,
        allowed_tools,
        delete_after_run,
        uses_memory,
        shell_output_format,
    } = body;

    let config = state.config.read().clone();
    if config.agent(&agent_alias).is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!(
                "Unknown agent {agent_alias:?} (no [agents.{agent_alias}] entry configured)"
            )})),
        )
            .into_response();
    }
    let tz = match normalize_optional_timezone(tz) {
        Ok(tz) => tz,
        Err(e) => return e.into_response(),
    };
    let schedule = match cron_schedule_from_api(schedule, tz) {
        Ok(schedule) => schedule,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = clawcrew_infra::cron_types::validate_delivery_config(delivery.as_ref()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Failed to add cron job: {e}")})),
        )
            .into_response();
    }

    // Determine job type: explicit field, or infer "agent" when prompt is provided.
    let is_agent =
        matches!(job_type.as_deref(), Some("agent")) || (job_type.is_none() && prompt.is_some());

    let result = if is_agent {
        if shell_output_format.is_some() {
            return bad_request(
                "shell_output_format is not applicable to agent jobs; agent execution ignores it",
            )
            .into_response();
        }
        let prompt = match prompt.as_deref() {
            Some(p) if !p.trim().is_empty() => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Missing 'prompt' for agent job"})),
                )
                    .into_response();
            }
        };

        let session_target = match session_target {
            Some(raw) => match clawcrew_infra::cron_types::SessionTarget::try_parse(&raw) {
                Ok(target) => target,
                Err(e) => return bad_request(e).into_response(),
            },
            None => clawcrew_infra::cron_types::SessionTarget::Isolated,
        };

        let default_delete = matches!(schedule, clawcrew_infra::cron_types::Schedule::At { .. });
        let delete_after_run = delete_after_run.unwrap_or(default_delete);

        crate::cron_adapter::add_agent_job(
            &config,
            &agent_alias,
            name,
            schedule,
            prompt,
            session_target,
            model,
            delivery,
            delete_after_run,
            allowed_tools,
            uses_memory.unwrap_or(true),
        )
    } else {
        let command = match command.as_deref() {
            Some(c) if !c.trim().is_empty() => c,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Missing 'command' for shell job"})),
                )
                    .into_response();
            }
        };

        let fmt = shell_output_format.unwrap_or_default();
        crate::cron_adapter::add_shell_job_with_approval_and_format(
            &config,
            &agent_alias,
            name,
            schedule,
            command,
            delivery,
            false,
            fmt,
        )
    };

    match result {
        Ok(job) => Json(serde_json::json!({"status": "ok", "job": job})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to add cron job: {e}")})),
        )
            .into_response(),
    }
}

/// GET /api/cron/:id/runs — list recent runs for a cron job
pub async fn handle_api_cron_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<CronRunsQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let limit = params.limit.unwrap_or(20).clamp(1, 100) as usize;
    let config = state.config.read().clone();

    // Verify the job exists before listing runs.
    if let Err(e) = crate::cron_adapter::get_job(&config, &id) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Cron job not found: {e}")})),
        )
            .into_response();
    }

    match crate::cron_adapter::list_runs(&config, &id, limit) {
        Ok(runs) => {
            let runs_json: Vec<serde_json::Value> = runs
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "job_id": r.job_id,
                        "started_at": r.started_at.to_rfc3339(),
                        "finished_at": r.finished_at.to_rfc3339(),
                        "status": r.status,
                        "output": r.output,
                        "duration_ms": r.duration_ms,
                    })
                })
                .collect();
            Json(serde_json::json!({"runs": runs_json})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to list cron runs: {e}")})),
        )
            .into_response(),
    }
}

/// POST /api/cron/:id/run — trigger a cron job manually
pub async fn handle_api_cron_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();

    let job = match crate::cron_adapter::get_job(&config, &id) {
        Ok(job) => job,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Cron job not found: {e}")})),
            )
                .into_response();
        }
    };

    let event_tx = Some(state.event_tx.clone());
    let result = crate::cron_adapter::run_manual_job(&config, &job, &event_tx).await;

    Json(serde_json::json!({
        "status": result.status,
        "job_id": result.job_id,
        "success": result.success,
        "output": result.output,
        "duration_ms": result.duration_ms,
        "started_at": result.started_at.to_rfc3339(),
        "finished_at": result.finished_at.to_rfc3339(),
    }))
    .into_response()
}

/// PATCH /api/cron/:id — update an existing cron job
pub async fn handle_api_cron_patch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<CronPatchBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let agent_alias = body.agent.clone();
    let CronPatchBody {
        agent: _,
        name,
        schedule: schedule_expr,
        tz,
        clear_tz,
        command,
        prompt,
        enabled,
        uses_memory,
        shell_output_format,
    } = body;
    let timezone_patch = match parse_timezone_patch(tz, clear_tz) {
        Ok(patch) => patch,
        Err(e) => return e.into_response(),
    };

    let existing = match crate::cron_adapter::get_job(&config, &id) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Cron job not found: {e}")})),
            )
                .into_response();
        }
    };
    let is_agent = matches!(
        existing.job_type,
        clawcrew_infra::cron_types::JobType::Agent
    );
    if shell_output_format.is_some() {
        if is_agent {
            return bad_request(
                "shell_output_format is not applicable to agent jobs; agent execution ignores it",
            )
            .into_response();
        }
        if existing.source == "declarative" {
            return bad_request(format!(
                "shell_output_format for declarative job '{id}' is set via \
                 cron.{id}.shell_output_format in config.toml, not the API; \
                 the DB column is not read for declarative jobs and this PATCH would have no effect"
            ))
            .into_response();
        }
    }
    let setting_shell_command = !is_agent && (command.is_some() || prompt.is_some());
    if setting_shell_command && config.agent(&agent_alias).is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!(
                "Unknown agent {a:?} (no [agents.{a}] entry configured)",
                a = agent_alias
            )})),
        )
            .into_response();
    }
    let new_expr = schedule_expr
        .as_deref()
        .map(str::trim)
        .filter(|expr| !expr.is_empty())
        .map(str::to_string);
    let timezone_changed = !matches!(timezone_patch, CronTimezonePatch::Preserve);
    let schedule = if new_expr.is_some() || timezone_changed {
        let (expr, existing_tz) = match (&existing.schedule, new_expr) {
            (_, Some(expr)) => {
                let existing_tz = match &existing.schedule {
                    clawcrew_infra::cron_types::Schedule::Cron { tz, .. } => tz.clone(),
                    _ => None,
                };
                (expr, existing_tz)
            }
            (clawcrew_infra::cron_types::Schedule::Cron { expr, tz }, None) => {
                (expr.clone(), tz.clone())
            }
            (_, None) => {
                return bad_request("tz can only be updated on cron schedules").into_response();
            }
        };
        let tz = match timezone_patch {
            CronTimezonePatch::Preserve => existing_tz,
            CronTimezonePatch::Set(tz) => Some(tz),
            CronTimezonePatch::Clear => None,
        };
        match cron_schedule_from_api(expr, tz) {
            Ok(schedule) => Some(schedule),
            Err(e) => return e.into_response(),
        }
    } else {
        None
    };
    let (patch_command, patch_prompt) = if is_agent {
        (None, command.or(prompt))
    } else {
        (command.or(prompt), None)
    };

    let patch = clawcrew_infra::cron_types::CronJobPatch {
        name,
        schedule,
        command: patch_command,
        prompt: patch_prompt,
        enabled,
        uses_memory,
        shell_output_format: shell_output_format.map(Into::into),
        ..clawcrew_infra::cron_types::CronJobPatch::default()
    };

    match crate::cron_adapter::update_shell_job_with_approval(
        &config,
        &agent_alias,
        &id,
        patch,
        false,
    ) {
        Ok(job) => Json(serde_json::json!({"status": "ok", "job": job})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to update cron job: {e}")})),
        )
            .into_response(),
    }
}

/// DELETE /api/cron/:id — remove a cron job
pub async fn handle_api_cron_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    match crate::cron_adapter::remove_job(&config, &id) {
        Ok(()) => Json(serde_json::json!({"status": "ok"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to remove cron job: {e}")})),
        )
            .into_response(),
    }
}

/// GET /api/cron/settings — return cron subsystem settings
pub async fn handle_api_cron_settings_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    Json(serde_json::json!({
        "enabled": config.scheduler.enabled,
        "catch_up_on_startup": config.scheduler.catch_up_on_startup,
        "max_run_history": config.scheduler.max_run_history,
    }))
    .into_response()
}

/// PATCH /api/cron/settings — update cron subsystem settings
pub async fn handle_api_cron_settings_patch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // Held through the swap below so a concurrent config writer can't land
    // between this read and the save.
    let _cfg_guard = std::sync::Arc::clone(&state.config_write_lock)
        .lock_owned()
        .await;
    let mut config = state.config.read().clone();

    if let Some(v) = body.get("enabled").and_then(|v| v.as_bool()) {
        config.scheduler.enabled = v;
        config.mark_dirty("scheduler.enabled");
    }
    if let Some(v) = body.get("catch_up_on_startup").and_then(|v| v.as_bool()) {
        config.scheduler.catch_up_on_startup = v;
        config.mark_dirty("scheduler.catch-up-on-startup");
    }
    if let Some(v) = body.get("max_run_history").and_then(|v| v.as_u64()) {
        config.scheduler.max_run_history = u32::try_from(v).unwrap_or(u32::MAX);
        config.mark_dirty("scheduler.max-run-history");
    }

    if let Err(e) = config.save_dirty().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to save config: {e}")})),
        )
            .into_response();
    }

    *state.config.write() = config.clone();

    Json(serde_json::json!({
        "status": "ok",
        "enabled": config.scheduler.enabled,
        "catch_up_on_startup": config.scheduler.catch_up_on_startup,
        "max_run_history": config.scheduler.max_run_history,
    }))
    .into_response()
}
