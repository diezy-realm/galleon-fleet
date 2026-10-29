//! End-to-End tests for configuration mutation, atomic disk persistence,
//! and live drift detection on a real filesystem without mock stores.
//!
//! Verifies:
//! 1. Physical temporary directory setup (`tempfile::tempdir()`) and valid `config.toml` file write.
//! 2. Configuration mutation via real RFC 6902 JSON Patch operations over the `/api/config` endpoint.
//! 3. Atomic persistence to disk via `persist_and_swap`, ensuring consistent on-disk TOML and in-memory swap.
//! 4. Live external modification on the physical disk file, followed by drift detection using `try_compute_drift`
//!    producing an accurate `ConfigDriftSummary`.
//! 5. Zero mock stores: all reads and writes touch the physical operating system filesystem.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use galleon_config::multi_agent::MemoryBackendKind;
use galleon_config::schema::{
    AliasedAgentConfig, Config, ModelProviderConfig, OllamaModelProviderConfig, RiskProfileConfig,
    RuntimeProfileConfig,
};
use galleon_gateway::{
    AppState, ConfigDriftSummary, build_gateway_router, persist_and_swap, try_compute_drift,
};
use reqwest::StatusCode;

/// Helper: initializes a real physical temporary directory with a valid `config.toml` on disk.
fn setup_physical_config(tmp: &tempfile::TempDir) -> Config {
    let data_dir = tmp.path().join("data");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&data_dir).expect("create test data directory");
    std::fs::create_dir_all(&workspace).expect("create test workspace directory");

    let config_path = tmp.path().join("config.toml");

    let mut config = Config {
        data_dir,
        config_path: config_path.clone(),
        ..Config::default()
    };
    config.gateway.port = 8080;
    config.gateway.host = "127.0.0.1".into();

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

    // Write physical file to disk using real filesystem
    let toml_content = toml::to_string_pretty(&config).expect("serialize config to toml");
    std::fs::write(&config_path, toml_content).expect("write initial physical config.toml");

    config
}

#[tokio::test]
async fn test_api_config_real_fs_mutation_atomic_persistence_and_swap() {
    // 1. Siapkan direktori fisik sementara menggunakan tempfile::tempdir() dan tulis file konfigurasi valid config.toml
    let tmp = tempfile::TempDir::new().expect("create physical tempdir");
    let config = setup_physical_config(&tmp);
    let config_path = config.config_path.clone();

    // Pastikan berkas fisik benar-benar ada di disk sebelum pengujian dimulai
    assert!(
        config_path.exists(),
        "config.toml must physically exist on disk"
    );

    let state = AppState::from_config(config);

    // Jalankan server Axum pada ephemeral TCP socket nyata (127.0.0.1:0)
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral TCP listener");
    let local_addr = listener.local_addr().expect("local socket address");
    let app = build_gateway_router(state.clone());
    let _server = galleon_spawn::spawn!(async move {
        axum::serve(listener, app)
            .await
            .expect("axum gateway serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");
    let base_url = format!("http://{local_addr}");

    // 2. Jalankan mutasi konfigurasi melalui operasi JSON Patch nyata (RFC 6902) via API /api/config
    let patch_ops = serde_json::json!([
        {
            "op": "replace",
            "path": "/gateway/port",
            "value": 9099
        },
        {
            "op": "replace",
            "path": "/gateway/host",
            "value": "127.0.0.2"
        }
    ]);

    let patch_resp = client
        .patch(format!("{base_url}/api/config"))
        .header("Content-Type", "application/json")
        .json(&patch_ops)
        .send()
        .await
        .expect("PATCH /api/config request");

    assert_eq!(
        patch_resp.status(),
        StatusCode::OK,
        "PATCH /api/config must succeed with 200 OK"
    );

    let patch_body: serde_json::Value = patch_resp.json().await.expect("parse patch response JSON");
    assert_eq!(
        patch_body["saved"], true,
        "response must indicate saved: true"
    );
    assert_eq!(
        patch_body["results"].as_array().map(|a| a.len()),
        Some(2),
        "two patch operations must be recorded in results"
    );

    // 3. Verifikasi proses persist_and_swap menulis berkas fisik ke disk secara atomik
    // dan isi berkas terbaca konsisten setelah swap.
    let on_disk_raw = std::fs::read_to_string(&config_path)
        .expect("read config.toml directly from physical filesystem");
    assert!(
        on_disk_raw.contains("9099"),
        "physical file on disk must contain the mutated port 9099"
    );
    assert!(
        on_disk_raw.contains("127.0.0.2"),
        "physical file on disk must contain the mutated gateway host"
    );

    // Parse ulang dari disk untuk memvalidasi integritas data fisik
    let on_disk_parsed: Config =
        toml::from_str(&on_disk_raw).expect("parse on-disk config.toml into Config");
    assert_eq!(
        on_disk_parsed.gateway.port, 9099,
        "on-disk parsed gateway.port must match patch value"
    );
    assert_eq!(
        on_disk_parsed.gateway.host, "127.0.0.2",
        "on-disk parsed gateway.host must match patch value"
    );

    // Verifikasi in-memory state telah ter-swap secara konsisten
    {
        let live = state.config.read();
        assert_eq!(
            live.gateway.port, 9099,
            "in-memory gateway.port must be updated after swap"
        );
        assert_eq!(
            live.gateway.host, "127.0.0.2",
            "in-memory gateway.host must be updated after swap"
        );
    }
    assert!(
        state.pending_reload.load(Ordering::Relaxed),
        "pending_reload flag must be asserted after swap"
    );
}

#[tokio::test]
async fn test_api_config_persist_and_swap_direct_atomic_disk_write() {
    let tmp = tempfile::TempDir::new().expect("create physical tempdir");
    let config = setup_physical_config(&tmp);
    let config_path = config.config_path.clone();
    let state = AppState::from_config(config);

    // Verifikasi pemanggilan persist_and_swap secara langsung dengan write lock
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let mut working = state.config.read().clone();
    working.gateway.port = 9250;
    working.gateway.host = "127.0.0.3".into();
    working.mark_dirty("gateway.port");
    working.mark_dirty("gateway.host");

    persist_and_swap(&state, working, &guard)
        .await
        .expect("persist_and_swap direct call must succeed");

    // Verifikasi file pada filesystem nyata
    let disk_content =
        std::fs::read_to_string(&config_path).expect("read physical file after direct swap");
    let disk_config: Config =
        toml::from_str(&disk_content).expect("parse physical file into Config");

    assert_eq!(
        disk_config.gateway.port, 9250,
        "disk config port must be 9250"
    );
    assert_eq!(
        disk_config.gateway.host, "127.0.0.3",
        "disk config host must be 127.0.0.3"
    );

    // Verifikasi state in-memory
    assert_eq!(state.config.read().gateway.port, 9250);
    assert_eq!(state.config.read().gateway.host, "127.0.0.3");
}

#[tokio::test]
async fn test_api_config_external_modification_live_drift_detection() {
    let tmp = tempfile::TempDir::new().expect("create physical tempdir");
    let config = setup_physical_config(&tmp);
    let config_path = config.config_path.clone();
    let state = AppState::from_config(config);

    // Jalankan server Axum untuk endpoint HTTP
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral TCP listener");
    let local_addr = listener.local_addr().expect("local socket address");
    let app = build_gateway_router(state.clone());
    let _server = galleon_spawn::spawn!(async move {
        axum::serve(listener, app)
            .await
            .expect("axum gateway serve");
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");
    let base_url = format!("http://{local_addr}");

    // Sebelum modifikasi eksternal: verifikasi drift adalah 0 (tidak ada pergeseran)
    let cfg = state.config.read().clone();
    let initial_drift: ConfigDriftSummary = try_compute_drift(&cfg)
        .await
        .expect("try_compute_drift must succeed");
    assert!(
        initial_drift.is_empty(),
        "initial config must show zero drift"
    );

    // 4. Lakukan modifikasi eksternal langsung pada berkas config.toml di disk
    let current_disk = std::fs::read_to_string(&config_path).expect("read current disk config");
    // Modifikasi port secara eksternal dari 8080 menjadi 7777 langsung di file fisik
    let modified_disk = current_disk.replace("port = 8080", "port = 7777");
    assert_ne!(
        current_disk, modified_disk,
        "file content must be altered by external modification"
    );
    std::fs::write(&config_path, &modified_disk)
        .expect("write external modification directly to physical disk");

    // Panggil fungsi try_compute_drift, dan verifikasi ringkasan drift (ConfigDriftSummary)
    let cfg = state.config.read().clone();
    let drift_summary: ConfigDriftSummary = try_compute_drift(&cfg)
        .await
        .expect("try_compute_drift after external edit");

    assert!(
        !drift_summary.is_empty(),
        "drift summary must detect the external modification"
    );

    let port_drift = drift_summary
        .iter()
        .find(|d| d.path == "gateway.port")
        .expect("drift summary must contain entry for gateway.port");

    assert_eq!(port_drift.path, "gateway.port");
    assert!(port_drift.drifted, "drifted must be true");
    assert!(!port_drift.secret, "gateway.port is not a secret");
    assert_eq!(
        port_drift.in_memory_value,
        Some(serde_json::Value::String("8080".to_string())),
        "in_memory_value must be 8080"
    );
    assert_eq!(
        port_drift.on_disk_value,
        Some(serde_json::Value::String("7777".to_string())),
        "on_disk_value must be 7777"
    );

    // Verifikasi endpoint HTTP GET /api/config/drift mengembalikan summary yang sama
    let drift_api_resp = client
        .get(format!("{base_url}/api/config/drift"))
        .send()
        .await
        .expect("GET /api/config/drift");
    assert_eq!(drift_api_resp.status(), StatusCode::OK);
    let drift_api_body: serde_json::Value = drift_api_resp.json().await.expect("drift JSON");
    let api_drifted = drift_api_body["drifted"].as_array().expect("drifted array");
    assert!(
        api_drifted.iter().any(|d| d["path"] == "gateway.port"
            && d["in_memory_value"] == "8080"
            && d["on_disk_value"] == "7777"),
        "API /api/config/drift must reflect the accurate drift summary"
    );

    // Verifikasi bahwa PATCH yang menyentuh field terdampak drift tanpa override DITOLAK (409 Conflict)
    let conflicting_patch = serde_json::json!([
        {
            "op": "replace",
            "path": "/gateway/port",
            "value": 9999
        }
    ]);
    let conflict_resp = client
        .patch(format!("{base_url}/api/config"))
        .header("Content-Type", "application/json")
        .json(&conflicting_patch)
        .send()
        .await
        .expect("PATCH /api/config with conflict");

    assert_eq!(
        conflict_resp.status(),
        StatusCode::CONFLICT,
        "PATCH without override must be rejected with 409 Conflict when drift exists"
    );
    let conflict_json: serde_json::Value = conflict_resp.json().await.expect("conflict JSON");
    assert_eq!(
        conflict_json["code"], "config_changed_externally",
        "error code must be config_changed_externally"
    );

    // Verifikasi bahwa PATCH dengan header X-ClawCrew-Override-Drift: true berhasil meng-override
    let override_resp = client
        .patch(format!("{base_url}/api/config"))
        .header("Content-Type", "application/json")
        .header("X-ClawCrew-Override-Drift", "true")
        .json(&conflicting_patch)
        .send()
        .await
        .expect("PATCH /api/config with override header");

    assert_eq!(
        override_resp.status(),
        StatusCode::OK,
        "PATCH with override header must succeed with 200 OK"
    );

    // Setelah override berhasil diaplikasikan dan di-swap:
    // 1. Nilai di disk sekarang 9999
    let post_override_disk =
        std::fs::read_to_string(&config_path).expect("read disk after override");
    assert!(post_override_disk.contains("9999"));

    // 2. Nilai in-memory sekarang 9999
    assert_eq!(state.config.read().gateway.port, 9999);

    // 3. Drift sekarang bersih (0 pergeseran)
    let cfg = state.config.read().clone();
    let post_override_drift: ConfigDriftSummary = try_compute_drift(&cfg)
        .await
        .expect("compute drift after override");
    assert!(
        post_override_drift.is_empty(),
        "drift must be fully resolved after successful override patch"
    );
}
