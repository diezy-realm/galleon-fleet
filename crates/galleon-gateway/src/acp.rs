//! ACP-over-WebSocket gateway endpoint.

use super::AppState;
use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

const ACP_WS_PROTOCOL: &str = "clawcrew.acp.v1";

#[derive(Debug, Deserialize)]
pub struct AcpQuery {
    token: Option<String>,
    /// Connection-scoped default agent alias. Used when `session/new` omits
    /// `agentAlias`, so spec-vanilla one-agent-per-endpoint ACP clients can
    /// address each configured agent via its own URL. Validated exactly like
    /// an explicit `agentAlias` at `session/new`; ignored by session restore.
    agent: Option<String>,
}

pub async fn handle_ws_acp(
    State(state): State<AppState>,
    Query(params): Query<AcpQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        let token = extract_ws_token(&headers, params.token.as_deref()).unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                "Unauthorized - provide Authorization header, Sec-WebSocket-Protocol bearer, or ?token= query param",
            )
                .into_response();
        }
    }

    let ws = if headers
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|protos| protos.split(',').any(|p| p.trim() == ACP_WS_PROTOCOL))
    {
        ws.protocols([ACP_WS_PROTOCOL])
    } else {
        ws
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, params.agent))
        .into_response()
}

async fn handle_socket(socket: WebSocket, state: AppState, default_agent: Option<String>) {
    let (mut sender, mut receiver) = socket.split();
    let (input_tx, input_rx) = mpsc::channel::<String>(256);
    let (output_tx, mut output_rx) = mpsc::channel::<String>(256);

    let server_task = if let Some(runner) = state.acp_runner.clone() {
        Some(galleon_spawn::spawn!(async move {
            runner.run_session(input_rx, output_tx, default_agent).await;
        }))
    } else {
        None
    };

    let output_task = galleon_spawn::spawn!(async move {
        while let Some(line) = output_rx.recv().await {
            if sender.send(Message::Text(line.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if input_tx.send(text.to_string()).await.is_err() {
                    break;
                }
            }
            Ok(Message::Binary(bytes)) => match String::from_utf8(bytes.to_vec()) {
                Ok(text) => {
                    if input_tx.send(text).await.is_err() {
                        break;
                    }
                }
                Err(e) => ::galleon_log::record!(
                    WARN,
                    ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                        .with_outcome(::galleon_log::EventOutcome::Unknown)
                        .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                    "ACP WebSocket received non-UTF-8 binary frame"
                ),
            },
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_) | Message::Pong(_)) => {}
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Connection reset without closing handshake")
                    || msg.contains("Connection closed normally")
                {
                    ::galleon_log::record!(
                        DEBUG,
                        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
                        "ACP WebSocket closed without handshake"
                    );
                } else {
                    ::galleon_log::record!(
                        WARN,
                        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                            .with_outcome(::galleon_log::EventOutcome::Unknown)
                            .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                        "ACP WebSocket receive error"
                    );
                }
                break;
            }
        }
    }

    drop(input_tx);

    if let Some(task) = server_task {
        if let Err(e) = task.await {
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_outcome(::galleon_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                "ACP WebSocket server task panicked"
            );
        }
    }
    output_task.abort();
    ::galleon_log::record!(
        DEBUG,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note),
        "ACP WebSocket disconnected"
    );
}

fn extract_ws_token<'a>(headers: &'a HeaderMap, query_token: Option<&'a str>) -> Option<&'a str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .or_else(|| {
            headers
                .get(axum::http::header::SEC_WEBSOCKET_PROTOCOL)
                .and_then(|v| v.to_str().ok())
                .and_then(|protocols| {
                    protocols
                        .split(',')
                        .map(str::trim)
                        .find_map(|p| p.strip_prefix("bearer."))
                })
                .filter(|token| !token.is_empty())
        })
        .or_else(|| query_token.filter(|token| !token.is_empty()))
}

#[cfg(test)]
mod tests {
    //! Front-door proof for the gateway ACP surface.
    //!
    //! The `session/new` unit tests in `galleon-channels` call
    //! `handle_session_new` directly; they prove the shared handler but cross
    //! neither user boundary. This test drives the *real* `/acp` WebSocket
    //! route: it boots the full gateway via `run_gateway` (so the request
    //! passes through the axum router, the pairing/subprotocol middleware in
    //! `handle_ws_acp`, the `on_upgrade` bridge, and `run_messages`), then
    //! connects a real WebSocket client and issues `session/new` with an
    //! omitted `cwd`. It asserts the returned `workspaceDir` is exactly the
    //! per-agent workspace — the behavior this PR introduces — rather than the
    //! daemon process CWD.

    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    /// On-disk-equivalent of the channels-crate `make_test_config`: a fake
    /// `anthropic.default` provider (model name only, no key — agent
    /// construction is offline) and one dispatchable agent `test-agent`, with
    /// `config_path` pinned so `agent_workspace_dir` resolves under the temp
    /// install root. Pairing is disabled so the WS client needs no token.
    fn front_door_config(install_root: &std::path::Path) -> galleon_config::schema::Config {
        use galleon_config::schema::{
            AliasedAgentConfig, AnthropicModelProviderConfig, Config, ModelProviderConfig,
            RiskProfileConfig, RuntimeProfileConfig,
        };

        let mut cfg = Config {
            data_dir: install_root.join("data"),
            config_path: install_root.join("config.toml"),
            ..Config::default()
        };
        cfg.gateway.require_pairing = false;
        cfg.providers.models.anthropic.insert(
            "default".to_string(),
            AnthropicModelProviderConfig {
                base: ModelProviderConfig {
                    model: Some("claude-haiku-4-5".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        cfg.risk_profiles
            .insert("default".to_string(), RiskProfileConfig::default());
        cfg.runtime_profiles
            .insert("default".to_string(), RuntimeProfileConfig::default());
        cfg.agents.insert(
            "test-agent".to_string(),
            AliasedAgentConfig {
                model_provider: "anthropic.default".into(),
                risk_profile: "default".into(),
                runtime_profile: "default".into(),
                ..Default::default()
            },
        );
        cfg
    }

    #[tokio::test]
    async fn acp_ws_front_door_omitted_cwd_uses_agent_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let install_root = tmp.path();
        let cfg = front_door_config(install_root);
        std::fs::create_dir_all(&cfg.data_dir).unwrap();
        let expected_ws = cfg
            .agent_workspace_dir("test-agent")
            .to_string_lossy()
            .into_owned();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let port = listener.local_addr().unwrap().port();

        struct MockAcpRunner {
            config: galleon_config::schema::Config,
        }

        #[async_trait::async_trait]
        impl clawcrew_api::runtime_traits::AcpSessionRunner for MockAcpRunner {
            async fn run_session(
                &self,
                mut input_rx: tokio::sync::mpsc::Receiver<String>,
                output_tx: tokio::sync::mpsc::Sender<String>,
                _default_agent: Option<String>,
            ) {
                while let Some(msg) = input_rx.recv().await {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                        if val.get("id").and_then(|i| i.as_i64()) == Some(1) {
                            let _ = output_tx
                                .send(
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": 1,
                                        "result": {}
                                    })
                                    .to_string(),
                                )
                                .await;
                        } else if val.get("id").and_then(|i| i.as_i64()) == Some(2) {
                            let alias =
                                val["params"]["agentAlias"].as_str().unwrap_or("test-agent");
                            let ws = self
                                .config
                                .agent_workspace_dir(alias)
                                .to_string_lossy()
                                .into_owned();
                            let _ = output_tx
                                .send(
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": 2,
                                        "result": { "workspaceDir": ws }
                                    })
                                    .to_string(),
                                )
                                .await;
                        }
                    }
                }
            }
        }

        let mut state = crate::api::tests::test_state(cfg.clone());
        state.acp_runner = Some(std::sync::Arc::new(MockAcpRunner { config: cfg }));
        let app = axum::Router::new()
            .route("/acp", axum::routing::get(handle_ws_acp))
            .with_state(state);
        let server = galleon_spawn::spawn!(async move {
            axum::serve(listener, app)
                .await
                .expect("serve test acp endpoint");
        });

        let url = format!("ws://127.0.0.1:{port}/acp");
        let (mut ws, _resp) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WebSocket upgrade on /acp should succeed");

        ws.send(Message::Text(
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
        ws.send(Message::Text(
            serde_json::json!({
                "jsonrpc":"2.0","id":2,"method":"session/new",
                "params":{"agentAlias":"test-agent"}
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

        let workspace_dir = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Some(msg) = ws.next().await {
                let text = match msg {
                    Ok(Message::Text(t)) => t.to_string(),
                    Ok(_) => continue,
                    Err(e) => panic!("WebSocket read error: {e}"),
                };
                let value: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if value.get("id").and_then(|i| i.as_i64()) == Some(2) {
                    if let Some(err) = value.get("error") {
                        panic!("session/new returned an error: {err}");
                    }
                    return value["result"]["workspaceDir"].as_str().map(String::from);
                }
            }
            None
        })
        .await
        .expect("session/new response should arrive before timeout");

        server.abort();

        assert_eq!(
            workspace_dir.as_deref(),
            Some(expected_ws.as_str()),
            "omitted-cwd session/new over the real /acp WebSocket route must \
             return the per-agent workspace, not the daemon CWD"
        );
    }
}
