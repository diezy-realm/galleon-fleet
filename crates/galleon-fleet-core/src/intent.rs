//! Intent classification models and deterministic responses for Quartermaster.
//!
//! Under SPEC-013 §Quartermaster (TASK-079):
//! - Port 1:1 regex from `engine/src/fleet/quartermaster_router.go`.
//! - Layer-1 router priority order:
//!   `Objective` (0.8) -> `EngineRoom` (0.8) -> `Report` (0.75) -> `Chat` (0.5 default).
//! - `EngineRoom` and `Report` responses are answered deterministically from real telemetry and metrics.
//! - Zero unwrap/expect in production code; static regexes are guarded by INVARIANT.

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::diagnostics::{DiagnosticItem, STATUS_HEALTHY};
use crate::metrics::FleetMetrics;

static RE_OBJECTIVE: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(
        r"(?i)\b(quest|misi|mission|siapkan|prepare|build|implement|deploy|rilis|release|kerjakan|execute|buatkan|rencana(kan)?|plan)\b",
    ) {
        Ok(re) => re,
        Err(_) => unreachable!(),
    }
});

static RE_REPORT: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(
        r"(?i)\b(status|laporan|report|ringkas|summary|summarize|briefing|progress|semua ship|all ships|fleet report)\b",
    ) {
        Ok(re) => re,
        Err(_) => unreachable!(),
    }
});

static RE_ENGINE_ROOM: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(
        r"(?i)\b(ram|cpu|memory|memori|engine room|engine-room|health|uptime|budget|biaya|cost|resource|diagnos|provider health)\b",
    ) {
        Ok(re) => re,
        Err(_) => unreachable!(),
    }
});

/// Quartermaster intent classification enum.
///
/// Direct 1:1 port of `QuartermasterIntent` in Go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// General question or conversation answered directly.
    Chat,
    /// Machine and telemetry status (RAM, CPU, health, diagnostics).
    EngineRoom,
    /// Fleet and vessel status summary.
    Report,
    /// Work or quest request (FleetOrderProposal / gated).
    Objective,
}

impl Intent {
    /// Returns the canonical snake_case string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::EngineRoom => "engine_room",
            Self::Report => "report",
            Self::Objective => "objective",
        }
    }
}

impl fmt::Display for Intent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Intent classification result carrying routing decision, confidence score, and rationale.
///
/// Port 1:1 of `IntentClassification` in Go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentClassification {
    pub intent: Intent,
    pub confidence: f64,
    pub reason: String,
}

/// Suggested action attached to Quartermaster response for dashboard navigation or actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedAction {
    pub action_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl SuggestedAction {
    /// Constructs a new suggested action without payload.
    #[must_use]
    pub fn new(label: impl Into<String>, action_type: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action_type: action_type.into(),
            payload: None,
        }
    }

    /// Constructs a new suggested action with payload.
    #[must_use]
    pub fn with_payload(
        label: impl Into<String>,
        action_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            label: label.into(),
            action_type: action_type.into(),
            payload: Some(payload),
        }
    }
}

/// Chat response model returned by Quartermaster endpoints (`POST /api/chat/quartermaster`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuartermasterChatResponse {
    pub reply: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_actions: Option<Vec<SuggestedAction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_artifact_preview: Option<serde_json::Value>,
}

impl QuartermasterChatResponse {
    /// Constructs a new basic chat response without actions or preview.
    #[must_use]
    pub fn new(reply: impl Into<String>) -> Self {
        Self {
            reply: reply.into(),
            suggested_actions: None,
            generated_artifact_preview: None,
        }
    }

    /// Constructs a chat response with suggested actions.
    #[must_use]
    pub fn with_suggested_actions(
        reply: impl Into<String>,
        suggested_actions: Vec<SuggestedAction>,
    ) -> Self {
        Self {
            reply: reply.into(),
            suggested_actions: Some(suggested_actions),
            generated_artifact_preview: None,
        }
    }
}

/// Classifies a user prompt into a high-level `Intent`.
///
/// Port 1:1 of Layer-1 regex router from `quartermaster_router.go`.
/// Order of evaluation:
/// 1. `Objective` (quest/mission/prepare/build/implement/deploy/release verbs)
/// 2. `EngineRoom` (machine/ram/cpu/memory/health/budget/diagnostics)
/// 3. `Report` (fleet report/status/summary/progress)
/// 4. `Chat` (default fallback)
#[must_use]
pub fn classify(prompt: &str) -> Intent {
    classify_intent(prompt).intent
}

/// Classifies a user prompt and returns detailed classification with confidence and rationale.
#[must_use]
pub fn classify_intent(prompt: &str) -> IntentClassification {
    let p = prompt.trim();
    if RE_OBJECTIVE.is_match(p) {
        IntentClassification {
            intent: Intent::Objective,
            confidence: 0.8,
            reason: "matched objective/quest verb".to_string(),
        }
    } else if RE_ENGINE_ROOM.is_match(p) {
        IntentClassification {
            intent: Intent::EngineRoom,
            confidence: 0.8,
            reason: "matched engine-room/resource term".to_string(),
        }
    } else if RE_REPORT.is_match(p) {
        IntentClassification {
            intent: Intent::Report,
            confidence: 0.75,
            reason: "matched report/status term".to_string(),
        }
    } else {
        IntentClassification {
            intent: Intent::Chat,
            confidence: 0.5,
            reason: "no specific signal; default chat".to_string(),
        }
    }
}

/// Generates a deterministic Quartermaster response for `Intent::Report` from real fleet metrics.
///
/// Matches the Go fixture `tests/fixtures/go-fleet/POST_api_chat_quartermaster.json` contract:
/// - Reply: `"Fleet Report: {vessels} active vessels, {quests} underway quests, {approvals} pending approvals, {artifacts} artifacts in Treasury. Uptime {uptime}."`
/// - Suggested Action: `"Open Mission Board"` (`actionType: "navigate"`, `payload: { "tab": "mission-board" }`).
#[must_use]
pub fn answer_report(
    metrics: &FleetMetrics,
    uptime_override: Option<&str>,
) -> QuartermasterChatResponse {
    let uptime_str = uptime_override
        .or(metrics.system_uptime.as_deref())
        .unwrap_or("—");

    let reply = format!(
        "Fleet Report: {} active vessels, {} underway quests, {} pending approvals, {} artifacts in Treasury. Uptime {}.",
        metrics.active_vessels,
        metrics.underway_quests,
        metrics.pending_approvals,
        metrics.total_artifacts,
        uptime_str
    );

    let action = SuggestedAction::with_payload(
        "Open Mission Board",
        "navigate",
        serde_json::json!({ "tab": "mission-board" }),
    );

    QuartermasterChatResponse::with_suggested_actions(reply, vec![action])
}

/// Generates a deterministic Quartermaster response for `Intent::EngineRoom` from real telemetry and diagnostics.
///
/// Follows honest telemetry principles under SPEC-013:
/// - Reply: `"Engine Room: {healthy}/{total} subsystems healthy, gateway latency {latency}, memory {memory}, {workers} active workers."`
/// - Unmeasured signals cleanly show honest `"—"`.
/// - Suggested Action: `"View Crow's Nest"` (`actionType: "navigate"`, `payload: { "tab": "crows-nest" }`).
#[must_use]
pub fn answer_engine_room(
    metrics: &FleetMetrics,
    diagnostics: &[DiagnosticItem],
) -> QuartermasterChatResponse {
    let total_diags = diagnostics.len();
    let healthy = diagnostics
        .iter()
        .filter(|d| d.status == STATUS_HEALTHY)
        .count();

    let latency_str = match metrics.gateway_latency_ms {
        Some(ms) => format!("{ms:.0}ms"),
        None => "—".to_string(),
    };

    let memory_str = match metrics.memory_used_mb {
        Some(mb) => format!("{mb:.1} MB"),
        None => "—".to_string(),
    };

    let workers_str = match metrics.active_workers {
        Some(w) => format!("{w}"),
        None => "—".to_string(),
    };

    let reply = format!(
        "Engine Room: {healthy}/{total_diags} subsystems healthy, gateway latency {latency_str}, memory {memory_str}, {workers_str} active workers."
    );

    let action = SuggestedAction::with_payload(
        "View Crow's Nest",
        "navigate",
        serde_json::json!({ "tab": "crows-nest" }),
    );

    QuartermasterChatResponse::with_suggested_actions(reply, vec![action])
}
