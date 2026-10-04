#![allow(
    clippy::to_string_in_format_args,
    clippy::useless_format,
    clippy::collapsible_if
)]

pub mod state;
pub mod gateway;
pub mod rate_limit;

#[cfg(feature = "a2a")]
pub mod a2a;
pub mod acp;
pub mod agent_owned_state;
pub mod api;
pub mod api_backup;
pub mod api_browse;
pub mod api_config;
pub mod api_logs;
pub mod api_pairing;
pub mod api_personality;
#[cfg(feature = "plugins-wasm")]
pub mod api_plugins;
pub mod api_quickstart;
pub mod api_sections;
pub mod api_skills;
pub mod api_sop;
pub mod api_sop_author;
mod api_sop_webhook;
pub mod api_tasks;
pub mod api_upload;
#[cfg(feature = "webauthn")]
pub mod api_webauthn;
#[cfg(any(
    feature = "channel-linq",
    feature = "channel-nextcloud",
    feature = "channel-whatsapp-cloud"
))]
pub mod api_webhook;
pub mod auth_rate_limit;
pub mod canvas;
pub mod hardware_context;
pub mod node_tool;
pub mod nodes;
pub mod openapi;
#[cfg(feature = "plugins-wasm")]
mod plugin_webhook;
pub mod security_headers;
pub mod session_queue;
pub mod sse;
pub mod static_files;
pub mod tls;
pub mod version;
pub mod grpc_system_gateway;
#[cfg(feature = "gateway-voice-duplex")]
pub mod voice_duplex;
#[cfg(any(
    feature = "channel-linq",
    feature = "channel-nextcloud",
    feature = "channel-whatsapp-cloud"
))]
mod webhook_ingress;
pub mod api_audit;
pub mod api_providers;
pub mod ws;
pub mod ws_approval;
pub mod ws_sop_runs;
pub mod handlers;

// Re-export key types from extracted modules
pub use state::{AppState, GatewaySupervision};
pub use gateway::{run_gateway, run_gateway_with_plugin_webhooks, start_grpc_server};
pub use rate_limit::{SlidingWindowRateLimiter, GatewayRateLimiter, IdempotencyStore};
pub use rate_limit::{RATE_LIMIT_WINDOW_SECS, RATE_LIMIT_MAX_KEYS_DEFAULT, IDEMPOTENCY_MAX_KEYS_DEFAULT};

// Types from other modules that extracted files need
pub use clawcrew_config::schema::Config;
pub use clawcrew_api::memory_traits::MemoryStrategy;
pub use clawcrew_runtime::tools::CanvasStore;
pub use clawcrew_config::pairing::PairingCodePolicy;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use axum::http::{header, HeaderMap, StatusCode};
use axum::Json;
use clawcrew_config::pairing::constant_time_eq;

// Config write guard is an OwnedMutexGuard
pub type ConfigWriteGuard = tokio::sync::OwnedMutexGuard<()>;

/// Gateway session key prefix to avoid collisions with channel sessions.
pub(crate) const GW_SESSION_PREFIX: &str = "gw_";

/// Return the canonical persistence key for a gateway session.
pub(crate) fn gateway_session_key(session_id: &str) -> String {
    format!(
        "{GW_SESSION_PREFIX}{}",
        clawcrew_api::session_keys::sanitize_session_key(session_id)
    )
}

/// Return the process-local cancellation key for a gateway session.
pub(crate) fn gateway_cancel_key(session_id: &str) -> String {
    format!("{GW_SESSION_PREFIX}{session_id}")
}

/// Register the current turn for a gateway session and cancel any replaced
/// turn before returning.
pub(crate) fn register_cancel_token(
    cancel_tokens: &Arc<
        std::sync::Mutex<
            std::collections::HashMap<String, Arc<tokio_util::sync::CancellationToken>>,
        >,
    >,
    session_key: &str,
    cancel_token: Arc<tokio_util::sync::CancellationToken>,
) {
    let mut guard = match cancel_tokens.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note),
                "cancel_tokens mutex was poisoned, recovering guard"
            );
            poisoned.into_inner()
        }
    };
    let previous_token = guard.insert(session_key.to_owned(), cancel_token);
    if let Some(previous_token) = previous_token {
        previous_token.cancel();
    }
}

/// Remove a turn's registry entry only while it still owns the session key.
pub(crate) fn remove_cancel_token_if_current(
    cancel_tokens: &Arc<
        std::sync::Mutex<
            std::collections::HashMap<String, Arc<tokio_util::sync::CancellationToken>>,
        >,
    >,
    session_key: &str,
    cancel_token: &Arc<tokio_util::sync::CancellationToken>,
) {
    let mut tokens = match cancel_tokens.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note),
                "cancel_tokens mutex was poisoned, recovering guard"
            );
            poisoned.into_inner()
        }
    };
    if tokens
        .get(session_key)
        .is_some_and(|current| Arc::ptr_eq(current, cancel_token))
    {
        tokens.remove(session_key);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdminReloadGate {
    Allow,
    RequireAuth,
    Forbidden,
    ForbiddenNoPairing,
}

pub fn admin_reload_gate(
    is_loopback: bool,
    allow_remote_admin: bool,
    require_pairing: bool,
) -> AdminReloadGate {
    if is_loopback {
        AdminReloadGate::Allow
    } else if !allow_remote_admin {
        AdminReloadGate::Forbidden
    } else if require_pairing {
        AdminReloadGate::RequireAuth
    } else {
        AdminReloadGate::ForbiddenNoPairing
    }
}

pub fn live_pairing_code_policy(state: &AppState) -> PairingCodePolicy {
    state.config.read().gateway.pairing_code
}

pub fn parse_client_ip(value: &str) -> Option<IpAddr> {
    let value = value.trim().trim_matches('"').trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip);
    }

    if let Ok(addr) = value.parse::<SocketAddr>() {
        return Some(addr.ip());
    }

    let value = value.trim_matches(['[', ']']);
    value.parse::<IpAddr>().ok()
}

pub fn forwarded_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(xff) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        for candidate in xff.split(',') {
            if let Some(ip) = parse_client_ip(candidate) {
                return Some(ip);
            }
        }
    }

    headers
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_client_ip)
}

pub fn client_key_from_request(
    peer: Option<SocketAddr>,
    headers: &HeaderMap,
    trust_forwarded: bool,
) -> String {
    if trust_forwarded {
        if let Some(ip) = forwarded_client_ip(headers) {
            return ip.to_string();
        }
    }
    peer.map(|p| p.ip().to_string()).unwrap_or_else(|| "unknown".to_string())
}

pub(crate) async fn persist_pairing_tokens(
    config: Arc<parking_lot::RwLock<clawcrew_config::schema::Config>>,
    pairing: &clawcrew_runtime::security::pairing::PairingGuard,
    config_write_lock: Arc<tokio::sync::Mutex<()>>,
) -> anyhow::Result<()> {
    let _guard = config_write_lock.lock_owned().await;
    let paired_tokens = pairing.tokens();
    let mut updated_cfg = { config.read().clone() };
    updated_cfg.gateway.paired_tokens = paired_tokens;
    updated_cfg.mark_dirty("gateway.paired_tokens");
    updated_cfg.save_dirty().await?;
    *config.write() = updated_cfg;
    Ok(())
}

pub fn needs_quickstart_for(model: &str) -> Option<anyhow::Error> {
    if model.trim().is_empty() {
        Some(anyhow::Error::msg("needs_quickstart: gateway has no model configured"))
    } else {
        None
    }
}

pub fn needs_quickstart_channel_reply() -> &'static str {
    "Quickstart configuration required: set [providers.models.<type>.<alias>] model = \"...\""
}

pub type WebhookJsonResponse = (StatusCode, Json<serde_json::Value>);

fn hash_webhook_secret(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)
}

fn configured_gateway_webhook_secret_hash(state: &AppState) -> Option<String> {
    state
        .config
        .read()
        .gateway
        .webhook_secret
        .as_deref()
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
        .map(hash_webhook_secret)
}

#[derive(Debug, Clone, Copy)]
pub struct WebhookAuthVerdict {
    pairing_verified: bool,
    secret_verified: bool,
    any_control_configured: bool,
}

impl WebhookAuthVerdict {
    pub fn may_dispatch_sop(self) -> bool {
        self.any_control_configured && (self.pairing_verified || self.secret_verified)
    }
}

pub fn authorize_webhook_request(
    state: &AppState,
    peer_addr: SocketAddr,
    headers: &HeaderMap,
) -> Result<WebhookAuthVerdict, WebhookJsonResponse> {
    let rate_key = client_key_from_request(Some(peer_addr), headers, state.trust_forwarded_headers);
    if !state.rate_limiter.allow_webhook(&rate_key) {
        let err = serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(err)));
    }

    let snapshot_secret_hash = configured_gateway_webhook_secret_hash(state);
    let require_pairing = state.pairing.require_pairing();
    let any_control_configured = require_pairing || snapshot_secret_hash.is_some();
    let mut pairing_verified = false;
    let mut secret_verified = false;

    if require_pairing {
        if let Err(e) = state.auth_limiter.check_rate_limit(&rate_key) {
            let err = serde_json::json!({
                "error": format!("Too many auth attempts. Try again in {}s.", e.retry_after_secs),
                "retry_after": e.retry_after_secs,
            });
            return Err((StatusCode::TOO_MANY_REQUESTS, Json(err)));
        }
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            state.auth_limiter.record_attempt(&rate_key);
            let err = serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            });
            return Err((StatusCode::UNAUTHORIZED, Json(err)));
        }
        pairing_verified = true;
    }

    if let Some(secret_hash) = snapshot_secret_hash.as_deref() {
        let header_hash = headers
            .get("X-Webhook-Secret")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(hash_webhook_secret);
        match header_hash {
            Some(val) if constant_time_eq(&val, secret_hash) => {
                secret_verified = true;
            }
            _ => {
                let err = serde_json::json!({"error": "Unauthorized — invalid or missing X-Webhook-Secret header"});
                return Err((StatusCode::UNAUTHORIZED, Json(err)));
            }
        }
    }

    Ok(WebhookAuthVerdict {
        pairing_verified,
        secret_verified,
        any_control_configured,
    })
}

fn idempotency_storage_key(namespace: Option<&str>, idempotency_key: &str) -> String {
    fn push_component(out: &mut String, value: &str) {
        out.push_str(&value.len().to_string());
        out.push(':');
        out.push_str(value);
    }

    let mut out = String::new();
    push_component(&mut out, namespace.unwrap_or(""));
    push_component(&mut out, idempotency_key);
    out
}

pub fn check_webhook_idempotency(
    state: &AppState,
    headers: &HeaderMap,
    namespace: Option<&str>,
) -> Option<WebhookJsonResponse> {
    let idempotency_key = headers
        .get("X-Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let storage_key = idempotency_storage_key(namespace, idempotency_key);
    if state.idempotency_store.record_if_new(&storage_key) {
        return None;
    }

    Some((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "duplicate",
            "idempotent": true,
            "message": "A prior request already reserved this idempotency key; no new dispatch was started"
        })),
    ))
}

pub fn require_sop_dispatch_credentials(
    verdict: WebhookAuthVerdict,
) -> Result<(), WebhookJsonResponse> {
    if verdict.may_dispatch_sop() {
        return Ok(());
    }
    let err = serde_json::json!({
        "error": "SOP webhook dispatch requires a configured credential: set `gateway.require_pairing = true` or `gateway.webhook_secret`"
    });
    Err((StatusCode::FORBIDDEN, Json(err)))
}

#[derive(Debug, Clone)]
pub struct GatewayChatOutcome {
    pub response: String,
}

fn resolve_gateway_chat_agent_alias(
    config: &Config,
    agent_override: Option<&str>,
) -> Option<String> {
    agent_override
        .map(ToString::to_string)
        .or_else(|| config.resolved_runtime_agent_alias().map(str::to_owned))
}

fn require_gateway_chat_agent_alias(
    config: &Config,
    agent_override: Option<&str>,
) -> anyhow::Result<String> {
    resolve_gateway_chat_agent_alias(config, agent_override).ok_or_else(|| {
        anyhow::Error::msg("webhook chat requires at least one configured [agents.<alias>] entry")
    })
}

pub async fn run_gateway_chat_with_tools(
    state: &AppState,
    message: &str,
    session_id: Option<&str>,
    agent_override: Option<&str>,
) -> anyhow::Result<GatewayChatOutcome> {
    if let Some(err) = needs_quickstart_for(&state.model) {
        return Err(err);
    }

    let config = state.config.read().clone();
    let agent_alias = require_gateway_chat_agent_alias(&config, agent_override)?;

    let cost_tracking_context = state.cost_tracker.as_ref().map(|tracker| {
        let pricing = clawcrew_runtime::agent::cost::build_model_provider_pricing(&config);
        clawcrew_runtime::agent::cost::ToolLoopCostTrackingContext::new(
            tracker.clone(),
            std::sync::Arc::new(pricing),
        )
        .with_agent_alias(&agent_alias)
    });
    let turn_usage = state.cost_tracker.as_ref().map(|_| {
        std::sync::Arc::new(parking_lot::Mutex::new(
            clawcrew_runtime::agent::cost::TurnUsage::default(),
        ))
    });
    let response = Box::pin(clawcrew_runtime::agent::cost::TOOL_LOOP_TURN_USAGE.scope(
        turn_usage.clone(),
        clawcrew_runtime::agent::cost::TOOL_LOOP_COST_TRACKING_CONTEXT.scope(
            cost_tracking_context,
            clawcrew_runtime::agent::process_message(
                config,
                &agent_alias,
                message,
                session_id,
                clawcrew_api::ingress::TurnOrigin::Interactive,
            ),
        ),
    ))
    .await?;
    Ok(GatewayChatOutcome { response })
}

#[derive(Debug)]
pub struct UnconfiguredModelProvider;

impl clawcrew_api::attribution::Attributable for UnconfiguredModelProvider {
    fn role(&self) -> clawcrew_api::attribution::Role {
        clawcrew_api::attribution::Role::Provider(clawcrew_api::attribution::ProviderKind::Model(
            clawcrew_api::attribution::ModelProviderKind::OpenAi,
        ))
    }
    fn alias(&self) -> &str {
        "unconfigured"
    }
}

#[async_trait::async_trait]
impl clawcrew_api::model_provider::ModelProvider for UnconfiguredModelProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: Option<f64>,
    ) -> anyhow::Result<String> {
        anyhow::bail!("model provider is not configured")
    }
}
