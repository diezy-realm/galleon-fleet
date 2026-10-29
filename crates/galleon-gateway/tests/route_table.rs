//! Integration tests for Gateway route table and authentication matrix (SPEC-012, TASK-064).
//!
//! Acceptance criteria:
//! - Router dijalankan di 127.0.0.1:0 (ephemeral TCP listener nyata)
//! - Untuk setiap baris consumer-inventory.md berstatus Rust:
//!   - Tanpa token -> 401 Unauthorized kecuali publik
//! - insta::assert_yaml_snapshot! merekam daftar (method, path, status)

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::Method;
use galleon_config::pairing::PairingGuard;
use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::{AppState, build_gateway_router, live_pairing_code_policy};
use serde::{Deserialize, Serialize};

fn load_consumer_inventory() -> String {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let inventory_path =
        manifest_dir.join("../../standard-docs/tech-standart/api-contracts/consumer-inventory.md");
    std::fs::read_to_string(&inventory_path).unwrap_or_else(|err| {
        panic!(
            "failed to read consumer-inventory.md at {}: {err}",
            inventory_path.display()
        )
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteStatus {
    pub method: String,
    pub path: String,
    pub status: u16,
}

#[derive(Debug, Clone)]
struct InventoryRoute {
    method: String,
    path: String,
    auth: String,
}

fn parse_rust_routes_from_inventory(markdown: &str) -> Vec<InventoryRoute> {
    let mut routes = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || trimmed.starts_with("|---") || trimmed.starts_with("| **") {
            continue;
        }
        let parts: Vec<&str> = trimmed
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() < 6 {
            continue;
        }
        let method_col = parts[0];
        let path_col = parts[1];
        let impl_col = parts[3];
        let auth_col = parts[5];

        if method_col == "Method" {
            continue;
        }

        // Baris berstatus Rust di consumer-inventory.md (bagian rute Rust gateway aktif)
        if impl_col.starts_with("Rust (`") {
            let path = path_col.trim_matches('`').to_string();
            let methods: Vec<&str> = method_col.split(',').map(str::trim).collect();
            for m in methods {
                routes.push(InventoryRoute {
                    method: m.to_string(),
                    path: path.clone(),
                    auth: auth_col.to_string(),
                });
            }
        }
    }
    routes
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

#[tokio::test]
async fn test_gateway_route_table_snapshot_and_401_matrix() {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let config = create_test_config(&tmp);
    let mut state = AppState::from_config(config);

    // Aktifkan autentikasi wajib pairing (require_pairing = true)
    state.pairing = Arc::new(PairingGuard::new(
        true,
        &[],
        live_pairing_code_policy(&state),
    ));

    // Jalankan router pada ephemeral TCP socket nyata 127.0.0.1:0
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral TCP listener");
    let local_addr = listener.local_addr().expect("local socket address");
    let app = build_gateway_router(state.clone());

    let _server = galleon_spawn::spawn!(async move {
        let make_service = app.into_make_service_with_connect_info::<SocketAddr>();
        axum::serve(listener, make_service)
            .await
            .expect("axum gateway serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");

    let inventory_raw = load_consumer_inventory();
    let inventory_routes = parse_rust_routes_from_inventory(&inventory_raw);
    assert!(
        !inventory_routes.is_empty(),
        "Harus ada rute berstatus Rust yang diparsing dari consumer-inventory.md"
    );

    let mut snapshot_results = Vec::new();

    for route in &inventory_routes {
        let method = match route.method.as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PATCH" => Method::PATCH,
            "DELETE" => Method::DELETE,
            "PUT" => Method::PUT,
            "OPTIONS" => Method::OPTIONS,
            other => panic!("Unsupported HTTP method: {other}"),
        };

        let request_path = if route.path.ends_with("/catalog/models") {
            format!("{}?provider=default", route.path)
        } else {
            route.path.clone()
        };

        let url = format!("http://{local_addr}{request_path}");
        let mut req = client.request(method.clone(), &url);
        if route.path.starts_with("/ws") {
            req = req
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==");
        } else if matches!(method, Method::POST | Method::PATCH | Method::PUT) {
            req = req.header("Content-Type", "application/json").body("{}");
        }

        let resp = req
            .send()
            .await
            .unwrap_or_else(|e| panic!("Failed request to {} {}: {e}", route.method, route.path));

        let status_code = resp.status().as_u16();

        // Verifikasi matriks autentikasi tanpa token:
        // Setiap rute non-publik terbukti 401 Unauthorized tanpa token
        let is_public = route.auth.eq_ignore_ascii_case("public");
        if is_public {
            assert_ne!(
                status_code, 401,
                "Rute publik {} {} tidak boleh mengembalikan 401 tanpa token",
                route.method, route.path
            );
        } else if route.method == "OPTIONS" {
            // OPTIONS request untuk CORS / schema preflight
            assert_ne!(
                status_code, 401,
                "OPTIONS {} {} adalah preflight schema/CORS",
                route.method, route.path
            );
        } else {
            assert_eq!(
                status_code, 401,
                "Rute non-publik {} {} wajib mengembalikan 401 Unauthorized tanpa token, namun menerima status {}",
                route.method, route.path, status_code
            );
        }

        snapshot_results.push(RouteStatus {
            method: route.method.clone(),
            path: route.path.clone(),
            status: status_code,
        });
    }

    // Urutkan snapshot berdasarkan method lalu path agar selalu deterministik
    snapshot_results.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.method.cmp(&b.method)));

    insta::assert_yaml_snapshot!("gateway_route_table_matrix", snapshot_results);
}
