//! Static fleet seed definitions, embedded assets, and seed response serializers.
//!
//! Under SPEC-013 §Seed (TASK-070, PRD default Q6):
//! - Canonical collections are embedded via `include_str!` with camelCase names
//!   (`treasuryLedger.json`, `chatMessages.json`, etc.).
//! - `GET /api/fleet/seed` returns `{"status":"ok","seed":{<camelCase>: [...]}}`,
//!   providing 15/15 populated collections and fixing F-16.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::collections::{CANONICAL_COLLECTIONS, POLICY_DATA_COLLECTIONS};
use crate::error::FleetError;

// Embedded seed assets from crates/galleon-fleet-core/seed/*.json
pub const SEED_APPROVALS: &str = include_str!("../seed/approvals.json");
pub const SEED_ARTIFACTS: &str = include_str!("../seed/artifacts.json");
pub const SEED_CHAT_MESSAGES: &str = include_str!("../seed/chatMessages.json");
pub const SEED_CREW: &str = include_str!("../seed/crew.json");
pub const SEED_GLOBAL_STEERING: &str = include_str!("../seed/globalSteering.json");
pub const SEED_JOURNAL_SESSIONS: &str = include_str!("../seed/journalSessions.json");
pub const SEED_LOGBOOK: &str = include_str!("../seed/logbook.json");
pub const SEED_NOTIFICATIONS: &str = include_str!("../seed/notifications.json");
pub const SEED_POLICIES: &str = include_str!("../seed/policies.json");
pub const SEED_QUESTS: &str = include_str!("../seed/quests.json");
pub const SEED_RISK_TIERS: &str = include_str!("../seed/riskTiers.json");
pub const SEED_SHIPS: &str = include_str!("../seed/ships.json");
pub const SEED_SQUADS: &str = include_str!("../seed/squads.json");
pub const SEED_STEERING_DIRECTIVES: &str = include_str!("../seed/steeringDirectives.json");
pub const SEED_TRAINING_HOOKS: &str = include_str!("../seed/trainingHooks.json");
pub const SEED_TRAINING_SKILLS: &str = include_str!("../seed/trainingSkills.json");
pub const SEED_TREASURY_LEDGER: &str = include_str!("../seed/treasuryLedger.json");

/// Typed container for all 15 canonical collections sent in `SeedResponse`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FleetSeedData {
    pub squads: serde_json::Value,
    pub ships: serde_json::Value,
    pub crew: serde_json::Value,
    pub quests: serde_json::Value,
    pub artifacts: serde_json::Value,
    pub approvals: serde_json::Value,
    pub logbook: serde_json::Value,
    pub treasury_ledger: serde_json::Value,
    pub notifications: serde_json::Value,
    pub journal_sessions: serde_json::Value,
    pub chat_messages: serde_json::Value,
    pub training_skills: serde_json::Value,
    pub global_steering: serde_json::Value,
    pub steering_directives: serde_json::Value,
    pub training_hooks: serde_json::Value,
}

/// HTTP API response schema for `GET /api/fleet/seed`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeedResponse {
    pub status: String,
    pub seed: FleetSeedData,
}

static SEED_RESPONSE_CACHE: OnceLock<String> = OnceLock::new();

/// Returns raw embedded JSON string for a given canonical collection or policy file.
pub fn get_seed_raw(name: &str) -> Option<&'static str> {
    match name {
        "approvals" => Some(SEED_APPROVALS),
        "artifacts" => Some(SEED_ARTIFACTS),
        "chatMessages" => Some(SEED_CHAT_MESSAGES),
        "crew" => Some(SEED_CREW),
        "globalSteering" => Some(SEED_GLOBAL_STEERING),
        "journalSessions" => Some(SEED_JOURNAL_SESSIONS),
        "logbook" => Some(SEED_LOGBOOK),
        "notifications" => Some(SEED_NOTIFICATIONS),
        "policies" => Some(SEED_POLICIES),
        "quests" => Some(SEED_QUESTS),
        "riskTiers" => Some(SEED_RISK_TIERS),
        "ships" => Some(SEED_SHIPS),
        "squads" => Some(SEED_SQUADS),
        "steeringDirectives" => Some(SEED_STEERING_DIRECTIVES),
        "trainingHooks" => Some(SEED_TRAINING_HOOKS),
        "trainingSkills" => Some(SEED_TRAINING_SKILLS),
        "treasuryLedger" => Some(SEED_TREASURY_LEDGER),
        _ => None,
    }
}

/// Parses and constructs the canonical 15-collection `FleetSeedData`.
pub fn load_fleet_seed_data() -> Result<FleetSeedData, FleetError> {
    Ok(FleetSeedData {
        squads: serde_json::from_str(SEED_SQUADS)?,
        ships: serde_json::from_str(SEED_SHIPS)?,
        crew: serde_json::from_str(SEED_CREW)?,
        quests: serde_json::from_str(SEED_QUESTS)?,
        artifacts: serde_json::from_str(SEED_ARTIFACTS)?,
        approvals: serde_json::from_str(SEED_APPROVALS)?,
        logbook: serde_json::from_str(SEED_LOGBOOK)?,
        treasury_ledger: serde_json::from_str(SEED_TREASURY_LEDGER)?,
        notifications: serde_json::from_str(SEED_NOTIFICATIONS)?,
        journal_sessions: serde_json::from_str(SEED_JOURNAL_SESSIONS)?,
        chat_messages: serde_json::from_str(SEED_CHAT_MESSAGES)?,
        training_skills: serde_json::from_str(SEED_TRAINING_SKILLS)?,
        global_steering: serde_json::from_str(SEED_GLOBAL_STEERING)?,
        steering_directives: serde_json::from_str(SEED_STEERING_DIRECTIVES)?,
        training_hooks: serde_json::from_str(SEED_TRAINING_HOOKS)?,
    })
}

/// Constructs a full `SeedResponse` containing `status: "ok"` and all 15 collections.
pub fn load_seed_response() -> Result<SeedResponse, FleetError> {
    let seed = load_fleet_seed_data()?;
    Ok(SeedResponse {
        status: "ok".to_string(),
        seed,
    })
}

/// Returns a pre-cached JSON string representation of `SeedResponse`.
pub fn get_seed_response_json() -> Result<&'static str, FleetError> {
    if let Some(cached) = SEED_RESPONSE_CACHE.get() {
        return Ok(cached.as_str());
    }
    let resp = load_seed_response()?;
    let serialized = serde_json::to_string(&resp)?;
    let _ = SEED_RESPONSE_CACHE.set(serialized);
    Ok(SEED_RESPONSE_CACHE
        .get()
        .map(|s| s.as_str())
        .unwrap_or_default())
}

/// Returns a map of all 15 canonical collections parsed as `serde_json::Value`.
pub fn get_canonical_seed_map() -> Result<BTreeMap<&'static str, serde_json::Value>, FleetError> {
    let mut map = BTreeMap::new();
    for &col in CANONICAL_COLLECTIONS {
        if let Some(raw) = get_seed_raw(col) {
            let val: serde_json::Value = serde_json::from_str(raw)?;
            map.insert(col, val);
        }
    }
    Ok(map)
}

/// Returns a map of all policy data collections (`policies`, `riskTiers`).
pub fn get_policy_seed_map() -> Result<BTreeMap<&'static str, serde_json::Value>, FleetError> {
    let mut map = BTreeMap::new();
    for &col in POLICY_DATA_COLLECTIONS {
        if let Some(raw) = get_seed_raw(col) {
            let val: serde_json::Value = serde_json::from_str(raw)?;
            map.insert(col, val);
        }
    }
    Ok(map)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_all_15_canonical_collections_have_seed() {
        assert_eq!(CANONICAL_COLLECTIONS.len(), 15);
        for &col in CANONICAL_COLLECTIONS {
            let raw = get_seed_raw(col).unwrap_or_else(|| panic!("missing seed for {col}"));
            let val: serde_json::Value =
                serde_json::from_str(raw).unwrap_or_else(|e| panic!("invalid json in {col}: {e}"));
            assert!(
                val.is_array(),
                "seed for canonical collection {col} must be array"
            );
            assert!(
                !val.as_array().unwrap().is_empty(),
                "seed for canonical collection {col} must not be empty"
            );
        }
    }

    #[test]
    fn test_policy_collections_have_seed() {
        for &col in POLICY_DATA_COLLECTIONS {
            let raw = get_seed_raw(col).unwrap_or_else(|| panic!("missing seed for {col}"));
            let val: serde_json::Value =
                serde_json::from_str(raw).unwrap_or_else(|e| panic!("invalid json in {col}: {e}"));
            assert!(val.is_array(), "seed for policy {col} must be array");
            assert!(
                !val.as_array().unwrap().is_empty(),
                "seed for policy {col} must not be empty"
            );
        }
    }

    #[test]
    fn test_unknown_collection_returns_none() {
        assert_eq!(get_seed_raw("unknown"), None);
        assert_eq!(get_seed_raw("chat_messages"), None); // snake_case is rejected
        assert_eq!(get_seed_raw("treasury_ledger"), None);
    }

    #[test]
    fn test_load_fleet_seed_data_succeeds() {
        let seed = load_fleet_seed_data().expect("load seed data");
        assert!(seed.squads.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.ships.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.crew.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.quests.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.artifacts.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.approvals.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.logbook.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.treasury_ledger.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.notifications.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.journal_sessions.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.chat_messages.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.training_skills.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.global_steering.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.steering_directives.as_array().map_or(0, |a| a.len()) >= 1);
        assert!(seed.training_hooks.as_array().map_or(0, |a| a.len()) >= 1);
    }

    #[test]
    fn test_seed_response_serializes_with_status_ok_and_camel_case() {
        let resp = load_seed_response().expect("load seed response");
        assert_eq!(resp.status, "ok");
        let json_str = serde_json::to_string(&resp).expect("serialize seed response");
        let val: serde_json::Value = serde_json::from_str(&json_str).expect("deserialize json");

        assert_eq!(val["status"], "ok");
        let seed_obj = val["seed"].as_object().expect("seed object");

        // Verify all 15 canonical collections exist as camelCase keys
        for &col in CANONICAL_COLLECTIONS {
            assert!(
                seed_obj.contains_key(col),
                "seed response missing camelCase key: {col}"
            );
            assert!(
                seed_obj[col].as_array().is_some_and(|a| !a.is_empty()),
                "collection {col} in seed response must be non-empty array"
            );
        }
    }

    #[test]
    fn test_hydrate_fleet_15_of_15_collections_acceptance() {
        let json_str = get_seed_response_json().expect("get seed response json");
        let parsed: serde_json::Value = serde_json::from_str(json_str).expect("valid json");

        assert_eq!(parsed["status"], "ok");

        // Mirrors fleetStore.ts:hydrateFleet() mapping exactly
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

        let mut hydrated_count = 0;
        for col in seed_map {
            if let Some(arr) = parsed["seed"].get(col).and_then(|d| d.as_array())
                && !arr.is_empty()
            {
                hydrated_count += 1;
            }
        }

        assert_eq!(
            hydrated_count, 15,
            "hydrateFleet web must be able to fill 15/15 collections from seed"
        );
    }
}
