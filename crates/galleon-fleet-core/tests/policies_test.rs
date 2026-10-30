//! Tests for fleet policies domain logic, seed loading, and store overrides (TASK-072).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleon_fleet_core::{
    FleetStore, load_seed_policies, load_seed_policies_response, load_seed_risk_tiers,
};
use serde_json::Value;

#[test]
fn test_load_seed_policies() {
    let policies = load_seed_policies().expect("load seed policies");
    assert_eq!(policies.len(), 3);

    assert_eq!(policies[0].id, "pol-1");
    assert_eq!(policies[0].enforcement, "strict");
    assert_eq!(policies[0].scope, "Fleet-wide");

    assert_eq!(policies[1].id, "pol-2");
    assert_eq!(policies[1].enforcement, "permissive");
    assert_eq!(policies[1].scope, "Specialist Ships");

    assert_eq!(policies[2].id, "pol-3");
    assert_eq!(policies[2].enforcement, "kernel-enforced");
    assert_eq!(policies[2].scope, "Host Sandbox");
}

#[test]
fn test_load_seed_risk_tiers() {
    let tiers = load_seed_risk_tiers().expect("load seed risk tiers");
    assert_eq!(tiers.len(), 4);

    assert_eq!(tiers[0].tier, 1);
    assert_eq!(tiers[0].name, "Read & Inspection");
    assert!(!tiers[0].approval_required);
    assert!(tiers[0].auto_retry);
    assert_eq!(tiers[0].max_budget_usd, 0.5);

    assert_eq!(tiers[1].tier, 2);
    assert_eq!(tiers[1].name, "Code Staging & Local Branch");
    assert!(!tiers[1].approval_required);
    assert!(tiers[1].auto_retry);
    assert_eq!(tiers[1].max_budget_usd, 2.0);

    assert_eq!(tiers[2].tier, 3);
    assert_eq!(tiers[2].name, "External Writes & PR Creation");
    assert!(tiers[2].approval_required);
    assert!(!tiers[2].auto_retry);
    assert_eq!(tiers[2].max_budget_usd, 5.0);

    assert_eq!(tiers[3].tier, 4);
    assert_eq!(tiers[3].name, "Production Deploy & Secret Rotation");
    assert!(tiers[3].approval_required);
    assert!(!tiers[3].auto_retry);
    assert_eq!(tiers[3].max_budget_usd, 10.0);
}

#[test]
fn test_load_seed_policies_response() {
    let resp = load_seed_policies_response().expect("load response");
    assert_eq!(resp.policies.len(), 3);
    assert_eq!(resp.risk_tiers.len(), 4);

    let serialized = serde_json::to_string(&resp).expect("serialize");
    let parsed: Value = serde_json::from_str(&serialized).expect("parse json");

    // Must have keys "policies" and "riskTiers"
    assert!(parsed.get("policies").is_some());
    assert!(parsed.get("riskTiers").is_some());

    // Verify camelCase field name for maxBudgetUSD
    let first_tier = &parsed["riskTiers"][0];
    assert!(first_tier.get("maxBudgetUSD").is_some());
    assert!(first_tier.get("approvalRequired").is_some());
    assert!(first_tier.get("autoRetry").is_some());
}

#[test]
fn test_store_fallback_to_seed_when_no_override() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = FleetStore::new(tmp.path());

    let resp = store.get_policies().expect("get policies from empty store");
    assert_eq!(resp.policies.len(), 3);
    assert_eq!(resp.risk_tiers.len(), 4);
}

#[tokio::test]
async fn test_store_override_policies_and_risk_tiers() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = FleetStore::new(tmp.path());

    let custom_policies = serde_json::json!([
        {
            "id": "custom-1",
            "name": "Custom Gate",
            "scope": "Custom Scope",
            "enforcement": "strict",
            "description": "Custom policy description"
        }
    ]);
    let custom_risk_tiers = serde_json::json!([
        {
            "tier": 99,
            "name": "Super High Risk",
            "approvalRequired": true,
            "autoRetry": false,
            "maxBudgetUSD": 999.0
        }
    ]);

    store
        .save_collection("policies", custom_policies.to_string().as_bytes())
        .await
        .expect("save custom policies");
    store
        .save_collection("riskTiers", custom_risk_tiers.to_string().as_bytes())
        .await
        .expect("save custom risk tiers");

    let resp = store.get_policies().expect("get custom policies");
    assert_eq!(resp.policies.len(), 1);
    assert_eq!(resp.policies[0].id, "custom-1");
    assert_eq!(resp.risk_tiers.len(), 1);
    assert_eq!(resp.risk_tiers[0].tier, 99);
    assert_eq!(resp.risk_tiers[0].max_budget_usd, 999.0);
}
