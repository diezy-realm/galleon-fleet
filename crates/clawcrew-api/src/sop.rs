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
