use serde::{Deserialize, Serialize};

/// A constraint applied to a Standard Operating Procedure run.
/// Part of the Constraint Layer (Separation of Duties).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SopConstraint {
    pub kind: ConstraintKind,
    /// Human-readable reason shown when the constraint blocks an action
    /// (surfaced via HookResult::Cancel message or step validation).
    pub reason: String,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
