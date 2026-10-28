use std::collections::HashMap;
use std::sync::Arc;

use clawcrew_api::memory_traits::MemoryStrategy;
use clawcrew_config::cost::tracker::CostTracker;
use clawcrew_config::pairing::PairingGuard;
use clawcrew_config::schema::Config;
use galleon_infra::session_backend::SessionBackend;
use galleon_memory::{self, Memory};
use galleon_providers::{self, ModelProvider};
use galleon_tools::canvas::CanvasStore;
use parking_lot::RwLock;

/// Shared state for all axum handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,

    /// Serializes the read-mutate-save-swap critical section of every HTTP
    /// handler that mutates `config`
    pub config_write_lock: Arc<tokio::sync::Mutex<()>>,
    pub model_provider: Arc<dyn ModelProvider>,
    pub model: String,
    pub temperature: Option<f64>,
    pub mem: Arc<dyn Memory>,
    pub memory_strategy: Arc<dyn MemoryStrategy>,
    pub auto_save: bool,
    pub pairing: Arc<PairingGuard>,
    pub trust_forwarded_headers: bool,
    pub rate_limiter: Arc<crate::GatewayRateLimiter>,
    pub auth_limiter: Arc<crate::auth_rate_limit::AuthRateLimiter>,
    pub idempotency_store: Arc<crate::IdempotencyStore>,
    #[cfg(feature = "channel-whatsapp-cloud")]
    pub whatsapp: HashMap<String, Arc<clawcrew_channels::whatsapp::WhatsAppChannel>>,
    #[cfg(feature = "channel-whatsapp-cloud")]
    pub whatsapp_app_secret: HashMap<String, Arc<str>>,
    #[cfg(feature = "channel-linq")]
    pub linq: HashMap<String, Arc<clawcrew_channels::linq::LinqChannel>>,
    #[cfg(feature = "channel-linq")]
    pub linq_signing_secrets: HashMap<String, Arc<str>>,
    #[cfg(feature = "channel-nextcloud")]
    pub nextcloud_talk:
        HashMap<String, Arc<clawcrew_channels::nextcloud_talk::NextcloudTalkChannel>>,
    #[cfg(feature = "channel-nextcloud")]
    pub nextcloud_talk_webhook_secret: HashMap<String, Arc<str>>,
    #[cfg(feature = "channel-email")]
    pub gmail_push: Option<Arc<clawcrew_channels::gmail_push::GmailPushChannel>>,
    pub observer: Arc<dyn clawcrew_api::observability_traits::Observer>,
    pub tools_registry: Arc<Vec<clawcrew_api::tool::ToolSpec>>,
    pub tools_registry_by_agent: Arc<HashMap<String, Arc<Vec<clawcrew_api::tool::ToolSpec>>>>,
    pub cost_tracker: Option<Arc<CostTracker>>,
    pub event_tx: tokio::sync::broadcast::Sender<serde_json::Value>,
    pub event_buffer: Arc<crate::sse::EventBuffer>,
    pub shutdown_tx: tokio::sync::watch::Sender<bool>,
    pub reload_tx: Option<tokio::sync::watch::Sender<bool>>,
    pub node_registry: Arc<crate::nodes::NodeRegistry>,
    pub mdns_peer_registry: crate::nodes::mdns::MdnsPeerRegistry,
    pub path_prefix: String,
    pub web_dist_dir: Option<std::path::PathBuf>,
    pub session_backend: Option<Arc<dyn SessionBackend>>,
    pub session_queue: Arc<crate::session_queue::SessionActorQueue>,
    pub device_registry: Option<Arc<crate::api_pairing::DeviceRegistry>>,
    pub pending_pairings: Option<Arc<crate::api_pairing::PairingStore>>,
    pub canvas_store: CanvasStore,
    #[cfg(feature = "webauthn")]
    pub webauthn: Option<Arc<crate::api_webauthn::WebAuthnState>>,
    pub cancel_tokens: Arc<
        std::sync::Mutex<
            std::collections::HashMap<String, Arc<tokio_util::sync::CancellationToken>>,
        >,
    >,
    pub pending_reload: Arc<std::sync::atomic::AtomicBool>,
    pub tui_registry: Option<Arc<dyn clawcrew_api::runtime_traits::TuiConnectionRegistry>>,
    pub sop_runner: Option<Arc<dyn clawcrew_api::sop::SopRunner>>,
    pub diagnostics: Option<Arc<dyn clawcrew_api::doctor::DiagnosticsRunner>>,
    pub quickstart: Option<Arc<dyn clawcrew_config::quickstart::QuickstartRunner>>,
    pub integration_catalog:
        Option<Arc<dyn clawcrew_config::integrations::IntegrationCatalogProvider>>,
    pub personality_runner: Option<Arc<dyn clawcrew_api::personality::PersonalityRunner>>,
    pub skills_runner: Option<Arc<dyn clawcrew_api::skill::SkillsRunner>>,
    pub acp_runner: Option<Arc<dyn clawcrew_api::runtime_traits::AcpSessionRunner>>,
    pub control_plane: Option<clawcrew_api::control_plane::ControlPlaneHandle>,
}

#[derive(Debug, Clone, Default)]
pub struct FallbackModelProvider;

#[async_trait::async_trait]
impl ModelProvider for FallbackModelProvider {
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

impl clawcrew_api::attribution::Attributable for FallbackModelProvider {
    fn role(&self) -> clawcrew_api::attribution::Role {
        clawcrew_api::attribution::Role::Provider(clawcrew_api::attribution::ProviderKind::Model(
            clawcrew_api::attribution::ModelProviderKind::Custom,
        ))
    }

    fn alias(&self) -> &str {
        "FallbackModelProvider"
    }
}

impl AppState {
    #[must_use]
    pub fn from_config(config: Config) -> Self {
        Self::from_config_and_controls(config, None, None, None, None, None)
    }

    #[must_use]
    pub fn from_config_and_controls(
        config: Config,
        external_event_tx: Option<tokio::sync::broadcast::Sender<serde_json::Value>>,
        reload_controls: Option<galleon_infra::gateway_controls::GatewayReloadControls>,
        tui_registry: Option<Arc<dyn clawcrew_api::runtime_traits::TuiConnectionRegistry>>,
        canvas_store: Option<CanvasStore>,
        sop_runner: Option<Arc<dyn clawcrew_api::sop::SopRunner>>,
    ) -> Self {
        let memory: Arc<dyn Memory> = Arc::new(galleon_memory::NoneMemory::new("gateway-runtime"));
        let model_provider: Arc<dyn ModelProvider> =
            match galleon_providers::create_model_provider("ollama", None) {
                Ok(p) => Arc::from(p),
                Err(_) => Arc::new(FallbackModelProvider),
            };

        let event_tx = external_event_tx.unwrap_or_else(|| tokio::sync::broadcast::channel(16).0);
        let (shutdown_tx, reload_tx) = if let Some(rc) = reload_controls {
            (rc.shutdown_tx, Some(rc.reload_tx))
        } else {
            (tokio::sync::watch::channel(false).0, None)
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            config_write_lock: Arc::new(tokio::sync::Mutex::new(())),
            model_provider,
            model: "default".into(),
            temperature: None,
            mem: memory.clone(),
            memory_strategy: Arc::new(galleon_memory::DefaultMemoryStrategy::with_config(
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
            rate_limiter: Arc::new(crate::GatewayRateLimiter::new(100, 100, 100)),
            auth_limiter: Arc::new(crate::auth_rate_limit::AuthRateLimiter::new()),
            idempotency_store: Arc::new(crate::IdempotencyStore::new(
                std::time::Duration::from_secs(300),
                1000,
            )),
            #[cfg(feature = "channel-whatsapp-cloud")]
            whatsapp: HashMap::new(),
            #[cfg(feature = "channel-whatsapp-cloud")]
            whatsapp_app_secret: HashMap::new(),
            #[cfg(feature = "channel-linq")]
            linq: HashMap::new(),
            #[cfg(feature = "channel-linq")]
            linq_signing_secrets: HashMap::new(),
            #[cfg(feature = "channel-nextcloud")]
            nextcloud_talk: HashMap::new(),
            #[cfg(feature = "channel-nextcloud")]
            nextcloud_talk_webhook_secret: HashMap::new(),
            #[cfg(feature = "channel-email")]
            gmail_push: None,
            observer: Arc::new(clawcrew_api::observability_traits::NoopObserver),
            tools_registry: Arc::new(Vec::new()),
            tools_registry_by_agent: Arc::new(HashMap::new()),
            cost_tracker: None,
            event_tx,
            event_buffer: Arc::new(crate::sse::EventBuffer::new(16)),
            shutdown_tx,
            reload_tx,
            node_registry: Arc::new(crate::nodes::NodeRegistry::new(16)),
            mdns_peer_registry: crate::nodes::mdns::MdnsPeerRegistry::default(),
            path_prefix: String::new(),
            web_dist_dir: None,
            session_backend: None,
            session_queue: Arc::new(crate::session_queue::SessionActorQueue::new(8, 30, 600)),
            device_registry: None,
            pending_pairings: None,
            canvas_store: canvas_store.unwrap_or_else(CanvasStore::new),
            #[cfg(feature = "webauthn")]
            webauthn: None,
            cancel_tokens: Arc::new(std::sync::Mutex::new(HashMap::new())),
            pending_reload: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            tui_registry,
            sop_runner,
            diagnostics: None,
            quickstart: None,
            integration_catalog: None,
            personality_runner: None,
            skills_runner: None,
            acp_runner: None,
            control_plane: None,
        }
    }
}

/// Daemon-owned services whose lifecycle matches one supervised gateway run.
pub struct GatewaySupervision {
    pub readiness: Option<galleon_infra::gateway_controls::GatewayReadinessReporter>,
    pub plugin_webhooks: Arc<clawcrew_api::webhook::PluginWebhookRegistry>,
}

impl GatewaySupervision {
    #[must_use]
    pub fn new(
        readiness: Option<galleon_infra::gateway_controls::GatewayReadinessReporter>,
        plugin_webhooks: Arc<clawcrew_api::webhook::PluginWebhookRegistry>,
    ) -> Self {
        Self {
            readiness,
            plugin_webhooks,
        }
    }

    pub fn report_ready(&self, addr: std::net::SocketAddr) {
        if let Some(reporter) = &self.readiness {
            reporter.report_ready(addr);
        }
    }
}
