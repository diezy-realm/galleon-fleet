use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use clawcrew_api::channel::{Channel, ChannelApprovalResponse, ChannelMessage, SendMessage};
pub use clawcrew_config::schema::MattermostListenMode;
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;
use tokio::{io::AsyncRead, io::AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Decoupled audio transcription trait for Mattermost channel.
pub trait AudioTranscriber: Send + Sync {
    fn transcribe<'a>(
        &'a self,
        audio_data: &'a [u8],
        file_name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send + 'a>>;
}

pub(crate) async fn read_response_body_limited(
    mut response: reqwest::Response,
    max_bytes: u64,
) -> anyhow::Result<Vec<u8>> {
    if let Some(content_length) = response.content_length()
        && content_length > max_bytes
    {
        anyhow::bail!(
            "response body content length {content_length} exceeds {max_bytes}-byte limit"
        );
    }

    let mut body = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        let chunk_len = u64::try_from(chunk.len()).unwrap_or(u64::MAX);
        let next_len = u64::try_from(body.len())
            .unwrap_or(u64::MAX)
            .saturating_add(chunk_len);
        if next_len > max_bytes {
            anyhow::bail!("response body exceeds {max_bytes}-byte limit");
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

pub(crate) fn build_yesno_approval_prompt(
    token: &str,
    tool_name: &str,
    arguments_summary: &str,
    position: Option<(u32, u32)>,
) -> String {
    let position_line = match position {
        Some((_, total)) if total <= 1 => String::new(),
        Some((index, total)) => format!("Tool call {index} of {total}\n"),
        None => String::new(),
    };
    let heading = galleon_i18n::get_required_cli_string("channel-approval-heading-shout");
    let tool_label = galleon_i18n::get_required_cli_string("channel-approval-tool-label");
    let args_label = galleon_i18n::get_required_cli_string("channel-approval-args-label");
    let yes_command = format!("{token} yes");
    let no_command = format!("{token} no");
    let always_command = format!("{token} always");
    let reply = galleon_i18n::get_required_cli_string_with_args(
        "channel-approval-reply-instruction-yesno",
        &[
            ("yes_command", yes_command.as_str()),
            ("no_command", no_command.as_str()),
            ("always_command", always_command.as_str()),
        ],
    );
    format!(
        "{heading} [{token}]\n{position_line}{tool_label}: {tool_name}\n{args_label}: {arguments_summary}\n\n{reply}"
    )
}

const MAX_MATTERMOST_AUDIO_BYTES: u64 = 25 * 1024 * 1024;
/// Cadence at which auto-discovery re-runs to pick up newly-created DMs
/// and team channel changes.
const DISCOVERY_REFRESH: Duration = Duration::from_secs(60);
/// Poll interval per discovery iteration. Matches the previous single-channel
/// cadence so operators see no change in latency.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// A prompt awaiting an answer, bound to the destination it was posted into.
///
/// The token alone is not enough to identify the answer. It travels in
/// plaintext into a channel, one `MattermostChannel` serves many channels
/// (`poll_channel` iterates targets; the WebSocket path sees every post the bot
/// can read), and peer-group membership is scoped to the alias rather than to a
/// room. Without the destination an authorized user could answer a prompt from a
/// channel it was never posted to.
struct PendingApproval {
    /// Distinguishes this registration from any later one that happens to draw
    /// the same token. `new_approval_token` is random but only six characters,
    /// so token equality alone is not registration identity — without this, an
    /// older still-armed guard could retire a newer request's prompt.
    generation: u64,
    /// Mattermost channel ID the prompt was posted into.
    channel_id: String,
    sender: tokio::sync::oneshot::Sender<ChannelApprovalResponse>,
}

/// How many retired tokens to remember. A token only needs to outlive the
/// in-flight events that were already in flight when it was retired, so this is
/// deliberately small and bounded — it is a race window, not a history.
const RETIRED_APPROVAL_TOKENS: usize = 64;

/// All approval bookkeeping, behind one lock.
///
/// The two maps and the retired-token ring are a single unit of state because
/// every operation on them has to be one indivisible transition. Splitting them
/// across separate locks — or taking the same lock more than once per decision —
/// lets the reaction path retire a token in between the text path's *recognize*
/// and *remove* steps, at which point the text resolver can no longer tell "this
/// was never mine" from "I just lost the race", and the losing `<token> yes` is
/// forwarded to the model as ordinary conversation.
#[derive(Default)]
struct ApprovalState {
    /// Live prompts, keyed by the token echoed back in a reply.
    pending: HashMap<String, PendingApproval>,
    /// Prompt post ID -> (token, generation), for resolving one-tap emoji
    /// reactions. The generation is carried so cleanup can drop only the
    /// binding belonging to the registration being retired.
    posts: HashMap<String, (String, u64)>,
    /// Tokens already answered, cancelled, or timed out. Retained briefly so a
    /// reply that lost the race is still recognized as approval traffic and
    /// suppressed rather than delivered to the model.
    retired: std::collections::VecDeque<String>,
    /// Monotonic registration counter. Never reused within a channel's life.
    next_generation: u64,
}

impl ApprovalState {
    /// Retire `token` only if the live registration is the one identified by
    /// `generation`.
    ///
    /// A guard armed for an earlier request may still be alive when a later
    /// request draws the same six-character token. Retiring by token alone would
    /// then delete the newer request's sender and post binding, silently
    /// stranding an approval the operator can still see.
    fn retire_generation(&mut self, token: &str, generation: u64) -> Option<PendingApproval> {
        let matches = self
            .pending
            .get(token)
            .is_some_and(|pending| pending.generation == generation);
        if !matches {
            return None;
        }
        let removed = self.pending.remove(token);
        self.posts.retain(|_, (bound, bound_generation)| {
            bound != token || *bound_generation != generation
        });
        self.remember_retired(token);
        removed
    }

    fn remember_retired(&mut self, token: &str) {
        if !self.retired.iter().any(|seen| seen == token) {
            if self.retired.len() == RETIRED_APPROVAL_TOKENS {
                self.retired.pop_front();
            }
            self.retired.push_back(token.to_string());
        }
    }

    /// Draw a token that is neither live nor inside the retired window.
    ///
    /// `new_approval_token` is six random characters, so redraws can collide.
    /// A collision with a *live* token would overwrite another request's
    /// registration; one with a *retired* token would make that request's late
    /// decision look like it belongs to this prompt. Both hand authority to the
    /// wrong request rather than merely denying one, so neither is acceptable.
    ///
    /// Returns `None` if no free token is found within the attempt budget. With
    /// a 36^6 space and a retired window of 64, that means something is very
    /// wrong, and refusing to prompt is safer than reusing an identity.
    fn allocate_token(&self) -> Option<String> {
        const ALLOCATION_ATTEMPTS: usize = 16;
        (0..ALLOCATION_ATTEMPTS)
            .map(|_| galleon_channel_core::approval::new_approval_token())
            .find(|token| !self.pending.contains_key(token) && !self.was_retired(token))
    }

    /// True when `token` is currently held by the registration identified by
    /// `generation` — the check every decision path makes before acting, so a
    /// later request that drew the same token cannot receive an older one's
    /// decision.
    fn owns(&self, token: &str, generation: u64) -> bool {
        self.pending
            .get(token)
            .is_some_and(|pending| pending.generation == generation)
    }

    fn was_retired(&self, token: &str) -> bool {
        self.retired.iter().any(|seen| seen == token)
    }
}

/// What the text resolver did with a post.
///
/// The distinction that matters is `NotFound` versus everything else: only text
/// this channel has no claim on may continue to the model. Every other outcome —
/// resolved, rejected, or too late — is approval-protocol traffic and is
/// swallowed, so a decision the operator typed can never surface as a chat
/// message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApprovalReplyOutcome {
    /// Not an approval reply for any prompt this channel knows about.
    NotFound,
    /// Recognized as approval traffic and consumed.
    Consumed,
}

/// Retires a prompt's state when the request future goes away.
///
/// Registration happens before the prompt is posted, but the explicit cleanup
/// branches only run if `request_approval_attributed` reaches its own match arm.
/// A future that is *dropped* never reaches them, and the routed approval caller
/// wraps this one in an outer `tokio::time::timeout` whose default (120s) is
/// shorter than this channel's own (300s) — so cancellation mid-prompt is a
/// supported configuration, not a theoretical executor behaviour. Without this
/// guard each routed timeout would strand a token and its post binding in a
/// long-lived channel, and a later reply or tap would appear to answer a prompt
/// whose tool call had already been abandoned.
///
/// `Drop` cannot await, which is why [`ApprovalState`] sits behind a synchronous
/// mutex: cleanup runs inline and is therefore deterministic to assert in tests,
/// rather than being deferred to a spawned task.
struct ApprovalGuard {
    approvals: Arc<Mutex<ApprovalState>>,
    token: String,
    /// The registration this guard owns. Cleanup is conditional on it, so a
    /// stale guard cannot retire a later request that reused the token.
    generation: u64,
    /// Cleared once the request has retired the token itself, so the normal
    /// completion path does not retire it twice.
    armed: bool,
}

impl ApprovalGuard {
    fn new(approvals: Arc<Mutex<ApprovalState>>, token: String, generation: u64) -> Self {
        Self {
            approvals,
            token,
            generation,
            armed: true,
        }
    }

    /// Retire the prompt now and stand the guard down.
    fn retire(&mut self) {
        if self.armed {
            self.approvals
                .lock()
                .retire_generation(&self.token, self.generation);
            self.armed = false;
        }
    }
}

impl Drop for ApprovalGuard {
    fn drop(&mut self) {
        self.retire();
    }
}

/// The channel ID half of a recipient.
///
/// Recipients are `channel_id` or `channel_id:root_id` for a threaded reply.
/// Binding is on the channel, not the thread: an operator answering in the
/// channel rather than inside the prompt's thread is still answering in the
/// right room, and Mattermost clients make it easy to do either.
fn recipient_channel_id(recipient: &str) -> &str {
    recipient
        .split_once(':')
        .map_or(recipient, |(channel, _)| channel)
}

/// Emoji that stand for the two one-tap approval decisions. Mattermost's bare
/// emoji names, not the `:shortcode:` spelling the API rejects.
const APPROVAL_EMOJI_APPROVE: &str = "white_check_mark";
const APPROVAL_EMOJI_DENY: &str = "x";

/// Map a reaction emoji to the decision it stands for.
///
/// Anything else is a normal reaction on the prompt post — an operator giving a
/// thumbs-up, say — and must not be read as an answer.
fn approval_response_for_emoji(emoji: &str) -> Option<ChannelApprovalResponse> {
    match emoji {
        APPROVAL_EMOJI_APPROVE => Some(ChannelApprovalResponse::Approve),
        APPROVAL_EMOJI_DENY => Some(ChannelApprovalResponse::Deny),
        _ => None,
    }
}

/// Application-level ping interval for the Mattermost WebSocket protocol.
const WS_PING_INTERVAL: Duration = Duration::from_secs(30);
/// Deadline for authentication and the server's `hello` event.
const WS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
/// Read timeout for the active WebSocket session. If no frame arrives
/// within this window the peer is considered unresponsive and the
/// listener exits into the reconnect path. Set to 3× ping interval so
/// the server can miss two pings before we declare it dead.
const WS_READ_TIMEOUT: Duration = Duration::from_secs(90);

/// One channel the bot will poll. `is_direct` flags DM (`type=D`) and group DM
/// (`type=G`) channels so the receive path can bypass `mention_only` for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetChannel {
    pub id: String,
    pub is_direct: bool,
    /// The room's Mattermost `purpose`, as returned by discovery.
    ///
    /// Already present in the `/api/v4/users/me/channels` response this filter
    /// parses, so retaining it costs no extra request. Empty purposes are
    /// normalised to `None` so "unset" and "blank" are one state.
    pub purpose: Option<String>,
}

/// Mattermost channel `type` is a single-character code: `O` = open/public,
/// `P` = private, `G` = group DM, `D` = direct DM. Group DMs are private
/// multi-user conversations and share the no-ambient-noise semantic with 1:1
/// DMs, so both are treated as "direct" for `mention_only` purposes.
pub(crate) fn is_direct_channel(channel_type: &str) -> bool {
    matches!(channel_type, "D" | "G")
}

/// Filter a raw `/api/v4/users/me/channels` response down to the channels the
/// bot should poll. Public/private channels are gated by `team_ids` (empty =
/// all teams); DM/group-DM channels are gated by `discover_dms`. DMs carry
/// no `team_id`, so the team allowlist deliberately doesn't apply to them.
pub(crate) fn filter_discovered_channels(
    channels: &[serde_json::Value],
    team_ids: &[String],
    discover_dms: bool,
) -> Vec<TargetChannel> {
    channels
        .iter()
        .filter_map(|c| {
            let id = c.get("id").and_then(|v| v.as_str())?;
            let ty = c.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let team = c.get("team_id").and_then(|v| v.as_str()).unwrap_or("");
            let direct = is_direct_channel(ty);
            if direct {
                if !discover_dms {
                    return None;
                }
            } else if !team_ids.is_empty() && !team_ids.iter().any(|allowed| allowed == team) {
                return None;
            }
            Some(TargetChannel {
                id: id.to_string(),
                is_direct: direct,
                purpose: c
                    .get("purpose")
                    .and_then(|purpose| purpose.as_str())
                    .map(str::trim)
                    .filter(|purpose| !purpose.is_empty())
                    .map(str::to_string),
            })
        })
        .collect()
}

/// Mattermost channel — polls channel posts via REST API v4.
/// Mattermost is API-compatible with many Slack patterns but uses a dedicated v4 structure.
pub struct MattermostChannel {
    base_url: String, // e.g., https://mm.example.com
    /// Static bot token from the config. Preferred over login when set.
    bot_token: Option<String>,
    /// Login ID for the password login flow. Used when `bot_token` is None.
    login_id: Option<String>,
    /// Password for the login flow. Used when `bot_token` is None.
    password: Option<String>,
    /// Resolved session token used by all API calls. Populated lazily on
    /// first use, either by copying `bot_token` or by performing the login
    /// flow with `login_id` and `password`.
    session_token: OnceCell<String>,
    /// (user_id, username) for the bot, fetched once from `/users/me`
    /// inside `get_bot_identity`. Read by `self_handle` /
    /// `self_addressed_mention` so the identity block reaches the prompt.
    bot_identity: OnceCell<(String, String)>,
    /// Channel IDs from config. Empty or `["*"]` triggers auto-discovery.
    channel_ids: Vec<String>,
    /// Team allowlist for auto-discovery. Empty = all teams.
    team_ids: Vec<String>,
    /// When true, auto-discovery includes DM (`type=D`) and group DM (`type=G`)
    /// channels. Defaults to true at construction; `with_discover_dms` overrides.
    discover_dms: bool,
    /// The alias key under `[channels.mattermost.<alias>]` this handle is
    /// bound to. Used to scope peer-group writes and resolver lookups.
    alias: String,
    /// Resolves inbound external peers from canonical state at message-time.
    /// No cache (see AGENTS.md "ABSOLUTE RULE — SINGLE SOURCE OF TRUTH").
    peer_resolver: Arc<dyn Fn() -> Vec<String> + Send + Sync>,
    /// When true (default), replies thread on the original post's root_id.
    /// When false, replies go to the channel root.
    thread_replies: bool,
    /// When true, only respond to messages that @-mention the bot.
    mention_only: bool,
    /// Handle for the background typing-indicator loop (aborted on stop_typing).
    typing_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Per-channel proxy URL override.
    proxy_url: Option<String>,
    transcription: Option<clawcrew_config::schema::TranscriptionConfig>,
    transcription_manager: Option<Arc<dyn AudioTranscriber>>,
    /// How this channel receives inbound messages. Defaults to `Polling`.
    listen_mode: MattermostListenMode,
    /// In-flight approval prompts, the post bindings that resolve one-tap
    /// reactions, and the recently-retired tokens — see [`ApprovalState`].
    ///
    /// A `parking_lot` mutex, like the rest of this file. The guard is never
    /// held across an `.await`: everything asynchronous a decision needs (the
    /// bot identity, the peer list) is resolved before the lock is taken, so
    /// each decision is one uninterruptible critical section. That is also what
    /// lets [`ApprovalGuard`] clean up synchronously from `Drop`, where awaiting
    /// is not possible.
    approvals: Arc<Mutex<ApprovalState>>,
    /// Seconds to wait for an operator reply before the runtime denies on its
    /// own authority. Mirrors `[channels.mattermost.<alias>].approval_timeout_secs`.
    approval_timeout_secs: u64,
    /// Whether this alias injects room purposes into the prompt. Mirrors
    /// `[channels.mattermost.<alias>].purpose_as_instructions`.
    purpose_as_instructions: bool,
    /// Room ID -> purpose, refreshed by discovery.
    ///
    /// Populated only when `purpose_as_instructions` is set, so an alias that
    /// has not opted in never retains channel-supplied text at all. Discovery
    /// re-runs on `DISCOVERY_REFRESH`, so an edited purpose reaches the prompt
    /// within that window rather than needing a restart.
    room_purposes: Arc<Mutex<HashMap<String, String>>>,
}

impl MattermostChannel {
    pub fn new(
        base_url: String,
        bot_token: Option<String>,
        login_id: Option<String>,
        password: Option<String>,
        channel_ids: Vec<String>,
        alias: impl Into<String>,
        peer_resolver: Arc<dyn Fn() -> Vec<String> + Send + Sync>,
        thread_replies: bool,
        mention_only: bool,
    ) -> Self {
        // Ensure base_url doesn't have a trailing slash for consistent path joining
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            base_url,
            bot_token,
            login_id,
            password,
            session_token: OnceCell::new(),
            bot_identity: OnceCell::new(),
            channel_ids,
            team_ids: Vec::new(),
            discover_dms: true,
            alias: alias.into(),
            peer_resolver,
            thread_replies,
            mention_only,
            typing_handle: Mutex::new(None),
            proxy_url: None,
            transcription: None,
            transcription_manager: None,
            listen_mode: MattermostListenMode::default(),
            approvals: Arc::new(Mutex::new(ApprovalState::default())),
            // Sourced from the config type so the in-Rust default cannot drift
            // from the serde one. `0` here would be an already-elapsed
            // deadline that denies every approval.
            approval_timeout_secs: clawcrew_config::schema::MattermostConfig::default()
                .approval_timeout_secs,
            purpose_as_instructions: false,
            room_purposes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Seconds to wait for an operator reply to an approval prompt before the
    /// runtime denies on its own authority.
    #[must_use]
    pub fn with_approval_timeout_secs(mut self, approval_timeout_secs: u64) -> Self {
        self.approval_timeout_secs = approval_timeout_secs;
        self
    }

    /// Opt this alias into injecting each room's Mattermost purpose into the
    /// system prompt as channel-supplied context.
    #[must_use]
    pub fn with_purpose_as_instructions(mut self, purpose_as_instructions: bool) -> Self {
        self.purpose_as_instructions = purpose_as_instructions;
        self
    }

    /// Record the purposes discovery just observed, replacing the previous set.
    ///
    /// Replacing rather than merging is deliberate: a purpose cleared in
    /// Mattermost must stop being injected, and a room the bot has left must
    /// stop contributing at all.
    fn refresh_room_purposes(&self, targets: &[TargetChannel]) {
        if !self.purpose_as_instructions {
            return;
        }
        let refreshed: HashMap<String, String> = targets
            .iter()
            .filter_map(|target| {
                target
                    .purpose
                    .as_ref()
                    .map(|purpose| (target.id.clone(), purpose.clone()))
            })
            .collect();
        // The whole feature is silent when it does not work: a missing purpose
        // looks exactly like a room that has none. Report what discovery
        // actually found so an operator can tell "no purpose set" from "the
        // purpose never reached us".
        ::clawcrew_log::record!(
            DEBUG,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
                ::serde_json::json!({
                    "alias": self.alias,
                    "rooms_discovered": targets.len(),
                    "rooms_with_purpose": refreshed.len(),
                })
            ),
            "Refreshed Mattermost room purposes"
        );
        *self.room_purposes.lock() = refreshed;
    }

    /// Restrict auto-discovery to the given team IDs. Empty = all teams the
    /// bot belongs to. No effect when `channel_ids` lists explicit IDs.
    pub fn with_team_ids(mut self, team_ids: Vec<String>) -> Self {
        self.team_ids = team_ids;
        self
    }

    /// Include (`true`, default) or omit (`false`) DM and group-DM channels
    /// during auto-discovery. No effect when `channel_ids` lists explicit IDs.
    pub fn with_discover_dms(mut self, discover_dms: bool) -> Self {
        self.discover_dms = discover_dms;
        self
    }

    /// Normalize a raw `channel_ids` entry: trim, drop blanks and the `*`
    /// wildcard sentinel. Returns `None` when the entry should not contribute
    /// to the explicit-scope list.
    pub(crate) fn normalized_channel_id(input: Option<&str>) -> Option<String> {
        input
            .map(str::trim)
            .filter(|v| !v.is_empty() && *v != "*")
            .map(ToOwned::to_owned)
    }

    /// Resolve the explicit channel scope from `channel_ids`. Returns `None`
    /// when the config asks for auto-discovery (empty list or wildcard-only).
    pub(crate) fn scoped_channel_ids(&self) -> Option<Vec<String>> {
        let mut seen = HashSet::new();
        let ids: Vec<String> = self
            .channel_ids
            .iter()
            .filter_map(|entry| Self::normalized_channel_id(Some(entry)))
            .filter(|id| seen.insert(id.clone()))
            .collect();
        if ids.is_empty() { None } else { Some(ids) }
    }

    pub(crate) async fn list_target_channels(&self) -> Result<Vec<TargetChannel>> {
        let token = self.token().await?.to_string();
        if let Some(ids) = self.scoped_channel_ids() {
            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                let resp = self
                    .http_client()
                    .get(format!("{}/api/v4/channels/{}", self.base_url, id))
                    .bearer_auth(&token)
                    .send()
                    .await
                    .with_context(|| format!("GET /channels/{id} failed"))?;
                if !resp.status().is_success() {
                    bail!(
                        "GET /channels/{id} returned {}: explicit channel_id is not accessible to this bot",
                        resp.status()
                    );
                }
                let body: serde_json::Value = resp
                    .json()
                    .await
                    .with_context(|| format!("decode /channels/{id} body"))?;
                let ty = body.get("type").and_then(|v| v.as_str()).unwrap_or("");
                out.push(TargetChannel {
                    id,
                    is_direct: is_direct_channel(ty),
                    // Same object shape as discovery, so the purpose is here too.
                    purpose: body
                        .get("purpose")
                        .and_then(|purpose| purpose.as_str())
                        .map(str::trim)
                        .filter(|purpose| !purpose.is_empty())
                        .map(str::to_string),
                });
            }
            return Ok(out);
        }
        let resp = self
            .http_client()
            .get(format!("{}/api/v4/users/me/channels", self.base_url))
            .bearer_auth(&token)
            .send()
            .await
            .context("GET /users/me/channels failed")?;
        if !resp.status().is_success() {
            bail!("GET /users/me/channels returned {}", resp.status());
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .context("decode /users/me/channels body")?;
        let arr = body.as_array().cloned().unwrap_or_default();
        Ok(filter_discovered_channels(
            &arr,
            &self.team_ids,
            self.discover_dms,
        ))
    }

    /// Return the alias under `[channels.mattermost.<alias>]` that this
    /// channel handle is bound to.
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Resolve the session token, performing the login flow on first call
    /// if `bot_token` is not set.
    async fn token(&self) -> Result<&str> {
        self.session_token
            .get_or_try_init(|| async {
                if let Some(ref t) = self.bot_token {
                    return Ok::<String, anyhow::Error>(t.clone());
                }
                let login_id = self.login_id.as_deref().ok_or_else(|| {
                    ::clawcrew_log::record!(
                        ERROR,
                        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                            .with_outcome(::clawcrew_log::EventOutcome::Failure)
                            .with_attrs(::serde_json::json!({
                                "missing": "login_id",
                                "reason": "no_bot_token",
                            })),
                        "mattermost: bot_token unset and login_id missing"
                    );
                    anyhow::Error::msg(
                        "bot_token is unset; configure either bot_token or both login_id and password",
                    )
                })?;
                let password = self.password.as_deref().ok_or_else(|| {
                    ::clawcrew_log::record!(
                        ERROR,
                        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                            .with_outcome(::clawcrew_log::EventOutcome::Failure)
                            .with_attrs(::serde_json::json!({
                                "missing": "password",
                                "reason": "no_bot_token",
                            })),
                        "mattermost: bot_token unset and password missing"
                    );
                    anyhow::Error::msg(
                        "bot_token is unset and password is missing; both login_id and password must be set",
                    )
                })?;
                self.login(login_id, password).await
            })
            .await
            .map(String::as_str)
    }

    /// Perform the Mattermost password login flow and return the session
    /// token. The session token is returned via the `Token` response header
    /// per Mattermost API v4.
    async fn login(&self, login_id: &str, password: &str) -> Result<String> {
        let resp = self
            .http_client()
            .post(format!("{}/api/v4/users/login", self.base_url))
            .json(&serde_json::json!({
                "login_id": login_id,
                "password": password,
            }))
            .send()
            .await
            .context("login request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("login failed ({status}): {body}");
        }
        let token = resp
            .headers()
            .get("Token")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                ::clawcrew_log::record!(
                    ERROR,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Fail)
                        .with_outcome(::clawcrew_log::EventOutcome::Failure),
                    "login succeeded but the response had no Token header"
                );
                anyhow::Error::msg("login succeeded but the response had no Token header")
            })?
            .to_string();
        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note),
            "login succeeded; session token cached"
        );
        Ok(token)
    }

    /// Set a per-channel proxy URL that overrides the global proxy config.
    pub fn with_proxy_url(mut self, proxy_url: Option<String>) -> Self {
        self.proxy_url = proxy_url;
        self
    }

    /// Configure voice transcription from a `[transcription]` snapshot.
    ///
    /// Compatibility and test path. The daemon routes every channel through
    /// `with_transcription_manager` with a manager built from live
    /// config and the owning agent's resolved provider; this path can only see
    /// the legacy section, so it binds a lone registered provider and
    /// otherwise leaves the choice unbound (see
    /// `transcription::manager_from_snapshot`).
    /// Configure voice transcription from a `[transcription]` snapshot (no-op stub without provider manager).
    pub fn with_transcription(
        mut self,
        config: clawcrew_config::schema::TranscriptionConfig,
    ) -> Self {
        self.transcription = Some(config);
        self
    }

    /// Store an already-built transcription manager, or nothing. The config is
    /// recorded only alongside a manager, so a channel never advertises
    /// transcription it cannot perform.
    pub fn with_transcription_manager(
        mut self,
        config: clawcrew_config::schema::TranscriptionConfig,
        manager: Option<std::sync::Arc<dyn AudioTranscriber>>,
    ) -> Self {
        if let Some(manager) = manager {
            self.transcription_manager = Some(manager);
            self.transcription = Some(config);
        }
        self
    }

    /// Store an audio transcriber provider.
    pub fn with_audio_transcriber(
        mut self,
        transcriber: Option<std::sync::Arc<dyn AudioTranscriber>>,
    ) -> Self {
        self.transcription_manager = transcriber;
        self
    }

    /// Set the listen mode. Defaults to `Polling` when not called.
    pub fn with_listen_mode(mut self, listen_mode: MattermostListenMode) -> Self {
        self.listen_mode = listen_mode;
        self
    }

    fn http_client(&self) -> reqwest::Client {
        clawcrew_config::schema::build_channel_proxy_client_with_timeouts(
            "channel.mattermost",
            self.proxy_url.as_deref(),
            30,
            10,
        )
    }

    /// Derive the WebSocket URL from the REST base URL.
    fn ws_url(&self) -> String {
        self.base_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1)
            + "/api/v4/websocket"
    }

    fn ws_auth_response(event: &serde_json::Value, auth_seq: i64) -> Option<bool> {
        (event.get("seq_reply").and_then(|v| v.as_i64()) == Some(auth_seq))
            .then(|| event.get("status").and_then(|v| v.as_str()) == Some("OK"))
    }

    fn ws_post_from_event(event: &serde_json::Value) -> Option<serde_json::Value> {
        let post = event.get("data")?.get("post")?.as_str()?;
        serde_json::from_str(post).ok()
    }

    async fn authenticate_websocket<S>(
        write: &mut SplitSink<WebSocketStream<S>, WsMessage>,
        read: &mut SplitStream<WebSocketStream<S>>,
        token: &str,
        auth_seq: i64,
        timeout: Duration,
    ) -> Result<String>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let auth = serde_json::json!({
            "seq": auth_seq,
            "action": "authentication_challenge",
            "data": { "token": token }
        });
        write
            .send(WsMessage::Text(auth.to_string().into()))
            .await
            .context("Mattermost WebSocket authentication send failed")?;

        let deadline = tokio::time::Instant::now() + timeout;
        let mut authenticated = false;
        let mut server_version = None;

        loop {
            if authenticated && server_version.is_some() {
                return Ok(server_version.unwrap_or_else(|| "unknown".to_string()));
            }

            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    bail!("Mattermost WebSocket authentication handshake timed out");
                }
                frame = read.next() => {
                    let text = match frame {
                        Some(Ok(WsMessage::Text(text))) => text,
                        Some(Ok(WsMessage::Ping(payload))) => {
                            write
                                .send(WsMessage::Pong(payload))
                                .await
                                .context("Mattermost WebSocket handshake pong failed")?;
                            continue;
                        }
                        Some(Ok(WsMessage::Close(frame))) => {
                            let reason = frame
                                .as_ref()
                                .map(|frame| frame.reason.as_ref())
                                .unwrap_or("");
                            bail!("Mattermost WebSocket closed during authentication: {reason}");
                        }
                        Some(Err(error)) => {
                            return Err(error).context("Mattermost WebSocket handshake read failed");
                        }
                        None => bail!("Mattermost WebSocket ended during authentication"),
                        Some(Ok(_)) => continue,
                    };

                    let event: serde_json::Value = serde_json::from_str(text.as_ref())
                        .context("Mattermost WebSocket handshake returned invalid JSON")?;

                    if let Some(ok) = Self::ws_auth_response(&event, auth_seq) {
                        if !ok {
                            bail!("Mattermost WebSocket authentication was rejected");
                        }
                        authenticated = true;
                    }

                    if event.get("event").and_then(|value| value.as_str()) == Some("hello") {
                        server_version = Some(
                            event
                                .get("data")
                                .and_then(|data| data.get("server_version"))
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                        );
                    }
                }
            }
        }
    }

    /// Check if a user ID is in the allowlist.
    /// Empty list means deny everyone. "*" means allow everyone.
    fn is_user_allowed(&self, user_id: &str) -> bool {
        let peers = (self.peer_resolver)();
        galleon_channel_core::allowlist::is_user_allowed(
            &peers,
            user_id,
            galleon_channel_core::allowlist::Match::Sensitive,
        )
    }

    /// Get the bot's own user ID and username so we can ignore our own messages
    /// and detect @-mentions by username. Result cached on the channel
    /// so `self_handle` / `self_addressed_mention` can read it sync.
    /// The authenticated bot identity, or an error when it cannot be resolved.
    ///
    /// A listener cannot run safely without this. Every inbound post is checked
    /// against the bot's own user ID to keep the bot from answering itself, and
    /// an empty ID matches nothing, so that check silently passes for every post
    /// the bot authored. Under a wildcard peer group and the default
    /// `mention_only = false`, the bot's own approval prompt would then be
    /// admitted as ordinary model input, feeding a security prompt's token and
    /// tool details back into the conversation.
    ///
    /// Failing rather than degrading is deliberate. The supervised listener
    /// retries with backoff, so a transient outage of Mattermost's
    /// authenticated-account endpoint resolves itself, while a durable one keeps
    /// the channel visibly down instead of quietly unsafe.
    async fn require_bot_identity(&self) -> Result<(String, String)> {
        let (bot_user_id, bot_username) = self.get_bot_identity().await;
        if bot_user_id.is_empty() {
            bail!(
                "Mattermost bot identity unavailable: the authenticated-account endpoint \
                 returned no user id; refusing to listen, because without it the bot cannot \
                 tell its own posts from an operator's"
            );
        }
        Ok((bot_user_id, bot_username))
    }

    async fn get_bot_identity(&self) -> (String, String) {
        if let Some(cached) = self.bot_identity.get() {
            return cached.clone();
        }
        let token = match self.token().await {
            Ok(t) => t.to_string(),
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                    "auth failed in get_bot_identity"
                );
                return (String::new(), String::new());
            }
        };
        let resp: Option<serde_json::Value> = async {
            self.http_client()
                .get(format!("{}/api/v4/users/me", self.base_url))
                .bearer_auth(&token)
                .send()
                .await
                .ok()?
                .json()
                .await
                .ok()
        }
        .await;

        let id = resp
            .as_ref()
            .and_then(|v| v.get("id"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        let username = resp
            .as_ref()
            .and_then(|v| v.get("username"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        // Cache only a *complete* identity. The ID is the half the approval path
        // depends on — it is what distinguishes the bot's own post or seeded
        // reaction from an operator's decision — so caching a username-only
        // response would pin an empty ID for the lifetime of the channel and
        // permanently disable the self-approval guard. Leaving it uncached costs
        // one extra request and lets the next call recover.
        if !id.is_empty() {
            let _ = self.bot_identity.set((id.clone(), username.clone()));
        }
        (id, username)
    }

    async fn try_transcribe_audio_attachment(&self, post: &serde_json::Value) -> Option<String> {
        let config = self.transcription.as_ref()?;
        let manager = self.transcription_manager.as_deref()?;

        let files = post
            .get("metadata")
            .and_then(|m| m.get("files"))
            .and_then(|f| f.as_array())?;

        let audio_file = files.iter().find(|f| is_audio_file(f))?;

        if let Some(duration_ms) = audio_file.get("duration").and_then(|d| d.as_u64()) {
            let duration_secs = duration_ms / 1000;
            if duration_secs > config.max_duration_secs {
                ::clawcrew_log::record!(DEBUG, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"duration_secs": duration_secs, "max": config.max_duration_secs})), "audio attachment exceeds max duration, skipping");
                return None;
            }
        }

        let file_id = audio_file.get("id").and_then(|i| i.as_str())?;
        let file_name = audio_file
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("audio");

        let token = match self.token().await {
            Ok(t) => t.to_string(),
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(
                            ::serde_json::json!({"error": format!("{}", e), "file_id": file_id})
                        ),
                    "audio download auth failed for"
                );
                return None;
            }
        };
        let response = match self
            .http_client()
            .get(format!("{}/api/v4/files/{}", self.base_url, file_id))
            .bearer_auth(&token)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(
                            ::serde_json::json!({"error": format!("{}", e), "file_id": file_id})
                        ),
                    "audio download failed for"
                );
                return None;
            }
        };

        if !response.status().is_success() {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Unknown),
                &format!("audio download returned {}: {file_id}", response.status())
            );
            return None;
        }

        let bytes = match read_response_body_limited(response, MAX_MATTERMOST_AUDIO_BYTES).await {
            Ok(b) => b,
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(
                            ::serde_json::json!({"error": format!("{}", e), "file_id": file_id})
                        ),
                    "failed to read audio bytes for"
                );
                return None;
            }
        };

        match manager.transcribe(&bytes, file_name).await {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    ::clawcrew_log::record!(
                        INFO,
                        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note),
                        "transcription returned empty text, skipping"
                    );
                    None
                } else {
                    Some(format!("[Voice] {trimmed}"))
                }
            }
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                    "audio transcription failed"
                );
                None
            }
        }
    }

    /// Create a post and return its ID.
    ///
    /// `send` discards the ID; the approval path needs it, because a reaction
    /// event identifies the post it landed on and nothing else. Kept as one
    /// method so both paths build the request body identically.
    async fn create_post(&self, message: &SendMessage) -> Result<String> {
        // Mattermost supports threading via 'root_id'.
        // We pack 'channel_id:root_id' into recipient if it's a thread.
        let (channel_id, root_id) = if let Some((c, r)) = message.recipient.split_once(':') {
            (c, Some(r))
        } else {
            (message.recipient.as_str(), None)
        };

        let mut body_map = serde_json::json!({
            "channel_id": channel_id,
            "message": message.content
        });

        if let Some(root) = root_id
            && let Some(body) = body_map.as_object_mut()
        {
            body.insert(
                "root_id".to_string(),
                serde_json::Value::String(root.to_string()),
            );
        }

        let token = self.token().await?;
        let resp = self
            .http_client()
            .post(format!("{}/api/v4/posts", self.base_url))
            .bearer_auth(token)
            .json(&body_map)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
            bail!("post failed ({status}): {body}");
        }

        // A post that succeeded but whose ID we cannot read is not an error for
        // `send`, which ignores it. Return an empty ID rather than failing the
        // send; the approval path treats an empty ID as "no reaction path".
        Ok(resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|body| {
                body.get("id")
                    .and_then(|id| id.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default())
    }

    /// React to a post as the bot.
    ///
    /// `emoji_name` is Mattermost's bare name (`white_check_mark`), not the
    /// `:shortcode:` spelling.
    async fn react_to_post(&self, post_id: &str, emoji_name: &str) -> Result<()> {
        let (bot_user_id, _) = self.get_bot_identity().await;
        if bot_user_id.is_empty() {
            bail!("cannot react without a bot user ID");
        }
        let token = self.token().await?;
        let resp = self
            .http_client()
            .post(format!("{}/api/v4/reactions", self.base_url))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "user_id": bot_user_id,
                "post_id": post_id,
                "emoji_name": emoji_name,
            }))
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response: {e}>"));
            bail!("reaction failed ({status}): {body}");
        }
        Ok(())
    }
}

impl ::clawcrew_api::attribution::Attributable for MattermostChannel {
    fn role(&self) -> ::clawcrew_api::attribution::Role {
        ::clawcrew_api::attribution::Role::Channel(
            ::clawcrew_api::attribution::ChannelKind::Mattermost,
        )
    }
    fn alias(&self) -> &str {
        &self.alias
    }
}

#[async_trait]
impl Channel for MattermostChannel {
    fn name(&self) -> &str {
        "mattermost"
    }

    fn self_handle(&self) -> Option<String> {
        self.bot_identity
            .get()
            .map(|(id, _)| id.clone())
            .filter(|id| !id.is_empty())
    }

    fn self_addressed_mention(&self) -> Option<String> {
        self.bot_identity
            .get()
            .map(|(_, username)| username.clone())
            .filter(|u| !u.is_empty())
            .map(|u| format!("@{u}"))
    }

    async fn send(&self, message: &SendMessage) -> Result<()> {
        self.create_post(message).await.map(|_| ())
    }

    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()> {
        match self.listen_mode {
            MattermostListenMode::Polling => self.listen_polling(tx).await,
            MattermostListenMode::Websocket => self.listen_websocket(tx).await,
        }
    }

    async fn health_check(&self) -> bool {
        let Ok(token) = self.token().await else {
            return false;
        };
        self.http_client()
            .get(format!("{}/api/v4/users/me", self.base_url))
            .bearer_auth(token)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn start_typing(&self, recipient: &str) -> Result<()> {
        // Cancel any existing typing loop before starting a new one.
        self.stop_typing(recipient).await?;

        let client = self.http_client();
        let token = self.token().await?.to_string();
        let base_url = self.base_url.clone();

        // recipient is "channel_id" or "channel_id:root_id"
        let (channel_id, parent_id) = match recipient.split_once(':') {
            Some((channel, parent)) => (channel.to_string(), Some(parent.to_string())),
            None => (recipient.to_string(), None),
        };

        let handle = clawcrew_spawn::spawn!(async move {
            let url = format!("{base_url}/api/v4/users/me/typing");
            loop {
                let mut body = serde_json::json!({ "channel_id": channel_id });
                if let Some(ref pid) = parent_id
                    && let Some(obj) = body.as_object_mut()
                {
                    obj.insert("parent_id".to_string(), serde_json::json!(pid));
                }

                if let Ok(r) = client
                    .post(&url)
                    .bearer_auth(&token)
                    .json(&body)
                    .send()
                    .await
                    && !r.status().is_success()
                {
                    ::clawcrew_log::record!(
                        DEBUG,
                        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                            .with_attrs(::serde_json::json!({"status": r.status().to_string()})),
                        "typing indicator failed"
                    );
                }

                // Mattermost typing events expire after ~6s; re-fire every 4s.
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            }
        });

        let mut guard = self.typing_handle.lock();
        *guard = Some(handle);

        Ok(())
    }

    async fn stop_typing(&self, _recipient: &str) -> Result<()> {
        let mut guard = self.typing_handle.lock();
        if let Some(handle) = guard.take() {
            handle.abort();
        }
        Ok(())
    }

    fn room_context(&self, room_id: &str) -> Option<clawcrew_api::channel::ChannelRoomContext> {
        if !self.purpose_as_instructions {
            return None;
        }
        // The caller passes this channel's own addressing string, which for a
        // threaded reply is `channel_id:root_id`. Purposes are per room, not
        // per thread, so strip the thread half before looking one up.
        let room_id = recipient_channel_id(room_id);
        let purpose = self.room_purposes.lock().get(room_id).cloned()?;
        let context = clawcrew_api::channel::ChannelRoomContext {
            purpose: Some(purpose),
        };
        (!context.is_empty()).then_some(context)
    }

    async fn request_approval(
        &self,
        recipient: &str,
        request: &clawcrew_api::channel::ChannelApprovalRequest,
    ) -> Result<Option<ChannelApprovalResponse>> {
        Ok(self
            .request_approval_attributed(recipient, request)
            .await?
            .map(|attributed| attributed.response))
    }

    /// Prompt the operator by posting a token-prefixed message and waiting for
    /// a reply that echoes the token.
    ///
    /// Mattermost's interactive buttons post to an integration URL, which a
    /// polling deployment has no endpoint for, so the prompt is the shared
    /// text form used by Signal and Discord's plaintext fallback and is parsed
    /// by the shared [`galleon_channel_core::approval::parse_approval_reply`].
    ///
    /// Only a real token echo counts as an operator decision. The dropped-sender
    /// and timeout arms are the runtime denying on its own authority and are
    /// attributed as such, so the model is never told a human refused when none
    /// was asked.
    async fn request_approval_attributed(
        &self,
        recipient: &str,
        request: &clawcrew_api::channel::ChannelApprovalRequest,
    ) -> Result<Option<clawcrew_api::channel::AttributedApprovalResponse>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        // Allocation and registration are one critical section, and the token is
        // drawn so it collides with neither a live prompt nor one still inside
        // the retired window. Six random characters are not an identity: a
        // redraw that lands on a token another request already owns would let a
        // decision meant for one prompt resolve the other.
        let (token, generation) = {
            let mut state = self.approvals.lock();
            let Some(token) = state.allocate_token() else {
                return Err(anyhow::Error::msg(
                    "could not allocate a free approval token; too many prompts in flight",
                ));
            };
            let generation = state.next_generation;
            state.next_generation += 1;
            state.pending.insert(
                token.clone(),
                PendingApproval {
                    generation,
                    channel_id: recipient_channel_id(recipient).to_string(),
                    sender: tx,
                },
            );
            (token, generation)
        };

        let text = build_yesno_approval_prompt(
            &token,
            &request.tool_name,
            &request.arguments_summary,
            request.position_counter(),
        );
        // Armed from here on: every exit below — including a dropped future —
        // retires this registration and any post bound to it. Cleanup is keyed
        // by generation, so a guard outliving its request cannot evict a later
        // one that drew the same token.
        let mut guard = ApprovalGuard::new(Arc::clone(&self.approvals), token.clone(), generation);

        let post_id = self.create_post(&SendMessage::new(text, recipient)).await?;

        // Offer the one-tap path only when we would actually see the tap.
        // Reactions reach us as `reaction_added` WebSocket events; the polling
        // listener reads posts and never sees them, so seeding the emoji there
        // would put two buttons on the prompt that silently do nothing.
        if self.listen_mode == clawcrew_config::schema::MattermostListenMode::Websocket
            && !post_id.is_empty()
        {
            // `create_post` awaited, so confirm this registration still owns the
            // token before binding its post. Binding unconditionally would let a
            // reaction on this post resolve whichever request holds the token
            // later, handing one prompt's decision to another.
            let still_live = {
                let mut state = self.approvals.lock();
                if state.owns(&token, generation) {
                    state
                        .posts
                        .insert(post_id.clone(), (token.clone(), generation));
                    true
                } else {
                    false
                }
            };
            if still_live {
                self.seed_approval_reactions(&post_id).await;
            }
        }

        let attributed =
            match tokio::time::timeout(Duration::from_secs(self.approval_timeout_secs), rx).await {
                Ok(Ok(response)) => {
                    clawcrew_api::channel::AttributedApprovalResponse::operator(response)
                }
                Ok(Err(_)) => clawcrew_api::channel::AttributedApprovalResponse::from_runtime(
                    ChannelApprovalResponse::Deny,
                    clawcrew_api::channel::ApprovalSource::Unreachable,
                ),
                Err(_) => clawcrew_api::channel::AttributedApprovalResponse::from_runtime(
                    ChannelApprovalResponse::Deny,
                    clawcrew_api::channel::ApprovalSource::TimedOut,
                ),
            };
        // One retirement for every outcome. The resolving path already removed
        // the pending entry, but the token still has to be remembered as retired
        // and any post binding dropped, so a tap arriving after the decision
        // cannot find stale state.
        guard.retire();
        Ok(Some(attributed))
    }
}

impl MattermostChannel {
    async fn listen_polling(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()> {
        // Resolve auth up front so misconfiguration fails fast at listen-time.
        let initial_token = self.token().await?.to_string();
        let (bot_user_id, bot_username) = self.require_bot_identity().await?;

        let auto_discover = self.scoped_channel_ids().is_none();
        let mut target_channels = self.list_target_channels().await?;
        self.refresh_room_purposes(&target_channels);
        let mut last_discovery = Instant::now();
        let mut last_create_at_by_channel: HashMap<String, i64> = HashMap::new();

        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
                ::serde_json::json!({
                    "alias": self.alias,
                    "channel_count": target_channels.len(),
                    "auto_discover": auto_discover,
                    "team_ids": self.team_ids,
                    "discover_dms": self.discover_dms,
                })
            ),
            "Mattermost channel listening (polling)"
        );

        loop {
            tokio::time::sleep(POLL_INTERVAL).await;

            // Runs for auto-discovery *or* purpose tracking. Gating this on
            // `auto_discover` alone meant an alias with explicit `channel_ids`
            // read each room's purpose once at startup and never again, so an
            // edited purpose needed a daemon restart to take effect.
            let refresh_due = (auto_discover || self.purpose_as_instructions)
                && last_discovery.elapsed() >= DISCOVERY_REFRESH;
            if refresh_due {
                match self.list_target_channels().await {
                    Ok(refreshed) => {
                        // Refresh purposes on every successful discovery, not
                        // only when the *set* of rooms changed: an edited
                        // purpose leaves the set identical.
                        self.refresh_room_purposes(&refreshed);
                        // Only auto-discovery may replace the target set; an
                        // explicit `channel_ids` scope is the operator's, not
                        // ours to widen.
                        if auto_discover && refreshed != target_channels {
                            ::clawcrew_log::record!(
                                INFO,
                                ::clawcrew_log::Event::new(
                                    module_path!(),
                                    ::clawcrew_log::Action::Note,
                                )
                                .with_attrs(::serde_json::json!({
                                    "alias": self.alias,
                                    "before": target_channels.len(),
                                    "after": refreshed.len(),
                                })),
                                "Mattermost auto-discovery refreshed channel list"
                            );
                            target_channels = refreshed;
                        }
                    }
                    Err(e) => {
                        ::clawcrew_log::record!(
                            WARN,
                            ::clawcrew_log::Event::new(
                                module_path!(),
                                ::clawcrew_log::Action::Note,
                            )
                            .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                            .with_attrs(::serde_json::json!({
                                "alias": self.alias,
                                "error": format!("{}", e),
                            })),
                            "Mattermost auto-discovery refresh failed; keeping previous channel list"
                        );
                    }
                }
                last_discovery = Instant::now();
            }

            if target_channels.is_empty() {
                continue;
            }

            #[allow(clippy::cast_possible_truncation)]
            let bootstrap_ms = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()) as i64;

            for target in target_channels.clone() {
                if self
                    .poll_channel(
                        &target,
                        &initial_token,
                        &bot_user_id,
                        &bot_username,
                        bootstrap_ms,
                        &mut last_create_at_by_channel,
                        &tx,
                    )
                    .await
                {
                    return Ok(());
                }
            }
        }
    }

    async fn listen_websocket(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()> {
        let token = self.token().await?.to_string();
        let (bot_user_id, bot_username) = self.require_bot_identity().await?;
        let auto_discover = self.scoped_channel_ids().is_none();
        let target_channels = self.list_target_channels().await?;
        self.refresh_room_purposes(&target_channels);
        let mut channel_direct_map: HashMap<String, bool> = target_channels
            .into_iter()
            .map(|target| (target.id, target.is_direct))
            .collect();

        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
                ::serde_json::json!({
                    "alias": self.alias,
                    "channel_count": channel_direct_map.len(),
                    "auto_discover": auto_discover,
                    "mode": "websocket",
                })
            ),
            "Mattermost WebSocket listening"
        );

        let ws_url = self.ws_url();
        let (ws_stream, _) = clawcrew_config::schema::ws_connect_with_proxy(
            &ws_url,
            "channel.mattermost",
            self.proxy_url.as_deref(),
        )
        .await
        .with_context(|| format!("Mattermost WebSocket connect failed: {ws_url}"))?;

        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
                ::serde_json::json!({
                    "alias": self.alias,
                    "ws_url": &ws_url,
                })
            ),
            "Mattermost WebSocket connected"
        );

        let (mut write, mut read) = ws_stream.split();
        let auth_seq = 1;
        let server_version = Self::authenticate_websocket(
            &mut write,
            &mut read,
            &token,
            auth_seq,
            WS_HANDSHAKE_TIMEOUT,
        )
        .await?;

        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
                ::serde_json::json!({
                    "alias": self.alias,
                    "server_version": server_version,
                })
            ),
            "Mattermost WebSocket authenticated"
        );

        let mut seq = auth_seq.wrapping_add(1);
        let mut ping_interval = tokio::time::interval(WS_PING_INTERVAL);
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        ping_interval.reset();

        let mut discovery_interval = tokio::time::interval(DISCOVERY_REFRESH);
        discovery_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        discovery_interval.reset();

        let mut last_frame = tokio::time::Instant::now();

        loop {
            let read_deadline = last_frame + WS_READ_TIMEOUT;
            tokio::select! {
                _ = discovery_interval.tick(), if auto_discover || self.purpose_as_instructions => {
                    match self.list_target_channels().await {
                        Ok(refreshed) => {
                            // Before the move below: an edited purpose leaves
                            // the room set identical, so this cannot hang off
                            // the set-changed branch.
                            self.refresh_room_purposes(&refreshed);
                            let refreshed_map: HashMap<String, bool> = refreshed
                                .into_iter()
                                .map(|target| (target.id, target.is_direct))
                                .collect();
                            // As in the polling loop: only auto-discovery may
                            // replace the operator's explicit scope.
                            if auto_discover && refreshed_map != channel_direct_map {
                                ::clawcrew_log::record!(
                                    INFO,
                                    ::clawcrew_log::Event::new(
                                        module_path!(),
                                        ::clawcrew_log::Action::Note,
                                    )
                                    .with_attrs(::serde_json::json!({
                                        "alias": self.alias,
                                        "before": channel_direct_map.len(),
                                        "after": refreshed_map.len(),
                                    })),
                                    "Mattermost WS in-session auto-discovery refreshed"
                                );
                                channel_direct_map = refreshed_map;
                            }
                        }
                        Err(error) => {
                            ::clawcrew_log::record!(
                                WARN,
                                ::clawcrew_log::Event::new(
                                    module_path!(),
                                    ::clawcrew_log::Action::Note,
                                )
                                .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                                .with_attrs(::serde_json::json!({
                                    "alias": self.alias,
                                    "error": error.to_string(),
                                })),
                                "Mattermost WS in-session discovery refresh failed"
                            );
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    let ping = serde_json::json!({"seq": seq, "action": "ping"});
                    write
                        .send(WsMessage::Text(ping.to_string().into()))
                        .await
                        .context("Mattermost WebSocket ping send failed")?;
                    seq = seq.wrapping_add(1);
                }
                frame = read.next() => {
                    let frame = match frame {
                        Some(Ok(frame)) => {
                            last_frame = tokio::time::Instant::now();
                            frame
                        }
                        Some(Err(error)) => {
                            return Err(error).context("Mattermost WebSocket read failed");
                        }
                        None => bail!("Mattermost WebSocket stream ended"),
                    };

                    let text = match frame {
                        WsMessage::Text(text) => text,
                        WsMessage::Ping(payload) => {
                            write
                                .send(WsMessage::Pong(payload))
                                .await
                                .context("Mattermost WebSocket pong send failed")?;
                            continue;
                        }
                        WsMessage::Close(frame) => {
                            let reason = frame
                                .as_ref()
                                .map(|frame| frame.reason.as_ref())
                                .unwrap_or("");
                            bail!("Mattermost WebSocket closed: {reason}");
                        }
                        _ => continue,
                    };

                    let event: serde_json::Value = match serde_json::from_str(text.as_ref()) {
                        Ok(event) => event,
                        Err(error) => {
                            ::clawcrew_log::record!(
                                WARN,
                                ::clawcrew_log::Event::new(
                                    module_path!(),
                                    ::clawcrew_log::Action::Note,
                                )
                                .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                                .with_attrs(::serde_json::json!({
                                    "alias": self.alias,
                                    "error": error.to_string(),
                                })),
                                "Mattermost WS event parse failed"
                            );
                            continue;
                        }
                    };

                    match event.get("event").and_then(|value| value.as_str()) {
                        Some("posted") => {}
                        // One-tap approvals. Reactions are never routed to the
                        // model, so this arm is terminal either way.
                        Some("reaction_added") => {
                            self.try_resolve_approval_reaction(&event).await;
                            continue;
                        }
                        // Channel metadata changed. Only the cached purpose is
                        // touched; nothing here is routed to the model.
                        Some("channel_updated") => {
                            self.apply_ws_channel_purpose_update(&event);
                            continue;
                        }
                        _ => continue,
                    }

                    let Some(post) = Self::ws_post_from_event(&event) else {
                        continue;
                    };
                    let channel_id = post
                        .get("channel_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    let Some(&is_direct) = channel_direct_map.get(channel_id) else {
                        continue;
                    };

                    if self
                        .process_inbound_post(
                            &post,
                            &bot_user_id,
                            &bot_username,
                            0,
                            channel_id,
                            is_direct,
                            &tx,
                        )
                        .await
                    {
                        return Ok(());
                    }
                }
                _ = tokio::time::sleep_until(read_deadline) => {
                    bail!(
                        "Mattermost WebSocket idle for {} seconds",
                        WS_READ_TIMEOUT.as_secs()
                    );
                }
            }
        }
    }
}

impl MattermostChannel {
    #[allow(clippy::too_many_arguments)]
    async fn process_inbound_post(
        &self,
        post: &serde_json::Value,
        bot_user_id: &str,
        bot_username: &str,
        last_create_at: i64,
        channel_id: &str,
        is_direct: bool,
        tx: &tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> bool {
        let effective_text = if post
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("")
            .trim()
            .is_empty()
            && post_has_audio_attachment(post)
        {
            self.try_transcribe_audio_attachment(post).await
        } else {
            None
        };

        // Approval replies are consumed here, before the normal message path.
        //
        // They must be intercepted ahead of `parse_mattermost_post` because
        // that function applies `mention_only`, and an operator answering
        // `abc123 yes` in a team channel has no reason to @-mention the bot.
        // The allowlist is re-checked explicitly rather than inherited, since
        // skipping the parse also skips its authorization gate — approving a
        // tool call is at least as privileged as sending a message.
        // Only text with no claim on a prompt continues to the model. Every
        // other outcome — resolved, rejected, or arriving after the prompt was
        // already retired — is approval-protocol traffic, so a decision the
        // operator typed can never surface as a chat message.
        if self.try_resolve_approval_reply(post, bot_user_id).await
            == ApprovalReplyOutcome::Consumed
        {
            return false;
        }

        let Some(message) = self.parse_mattermost_post(
            post,
            bot_user_id,
            bot_username,
            last_create_at,
            channel_id,
            effective_text.as_deref(),
            is_direct,
        ) else {
            return false;
        };

        tx.send(message).await.is_err()
    }

    /// Put the two decision emoji on the prompt post so the operator can tap
    /// one instead of typing the token back.
    ///
    /// Best-effort: a failure here leaves the text path, which is the
    /// authoritative one, completely intact. Failing the approval request
    /// because an emoji could not be placed would deny a tool call over a
    /// cosmetic problem, so this only warns.
    ///
    /// Deliberately two emoji, not three. `always` is also accepted by the text
    /// path, but it grants a session-scoped allowlist entry rather than
    /// permitting one call, and a mis-tap next to `approve` is far too cheap a
    /// way to widen that. Escalating to `always` stays a typed decision.
    async fn seed_approval_reactions(&self, post_id: &str) {
        for emoji in [APPROVAL_EMOJI_APPROVE, APPROVAL_EMOJI_DENY] {
            if let Err(error) = self.react_to_post(post_id, emoji).await {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Failure)
                        .with_attrs(::serde_json::json!({
                            "alias": self.alias,
                            "post_id": post_id,
                            "emoji": emoji,
                            "error": error.to_string(),
                        })),
                    "failed to seed Mattermost approval reaction; the text reply path still works"
                );
            }
        }
    }

    /// Drop both handles on a prompt the runtime is about to answer itself.
    ///
    /// Leaving either behind would let a late reply or a late tap resolve a
    /// prompt whose caller has already been told the answer.
    /// Decode a `reaction_added` event's payload. Mattermost sends the reaction
    /// as a JSON *string* under `data.reaction`, the same shape as `data.post`.
    fn ws_reaction_from_event(event: &serde_json::Value) -> Option<serde_json::Value> {
        let reaction = event.get("data")?.get("reaction")?.as_str()?;
        serde_json::from_str(reaction).ok()
    }

    /// Decode a `channel_updated` event's payload. Mattermost sends the channel
    /// as a JSON *string* under `data.channel`, the same shape as `data.post`
    /// and `data.reaction`.
    fn ws_channel_from_event(event: &serde_json::Value) -> Option<serde_json::Value> {
        let channel = event.get("data")?.get("channel")?.as_str()?;
        serde_json::from_str(channel).ok()
    }

    /// Apply a `channel_updated` event to the cached purpose for that one room.
    ///
    /// This is the only signal that arrives when the purpose is *edited*.
    /// Discovery compares the set of rooms, and a purpose edit leaves that set
    /// identical, so without this the change waits out the full refresh
    /// interval. Handling the event makes the edit effective almost
    /// immediately in WebSocket mode; polling keeps the interval as its ceiling.
    fn apply_ws_channel_purpose_update(&self, event: &serde_json::Value) {
        if !self.purpose_as_instructions {
            return;
        }
        let Some(channel) = Self::ws_channel_from_event(event) else {
            return;
        };
        let Some(id) = channel.get("id").and_then(|id| id.as_str()) else {
            return;
        };
        let purpose = channel
            .get("purpose")
            .and_then(|purpose| purpose.as_str())
            .map(str::trim)
            .filter(|purpose| !purpose.is_empty());

        let mut purposes = self.room_purposes.lock();
        match purpose {
            Some(purpose) => {
                purposes.insert(id.to_string(), purpose.to_string());
            }
            // Cleared in Mattermost: stop injecting it rather than leaving the
            // last value cached until the next discovery pass.
            None => {
                purposes.remove(id);
            }
        }
    }

    /// Resolve a pending approval from a reaction on its prompt post.
    ///
    /// Returns `true` when the reaction was consumed as a decision. Unlike the
    /// reply path there is nothing to keep away from the model — a reaction is
    /// never routed as a message — so the return value exists for tests and
    /// logging rather than for control flow.
    async fn try_resolve_approval_reaction(&self, event: &serde_json::Value) -> bool {
        let Some(reaction) = Self::ws_reaction_from_event(event) else {
            return false;
        };
        let emoji = reaction
            .get("emoji_name")
            .and_then(|emoji| emoji.as_str())
            .unwrap_or("");
        let Some(response) = approval_response_for_emoji(emoji) else {
            return false;
        };
        let post_id = reaction
            .get("post_id")
            .and_then(|post| post.as_str())
            .unwrap_or("");

        // Cheap check first: a reaction on any other post is not ours, and
        // rejecting it here keeps unrelated chatter out of the auth logs.
        let Some((token, generation)) = self.approvals.lock().posts.get(post_id).cloned() else {
            return false;
        };

        let user_id = reaction
            .get("user_id")
            .and_then(|user| user.as_str())
            .unwrap_or("");

        // Resolved before the lock so the decision below stays one
        // uninterruptible critical section.
        let (bot_user_id, _) = self.get_bot_identity().await;

        // The bot seeds both emoji itself, so its own reactions come straight
        // back as events. Answering them would auto-approve every prompt the
        // instant it is posted. An event with no author is rejected for the same
        // reason it is on the text path: a wildcard peer group would otherwise
        // match the empty string and treat "no identity" as authorized.
        //
        // An unknown bot ID fails closed here too, and this path is the sharper
        // case: the bot's own seeded ✅ arrives within milliseconds of the
        // prompt, so skipping the self-check while identity is unresolved would
        // approve the tool call almost immediately and without any human input.
        let reason = if user_id.is_empty() {
            Some("missing_author_identity")
        } else if bot_user_id.is_empty() {
            Some("unknown_bot_identity")
        } else if user_id == bot_user_id {
            // The bot's own seeded reaction: expected on every prompt, so this
            // is routine rather than a rejection worth logging.
            return false;
        } else {
            None
        };
        if let Some(reason) = reason {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                    .with_outcome(::clawcrew_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({"token": token, "reason": reason})),
                "refusing to decide an approval without a trustworthy bot identity"
            );
            return false;
        }

        if !self.is_user_allowed(user_id) {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                    .with_outcome(::clawcrew_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({
                        "user_id": user_id,
                        "token": token,
                        "emoji": emoji,
                    })),
                "ignoring approval reaction from unauthorized user"
            );
            // Leave the prompt pending so an authorized operator can still
            // answer it, by reaction or by reply.
            return false;
        }

        // Retire by generation, not by token. This post belongs to one specific
        // registration; if a later request has since drawn the same token, a tap
        // on this older, still-visible prompt must not resolve it. Retiring also
        // drops the post binding, so a second tap finds nothing.
        let Some(pending) = self.approvals.lock().retire_generation(&token, generation) else {
            return false;
        };
        // A closed receiver means the waiter already timed out; the decision is
        // simply dropped.
        let _ = pending.sender.send(response);
        true
    }

    /// Consume `post` if it is an authorized reply to a pending approval
    /// prompt. Returns `true` when the post was consumed, so the caller skips
    /// the normal message path and the reply never reaches the model.
    /// `bot_user_id` is the authenticated identity from `get_bot_identity`.
    /// It is passed in rather than read here because this runs *before*
    /// `parse_mattermost_post`, and so inherits none of that parser's
    /// protections — including its self-loop guard.
    async fn try_resolve_approval_reply(
        &self,
        post: &serde_json::Value,
        bot_user_id: &str,
    ) -> ApprovalReplyOutcome {
        let text = post
            .get("message")
            .and_then(|message| message.as_str())
            .unwrap_or("");
        let Some((token, response)) = galleon_channel_core::approval::parse_approval_reply(text)
        else {
            return ApprovalReplyOutcome::NotFound;
        };

        let user_id = post
            .get("user_id")
            .and_then(|user| user.as_str())
            .unwrap_or("");
        let reply_channel = post
            .get("channel_id")
            .and_then(|channel| channel.as_str())
            .unwrap_or("");

        // Ownership first. A refusal below must never swallow text this channel
        // has no claim on: with the identity checks ahead of this lookup, an
        // unresolved bot identity turned *every* six-character approval-shaped
        // message into `Consumed`, losing unrelated user text instead of
        // forwarding it. Peeking here is safe even though the decision itself
        // happens in a later critical section, because a token retired in
        // between is still recognized through `was_retired`.
        // The observed registration is captured here, not just the fact that the
        // token is ours. `get_bot_identity` below may await, and a request that
        // draws this token during that window must not receive this reply.
        let observed_generation = {
            let state = self.approvals.lock();
            match state.pending.get(&token) {
                Some(pending) => Some(pending.generation),
                None if state.was_retired(&token) => None,
                None => return ApprovalReplyOutcome::NotFound,
            }
        };

        // The token is ours, so a decision is genuinely at stake — recover the
        // identity if the listener started before `/users/me` was reachable.
        // The listener copies the ID once at startup and never refetches, so
        // without this a transient lookup failure would leave every typed
        // approval permanently unanswerable until the daemon was restarted.
        // Empty results are deliberately not cached, so this recovers as soon as
        // the endpoint does; a successful lookup is cached and costs nothing
        // thereafter.
        let recovered;
        let bot_user_id = if bot_user_id.is_empty() {
            recovered = self.get_bot_identity().await.0;
            recovered.as_str()
        } else {
            bot_user_id
        };

        // Identity is checked before the allowlist, not through it. A wildcard
        // peer group matches any string, including the empty one this yields for
        // a malformed event and including the bot's own ID — so consulting the
        // allowlist first would let the bot's own approval-shaped post answer a
        // live prompt, and would treat "no identity" as "some authorized user".
        //
        // An *unknown* bot ID fails closed rather than skipping the comparison.
        // Treating it as "no self-check needed" would invert the guard exactly
        // when identity is least certain: under a wildcard peer group the bot's
        // own post would then pass authorization and approve the tool it had
        // just prompted for.
        let reason = if user_id.is_empty() {
            Some("missing_author_identity")
        } else if bot_user_id.is_empty() {
            Some("unknown_bot_identity")
        } else if user_id == bot_user_id {
            Some("self_authored")
        } else {
            None
        };
        if let Some(reason) = reason {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                    .with_outcome(::clawcrew_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({"token": token, "reason": reason})),
                "refusing to decide an approval without a trustworthy author identity"
            );
            // Consumed, and the prompt stays pending: approval-shaped text must
            // never reach the model, and refusing here only defers the decision
            // to a reply made once identity can be established.
            return ApprovalReplyOutcome::Consumed;
        }

        // One critical section for the whole decision. Recognition, destination,
        // and removal cannot be split: the reaction path shares this state, and
        // a token retired between two of those steps would otherwise look like
        // text that was never ours and be forwarded to the model.
        let (outcome, resolved) = {
            let mut state = self.approvals.lock();

            let Some(pending) = state.pending.get(&token) else {
                // Already answered, cancelled, or timed out: still ours, so the
                // losing reply is swallowed rather than delivered as chat.
                return if state.was_retired(&token) {
                    ApprovalReplyOutcome::Consumed
                } else {
                    ApprovalReplyOutcome::NotFound
                };
            };

            // A different registration holds the token than the one this reply
            // was recognized against — it was reused while the identity lookup
            // awaited. The reply is still approval traffic, so it is swallowed,
            // but it must not answer a prompt it was never shown.
            if observed_generation != Some(pending.generation) {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                        .with_outcome(::clawcrew_log::EventOutcome::Failure)
                        .with_attrs(::serde_json::json!({"token": token})),
                    "ignoring approval reply whose prompt was replaced by a newer request"
                );
                return ApprovalReplyOutcome::Consumed;
            }

            // The reply has to come from the channel the prompt went to. The
            // token is readable by everyone who can see the prompt, and this one
            // channel instance serves many Mattermost channels, so without this
            // an authorized user could carry a token into a different room and
            // answer there. Peer-group membership is a separate, necessary
            // condition: it says *who* may decide, not *where*.
            if pending.channel_id != reply_channel {
                let origin_channel = pending.channel_id.clone();
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                        .with_outcome(::clawcrew_log::EventOutcome::Failure)
                        .with_attrs(::serde_json::json!({
                            "token": token,
                            "reply_channel": reply_channel,
                            "origin_channel": origin_channel,
                        })),
                    "ignoring approval reply from a channel the prompt was not posted to"
                );
                // Consumed, not resolved: the prompt stays answerable in the
                // channel it actually belongs to.
                return ApprovalReplyOutcome::Consumed;
            }

            if !self.is_user_allowed(user_id) {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                        .with_outcome(::clawcrew_log::EventOutcome::Failure)
                        .with_attrs(::serde_json::json!({"user_id": user_id, "token": token})),
                    "ignoring approval reply from unauthorized user"
                );
                // Leaves the prompt pending so an authorized operator can still
                // answer, but never forwards the attempt to the model.
                return ApprovalReplyOutcome::Consumed;
            }

            (
                ApprovalReplyOutcome::Consumed,
                observed_generation
                    .and_then(|generation| state.retire_generation(&token, generation))
                    .map(|pending| pending.sender),
            )
        };

        if let Some(sender) = resolved {
            // A closed receiver means the waiter already gave up; the decision
            // is dropped and the reply is still consumed.
            let _ = sender.send(response);
        }
        outcome
    }

    #[allow(clippy::too_many_arguments)]
    async fn poll_channel(
        &self,
        target: &TargetChannel,
        token: &str,
        bot_user_id: &str,
        bot_username: &str,
        bootstrap_ms: i64,
        cursors: &mut HashMap<String, i64>,
        tx: &tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> bool {
        let cursor = *cursors.entry(target.id.clone()).or_insert(bootstrap_ms);

        let resp = match self
            .http_client()
            .get(format!(
                "{}/api/v4/channels/{}/posts",
                self.base_url, target.id
            ))
            .bearer_auth(token)
            .query(&[("since", cursor.to_string())])
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({
                            "alias": self.alias,
                            "channel_id": target.id,
                            "error": format!("{}", e),
                        })),
                    "Mattermost poll error"
                );
                return false;
            }
        };

        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({
                            "alias": self.alias,
                            "channel_id": target.id,
                            "error": format!("{}", e),
                        })),
                    "Mattermost parse error"
                );
                return false;
            }
        };

        let Some(posts) = data.get("posts").and_then(|p| p.as_object()) else {
            return false;
        };

        let mut post_list: Vec<_> = posts.values().collect();
        post_list.sort_by_key(|p| p.get("create_at").and_then(|c| c.as_i64()).unwrap_or(0));

        let cursor_before_batch = cursor;
        let mut new_cursor = cursor;
        for post in post_list {
            let create_at = post
                .get("create_at")
                .and_then(|c| c.as_i64())
                .unwrap_or(new_cursor);
            new_cursor = new_cursor.max(create_at);

            if self
                .process_inbound_post(
                    post,
                    bot_user_id,
                    bot_username,
                    cursor_before_batch,
                    &target.id,
                    target.is_direct,
                    tx,
                )
                .await
            {
                return true;
            }
        }
        cursors.insert(target.id.clone(), new_cursor);
        false
    }

    fn parse_mattermost_post(
        &self,
        post: &serde_json::Value,
        bot_user_id: &str,
        bot_username: &str,
        last_create_at: i64,
        channel_id: &str,
        injected_text: Option<&str>,
        is_direct: bool,
    ) -> Option<ChannelMessage> {
        let id = post.get("id").and_then(|i| i.as_str()).unwrap_or("");
        let user_id = post.get("user_id").and_then(|u| u.as_str()).unwrap_or("");
        let text = post.get("message").and_then(|m| m.as_str()).unwrap_or("");
        let create_at = post.get("create_at").and_then(|c| c.as_i64()).unwrap_or(0);
        let root_id = post.get("root_id").and_then(|r| r.as_str()).unwrap_or("");

        // Without a bot identity the self-loop check below cannot be enforced:
        // an empty `bot_user_id` equals no real author, so every post the bot
        // wrote would pass it. Refuse to admit anything rather than admit the
        // bot's own posts.
        //
        // `require_bot_identity` already prevents a listener from running in
        // this state, so in production this is unreachable. It is kept as the
        // second half of the boundary: the guarantee then belongs to the
        // admission path itself rather than to a precondition a future caller
        // might bypass.
        if bot_user_id.is_empty() {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                    .with_outcome(::clawcrew_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({"alias": self.alias, "post_id": id})),
                "refusing to admit a Mattermost post without a known bot identity"
            );
            return None;
        }

        if user_id == bot_user_id || create_at <= last_create_at {
            return None;
        }

        let effective_text = if text.is_empty() {
            injected_text?
        } else {
            text
        };

        if !self.is_user_allowed(user_id) {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({"user_id": user_id})),
                "ignoring message from unauthorized user"
            );
            return None;
        }

        // DM and group-DM channels have no ambient noise to filter against, so
        // mention_only is bypassed for them. The flag still applies on public
        // and private team channels.
        let content = if self.mention_only && !is_direct {
            let normalized =
                normalize_mattermost_content(effective_text, bot_user_id, bot_username, post);
            normalized?
        } else {
            effective_text.to_string()
        };

        // Reply routing depends on thread_replies config:
        //   - Existing thread (root_id set): always stay in the thread.
        //   - Top-level post + thread_replies=true: thread on the original post.
        //   - Top-level post + thread_replies=false: reply at channel level.
        let reply_target = if !root_id.is_empty() {
            format!("{}:{}", channel_id, root_id)
        } else if self.thread_replies {
            format!("{}:{}", channel_id, id)
        } else {
            channel_id.to_string()
        };

        Some(ChannelMessage {
            id: format!("mattermost_{id}"),
            sender: user_id.to_string(),
            reply_target,
            content,
            channel: "mattermost".to_string(),
            channel_alias: Some(self.alias.clone()),
            #[allow(clippy::cast_sign_loss)]
            timestamp: (create_at / 1000) as u64,
            thread_ts: None,
            interruption_scope_id: None,
            attachments: vec![],
            subject: None,

            ..Default::default()
        })
    }
}

fn post_has_audio_attachment(post: &serde_json::Value) -> bool {
    let files = post
        .get("metadata")
        .and_then(|m| m.get("files"))
        .and_then(|f| f.as_array());
    let Some(files) = files else { return false };
    files.iter().any(is_audio_file)
}

fn is_audio_file(file: &serde_json::Value) -> bool {
    let mime = file.get("mime_type").and_then(|m| m.as_str()).unwrap_or("");
    if mime.starts_with("audio/") {
        return true;
    }
    let ext = file.get("extension").and_then(|e| e.as_str()).unwrap_or("");
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "ogg" | "mp3" | "m4a" | "wav" | "opus" | "flac"
    )
}

#[cfg(test)]
fn contains_bot_mention_mm(
    text: &str,
    bot_user_id: &str,
    bot_username: &str,
    post: &serde_json::Value,
) -> bool {
    // 1. Text-based: @username (case-insensitive, word-boundary aware)
    if !find_bot_mention_spans(text, bot_username).is_empty() {
        return true;
    }

    // 2. Metadata-based: Mattermost may include a "metadata.mentions" array of user IDs.
    if !bot_user_id.is_empty()
        && let Some(mentions) = post
            .get("metadata")
            .and_then(|m| m.get("mentions"))
            .and_then(|m| m.as_array())
        && mentions.iter().any(|m| m.as_str() == Some(bot_user_id))
    {
        return true;
    }

    false
}

fn is_mattermost_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
}

fn find_bot_mention_spans(text: &str, bot_username: &str) -> Vec<(usize, usize)> {
    if bot_username.is_empty() {
        return Vec::new();
    }

    let mention = format!("@{}", bot_username.to_ascii_lowercase());
    let mention_len = mention.len();
    if mention_len == 0 {
        return Vec::new();
    }

    let mention_bytes = mention.as_bytes();
    let text_bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;

    while index + mention_len <= text_bytes.len() {
        let is_match = text_bytes[index] == b'@'
            && text_bytes[index..index + mention_len]
                .iter()
                .zip(mention_bytes.iter())
                .all(|(left, right)| left.eq_ignore_ascii_case(right));

        if is_match {
            let end = index + mention_len;
            let at_boundary = text[end..]
                .chars()
                .next()
                .is_none_or(|next| !is_mattermost_username_char(next));
            if at_boundary {
                spans.push((index, end));
                index = end;
                continue;
            }
        }

        let step = text[index..].chars().next().map_or(1, char::len_utf8);
        index += step;
    }

    spans
}

fn normalize_mattermost_content(
    text: &str,
    bot_user_id: &str,
    bot_username: &str,
    post: &serde_json::Value,
) -> Option<String> {
    let mention_spans = find_bot_mention_spans(text, bot_username);
    let metadata_mentions_bot = !bot_user_id.is_empty()
        && post
            .get("metadata")
            .and_then(|m| m.get("mentions"))
            .and_then(|m| m.as_array())
            .is_some_and(|mentions| mentions.iter().any(|m| m.as_str() == Some(bot_user_id)));

    if mention_spans.is_empty() && !metadata_mentions_bot {
        return None;
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
include!("tests.rs");
