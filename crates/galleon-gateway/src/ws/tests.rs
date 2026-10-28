use super::*;
use crate::AppState;
use crate::GW_SESSION_PREFIX;
use crate::ws_approval::{WsApprovalChannel, new_pending_approvals};
use axum::{
    Json, Router,
    extract::ws::Message,
    http::{HeaderMap, header},
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message as ClientMessage};

#[test]
fn ws_terminal_failure_uses_localized_message_without_reclassifying_diagnostic() {
    let diagnostic = "provider completed without final text or tool calls";
    let localized = "Réponse terminale invalide.";

    let frame = ws_turn_failure_frame(diagnostic, Some(localized), true);

    assert_eq!(frame["type"], "error");
    assert_eq!(frame["message"], "Réponse terminale invalide.");
    assert_eq!(frame["code"], "PROVIDER_ERROR");
    assert!(
        !frame["message"]
            .as_str()
            .unwrap_or_default()
            .contains(diagnostic),
        "WebSocket delivery must not fall back to the diagnostic when Fluent supplies text"
    );
}

#[test]
fn done_context_limits_prefer_active_usage_route() {
    let stale_startup_limits = galleon_config::schema::ResolvedContextLimits {
        model_context_window: 200_000,
        context_token_budget: 180_000,
        model_context_window_source: galleon_config::schema::ModelContextWindowSource::Configured,
    };
    assert_eq!(
        resolve_done_context_limits(Some(7_200), Some(8_000), stale_startup_limits),
        (7_200, Some(8_000)),
        "the done frame must report the route that produced the usage event"
    );
}

#[test]
fn done_context_limits_fall_back_to_agent_active_route_without_usage() {
    let active_limits = galleon_config::schema::ResolvedContextLimits {
        model_context_window: 8_000,
        context_token_budget: 0,
        model_context_window_source: galleon_config::schema::ModelContextWindowSource::Configured,
    };
    assert_eq!(
        resolve_done_context_limits(None, None, active_limits),
        (0, Some(8_000)),
    );
}

#[test]
fn done_context_limits_omit_unknown_compatibility_capacity() {
    let active_limits = galleon_config::schema::ResolvedContextLimits {
        model_context_window: galleon_config::schema::UNCONFIGURED_CONTEXT_WINDOW_FALLBACK,
        context_token_budget: 16_000,
        model_context_window_source:
            galleon_config::schema::ModelContextWindowSource::CompatibilityFallback,
    };
    assert_eq!(
        resolve_done_context_limits(None, None, active_limits),
        (16_000, None),
    );
}

// B4: the final served route overrides stale usage values in the terminal
// frame. A route switch after an earlier usage-bearing call (a no-usage
// vision reply) must report the FINAL route, not the earlier one.
#[test]
fn done_frame_prefers_final_served_route_over_stale_usage() {
    let final_vision = galleon_config::schema::ResolvedContextLimits {
        model_context_window: 8_000,
        context_token_budget: 7_200,
        model_context_window_source: galleon_config::schema::ModelContextWindowSource::Configured,
    };
    // Earlier usage-bearing text route left 180k/200k on the wire trackers.
    assert_eq!(
        done_frame_context_limits(Some(final_vision), Some(180_000), Some(200_000), None),
        (7_200, Some(8_000)),
        "a final no-usage vision route must override the earlier text route's usage numbers"
    );
}

// B4: with no served call (cache hit), the frame falls back to usage values,
// then to the resolved fallback route — preserving legacy behavior.
#[test]
fn done_frame_falls_back_when_no_call_served() {
    let fallback = galleon_config::schema::ResolvedContextLimits {
        model_context_window: 200_000,
        context_token_budget: 180_000,
        model_context_window_source: galleon_config::schema::ModelContextWindowSource::Configured,
    };
    // No final route, no usage: fall back to the resolved route.
    assert_eq!(
        done_frame_context_limits(None, None, None, Some(fallback)),
        (180_000, Some(200_000)),
    );
    // No final route but usage present: usage wins (legacy path).
    assert_eq!(
        done_frame_context_limits(None, Some(7_200), Some(8_000), Some(fallback)),
        (7_200, Some(8_000)),
    );
}

#[test]
fn usage_less_final_route_clears_the_previous_context_fill() {
    let mut fold = UsageFold::default();
    fold.apply(usage_event(
        "openai.default",
        "model-a",
        Some(1_024),
        None,
        Some(64),
        None,
        true,
    ));
    fold.apply(usage_event(
        "openai.default",
        "model-a",
        None,
        None,
        Some(32),
        None,
        true,
    ));

    assert_eq!(fold.total_input_tokens, Some(1_024));
    assert_eq!(fold.total_output_tokens, Some(96));
    assert_eq!(fold.last_input_tokens, None);
}

#[test]
fn websocket_handler_projects_anthropic_empty_terminal_stream_as_user_error() {
    // This production-shaped fixture exceeds the Linux test harness's
    // default stack; isolate only this test instead of weakening CI-wide
    // stack limits or dropping the real WebSocket boundary coverage.
    std::thread::Builder::new()
        .name("ws-empty-terminal-regression".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("test runtime")
                .block_on(
                    websocket_handler_projects_anthropic_empty_terminal_stream_as_user_error_inner(
                    ),
                );
        })
        .expect("spawn WebSocket regression thread")
        .join()
        .expect("WebSocket regression thread must not panic");
}

async fn websocket_handler_projects_anthropic_empty_terminal_stream_as_user_error_inner() {
    clawcrew_runtime::install_agent_runner();
    // This is a real WebSocket upgrade and a real agent built from live
    // config. The local Anthropic-shaped server completes an empty SSE
    // response, then returns an empty non-stream fallback, exercising the
    // production path through `process_chat_message` to the client.
    let mock_app = Router::new().route(
        "/v1/messages",
        post(|Json(request): Json<serde_json::Value>| async move {
            if request["stream"].as_bool() == Some(true) {
                (
                    [(header::CONTENT_TYPE, "text/event-stream")],
                    "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_test\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"usage\":{\"input_tokens\":1}}}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n",
                )
                    .into_response()
            } else {
                Json(serde_json::json!({
                    "id": "msg_test",
                    "type": "message",
                    "role": "assistant",
                    "model": "claude-test",
                    "content": [],
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 1, "output_tokens": 1}
                }))
                .into_response()
            }
        }),
    );
    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local Anthropic fixture");
    let mock_addr = mock_listener.local_addr().expect("fixture address");
    let mock_server = galleon_spawn::spawn!(async move {
        axum::serve(mock_listener, mock_app)
            .await
            .expect("local Anthropic fixture serves");
    });

    let tmp = tempfile::tempdir().expect("temporary gateway workspace");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("gateway workspace");
    let mut config = galleon_config::schema::Config {
        data_dir: workspace.clone(),
        config_path: tmp.path().join("config.toml"),
        ..Default::default()
    };
    config.memory.backend = "none".to_string();
    config.reliability.provider_retries = 0;
    config.providers.models.anthropic.insert(
        "fixture".to_string(),
        galleon_config::schema::AnthropicModelProviderConfig {
            base: galleon_config::schema::ModelProviderConfig {
                api_key: Some("test-key".to_string()),
                uri: Some(format!("http://{mock_addr}")),
                model: Some("claude-test".to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    config.risk_profiles.insert(
        "fixture".to_string(),
        galleon_config::schema::RiskProfileConfig::default(),
    );
    config.runtime_profiles.insert(
        "fixture".to_string(),
        galleon_config::schema::RuntimeProfileConfig::default(),
    );
    config.agents.insert(
        "web".to_string(),
        galleon_config::schema::AliasedAgentConfig {
            model_provider: "anthropic.fixture".into(),
            risk_profile: "fixture".into(),
            runtime_profile: "fixture".into(),
            workspace: galleon_config::multi_agent::AgentWorkspaceConfig {
                path: Some(workspace),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let state = crate::api::tests::test_state(config);
    let gateway_app = Router::new()
        .route("/ws/chat", get(handle_ws_chat))
        .with_state(state);
    let gateway_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local WebSocket gateway");
    let gateway_addr = gateway_listener.local_addr().expect("gateway address");
    let gateway_server = galleon_spawn::spawn!(async move {
        axum::serve(gateway_listener, gateway_app)
            .await
            .expect("local WebSocket gateway serves");
    });

    let (mut client, _) = connect_async(format!("ws://{gateway_addr}/ws/chat?agent=web"))
        .await
        .expect("WebSocket upgrade");
    let first = client
        .next()
        .await
        .expect("session_start frame")
        .expect("session_start");
    assert!(
        first
            .into_text()
            .expect("text session_start")
            .contains("session_start")
    );
    client
        .send(ClientMessage::Text(r#"{"type":"connect"}"#.into()))
        .await
        .expect("connect frame");
    let connected = client
        .next()
        .await
        .expect("connected frame")
        .expect("connected");
    assert!(
        connected
            .into_text()
            .expect("text connected")
            .contains("connected")
    );
    client
        .send(ClientMessage::Text(
            r#"{"type":"message","content":"test"}"#.into(),
        ))
        .await
        .expect("chat message");

    let mut terminal_error = None;
    for _ in 0..8 {
        let frame = tokio::time::timeout(Duration::from_secs(3), client.next())
            .await
            .expect("gateway response deadline")
            .expect("gateway stays connected")
            .expect("gateway frame");
        let text = frame.into_text().expect("text gateway frame");
        let json: serde_json::Value = serde_json::from_str(&text).expect("JSON gateway frame");
        if json["type"] == "error" {
            terminal_error = Some(json);
            break;
        }
    }

    let error = terminal_error.expect("empty terminal response reaches WebSocket client");
    assert_eq!(error["code"], "PROVIDER_ERROR");
    assert_eq!(
        error["message"],
        galleon_providers::terminal_error::semantic_empty_terminal_completion_message_with_renderer(
            None,
            render_cli_string,
        ),
    );
    assert_ne!(
        error["message"], "provider completed without final text or tool calls",
        "stable diagnostic must not leak into the user-facing WebSocket frame"
    );

    gateway_server.abort();
    mock_server.abort();
}

#[test]
fn websocket_connect_persists_a_restore_time_trim_before_going_live() {
    // Same contract as the ACP/RPC restore-persistence regressions: an
    // over-cap restored transcript trims in memory as soon as the socket
    // upgrade seeds it, before the session ever goes live. That retained
    // projection and its corrected breadcrumb must already be durable at
    // that point, so a reconnect that never sends a `message` frame does
    // not reload the untrimmed prefix and repeat the trim. Runs on a
    // larger-stack thread for the same reason as the sibling WebSocket
    // regression above: real agent construction exceeds the default test
    // harness stack.
    std::thread::Builder::new()
        .name("ws-restore-trim-regression".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("test runtime")
                .block_on(websocket_connect_persists_a_restore_time_trim_before_going_live_inner());
        })
        .expect("spawn WebSocket regression thread")
        .join()
        .expect("WebSocket regression thread must not panic");
}

async fn websocket_connect_persists_a_restore_time_trim_before_going_live_inner() {
    clawcrew_runtime::install_agent_runner();
    use galleon_infra::session_backend::SessionBackend;

    let tmp = tempfile::tempdir().expect("temporary gateway workspace");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("gateway workspace");
    let mut config = galleon_config::schema::Config {
        data_dir: workspace.clone(),
        config_path: tmp.path().join("config.toml"),
        ..Default::default()
    };
    config.memory.backend = "none".to_string();
    // A deliberately unreachable provider: no chat turn is ever issued
    // by this test, so agent construction must succeed without a live
    // network round trip.
    config.providers.models.anthropic.insert(
        "fixture".to_string(),
        galleon_config::schema::AnthropicModelProviderConfig {
            base: galleon_config::schema::ModelProviderConfig {
                api_key: Some("test-key".to_string()),
                uri: Some("http://127.0.0.1:1".to_string()),
                model: Some("claude-test".to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    config.risk_profiles.insert(
        "fixture".to_string(),
        galleon_config::schema::RiskProfileConfig::default(),
    );
    config.runtime_profiles.insert(
        "fixture".to_string(),
        galleon_config::schema::RuntimeProfileConfig {
            max_history_messages: Some(2),
            ..Default::default()
        },
    );
    config.agents.insert(
        "web".to_string(),
        galleon_config::schema::AliasedAgentConfig {
            model_provider: "anthropic.fixture".into(),
            risk_profile: "fixture".into(),
            runtime_profile: "fixture".into(),
            workspace: galleon_config::multi_agent::AgentWorkspaceConfig {
                path: Some(workspace),
                ..Default::default()
            },
            ..Default::default()
        },
    );

    let backend: Arc<dyn SessionBackend> =
        Arc::new(galleon_infra::session_store::SessionStore::new(tmp.path()).unwrap());
    let session_id = "restore-trim-session";
    let session_key = format!("{GW_SESSION_PREFIX}{session_id}");
    let over_cap = vec![
        galleon_providers::ChatMessage::user("turn one request"),
        galleon_providers::ChatMessage::assistant("turn one answer"),
        galleon_providers::ChatMessage::user("turn two request"),
        galleon_providers::ChatMessage::assistant("turn two answer"),
        galleon_providers::ChatMessage::user("turn three request"),
        galleon_providers::ChatMessage::assistant("turn three answer"),
    ];
    backend
        .replace_conversation_state(&session_key, &over_cap, false)
        .unwrap();

    let mut state = crate::api::tests::test_state(config);
    state.session_backend = Some(backend.clone());
    let gateway_app = Router::new()
        .route("/ws/chat", get(handle_ws_chat))
        .with_state(state);
    let gateway_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local WebSocket gateway");
    let gateway_addr = gateway_listener.local_addr().expect("gateway address");
    let gateway_server = galleon_spawn::spawn!(async move {
        axum::serve(gateway_listener, gateway_app)
            .await
            .expect("local WebSocket gateway serves");
    });

    let (mut client, _) = connect_async(format!(
        // This URL connects only to the test's loopback listener.
        "ws://{gateway_addr}/ws/chat?agent=web&session_id={session_id}" // nosemgrep: javascript.lang.security.detect-insecure-websocket.detect-insecure-websocket
    ))
    .await
    .expect("WebSocket upgrade");
    let first = client
        .next()
        .await
        .expect("session_start frame")
        .expect("session_start");
    assert!(
        first
            .into_text()
            .expect("text session_start")
            .contains("session_start")
    );

    // The handler seeds and restore-trims the agent only after it
    // receives its first frame (a `connect` control frame or the
    // fallback path below), so send exactly that and nothing else: no
    // `message` chat frame ever follows, mirroring a reconnect that
    // never issues another prompt.
    client
        .send(ClientMessage::Text(r#"{"type":"connect"}"#.into()))
        .await
        .expect("connect frame");
    let connected = client
        .next()
        .await
        .expect("connected frame")
        .expect("connected");
    assert!(
        connected
            .into_text()
            .expect("text connected")
            .contains("connected")
    );
    drop(client);

    // Give the server task a moment to finish the restore-time
    // persistence that runs before it starts waiting on the socket for
    // more frames.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let stored = backend.load(&session_key);
    assert!(
        stored.len() < over_cap.len(),
        "restore-time trim must persist the retained (capped) transcript, not the \
         untrimmed rows it was seeded from: {stored:?}"
    );
    assert!(
        !stored
            .iter()
            .any(|message| message.content == "turn one request"),
        "the oldest trimmed turn must not survive in the durable store: {stored:?}"
    );
    assert_eq!(
        backend.get_session_trim_breadcrumb(&session_key).unwrap(),
        Some(true),
        "restore-time trim must persist the corrected breadcrumb"
    );

    gateway_server.abort();
}

#[tokio::test]
async fn websocket_ping_interval_skips_missed_ticks() {
    let mut config = galleon_config::schema::Config::default();
    config.gateway.websocket_ping_interval_secs = 1;

    let interval = websocket_ping_interval(&config).expect("enabled ping interval");

    assert_eq!(
        interval.missed_tick_behavior(),
        tokio::time::MissedTickBehavior::Skip
    );
}

#[test]
fn websocket_ping_interval_handles_unvalidated_overflow_without_panicking() {
    let mut config = galleon_config::schema::Config::default();
    config.gateway.websocket_ping_interval_secs = u64::MAX;

    assert!(websocket_ping_interval(&config).is_none());
}

#[tokio::test]
async fn idle_chat_route_pings_before_and_preserves_the_first_client_message() {
    use axum::{Router, routing::get};
    use galleon_config::{
        multi_agent::MemoryBackendKind,
        schema::{AliasedAgentConfig, Config},
    };
    use tokio_tungstenite::{connect_async, tungstenite::Message as ClientMessage};

    let tmp = tempfile::TempDir::new().expect("temporary config root");
    let mut config = Config {
        data_dir: tmp.path().join("data"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.data_dir).expect("test data directory");
    config.gateway.websocket_ping_interval_secs = 1;
    let mut agent = AliasedAgentConfig::default();
    agent.memory.backend = MemoryBackendKind::None;
    config.agents.insert("web".to_string(), agent);

    let app = Router::new()
        .route("/ws/chat", get(handle_ws_chat))
        .with_state(crate::api::test_state(config));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("test listener");
    let address = listener.local_addr().expect("test listener address");
    let server = galleon_spawn::spawn!(async move {
        axum::serve(listener, app)
            .await
            .expect("test gateway server");
    });

    let (mut socket, _) = connect_async(format!(
        // This URL connects only to the test's loopback listener.
        "ws://{address}/ws/chat?agent=web&session_id=idle-test" // nosemgrep: javascript.lang.security.detect-insecure-websocket.detect-insecure-websocket
    ))
    .await
    .expect("chat WebSocket upgrade");

    let session_start = tokio::time::timeout(Duration::from_secs(1), socket.next())
        .await
        .expect("session_start timeout")
        .expect("session_start frame")
        .expect("session_start transport");
    assert!(matches!(session_start, ClientMessage::Text(_)));

    let ping = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .expect("idle ping timeout")
        .expect("idle ping frame")
        .expect("idle ping transport");
    assert!(matches!(ping, ClientMessage::Ping(_)));

    socket
        .send(ClientMessage::Text(
            serde_json::json!({"type": "message", "content": "hello"})
                .to_string()
                .into(),
        ))
        .await
        .expect("first chat message after idle ping");

    let response = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let frame = socket
                .next()
                .await
                .expect("response frame")
                .expect("response transport");
            if let ClientMessage::Text(text) = frame {
                break serde_json::from_str::<serde_json::Value>(&text)
                    .expect("JSON response frame");
            }
        }
    })
    .await
    .expect("first chat response timeout");

    assert_eq!(response["code"], "NEEDS_ONBOARDING");
    server.abort();
}

#[test]
fn first_chat_message_content_preserves_the_message_for_dispatch() {
    let text = serde_json::json!({
        "type": "message",
        "content": "hello after an idle keepalive"
    })
    .to_string();

    assert_eq!(
        first_chat_message_content(&text).as_deref(),
        Some("hello after an idle keepalive")
    );
}

#[test]
fn ws_turn_has_a_single_channel_identity() {
    // Regression: `Agent.channel_name` was set to "ws" to match the
    // back-channel registration key while the turn span still recorded
    // `channel = "wss"`, so one turn was attributed to two channel names.
    // All three uses now derive from WS_CHANNEL_KEY; this pins the value
    // to the historical ingress name so observability stays stable and
    // interactive-tool lookups still resolve.
    assert_eq!(
        WS_CHANNEL_KEY, "wss",
        "WS ingress identity must stay `wss` — it is the name already used by \
         the turn span and SSE `channel` field; changing it splits attribution"
    );
}

#[tokio::test]
async fn ws_back_channel_registers_under_the_ingress_identity() {
    // The interactive tools (`ask_user`, `poll`, `escalate_to_human`) look
    // the channel up by the agent's channel name. If the registration key
    // and WS_CHANNEL_KEY ever diverge, that lookup misses and the tools
    // silently fall back to an arbitrary seeded channel — the original bug.
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let pending = new_pending_approvals();
    let approval_channel = Arc::new(WsApprovalChannel::new(
        tx,
        pending,
        Duration::from_secs(WS_APPROVAL_TIMEOUT_SECS),
    ));

    let handle: galleon_tools::send_via::PerToolChannelHandle =
        Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new()));
    handle.write().insert(
        WS_CHANNEL_KEY.to_string(),
        approval_channel as Arc<dyn clawcrew_api::channel::Channel>,
    );

    // Interactive tools resolve the back-channel by the agent's channel
    // name; this is the lookup `ask_user` / `poll` / `escalate_to_human`
    // perform against their shared channel map.
    let resolved = handle.read().get(WS_CHANNEL_KEY).cloned();
    assert!(
        resolved.is_some(),
        "back-channel must be resolvable by the same key the agent reports \
         as its channel name ({WS_CHANNEL_KEY})"
    );
    assert!(
        !resolved.unwrap().supports_outbound_send(),
        "WS approval channel must declare that `send` does not deliver, so \
         poll/escalate_to_human fail honestly instead of reporting false success"
    );
}

#[test]
fn restore_trim_uses_live_history_trimmed_frame_shape() {
    let frame =
        history_trimmed_ws_frame(12, 3, "message limit", None, None, None, None, None, None);

    assert_eq!(
        frame,
        serde_json::json!({
            "type": "history_trimmed",
            "dropped_messages": 12,
            "kept_turns": 3,
            "reason": "message limit",
        })
    );
}

#[test]
fn safeguard_fallback_frame_carries_models_and_kind_without_category() {
    use galleon_providers::{SafeguardFallbackKind, SafeguardFallbackNotice};
    // A client-side notice with a non-empty classifier category — the
    // category must be dropped on the way to the browser.
    let notice = SafeguardFallbackNotice {
        kind: SafeguardFallbackKind::ClientSide,
        requested_model: "claude-fable-5".to_string(),
        served_model: "claude-opus-4-8".to_string(),
        category: Some("classifier-token".to_string()),
    };

    let frame = safeguard_fallback_ws_frame(Some(&notice))
        .expect("a recorded notice must produce a WS frame");

    assert_eq!(frame["type"], "safeguard_fallback");
    assert_eq!(frame["fallback_kind"], "client");
    assert_eq!(frame["requested_model"], "claude-fable-5");
    assert_eq!(frame["served_model"], "claude-opus-4-8");
    // Privacy contract: the classifier category (and any refusal
    // explanation) must NEVER cross the wire to the browser.
    assert!(
        frame.get("category").is_none(),
        "category key must not be serialized into the frame: {frame}"
    );
    assert!(
        !frame.to_string().contains("classifier-token"),
        "category value leaked into the safeguard frame: {frame}"
    );
}

#[test]
fn safeguard_fallback_frame_maps_server_side_kind() {
    use galleon_providers::{SafeguardFallbackKind, SafeguardFallbackNotice};
    let notice = SafeguardFallbackNotice {
        kind: SafeguardFallbackKind::ServerSide,
        requested_model: "claude-fable-5".to_string(),
        served_model: "claude-opus-4-8".to_string(),
        category: None,
    };

    let frame = safeguard_fallback_ws_frame(Some(&notice))
        .expect("a recorded notice must produce a WS frame");

    assert_eq!(frame["fallback_kind"], "server");
    assert_eq!(frame["type"], "safeguard_fallback");
}

#[test]
fn safeguard_fallback_frame_preserves_composed_recovery() {
    use galleon_providers::{SafeguardFallbackKind, SafeguardFallbackNotice};
    let notice = SafeguardFallbackNotice {
        kind: SafeguardFallbackKind::ClientAndServer,
        requested_model: "model-a".to_string(),
        served_model: "model-c".to_string(),
        category: Some("private-category".to_string()),
    };

    let frame = safeguard_fallback_ws_frame(Some(&notice)).expect("composed frame");
    assert_eq!(frame["fallback_kind"], "client_server");
    assert_eq!(frame["requested_model"], "model-a");
    assert_eq!(frame["served_model"], "model-c");
    assert!(!frame.to_string().contains("private-category"));
}

#[test]
fn safeguard_success_sequence_emits_exactly_one_notice_before_done() {
    use galleon_providers::{SafeguardFallbackKind, SafeguardFallbackNotice};
    let notice = SafeguardFallbackNotice {
        kind: SafeguardFallbackKind::ClientAndServer,
        requested_model: "model-a".to_string(),
        served_model: "model-c".to_string(),
        category: Some("private-category".to_string()),
    };

    let frames = success_terminal_ws_frames(
        Some(&notice),
        serde_json::json!({"type": "done", "full_response": "accepted"}),
    );
    let frame_types: Vec<_> = frames
        .iter()
        .filter_map(|frame| frame["type"].as_str())
        .collect();

    assert_eq!(frame_types, ["safeguard_fallback", "done"]);
    assert_eq!(
        frame_types
            .iter()
            .filter(|kind| **kind == "safeguard_fallback")
            .count(),
        1
    );
    assert!(!frames[0].to_string().contains("private-category"));
    assert_eq!(frames[1]["full_response"], "accepted");
}

#[test]
fn no_safeguard_notice_yields_no_frame() {
    // No notice drained this turn → no `safeguard_fallback` frame is sent.
    assert!(safeguard_fallback_ws_frame(None).is_none());
}

#[test]
fn history_trimmed_frame_carries_token_accounting_when_present() {
    let frame = history_trimmed_ws_frame(
        12,
        3,
        "context token budget exceeded",
        Some(500_000),
        Some(612_000),
        Some(117_000),
        Some("provider"),
        Some("calibrated"),
        None,
    );

    assert_eq!(frame["type"], "history_trimmed");
    assert_eq!(frame["token_budget"], 500_000);
    assert_eq!(frame["tokens_before"], 612_000);
    assert_eq!(frame["tokens_after"], 117_000);
    assert_eq!(frame["tokens_before_source"], "provider");
    assert_eq!(frame["tokens_after_source"], "calibrated");
}

#[test]
fn sop_ws_error_frames_resolve_via_fluent() {
    // The SOP WebSocket error frames are UI-surfaced and route through the
    // embedded en/cli.ftl. A renamed/typo'd key would silently ship the
    // missing-key fallback `{cli-sop-ws-...}` to the browser; guard against it.
    for key in [
        "cli-sop-ws-invalid-approval",
        "cli-sop-ws-engine-lock-poisoned",
        "cli-sop-ws-subsystem-disabled",
    ] {
        let s = galleon_i18n::get_required_cli_string(key);
        assert!(
            !s.starts_with('{') || !s.ends_with('}'),
            "fluent missing-key fallback leaked for {key}: {s:?}"
        );
    }
    let resolved = galleon_i18n::get_required_cli_string_with_args(
        "cli-sop-ws-resolve-failed",
        &[("error", "boom")],
    );
    assert!(
        resolved.contains("boom"),
        "the resolve-failed frame must interpolate the error: {resolved:?}"
    );
}

#[test]
fn extract_ws_token_from_authorization_header() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer zc_test123".parse().unwrap());
    assert_eq!(extract_ws_token(&headers, None), Some("zc_test123"));
}

#[test]
fn extract_ws_token_from_subprotocol() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "sec-websocket-protocol",
        "clawcrew.v1, bearer.zc_sub456".parse().unwrap(),
    );
    assert_eq!(extract_ws_token(&headers, None), Some("zc_sub456"));
}

#[test]
fn extract_ws_token_from_query_param() {
    let headers = HeaderMap::new();
    assert_eq!(
        extract_ws_token(&headers, Some("zc_query789")),
        Some("zc_query789")
    );
}

#[test]
fn extract_ws_token_precedence_header_over_subprotocol() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer zc_header".parse().unwrap());
    headers.insert("sec-websocket-protocol", "bearer.zc_sub".parse().unwrap());
    assert_eq!(
        extract_ws_token(&headers, Some("zc_query")),
        Some("zc_header")
    );
}

#[test]
fn extract_ws_token_precedence_subprotocol_over_query() {
    let mut headers = HeaderMap::new();
    headers.insert("sec-websocket-protocol", "bearer.zc_sub".parse().unwrap());
    assert_eq!(extract_ws_token(&headers, Some("zc_query")), Some("zc_sub"));
}

#[test]
fn extract_ws_token_returns_none_when_empty() {
    let headers = HeaderMap::new();
    assert_eq!(extract_ws_token(&headers, None), None);
}

#[test]
fn extract_ws_token_skips_empty_header_value() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer ".parse().unwrap());
    assert_eq!(
        extract_ws_token(&headers, Some("zc_fallback")),
        Some("zc_fallback")
    );
}

#[test]
fn extract_ws_token_skips_empty_query_param() {
    let headers = HeaderMap::new();
    assert_eq!(extract_ws_token(&headers, Some("")), None);
}

#[test]
fn extract_ws_token_subprotocol_with_multiple_entries() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "sec-websocket-protocol",
        "clawcrew.v1, bearer.zc_tok, other".parse().unwrap(),
    );
    assert_eq!(extract_ws_token(&headers, None), Some("zc_tok"));
}

#[test]
fn session_scoped_events_only_match_their_session() {
    let target_event = serde_json::json!({
        "type": "message",
        "session_id": "operator-1",
        "content": "deploy finished"
    });
    let other_event = serde_json::json!({
        "type": "message",
        "session_id": "operator-2",
        "content": "different session"
    });
    // No session_id and not on the global whitelist → dropped.
    let nameless_observability = serde_json::json!({
        "type": "agent_start",
        "source": "observability",
        "model": "gpt-4o"
    });
    // No session_id but on the global whitelist (`cron_result`) → forwarded.
    let cron = serde_json::json!({
        "type": "cron_result",
        "output": "global notification"
    });

    assert!(event_matches_session(&target_event, "operator-1"));
    assert!(!event_matches_session(&other_event, "operator-1"));
    assert!(!event_matches_session(
        &nameless_observability,
        "operator-1"
    ));
    assert!(event_matches_session(&cron, "operator-1"));
}

#[test]
fn event_matches_session_defaults_drops_unwhitelisted_no_session_frames() {
    // The pre-contract was `None => true`, which silently leaked
    // every BroadcastObserver telemetry frame (including `error`) into
    // every chat WebSocket. The fix flips the default; verify each
    // observed-in-the-wild leak shape is now blocked.
    for ty in [
        "agent_start",
        "agent_end",
        "llm_request",
        "tool_call",
        "tool_call_start",
        "error",
    ] {
        let frame = serde_json::json!({
            "type": ty,
            "source": "observability",
            "timestamp": "2026-06-04T00:00:00Z",
        });
        assert!(
            !event_matches_session(&frame, "operator-1"),
            "{ty} observability frame must be dropped from chat WS"
        );
    }
}

#[tokio::test]
async fn ws_memory_resolution_honors_agent_backend_none_over_install_backend() {
    use galleon_config::multi_agent::MemoryBackendKind;
    use galleon_config::schema::{AliasedAgentConfig, Config};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        data_dir: tmp.path().join("data"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    config.memory.backend = "sqlite.default".to_string();

    let mut agent = AliasedAgentConfig::default();
    agent.memory.backend = MemoryBackendKind::None;
    config.agents.insert("web".to_string(), agent);

    let memory = resolve_ws_memory_handle(&config, "web")
        .await
        .expect("WS per-agent memory resolution");

    assert!(
        memory.is_none(),
        "WebSocket consolidation must disable memory when the agent backend is none"
    );
}

#[test]
fn event_matches_session_passes_session_scoped_chat_messages() {
    // /api/sessions/{id}/messages broadcasts a session-scoped assistant
    // injection — that frame must reach the chat for its session.
    let assistant_inject = serde_json::json!({
        "type": "message",
        "session_id": "operator-1",
        "role": "assistant",
        "content": "hello",
    });
    assert!(event_matches_session(&assistant_inject, "operator-1"));
    assert!(!event_matches_session(&assistant_inject, "operator-2"));
}

#[test]
fn observability_tagged_frames_are_filtered() {
    // The defense-in-depth helper: any frame with source="observability"
    // is telemetry, regardless of type or session_id presence.
    let obs = serde_json::json!({
        "type": "tool_call",
        "source": "observability",
        "tool": "shell",
    });
    assert!(is_observability_telemetry(&obs));

    let chat = serde_json::json!({
        "type": "tool_call",
        "id": "call-1",
        "name": "file_write",
        "args": {"path": "/tmp/x"},
    });
    assert!(!is_observability_telemetry(&chat));
}

#[test]
fn observability_telemetry_filter_handles_malformed_source_field() {
    // Edge cases the previous tool-frame discriminator covered: ensure
    // the source-tag check doesn't false-positive on weird `source`
    // values that happen to coexist with chat-shaped frames.
    for source in [
        serde_json::Value::Null,
        serde_json::json!(""),
        serde_json::json!(42),
        serde_json::json!("api"),
        serde_json::json!({"nested": "x"}),
    ] {
        let frame = serde_json::json!({
            "type": "tool_call",
            "id": "call-1",
            "name": "file_write",
            "source": source,
        });
        assert!(
            !is_observability_telemetry(&frame),
            "frame with source={frame:?} must not be flagged as observability telemetry",
        );
    }
}

#[test]
fn chat_tool_frames_pass_through_when_session_scoped() {
    // Real chat tool frames (ws.rs process_chat_message) are streamed
    // over the per-turn channel, not the broadcast bus, but if anything
    // ever rebroadcasts one with the right session_id it must pass.
    let chat_tool_call = serde_json::json!({
        "type": "tool_call",
        "session_id": "operator-1",
        "id": "call-1",
        "name": "file_write",
        "args": {"path": "/tmp/x"},
    });
    assert!(event_matches_session(&chat_tool_call, "operator-1"));
    assert!(!is_observability_telemetry(&chat_tool_call));
}

#[test]
fn resolve_session_cwd_uses_requested_cwd() {
    let requested = tempfile::tempdir().unwrap();
    let fallback = tempfile::tempdir().unwrap();

    let resolved =
        resolve_session_cwd(Some(requested.path().to_str().unwrap()), fallback.path()).unwrap();

    assert_eq!(resolved, requested.path().canonicalize().unwrap());
}

#[test]
fn resolve_session_cwd_uses_default_workspace_without_request() {
    let fallback = tempfile::tempdir().unwrap();

    let resolved = resolve_session_cwd(None, fallback.path()).unwrap();

    assert_eq!(resolved, fallback.path().canonicalize().unwrap());
}

#[test]
fn resolve_ws_session_cwd_defaults_to_agent_workspace_without_request() {
    use galleon_config::schema::{AliasedAgentConfig, Config};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        data_dir: tmp.path().join("data"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    config
        .agents
        .insert("web".to_string(), AliasedAgentConfig::default());
    std::fs::create_dir_all(&config.data_dir).unwrap();
    let agent_workspace = config.agent_workspace_dir("web");
    assert!(!agent_workspace.exists());

    let resolved = resolve_ws_session_cwd(None, &config, "web").unwrap();

    assert!(agent_workspace.exists());
    assert_eq!(resolved, agent_workspace.canonicalize().unwrap());
    assert_ne!(resolved, config.data_dir.canonicalize().unwrap());
}

#[test]
fn resolve_ws_session_cwd_keeps_requested_cwd_strict() {
    use galleon_config::schema::{AliasedAgentConfig, Config};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        data_dir: tmp.path().join("data"),
        config_path: tmp.path().join("config.toml"),
        ..Config::default()
    };
    config
        .agents
        .insert("web".to_string(), AliasedAgentConfig::default());
    let agent_workspace = config.agent_workspace_dir("web");
    let missing_requested = tmp.path().join("missing");

    let err = resolve_ws_session_cwd(Some(missing_requested.to_str().unwrap()), &config, "web")
        .expect_err("explicit missing cwd should be rejected");

    assert!(!agent_workspace.exists());
    assert!(err.to_string().contains("cwd is not a usable directory"));
}

#[test]
fn resolve_session_cwd_rejects_missing_directory() {
    let fallback = tempfile::tempdir().unwrap();
    let missing = fallback.path().join("missing");

    let err = resolve_session_cwd(Some(missing.to_str().unwrap()), fallback.path())
        .expect_err("missing cwd should be rejected");

    assert!(err.to_string().contains("cwd is not a usable directory"));
}

#[test]
fn needs_onboarding_ws_error_points_to_onboard() {
    let config = galleon_config::schema::Config::default();
    let frame =
        needs_onboarding_ws_error(&config).expect("empty model must produce a WS onboarding error");

    assert_eq!(frame["type"], "error");
    assert_eq!(frame["error"], "needs_onboarding");
    assert_eq!(frame["code"], "NEEDS_ONBOARDING");
    assert_eq!(frame["url"], "/onboard");
    let message = frame["message"]
        .as_str()
        .expect("onboarding WS error must include a message");
    assert!(
        !message.starts_with('{') && !message.ends_with('}'),
        "missing Fluent key fallback leaked into WS error message: {message:?}"
    );
    assert!(
        message.to_lowercase().contains("quickstart"),
        "WS setup-gap message must explain the setup gap: {message:?}"
    );
}

#[test]
fn needs_onboarding_ws_error_uses_current_configured_model() {
    let mut config = galleon_config::schema::Config::default();
    config.providers.models.openai.insert(
        "default".to_string(),
        galleon_config::schema::OpenAIModelProviderConfig {
            base: galleon_config::schema::ModelProviderConfig {
                model: Some("openai/gpt-4o-mini".to_string()),
                api_key: Some("sk-test".to_string()),
                ..Default::default()
            },
        },
    );

    assert!(
        needs_onboarding_ws_error(&config).is_none(),
        "current configured model must allow WebSocket agent construction to continue"
    );
}

// The mid-turn `client_msg` arm in `forward_fut`
// must (a) classify stream-end / close / error frames as "client gone"
// and (b) cancel the turn token so `tokio::join!(turn_fut, forward_fut)`
// can return — a bare `continue` hot-loops the select forever.
#[derive(Debug, PartialEq, Eq)]
enum DisconnectAction {
    Break,
    Continue,
    ProcessText,
}

fn classify_client_msg(
    msg: Option<Result<axum::extract::ws::Message, &'static str>>,
) -> DisconnectAction {
    use axum::extract::ws::Message;
    match msg {
        Some(Ok(Message::Text(_))) => DisconnectAction::ProcessText,
        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => DisconnectAction::Break,
        _ => DisconnectAction::Continue,
    }
}

#[test]
fn mid_turn_client_msg_breaks_on_stream_end_close_or_err() {
    use axum::extract::ws::Message;
    assert_eq!(classify_client_msg(None), DisconnectAction::Break);
    assert_eq!(
        classify_client_msg(Some(Ok(Message::Close(None)))),
        DisconnectAction::Break,
    );
    assert_eq!(
        classify_client_msg(Some(Err("io"))),
        DisconnectAction::Break,
    );
    assert_eq!(
        classify_client_msg(Some(Ok(Message::Ping(Default::default())))),
        DisconnectAction::Continue,
    );
    assert_eq!(
        classify_client_msg(Some(Ok(Message::Text("{}".into())))),
        DisconnectAction::ProcessText,
    );
}

#[test]
fn mid_turn_disconnect_cancel_unblocks_joined_turn() {
    let token = tokio_util::sync::CancellationToken::new();
    let clone_for_turn = token.clone();
    assert!(!clone_for_turn.is_cancelled());
    token.cancel();
    assert!(
        clone_for_turn.is_cancelled(),
        "cloned token (held by turn_fut via agent.turn_streamed) must observe cancellation"
    );
}

#[test]
fn session_queue_errors_map_to_explicit_websocket_codes() {
    use crate::session_queue::SessionQueueError;

    assert_eq!(
        session_queue_ws_error_code(&SessionQueueError::QueueFull {
            session_id: "gw_test".into(),
            depth: 2,
        }),
        "SESSION_QUEUE_FULL"
    );
    assert_eq!(
        session_queue_ws_error_code(&SessionQueueError::Timeout {
            session_id: "gw_test".into(),
        }),
        "SESSION_QUEUE_TIMEOUT"
    );
}

struct DeletedSessionBackend {
    append_calls: std::sync::Mutex<Vec<String>>,
    rewrite_calls: std::sync::Mutex<Vec<String>>,
}

impl galleon_infra::session_backend::SessionBackend for DeletedSessionBackend {
    fn load(&self, _session_key: &str) -> Vec<galleon_providers::ChatMessage> {
        Vec::new()
    }
    fn append(
        &self,
        session_key: &str,
        message: &galleon_providers::ChatMessage,
    ) -> std::io::Result<()> {
        self.append_calls.lock().unwrap().push(format!(
            "{}:{}:{}",
            session_key, message.role, message.content
        ));
        Ok(())
    }
    fn rewrite_messages(
        &self,
        session_key: &str,
        messages: &[galleon_providers::ChatMessage],
    ) -> std::io::Result<()> {
        self.rewrite_calls
            .lock()
            .unwrap()
            .push(format!("{}:{}", session_key, messages.len()));
        Ok(())
    }
    fn remove_last(&self, _session_key: &str) -> std::io::Result<bool> {
        Ok(false)
    }
    fn list_sessions(&self) -> Vec<String> {
        Vec::new()
    }
    fn session_exists(&self, _session_key: &str) -> bool {
        // The user deleted the session between cancel and append.
        false
    }
}

#[test]
fn replace_conversation_state_skips_deleted_session() {
    let backend = DeletedSessionBackend {
        append_calls: std::sync::Mutex::new(Vec::new()),
        rewrite_calls: std::sync::Mutex::new(Vec::new()),
    };
    let durable = vec![
        galleon_providers::ChatMessage::user("hi"),
        galleon_providers::ChatMessage::assistant("[interrupted by user]"),
    ];

    replace_conversation_state_unless_deleted(&backend, "gw_deleted", &durable, false);

    assert!(
        backend.rewrite_calls.lock().unwrap().is_empty(),
        "replacing durable state must not resurrect a session whose \
         session_exists() returned false"
    );
}

/// A backend that implements only the required `SessionBackend`
/// primitives (`load`/`append`/`remove_last`/`list_sessions`) and does
/// NOT override `rewrite_messages`, exercising the trait's default
/// replacement implementation built from those primitives.
struct AppendOnlyBackend {
    messages: std::sync::Mutex<Vec<galleon_providers::ChatMessage>>,
}

impl galleon_infra::session_backend::SessionBackend for AppendOnlyBackend {
    fn load(&self, _session_key: &str) -> Vec<galleon_providers::ChatMessage> {
        self.messages.lock().unwrap().clone()
    }
    fn append(
        &self,
        _session_key: &str,
        message: &galleon_providers::ChatMessage,
    ) -> std::io::Result<()> {
        self.messages.lock().unwrap().push(message.clone());
        Ok(())
    }
    fn remove_last(&self, _session_key: &str) -> std::io::Result<bool> {
        Ok(self.messages.lock().unwrap().pop().is_some())
    }
    fn list_sessions(&self) -> Vec<String> {
        Vec::new()
    }
}

#[test]
fn append_only_backend_durably_replaces_state_through_the_websocket_persistence_boundary() {
    // A production `SessionBackend` that only implements the required
    // append/remove_last primitives must still durably persist the
    // agent's authoritative post-turn history through the WebSocket
    // completion path — the default `rewrite_messages` must not
    // silently no-op and drop the caller's replacement.
    let backend = AppendOnlyBackend {
        messages: std::sync::Mutex::new(vec![
            galleon_providers::ChatMessage::user("stale first turn"),
            galleon_providers::ChatMessage::assistant("stale reply"),
        ]),
    };
    let authoritative = vec![
        galleon_providers::ChatMessage::user("trimmed second turn"),
        galleon_providers::ChatMessage::assistant("final reply"),
    ];

    replace_conversation_state_unless_deleted(&backend, "gw_append_only", &authoritative, true);

    let persisted = backend.messages.lock().unwrap().clone();
    assert_eq!(
        persisted
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>(),
        authoritative
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>(),
        "replacement must durably overwrite the stale transcript, not silently no-op"
    );
}

/// A backend whose durable replacement always fails, standing in for a
/// disk or other operational failure at the WebSocket persistence
/// boundary.
struct FailingReplaceBackend;

impl galleon_infra::session_backend::SessionBackend for FailingReplaceBackend {
    fn load(&self, _session_key: &str) -> Vec<galleon_providers::ChatMessage> {
        Vec::new()
    }
    fn append(
        &self,
        _session_key: &str,
        _message: &galleon_providers::ChatMessage,
    ) -> std::io::Result<()> {
        Ok(())
    }
    fn remove_last(&self, _session_key: &str) -> std::io::Result<bool> {
        Ok(false)
    }
    fn list_sessions(&self) -> Vec<String> {
        Vec::new()
    }
    fn session_exists(&self, _session_key: &str) -> bool {
        // Unlike `DeletedSessionBackend`, this session is still live;
        // only the durable write itself fails.
        true
    }
    fn rewrite_messages(
        &self,
        _session_key: &str,
        _messages: &[galleon_providers::ChatMessage],
    ) -> std::io::Result<()> {
        Err(std::io::Error::other(
            "simulated durable replacement failure",
        ))
    }
}

#[test]
fn replace_conversation_state_unless_deleted_reports_durable_failure() {
    // The WebSocket completion path must not claim the turn's history
    // was durably persisted when the backend actually failed the write:
    // the caller uses this to decide whether the authoritative-history
    // guarantee held for this turn.
    let backend = FailingReplaceBackend;
    let durable = vec![galleon_providers::ChatMessage::user("hi")];

    assert!(
        !replace_conversation_state_unless_deleted(&backend, "gw_failing", &durable, false),
        "a failed durable replacement must be reported to the caller, not swallowed"
    );
}

/// A `Sink<Message>` that just collects the text frames sent to it, so a handler
/// smoke can inspect the response without a real WebSocket.
struct CollectSink(Vec<String>);
impl futures_util::Sink<Message> for CollectSink {
    type Error = std::convert::Infallible;
    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn start_send(self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        if let Message::Text(t) = item {
            self.get_mut().0.push(t.to_string());
        }
        Ok(())
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn ws_sop_frame_enforces_policy_membership_via_auth_subject() {
    use galleon_config::pairing::PairingGuard;
    // Reuse the HTTP policied-gate harness: a run parked at a `prod` policy whose
    // group is granted to the paired-token subject (bare, any source).
    let (state, run_id) = crate::api_sop::tests::state_with_policied_gate("ws-tok");
    let member = PairingGuard::token_hash("ws-tok");
    let outsider = PairingGuard::token_hash("someone-else");
    let frame = serde_json::json!({
        "kind": "sop",
        "run_id": run_id,
        "decision": "approve",
    });
    let run_status = |st: &AppState| {
        st.sop_runner
            .as_ref()
            .unwrap()
            .list_runs(None)
            .unwrap()
            .iter()
            .find(|r| r.run_id == run_id)
            .map(|r| format!("{:?}", r.status))
    };

    // A non-member WS subject is rejected; the gate stays waiting.
    let mut sink = CollectSink(Vec::new());
    assert!(
        handle_ws_sop_frame(&frame, &state, "sess-1", Some(&outsider), &mut sink).await,
        "a sop-kind frame is handled"
    );
    assert!(
        sink.0.iter().any(|m| m.contains("not_authorized")),
        "a non-member WS caller is not authorized: {:?}",
        sink.0
    );
    assert_eq!(
        run_status(&state).as_deref(),
        Some("WaitingApproval"),
        "the gate stays waiting after a non-member WS attempt"
    );

    // The member WS subject clears the policied gate.
    let mut sink = CollectSink(Vec::new());
    handle_ws_sop_frame(&frame, &state, "sess-1", Some(&member), &mut sink).await;
    assert!(
        sink.0.iter().any(|m| m.contains("resumed")),
        "an authenticated member clears the gate over WS: {:?}",
        sink.0
    );
    assert_ne!(
        run_status(&state).as_deref(),
        Some("WaitingApproval"),
        "the gate is cleared once an authorized WS member approves"
    );
}

/// done-frame model_context_window: present when the provider has an
/// explicit `context_window`, absent when it does not.
#[test]
fn done_frame_model_context_window_presence_tracks_provider_config() {
    use galleon_config::schema::{AliasedAgentConfig, Config, RuntimeProfileConfig};
    use std::collections::HashMap;

    // (provider_alias, context_window, expected_model_window,
    // expected_max_context_tokens)
    let cases: &[(&str, Option<usize>, Option<u64>, u64)] = &[
        // Provider has no context_window — field must be absent.
        ("openrouter.default", None, None, 32_000),
        // Provider sets context_window — field must appear on the wire.
        (
            "openrouter.glm-5.2",
            Some(1_000_000),
            Some(1_000_000),
            800_000,
        ),
    ];

    for &(provider_alias, context_window, expected_window, expected_max_ctx) in cases {
        let mut runtime_profiles = HashMap::new();
        runtime_profiles.insert(
            "coding".to_string(),
            RuntimeProfileConfig {
                max_context_tokens: Some(expected_max_ctx as usize),
                ..RuntimeProfileConfig::default()
            },
        );

        let (vendor, model_alias) = provider_alias.split_once('.').unwrap();
        let mut agents = HashMap::new();
        agents.insert(
            "coder".to_string(),
            AliasedAgentConfig {
                enabled: true,
                runtime_profile: "coding".into(),
                model_provider: provider_alias.into(),
                ..AliasedAgentConfig::default()
            },
        );

        let mut providers = galleon_config::providers::Providers::default();
        let entry = providers
            .models
            .ensure(vendor, model_alias)
            .expect("ensure creates entry");
        entry.model = Some("glm-5.2".to_string());
        if let Some(w) = context_window {
            entry.context_window = Some(w);
        }

        let cfg = Config {
            agents,
            runtime_profiles,
            providers,
            ..Config::default()
        };

        let limits = cfg.resolved_context_limits_for_route("coder", provider_alias, "glm-5.2");
        let max_ctx = limits.context_token_budget as u64;
        let model_ctx_window = limits.configured_model_context_window().map(|v| v as u64);
        assert_eq!(
            model_ctx_window, expected_window,
            "model_provider_context_window_opt({provider_alias}) must return {expected_window:?}"
        );

        let meta = DoneFrameMeta {
            full_response: "ok",
            input_tokens: Some(100),
            output_tokens: Some(50),
            tokens_used: Some(150),
            cost_usd: Some(0.001),
            model: "glm-5.2",
            provider: provider_alias,
            provider_ref: provider_alias,
            max_context_tokens: max_ctx,
            model_context_window: model_ctx_window,
            last_input_tokens: Some(100),
            last_serving_provider_ref: Some(provider_alias),
            last_serving_model: Some("glm-5.2"),
        };
        let done = build_done_frame_json(&meta, &[]);
        let v: serde_json::Value = serde_json::from_str(&done.to_string()).unwrap();

        assert_eq!(v["type"], "done");
        assert_eq!(
            v["max_context_tokens"], expected_max_ctx,
            "profile budget must be emitted"
        );
        // Provider field carries the full configured reference (e.g., "openai.vendor"),
        // matching pre-change gateway behavior. provider_ref carries the serving identity.
        assert_eq!(v["provider"], provider_alias);
        assert_eq!(v["provider_ref"], provider_alias);
        assert_eq!(v["last_serving_provider_ref"], provider_alias);
        assert_eq!(v["last_serving_model"], "glm-5.2");
        assert!(
            v["usage_by_provider"].as_array().unwrap().is_empty(),
            "usage_by_provider must be empty when no Usage events accumulated"
        );
        if let Some(window) = expected_window {
            assert_eq!(
                v["model_context_window"], window,
                "done-frame must carry the provider's explicit context_window"
            );
        } else {
            assert!(
                v.get("model_context_window").is_none(),
                "model_context_window must be absent when provider has no context_window"
            );
        }
    }
}

/// Regression: done-frame model_context_window follows the live provider
/// after either a session/configure A→B switch or an in-turn model switch,
/// not the static agent alias. Both paths use the same shared resolver.
#[test]
fn done_frame_model_window_follows_live_provider_switch() {
    use galleon_config::schema::{AliasedAgentConfig, Config, RuntimeProfileConfig};
    use std::collections::HashMap;

    // (switched_provider_alias, scenario_label)
    let cases: &[(&str, &str)] = &[
        ("ollama.provider-b", "session/configure switch"),
        ("ollama.llama3", "in-turn model_switch"),
    ];

    for &(live_provider_ref, label) in cases {
        let mut runtime_profiles = HashMap::new();
        runtime_profiles.insert(
            "coding".to_string(),
            RuntimeProfileConfig {
                max_context_tokens: Some(800_000),
                ..RuntimeProfileConfig::default()
            },
        );

        let mut agents = HashMap::new();
        agents.insert(
            "coder".to_string(),
            AliasedAgentConfig {
                enabled: true,
                runtime_profile: "coding".into(),
                model_provider: "openrouter.glm-5.2".into(),
                ..AliasedAgentConfig::default()
            },
        );

        let mut providers = galleon_config::providers::Providers::default();
        // Provider A (static binding) — no context_window.
        providers
            .models
            .ensure("openrouter", "glm-5.2")
            .expect("ensure A");
        // Provider B (switched-to) — has context_window.
        let (b_vendor, b_alias) = live_provider_ref.split_once('.').unwrap();
        let entry_b = providers
            .models
            .ensure(b_vendor, b_alias)
            .expect("ensure B");
        entry_b.context_window = Some(1_000_000);
        entry_b.model = Some("glm-5.2".to_string());

        let cfg = Config {
            agents,
            runtime_profiles,
            providers,
            ..Config::default()
        };

        let limits = cfg.resolved_context_limits_for_route("coder", live_provider_ref, "glm-5.2");
        let model_ctx_window = limits.configured_model_context_window().map(|v| v as u64);
        assert_eq!(
            model_ctx_window,
            Some(1_000_000),
            "resolver must return B's window for {label}, not A's"
        );

        let max_ctx = limits.context_token_budget as u64;
        let meta = DoneFrameMeta {
            full_response: "ok",
            input_tokens: Some(100),
            output_tokens: Some(50),
            tokens_used: Some(150),
            cost_usd: Some(0.001),
            model: "glm-5.2",
            provider: live_provider_ref,
            provider_ref: live_provider_ref,
            max_context_tokens: max_ctx,
            model_context_window: model_ctx_window,
            last_input_tokens: Some(100),
            last_serving_provider_ref: Some(live_provider_ref),
            last_serving_model: Some("glm-5.2"),
        };
        let done = build_done_frame_json(&meta, &[]);
        let v: serde_json::Value = serde_json::from_str(&done.to_string()).unwrap();

        assert_eq!(v["type"], "done");
        assert_eq!(
            v["model_context_window"], 1_000_000,
            "done-frame must carry B's live window after {label}, not A's static alias window"
        );
        // Provider field carries the full configured reference (e.g., "openrouter.b"),
        // matching pre-change gateway behavior. provider_ref carries the serving identity.
        assert_eq!(v["provider"], live_provider_ref);
        assert_eq!(v["provider_ref"], live_provider_ref);
        assert_eq!(v["last_serving_provider_ref"], live_provider_ref);
        assert_eq!(v["last_serving_model"], "glm-5.2");
    }
}

/// Blocking (note12): same-profile fallback to a different model must not
/// borrow the primary model's capacity. Configures `openai.default` for
/// model-a with a 200k window and fallback_models=[model-b]. Serving
/// model-b under the same provider_ref must omit `model_context_window`
/// so clients fall back to the trim budget.
#[test]
fn done_frame_model_window_omitted_on_same_profile_model_fallback() {
    use galleon_config::schema::{AliasedAgentConfig, Config, RuntimeProfileConfig};
    use std::collections::HashMap;

    let mut runtime_profiles = HashMap::new();
    runtime_profiles.insert(
        "coding".to_string(),
        RuntimeProfileConfig {
            max_context_tokens: Some(800_000),
            ..RuntimeProfileConfig::default()
        },
    );

    let mut agents = HashMap::new();
    agents.insert(
        "coder".to_string(),
        AliasedAgentConfig {
            enabled: true,
            runtime_profile: "coding".into(),
            model_provider: "openai.default".into(),
            ..AliasedAgentConfig::default()
        },
    );

    let mut providers = galleon_config::providers::Providers::default();
    let entry = providers
        .models
        .ensure("openai", "default")
        .expect("ensure entry");
    entry.model = Some("model-a".to_string());
    entry.fallback_models = vec!["model-b".to_string()];
    entry.context_window = Some(200_000);

    let cfg = Config {
        agents,
        runtime_profiles,
        providers,
        ..Config::default()
    };

    // Shared resolution: primary matches, fallback does not.
    assert_eq!(
        cfg.model_provider_context_window_opt("openai.default", "model-a"),
        Some(200_000)
    );
    assert_eq!(
        cfg.model_provider_context_window_opt("openai.default", "model-b"),
        None,
        "fallback model must not borrow the primary's capacity"
    );

    // Gateway projection: drive the production fold with a model-b Usage
    // event, then resolve exactly as the WS handler does.
    let mut fold = UsageFold::default();
    fold.apply(usage_event(
        "openai.default",
        "model-b",
        Some(1000),
        None,
        Some(500),
        Some(0.01),
        true,
    ));
    assert_eq!(fold.last_provider_ref.as_deref(), Some("openai.default"));
    assert_eq!(fold.last_model.as_deref(), Some("model-b"));

    let effective_model = fold.last_model.as_deref().unwrap_or("model-a");
    let provider_ref = fold.last_provider_ref.as_deref().unwrap();
    let limits = cfg.resolved_context_limits_for_route("coder", provider_ref, effective_model);
    let model_ctx_window = limits.configured_model_context_window().map(|v| v as u64);
    assert!(
        model_ctx_window.is_none(),
        "gateway must omit window when served model differs from configured primary"
    );

    let max_ctx = limits.context_token_budget as u64;
    let meta = DoneFrameMeta {
        full_response: "ok",
        input_tokens: Some(1000),
        output_tokens: Some(500),
        tokens_used: Some(1500),
        cost_usd: Some(0.01),
        model: effective_model,
        provider: "openai.default",
        provider_ref,
        max_context_tokens: max_ctx,
        model_context_window: model_ctx_window,
        last_input_tokens: Some(1000),
        last_serving_provider_ref: Some(provider_ref),
        last_serving_model: Some(effective_model),
    };
    let done = build_done_frame_json(&meta, &[]);
    let v: serde_json::Value = serde_json::from_str(&done.to_string()).unwrap();
    assert!(
        v.get("model_context_window").is_none(),
        "done-frame must omit model_context_window on same-profile fallback"
    );
    assert_eq!(v["max_context_tokens"], 32_000);
    assert_eq!(v["last_serving_model"], "model-b");
}

/// Regression: two models served under one provider_ref produce two
/// distinct breakdown entries keyed by (provider_ref, model). Drives the
/// production fold so a regression in `UsageFold::apply` is caught
/// (a hand-rolled map would pass even if the fold broke).
#[test]
fn usage_by_provider_same_provider_two_models() {
    let mut fold = UsageFold::default();
    fold.apply(usage_event(
        "openrouter.vertex",
        "model-a",
        Some(1000),
        None,
        Some(500),
        Some(0.01),
        true,
    ));
    fold.apply(usage_event(
        "openrouter.vertex",
        "model-b",
        Some(2000),
        Some(100),
        Some(1000),
        Some(0.02),
        true,
    ));

    // Same provider_ref but different models: exactly 2 entries, sorted
    // by model name, with per-model tokens/cost (including cached).
    let entries = fold.take_sorted_entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].provider_ref, "openrouter.vertex");
    assert_eq!(entries[0].model, "model-a");
    assert_eq!(entries[0].input_tokens, 1000);
    assert_eq!(entries[0].output_tokens, 500);
    assert_eq!(entries[0].cached_input_tokens, 0);
    assert_eq!(entries[0].cost_usd, 0.01);
    assert_eq!(entries[1].provider_ref, "openrouter.vertex");
    assert_eq!(entries[1].model, "model-b");
    assert_eq!(entries[1].input_tokens, 2000);
    assert_eq!(entries[1].output_tokens, 1000);
    assert_eq!(entries[1].cached_input_tokens, 100);
    assert_eq!(entries[1].cost_usd, 0.02);
    assert_eq!(UsageFold::total_cost_usd(&entries), Some(0.03));
}

/// Build a `TurnEvent::Usage` for fold tests.
fn usage_event(
    provider_ref: &str,
    model: &str,
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost_usd: Option<f64>,
    accepted: bool,
) -> clawcrew_api::agent::TurnEvent {
    clawcrew_api::agent::TurnEvent::Usage {
        input_tokens,
        cached_input_tokens,
        output_tokens,
        cost_usd,
        context_token_budget: None,
        model_context_window: None,
        provider_ref: provider_ref.to_string(),
        model: model.to_string(),
        accepted,
    }
}

/// Blocking regression (note9 blocker 1): an accepted Usage event followed
/// by a rejected billed Usage event must keep the accepted-serving
/// snapshot while billing both attempts. Drives `UsageFold::apply` — the
/// exact function the WS handler invokes per event — then renders the
/// done frame from the fold state with the same field mapping the handler
/// uses, asserting the wire-observable contract.
#[test]
fn usage_fold_accepted_then_rejected_billed_keeps_accepted_snapshot() {
    let mut fold = UsageFold::default();
    fold.apply(usage_event(
        "openrouter.a",
        "model-a",
        Some(1000),
        None,
        Some(500),
        Some(0.01),
        true,
    ));
    fold.apply(usage_event(
        "openrouter.b",
        "model-b",
        Some(2000),
        None,
        Some(1000),
        Some(0.02),
        false,
    ));

    // Snapshot stays accepted-sourced: the rejected attempt must not
    // re-point identity or the meter ceiling.
    assert_eq!(fold.last_provider_ref.as_deref(), Some("openrouter.a"));
    assert_eq!(fold.last_model.as_deref(), Some("model-a"));
    assert_eq!(fold.last_input_tokens, Some(1000));
    // Billing aggregates both attempts.
    assert_eq!(fold.total_input_tokens, Some(3000));
    assert_eq!(fold.total_output_tokens, Some(1500));

    let entries = fold.take_sorted_entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].model, "model-a");
    assert_eq!(entries[0].cost_usd, 0.01);
    assert_eq!(entries[1].model, "model-b");
    assert_eq!(entries[1].cost_usd, 0.02);
    let cost_usd = UsageFold::total_cost_usd(&entries);
    assert_eq!(cost_usd, Some(0.03));

    // Wire contract: done frame carries both attempts in the ledger and
    // cost total, but the serving identity and meter snapshot come from
    // the accepted event.
    let meta = DoneFrameMeta {
        full_response: "ok",
        input_tokens: Some(3000),
        output_tokens: Some(1500),
        tokens_used: Some(4500),
        cost_usd,
        model: "model-a",
        provider: "openrouter.a",
        provider_ref: "openrouter.a",
        max_context_tokens: 800_000,
        model_context_window: Some(1_000_000),
        last_input_tokens: Some(1000),
        last_serving_provider_ref: Some("openrouter.a"),
        last_serving_model: Some("model-a"),
    };
    let done = build_done_frame_json(&meta, &entries);
    let v: serde_json::Value = serde_json::from_str(&done.to_string()).unwrap();
    assert_eq!(v["cost_usd"], 0.03);
    assert_eq!(v["last_serving_provider_ref"], "openrouter.a");
    assert_eq!(v["last_serving_model"], "model-a");
    assert_eq!(v["last_input_tokens"], 1000);
    assert_eq!(v["model_context_window"], 1_000_000);
    let ubp = v["usage_by_provider"].as_array().unwrap();
    assert_eq!(ubp.len(), 2);

    // Negative control: a rejected event with input_tokens: None must
    // neither move the snapshot nor clear the accepted prompt size.
    let mut fold = UsageFold::default();
    fold.apply(usage_event(
        "openrouter.a",
        "model-a",
        Some(1000),
        None,
        Some(500),
        Some(0.01),
        true,
    ));
    fold.apply(usage_event(
        "openrouter.b",
        "model-b",
        None,
        None,
        None,
        Some(0.005),
        false,
    ));
    assert_eq!(fold.last_provider_ref.as_deref(), Some("openrouter.a"));
    assert_eq!(fold.last_model.as_deref(), Some("model-a"));
    assert_eq!(
        fold.last_input_tokens,
        Some(1000),
        "rejected usage-less event must not clear the accepted snapshot"
    );
    assert_eq!(fold.total_input_tokens, Some(1000));
    let entries = fold.take_sorted_entries();
    assert_eq!(entries.len(), 2, "rejected attempt is still billed");
    assert_eq!(UsageFold::total_cost_usd(&entries), Some(0.015));
}

/// Boundary regression (note9 warning 2, via the production fold):
/// provider A reports usage, then accepted provider B succeeds usage-less
/// (`input_tokens: None`). Identity and the null meter snapshot must
/// follow B while the billing ledger retains A's tokens.
#[test]
fn usage_fold_cross_provider_accepted_usageless_moves_snapshot_not_ledger() {
    let mut fold = UsageFold::default();
    fold.apply(usage_event(
        "openrouter.a",
        "model-a",
        Some(1000),
        None,
        Some(100),
        Some(0.02),
        true,
    ));
    fold.apply(usage_event(
        "ollama.b",
        "model-b",
        None,
        None,
        Some(50),
        None,
        true,
    ));

    // Identity follows the accepted usage-less B; the prompt-size snapshot
    // is cleared rather than going stale.
    assert_eq!(fold.last_provider_ref.as_deref(), Some("ollama.b"));
    assert_eq!(fold.last_model.as_deref(), Some("model-b"));
    assert_eq!(fold.last_input_tokens, None);
    // Billing keeps A's tokens plus B's output.
    assert_eq!(fold.total_input_tokens, Some(1000));
    assert_eq!(fold.total_output_tokens, Some(150));

    let entries = fold.take_sorted_entries();
    assert_eq!(entries.len(), 2);
    let a = entries.iter().find(|e| e.model == "model-a").unwrap();
    assert_eq!(
        (a.input_tokens, a.output_tokens, a.cost_usd),
        (1000, 100, 0.02)
    );

    // Wire contract with B's explicit window: meter ceiling resolves from
    // B, usage snapshot is null, ledger carries A's entry.
    let meta = DoneFrameMeta {
        full_response: "ok",
        input_tokens: Some(1000),
        output_tokens: Some(150),
        tokens_used: Some(1150),
        cost_usd: UsageFold::total_cost_usd(&entries),
        model: "model-b",
        provider: "ollama.b",
        provider_ref: "ollama.b",
        max_context_tokens: 800_000,
        model_context_window: Some(1_000_000),
        last_input_tokens: None,
        last_serving_provider_ref: Some("ollama.b"),
        last_serving_model: Some("model-b"),
    };
    let done = build_done_frame_json(&meta, &entries);
    let v: serde_json::Value = serde_json::from_str(&done.to_string()).unwrap();
    assert_eq!(v["model"], "model-b");
    assert_eq!(v["last_serving_model"], "model-b");
    assert_eq!(v["model_context_window"], 1_000_000);
    assert!(
        v["last_input_tokens"].is_null(),
        "last_input_tokens must be null when the accepted final call is usage-less"
    );
}

#[test]
fn done_frame_accepted_usageless_route_serializes_null_snapshot() {
    // Standalone wire-boundary pin for the accepted usage-less route,
    // independent of UsageFold: identity follows the serving route, the
    // snapshot serializes as explicit null, and the window stays omitted
    // when the provider configures none.
    let entries = vec![ProviderUsageEntry {
        provider_ref: "openrouter.a".to_string(),
        model: "model-a".to_string(),
        input_tokens: 1000,
        output_tokens: 100,
        cached_input_tokens: 0,
        cost_usd: 0.02,
    }];
    let meta = DoneFrameMeta {
        full_response: "ok",
        input_tokens: Some(1000),
        output_tokens: Some(100),
        tokens_used: Some(1100),
        cost_usd: UsageFold::total_cost_usd(&entries),
        model: "model-a",
        provider: "openrouter.a",
        provider_ref: "openrouter.a",
        max_context_tokens: 800_000,
        model_context_window: None,
        last_input_tokens: None,
        last_serving_provider_ref: Some("openrouter.a"),
        last_serving_model: Some("model-a"),
    };
    let done = build_done_frame_json(&meta, &entries);
    let v: serde_json::Value = serde_json::from_str(&done.to_string()).unwrap();
    assert!(v["last_input_tokens"].is_null());
    assert_eq!(v["last_serving_provider_ref"], "openrouter.a");
    assert_eq!(v["usage_by_provider"][0]["input_tokens"], 1000);
    assert!(
        v.get("model_context_window").is_none(),
        "model_context_window is additive and omitted when unset"
    );
}
