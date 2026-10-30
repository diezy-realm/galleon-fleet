//! End-to-End tests for Axum Gateway REST API & WebSocket Protocol Lifecycle.
//!
//! Verifies real ephemeral TCP socket binding (127.0.0.1:0), real HTTP requests via
//! reqwest to /health, /ready, /api/sections/*, and real WebSocket connection via
//! tokio-tungstenite with handshake, ping/pong heartbeats, and chat stream frame delivery.
//! Zero mockup transport or stub routers used.

use galleon_config::multi_agent::MemoryBackendKind;
use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::{AppState, build_gateway_router};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Helper: creates an isolated test Config with a valid enabled default agent.
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
    config.gateway.websocket_ping_interval_secs = 2;
    config.gateway.require_pairing = false;

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
    agent.memory.backend = MemoryBackendKind::None;
    config.agents.insert("default".to_string(), agent);

    config
}

#[tokio::test]
async fn test_gateway_ephemeral_tcp_binding_and_http_endpoints_lifecycle() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    // 1. Jalankan Axum gateway binding pada ephemeral TCP socket lokal (127.0.0.1:0)
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral TCP listener");
    let local_addr = listener.local_addr().expect("local socket address");
    assert_ne!(local_addr.port(), 0, "ephemeral port must be allocated");

    let app = build_gateway_router(state.clone());
    let server_handle = galleon_spawn::spawn!(async move {
        axum::serve(listener, app)
            .await
            .expect("axum gateway serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");

    let base_url = format!("http://{local_addr}");

    // 2. Request HTTP nyata (reqwest) ke rute /health
    let health_resp = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("GET /health");
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);
    let health_json: serde_json::Value = health_resp.json().await.expect("health JSON body");
    assert_eq!(health_json["status"], "ok");
    assert!(health_json.get("runtime").is_some());

    // Request HTTP nyata ke rute /ready
    let ready_resp = client
        .get(format!("{base_url}/ready"))
        .send()
        .await
        .expect("GET /ready");
    assert_eq!(ready_resp.status(), reqwest::StatusCode::OK);
    let ready_json: serde_json::Value = ready_resp.json().await.expect("ready JSON body");
    assert_eq!(ready_json["status"], "ready");

    // Request HTTP nyata ke rute /api/sections
    let sections_resp = client
        .get(format!("{base_url}/api/sections"))
        .send()
        .await
        .expect("GET /api/sections");
    assert_eq!(sections_resp.status(), reqwest::StatusCode::OK);
    let sections_json: serde_json::Value = sections_resp.json().await.expect("sections JSON body");
    assert!(sections_json.get("sections").is_some() || sections_json.is_object());

    // Request HTTP nyata ke rute /api/sections/status
    let status_resp = client
        .get(format!("{base_url}/api/sections/status"))
        .send()
        .await
        .expect("GET /api/sections/status");
    assert_eq!(status_resp.status(), reqwest::StatusCode::OK);
    let status_json: serde_json::Value =
        status_resp.json().await.expect("sections status JSON body");
    assert!(status_json.get("needs_quickstart").is_some());

    // Request HTTP nyata ke rute /api/sections/agent-options
    let agent_opts_resp = client
        .get(format!("{base_url}/api/sections/agent-options"))
        .send()
        .await
        .expect("GET /api/sections/agent-options");
    assert_eq!(agent_opts_resp.status(), reqwest::StatusCode::OK);
    let agent_opts_json: serde_json::Value =
        agent_opts_resp.json().await.expect("agent options JSON");
    assert!(agent_opts_json.get("agents").is_some());

    // Request HTTP nyata ke rute /api/sections/catalog
    let catalog_resp = client
        .get(format!("{base_url}/api/sections/catalog"))
        .send()
        .await
        .expect("GET /api/sections/catalog");
    assert_eq!(catalog_resp.status(), reqwest::StatusCode::OK);
    let catalog_json: serde_json::Value = catalog_resp.json().await.expect("catalog JSON body");
    assert!(catalog_json.get("model_providers").is_some());

    // Request HTTP nyata ke rute /api/sections/memory
    let memory_sec_resp = client
        .get(format!("{base_url}/api/sections/memory"))
        .send()
        .await
        .expect("GET /api/sections/memory");
    assert_eq!(memory_sec_resp.status(), reqwest::StatusCode::OK);
    let memory_sec_json: serde_json::Value =
        memory_sec_resp.json().await.expect("memory section JSON");
    assert!(memory_sec_json.get("items").is_some() || memory_sec_json.is_object());

    // Clean shutdown of test server
    server_handle.abort();
}

#[tokio::test]
async fn test_gateway_websocket_handshake_ping_pong_and_stream_lifecycle() {
    galleon_runtime::install_agent_runner();

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);
    let state = AppState::from_config(config);

    // 1. Jalankan Axum gateway binding pada ephemeral TCP socket lokal (127.0.0.1:0)
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral TCP listener");
    let local_addr = listener.local_addr().expect("local socket address");
    let app = build_gateway_router(state.clone());
    let server_handle = galleon_spawn::spawn!(async move {
        axum::serve(listener, app)
            .await
            .expect("axum gateway serve");
    });

    let ws_url = format!("ws://{local_addr}/ws?agent=default&session_id=e2e-session-1");

    // 2. Lakukan koneksi WebSocket nyata (tokio-tungstenite) ke endpoint /ws dan selesaikan handshake upgrade
    let (mut ws_stream, response) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("WebSocket connect and handshake upgrade");
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SWITCHING_PROTOCOLS
    );

    // Frame awal session_start dikirimkan oleh server
    let first_frame = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .expect("session_start timeout")
        .expect("session_start frame")
        .expect("session_start transport");

    let session_start_json: serde_json::Value = match first_frame {
        WsMessage::Text(text) => serde_json::from_str(&text).expect("parse session_start JSON"),
        other => panic!("expected text frame session_start, got: {other:?}"),
    };
    assert_eq!(session_start_json["type"], "session_start");
    assert_eq!(session_start_json["session_id"], "e2e-session-1");

    // 3. Jalankan frame heartbeat ping/pong nyata
    let ping_payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
    ws_stream
        .send(WsMessage::Ping(ping_payload.clone().into()))
        .await
        .expect("send Ping frame");

    let pong_frame = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .expect("pong timeout")
        .expect("pong frame")
        .expect("pong transport");

    match pong_frame {
        WsMessage::Pong(payload) => {
            assert_eq!(
                payload.as_ref(),
                ping_payload.as_slice(),
                "Pong payload must echo Ping payload"
            );
        }
        other => panic!("expected Pong frame, got: {other:?}"),
    }

    // 4. Uji koneksi dan penerimaan frame chat stream secara utuh
    // Client mengirim frame connect
    let connect_payload = serde_json::json!({
        "type": "connect",
        "session_id": "e2e-session-1",
        "device_name": "qa-e2e-runner",
        "capabilities": ["stream", "chat"]
    });
    ws_stream
        .send(WsMessage::Text(connect_payload.to_string().into()))
        .await
        .expect("send connect frame");

    let ack_frame = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .expect("connected ack timeout")
        .expect("connected ack frame")
        .expect("connected ack transport");

    let ack_json: serde_json::Value = match ack_frame {
        WsMessage::Text(text) => serde_json::from_str(&text).expect("parse connected ack JSON"),
        other => panic!("expected text connected ack, got: {other:?}"),
    };
    assert_eq!(ack_json["type"], "connected");

    // Beri sedikit jeda agar session loop selesai inisialisasi broadcast subscription
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Uji penerimaan frame chat stream dari event broadcasting ke session ini
    let stream_event = serde_json::json!({
        "session_id": "e2e-session-1",
        "type": "chat_stream",
        "delta": "real time token stream",
        "index": 1
    });
    state
        .event_tx
        .send(stream_event)
        .expect("broadcast chat stream event to session");

    let received_stream_frame = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .expect("stream frame timeout")
        .expect("stream frame")
        .expect("stream transport");

    let stream_json: serde_json::Value = match received_stream_frame {
        WsMessage::Text(text) => serde_json::from_str(&text).expect("parse stream JSON"),
        other => panic!("expected text stream frame, got: {other:?}"),
    };
    assert_eq!(stream_json["type"], "chat_stream");
    assert_eq!(stream_json["delta"], "real time token stream");
    assert_eq!(stream_json["session_id"], "e2e-session-1");

    // Close WebSocket cleanly
    ws_stream
        .send(WsMessage::Close(None))
        .await
        .expect("send Close frame");

    server_handle.abort();
}

#[tokio::test]
async fn test_run_gateway_supervision_and_readiness_lifecycle() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let config = create_test_config(&tmp);

    let (ready_tx, mut ready_rx) = tokio::sync::watch::channel(None);
    let readiness = galleon_infra::gateway_controls::GatewayReadinessReporter::new(move |addr| {
        let _ = ready_tx.send(Some(addr));
    });

    let (shutdown_tx, _) = tokio::sync::watch::channel(false);
    let (reload_tx, _) = tokio::sync::watch::channel(false);
    let reload_controls = galleon_infra::gateway_controls::GatewayReloadControls {
        shutdown_tx: shutdown_tx.clone(),
        reload_tx,
    };

    let server_task = galleon_spawn::spawn!(async move {
        galleon_gateway::run_gateway(
            "127.0.0.1",
            0,
            config,
            None,
            Some(reload_controls),
            None,
            None,
            None,
            Some(readiness),
        )
        .await
    });

    // Wait for gateway to report ready on ephemeral port
    ready_rx
        .wait_for(|addr| addr.is_some())
        .await
        .expect("readiness reporter must fire");

    let bound_addr = ready_rx.borrow().unwrap();
    assert_ne!(bound_addr.port(), 0);

    // Verify the HTTP probe on the reported address
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{bound_addr}/ready"))
        .send()
        .await
        .expect("GET /ready on bound addr");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    // Trigger graceful shutdown
    let _ = shutdown_tx.send(true);
    let server_result = tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("server shutdown timeout")
        .expect("task join");
    assert!(server_result.is_ok(), "run_gateway exited cleanly");
}
