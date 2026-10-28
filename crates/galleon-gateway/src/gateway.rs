use anyhow::Result;
use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};

use crate::state::{AppState, GatewaySupervision};

/// Binds the `SystemGateway` gRPC server on `127.0.0.1:50052` and serves it
/// until the process exits. D1: lets the Go engine call back into Rust for
/// native-tool execution (bash/git/file I/O). Loopback-only — this is an
/// internal engine<->gateway channel, never exposed on a public interface.
pub async fn start_grpc_server() -> Result<()> {
    use crate::grpc_system_gateway::system_gateway_server::SystemGatewayServer;

    let addr: SocketAddr = "127.0.0.1:50052"
        .parse()
        .map_err(|e| anyhow::anyhow!("failed to parse loopback addr: {e}"))?;
    ::galleon_log::record!(
        INFO,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
        "ClawCrew SystemGateway gRPC server listening on {addr}"
    );

    tonic::transport::Server::builder()
        .add_service(SystemGatewayServer::new(
            crate::grpc_system_gateway::SystemGatewayService::new(None),
        ))
        .serve(addr)
        .await?;
    Ok(())
}

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

    let bind_addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    let actual_addr = listener.local_addr()?;

    supervision.report_ready(actual_addr);

    let mut shutdown_rx = state.shutdown_tx.subscribe();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            while let Ok(should_exit) = shutdown_rx.wait_for(|&x| x).await {
                if *should_exit {
                    break;
                }
            }
        })
        .await?;

    Ok(())
}

/// GET /health — public component health snapshot
pub async fn handle_health(State(state): State<AppState>) -> impl IntoResponse {
    let body = serde_json::json!({
        "status": "ok",
        "paired": state.pairing.is_paired(),
        "require_pairing": state.pairing.require_pairing(),
        "runtime": galleon_infra::health::snapshot(),
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
    Router::new()
        // ── Health and Ready ──
        .route("/health", get(handle_health))
        .route("/ready", get(handle_ready))
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
        // ── WebSocket agent chat ──
        .route("/ws", get(crate::ws::handle_ws_chat))
        .route("/ws/chat", get(crate::ws::handle_ws_chat))
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
        .with_state(state)
}
