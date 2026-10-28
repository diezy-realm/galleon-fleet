//! REST API handlers for the web dashboard.
//! All `/api/*` routes require bearer token authentication (PairingGuard).

pub mod channels;
pub mod cron;
pub mod integrations;
pub mod memory;
pub mod sessions;

pub use channels::*;
pub use cron::*;
pub use integrations::*;
pub use memory::*;
pub use sessions::*;

use super::AppState;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ── Bearer token auth extractor ─────────────────────────────────

/// Extract and validate bearer token from Authorization header.
pub(crate) fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
}

/// Verify bearer token against PairingGuard. Returns error response if unauthorized.
pub(crate) fn require_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if !state.pairing.require_pairing() {
        return Ok(());
    }

    let token = extract_bearer_token(headers).unwrap_or("");
    // Defense-in-depth: reject empty tokens explicitly so a future
    // refactor of is_authenticated cannot accidentally treat "" as valid.
    if token.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            })),
        ));
    }
    if state.pairing.is_authenticated(token) {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            })),
        ))
    }
}

// ── Handlers ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct StatusNodes {
    pub connected: Vec<String>,
    pub mdns_peers: Vec<crate::nodes::mdns::MdnsPeerSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct StatusResponse {
    pub version: String,
    /// Dotted `<type>.<alias>` of the resolved model provider, or `null` when none is configured.
    /// The unqualified provider name is reserved; this field is always qualified.
    pub model_provider: Option<String>,
    pub model: String,
    /// Resolved model temperature, or `null` when no temperature is configured.
    pub temperature: Option<f64>,
    pub uptime_seconds: u64,
    /// RFC 3339 UTC wall-clock time when the daemon started.
    pub daemon_started_at: String,
    pub gateway_port: u16,
    pub locale: String,
    pub memory_backend: String,
    pub paired: bool,
    pub channels: BTreeMap<String, bool>,
    pub nodes: StatusNodes,
    pub health: galleon_infra::health::HealthSnapshot,
    /// Configured agent alias used for per-agent resolution, or `null` for the install-wide view.
    pub agent_alias: Option<String>,
    /// Self-process resource snapshot; unsupported hosts report zero memory and a null CPU value.
    pub process: galleon_infra::process_stats::ProcessStats,
    /// Whether the gateway polls for newer releases and shows an update indicator.
    pub check_updates: bool,
    /// Whether browser-triggered self-upgrade is enabled.
    pub allow_self_upgrade: bool,
    /// How the daemon is restarted after an upgrade: supervised, desktop_supervised,
    /// self-respawn, or manual.
    pub restart_mode: crate::version::RestartMode,
    /// Operator-facing command or instruction for completing an upgrade restart.
    pub restart_hint: String,
}

/// Query parameters for `GET /api/status`. Pass `?agent=<alias>` to
/// have `model_provider`, `model`, `temperature`, and `memory_backend`
/// reflect that specific agent's resolved config; omit it for the
/// install-wide summary.
#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    #[serde(default)]
    pub agent: Option<String>,
}

/// GET /api/status — system status overview
pub async fn handle_api_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<StatusQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let health = galleon_infra::health::snapshot();

    // Per-alias map keyed by composite `<type>.<alias>`. Every
    // populated `[channels.<type>.<alias>]` is a separate dashboard row.
    let mut channels = BTreeMap::new();
    for info in config.channels_by_alias() {
        let composite = format!("{}.{}", info.channel_type, info.alias);
        channels.insert(composite, true);
    }

    let locale = config
        .locale
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(galleon_i18n::detect_locale);

    // Per-agent resolution when `?agent=<alias>` is supplied. Falls back
    // to the install-wide first-of-each view when the alias is unknown
    // (so the dashboard's old shape still renders during onboarding,
    // before any agent exists).
    let agent_alias = query.agent.as_deref().filter(|s| !s.trim().is_empty());
    let (model_provider, model, temperature, memory_backend) =
        match agent_alias.and_then(|alias| config.agent(alias).map(|a| (alias, a))) {
            Some((alias, agent)) => {
                let provider_ref = if agent.model_provider.is_empty() {
                    None
                } else {
                    Some(agent.model_provider.as_str().to_string())
                };
                let resolved = config.resolved_model_provider_for_agent(alias);
                let model = resolved
                    .as_ref()
                    .and_then(|(_, _, cfg)| cfg.model.clone())
                    .unwrap_or_default();
                let temperature: Option<f64> =
                    resolved.as_ref().and_then(|(_, _, cfg)| cfg.temperature);
                let backend_kind = agent.memory.backend;
                let backend = serde_json::to_value(backend_kind)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| format!("{backend_kind:?}").to_lowercase());
                (provider_ref, model, temperature, backend)
            }
            None => (
                config
                    .providers
                    .models
                    .iter_entries()
                    .next()
                    .map(|(ty, alias, _)| format!("{ty}.{alias}")),
                state.model.clone(),
                state.temperature,
                state.mem.name().to_string(),
            ),
        };

    let process = galleon_infra::process_stats::sample();

    // Upgrade affordance: whether the dashboard should poll for updates / offer
    // the upgrade button, and which restart command to show afterwards.
    let restart = crate::version::detect_restart();

    let body = StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model_provider,
        model,
        temperature,
        uptime_seconds: health.uptime_seconds,
        daemon_started_at: galleon_infra::health::daemon_started_at(),
        gateway_port: config.gateway.port,
        locale,
        memory_backend,
        paired: state.pairing.is_paired(),
        channels,
        nodes: StatusNodes {
            connected: state.node_registry.node_ids(),
            mdns_peers: state.mdns_peer_registry.snapshots(),
        },
        health,
        agent_alias: agent_alias.map(String::from),
        process,
        check_updates: config.gateway.check_updates,
        allow_self_upgrade: config.gateway.allow_self_upgrade,
        restart_mode: restart.mode,
        restart_hint: restart.hint,
    };

    Json(body).into_response()
}

#[derive(Debug, Deserialize)]
pub struct ToolsQuery {
    #[serde(default)]
    pub agent: Option<String>,
}

/// GET /api/tools - list registered tool specs, optionally scoped to `?agent=`
pub async fn handle_api_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ToolsQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let registry = query
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .and_then(|alias| state.tools_registry_by_agent.get(alias).cloned())
        .unwrap_or_else(|| std::sync::Arc::clone(&state.tools_registry));

    let tools: Vec<serde_json::Value> = registry
        .iter()
        .map(|spec| {
            let mut tool = serde_json::json!({
                "name": spec.name,
                "description": spec.description,
                "parameters": spec.parameters,
            });
            if let Some(output) = &spec.output {
                tool["output"] = output.clone();
            }
            if !spec.param_domains.is_empty() {
                tool["param_domains"] = serde_json::json!(spec.param_domains);
            }
            tool
        })
        .collect();

    Json(serde_json::json!({"tools": tools})).into_response()
}

/// POST /api/doctor — run diagnostics
pub async fn handle_api_doctor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let results = if let Some(diag) = &state.diagnostics {
        diag.run_diagnostics()
    } else {
        Vec::new()
    };

    let ok_count = results
        .iter()
        .filter(|r| r.severity == clawcrew_api::doctor::Severity::Ok)
        .count();
    let warn_count = results
        .iter()
        .filter(|r| r.severity == clawcrew_api::doctor::Severity::Warn)
        .count();
    let error_count = results
        .iter()
        .filter(|r| r.severity == clawcrew_api::doctor::Severity::Error)
        .count();

    Json(serde_json::json!({
        "results": results,
        "summary": {
            "ok": ok_count,
            "warnings": warn_count,
            "errors": error_count,
        }
    }))
    .into_response()
}

/// Query parameters for `GET /api/cost`. When `agent` is set, the
/// returned summary filters to records attributed to that alias.
#[derive(Debug, Deserialize)]
pub struct CostQuery {
    #[serde(default)]
    pub agent: Option<String>,
    /// RFC3339 UTC instants — caller-computed window bounds. The
    /// dashboard derives them in the operator's local timezone so
    /// "today" means the operator's today, not the daemon's UTC today.
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}

/// GET /api/cost — cost summary over `[from, to)` (either bound omitted
/// = unbounded on that side). Pass `?agent=<alias>` for the per-agent
/// view, which ignores from/to and returns the alias's session+daily
/// rollup.
pub async fn handle_api_cost(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CostQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let parse_bound = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    };
    let from = query.from.as_deref().and_then(parse_bound);
    let to = query.to.as_deref().and_then(parse_bound);

    if let Some(ref tracker) = state.cost_tracker {
        let result = match query.agent.as_deref().filter(|s| !s.is_empty()) {
            Some(alias) => tracker.get_summary_for_agent(alias),
            None => tracker.get_summary_in_bounds(from, to),
        };
        match result {
            Ok(summary) => Json(serde_json::json!({"cost": summary})).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Cost summary failed: {e}")})),
            )
                .into_response(),
        }
    } else {
        Json(serde_json::json!({
            "cost": {
                "session_cost_usd": 0.0,
                "daily_cost_usd": 0.0,
                "monthly_cost_usd": 0.0,
                "total_tokens": 0,
                "request_count": 0,
                "by_model": {},
                "by_agent": {},
            }
        }))
        .into_response()
    }
}

/// GET /api/cli-tools — discovered CLI tools
pub async fn handle_api_cli_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // `discover_cli_tools` spawns child processes and blocks; keep it off the
    // async executor so a slow PATH scan can't stall other gateway requests.
    let tools = match tokio::task::spawn_blocking(|| {
        galleon_tools::cli_discovery::discover_cli_tools(&[], &[])
    })
    .await
    {
        Ok(tools) => tools,
        Err(e) => {
            // The blocking task panicked; degrade to an empty list rather
            // than failing the request, but record why it was empty.
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                "cli-tools discovery task failed; returning empty list"
            );
            Vec::new()
        }
    };

    Json(serde_json::json!({"cli_tools": tools})).into_response()
}

/// GET /api/tuis — list connected TUI sessions
pub async fn handle_api_tuis(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let tuis: Vec<serde_json::Value> = state
        .tui_registry
        .as_ref()
        .map(|r| {
            r.list_connected_tuis()
                .into_iter()
                .map(|e| {
                    serde_json::json!({
                        "tui_id": e.tui_id,
                        "connected_at": e.connected_at,
                        "peer_label": e.peer_label,
                        "transport": e.transport,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Json(serde_json::json!({ "tuis": tuis })).into_response()
}

/// GET /api/health — component health snapshot
pub async fn handle_api_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let snapshot = galleon_infra::health::snapshot();
    Json(serde_json::json!({"health": snapshot})).into_response()
}

// ── Helpers ─────────────────────────────────────────────────────

pub async fn handle_claude_code_hook(
    State(state): State<AppState>,
    Json(payload): Json<galleon_tools::claude_code_runner::ClaudeCodeHookEvent>,
) -> impl IntoResponse {
    // Do not require bearer-token auth: Claude Code subprocesses cannot easily
    // obtain a pairing token, and the hook carries a session_id that ties it
    // back to a session we spawned.
    let _ = &state; // retained for future Slack update wiring

    ::clawcrew_log::record!(
        INFO,
        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
            ::serde_json::json!({
                "session_id": payload.session_id,
                "event_type": payload.event_type,
                "tool_name": payload.tool_name,
                "summary": payload.summary
            })
        ),
        "Claude Code hook event received"
    );

    Json(serde_json::json!({ "ok": true }))
}

// Shared test helper: `api_config` tests reuse this AppState builder for the
// agent rename/delete cascade handlers/coverage).

#[cfg(test)]
pub(crate) use axum::extract::Path;
#[cfg(test)]
pub(crate) use tests::test_state;

#[cfg(test)]
pub(crate) mod tests;
