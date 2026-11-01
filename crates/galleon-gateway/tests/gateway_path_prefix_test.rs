//! Integration tests for Gateway `path_prefix` configuration (SPEC-017, TASK-103).
//!
//! Acceptance criteria:
//! - path_prefix = '/galleon' -> GET /galleon/health 200
//! - path_prefix = '/galleon' -> GET /health 404
//! - path_prefix = '/galleon' -> GET /galleon/ 308 (redirects to /galleon)

#![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]
use std::net::SocketAddr;
use std::time::Duration;

use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::{build_gateway_router, AppState};

fn create_test_config(tmp: &tempfile::TempDir, path_prefix: Option<String>) -> Config {
    let data_dir = tmp.path().join("data");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&data_dir).expect("create test data directory");
    std::fs::create_dir_all(&workspace).expect("create test workspace directory");

    let mut config = Config {
        data_dir,
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };

    config.gateway.path_prefix = path_prefix;

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
async fn test_path_prefix_galleon_routes_and_redirect() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp, Some("/galleon".to_string()));
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
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");

    // 1. GET /galleon/health -> 200 OK
    let resp_nested = client
        .get(format!("http://{local_addr}/galleon/health"))
        .send()
        .await
        .expect("GET /galleon/health");
    assert_eq!(
        resp_nested.status(),
        reqwest::StatusCode::OK,
        "GET /galleon/health should return 200 OK when path_prefix is /galleon"
    );

    // 2. GET /health -> 404 NOT FOUND
    let resp_unprefixed = client
        .get(format!("http://{local_addr}/health"))
        .send()
        .await
        .expect("GET /health");
    assert_eq!(
        resp_unprefixed.status(),
        reqwest::StatusCode::NOT_FOUND,
        "GET /health should return 404 NOT FOUND when path_prefix is /galleon"
    );

    // 3. GET /galleon/ -> 308 Permanent Redirect to /galleon
    let resp_trailing = client
        .get(format!("http://{local_addr}/galleon/"))
        .send()
        .await
        .expect("GET /galleon/");
    assert_eq!(
        resp_trailing.status(),
        reqwest::StatusCode::PERMANENT_REDIRECT,
        "GET /galleon/ should return 308 PERMANENT_REDIRECT"
    );
    assert_eq!(
        resp_trailing
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/galleon"),
        "Location header should redirect to /galleon"
    );
}

#[tokio::test]
async fn test_path_prefix_none_default_routing() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp, None);
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

    // Without prefix, /health returns 200 OK
    let resp = client
        .get(format!("http://{local_addr}/health"))
        .send()
        .await
        .expect("GET /health");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
}
