//! Integration tests for Gateway layers and ConnectInfo (SPEC-012, TASK-059).
//!
//! Acceptance criteria:
//! - Body > 64 KB -> 413 Payload Too Large
//! - Header keamanan ada di respons /health
//! - ConnectInfo tersedia untuk handler
//! - TimeoutLayer dan SecurityHeaders diaplikasikan

#![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]
use galleon_config::schema::{
    AliasedAgentConfig, Config, GatewayTlsConfig, ModelProviderConfig, OllamaModelProviderConfig,
    RiskProfileConfig, RuntimeProfileConfig,
};
use galleon_gateway::{AppState, build_gateway_router};
use std::net::SocketAddr;
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

#[tokio::test]
async fn test_security_headers_present_on_health() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind listener");
    let local_addr = listener.local_addr().expect("local addr");
    let app = build_gateway_router(state);

    let _server = galleon_spawn::spawn!(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    let resp = client
        .get(format!("http://{local_addr}/health"))
        .send()
        .await
        .expect("GET /health");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    let headers = resp.headers();
    assert_eq!(
        headers
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        headers.get("x-frame-options").and_then(|v| v.to_str().ok()),
        Some("DENY")
    );
    assert_eq!(
        headers.get("referrer-policy").and_then(|v| v.to_str().ok()),
        Some("no-referrer")
    );
    assert!(headers.contains_key("content-security-policy"));
    assert_eq!(
        headers
            .get("cross-origin-opener-policy")
            .and_then(|v| v.to_str().ok()),
        Some("same-origin")
    );
    assert_eq!(
        headers
            .get("cross-origin-resource-policy")
            .and_then(|v| v.to_str().ok()),
        Some("same-origin")
    );
    assert_eq!(
        headers
            .get("x-permitted-cross-domain-policies")
            .and_then(|v| v.to_str().ok()),
        Some("none")
    );
    assert!(headers.contains_key("permissions-policy"));
    // Non-TLS should NOT have HSTS
    assert!(!headers.contains_key("strict-transport-security"));
}

#[tokio::test]
async fn test_security_headers_with_hsts_when_tls_enabled() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let mut config = create_test_config(&tmp);
    config.gateway.tls = Some(GatewayTlsConfig {
        enabled: true,
        cert_path: String::new(),
        key_path: String::new(),
        client_auth: None,
    });
    let state = AppState::from_config(config);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind listener");
    let local_addr = listener.local_addr().expect("local addr");
    let app = build_gateway_router(state);

    let _server = galleon_spawn::spawn!(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    let resp = client
        .get(format!("http://{local_addr}/health"))
        .send()
        .await
        .expect("GET /health");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    let headers = resp.headers();
    assert_eq!(
        headers
            .get("strict-transport-security")
            .and_then(|v| v.to_str().ok()),
        Some("max-age=63072000; includeSubDomains")
    );
}

#[tokio::test]
async fn test_body_limit_exceeded_returns_413() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind listener");
    let local_addr = listener.local_addr().expect("local addr");
    let app = build_gateway_router(state);

    let _server = galleon_spawn::spawn!(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    // MAX_BODY_SIZE is 64 KiB = 65_536 bytes.
    // Send a payload > 64 KiB (e.g. 65_537 bytes) to a core_router endpoint that reads the body (e.g. PATCH /api/config)
    let oversized_bytes = vec![b' '; 65_537];
    let resp = client
        .patch(format!("http://{local_addr}/api/config"))
        .header("content-type", "application/json")
        .body(oversized_bytes)
        .send()
        .await
        .expect("PATCH /api/config oversized");

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        "payload > 64 KB must return 413 Payload Too Large"
    );

    // Verify small payload within 64 KB limit does NOT return 413
    let small_payload = serde_json::to_vec(&serde_json::json!([])).expect("json");
    let resp_small = client
        .patch(format!("http://{local_addr}/api/config"))
        .header("content-type", "application/json")
        .body(small_payload)
        .send()
        .await
        .expect("PATCH /api/config small");

    assert_ne!(
        resp_small.status(),
        reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        "payload within 64 KB limit must not return 413"
    );
}

#[tokio::test]
async fn test_connect_info_available_for_handler() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind listener");
    let local_addr = listener.local_addr().expect("local addr");

    // Attach a route that extracts ConnectInfo<SocketAddr>
    let app = build_gateway_router(state).route(
        "/test/connect-info",
        axum::routing::get(
            |axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<SocketAddr>| async move {
                axum::Json(serde_json::json!({
                    "ip": peer.ip().to_string(),
                    "port": peer.port(),
                }))
            },
        ),
    );

    let _server = galleon_spawn::spawn!(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    let resp = client
        .get(format!("http://{local_addr}/test/connect-info"))
        .send()
        .await
        .expect("GET /test/connect-info");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let json: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(json["ip"], "127.0.0.1");
    assert!(json["port"].as_u64().unwrap_or(0) > 0);
}
