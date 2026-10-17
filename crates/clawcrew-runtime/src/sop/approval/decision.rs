use crate::sop::types::SopRunAction;


/// Approve or deny a waiting SOP gate.
pub use clawcrew_api::sop::ApprovalDecision;


/// What `resolve_gate` did. Returned to the caller (tool / CLI / gateway) so it
/// can report, and so the executor/tick can act on a resumed action.
#[derive(Debug, Clone)]
pub enum ResolveOutcome {
    /// Approved: the next `ExecuteStep` action (the cleared gate).
    Resumed(Box<SopRunAction>),
    /// Denied: the run is Cancelled; no further action.
    Denied,
    /// Revised: the checkpoint's predecessor re-drafted with the operator's
    /// guidance and the gate re-presented (revision + 1). The run stays parked.
    Revised,
    /// Idempotent: the run was already resolved within the grace window (a late
    /// timeout racing a human decision). No double ledger row, no double persist.
    AlreadyResolved,
    /// The run is not in `WaitingApproval` (not found / wrong status). Typed, not a panic.
    NotWaiting,
    /// `approval_mode` forbids this principal from clearing the gate (an agent under
    /// `OutOfBandRequired`, or a non-agent under `AgentTool`). The gate stays open.
    RejectedSelfApproval,
    /// A constraint forbids this principal from clearing the gate (e.g. Dynamic SoD DistinctPerformer:
    /// PR author cannot approve their own PR at a review gate).
    RejectedConstraint(String),
    /// Approved, but every execution slot is full: re-admitting the run would exceed
    /// the SOP's `max_concurrent` or the global `max_concurrent_total`. The gate is
    /// left `WaitingApproval` (untouched, re-resolvable, no ledger row, no claim);
    /// a later resolve or the timeout tick's retry resumes it once a slot frees. This
    /// is routine backpressure, not a rejection - the caps are enforced on resume.
    DeferredAtCapacity,
}

impl ResolveOutcome {
    /// True when the gate was actually cleared (approved + resumed).
    pub fn is_resumed(&self) -> bool {
        matches!(self, ResolveOutcome::Resumed(_))
    }

    /// A stable label for logs / CLI output.
    pub fn label(&self) -> &'static str {
        match self {
            ResolveOutcome::Resumed(_) => "resumed",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_serde_round_trips() {
        let a: ApprovalDecision = serde_json::from_str(r#""approve""#).unwrap();
        assert_eq!(a, ApprovalDecision::Approve);
        let d: ApprovalDecision = serde_json::from_str(r#"{"deny":{"reason":"nope"}}"#).unwrap();
        assert_eq!(
            d,
            ApprovalDecision::Deny {
                reason: Some("nope".to_string())
            }
        );
        // Deny without a reason round-trips too.
        let d2: ApprovalDecision = serde_json::from_str(r#"{"deny":{}}"#).unwrap();
        assert_eq!(d2, ApprovalDecision::Deny { reason: None });
    }

    #[test]
    fn outcome_labels_and_is_resumed() {
        assert!(
            ResolveOutcome::Resumed(Box::new(SopRunAction::Completed {
                run_id: "r".into(),
                sop_name: "s".into(),
            }))
            .is_resumed()
        );
        assert!(!ResolveOutcome::Denied.is_resumed());
        assert_eq!(ResolveOutcome::NotWaiting.label(), "not_waiting");
        assert_eq!(
            ResolveOutcome::RejectedSelfApproval.label(),
            "rejected_self_approval"
        );
    }
}
