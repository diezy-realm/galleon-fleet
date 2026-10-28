//! Channels REST API handlers and readiness checks.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use clawcrew_config::schema::{ChannelAliasInfo, Config};
use serde::Serialize;

use super::require_auth;
use crate::AppState;

/// GET /api/channels — list configured channels with status
pub async fn handle_api_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let health = galleon_infra::health::snapshot();
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

pub(crate) fn compiled_readiness_key_for_alias<'a>(
    config: &'a Config,
    info: &'a ChannelAliasInfo,
) -> &'a str {
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
pub(crate) enum ChannelReadinessState {
    Ready,
    Missing,
    Unknown,
}

pub(crate) const CHANNEL_LISTENER_HEALTH_MAX_AGE_SECS: i64 = 30;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChannelReadiness {
    pub(crate) enabled: ChannelReadinessState,
    pub(crate) bound_to_agent: ChannelReadinessState,
    pub(crate) authenticated: ChannelReadinessState,
    pub(crate) listening: ChannelReadinessState,
    pub(crate) requirements: Vec<String>,
    pub(crate) notes: Vec<String>,
}

pub(crate) fn channel_readiness(
    config: &clawcrew_config::schema::Config,
    info: &clawcrew_config::schema::ChannelAliasInfo,
    health: &galleon_infra::health::HealthSnapshot,
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

pub(crate) fn channel_readiness_summary(
    readiness: &ChannelReadiness,
) -> (&'static str, &'static str) {
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
    health: &galleon_infra::health::HealthSnapshot,
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

fn component_health_ok_and_fresh(component: &galleon_infra::health::ComponentHealth) -> bool {
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
