//! HTTP handlers for Galleon Fleet Quartermaster endpoint (`POST /api/chat/quartermaster`).
//!
//! Under SPEC-013 §Quartermaster (TASK-080):
//! - `POST /api/chat/quartermaster`:
//!   - `require_auth` Bearer token check.
//!   - Evaluates message via `galleon_fleet_core::intent::classify`.
//!   - `Intent::Report` -> deterministic reply from real fleet metrics via `answer_report`.
//!   - `Intent::EngineRoom` -> deterministic reply from real telemetry & diagnostics via `answer_engine_room`.
//!   - All other intents (including `Intent::Chat` and `Intent::Objective`) are forwarded to
//!     `galleon_api::agent::create_agent_session` + `turn_streamed_with_steering_state`.
//!   - `'objective'` is treated as regular chat without claiming a draft without artifacts (fixing F-13/F-17).
//!   - Agent alias resolution:
//!     1. Explicit `agent_alias` field in request (if configured; if unconfigured -> 422).
//!     2. `[agents.quartermaster]` if configured in config.
//!     3. Sole agent alias if exactly one agent is configured.
//!     4. Otherwise 422 Unprocessable Entity with available aliases list.
//!   - Provider failure returns 502 Bad Gateway with honest error message.
//!   - Zero unwrap/expect in production code paths.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use galleon_config::schema::Config;
use galleon_fleet_core::{
    answer_engine_room, answer_report, classify, DiagnosticItem, FleetStore, Intent,
    QuartermasterChatResponse, FLEET_MAX_BODY,
};
use serde::Deserialize;
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Request payload for `POST /api/chat/quartermaster`.
#[derive(Debug, Deserialize)]
pub struct QuartermasterChatRequest {
    /// The user message or prompt. Accepts alias `content` or `prompt` for flexibility.
    #[serde(alias = "content", alias = "prompt")]
    pub message: String,
    /// Optional agent alias override. If omitted, defaults to `quartermaster` or sole agent.
    #[serde(default)]
    pub agent_alias: Option<String>,
    /// Optional context object sent by frontend clients.
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

/// Resolves the effective agent alias for Quartermaster execution.
///
/// Under SPEC-013 §Quartermaster:
/// 1. If explicit `agent_alias` is supplied in the request:
///    - If found in `config.agents`, use it.
///    - If not found, returns 422 with available aliases.
/// 2. If `[agents.quartermaster]` is configured, use `"quartermaster"`.
/// 3. If exactly one agent is configured in `config.agents`, use that sole agent.
/// 4. Otherwise, returns 422 with the list of available aliases.
pub fn resolve_quartermaster_agent_alias(
    config: &Config,
    requested_alias: Option<&str>,
) -> Result<String, Response> {
    if let Some(alias) = requested_alias.map(str::trim).filter(|s| !s.is_empty()) {
        if config.agents.contains_key(alias) {
            return Ok(alias.to_string());
        }
        let mut available: Vec<String> = config.agents.keys().cloned().collect();
        available.sort();
        let err = json!({
            "error": format!("Requested agent `{alias}` is not configured"),
            "available_aliases": available,
        });
        return Err((StatusCode::UNPROCESSABLE_ENTITY, axum::Json(err)).into_response());
    }

    if config.agents.contains_key("quartermaster") {
        return Ok("quartermaster".to_string());
    }

    if config.agents.len() == 1 {
        if let Some(sole_alias) = config.agents.keys().next() {
            return Ok(sole_alias.clone());
        }
    }

    let mut available: Vec<String> = config.agents.keys().cloned().collect();
    available.sort();
    let err = json!({
        "error": "No agent_alias specified and could not deduce default agent. Configure [agents.quartermaster], define exactly one agent, or specify agent_alias in the request.",
        "available_aliases": available,
    });
    Err((StatusCode::UNPROCESSABLE_ENTITY, axum::Json(err)).into_response())
}

/// Helper to obtain RSS memory usage for honest telemetry metrics.
fn current_memory_used_mb() -> Option<f64> {
    let stats = galleon_infra::process_stats::sample();
    if stats.rss_bytes > 0 {
        Some(stats.rss_bytes as f64 / (1024.0 * 1024.0))
    } else {
        None
    }
}

/// Assembles quartermaster routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/chat/quartermaster",
            post(handle_chat_quartermaster_state),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing Quartermaster endpoints with dedicated AppState.
pub fn router_for_state(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/chat/quartermaster",
            post(handle_chat_quartermaster_state),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

async fn handle_chat_quartermaster_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(payload): axum::Json<QuartermasterChatRequest>,
) -> Response {
    handle_chat_quartermaster_impl(&state, &headers, payload).await
}

/// Internal implementation of `POST /api/chat/quartermaster`.
pub async fn handle_chat_quartermaster_impl(
    state: &AppState,
    headers: &HeaderMap,
    payload: QuartermasterChatRequest,
) -> Response {
    // 1. Mandatory authentication check
    if let Err(auth_err) = crate::api::require_auth(state, headers) {
        return auth_err.into_response();
    }

    let trimmed_message = payload.message.trim();
    if trimmed_message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({ "error": "message cannot be empty" })),
        )
            .into_response();
    }

    // 2. Classify intent via 1:1 regex layer
    let intent = classify(trimmed_message);

    match intent {
        Intent::Report => {
            let store = {
                let config = state.config.read();
                FleetStore::from_data_dir(&config.data_dir)
            };
            let memory_used_mb = current_memory_used_mb();
            let metrics = store.get_metrics(memory_used_mb);

            let uptime_seconds = galleon_infra::health::snapshot().uptime_seconds;
            let hours = uptime_seconds / 3600;
            let minutes = (uptime_seconds % 3600) / 60;
            let uptime = format!("{hours}h {minutes}m");

            let resp = answer_report(&metrics, Some(&uptime));
            (StatusCode::OK, axum::Json(resp)).into_response()
        }
        Intent::EngineRoom => {
            let store = {
                let config = state.config.read();
                FleetStore::from_data_dir(&config.data_dir)
            };
            let memory_used_mb = current_memory_used_mb();
            let metrics = store.get_metrics(memory_used_mb);

            let results = if let Some(diag) = &state.diagnostics {
                diag.run_diagnostics()
            } else {
                let config = state.config.read();
                galleon_runtime::doctor::diagnose(&config)
            };
            let diag_items: Vec<DiagnosticItem> = results
                .iter()
                .enumerate()
                .map(|(idx, res)| crate::api_fleet::diagnostics::map_diag_result_to_item(idx, res))
                .collect();

            let resp = answer_engine_room(&metrics, &diag_items);
            (StatusCode::OK, axum::Json(resp)).into_response()
        }
        _ => {
            // All other intents (Chat, Objective, etc.) are forwarded to agent
            // session. 'objective' is treated as regular chat without claiming a draft.
            let agent_alias = {
                let config = state.config.read();
                resolve_quartermaster_agent_alias(&config, payload.agent_alias.as_deref())
            };

            let agent_alias = match agent_alias {
                Ok(alias) => alias,
                Err(resp) => return resp,
            };

            let session_res = galleon_api::agent::create_agent_session(
                Arc::clone(&state.config),
                &agent_alias,
                None,
                Some(state.canvas_store.clone()),
            )
            .await;

            let mut agent = match session_res {
                Ok(agent) => agent,
                Err(err) => {
                    ::galleon_log::record!(
                        ERROR,
                        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
                            .with_outcome(::galleon_log::EventOutcome::Failure)
                            .with_attrs(json!({
                                "agent": &agent_alias,
                                "error": format!("{err:#}")
                            })),
                        "Quartermaster failed to create agent session"
                    );
                    let err_json = json!({
                        "error": format!("Failed to create agent session: {err:#}")
                    });
                    return (StatusCode::BAD_GATEWAY, axum::Json(err_json)).into_response();
                }
            };

            agent.set_channel_name("quartermaster".to_string());

            let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(64);
            let drain_handle = ::galleon_spawn::spawn!(async move {
                let mut accumulated = String::new();
                while let Some(event) = event_rx.recv().await {
                    if let galleon_api::agent::TurnEvent::Chunk { delta } = event {
                        accumulated.push_str(&delta);
                    }
                }
                accumulated
            });

            let turn_result = agent
                .turn_streamed_with_steering_state(trimmed_message, event_tx, None, None)
                .await;

            let streamed_text = drain_handle.await.unwrap_or_default();

            match turn_result {
                Ok(turn_success) => {
                    let reply = if !turn_success.response.is_empty() {
                        turn_success.response
                    } else {
                        streamed_text
                    };
                    let response = QuartermasterChatResponse::new(reply);
                    (StatusCode::OK, axum::Json(response)).into_response()
                }
                Err(turn_err) => {
                    ::galleon_log::record!(
                        ERROR,
                        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Fail)
                            .with_outcome(::galleon_log::EventOutcome::Failure)
                            .with_attrs(json!({
                                "agent": &agent_alias,
                                "error": format!("{:#}", turn_err.error)
                            })),
                        "Quartermaster agent turn execution failed"
                    );
                    let err_json = json!({
                        "error": format!("Agent turn failed: {:#}", turn_err.error)
                    });
                    (StatusCode::BAD_GATEWAY, axum::Json(err_json)).into_response()
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request};
    use galleon_api::agent::{
        set_agent_runner, AgentRunner, AgentSession, AgentTurnError, AgentTurnSuccess, TurnEvent,
    };
    use galleon_config::pairing::{PairingCodePolicy, PairingGuard};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    static AGENT_TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    struct MockQuartermasterSession {
        reply: String,
        fail_turn: bool,
    }

    #[async_trait::async_trait]
    impl AgentSession for MockQuartermasterSession {
        fn set_channel_name(&mut self, _name: String) {}
        fn set_memory_session_id(&mut self, _session_id: Option<String>) {}
        fn set_history_has_trim_breadcrumb(&mut self, _flag: bool) {}
        fn history_has_trim_breadcrumb(&self) -> bool {
            false
        }
        fn seed_history_with_event(
            &mut self,
            _messages: &[galleon_api::model_provider::ChatMessage],
        ) -> Option<TurnEvent> {
            None
        }
        fn register_channel(
            &self,
            _name: &str,
            _channel: std::sync::Arc<dyn galleon_api::channel::Channel>,
        ) {
        }
        fn channel_handles_any(&self) -> &dyn std::any::Any {
            self
        }
        fn attribution_fields(&self) -> (String, String, String) {
            (
                "test_provider".into(),
                "test_model".into(),
                "test_agent".into(),
            )
        }
        fn history(&self) -> &[galleon_api::model_provider::ConversationMessage] {
            &[]
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
            _user_message: &str,
            event_tx: tokio::sync::mpsc::Sender<TurnEvent>,
            _cancel_token: Option<tokio_util::sync::CancellationToken>,
            _steering_rx: Option<&mut tokio::sync::mpsc::Receiver<String>>,
        ) -> std::result::Result<AgentTurnSuccess, AgentTurnError> {
            if self.fail_turn {
                return Err(AgentTurnError {
                    error: anyhow::Error::msg("simulated model execution failure"),
                    committed_response: String::new(),
                    new_messages: vec![],
                });
            }
            let _ = event_tx
                .send(TurnEvent::Chunk {
                    delta: self.reply.clone(),
                })
                .await;
            Ok(AgentTurnSuccess {
                response: self.reply.clone(),
                new_messages: vec![],
                provider_name: "test_provider".into(),
                model: "test_model".into(),
                final_context_limits: None,
                safeguard_fallback: None,
            })
        }
    }

    struct MockQuartermasterRunner {
        simulated_reply: String,
        fail_create: bool,
        fail_turn: bool,
    }

    #[async_trait::async_trait]
    impl AgentRunner for MockQuartermasterRunner {
        async fn create_session(
            &self,
            _config: Box<dyn std::any::Any + Send + Sync>,
            _agent_alias: &str,
            _session_cwd: Option<&std::path::Path>,
            _canvas_store: Option<Box<dyn std::any::Any + Send + Sync>>,
        ) -> anyhow::Result<Box<dyn AgentSession>> {
            if self.fail_create {
                anyhow::bail!("simulated runner create_session failure");
            }
            Ok(Box::new(MockQuartermasterSession {
                reply: self.simulated_reply.clone(),
                fail_turn: self.fail_turn,
            }))
        }

        async fn process_message(
            &self,
            _config: Box<dyn std::any::Any + Send>,
            _agent_alias: &str,
            _message: &str,
            _session_id: Option<&str>,
            _origin: galleon_api::ingress::TurnOrigin,
        ) -> anyhow::Result<String> {
            if self.fail_create || self.fail_turn {
                anyhow::bail!("mock failure");
            }
            Ok(self.simulated_reply.clone())
        }
    }

    fn create_test_state() -> AppState {
        let mut config = Config::default();
        config.agents.insert(
            "quartermaster".to_string(),
            galleon_config::schema::AliasedAgentConfig::default(),
        );
        crate::api::tests::test_state(config)
    }

    fn create_test_state_with_auth(valid_token: &str) -> AppState {
        let mut config = Config::default();
        config.agents.insert(
            "quartermaster".to_string(),
            galleon_config::schema::AliasedAgentConfig::default(),
        );
        let mut state = crate::api::tests::test_state(config);
        state.pairing = Arc::new(PairingGuard::new(
            true,
            &[valid_token.to_string()],
            PairingCodePolicy::default(),
        ));
        state
    }

    #[tokio::test]
    async fn test_quartermaster_unauthorized_without_token() {
        let state = create_test_state_with_auth("secret-token");
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "message": "report status" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_quartermaster_empty_message_returns_bad_request() {
        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "message": "   " }).to_string()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(parsed["error"], "message cannot be empty");
    }

    #[tokio::test]
    async fn test_quartermaster_report_intent_deterministic_reply() {
        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "message": "report status" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        let reply = parsed["reply"].as_str().unwrap();
        assert!(reply.starts_with("Fleet Report:"));
        assert!(parsed["suggestedActions"].is_array());
        assert_eq!(parsed["suggestedActions"][0]["label"], "Open Mission Board");
    }

    #[tokio::test]
    async fn test_quartermaster_engine_room_intent_deterministic_reply() {
        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "message": "check cpu and memory telemetry" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        let reply = parsed["reply"].as_str().unwrap();
        assert!(reply.starts_with("Engine Room:"));
        assert!(parsed["suggestedActions"].is_array());
        assert_eq!(parsed["suggestedActions"][0]["label"], "View Crow's Nest");
    }

    #[tokio::test]
    async fn test_quartermaster_chat_intent_forwarded_to_agent() {
        let _guard = AGENT_TEST_MUTEX.lock().await;

        let mock_runner = Arc::new(MockQuartermasterRunner {
            simulated_reply: "Quartermaster at your service, Captain!".to_string(),
            fail_create: false,
            fail_turn: false,
        });
        set_agent_runner(mock_runner);

        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "message": "Hello Quartermaster, what is your role?" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(parsed["reply"], "Quartermaster at your service, Captain!");
    }

    #[tokio::test]
    async fn test_quartermaster_objective_intent_treated_as_regular_chat_no_fake_draft() {
        let _guard = AGENT_TEST_MUTEX.lock().await;

        let mock_runner = Arc::new(MockQuartermasterRunner {
            simulated_reply: "I am ready to plan this quest with the crew.".to_string(),
            fail_create: false,
            fail_turn: false,
        });
        set_agent_runner(mock_runner);

        let state = create_test_state();
        let app = router_for_state(state);

        // Word 'prepare' or 'plan' triggers Intent::Objective under regex layer
        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "message": "prepare mission for navigation overhaul" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        let reply = parsed["reply"].as_str().unwrap();

        // Acceptance criterion: No 'I will draft a Fleet Order' response without artifacts
        assert!(
            !reply.contains("I will draft a Fleet Order"),
            "Must NOT return legacy fake draft claim"
        );
        assert_eq!(reply, "I am ready to plan this quest with the crew.");
    }

    #[tokio::test]
    async fn test_quartermaster_agent_alias_resolution_and_validation() {
        let mut config = Config::default();

        // 1. Empty config with no alias -> 422
        let res_empty = resolve_quartermaster_agent_alias(&config, None);
        assert!(res_empty.is_err());

        // 2. Sole agent configured -> resolves to that agent
        config.agents.insert(
            "lone-navigator".to_string(),
            galleon_config::schema::AliasedAgentConfig::default(),
        );
        let res_sole = resolve_quartermaster_agent_alias(&config, None);
        assert_eq!(res_sole.unwrap(), "lone-navigator");

        // 3. Multiple agents with quartermaster -> resolves to quartermaster
        config.agents.insert(
            "quartermaster".to_string(),
            galleon_config::schema::AliasedAgentConfig::default(),
        );
        let res_qm = resolve_quartermaster_agent_alias(&config, None);
        assert_eq!(res_qm.unwrap(), "quartermaster");

        // 4. Explicit valid agent requested -> resolves to requested
        let res_req = resolve_quartermaster_agent_alias(&config, Some("lone-navigator"));
        assert_eq!(res_req.unwrap(), "lone-navigator");

        // 5. Explicit unconfigured agent requested -> 422
        let res_unknown = resolve_quartermaster_agent_alias(&config, Some("phantom"));
        assert!(res_unknown.is_err());
    }

    #[tokio::test]
    async fn test_quartermaster_provider_failure_returns_502() {
        let _guard = AGENT_TEST_MUTEX.lock().await;

        let mock_runner = Arc::new(MockQuartermasterRunner {
            simulated_reply: String::new(),
            fail_create: false,
            fail_turn: true,
        });
        set_agent_runner(mock_runner);

        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "message": "chat with uncontactable provider" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(parsed["error"]
            .as_str()
            .unwrap()
            .contains("model execution failure"));
    }

    #[tokio::test]
    async fn test_quartermaster_contract_shape_parity_go_fixtures() {
        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/chat/quartermaster")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "message": "report status and active vessels" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        let parsed_obj = parsed.as_object().unwrap();

        // Load Go fixture contract for POST /api/chat/quartermaster
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/go-fleet/POST_api_chat_quartermaster.json");
        let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go fixture");
        let fixture_val: Value = serde_json::from_str(&fixture_str).expect("parse Go fixture");
        let fixture_obj = fixture_val.as_object().expect("fixture must be object");

        // Verify shape parity: standard fields match expected types
        for key in &["reply", "suggestedActions"] {
            assert!(
                parsed_obj.contains_key(*key),
                "Missing expected key from contract: {key}"
            );
        }

        let parsed_actions = parsed_obj["suggestedActions"].as_array().unwrap();
        let fixture_actions = fixture_obj["suggestedActions"].as_array().unwrap();
        assert!(!parsed_actions.is_empty());
        assert_eq!(parsed_actions.len(), fixture_actions.len());

        let action_obj = parsed_actions[0].as_object().unwrap();
        let fixture_action_obj = fixture_actions[0].as_object().unwrap();
        for key in fixture_action_obj.keys() {
            assert!(
                action_obj.contains_key(key),
                "Missing key in action item: {key}"
            );
        }
    }
}
