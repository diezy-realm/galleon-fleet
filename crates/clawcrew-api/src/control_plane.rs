//! The durable task/run registry contract — EPIC A's stable seam.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Background delegation task.
    Delegate,
    /// Subagent task spawned under the runtime.
    Subagent,
    /// Goal-mode task. Goal-specific state lives in the goal extension table;
    /// lifecycle and route ownership still live on [`TaskRecord`].
    Goal,
    /// Multi-step autonomous workflow (TaskRunner spec).
    Workflow,
    /// Peer inbox task.
    PeerInbox,
    /// Cron job run.
    Cron,
    /// SOP run.
    Sop,
    /// A2A (agent-to-agent) task.
    A2a,
    /// Remote turn task.
    RemoteTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is queued and waiting for executor admission.
    Queued,
    /// Task is waiting for a blocker, approval, or external event.
    Waiting,
    /// Task is validating its current checkpoint or output.
    Validating,
    /// Task is currently eligible to execute or already executing.
    Running,
    /// Task execution failed, but a retry policy permits another attempt.
    Retrying,
    /// Task is intentionally stopped but resumable.
    Paused,
    /// Task finished successfully.
    Completed,
    /// Task ended with an error.
    Failed,
    /// Task was intentionally cancelled.
    Cancelled,
    /// Written by the reaper/recovery sweep from OUTSIDE the task body — the state
    /// today's enum literally cannot represent (task-lifecycle-supervision gap).
    Lost,
    /// Heartbeat exceeded its grace window / the task passed `max_runtime`.
    TimedOut,
    /// Task recovered from restart but needs user/operator review before continuing.
    NeedsReview,
}

/// Version of the persisted generic task status contract.
pub const TASK_STATUS_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryOutcome {
    /// The task was registered during the current process lifetime.
    #[default]
    Fresh,
    /// An existing task was claimed by a successor owner after restart.
    Resumed,
    /// Recovery proved that the previous owner disappeared before settlement.
    Lost,
    /// Recovery cannot safely determine whether external work was completed.
    NeedsReview,
}

impl TaskStatus {
    /// A task is terminal once it can no longer transition. The reaper only
    /// reconciles non-terminal records.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled
                | TaskStatus::Lost
                | TaskStatus::TimedOut
        )
    }
}

/// The state of a cancellation request against a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CancellationState {
    /// No cancellation requested.
    #[default]
    None,
    /// Cancellation requested by an operator or parent task.
    Requested,
    /// Cancellation request acknowledged by the task loop.
    Acknowledged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    /// Stable task id. Producers validate it at registration boundaries.
    pub id: String,
    /// Durable task domain type.
    pub kind: TaskKind,
    /// Agent alias that owns and executes this task.
    pub agent: String,
    /// Canonical lifecycle state for the task.
    pub status: TaskStatus,
    /// OS pid of the daemon that created the task; paired with `owner_boot_id` so a
    /// recycled pid on a later boot is not mistaken for the live owner.
    #[serde(default)]
    pub owner_pid: u32,
    /// Daemon run-id; survives PID reuse and distinguishes a prior-boot orphan from
    /// a live same-boot task.
    #[serde(default)]
    pub owner_boot_id: String,
    /// Optional owner heartbeat timestamp in RFC3339 form.
    /// Only tasks that actively heartbeat may be timed out by heartbeat age; an
    /// absent heartbeat is not a derived runtime duration.
    #[serde(default)]
    pub heartbeat_at: Option<String>,
    /// Monotonic persisted recursion depth for delegation/subagent governors.
    #[serde(default)]
    pub depth: u32,
    /// Parent task id for synchronous child work, when one exists.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Trusted route/reply target that originated the task.
    /// Goal admission and visibility checks use this canonical route instead of
    /// trusting model-supplied task selectors.
    #[serde(default)]
    pub originator_route: Option<String>,
    /// Whether user-visible completion delivery has been confirmed.
    #[serde(default)]
    pub delivered: bool,
    /// Optional idempotency key for completion/delivery operations.
    #[serde(default)]
    pub idem_key: Option<String>,
    #[serde(default)]
    pub principal_id: Option<String>,
    /// Optional session key for tasks associated with a user/agent session.
    #[serde(default)]
    pub session_key: Option<String>,
    /// Optional workspace boundary for the task.
    #[serde(default)]
    pub workspace: Option<String>,
    /// The cancellation state for this task.
    #[serde(default)]
    pub cancellation_state: CancellationState,
    /// Producer checkpoint identity, when the work has a resumable boundary.
    #[serde(default)]
    pub checkpoint_id: Option<String>,
    /// Outcome assigned by startup/reaper recovery.
    #[serde(default)]
    pub recovery_outcome: RecoveryOutcome,
    /// Task registration/start timestamp in RFC3339 form.
    pub started_at: String,
    /// Terminal transition timestamp in RFC3339 form.
    #[serde(default)]
    pub finished_at: Option<String>,
}

/// One atomic read of a task's lifecycle and terminal payload columns.
///
/// `TaskRecord` remains the shared lifecycle record used by generic task
/// producers. Consumers that need terminal output use this view so status and
/// payload cannot come from different reads or stores.
#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub task: TaskRecord,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: i64,
    pub task_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

/// A durably recorded terminal transition that has not yet been applied to the
/// canonical [`TaskRecord`]. The intent is recovery metadata, not a second
/// lifecycle authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSettlementIntent {
    pub task_id: String,
    pub owner_pid: u32,
    pub owner_boot_id: String,
    pub desired_status: TaskStatus,
    pub artifact_path: String,
    pub artifact_ref: Option<String>,
    pub artifact_sha256: String,
    pub terminal_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerAction {
    Started,
    ExecutionFailed,
    ValidationPassed,
    ValidationFailed,
    Retried,
    Paused,
    Resumed,
    Completed,
    ArtifactAdded,
    EphemeralDiffYielded,
    /// An approval gate was granted (P1.3 evidence).
    ApprovalGranted,
    /// A review-gated step is waiting on operator approval before execution.
    AwaitingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Task ID this ledger belongs to.
    pub task_id: String,
    /// Step ID this action relates to.
    pub step_id: String,
    /// Timestamp of the ledger event in RFC3339.
    pub timestamp: String,
    /// Type of action performed.
    pub action: LedgerAction,
    /// Result, output, or error detail.
    pub details: String,
    /// Correlation ID linking this evidence to its originating request/turn
    /// (P1.3). Optional and defaulted so pre-existing ledger rows still parse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

/// THE stable seam. One trait, backed once by SQLite. The ACP session store and the
/// delegate/subagent/peer producers all converge here (CROSS-CUTTING epic-A D1).
#[async_trait::async_trait]
pub trait TaskRegistry: Send + Sync {
    /// Register a new unit of work. Idempotent on `rec.id`.
    async fn create(&self, rec: TaskRecord) -> anyhow::Result<()>;
    /// Stamp a liveness beat for `id` from the heart-beating owner.
    async fn heartbeat(&self, id: &str, owner_boot_id: &str) -> anyhow::Result<()>;
    /// Persist the producer's latest resumable checkpoint.
    async fn update_checkpoint(
        &self,
        id: &str,
        owner_boot_id: &str,
        checkpoint_id: Option<String>,
    ) -> anyhow::Result<()> {
        let _ = (id, owner_boot_id, checkpoint_id);
        anyhow::bail!("task registry does not support checkpoints")
    }
    /// Transition `id` to `status`, optionally recording terminal output/error.
    async fn update_status(
        &self,
        id: &str,
        status: TaskStatus,
        output: Option<String>,
        error: Option<String>,
    ) -> anyhow::Result<()>;
    /// Atomically settle a non-terminal task and record its terminal payload.
    /// Returns `true` only for the caller that won the terminal transition.
    async fn transition_terminal(
        &self,
        id: &str,
        status: TaskStatus,
        output: Option<String>,
        error: Option<String>,
    ) -> anyhow::Result<bool> {
        let _ = (id, status, output, error);
        anyhow::bail!("task registry does not support atomic terminal transitions")
    }
    /// Atomically settle `id` only when the captured owner still matches and
    /// the task remains non-terminal. Returns `false` when ownership or
    /// lifecycle changed first.
    async fn transition_terminal_if_owner(
        &self,
        id: &str,
        owner_pid: u32,
        owner_boot_id: &str,
        status: TaskStatus,
        output: Option<String>,
        error: Option<String>,
    ) -> anyhow::Result<bool> {
        let _ = (id, owner_pid, owner_boot_id, status, output, error);
        anyhow::bail!("task registry does not support owner-checked terminal transitions")
    }
    /// Claim a task for a given agent and session.
    async fn claim(&self, id: &str, owner_pid: u32, owner_boot_id: &str) -> anyhow::Result<bool> {
        let _ = (id, owner_pid, owner_boot_id);
        anyhow::bail!("task registry does not support atomic claims")
    }
    /// Transfer ownership of a task to a new owner with authorization.
    async fn transfer_ownership(
        &self,
        id: &str,
        new_owner_pid: u32,
        new_owner_boot_id: &str,
        auth_token: &str,
    ) -> anyhow::Result<()> {
        let _ = (id, new_owner_pid, new_owner_boot_id, auth_token);
        anyhow::bail!("task registry does not support ownership transfer")
    }
    /// Atomically aggregate a terminal child result into its parent, ensuring exactly-once semantics.
    async fn aggregate_child_result(
        &self,
        parent_id: &str,
        child_id: &str,
        child_status: TaskStatus,
        child_output: Option<String>,
    ) -> anyhow::Result<bool> {
        let _ = (parent_id, child_id, child_status, child_output);
        anyhow::bail!("task registry does not support exactly-once child result aggregation")
    }
    /// Request cancellation of a task.
    async fn request_cancellation(&self, id: &str) -> anyhow::Result<bool> {
        let _ = id;
        anyhow::bail!("task registry does not support cancellation requests")
    }
    /// Cascade cancellation to all descendant tasks of a parent.
    async fn cascade_cancellation(&self, parent_id: &str) -> anyhow::Result<()> {
        let _ = parent_id;
        anyhow::bail!("task registry does not support cascade cancellation")
    }
    /// Retrieve all direct child tasks of a parent.
    async fn get_child_tasks(&self, parent_id: &str) -> anyhow::Result<Vec<TaskRecord>> {
        let _ = parent_id;
        anyhow::bail!("task registry does not support get_child_tasks")
    }
    /// Retrieve every descendant task of a root (recursive, whole subtree).
    async fn list_descendants(&self, root_id: &str) -> anyhow::Result<Vec<TaskRecord>> {
        let _ = root_id;
        anyhow::bail!("task registry does not support list_descendants")
    }
    /// Acknowledge a cancellation request.
    async fn acknowledge_cancellation(
        &self,
        id: &str,
        owner_pid: u32,
        owner_boot_id: &str,
    ) -> anyhow::Result<bool> {
        let _ = (id, owner_pid, owner_boot_id);
        anyhow::bail!("task registry does not support cancellation acknowledgement")
    }
    /// Record an unapplied terminal transition before its artifact is published.
    /// Returns `false` when the task is no longer non-terminal or owner-matched.
    async fn persist_terminal_settlement_intent(
        &self,
        intent: TerminalSettlementIntent,
    ) -> anyhow::Result<bool> {
        let _ = intent;
        anyhow::bail!("task registry does not support terminal settlement intents")
    }
    /// List terminal transitions that still need recovery.
    async fn list_terminal_settlement_intents(
        &self,
    ) -> anyhow::Result<Vec<TerminalSettlementIntent>> {
        anyhow::bail!("task registry does not support terminal settlement intents")
    }
    /// Record an observability event for a task.
    async fn record_task_event(
        &self,
        task_id: &str,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let _ = (task_id, event_type, payload);
        anyhow::bail!("task registry does not support task events")
    }
    /// Read a bounded page of task events for an authenticated projection.
    async fn list_task_events(
        &self,
        task_id: &str,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<TaskEvent>> {
        let _ = (task_id, limit, offset);
        anyhow::bail!("task registry does not support task event reads")
    }
    /// Read the ordered TaskRunner work ledger for a task (checkpoint timeline).
    async fn list_task_ledger(&self, task_id: &str) -> anyhow::Result<Vec<LedgerEntry>> {
        let _ = task_id;
        anyhow::bail!("task registry does not support task ledger reads")
    }
    /// Apply a settlement intent and delete that unchanged intent in one SQLite
    /// transaction. `resolved_status` is normally the intent's desired status;
    /// recovery may conservatively resolve a corrupt artifact as `Failed`.
    async fn promote_terminal_settlement(
        &self,
        intent: &TerminalSettlementIntent,
        resolved_status: TaskStatus,
        output: Option<String>,
        error: Option<String>,
    ) -> anyhow::Result<bool> {
        let _ = (intent, resolved_status, output, error);
        anyhow::bail!("task registry does not support terminal settlement intents")
    }
    /// Remove an unchanged stale intent without changing the canonical task row.
    async fn discard_terminal_settlement_intent(
        &self,
        intent: &TerminalSettlementIntent,
    ) -> anyhow::Result<bool> {
        let _ = intent;
        anyhow::bail!("task registry does not support terminal settlement intents")
    }
    /// Operator control: reopen a non-terminal-final task (`paused`,
    /// `needs_review`, or `failed`) back to `running` with an audit event.
    /// Terminal-success/cancelled/lost/timed-out rows are never reopened.
    /// Returns `true` only when the row was actually reopened.
    async fn reopen_for_operator(&self, id: &str) -> anyhow::Result<bool> {
        let _ = id;
        anyhow::bail!("task registry does not support operator reopen")
    }
    async fn claim_owner(
        &self,
        id: &str,
        owner_pid: u32,
        owner_boot_id: &str,
    ) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<TaskRecord>>;
    async fn get_snapshot(&self, id: &str) -> anyhow::Result<Option<TaskSnapshot>> {
        Ok(self.get(id).await?.map(|task| TaskSnapshot {
            task,
            output: None,
            error: None,
        }))
    }
    async fn list_running(&self) -> anyhow::Result<Vec<TaskRecord>>;
    /// List all task records for read-only projections such as the dashboard.
    async fn list_all(&self) -> anyhow::Result<Vec<TaskRecord>>;
    async fn list_by_agent(&self, agent: &str) -> anyhow::Result<Vec<TaskRecord>>;
    /// Reaper/recovery seam: mark a record terminal-loss ONLY when this process is
    /// authoritative for it. Returns `false` (no write) when another live process
    /// owns it. `now_boot_id` remains part of the recovery seam for reaper filtering.
    async fn reconcile_lost(&self, id: &str, now_boot_id: &str) -> anyhow::Result<bool>;
    /// Mark a stale-heartbeat task timed out only if the observed owner and
    /// heartbeat still match. Returns `false` when liveness changed first.
    async fn reconcile_timed_out(
        &self,
        id: &str,
        owner_pid: u32,
        owner_boot_id: &str,
        heartbeat_at: &str,
    ) -> anyhow::Result<bool> {
        let _ = (id, owner_pid, owner_boot_id, heartbeat_at);
        anyhow::bail!("task registry does not support atomic timeout reconciliation")
    }
}

/// The live control-plane, shared (cheaply, via `Arc`/clone) across producers and
/// the reaper.
#[derive(Clone)]
pub struct ControlPlaneHandle {
    pub store: Arc<dyn TaskRegistry>,
    pub boot_id: String,
}

impl ControlPlaneHandle {
    #[must_use]
    pub fn new(store: Arc<dyn TaskRegistry>, boot_id: impl Into<String>) -> Self {
        Self {
            store,
            boot_id: boot_id.into(),
        }
    }
}

static CONTROL_PLANE: std::sync::OnceLock<ControlPlaneHandle> = std::sync::OnceLock::new();

/// Install the daemon's control-plane handle. Called ONCE at boot
/// (`daemon::run`). Subsequent calls are ignored (returns `false`), so a reload
/// iteration cannot swap the live store out from under in-flight producers.
pub fn init_control_plane(handle: ControlPlaneHandle) -> bool {
    CONTROL_PLANE.set(handle).is_ok()
}

/// The daemon-owned control plane, or `None` when this process has not booted a
/// daemon. Producers that require durable lifecycle state must either attach to
/// the configured store explicitly or reject the operation.
pub fn control_plane() -> Option<&'static ControlPlaneHandle> {
    CONTROL_PLANE.get()
}

pub mod task_registry {
    pub use super::*;
}
