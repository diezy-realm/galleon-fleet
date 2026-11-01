//! Integration tests for Gateway admin routes (SPEC-012, TASK-061).
//!
//! Acceptance criteria:
//! - Request admin dari non-loopback -> 403 Forbidden
//! - Endpoint /admin/shutdown, /admin/reload, /admin/paircode, /admin/paircode/new bekerja
//! - Endpoint /admin/sop/pending, /admin/sop/approve, /admin/sop/deny bekerja sesuai CLI

#![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode};
use galleon_config::pairing::PairingGuard;
use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::api_pairing::DeviceRegistry;
use galleon_gateway::{AppState, build_gateway_router, live_pairing_code_policy};
use serde_json::Value;
use tower::ServiceExt;

fn create_test_config(tmp: &tempfile::TempDir) -> Config {
    let data_dir = tmp.path().join("data");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&data_dir).expect("create test data directory");
    std::fs::create_dir_all(&workspace).expect("create test workspace directory");

    let mut config = Config {
        data_dir,
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };

    config.providers.models.ollama.insert(
        "default".to_string(),
        OllamaModelProviderConfig {
            base: ModelProviderConfig {
                model: Some("llama3.2".to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
    );

    config
        .risk_profiles
        .insert("default".to_string(), RiskProfileConfig::default());

    config
        .runtime_profiles
        .insert("default".to_string(), RuntimeProfileConfig::default());

    let mut agent = AliasedAgentConfig {
        model_provider: "ollama.default".into(),
        risk_profile: "default".into(),
        runtime_profile: "default".into(),
        workspace: galleon_config::multi_agent::AgentWorkspaceConfig {
            path: Some(workspace),
            ..Default::default()
        },
        ..Default::default()
    };
    agent.enabled = true;
    config.agents.insert("default".to_string(), agent);

    config
}

async fn spawn_test_server(state: AppState) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind listener");
    let local_addr = listener.local_addr().expect("local addr");
    let app = build_gateway_router(state);

    let server = galleon_spawn::spawn!(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve");
    });

    (local_addr, server)
}

fn remote_peer() -> SocketAddr {
    SocketAddr::from(([198, 51, 100, 1], 54321))
}

#[tokio::test]
async fn test_admin_routes_reject_non_loopback_with_403() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let mut state = AppState::from_config(config);
    state.pairing = Arc::new(PairingGuard::new(
        true,
        &[],
        live_pairing_code_policy(&state),
    ));

    let remote = remote_peer();
    let endpoints = [
        ("POST", "/admin/shutdown", Body::empty()),
        ("POST", "/admin/reload", Body::empty()),
        ("GET", "/admin/paircode", Body::empty()),
        ("POST", "/admin/paircode/new", Body::empty()),
        ("GET", "/admin/sop/pending", Body::empty()),
        (
            "POST",
            "/admin/sop/approve",
            Body::from(serde_json::to_vec(&serde_json::json!({"run_id": "r1"})).unwrap()),
        ),
        (
            "POST",
            "/admin/sop/deny",
            Body::from(
                serde_json::to_vec(&serde_json::json!({"run_id": "r1", "reason": "denied"}))
                    .unwrap(),
            ),
        ),
    ];

    for (method, path, body) in endpoints {
        let app = build_gateway_router(state.clone());
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(body)
            .expect("request builder");

        req.extensions_mut().insert(MockConnectInfo(remote));

        let resp = app.oneshot(req).await.expect("oneshot response");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Non-loopback call to {method} {path} must return 403 Forbidden"
        );
    }
}

#[tokio::test]
async fn test_admin_paircode_and_shutdown_loopback() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let mut state = AppState::from_config(config);
    state.pairing = Arc::new(PairingGuard::new(
        true,
        &[],
        live_pairing_code_policy(&state),
    ));
    let registry = DeviceRegistry::try_new(tmp.path()).expect("create registry");
    state.device_registry = Some(Arc::new(registry));

    let shutdown_rx = state.shutdown_tx.subscribe();
    let (addr, _server) = spawn_test_server(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    // 1. GET /admin/paircode from localhost succeeds
    let resp = client
        .get(format!("http://{addr}/admin/paircode"))
        .send()
        .await
        .expect("GET /admin/paircode");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["success"], true);
    assert!(body["pairing_code"].is_string());

    // 2. POST /admin/paircode/new generates new code
    let resp = client
        .post(format!("http://{addr}/admin/paircode/new"))
        .send()
        .await
        .expect("POST /admin/paircode/new");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["success"], true);
    assert!(body["pairing_code"].is_string());

    // 3. POST /admin/shutdown initiates graceful shutdown
    assert!(!*shutdown_rx.borrow(), "shutdown should not be sent yet");
    let resp = client
        .post(format!("http://{addr}/admin/shutdown"))
        .send()
        .await
        .expect("POST /admin/shutdown");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Gateway shutdown initiated");

    // Verify shutdown channel received signal
    assert!(
        *shutdown_rx.borrow(),
        "shutdown_tx should have received true signal"
    );
}

#[tokio::test]
async fn test_admin_reload_loopback() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let mut state = AppState::from_config(config);
    state.pairing = Arc::new(PairingGuard::new(
        true,
        &[],
        live_pairing_code_policy(&state),
    ));

    let (reload_tx, reload_rx) = tokio::sync::watch::channel(false);
    state.reload_tx = Some(reload_tx);

    let (addr, _server) = spawn_test_server(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    let resp = client
        .post(format!("http://{addr}/admin/reload"))
        .send()
        .await
        .expect("POST /admin/reload");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Daemon reload initiated");

    // Wait briefly for the reload background task to fire (200ms delay in handler)
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert!(
        *reload_rx.borrow(),
        "reload_tx should have received true signal"
    );
}

#[tokio::test]
async fn test_admin_sop_pending_loopback() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    let (addr, _server) = spawn_test_server(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    // Without sop_runner, /admin/sop/pending is 503 Service Unavailable (sop_disabled)
    let resp = client
        .get(format!("http://{addr}/admin/sop/pending"))
        .send()
        .await
        .expect("GET /admin/sop/pending");
    assert_eq!(resp.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["error"], "SOP subsystem not enabled");
}
