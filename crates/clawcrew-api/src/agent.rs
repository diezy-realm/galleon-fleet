use std::sync::{Arc, Mutex};
use crate::plan::PlanEntry;

/// Callback type for checking if model has been switched during tool execution.
/// Returns Some((model_provider, model)) if a switch was requested, None otherwise.
pub type ModelSwitchCallback = Arc<Mutex<Option<(String, String)>>>;

/// The provider/model that actually served the most recent LLM call in a tool loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServedRoute {
    pub provider_name: String,
    pub model: String,
}

/// Shared sink a caller hands the loop to observe the final serving route.
pub type ServedRouteSink = Arc<Mutex<Option<ServedRoute>>>;

/// Error sentinel indicating that the tool loop execution was cancelled.
#[derive(Debug)]
pub struct ToolLoopCancelled;

impl std::fmt::Display for ToolLoopCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("tool loop cancelled")
    }
}

impl std::error::Error for ToolLoopCancelled {}

/// Whether this error was caused by a cancelled tool loop.
pub fn is_tool_loop_cancelled(err: &anyhow::Error) -> bool {
    err.chain().any(|source| source.is::<ToolLoopCancelled>())
}

/// Sentential error carrying a requested model switch from within a tool turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSwitchRequested {
    pub model_provider: String,
    pub model: String,
}

impl std::fmt::Display for ModelSwitchRequested {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "model switch requested to {} {}",
            self.model_provider, self.model
        )
    }
}

impl std::error::Error for ModelSwitchRequested {}

/// Extract requested model switch targets if the error contains [`ModelSwitchRequested`].
pub fn is_model_switch_requested(err: &anyhow::Error) -> Option<(String, String)> {
    err.chain()
        .filter_map(|source| source.downcast_ref::<ModelSwitchRequested>())
        .map(|e| (e.model_provider.clone(), e.model.clone()))
        .next()
}

/// Structured metadata for a tool that produced a file artifact (e.g.
/// `deliver_file`). Carried on [`TurnEvent::ToolResult`] so a channel attaches
/// the file from typed fields instead of parsing a text trailer out of the
/// free-form `output` string. Trailer parsing let a crafted filename forge the
/// delivered path (arbitrary-file-read / confused-deputy class).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolArtifact {
    /// Absolute path of the delivered file on the agent host.
    pub path: String,
    /// Stable citation URI the client can reference (e.g. `attachment://…`).
    pub uri: String,
    /// Original filename.
    pub filename: String,
    /// Human-readable chat label; defaults to the filename.
    pub title: String,
    /// MIME type.
    pub mime: String,
    /// Size in bytes.
    pub size: u64,
}

impl ToolArtifact {
    /// Build from a tool's structured `output_data` when it declares a delivered
    /// file (`delivered: true` with a non-empty `path`). Returns `None` for any
    /// other structured output, keeping this a channel-neutral convention rather
    /// than a hook tied to one tool name.
    pub fn from_delivered_data(data: &serde_json::Value) -> Option<Self> {
        if data.get("delivered").and_then(serde_json::Value::as_bool) != Some(true) {
            return None;
        }
        let field = |key: &str| {
            data.get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let path = field("path");
        if path.is_empty() {
            return None;
        }
        Some(Self {
            uri: field("uri"),
            filename: field("filename"),
            title: field("title"),
            mime: field("mimeType"),
            size: data
                .get("bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            path,
        })
    }
}

/// Provenance of a token count carried on a history-trim event. Lets clients
/// distinguish provider-reported usage from a local estimate and from a mix of
/// the two (the reported-budget trim path scales an estimate to a
/// provider-reported figure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenCountSource {
    /// Count comes from provider-reported usage.
    Provider,
    /// Count is locally estimated.
    #[serde(rename = "estimate")]
    Estimated,
    /// Count is calibrated from provider-reported usage and a local estimate.
    Calibrated,
}

impl TokenCountSource {
    /// Wire value used by the WS, SSE, RPC, and ACP surfaces.
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenCountSource::Provider => "provider",
            TokenCountSource::Estimated => "estimate",
            TokenCountSource::Calibrated => "calibrated",
        }
    }
}

pub const NO_TOOLS_TASK_FRAMING: &str = "No tools are available for this turn";
pub const NATIVE_TOOLS_TASK_FRAMING: &str = "Use tools when the request requires action";

/// Canonical English breadcrumb emitted when prior turns are trimmed from history.
pub const HISTORY_TRIM_BREADCRUMB_CANONICAL: &str =
    "[earlier turns omitted to fit the context window]";

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum TurnEvent {
    /// A text chunk from the LLM response (may arrive many times).
    Chunk {
        delta: String,
    },
    /// A reasoning/thinking chunk from a thinking model (may arrive many times).
    Thinking {
        delta: String,
    },
    /// The agent is invoking a tool.
    ToolCall {
        /// Stable correlation ID shared with the matching [`TurnEvent::ToolResult`].
        id: String,
        name: String,
        args: serde_json::Value,
    },
    /// A tool has returned a result.
    ToolResult {
        /// Stable correlation ID shared with the originating [`TurnEvent::ToolCall`].
        id: String,
        name: String,
        output: String,
        /// Typed metadata for a file-producing tool (e.g. `deliver_file`), so
        /// channels attach the file structurally instead of parsing `output`.
        /// `None` for ordinary tools.
        artifact: Option<ToolArtifact>,
    },
    Plan {
        entries: Vec<PlanEntry>,
    },
    ApprovalRequest {
        /// Correlation ID. The matching response frame must echo it.
        request_id: String,
        tool_name: String,
        /// Human-readable, secret-redacted summary of the tool arguments.
        /// Synthesised by `crate::approval::summarize_args`; never the raw
        /// `args` value.
        arguments_summary: String,
        /// How long the channel will wait before auto-denying.
        timeout_secs: u64,
    },
    /// Older whole turns were dropped to fit either the context token budget or
    /// the configured message limit. Surfaces a user-visible "context was cut
    /// here" marker so trimming is never silent. Emitted whenever a trim occurs.
    HistoryTrimmed {
        dropped_messages: usize,
        kept_turns: usize,
        reason: String,
        /// Configured context token budget in effect at trim time. `None` for
        /// message-limit trims, which carry no token accounting.
        token_budget: Option<u64>,
        /// Token count before trimming.
        tokens_before: Option<u64>,
        /// Token count after trimming.
        tokens_after: Option<u64>,
        /// Provenance of `tokens_before`.
        tokens_before_source: Option<TokenCountSource>,
        /// Provenance of `tokens_after`.
        tokens_after_source: Option<TokenCountSource>,
        /// The retained provider-facing request cannot be brought under the
        /// configured budget because only the protected newest turn (plus
        /// tool schemas) remains. History MAY have been trimmed on the way to
        /// that floor, so this flag — not `dropped_messages == 0` — is the
        /// authoritative "unsatisfiable" signal. `None`/absent for ordinary
        /// trims.
        unsatisfiable_floor: Option<bool>,
    },
    /// Per-LLM-call token usage and cost; a turn may emit several, one per
    /// model call. `None` means "unavailable for this call", not zero.
    /// The `provider_ref` and `model` identify the provider and model used
    /// for cost attribution and context window resolution.
    ///
    /// For vision routing and reliable fallback, these carry the actual served
    /// provider and model. For dynamic routing models (e.g. `openrouter/auto`)
    /// where the provider selects the concrete model and the `ModelProvider`
    /// trait cannot expose the response-reported model, `model` carries the
    /// requested routing model (e.g. `openrouter/auto`) rather than the
    /// upstream-selected concrete model. This is a known limitation of the
    /// current provider trait.
    Usage {
        input_tokens: Option<u64>,
        /// Tokens served from the provider's prompt cache (e.g. Anthropic
        /// `cache_read_input_tokens`, OpenAI `cached_tokens`). Subset of
        /// `input_tokens` — adding would double-count.
        cached_input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        cost_usd: Option<f64>,
        /// Proactive trim threshold resolved for the provider/model route that
        /// produced this usage sample. Zero means proactive trimming is disabled.
        context_token_budget: Option<u64>,
        /// Configured full context capacity for that same provider/model route.
        /// `None` means the runtime used the compatibility fallback because no
        /// authoritative capacity was configured.
        model_context_window: Option<u64>,
        /// The `<type>.<alias>` config reference of the provider that served
        /// this call. Used for accurate context window resolution and cost
        /// attribution when vision routing or provider switches are active.
        provider_ref: String,
        /// The model that actually served this call. When vision routing or
        /// a reliable fallback selects a different model, this carries the
        /// served model — not the turn-start model.
        model: String,
        /// Whether this usage event corresponds to the semantically accepted
        /// response (`true`) or a rejected physical attempt (`false`). Rejected
        /// attempts are billing-only telemetry; they do not update the accepted
        /// context snapshot (ACP session token count, context-meter ceiling).
        accepted: bool,
    },
}

#[cfg(test)]
mod plan_event_tests {
    use super::*;
    use crate::plan::{PlanEntry, PlanPriority, PlanStatus};

    #[test]
    fn plan_turn_event_carries_entries() {
        let ev = TurnEvent::Plan {
            entries: vec![PlanEntry {
                content: "Step one".to_string(),
                status: PlanStatus::Pending,
                priority: PlanPriority::Medium,
                active_form: None,
            }],
        };
        match ev {
            TurnEvent::Plan { entries } => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].content, "Step one");
            }
            _ => panic!("expected TurnEvent::Plan"),
        }
    }
}

#[cfg(test)]
mod tool_artifact_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projects_delivered_data_into_typed_fields() {
        let data = json!({
            "delivered": true,
            "uri": "attachment://deliver/report.pdf",
            "path": "/ws/uploads/ab.pdf",
            "filename": "report.pdf",
            "title": "Quarterly report",
            "mimeType": "application/pdf",
            "bytes": 1234,
        });
        let a = ToolArtifact::from_delivered_data(&data).expect("delivered data yields artifact");
        assert_eq!(a.path, "/ws/uploads/ab.pdf");
        assert_eq!(a.uri, "attachment://deliver/report.pdf");
        assert_eq!(a.filename, "report.pdf");
        assert_eq!(a.title, "Quarterly report");
        assert_eq!(a.mime, "application/pdf");
        assert_eq!(a.size, 1234);
    }

    #[test]
    fn non_delivered_data_is_ignored() {
        // Ordinary structured tool output must not be mistaken for a file artifact.
        assert!(ToolArtifact::from_delivered_data(&json!({"result": 42})).is_none());
        assert!(
            ToolArtifact::from_delivered_data(&json!({"delivered": false, "path": "/x"})).is_none()
        );
    }

    #[test]
    fn delivered_without_path_is_ignored() {
        assert!(ToolArtifact::from_delivered_data(&json!({"delivered": true})).is_none());
        assert!(
            ToolArtifact::from_delivered_data(&json!({"delivered": true, "path": ""})).is_none()
        );
    }
}
