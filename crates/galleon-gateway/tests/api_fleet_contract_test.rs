//! Contract tests for ported Fleet API endpoints (SPEC-013, BL-088, TASK-081).
//!
//! Acceptance criteria:
//! - RequestBodyLimitLayer::new(FLEET_MAX_BODY) is enforced on fleet router:
//!   - PUT collection 500 KB -> 200 OK
//!   - PUT collection 2 MiB -> 413 Payload Too Large
//! - Require_auth enforced: unauthenticated requests -> 401 Unauthorized
//! - All 19 ported endpoints match the shape of `tests/fixtures/go-fleet/*.json`
//!   (ignoring volatile keys documented in VOLATILE.md and intentional differences in SPEC-013).

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use galleon_config::pairing::PairingGuard;
use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::{AppState, build_gateway_router, live_pairing_code_policy};
use serde_json::Value;

const AUTH_TOKEN: &str = "test-fleet-contract-token";

fn get_repo_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| manifest_dir.join("../.."))
}

fn load_fixture(fixture_filename: &str) -> Value {
    let fixture_path = get_repo_root()
        .join("tests/fixtures/go-fleet")
        .join(fixture_filename);
    let content = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", fixture_path.display()));
    serde_json::from_str(&content).unwrap_or_else(|e| {
        panic!(
            "failed to parse JSON from fixture {}: {e}",
            fixture_path.display()
        )
    })
}

fn copy_dir_all(src: &Path, dst: &Path) {
    if !src.exists() {
        return;
    }
    std::fs::create_dir_all(dst).expect("create dest dir");
    for entry in std::fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("valid entry");
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path);
        } else {
            let _ = std::fs::copy(&src_path, &dst_path);
        }
    }
}

fn create_test_environment(tmp: &tempfile::TempDir) -> AppState {
    let data_dir = tmp.path().join("data");
    let fleet_dir = data_dir.join("fleet");
    let workspace_dir = tmp.path().join("workspace");
    std::fs::create_dir_all(&fleet_dir).expect("create fleet dir");
    std::fs::create_dir_all(&workspace_dir).expect("create workspace dir");

    // Copy seed files into fleet directory
    let repo_root = get_repo_root();
    let seed_src = repo_root.join("crates/galleon-fleet-core/seed");
    if seed_src.exists() {
        copy_dir_all(&seed_src, &fleet_dir);
    }
    // Also copy engine/data if present
    let engine_data = repo_root.join("engine/data");
    if engine_data.exists() {
        copy_dir_all(&engine_data, &fleet_dir);
    }

    // Initialize git repository in tmp.path() (install_root_dir) for git-status and terminal exec
    let _ = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(tmp.path())
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(tmp.path())
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.email", "test@galleon.local"])
        .current_dir(tmp.path())
        .output();
    let readme_path = tmp.path().join("README.md");
    let _ = std::fs::write(&readme_path, "# Galleon Fleet\n");
    let _ = std::process::Command::new("git")
        .args(["add", "README.md"])
        .current_dir(tmp.path())
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(tmp.path())
        .output();

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
            path: Some(workspace_dir.clone()),
            ..Default::default()
        },
        ..Default::default()
    };
    agent.enabled = true;
    config.agents.insert("default".to_string(), agent);

    let mut state = AppState::from_config(config);

    // Initialize PairingGuard with pre-authorized token
    state.pairing = Arc::new(PairingGuard::new(
        true,
        &[AUTH_TOKEN.to_string()],
        live_pairing_code_policy(&state),
    ));

    state
}

async fn spawn_test_gateway(state: AppState) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral TCP listener");
    let local_addr = listener.local_addr().expect("local socket address");
    let app = build_gateway_router(state);

    galleon_spawn::spawn!(async move {
        let make_service = app.into_make_service_with_connect_info::<SocketAddr>();
        let _ = axum::serve(listener, make_service).await;
    });

    local_addr
}

/// Recursively verifies that `rust` matches the shape (keys and types) of `expected`.
/// Volatile keys from VOLATILE.md and intentional differences from SPEC-013 are accommodated.
fn assert_same_shape(
    rust: &Value,
    expected: &Value,
    ignored_keys: &HashSet<&str>,
    nullable_keys: &HashSet<&str>,
    optional_keys: &HashSet<&str>,
    path: &str,
) {
    match (rust, expected) {
        (Value::Object(r_map), Value::Object(e_map)) => {
            for (key, exp_sub) in e_map {
                let key_str = key.as_str();

                // Ignored volatile keys: skip deep comparison
                if ignored_keys.contains(key_str) {
                    continue;
                }

                // Optional keys that may be omitted on success (e.g. error, stderr)
                if optional_keys.contains(key_str) && !r_map.contains_key(key) {
                    continue;
                }

                // Intentional differences from SPEC-013
                if key_str == "goVersion" {
                    assert!(
                        r_map.contains_key("version"),
                        "Rust response at '{path}' should replace goVersion with version"
                    );
                    continue;
                }
                if key_str == "engine_port" {
                    // SPEC-013: engine_port removed in Rust
                    continue;
                }

                assert!(
                    r_map.contains_key(key),
                    "Missing expected key '{key}' at path '{path}'. Available keys: {:?}",
                    r_map.keys().collect::<Vec<_>>()
                );

                let rust_sub = &r_map[key];
                if rust_sub.is_null()
                    && (nullable_keys.contains(key_str) || ignored_keys.contains(key_str))
                {
                    // Allowed null for documented intentional / volatile field
                    continue;
                }

                let sub_path = format!("{path}.{key}");
                assert_same_shape(
                    rust_sub,
                    exp_sub,
                    ignored_keys,
                    nullable_keys,
                    optional_keys,
                    &sub_path,
                );
            }
        }
        (Value::Array(r_arr), Value::Array(e_arr)) => {
            if !e_arr.is_empty() && !r_arr.is_empty() {
                assert_same_shape(
                    &r_arr[0],
                    &e_arr[0],
                    ignored_keys,
                    nullable_keys,
                    optional_keys,
                    &format!("{path}[0]"),
                );
            }
        }
        (Value::String(_), Value::String(_)) => {}
        (Value::Number(_), Value::Number(_)) => {}
        (Value::Bool(_), Value::Bool(_)) => {}
        (Value::Null, Value::Null) => {}
        (Value::Null, _) if nullable_keys.iter().any(|k| path.ends_with(k)) => {}
        (r, e) => {
            panic!(
                "Type mismatch at path '{path}': expected {:?}, got {:?}",
                e, r
            );
        }
    }
}

#[tokio::test]
async fn test_collections_payload_limit_enforcement() {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let state = create_test_environment(&tmp);
    let addr = spawn_test_gateway(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("client");

    // 1. Unauthenticated PUT -> 401 Unauthorized
    let resp = client
        .put(format!("http://{addr}/api/collections/squads"))
        .header("Content-Type", "application/json")
        .body("[]")
        .send()
        .await
        .expect("send unauth request");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 2. Authenticated PUT with 500 KB payload -> 200 OK
    // (500 KB collection must be accepted by fleet 1 MiB limit, whereas core 64 KB limit would reject)
    let item_chunk = "a".repeat(1000);
    let items_count = 500; // ~500 KB total
    let mut payload_500kb = Vec::new();
    for i in 0..items_count {
        payload_500kb.push(serde_json::json!({
            "id": format!("item-{i}"),
            "payload": item_chunk,
        }));
    }
    let body_500kb = serde_json::to_vec(&payload_500kb).expect("serialize 500kb");
    assert!(body_500kb.len() >= 500_000 && body_500kb.len() < 1_000_000);

    let resp = client
        .put(format!("http://{addr}/api/collections/squads"))
        .header("Authorization", format!("Bearer {AUTH_TOKEN}"))
        .header("Content-Type", "application/json")
        .body(body_500kb)
        .send()
        .await
        .expect("send 500kb request");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "PUT 500 KB collection must succeed under 1 MiB FLEET_MAX_BODY limit"
    );

    // 3. Authenticated PUT with 2 MiB payload -> 413 Payload Too Large
    let big_chunk = "b".repeat(2000);
    let big_items_count = 1100; // ~2.2 MiB total
    let mut payload_2mb = Vec::new();
    for i in 0..big_items_count {
        payload_2mb.push(serde_json::json!({
            "id": format!("big-{i}"),
            "payload": big_chunk,
        }));
    }
    let body_2mb = serde_json::to_vec(&payload_2mb).expect("serialize 2mb");
    assert!(body_2mb.len() > 1_048_576);

    let resp = client
        .put(format!("http://{addr}/api/collections/squads"))
        .header("Authorization", format!("Bearer {AUTH_TOKEN}"))
        .header("Content-Type", "application/json")
        .body(body_2mb)
        .send()
        .await
        .expect("send 2mb request");
    assert_eq!(
        resp.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "PUT 2 MiB collection must return 413 Payload Too Large"
    );
}

#[tokio::test]
async fn test_all_ported_endpoints_match_go_contract_shape() {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let state = create_test_environment(&tmp);
    let addr = spawn_test_gateway(state).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("client");

    let auth_header = format!("Bearer {AUTH_TOKEN}");

    // Define volatile keys based on VOLATILE.md
    let mut ignored_keys = HashSet::new();
    ignored_keys.insert("timestamp");
    ignored_keys.insert("createdAt");
    ignored_keys.insert("generatedAt");
    ignored_keys.insert("date");
    ignored_keys.insert("uptime");
    ignored_keys.insert("duration");
    ignored_keys.insert("pid");
    ignored_keys.insert("activeProcesses");
    ignored_keys.insert("active_threads");
    ignored_keys.insert("memoryUsedMB");
    ignored_keys.insert("usedMB");
    ignored_keys.insert("percent");
    ignored_keys.insert("memory_db_mb");
    ignored_keys.insert("memory");
    ignored_keys.insert("id");
    ignored_keys.insert("auditId");
    ignored_keys.insert("checksum");
    ignored_keys.insert("url");
    ignored_keys.insert("dataUrl");
    ignored_keys.insert("detail");

    // Nullable fields documented in SPEC-013 §Model Data
    let mut nullable_keys = HashSet::new();
    nullable_keys.insert("systemUptime");
    nullable_keys.insert("gatewayLatencyMs");
    nullable_keys.insert("activeWorkers");
    nullable_keys.insert("gateway_latency_ms");
    nullable_keys.insert("isolation_mode");
    nullable_keys.insert("memory_db_mb");
    nullable_keys.insert("latencyMs");
    nullable_keys.insert("active_threads");
    nullable_keys.insert("treasuryBurnRate");
    nullable_keys.insert("requiresApproval");
    nullable_keys.insert("stderr");
    nullable_keys.insert("error");
    nullable_keys.insert("auditId");
    nullable_keys.insert("memoryUsedMB");

    // Optional fields that may be omitted on success or depending on state
    let mut optional_keys = HashSet::new();
    optional_keys.insert("error");
    optional_keys.insert("stderr");
    optional_keys.insert("requiresApproval");
    optional_keys.insert("requires_approval");
    optional_keys.insert("suggestedActions");

    // Test 1: GET /api/fleet/metrics
    {
        let resp = client
            .get(format!("http://{addr}/api/fleet/metrics"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/fleet/metrics");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_fleet_metrics.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "fleet_metrics",
        );
    }

    // Test 2: POST /api/fleet/deck-bell
    {
        let resp = client
            .post(format!("http://{addr}/api/fleet/deck-bell"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("POST /api/fleet/deck-bell");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("POST_api_fleet_deck_bell.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "deck_bell",
        );
    }

    // Test 3: GET /api/collections/squads (seed data from store)
    {
        let resp = client
            .get(format!("http://{addr}/api/collections/squads"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/collections/squads");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_collections_squads.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "get_collections",
        );
    }

    // Test 4: PUT /api/collections/squads
    {
        let put_body = serde_json::json!([{
            "id": "squad-dev",
            "name": "Core Software Delivery Squad"
        }]);
        let resp = client
            .put(format!("http://{addr}/api/collections/squads"))
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&put_body)
            .send()
            .await
            .expect("PUT /api/collections/squads");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("PUT_api_collections_squads.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "put_collections",
        );
    }

    // Test 5: GET /api/system/metrics
    {
        let resp = client
            .get(format!("http://{addr}/api/system/metrics"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/system/metrics");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_system_metrics.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "system_metrics",
        );
    }

    // Test 6: GET /api/network/qrcode?url=http://127.0.0.1:9090
    {
        let resp = client
            .get(format!(
                "http://{addr}/api/network/qrcode?url=http://127.0.0.1:9090"
            ))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/network/qrcode");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_network_qrcode.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "network_qrcode",
        );
    }

    // Test 7: GET /api/system/executive-briefing
    {
        let resp = client
            .get(format!("http://{addr}/api/system/executive-briefing"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/system/executive-briefing");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_system_executive_briefing.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "executive_briefing",
        );
    }

    // Test 8: GET /api/providers/harbor
    {
        let resp = client
            .get(format!("http://{addr}/api/providers/harbor"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/providers/harbor");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_providers_harbor.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "providers_harbor",
        );
    }

    // Test 9: GET /api/diagnostics
    {
        let resp = client
            .get(format!("http://{addr}/api/diagnostics"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/diagnostics");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_diagnostics.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "diagnostics",
        );
    }

    // Test 10: POST /api/snapshots
    {
        let snap_body = serde_json::json!({
            "label": "Genesis Commissioning State"
        });
        let resp = client
            .post(format!("http://{addr}/api/snapshots"))
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&snap_body)
            .send()
            .await
            .expect("POST /api/snapshots");
        assert_eq!(resp.status(), StatusCode::CREATED);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("POST_api_snapshots.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "post_snapshots",
        );
    }

    // Test 11: GET /api/snapshots
    {
        let resp = client
            .get(format!("http://{addr}/api/snapshots"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/snapshots");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_snapshots.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "get_snapshots",
        );
    }

    // Test 12: GET /api/engine/processes
    {
        let resp = client
            .get(format!("http://{addr}/api/engine/processes"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/engine/processes");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_engine_processes.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "engine_processes",
        );
    }

    // Test 13: POST /api/terminal/exec
    {
        let term_body = serde_json::json!({
            "command": "git status"
        });
        let resp = client
            .post(format!("http://{addr}/api/terminal/exec"))
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&term_body)
            .send()
            .await
            .expect("POST /api/terminal/exec");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("POST_api_terminal_exec.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "terminal_exec",
        );
    }

    // Test 14: GET /api/fleet/policies
    {
        let resp = client
            .get(format!("http://{addr}/api/fleet/policies"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/fleet/policies");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_fleet_policies.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "fleet_policies",
        );
    }

    // Test 15: POST /api/chat/quartermaster (deterministic report status)
    {
        let chat_body = serde_json::json!({
            "message": "report status"
        });
        let resp = client
            .post(format!("http://{addr}/api/chat/quartermaster"))
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&chat_body)
            .send()
            .await
            .expect("POST /api/chat/quartermaster");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("POST_api_chat_quartermaster.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "chat_quartermaster",
        );
    }

    // Test 16: GET /api/system/network
    {
        let resp = client
            .get(format!("http://{addr}/api/system/network"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/system/network");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_system_network.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "system_network",
        );
    }

    // Test 17: GET /api/project/git-status?path=.
    {
        let resp = client
            .get(format!("http://{addr}/api/project/git-status?path=."))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/project/git-status");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_project_git_status.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "project_git_status",
        );
    }

    // Test 18: GET /api/v1/audit/trail
    {
        let resp = client
            .get(format!("http://{addr}/api/v1/audit/trail"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/v1/audit/trail");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        let fixture = load_fixture("GET_api_v1_audit_trail.json");
        assert_same_shape(
            &rust_json,
            &fixture,
            &ignored_keys,
            &nullable_keys,
            &optional_keys,
            "audit_trail",
        );
    }

    // Test 19: GET /api/fleet/seed
    {
        let resp = client
            .get(format!("http://{addr}/api/fleet/seed"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .expect("GET /api/fleet/seed");
        assert_eq!(resp.status(), StatusCode::OK);
        let rust_json: Value = resp.json().await.expect("json");
        // Verify seed response has status "ok" and 15 camelCase collections
        assert_eq!(rust_json["status"], "ok");
        let seed_obj = rust_json["seed"].as_object().expect("seed object");
        let expected_collections = [
            "squads",
            "ships",
            "crew",
            "trainingSkills",
            "globalSteering",
            "steeringDirectives",
            "trainingHooks",
            "quests",
            "artifacts",
            "approvals",
            "logbook",
            "treasuryLedger",
            "notifications",
            "journalSessions",
            "chatMessages",
        ];
        for col in expected_collections {
            assert!(
                seed_obj.contains_key(col),
                "Seed response missing collection '{col}'"
            );
        }
    }
}
