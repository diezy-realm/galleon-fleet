//! Unit and contract tests for Executive Briefing (TASK-074, SPEC-013, BL-088).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleon_fleet_core::{
    calculate_executive_briefing, calculate_treasury_burn_rate, ExecutiveBriefing, FleetError,
    FleetStore, DEFAULT_BRIEFING_TITLE,
};
use serde_json::Value;

#[test]
fn test_executive_briefing_contract_shape_against_go_fixture() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/go-fleet/GET_api_system_executive_briefing.json");
    let fixture_str =
        std::fs::read_to_string(&fixture_path).expect("read Go executive briefing fixture");
    let fixture_val: Value =
        serde_json::from_str(&fixture_str).expect("parse Go executive briefing fixture");

    let briefing = ExecutiveBriefing {
        active_alerts: vec!["2 pending Captain's Approval await signature".to_string()],
        admiral_status: "Awaiting Captain — 2 approval(s) pending".to_string(),
        date: "Thursday, 29 October 2026".to_string(),
        generated_at: "2026-10-29T20:39:48.660334237+07:00".to_string(),
        key_achievements: vec![
            "2 code artifact(s) indexed into the Treasury Gallery".to_string(),
            "7 specialist(s) commissioned across the fleet roster".to_string(),
            "1 voyage(s) currently underway".to_string(),
        ],
        recommended_focus:
            "Review and sign the pending approval(s), then reassign idle Ships to the quest backlog."
                .to_string(),
        title: DEFAULT_BRIEFING_TITLE.to_string(),
        treasury_burn_rate: Some("Engine Room resident memory: 2.9 MB".to_string()),
    };

    let serialized = serde_json::to_value(&briefing).expect("serialize briefing");

    // Verify all keys from the fixture exist with matching types
    assert_eq!(serialized["title"], fixture_val["title"]);
    assert!(serialized["activeAlerts"].is_array());
    assert!(serialized["admiralStatus"].is_string());
    assert!(serialized["date"].is_string());
    assert!(serialized["generatedAt"].is_string());
    assert!(serialized["keyAchievements"].is_array());
    assert!(serialized["recommendedFocus"].is_string());
    assert!(serialized["treasuryBurnRate"].is_string());
}

#[tokio::test]
async fn test_calculate_executive_briefing_empty_store() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    let briefing = calculate_executive_briefing(
        &store,
        "Friday, 30 October 2026".to_string(),
        "2026-10-30T09:00:00Z".to_string(),
    );

    assert_eq!(briefing.title, DEFAULT_BRIEFING_TITLE);
    assert_eq!(briefing.admiral_status, "Operational Ready");
    assert!(briefing.active_alerts.is_empty());
    assert_eq!(
        briefing.recommended_focus,
        "Patrol steady; no decisions pending."
    );
    assert_eq!(
        briefing.key_achievements,
        vec![
            "0 code artifact(s) indexed into the Treasury Gallery".to_string(),
            "0 specialist(s) commissioned across the fleet roster".to_string(),
            "0 voyage(s) currently underway".to_string(),
        ]
    );
    // When treasuryLedger is absent, treasuryBurnRate is None (serializes to null)
    assert!(briefing.treasury_burn_rate.is_none());

    let val = serde_json::to_value(&briefing).expect("serialize");
    assert!(val["treasuryBurnRate"].is_null());

    Ok(())
}

#[tokio::test]
async fn test_calculate_executive_briefing_with_data_and_burn_rate() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    // 1. Artifacts
    store
        .save_collection("artifacts", b"[{\"id\":\"art-1\"},{\"id\":\"art-2\"}]")
        .await?;

    // 2. Specialists (crew)
    store
        .save_collection("crew", b"[{\"id\":\"c1\"},{\"id\":\"c2\"},{\"id\":\"c3\"}]")
        .await?;

    // 3. Quests (underway)
    store
        .save_collection(
            "quests",
            b"[
                {\"id\":\"q1\",\"status\":\"underway\"},
                {\"id\":\"q2\",\"status\":\"draft\"}
            ]",
        )
        .await?;

    // 4. Approvals (pending)
    store
        .save_collection(
            "approvals",
            b"[
                {\"id\":\"a1\",\"status\":\"pending\"},
                {\"id\":\"a2\",\"status\":\"pending\"},
                {\"id\":\"a3\",\"status\":\"approved\"}
            ]",
        )
        .await?;

    // 5. Treasury ledger
    store
        .save_collection(
            "treasuryLedger",
            b"[
                {\"id\":\"tr-1\",\"costUSD\":0.12},
                {\"id\":\"tr-2\",\"costUSD\":0.04},
                {\"id\":\"tr-3\",\"costUSD\":0.00}
            ]",
        )
        .await?;

    let briefing = store.get_executive_briefing(
        "Friday, 30 October 2026".to_string(),
        "2026-10-30T09:00:00Z".to_string(),
    );

    assert_eq!(
        briefing.admiral_status,
        "Awaiting Captain — 2 approval(s) pending"
    );
    assert_eq!(
        briefing.active_alerts,
        vec!["2 pending Captain's Approval await signature".to_string()]
    );
    assert_eq!(
        briefing.recommended_focus,
        "Review and sign the pending approval(s), then reassign idle Ships to the quest backlog."
    );
    assert_eq!(
        briefing.key_achievements,
        vec![
            "2 code artifact(s) indexed into the Treasury Gallery".to_string(),
            "3 specialist(s) commissioned across the fleet roster".to_string(),
            "1 voyage(s) currently underway".to_string(),
        ]
    );

    // Burn rate calculation: 0.12 + 0.04 + 0.00 = 0.16 -> "$0.16"
    assert_eq!(briefing.treasury_burn_rate, Some("$0.16".to_string()));

    let val = serde_json::to_value(&briefing).expect("serialize");
    assert_eq!(val["treasuryBurnRate"], "$0.16");

    Ok(())
}

#[tokio::test]
async fn test_calculate_treasury_burn_rate_empty_or_no_cost() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    // Absent collection -> None
    assert_eq!(calculate_treasury_burn_rate(&store), None);

    // Empty array -> None
    store.save_collection("treasuryLedger", b"[]").await?;
    assert_eq!(calculate_treasury_burn_rate(&store), None);

    // Array without costUSD property -> None
    store
        .save_collection("treasuryLedger", b"[{\"id\":\"tr-0\"}]")
        .await?;
    assert_eq!(calculate_treasury_burn_rate(&store), None);

    // Array with valid costUSD -> Some("$0.00")
    store
        .save_collection("treasuryLedger", b"[{\"id\":\"tr-0\",\"costUSD\":0.0}]")
        .await?;
    assert_eq!(
        calculate_treasury_burn_rate(&store),
        Some("$0.00".to_string())
    );

    Ok(())
}
