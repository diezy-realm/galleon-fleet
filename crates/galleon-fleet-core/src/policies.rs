//! Fleet security and operational policies.
//!
//! Under SPEC-013 §Policies:
//! - `GET /api/fleet/policies` returns operational policies and risk tier definitions.
//! - In Rust, `galleon_config::policy::CommandRiskLevel` (`galleon-config/src/policy.rs:13`,
//!   `Low`, `Medium`, `High`) is the backend operational source of truth for command risk scoring.
//! - Display / budget tiers (`riskTiers`, 1..4) with `maxBudgetUSD` are seeded in
//!   `crates/galleon-fleet-core/seed/riskTiers.json` and served over HTTP.
//! - Operational policies are seeded in `crates/galleon-fleet-core/seed/policies.json`.
//! - Store precedence: `<root>/policies.json` and `<root>/riskTiers.json` override seed
//!   if present in `FleetStore`.

use serde::{Deserialize, Serialize};

use crate::error::FleetError;
use crate::seed::{SEED_POLICIES, SEED_RISK_TIERS};

/// Operational policy definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FleetPolicy {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub enforcement: String,
    pub description: String,
}

/// Risk tier definition for agent task classification and budget gating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RiskTier {
    pub tier: u32,
    pub name: String,
    pub approval_required: bool,
    pub auto_retry: bool,
    #[serde(rename = "maxBudgetUSD")]
    pub max_budget_usd: f64,
}

/// Response payload for `GET /api/fleet/policies`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetPoliciesResponse {
    pub policies: Vec<FleetPolicy>,
    pub risk_tiers: Vec<RiskTier>,
}

/// Parses and returns canonical seed policies.
pub fn load_seed_policies() -> Result<Vec<FleetPolicy>, FleetError> {
    serde_json::from_str(SEED_POLICIES).map_err(FleetError::from)
}

/// Parses and returns canonical seed risk tiers.
pub fn load_seed_risk_tiers() -> Result<Vec<RiskTier>, FleetError> {
    serde_json::from_str(SEED_RISK_TIERS).map_err(FleetError::from)
}

/// Constructs a `FleetPoliciesResponse` from the embedded seed data.
pub fn load_seed_policies_response() -> Result<FleetPoliciesResponse, FleetError> {
    Ok(FleetPoliciesResponse {
        policies: load_seed_policies()?,
        risk_tiers: load_seed_risk_tiers()?,
    })
}
