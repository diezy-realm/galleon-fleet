use super::*;
use crate::{GatewayRateLimiter, IdempotencyStore, nodes};
use async_trait::async_trait;
use axum::http::StatusCode;
use axum::{
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use clawcrew_config::api_error::ConfigApiCode;
use clawcrew_config::pairing::PairingGuard;
use clawcrew_providers::ModelProvider;
use http_body_util::BodyExt;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn invalid_etag_value_is_omitted_instead_of_panicking() {
    let mut response = StatusCode::OK.into_response();
    insert_etag(&mut response, "invalid\nheader");
    assert!(!response.headers().contains_key(header::ETAG));
}

// dirty_entry_for / CascadeReport::dirty_paths tests live in
// clawcrew_config::alias_refs — single source of truth (the gateway and CLI
// both consume the promoted helper).

#[derive(Default)]
struct MockModelProvider;

#[async_trait]
impl ModelProvider for MockModelProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: Option<f64>,
    ) -> anyhow::Result<String> {
        Ok("ok".into())
    }
}

impl ::clawcrew_api::attribution::Attributable for MockModelProvider {
    fn role(&self) -> ::clawcrew_api::attribution::Role {
        ::clawcrew_api::attribution::Role::Provider(
            ::clawcrew_api::attribution::ProviderKind::Model(
                ::clawcrew_api::attribution::ModelProviderKind::Custom,
            ),
        )
    }

    fn alias(&self) -> &str {
        "MockModelProvider"
    }
}

fn temp_config(tmp: &tempfile::TempDir) -> clawcrew_config::schema::Config {
    let data_dir = tmp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    clawcrew_config::schema::Config {
        config_path: tmp.path().join("config.toml"),
        data_dir,
        ..Default::default()
    }
}

pub(crate) fn test_state(config: clawcrew_config::schema::Config) -> AppState {
    let memory: Arc<dyn clawcrew_memory::Memory> =
        Arc::new(clawcrew_memory::NoneMemory::new("api-config-test"));
    AppState {
        config: Arc::new(RwLock::new(config)),
        config_write_lock: Arc::new(tokio::sync::Mutex::new(())),
        model_provider: Arc::new(MockModelProvider),
        model: "test-model".into(),
        temperature: None,
        mem: memory.clone(),
        memory_strategy: Arc::new(clawcrew_memory::DefaultMemoryStrategy::with_config(
            memory,
            clawcrew_config::schema::MemoryConfig::default(),
            std::path::PathBuf::new(),
        )),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(
            false,
            &[],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        )),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        auth_limiter: Arc::new(crate::auth_rate_limit::AuthRateLimiter::new()),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        #[cfg(feature = "channel-whatsapp-cloud")]
        whatsapp: std::collections::HashMap::new(),
        #[cfg(feature = "channel-whatsapp-cloud")]
        whatsapp_app_secret: std::collections::HashMap::new(),
        #[cfg(feature = "channel-linq")]
        linq: std::collections::HashMap::new(),
        #[cfg(feature = "channel-linq")]
        linq_signing_secrets: std::collections::HashMap::new(),
        #[cfg(feature = "channel-nextcloud")]
        nextcloud_talk: std::collections::HashMap::new(),
        #[cfg(feature = "channel-nextcloud")]
        nextcloud_talk_webhook_secret: std::collections::HashMap::new(),
        #[cfg(feature = "channel-email")]
        gmail_push: None,
        observer: Arc::new(clawcrew_api::observability_traits::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_by_agent: Arc::new(std::collections::HashMap::new()),
        cost_tracker: None,
        event_tx: tokio::sync::broadcast::channel(16).0,
        event_buffer: Arc::new(crate::sse::EventBuffer::new(16)),
        shutdown_tx: tokio::sync::watch::channel(false).0,
        reload_tx: None,
        node_registry: Arc::new(nodes::NodeRegistry::new(16)),
        mdns_peer_registry: nodes::mdns::MdnsPeerRegistry::default(),
        path_prefix: String::new(),
        web_dist_dir: None,
        session_backend: None,
        session_queue: Arc::new(crate::session_queue::SessionActorQueue::new(8, 30, 600)),
        device_registry: None,
        pending_pairings: None,
        canvas_store: galleon_tools::canvas::CanvasStore::new(),
        #[cfg(feature = "webauthn")]
        webauthn: None,
        cancel_tokens: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        pending_reload: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        tui_registry: None,
        sop_runner: None,
        diagnostics: None,
        quickstart: None,
        integration_catalog: None,
        personality_runner: None,
        skills_runner: None,
        acp_runner: None,
        control_plane: None,
    }
}

async fn response_json(response: Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body")
        .to_bytes();
    let json = serde_json::from_slice(&body).expect("valid json response");
    (status, json)
}

// Every config in this module must come from `temp_config`: the success-path
// tests below fall through into real persistence (`persist_and_swap` ->
// `save_dirty`), and a bare `Config::default()` would write the developer's
// live `~/.clawcrew/config.toml`.

#[tokio::test]
async fn prop_put_does_not_materialize_resource_keyed_rate_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let state = test_state(temp_config(&tmp));
    let (status, json) = response_json(
        handle_prop_put(
            State(state.clone()),
            HeaderMap::new(),
            axum::Json(PropPutBody {
                path: "cost.rates.providers.models.openai.gpt-5.input_per_mtok".to_string(),
                value: serde_json::json!(1.5),
                comment: None,
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["code"], "path_not_found");
    assert!(
        state
            .config
            .read()
            .cost
            .rates
            .providers
            .models
            .openai
            .is_empty()
    );
}

#[tokio::test]
async fn prop_put_on_dotted_resource_id_does_not_plant_phantom_sibling() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = temp_config(&tmp);
    config
        .create_map_key("cost.rates.providers.models.openai", "gpt-4.1")
        .unwrap();
    let state = test_state(config);
    let (status, _json) = response_json(
        handle_prop_put(
            State(state.clone()),
            HeaderMap::new(),
            axum::Json(PropPutBody {
                path: "cost.rates.providers.models.openai.gpt-4.1.input_per_mtok".to_string(),
                value: serde_json::json!(1.5),
                comment: None,
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    // In-memory, not the saved TOML: a dotted map key never reaches disk
    // through the incremental dirty-path write, so a disk assertion would be
    // vacuous in both directions.
    assert_eq!(
        state
            .config
            .read()
            .get_map_keys("cost.rates.providers.models.openai")
            .expect("known section"),
        vec!["gpt-4.1".to_string()],
    );
}

#[tokio::test]
async fn prop_put_still_materializes_operator_chosen_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let state = test_state(temp_config(&tmp));
    // Secret path: the response envelope is `{path, populated}`, not `value`.
    let (status, _json) = response_json(
        handle_prop_put(
            State(state.clone()),
            HeaderMap::new(),
            axum::Json(PropPutBody {
                path: "channels.telegram.newbot.bot_token".to_string(),
                value: serde_json::json!("tok"),
                comment: None,
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(state.config.read().channels.telegram.contains_key("newbot"));
}

#[tokio::test]
async fn authenticated_patch_route_rejects_masked_or_empty_secret_values_without_mutation() {
    use axum::body::Body;
    use axum::routing::patch;
    use tower::ServiceExt;

    let test_auth = "test-token";
    for op in ["add", "replace"] {
        for value in [clawcrew_config::traits::MASKED_SECRET, "****", ""] {
            let tmp = tempfile::tempdir().unwrap();
            let mut config = temp_config(&tmp);
            config.gateway.webhook_secret = Some("original-secret".into());
            config.save().await.unwrap();
            let disk_before = tokio::fs::read(&config.config_path).await.unwrap();
            let live_before = config.gateway.webhook_secret.clone();
            let mut state = test_state(config);
            state.pairing = Arc::new(PairingGuard::new(
                true,
                &[test_auth.to_string()],
                clawcrew_config::pairing::PairingCodePolicy::default(),
            ));
            let app = axum::Router::new()
                .route("/api/config", patch(handle_patch))
                .with_state(state.clone());

            let (status, json) = response_json(
                app.oneshot(
                    axum::http::Request::builder()
                        .method("PATCH")
                        .uri("/api/config")
                        .header(header::AUTHORIZATION, format!("Bearer {test_auth}"))
                        .header(header::CONTENT_TYPE, "application/json")
                        .header("x-clawcrew-override-drift", "true")
                        .body(Body::from(
                            serde_json::json!([{
                                "op": op,
                                "path": "/gateway/webhook_secret",
                                "value": value,
                            }])
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap(),
            )
            .await;

            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(json["code"], "validation_failed");
            assert_eq!(json["path"], "gateway.webhook_secret");
            assert_eq!(json["op_index"], 0);
            let disk_path = state.config.read().config_path.clone();
            let disk_after = tokio::fs::read(disk_path).await.unwrap();
            assert_eq!(
                disk_after, disk_before,
                "rejected {op} value {value:?} must not change disk"
            );
            assert_eq!(
                state.config.read().gateway.webhook_secret,
                live_before,
                "rejected {op} value {value:?} must not change live config"
            );
            assert!(
                !state
                    .pending_reload
                    .load(std::sync::atomic::Ordering::Relaxed)
            );
        }
    }
}

#[tokio::test]
async fn migration_preserves_runtime_paths_and_publishes_schema_version() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("selected-config.toml");
    let data_dir = tmp.path().join("selected-data");
    let legacy = clawcrew_config::migration::generate(
        2,
        &clawcrew_config::migration::GenerateOptions::default(),
    )
    .unwrap();
    tokio::fs::write(&config_path, legacy).await.unwrap();
    let state = test_state(clawcrew_config::schema::Config {
        config_path: config_path.clone(),
        data_dir: data_dir.clone(),
        ..Default::default()
    });

    let (status, json) =
        response_json(handle_migrate(State(state.clone()), HeaderMap::new()).await).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["migrated"], true);
    assert_eq!(
        state.config.read().config_path,
        config_path,
        "migration must retain the runtime-selected config path"
    );
    assert_eq!(
        state.config.read().data_dir,
        data_dir,
        "migration must retain the runtime-selected data directory"
    );
    assert!(
        state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );

    let raw = tokio::fs::read_to_string(&config_path).await.unwrap();
    let disk_value: toml::Value = toml::from_str(&raw).unwrap();
    let live_schema_version = state.config.read().schema_version;
    assert_eq!(
        live_schema_version,
        clawcrew_config::migration::CURRENT_SCHEMA_VERSION,
        "migration must publish the upgraded schema version live"
    );
    assert_eq!(
        disk_value
            .get("schema_version")
            .and_then(toml::Value::as_integer),
        Some(i64::from(
            clawcrew_config::migration::CURRENT_SCHEMA_VERSION
        ))
    );
}

/// Regression test for the lost-update race `persist_and_swap` callers
/// must not reintroduce: a handler used to read-clone config, save the
/// clone to disk (an `.await` with no lock held), then swap the clone
/// back over live config wholesale. A write landed on `state.config`
/// during that save window was silently erased by the swap.
///
/// Drives `handle_prop_put` (a real `persist_and_swap` caller) with the
/// witness lock held externally. A single Pending poll wouldn't
/// distinguish "blocked on `config_write_lock`" from "transiently
/// Pending on unrelated I/O", so this polls the handler repeatedly with
/// a no-op waker while the witness stays held and asserts it never
/// completes -- proving it stays parked on the lock, not that it merely
/// yielded once. Only after the external guard is dropped does the
/// concurrent write become visible to the handler's own read, so both
/// changes land instead of one clobbering the other.
#[tokio::test]
async fn config_write_lock_serializes_prop_put_against_concurrent_writer() {
    let tmp = tempfile::tempdir().unwrap();
    let state = test_state(temp_config(&tmp));

    // Simulate another in-flight config mutation already holding the
    // witness for its own read-mutate-save-swap section.
    let held_guard = Arc::clone(&state.config_write_lock).lock_owned().await;

    let mut handler_fut = Box::pin(handle_prop_put(
        State(state.clone()),
        HeaderMap::new(),
        axum::Json(PropPutBody {
            path: "channels.telegram.newbot.bot_token".to_string(),
            value: serde_json::json!("tok"),
            comment: None,
        }),
    ));

    // Bounded, sleep-free: poll with a no-op waker 50 times while
    // `held_guard` stays live and assert Pending every time. The
    // handler must not race ahead of an externally held witness no
    // matter how many times it's polled.
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    for _ in 0..50 {
        assert!(
            std::future::Future::poll(handler_fut.as_mut(), &mut cx).is_pending(),
            "handle_prop_put must stay parked on config_write_lock \
                 acquisition for as long as another writer holds it, not \
                 race ahead to read a stale config"
        );
    }

    // Land a distinct, concurrent write directly on live config while
    // handle_prop_put is parked waiting for the lock.
    state.config.write().gateway.port = 55555;

    // Release the externally held guard so the parked handler can
    // proceed; it now reads config with the write above already applied.
    drop(held_guard);

    let response = handler_fut.await;
    assert_eq!(response.status(), StatusCode::OK);

    let live = state.config.read();
    assert_eq!(
        live.gateway.port, 55555,
        "the concurrent writer's change must survive — no lost update"
    );
    assert!(
        live.channels.telegram.contains_key("newbot"),
        "handle_prop_put's own change must also land"
    );
}

#[tokio::test]
async fn patch_add_does_not_materialize_resource_keyed_rate_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let config = temp_config(&tmp);
    // `compute_drift` reads the on-disk file, so it has to exist.
    config.save().await.unwrap();
    let state = test_state(config);
    let (status, json) = response_json(
        handle_patch(
            State(state.clone()),
            HeaderMap::new(),
            axum::Json(serde_json::json!([{
                "op": "add",
                "path": "/cost/rates/providers/models/openai/gpt-5/input_per_mtok",
                "value": 1.5
            }])),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["code"], "path_not_found");
    assert!(
        state
            .config
            .read()
            .cost
            .rates
            .providers
            .models
            .openai
            .is_empty()
    );
}

#[tokio::test]
async fn patch_allow_from_deny_all_persists_and_reloads_tool_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = temp_config(&tmp);
    config.risk_profiles.insert(
        "default".to_string(),
        clawcrew_config::schema::RiskProfileConfig {
            deny_all_tools: true,
            ..Default::default()
        },
    );
    config.save().await.unwrap();

    let state = test_state(config);
    let (status, json) = response_json(
        handle_patch(
            State(state),
            HeaderMap::new(),
            axum::Json(serde_json::json!([
                {
                    "op": "replace",
                    "path": "/risk_profiles/default/allowed_tools",
                    "value": ["shell"]
                },
                {
                    "op": "replace",
                    "path": "/risk_profiles/default/deny_all_tools",
                    "value": false
                }
            ])),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["saved"], true);

    let written = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
    let reloaded = clawcrew_config::migration::migrate_to_current(&written).unwrap();
    let profile = reloaded.risk_profiles.get("default").unwrap();
    assert_eq!(profile.allowed_tools, vec!["shell"]);
    assert!(!profile.deny_all_tools);

    let policy = clawcrew_config::policy::SecurityPolicy::from_profiles(profile, None, tmp.path());
    assert!(policy.is_tool_allowed("shell"));
    assert!(!policy.is_tool_allowed("memory_recall"));
}

#[tokio::test]
async fn delete_map_key_handler_cascades_model_provider_and_persists() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = temp_config(&tmp);
    config
        .providers
        .models
        .ensure("anthropic", "default")
        .unwrap();
    config
        .providers
        .models
        .ensure("openai", "main")
        .unwrap()
        .fallback = vec!["anthropic.default".into()];
    config.agents.insert(
        "triage".to_string(),
        clawcrew_config::schema::AliasedAgentConfig {
            classifier_provider: "anthropic.default".into(),
            ..Default::default()
        },
    );
    config.save().await.unwrap();

    let state = test_state(config);
    let (status, json) = response_json(
        handle_delete_map_key(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::extract::Query(MapKeyQuery {
                path: "providers.models.anthropic".to_string(),
                key: "default".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["path"], "providers.models.anthropic");
    assert_eq!(json["key"], "default");
    let cfg = state.config.read();
    assert!(cfg.providers.models.find("anthropic", "default").is_none());
    assert!(cfg.agents["triage"].classifier_provider.is_empty());
    assert!(
        cfg.providers
            .models
            .find("openai", "main")
            .unwrap()
            .fallback
            .is_empty()
    );
    drop(cfg);
    let written = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
    assert!(!written.contains("anthropic.default"));
}

#[tokio::test]
async fn delete_map_key_handler_refuses_model_provider_hard_ref_without_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = temp_config(&tmp);
    config
        .providers
        .models
        .ensure("anthropic", "default")
        .unwrap();
    config.agents.insert(
        "researcher".to_string(),
        clawcrew_config::schema::AliasedAgentConfig {
            model_provider: "anthropic.default".into(),
            ..Default::default()
        },
    );
    config.save().await.unwrap();

    let state = test_state(config);
    let (status, json) = response_json(
        handle_delete_map_key(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::extract::Query(MapKeyQuery {
                path: "providers.models.anthropic".to_string(),
                key: "default".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "validation_failed");
    let cfg = state.config.read();
    assert!(cfg.providers.models.find("anthropic", "default").is_some());
    assert_eq!(
        cfg.agents["researcher"].model_provider.as_str(),
        "anthropic.default"
    );
}

#[tokio::test]
async fn delete_map_key_handler_cascades_channel_and_persists() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = temp_config(&tmp);
    config.create_map_key("channels.discord", "main").unwrap();
    config.agents.insert(
        "ops".to_string(),
        clawcrew_config::schema::AliasedAgentConfig {
            channels: vec!["discord.main".into()],
            ..Default::default()
        },
    );
    config
        .escalation
        .alert_channels
        .push("discord.main".to_string());
    config.save().await.unwrap();

    let state = test_state(config);
    let (status, json) = response_json(
        handle_delete_map_key(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::extract::Query(MapKeyQuery {
                path: "channels.discord".to_string(),
                key: "main".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["path"], "channels.discord");
    assert_eq!(json["key"], "main");
    let cfg = state.config.read();
    assert!(
        !cfg.get_map_keys("channels.discord")
            .unwrap_or_default()
            .iter()
            .any(|k| k == "main")
    );
    assert!(cfg.agents["ops"].channels.is_empty());
    assert!(cfg.escalation.alert_channels.is_empty());
    drop(cfg);
    let written = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
    assert!(!written.contains("discord.main"));
}

#[tokio::test]
async fn delete_plan_rejects_unsupported_tts_provider_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = temp_config(&tmp);
    config
        .create_map_key("providers.tts.elevenlabs", "default")
        .unwrap();
    config.agents.insert(
        "voice".to_string(),
        clawcrew_config::schema::AliasedAgentConfig {
            tts_provider: "elevenlabs.default".into(),
            ..Default::default()
        },
    );

    let state = test_state(config);
    let (status, json) = response_json(
        handle_delete_plan(
            axum::extract::State(state),
            axum::http::HeaderMap::new(),
            axum::extract::Query(MapKeyQuery {
                path: "providers.tts.elevenlabs".to_string(),
                key: "default".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "op_not_supported");
    assert_eq!(json["path"], "providers.tts.elevenlabs.default");
}

#[tokio::test]
async fn delete_map_key_handler_rejects_unsupported_tts_without_raw_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = temp_config(&tmp);
    config
        .create_map_key("providers.tts.elevenlabs", "default")
        .unwrap();
    config.save().await.unwrap();

    let state = test_state(config);
    let (status, json) = response_json(
        handle_delete_map_key(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::extract::Query(MapKeyQuery {
                path: "providers.tts.elevenlabs".to_string(),
                key: "default".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "op_not_supported");
    assert!(
        state
            .config
            .read()
            .get_map_keys("providers.tts.elevenlabs")
            .unwrap_or_default()
            .iter()
            .any(|k| k == "default"),
        "unsupported provider delete must not fall back to raw deletion"
    );
}

#[test]
fn delete_cascade_resolves_custom_workspace_before_removing_entry() {
    let custom = std::path::PathBuf::from("/var/lib/zc-test/custom-victim-ws");
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.agents.insert(
        "victim".to_string(),
        clawcrew_config::schema::AliasedAgentConfig::default(),
    );
    cfg.agents.get_mut("victim").unwrap().workspace.path = Some(custom.clone());

    // While the entry exists → the custom path (what the handler captures).
    assert_eq!(cfg.agent_workspace_dir("victim"), custom);

    // After the cascade removes the entry → it falls back to the DEFAULT
    // path; that is exactly why resolution must happen before the cascade.
    clawcrew_config::alias_refs::delete_with_cascade(
        &mut cfg,
        &clawcrew_config::alias_refs::AliasKind::Agent,
        "victim",
        clawcrew_config::alias_refs::CascadePolicy::RefuseOnHard,
    )
    .expect("soft-only agent delete succeeds");
    assert!(!cfg.agents.contains_key("victim"));
    assert_ne!(
        cfg.agent_workspace_dir("victim"),
        custom,
        "after removal the custom workspace path defaults — resolve BEFORE the cascade"
    );
}

#[tokio::test]
async fn renamed_workspace_move_failure_is_surfaced() {
    // A failed workspace move during rename must surface a warning (so the
    // caller learns config/DB moved to `to` while the workspace is stranded
    // at `from`), not be swallowed as a clean success.
    let tmp = tempfile::tempdir().unwrap();
    let old_ws = tmp.path().join("from-ws");
    std::fs::create_dir_all(&old_ws).unwrap();
    // Force the move to fail: new_ws's parent is a FILE, so create_dir_all
    // and rename both fail.
    let blocker = tmp.path().join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let new_ws = blocker.join("to-ws");

    let warning = move_renamed_workspace(&old_ws, &new_ws).await;
    assert!(
        warning.is_some(),
        "a failed workspace move must surface a warning"
    );
    assert!(warning.unwrap().contains("workspace move"));
    assert!(old_ws.exists(), "source dir stays put when the move fails");

    // Nothing-to-move paths return None (no spurious warning).
    assert!(move_renamed_workspace(&old_ws, &old_ws).await.is_none());
    let missing = tmp.path().join("does-not-exist");
    assert!(move_renamed_workspace(&missing, &new_ws).await.is_none());
}

#[tokio::test]
async fn agent_rename_leaves_owned_state_put_when_persist_fails() {
    let tmp = tempfile::tempdir().unwrap();
    // Force config persistence to FAIL by making `config_path` itself a
    // directory - save_dirty's atomic write can't replace a dir. Its parent
    // (the install root) stays a real dir, so the agent-workspace creation
    // and the cron seed below still work. data_dir is separate + writable.
    let cfg_dir = tmp.path().join("config.toml");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let mut config = clawcrew_config::schema::Config {
        config_path: cfg_dir,
        data_dir: tmp.path().join("data"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    // Agent under `from` with a resolvable risk_profile + an allowed cron
    // command, so cron::add_job accepts a job tied to the agent.
    let from_agent = clawcrew_config::schema::AliasedAgentConfig {
        risk_profile: "default".into(),
        ..Default::default()
    };
    config.agents.insert("from".to_string(), from_agent);
    config
        .risk_profiles
        .entry("default".into())
        .or_default()
        .allowed_commands = vec!["echo".into()];
    config.runtime_profiles.entry("default".into()).or_default();

    // Seed an owned-state row (a cron job) under `from` - the move-probe.
    let cron_store = clawcrew_infra::cron_store::CronStore::new(&config.data_dir).unwrap();
    cron_store
        .add_job("from", "* * * * *", "echo hi")
        .expect("seed cron job");
    assert_eq!(cron_store.list_jobs_by_agent("from").unwrap().len(), 1);

    let state = crate::api::test_state(config.clone());
    let body = RenameMapKeyBody {
        path: "agents".to_string(),
        from: "from".to_string(),
        to: "to".to_string(),
    };
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let resp = rename_agent_cascade(&state, config.clone(), &body, guard).await;

    // Persist failed -> error response, not a clean rename.
    assert!(
        !resp.status().is_success(),
        "a failed config persist must surface an error"
    );
    // Owned state did NOT move: the cron job stays under `from`.
    assert_eq!(
        cron_store.list_jobs_by_agent("from").unwrap().len(),
        1,
        "cron must stay under `from` when persist fails (no premature move)"
    );
    assert!(
        cron_store.list_jobs_by_agent("to").unwrap().is_empty(),
        "cron must NOT have moved to `to` when persist fails"
    );
    // In-memory config was never swapped: still names `from`.
    assert!(state.config.read().agents.contains_key("from"));
    assert!(!state.config.read().agents.contains_key("to"));
}

#[tokio::test]
async fn agent_rename_moves_owned_state_after_successful_persist() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = clawcrew_config::schema::Config {
        config_path: tmp.path().join("config.toml"), // writable -> persist OK
        data_dir: tmp.path().join("data"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    // Agent under `from` with a resolvable risk_profile + an allowed cron
    // command, so cron::add_job accepts a job tied to the agent.
    let from_agent = clawcrew_config::schema::AliasedAgentConfig {
        risk_profile: "default".into(),
        ..Default::default()
    };
    config.agents.insert("from".to_string(), from_agent);
    config
        .risk_profiles
        .entry("default".into())
        .or_default()
        .allowed_commands = vec!["echo".into()];
    config.runtime_profiles.entry("default".into()).or_default();
    // Create the agent's default workspace dir so the move has something to move.
    let old_ws = config.agent_workspace_dir("from");
    std::fs::create_dir_all(&old_ws).unwrap();
    let cron_store = clawcrew_infra::cron_store::CronStore::new(&config.data_dir).unwrap();
    cron_store
        .add_job("from", "* * * * *", "echo hi")
        .expect("seed cron job");

    let state = crate::api::test_state(config.clone());
    let body = RenameMapKeyBody {
        path: "agents".to_string(),
        from: "from".to_string(),
        to: "to".to_string(),
    };
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let resp = rename_agent_cascade(&state, config.clone(), &body, guard).await;
    assert!(resp.status().is_success(), "a clean rename returns success");

    // Config swapped to `to`.
    assert!(state.config.read().agents.contains_key("to"));
    assert!(!state.config.read().agents.contains_key("from"));
    // Cron re-pointed to `to` - the move happened, after a successful persist.
    assert_eq!(
        cron_store.list_jobs_by_agent("to").unwrap().len(),
        1,
        "cron moves to `to` once persist succeeds"
    );
    assert!(cron_store.list_jobs_by_agent("from").unwrap().is_empty());
    // Workspace moved to the new alias path.
    assert!(
        state.config.read().agent_workspace_dir("to").exists(),
        "workspace moved to `to`"
    );
    assert!(!old_ws.exists(), "old workspace no longer present");
    // (MockMemory.rename_agent is unsupported, so the response `warnings`
    // carries that one known memory line - cron + workspace prove the move.)
}

#[tokio::test]
async fn agent_rename_resume_converges_when_config_already_to() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = clawcrew_config::schema::Config {
        config_path: tmp.path().join("config.toml"), // writable
        data_dir: tmp.path().join("data"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    let from_agent = clawcrew_config::schema::AliasedAgentConfig {
        risk_profile: "default".into(),
        ..Default::default()
    };
    config.agents.insert("from".to_string(), from_agent);
    config
        .risk_profiles
        .entry("default".into())
        .or_default()
        .allowed_commands = vec!["echo".into()];
    config.runtime_profiles.entry("default".into()).or_default();

    // Seed the lagging owned state + workspace under `from`.
    let cron_store = clawcrew_infra::cron_store::CronStore::new(&config.data_dir).unwrap();
    cron_store
        .add_job("from", "* * * * *", "echo hi")
        .expect("seed lagged cron job under `from`");
    let old_ws = config.agent_workspace_dir("from");
    std::fs::create_dir_all(&old_ws).unwrap();

    // Simulate the post-persist window: config already committed the rename
    // to `to` (so `from` is gone from config), while the cron row + workspace
    // above still lag at `from`. The cron DB lives under data_dir and survives
    // this in-memory config edit.
    config.agents.remove("from");
    config.agents.insert(
        "to".to_string(),
        clawcrew_config::schema::AliasedAgentConfig {
            risk_profile: "default".into(),
            ..Default::default()
        },
    );

    let state = crate::api::test_state(config.clone());
    let body = RenameMapKeyBody {
        path: "agents".to_string(),
        from: "from".to_string(),
        to: "to".to_string(),
    };
    // Re-issue the SAME rename. Beforethis returned 404 (from absent in
    // the committed config); now it resumes and re-runs the lagging effects.
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let resp = rename_agent_cascade(&state, config.clone(), &body, guard).await;
    assert!(
        resp.status().is_success(),
        "re-issuing a rename after a post-persist lag must converge, not 404"
    );

    // Owned state converged onto `to`.
    assert_eq!(
        cron_store.list_jobs_by_agent("to").unwrap().len(),
        1,
        "lagged cron re-points to `to` on resume"
    );
    assert!(
        cron_store.list_jobs_by_agent("from").unwrap().is_empty(),
        "no cron left under `from` after convergence"
    );
    // Workspace converged onto `to`.
    assert!(
        state.config.read().agent_workspace_dir("to").exists(),
        "workspace moved to `to` on resume"
    );
    assert!(!old_ws.exists(), "old `from` workspace no longer present");
    // Config still names `to` and never regained `from` (no double-rename).
    assert!(state.config.read().agents.contains_key("to"));
    assert!(!state.config.read().agents.contains_key("from"));
}

#[tokio::test]
async fn agent_rename_unrelated_collision_is_not_treated_as_resume() {
    let tmp = tempfile::tempdir().unwrap();
    let config = clawcrew_config::schema::Config {
        config_path: tmp.path().join("config.toml"), // writable
        data_dir: tmp.path().join("data"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();

    // Committed-`to` shape: config has `to`, the source `gone` is absent.
    // Crucially there is NO residue under `gone` - no workspace dir, no cron
    // job, no acp/memory/session rows. This is an unrelated request (or an
    // already-fully-converged duplicate), not a partial-failure resume.
    let mut config = config;
    config.agents.insert(
        "to".to_string(),
        clawcrew_config::schema::AliasedAgentConfig {
            risk_profile: "default".into(),
            ..Default::default()
        },
    );
    config.risk_profiles.entry("default".into()).or_default();
    config.runtime_profiles.entry("default".into()).or_default();
    // Guard the test's own premise: the source workspace must not exist.
    assert!(
        !config.agent_workspace_dir("gone").exists(),
        "precondition: no residue workspace under the absent source"
    );

    let state = crate::api::test_state(config.clone());
    let body = RenameMapKeyBody {
        path: "agents".to_string(),
        from: "gone".to_string(),
        to: "to".to_string(),
    };
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let resp = rename_agent_cascade(&state, config.clone(), &body, guard).await;

    // No residue → NOT a resume → the normal branch runs `rename_with_cascade`
    // with `gone` absent → NotFound → an error response, not a silent success.
    assert!(
        !resp.status().is_success(),
        "an unrelated `gone -> to` with no residue must surface an error, not be silently treated as a resume"
    );
    // Config untouched: no rename happened, `to` still present, `gone` absent.
    assert!(state.config.read().agents.contains_key("to"));
    assert!(!state.config.read().agents.contains_key("gone"));
}

#[test]
fn map_prop_error_classifies_unknown_property() {
    let err = anyhow::Error::msg("Unknown property 'foo.bar'");
    let api_err = map_prop_error(err, "foo.bar");
    assert_eq!(api_err.code, ConfigApiCode::PathNotFound);
}

#[test]
fn map_prop_error_classifies_type_mismatch() {
    // The classifier (config::api_error::classify_validation_message) now
    // matches "type mismatch" → ValueTypeMismatch; was ValidationFailed.
    let err = anyhow::Error::msg("type mismatch: expected u64");
    let api_err = map_prop_error(err, "scheduler.max_concurrent");
    assert_eq!(api_err.code, ConfigApiCode::ValueTypeMismatch);
}

#[test]
fn map_prop_error_falls_back_to_validation_on_unknown_message() {
    let err = anyhow::Error::msg("some completely unrecognized validator message");
    let api_err = map_prop_error(err, "scheduler.max_concurrent");
    assert_eq!(api_err.code, ConfigApiCode::ValidationFailed);
}

#[test]
fn json_pointer_to_dotted_handles_pointer_form() {
    assert_eq!(
        json_pointer_to_dotted("/providers/models/openrouter/api-key"),
        "providers.models.openrouter.api-key"
    );
}

#[test]
fn json_pointer_to_dotted_passes_dotted_through() {
    assert_eq!(
        json_pointer_to_dotted("providers.models.openrouter.api-key"),
        "providers.models.openrouter.api-key"
    );
    assert_eq!(
        json_pointer_to_dotted("scheduler.max_concurrent"),
        "scheduler.max_concurrent"
    );
}

#[test]
fn json_pointer_to_dotted_handles_empty_root() {
    assert_eq!(json_pointer_to_dotted(""), "");
    assert_eq!(json_pointer_to_dotted("/"), "");
}

use clawcrew_config::traits::PropKind;

#[test]
fn test_op_coercion_bool_typed_value_matches_stored() {
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.risk_profiles.insert(
        "default".into(),
        clawcrew_config::schema::RiskProfileConfig::default(),
    );
    cfg.set_prop("risk_profiles.default.workspace_only", "true")
        .expect("set_prop bool");
    let actual = cfg
        .get_prop("risk_profiles.default.workspace_only")
        .expect("get_prop");
    let want_typed = json_to_setprop_string(&serde_json::json!(true), Some(PropKind::Bool))
        .expect("coerce bool true");
    assert_eq!(
        actual, want_typed,
        "bool field: typed JSON `true` must coerce to the same display string \
             as `get_prop` returns; got actual={actual:?} want_typed={want_typed:?}"
    );

    // Legacy string-form (`Value::String("true")`) for the same bool
    // field must also coerce to the same string — back-compat for
    // clients that send strings instead of booleans.
    let want_string = json_to_setprop_string(&serde_json::json!("true"), Some(PropKind::Bool))
        .expect("coerce bool from string");
    assert_eq!(actual, want_string);
}

#[test]
fn test_op_coercion_integer_typed_value_matches_stored() {
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.set_prop("gateway.port", "42617")
        .expect("set_prop integer");
    let actual = cfg.get_prop("gateway.port").expect("get_prop");
    let want_typed = json_to_setprop_string(&serde_json::json!(42617), Some(PropKind::Integer))
        .expect("coerce integer");
    assert_eq!(
        actual, want_typed,
        "integer field coercion: actual={actual:?} want_typed={want_typed:?}"
    );

    // Legacy string-form must also coerce equivalently.
    let want_string = json_to_setprop_string(&serde_json::json!("42617"), Some(PropKind::Integer))
        .expect("coerce integer from string");
    assert_eq!(actual, want_string);
}

#[test]
fn test_op_coercion_float_typed_value_matches_stored() {
    let mut cfg = clawcrew_config::schema::Config::default();
    // autonomy doesn't carry floats today; use a model_provider temperature
    // by setting a known model provider entry. The model providers map
    // is set up via map keys, so use a path that's unambiguously float.
    // Fall back to set_prop on a known float location:
    match cfg.set_prop("providers.models.openai.temperature", "0.7") {
        Ok(()) => {
            let actual = cfg
                .get_prop("providers.models.openai.temperature")
                .expect("get_prop float");
            let want_typed = json_to_setprop_string(&serde_json::json!(0.7), Some(PropKind::Float))
                .expect("coerce float typed");
            assert_eq!(
                actual, want_typed,
                "float field coercion: actual={actual:?} want_typed={want_typed:?}"
            );
        }
        Err(_) => {
            // Float path not available on default Config — skip without
            // failing. The bool and integer tests cover the same
            // invariant; float just pins the additional case.
        }
    }
}

#[test]
fn test_op_coercion_string_field_no_regression() {
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.set_prop("gateway.host", "10.0.0.1")
        .expect("set_prop string");
    let actual = cfg.get_prop("gateway.host").expect("get_prop string");
    let want_typed = json_to_setprop_string(&serde_json::json!("10.0.0.1"), Some(PropKind::String))
        .expect("coerce string");
    assert_eq!(actual, want_typed);
}

#[test]
fn test_op_coercion_mismatched_value_correctly_fails() {
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.risk_profiles.insert(
        "default".into(),
        clawcrew_config::schema::RiskProfileConfig::default(),
    );
    cfg.set_prop("risk_profiles.default.workspace_only", "true")
        .expect("set_prop");
    let actual = cfg
        .get_prop("risk_profiles.default.workspace_only")
        .expect("get_prop");
    let want = json_to_setprop_string(&serde_json::json!(false), Some(PropKind::Bool))
        .expect("coerce bool false");
    assert_ne!(
        actual, want,
        "bool true must not match bool false after coercion — \
             a mismatched test op should fail with ValidationFailed"
    );
}

// ── Integration-flavored tests: drift detection + comment writing ──

use std::path::PathBuf;

fn temp_config_path() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("config.toml");
    (tmp, path)
}

#[tokio::test]
async fn compute_drift_returns_empty_when_in_memory_matches_disk() {
    let (_tmp, path) = temp_config_path();
    let cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    // Write the in-memory state to disk first so they agree by definition.
    cfg.save().await.expect("save");

    let drift = compute_drift(&cfg).await;
    assert!(
        drift.is_empty(),
        "expected no drift right after save, got {drift:?}"
    );
}

#[tokio::test]
async fn compute_drift_surfaces_mismatched_non_secret_field() {
    let (_tmp, path) = temp_config_path();
    let mut cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    cfg.save().await.expect("initial save");

    // Mutate the in-memory config without saving.
    cfg.set_prop("gateway.host", "10.0.0.1").expect("set_prop");

    let drift = compute_drift(&cfg).await;
    let entry = drift
        .iter()
        .find(|d| d.path == "gateway.host")
        .expect("expected gateway.host in drift summary");
    assert!(!entry.secret);
    assert!(entry.drifted);
    assert!(entry.in_memory_value.is_some());
    assert!(entry.on_disk_value.is_some());
}

#[tokio::test]
async fn compute_drift_returns_empty_when_no_disk_file() {
    let (_tmp, path) = temp_config_path();
    let cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    // Don't save — file does not exist.
    let drift = compute_drift(&cfg).await;
    assert!(drift.is_empty());
}

#[tokio::test]
async fn apply_comments_writes_decoration_to_existing_value() {
    let (_tmp, path) = temp_config_path();
    let mut cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    cfg.set_prop("gateway.host", "10.0.0.5").expect("set_prop");
    cfg.save().await.expect("save");

    clawcrew_config::comment_writer::apply_comments(
        &path,
        &[("gateway.host".into(), "raised after Q3 backlog".into())],
    )
    .await
    .expect("apply_comments");

    let raw = tokio::fs::read_to_string(&path).await.expect("read back");
    // Existence check: the comment text appears in the file.
    assert!(
        raw.contains("# raised after Q3 backlog"),
        "expected comment in file, got:\n{raw}"
    );

    // Positional check: the comment appears IMMEDIATELY ABOVE `host = ...`,
    // not somewhere else in the file. The previous version of the helper
    // wrote the prefix between `=` and the value, producing broken TOML —
    // this assertion would have caught that bug.
    let lines: Vec<&str> = raw.lines().collect();
    let host_line_idx = lines
        .iter()
        .position(|l| l.trim_start().starts_with("host"))
        .expect("host = line in saved config");
    assert!(
        host_line_idx > 0,
        "host line is at top — comment can't precede it"
    );
    let above = lines[host_line_idx - 1];
    assert_eq!(
        above.trim(),
        "# raised after Q3 backlog",
        "expected comment immediately above `host = ...`, got line above:\n  {above:?}\nfull file:\n{raw}"
    );

    // Round-trip check: re-parsing the file must succeed (broken
    // decoration target produces malformed TOML).
    let _: toml::Value = toml::from_str(&raw)
        .unwrap_or_else(|e| panic!("re-parse failed after apply_comments: {e}\nfile:\n{raw}"));
}

#[test]
fn scrub_credentials_catches_credential_shaped_strings() {
    use galleon_security_scrub::scrub_credentials;

    let cases = [
        // Field=value style log line.
        (
            "api-key=sk-live-abcdef-1234567890",
            "sk-live-abcdef-1234567890",
        ),
        // JSON-ish quoted key-value pair.
        (
            r#""token": "sk-test-supersecret-12345""#,
            "sk-test-supersecret-12345",
        ),
        // Explicit secret key.
        (
            "secret: hunter2-not-a-real-password",
            "hunter2-not-a-real-password",
        ),
        // Bearer credential pair.
        (
            "credential: bearer-token-abcdef-9876",
            "bearer-token-abcdef-9876",
        ),
    ];
    for (input, raw_secret) in cases {
        let scrubbed = scrub_credentials(input);
        assert!(
            !scrubbed.contains(raw_secret),
            "scrubber missed `{raw_secret}` in:\n  input    : {input}\n  scrubbed : {scrubbed}"
        );
        assert!(
            scrubbed.contains("REDACTED"),
            "expected REDACTED marker in:\n  input    : {input}\n  scrubbed : {scrubbed}"
        );
    }
}

#[tokio::test]
async fn compute_drift_detects_external_edit_to_field() {
    // Persist initial state, externally edit the file, drift surfaces
    // the touched path. This is the substrate the PATCH 409 guard fires on.
    let (_tmp, path) = temp_config_path();
    let mut cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    cfg.set_prop("gateway.host", "10.0.0.1").expect("set");
    cfg.save().await.expect("save");

    // Simulate a hand-edit while the daemon "wasn't looking".
    let on_disk = tokio::fs::read_to_string(&path).await.unwrap();
    let edited = on_disk.replace("10.0.0.1", "192.168.1.1");
    tokio::fs::write(&path, edited).await.unwrap();

    // In-memory still believes 10.0.0.1; on-disk now says 192.168.1.1.
    let drift = compute_drift(&cfg).await;
    let entry = drift
        .iter()
        .find(|d| d.path == "gateway.host")
        .expect("expected gateway.host in drift summary after external edit");
    assert!(entry.drifted);
    assert_eq!(
        entry.in_memory_value,
        Some(serde_json::Value::String("10.0.0.1".into()))
    );
    assert_eq!(
        entry.on_disk_value,
        Some(serde_json::Value::String("192.168.1.1".into()))
    );
}

#[test]
fn secret_response_only_carries_path_and_populated_flag() {
    // Belt-and-braces: serialize a SecretResponse and assert the JSON
    // shape carries neither a `value` field nor a length-leaking string.
    // If anyone ever adds a field to SecretResponse, this test fires.
    let r = SecretResponse {
        path: "providers.models.ollama.api-key".into(),
        populated: true,
    };
    let json = serde_json::to_value(&r).expect("serialize");
    let obj = json.as_object().expect("object");
    let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        vec!["path", "populated"],
        "SecretResponse must carry only path + populated"
    );
    assert!(!obj.contains_key("value"));
    assert!(!obj.contains_key("length"));
    assert!(!obj.contains_key("hash"));
    assert!(!obj.contains_key("masked"));
}

#[test]
fn lookup_prop_field_synthesizes_dynamic_http_request_secret_metadata() {
    let cfg = clawcrew_config::schema::Config::default();
    let field = lookup_prop_field(&cfg, "http_request.secrets.api_token")
        .expect("dynamic http_request secret metadata");

    assert_eq!(field.kind, PropKind::String);
    assert!(field.is_secret);
    assert_eq!(
        field.credential_class,
        Some(clawcrew_config::traits::CredentialSurfaceClass::EncryptedSecret)
    );
}

#[test]
fn list_entry_for_secret_omits_value_field() {
    let entry = ListEntry {
        path: "providers.models.ollama.api-key".into(),
        category: "providers.models".into(),
        kind: "string",
        type_hint: "Option<String>",
        value: None,
        populated: true,
        is_secret: true,
        is_env_overridden: false,
        enum_variants: vec![],
        section: Some("providers.models"),
        tab: "",
        multiline: false,
    };
    let json = serde_json::to_value(&entry).expect("serialize");
    let obj = json.as_object().expect("object");
    // skip_serializing_if on `value` means it must be absent.
    assert!(
        !obj.contains_key("value"),
        "secret list entry leaks `value` field"
    );
    // is_secret marker must be present so the dashboard can render it as locked.
    assert_eq!(obj.get("is_secret"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(obj.get("populated"), Some(&serde_json::Value::Bool(true)));
}

#[test]
fn gateway_paired_tokens_is_gateway_managed() {
    // The `Configurable` derive emits prop-field names in the field's
    // snake_case form, so the canonical name is `gateway.paired_tokens`
    // (underscore). The matcher must use that exact string, otherwise the
    // guard never fires and the secret keeps surfacing as drift.
    assert!(
        is_gateway_managed_field("gateway.paired_tokens"),
        "gateway.paired_tokens must be treated as gateway-managed"
    );
    // The old hyphenated form never matched a real prop-field name.
    assert!(!is_gateway_managed_field("gateway.paired-tokens"));

    // Guard against the field being renamed or the derive changing its
    // naming convention out from under the matcher.
    let cfg = clawcrew_config::schema::Config::default();
    assert!(
        cfg.prop_fields()
            .iter()
            .any(|p| p.name == "gateway.paired_tokens"),
        "expected a prop-field named gateway.paired_tokens"
    );
}

#[tokio::test]
async fn compute_drift_excludes_gateway_paired_tokens() {
    let (_tmp, path) = temp_config_path();
    let mut cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    cfg.save().await.expect("initial save");

    // Mutate the gateway-managed secret in memory without saving. Drift
    // detection must not surface it because the gateway owns it.
    cfg.gateway.paired_tokens = vec!["minted-by-the-gateway".into()];

    let drift = compute_drift(&cfg).await;
    assert!(
        !drift.iter().any(|d| d.path == "gateway.paired_tokens"),
        "gateway.paired_tokens must never appear in drift, got {drift:?}"
    );
}

#[tokio::test]
async fn compute_drift_excludes_env_overridden_secret() {
    let (_tmp, path) = temp_config_path();
    let mut cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    cfg.save().await.expect("initial save");

    cfg.composio.api_key = Some("injected-via-env".into());
    cfg.env_overridden_paths = std::collections::HashSet::from(["composio.api_key".to_string()]);

    let drift = compute_drift(&cfg).await;
    assert!(
        !drift.iter().any(|d| d.path == "composio.api_key"),
        "env-overridden secret must never appear in drift, got {drift:?}"
    );
}

#[test]
fn every_gateway_secret_is_classified() {
    const OPERATOR_EDITED_GATEWAY_SECRETS: &[&str] = &["gateway.webhook_secret"];

    let cfg = clawcrew_config::schema::Config::default();
    let unclassified: Vec<String> = cfg
        .prop_fields()
        .iter()
        .filter(|p| p.is_secret && p.name.starts_with("gateway."))
        .map(|p| p.name.clone())
        .filter(|name| {
            !is_gateway_managed_field(name)
                && !OPERATOR_EDITED_GATEWAY_SECRETS.contains(&name.as_str())
        })
        .collect();

    assert!(
        unclassified.is_empty(),
        "new [gateway] secret field(s) {unclassified:?} are not classified.\n\
             If the gateway mints/rotates/persists this field itself, add it to \
             `is_gateway_managed_field`.\n\
             If operators edit it directly in config.toml, add it to the \
             OPERATOR_EDITED_GATEWAY_SECRETS list in this test."
    );
}

#[test]
fn drift_entry_for_secret_omits_both_values() {
    let entry = DriftEntry {
        path: "providers.models.ollama.api-key".into(),
        secret: true,
        drifted: true,
        in_memory_value: None,
        on_disk_value: None,
    };
    let json = serde_json::to_value(&entry).expect("serialize");
    let obj = json.as_object().expect("object");
    assert!(
        !obj.contains_key("in_memory_value"),
        "secret drift entry leaks in_memory_value"
    );
    assert!(
        !obj.contains_key("on_disk_value"),
        "secret drift entry leaks on_disk_value"
    );
    assert_eq!(obj.get("secret"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(obj.get("drifted"), Some(&serde_json::Value::Bool(true)));
}

#[tokio::test]
async fn apply_comments_clears_existing_comment_when_passed_empty() {
    let (_tmp, path) = temp_config_path();
    let mut cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    cfg.set_prop("gateway.host", "10.0.0.5").expect("set_prop");
    cfg.save().await.expect("save");

    clawcrew_config::comment_writer::apply_comments(
        &path,
        &[("gateway.host".into(), "first reason".into())],
    )
    .await
    .expect("apply first comment");
    clawcrew_config::comment_writer::apply_comments(
        &path,
        &[("gateway.host".into(), String::new())],
    )
    .await
    .expect("apply empty");

    let raw = tokio::fs::read_to_string(&path).await.expect("read back");
    assert!(
        !raw.contains("first reason"),
        "expected the prior comment to be cleared, got:\n{raw}"
    );
}

#[tokio::test]
async fn agent_delete_leaves_owned_state_intact_when_persist_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg_dir = tmp.path().join("config.toml");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let mut config = clawcrew_config::schema::Config {
        config_path: cfg_dir,
        data_dir: tmp.path().join("data"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    // Real default-workspace dir for the agent so the archive step has
    // something to act on (and so a buggy pre-fix run would visibly move
    // it under `agents/_deleted/`).
    let agent = clawcrew_config::schema::AliasedAgentConfig {
        risk_profile: "default".into(),
        ..Default::default()
    };
    config.agents.insert("victim".to_string(), agent);
    config
        .risk_profiles
        .entry("default".into())
        .or_default()
        .allowed_commands = vec!["echo".into()];
    config.runtime_profiles.entry("default".into()).or_default();
    let old_ws = config.agent_workspace_dir("victim");
    std::fs::create_dir_all(&old_ws).unwrap();
    // Seed an owned-state row (a cron job) under `victim` — the delete probe.
    let cron_store = clawcrew_infra::cron_store::CronStore::new(&config.data_dir).unwrap();
    cron_store
        .add_job("victim", "* * * * *", "echo hi")
        .expect("seed cron job");
    assert_eq!(cron_store.list_jobs_by_agent("victim").unwrap().len(), 1);

    let state = crate::api::test_state(config.clone());
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let resp = delete_agent_cascade(&state, config.clone(), "victim", guard).await;

    // Persist failed -> error response, not a clean delete.
    assert!(
        !resp.status().is_success(),
        "a failed config persist must surface an error"
    );
    // Owned state did NOT move: the cron job stays under `victim`.
    assert_eq!(
        cron_store.list_jobs_by_agent("victim").unwrap().len(),
        1,
        "cron must stay under `victim` when persist fails (no premature purge)"
    );
    // Workspace was NOT archived: still on disk at the original path.
    assert!(
        old_ws.exists(),
        "workspace must NOT have been archived when persist fails"
    );
    let archive_root = config.data_dir.join("agents").join("_deleted");
    assert!(
        !archive_root.exists(),
        "no archive directory must be created when persist fails"
    );
    // In-memory config was never swapped: still names `victim`.
    assert!(state.config.read().agents.contains_key("victim"));
}

#[tokio::test]
async fn agent_delete_purges_owned_state_after_successful_persist() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = clawcrew_config::schema::Config {
        config_path: tmp.path().join("config.toml"), // writable -> persist OK
        data_dir: tmp.path().join("data"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    let agent = clawcrew_config::schema::AliasedAgentConfig {
        risk_profile: "default".into(),
        ..Default::default()
    };
    config.agents.insert("victim".to_string(), agent);
    config
        .risk_profiles
        .entry("default".into())
        .or_default()
        .allowed_commands = vec!["echo".into()];
    config.runtime_profiles.entry("default".into()).or_default();
    let old_ws = config.agent_workspace_dir("victim");
    std::fs::create_dir_all(&old_ws).unwrap();
    let cron_store = clawcrew_infra::cron_store::CronStore::new(&config.data_dir).unwrap();
    cron_store
        .add_job("victim", "* * * * *", "echo hi")
        .expect("seed cron job");

    let state = crate::api::test_state(config.clone());
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let resp = delete_agent_cascade(&state, config.clone(), "victim", guard).await;
    assert!(resp.status().is_success(), "a clean delete returns success");

    // Config swapped: `victim` is GONE.
    assert!(
        !state.config.read().agents.contains_key("victim"),
        "agent removed from persisted config"
    );
    // Cron job purged: the cascade ran after a successful persist.
    assert!(
        cron_store.list_jobs_by_agent("victim").unwrap().is_empty(),
        "cron purged once persist succeeds"
    );
    // Workspace archived: source dir gone, archive dir populated.
    assert!(
        !old_ws.exists(),
        "old workspace no longer at the original path"
    );
    let archive_root = config.data_dir.join("agents").join("_deleted");
    assert!(archive_root.exists(), "archive directory was created");
    let archived_ws = std::fs::read_dir(&archive_root)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().join("workspace").exists())
        .expect("an archive entry for `victim` with a workspace/ subdir");
    assert!(
        archived_ws
            .file_name()
            .to_string_lossy()
            .starts_with("victim-"),
        "archive entry name must start with `victim-`"
    );
}

#[tokio::test]
async fn agent_delete_response_carries_partial_failure_warnings() {
    use axum::body::to_bytes;

    let tmp = tempfile::tempdir().unwrap();
    let mut config = clawcrew_config::schema::Config {
        config_path: tmp.path().join("config.toml"),
        data_dir: tmp.path().join("data"),
        ..Default::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    let agent = clawcrew_config::schema::AliasedAgentConfig {
        risk_profile: "default".into(),
        ..Default::default()
    };
    config.agents.insert("victim".to_string(), agent);
    config
        .risk_profiles
        .entry("default".into())
        .or_default()
        .allowed_commands = vec!["echo".into()];
    config.runtime_profiles.entry("default".into()).or_default();
    let agents_dir = config.data_dir.join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    let deleted_marker = agents_dir.join("_deleted");
    std::fs::write(&deleted_marker, b"").expect("seed _deleted blocker file");
    let old_ws = config.agent_workspace_dir("victim");
    std::fs::create_dir_all(&old_ws).unwrap();
    // Drop a real file inside the workspace so the cascade has something
    // to archive (and so we can detect a successful archive).
    std::fs::write(old_ws.join("marker.txt"), b"hi").unwrap();
    let cron_store = clawcrew_infra::cron_store::CronStore::new(&config.data_dir).unwrap();
    cron_store
        .add_job("victim", "* * * * *", "echo hi")
        .expect("seed cron job");

    let state = crate::api::test_state(config.clone());
    let guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let resp = delete_agent_cascade(&state, config.clone(), "victim", guard).await;

    // The HTTP call is still 200 OK — partial failure is not an error
    // response, it is a successful response with `warnings` populated.
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    // Parse the response body and assert the `warnings` field is present
    // and non-empty. We assert the SPECIFIC shape the operator sees:
    // an array of strings, one per failed side-effect.
    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let warnings = json
        .get("warnings")
        .and_then(|v| v.as_array())
        .expect("response must carry a `warnings` array");
    assert!(
        !warnings.is_empty(),
        "partial-failure response must surface at least one warning, got: {warnings:?}"
    );
    // At least one warning should mention the archive dir (creation or rename).
    let joined = warnings
        .iter()
        .map(|v| v.as_str().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("archive"),
        "warnings should mention archive-side failures, got: {joined}"
    );
}

fn config_with_telegram_alias(
    tmp: &tempfile::TempDir,
    alias: &str,
) -> clawcrew_config::schema::Config {
    let mut config = temp_config(tmp);
    config.channels.telegram.insert(
        alias.to_string(),
        clawcrew_config::schema::TelegramConfig {
            enabled: true,
            bot_token: "test-token".to_string(),
            api_base_url: clawcrew_config::schema::TELEGRAM_OFFICIAL_API_BASE_URL.to_string(),
            ..Default::default()
        },
    );
    config
}

/// Trust-boundary regression: the bind route must reject an
/// unauthenticated request before any config mutation. Pairing is
/// required and no token is presented, so the handler returns 401 and
/// leaves the peer group untouched.
#[tokio::test]
async fn channel_bind_rejects_unauthenticated_request() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_with_telegram_alias(&tmp, "alerts");
    let mut state = test_state(config);
    state.pairing = Arc::new(PairingGuard::new(
        true,
        &[],
        clawcrew_config::pairing::PairingCodePolicy::default(),
    ));

    let (status, _json) = response_json(
        handle_api_channel_bind(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::Json(ChannelBindBody {
                channel_type: "telegram".to_string(),
                alias: "alerts".to_string(),
                identity: "123456789".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        state
            .config
            .read()
            .channel_external_peers("telegram", "alerts")
            .is_empty(),
        "a rejected bind must not mutate the peer group"
    );
}

/// Trust-boundary regression: binding into a `[channels.telegram.<alias>]`
/// that does not exist must 404 rather than mint a peer group the runtime
/// never resolves authorization from.
#[tokio::test]
async fn channel_bind_phantom_alias_is_404() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_with_telegram_alias(&tmp, "alerts");
    let state = test_state(config);

    let (status, _json) = response_json(
        handle_api_channel_bind(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::Json(ChannelBindBody {
                channel_type: "telegram".to_string(),
                alias: "ghost".to_string(),
                identity: "123456789".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        state
            .config
            .read()
            .channel_external_peers("telegram", "ghost")
            .is_empty(),
        "a phantom-alias bind must not create a peer group"
    );
}

/// Trust-boundary regression: an `ignore` naming the identity outranks the
/// grant a bind writes, so the endpoint must say so rather than answer
/// `saved` for a peer the running channel still rejects. The shared bind
/// core appended the grant and returned `Ok(true)`, and this handler
/// reported success on it.
#[tokio::test]
async fn channel_bind_refuses_an_identity_an_ignore_denies() {
    use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
    use clawcrew_config::providers::ChannelRef;

    let tmp = tempfile::tempdir().unwrap();
    let mut config = config_with_telegram_alias(&tmp, "alerts");
    config.peer_groups.insert(
        "telegram_alerts_blocked".to_string(),
        PeerGroupConfig {
            channel: ChannelRef::new("telegram.alerts".to_string()),
            ignore: vec![PeerUsername::new("123456789".to_string())],
            ..PeerGroupConfig::default()
        },
    );
    let state = test_state(config);

    let (status, json) = response_json(
        handle_api_channel_bind(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::Json(ChannelBindBody {
                channel_type: "telegram".to_string(),
                alias: "alerts".to_string(),
                identity: "123456789".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let message = json.to_string();
    assert!(
        message.contains("ignore"),
        "the operator has to be told what to edit, got: {message}"
    );
    assert!(
        state
            .config
            .read()
            .peer_groups
            .get("telegram_alerts")
            .is_none_or(|g| g.external_peers.is_empty()),
        "a refused bind must not mutate the peer group"
    );
}

/// The HTTP surface of the same rule: peer-group keys are arbitrary, so a
/// group named `telegram_alerts` may be bound to a different instance. The
/// endpoint must refuse rather than report `saved` for a write the
/// requested channel's reader never sees.
#[tokio::test]
async fn channel_bind_refuses_a_group_key_bound_to_another_channel() {
    use clawcrew_config::multi_agent::PeerGroupConfig;
    use clawcrew_config::providers::ChannelRef;

    let tmp = tempfile::tempdir().unwrap();
    let mut config = config_with_telegram_alias(&tmp, "alerts");
    config.peer_groups.insert(
        "telegram_alerts".to_string(),
        PeerGroupConfig {
            channel: ChannelRef::new("telegram.other".to_string()),
            ..PeerGroupConfig::default()
        },
    );
    let state = test_state(config);

    let (status, json) = response_json(
        handle_api_channel_bind(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::Json(ChannelBindBody {
                channel_type: "telegram".to_string(),
                alias: "alerts".to_string(),
                identity: "123456789".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        json.to_string().contains("telegram.other"),
        "the operator has to be told which group owns the key, got: {json}"
    );
    assert!(
        state.config.read().peer_groups["telegram_alerts"]
            .external_peers
            .is_empty(),
        "a refused bind must not write into another channel's group"
    );
}

/// The inverse: an unrelated `ignore` must not block a legitimate bind, or
/// the deny check would have turned the endpoint off.
#[tokio::test]
async fn channel_bind_still_binds_an_unignored_identity() {
    use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
    use clawcrew_config::providers::ChannelRef;

    let tmp = tempfile::tempdir().unwrap();
    let mut config = config_with_telegram_alias(&tmp, "alerts");
    config.peer_groups.insert(
        "telegram_alerts_blocked".to_string(),
        PeerGroupConfig {
            channel: ChannelRef::new("telegram.alerts".to_string()),
            ignore: vec![PeerUsername::new("999999999".to_string())],
            ..PeerGroupConfig::default()
        },
    );
    let state = test_state(config);

    let (status, _json) = response_json(
        handle_api_channel_bind(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::Json(ChannelBindBody {
                channel_type: "telegram".to_string(),
                alias: "alerts".to_string(),
                identity: "123456789".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        state
            .config
            .read()
            .channel_external_peers("telegram", "alerts")
            .contains(&"123456789".to_string())
    );
}

/// The gateway and other writers hold separate `Config` copies of one file.
/// An `ignore` saved concurrently is therefore absent from the gateway's
/// snapshot, and a bind that trusted that snapshot both missed the deny and
/// wrote the stale policy table back over it. The handler re-reads the
/// persisted `peer_groups` under the write lock instead, so the deny is
/// authoritative for the check and survives the save.
#[tokio::test]
async fn channel_bind_refuses_a_deny_only_the_other_handle_has_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let gateway_config = config_with_telegram_alias(&tmp, "alerts");
    let config_path = gateway_config.config_path.clone();

    // What another writer handle persisted after the gateway took its snapshot.
    // The gateway's own copy still has no `peer_groups` at all.
    std::fs::write(
        &config_path,
        "[peer_groups.telegram_alerts]\n\
             channel = \"telegram.alerts\"\n\
             ignore = [\"999999999\"]\n",
    )
    .unwrap();

    let state = test_state(gateway_config);
    assert!(
        state.config.read().peer_groups.is_empty(),
        "the gateway snapshot must start stale for this to test anything"
    );

    let (status, _json) = response_json(
        handle_api_channel_bind(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::Json(ChannelBindBody {
                channel_type: "telegram".to_string(),
                alias: "alerts".to_string(),
                identity: "999999999".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a deny this handle had not seen must still refuse the bind"
    );
    let on_disk = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        on_disk.contains("999999999"),
        "the persisted deny must survive, got:\n{on_disk}"
    );
}

/// The other half: an unrelated bind still succeeds, and it must not drop
/// the deny the gateway's snapshot never carried.
#[tokio::test]
async fn channel_bind_preserves_a_deny_saved_by_the_other_handle() {
    let tmp = tempfile::tempdir().unwrap();
    let gateway_config = config_with_telegram_alias(&tmp, "alerts");
    let config_path = gateway_config.config_path.clone();

    // `sops_dir` stands in for any unrelated key another writer persisted
    // after this handle took its snapshot. A full `save` of the stale
    // snapshot would drop it along with the deny.
    std::fs::write(
        &config_path,
        "sops_dir = \"/srv/written-by-the-other-handle\"\n\
             \n\
             [peer_groups.telegram_alerts]\n\
             channel = \"telegram.alerts\"\n\
             ignore = [\"999999999\"]\n",
    )
    .unwrap();

    let state = test_state(gateway_config);
    let (status, _json) = response_json(
        handle_api_channel_bind(
            axum::extract::State(state.clone()),
            axum::http::HeaderMap::new(),
            axum::Json(ChannelBindBody {
                channel_type: "telegram".to_string(),
                alias: "alerts".to_string(),
                identity: "123456789".to_string(),
            }),
        )
        .await,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let on_disk = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        on_disk.contains("123456789"),
        "the new grant must be persisted, got:\n{on_disk}"
    );
    assert!(
        on_disk.contains("999999999"),
        "the other handle's deny must not be erased, got:\n{on_disk}"
    );
    assert!(
        on_disk.contains("/srv/written-by-the-other-handle"),
        "an unrelated persisted key must survive the bind, got:\n{on_disk}"
    );
}

#[tokio::test]
async fn channel_bind_reports_the_custom_group_it_actually_wrote() {
    use clawcrew_config::multi_agent::PeerGroupConfig;
    use clawcrew_config::providers::ChannelRef;

    // The writer selects its target by the group's `channel` field, so a
    // custom key is a legitimate destination. Reporting the conventional
    // `telegram_alerts` would name a group that does not exist, and an
    // operator following the response would edit the wrong block.
    let tmp = tempfile::tempdir().unwrap();
    let mut config = config_with_telegram_alias(&tmp, "alerts");
    config.peer_groups.insert(
        "ops".to_string(),
        PeerGroupConfig {
            channel: ChannelRef::new("telegram.alerts".to_string()),
            ..PeerGroupConfig::default()
        },
    );
    let state = test_state(config);

    let bind = |state: AppState| async move {
        response_json(
            handle_api_channel_bind(
                axum::extract::State(state),
                axum::http::HeaderMap::new(),
                axum::Json(ChannelBindBody {
                    channel_type: "telegram".to_string(),
                    alias: "alerts".to_string(),
                    identity: "123456789".to_string(),
                }),
            )
            .await,
        )
        .await
    };

    let (status, json) = bind(state.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["saved"], true);
    assert_eq!(
        json["group"], "ops",
        "the response names the group the identity was written into"
    );
    assert!(
        state
            .config
            .read()
            .peer_groups
            .get("ops")
            .expect("the custom group is the destination")
            .external_peers
            .iter()
            .any(|peer| peer.as_str() == "123456789")
    );
    assert!(
        !state
            .config
            .read()
            .peer_groups
            .contains_key("telegram_alerts"),
        "the conventional group was never created, which is why naming it would mislead"
    );

    // The idempotent answer has to name the same real group.
    let (status, json) = bind(state.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["already_bound"], true);
    assert_eq!(json["group"], "ops");
}

#[tokio::test]
async fn refresh_context_window_allows_slow_response_and_forwards_api_key() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models"))
        .and(header("authorization", "Bearer test-api-key-123"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(3_100))
                .set_body_json(serde_json::json!({
                    "data": [{
                        "id": "llama-3.1-70b",
                        "context_length": 4096
                    }]
                })),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let (_tmp, path) = temp_config_path();
    let mut cfg = clawcrew_config::schema::Config {
        config_path: path.clone(),
        ..Default::default()
    };
    cfg.providers.models.groq.insert(
        "test".to_string(),
        clawcrew_config::schema::GroqModelProviderConfig {
            base: clawcrew_config::schema::ModelProviderConfig {
                model: Some("llama-3.1-70b".into()),
                api_key: Some("test-api-key-123".into()),
                uri: Some(mock.uri()),
                ..Default::default()
            },
        },
    );
    cfg.save().await.expect("initial save");

    let state = crate::api::test_state(cfg);

    let app = axum::Router::new()
        .route(
            "/api/config/model-providers/{type}/{alias}/refresh-context-window",
            axum::routing::post(handle_refresh_context_window),
        )
        .with_state(state);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/config/model-providers/groq/test/refresh-context-window")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "expected 200, got {}",
        response.status()
    );

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body")
        .to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(json["path"], "providers.models.groq.test");
    assert_eq!(json["context_window"], 4096);
    assert!(
        !body_str.contains("test-api-key-123"),
        "API key leaked in response body"
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "expected exactly one request to mock");
    assert_eq!(
        requests[0]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer test-api-key-123"
    );
}
