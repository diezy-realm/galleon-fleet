//! Session REST API handlers.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Json,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde::Deserialize;

use super::require_auth;
use crate::{AppState, GW_SESSION_PREFIX, gateway_cancel_key, gateway_session_key};

#[derive(Deserialize)]
pub struct SessionMessagePostBody {
    pub content: String,
}

// ── Session API handlers ─────────────────────────────────────────

/// GET /api/sessions — list gateway sessions
pub async fn handle_api_sessions_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let Some(ref backend) = state.session_backend else {
        return Json(serde_json::json!({
            "sessions": [],
            "message": "Session persistence is disabled"
        }))
        .into_response();
    };

    // Include every session that's attributable (agent_alias stamped,
    // or a channel_id that resolves to an owning agent).
    // Pre-migration rows with neither set are skipped as orphans.
    let config = state.config.read().clone();
    let all_metadata = backend.list_sessions_with_metadata();
    let sessions: Vec<serde_json::Value> = all_metadata
        .into_iter()
        .filter(|meta| meta.agent_alias.is_some() || meta.channel_id.is_some())
        .map(|meta| {
            // Resolve owning agent: prefer the stamped alias, otherwise
            // reverse-look-up via channel_id (= `<type>.<alias>`) against
            // each agent's `channels` list.
            let agent_alias = meta.agent_alias.clone().or_else(|| {
                meta.channel_id
                    .as_deref()
                    .and_then(|c| config.agent_for_channel(c))
                    .map(str::to_string)
            });
            // Drop the gw_ prefix for display; channel keys stay as-is so
            // the frontend can show the channel context inline.
            let session_id = meta
                .key
                .strip_prefix("gw_")
                .map(str::to_string)
                .unwrap_or_else(|| meta.key.clone());
            let mut entry = serde_json::json!({
                // Display form: `gw_` stripped for gateway sessions, full
                // composite for channel-driven sessions.
                "session_id": session_id,
                // Full DB key for API operations (delete, messages, abort).
                "session_key": meta.key.clone(),
                "created_at": meta.created_at.to_rfc3339(),
                "last_activity": meta.last_activity.to_rfc3339(),
                "message_count": meta.message_count,
                "agent_alias": agent_alias,
                "channel_id": meta.channel_id,
            });
            if let Some(name) = meta.name {
                entry["name"] = serde_json::Value::String(name);
            }
            entry
        })
        .collect();

    Json(serde_json::json!({ "sessions": sessions })).into_response()
}

/// Resolve a path `{id}` to the persisted session key.
///
/// `GET /api/sessions` advertises both a display `session_id` (`gw_` stripped
/// for gateway sessions) and the full DB `session_key` for API operations.
/// Accept either form so callers that reuse `session_key` do not get a doubled
/// `gw_` prefix (`gw_gw_<id>`).
///
/// Resolution prefers **key existence** over punctuation heuristics (WebSocket
/// clients may pick display ids that contain `_`, e.g. `team_alpha` →
/// `gw_team_alpha`):
/// 1. exact `id` if it already exists as a session key
/// 2. raw `gw_{id}` if it already exists
/// 3. the canonical sanitized gateway key
/// 4. namespace fallback: keep a `gw_`-prefixed id as-is; otherwise use the
///    canonical sanitized gateway key
pub(crate) fn resolve_gateway_session_key(id: &str, exists: impl Fn(&str) -> bool) -> String {
    if exists(id) {
        return id.to_string();
    }
    if !id.starts_with("gw_") {
        let legacy_key = format!("{GW_SESSION_PREFIX}{id}");
        if exists(&legacy_key) {
            return legacy_key;
        }
        return gateway_session_key(id);
    }
    id.to_string()
}

/// Resolve an API session id to a live process-local cancellation key.
///
/// Cancellation registration deliberately keeps the accepted display id raw,
/// because sanitization would make `team.alpha` and `team_alpha` share a
/// token. A full persisted key is checked first so callers can pass the
/// `session_key` returned by `GET /api/sessions`; when the caller passes a
/// display id, the raw gateway prefix is added exactly once. There is no
/// sanitized fallback here: guessing from a lossy key could abort a different
/// live session.
pub(crate) fn resolve_gateway_cancel_key(
    id: &str,
    exists: impl Fn(&str) -> bool,
) -> Option<String> {
    if exists(id) {
        return Some(id.to_string());
    }

    let display_key = gateway_cancel_key(id);
    exists(&display_key).then_some(display_key)
}

/// Display session id used by WebSocket `?session_id=` / event filters.
pub(crate) fn gateway_display_session_id(session_key: &str) -> &str {
    session_key.strip_prefix("gw_").unwrap_or(session_key)
}

/// GET /api/sessions/{id}/messages — load persisted gateway WebSocket chat transcript
pub async fn handle_api_session_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let Some(ref backend) = state.session_backend else {
        return Json(serde_json::json!({
            "session_id": id,
            "messages": [],
            "session_persistence": false,
        }))
        .into_response();
    };

    let session_key = resolve_gateway_session_key(&id, |key| backend.session_exists(key));
    let msgs = backend.load_with_timestamps(&session_key);
    let messages: Vec<serde_json::Value> = msgs
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "role": m.message.role,
                "content": m.message.content,
                "created_at": m.created_at.map(|dt| dt.to_rfc3339()),
            })
        })
        .collect();

    Json(serde_json::json!({
        "session_id": id,
        "messages": messages,
        "session_persistence": true,
    }))
    .into_response()
}

/// POST /api/sessions/{id}/messages — push a visible notification into a gateway session
pub async fn handle_api_session_message_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<SessionMessagePostBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    if body.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "content is required"})),
        )
            .into_response();
    }

    let Some(ref backend) = state.session_backend else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Session persistence is disabled"})),
        )
            .into_response();
    };

    let session_key = resolve_gateway_session_key(&id, |key| backend.session_exists(key));
    if !backend.session_exists(&session_key) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response();
    }

    let _session_guard = match state.session_queue.acquire(&session_key).await {
        Ok(guard) => guard,
        Err(crate::session_queue::SessionQueueError::QueueFull { .. }) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({"error": "Session queue is full"})),
            )
                .into_response();
        }
        Err(crate::session_queue::SessionQueueError::Timeout { .. }) => {
            return (
                StatusCode::REQUEST_TIMEOUT,
                Json(serde_json::json!({"error": "Timed out waiting for session queue"})),
            )
                .into_response();
        }
    };

    let message = clawcrew_providers::ChatMessage::assistant(&body.content);
    if let Err(e) = backend.append(&session_key, &message) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to append session message: {e}")})),
        )
            .into_response();
    }

    // Match WS `?session_id=` / `event_matches_session` (display id), not the
    // path string — callers that pass the full `session_key` must still notify
    // the connected WebSocket.
    let display_session_id = gateway_display_session_id(&session_key);
    let event = serde_json::json!({
        "type": "message",
        "session_id": display_session_id,
        "role": "assistant",
        "content": body.content.clone(),
        "source": "api",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    let _ = state.event_tx.send(event);

    Json(serde_json::json!({
        "status": "ok",
        "session_id": display_session_id,
        "message": {
            "role": "assistant",
            "content": message.content,
        },
        "session_persistence": true,
    }))
    .into_response()
}

/// DELETE /api/sessions/{id} — delete a gateway session
pub async fn handle_api_session_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let Some(ref backend) = state.session_backend else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session persistence is disabled"})),
        )
            .into_response();
    };

    let session_key = resolve_gateway_session_key(&id, |key| backend.session_exists(key));

    let cancellation = {
        let mut tokens = state
            .cancel_tokens
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        resolve_gateway_cancel_key(&id, |key| tokens.contains_key(key))
            .and_then(|cancel_key| tokens.remove(&cancel_key).map(|token| (cancel_key, token)))
    };
    if let Some((cancel_key, token)) = cancellation {
        token.cancel();
        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
                ::serde_json::json!({
                    "session_key": session_key,
                    "cancel_key": cancel_key,
                })
            ),
            "cancelled in-flight turn for deleted session"
        );
    }

    match backend.delete_session(&session_key) {
        Ok(true) => Json(serde_json::json!({"deleted": true, "session_id": id})).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to delete session: {e}")})),
        )
            .into_response(),
    }
}

/// PUT /api/sessions/{id} — rename a gateway session
pub async fn handle_api_session_rename(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let Some(ref backend) = state.session_backend else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session persistence is disabled"})),
        )
            .into_response();
    };

    let name = body["name"].as_str().unwrap_or("").trim();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "name is required"})),
        )
            .into_response();
    }

    let session_key = resolve_gateway_session_key(&id, |key| backend.session_exists(key));

    // Verify the session exists before renaming
    if !backend.session_exists(&session_key) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response();
    }

    match backend.set_session_name(&session_key, name) {
        Ok(()) => Json(serde_json::json!({"session_id": id, "name": name})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to rename session: {e}")})),
        )
            .into_response(),
    }
}

/// GET /api/sessions/running — list sessions currently in "running" state
pub async fn handle_api_sessions_running(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let Some(ref backend) = state.session_backend else {
        return Json(serde_json::json!({
            "sessions": [],
            "message": "Session persistence is disabled"
        }))
        .into_response();
    };

    let running = backend.list_running_sessions();
    let sessions: Vec<serde_json::Value> = running
        .into_iter()
        .filter_map(|meta| {
            let session_id = meta.key.strip_prefix("gw_")?;
            // Session metadata projection (P1.5): observable provider fallbacks.
            let session_meta = clawcrew_infra::session_metadata::session_metadata(&meta.key);
            Some(serde_json::json!({
                "session_id": session_id,
                "created_at": meta.created_at.to_rfc3339(),
                "last_activity": meta.last_activity.to_rfc3339(),
                "message_count": meta.message_count,
                "provider_fallbacks": session_meta.provider_fallbacks.len(),
                "last_fallback": session_meta.provider_fallbacks.last(),
                "compactions": session_meta.compactions,
            }))
        })
        .collect();

    Json(serde_json::json!({ "sessions": sessions })).into_response()
}

/// GET /api/sessions/stream — authenticated bounded session-activity stream.
///
/// Emits a `sessions` event only when the running-session snapshot changes
/// (bounded, no duplicate frames), so the dashboard tracks live session
/// activity without polling.
pub async fn handle_api_sessions_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let backend = state.session_backend.clone();
    let stream =
        futures_util::stream::unfold((backend, String::new()), |(backend, last)| async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let Some(backend) = backend else {
                let event = Event::default().event("disabled").data("{}");
                return Some((Ok::<Event, Infallible>(event), (None, last)));
            };
            let sessions: Vec<serde_json::Value> = backend
                .list_running_sessions()
                .into_iter()
                .filter_map(|meta| {
                    let session_id = meta.key.strip_prefix(GW_SESSION_PREFIX)?;
                    let session_meta =
                        clawcrew_infra::session_metadata::session_metadata(&meta.key);
                    Some(serde_json::json!({
                        "session_id": session_id,
                        "last_activity": meta.last_activity.to_rfc3339(),
                        "message_count": meta.message_count,
                        "provider_fallbacks": session_meta.provider_fallbacks.len(),
                        "compactions": session_meta.compactions,
                    }))
                })
                .collect();
            let payload = serde_json::json!({ "sessions": sessions }).to_string();
            if payload == last {
                let comment = Event::default().comment("no session change");
                return Some((Ok::<Event, Infallible>(comment), (Some(backend), last)));
            }
            let event = Event::default().event("sessions").data(payload.clone());
            Some((Ok::<Event, Infallible>(event), (Some(backend), payload)))
        });
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// GET /api/sessions/{id}/state — get session state
pub async fn handle_api_session_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let Some(ref backend) = state.session_backend else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session persistence is disabled"})),
        )
            .into_response();
    };

    let session_key = resolve_gateway_session_key(&id, |key| backend.session_exists(key));
    match backend.get_session_state(&session_key) {
        Ok(Some(ss)) => {
            let mut resp = serde_json::json!({
                "session_id": id,
                "state": ss.state,
            });
            if let Some(turn_id) = ss.turn_id {
                resp["turn_id"] = serde_json::Value::String(turn_id);
            }
            if let Some(started) = ss.turn_started_at {
                resp["turn_started_at"] = serde_json::Value::String(started.to_rfc3339());
            }
            Json(resp).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to get session state: {e}")})),
        )
            .into_response(),
    }
}

// ── Session abort endpoint ────────────────────────────────────────

pub async fn handle_api_session_abort(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // Resolve + look up under one lock so display ids and full session keys
    // both address the raw, collision-safe cancellation key.
    let (cancel_key, token) = {
        let tokens = state
            .cancel_tokens
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cancel_key = resolve_gateway_cancel_key(&id, |key| tokens.contains_key(key));
        let token = cancel_key
            .as_deref()
            .and_then(|key| tokens.get(key).cloned());
        (cancel_key, token)
    };

    if let (Some(cancel_key), Some(token)) = (cancel_key, token) {
        token.cancel();
        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                .with_attrs(::serde_json::json!({"cancel_key": cancel_key})),
            "session abort requested"
        );
        Json(serde_json::json!({ "status": "aborted" })).into_response()
    } else {
        Json(serde_json::json!({ "status": "no_active_response" })).into_response()
    }
}

// ── Claude Code hook endpoint ────────────────────────────────────
