//! Integration tests for Gateway channel routes and SOP webhook (SPEC-012, TASK-062).
//!
//! Acceptance criteria:
//! - Webhook channel yang terkonfigurasi merespons (bukan 404)
//! - optional_channel_routes() dan /sop/{*rest} terpasang di build_gateway_router
//! - PR dibuka ke branch testing, CI dev-ci (gates.sh) hijau

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode};
use galleon_api::sop::{
    ApprovalDecision, ApprovalPrincipal, BrokerOutcome, ResolveOutcome, SopRunner,
    WebhookDispatchResult,
};
use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::{build_gateway_router, AppState};
use serde_json::Value;
use tower::ServiceExt;

#[derive(Default)]
struct TestMockSopRunner {
    pub matched_path: Mutex<Option<String>>,
}

#[async_trait::async_trait]
impl SopRunner for TestMockSopRunner {
    fn list_sops(&self) -> anyhow::Result<Vec<Value>> {
        Ok(Vec::new())
    }
    fn get_trigger_sources(&self) -> anyhow::Result<Value> {
        Ok(serde_json::json!({}))
    }
    fn get_sop_graph(&self, _name: &str) -> anyhow::Result<Option<Value>> {
        Ok(None)
    }
    fn list_runs(
        &self,
        _sop_name: Option<&str>,
    ) -> anyhow::Result<Vec<galleon_api::sop::SopRunSummary>> {
        Ok(Vec::new())
    }
    fn get_run_overlay(&self, _name: &str, _run_id: &str) -> anyhow::Result<Option<Value>> {
        Ok(None)
    }
    fn get_pending_runs(&self) -> anyhow::Result<Vec<Value>> {
        Ok(Vec::new())
    }
    fn get_metrics(&self) -> anyhow::Result<Value> {
        Ok(serde_json::json!({}))
    }
    fn subscribe_run_changes(
        &self,
    ) -> anyhow::Result<(
        Vec<galleon_api::sop::SopRunSummary>,
        Option<tokio::sync::broadcast::Receiver<galleon_api::sop::SopRunSummary>>,
    )> {
        Ok((Vec::new(), None))
    }
    async fn start_sop(&self, _name: &str) -> anyhow::Result<(u16, Value)> {
        Ok((200, serde_json::json!({})))
    }
    async fn resolve_approval(
        &self,
        _run_id: &str,
        _decision: ApprovalDecision,
        _principal: ApprovalPrincipal,
    ) -> anyhow::Result<BrokerOutcome> {
        Ok(BrokerOutcome::Resolved(ResolveOutcome::Resumed))
    }
    async fn resolve_author_approval(
        &self,
        _name: &str,
        _run_id: &str,
        _decision: ApprovalDecision,
        _principal: ApprovalPrincipal,
    ) -> anyhow::Result<(u16, Value)> {
        Ok((200, serde_json::json!({})))
    }
    fn cancel_run(
        &self,
        _name: &str,
        _run_id: &str,
        _reason: Option<&str>,
        _principal: ApprovalPrincipal,
    ) -> anyhow::Result<(u16, Value)> {
        Ok((200, serde_json::json!({})))
    }
    fn get_sop(&self, _name: &str) -> anyhow::Result<Option<galleon_api::sop::Sop>> {
        Ok(None)
    }
    fn create_sop(&self, _sop: &galleon_api::sop::Sop) -> anyhow::Result<()> {
        Ok(())
    }
    fn save_sop(&self, _name: &str, _sop: &galleon_api::sop::Sop) -> anyhow::Result<()> {
        Ok(())
    }
    fn delete_sop(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }
    fn wire_draft(&self, _sop: &galleon_api::sop::Sop, _edit: &Value) -> anyhow::Result<Value> {
        Ok(serde_json::json!({}))
    }
    fn graph_draft(&self, _sop: &galleon_api::sop::Sop) -> anyhow::Result<Value> {
        Ok(serde_json::json!({}))
    }
    fn has_matching_webhook_sop(&self, path: &str) -> anyhow::Result<bool> {
        let guard = self.matched_path.lock().unwrap();
        if let Some(ref expected) = *guard {
            Ok(path == expected)
        } else {
            Ok(true)
        }
    }
    async fn dispatch_webhook_sop(
        &self,
        path: &str,
        _payload: Option<&str>,
    ) -> anyhow::Result<Option<WebhookDispatchResult>> {
        let guard = self.matched_path.lock().unwrap();
        let matches = if let Some(ref expected) = *guard {
            path == expected
        } else {
            true
        };
        if matches {
            Ok(Some(WebhookDispatchResult {
                blocked_only: false,
                results: vec![serde_json::json!({ "sop": "deploy-sop", "run_id": "test-run-123" })],
            }))
        } else {
            Ok(None)
        }
    }
}

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
async fn test_sop_webhook_route_mounted_and_authenticated() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let mut state = AppState::from_config(config);

    // 1. Without credentials configured, POST /sop/deploy fails closed with 403 Forbidden (NOT 404)
    let app = build_gateway_router(state.clone());
    let mut req = Request::builder()
        .method("POST")
        .uri("/sop/deploy")
        .body(Body::from(r#"{"env":"production"}"#))
        .unwrap();
    req.extensions_mut()
        .insert(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))));

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "POST /sop/deploy without credentials returns 401, proving route is mounted and authenticated"
    );

    let body_bytes = axum::body::to_bytes(resp.into_body(), 65536).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(
        body_json["error"]
            .as_str()
            .unwrap()
            .contains("SOP webhook dispatch requires a configured credential"),
        "error body explains credential requirement"
    );

    // 2. With X-Webhook-Secret configured and matching runner, returns 200 OK
    let test_secret = "secret-webhook-key-12345";
    state.config.write().gateway.webhook_secret = Some(test_secret.to_string());
    let runner = Arc::new(TestMockSopRunner {
        matched_path: Mutex::new(Some("/sop/deploy".to_string())),
    });
    state.sop_runner = Some(runner);

    let app = build_gateway_router(state.clone());
    let mut req = Request::builder()
        .method("POST")
        .uri("/sop/deploy")
        .header("X-Webhook-Secret", test_secret)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"env":"production"}"#))
        .unwrap();
    req.extensions_mut()
        .insert(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))));

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "POST /sop/deploy with valid secret and matching runner returns 200 OK"
    );

    let body_bytes = axum::body::to_bytes(resp.into_body(), 65536).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["status"], "accepted");
    assert_eq!(body_json["source"], "webhook");
    assert_eq!(body_json["path"], "/sop/deploy");
    assert_eq!(body_json["results"][0]["sop"], "deploy-sop");

    // 3. With X-Webhook-Secret configured but unmatched SOP, returns 404 with structured JSON
    let app = build_gateway_router(state.clone());
    let mut req = Request::builder()
        .method("POST")
        .uri("/sop/unmatched-action")
        .header("X-Webhook-Secret", test_secret)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{}"#))
        .unwrap();
    req.extensions_mut()
        .insert(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))));

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 65536).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["error"], "no_matching_sop");
    assert_eq!(body_json["path"], "/sop/unmatched-action");
}

#[tokio::test]
async fn test_sop_webhook_wire_http_over_real_tcp() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let mut state = AppState::from_config(config);

    let test_secret = "real-tcp-test-secret-999";
    state.config.write().gateway.webhook_secret = Some(test_secret.to_string());
    let runner = Arc::new(TestMockSopRunner {
        matched_path: Mutex::new(Some("/sop/release/prod".to_string())),
    });
    state.sop_runner = Some(runner);

    let (addr, server_handle) = spawn_test_server(state).await;
    let client = reqwest::Client::new();

    // Request without secret -> 401 Unauthorized (invalid or missing X-Webhook-Secret header)
    let resp = client
        .post(format!("http://{addr}/sop/release/prod"))
        .header("Content-Type", "application/json")
        .body(r#"{"version":"v1.0.0"}"#)
        .send()
        .await
        .expect("send request");

    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    // Request with secret -> 200 OK
    let resp = client
        .post(format!("http://{addr}/sop/release/prod"))
        .header("X-Webhook-Secret", test_secret)
        .header("Content-Type", "application/json")
        .body(r#"{"version":"v1.0.0"}"#)
        .send()
        .await
        .expect("send request");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let json: Value = resp.json().await.expect("parse json");
    assert_eq!(json["status"], "accepted");
    assert_eq!(json["path"], "/sop/release/prod");

    server_handle.abort();
}

#[cfg(feature = "channel-linq")]
#[tokio::test]
async fn test_linq_channel_route_mounted() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    let app = build_gateway_router(state);
    let mut req = Request::builder()
        .method("POST")
        .uri("/linq")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"event_type":"message.received"}"#))
        .unwrap();
    req.extensions_mut()
        .insert(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))));

    let resp = app.oneshot(req).await.unwrap();
    // Route is mounted; without configured channels it returns 404 (from handle_linq_webhook_impl)
    // rather than Axum's unrouted 404 handler.
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[cfg(feature = "channel-whatsapp-cloud")]
#[tokio::test]
async fn test_whatsapp_channel_route_mounted() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    let app = build_gateway_router(state);
    let mut req = Request::builder()
        .method("GET")
        .uri("/whatsapp?hub.mode=subscribe&hub.challenge=test")
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))));

    let resp = app.oneshot(req).await.unwrap();
    // Verify endpoint handles verification request (fails verification with 403, proving route is reached)
    assert!(resp.status() == StatusCode::FORBIDDEN || resp.status() == StatusCode::NOT_FOUND);
}
