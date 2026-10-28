//! The runner: retargets evaluation to replay trace fixtures through the Go engine turn service interface.

use std::path::Path;
use std::sync::Arc;

use galleon_api::model_provider::ModelProvider;
use galleon_config::schema::MemoryConfig;
use galleon_memory::{Memory, create_memory};
use clawcrew_runtime::agent::agent::Agent;
use clawcrew_runtime::agent::dispatcher::NativeToolDispatcher;
use serde::{Deserialize, Serialize};

use crate::Mode;
use crate::case::{LlmTrace, load_suite};
use crate::grader::evaluate_expects;
use crate::observer::RecordingObserver;
use crate::record::RunRecord;
use crate::replay::TraceLlmProvider;
use crate::report::{CaseReport, SuiteReport};
use crate::tools::default_tools;

/// Request payload sent to the Go engine turn service (`StartTurn`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoTurnRequest {
    /// Session identifier for grouping turns.
    pub session_id: String,
    /// Agent identifier / alias (e.g., "eval" or "navigator").
    pub agent_id: String,
    /// User prompt / message initiating this turn.
    pub prompt: String,
    /// Serialized context window or metadata (if any).
    #[serde(default)]
    pub context_window: Vec<u8>,
}

/// The event types streamed during Go engine turn execution (`TurnResponse.EventType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoTurnEventType {
    ThoughtChunk,
    TextChunk,
    ToolCallStarted,
    ToolCallFinished,
    SubagentSpawned,
    TurnCompleted,
    Error,
}

/// A structured turn event emitted by the Go engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoTurnEvent {
    pub event_type: GoTurnEventType,
    pub content: String,
    pub subagent_id: Option<String>,
    pub error_message: Option<String>,
}

/// The completed outcome of a single turn through the Go engine service.
#[derive(Debug, Clone, Default)]
pub struct GoTurnOutcome {
    pub final_response: String,
    pub events: Vec<GoTurnEvent>,
}

/// Trait abstracting the Go engine turn execution boundary (`StartTurn` / `ExecuteTurn`).
#[async_trait::async_trait]
pub trait GoEngineTurnService: Send + Sync {
    /// Dispatches a turn through the Go engine interface and returns the outcome.
    async fn start_turn(&mut self, request: GoTurnRequest) -> anyhow::Result<GoTurnOutcome>;

    /// Concludes the evaluation run and returns the canonical RunRecord.
    fn into_run_record(self, final_response: String) -> anyhow::Result<RunRecord>;
}

/// In-process Go engine turn service client that drives deterministic trace replays.
pub struct MockGoEngineTurnClient {
    agent: Agent,
    replay_handle: crate::replay::ReplayHandle,
    observer: Arc<RecordingObserver>,
    turn_index: usize,
    _tmp_dir: tempfile::TempDir,
}

impl MockGoEngineTurnClient {
    /// Builds an isolated mock Go engine turn client for the given trace.
    pub async fn from_trace(trace: &LlmTrace) -> anyhow::Result<Self> {
        // Each case gets an isolated temp workspace and an ephemeral "none" memory
        // backend so cases cannot observe one another.
        let tmp = tempfile::tempdir()?;

        let mem_cfg = MemoryConfig {
            backend: "none".into(),
            ..MemoryConfig::default()
        };
        let memory: Arc<dyn Memory> = Arc::from(create_memory(&mem_cfg, tmp.path(), None)?);

        let observer = Arc::new(RecordingObserver::new());
        let replay = TraceLlmProvider::from_trace(trace);
        let replay_handle = replay.handle();
        let provider: Box<dyn ModelProvider> = Box::new(replay);

        // The engine's tool registry is sealed (`ScopedToolRegistry`), mintable only
        // through the one assembly seam. Route the eval harness's fixed tool set
        // through it with a permissive default policy so `assemble` is an identity
        // over `default_tools()` (nothing added, nothing dropped): the eval agent
        // sees exactly the same tools as before the seal.
        let eval_config = galleon_config::schema::Config::default();
        let eval_security = Arc::new(galleon_config::policy::SecurityPolicy::default());
        let eval_registry =
            clawcrew_runtime::tools::scoped::ScopedToolRegistry::assemble(
                clawcrew_runtime::tools::scoped::ScopedAssembly {
                    config: &eval_config,
                    agent_alias: "eval",
                    security: &eval_security,
                    built: clawcrew_runtime::tools::AllToolsResult::from_prebuilt_tools(
                        default_tools(),
                    ),
                    skills: &[],
                    runtime: Arc::new(clawcrew_runtime::platform::NativeRuntime::new()),
                    caller_allowed: None,
                    connect_mcp: false,
                    connect_peripherals: false,
                    exclude_memory: false,
                    acp_delivery: false,
                    list_deferred_mcp_specs: false,
                    emit_assembly_logs: false,
                    mcp_registry: None,
                },
            )
            .await
            .registry;

        let agent = Agent::builder()
            .model_provider(provider)
            .tools(eval_registry)
            .memory(memory)
            .observer(observer.clone())
            .tool_dispatcher(Box::new(NativeToolDispatcher))
            .workspace_dir(tmp.path().to_path_buf())
            .build()?;

        Ok(Self {
            agent,
            replay_handle,
            observer,
            turn_index: 0,
            _tmp_dir: tmp,
        })
    }

    /// Access the recording observer for inspecting metrics.
    pub fn observer(&self) -> &Arc<RecordingObserver> {
        &self.observer
    }
}

#[async_trait::async_trait]
impl GoEngineTurnService for MockGoEngineTurnClient {
    async fn start_turn(&mut self, request: GoTurnRequest) -> anyhow::Result<GoTurnOutcome> {
        let calls_before = self.observer.calls().len();

        // Drive the underlying turn execution through the agent turn engine
        let response = self.agent.turn(&request.prompt).await?;

        // Enforce the turn boundary: every step scripted for this turn must have been
        // consumed before the next turn begins, so responses cannot bleed across turns.
        self.replay_handle.finish_turn(self.turn_index)?;
        self.turn_index += 1;

        let all_calls = self.observer.calls();
        let new_calls = &all_calls[calls_before..];

        let mut events = Vec::with_capacity(new_calls.len() * 2 + 2);
        for call in new_calls {
            events.push(GoTurnEvent {
                event_type: GoTurnEventType::ToolCallStarted,
                content: format!("{}({})", call.name, call.arguments),
                subagent_id: None,
                error_message: None,
            });
            events.push(GoTurnEvent {
                event_type: GoTurnEventType::ToolCallFinished,
                content: call.result.clone(),
                subagent_id: None,
                error_message: if call.success {
                    None
                } else {
                    Some("tool execution failed".into())
                },
            });
        }

        events.push(GoTurnEvent {
            event_type: GoTurnEventType::TextChunk,
            content: response.clone(),
            subagent_id: None,
            error_message: None,
        });

        events.push(GoTurnEvent {
            event_type: GoTurnEventType::TurnCompleted,
            content: String::new(),
            subagent_id: None,
            error_message: None,
        });

        Ok(GoTurnOutcome {
            final_response: response,
            events,
        })
    }

    fn into_run_record(self, final_response: String) -> anyhow::Result<RunRecord> {
        let (input_tokens, output_tokens) = self.observer.tokens();
        let tool_calls = self.observer.calls();
        Ok(RunRecord {
            final_response,
            history: self.agent.history().to_vec(),
            tool_calls,
            input_tokens,
            output_tokens,
        })
    }
}

/// Run every `*.json` trace fixture in `dir` and return an aggregated report.
pub async fn run_suite(dir: &Path, mode: Mode) -> anyhow::Result<SuiteReport> {
    if mode == Mode::Live {
        anyhow::bail!("live mode is not implemented yet (Phase 0 supports --mode replay only)");
    }

    let traces = load_suite(dir)?;
    if traces.is_empty() {
        anyhow::bail!("no *.json trace fixtures found in {}", dir.display());
    }

    let mut cases = Vec::with_capacity(traces.len());
    for (path, trace) in traces {
        let name = trace.model_name.clone();
        let source = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        let report = match run_case(&trace).await {
            Ok(record) => CaseReport {
                name,
                source,
                grades: evaluate_expects(&trace.expects, &record),
                error: None,
            },
            Err(e) => CaseReport {
                name,
                source,
                grades: vec![],
                error: Some(e.to_string()),
            },
        };
        cases.push(report);
    }

    Ok(SuiteReport { cases })
}

/// Replay a single trace through the Go engine turn service interface and capture the run.
pub async fn run_case(trace: &LlmTrace) -> anyhow::Result<RunRecord> {
    let service = MockGoEngineTurnClient::from_trace(trace).await?;
    run_case_with_service(trace, service).await
}

/// Drives a trace through an arbitrary Go engine turn service implementation.
pub async fn run_case_with_service<S: GoEngineTurnService>(
    trace: &LlmTrace,
    mut service: S,
) -> anyhow::Result<RunRecord> {
    let mut final_response = String::new();
    for turn in &trace.turns {
        let req = GoTurnRequest {
            session_id: format!("eval-{}", trace.model_name),
            agent_id: "eval".to_string(),
            prompt: turn.user_input.clone(),
            context_window: Vec::new(),
        };
        let outcome = service.start_turn(req).await?;
        final_response = outcome.final_response;
    }

    service.into_run_record(final_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE: &str = r#"{
        "model_name": "test-smoke-greeting",
        "turns": [{
            "user_input": "Hello, how are you?",
            "steps": [{ "response": { "type": "text", "content": "Hello! I'm doing well.", "input_tokens": 20, "output_tokens": 15 } }]
        }],
        "expects": { "response_contains": ["Hello"], "response_not_contains": ["error"], "max_tool_calls": 0 }
    }"#;

    const ECHO: &str = r#"{
        "model_name": "test-single-tool-echo",
        "turns": [{
            "user_input": "Echo hello for me",
            "steps": [
                { "response": { "type": "tool_calls", "tool_calls": [{ "id": "call_1", "name": "echo", "arguments": {"message": "hello"} }], "input_tokens": 30, "output_tokens": 15 } },
                { "response": { "type": "text", "content": "The echo tool said: hello", "input_tokens": 50, "output_tokens": 10 } }
            ]
        }],
        "expects": { "response_contains": ["hello"], "tools_used": ["echo"], "max_tool_calls": 1, "all_tools_succeeded": true }
    }"#;

    #[tokio::test]
    async fn replays_text_only_trace() {
        let trace: LlmTrace = serde_json::from_str(SMOKE).unwrap();
        let record = run_case(&trace).await.unwrap();
        assert!(record.final_response.contains("Hello"));
        assert!(record.tool_calls.is_empty());
        let grades = evaluate_expects(&trace.expects, &record);
        assert!(grades.iter().all(|g| g.passed), "grades: {grades:?}");
    }

    #[tokio::test]
    async fn replays_tool_call_trace() {
        let trace: LlmTrace = serde_json::from_str(ECHO).unwrap();
        let record = run_case(&trace).await.unwrap();
        assert_eq!(record.tool_names(), vec!["echo"]);
        assert!(record.all_tools_succeeded());
        let grades = evaluate_expects(&trace.expects, &record);
        assert!(grades.iter().all(|g| g.passed), "grades: {grades:?}");
    }

    #[tokio::test]
    async fn go_engine_turn_service_emits_structured_events() {
        let trace: LlmTrace = serde_json::from_str(ECHO).unwrap();
        let mut client = MockGoEngineTurnClient::from_trace(&trace).await.unwrap();
        let req = GoTurnRequest {
            session_id: "test-session".into(),
            agent_id: "eval".into(),
            prompt: "Echo hello for me".into(),
            context_window: Vec::new(),
        };
        let outcome = client.start_turn(req).await.unwrap();
        assert!(outcome.final_response.contains("hello"));

        let event_types: Vec<GoTurnEventType> =
            outcome.events.iter().map(|e| e.event_type).collect();
        assert!(event_types.contains(&GoTurnEventType::ToolCallStarted));
        assert!(event_types.contains(&GoTurnEventType::ToolCallFinished));
        assert!(event_types.contains(&GoTurnEventType::TextChunk));
        assert!(event_types.contains(&GoTurnEventType::TurnCompleted));
    }

    #[tokio::test]
    async fn run_record_carries_dispatched_arguments_and_tool_results() {
        // End-to-end proof that the boundary payloads survive a real agent run:
        // the Unicode argument reaches the tool and the tool's own output comes
        // back, independent of whatever text the replay provider scripted.
        const UNICODE: &str = r#"{
            "model_name": "test-unicode-roundtrip",
            "turns": [{
                "user_input": "Répète: naïve café 日本語 ✓",
                "steps": [
                    { "response": { "type": "tool_calls", "tool_calls": [{ "id": "call_1", "name": "echo", "arguments": {"message": "naïve café 日本語 ✓"} }] } },
                    { "response": { "type": "text", "content": "done" } }
                ]
            }],
            "expects": {}
        }"#;
        let trace: LlmTrace = serde_json::from_str(UNICODE).unwrap();
        let record = run_case(&trace).await.unwrap();
        assert_eq!(record.tool_calls.len(), 1, "calls: {:?}", record.tool_calls);
        let call = &record.tool_calls[0];
        assert_eq!(call.name, "echo");
        assert!(
            call.arguments.contains("naïve café 日本語 ✓"),
            "dispatched arguments did not carry the Unicode message: {:?}",
            call.arguments
        );
        assert_eq!(
            call.result, "naïve café 日本語 ✓",
            "tool result did not round-trip the Unicode message"
        );
        assert_eq!(record.final_response, "done");
    }

    #[tokio::test]
    async fn a_call_missing_the_expected_argument_reaches_the_tool_and_the_loop_continues() {
        const MALFORMED: &str = r#"{
            "model_name": "test-missing-tool-argument",
            "turns": [{
                "user_input": "Echo my greeting for clawcrew_user.",
                "steps": [
                    { "response": { "type": "tool_calls", "tool_calls": [{ "id": "call_1", "name": "echo", "arguments": {"wrong_key": "hello"} }] } },
                    { "response": { "type": "text", "content": "The echo tool received no usable message." } }
                ]
            }],
            "expects": {}
        }"#;
        let trace: LlmTrace = serde_json::from_str(MALFORMED).unwrap();
        let record = run_case(&trace).await.unwrap();

        assert_eq!(record.tool_calls.len(), 1, "calls: {:?}", record.tool_calls);
        let call = &record.tool_calls[0];
        assert_eq!(call.name, "echo");
        assert!(
            call.arguments.contains("wrong_key"),
            "the malformed key did not survive to the dispatch boundary: {:?}",
            call.arguments
        );
        assert!(
            call.result.contains("(empty)"),
            "the tool's missing-message fallback did not reach the record: {:?}",
            call.result
        );
        assert!(call.success, "the dispatch itself must not be an error");
        assert_eq!(
            record.final_response,
            "The echo tool received no usable message."
        );
    }

    #[tokio::test]
    async fn live_mode_is_rejected_in_phase_0() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_suite(dir.path(), Mode::Live).await.unwrap_err();
        assert!(err.to_string().contains("live mode is not implemented"));
    }

    #[tokio::test]
    async fn run_suite_rejects_a_zero_turn_fixture_instead_of_reporting_it_green() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("zero_turns.json"),
            r#"{"model_name":"empty-turns","turns":[],"expects":{"max_tool_calls":0}}"#,
        )
        .unwrap();

        let err = run_suite(dir.path(), Mode::Replay)
            .await
            .expect_err("a suite holding a zero-turn fixture must not report a pass");

        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("declares no conversation turns"),
            "the suite must fail on the zero-turn fixture, got: {rendered}"
        );
    }

    const MULTI_TURN: &str = r#"{
        "model_name": "test-multi-turn",
        "turns": [
            { "user_input": "Hi", "steps": [{ "response": { "type": "text", "content": "Hello there." } }] },
            { "user_input": "And goodbye?", "steps": [{ "response": { "type": "text", "content": "Goodbye!" } }] }
        ],
        "expects": {}
    }"#;

    #[tokio::test]
    async fn replays_multi_turn_trace_in_order() {
        let trace: LlmTrace = serde_json::from_str(MULTI_TURN).unwrap();
        let record = run_case(&trace).await.unwrap();
        assert!(
            record.final_response.contains("Goodbye"),
            "final response: {:?}",
            record.final_response
        );
    }

    #[tokio::test]
    async fn over_specified_turn_is_an_error() {
        let trace: LlmTrace = serde_json::from_str(
            r#"{
                "model_name": "test-over-specified",
                "turns": [{ "user_input": "Hi", "steps": [
                    { "response": { "type": "text", "content": "Hello there." } },
                    { "response": { "type": "text", "content": "unused extra step" } }
                ] }],
                "expects": {}
            }"#,
        )
        .unwrap();
        let err = match run_case(&trace).await {
            Ok(_) => panic!("expected an error: an over-specified turn left a step unconsumed"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("over-specifies") || msg.contains("never requested"),
            "unexpected error: {msg}"
        );
    }
}
