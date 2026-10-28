use super::*;
use crate::AppState;
use axum::extract::State;
use clawcrew_config::api_error::{ConfigApiCode, ConfigApiError};
use clawcrew_config::dto::*;

const DEV_CONFIG_TEMPLATE: &str = include_str!("../../../../dev/config.template.toml");
const DEV_HARNESS_TEMPLATE: &str = include_str!("../../../../dev/config.harness-test.toml");

fn parse_dev_template(raw: &str, name: &str) -> clawcrew_config::schema::Config {
    toml::from_str(raw).unwrap_or_else(|err| panic!("{name} must parse as schema V3: {err}"))
}

fn assert_common_dev_template_contract(cfg: &clawcrew_config::schema::Config) {
    assert_eq!(cfg.schema_version, 3);

    let provider = cfg
        .providers
        .models
        .ollama
        .get("default")
        .expect("Ollama default provider must exist");
    assert_eq!(provider.base.model.as_deref(), Some("llama3.2"));
    assert_eq!(
        provider.base.uri.as_deref(),
        Some("http://host.docker.internal:11434")
    );
    assert_eq!(provider.base.api_key, None);
    assert_eq!(provider.base.temperature, Some(0.7));

    let agent = cfg.agents.get("default").expect("default agent must exist");
    assert!(agent.enabled);
    assert_eq!(agent.model_provider.as_str(), "ollama.default");
    assert_eq!(agent.risk_profile.as_str(), "default");
    assert_eq!(agent.runtime_profile.as_str(), "default");
    assert_eq!(
        cfg.agent_workspace_dir("default"),
        std::path::PathBuf::from("/clawcrew-data/workspace")
    );

    let risk = cfg
        .risk_profiles
        .get("default")
        .expect("default risk profile must exist");
    assert_eq!(
        risk.level,
        clawcrew_config::autonomy::AutonomyLevel::Supervised
    );
    assert!(cfg.runtime_profile_for_agent("default").is_some());

    let status = derive_section_status(cfg);
    assert!(!status.needs_quickstart, "missing: {:?}", status.missing);
    assert_eq!(status.reason, "has_dispatchable_agent");

    assert_eq!(cfg.gateway.port, 42617);
    assert_eq!(cfg.gateway.host, "[::]");
    assert!(cfg.gateway.allow_public_bind);
    assert!(!cfg.gateway.require_pairing);
}

#[test]
fn dev_config_template_preserves_dispatch_contract() {
    let cfg = parse_dev_template(DEV_CONFIG_TEMPLATE, "dev/config.template.toml");
    assert_common_dev_template_contract(&cfg);

    assert_eq!(
        cfg.gateway.web_dist_dir.as_deref(),
        Some("/usr/share/clawcrewlabs/web/dist")
    );
    assert!(!cfg.cost.enabled);
    assert_eq!(cfg.cost.daily_limit_usd, 10.0);
    assert_eq!(cfg.cost.monthly_limit_usd, 100.0);
    assert_eq!(cfg.cost.warn_at_percent, 80);
    assert!(!cfg.cost.allow_override);
}

#[test]
fn dev_harness_template_preserves_runtime_contract() {
    let cfg = parse_dev_template(DEV_HARNESS_TEMPLATE, "dev/config.harness-test.toml");
    assert_common_dev_template_contract(&cfg);

    let runtime = cfg
        .runtime_profile_for_agent("default")
        .expect("harness agent must resolve its runtime profile");
    assert_eq!(runtime.max_tool_iterations, 50);
    assert_eq!(runtime.max_tool_result_chars, Some(50_000));
    assert_eq!(runtime.max_context_tokens, Some(32_000));
    assert!(!runtime.context_compression.enabled);
    assert_eq!(runtime.context_compression.tool_result_retrim_chars, 2_000);
    assert_eq!(
        cfg.risk_profiles["default"].auto_approve,
        [
            "file_read",
            "file_write",
            "file_edit",
            "memory_recall",
            "memory_store",
            "web_search_tool",
            "web_fetch",
            "calculator",
            "glob_search",
            "content_search",
            "image_info",
            "git_operations",
        ]
    );

    assert_eq!(cfg.memory.backend, "sqlite");
    assert!(cfg.memory.auto_save);
    assert!(cfg.memory.hygiene_enabled);
    assert_eq!(cfg.memory.archive_after_days, 7);
    assert_eq!(cfg.memory.purge_after_days, 30);
    assert_eq!(cfg.memory.embedding_provider, "none");
}

#[test]
fn build_agent_options_returns_every_configured_alias() {
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.create_map_key("providers.models.anthropic", "default")
        .unwrap();
    cfg.create_map_key("risk_profiles", "alpha_risk").unwrap();
    cfg.create_map_key("runtime_profiles", "alpha_runtime")
        .unwrap();
    cfg.create_map_key("skill_bundles", "alpha_skills").unwrap();
    cfg.create_map_key("knowledge_bundles", "alpha_knowledge")
        .unwrap();
    cfg.create_map_key("mcp_bundles", "alpha_mcp").unwrap();
    cfg.create_map_key("agents", "alpha_agent").unwrap();

    let resp = build_agent_options(&cfg);

    assert_eq!(resp.model_providers, vec!["anthropic.default".to_string()]);
    assert_eq!(resp.risk_profiles, vec!["alpha_risk".to_string()]);
    assert_eq!(resp.runtime_profiles, vec!["alpha_runtime".to_string()]);
    assert_eq!(resp.skill_bundles, vec!["alpha_skills".to_string()]);
    assert_eq!(resp.knowledge_bundles, vec!["alpha_knowledge".to_string()],);
    assert_eq!(resp.mcp_bundles, vec!["alpha_mcp".to_string()]);
    assert_eq!(resp.agents, vec!["alpha_agent".to_string()]);
}

#[test]
fn typed_provider_catalog_keys_create_snake_config_sections() {
    let mut cfg = clawcrew_config::schema::Config::default();
    let cases = [
        ("providers.models", "atomic_chat"),
        ("providers.models", "gemini_cli"),
        ("providers.transcription", "local_whisper"),
    ];

    for (family, key) in cases {
        let path = format!("{family}.{key}");
        cfg.create_map_key(&path, "default")
            .unwrap_or_else(|e| panic!("{key} should map to `{path}`: {e}"));
    }

    assert!(
        cfg.providers.models.atomic_chat.contains_key("default"),
        "created Atomic Chat alias should land in the atomic_chat provider map",
    );
    assert!(
        cfg.providers.models.gemini_cli.contains_key("default"),
        "created Gemini CLI alias should land in the gemini_cli provider map",
    );
    assert!(
        cfg.providers
            .transcription
            .local_whisper
            .contains_key("default"),
        "created Local Whisper alias should land in the local_whisper provider map",
    );
}

#[test]
fn derive_section_status_requires_dispatchable_agent() {
    let mut cfg = clawcrew_config::schema::Config::default();
    let resp = derive_section_status(&cfg);
    assert!(resp.needs_quickstart);
    assert_eq!(resp.reason, "fresh_install");

    cfg.create_map_key("providers.models.anthropic", "default")
        .unwrap();
    let resp = derive_section_status(&cfg);
    assert!(
        resp.needs_quickstart,
        "provider configured without a bound agent must not flip needs_quickstart"
    );
    assert_eq!(resp.reason, "incomplete_agent");
    assert!(resp.has_partial_state);

    cfg.create_map_key("risk_profiles", "default").unwrap();
    cfg.create_map_key("runtime_profiles", "default").unwrap();
    cfg.create_map_key("agents", "default").unwrap();
    let resp = derive_section_status(&cfg);
    assert!(
        resp.needs_quickstart,
        "agent without provider/profile bindings must still need onboarding"
    );
    assert_eq!(resp.reason, "incomplete_agent");
    assert!(
        resp.missing
            .iter()
            .any(|m| m == "Set a model provider for agent `default`.")
    );

    let agent = cfg.agents.get_mut("default").unwrap();
    agent.model_provider = "anthropic.default".into();
    agent.risk_profile = "default".into();
    agent.runtime_profile = "default".into();
    let resp = derive_section_status(&cfg);
    assert!(
        resp.needs_quickstart,
        "provider alias without a selected model must still need onboarding"
    );
    assert!(
        resp.missing
            .iter()
            .any(|m| m == "Choose a model for model provider `anthropic.default`.")
    );

    cfg.set_prop("providers.models.anthropic.default.model", "claude-sonnet")
        .unwrap();
    let resp = derive_section_status(&cfg);
    assert!(
        resp.needs_quickstart,
        "hosted provider alias without credential/auth must still need onboarding"
    );
    assert!(
        resp.missing
            .iter()
            .any(|m| m == "Set credential/auth for model provider `anthropic.default`.")
    );

    cfg.set_prop("providers.models.anthropic.default.api_key", "sk-test")
        .unwrap();
    let resp = derive_section_status(&cfg);
    assert!(!resp.needs_quickstart);
    assert_eq!(resp.reason, "has_dispatchable_agent");
    assert!(!resp.has_partial_state || resp.missing.is_empty());
}

#[test]
fn derive_section_status_completed_sections_without_dispatchable_agent_stays_pending() {
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.onboard_state
        .completed_sections
        .push("providers.models".into());
    let resp = derive_section_status(&cfg);
    assert!(
        resp.needs_quickstart,
        "completed_sections marker without a dispatchable agent must NOT flip the redirect"
    );
    assert_eq!(resp.reason, "incomplete_agent");
}

#[test]
fn apply_first_run_agent_defaults_binds_existing_provider_and_profiles() {
    let mut cfg = clawcrew_config::schema::Config::default();
    cfg.create_map_key("providers.models.anthropic", "work")
        .unwrap();
    cfg.create_map_key("risk_profiles", "default").unwrap();
    cfg.create_map_key("runtime_profiles", "deep_work").unwrap();
    cfg.create_map_key("agents", "default").unwrap();

    apply_first_run_agent_defaults(&mut cfg, "default");

    let agent = cfg.agents.get("default").unwrap();
    assert_eq!(agent.model_provider.as_str(), "anthropic.work");
    assert_eq!(agent.risk_profile, "default");
    assert_eq!(agent.runtime_profile, "deep_work");
}

#[test]
fn memory_section_ready_tracks_onboarding_progress_not_default_backend() {
    let cfg = clawcrew_config::schema::Config::default();
    assert!(
        !section_ready(&cfg, "memory", false),
        "fresh onboarding should not show Memory checked merely because a default backend exists"
    );
    assert!(
        section_ready(&cfg, "memory", true),
        "Memory should show checked after the user has advanced through that section"
    );
}

fn empty_cfg() -> clawcrew_config::schema::Config {
    clawcrew_config::schema::Config::default()
}

fn section_test_state(config: clawcrew_config::schema::Config) -> AppState {
    let memory: std::sync::Arc<dyn clawcrew_api::memory_traits::Memory> =
        std::sync::Arc::new(galleon_memory::NoneMemory::new("none"));
    AppState {
        config: std::sync::Arc::new(parking_lot::RwLock::new(config)),
        config_write_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        model_provider: std::sync::Arc::new(crate::UnconfiguredModelProvider),
        model: "test-model".to_string(),
        temperature: None,
        mem: memory.clone(),
        memory_strategy: std::sync::Arc::new(
            galleon_memory::strategy::DefaultMemoryStrategy::with_config(
                memory,
                clawcrew_config::schema::MemoryConfig::default(),
                std::path::PathBuf::new(),
            ),
        ),
        auto_save: false,
        pairing: std::sync::Arc::new(clawcrew_config::pairing::PairingGuard::new(
            false,
            &[],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        )),
        trust_forwarded_headers: false,
        rate_limiter: std::sync::Arc::new(crate::GatewayRateLimiter::new(100, 100, 100)),
        auth_limiter: std::sync::Arc::new(crate::auth_rate_limit::AuthRateLimiter::new()),
        idempotency_store: std::sync::Arc::new(crate::IdempotencyStore::new(
            std::time::Duration::from_secs(300),
            1000,
        )),
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
        observer: std::sync::Arc::new(clawcrew_api::observability_traits::NoopObserver),
        tools_registry: std::sync::Arc::new(Vec::new()),
        tools_registry_by_agent: std::sync::Arc::new(std::collections::HashMap::new()),
        cost_tracker: None,
        event_tx: tokio::sync::broadcast::channel(16).0,
        event_buffer: std::sync::Arc::new(crate::sse::EventBuffer::new(16)),
        shutdown_tx: tokio::sync::watch::channel(false).0,
        reload_tx: None,
        node_registry: std::sync::Arc::new(crate::nodes::NodeRegistry::new(16)),
        mdns_peer_registry: crate::nodes::mdns::MdnsPeerRegistry::default(),
        path_prefix: String::new(),
        web_dist_dir: None,
        session_backend: None,
        session_queue: std::sync::Arc::new(crate::session_queue::SessionActorQueue::new(
            8, 30, 600,
        )),
        device_registry: None,
        pending_pairings: None,
        canvas_store: galleon_tools::canvas::CanvasStore::new(),
        #[cfg(feature = "webauthn")]
        webauthn: None,
        cancel_tokens: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        pending_reload: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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

#[test]
fn handle_sections_derives_every_top_level_field_from_schema() {
    // Regression: the section list must be schema-driven, not the old
    // hardcoded 6. Adding a new top-level field to `Config` should make
    // it appear here automatically.
    let cfg = empty_cfg();
    let mut roots: std::collections::BTreeSet<String> = cfg
        .prop_fields()
        .iter()
        .filter_map(|f| f.name.split('.').next().map(str::to_string))
        .collect();
    // Mirror handle_sections: map-keyed sections surface even when
    // their HashMap is empty (prop_fields only emits paths for
    // populated entries).
    for s in clawcrew_config::schema::Config::map_key_sections() {
        if let Some(first) = s.path.split('.').next() {
            roots.insert(first.to_string());
        }
    }
    for hidden in HIDDEN_TOP_LEVEL {
        roots.remove(*hidden);
    }
    // The 5 onboarding sections must still be in the derived set.
    for required in ["providers", "channels", "memory", "hardware", "tunnel"] {
        assert!(
            roots.contains(required),
            "derived sections must include onboarding section `{required}`; got {roots:?}",
        );
    }
    // Plus a sample of the runtime sections that used to be invisible.
    for runtime in ["gateway", "observability", "scheduler", "security"] {
        assert!(
            roots.contains(runtime),
            "derived sections must include runtime section `{runtime}`; got {roots:?}",
        );
    }
    // System / housekeeping fields must NOT surface.
    for hidden in HIDDEN_TOP_LEVEL {
        assert!(
            !roots.contains(*hidden),
            "hidden top-level `{hidden}` must not appear",
        );
    }
    for hidden in ["onboard_state", "onboard-state"] {
        assert!(
            !roots.contains(hidden),
            "onboarding bookkeeping root `{hidden}` must not appear",
        );
    }
}

#[tokio::test]
async fn handle_sections_emits_stable_group_key_with_english_fallback() {
    use http_body_util::BodyExt;

    let response = handle_sections(
        State(section_test_state(empty_cfg())),
        axum::http::HeaderMap::new(),
    )
    .await;
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let result: ConfigSectionsResult = serde_json::from_slice(&body).unwrap();
    let cron = result
        .sections
        .iter()
        .find(|section| section.key == "cron")
        .expect("schema-derived response should include cron");
    assert_eq!(cron.group, "Agent");
    assert_eq!(cron.group_key, "agent");
}

#[test]
fn channels_select_initializes_subsection_so_set_prop_works() {
    let mut cfg = empty_cfg();
    assert!(cfg.channels.matrix.is_empty(), "fresh config: matrix unset");

    cfg.create_map_key("channels.matrix", "mymatrixalias")
        .expect("create_map_key must succeed for channels.matrix");
    assert!(
        cfg.channels.matrix.contains_key("mymatrixalias"),
        "channels.matrix must have alias after create_map_key",
    );

    // The form would issue a PATCH whose set_prop call hits this path.
    cfg.set_prop(
        "channels.matrix.mymatrixalias.allowed_rooms",
        r#"["alice","bob"]"#,
    )
    .expect("set_prop on initialized matrix subsection must succeed");
    assert_eq!(
        cfg.channels
            .matrix
            .get("mymatrixalias")
            .unwrap()
            .allowed_rooms,
        vec!["alice".to_string(), "bob".to_string()],
    );
}

#[test]
fn providers_picker_sources_from_list_providers() {
    // Single source of truth: galleon_providers::list_model_providers().
    // Anthropic / OpenAI / OpenRouter must surface in the picker.
    let cfg = empty_cfg();
    let items = providers_picker(&cfg);
    let names: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
    assert!(
        names.contains(&"anthropic"),
        "expected anthropic in picker, got: {names:?}"
    );
    assert!(names.contains(&"openai"), "expected openai in picker");
    assert!(
        names.contains(&"openrouter"),
        "expected openrouter in picker"
    );

    // Display name is human-readable, not the canonical key.
    let anthropic = items.iter().find(|i| i.key == "anthropic").unwrap();
    assert_eq!(anthropic.label, "Anthropic");

    // Local-only model_providers carry a description hint.
    let local = items.iter().find(|i| i.description.is_some());
    assert!(
        local.is_some(),
        "at least one model_provider should be marked local"
    );

    // Empty config has no model_provider aliases — no badges yet.
    assert!(
        items.iter().all(|i| i.badge.is_none()),
        "fresh config shouldn't mark any model_provider as present"
    );
}

#[test]
fn providers_picker_marks_alias_readiness() {
    // Typed-family layout: each canonical family is a map-keyed
    // sub-section at `model_providers.<family>` whose entries are
    // operator-named aliases. Creating the alias alone is not enough
    // for chat dispatch; it still needs model + credential/auth.
    let mut cfg = empty_cfg();
    cfg.create_map_key("providers.models.anthropic", "default")
        .expect("create_map_key");
    let items = providers_picker(&cfg);
    let anthropic = items.iter().find(|i| i.key == "anthropic").unwrap();
    assert_eq!(
        anthropic.badge.as_deref(),
        Some("needs setup"),
        "anthropic should need setup after adding an empty alias"
    );

    cfg.set_prop(
        "providers.models.anthropic.default.model",
        "claude-sonnet-4-5",
    )
    .expect("set model");
    cfg.set_prop("providers.models.anthropic.default.api_key", "sk-test")
        .expect("set api key");
    let items = providers_picker(&cfg);
    let anthropic = items.iter().find(|i| i.key == "anthropic").unwrap();
    assert_eq!(
        anthropic.badge.as_deref(),
        Some("configured"),
        "anthropic should be marked configured once required chat fields are present"
    );
}

#[test]
fn memory_picker_sources_from_selectable_backends() {
    let cfg = empty_cfg();
    let items = memory_picker(&cfg);
    // Mirrors galleon_memory::selectable_memory_backends() exactly.
    let keys: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
    assert!(keys.contains(&"sqlite"));
    assert!(keys.contains(&"none"));
    // Fresh onboarding should not imply the user selected the default.
    let active = items.iter().find(|i| i.badge.as_deref() == Some("active"));
    assert!(
        active.is_none(),
        "fresh onboarding should not mark a memory backend active before the user confirms the step"
    );
}

#[test]
fn channels_picker_walks_schema_via_init_defaults() {
    // Pure schema discovery — same trick the TUI uses. Whatever channels
    // the build has compiled in (matrix / discord / slack / etc.) appears
    // in the picker without any hand-maintained list. Test asserts a
    // representative sample compiled into the default `ci-all` build.
    let cfg = empty_cfg();
    let items = schema_walk_picker(&cfg, "channels");
    let keys: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
    assert!(!keys.is_empty(), "channel picker must not be empty");
    // Channels that are unconditionally compiled (no feature gate)
    // should always appear:
    for expected in ["telegram", "slack", "discord"] {
        assert!(
            keys.contains(&expected),
            "expected `{expected}` in channels picker, got: {keys:?}"
        );
    }
    // Fresh config — nothing configured yet.
    assert!(
        items.iter().all(|i| i.badge.is_none()),
        "fresh config shouldn't mark any channel as configured"
    );
}

#[test]
fn channels_picker_marks_configured_after_create_map_key() {
    let mut cfg = empty_cfg();
    cfg.create_map_key("channels.matrix", "mymatrixalias")
        .expect("create_map_key must succeed for channels.matrix");
    let items = schema_walk_picker(&cfg, "channels");
    let matrix = items.iter().find(|i| i.key == "matrix").unwrap();
    assert_eq!(
        matrix.badge.as_deref(),
        Some("configured"),
        "matrix should be marked configured after create_map_key"
    );
}

#[test]
fn tunnel_picker_includes_synthetic_none() {
    let cfg = empty_cfg();
    let items = tunnel_provider_picker(&cfg);
    assert_eq!(
        items[0].key, "none",
        "`none` must be the first entry in the tunnel picker"
    );
    // `none` is the active default for a fresh config.
    assert_eq!(items[0].badge.as_deref(), Some("active"));
}

#[test]
fn tunnel_picker_surfaces_all_option_backed_providers_on_fresh_config() {
    let cfg = empty_cfg();
    let items = tunnel_provider_picker(&cfg);
    let keys: std::collections::BTreeSet<&str> = items.iter().map(|i| i.key.as_str()).collect();
    for required in [
        "none",
        "cloudflare",
        "tailscale",
        "ngrok",
        "openvpn",
        "pinggy",
        "custom",
    ] {
        assert!(
            keys.contains(required),
            "tunnel picker on a fresh config must include `{required}`; got: {:?}",
            keys.into_iter().collect::<Vec<_>>(),
        );
    }
}

#[test]
fn tunnel_picker_marks_active_provider_from_configured_section() {
    let mut cfg = empty_cfg();
    cfg.tunnel.tunnel_provider = "tailscale".to_string();
    let items = tunnel_provider_picker(&cfg);
    let active: Vec<&str> = items
        .iter()
        .filter(|i| i.badge.as_deref() == Some("active"))
        .map(|i| i.key.as_str())
        .collect();
    assert_eq!(
        active,
        vec!["tailscale"],
        "exactly one entry should be active after setting `tunnel.tunnel_provider = tailscale`"
    );
    assert_eq!(
        items[0].key, "none",
        "`none` is still the first picker entry even when another provider is active"
    );
}

#[test]
fn tunnel_picker_badges_present_option_fields_as_configured() {
    let mut cfg = empty_cfg();
    cfg.tunnel.tunnel_provider = "cloudflare".to_string();
    // Materialize the cloudflare nested Option directly so the picker sees
    // it as present, without writing the developer's real config.
    cfg.tunnel.cloudflare = Some(clawcrew_config::schema::CloudflareTunnelConfig::default());
    let items = tunnel_provider_picker(&cfg);
    let cloudflare = items
        .iter()
        .find(|i| i.key == "cloudflare")
        .expect("cloudflare should appear in the picker");
    assert_eq!(
        cloudflare.badge.as_deref(),
        Some("active"),
        "cloudflare is the active provider"
    );
    // `tailscale` is statically known but the user hasn't configured
    // it — its Option is still None, so no badge.
    let tailscale = items
        .iter()
        .find(|i| i.key == "tailscale")
        .expect("tailscale should appear in the picker");
    assert!(
        tailscale.badge.is_none(),
        "tailscale's Option is None, so no badge; got: {:?}",
        tailscale.badge
    );
}

#[tokio::test]
async fn tunnel_select_updates_canonical_provider_and_active_picker_badge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = clawcrew_config::schema::Config {
        config_path: tmp.path().join("config.toml"),
        ..Default::default()
    };
    let state = section_test_state(cfg);

    let response = handle_section_select(
        State(state.clone()),
        axum::http::HeaderMap::new(),
        axum::extract::Path(SectionItemPath {
            section: "tunnel".to_string(),
            key: "cloudflare".to_string(),
        }),
        None,
    )
    .await;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let cfg = state.config.read().clone();
    assert_eq!(cfg.tunnel.tunnel_provider, "cloudflare");
    assert!(
        state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    let raw = tokio::fs::read_to_string(&cfg.config_path).await.unwrap();
    let disk_value: toml::Value = toml::from_str(&raw).unwrap();
    assert_eq!(
        disk_value
            .get("tunnel")
            .and_then(|value| value.get("tunnel_provider"))
            .and_then(toml::Value::as_str),
        Some(cfg.tunnel.tunnel_provider.as_str()),
        "section selection must persist and publish the selected provider"
    );
    let items = tunnel_provider_picker(&cfg);
    let cloudflare = items
        .iter()
        .find(|item| item.key == "cloudflare")
        .expect("cloudflare should appear in the picker");
    assert_eq!(cloudflare.badge.as_deref(), Some("active"));
}

#[tokio::test]
async fn existing_section_selection_without_changes_skips_reload() {
    use http_body_util::BodyExt;

    let tmp = tempfile::tempdir().expect("tempdir");
    let config_path = tmp.path().join("config.toml");
    let mut cfg = clawcrew_config::schema::Config {
        config_path: config_path.clone(),
        ..Default::default()
    };
    cfg.create_map_key("providers.models.anthropic", "default")
        .expect("seed model provider alias");
    cfg.save().await.expect("seed disk config");
    let disk_before = tokio::fs::read(&config_path)
        .await
        .expect("read seed config");
    let state = section_test_state(cfg);
    let live_before = state.config.read().clone();

    let response = handle_section_select(
        State(state.clone()),
        axum::http::HeaderMap::new(),
        axum::extract::Path(SectionItemPath {
            section: "providers.models".to_string(),
            key: "anthropic".to_string(),
        }),
        None,
    )
    .await;
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json response");
    assert_eq!(json["fields_prefix"], "providers.models.anthropic.default");
    assert_eq!(json["created"], false);

    assert_eq!(tokio::fs::read(&config_path).await.unwrap(), disk_before);
    let live_after = state.config.read().clone();
    let live_before_value = toml::Value::try_from(&live_before).unwrap();
    let live_after_value = toml::Value::try_from(&live_after).unwrap();
    assert_eq!(
        live_after_value, live_before_value,
        "an existing selection must not replace the live snapshot"
    );
    assert!(
        !state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );
}

#[tokio::test]
async fn unchanged_memory_and_tunnel_selections_skip_reload() {
    use http_body_util::BodyExt;

    let memory_tmp = tempfile::tempdir().expect("memory tempdir");
    let memory_path = memory_tmp.path().join("config.toml");
    let mut memory_cfg = clawcrew_config::schema::Config {
        config_path: memory_path.clone(),
        ..Default::default()
    };
    memory_cfg.memory.backend = "sqlite".to_string();
    memory_cfg
        .onboard_state
        .completed_sections
        .push("memory".to_string());
    memory_cfg.save().await.expect("seed memory config");
    let memory_disk_before = tokio::fs::read(&memory_path)
        .await
        .expect("read seed memory config");
    let memory_state = section_test_state(memory_cfg);
    let memory_live_before = memory_state.config.read().clone();

    let memory_response = handle_section_select(
        State(memory_state.clone()),
        axum::http::HeaderMap::new(),
        axum::extract::Path(SectionItemPath {
            section: "memory".to_string(),
            key: "sqlite".to_string(),
        }),
        None,
    )
    .await;
    assert_eq!(memory_response.status(), axum::http::StatusCode::OK);
    let memory_body = memory_response
        .into_body()
        .collect()
        .await
        .expect("memory response body")
        .to_bytes();
    let memory_json: serde_json::Value =
        serde_json::from_slice(&memory_body).expect("memory json response");
    assert_eq!(memory_json["fields_prefix"], "memory");
    assert_eq!(memory_json["created"], false);
    assert_eq!(
        tokio::fs::read(&memory_path).await.unwrap(),
        memory_disk_before
    );
    assert_eq!(
        toml::Value::try_from(&*memory_state.config.read()).unwrap(),
        toml::Value::try_from(&memory_live_before).unwrap(),
        "an unchanged memory selection must not replace the live snapshot"
    );
    assert!(
        !memory_state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );

    let tunnel_tmp = tempfile::tempdir().expect("tunnel tempdir");
    let tunnel_path = tunnel_tmp.path().join("config.toml");
    let mut tunnel_cfg = clawcrew_config::schema::Config {
        config_path: tunnel_path.clone(),
        ..Default::default()
    };
    tunnel_cfg.tunnel.tunnel_provider = "cloudflare".to_string();
    tunnel_cfg.tunnel.cloudflare = Some(Default::default());
    tunnel_cfg.save().await.expect("seed tunnel config");
    let tunnel_disk_before = tokio::fs::read(&tunnel_path)
        .await
        .expect("read seed tunnel config");
    let tunnel_state = section_test_state(tunnel_cfg);
    let tunnel_live_before = tunnel_state.config.read().clone();

    let tunnel_response = handle_section_select(
        State(tunnel_state.clone()),
        axum::http::HeaderMap::new(),
        axum::extract::Path(SectionItemPath {
            section: "tunnel".to_string(),
            key: "cloudflare".to_string(),
        }),
        None,
    )
    .await;
    assert_eq!(tunnel_response.status(), axum::http::StatusCode::OK);
    let tunnel_body = tunnel_response
        .into_body()
        .collect()
        .await
        .expect("tunnel response body")
        .to_bytes();
    let tunnel_json: serde_json::Value =
        serde_json::from_slice(&tunnel_body).expect("tunnel json response");
    assert_eq!(tunnel_json["fields_prefix"], "tunnel.cloudflare");
    assert_eq!(tunnel_json["created"], false);
    assert_eq!(
        tokio::fs::read(&tunnel_path).await.unwrap(),
        tunnel_disk_before
    );
    assert_eq!(
        toml::Value::try_from(&*tunnel_state.config.read()).unwrap(),
        toml::Value::try_from(&tunnel_live_before).unwrap(),
        "an unchanged tunnel selection must not replace the live snapshot"
    );
    assert!(
        !tunnel_state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );
}

#[tokio::test]
async fn unchanged_memory_selection_rejects_disk_drift() {
    use http_body_util::BodyExt;

    let tmp = tempfile::tempdir().expect("memory tempdir");
    let config_path = tmp.path().join("config.toml");
    let mut live_cfg = clawcrew_config::schema::Config {
        config_path: config_path.clone(),
        ..Default::default()
    };
    live_cfg.memory.backend = "sqlite".to_string();
    live_cfg
        .onboard_state
        .completed_sections
        .push("memory".to_string());
    live_cfg.save().await.expect("seed live memory config");
    let state = section_test_state(live_cfg);

    let mut disk_cfg = state.config.read().clone();
    disk_cfg.memory.backend = "postgres".to_string();
    disk_cfg.save().await.expect("write external memory drift");
    let disk_before = tokio::fs::read(&config_path)
        .await
        .expect("read drifted memory config");
    let live_before = state.config.read().clone();

    let response = handle_section_select(
        State(state.clone()),
        axum::http::HeaderMap::new(),
        axum::extract::Path(SectionItemPath {
            section: "memory".to_string(),
            key: "sqlite".to_string(),
        }),
        None,
    )
    .await;

    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("memory response body")
        .to_bytes();
    let error: ConfigApiError = serde_json::from_slice(&body).expect("memory error response");
    assert_eq!(error.code, ConfigApiCode::ConfigChangedExternally);
    assert_eq!(tokio::fs::read(&config_path).await.unwrap(), disk_before);
    assert_eq!(
        toml::Value::try_from(&*state.config.read()).unwrap(),
        toml::Value::try_from(&live_before).unwrap(),
        "a rejected memory selection must preserve the live snapshot"
    );
    assert!(
        !state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );
}

#[tokio::test]
async fn unchanged_tunnel_selection_rejects_default_disk_drift() {
    use http_body_util::BodyExt;

    let tmp = tempfile::tempdir().expect("tunnel tempdir");
    let config_path = tmp.path().join("config.toml");
    let mut live_cfg = clawcrew_config::schema::Config {
        config_path: config_path.clone(),
        ..Default::default()
    };
    live_cfg.tunnel.tunnel_provider = "tailscale".to_string();
    live_cfg.tunnel.tailscale = Some(Default::default());
    live_cfg.save().await.expect("seed live tunnel config");
    let state = section_test_state(live_cfg);

    let mut disk_cfg = state.config.read().clone();
    disk_cfg
        .tunnel
        .tailscale
        .as_mut()
        .expect("tailscale defaults")
        .funnel = true;
    disk_cfg.save().await.expect("write external tunnel drift");
    let disk_before = tokio::fs::read(&config_path)
        .await
        .expect("read drifted tunnel config");
    let live_before = state.config.read().clone();

    let response = handle_section_select(
        State(state.clone()),
        axum::http::HeaderMap::new(),
        axum::extract::Path(SectionItemPath {
            section: "tunnel".to_string(),
            key: "tailscale".to_string(),
        }),
        None,
    )
    .await;

    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("tunnel response body")
        .to_bytes();
    let error: ConfigApiError = serde_json::from_slice(&body).expect("tunnel error response");
    assert_eq!(error.code, ConfigApiCode::ConfigChangedExternally);
    assert_eq!(tokio::fs::read(&config_path).await.unwrap(), disk_before);
    assert_eq!(
        toml::Value::try_from(&*state.config.read()).unwrap(),
        toml::Value::try_from(&live_before).unwrap(),
        "a rejected tunnel selection must preserve the live snapshot"
    );
    assert!(
        !state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );
}

#[tokio::test]
async fn unchanged_memory_and_tunnel_selections_reject_uninspectable_config() {
    use http_body_util::BodyExt;

    for (section, key) in [("memory", "sqlite"), ("tunnel", "cloudflare")] {
        for malformed in [false, true] {
            let tmp = tempfile::tempdir().expect("section tempdir");
            let config_path = tmp.path().join("config.toml");
            let mut live_cfg = clawcrew_config::schema::Config {
                config_path: config_path.clone(),
                ..Default::default()
            };
            if section == "memory" {
                live_cfg.memory.backend = key.to_string();
                live_cfg
                    .onboard_state
                    .completed_sections
                    .push(section.to_string());
            } else {
                live_cfg.tunnel.tunnel_provider = key.to_string();
                live_cfg.tunnel.cloudflare = Some(Default::default());
            }

            let malformed_contents = b"[memory\nbackend = \"sqlite\"";
            if malformed {
                tokio::fs::write(&config_path, malformed_contents)
                    .await
                    .expect("write malformed config");
            } else {
                std::fs::create_dir(&config_path).expect("create unreadable config path");
            }

            let state = section_test_state(live_cfg);
            let live_before = state.config.read().clone();
            let response = handle_section_select(
                State(state.clone()),
                axum::http::HeaderMap::new(),
                axum::extract::Path(SectionItemPath {
                    section: section.to_string(),
                    key: key.to_string(),
                }),
                None,
            )
            .await;

            assert_eq!(
                response.status(),
                axum::http::StatusCode::CONFLICT,
                "{section} must reject an {} canonical config",
                if malformed { "malformed" } else { "unreadable" }
            );
            let body = response
                .into_body()
                .collect()
                .await
                .expect("section response body")
                .to_bytes();
            let error: ConfigApiError =
                serde_json::from_slice(&body).expect("section error response");
            assert_eq!(error.code, ConfigApiCode::ConfigChangedExternally);
            if malformed {
                assert_eq!(
                    tokio::fs::read(&config_path).await.unwrap(),
                    malformed_contents
                );
            } else {
                assert!(config_path.is_dir());
            }
            assert_eq!(
                toml::Value::try_from(&*state.config.read()).unwrap(),
                toml::Value::try_from(&live_before).unwrap(),
                "a rejected {section} selection must preserve the live snapshot"
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
async fn section_select_snapshot_read_failure_stops_before_save() {
    use http_body_util::BodyExt;

    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::create_dir_all(&config_path).unwrap();
    let state = section_test_state(clawcrew_config::schema::Config {
        config_path: config_path.clone(),
        ..Default::default()
    });
    let live_before = state.config.read().clone();

    let response = handle_section_select(
        State(state.clone()),
        axum::http::HeaderMap::new(),
        axum::extract::Path(SectionItemPath {
            section: "tunnel".to_string(),
            key: "cloudflare".to_string(),
        }),
        None,
    )
    .await;

    assert_eq!(
        response.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body = response
        .into_body()
        .collect()
        .await
        .expect("snapshot failure response body")
        .to_bytes();
    let error: ConfigApiError = serde_json::from_slice(&body).expect("snapshot failure response");
    assert_eq!(error.code, ConfigApiCode::ReloadFailed);
    assert!(
        error
            .message
            .contains("failed to snapshot existing config before save"),
        "the handler must reject the unreadable snapshot before attempting a save: {}",
        error.message
    );
    assert!(
        config_path.is_dir(),
        "snapshot admission failure must retain the prior disk state"
    );
    assert_eq!(
        toml::Value::try_from(&*state.config.read()).unwrap(),
        toml::Value::try_from(&live_before).unwrap(),
        "snapshot admission failure must not publish the working snapshot"
    );
    assert!(
        !state
            .pending_reload
            .load(std::sync::atomic::Ordering::Relaxed)
    );
}

#[test]
fn one_tier_alias_map_picker_is_empty_for_unconfigured_section() {
    let cfg = empty_cfg();
    for section in [
        "peer_groups",
        "cron",
        "mcp_bundles",
        "knowledge_bundles",
        "skill_bundles",
        "risk_profiles",
        "runtime_profiles",
    ] {
        let items = one_tier_alias_map_picker(&cfg, section);
        assert!(
            items.is_empty(),
            "`{section}` picker must be empty on a fresh config, got: {:?}",
            items.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(),
        );
    }
}

#[test]
fn one_tier_alias_map_picker_surfaces_created_aliases() {
    let cases: &[(&str, &str)] = &[
        ("peer_groups", "team_chat"),
        ("cron", "daily_brief"),
        ("mcp_bundles", "core_tools"),
        ("knowledge_bundles", "house_docs"),
        ("skill_bundles", "ops_skills"),
        ("risk_profiles", "tight"),
        ("runtime_profiles", "fast_model"),
    ];
    for (section, alias) in cases {
        let mut cfg = empty_cfg();
        cfg.create_map_key(section, alias)
            .unwrap_or_else(|e| panic!("create_map_key({section}, {alias}) failed: {e}"));
        let items = one_tier_alias_map_picker(&cfg, section);
        assert!(
            items.iter().any(|i| i.key == *alias),
            "`{section}` picker should surface `{alias}` after create_map_key; got: {:?}",
            items.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(),
        );
        let entry = items.iter().find(|i| i.key == *alias).unwrap();
        assert_eq!(
            entry.badge.as_deref(),
            Some("configured"),
            "`{section}.{alias}` should be badged `configured`",
        );
    }
}

#[test]
fn picker_dispatch_covers_every_section_variant() {
    use clawcrew_config::sections::Section;
    let cfg = empty_cfg();
    // The full Section surface = wizard steps + explorer-only.
    // Spelling them out here pins both groups, so adding a row to
    // the `sections!` macro forces an update here too.
    let all: &[Section] = &[
        Section::ModelProviders,
        Section::TtsProviders,
        Section::TranscriptionProviders,
        Section::Channels,
        Section::Memory,
        Section::Hardware,
        Section::Tunnel,
        Section::Agents,
        Section::PeerGroups,
        Section::Storage,
        Section::Cron,
        Section::Mcp,
        Section::McpBundles,
        Section::KnowledgeBundles,
        Section::SkillBundles,
        Section::RiskProfiles,
        Section::RuntimeProfiles,
    ];
    let direct_form = [Section::Hardware, Section::Mcp];
    for section in all {
        match picker_items_for(*section, &cfg) {
            PickerDispatch::Items(_items) => {
                assert!(
                    !direct_form.contains(section),
                    "{section:?} is marked DirectForm but dispatched to Items",
                );
            }
            PickerDispatch::DirectForm => {
                assert!(
                    direct_form.contains(section),
                    "{section:?} returned DirectForm but is not in the DirectForm set; \
                     either give it a picker arm or add it to the DirectForm list",
                );
            }
        }
    }
}

#[test]
fn storage_picker_lists_all_kinds_and_marks_created() {
    let cfg = empty_cfg();
    let items = storage_picker(&cfg);
    let keys: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
    for expected in ["sqlite", "postgres", "qdrant", "markdown", "lucid"] {
        assert!(
            keys.contains(&expected),
            "storage picker must list `{expected}`, got: {keys:?}",
        );
    }
    // Fresh config — no kind should be badged.
    assert!(
        items.iter().all(|i| i.badge.is_none()),
        "fresh config: no storage kind should be marked configured",
    );

    // Create a sqlite instance; the sqlite row should flip to configured.
    let mut cfg2 = empty_cfg();
    cfg2.create_map_key("storage.sqlite", "primary")
        .expect("create_map_key(storage.sqlite, primary) must succeed");
    let items = storage_picker(&cfg2);
    let sqlite = items.iter().find(|i| i.key == "sqlite").unwrap();
    assert_eq!(
        sqlite.badge.as_deref(),
        Some("created"),
        "storage.sqlite should be marked created after adding an alias",
    );
    assert!(
        sqlite.description.is_some(),
        "storage picker should explain each backend tradeoff",
    );
}
