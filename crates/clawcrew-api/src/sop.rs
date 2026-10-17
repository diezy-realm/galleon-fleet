//! Standard Operating Procedure (SOP) wire types, enumerations, and traits.
//!
//! Provides the canonical contract for trigger sources, run status, step status,
//! and filesystem change events shared across channels, runtime, and the Go engine.

use std::fmt;
use serde::{Deserialize, Serialize};

// ── Priority ────────────────────────────────────────────────────

/// SOP priority level, used for execution mode resolution and scheduling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SopPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl fmt::Display for SopPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ── Execution Mode ──────────────────────────────────────────────

/// How much autonomy the agent has when executing an SOP.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SopExecutionMode {
    /// Execute all steps without human approval.
    Auto,
    /// Request approval before starting, then execute all steps.
    #[default]
    Supervised,
    /// Request approval before each step.
    StepByStep,
    /// Critical/High → Auto, Normal/Low → Supervised.
    PriorityBased,
    /// Execute steps sequentially without LLM round-trips.
    Deterministic,
}

impl fmt::Display for SopExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Supervised => write!(f, "supervised"),
            Self::StepByStep => write!(f, "step_by_step"),
            Self::PriorityBased => write!(f, "priority_based"),
            Self::Deterministic => write!(f, "deterministic"),
        }
    }
}

// ── Filesystem event kind ───────────────────────────────────────

/// A normalized filesystem change kind reported by watcher/channel integrations.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum_macros::EnumIter,
    strum_macros::IntoStaticStr,
)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum FilesystemEventKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

impl fmt::Display for FilesystemEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
        }
    }
}

impl std::str::FromStr for FilesystemEventKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "created" | "create" => Ok(Self::Created),
            "modified" | "modify" => Ok(Self::Modified),
            "deleted" | "delete" => Ok(Self::Deleted),
            "renamed" | "rename" => Ok(Self::Renamed),
            _ => Err(()),
        }
    }
}

// ── Trigger Source ──────────────────────────────────────────────

/// The source type of an incoming event that may trigger an SOP.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum_macros::EnumIter,
    strum_macros::IntoStaticStr,
    strum_macros::Display,
)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SopTriggerSource {
    Mqtt,
    Webhook,
    Cron,
    Peripheral,
    Filesystem,
    Calendar,
    Channel,
    Manual,
    Amqp,
    Subagent,
}

/// Incoming event that may trigger an SOP execution run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SopEvent {
    pub source: SopTriggerSource,
    /// Topic, path, or signal identifier (depends on source type).
    #[serde(default)]
    pub topic: Option<String>,
    /// Raw payload (JSON string, sensor reading, etc.).
    #[serde(default)]
    pub payload: Option<String>,
    /// When the event occurred (ISO-8601).
    pub timestamp: String,
}

// ── Run Status ──────────────────────────────────────────────────

/// Status of an SOP execution run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SopRunStatus {
    Pending,
    Running,
    /// An operator requested cancellation while work was in flight.
    CancelRequested,
    WaitingApproval,
    /// Paused at a checkpoint in a deterministic workflow.
    PausedCheckpoint,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for SopRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::CancelRequested => write!(f, "cancel_requested"),
            Self::WaitingApproval => write!(f, "waiting_approval"),
            Self::PausedCheckpoint => write!(f, "paused_checkpoint"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ── Step Status ─────────────────────────────────────────────────

/// Result status of a single step execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SopStepStatus {
    Completed,
    Failed,
    Skipped,
}

impl fmt::Display for SopStepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

// ── Step Kind ───────────────────────────────────────────────────

/// Discriminant for what type of work a step performs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SopStepKind {
    /// Normal step — executed by the agent (or deterministic handler).
    #[default]
    Execute,
    /// Checkpoint step — pauses execution and waits for human approval.
    Checkpoint,
    /// Deterministic capability step - executed by the SOP capability registry.
    Capability,
}

impl fmt::Display for SopStepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Execute => write!(f, "execute"),
            Self::Checkpoint => write!(f, "checkpoint"),
            Self::Capability => write!(f, "capability"),
        }
    }
}

// ── Trait ───────────────────────────────────────────────────────

/// Trait handle for inspecting or interacting with an SOP engine.
pub trait SopEngineHandle: Send + Sync {
    /// Check whether the engine is active.
    fn is_active(&self) -> bool;
}

/// Result of dispatching an untrusted fan-in event into the SOP engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SopFanInResult {
    /// True when backpressure / deferred admission requires the transport to
    /// requeue or redeliver the event (e.g. AMQP durable ack).
    pub needs_redelivery: bool,
}

/// Untrusted fan-in dispatcher for channels and transports.
///
/// Decouples channel adapters (AMQP, Filesystem, MQTT, etc.) from the
/// concrete `SopEngine` and `SopAuditLogger` implementation in `clawcrew-runtime`.
#[async_trait::async_trait]
pub trait SopFanInDispatcher: Send + Sync {
    /// Dispatch an untrusted event from an external source.
    async fn dispatch_untrusted(
        &self,
        source: SopTriggerSource,
        topic: Option<&str>,
        payload: Option<&str>,
        target_sop: Option<&str>,
        dedup: Option<(String, bool)>,
    ) -> SopFanInResult;

    /// Resolve a long-lived gate prompt answered from a channel.
    async fn resolve_gate(
        &self,
        _msg: &crate::channel::ChannelMessage,
        _gate_prompt_channels: &[std::sync::Arc<dyn crate::channel::Channel>],
        _gate_channel_route_keys: &[String],
    ) -> bool {
        false
    }
}

// ── Approval and Broker Wire Types ─────────────────────────────

/// The transport a gate resolution arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSource {
    Agent,
    Cli,
    Ws,
    Http,
    System,
    Channel,
}

/// Attribution metadata for who resolved an approval gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalPrincipal {
    pub source: ApprovalSource,
    pub identity: Option<String>,
    pub channel: Option<String>,
}

impl ApprovalPrincipal {
    pub fn agent(agent_alias: &str) -> Self {
        Self {
            source: ApprovalSource::Agent,
            identity: Some(agent_alias.to_string()),
            channel: Some(agent_alias.to_string()),
        }
    }

    pub fn cli(os_user: Option<String>) -> Self {
        Self {
            source: ApprovalSource::Cli,
            identity: os_user,
            channel: Some("cli".to_string()),
        }
    }

    pub fn ws(conn_id: String, subject: Option<String>) -> Self {
        Self {
            source: ApprovalSource::Ws,
            identity: subject,
            channel: Some(conn_id),
        }
    }

    pub fn http(subject: Option<String>) -> Self {
        Self {
            source: ApprovalSource::Http,
            identity: subject,
            channel: None,
        }
    }

    pub fn system() -> Self {
        Self {
            source: ApprovalSource::System,
            identity: None,
            channel: None,
        }
    }

    pub fn channel(channel_key: String, user: Option<String>) -> Self {
        Self {
            source: ApprovalSource::Channel,
            identity: user,
            channel: Some(channel_key),
        }
    }

    pub fn is_out_of_band(&self) -> bool {
        self.source != ApprovalSource::Agent
    }

    pub fn is_system(&self) -> bool {
        self.source == ApprovalSource::System
    }

    pub fn actor_label(&self) -> String {
        self.identity
            .clone()
            .unwrap_or_else(|| self.source_label().to_string())
    }

    fn voter_source(&self) -> &'static str {
        match self.source {
            ApprovalSource::Http | ApprovalSource::Ws => "gateway",
            _ => self.source_label(),
        }
    }

    pub fn voter_key(&self) -> String {
        if self.source == ApprovalSource::Channel {
            return match (self.channel.as_deref(), self.identity.as_deref()) {
                (Some(channel), Some(id)) => format!("channel:{channel}:{id}"),
                (Some(channel), None) => format!("channel:{channel}"),
                (None, Some(id)) => format!("channel:{id}"),
                (None, None) => "channel".to_string(),
            };
        }
        match &self.identity {
            Some(id) => format!("{}:{}", self.voter_source(), id),
            None => self.voter_source().to_string(),
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self.source {
            ApprovalSource::Agent => "agent",
            ApprovalSource::Cli => "cli",
            ApprovalSource::Ws => "ws",
            ApprovalSource::Http => "http",
            ApprovalSource::System => "system",
            ApprovalSource::Channel => "channel",
        }
    }
}

/// Approve or deny a waiting SOP gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Deny {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Amend {
        text: String,
    },
    Revise {
        guidance: String,
    },
}

/// Result of resolving a gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolveOutcome {
    Resumed,
    Denied,
    Revised,
    AlreadyResolved,
    NotWaiting,
    RejectedSelfApproval,
    RejectedConstraint(String),
    DeferredAtCapacity,
}

impl ResolveOutcome {
    pub fn is_resumed(&self) -> bool {
        matches!(self, ResolveOutcome::Resumed)
    }

    pub fn label(&self) -> &'static str {
        match self {
            ResolveOutcome::Resumed => "resumed",
            ResolveOutcome::Denied => "denied",
            ResolveOutcome::Revised => "revised",
            ResolveOutcome::AlreadyResolved => "already_resolved",
            ResolveOutcome::NotWaiting => "not_waiting",
            ResolveOutcome::RejectedSelfApproval => "rejected_self_approval",
            ResolveOutcome::RejectedConstraint(_) => "rejected_constraint",
            ResolveOutcome::DeferredAtCapacity => "deferred_at_capacity",
        }
    }
}

/// High-level authorization and quorum outcome from an approval broker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrokerOutcome {
    Resolved(ResolveOutcome),
    PendingQuorum { have: usize, need: usize },
    NotAuthorized { required_group: String },
    PolicyMissing { name: String },
    PolicyUnavailable { reason: String },
    NotWaiting,
}

impl BrokerOutcome {
    pub fn label(&self) -> String {
        match self {
            BrokerOutcome::Resolved(o) => o.label().to_string(),
            BrokerOutcome::NotWaiting => "not_waiting".to_string(),
            BrokerOutcome::PendingQuorum { have, need } => {
                format!("pending_quorum ({have}/{need})")
            }
            BrokerOutcome::NotAuthorized { required_group } => {
                format!("not_authorized (requires group '{required_group}')")
            }
            BrokerOutcome::PolicyMissing { name } => {
                format!("policy_missing ('{name}')")
            }
            BrokerOutcome::PolicyUnavailable { reason } => {
                format!("policy_unavailable ({reason})")
            }
        }
    }
}

/// Summary of an SOP run for listing and live feeds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SopRunSummary {
    pub run_id: String,
    pub sop_name: String,
    pub status: SopRunStatus,
    pub current_step: u32,
    pub total_steps: u32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub trigger_source: String,
    pub active: bool,
}

// ── SOP Wire Definitions ───────────────────────────────────────

/// Admission policy for concurrent trigger firings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SopAdmissionPolicy {
    #[default]
    Parallel,
    Hold,
    Coalesce,
    Drop,
}

/// RACI governance assignment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct RaciAssignment {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub responsible: Vec<String>,
    #[serde(default)]
    pub accountable: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consulted: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub informed: Vec<String>,
}

/// Constraint policy kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstraintKind {
    RoleRestriction {
        role: String,
        forbidden_tools: Vec<String>,
    },
    DistinctPerformer {
        step_a: u32,
        step_b: u32,
    },
}

/// SOP constraint definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SopConstraint {
    #[serde(flatten)]
    pub kind: ConstraintKind,
    #[serde(default)]
    pub reason: String,
}

/// Named output port on a switch step.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SwitchRule {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goto: Option<u32>,
}

/// Step routing policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct StepRouting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub terminal: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub switch: Vec<SwitchRule>,
}

/// Step failure recovery policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StepFailure {
    #[default]
    Fail,
    Retry { max: u32 },
    Goto { step: u32 },
}

/// Per-step allow/deny tool scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct StepToolScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

/// Capability invocation within a step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct StepCall {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Single step in a Standard Operating Procedure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SopStep {
    pub number: u32,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_tools: Vec<String>,
    #[serde(default)]
    pub requires_confirmation: bool,
    #[serde(default)]
    pub kind: SopStepKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<StepToolScope>,
    #[serde(default)]
    pub routing: StepRouting,
    #[serde(default)]
    pub on_failure: StepFailure,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<SopExecutionMode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<StepCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_input: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dos: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub donts: Vec<String>,
}

impl Default for SopStep {
    fn default() -> Self {
        Self {
            number: 0,
            title: String::new(),
            body: String::new(),
            suggested_tools: Vec::new(),
            requires_confirmation: false,
            kind: SopStepKind::Execute,
            schema: None,
            scope: None,
            routing: StepRouting::default(),
            on_failure: StepFailure::default(),
            mode: None,
            calls: Vec::new(),
            agent: None,
            capability: None,
            capability_input: None,
            policy: None,
            gate_prompt: None,
            edit: None,
            dos: Vec::new(),
            donts: Vec::new(),
        }
    }
}

/// Trigger definition for starting an SOP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SopTrigger {
    Mqtt {
        topic: String,
        #[serde(default)]
        condition: Option<String>,
    },
    Webhook {
        path: String,
    },
    Cron {
        expression: String,
    },
    Peripheral {
        board: String,
        signal: String,
        #[serde(default)]
        condition: Option<String>,
    },
    Filesystem {
        path: String,
        #[serde(default)]
        event: Option<String>,
        #[serde(default)]
        condition: Option<String>,
    },
    Calendar {
        calendar: String,
        #[serde(default)]
        lookahead_minutes: Option<u64>,
    },
    Channel {
        channel: String,
        #[serde(default)]
        condition: Option<String>,
    },
    Manual,
    Amqp {
        queue: String,
        #[serde(default)]
        condition: Option<String>,
    },
    Subagent {
        parent_sop: String,
        #[serde(default)]
        condition: Option<String>,
    },
}

/// Standard Operating Procedure definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct Sop {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_sop_version")]
    pub version: String,
    #[serde(default)]
    pub priority: SopPriority,
    #[serde(default)]
    pub execution_mode: SopExecutionMode,
    #[serde(default)]
    pub triggers: Vec<SopTrigger>,
    #[serde(default)]
    pub steps: Vec<SopStep>,
    #[serde(default)]
    pub cooldown_secs: u64,
    #[serde(default = "default_sop_max_concurrent")]
    pub max_concurrent: u32,
    #[serde(skip)]
    #[cfg_attr(feature = "schema-export", schemars(skip))]
    pub location: Option<std::path::PathBuf>,
    #[serde(default)]
    pub deterministic: bool,
    #[serde(default)]
    pub admission_policy: SopAdmissionPolicy,
    #[serde(default)]
    pub max_pending_approvals: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raci: Option<RaciAssignment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<SopConstraint>,
}

fn default_sop_version() -> String {
    "1.0.0".to_string()
}

fn default_sop_max_concurrent() -> u32 {
    1
}

impl Default for Sop {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            version: default_sop_version(),
            priority: SopPriority::Normal,
            execution_mode: SopExecutionMode::Supervised,
            triggers: Vec::new(),
            steps: Vec::new(),
            cooldown_secs: 0,
            max_concurrent: default_sop_max_concurrent(),
            location: None,
            deterministic: false,
            admission_policy: SopAdmissionPolicy::Parallel,
            max_pending_approvals: 0,
            agent: None,
            raci: None,
            constraints: Vec::new(),
        }
    }
}

// ── Trigger Source Registry DTOs ───────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ConditionValueType {
    #[default]
    String,
    Number,
    Bool,
    Enum,
    DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ConditionField {
    pub path: String,
    pub label: String,
    #[serde(default)]
    pub value_type: ConditionValueType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct PayloadContract {
    pub open: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub direct: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ConditionField>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TriggerFieldKind {
    #[default]
    Text,
    List,
    Expression,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct TriggerField {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub multi: bool,
    #[serde(default)]
    pub kind: TriggerFieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct BoundTriggerSource {
    pub source: String,
    pub fields: Vec<TriggerField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PayloadContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ChannelAlias {
    pub alias: String,
    pub owning_agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ChannelTriggerKind {
    pub channel: String,
    pub aliases: Vec<ChannelAlias>,
    pub configured: bool,
    pub setup_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<PayloadContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ConditionOpSpec {
    pub token: String,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct TriggerSourceRegistry {
    pub sources: Vec<String>,
    pub bound: Vec<BoundTriggerSource>,
    pub channels: Vec<ChannelTriggerKind>,
    #[serde(default)]
    pub operators: Vec<ConditionOpSpec>,
}

// ── SopRunner Trait ────────────────────────────────────────────

/// Runner interface that decouples gateway/API callers from runtime's SOP engine.
#[async_trait::async_trait]
pub trait SopRunner: Send + Sync {
    /// List all SOP definitions in the configured SOPs directory.
    fn list_sops(&self) -> anyhow::Result<Vec<serde_json::Value>>;

    /// Get trigger source catalog and available channel configurations.
    fn get_trigger_sources(&self) -> anyhow::Result<serde_json::Value>;

    /// Build blueprint graph projection for a named SOP.
    fn get_sop_graph(&self, name: &str) -> anyhow::Result<Option<serde_json::Value>>;

    /// List active and retained SOP run summaries.
    fn list_runs(&self, sop_name: Option<&str>) -> anyhow::Result<Vec<SopRunSummary>>;

    /// Build live visual overlay for a specific SOP run.
    fn get_run_overlay(&self, name: &str, run_id: &str) -> anyhow::Result<Option<serde_json::Value>>;

    /// List runs currently waiting for human approval.
    fn get_pending_runs(&self) -> anyhow::Result<Vec<serde_json::Value>>;

    /// Return global metrics snapshot for SOP runs.
    fn get_metrics(&self) -> anyhow::Result<serde_json::Value>;

    /// Subscribe to live run change feed, returning initial snapshot + broadcast receiver.
    fn subscribe_run_changes(
        &self,
    ) -> anyhow::Result<(
        Vec<SopRunSummary>,
        Option<tokio::sync::broadcast::Receiver<SopRunSummary>>,
    )>;

    /// Trigger a named SOP manually and launch execution, returning status code and response body.
    async fn start_sop(&self, name: &str) -> anyhow::Result<(u16, serde_json::Value)>;

    /// Resolve a waiting approval gate and advance execution.
    async fn resolve_approval(
        &self,
        run_id: &str,
        decision: ApprovalDecision,
        principal: ApprovalPrincipal,
    ) -> anyhow::Result<BrokerOutcome>;

    /// Resolve an authoring surface approval decision and return (status_code, json_body).
    async fn resolve_author_approval(
        &self,
        name: &str,
        run_id: &str,
        decision: ApprovalDecision,
        principal: ApprovalPrincipal,
    ) -> anyhow::Result<(u16, serde_json::Value)>;

    /// Cancel a running or waiting SOP run, returning (status_code, json_body).
    fn cancel_run(
        &self,
        name: &str,
        run_id: &str,
        reason: Option<&str>,
        principal: ApprovalPrincipal,
    ) -> anyhow::Result<(u16, serde_json::Value)>;

    /// Get a full SOP definition by name.
    fn get_sop(&self, name: &str) -> anyhow::Result<Option<Sop>>;

    /// Create a new SOP definition on disk.
    fn create_sop(&self, sop: &Sop) -> anyhow::Result<()>;

    /// Save/update an existing SOP definition on disk.
    fn save_sop(&self, name: &str, sop: &Sop) -> anyhow::Result<()>;

    /// Delete an existing SOP definition from disk.
    fn delete_sop(&self, name: &str) -> anyhow::Result<()>;

    /// Project a graph draft with one in-flight wire edit applied.
    fn wire_draft(&self, sop: &Sop, edit: &serde_json::Value) -> anyhow::Result<serde_json::Value>;

    /// Project a graph draft without saving.
    fn graph_draft(&self, sop: &Sop) -> anyhow::Result<serde_json::Value>;

    /// Check whether any SOP trigger matches an inbound webhook path.
    fn has_matching_webhook_sop(&self, path: &str) -> anyhow::Result<bool>;

    /// Dispatch an inbound webhook payload to the matching SOP trigger.
    async fn dispatch_webhook_sop(
        &self,
        path: &str,
        payload: Option<&str>,
    ) -> anyhow::Result<Option<WebhookDispatchResult>>;

    /// Get raw run detail as JSON for testing or detailed inspection.
    fn get_run_detail(&self, _run_id: &str) -> anyhow::Result<Option<serde_json::Value>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDispatchResult {
    pub blocked_only: bool,
    pub results: Vec<serde_json::Value>,
}

