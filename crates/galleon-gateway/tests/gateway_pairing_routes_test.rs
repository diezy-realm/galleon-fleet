//! Integration tests for Gateway pairing routes (SPEC-012, TASK-060).
//!
//! Acceptance criteria:
//! - ACP bridge path /pair dan /pair/code merespons sesuai OLD
//! - Rute /api/devices* butuh Bearer (401 tanpa token)
//! - Handler api_pairing.rs (POST /api/pairing/initiate, POST /api/pair,
//!   GET /api/devices, PATCH /api/devices/me/capabilities, DELETE /api/devices/{id},
//!   POST /api/devices/{id}/token/rotate) terpasang dan berfungsi.

#![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]
use axum::http::StatusCode;
use galleon_config::pairing::PairingGuard;
use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::api_pairing::DeviceRegistry;
use galleon_gateway::{AppState, build_gateway_router, live_pairing_code_policy};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

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

#[tokio::test]
async fn test_acp_bridge_legacy_pairing_flow() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let mut state = AppState::from_config(config);
    state.pairing = Arc::new(PairingGuard::new(
        true,
        &[],
        live_pairing_code_policy(&state),
    ));

    // Initialize DeviceRegistry backed by test workspace
    let registry = DeviceRegistry::try_new(tmp.path()).expect("create registry");
    state.device_registry = Some(Arc::new(registry));

    let (addr, _server) = spawn_test_server(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    // 1. GET /pair/code before pairing returns the initial setup code
    let resp = client
        .get(format!("http://{addr}/pair/code"))
        .send()
        .await
        .expect("GET /pair/code");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["success"], true);
    assert_eq!(body["pairing_required"], true);
    let pairing_code = body["pairing_code"]
        .as_str()
        .expect("pairing code string")
        .to_string();
    assert!(!pairing_code.is_empty());

    // 2. POST /pair with wrong or missing code returns 403 Forbidden
    let resp = client
        .post(format!("http://{addr}/pair"))
        .header("X-Pairing-Code", "invalid-code")
        .send()
        .await
        .expect("POST /pair invalid");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["error"], "Invalid pairing code");

    // 3. POST /pair with correct X-Pairing-Code header returns 200 and bearer token
    let resp = client
        .post(format!("http://{addr}/pair"))
        .header("X-Pairing-Code", &pairing_code)
        .send()
        .await
        .expect("POST /pair valid");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["paired"], true);
    assert_eq!(body["persisted"], true);
    let token = body["token"].as_str().expect("bearer token").to_string();
    assert!(!token.is_empty());

    // 4. GET /pair/code after first pairing returns pairing_code: null (OLD behavior)
    let resp = client
        .get(format!("http://{addr}/pair/code"))
        .send()
        .await
        .expect("GET /pair/code after paired");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["success"], true);
    assert_eq!(body["pairing_required"], true);
    assert_eq!(body["pairing_code"], Value::Null);
}

#[tokio::test]
async fn test_api_devices_routes_require_bearer_401() {
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

    let (addr, _server) = spawn_test_server(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    // All /api/devices* endpoints and /api/pairing/initiate must reject requests without Bearer token
    let endpoints = vec![
        ("POST", format!("http://{addr}/api/pairing/initiate"), None),
        ("GET", format!("http://{addr}/api/devices"), None),
        (
            "PATCH",
            format!("http://{addr}/api/devices/me/capabilities"),
            Some(serde_json::json!({"capabilities": ["microphone"]})),
        ),
        (
            "POST",
            format!("http://{addr}/api/devices/me/capabilities"),
            Some(serde_json::json!({"capabilities": ["microphone"]})),
        ),
        ("DELETE", format!("http://{addr}/api/devices/dev-123"), None),
        (
            "POST",
            format!("http://{addr}/api/devices/dev-123/token/rotate"),
            None,
        ),
    ];

    for (method, url, json_body) in endpoints {
        let mut req = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => unreachable!(),
        };
        if let Some(body) = json_body {
            req = req.json(&body);
        }
        let resp = req
            .send()
            .await
            .unwrap_or_else(|e| panic!("failed {method} {url}: {e}"));
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Endpoint {method} {url} without Bearer must return 401 Unauthorized"
        );
    }
}

#[tokio::test]
async fn test_api_devices_and_pairing_handlers_with_valid_bearer() {
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

    let (addr, _server) = spawn_test_server(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    // Get code and pair first
    let resp = client
        .get(format!("http://{addr}/pair/code"))
        .send()
        .await
        .expect("GET /pair/code");
    let body: Value = resp.json().await.expect("json body");
    let code = body["pairing_code"].as_str().expect("code");

    let resp = client
        .post(format!("http://{addr}/pair"))
        .header("X-Pairing-Code", code)
        .send()
        .await
        .expect("POST /pair");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    let token = body["token"].as_str().expect("token").to_string();

    // 1. GET /api/devices with Bearer token -> 200 OK and returns device list
    let resp = client
        .get(format!("http://{addr}/api/devices"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("GET /api/devices");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("devices json");
    let devices = body["devices"].as_array().expect("devices array");
    assert_eq!(devices.len(), 1);
    let device_id = devices[0]["id"].as_str().expect("device id").to_string();

    // 2. PATCH /api/devices/me/capabilities with Bearer token -> 200 OK
    let resp = client
        .patch(format!("http://{addr}/api/devices/me/capabilities"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "capabilities": ["microphone", "screen_recording"]
        }))
        .send()
        .await
        .expect("PATCH /api/devices/me/capabilities");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(
        body["capabilities"],
        serde_json::json!(["microphone", "screen_recording"])
    );

    // 3. POST /api/pairing/initiate with Bearer token -> 200 OK, generates new code
    let resp = client
        .post(format!("http://{addr}/api/pairing/initiate"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("POST /api/pairing/initiate");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    let new_pairing_code = body["pairing_code"].as_str().expect("new code").to_string();
    assert!(!new_pairing_code.is_empty());

    // 4. POST /api/pair is public (accepts code without bearer)
    // 4a. Wrong code returns 400 Bad Request ("Invalid or expired pairing code")
    let resp = client
        .post(format!("http://{addr}/api/pair"))
        .json(&serde_json::json!({
            "code": "wrong-code"
        }))
        .send()
        .await
        .expect("POST /api/pair invalid");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 4b. Valid code generated by initiate_pairing pairs successfully
    let resp = client
        .post(format!("http://{addr}/api/pair"))
        .json(&serde_json::json!({
            "code": new_pairing_code,
            "device_name": "secondary-device",
            "device_type": "mobile"
        }))
        .send()
        .await
        .expect("POST /api/pair valid");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("pair response");
    assert_eq!(body["paired"], true);
    assert_eq!(body["persisted"], true);
    let second_token = body["token"].as_str().expect("second token");
    assert!(!second_token.is_empty());

    // 5. POST /api/devices/{id}/token/rotate with Bearer -> 200 OK (revokes old token)
    let resp = client
        .post(format!(
            "http://{addr}/api/devices/{device_id}/token/rotate"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("POST rotate token");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["device_id"], device_id);

    // After rotation, old token is revoked (returns 401)
    let resp = client
        .get(format!("http://{addr}/api/devices"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("GET /api/devices with revoked token");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // List devices using second_token: only secondary-device remains
    let resp = client
        .get(format!("http://{addr}/api/devices"))
        .header("Authorization", format!("Bearer {second_token}"))
        .send()
        .await
        .expect("GET /api/devices with second_token");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("devices json");
    let devices = body["devices"].as_array().expect("devices array");
    assert_eq!(devices.len(), 1);
    let second_device_id = devices[0]["id"].as_str().expect("device id").to_string();

    // 6. DELETE /api/devices/{id} with second_token -> 200 OK
    let resp = client
        .delete(format!("http://{addr}/api/devices/{second_device_id}"))
        .header("Authorization", format!("Bearer {second_token}"))
        .send()
        .await
        .expect("DELETE device");
    assert_eq!(resp.status(), StatusCode::OK);
}
