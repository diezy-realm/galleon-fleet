use anyhow::Result;
use std::net::SocketAddr;
use std::time::Duration;

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::state::{AppState, GatewaySupervision};

/// Run the HTTP gateway using axum with proper HTTP/1.1 compliance.
pub async fn run_gateway(
    host: &str,
    port: u16,
    config: crate::Config,
    external_event_tx: Option<tokio::sync::broadcast::Sender<serde_json::Value>>,
    reload_controls: Option<galleon_infra::gateway_controls::GatewayReloadControls>,
    tui_registry: Option<std::sync::Arc<dyn galleon_api::runtime_traits::TuiConnectionRegistry>>,
    canvas_store: Option<crate::CanvasStore>,
    sop_runner: Option<std::sync::Arc<dyn galleon_api::sop::SopRunner>>,
    readiness: Option<galleon_infra::gateway_controls::GatewayReadinessReporter>,
) -> Result<()> {
    Box::pin(run_gateway_with_plugin_webhooks(
        host,
        port,
        config,
        external_event_tx,
        reload_controls,
        tui_registry,
        canvas_store,
        sop_runner,
        GatewaySupervision::new(
            readiness,
            std::sync::Arc::new(galleon_api::webhook::PluginWebhookRegistry::new()),
        ),
    ))
    .await
}

/// Run the supervised gateway with the daemon generation's channel-plugin
/// webhook registry.
#[allow(clippy::too_many_lines)]
pub async fn run_gateway_with_plugin_webhooks(
    host: &str,
    port: u16,
    config: crate::Config,
    external_event_tx: Option<tokio::sync::broadcast::Sender<serde_json::Value>>,
    reload_controls: Option<galleon_infra::gateway_controls::GatewayReloadControls>,
    tui_registry: Option<std::sync::Arc<dyn galleon_api::runtime_traits::TuiConnectionRegistry>>,
    canvas_store: Option<crate::CanvasStore>,
    sop_runner: Option<std::sync::Arc<dyn galleon_api::sop::SopRunner>>,
    supervision: GatewaySupervision,
) -> Result<()> {
    let state = AppState::from_config_and_controls(
        config,
        external_event_tx,
        reload_controls,
        tui_registry,
        canvas_store,
        sop_runner,
    );
    let app = build_gateway_router(state.clone());

    let _tls_acceptor = match &state.config.read().gateway.tls {
        Some(tls_cfg) if tls_cfg.enabled => match crate::tls::build_tls_acceptor(tls_cfg) {
            Ok(acceptor) => Some(acceptor),
            Err(e) => {
                return Err(anyhow::Error::msg(format!("Gateway TLS setup failed: {e}")));
            }
        },
        _ => None,
    };

    let bind_addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    let actual_addr = listener.local_addr()?;

    // ── Tunnel ────────────────────────────────────────────────
    let tunnel_cfg = state.config.read().tunnel.clone();
    let tunnel = match galleon_runtime::tunnel::create_tunnel(&tunnel_cfg) {
        Ok(t) => t,
        Err(e) => {
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_outcome(::galleon_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({
                        "tunnel_provider": tunnel_cfg.tunnel_provider,
                        "error": format!("{e}"),
                    })),
                "Gateway: tunnel adapter failed to construct; booting without a \
                 tunnel. Fix [tunnel] and POST /admin/reload."
            );
            None
        }
    };
    let mut _tunnel_url: Option<String> = None;

    if let Some(ref tun) = tunnel {
        ::galleon_log::record!(
            INFO,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note).with_attrs(
                ::serde_json::json!({
                    "tunnel_provider": tun.name(),
                })
            ),
            &format!("Starting {} tunnel...", tun.name())
        );
        match tun.start(host, actual_addr.port()).await {
            Ok(url) => {
                ::galleon_log::record!(
                    INFO,
                    ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                        .with_attrs(::serde_json::json!({
                            "tunnel_provider": tun.name(),
                            "url": &url,
                        })),
                    &format!("Tunnel active: {url}")
                );
                _tunnel_url = Some(url);
            }
            Err(e) => {
                ::galleon_log::record!(
                    WARN,
                    ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                        .with_outcome(::galleon_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({
                            "tunnel_provider": tun.name(),
                            "error": format!("{e}"),
                        })),
                    &format!("Tunnel failed to start: {e}; falling back to local-only mode.")
                );
            }
        }
    }

    supervision.report_ready(actual_addr);

    let mut shutdown_rx = state.shutdown_tx.subscribe();
    let serve_res = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        while let Ok(should_exit) = shutdown_rx.wait_for(|&x| x).await {
            if *should_exit {
                break;
            }
        }
    })
    .await;

    if let Some(ref tun) = tunnel {
        if let Err(e) = tun.stop().await {
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_outcome(::galleon_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({
                        "tunnel_provider": tun.name(),
                        "error": format!("{e}"),
                    })),
                &format!("Tunnel stop failed: {e}")
            );
        }
    }

    serve_res?;

    Ok(())
}

fn public_health_snapshot() -> serde_json::Value {
    let snapshot = galleon_infra::health::snapshot();
    let components = snapshot
        .components
        .into_iter()
        .map(|(name, component)| {
            (
                name,
                serde_json::json!({
                    "status": component.status,
                    "updated_at": component.updated_at,
                    "last_ok": component.last_ok,
                    "restart_count": component.restart_count,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    serde_json::json!({
        "pid": snapshot.pid,
        "updated_at": snapshot.updated_at,
        "uptime_seconds": snapshot.uptime_seconds,
        "components": components,
    })
}

/// GET /health — public component health snapshot
pub async fn handle_health(State(state): State<AppState>) -> impl IntoResponse {
    let body = serde_json::json!({
        "status": "ok",
        "paired": state.pairing.is_paired(),
        "require_pairing": state.pairing.require_pairing(),
        "runtime": public_health_snapshot(),
    });
    (axum::http::StatusCode::OK, Json(body))
}

/// GET /ready — lightweight server readiness probe
pub async fn handle_ready() -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "status": "ready"
        })),
    )
}

/// Construct the Axum Router with all core endpoints: REST, WebSocket,
/// health, readiness, and config sections.
pub fn build_gateway_router(state: AppState) -> Router {
    let core_router = Router::new()
        // ── Health, Ready, and Legacy Pairing ──
        .route("/health", get(handle_health))
        .route("/ready", get(handle_ready))
        .route("/pair", post(crate::api_pairing::handle_pair))
        .route("/pair/code", get(crate::api_pairing::handle_pair_code))
        // ── Section routes (/api/sections/*) ──
        .route("/api/sections", get(crate::api_sections::handle_sections))
        .route(
            "/api/sections/status",
            get(crate::api_sections::handle_section_status),
        )
        .route(
            "/api/sections/agent-options",
            get(crate::api_sections::handle_agent_options),
        )
        .route(
            "/api/sections/catalog",
            get(crate::api_sections::handle_catalog),
        )
        .route(
            "/api/sections/catalog/models",
            get(crate::api_sections::handle_catalog_models),
        )
        .route(
            "/api/sections/{section}",
            get(crate::api_sections::handle_section_picker),
        )
        .route(
            "/api/sections/{section}/items/{key}",
            post(crate::api_sections::handle_section_select),
        )
        // ── Dashboard compatibility alias (/api/config/sections/*) ──
        .route(
            "/api/config/sections",
            get(crate::api_sections::handle_sections),
        )
        .route(
            "/api/config/sections/{section}",
            get(crate::api_sections::handle_section_picker),
        )
        .route(
            "/api/config/sections/{section}/items/{key}",
            post(crate::api_sections::handle_section_select),
        )
        .route(
            "/api/config/catalog",
            get(crate::api_sections::handle_catalog),
        )
        .route(
            "/api/config/catalog/models",
            get(crate::api_sections::handle_catalog_models),
        )
        .route(
            "/api/config/status",
            get(crate::api_sections::handle_section_status),
        )
        .route(
            "/api/config/agent-options",
            get(crate::api_sections::handle_agent_options),
        )
        // ── WebSocket agent chat & ACP ──
        .route("/ws", get(crate::ws::handle_ws_chat))
        .route("/ws/chat", get(crate::ws::handle_ws_chat))
        .route("/acp", get(crate::acp::handle_ws_acp))
        // ── SSE events ──
        .route("/api/events", get(crate::sse::handle_sse_events))
        // ── Core REST endpoints ──
        .route("/api/status", get(crate::api::handle_api_status))
        .route("/api/health", get(crate::api::handle_api_health))
        .route(
            "/api/config",
            get(crate::api_config::handle_config_get)
                .patch(crate::api_config::handle_patch)
                .options(crate::api_config::handle_options_config),
        )
        .route(
            "/api/config/drift",
            get(crate::api_config::handle_drift),
        )
        .route("/api/logs", get(crate::api_logs::handle_api_logs))
        // ── Pairing + Device management API ──
        .route(
            "/api/pairing/initiate",
            post(crate::api_pairing::initiate_pairing),
        )
        .route(
            "/api/pair",
            post(crate::api_pairing::submit_pairing_enhanced),
        )
        .route("/api/devices", get(crate::api_pairing::list_devices))
        .route(
            "/api/devices/me/capabilities",
            patch(crate::api_pairing::update_my_capabilities)
                .post(crate::api_pairing::update_my_capabilities),
        )
        .route(
            "/api/devices/{id}",
            delete(crate::api_pairing::revoke_device),
        )
        .route(
            "/api/devices/{id}/token/rotate",
            post(crate::api_pairing::rotate_token),
        )
        .route(
            "/api/auth/ratelimit",
            get(|| async {
                Json(serde_json::json!({
                    "status": "active",
                    "max_attempts": crate::auth_rate_limit::MAX_ATTEMPTS,
                    "window_secs": crate::auth_rate_limit::WINDOW_SECS,
                    "lockout_secs": crate::auth_rate_limit::LOCKOUT_SECS,
                }))
            }),
        )
        // ── Admin routes (localhost-restricted) ──
        .route(
            "/admin/shutdown",
            post(crate::api_admin::handle_admin_shutdown),
        )
        .route(
            "/admin/reload",
            post(crate::api_admin::handle_admin_reload),
        )
        .route(
            "/admin/paircode",
            get(crate::api_admin::handle_admin_paircode),
        )
        .route(
            "/admin/paircode/new",
            post(crate::api_admin::handle_admin_paircode_new),
        )
        .route(
            "/admin/sop/pending",
            get(crate::api_sop::handle_sop_pending),
        )
        .route(
            "/admin/sop/approve",
            post(crate::api_sop::handle_sop_approve),
        )
        .route(
            "/admin/sop/deny",
            post(crate::api_sop::handle_sop_deny),
        )
        // ── Channel routes and SOP webhook ──
        .merge(crate::optional_channel_routes())
        .merge(crate::sop_webhook_routes())
        .with_state(state.clone())
        .layer(RequestBodyLimitLayer::new(crate::MAX_BODY_SIZE));

    // Router fleet (TASK-081) with its own FLEET_MAX_BODY (1 MiB) limit under require_auth
    let fleet_router = crate::api_fleet::routes(state.clone())
        .layer(RequestBodyLimitLayer::new(
            galleon_fleet_core::FLEET_MAX_BODY,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_fleet_auth,
        ))
        .with_state(state.clone());

    let app = core_router.merge(fleet_router);

    let app = app.layer(TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(crate::REQUEST_TIMEOUT_SECS),
    ));

    // Nest under path prefix when configured (axum strips prefix before routing).
    // nest() at "/prefix" handles both "/prefix" and "/prefix/*" but not "/prefix/"
    // with a trailing slash, so we add a fallback redirect for that case.
    let path_prefix = {
        let cfg = state.config.read();
        cfg.gateway
            .path_prefix
            .as_deref()
            .filter(|p| !p.is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                if !state.path_prefix.is_empty() {
                    Some(state.path_prefix.clone())
                } else {
                    None
                }
            })
    };

    let app = if let Some(prefix) = path_prefix.as_deref() {
        let redirect_target: std::sync::Arc<str> = prefix.into();
        Router::new().nest(prefix, app).route(
            &format!("{prefix}/"),
            get({
                let redirect_target = std::sync::Arc::clone(&redirect_target);
                move || {
                    let redirect_target = std::sync::Arc::clone(&redirect_target);
                    async move { axum::response::Redirect::permanent(&redirect_target) }
                }
            }),
        )
    } else {
        app
    };

    let tls_enabled = state
        .config
        .read()
        .gateway
        .tls
        .as_ref()
        .is_some_and(|tls_cfg| tls_cfg.enabled);

    if tls_enabled {
        app.layer(axum::middleware::from_fn(
            crate::security_headers::apply_with_hsts,
        ))
    } else {
        app.layer(axum::middleware::from_fn(crate::security_headers::apply))
    }
}

/// Enforces `require_auth` Bearer token authentication across all fleet endpoints.
async fn require_fleet_auth(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let Err((status, body)) = crate::api::require_auth(&state, req.headers()) {
        return (status, body).into_response();
    }
    next.run(req).await
}
