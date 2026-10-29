#![allow(
    clippy::to_string_in_format_args,
    clippy::useless_format,
    clippy::collapsible_if,
    clippy::result_large_err,
    unused,
    dead_code
)]

pub mod gateway;
pub mod rate_limit;
pub mod state;

#[cfg(feature = "a2a")]
pub mod a2a;
pub mod acp;
pub mod agent_owned_state;
pub mod api;
pub mod api_admin;
pub mod api_audit;
pub mod api_backup;
pub mod api_browse;
pub mod api_config;
pub mod api_fleet;
pub mod api_logs;
pub mod api_pairing;
pub mod api_personality;
#[cfg(feature = "plugins-wasm")]
pub mod api_plugins;
pub mod api_providers;
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
pub mod cron_adapter;
pub mod grpc_system_gateway;
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
pub mod strangler_runner;
pub mod tls;
pub mod version;
#[cfg(feature = "gateway-voice-duplex")]
pub mod voice_duplex;
#[cfg(any(
    feature = "channel-linq",
    feature = "channel-nextcloud",
    feature = "channel-whatsapp-cloud"
))]
mod webhook_ingress;
pub mod ws;

#[cfg(test)]
pub(crate) mod tests;

/// Maximum request body size (64KB) — prevents memory exhaustion
pub const MAX_BODY_SIZE: usize = 65_536;
/// Default request timeout (30s) — prevents slow-loris attacks.
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
pub const LONG_RUNNING_REQUEST_TIMEOUT_SECS: u64 = 600;

/// Gateway request timeout (seconds) for routes other than the long-running
/// cron-trigger endpoint. Reads from typed config.
pub fn gateway_request_timeout_secs(cfg: &galleon_config::schema::GatewayConfig) -> u64 {
    cfg.request_timeout_secs
}

/// Manual cron-trigger request timeout (seconds), exempt from the
/// gateway-wide [`gateway_request_timeout_secs`] limit so synchronous agent
/// jobs can run to completion. Reads from typed config.
pub fn gateway_long_running_request_timeout_secs(
    cfg: &galleon_config::schema::GatewayConfig,
) -> u64 {
    cfg.long_running_request_timeout_secs
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebDashboardAvailability {
    Embedded,
    Filesystem(std::path::PathBuf),
}

pub fn resolve_web_dashboard_availability(
    config: &galleon_config::schema::Config,
) -> Option<WebDashboardAvailability> {
    #[cfg(feature = "embedded-web")]
    {
        let _ = config;
        Some(WebDashboardAvailability::Embedded)
    }
    #[cfg(not(feature = "embedded-web"))]
    {
        resolve_web_dist_dir(config).map(WebDashboardAvailability::Filesystem)
    }
}

pub(crate) fn has_servable_dashboard_index(dir: &std::path::Path) -> bool {
    let Ok(canonical_root) = std::fs::canonicalize(dir) else {
        return false;
    };
    let Ok(canonical_index) = std::fs::canonicalize(dir.join("index.html")) else {
        return false;
    };
    canonical_index.starts_with(&canonical_root) && canonical_index.is_file()
}

pub fn resolve_web_dist_dir(config: &galleon_config::schema::Config) -> Option<std::path::PathBuf> {
    match config
        .gateway
        .web_dist_dir
        .as_ref()
        .map(std::path::PathBuf::from)
    {
        Some(dir) if has_servable_dashboard_index(&dir) => Some(dir),
        _ => None,
    }
}

pub fn static_file_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/_app/", axum::routing::get(static_files::handle_static))
        .route(
            "/_app/{*path}",
            axum::routing::get(static_files::handle_static),
        )
}

pub(crate) fn normalize_max_keys(configured: usize, fallback: usize) -> usize {
    if configured == 0 {
        fallback.max(1)
    } else {
        configured
    }
}

pub(crate) fn default_agent_alias(config: &galleon_config::schema::Config) -> Option<String> {
    config
        .agents
        .iter()
        .filter(|(_, a)| a.enabled)
        .map(|(alias, _)| alias.clone())
        .min()
}

pub(crate) fn format_paircode_recovery_command(_host: &str, port: u16) -> String {
    format!("clawcrew gateway get-paircode --new --port {port}")
}

pub(crate) fn already_paired_pairing_notice(
    host: &str,
    port: u16,
    path_prefix: &str,
) -> Vec<String> {
    vec![
        "  🔒 Pairing: ACTIVE — this gateway is already paired, so no new \
         one-time code was generated on this start."
            .to_string(),
        format!(
            "     To pair another device, run: {}",
            format_paircode_recovery_command(host, port)
        ),
        format!(
            "     Fallback (localhost only): {}",
            format_paircode_recovery_curl(host, port, path_prefix)
        ),
    ]
}

pub(crate) fn format_paircode_recovery_curl(host: &str, port: u16, path_prefix: &str) -> String {
    let recovery_host = paircode_recovery_curl_host(host);
    format!("curl -s -X POST http://{recovery_host}:{port}{path_prefix}/admin/paircode/new")
}

pub(crate) fn paircode_recovery_curl_host(host: &str) -> &str {
    match host {
        "127.0.0.1" | "localhost" => host,
        "::1" => "[::1]",
        _ => "127.0.0.1",
    }
}

#[cfg(unix)]
const EMFILE: i32 = 24;
#[cfg(unix)]
const ENFILE: i32 = 23;

pub(crate) fn is_recoverable_accept_error(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    if matches!(
        e.kind(),
        ErrorKind::ConnectionAborted | ErrorKind::Interrupted | ErrorKind::WouldBlock
    ) {
        return true;
    }
    #[cfg(unix)]
    if let Some(code) = e.raw_os_error() {
        return code == EMFILE || code == ENFILE;
    }
    false
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GatewayChatDispatchCapture {
    pub(crate) message: String,
    pub(crate) session_id: Option<String>,
    pub(crate) agent_override: Option<String>,
}

#[cfg(test)]
static GATEWAY_CHAT_DISPATCH_CAPTURES: std::sync::Mutex<Vec<GatewayChatDispatchCapture>> =
    std::sync::Mutex::new(Vec::new());

#[cfg(test)]
static GATEWAY_CHAT_DISPATCH_CAPTURE_TEST_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

#[cfg(test)]
pub(crate) async fn lock_gateway_chat_dispatch_capture_for_test()
-> tokio::sync::MutexGuard<'static, ()> {
    GATEWAY_CHAT_DISPATCH_CAPTURE_TEST_LOCK.lock().await
}

#[cfg(test)]
pub(crate) fn clear_gateway_chat_dispatch_captures_for_test() {
    GATEWAY_CHAT_DISPATCH_CAPTURES
        .lock()
        .expect("gateway chat dispatch capture mutex poisoned")
        .clear();
}

#[cfg(test)]
pub(crate) fn gateway_chat_dispatch_captures_for_test() -> Vec<GatewayChatDispatchCapture> {
    GATEWAY_CHAT_DISPATCH_CAPTURES
        .lock()
        .expect("gateway chat dispatch capture mutex poisoned")
        .clone()
}

#[cfg(test)]
pub(crate) fn record_gateway_chat_dispatch_capture_for_test(
    message: &str,
    session_id: Option<&str>,
    agent_override: Option<&str>,
) {
    GATEWAY_CHAT_DISPATCH_CAPTURES
        .lock()
        .expect("gateway chat dispatch capture mutex poisoned")
        .push(GatewayChatDispatchCapture {
            message: message.to_string(),
            session_id: session_id.map(ToString::to_string),
            agent_override: agent_override.map(ToString::to_string),
        });
}
pub mod handlers;
pub mod ws_approval;
pub mod ws_sop_runs;

// Re-export key types from extracted modules
pub use api_admin::*;
pub use api_config::{
    ConfigDriftSummary, DriftEntry, compute_drift, persist_and_swap, try_compute_drift,
};
pub use api_pairing::{handle_pair, handle_pair_code};
pub use gateway::{
    build_gateway_router, handle_health, handle_ready, run_gateway,
    run_gateway_with_plugin_webhooks, start_grpc_server,
};
pub use rate_limit::{GatewayRateLimiter, IdempotencyStore, SlidingWindowRateLimiter};
pub use rate_limit::{
    IDEMPOTENCY_MAX_KEYS_DEFAULT, RATE_LIMIT_MAX_KEYS_DEFAULT, RATE_LIMIT_WINDOW_SECS,
};
pub use state::{AppState, GatewaySupervision};
pub use strangler_runner::{
    StranglerAgentRunner, StranglerAgentSession, install_strangler_agent_runner,
};

// Types from other modules that extracted files need
pub use galleon_api::memory_traits::MemoryStrategy;
pub use galleon_config::pairing::PairingCodePolicy;
pub use galleon_config::schema::Config;
pub use galleon_tools::canvas::CanvasStore;

use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use galleon_api::channel::Channel;
use galleon_config::pairing::constant_time_eq;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

// Config write guard is an OwnedMutexGuard
pub type ConfigWriteGuard = tokio::sync::OwnedMutexGuard<()>;

/// Gateway session key prefix to avoid collisions with channel sessions.
pub(crate) const GW_SESSION_PREFIX: &str = "gw_";

/// Return the canonical persistence key for a gateway session.
pub(crate) fn gateway_session_key(session_id: &str) -> String {
    format!(
        "{GW_SESSION_PREFIX}{}",
        galleon_api::session_keys::sanitize_session_key(session_id)
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
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
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
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
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
    peer.map(|p| p.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) async fn persist_pairing_tokens(
    config: Arc<parking_lot::RwLock<galleon_config::schema::Config>>,
    pairing: &galleon_config::pairing::PairingGuard,
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
        Some(anyhow::Error::msg(
            "needs_quickstart: gateway has no model configured. Complete \
             browser quickstart at /quickstart, or set [providers.models.<type>.<alias>] \
             model = \"...\" before sending messages.",
        ))
    } else {
        None
    }
}

pub fn needs_quickstart_channel_reply() -> &'static str {
    "Quickstart configuration required: set [providers.models.<type>.<alias>] model = \"...\""
}

/// True when `e` carries the marker produced by `needs_quickstart_for`.
pub fn is_needs_quickstart_err(e: &anyhow::Error) -> bool {
    e.to_string().contains("needs_quickstart")
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

    let mut key = String::new();
    match namespace {
        Some(namespace) => {
            push_component(&mut key, "ns");
            push_component(&mut key, namespace);
        }
        None => push_component(&mut key, "global"),
    }
    push_component(&mut key, idempotency_key);
    key
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
        "error": "SOP webhook dispatch requires a configured credential: set \
                  `gateway.require_pairing = true` and authenticate with \
                  `Authorization: Bearer <paired-token>` (pair first via POST /pair), or set \
                  `gateway.webhook_secret` and send X-Webhook-Secret."
    });
    Err((StatusCode::UNAUTHORIZED, Json(err)))
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

pub fn optional_channel_routes() -> Router<AppState> {
    let router: Router<AppState> = Router::new();
    #[cfg(feature = "channel-whatsapp-cloud")]
    let router = router
        .route("/whatsapp", get(handle_whatsapp_verify))
        .route("/whatsapp", post(handle_whatsapp_message))
        .route("/whatsapp/{alias}", get(handle_whatsapp_verify_alias))
        .route("/whatsapp/{alias}", post(handle_whatsapp_message_alias));
    #[cfg(feature = "channel-linq")]
    let router = router
        .route("/linq", post(handle_linq_webhook))
        .route("/linq/{alias}", post(handle_linq_webhook_alias));
    #[cfg(feature = "channel-nextcloud")]
    let router = router
        .route("/nextcloud-talk", post(handle_nextcloud_talk_webhook))
        .route(
            "/nextcloud-talk/{alias}",
            post(handle_nextcloud_talk_webhook_alias),
        );
    #[cfg(feature = "channel-email")]
    let router = router.route("/webhook/gmail", post(handle_gmail_push_webhook));
    router
}

pub fn sop_webhook_routes() -> Router<AppState> {
    Router::new().route("/sop/{*rest}", post(api_sop_webhook::handle_sop_webhook))
}

#[cfg(any(
    feature = "channel-linq",
    feature = "channel-nextcloud",
    feature = "channel-whatsapp-cloud"
))]
pub(crate) fn sender_session_id(
    channel: &str,
    msg: &galleon_api::channel::ChannelMessage,
) -> String {
    match &msg.thread_ts {
        Some(thread_id) => format!("{channel}_{thread_id}_{}", msg.sender),
        None => format!("{channel}_{}", msg.sender),
    }
}

#[cfg(feature = "channel-whatsapp-cloud")]
fn whatsapp_memory_key(msg: &galleon_api::channel::ChannelMessage) -> String {
    format!("whatsapp_{}_{}", msg.sender, msg.id)
}

#[cfg(feature = "channel-linq")]
fn linq_memory_key(msg: &galleon_api::channel::ChannelMessage) -> String {
    format!("linq_{}_{}", msg.sender, msg.id)
}

#[cfg(feature = "channel-linq")]
fn linq_channel_ref(alias: &str) -> String {
    format!("linq.{alias}")
}

#[cfg(feature = "channel-nextcloud")]
fn nextcloud_talk_memory_key(msg: &galleon_api::channel::ChannelMessage) -> String {
    format!("nextcloud_talk_{}_{}", msg.sender, msg.id)
}

/// Verify `WhatsApp` webhook signature (`X-Hub-Signature-256`).
/// Returns true if the signature is valid, false otherwise.
/// See: <https://developers.facebook.com/docs/graph-api/webhooks/getting-started#verification-requests>
pub fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Signature format: "sha256=<hex_signature>"
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    // Decode hex signature
    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    // Compute HMAC-SHA256
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected).is_ok()
}

/// `WhatsApp` verification query params
#[derive(serde::Deserialize)]
pub struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

/// GET /whatsapp — Meta webhook verification (bare path, deprecated fallback).
#[cfg(feature = "channel-whatsapp-cloud")]
pub(crate) async fn handle_whatsapp_verify(
    State(state): State<AppState>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> Response {
    handle_whatsapp_verify_impl(state, None, params).await
}

/// GET /whatsapp/{alias} — Meta webhook verification for a specific instance.
#[cfg(feature = "channel-whatsapp-cloud")]
pub(crate) async fn handle_whatsapp_verify_alias(
    State(state): State<AppState>,
    Path(alias): Path<String>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> Response {
    handle_whatsapp_verify_impl(state, Some(alias), params).await
}

#[cfg(feature = "channel-whatsapp-cloud")]
async fn handle_whatsapp_verify_impl(
    state: AppState,
    alias: Option<String>,
    params: WhatsAppVerifyQuery,
) -> Response {
    let resolved = api_webhook::resolve(&state.whatsapp, alias.as_deref());
    let Some((_alias, wa)) = resolved.entry() else {
        return api_webhook::not_found("whatsapp");
    };

    // Verify the token matches (constant-time comparison to prevent timing attacks)
    let token_matches = params
        .verify_token
        .as_deref()
        .is_some_and(|t| constant_time_eq(t, wa.verify_token()));
    let resp = if params.mode.as_deref() == Some("subscribe") && token_matches {
        if let Some(ch) = params.challenge {
            ::galleon_log::record!(
                INFO,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_attrs(::serde_json::json!({"channel": "whatsapp"})),
                "webhook verified successfully"
            );
            (StatusCode::OK, ch).into_response()
        } else {
            (StatusCode::BAD_REQUEST, "Missing hub.challenge".to_string()).into_response()
        }
    } else {
        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                .with_outcome(::galleon_log::EventOutcome::Unknown)
                .with_attrs(::serde_json::json!({"channel": "whatsapp"})),
            "webhook verification failed — token mismatch"
        );
        (StatusCode::FORBIDDEN, "Forbidden".to_string()).into_response()
    };
    api_webhook::tag_deprecation(resp, resolved, "whatsapp")
}

/// POST /whatsapp — incoming message webhook (bare path, deprecated fallback).
#[cfg(feature = "channel-whatsapp-cloud")]
pub(crate) async fn handle_whatsapp_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_whatsapp_message_impl(state, None, headers, body).await
}

/// POST /whatsapp/{alias} — incoming message webhook for a specific instance.
#[cfg(feature = "channel-whatsapp-cloud")]
pub(crate) async fn handle_whatsapp_message_alias(
    State(state): State<AppState>,
    Path(alias): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_whatsapp_message_impl(state, Some(alias), headers, body).await
}

#[cfg(feature = "channel-whatsapp-cloud")]
async fn handle_whatsapp_message_impl(
    state: AppState,
    alias: Option<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let resolved = api_webhook::resolve(&state.whatsapp, alias.as_deref());
    let Some((alias_key, wa)) = resolved.entry() else {
        return api_webhook::not_found("whatsapp");
    };
    let app_secret = state.whatsapp_app_secret.get(alias_key).cloned();
    let resp =
        process_whatsapp_message(&state, alias_key, wa, app_secret.as_deref(), headers, body).await;
    api_webhook::tag_deprecation(resp.into_response(), resolved, "whatsapp")
}

/// Verify, parse, and dispatch a WhatsApp webhook payload for one resolved
/// instance. `app_secret` is that instance's `X-Hub-Signature-256` secret.
#[cfg(feature = "channel-whatsapp-cloud")]
async fn process_whatsapp_message(
    state: &AppState,
    alias: &str,
    wa: &Arc<galleon_channels::whatsapp::WhatsAppChannel>,
    app_secret: Option<&str>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    let verified = match webhook_ingress::authenticate(
        &webhook_ingress::WHATSAPP_WEBHOOK,
        alias,
        app_secret,
        &headers,
        body,
        |secret, headers, body| {
            let signature = headers
                .get("X-Hub-Signature-256")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            verify_whatsapp_signature(secret, body, signature)
        },
    ) {
        Ok(verified) => verified,
        Err(refusal) => return refusal.into_response(&webhook_ingress::WHATSAPP_WEBHOOK),
    };

    let mut verified = match verified.parse_messages(|body| {
        let payload = serde_json::from_slice::<serde_json::Value>(body).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid JSON payload"})),
            )
        })?;
        Ok::<_, (StatusCode, Json<serde_json::Value>)>(wa.parse_webhook_payload(&payload))
    }) {
        Ok(verified) => verified,
        Err(response) => return response,
    };

    // Route approval replies to pending approval requests before dispatching
    // to the agent.
    let mut approvals = wa.pending_approvals().lock().await;
    verified.retain(|msg| {
        let Some((token, response)) = galleon_channels::util::parse_approval_reply(&msg.content)
        else {
            return true;
        };
        let Some(sender) = approvals.remove(&token) else {
            return true;
        };
        let _ = sender.send(response);
        false
    });
    drop(approvals);

    let channel: Arc<dyn Channel> = wa.clone();
    webhook_ingress::dispatch_verified_webhook(
        state,
        verified,
        webhook_ingress::WebhookDispatchContext {
            channel,
            memory_key: whatsapp_memory_key,
            agent_override: None,
            mode: webhook_ingress::WebhookDispatchMode::Synchronous,
            #[cfg(test)]
            suppress_reply_send: true,
        },
    )
    .await
}

/// POST /linq — incoming message webhook (bare path, deprecated fallback).
#[cfg(feature = "channel-linq")]
pub(crate) async fn handle_linq_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_linq_webhook_impl(state, None, headers, body).await
}

/// POST /linq/{alias} — incoming message webhook for a specific instance.
#[cfg(feature = "channel-linq")]
pub(crate) async fn handle_linq_webhook_alias(
    State(state): State<AppState>,
    Path(alias): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_linq_webhook_impl(state, Some(alias), headers, body).await
}

#[cfg(feature = "channel-linq")]
async fn handle_linq_webhook_impl(
    state: AppState,
    alias: Option<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let resolved = api_webhook::resolve(&state.linq, alias.as_deref());
    let Some((alias_key, linq)) = resolved.entry() else {
        return api_webhook::not_found("linq");
    };
    let signing_secret = state.linq_signing_secrets.get(alias_key).cloned();
    let resp = process_linq_webhook(
        &state,
        alias_key,
        linq,
        signing_secret.as_deref(),
        headers,
        body,
    )
    .await;
    api_webhook::tag_deprecation(resp.into_response(), resolved, "linq")
}

/// Verify, parse, and dispatch a Linq webhook payload for one resolved instance.
/// `signing_secret` is that instance's `X-Webhook-Signature` secret.
#[cfg(feature = "channel-linq")]
async fn process_linq_webhook(
    state: &AppState,
    alias: &str,
    linq: &Arc<galleon_channels::linq::LinqChannel>,
    signing_secret: Option<&str>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    let verified = match webhook_ingress::authenticate(
        &webhook_ingress::LINQ_WEBHOOK,
        alias,
        signing_secret,
        &headers,
        body,
        |secret, headers, body| {
            let body_str = String::from_utf8_lossy(body);
            let timestamp = headers
                .get("X-Webhook-Timestamp")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let signature = headers
                .get("X-Webhook-Signature")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            galleon_channels::linq::verify_linq_signature(secret, &body_str, timestamp, signature)
        },
    ) {
        Ok(verified) => verified,
        Err(refusal) => return refusal.into_response(&webhook_ingress::LINQ_WEBHOOK),
    };

    let verified = match verified.parse_messages(|body| {
        let payload = serde_json::from_slice::<serde_json::Value>(body).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid JSON payload"})),
            )
        })?;
        Ok::<_, (StatusCode, Json<serde_json::Value>)>(linq.parse_webhook_payload(&payload))
    }) {
        Ok(verified) => verified,
        Err(response) => return response,
    };

    if verified.is_empty() {
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
    }

    let channel_ref = linq_channel_ref(alias);
    let (agent_override, has_channel_bindings) = {
        let config = state.config.read();
        (
            config.agent_for_channel(&channel_ref).map(str::to_owned),
            config
                .agents
                .values()
                .any(|agent| !agent.channels.is_empty()),
        )
    };
    if agent_override.is_none() && has_channel_bindings {
        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Reject)
                .with_outcome(::galleon_log::EventOutcome::Failure)
                .with_attrs(::serde_json::json!({"channel": "linq", "alias": alias})),
            "Linq webhook ignored because no enabled agent owns the channel alias"
        );
        return (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ignored", "reason": "no_agent_for_channel"})),
        );
    }

    let channel: Arc<dyn Channel> = linq.clone();
    webhook_ingress::dispatch_verified_webhook(
        state,
        verified,
        webhook_ingress::WebhookDispatchContext {
            channel,
            memory_key: linq_memory_key,
            agent_override,
            mode: webhook_ingress::WebhookDispatchMode::Synchronous,
            #[cfg(test)]
            suppress_reply_send: true,
        },
    )
    .await
}

/// POST /nextcloud-talk — incoming message webhook (bare path, deprecated).
#[cfg(feature = "channel-nextcloud")]
pub(crate) async fn handle_nextcloud_talk_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_nextcloud_talk_webhook_impl(state, None, headers, body).await
}

/// POST /nextcloud-talk/{alias} — incoming message webhook for one instance.
#[cfg(feature = "channel-nextcloud")]
pub(crate) async fn handle_nextcloud_talk_webhook_alias(
    State(state): State<AppState>,
    Path(alias): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_nextcloud_talk_webhook_impl(state, Some(alias), headers, body).await
}

#[cfg(feature = "channel-nextcloud")]
async fn handle_nextcloud_talk_webhook_impl(
    state: AppState,
    alias: Option<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let resolved = api_webhook::resolve(&state.nextcloud_talk, alias.as_deref());
    let Some((alias_key, nextcloud_talk)) = resolved.entry() else {
        return api_webhook::not_found("nextcloud-talk");
    };
    let webhook_secret = state.nextcloud_talk_webhook_secret.get(alias_key).cloned();
    let resp = process_nextcloud_talk_webhook(
        &state,
        alias_key,
        nextcloud_talk,
        webhook_secret.as_deref(),
        headers,
        body,
    )
    .await;
    api_webhook::tag_deprecation(resp.into_response(), resolved, "nextcloud-talk")
}

/// Verify, parse, and dispatch a Nextcloud Talk webhook payload for one resolved
/// instance. `webhook_secret` is that instance's HMAC signing secret.
#[cfg(feature = "channel-nextcloud")]
async fn process_nextcloud_talk_webhook(
    state: &AppState,
    alias: &str,
    nextcloud_talk: &Arc<galleon_channels::nextcloud_talk::NextcloudTalkChannel>,
    webhook_secret: Option<&str>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    let verified = match webhook_ingress::authenticate(
        &webhook_ingress::NEXTCLOUD_TALK_WEBHOOK,
        alias,
        webhook_secret,
        &headers,
        body,
        |secret, headers, body| {
            let body_str = String::from_utf8_lossy(body);
            let random = headers
                .get("X-Nextcloud-Talk-Random")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let signature = headers
                .get("X-Nextcloud-Talk-Signature")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            galleon_channels::nextcloud_talk::verify_nextcloud_talk_signature(
                secret, random, &body_str, signature,
            )
        },
    ) {
        Ok(verified) => verified,
        Err(refusal) => return refusal.into_response(&webhook_ingress::NEXTCLOUD_TALK_WEBHOOK),
    };

    let verified = match verified.parse_messages(|body| {
        let payload = serde_json::from_slice::<serde_json::Value>(body).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid JSON payload"})),
            )
        })?;
        Ok::<_, (StatusCode, Json<serde_json::Value>)>(
            nextcloud_talk.parse_webhook_payload(&payload),
        )
    }) {
        Ok(verified) => verified,
        Err(response) => return response,
    };

    let channel: Arc<dyn Channel> = nextcloud_talk.clone();
    webhook_ingress::dispatch_verified_webhook(
        state,
        verified,
        webhook_ingress::WebhookDispatchContext {
            channel,
            memory_key: nextcloud_talk_memory_key,
            agent_override: None,
            mode: webhook_ingress::WebhookDispatchMode::FastAck,
            #[cfg(test)]
            suppress_reply_send: true,
        },
    )
    .await
}

/// Maximum request body size for the Gmail webhook endpoint (1 MB).
#[cfg(feature = "channel-email")]
const GMAIL_WEBHOOK_MAX_BODY: usize = 1024 * 1024;

/// POST /webhook/gmail — incoming Gmail Pub/Sub push notification
#[cfg(feature = "channel-email")]
pub(crate) async fn handle_gmail_push_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let Some(ref gmail_push) = state.gmail_push else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Gmail push not configured"})),
        );
    };

    if body.len() > GMAIL_WEBHOOK_MAX_BODY {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "Request body too large"})),
        );
    }

    let secret = gmail_push.config.webhook_secret.clone();
    if !secret.is_empty() {
        let provided = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|auth| auth.strip_prefix("Bearer "))
            .unwrap_or("");

        if provided != secret {
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_outcome(::galleon_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({"channel": "gmail_push"})),
                "webhook: unauthorized request"
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            );
        }
    }

    let body_str = String::from_utf8_lossy(&body);
    let envelope: galleon_channels::gmail_push::PubSubEnvelope =
        match serde_json::from_str(&body_str) {
            Ok(e) => e,
            Err(e) => {
                ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_outcome(::galleon_log::EventOutcome::Unknown)
                    .with_attrs(
                        ::serde_json::json!({"error": format!("{}", e), "channel": "gmail_push"})
                    ),
                "webhook: invalid payload"
            );
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid Pub/Sub envelope"})),
                );
            }
        };

    let channel = Arc::clone(gmail_push);
    galleon_spawn::spawn!(async move {
        if let Err(e) = channel.handle_notification(&envelope).await {
            ::galleon_log::record!(
                ERROR,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
                    .with_outcome(::galleon_log::EventOutcome::Failure)
                    .with_attrs(
                        ::serde_json::json!({"channel": "gmail_push", "error": format!("{}", e)})
                    ),
                "push notification processing failed"
            );
        }
    });

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
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
        let pricing = galleon_config::cost::build_model_provider_pricing(&config);
        galleon_config::cost::ToolLoopCostTrackingContext::new(
            tracker.clone(),
            std::sync::Arc::new(pricing),
        )
        .with_agent_alias(&agent_alias)
    });
    let turn_usage = state.cost_tracker.as_ref().map(|_| {
        std::sync::Arc::new(parking_lot::Mutex::new(
            galleon_config::cost::TurnUsage::default(),
        ))
    });
    #[cfg(test)]
    record_gateway_chat_dispatch_capture_for_test(message, session_id, agent_override);

    let response = Box::pin(galleon_config::cost::TOOL_LOOP_TURN_USAGE.scope(
        turn_usage.clone(),
        galleon_config::cost::TOOL_LOOP_COST_TRACKING_CONTEXT.scope(
            cost_tracking_context,
            galleon_api::agent::process_message(
                Box::new(config),
                &agent_alias,
                message,
                session_id,
                galleon_api::ingress::TurnOrigin::Interactive,
            ),
        ),
    ))
    .await?;
    Ok(GatewayChatOutcome { response })
}

#[derive(Debug)]
pub struct UnconfiguredModelProvider;

impl galleon_api::attribution::Attributable for UnconfiguredModelProvider {
    fn role(&self) -> galleon_api::attribution::Role {
        galleon_api::attribution::Role::Provider(galleon_api::attribution::ProviderKind::Model(
            galleon_api::attribution::ModelProviderKind::OpenAi,
        ))
    }
    fn alias(&self) -> &str {
        "unconfigured"
    }
}

#[async_trait::async_trait]
impl galleon_api::model_provider::ModelProvider for UnconfiguredModelProvider {
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

#[cfg(test)]
mod chat_regression_tests {
    use super::*;
    use galleon_api::agent::{AgentRunner, AgentSession, set_agent_runner};
    use galleon_api::ingress::TurnOrigin;
    use std::sync::{Arc, Mutex};

    struct MockChatRunner {
        recorded_message: Arc<Mutex<Option<String>>>,
        recorded_session: Arc<Mutex<Option<Option<String>>>>,
        recorded_origin: Arc<Mutex<Option<TurnOrigin>>>,
        simulated_response: String,
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl AgentRunner for MockChatRunner {
        async fn create_session(
            &self,
            _config: Box<dyn std::any::Any + Send + Sync>,
            _agent_alias: &str,
            _session_cwd: Option<&std::path::Path>,
            _canvas_store: Option<Box<dyn std::any::Any + Send + Sync>>,
        ) -> anyhow::Result<Box<dyn AgentSession>> {
            anyhow::bail!("create_session not implemented for mock")
        }

        async fn process_message(
            &self,
            _config: Box<dyn std::any::Any + Send>,
            _agent_alias: &str,
            message: &str,
            session_id: Option<&str>,
            origin: TurnOrigin,
        ) -> anyhow::Result<String> {
            *self.recorded_message.lock().unwrap() = Some(message.to_string());
            *self.recorded_session.lock().unwrap() = Some(session_id.map(ToString::to_string));
            *self.recorded_origin.lock().unwrap() = Some(origin);

            if self.should_fail {
                anyhow::bail!("simulated turn error")
            } else {
                Ok(self.simulated_response.clone())
            }
        }
    }

    static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn test_run_gateway_chat_with_tools_exercises_process_message_success() {
        let _guard = TEST_MUTEX.lock().await;
        let recorded_message = Arc::new(Mutex::new(None));
        let recorded_session = Arc::new(Mutex::new(None));
        let recorded_origin = Arc::new(Mutex::new(None));

        let runner = Arc::new(MockChatRunner {
            recorded_message: Arc::clone(&recorded_message),
            recorded_session: Arc::clone(&recorded_session),
            recorded_origin: Arc::clone(&recorded_origin),
            simulated_response: "hello back from mock agent".to_string(),
            should_fail: false,
        });

        set_agent_runner(runner);

        let config = galleon_config::schema::Config::default();
        let state = crate::api_config::tests::test_state(config);

        let outcome = run_gateway_chat_with_tools(
            &state,
            "test chat message",
            Some("session-123"),
            Some("main"),
        )
        .await
        .expect("chat should succeed");

        assert_eq!(outcome.response, "hello back from mock agent");
        assert_eq!(
            recorded_message.lock().unwrap().as_deref(),
            Some("test chat message")
        );
        assert_eq!(
            recorded_session
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .as_deref(),
            Some("session-123")
        );
        assert_eq!(
            *recorded_origin.lock().unwrap(),
            Some(TurnOrigin::Interactive)
        );
    }

    #[tokio::test]
    async fn test_run_gateway_chat_with_tools_exercises_process_message_error_propagation() {
        let _guard = TEST_MUTEX.lock().await;
        let runner = Arc::new(MockChatRunner {
            recorded_message: Arc::new(Mutex::new(None)),
            recorded_session: Arc::new(Mutex::new(None)),
            recorded_origin: Arc::new(Mutex::new(None)),
            simulated_response: String::new(),
            should_fail: true,
        });

        set_agent_runner(runner);

        let config = galleon_config::schema::Config::default();
        let state = crate::api_config::tests::test_state(config);

        let err = run_gateway_chat_with_tools(&state, "failing message", None, Some("main"))
            .await
            .expect_err("chat should fail when process_message fails");

        assert!(err.to_string().contains("simulated turn error"));
    }
}
