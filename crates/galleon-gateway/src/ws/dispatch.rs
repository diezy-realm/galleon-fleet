//! WebSocket message dispatching, frame construction, and turn execution.

use super::*;
use crate::ws_approval::PendingApprovals;
use crate::{gateway_cancel_key, register_cancel_token, remove_cancel_token_if_current};
use axum::extract::ws::{Message, WebSocket};
use galleon_api::channel::ChannelApprovalResponse;
use galleon_api::sop::{
    ApprovalDecision as SopApprovalDecision, ApprovalPrincipal as SopApprovalPrincipal,
};
use futures_util::SinkExt;
use std::sync::Arc;

pub(crate) async fn handle_ws_sop_frame<S>(
    parsed: &serde_json::Value,
    state: &AppState,
    session_id: &str,
    auth_subject: Option<&str>,
    sender: &mut S,
) -> bool
where
    S: SinkExt<Message> + Unpin,
{
    if parsed["kind"].as_str() != Some("sop") {
        return false;
    }
    let run_id = parsed["run_id"].as_str().unwrap_or("").to_string();
    let decision = match parsed["decision"].as_str().unwrap_or("") {
        "approve" => Some(SopApprovalDecision::Approve),
        // Thread the optional reason through, like the HTTP/CLI deny surfaces, so
        // the ledger records it.
        "deny" => Some(SopApprovalDecision::Deny {
            reason: parsed["reason"].as_str().map(str::to_string),
        }),
        _ => None,
    };
    // run_id + a valid decision are both required; the let-else avoids an expect
    // on the downstream resolve (codebase rule: no expect/unwrap in production).
    let Some(decision) = decision.filter(|_| !run_id.is_empty()) else {
        let err = serde_json::json!({
            "type": "error",
            "message": galleon_i18n::get_required_cli_string(
                "cli-sop-ws-invalid-approval"
            ),
            "code": "INVALID_APPROVAL_RESPONSE"
        });
        let _ = sender.send(Message::Text(err.to_string().into())).await;
        return true;
    };
    let frame = if let Some(runner) = state.sop_runner.as_ref() {
        let principal =
            SopApprovalPrincipal::ws(session_id.to_string(), auth_subject.map(str::to_string));
        match runner.resolve_approval(&run_id, decision, principal).await {
            Ok(outcome) => serde_json::json!({
                "type": "sop_approval_result",
                "run_id": run_id,
                "outcome": outcome.label(),
            }),
            Err(e) => serde_json::json!({
                "type": "error",
                "message": galleon_i18n::get_required_cli_string_with_args(
                    "cli-sop-ws-resolve-failed",
                    &[("error", &e.to_string())],
                ),
                "code": "SOP_RESOLVE_FAILED"
            }),
        }
    } else {
        serde_json::json!({
            "type": "error",
            "message": galleon_i18n::get_required_cli_string(
                "cli-sop-ws-subsystem-disabled"
            ),
            "code": "SOP_DISABLED"
        })
    };
    let _ = sender.send(Message::Text(frame.to_string().into())).await;
    true
}

pub(crate) fn history_trimmed_ws_frame(
    dropped_messages: usize,
    kept_turns: usize,
    reason: &str,
    token_budget: Option<u64>,
    tokens_before: Option<u64>,
    tokens_after: Option<u64>,
    tokens_before_source: Option<&str>,
    tokens_after_source: Option<&str>,
    unsatisfiable_floor: Option<bool>,
) -> serde_json::Value {
    let mut frame = serde_json::json!({
        "type": "history_trimmed",
        "dropped_messages": dropped_messages,
        "kept_turns": kept_turns,
        "reason": reason,
    });
    if let Some(token_budget) = token_budget {
        frame["token_budget"] = token_budget.into();
    }
    if let Some(tokens_before) = tokens_before {
        frame["tokens_before"] = tokens_before.into();
    }
    if let Some(tokens_after) = tokens_after {
        frame["tokens_after"] = tokens_after.into();
    }
    if let Some(tokens_before_source) = tokens_before_source {
        frame["tokens_before_source"] = tokens_before_source.into();
    }
    if let Some(tokens_after_source) = tokens_after_source {
        frame["tokens_after_source"] = tokens_after_source.into();
    }
    if let Some(unsatisfiable_floor) = unsatisfiable_floor {
        frame["unsatisfiable_floor"] = unsatisfiable_floor.into();
    }
    frame
}

/// Build the display-only `safeguard_fallback` WS frame from a drained
/// [`galleon_providers::SafeguardFallbackNotice`], or `None` when no notice
/// was recorded this turn (so "no notice → no frame" is enforced here).
///
/// Privacy contract: only the requested/served model names and the fallback
/// layer (`server`/`client`) cross the wire. The classifier `category` (and any
/// refusal explanation) are logs-only and MUST NEVER reach the browser — this
/// helper deliberately never reads `notice.category`.
pub(crate) fn safeguard_fallback_ws_frame(
    notice: Option<&galleon_providers::SafeguardFallbackNotice>,
) -> Option<serde_json::Value> {
    let notice = notice?;
    let fallback_kind = match notice.kind {
        galleon_providers::SafeguardFallbackKind::ServerSide => "server",
        galleon_providers::SafeguardFallbackKind::ClientSide => "client",
        galleon_providers::SafeguardFallbackKind::ClientAndServer => "client_server",
    };
    Some(serde_json::json!({
        "type": "safeguard_fallback",
        "fallback_kind": fallback_kind,
        "requested_model": notice.requested_model,
        "served_model": notice.served_model,
    }))
}

pub(crate) fn success_terminal_ws_frames(
    safeguard_notice: Option<&galleon_providers::SafeguardFallbackNotice>,
    done: serde_json::Value,
) -> Vec<serde_json::Value> {
    safeguard_fallback_ws_frame(safeguard_notice)
        .into_iter()
        .chain(std::iter::once(done))
        .collect()
}

pub(crate) fn needs_onboarding_ws_error(
    config: &galleon_config::schema::Config,
) -> Option<serde_json::Value> {
    let model = config.resolve_default_model().unwrap_or_default();
    crate::needs_quickstart_for(&model)?;
    Some(serde_json::json!({
        "type": "error",
        "error": "needs_onboarding",
        "code": "NEEDS_ONBOARDING",
        "message": crate::needs_quickstart_channel_reply(),
        "url": "/onboard",
    }))
}

pub(crate) fn first_chat_message_content(text: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(text).ok()?;
    (parsed["type"].as_str() == Some("message"))
        .then(|| parsed["content"].as_str().unwrap_or("").to_string())
        .filter(|content| !content.is_empty())
}

pub(crate) fn resolve_done_context_limits(
    usage_budget: Option<u64>,
    usage_model_window: Option<u64>,
    active_limits: galleon_config::schema::ResolvedContextLimits,
) -> (u64, Option<u64>) {
    (
        usage_budget.unwrap_or(active_limits.context_token_budget as u64),
        usage_model_window.or_else(|| {
            active_limits
                .configured_model_context_window()
                .map(|tokens| tokens as u64)
        }),
    )
}

/// Terminal-frame budget/window for the `done` event.
///
/// `final_limits` is the route that actually served the LAST call, carried out
/// of the turn loop. When present it is AUTHORITATIVE and overrides the
/// usage-derived values, which can be stale: an earlier usage-bearing route
/// before a final no-usage call (e.g. a vision reply without token usage) would
/// otherwise leave the frame on the earlier route's numbers. Only when no call
/// was served this turn (a cache hit) does `final_limits` become `None`, and the
/// frame falls back to the usage values, then to `fallback_limits`.
pub(crate) fn done_frame_context_limits(
    final_limits: Option<galleon_config::schema::ResolvedContextLimits>,
    usage_budget: Option<u64>,
    usage_model_window: Option<u64>,
    fallback_limits: Option<galleon_config::schema::ResolvedContextLimits>,
) -> (u64, Option<u64>) {
    match final_limits {
        Some(limits) => (
            limits.context_token_budget as u64,
            limits.configured_model_context_window().map(|t| t as u64),
        ),
        None => resolve_done_context_limits(
            usage_budget,
            usage_model_window,
            fallback_limits.unwrap_or(galleon_config::schema::ResolvedContextLimits {
                model_context_window: galleon_config::schema::UNCONFIGURED_CONTEXT_WINDOW_FALLBACK,
                context_token_budget: 0,
                model_context_window_source:
                    galleon_config::schema::ModelContextWindowSource::CompatibilityFallback,
            }),
        ),
    }
}

/// Per-provider usage snapshot in the `usage_by_provider` done-frame array.
/// Tracks ALL billable attempts (accepted + rejected Reliable attempts).
/// The scalar `cost_usd` in the done frame is the sum of `usage_by_provider[*].cost_usd`,
/// making the breakdown the single source of truth for both token counts and cost.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProviderUsageEntry {
    pub(crate) provider_ref: String,
    pub(crate) model: String,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) cached_input_tokens: u64,
    pub(crate) cost_usd: f64,
}

/// Fold state for `TurnEvent::Usage` inside the WS chat loop.
///
/// `process_chat_message` owns one per turn and feeds it every Usage event
/// verbatim; the done/cancel frame paths then read the accumulated state.
/// Billing aggregation (turn-wide totals plus the per-(provider, model)
/// breakdown) accumulates every billable attempt, including rejected ones.
/// The accepted-serving snapshot (`last_provider_ref` / `last_model` /
/// `last_input_tokens`) advances only on `accepted: true` events, so a later
/// billed rejected attempt cannot re-point the terminal identity or the
/// context-meter ceiling (see the `TurnEvent::Usage` contract).
#[derive(Debug, Default)]
pub(crate) struct UsageFold {
    pub(crate) total_input_tokens: Option<u64>,
    pub(crate) total_output_tokens: Option<u64>,
    pub(crate) last_provider_ref: Option<String>,
    pub(crate) last_model: Option<String>,
    pub(crate) last_input_tokens: Option<u64>,
    pub(crate) last_context_token_budget: Option<u64>,
    pub(crate) last_model_context_window: Option<u64>,
    pub(crate) usage_by_provider: std::collections::HashMap<(String, String), ProviderUsageEntry>,
}

impl UsageFold {
    pub(crate) fn apply(&mut self, event: galleon_api::agent::TurnEvent) {
        let galleon_api::agent::TurnEvent::Usage {
            input_tokens,
            cached_input_tokens,
            output_tokens,
            cost_usd,
            context_token_budget,
            model_context_window,
            provider_ref,
            model: served_model,
            accepted,
        } = event
        else {
            return;
        };
        // Turn-wide billing totals accumulate every billable attempt,
        // including rejected ones (`accepted: false` is billing-only
        // telemetry per the TurnEvent::Usage contract). Only the
        // accepted-serving snapshot below is gated on `accepted`.
        if let Some(it) = input_tokens {
            self.total_input_tokens = Some(self.total_input_tokens.unwrap_or(0) + it);
        }
        if let Some(ot) = output_tokens {
            self.total_output_tokens = Some(self.total_output_tokens.unwrap_or(0) + ot);
        }
        if accepted {
            self.last_provider_ref = Some(provider_ref.clone());
            self.last_model = Some(served_model.clone());
            self.last_context_token_budget = context_token_budget;
            self.last_model_context_window = model_context_window;
            if let Some(it) = input_tokens {
                self.last_input_tokens = Some(it);
            } else {
                // Accepted call returned no usage data; clear the previous
                // route's input snapshot to prevent stale values from a
                // different route being rendered against this route's
                // context window.
                self.last_input_tokens = None;
            }
        }
        // Per-(provider, model) breakdown accumulation.
        // The event-owned strings move into the map key; entry
        // fields clone from the key only on insert, so repeat
        // events for a known pair cost no extra clones.
        let entry = self
            .usage_by_provider
            .entry((provider_ref, served_model))
            .or_insert_with_key(|(provider_ref, model)| ProviderUsageEntry {
                provider_ref: provider_ref.clone(),
                model: model.clone(),
                ..Default::default()
            });
        if let Some(it) = input_tokens {
            entry.input_tokens = entry.input_tokens.saturating_add(it);
        }
        if let Some(ot) = output_tokens {
            entry.output_tokens = entry.output_tokens.saturating_add(ot);
        }
        if let Some(ct) = cached_input_tokens {
            entry.cached_input_tokens = entry.cached_input_tokens.saturating_add(ct);
        }
        if let Some(cu) = cost_usd {
            entry.cost_usd += cu;
        }
    }

    /// Sorted per-(provider, model) breakdown in wire order; drains the map.
    /// `total_cost_usd` sums over this vector — never over the HashMap
    /// directly — so float accumulation order (and the emitted total) is
    /// deterministic.
    pub(crate) fn take_sorted_entries(&mut self) -> Vec<ProviderUsageEntry> {
        let mut entries: Vec<_> = std::mem::take(&mut self.usage_by_provider)
            .into_values()
            .collect();
        entries.sort_by(|a, b| {
            a.provider_ref
                .cmp(&b.provider_ref)
                .then(a.model.cmp(&b.model))
        });
        entries
    }

    /// Turn-wide cost total over the sorted breakdown; `None` when nothing
    /// billable was recorded.
    pub(crate) fn total_cost_usd(entries: &[ProviderUsageEntry]) -> Option<f64> {
        let sum: f64 = entries.iter().map(|e| e.cost_usd).sum();
        if sum > 0.0 { Some(sum) } else { None }
    }
}

/// Scalar inputs to build a done-frame JSON.
/// `usage_by_provider` is kept as a separate arg (different concern).
/// `cost_usd` is derived as the sum of `usage_by_provider[*].cost_usd`,
/// which includes ALL billable attempts (accepted + rejected).
#[derive(Debug, Clone)]
pub(crate) struct DoneFrameMeta<'a> {
    pub(crate) full_response: &'a str,
    pub(crate) input_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
    pub(crate) tokens_used: Option<u64>,
    pub(crate) cost_usd: Option<f64>,
    pub(crate) model: &'a str,
    pub(crate) provider: &'a str,
    pub(crate) provider_ref: &'a str,
    pub(crate) max_context_tokens: u64,
    pub(crate) model_context_window: Option<u64>,
    pub(crate) last_input_tokens: Option<u64>,
    pub(crate) last_serving_provider_ref: Option<&'a str>,
    pub(crate) last_serving_model: Option<&'a str>,
}

/// Build the `done`-frame JSON.
/// `provider` carries the full configured `<type>.<alias>` ref, matching the
/// long-standing wire semantic; `provider_ref` carries the serving identity
/// resolved from usage events (falling back to the turn-start provider).
pub(crate) fn build_done_frame_json(
    meta: &DoneFrameMeta,
    usage_by_provider: &[ProviderUsageEntry],
) -> serde_json::Value {
    let mut done = serde_json::json!({
        "type": "done",
        "full_response": meta.full_response,
        "input_tokens": meta.input_tokens,
        "output_tokens": meta.output_tokens,
        "tokens_used": meta.tokens_used,
        "cost_usd": meta.cost_usd,
        "model": meta.model,
        "provider": meta.provider,
        "provider_ref": meta.provider_ref,
        "max_context_tokens": meta.max_context_tokens,
        "last_input_tokens": meta.last_input_tokens,
        "last_serving_provider_ref": meta.last_serving_provider_ref,
        "last_serving_model": meta.last_serving_model,
        "usage_by_provider": usage_by_provider.iter().map(|e| serde_json::json!({
            "provider_ref": e.provider_ref,
            "model": e.model,
            "input_tokens": e.input_tokens,
            "output_tokens": e.output_tokens,
            "cached_input_tokens": e.cached_input_tokens,
            "cost_usd": e.cost_usd,
        })).collect::<Vec<_>>(),
    });
    if let Some(window) = meta.model_context_window {
        done["model_context_window"] = serde_json::Value::from(window);
    }
    done
}

/// Process a single chat message through the agent and send the response.
/// Uses [`Agent::turn_streamed`] so that intermediate text chunks, tool calls,
/// and tool results are forwarded to the WebSocket client in real time.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_chat_message(
    state: &AppState,
    agent: &mut (dyn galleon_api::agent::AgentSession + '_),
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    receiver: &mut futures_util::stream::SplitStream<WebSocket>,
    approval_event_rx: &mut tokio::sync::mpsc::Receiver<galleon_api::agent::TurnEvent>,
    pending_approvals: &PendingApprovals,
    ping_interval: &mut Option<tokio::time::Interval>,
    ws_memory: &Option<Arc<dyn galleon_memory::Memory>>,
    content: &str,
    session_key: &str,
    session_id: &str,
    // Transport-authenticated approval subject (paired-token hash), threaded so a
    // mid-turn SOP approval frame carries the same identity as the top-level path.
    auth_subject: Option<&str>,
) {
    use galleon_api::agent::TurnEvent;
    use futures_util::StreamExt as _;

    let (turn_alias, turn_provider, turn_model) = agent.attribution_fields();
    let provider_label = turn_provider.clone();
    let cost_tracking_context = state.cost_tracker.as_ref().map(|tracker| {
        let config = state.config.read();
        let pricing = galleon_config::cost::build_model_provider_pricing(&config);
        galleon_config::cost::ToolLoopCostTrackingContext::new(tracker.clone(), Arc::new(pricing))
            .with_agent_alias(&turn_alias)
    });
    let turn_usage = state.cost_tracker.as_ref().map(|_| {
        Arc::new(parking_lot::Mutex::new(
            galleon_config::cost::TurnUsage::default(),
        ))
    });

    // Broadcast agent_start event
    let _ = state.event_tx.send(serde_json::json!({
        "type": "agent_start",
        "model_provider": provider_label,
        "model": turn_model,
    }));

    // Set session state to running
    let turn_id = uuid::Uuid::new_v4().to_string();
    if let Some(ref backend) = state.session_backend {
        let _ = backend.set_session_state(session_key, "running", Some(&turn_id));
    }

    // ── Cancellation token lifecycle ─────────────────────────────
    // Create a token before the turn starts so the abort endpoint
    // can cancel it. Remove it after the turn completes regardless
    // of outcome (normal, error, or cancelled). Registration uses the
    // process-local cancellation key shared with the webhook SSE transport,
    // while persistence stays on the raw transcript key.
    let cancel_token = Arc::new(tokio_util::sync::CancellationToken::new());
    register_cancel_token(
        &state.cancel_tokens,
        &gateway_cancel_key(session_id),
        Arc::clone(&cancel_token),
    );

    // Channel for streaming turn events from the agent.
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<TurnEvent>(64);
    let (steering_tx, mut steering_rx) = tokio::sync::mpsc::channel::<String>(32);

    let content_owned = content.to_string();
    let session_key_owned = session_key.to_string();
    // The shared Agent turn boundary owns safeguard attribution and returns it
    // alongside the undecorated transcript. This transport only renders the
    // typed result as a standalone WS frame.
    let turn_fut = Box::pin(async {
        use ::galleon_log::Instrument as _;
        let span = ::galleon_log::info_span!(
            target: "clawcrew_log_internal_scope",
            "clawcrew_scope",
            session_key = %session_key_owned,
            agent_alias = %turn_alias,
            model_provider = %turn_provider,
            model = %turn_model,
            channel = WS_CHANNEL_KEY,
        );
        galleon_api::scope_session_key(
            Some(session_key_owned.clone()),
            galleon_config::cost::TOOL_LOOP_TURN_USAGE.scope(
                turn_usage.clone(),
                galleon_config::cost::TOOL_LOOP_COST_TRACKING_CONTEXT.scope(
                    cost_tracking_context.clone(),
                    agent
                        .turn_streamed_with_steering_state(
                            &content_owned,
                            event_tx,
                            Some(cancel_token.as_ref().clone()),
                            Some(&mut steering_rx),
                        )
                        .instrument(span),
                ),
            ),
        )
        .await
    });

    // Drive both futures concurrently: the agent turn produces events
    // and we relay them over WebSocket. Track streamed chunks so we
    // can reconstruct partial content on cancellation.
    let mut accumulated_text = String::new();

    // Aggregate token usage across all LLM calls in this turn.
    // The agent emits TurnEvent::Usage once per LLM call when the provider
    // surfaces usage; we sum to produce a single done-frame total.
    // `UsageFold` holds both the billing aggregation (every billable attempt,
    // including rejected ones) and the accepted-serving snapshot (accepted
    // events only) that the done/cancel frames render.
    let mut usage_fold = UsageFold::default();

    let forward_fut = async {
        let mut cancel_drained = false;
        loop {
            tokio::select! {
                            biased;
                            _ = cancel_token.cancelled(), if !cancel_drained => {
                                let drained: Vec<_> = pending_approvals.lock().drain().collect();
                                drop(drained);
                                cancel_drained = true;
                                // Fall through; the agent loop will now wake from the
                                // approval await, see the cancel token, and propagate
                                // a ToolLoopCancelled error which closes event_rx and
                                // breaks this loop on the `event_rx.recv()` arm below.
                            }
                            client_msg = receiver.next() => {
                                let text = match client_msg {
                                    Some(Ok(Message::Text(text))) => text,
                                    Some(Ok(Message::Ping(payload))) => {
                                        if sender.send(Message::Pong(payload)).await.is_err() {
                                            cancel_token.cancel();
                                            break;
                                        }
                                        continue;
                                    }
                                    Some(Ok(Message::Pong(_))) => continue,
                                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                                        cancel_token.cancel();
                                        break;
                                    }
                                    _ => continue,
                                };
                                let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) else {
                                    let err = serde_json::json!({
                                        "type": "error",
                                        "message": "Invalid JSON. Send {\"type\":\"message\",\"content\":\"your text\"}",
                                        "code": "INVALID_JSON"
                                    });
                                    let _ = sender.send(Message::Text(err.to_string().into())).await;
                                    continue;
                                };
                                match parsed["type"].as_str() {
                                    Some("approval_response") => {
                                        // A SOP-kind frame is a gate resolution (keyed by run_id),
                                        // not a tool-prompt response (keyed by request_id). Resolve
                                        // it here too so it is answered mid-turn instead of being
                                        // silently dropped on the request_id path below.
                                        if handle_ws_sop_frame(
                                            &parsed,
                                            state,
                                            session_id,
                                            auth_subject,
                                            &mut *sender,
                                        )
                                        .await
                                        {
                                            continue;
                                        }
                                        let request_id = parsed["request_id"].as_str().unwrap_or("");
                                        let decision = match parsed["decision"].as_str().unwrap_or("") {
                                            "approve" => Some(ChannelApprovalResponse::Approve),
                                            "always" => Some(ChannelApprovalResponse::AlwaysApprove),
                                            "deny" => Some(ChannelApprovalResponse::Deny),
                                            _ => None,
                                        };
                                        if request_id.is_empty() || decision.is_none() {
                                            continue;
                                        }
                                        if let (Some(decision), Some(tx)) = (decision, pending_approvals.lock().remove(request_id)) {
                                            let _ = tx.send(decision);
                                        } else {
                                            ::galleon_log::record!(DEBUG, ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note).with_attrs(::serde_json::json!({"request_id": request_id})), "approval_response with no matching pending request (mid-turn)");
                                        }
                                    }
                                    Some("message") => {
                                        let content = parsed["content"].as_str().unwrap_or("").to_string();
                                        if content.is_empty() {
                                            let err = serde_json::json!({
                                                "type": "error",
                                                "message": "Message content cannot be empty",
                                                "code": "EMPTY_CONTENT"
                                            });
                                            let _ = sender.send(Message::Text(err.to_string().into())).await;
                                            continue;
                                        }
                                        match steering_tx.try_send(content) {
                                            Ok(()) => {}
                                            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                                let err = serde_json::json!({
                                                    "type": "error",
                                                    "message": "Steering queue is full for the running turn",
                                                    "code": "STEERING_QUEUE_FULL"
                                                });
                                                let _ = sender.send(Message::Text(err.to_string().into())).await;
                                            }
                                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                                let err = serde_json::json!({
                                                    "type": "error",
                                                    "message": "Running turn is no longer accepting steering messages",
                                                    "code": "STEERING_CLOSED"
                                                });
                                                let _ = sender.send(Message::Text(err.to_string().into())).await;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            approval = approval_event_rx.recv() => {
                                let Some(event) = approval else { continue };
                                if let TurnEvent::ApprovalRequest {
                                    request_id,
                                    tool_name,
                                    arguments_summary,
                                    timeout_secs,
                                } = event {
                                    let frame = serde_json::json!({
                                        "type": "approval_request",
                                        "request_id": request_id,
                                        "tool": tool_name,
                                        "arguments_summary": arguments_summary,
                                        "timeout_secs": timeout_secs,
                                    });
                                    let _ = sender.send(Message::Text(frame.to_string().into())).await;
                                }
                            }
                            _ = tick_websocket_ping(ping_interval) => {
                                if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                                    cancel_token.cancel();
                                    break;
                                }
                            }
                                event_opt = event_rx.recv() => {
                                let Some(event) = event_opt else { break };
                                let ws_msg = match event {
            usage_event @ TurnEvent::Usage { .. } => {
                                        // The fold below is the production event path under
                                        // test (see UsageFold regression tests): billing
                                        // aggregates every billable attempt while the
                                        // accepted-serving snapshot advances on accepted
                                        // events only.
                                        usage_fold.apply(usage_event);
                                        continue;
                                    }
                                    TurnEvent::Chunk { ref delta } => {
                                        accumulated_text.push_str(delta);
                                        serde_json::json!({ "type": "chunk", "content": delta })
                                    }
                                    TurnEvent::Thinking { delta } => {
                                        serde_json::json!({ "type": "thinking", "content": delta })
                                    }
                                    TurnEvent::ToolCall { id, name, args } => {
                                        serde_json::json!({ "type": "tool_call", "id": id, "name": name, "args": args })
                                    }
                                    TurnEvent::ToolResult {
                                        id, name, output, ..
                                    } => {
                                        serde_json::json!({ "type": "tool_result", "id": id, "name": name, "output": output })
                                    }
                                    TurnEvent::ApprovalRequest {
                                        request_id,
                                        tool_name,
                                        arguments_summary,
                                        timeout_secs,
                                    } => serde_json::json!({
                                        "type": "approval_request",
                                        "request_id": request_id,
                                        "tool": tool_name,
                                        "arguments_summary": arguments_summary,
                                        "timeout_secs": timeout_secs,
                                    }),
                                    TurnEvent::HistoryTrimmed {
                                        dropped_messages,
                                        kept_turns,
                                        reason,
                                        token_budget,
                                        tokens_before,
                                        tokens_after,
                                        tokens_before_source,
                                        tokens_after_source,
                                        unsatisfiable_floor,
                                    } => history_trimmed_ws_frame(
                                        dropped_messages,
                                        kept_turns,
                                        &reason,
                                        token_budget,
                                        tokens_before,
                                        tokens_after,
                                        tokens_before_source.map(|s| s.as_str()),
                                        tokens_after_source.map(|s| s.as_str()),
                                        unsatisfiable_floor,
                                    ),
                                    TurnEvent::Plan { entries } => serde_json::json!({
                                        "type": "plan",
                                        "entries": entries,
                                    }),
                                    _ => continue,
                                };
                                let _ = sender.send(Message::Text(ws_msg.to_string().into())).await;
                            }
                        }
        }
    };

    let (result, ()) = tokio::join!(turn_fut, forward_fut);

    // ── Remove cancel token (turn finished) ──────────────────────
    remove_cancel_token_if_current(
        &state.cancel_tokens,
        &gateway_cancel_key(session_id),
        &cancel_token,
    );

    // Check if this turn was cancelled. `turn_streamed` propagates
    // `ToolLoopCancelled` through anyhow, so we detect it here.
    let was_cancelled = match &result {
        Err(e) => galleon_api::agent::is_tool_loop_cancelled(&e.error),
        Ok(_) => false,
    };

    if was_cancelled {
        if let Some(ref backend) = state.session_backend
            && backend.session_exists(session_key)
        {
            // Persist the agent's authoritative post-turn history as one state,
            // even when the turn produced noDelta or was hard-cancelled. The
            // live agent may have already trimmed older turns before the
            // cancellation was observed; persisting only a delta marker would
            // leave the durable store with the pre-trim transcript that the
            // next restore would resurrect.
            let needs_marker = match &result {
                Err(error) => !has_assistant_chat_message(&error.new_messages),
                Ok(_) => false,
            };
            let durable = if needs_marker {
                let marker = galleon_i18n::get_required_cli_string("turn-interrupted-by-user");
                let truncated = if accumulated_text.is_empty() {
                    marker
                } else {
                    format!("{accumulated_text}\n\n{marker}")
                };
                let mut d = galleon_providers::durable_chat_messages(agent.history());
                d.push(galleon_providers::ChatMessage::assistant(&truncated));
                d
            } else {
                galleon_providers::durable_chat_messages(agent.history())
            };
            let crumb = agent.history_has_trim_breadcrumb();
            replace_conversation_state_unless_deleted(
                backend.as_ref(),
                session_key,
                &durable,
                crumb,
            );
        }

        // Inform the client the turn was aborted
        let aborted = serde_json::json!({ "type": "aborted" });
        let _ = sender.send(Message::Text(aborted.to_string().into())).await;

        if let Some(ref backend) = state.session_backend
            && backend.session_exists(session_key)
        {
            let _ = backend.set_session_state(session_key, "idle", None);
        }

        // Broadcast agent_end event
        let cancel_model = usage_fold.last_model.as_deref().unwrap_or(&turn_model);
        let cancel_provider_ref = usage_fold
            .last_provider_ref
            .as_deref()
            .unwrap_or(&provider_label);
        let _ = state.event_tx.send(serde_json::json!({
            "type": "agent_end",
            "model_provider": &provider_label,
            "model": cancel_model,
            "provider_ref": cancel_provider_ref,
        }));

        // Trace the cancelled turn so the doctor / replay tool sees it
        // alongside successful turns.follow-through.
        ::galleon_log::record!(
            INFO,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Cancel)
                .with_outcome(::galleon_log::EventOutcome::Failure)
                .with_attrs(::serde_json::json!({
                    "model_provider": &provider_label,
                    "model": cancel_model,
                    "provider_ref": cancel_provider_ref,
                    "session_key": session_key,
                    "reason": "interrupted by user",
                    "cancelled": true,
                    "trace_id": turn_id,
                })),
            "gateway_ws_turn"
        );

        return;
    }

    match result {
        Ok(outcome) => {
            if let Some(ref backend) = state.session_backend {
                persist_agent_conversation_state(backend.as_ref(), session_key, agent);
            }

            // Fire-and-forget memory consolidation so facts from WS sessions
            // are extracted to long-term memory (Daily + Core categories).
            if state.auto_save {
                if let Some(mem) = ws_memory.clone() {
                    let model_provider = state.model_provider.clone();
                    let model = state.model.clone();
                    let temperature = state.temperature;
                    let memory_config = state.config.read().memory.clone();
                    let user_msg = content.to_string();
                    let assistant_resp = outcome.response.clone();
                    galleon_spawn::spawn!(async move {
                        if let Err(e) = galleon_memory::consolidation::consolidate_turn(
                            model_provider.as_ref(),
                            &model,
                            temperature,
                            mem.as_ref(),
                            &memory_config,
                            &user_msg,
                            &assistant_resp,
                        )
                        .await
                        {
                            ::galleon_log::record!(
                                DEBUG,
                                ::galleon_log::Event::new(
                                    module_path!(),
                                    ::galleon_log::Action::Note
                                )
                                .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                                "WS memory consolidation skipped"
                            );
                        }
                    });
                } else {
                    ::galleon_log::record!(
                        DEBUG,
                        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
                        "WS memory consolidation skipped"
                    );
                }
            }

            let total_tokens = match (
                usage_fold.total_input_tokens,
                usage_fold.total_output_tokens,
            ) {
                (Some(i), Some(o)) => Some(i.saturating_add(o)),
                (Some(i), None) => Some(i),
                (None, Some(o)) => Some(o),
                (None, None) => None,
            };
            // Deterministic ordering: sort by (provider_ref, model) so the wire
            // format is stable (avoids flaky assertions in tests). cost_usd is
            // summed over this sorted vector — never over the HashMap directly —
            // so float accumulation order (and the emitted total) is stable.
            let usage_by_provider_vec = usage_fold.take_sorted_entries();
            // cost_usd is the sum of all billable attempts' cost_usd from
            // usage_by_provider (which now includes rejected attempts). This makes
            // the breakdown the single source of truth.
            let cost_usd = UsageFold::total_cost_usd(&usage_by_provider_vec);

            let active_provider = outcome.provider_name.clone();
            let active_model = outcome.model.clone();
            // The route that actually served the FINAL call is authoritative for
            // the terminal frame (see `done_frame_context_limits`). Resolve from
            // the final route only when no call was served (e.g. a cache hit).
            let fallback_limits = outcome.final_context_limits.is_none().then(|| {
                let (budget, window) =
                    agent.context_limits_for_route_raw(&active_provider, &active_model);
                galleon_config::schema::ResolvedContextLimits {
                    model_context_window: window.unwrap_or(
                        galleon_config::schema::UNCONFIGURED_CONTEXT_WINDOW_FALLBACK as u64,
                    ) as usize,
                    context_token_budget: budget.unwrap_or(0) as usize,
                    model_context_window_source: if window.is_some() {
                        galleon_config::schema::ModelContextWindowSource::Configured
                    } else {
                        galleon_config::schema::ModelContextWindowSource::CompatibilityFallback
                    },
                }
            });
            let final_limits = outcome.final_context_limits.map(|l| {
                galleon_config::schema::ResolvedContextLimits {
                    model_context_window: l.model_context_window,
                    context_token_budget: l.context_token_budget,
                    model_context_window_source: if l.is_compatibility_fallback {
                        galleon_config::schema::ModelContextWindowSource::CompatibilityFallback
                    } else {
                        galleon_config::schema::ModelContextWindowSource::Configured
                    },
                }
            });
            let (max_context_tokens, model_context_window) = done_frame_context_limits(
                final_limits,
                usage_fold.last_context_token_budget,
                usage_fold.last_model_context_window,
                fallback_limits,
            );
            let effective_model = usage_fold.last_model.as_deref().unwrap_or(&active_model);
            let provider_ref_full = usage_fold
                .last_provider_ref
                .as_deref()
                .unwrap_or(&active_provider);
            let meta = DoneFrameMeta {
                full_response: &outcome.response,
                input_tokens: usage_fold.total_input_tokens,
                output_tokens: usage_fold.total_output_tokens,
                tokens_used: total_tokens,
                cost_usd,
                model: effective_model,
                provider: &provider_label,
                provider_ref: provider_ref_full,
                max_context_tokens,
                model_context_window,
                last_input_tokens: usage_fold.last_input_tokens,
                last_serving_provider_ref: usage_fold.last_provider_ref.as_deref(),
                last_serving_model: usage_fold.last_model.as_deref(),
            };
            // Surface an at-most-one safety-safeguard downgrade notice just
            // before the terminal `done` frame, so the web chat shows the
            // silent model switch the same way the messaging channels append a
            // footer. Display-only: emitted as its own frame, never added to
            // `outcome.new_messages`, so it is not persisted into the session
            // transcript. Privacy: only the model names cross the wire — the
            // classifier `category` (and any refusal explanation) never do.
            let safeguard_notice = outcome.safeguard_fallback.as_ref().map(|s| {
                galleon_providers::SafeguardFallbackNotice {
                    kind: match s.kind {
                        galleon_api::agent::AgentSafeguardKind::ServerSide => {
                            galleon_providers::SafeguardFallbackKind::ServerSide
                        }
                        galleon_api::agent::AgentSafeguardKind::ClientSide => {
                            galleon_providers::SafeguardFallbackKind::ClientSide
                        }
                        galleon_api::agent::AgentSafeguardKind::ClientAndServer => {
                            galleon_providers::SafeguardFallbackKind::ClientAndServer
                        }
                    },
                    requested_model: s.requested_model.clone(),
                    served_model: s.served_model.clone(),
                    category: None,
                }
            });
            let done = build_done_frame_json(&meta, &usage_by_provider_vec);
            for frame in success_terminal_ws_frames(safeguard_notice.as_ref(), done) {
                let _ = sender.send(Message::Text(frame.to_string().into())).await;
            }

            // Set session state to idle
            if let Some(ref backend) = state.session_backend {
                let _ = backend.set_session_state(session_key, "idle", None);
            }

            // Broadcast agent_end event
            let _ = state.event_tx.send(serde_json::json!({
                "type": "agent_end",
                "model_provider": &provider_label,
                "model": effective_model,
                "provider_ref": provider_ref_full,
            }));

            // Append a runtime-trace.jsonl record so a `clawcrew doctor`
            // sweep sees gateway WS turns alongside channel and CLI turns.
            // Closes the gateway-side trace gap from
            ::galleon_log::record!(
                INFO,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Complete)
                    .with_outcome(::galleon_log::EventOutcome::Success)
                    .with_attrs(::serde_json::json!({
                        "model_provider": &provider_label,
                        "model": effective_model,
                        "provider_ref": provider_ref_full,
                        "session_key": session_key,
                        "input_tokens": usage_fold.total_input_tokens,
                        "output_tokens": usage_fold.total_output_tokens,
                        "tokens_used": total_tokens,
                        "cost_usd": cost_usd,
                        "last_input_tokens": usage_fold.last_input_tokens,
                        "trace_id": turn_id,
                    })),
                "gateway_ws_turn"
            );
        }
        Err(e) => {
            if let Some(ref backend) = state.session_backend {
                persist_agent_conversation_state(backend.as_ref(), session_key, agent);
            }

            // Set session state to error
            if let Some(ref backend) = state.session_backend {
                let _ = backend.set_session_state(session_key, "error", Some(&turn_id));
            }

            ::galleon_log::record!(
                ERROR,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
                    .with_outcome(::galleon_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({"error": format!("{}", e.error)})),
                "Agent turn failed"
            );
            let user_message =
                galleon_providers::terminal_error::terminal_completion_error_message_with_renderer(
                    &e.error,
                    None,
                    render_cli_string,
                );
            let err = send_ws_turn_failure(sender, &e.error, user_message.as_deref()).await;

            // Broadcast error event
            let _ = state.event_tx.send(serde_json::json!({
                "type": "error",
                "component": "ws_chat",
                "message": err["message"],
            }));

            // Trace the failed turn so the doctor / replay tool sees the
            // failure mode and the turn_id can be cross-referenced with
            // costs.jsonl.follow-through.
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
                    .with_outcome(::galleon_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({
                        "model_provider": provider_label,
                        "model": turn_model,
                        "session_key": session_key,
                        "error": galleon_providers::sanitize_api_error(&e.error.to_string()),
                        "error_code": err["code"],
                        "trace_id": turn_id,
                    })),
                "gateway_ws_turn"
            );
        }
    }
}

pub(crate) fn render_cli_string(key: &str, args: &[(&str, &str)]) -> String {
    if args.is_empty() {
        galleon_i18n::get_required_cli_string(key)
    } else {
        galleon_i18n::get_required_cli_string_with_args(key, args)
    }
}

/// Serialize a failed turn for the WebSocket boundary without letting a
/// localized user message alter the stable diagnostic used for classification.
pub(crate) fn ws_turn_failure_frame(
    diagnostic: &str,
    user_message: Option<&str>,
    is_terminal_provider_failure: bool,
) -> serde_json::Value {
    let sanitized = galleon_providers::sanitize_api_error(diagnostic);
    let error_code = if is_terminal_provider_failure {
        "PROVIDER_ERROR"
    } else if sanitized.to_lowercase().contains("api key")
        || sanitized.to_lowercase().contains("authentication")
        || sanitized.to_lowercase().contains("unauthorized")
    {
        "AUTH_ERROR"
    } else if sanitized.to_lowercase().contains("model_provider")
        || sanitized.to_lowercase().contains("model")
    {
        "PROVIDER_ERROR"
    } else {
        "AGENT_ERROR"
    };
    serde_json::json!({
        "type": "error",
        "message": user_message.unwrap_or(&sanitized),
        "code": error_code,
    })
}

pub(crate) async fn send_ws_turn_failure<S>(
    sender: &mut S,
    error: &anyhow::Error,
    user_message: Option<&str>,
) -> serde_json::Value
where
    S: futures_util::Sink<Message> + Unpin,
{
    let frame = ws_turn_failure_frame(&error.to_string(), user_message, user_message.is_some());
    let _ = sender.send(Message::Text(frame.to_string().into())).await;
    frame
}
