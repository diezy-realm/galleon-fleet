//! Integration tests for Gateway tunnel restoration and lifecycle (SPEC-017, TASK-104).
//!
//! Acceptance criteria:
//! - Tunnel runs when configured (tunnel_provider = "custom" with start_command 'echo https://example.test') -> URL recorded.
//! - Tunnel failure or invalid provider does not crash the gateway (gateway stays running, /ready returns 200).
//! - Gateway shuts down cleanly and stops the tunnel.

#![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]
use std::time::Duration;

use galleon_config::schema::{
    AliasedAgentConfig, Config, CustomTunnelConfig, ModelProviderConfig, OllamaModelProviderConfig,
    RiskProfileConfig, RuntimeProfileConfig,
};

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
    config.agents.insert("default".to_string(), agent);

    config
}

#[tokio::test]
async fn test_custom_tunnel_url_recorded_and_gateway_serves() {
    galleon_log::try_install_capture_subscriber();
    let mut log_rx = galleon_log::subscribe_or_install();
    while log_rx.try_recv().is_ok() {}

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let mut config = create_test_config(&tmp);

    config.tunnel.tunnel_provider = "custom".to_string();
    config.tunnel.custom = Some(CustomTunnelConfig {
        start_command: "echo https://example.test".to_string(),
        health_url: None,
        url_pattern: None,
    });

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

    // Gateway reports ready after tunnel initialization
    ready_rx
        .wait_for(|addr| addr.is_some())
        .await
        .expect("readiness reporter must fire");

    let bound_addr = ready_rx.borrow().unwrap();
    assert_ne!(bound_addr.port(), 0);

    // Verify gateway HTTP probe responds 200 OK
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{bound_addr}/ready"))
        .send()
        .await
        .expect("GET /ready on bound addr");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    // Verify tunnel URL was recorded in log events
    let mut url_found = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let step = remaining.min(Duration::from_millis(50));
        if let Ok(Ok(event_val)) = tokio::time::timeout(step, log_rx.recv()).await {
            let msg = event_val
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let attr_url = event_val
                .get("attributes")
                .and_then(|v| v.get("url"))
                .and_then(|u| u.as_str())
                .unwrap_or_default();

            if msg.contains("https://example.test") || attr_url.contains("https://example.test") {
                url_found = true;
                break;
            }
        }
    }
    assert!(
        url_found,
        "expected tunnel URL 'https://example.test' to be recorded in logs"
    );

    // Clean shutdown
    let _ = shutdown_tx.send(true);
    let server_result = tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("server shutdown timeout")
        .expect("task join");
    assert!(
        server_result.is_ok(),
        "run_gateway exited cleanly with tunnel stop"
    );
}

#[tokio::test]
async fn test_invalid_tunnel_provider_does_not_crash_gateway() {
    galleon_log::try_install_capture_subscriber();
    let mut log_rx = galleon_log::subscribe_or_install();
    while log_rx.try_recv().is_ok() {}

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let mut config = create_test_config(&tmp);

    // Configure an invalid tunnel provider
    config.tunnel.tunnel_provider = "invalid-tunnel-provider-xyz".to_string();

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

    // Gateway must still boot and report ready despite invalid tunnel
    ready_rx
        .wait_for(|addr| addr.is_some())
        .await
        .expect("gateway must boot and report ready even if tunnel fails");

    let bound_addr = ready_rx.borrow().unwrap();
    assert_ne!(bound_addr.port(), 0);

    // Verify HTTP probe still works
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{bound_addr}/ready"))
        .send()
        .await
        .expect("GET /ready on bound addr");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    // Verify a warning was recorded for tunnel failure
    let mut warn_found = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let step = remaining.min(Duration::from_millis(50));
        if let Ok(Ok(event_val)) = tokio::time::timeout(step, log_rx.recv()).await {
            let msg = event_val
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if msg.contains("tunnel") || msg.contains("Tunnel") {
                warn_found = true;
                break;
            }
        }
    }
    assert!(
        warn_found,
        "expected a warning log about tunnel adapter failure"
    );

    // Clean shutdown
    let _ = shutdown_tx.send(true);
    let server_result = tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("server shutdown timeout")
        .expect("task join");
    assert!(server_result.is_ok(), "run_gateway exited cleanly");
}
