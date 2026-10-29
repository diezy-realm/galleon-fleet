//! Admin routes and handlers for galleon-gateway (SPEC-012, TASK-061).
//!
//! Provides localhost-restricted administrative operations:
//! - POST `/admin/shutdown`: graceful shutdown
//! - POST `/admin/reload`: reload daemon supervisor / config
//! - GET `/admin/paircode`: fetch current pairing code
//! - POST `/admin/paircode/new`: mint new pairing code with optional rotation

use std::net::SocketAddr;

use axum::{
    Json,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::state::AppState;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AdminResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct AdminPaircodeQuery {
    #[serde(default)]
    pub rotate: Option<String>,
}

/// Reject requests that do not originate from a loopback address.
pub fn require_localhost(peer: &SocketAddr) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if peer.ip().is_loopback() {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Admin endpoints are restricted to localhost"
            })),
        ))
    }
}

/// POST /admin/shutdown — graceful shutdown from CLI (localhost only)
pub async fn handle_admin_shutdown(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_localhost(&peer)?;
    ::galleon_log::record!(
        INFO,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
        "admin shutdown request received; initiating graceful shutdown"
    );

    let body = AdminResponse {
        success: true,
        message: "Gateway shutdown initiated".to_string(),
    };

    let _ = state.shutdown_tx.send(true);

    Ok((StatusCode::OK, Json(body)))
}

/// POST /admin/reload — reload daemon supervisor or config
pub async fn handle_admin_reload(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let allow_remote = state.config.read().gateway.allow_remote_admin;
    let require_pairing = state.pairing.require_pairing();
    match crate::admin_reload_gate(peer.ip().is_loopback(), allow_remote, require_pairing) {
        crate::AdminReloadGate::Allow => {}
        crate::AdminReloadGate::RequireAuth => crate::api::require_auth(&state, &headers)?,
        crate::AdminReloadGate::Forbidden => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "Remote admin reload is disabled. Call from localhost, \
                              or set gateway.allow_remote_admin = true (with pairing \
                              enabled, then pair) to allow authenticated remote reloads."
                })),
            ));
        }
        crate::AdminReloadGate::ForbiddenNoPairing => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "Remote admin reload requires pairing. \
                              gateway.allow_remote_admin is enabled but \
                              gateway.require_pairing is off, so remote callers \
                              cannot be authenticated. Enable require_pairing, or \
                              call /admin/reload from localhost."
                })),
            ));
        }
    }

    let Some(reload_tx) = state.reload_tx.clone() else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "no daemon supervisor — running as standalone gateway. \
                          Restart the process to pick up config changes."
            })),
        ));
    };

    ::galleon_log::record!(
        INFO,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
        "admin reload request received"
    );
    state
        .pending_reload
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let shutdown_tx = state.shutdown_tx.clone();
    galleon_spawn::spawn!(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let _ = shutdown_tx.send(true);
        let _ = reload_tx.send(true);
    });

    Ok((
        StatusCode::OK,
        Json(AdminResponse {
            success: true,
            message: "Daemon reload initiated".to_string(),
        }),
    ))
}

/// GET /admin/paircode — fetch current pairing code (localhost only)
pub async fn handle_admin_paircode(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_localhost(&peer)?;
    let code = state.pairing.pairing_code();

    let body = if let Some(c) = code {
        serde_json::json!({
            "success": true,
            "pairing_required": state.pairing.require_pairing(),
            "pairing_code": c,
            "message": "Use this one-time code to pair"
        })
    } else {
        serde_json::json!({
            "success": true,
            "pairing_required": state.pairing.require_pairing(),
            "pairing_code": null,
            "message": if state.pairing.require_pairing() {
                "Pairing is active but no new code available (already paired or code expired)"
            } else {
                "Pairing is disabled for this gateway"
            }
        })
    };

    Ok((StatusCode::OK, Json(body)))
}

/// POST /admin/paircode/new — mint new pairing code (localhost only)
pub async fn handle_admin_paircode_new(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(params): Query<AdminPaircodeQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_localhost(&peer)?;

    if !state.pairing.require_pairing() {
        let body = serde_json::json!({
            "success": false,
            "pairing_required": false,
            "pairing_code": null,
            "message": "Pairing is disabled for this gateway"
        });
        return Ok((StatusCode::BAD_REQUEST, Json(body)));
    }

    let rotate = params
        .rotate
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let revocation_message = match rotate {
        Some("all") => {
            let revoked = state.pairing.revoke_all_tokens();
            if let Some(registry) = state.device_registry.as_ref() {
                if let Err(e) = registry.clear() {
                    let body = serde_json::json!({
                        "success": false,
                        "pairing_required": true,
                        "pairing_code": null,
                        "message": format!("Tokens revoked in memory but device registry clear failed: {e}"),
                    });
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(body)));
                }
            }
            if let Err(e) = Box::pin(crate::persist_pairing_tokens(
                state.config.clone(),
                &state.pairing,
                state.config_write_lock.clone(),
            ))
            .await
            {
                let body = serde_json::json!({
                    "success": false,
                    "pairing_required": true,
                    "pairing_code": null,
                    "message": format!("Tokens revoked in memory but config persist failed: {e}"),
                });
                return Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(body)));
            }
            ::galleon_log::record!(
                INFO,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_attrs(::serde_json::json!({"revoked": revoked})),
                "all paired tokens revoked via admin endpoint"
            );
            Some(format!(
                "Revoked all {revoked} paired token(s) and cleared the device registry."
            ))
        }
        Some(device_id) => {
            let Some(registry) = state.device_registry.as_ref() else {
                let body = serde_json::json!({
                    "success": false,
                    "pairing_required": true,
                    "pairing_code": null,
                    "message": "Device registry is disabled; cannot rotate a single device.",
                });
                return Ok((StatusCode::SERVICE_UNAVAILABLE, Json(body)));
            };
            let token_hash = match registry.revoke(device_id) {
                Ok(Some(hash)) => hash,
                Ok(None) => {
                    let body = serde_json::json!({
                        "success": false,
                        "pairing_required": true,
                        "pairing_code": null,
                        "message": format!("Device '{device_id}' not found; nothing revoked."),
                    });
                    return Ok((StatusCode::NOT_FOUND, Json(body)));
                }
                Err(e) => {
                    let body = serde_json::json!({
                        "success": false,
                        "pairing_required": true,
                        "pairing_code": null,
                        "message": format!("Device registry error: {e}"),
                    });
                    return Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(body)));
                }
            };
            state.pairing.revoke_token_hash(&token_hash);
            if let Err(e) = Box::pin(crate::persist_pairing_tokens(
                state.config.clone(),
                &state.pairing,
                state.config_write_lock.clone(),
            ))
            .await
            {
                let body = serde_json::json!({
                    "success": false,
                    "pairing_required": true,
                    "pairing_code": null,
                    "message": format!("Token revoked in memory but config persist failed: {e}"),
                });
                return Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(body)));
            }
            ::galleon_log::record!(
                INFO,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
                "single device token revoked via admin endpoint"
            );
            Some(format!(
                "Revoked the bearer token for device '{device_id}'."
            ))
        }
        None => None,
    };

    let code = state
        .pairing
        .generate_new_pairing_code(crate::live_pairing_code_policy(&state))
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "pairing_required": false,
                    "pairing_code": null,
                    "message": "Pairing is disabled for this gateway"
                })),
            )
        })?;

    if rotate.is_none() {
        ::galleon_log::record!(
            INFO,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
            "new pairing code generated via admin endpoint"
        );
    }

    let message = match revocation_message {
        Some(revoked) => {
            format!("{revoked} Use this one-time code to re-pair.")
        }
        None => "New pairing code generated — use this one-time code to pair".to_string(),
    };

    let body = serde_json::json!({
        "success": true,
        "pairing_required": true,
        "pairing_code": code,
        "message": message,
    });
    Ok((StatusCode::OK, Json(body)))
}
