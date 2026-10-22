//! WebSocket session lifecycle, connection loops, and heartbeat management.

use super::*;
use crate::GW_SESSION_PREFIX;
use crate::ws_approval::{PendingApprovals, WsApprovalChannel, new_pending_approvals};
use axum::extract::ws::{Message, WebSocket};
use clawcrew_api::channel::ChannelApprovalResponse;
use futures_util::{SinkExt, StreamExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn websocket_ping_interval(
    config: &clawcrew_config::schema::Config,
) -> Option<tokio::time::Interval> {
    let seconds = config.gateway.websocket_ping_interval_secs;
    if seconds == 0 {
        return None;
    }

    let period = Duration::from_secs(seconds);
    let start = tokio::time::Instant::now().checked_add(period)?;
    let mut interval = tokio::time::interval_at(start, period);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    Some(interval)
}

pub(crate) async fn tick_websocket_ping(interval: &mut Option<tokio::time::Interval>) {
    if let Some(interval) = interval.as_mut() {
        interval.tick().await;
    } else {
        std::future::pending::<()>().await;
    }
}

pub(crate) async fn resolve_ws_memory_handle(
    config: &clawcrew_config::schema::Config,
    agent_alias: &str,
) -> anyhow::Result<Option<Arc<dyn clawcrew_memory::Memory>>> {
    if config.agent(agent_alias).is_some_and(|agent| {
        matches!(
            agent.memory.backend,
            clawcrew_config::multi_agent::MemoryBackendKind::None
        )
    }) {
        return Ok(None);
    }

    let api_key = config
        .resolved_model_provider_for_agent(agent_alias)
        .and_then(|(_, _, cfg)| cfg.api_key.clone());
    clawcrew_memory::create_memory_for_agent(config, agent_alias, api_key.as_deref())
        .await
        .map(Some)
}

pub(crate) async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    agent_alias: String,
    session_id: Option<String>,
    session_name: Option<String>,
    session_cwd: Option<String>,
    // The transport-authenticated approval subject (paired-token hash), if the
    // connection was authenticated. Threaded to SOP approval frames so a policied
    // gate can be satisfied by an identified WS caller.
    auth_subject: Option<String>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Resolve session ID: use provided or generate a new UUID
    let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    // Transcript identity keeps the raw display id (`gw_{session_id}`) so
    // existing persisted histories and metadata rows stay resumable across
    // reconnects.
    let session_key = format!("{GW_SESSION_PREFIX}{session_id}");
    // Match the sanitized form persisted by memory backend migrations.
    let mut memory_session_id = clawcrew_api::session_keys::sanitize_session_key(&session_id);

    // Hydrate session metadata from persistence (if available). Agent
    // construction is deferred until after the optional `connect` frame so the
    // client can provide a per-session cwd for the security sandbox root.
    let config = state.config.read().clone();
    let ws_memory = match resolve_ws_memory_handle(&config, &agent_alias).await {
        Ok(memory) => memory,
        Err(e) => {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({
                        "agent": &agent_alias,
                        "error": format!("{e:#}"),
                    })),
                "WS per-agent memory resolution failed; consolidation disabled for connection"
            );
            None
        }
    };
    let mut resumed = false;
    let mut message_count: usize = 0;
    let mut effective_name: Option<String> = None;
    let mut stored_messages = Vec::new();
    if let Some(ref backend) = state.session_backend {
        // Fail closed: an unreadable transcript must not become an empty
        // history that a later turn authoritatively persists over the
        // existing durable session. Terminate this connection instead.
        let messages = match backend.try_load(&session_key) {
            Ok(messages) => messages,
            Err(e) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({
                            "session_key": session_key,
                            "error": format!("{}", e),
                        })),
                    "Failed to load WS session transcript; refusing to open with unverified history"
                );
                let err = serde_json::json!({
                    "type": "error",
                    "message": "session restore unavailable; retry the connection",
                    "code": "SESSION_RESTORE_UNAVAILABLE"
                });
                let _ = sender.send(Message::Text(err.to_string().into())).await;
                return;
            }
        };
        if !messages.is_empty() {
            message_count = messages.len();
            stored_messages = messages;
            resumed = true;
        }
        // Set session name if provided (non-empty) on connect
        if let Some(ref name) = session_name
            && !name.is_empty()
        {
            let _ = backend.set_session_name(&session_key, name);
            effective_name = Some(name.clone());
        }
        // If no name was provided via query param, load the stored name
        if effective_name.is_none() {
            effective_name = backend.get_session_name(&session_key).unwrap_or(None);
        }
        // Stamp the agent alias so future /api/sessions queries and
        // per-agent filters can attribute this session to its agent.
        let _ = backend.set_session_agent_alias(&session_key, &agent_alias);
    }

    // Send session_start message to client
    let mut session_start = serde_json::json!({
        "type": "session_start",
        "session_id": session_id,
        "resumed": resumed,
        "message_count": message_count,
    });
    if let Some(ref name) = effective_name {
        session_start["name"] = serde_json::Value::String(name.clone());
    }
    let _ = sender
        .send(Message::Text(session_start.to_string().into()))
        .await;

    let mut first_msg_fallback: Option<String> = None;
    let mut requested_cwd = session_cwd;
    let mut ping_interval = websocket_ping_interval(&config);

    loop {
        let first = tokio::select! {
            first = receiver.next() => first,
            _ = tick_websocket_ping(&mut ping_interval) => {
                if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                    return;
                }
                continue;
            }
        };

        match first {
            Some(Ok(Message::Text(text))) => {
                if let Ok(cp) = serde_json::from_str::<ConnectParams>(&text) {
                    if cp.msg_type == "connect" {
                        ::clawcrew_log::record!(DEBUG, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"session_id": cp.session_id, "device_name": cp.device_name, "capabilities": cp.capabilities, "cwd": cp.cwd})), "WebSocket connect params received");
                        if let Some(sid) = &cp.session_id {
                            memory_session_id =
                                clawcrew_api::session_keys::sanitize_session_key(sid);
                            ::clawcrew_log::record!(
                                DEBUG,
                                ::clawcrew_log::Event::new(
                                    module_path!(),
                                    ::clawcrew_log::Action::Note
                                )
                                .with_attrs(::serde_json::json!({"session_id": sid})),
                                "WebSocket connect session override received"
                            );
                        }
                        if cp.cwd.is_some() {
                            requested_cwd = cp.cwd;
                        }
                        let ack = serde_json::json!({
                            "type": "connected",
                            "message": "Connection established"
                        });
                        let _ = sender.send(Message::Text(ack.to_string().into())).await;
                    } else {
                        // Not a connect message — fall through to normal processing
                        first_msg_fallback = Some(text.to_string());
                    }
                } else {
                    // Not parseable as ConnectParams — fall through
                    first_msg_fallback = Some(text.to_string());
                }
                break;
            }
            Some(Ok(Message::Ping(payload))) => {
                if sender.send(Message::Pong(payload)).await.is_err() {
                    return;
                }
            }
            Some(Ok(Message::Pong(_))) => {}
            Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
            Some(Ok(_)) => {}
        }
    }

    let session_cwd = match resolve_ws_session_cwd(requested_cwd.as_deref(), &config, &agent_alias)
    {
        Ok(cwd) => cwd,
        Err(e) => {
            let err = serde_json::json!({
                "type": "error",
                "message": e.to_string(),
                "code": "INVALID_CWD"
            });
            let _ = sender.send(Message::Text(err.to_string().into())).await;
            return;
        }
    };

    if let Some(err) = needs_onboarding_ws_error(&config) {
        let _ = sender.send(Message::Text(err.to_string().into())).await;
        return;
    }

    let mut agent = match clawcrew_api::agent::create_agent_session(
        Arc::clone(&state.config),
        &agent_alias,
        Some(&session_cwd),
        Some(state.canvas_store.clone()),
    )
    .await
    {
        Ok(a) => a,
        Err(e) => {
            ::clawcrew_log::record!(
                ERROR,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Fail)
                    .with_outcome(::clawcrew_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                "Agent initialization failed"
            );
            let err = serde_json::json!({
                "type": "error",
                "message": format!("Failed to initialise agent: {e}"),
                "code": "AGENT_INIT_FAILED"
            });
            let _ = sender.send(Message::Text(err.to_string().into())).await;
            let _ = sender
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 1011,
                    reason: axum::extract::ws::Utf8Bytes::from_static(
                        "Agent initialization failed",
                    ),
                })))
                .await;
            return;
        }
    };
    // Keep ONE ingress identity for the WebSocket turn: the turn span records
    // `channel = "wss"`, and observer events derive from `Agent.channel_name`,
    // so this must stay `wss` or a single turn is split across two names.
    // The back-channel is registered under the same `wss` key below, which is
    // what lets ask_user/poll/escalate_to_human default to this conversation.
    agent.set_channel_name(WS_CHANNEL_KEY.to_string());
    agent.set_memory_session_id(Some(memory_session_id));
    let restore_trim_event = if stored_messages.is_empty() {
        None
    } else {
        // Breadcrumb provenance is the backend's own canonical record
        // alongside the transcript, never inferred from message text: a
        // genuine first user turn that happens to equal the localized
        // breadcrumb string must keep its turn-boundary role, and a crumb
        // persisted under another locale must stay classified as synthetic.
        // Sessions from before this was tracked (`None`) restore as `false`,
        // matching the pre-existing fallback for backends that don't
        // support it. Ownership must be set BEFORE seeding: seeding trims
        // immediately if the restored transcript is over the structured cap,
        // and that seed-time trim reads the agent's current breadcrumb flag
        // to decide whether a leading synthetic marker counts as a real
        // turn. Setting it after would let that first trim mistreat it.
        let crumb_res = state
            .session_backend
            .as_ref()
            .map(|backend| backend.get_session_trim_breadcrumb(&session_key));
        match crumb_res {
            Some(Err(e)) => {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({
                            "session_key": session_key,
                            "error": format!("{}", e),
                        })),
                    "Failed to read trim breadcrumb provenance for WS restore; refusing to open with unverified history"
                );
                let err = serde_json::json!({
                    "type": "error",
                    "message": "session restore unavailable; retry the connection",
                    "code": "SESSION_RESTORE_UNAVAILABLE"
                });
                let _ = sender.send(Message::Text(err.to_string().into())).await;
                return;
            }
            Some(Ok(opt)) => {
                agent.set_history_has_trim_breadcrumb(opt.unwrap_or(false));
                agent.seed_history_with_event(&stored_messages)
            }
            None => {
                agent.set_history_has_trim_breadcrumb(false);
                agent.seed_history_with_event(&stored_messages)
            }
        }
    };

    // Seed-time trim only fires when the restored history exceeded the
    // structured cap, so it dropped rows, not just relabeled them. Mirror the
    // ACP restore contract (`replace_transcript` in `handle_session_load` /
    // `handle_session_resume`): persist the retained projection and corrected
    // breadcrumb before the session goes live, or a reconnect/restart before
    // the next prompt reloads the untrimmed durable prefix and repeats the
    // trim, leaving the live agent and the durable session disagreeing.
    if restore_trim_event.is_some()
        && let Some(ref backend) = state.session_backend
        && backend.session_exists(&session_key)
        && !persist_agent_conversation_state(backend.as_ref(), &session_key, &agent)
    {
        let err = serde_json::json!({
            "type": "error",
            "message": "session restore unavailable; retry the connection",
            "code": "SESSION_RESTORE_UNAVAILABLE"
        });
        let _ = sender.send(Message::Text(err.to_string().into())).await;
        return;
    }

    let (approval_event_tx, mut approval_event_rx) =
        tokio::sync::mpsc::channel::<clawcrew_api::agent::TurnEvent>(8);
    let pending_approvals: PendingApprovals = new_pending_approvals();
    let approval_channel = Arc::new(WsApprovalChannel::new(
        approval_event_tx.clone(),
        pending_approvals.clone(),
        Duration::from_secs(WS_APPROVAL_TIMEOUT_SECS),
    ));
    agent.register_channel(WS_CHANNEL_KEY, approval_channel.clone());

    let channel_names = if let Some(ch) = agent
        .channel_handles_any()
        .downcast_ref::<clawcrew_tools::send_via::AgentChannelHandles>(
    ) {
        clawcrew_channels::orchestrator::register_channels_for_tools(
            &config,
            &ch.ask_user,
            &ch.channel_room,
            &Some(ch.reaction.clone()),
            &ch.poll,
            &ch.escalate,
        )
    } else {
        Vec::new()
    };
    if !channel_names.is_empty() {
        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
                ::serde_json::json!({"channels": channel_names, "session": session_key})
            ),
            "Seeded {} channel(s) into dashboard agent session",
        );
    }

    // Seeding happens before the connection's agent setup is complete. Forward
    // its one-shot trim outcome only after channels are registered, so restore
    // notifications cannot race setup or be emitted twice.
    if let Some(clawcrew_api::agent::TurnEvent::HistoryTrimmed {
        dropped_messages,
        kept_turns,
        reason,
        token_budget,
        tokens_before,
        tokens_after,
        tokens_before_source,
        tokens_after_source,
        unsatisfiable_floor,
    }) = restore_trim_event
    {
        let frame = history_trimmed_ws_frame(
            dropped_messages,
            kept_turns,
            &reason,
            token_budget,
            tokens_before,
            tokens_after,
            tokens_before_source.map(|s| s.as_str()),
            tokens_after_source.map(|s| s.as_str()),
            unsatisfiable_floor,
        );
        let _ = sender.send(Message::Text(frame.to_string().into())).await;
    }

    // Process the first message if it was not a connect frame
    if let Some(ref text) = first_msg_fallback {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
            if parsed["type"].as_str() == Some("message") {
                if let Some(content) = first_chat_message_content(text) {
                    let _session_guard = match state.session_queue.acquire(&session_key).await {
                        Ok(guard) => guard,
                        Err(e) => {
                            let err = serde_json::json!({
                                "type": "error",
                                "message": e.to_string(),
                                "code": session_queue_ws_error_code(&e)
                            });
                            let _ = sender.send(Message::Text(err.to_string().into())).await;
                            return;
                        }
                    };
                    process_chat_message(
                        &state,
                        &mut agent,
                        &mut sender,
                        &mut receiver,
                        &mut approval_event_rx,
                        &pending_approvals,
                        &mut ping_interval,
                        &ws_memory,
                        &content,
                        &session_key,
                        &session_id,
                        auth_subject.as_deref(),
                    )
                    .await;
                }
            } else {
                let unknown_type = parsed["type"].as_str().unwrap_or("unknown");
                let err = serde_json::json!({
                    "type": "error",
                    "message": format!(
                        "Unsupported message type \"{unknown_type}\". Send {{\"type\":\"message\",\"content\":\"your text\"}}"
                    )
                });
                let _ = sender.send(Message::Text(err.to_string().into())).await;
            }
        } else {
            let err = serde_json::json!({
                "type": "error",
                "message": "Invalid JSON. Send {\"type\":\"message\",\"content\":\"your text\"}"
            });
            let _ = sender.send(Message::Text(err.to_string().into())).await;
        }
    }

    // Subscribe to the shared broadcast channel so cron/heartbeat events
    // are forwarded to this WebSocket client.
    let mut broadcast_rx = state.event_tx.subscribe();

    loop {
        tokio::select! {
            // ── Keepalive ─────────────────────────────────────────────
            _ = tick_websocket_ping(&mut ping_interval) => {
                if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }

            // ── Client message ────────────────────────────────────────
            client_msg = receiver.next() => {
                let Some(msg) = client_msg else { break };
                let msg = match msg {
                    Ok(Message::Text(text)) => text,
                    Ok(Message::Ping(payload)) => {
                        if sender.send(Message::Pong(payload)).await.is_err() { break; }
                        continue;
                    }
                    Ok(Message::Pong(_)) => continue,
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => continue,
                };

                // Parse incoming message
                let parsed: serde_json::Value = match serde_json::from_str(&msg) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = serde_json::json!({
                            "type": "error",
                            "message": format!("Invalid JSON: {}", e),
                            "code": "INVALID_JSON"
                        });
                        let _ = sender.send(Message::Text(err.to_string().into())).await;
                        continue;
                    }
                };

                let msg_type = parsed["type"].as_str().unwrap_or("");

                // ── Voice duplex event dispatch (gated by feature flag + runtime config) ──
                #[cfg(feature = "gateway-voice-duplex")]
                {
                    // Multi-instance shape: presence in the map = enabled.
                    let duplex_enabled = !state.config.read().channels.voice_duplex.is_empty();
                    if duplex_enabled {
                        if let Some(voice_event) = crate::voice_duplex::try_parse_voice_event(&msg) {
                            if let Some(error_frame) = crate::voice_duplex::handle_voice_event(voice_event) {
                                let _ = sender.send(Message::Text(error_frame.to_string().into())).await;
                            }
                            continue;
                        }
                    }
                }

                // ── approval_response (operator answered a tool prompt) ──
                if msg_type == "approval_response" {
                    // EPIC C: a SOP-kind frame resolves a SOP gate via the shared
                    // engine + resolve_gate (keyed by run_id), NOT the tool-prompt
                    // pending_approvals map (keyed by request_id). The principal is
                    // transport-derived (ws + session id), never from the frame.
                    if handle_ws_sop_frame(
                        &parsed,
                        &state,
                        &session_id,
                        auth_subject.as_deref(),
                        &mut sender,
                    )
                    .await
                    {
                        continue;
                    }
                    let request_id = parsed["request_id"].as_str().unwrap_or("");
                    let decision_str = parsed["decision"].as_str().unwrap_or("");
                    let decision = match decision_str {
                        "approve" => Some(ChannelApprovalResponse::Approve),
                        "always" => Some(ChannelApprovalResponse::AlwaysApprove),
                        "deny" => Some(ChannelApprovalResponse::Deny),
                        _ => None,
                    };
                    if request_id.is_empty() || decision.is_none() {
                        let err = serde_json::json!({
                            "type": "error",
                            "message": "approval_response requires request_id and decision in {approve,deny,always}",
                            "code": "INVALID_APPROVAL_RESPONSE"
                        });
                        let _ = sender.send(Message::Text(err.to_string().into())).await;
                        continue;
                    }
                    if let (Some(decision), Some(tx)) = (decision, pending_approvals.lock().remove(request_id)) {
                        let _ = tx.send(decision);
                    } else {
                        ::clawcrew_log::record!(DEBUG, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"request_id": request_id})), "approval_response with no matching pending request");
                    }
                    continue;
                }

                if msg_type != "message" {
                    let err = serde_json::json!({
                        "type": "error",
                        "message": format!(
                            "Unsupported message type \"{msg_type}\". Send {{\"type\":\"message\",\"content\":\"your text\"}}"
                        ),
                        "code": "UNKNOWN_MESSAGE_TYPE"
                    });
                    let _ = sender.send(Message::Text(err.to_string().into())).await;
                    continue;
                }

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

                // Acquire session lock to serialize concurrent turns
                let _session_guard = match state.session_queue.acquire(&session_key).await {
                    Ok(guard) => guard,
                    Err(e) => {
                        let err = serde_json::json!({
                            "type": "error",
                            "message": e.to_string(),
                            "code": session_queue_ws_error_code(&e)
                        });
                        let _ = sender.send(Message::Text(err.to_string().into())).await;
                        continue;
                    }
                };

                process_chat_message(
                    &state,
                    &mut agent,
                    &mut sender,
                    &mut receiver,
                    &mut approval_event_rx,
                    &pending_approvals,
                    &mut ping_interval,
                    &ws_memory,
                    &content,
                    &session_key,
                    &session_id,
                        auth_subject.as_deref(),
                )
                .await;
            }

            // ── Broadcast event (cron/heartbeat results) ──────────────
            event = broadcast_rx.recv() => {
                if let Ok(event) = event
                    && event_matches_session(&event, &session_id)
                    && !is_observability_telemetry(&event)
                {
                    let _ = sender.send(Message::Text(event.to_string().into())).await;
                }
            }

            approval_event = approval_event_rx.recv() => {
                let Some(event) = approval_event else { break };
                // Forward the runtime-produced summary without inspecting or
                // reconstructing it from the raw argument object.
                let frame = match event {
                    clawcrew_api::agent::TurnEvent::ApprovalRequest {
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
                    other => {
                        ::clawcrew_log::record!(WARN, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_outcome(::clawcrew_log::EventOutcome::Unknown).with_attrs(::serde_json::json!({"kind": format!("{:?}", other)})), "non-ApprovalRequest event leaked into approval channel");
                        continue;
                    }
                };
                let _ = sender.send(Message::Text(frame.to_string().into())).await;
            }
        }
    }
}

pub(crate) fn resolve_session_cwd(
    requested_cwd: Option<&str>,
    default_workspace: &Path,
) -> anyhow::Result<PathBuf> {
    let cwd = requested_cwd
        .map(PathBuf::from)
        .unwrap_or_else(|| default_workspace.to_path_buf());
    std::fs::canonicalize(&cwd).map_err(|e| {
        ::clawcrew_log::record!(
            WARN,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                .with_outcome(::clawcrew_log::EventOutcome::Failure)
                .with_attrs(::serde_json::json!({
                    "cwd": cwd.display().to_string(),
                    "error": format!("{}", e),
                })),
            "ws session cwd rejected"
        );
        anyhow::Error::msg(format!(
            "cwd is not a usable directory ({}): {e}",
            cwd.display()
        ))
    })
}

pub(crate) fn resolve_ws_session_cwd(
    requested_cwd: Option<&str>,
    config: &clawcrew_config::schema::Config,
    agent_alias: &str,
) -> anyhow::Result<PathBuf> {
    let agent_workspace = config.agent_workspace_dir(agent_alias);
    if requested_cwd.is_none() {
        std::fs::create_dir_all(&agent_workspace).map_err(|e| {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Reject)
                    .with_outcome(::clawcrew_log::EventOutcome::Failure)
                    .with_attrs(::serde_json::json!({
                        "agent": agent_alias,
                        "cwd": agent_workspace.display().to_string(),
                        "error": format!("{}", e),
                    })),
                "ws agent workspace cwd rejected"
            );
            anyhow::Error::msg(format!(
                "cwd is not a usable directory ({}): {e}",
                agent_workspace.display()
            ))
        })?;
    }
    resolve_session_cwd(requested_cwd, &agent_workspace)
}

pub(crate) fn session_queue_ws_error_code(
    error: &crate::session_queue::SessionQueueError,
) -> &'static str {
    match error {
        crate::session_queue::SessionQueueError::QueueFull { .. } => "SESSION_QUEUE_FULL",
        crate::session_queue::SessionQueueError::Timeout { .. } => "SESSION_QUEUE_TIMEOUT",
    }
}

/// Replace the session's durable transcript and breadcrumb flag with
/// `durable`/`breadcrumb_present`, unless the session was deleted between the
/// turn starting and this post-turn persistence — in which case the
/// `aborted` / `done` / `error` frames are still sent to the client, but the
/// row `DELETE /api/sessions/{id}` just wiped is not re-created.
///
/// The existence probe and the replacement run inside the backend's guarded
/// `replace_conversation_state_if_exists`, not as two separate operations: a
/// bare `session_exists` check followed by a replace is a check-then-act
/// race, and a delete committing between the two would be silently undone by
/// the replace recreating the session row/files.
///
/// Returns `false` when the durable write itself failed (logged with session
/// context), so the caller does not report the turn as durably persisted
/// when the store disagrees. A skipped write (session already gone) returns
/// `true`: there is nothing left to persist.
pub(crate) fn replace_conversation_state_unless_deleted(
    backend: &dyn clawcrew_infra::session_backend::SessionBackend,
    session_key: &str,
    durable: &[clawcrew_providers::ChatMessage],
    breadcrumb_present: bool,
) -> bool {
    match backend.replace_conversation_state_if_exists(session_key, durable, breadcrumb_present) {
        Ok(_) => true,
        Err(e) => {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({
                        "session_key": session_key,
                        "error": format!("{}", e),
                    })),
                "Failed to persist authoritative post-turn conversation state"
            );
            false
        }
    }
}

/// Replace the session's durable transcript and breadcrumb flag with the
/// agent's own authoritative post-turn history, as one state. Appending only
/// the turn's delta on top of a transcript the agent's loop already trimmed
/// underneath it can resurrect turns the live agent dropped; replacing with
/// `agent.history()` keeps the store in sync with what the agent actually
/// retains.
pub(crate) fn persist_agent_conversation_state(
    backend: &dyn clawcrew_infra::session_backend::SessionBackend,
    session_key: &str,
    agent: &(dyn clawcrew_api::agent::AgentSession + '_),
) -> bool {
    let durable = clawcrew_providers::durable_chat_messages(agent.history());
    replace_conversation_state_unless_deleted(
        backend,
        session_key,
        &durable,
        agent.history_has_trim_breadcrumb(),
    )
}

pub(crate) fn has_assistant_chat_message(
    messages: &[clawcrew_providers::ConversationMessage],
) -> bool {
    messages.iter().any(|message| {
        matches!(
            message,
            clawcrew_providers::ConversationMessage::Chat(message)
                if message.role == "assistant"
        )
    })
}

pub(crate) fn event_matches_session(event: &serde_json::Value, session_id: &str) -> bool {
    match event.get("session_id").and_then(|value| value.as_str()) {
        Some(event_session_id) => event_session_id == session_id,
        None => is_global_chat_event(event),
    }
}

pub(crate) fn is_global_chat_event(event: &serde_json::Value) -> bool {
    matches!(
        event.get("type").and_then(serde_json::Value::as_str),
        Some("cron_result")
    )
}

pub(crate) fn is_observability_telemetry(event: &serde_json::Value) -> bool {
    event.get("source").and_then(serde_json::Value::as_str) == Some("observability")
}
