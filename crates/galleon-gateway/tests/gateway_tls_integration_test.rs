//! Integration tests for Gateway TLS and mTLS support (SPEC-017, TASK-102).
//!
//! Acceptance criteria:
//! - tls.enabled = true -> TLS aktif (test)
//! - mTLS mengikuti client_auth
//! - GET /health via TLS 200; HTTP polos gagal handshake

#![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]
use futures_util::{SinkExt, StreamExt};
use galleon_config::multi_agent::MemoryBackendKind;
use galleon_config::schema::{
    AliasedAgentConfig, Config, GatewayClientAuthConfig, GatewayTlsConfig, ModelProviderConfig,
    OllamaModelProviderConfig, RiskProfileConfig, RuntimeProfileConfig,
};
use std::io::Write;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message as WsMessage;

fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
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

fn generate_ca() -> (String, String, rcgen::KeyPair) {
    let ca_key = rcgen::KeyPair::generate().unwrap();
    let mut ca_params = rcgen::CertificateParams::new(vec!["Test CA".into()]).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    (ca_cert.pem(), ca_key.serialize_pem(), ca_key)
}

fn generate_server_cert(ca_key: &rcgen::KeyPair) -> (String, String) {
    let ca_key_clone = rcgen::KeyPair::from_pem(&ca_key.serialize_pem()).unwrap();
    let mut ca_params = rcgen::CertificateParams::new(vec!["Test CA".into()]).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca = ca_params.self_signed(&ca_key_clone).unwrap();

    let mut server_params =
        rcgen::CertificateParams::new(vec!["localhost".into(), "127.0.0.1".into()]).unwrap();
    server_params.is_ca = rcgen::IsCa::NoCa;
    let server_key = rcgen::KeyPair::generate().unwrap();
    let server_cert = server_params
        .signed_by(&server_key, &ca, &ca_key_clone)
        .unwrap();
    (server_cert.pem(), server_key.serialize_pem())
}

fn generate_client_cert(ca_key: &rcgen::KeyPair) -> (String, String) {
    let ca_key_clone = rcgen::KeyPair::from_pem(&ca_key.serialize_pem()).unwrap();
    let mut ca_params = rcgen::CertificateParams::new(vec!["Test CA".into()]).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca = ca_params.self_signed(&ca_key_clone).unwrap();

    let mut client_params = rcgen::CertificateParams::new(vec!["Test Client".into()]).unwrap();
    client_params.is_ca = rcgen::IsCa::NoCa;
    let client_key = rcgen::KeyPair::generate().unwrap();
    let client_cert = client_params
        .signed_by(&client_key, &ca, &ca_key_clone)
        .unwrap();
    (client_cert.pem(), client_key.serialize_pem())
}

fn write_temp_file(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

#[tokio::test]
async fn test_gateway_tls_health_200_and_plain_http_handshake_fails() {
    ensure_crypto_provider();
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let mut config = create_test_config(&tmp);

    let (ca_cert_pem, _, ca_key) = generate_ca();
    let (server_cert_pem, server_key_pem) = generate_server_cert(&ca_key);

    let cert_file = write_temp_file(&server_cert_pem);
    let key_file = write_temp_file(&server_key_pem);

    config.gateway.tls = Some(GatewayTlsConfig {
        enabled: true,
        cert_path: cert_file.path().to_str().unwrap().to_string(),
        key_path: key_file.path().to_str().unwrap().to_string(),
        client_auth: None,
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

    ready_rx
        .wait_for(|addr| addr.is_some())
        .await
        .expect("readiness reporter must fire");

    let bound_addr = ready_rx.borrow().unwrap();
    assert_ne!(bound_addr.port(), 0);

    // 1. GET /health via TLS succeeds with 200 OK and HSTS header
    let root_cert = reqwest::Certificate::from_pem(ca_cert_pem.as_bytes()).unwrap();
    let https_client = reqwest::Client::builder()
        .add_root_certificate(root_cert)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("https client build");

    let resp = https_client
        .get(format!("https://localhost:{}/health", bound_addr.port()))
        .send()
        .await
        .expect("GET /health via HTTPS");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let headers = resp.headers();
    assert!(
        headers.contains_key("strict-transport-security"),
        "HSTS header must be present on TLS responses"
    );
    let body: serde_json::Value = resp.json().await.expect("json parse");
    assert_eq!(body["status"], "ok");

    // 2. HTTP polos fails handshake
    let plain_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("plain client build");

    let plain_resp = plain_client
        .get(format!("http://localhost:{}/health", bound_addr.port()))
        .send()
        .await;

    assert!(
        plain_resp.is_err(),
        "plain HTTP request to TLS gateway must fail handshake"
    );

    // Also verify via raw TCP that plain HTTP request receives no HTTP 200
    let mut tcp_conn = tokio::net::TcpStream::connect(bound_addr)
        .await
        .expect("connect plain tcp");
    tcp_conn
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .expect("write plain HTTP request");
    let mut buf = Vec::new();
    let _ = tcp_conn.read_to_end(&mut buf).await;
    let resp_str = String::from_utf8_lossy(&buf);
    assert!(
        !resp_str.contains("HTTP/1.1 200"),
        "plain HTTP must not receive 200 OK from TLS listener"
    );

    // Shutdown gateway cleanly
    let _ = shutdown_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("timeout waiting for gateway shutdown");
}

#[tokio::test]
async fn test_gateway_mtls_client_auth_enforced() {
    ensure_crypto_provider();
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let mut config = create_test_config(&tmp);

    let (ca_cert_pem, _, ca_key) = generate_ca();
    let (server_cert_pem, server_key_pem) = generate_server_cert(&ca_key);
    let (client_cert_pem, client_key_pem) = generate_client_cert(&ca_key);

    let cert_file = write_temp_file(&server_cert_pem);
    let key_file = write_temp_file(&server_key_pem);
    let ca_file = write_temp_file(&ca_cert_pem);

    config.gateway.tls = Some(GatewayTlsConfig {
        enabled: true,
        cert_path: cert_file.path().to_str().unwrap().to_string(),
        key_path: key_file.path().to_str().unwrap().to_string(),
        client_auth: Some(GatewayClientAuthConfig {
            enabled: true,
            ca_cert_path: ca_file.path().to_str().unwrap().to_string(),
            require_client_cert: true,
            pinned_certs: vec![],
            crl_path: String::new(),
        }),
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

    ready_rx
        .wait_for(|addr| addr.is_some())
        .await
        .expect("readiness reporter must fire");

    let bound_addr = ready_rx.borrow().unwrap();

    // 1. Client without client certificate fails handshake when mTLS is required
    let root_cert = reqwest::Certificate::from_pem(ca_cert_pem.as_bytes()).unwrap();
    let https_client_no_cert = reqwest::Client::builder()
        .add_root_certificate(root_cert.clone())
        .timeout(Duration::from_secs(3))
        .build()
        .expect("client without client cert");

    let unauth_resp = https_client_no_cert
        .get(format!("https://localhost:{}/health", bound_addr.port()))
        .send()
        .await;

    assert!(
        unauth_resp.is_err(),
        "client without client certificate must fail mTLS handshake"
    );

    // 2. Client with valid client certificate succeeds
    let client_identity_pem = format!("{client_cert_pem}\n{client_key_pem}");
    let identity = reqwest::Identity::from_pem(client_identity_pem.as_bytes())
        .expect("create client identity");
    let https_client_with_cert = reqwest::Client::builder()
        .add_root_certificate(root_cert)
        .identity(identity)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client with client cert");

    let auth_resp = https_client_with_cert
        .get(format!("https://localhost:{}/health", bound_addr.port()))
        .send()
        .await
        .expect("GET /health with client cert");

    assert_eq!(auth_resp.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = auth_resp.json().await.expect("json parse");
    assert_eq!(body["status"], "ok");

    // Shutdown gateway cleanly
    let _ = shutdown_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("timeout waiting for gateway shutdown");
}

#[test]
fn test_public_bind_detection() {
    assert!(!galleon_config::pairing::is_public_bind("127.0.0.1"));
    assert!(!galleon_config::pairing::is_public_bind("localhost"));
    assert!(!galleon_config::pairing::is_public_bind("::1"));
    assert!(galleon_config::pairing::is_public_bind("0.0.0.0"));
    assert!(galleon_config::pairing::is_public_bind("192.168.1.100"));
}

#[tokio::test]
async fn test_gateway_tls_websocket_upgrade_wss() {
    ensure_crypto_provider();
    galleon_runtime::install_agent_runner();

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let mut config = create_test_config(&tmp);

    let (ca_cert_pem, _, ca_key) = generate_ca();
    let (server_cert_pem, server_key_pem) = generate_server_cert(&ca_key);

    let cert_file = write_temp_file(&server_cert_pem);
    let key_file = write_temp_file(&server_key_pem);
    let ca_file = write_temp_file(&ca_cert_pem);

    config.gateway.tls = Some(GatewayTlsConfig {
        enabled: true,
        cert_path: cert_file.path().to_str().unwrap().to_string(),
        key_path: key_file.path().to_str().unwrap().to_string(),
        client_auth: None,
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

    ready_rx
        .wait_for(|addr| addr.is_some())
        .await
        .expect("readiness reporter must fire");

    let bound_addr = ready_rx.borrow().unwrap();

    // 1. Establish TCP connection
    let tcp_conn = tokio::net::TcpStream::connect(bound_addr)
        .await
        .expect("connect TCP");

    // 2. Wrap TCP in TLS stream using server's CA
    let mut root_store = rustls::RootCertStore::empty();
    let ca_certs = galleon_tls::load_certs(ca_file.path().to_str().unwrap())
        .expect("load CA certs");
    for cert in ca_certs {
        root_store.add(cert).expect("add CA cert");
    }
    let client_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(client_config));
    let server_name = rustls::pki_types::ServerName::try_from("localhost")
        .expect("localhost server name")
        .to_owned();
    let tls_stream = connector
        .connect(server_name, tcp_conn)
        .await
        .expect("TLS handshake");

    // 3. Connect WSS via tokio-tungstenite over the TLS stream
    let ws_url = format!(
        "wss://localhost:{}/ws?agent=default&session_id=tls-ws-session-1",
        bound_addr.port()
    );
    let (mut ws_stream, response) = tokio_tungstenite::client_async(&ws_url, tls_stream)
        .await
        .expect("WSS handshake and upgrade");

    assert_eq!(
        response.status(),
        axum::http::StatusCode::SWITCHING_PROTOCOLS,
        "WSS response status must be 101 Switching Protocols"
    );

    // 4. Verify session_start frame received over TLS WebSocket
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
    assert_eq!(session_start_json["session_id"], "tls-ws-session-1");

    // 5. Verify ping/pong over TLS WebSocket
    let ping_payload = vec![0xCA, 0xFE, 0xBA, 0xBE];
    ws_stream
        .send(WsMessage::Ping(ping_payload.clone().into()))
        .await
        .expect("send Ping frame over TLS");

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
                "Pong payload must echo Ping payload over TLS"
            );
        }
        other => panic!("expected Pong frame, got: {other:?}"),
    }

    // 6. Clean shutdown
    let _ = shutdown_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("timeout waiting for gateway shutdown");
}
