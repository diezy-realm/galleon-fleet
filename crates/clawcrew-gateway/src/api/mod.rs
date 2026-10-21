//! REST API handlers for the web dashboard.
//! All `/api/*` routes require bearer token authentication (PairingGuard).

pub mod cron;
pub mod sessions;

pub use cron::*;
pub use sessions::*;

use super::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json},
};
use clawcrew_config::schema::{ChannelAliasInfo, Config};
use clawcrew_memory::MemoryEntry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MEMORY_API_CONTENT_MAX_CHARS: usize = 4096;

fn integration_entry_json(
    entry: &clawcrew_config::integrations::IntegrationEntry,
) -> serde_json::Value {
    serde_json::json!({
        "name": &entry.name,
        "description": &entry.description,
        "category": entry.category,
        "category_label": entry.category.label(),
        "status": entry.status,
        // Canonical config map key (provider family key / ChannelsConfig map
        // key) for deep links; null when the entry has no config section.
        "key": &entry.key,
    })
}

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

// ── Query parameters ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MemoryQuery {
    pub query: Option<String>,
    pub category: Option<String>,
    /// Filter memories created at or after (RFC 3339 / ISO 8601)
    pub since: Option<String>,
    /// Filter memories created at or before (RFC 3339 / ISO 8601)
    pub until: Option<String>,
    /// When set to a configured agent alias, the request goes through
    /// that agent's per-alias memory backend (so SQL backends filter by
    /// the agent's UUID, Markdown reads only that agent's directory,
    /// etc.). Omit for the install-wide view.
    #[serde(default)]
    pub agent: Option<String>,
}

#[derive(Deserialize)]
pub struct MemoryStoreBody {
    pub key: String,
    pub content: String,
    pub category: Option<String>,
    /// Configured agent alias to write under. When omitted the store goes
    /// to the install-wide memory backend (no per-agent attribution).
    #[serde(default)]
    pub agent: Option<String>,
}

#[derive(Deserialize)]
pub struct MemoryDeleteQuery {
    /// Configured agent alias to delete from. Omit for the install-wide
    /// backend.
    #[serde(default)]
    pub agent: Option<String>,
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
    pub health: clawcrew_infra::health::HealthSnapshot,
    /// Configured agent alias used for per-agent resolution, or `null` for the install-wide view.
    pub agent_alias: Option<String>,
    /// Self-process resource snapshot; unsupported hosts report zero memory and a null CPU value.
    pub process: clawcrew_infra::process_stats::ProcessStats,
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
    let health = clawcrew_infra::health::snapshot();

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

    let process = clawcrew_infra::process_stats::sample();

    // Upgrade affordance: whether the dashboard should poll for updates / offer
    // the upgrade button, and which restart command to show afterwards.
    let restart = crate::version::detect_restart();

    let body = StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model_provider,
        model,
        temperature,
        uptime_seconds: health.uptime_seconds,
        daemon_started_at: clawcrew_infra::health::daemon_started_at(),
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
/// GET /api/integrations — list all integrations with status
pub async fn handle_api_integrations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let entries = state
        .integration_catalog
        .as_ref()
        .map(|c| c.all_integrations(&config))
        .unwrap_or_default();

    let integrations: Vec<serde_json::Value> = entries.iter().map(integration_entry_json).collect();

    Json(serde_json::json!({"integrations": integrations})).into_response()
}

/// GET /api/integrations/settings — return per-integration settings (enabled + category)
pub async fn handle_api_integrations_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let entries = state
        .integration_catalog
        .as_ref()
        .map(|c| c.all_integrations(&config))
        .unwrap_or_default();

    let mut settings = serde_json::Map::new();
    for entry in &entries {
        let enabled = matches!(
            entry.status,
            clawcrew_config::integrations::IntegrationStatus::Active
        );
        settings.insert(
            entry.name.clone(),
            serde_json::json!({
                "enabled": enabled,
                "category": entry.category,
                "status": entry.status,
            }),
        );
    }

    Json(serde_json::json!({"settings": settings})).into_response()
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

async fn resolve_memory_handle(
    state: &AppState,
    agent_alias: Option<&str>,
) -> Result<std::sync::Arc<dyn clawcrew_memory::Memory>, (StatusCode, Json<serde_json::Value>)> {
    let alias = match agent_alias.map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => a,
        None => return Ok(state.mem.clone()),
    };
    let config = state.config.read().clone();
    if config.agent(alias).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!(
                "Unknown agent {alias:?} (no [agents.{alias}] entry configured)"
            )})),
        ));
    }
    let api_key = config
        .resolved_model_provider_for_agent(alias)
        .and_then(|(_, _, cfg)| cfg.api_key.clone());
    clawcrew_memory::create_memory_for_agent(&config, alias, api_key.as_deref())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::json!({"error": format!("Failed to build per-agent memory: {e:#}")}),
                ),
            )
        })
}

/// GET /api/memory — list or search memory entries
pub async fn handle_api_memory_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<MemoryQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let mem = match resolve_memory_handle(&state, params.agent.as_deref()).await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    // Use recall when query or time range is provided
    if params.query.is_some() || params.since.is_some() || params.until.is_some() {
        let query = params.query.as_deref().unwrap_or("");
        let since = params.since.as_deref();
        let until = params.until.as_deref();
        match mem.recall(query, 50, None, since, until).await {
            Ok(entries) => {
                let entries = match params.category.as_deref() {
                    Some(cat) => entries
                        .into_iter()
                        .filter(|e| e.category.to_string() == cat)
                        .collect(),
                    None => entries,
                };
                Json(serde_json::json!({
                    "entries": sanitize_memory_entries_for_api(entries)
                }))
                .into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Memory recall failed: {e}")})),
            )
                .into_response(),
        }
    } else {
        // List mode
        let category = params.category.as_deref().map(|cat| match cat {
            "core" => clawcrew_memory::MemoryCategory::Core,
            "daily" => clawcrew_memory::MemoryCategory::Daily,
            "conversation" => clawcrew_memory::MemoryCategory::Conversation,
            other => clawcrew_memory::MemoryCategory::Custom(other.to_string()),
        });

        match mem.list(category.as_ref(), None).await {
            Ok(entries) => Json(serde_json::json!({
                "entries": sanitize_memory_entries_for_api(entries)
            }))
            .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Memory list failed: {e}")})),
            )
                .into_response(),
        }
    }
}

fn sanitize_memory_entries_for_api(entries: Vec<MemoryEntry>) -> Vec<MemoryEntry> {
    entries
        .into_iter()
        .map(|mut entry| {
            entry.content = truncate_with_ellipsis_total_chars(entry.content);
            entry
        })
        .collect()
}

fn truncate_with_ellipsis_total_chars(mut s: String) -> String {
    if s.char_indices().nth(MEMORY_API_CONTENT_MAX_CHARS).is_none() {
        return s;
    }

    let keep_chars = MEMORY_API_CONTENT_MAX_CHARS - 3;
    let cut_idx = s
        .char_indices()
        .nth(keep_chars)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len());
    s.truncate(cut_idx);
    s.push_str("...");
    s
}

/// POST /api/memory — store a memory entry
pub async fn handle_api_memory_store(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MemoryStoreBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let category = body
        .category
        .as_deref()
        .map(|cat| match cat {
            "core" => clawcrew_memory::MemoryCategory::Core,
            "daily" => clawcrew_memory::MemoryCategory::Daily,
            "conversation" => clawcrew_memory::MemoryCategory::Conversation,
            other => clawcrew_memory::MemoryCategory::Custom(other.to_string()),
        })
        .unwrap_or(clawcrew_memory::MemoryCategory::Core);

    let mem = match resolve_memory_handle(&state, body.agent.as_deref()).await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    match mem.store(&body.key, &body.content, category, None).await {
        Ok(()) => Json(serde_json::json!({"status": "ok"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Memory store failed: {e}")})),
        )
            .into_response(),
    }
}

/// DELETE /api/memory/:key — delete a memory entry
pub async fn handle_api_memory_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Query(query): Query<MemoryDeleteQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let mem = match resolve_memory_handle(&state, query.agent.as_deref()).await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    match mem.forget(&key).await {
        Ok(deleted) => {
            Json(serde_json::json!({"status": "ok", "deleted": deleted})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Memory forget failed: {e}")})),
        )
            .into_response(),
    }
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
        clawcrew_tools::cli_discovery::discover_cli_tools(&[], &[])
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

/// GET /api/channels — list configured channels with status
pub async fn handle_api_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let health = clawcrew_infra::health::snapshot();
    // One entry per `[channels.<type>.<alias>]` block. Owning
    // agent comes from the agents.<alias>.channels reverse lookup.
    let channels: Vec<serde_json::Value> = config
        .channels_by_alias()
        .into_iter()
        .map(|info| {
            let composite = format!("{}.{}", info.channel_type, info.alias);
            let compiled_key = compiled_readiness_key_for_alias(&config, &info);
            let compiled = clawcrew_channels::listing::is_channel_type_compiled(compiled_key);
            let readiness = channel_readiness(&config, &info, &health, &state);
            let (status, health_status) = if compiled {
                channel_readiness_summary(&readiness)
            } else {
                ("not_compiled", "unavailable")
            };
            serde_json::json!({
                "name": composite,
                "type": info.channel_type,
                "alias": info.alias,
                "owning_agent": info.owning_agent,
                "enabled": info.enabled,
                "compiled": compiled,
                "status": status,
                "message_count": 0,
                "last_message_at": null,
                "health": health_status,
                "readiness": readiness,
            })
        })
        .collect();

    Json(serde_json::json!({ "channels": channels })).into_response()
}

/// POST /api/channels/{channel}/relink — replace a QR channel's pairing.
///
/// `{channel}` is the composite `<type>.<alias>` name returned by
/// `GET /api/channels`. Dispatches to the channel-owned relink hook
/// ([`clawcrew_channels::login_relink::relink`]); the gateway performs no
/// file operations of its own and holds no knowledge of channel session
/// layouts.
///
/// Responses (all authenticated via the standard bearer guard):
///
/// - `200` with `"outcome": "cleared"` — persisted login removed
///   (`"removed"` lists the paths). `"restart_required": true`: the running
///   channel keeps its in-memory session until the daemon restarts it, so
///   the caller follows up with `POST /admin/reload` (which enforces its
///   own, stricter admin policy — relink deliberately does not bypass it).
/// - `200` with `"outcome": "nothing_to_clear"` — the channel supports
///   relinking but held no persisted login; the next start already mints a
///   fresh QR.
/// - `409` with `"outcome": "unsupported"` — the channel type has no relink
///   hook (it does not use QR-pairing sessions) or its feature is not
///   compiled into this binary. **Explicit no-op: nothing was touched.**
/// - `404` — no `[channels.<type>.<alias>]` block matches `{channel}`.
pub async fn handle_api_channel_relink(
    State(state): State<AppState>,
    Path(channel): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let Some(info) = config
        .channels_by_alias()
        .into_iter()
        .find(|info| format!("{}.{}", info.channel_type, info.alias) == channel)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("unknown channel {channel} — use the composite name from GET /api/channels"),
            })),
        )
            .into_response();
    };

    // Resolve the string key to the typed QR-pairing channel once; probe
    // and relink dispatch on the same enum. `None` means the channel type
    // has no relink hook or its feature is not compiled — an explicit
    // no-op conflict where nothing is touched.
    let compiled_key = compiled_readiness_key_for_alias(&config, &info);
    let Some(qr_channel) = clawcrew_channels::listing::qr_pairing_channel(compiled_key) else {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "channel": channel,
                "outcome": "unsupported",
                "error": format!(
                    "channel type {} has no relink operation (it does not use QR-pairing sessions) \
                     or the feature is not compiled into this binary; nothing was changed",
                    info.channel_type
                ),
            })),
        )
            .into_response();
    };

    match clawcrew_channels::login_relink::relink(qr_channel, &config, &info.alias) {
        Ok(clawcrew_channels::login_relink::RelinkOutcome::Cleared { removed }) => {
            ::clawcrew_log::record!(
                INFO,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_attrs(::serde_json::json!({"channel": channel, "removed": removed})),
                "channel persisted login cleared for relink"
            );
            Json(serde_json::json!({
                "channel": channel,
                "outcome": "cleared",
                "removed": removed,
                "restart_required": true,
                "note": "restart the channel (POST /admin/reload) to begin the fresh QR pairing",
            }))
            .into_response()
        }
        Ok(clawcrew_channels::login_relink::RelinkOutcome::NothingToClear) => {
            Json(serde_json::json!({
                "channel": channel,
                "outcome": "nothing_to_clear",
                "removed": [],
                "restart_required": false,
                "note": "no persisted login was stored; the next channel start already begins a fresh QR pairing",
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "channel": channel,
                "error": format!("failed to clear persisted login: {e}"),
            })),
        )
            .into_response(),
    }
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

fn compiled_readiness_key_for_alias<'a>(config: &'a Config, info: &'a ChannelAliasInfo) -> &'a str {
    if info.channel_type == "whatsapp"
        && config
            .channels
            .whatsapp
            .get(&info.alias)
            .is_some_and(|whatsapp| whatsapp.backend_type() == "web")
    {
        "whatsapp-web"
    } else {
        info.channel_type.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ChannelReadinessState {
    Ready,
    Missing,
    Unknown,
}

const CHANNEL_LISTENER_HEALTH_MAX_AGE_SECS: i64 = 30;

#[derive(Debug, Clone, Serialize)]
struct ChannelReadiness {
    enabled: ChannelReadinessState,
    bound_to_agent: ChannelReadinessState,
    authenticated: ChannelReadinessState,
    listening: ChannelReadinessState,
    requirements: Vec<String>,
    notes: Vec<String>,
}

fn channel_readiness(
    config: &clawcrew_config::schema::Config,
    info: &clawcrew_config::schema::ChannelAliasInfo,
    health: &clawcrew_infra::health::HealthSnapshot,
    state: &AppState,
) -> ChannelReadiness {
    let mut readiness = ChannelReadiness {
        enabled: if info.enabled {
            ChannelReadinessState::Ready
        } else {
            ChannelReadinessState::Missing
        },
        bound_to_agent: if info.owning_agent.is_some() {
            ChannelReadinessState::Ready
        } else {
            ChannelReadinessState::Missing
        },
        authenticated: ChannelReadinessState::Unknown,
        listening: ChannelReadinessState::Unknown,
        requirements: Vec::new(),
        notes: Vec::new(),
    };

    if readiness.enabled == ChannelReadinessState::Missing {
        readiness
            .requirements
            .push("Enable this channel alias.".to_string());
    }
    if readiness.bound_to_agent == ChannelReadinessState::Missing {
        readiness
            .requirements
            .push("Bind this channel to an enabled agent.".to_string());
    }

    if readiness.enabled == ChannelReadinessState::Ready
        && readiness.bound_to_agent == ChannelReadinessState::Ready
    {
        if info.channel_type == "webhook" {
            apply_webhook_readiness(config, &info.alias, health, state, &mut readiness);
        } else {
            apply_persisted_login_readiness(config, info, &mut readiness);
        }
    }

    readiness
}

/// Fill `readiness.authenticated` from the channel-owned persisted-login
/// probe (`clawcrew_channels::login_probe`). The probe resolves the same
/// on-disk session signal each QR-pairing channel uses at startup to decide
/// between resuming a session and minting a fresh QR code; nothing is
/// cached and nothing is written. Channel types without a typed QR-pairing
/// key (no probe, or feature not compiled) keep `authenticated: unknown`
/// and the existing "not checked yet" note.
fn apply_persisted_login_readiness(
    config: &clawcrew_config::schema::Config,
    info: &clawcrew_config::schema::ChannelAliasInfo,
    readiness: &mut ChannelReadiness,
) {
    use clawcrew_channels::login_probe::PersistedLogin;

    // Resolve the string key to the typed QR-pairing channel once; all
    // downstream dispatch is on the enum.
    let compiled_key = compiled_readiness_key_for_alias(config, info);
    let Some(channel) = clawcrew_channels::listing::qr_pairing_channel(compiled_key) else {
        readiness.notes.push(format!(
            "Live readiness is not checked for `{}` channels yet.",
            info.channel_type
        ));
        return;
    };

    match clawcrew_channels::login_probe::persisted_login(channel, config, &info.alias) {
        PersistedLogin::Present => {
            readiness.authenticated = ChannelReadinessState::Ready;
            readiness.notes.push(format!(
                "Live listener readiness is not checked for `{}` channels yet.",
                info.channel_type
            ));
        }
        PersistedLogin::Absent => {
            readiness.authenticated = ChannelReadinessState::Missing;
            readiness.requirements.push(
                "Pair this channel: no persisted login session was found on disk.".to_string(),
            );
        }
    }
}

fn channel_readiness_summary(readiness: &ChannelReadiness) -> (&'static str, &'static str) {
    if readiness.enabled == ChannelReadinessState::Missing
        || readiness.bound_to_agent == ChannelReadinessState::Missing
    {
        return ("inactive", "degraded");
    }

    if readiness.authenticated == ChannelReadinessState::Missing
        || readiness.listening == ChannelReadinessState::Missing
    {
        return ("error", "down");
    }

    if readiness.authenticated == ChannelReadinessState::Ready
        && readiness.listening == ChannelReadinessState::Ready
    {
        ("active", "healthy")
    } else {
        // At least one probe is Unknown and none reported Missing: not
        // enough signal to call the channel either healthy or down.
        ("unknown", "degraded")
    }
}

fn apply_webhook_readiness(
    config: &clawcrew_config::schema::Config,
    alias: &str,
    health: &clawcrew_infra::health::HealthSnapshot,
    state: &AppState,
    readiness: &mut ChannelReadiness,
) {
    let Some(webhook) = config.channels.webhook.get(alias) else {
        readiness.authenticated = ChannelReadinessState::Missing;
        readiness.listening = ChannelReadinessState::Missing;
        readiness
            .requirements
            .push("Webhook config block is missing.".to_string());
        return;
    };

    if state.pairing.require_pairing() && !state.pairing.is_paired() {
        readiness.authenticated = ChannelReadinessState::Missing;
        readiness
            .requirements
            .push("Pair the gateway before using the webhook endpoint.".to_string());
    } else {
        readiness.authenticated = ChannelReadinessState::Ready;
    }

    let component = format!("channel:webhook.{alias}");
    let component_health = health.components.get(&component);
    let component_status = component_health.map(|component| component.status.as_str());
    let supervised_listener_ok = component_health.is_some_and(component_health_ok_and_fresh);
    let listen_path = normalized_webhook_path(webhook.listen_path.as_deref());

    if supervised_listener_ok {
        readiness.listening = ChannelReadinessState::Ready;
    } else if component_status == Some("error") {
        readiness.listening = ChannelReadinessState::Missing;
        readiness.requirements.push(format!(
            "Resolve the listener error for `webhook.{alias}` before using this channel."
        ));
    } else {
        readiness.listening = ChannelReadinessState::Missing;
        readiness.requirements.push(format!(
            "Start a channel listener for `webhook.{alias}` on port {}{}.",
            webhook.port, listen_path
        ));
    }
}

fn component_health_ok_and_fresh(component: &clawcrew_infra::health::ComponentHealth) -> bool {
    if component.status != "ok" {
        return false;
    }

    let Ok(updated_at) = chrono::DateTime::parse_from_rfc3339(&component.updated_at) else {
        return false;
    };
    let age = chrono::Utc::now().signed_duration_since(updated_at.with_timezone(&chrono::Utc));
    age >= chrono::Duration::zero()
        && age <= chrono::Duration::seconds(CHANNEL_LISTENER_HEALTH_MAX_AGE_SECS)
}

fn normalized_webhook_path(path: Option<&str>) -> String {
    let trimmed = path.unwrap_or("/webhook").trim();
    if trimmed.is_empty() {
        "/webhook".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

/// GET /api/health — component health snapshot
pub async fn handle_api_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let snapshot = clawcrew_infra::health::snapshot();
    Json(serde_json::json!({"health": snapshot})).into_response()
}

// ── Helpers ─────────────────────────────────────────────────────

pub async fn handle_claude_code_hook(
    State(state): State<AppState>,
    Json(payload): Json<clawcrew_tools::claude_code_runner::ClaudeCodeHookEvent>,
) -> impl IntoResponse {
    // Do not require bearer-token auth: Claude Code subprocesses cannot easily
    // obtain a pairing token, and the hook carries a session_id that ties it
    // back to a session we spawned.
    let _ = &state; // retained for future Slack update wiring

    ::clawcrew_log::record!(INFO, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"session_id": payload.session_id, "event_type": payload.event_type, "tool_name": payload.tool_name, "summary": payload.summary})), "Claude Code hook event received");

    Json(serde_json::json!({ "ok": true }))
}

// Shared test helper: `api_config` tests reuse this AppState builder for the
// agent rename/delete cascade handlers/coverage).

#[cfg(test)]
pub(crate) use tests::test_state;

#[cfg(test)]
pub(crate) mod tests;
