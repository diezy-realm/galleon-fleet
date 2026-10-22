//! Strangler pattern boundary layer delegating Agent execution from the Axum gateway
//! to the Go AgentEngine service over gRPC/IPC (SPEC-006 / T3-05a).

use anyhow::{Context, Result};
use async_trait::async_trait;
use clawcrew_api::agent::{AgentRunner, AgentSession, AgentTurnError, AgentTurnSuccess, TurnEvent};
use clawcrew_api::model_provider::{ChatMessage, ConversationMessage};
use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;

use crate::grpc_system_gateway::agent_engine_client::AgentEngineClient;
use crate::grpc_system_gateway::turn_response::EventType;
use crate::grpc_system_gateway::{HealthCheckRequest, TurnRequest};

pub const DEFAULT_ENGINE_GRPC_ENDPOINT: &str = "http://127.0.0.1:50051";

/// Strangler Façade implementing `AgentRunner` to dispatch turns to Go `AgentEngine`.
#[derive(Clone, Debug)]
pub struct StranglerAgentRunner {
    endpoint: String,
}

impl Default for StranglerAgentRunner {
    fn default() -> Self {
        Self::new(DEFAULT_ENGINE_GRPC_ENDPOINT)
    }
}

impl StranglerAgentRunner {
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    /// Connect to the Go AgentEngine gRPC service.
    pub async fn connect(&self) -> Result<AgentEngineClient<Channel>> {
        AgentEngineClient::connect(self.endpoint.clone())
            .await
            .with_context(|| format!("failed to connect to AgentEngine at {}", self.endpoint))
    }

    /// Check if Go AgentEngine is serving.
    pub async fn check_health(&self) -> Result<bool> {
        let mut client = self.connect().await?;
        let resp = client
            .health_check(tonic::Request::new(HealthCheckRequest {
                service: "clawcrew.agent.AgentEngine".to_string(),
            }))
            .await?;
        Ok(resp.into_inner().status == 1) // SERVING = 1
    }
}

#[async_trait]
impl AgentRunner for StranglerAgentRunner {
    async fn create_session(
        &self,
        _config: Box<dyn Any + Send + Sync>,
        agent_alias: &str,
        session_cwd: Option<&Path>,
        _canvas_store: Option<Box<dyn Any + Send + Sync>>,
    ) -> Result<Box<dyn AgentSession>> {
        let session = StranglerAgentSession::new(
            self.endpoint.clone(),
            agent_alias.to_string(),
            session_cwd.map(Path::to_path_buf),
        );
        Ok(Box::new(session))
    }

    async fn process_message(
        &self,
        _config: Box<dyn Any + Send>,
        agent_alias: &str,
        message: &str,
        session_id: Option<&str>,
        _origin: clawcrew_api::ingress::TurnOrigin,
    ) -> Result<String> {
        let mut client = self.connect().await?;
        let req = TurnRequest {
            session_id: session_id.unwrap_or("default").to_string(),
            agent_id: agent_alias.to_string(),
            prompt: message.to_string(),
            context_window: Vec::new(),
        };

        let mut stream = client
            .start_turn(tonic::Request::new(req))
            .await
            .context("failed to start turn on AgentEngine")?
            .into_inner();

        let mut accumulated_response = String::new();
        while let Some(resp) = stream
            .message()
            .await
            .context("error reading turn stream")?
        {
            if resp.r#type == EventType::TextChunk as i32 {
                accumulated_response.push_str(&resp.content);
            } else if resp.r#type == EventType::Error as i32 {
                let err_msg = if !resp.error_message.is_empty() {
                    resp.error_message
                } else {
                    resp.content
                };
                anyhow::bail!("AgentEngine error: {err_msg}");
            }
        }

        Ok(accumulated_response)
    }
}

/// Install the strangler runner into the global agent runner registry.
pub fn install_strangler_agent_runner(endpoint: Option<String>) {
    let runner = match endpoint {
        Some(ep) => StranglerAgentRunner::new(ep),
        None => StranglerAgentRunner::default(),
    };
    clawcrew_api::agent::set_agent_runner(Arc::new(runner));
}

/// Stateful gRPC session proxying turns to Go `AgentEngine`.
pub struct StranglerAgentSession {
    endpoint: String,
    agent_alias: String,
    pub session_cwd: Option<PathBuf>,
    channel_name: String,
    memory_session_id: Option<String>,
    history_has_trim_breadcrumb: bool,
    history: Vec<ConversationMessage>,
    channel_handles: clawcrew_tools::send_via::AgentChannelHandles,
}

impl StranglerAgentSession {
    #[must_use]
    pub fn new(endpoint: String, agent_alias: String, session_cwd: Option<PathBuf>) -> Self {
        Self {
            endpoint,
            agent_alias,
            session_cwd,
            channel_name: "wss".to_string(),
            memory_session_id: None,
            history_has_trim_breadcrumb: false,
            history: Vec::new(),
            channel_handles: clawcrew_tools::send_via::AgentChannelHandles::default(),
        }
    }
}

#[async_trait]
impl AgentSession for StranglerAgentSession {
    fn set_channel_name(&mut self, name: String) {
        self.channel_name = name;
    }

    fn set_memory_session_id(&mut self, session_id: Option<String>) {
        self.memory_session_id = session_id;
    }

    fn set_history_has_trim_breadcrumb(&mut self, flag: bool) {
        self.history_has_trim_breadcrumb = flag;
    }

    fn history_has_trim_breadcrumb(&self) -> bool {
        self.history_has_trim_breadcrumb
    }

    fn seed_history_with_event(&mut self, messages: &[ChatMessage]) -> Option<TurnEvent> {
        for msg in messages {
            self.history.push(ConversationMessage::Chat(msg.clone()));
        }
        None
    }

    fn register_channel(&self, name: &str, channel: Arc<dyn clawcrew_api::channel::Channel>) {
        self.channel_handles.register_channel(name, channel);
    }

    fn channel_handles_any(&self) -> &dyn Any {
        &self.channel_handles
    }

    fn attribution_fields(&self) -> (String, String, String) {
        (
            "go-engine".to_string(),
            "agent-engine".to_string(),
            self.channel_name.clone(),
        )
    }

    fn history(&self) -> &[ConversationMessage] {
        &self.history
    }

    fn context_limits_for_route_raw(
        &self,
        _provider_name: &str,
        _model: &str,
    ) -> (Option<u64>, Option<u64>) {
        (None, None)
    }

    async fn turn_streamed_with_steering_state(
        &mut self,
        user_message: &str,
        event_tx: mpsc::Sender<TurnEvent>,
        cancel_token: Option<CancellationToken>,
        _steering_rx: Option<&mut mpsc::Receiver<String>>,
    ) -> std::result::Result<AgentTurnSuccess, AgentTurnError> {
        if cancel_token.as_ref().is_some_and(|t| t.is_cancelled()) {
            return Err(AgentTurnError {
                error: anyhow::anyhow!("Turn execution cancelled"),
                committed_response: String::new(),
                new_messages: Vec::new(),
            });
        }

        let mut client = match AgentEngineClient::connect(self.endpoint.clone()).await {
            Ok(c) => c,
            Err(e) => {
                return Err(AgentTurnError {
                    error: anyhow::anyhow!(
                        "Failed to connect to AgentEngine at {}: {e}",
                        self.endpoint
                    ),
                    committed_response: String::new(),
                    new_messages: Vec::new(),
                });
            }
        };

        let session_id = self
            .memory_session_id
            .clone()
            .unwrap_or_else(|| "default-session".to_string());

        let req = TurnRequest {
            session_id,
            agent_id: self.agent_alias.clone(),
            prompt: user_message.to_string(),
            context_window: Vec::new(),
        };

        let mut stream = match client.start_turn(tonic::Request::new(req)).await {
            Ok(resp) => resp.into_inner(),
            Err(status) => {
                return Err(AgentTurnError {
                    error: anyhow::anyhow!("AgentEngine start_turn RPC failed: {status}"),
                    committed_response: String::new(),
                    new_messages: Vec::new(),
                });
            }
        };

        let mut accumulated_response = String::new();

        loop {
            tokio::select! {
                biased;
                _ = async {
                    if let Some(token) = cancel_token.as_ref() {
                        token.cancelled().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {
                    return Err(AgentTurnError {
                        error: anyhow::anyhow!("Turn execution cancelled"),
                        committed_response: accumulated_response,
                        new_messages: Vec::new(),
                    });
                }
                item = stream.message() => {
                    match item {
                        Ok(Some(resp)) => {
                            if resp.r#type == EventType::ThoughtChunk as i32 {
                                let _ = event_tx.send(TurnEvent::Thinking { delta: resp.content }).await;
                            } else if resp.r#type == EventType::TextChunk as i32 {
                                accumulated_response.push_str(&resp.content);
                                let _ = event_tx.send(TurnEvent::Chunk { delta: resp.content }).await;
                            } else if resp.r#type == EventType::ToolCallStarted as i32 {
                                let (id, name, args) = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp.content) {
                                    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or(&resp.subagent_id).to_string();
                                    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("tool").to_string();
                                    let args = val.get("args").cloned().unwrap_or(serde_json::Value::Null);
                                    (id, name, args)
                                } else {
                                    (resp.subagent_id.clone(), resp.content.clone(), serde_json::Value::Null)
                                };
                                let _ = event_tx.send(TurnEvent::ToolCall { id, name, args }).await;
                            } else if resp.r#type == EventType::ToolCallFinished as i32 {
                                let (id, name, output) = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp.content) {
                                    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or(&resp.subagent_id).to_string();
                                    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("tool").to_string();
                                    let output = val.get("output").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    (id, name, output)
                                } else {
                                    (resp.subagent_id.clone(), "tool".to_string(), resp.content.clone())
                                };
                                let _ = event_tx.send(TurnEvent::ToolResult { id, name, output, artifact: None }).await;
                            } else if resp.r#type == EventType::TurnCompleted as i32 {
                                break;
                            } else if resp.r#type == EventType::Error as i32 {
                                let err_msg = if !resp.error_message.is_empty() {
                                    resp.error_message
                                } else {
                                    resp.content
                                };
                                return Err(AgentTurnError {
                                    error: anyhow::anyhow!("AgentEngine error event: {err_msg}"),
                                    committed_response: accumulated_response,
                                    new_messages: Vec::new(),
                                });
                            }
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(status) => {
                            return Err(AgentTurnError {
                                error: anyhow::anyhow!("gRPC streaming error: {status}"),
                                committed_response: accumulated_response,
                                new_messages: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        let user_msg = ConversationMessage::Chat(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });
        let assistant_msg = ConversationMessage::Chat(ChatMessage {
            role: "assistant".to_string(),
            content: accumulated_response.clone(),
        });

        self.history.push(user_msg.clone());
        self.history.push(assistant_msg.clone());

        Ok(AgentTurnSuccess {
            response: accumulated_response,
            new_messages: vec![user_msg, assistant_msg],
            provider_name: "go-engine".to_string(),
            model: "agent-engine".to_string(),
            final_context_limits: None,
            safeguard_fallback: None,
        })
    }
}

/// Request payload for HTTP `/api/turn` route
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ApiTurnPayload {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub context_window: Vec<u8>,
}

/// Dispatches `/api/turn` HTTP requests by streaming events from the Go AgentEngine.
pub async fn handle_api_turn(
    endpoint: Option<&str>,
    payload: ApiTurnPayload,
) -> Result<axum::response::Response, (axum::http::StatusCode, String)> {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use futures_util::StreamExt;

    let target_endpoint = endpoint.unwrap_or(DEFAULT_ENGINE_GRPC_ENDPOINT);
    let mut client = match AgentEngineClient::connect(target_endpoint.to_string()).await {
        Ok(c) => c,
        Err(e) => {
            return Err((
                StatusCode::BAD_GATEWAY,
                format!("failed to connect to Go AgentEngine: {e}"),
            ));
        }
    };

    let req = TurnRequest {
        session_id: payload.session_id,
        agent_id: payload.agent_id,
        prompt: payload.prompt,
        context_window: payload.context_window,
    };

    let stream = match client.start_turn(tonic::Request::new(req)).await {
        Ok(resp) => resp.into_inner(),
        Err(status) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("AgentEngine start_turn error: {status}"),
            ));
        }
    };

    let event_stream = stream.map(|item| match item {
        Ok(resp) => {
            let data = serde_json::json!({
                "type": resp.r#type,
                "content": resp.content,
                "subagent_id": resp.subagent_id,
                "error_message": resp.error_message,
            });
            Ok::<_, std::convert::Infallible>(
                axum::response::sse::Event::default().data(data.to_string()),
            )
        }
        Err(status) => {
            let err_data = serde_json::json!({
                "error": status.message(),
            });
            Ok(axum::response::sse::Event::default()
                .event("error")
                .data(err_data.to_string()))
        }
    });

    Ok(axum::response::sse::Sse::new(event_stream).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc_system_gateway::agent_engine_server::{AgentEngine, AgentEngineServer};
    use crate::grpc_system_gateway::{
        HealthCheckResponse, QueryRequest, QueryResponse, TurnResponse,
    };
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;
    use tonic::{Request, Response, Status};

    struct MockAgentEngine {
        fail_stream: bool,
    }

    #[tonic::async_trait]
    impl AgentEngine for MockAgentEngine {
        type StartTurnStream = ReceiverStream<Result<TurnResponse, Status>>;

        async fn start_turn(
            &self,
            request: Request<TurnRequest>,
        ) -> Result<Response<Self::StartTurnStream>, Status> {
            let req = request.into_inner();
            let (tx, rx) = mpsc::channel(16);

            let fail = self.fail_stream;
            tokio::spawn(async move {
                if fail {
                    let _ = tx
                        .send(Ok(TurnResponse {
                            r#type: EventType::Error as i32,
                            content: "engine execution failure".to_string(),
                            subagent_id: String::new(),
                            error_message: "engine execution failure".to_string(),
                        }))
                        .await;
                    return;
                }

                // 1. Thought
                let _ = tx
                    .send(Ok(TurnResponse {
                        r#type: EventType::ThoughtChunk as i32,
                        content: format!("Analyzing prompt: {}", req.prompt),
                        subagent_id: String::new(),
                        error_message: String::new(),
                    }))
                    .await;

                // 2. Text chunks
                let _ = tx
                    .send(Ok(TurnResponse {
                        r#type: EventType::TextChunk as i32,
                        content: "Hello from ".to_string(),
                        subagent_id: String::new(),
                        error_message: String::new(),
                    }))
                    .await;
                let _ = tx
                    .send(Ok(TurnResponse {
                        r#type: EventType::TextChunk as i32,
                        content: "Go Engine!".to_string(),
                        subagent_id: String::new(),
                        error_message: String::new(),
                    }))
                    .await;

                // 3. Tool call & result
                let _ = tx
                    .send(Ok(TurnResponse {
                        r#type: EventType::ToolCallStarted as i32,
                        content: serde_json::json!({
                            "id": "call-1",
                            "name": "read_file",
                            "args": {"path": "main.go"}
                        })
                        .to_string(),
                        subagent_id: "sub-1".to_string(),
                        error_message: String::new(),
                    }))
                    .await;

                let _ = tx
                    .send(Ok(TurnResponse {
                        r#type: EventType::ToolCallFinished as i32,
                        content: serde_json::json!({
                            "id": "call-1",
                            "name": "read_file",
                            "output": "package main"
                        })
                        .to_string(),
                        subagent_id: "sub-1".to_string(),
                        error_message: String::new(),
                    }))
                    .await;

                // 4. Completed
                let _ = tx
                    .send(Ok(TurnResponse {
                        r#type: EventType::TurnCompleted as i32,
                        content: String::new(),
                        subagent_id: String::new(),
                        error_message: String::new(),
                    }))
                    .await;
            });

            Ok(Response::new(ReceiverStream::new(rx)))
        }

        async fn quick_query(
            &self,
            _request: Request<QueryRequest>,
        ) -> Result<Response<QueryResponse>, Status> {
            Ok(Response::new(QueryResponse { matches: vec![] }))
        }

        async fn health_check(
            &self,
            _request: Request<HealthCheckRequest>,
        ) -> Result<Response<HealthCheckResponse>, Status> {
            Ok(Response::new(HealthCheckResponse { status: 1 }))
        }
    }

    async fn start_mock_engine(fail_stream: bool) -> (String, tokio::sync::oneshot::Sender<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let service = MockAgentEngine { fail_stream };

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(AgentEngineServer::new(service))
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = shutdown_rx.await;
                    },
                )
                .await
                .unwrap();
        });

        (format!("http://{addr}"), shutdown_tx)
    }

    #[tokio::test]
    async fn test_strangler_agent_runner_health_and_process_message() {
        let (endpoint, shutdown_tx) = start_mock_engine(false).await;
        let runner = StranglerAgentRunner::new(endpoint);

        let healthy = runner.check_health().await.unwrap();
        assert!(healthy, "mock Go engine should be healthy");

        let response = runner
            .process_message(
                Box::new(()),
                "test-agent",
                "Hi engine",
                Some("sess-1"),
                clawcrew_api::ingress::TurnOrigin::Interactive,
            )
            .await
            .unwrap();

        assert_eq!(response, "Hello from Go Engine!");
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_strangler_agent_session_turn_streamed() {
        let (endpoint, shutdown_tx) = start_mock_engine(false).await;
        let runner = StranglerAgentRunner::new(endpoint);

        let mut session = runner
            .create_session(Box::new(()), "code-agent", None, None)
            .await
            .unwrap();

        let (event_tx, mut event_rx) = mpsc::channel(32);

        let outcome = session
            .turn_streamed_with_steering_state("ping prompt", event_tx, None, None)
            .await
            .expect("turn should succeed");

        assert_eq!(outcome.response, "Hello from Go Engine!");
        assert_eq!(outcome.provider_name, "go-engine");
        assert_eq!(outcome.model, "agent-engine");
        assert_eq!(outcome.new_messages.len(), 2);

        // Verify streamed events were received in order
        let mut events = Vec::new();
        while let Ok(evt) = event_rx.try_recv() {
            events.push(evt);
        }

        assert!(
            events
                .iter()
                .any(|e| matches!(e, TurnEvent::Thinking { .. }))
        );
        assert!(events.iter().any(|e| matches!(e, TurnEvent::Chunk { .. })));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TurnEvent::ToolCall { .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TurnEvent::ToolResult { .. }))
        );

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_strangler_agent_session_error_handling() {
        let (endpoint, shutdown_tx) = start_mock_engine(true).await;
        let runner = StranglerAgentRunner::new(endpoint);

        let mut session = runner
            .create_session(Box::new(()), "code-agent", None, None)
            .await
            .unwrap();

        let (event_tx, _event_rx) = mpsc::channel(32);

        let err = session
            .turn_streamed_with_steering_state("failing prompt", event_tx, None, None)
            .await
            .expect_err("turn should fail when engine emits error");

        assert!(err.error.to_string().contains("engine execution failure"));
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_strangler_agent_session_cancellation() {
        let (endpoint, shutdown_tx) = start_mock_engine(false).await;
        let runner = StranglerAgentRunner::new(endpoint);

        let mut session = runner
            .create_session(Box::new(()), "code-agent", None, None)
            .await
            .unwrap();

        let cancel_token = CancellationToken::new();
        cancel_token.cancel(); // Pre-cancelled

        let (event_tx, _event_rx) = mpsc::channel(32);

        let err = session
            .turn_streamed_with_steering_state(
                "cancelled prompt",
                event_tx,
                Some(cancel_token),
                None,
            )
            .await
            .expect_err("turn should be cancelled");

        assert!(err.error.to_string().contains("cancelled"));
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_handle_api_turn_sse() {
        let (endpoint, shutdown_tx) = start_mock_engine(false).await;

        let payload = ApiTurnPayload {
            session_id: "s1".to_string(),
            agent_id: "a1".to_string(),
            prompt: "hello".to_string(),
            context_window: Vec::new(),
        };

        let response = handle_api_turn(Some(&endpoint), payload).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let _ = shutdown_tx.send(());
    }
}
