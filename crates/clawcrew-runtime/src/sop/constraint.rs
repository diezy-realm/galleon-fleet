use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use clawcrew_api::hook::{HookHandler, HookResult, ToolCallHookContext};
use serde::{Deserialize, Serialize};

use super::types::{Sop, SopStepResult};

/// A constraint applied to a Standard Operating Procedure run.
/// Part of the Constraint Layer (Separation of Duties).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SopConstraint {
    #[serde(flatten)]
    pub kind: ConstraintKind,
    /// Human-readable reason shown when the constraint blocks an action
    /// (surfaced via HookResult::Cancel message or step validation).
    pub reason: String,
}

impl SopConstraint {
    /// Validates whether an action/step execution violates any DistinctPerformer constraints
    /// against prior completed steps.
    pub fn check_distinct_performer(
        constraints: &[SopConstraint],
        step_number: u32,
        current_performer: Option<&str>,
        step_results: &[SopStepResult],
    ) -> Result<(), String> {
        let Some(current_performer) = current_performer else {
            return Ok(());
        };
        let current_performer = current_performer.trim();
        if current_performer.is_empty() {
            return Ok(());
        }

        for constraint in constraints {
            if let ConstraintKind::DistinctPerformer { step_a, step_b } = &constraint.kind {
                let prior_step_target = if *step_b == step_number {
                    Some(*step_a)
                } else if *step_a == step_number {
                    Some(*step_b)
                } else {
                    None
                };

                if let Some(prior_step_num) = prior_step_target {
                    let prior_performer = step_results
                        .iter()
                        .find(|r| r.step_number == prior_step_num)
                        .and_then(|r| r.effective_agent.as_deref());

                    if let Some(prior_agent) = prior_performer {
                        if prior_agent.trim().eq_ignore_ascii_case(current_performer) {
                            return Err(format!(
                                "Dynamic Separation of Duties violation: Step {} and Step {} cannot have the same performer '{}': {}",
                                step_a, step_b, current_performer, constraint.reason
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// The specific policy rule of a SOP constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstraintKind {
    /// A named role may never use the listed tool kinds, anywhere in this SOP run.
    RoleRestriction {
        role: String,
        forbidden_tools: Vec<String>,
    },
    /// Two named steps must be executed by different performers (Dynamic Separation of Duties).
    DistinctPerformer {
        step_a: u32,
        step_b: u32,
    },
}

/// Hook handler enforcing RoleRestriction constraints on tool execution.
/// Intercepts tool calls before execution via `before_tool_call` and cancels if forbidden.
pub struct RoleRestrictionHook {
    active_role: Arc<RwLock<String>>,
    constraints: Vec<SopConstraint>,
}

impl RoleRestrictionHook {
    pub fn new(role: impl Into<String>, constraints: Vec<SopConstraint>) -> Self {
        Self {
            active_role: Arc::new(RwLock::new(role.into())),
            constraints,
        }
    }

    pub fn from_sop(sop: &Sop, role: impl Into<String>) -> Self {
        Self::new(role, sop.constraints.clone())
    }

    pub fn from_constraints(constraints: Vec<SopConstraint>) -> Self {
        Self {
            active_role: Arc::new(RwLock::new(String::new())),
            constraints,
        }
    }

    pub fn set_role(&self, role: impl Into<String>) {
        if let Ok(mut lock) = self.active_role.write() {
            *lock = role.into();
        }
    }

    pub fn role(&self) -> String {
        self.active_role
            .read()
            .map(|r| r.clone())
            .unwrap_or_default()
    }
}

#[async_trait]
impl HookHandler for RoleRestrictionHook {
    fn name(&self) -> &str {
        "sop-role-restriction"
    }

    fn priority(&self) -> i32 {
        // High priority so restriction runs before standard mutating/auditing hooks
        100
    }

    async fn before_tool_call(
        &self,
        name: String,
        args: serde_json::Value,
    ) -> HookResult<(String, serde_json::Value)> {
        let current_role = self.role();
        if current_role.is_empty() {
            return HookResult::Continue((name, args));
        }

        for constraint in &self.constraints {
            if let ConstraintKind::RoleRestriction {
                role,
                forbidden_tools,
            } = &constraint.kind {
                if !role.eq_ignore_ascii_case(&current_role) {
                    continue;
                }
                for forbidden in forbidden_tools {
                    if forbidden == "*" || forbidden.eq_ignore_ascii_case(&name) {
                        let msg = format!(
                            "Role '{}' is forbidden from using tool '{}': {}",
                            current_role, name, constraint.reason
                        );
                        return HookResult::Cancel(msg);
                    }
                }
            }
        }
        HookResult::Continue((name, args))
    }

    async fn before_tool_call_with_context(
        &self,
        _context: &ToolCallHookContext,
        name: String,
        args: serde_json::Value,
    ) -> HookResult<(String, serde_json::Value)> {
        self.before_tool_call(name, args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sop::types::SopStepStatus;

    #[test]
    fn serde_role_restriction_roundtrip() {
        let constraint = SopConstraint {
            kind: ConstraintKind::RoleRestriction {
                role: "marketing_lead".into(),
                forbidden_tools: vec!["file_write".into(), "shell".into()],
            },
            reason: "Lead role cannot execute destructive mutations directly".into(),
        };
        let serialized = serde_json::to_string(&constraint).unwrap();
        let deserialized: SopConstraint = serde_json::from_str(&serialized).unwrap();
        assert_eq!(constraint, deserialized);
    }

    #[test]
    fn serde_distinct_performer_roundtrip() {
        let constraint = SopConstraint {
            kind: ConstraintKind::DistinctPerformer {
                step_a: 1,
                step_b: 2,
            },
            reason: "Author and reviewer must be distinct agents".into(),
        };
        let serialized = serde_json::to_string(&constraint).unwrap();
        let deserialized: SopConstraint = serde_json::from_str(&serialized).unwrap();
        assert_eq!(constraint, deserialized);
    }

    #[tokio::test]
    async fn role_restriction_blocks_engineer_from_deployment_tool() {
        let constraints = vec![SopConstraint {
            kind: ConstraintKind::RoleRestriction {
                role: "engineer".into(),
                forbidden_tools: vec!["deploy_production".into(), "k8s_delete".into()],
            },
            reason: "Engineer role cannot perform direct production deployment".into(),
        }];

        let hook = RoleRestrictionHook::new("engineer", constraints);

        // Engineer trying to execute deploy_production must be blocked
        let res = hook
            .before_tool_call("deploy_production".into(), serde_json::json!({}))
            .await;
        match res {
            HookResult::Cancel(reason) => {
                assert!(reason.contains("Role 'engineer' is forbidden from using tool 'deploy_production'"));
                assert!(reason.contains("Engineer role cannot perform direct production deployment"));
            }
            HookResult::Continue(_) => panic!("Expected HookResult::Cancel, got Continue"),
        }

        // Engineer executing allowed tools must continue
        let res = hook
            .before_tool_call("read_code".into(), serde_json::json!({}))
            .await;
        assert!(matches!(res, HookResult::Continue(_)));

        // If role is updated to devops_lead, deploy_production is allowed
        hook.set_role("devops_lead");
        let res = hook
            .before_tool_call("deploy_production".into(), serde_json::json!({}))
            .await;
        assert!(matches!(res, HookResult::Continue(_)));
    }

    #[test]
    fn distinct_performer_enforces_author_reviewer_separation() {
        let constraints = vec![SopConstraint {
            kind: ConstraintKind::DistinctPerformer {
                step_a: 1,
                step_b: 2,
            },
            reason: "Author and reviewer must be distinct agents".into(),
        }];

        let prior_results = vec![SopStepResult {
            step_number: 1,
            status: SopStepStatus::Completed,
            output: "PR created".into(),
            started_at: "2026-10-07T00:00:00Z".into(),
            completed_at: Some("2026-10-07T00:01:00Z".into()),
            effective_agent: Some("agent-author".into()),
            tool_calls: vec![],
        }];

        // Step 2 executed by same agent-author must fail validation
        let err = SopConstraint::check_distinct_performer(
            &constraints,
            2,
            Some("agent-author"),
            &prior_results,
        );
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(msg.contains("Dynamic Separation of Duties violation"));
        assert!(msg.contains("Step 1 and Step 2 cannot have the same performer 'agent-author'"));

        // Step 2 executed by different agent-reviewer must succeed
        let ok = SopConstraint::check_distinct_performer(
            &constraints,
            2,
            Some("agent-reviewer"),
            &prior_results,
        );
        assert!(ok.is_ok());

        // Step 3 (not governed by DistinctPerformer) executed by agent-author must succeed
        let ok3 = SopConstraint::check_distinct_performer(
            &constraints,
            3,
            Some("agent-author"),
            &prior_results,
        );
        assert!(ok3.is_ok());
    }
}
