//! Integration tests for galleon-fleet-core seed module (TASK-070, SPEC-013).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use galleon_fleet_core::{
    CANONICAL_COLLECTIONS, FleetStore, POLICY_DATA_COLLECTIONS, get_canonical_seed_map,
    get_policy_seed_map, get_seed_raw, get_seed_response_json, load_seed_response,
};
use tempfile::TempDir;

#[test]
fn test_canonical_collections_allowlist_count() {
    assert_eq!(
        CANONICAL_COLLECTIONS.len(),
        15,
        "SPEC-013 requires exactly 15 canonical collections"
    );
}

#[test]
fn test_all_15_canonical_collections_have_seed_files() {
    for &col in CANONICAL_COLLECTIONS {
        let raw = get_seed_raw(col).unwrap_or_else(|| panic!("missing seed for {col}"));
        assert!(
            !raw.trim().is_empty(),
            "seed file for {col} must not be empty"
        );

        let parsed: serde_json::Value =
            serde_json::from_str(raw).unwrap_or_else(|e| panic!("invalid JSON in seed {col}: {e}"));
        assert!(
            parsed.is_array(),
            "seed payload for {col} must be a JSON array"
        );
        let arr = parsed.as_array().unwrap();
        assert!(
            !arr.is_empty(),
            "seed payload for {col} must contain at least 1 item"
        );
    }
}

#[test]
fn test_all_policy_collections_have_seed_files() {
    for &col in POLICY_DATA_COLLECTIONS {
        let raw = get_seed_raw(col).unwrap_or_else(|| panic!("missing seed for {col}"));
        let parsed: serde_json::Value =
            serde_json::from_str(raw).unwrap_or_else(|e| panic!("invalid JSON in {col}: {e}"));
        assert!(parsed.is_array(), "policy seed {col} must be a JSON array");
        assert!(
            !parsed.as_array().unwrap().is_empty(),
            "policy seed {col} must not be empty"
        );
    }
}

#[test]
fn test_seed_response_structure_and_camel_case_keys() {
    let resp = load_seed_response().expect("load seed response");
    assert_eq!(resp.status, "ok");

    let serialized = serde_json::to_string(&resp).expect("serialize seed response");
    let val: serde_json::Value =
        serde_json::from_str(&serialized).expect("deserialize seed response");

    assert_eq!(val["status"], "ok");
    assert!(val.get("seed").is_some(), "must contain top-level 'seed'");

    let seed_obj = val["seed"].as_object().expect("seed must be an object");
    assert_eq!(
        seed_obj.len(),
        15,
        "seed must contain exactly the 15 canonical collections"
    );

    let expected_keys: HashSet<&str> = CANONICAL_COLLECTIONS.iter().copied().collect();
    let actual_keys: HashSet<&str> = seed_obj.keys().map(|s| s.as_str()).collect();

    assert_eq!(
        actual_keys, expected_keys,
        "seed object keys must exactly match the 15 canonical camelCase collections"
    );

    // Verify snake_case legacy keys do not exist in the response
    assert!(!seed_obj.contains_key("chat_messages"));
    assert!(!seed_obj.contains_key("global_steering"));
    assert!(!seed_obj.contains_key("journal_sessions"));
    assert!(!seed_obj.contains_key("steering_directives"));
    assert!(!seed_obj.contains_key("training_hooks"));
    assert!(!seed_obj.contains_key("training_skills"));
    assert!(!seed_obj.contains_key("treasury_ledger"));

    // Verify camelCase replacement keys exist
    assert!(seed_obj.contains_key("chatMessages"));
    assert!(seed_obj.contains_key("globalSteering"));
    assert!(seed_obj.contains_key("journalSessions"));
    assert!(seed_obj.contains_key("steeringDirectives"));
    assert!(seed_obj.contains_key("trainingHooks"));
    assert!(seed_obj.contains_key("trainingSkills"));
    assert!(seed_obj.contains_key("treasuryLedger"));
}

#[test]
fn test_hydrate_fleet_web_populates_15_of_15_collections_key_acceptance() {
    // Key acceptance criterion from TASK-070:
    // "hydrateFleet web dapat mengisi 15/15 koleksi dari seed (test kunci)"
    // Mirrors fleetStore.ts:hydrateFleet() exact logic:
    let json_str = get_seed_response_json().expect("get seed response json");
    let res: serde_json::Value =
        serde_json::from_str(json_str).expect("parse json for hydrateFleet");

    assert_eq!(res["status"], "ok");

    // The 15 keys accessed in fleetStore.ts:hydrateFleet():
    let seed_map = [
        "squads",
        "ships",
        "crew",
        "quests",
        "artifacts",
        "approvals",
        "logbook",
        "treasuryLedger",
        "notifications",
        "journalSessions",
        "chatMessages",
        "trainingSkills",
        "globalSteering",
        "steeringDirectives",
        "trainingHooks",
    ];

    let mut hydrated_collections = Vec::new();
    for col in seed_map {
        if let Some(arr) = res["seed"].get(col).and_then(|d| d.as_array())
            && !arr.is_empty()
        {
            hydrated_collections.push(col);
        }
    }

    assert_eq!(
        hydrated_collections.len(),
        15,
        "hydrateFleet must populate all 15 collections; got: {:?}",
        hydrated_collections
    );
}

#[test]
fn test_get_canonical_and_policy_seed_maps() {
    let canonical_map = get_canonical_seed_map().expect("get canonical seed map");
    assert_eq!(canonical_map.len(), 15);
    for &col in CANONICAL_COLLECTIONS {
        assert!(canonical_map.contains_key(col));
        assert!(canonical_map[col].is_array());
    }

    let policy_map = get_policy_seed_map().expect("get policy seed map");
    assert_eq!(policy_map.len(), 2);
    assert!(policy_map.contains_key("policies"));
    assert!(policy_map.contains_key("riskTiers"));
}

#[tokio::test]
async fn test_seed_payload_can_be_saved_to_fleet_store() {
    let tmp = TempDir::new().expect("temp dir");
    let store = FleetStore::new(tmp.path().join("fleet"));

    for &col in CANONICAL_COLLECTIONS {
        let raw = get_seed_raw(col).expect("seed raw");
        store
            .save_collection(col, raw.as_bytes())
            .await
            .expect("save seed to store");

        let fetched = store.get_collection(col).expect("get collection");
        assert!(fetched.is_some());
        let val: serde_json::Value =
            serde_json::from_slice(&fetched.unwrap()).expect("parse fetched");
        assert!(val.is_array());
        assert!(!val.as_array().unwrap().is_empty());
    }
}
