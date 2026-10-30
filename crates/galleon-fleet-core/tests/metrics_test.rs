//! Unit and contract tests for Fleet Metrics and Deck Bell (TASK-071, SPEC-013, BL-088).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleon_fleet_core::{
    DECK_BELL_MESSAGE, DeckBellResponse, FleetError, FleetMetrics, FleetStore,
    calculate_fleet_metrics,
};
use serde_json::Value;

#[test]
fn test_deck_bell_default_and_message_contract() {
    let bell = DeckBellResponse::default();
    assert!(bell.success);
    assert_eq!(bell.message, DECK_BELL_MESSAGE);

    // Verify against Go fixture POST_api_fleet_deck_bell.json
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/go-fleet/POST_api_fleet_deck_bell.json");
    let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go deck bell fixture");
    let fixture_val: Value =
        serde_json::from_str(&fixture_str).expect("parse Go deck bell fixture");

    let serialized = serde_json::to_value(&bell).expect("serialize bell response");
    assert_eq!(serialized["success"], fixture_val["success"]);
    assert_eq!(serialized["message"], fixture_val["message"]);
}

#[tokio::test]
async fn test_calculate_fleet_metrics_empty_store() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    let metrics = calculate_fleet_metrics(&store, None);
    assert_eq!(metrics.active_vessels, 0);
    assert_eq!(metrics.underway_quests, 0);
    assert_eq!(metrics.pending_approvals, 0);
    assert_eq!(metrics.total_treasury_ledgers, 0);
    assert_eq!(metrics.total_artifacts, 0);
    assert_eq!(metrics.total_specialists, 0);
    assert_eq!(metrics.total_squads, 0);
    assert!(metrics.system_uptime.is_none());
    assert!(metrics.gateway_latency_ms.is_none());
    assert!(metrics.active_workers.is_none());
    assert!(metrics.memory_used_mb.is_none());

    // With memory passed
    let metrics_with_mem = store.get_metrics(Some(42.5));
    assert_eq!(metrics_with_mem.memory_used_mb, Some(42.5));

    Ok(())
}

#[tokio::test]
async fn test_calculate_fleet_metrics_counts_and_status_filters() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    // 1. Ships (activeVessels)
    store
        .save_collection(
            "ships",
            b"[{\"id\":\"s1\"},{\"id\":\"s2\"},{\"id\":\"s3\"}]",
        )
        .await?;
    assert_eq!(store.count_collection("ships"), 3);

    // 2. Quests (underwayQuests: "underway" | "in_progress" | "active")
    store
        .save_collection(
            "quests",
            b"[
                {\"id\":\"q1\",\"status\":\"underway\"},
                {\"id\":\"q2\",\"status\":\"in_progress\"},
                {\"id\":\"q3\",\"status\":\"active\"},
                {\"id\":\"q4\",\"status\":\"completed\"},
                {\"id\":\"q5\",\"status\":\"draft\"}
            ]",
        )
        .await?;
    assert_eq!(
        store.count_by_status("quests", &["underway", "in_progress", "active"]),
        3
    );

    // 3. Approvals (pendingApprovals: "pending")
    store
        .save_collection(
            "approvals",
            b"[
                {\"id\":\"a1\",\"status\":\"pending\"},
                {\"id\":\"a2\",\"status\":\"approved\"},
                {\"id\":\"a3\",\"status\":\"pending\"},
                {\"id\":\"a4\",\"status\":\"rejected\"}
            ]",
        )
        .await?;
    assert_eq!(store.count_by_status("approvals", &["pending"]), 2);

    // 4. Treasury ledger
    store
        .save_collection("treasuryLedger", b"[{\"id\":\"t1\"},{\"id\":\"t2\"}]")
        .await?;
    assert_eq!(store.count_collection("treasuryLedger"), 2);

    // 5. Artifacts
    store
        .save_collection(
            "artifacts",
            b"[{\"id\":\"art1\"},{\"id\":\"art2\"},{\"id\":\"art3\"},{\"id\":\"art4\"}]",
        )
        .await?;
    assert_eq!(store.count_collection("artifacts"), 4);

    // 6. Crew (totalSpecialists)
    store
        .save_collection(
            "crew",
            b"[{\"id\":\"c1\"},{\"id\":\"c2\"},{\"id\":\"c3\"},{\"id\":\"c4\"},{\"id\":\"c5\"}]",
        )
        .await?;
    assert_eq!(store.count_collection("crew"), 5);

    // 7. Squads
    store
        .save_collection("squads", b"[{\"id\":\"sq1\"}]")
        .await?;
    assert_eq!(store.count_collection("squads"), 1);

    let metrics = store.get_metrics(Some(12.75));
    assert_eq!(metrics.active_vessels, 3);
    assert_eq!(metrics.underway_quests, 3);
    assert_eq!(metrics.pending_approvals, 2);
    assert_eq!(metrics.total_treasury_ledgers, 2);
    assert_eq!(metrics.total_artifacts, 4);
    assert_eq!(metrics.total_specialists, 5);
    assert_eq!(metrics.total_squads, 1);
    assert_eq!(metrics.memory_used_mb, Some(12.75));
    assert_eq!(metrics.system_uptime, None);
    assert_eq!(metrics.gateway_latency_ms, None);
    assert_eq!(metrics.active_workers, None);

    Ok(())
}

#[tokio::test]
async fn test_calculate_fleet_metrics_resilient_to_malformed_and_non_array_files()
-> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    // Write malformed and non-array files directly into store root
    let root = store.root();
    std::fs::write(root.join("ships.json"), b"not valid json").map_err(FleetError::Io)?;
    std::fs::write(root.join("quests.json"), b"{\"object\":\"not an array\"}")
        .map_err(FleetError::Io)?;

    assert_eq!(store.count_collection("ships"), 0);
    assert_eq!(store.count_by_status("quests", &["active"]), 0);

    let metrics = store.get_metrics(None);
    assert_eq!(metrics.active_vessels, 0);
    assert_eq!(metrics.underway_quests, 0);

    Ok(())
}

#[test]
fn test_metrics_json_serialization_exact_shape_vs_go_fixture() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/go-fleet/GET_api_fleet_metrics.json");
    let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go fixture");
    let fixture_val: Value = serde_json::from_str(&fixture_str).expect("parse Go fixture");
    let fixture_obj = fixture_val.as_object().expect("fixture must be object");

    let sample_metrics = FleetMetrics {
        active_vessels: 3,
        underway_quests: 1,
        pending_approvals: 2,
        total_treasury_ledgers: 0,
        total_artifacts: 2,
        total_specialists: 7,
        total_squads: 2,
        system_uptime: None,
        gateway_latency_ms: None,
        active_workers: None,
        memory_used_mb: Some(1.192),
    };

    let serialized = serde_json::to_value(&sample_metrics).expect("serialize sample metrics");
    let serialized_obj = serialized.as_object().expect("serialized must be object");

    // 1. Exact key-set parity: Rust response has all keys from Go fixture
    for key in fixture_obj.keys() {
        assert!(
            serialized_obj.contains_key(key),
            "serialized Rust JSON is missing expected fixture key: '{key}'"
        );
    }

    // 2. No unexpected extra keys
    for key in serialized_obj.keys() {
        assert!(
            fixture_obj.contains_key(key),
            "serialized Rust JSON contains unexpected key not in Go fixture: '{key}'"
        );
    }

    // 3. Exact camelCase key casing assertions:
    assert!(serialized_obj.contains_key("memoryUsedMB"));
    assert!(serialized_obj.contains_key("gatewayLatencyMs"));
    assert!(serialized_obj.contains_key("activeWorkers"));
    assert!(serialized_obj.contains_key("systemUptime"));
    assert!(serialized_obj.contains_key("activeVessels"));
    assert!(serialized_obj.contains_key("underwayQuests"));
    assert!(serialized_obj.contains_key("pendingApprovals"));
    assert!(serialized_obj.contains_key("totalTreasuryLedgers"));
    assert!(serialized_obj.contains_key("totalArtifacts"));
    assert!(serialized_obj.contains_key("totalSpecialists"));
    assert!(serialized_obj.contains_key("totalSquads"));

    // 4. Intentional F-13 honest differences: unmeasured telemetry serialized as null
    assert!(serialized_obj["systemUptime"].is_null());
    assert!(serialized_obj["gatewayLatencyMs"].is_null());
    assert!(serialized_obj["activeWorkers"].is_null());

    // 5. Non-volatile collection counts are numbers
    assert!(serialized_obj["activeVessels"].is_number());
    assert!(serialized_obj["underwayQuests"].is_number());
    assert!(serialized_obj["pendingApprovals"].is_number());
    assert!(serialized_obj["totalTreasuryLedgers"].is_number());
    assert!(serialized_obj["totalArtifacts"].is_number());
    assert!(serialized_obj["totalSpecialists"].is_number());
    assert!(serialized_obj["totalSquads"].is_number());
}
