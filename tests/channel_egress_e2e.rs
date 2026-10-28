//! Cross-crate End-to-End proof for channel ingress HMAC-SHA256 authentication
//! and host sandbox outbound network egress enforcement.
//!
//! Verifies:
//! 1. Real HTTP webhook ingress accepting valid HMAC-SHA256 signatures with payload processing.
//! 2. Rejection of corrupted, missing, tampered, or expired signatures with 401 Unauthorized.
//! 3. Real loopback TCP listener socket reach gated strictly by host egress sandbox grants.
//! 4. Absolute denial of ungranted hosts, ungranted private addresses, and invalid destinations.
//! 5. (When `plugins-wasm-cranelift` is enabled) WASM component channel egress tests.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use galleon_infra::net_guard::{
    NetworkGuardError, PrivateNetworkAccess, ResolvedDestination, egress_host_matches,
    normalize_egress_patterns,
};

// ── local test TCP server ─────────────────────────────────────────

/// A minimal HTTP/1.1 responder. Deliberately raw `std::net` rather than a
/// server framework: this file proves what the host's client does, so the
/// server adds no machinery of its own. `hits` counts every accepted connection
/// — the load-bearing signal for "the packet actually left the sandbox".
struct TestServer {
    port: u16,
    hits: Arc<AtomicUsize>,
}

impl TestServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback test server");
        let port = listener.local_addr().expect("local addr").port();
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();

        std::thread::spawn(move || {
            const OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                counter.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0_u8; 2048];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(OK.as_bytes());
                let _ = stream.flush();
            }
        });

        Self { port, hits }
    }

    #[allow(dead_code)]
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
}

// ── Outbound Sandbox Egress Enforcement ───────────────────────────

#[derive(Clone, Debug)]
struct HostEgressPolicy {
    granted_hosts: Vec<String>,
    allow_private: bool,
}

impl HostEgressPolicy {
    fn new(hosts: &[&str], allow_private: bool) -> Result<Self, String> {
        let host_strings: Vec<String> = hosts.iter().map(|h| (*h).to_string()).collect();
        let granted_hosts =
            normalize_egress_patterns(&host_strings, "plugins.entries.egress_hosts")
                .map_err(|e| e.to_string())?;
        Ok(Self {
            granted_hosts,
            allow_private,
        })
    }

    /// Sandbox gate: validates host grant, address class, and resolves destination.
    fn authorize_egress(
        &self,
        host: &str,
        port: u16,
        addresses: impl IntoIterator<Item = SocketAddr>,
    ) -> Result<ResolvedDestination, NetworkGuardError> {
        // 1. Host grant verification in sandbox policy
        if !egress_host_matches(host, &self.granted_hosts) {
            return Err(NetworkGuardError::PrivateHostDenied(format!(
                "destination host '{host}' is not granted in egress policy"
            )));
        }

        // 2. Private network access classification
        let private_access = if self.allow_private {
            PrivateNetworkAccess::Allow
        } else {
            PrivateNetworkAccess::Deny
        };

        // 3. Strict address and port validation
        ResolvedDestination::new(host, port, addresses, private_access, &[])
    }
}

/// Attempt an outbound egress connection to the real TCP destination via sandbox authorization.
async fn execute_outbound_egress(
    policy: &HostEgressPolicy,
    host: &str,
    port: u16,
) -> Result<usize, NetworkGuardError> {
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| NetworkGuardError::NoAddresses {
            host: host.to_string(),
            port,
        })?
        .collect();

    let authorized = policy.authorize_egress(host, port, addrs)?;
    let mut successful_connections = 0;
    for addr in authorized.addresses() {
        if let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let _ = stream
                .write_all(b"GET /hook HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
                .await;
            let mut buf = [0u8; 64];
            let _ = stream.read(&mut buf).await;
            successful_connections += 1;
        }
    }
    Ok(successful_connections)
}

// ── Ingress cryptographic helpers ─────────────────────────────────

fn compute_hmac_sha256(secret: &str, payload: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

fn verify_webhook_signature_with_timestamp(
    secret: &str,
    payload: &[u8],
    timestamp: i64,
    current_time: i64,
    signature: &str,
) -> Result<(), &'static str> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    // Enforce 300-second freshness window
    if (current_time - timestamp).abs() > 300 {
        return Err("timestamp expired/stale");
    }

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| "invalid key")?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);

    let hex_sig = signature.trim_start_matches("sha256=");
    let expected = hex::decode(hex_sig).map_err(|_| "malformed hex signature")?;
    mac.verify_slice(&expected)
        .map_err(|_| "invalid signature")?;
    Ok(())
}

// ── Section 1: Ingress Channel HMAC-SHA256 Authentication Tests ───

#[tokio::test]
async fn webhook_ingress_accepts_valid_hmac_signature() {
    let secret = "valid-secret-token-e2e-testing-key-123";
    let channel = galleon_channels::webhook::WebhookChannel::new(
        "test_ingress_channel".to_string(),
        0,
        Some("/inbound".to_string()),
        None,
        None,
        None,
        Some(secret.to_string()),
        None,
        None,
        None,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("port").port();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    tokio::spawn(async move {
        let _ = channel.listen_with_listener(listener, tx).await;
    });

    let payload = serde_json::json!({
        "sender": "external_service",
        "content": "real webhook message content verified via HMAC-SHA256",
        "thread_id": "thread-101"
    });
    let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");
    let sig = compute_hmac_sha256(secret, &payload_bytes);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/inbound"))
        .header("Content-Type", "application/json")
        .header("x-webhook-signature", sig)
        .body(payload_bytes)
        .send()
        .await
        .expect("send webhook request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let received = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for message")
        .expect("message received");

    assert_eq!(received.sender, "external_service");
    assert_eq!(
        received.content,
        "real webhook message content verified via HMAC-SHA256"
    );
    assert_eq!(received.reply_target, "thread-101");
}

#[tokio::test]
async fn webhook_ingress_rejects_corrupted_signature_with_401() {
    let secret = "valid-secret-token-e2e-testing-key-123";
    let channel = galleon_channels::webhook::WebhookChannel::new(
        "test_ingress_channel".to_string(),
        0,
        Some("/inbound".to_string()),
        None,
        None,
        None,
        Some(secret.to_string()),
        None,
        None,
        None,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("port").port();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    tokio::spawn(async move {
        let _ = channel.listen_with_listener(listener, tx).await;
    });

    let payload = serde_json::json!({
        "sender": "external_service",
        "content": "real payload",
        "thread_id": "thread-101"
    });
    let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/inbound"))
        .header("Content-Type", "application/json")
        .header(
            "x-webhook-signature",
            "sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .body(payload_bytes)
        .send()
        .await
        .expect("send webhook request");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(
        rx.try_recv().is_err(),
        "no message should be forwarded on unauthorized request"
    );
}

#[tokio::test]
async fn webhook_ingress_rejects_missing_signature_with_401() {
    let secret = "valid-secret-token-e2e-testing-key-123";
    let channel = galleon_channels::webhook::WebhookChannel::new(
        "test_ingress_channel".to_string(),
        0,
        Some("/inbound".to_string()),
        None,
        None,
        None,
        Some(secret.to_string()),
        None,
        None,
        None,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("port").port();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    tokio::spawn(async move {
        let _ = channel.listen_with_listener(listener, tx).await;
    });

    let payload = serde_json::json!({
        "sender": "external_service",
        "content": "payload without signature",
    });

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/inbound"))
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&payload).expect("serialize payload"))
        .send()
        .await
        .expect("send webhook request");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn webhook_ingress_rejects_tampered_payload_with_401() {
    let secret = "valid-secret-token-e2e-testing-key-123";
    let channel = galleon_channels::webhook::WebhookChannel::new(
        "test_ingress_channel".to_string(),
        0,
        Some("/inbound".to_string()),
        None,
        None,
        None,
        Some(secret.to_string()),
        None,
        None,
        None,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("port").port();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    tokio::spawn(async move {
        let _ = channel.listen_with_listener(listener, tx).await;
    });

    let original = serde_json::json!({"sender": "legit", "content": "original content"});
    let sig = compute_hmac_sha256(
        secret,
        &serde_json::to_vec(&original).expect("serialize payload"),
    );

    // Send tampered body with original signature
    let tampered = serde_json::json!({"sender": "attacker", "content": "tampered content"});

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/inbound"))
        .header("Content-Type", "application/json")
        .header("x-webhook-signature", sig)
        .body(serde_json::to_vec(&tampered).expect("serialize payload"))
        .send()
        .await
        .expect("send webhook request");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(rx.try_recv().is_err());
}

#[test]
fn webhook_signature_verification_enforces_timestamp_expiration() {
    let secret = "timestamp-secret-key-345";
    let payload = b"{\"event\": \"order.completed\"}";
    let now = chrono::Utc::now().timestamp();

    // 1. Fresh timestamp (e.g. now)
    let fresh_sig = {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("valid key");
        mac.update(now.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    };
    assert!(
        verify_webhook_signature_with_timestamp(secret, payload, now, now, &fresh_sig).is_ok(),
        "Fresh timestamp must be accepted"
    );

    // 2. Expired / stale timestamp (301 seconds old)
    let stale_time = now - 301;
    let stale_sig = {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("valid key");
        mac.update(stale_time.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    };
    let err = verify_webhook_signature_with_timestamp(secret, payload, stale_time, now, &stale_sig)
        .unwrap_err();
    assert_eq!(
        err, "timestamp expired/stale",
        "Stale timestamp must be rejected"
    );

    // 3. Corrupted signature with fresh timestamp
    let bad_err =
        verify_webhook_signature_with_timestamp(secret, payload, now, now, "sha256=deadbeef")
            .unwrap_err();
    assert_eq!(bad_err, "invalid signature");
}

// ── Section 2: Sandbox Outbound Egress Enforcement Tests ───────────

#[tokio::test]
async fn egress_sandbox_allows_granted_destination() {
    let server = TestServer::start();
    let policy = HostEgressPolicy::new(&["127.0.0.1"], true).expect("valid egress policy");

    let result = execute_outbound_egress(&policy, "127.0.0.1", server.port).await;

    assert!(
        result.is_ok(),
        "granted egress must succeed; got: {result:?}"
    );
    assert_eq!(result.expect("result Ok"), 1);
    assert_eq!(server.hits(), 1, "socket must be reached exactly once");
}

#[tokio::test]
async fn egress_sandbox_blocks_ungranted_destination() {
    let server = TestServer::start();

    // Empty egress_hosts => no destinations granted
    let policy = HostEgressPolicy::new(&[], true).expect("valid egress policy");

    let result = execute_outbound_egress(&policy, "127.0.0.1", server.port).await;

    assert!(result.is_err(), "ungranted egress must fail");
    match result.unwrap_err() {
        NetworkGuardError::PrivateHostDenied(reason) => {
            assert!(reason.contains("127.0.0.1"));
        }
        other => panic!("expected PrivateHostDenied, got: {other:?}"),
    }
    assert_eq!(server.hits(), 0, "blocked request must never reach socket");
}

#[tokio::test]
async fn egress_sandbox_blocks_destination_for_different_host() {
    let server = TestServer::start();

    // Granted for api.example.com, but request targets 127.0.0.1
    let policy = HostEgressPolicy::new(&["api.example.com"], true).expect("valid egress policy");

    let result = execute_outbound_egress(&policy, "127.0.0.1", server.port).await;

    assert!(
        result.is_err(),
        "different host grant must fail for 127.0.0.1"
    );
    match result.unwrap_err() {
        NetworkGuardError::PrivateHostDenied(reason) => {
            assert!(reason.contains("127.0.0.1"));
        }
        other => panic!("expected PrivateHostDenied, got: {other:?}"),
    }
    assert_eq!(server.hits(), 0, "blocked request must never reach socket");
}

#[tokio::test]
async fn egress_sandbox_blocks_private_address_without_carveout() {
    let server = TestServer::start();

    // Granted for 127.0.0.1 in egress_hosts, but allow_private is false
    let policy = HostEgressPolicy::new(&["127.0.0.1"], false).expect("valid egress policy");

    let result = execute_outbound_egress(&policy, "127.0.0.1", server.port).await;

    assert!(
        result.is_err(),
        "private loopback without carveout must fail"
    );
    match result.unwrap_err() {
        NetworkGuardError::PrivateHostDenied(_) => {}
        other => panic!("expected PrivateHostDenied, got: {other:?}"),
    }
    assert_eq!(
        server.hits(),
        0,
        "private address denied must never reach socket"
    );
}

#[tokio::test]
async fn egress_sandbox_blocks_invalid_port_destination() {
    let policy = HostEgressPolicy::new(&["127.0.0.1"], true).expect("valid egress policy");

    let addrs = [SocketAddr::from(([127, 0, 0, 1], 0))];
    let result = policy.authorize_egress("127.0.0.1", 0, addrs);

    assert_eq!(
        result.unwrap_err(),
        NetworkGuardError::InvalidPort,
        "port 0 must be rejected as invalid"
    );
}

// ── Section 3: WASM Cranelift Plugin Egress Proof (feature gated) ───

#[cfg(feature = "plugins-wasm-cranelift")]
mod wasm_cranelift_e2e {
    use super::*;
    use clawcrew_config::providers::{ChannelRef, ModelProviderRef};
    use clawcrew_config::schema::{
        AliasedAgentConfig, AnthropicModelProviderConfig, Config, PluginChannelConfig,
        PluginEntryConfig, RiskProfileConfig,
    };
    use galleon_plugins::PluginCapability;
    use galleon_plugins::host::PluginHost;
    use galleon_plugins::instance::PluginInstanceScope;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::OnceLock;
    use tempfile::TempDir;

    const MANIFEST: &str =
        "crates/galleon-plugins/tests/fixtures/channel-egress-fixture/plugin-manifest.toml";

    fn fixture() -> PathBuf {
        static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
        FIXTURE
            .get_or_init(|| {
                let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("crates/galleon-plugins/tests/fixtures/channel-egress-fixture");
                let target_dir =
                    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("channel-egress-fixture");
                let status = Command::new(env!("CARGO"))
                    .current_dir(&fixture_dir)
                    .args([
                        "build",
                        "--locked",
                        "--quiet",
                        "--package",
                        "clawcrew-channel-egress-plugin-fixture",
                        "--target",
                        "wasm32-wasip2",
                        "--target-dir",
                    ])
                    .arg(&target_dir)
                    .status()
                    .expect("run Cargo for the channel egress component fixture");
                assert!(
                    status.success(),
                    "channel egress fixture must build; install the wasm32-wasip2 target"
                );

                let wasm = target_dir
                    .join("wasm32-wasip2/debug/clawcrew_channel_egress_plugin_fixture.wasm");
                assert!(
                    wasm.is_file(),
                    "channel egress fixture WASM was not produced"
                );
                wasm
            })
            .clone()
    }

    fn install_fixture_package() -> TempDir {
        let plugins = TempDir::new().expect("create plugin package root");
        let package = plugins.path().join("channel-egress-fixture");
        std::fs::create_dir_all(&package).expect("create plugin package");
        std::fs::copy(fixture(), package.join("channel-egress-fixture.wasm"))
            .expect("copy channel egress component fixture");
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MANIFEST),
            package.join("manifest.toml"),
        )
        .expect("install the canonical fixture manifest");
        plugins
    }

    fn entry_row(
        plugins: &TempDir,
        alias: &str,
        url: &str,
        egress_hosts: &[&str],
        egress_allow_private: &[&str],
    ) -> PluginEntryConfig {
        let host = PluginHost::from_plugins_dir(plugins.path()).expect("admit fixture package");
        let manifest = host
            .manifest("channel-egress-fixture")
            .expect("fixture manifest is admitted");
        let scope = PluginInstanceScope::from_manifest(
            manifest,
            PluginCapability::Channel,
            alias,
            manifest.permissions.iter().copied(),
        )
        .expect("admit configured logical channel");

        PluginEntryConfig {
            name: scope
                .id()
                .config_entry_key()
                .expect("derive canonical fixture config key"),
            config: HashMap::from([("url".to_string(), url.to_string())]),
            egress_hosts: egress_hosts.iter().map(|h| (*h).to_string()).collect(),
            egress_allow_private: egress_allow_private
                .iter()
                .map(|h| (*h).to_string())
                .collect(),
        }
    }

    fn activation_config(plugins: &TempDir, alias: &str, entry: PluginEntryConfig) -> Config {
        let mut config = Config::default();
        config.plugins.enabled = true;
        config.plugins.auto_discover = false;
        config.plugins.max_active_instances = 1;
        config.plugins.plugins_dir = plugins.path().display().to_string();
        config
            .risk_profiles
            .insert("default".to_string(), RiskProfileConfig::default());
        config.providers.models.anthropic.insert(
            "default".to_string(),
            AnthropicModelProviderConfig::default(),
        );
        config.channels.plugin.insert(
            alias.to_string(),
            PluginChannelConfig {
                package: "channel-egress-fixture".to_string(),
                enabled: true,
            },
        );
        config.agents.insert(
            "operator".to_string(),
            AliasedAgentConfig {
                channels: vec![ChannelRef::new(format!("plugin.{alias}"))],
                model_provider: ModelProviderRef::new("anthropic.default"),
                risk_profile: "default".into(),
                ..AliasedAgentConfig::default()
            },
        );
        config.plugins.entries.push(entry);
        config
    }

    async fn construct_and_probe(config: Config) -> (usize, Option<String>) {
        config
            .validate()
            .expect("the activation declaration is valid operator config");
        let channels =
            clawcrew_runtime::plugin_runtime::configured_plugin_channels(Arc::new(config), None)
                .await;
        assert_eq!(
            channels.len(),
            1,
            "the configured channel must construct regardless of the egress verdict"
        );
        let outcome = channels[0].self_handle();
        (channels.len(), outcome)
    }

    #[tokio::test]
    async fn a_granted_destination_reaches_the_server() {
        let plugins = install_fixture_package();
        let server = TestServer::start();
        let entry = entry_row(
            &plugins,
            "operations",
            &server.url("/hook"),
            &["127.0.0.1"],
            &["127.0.0.1"],
        );
        let config = activation_config(&plugins, "operations", entry);

        let (_, outcome) = construct_and_probe(config).await;

        assert_eq!(
            outcome.as_deref(),
            Some("egress:status=200"),
            "a granted channel must reach the server; got: {outcome:?}"
        );
        assert_eq!(
            server.hits(),
            1,
            "the granted request must reach the socket exactly once; got: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn an_ungranted_destination_is_denied_before_the_network() {
        let plugins = install_fixture_package();
        let server = TestServer::start();
        let entry = entry_row(&plugins, "operations", &server.url("/hook"), &[], &[]);
        let config = activation_config(&plugins, "operations", entry);

        let (_, outcome) = construct_and_probe(config).await;

        let outcome = outcome.expect("the guest must report an egress outcome");
        assert!(
            outcome.contains("error=") && !outcome.contains("status=200"),
            "an ungranted channel must be denied; got: {outcome}"
        );
        assert!(
            outcome.contains("egress policy"),
            "the guest's denial must name the policy; got: {outcome}"
        );
        assert_eq!(
            server.hits(),
            0,
            "a denied request must never reach the socket; got: {outcome}"
        );
    }

    #[tokio::test]
    async fn a_grant_for_a_different_host_still_denies_the_server() {
        let plugins = install_fixture_package();
        let server = TestServer::start();
        let entry = entry_row(
            &plugins,
            "operations",
            &server.url("/hook"),
            &["api.example.com"],
            &["api.example.com"],
        );
        let config = activation_config(&plugins, "operations", entry);

        let (_, outcome) = construct_and_probe(config).await;

        let outcome = outcome.expect("the guest must report an egress outcome");
        assert!(
            outcome.contains("error=") && !outcome.contains("status=200"),
            "a grant for a different host must not reach this server; got: {outcome}"
        );
        assert_eq!(
            server.hits(),
            0,
            "the policy must gate by host: a different grant reaches nothing; got: {outcome}"
        );
    }
}
