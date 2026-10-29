//! Collection schemas and allowlists for Galleon Fleet.
//!
//! Under SPEC-013 §Koleksi, collections accessible via `/api/collections/{name}`
//! are strictly restricted to an allowlist of 15 canonical camelCase collections
//! used by the web dashboard, alongside fleet policy/risk tier data files.
//! Arbitrary collection names and path traversal attempts are rejected.

use crate::error::FleetError;
use crate::store::FLEET_MAX_BODY;

/// Canonical collection names (camelCase) supported by web dashboard and fleet core
/// (per SPEC-013 §Koleksi & `web/src/store/fleetStore.ts:340–372`).
pub const CANONICAL_COLLECTIONS: &[&str] = &[
    "squads",
    "ships",
    "crew",
    "trainingSkills",
    "globalSteering",
    "steeringDirectives",
    "trainingHooks",
    "quests",
    "artifacts",
    "approvals",
    "logbook",
    "treasuryLedger",
    "notifications",
    "journalSessions",
    "chatMessages",
];

/// Policy data files seeded alongside collections in fleet storage.
pub const POLICY_DATA_COLLECTIONS: &[&str] = &["policies", "riskTiers"];

/// Check whether a name is one of the 15 canonical camelCase collections.
pub fn is_canonical_collection(name: &str) -> bool {
    CANONICAL_COLLECTIONS.contains(&name)
}

/// Check whether a name is one of the policy data files.
pub fn is_policy_collection(name: &str) -> bool {
    POLICY_DATA_COLLECTIONS.contains(&name)
}

/// Check whether a name is in the allowlist of accessible collections.
pub fn is_allowed_collection(name: &str) -> bool {
    is_canonical_collection(name) || is_policy_collection(name)
}

/// Resolves an incoming collection name against the allowlist.
///
/// Returns the matched `&'static str` on success, or `None` if the name
/// is not allowed. Because callers construct file paths solely using
/// this `'static` string rather than raw user input, path traversal
/// attacks (e.g. `../../evil`) are structurally prevented.
pub fn resolve_allowed_collection(name: &str) -> Option<&'static str> {
    CANONICAL_COLLECTIONS
        .iter()
        .find(|&&col| col == name)
        .or_else(|| POLICY_DATA_COLLECTIONS.iter().find(|&&col| col == name))
        .copied()
}

/// Validates that raw payload bytes represent a valid JSON array within the 1 MiB limit.
///
/// Returns:
/// - `Err(FleetError::PayloadTooLarge)` if size exceeds `FLEET_MAX_BODY` (1 MiB).
/// - `Err(FleetError::NotAJsonArray)` if the payload is not valid JSON or top-level is not an array.
/// - `Ok(())` if the payload is a valid JSON array within limits.
pub fn validate_collection_payload(bytes: &[u8]) -> Result<(), FleetError> {
    if bytes.len() > FLEET_MAX_BODY {
        return Err(FleetError::PayloadTooLarge {
            size: bytes.len(),
            limit: FLEET_MAX_BODY,
        });
    }

    let val: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| FleetError::NotAJsonArray)?;

    if !val.is_array() {
        return Err(FleetError::NotAJsonArray);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_collections_count_is_fifteen() {
        assert_eq!(CANONICAL_COLLECTIONS.len(), 15);
    }

    #[test]
    fn test_allowlist_resolves_all_canonical_collections() {
        for &col in CANONICAL_COLLECTIONS {
            assert!(is_canonical_collection(col));
            assert!(is_allowed_collection(col));
            assert_eq!(resolve_allowed_collection(col), Some(col));
        }
    }

    #[test]
    fn test_allowlist_resolves_policy_collections() {
        for &col in POLICY_DATA_COLLECTIONS {
            assert!(is_policy_collection(col));
            assert!(is_allowed_collection(col));
            assert_eq!(resolve_allowed_collection(col), Some(col));
        }
    }

    #[test]
    fn test_allowlist_rejects_unknown_collections() {
        let unknown = [
            "notes",
            "users",
            "passwords",
            "config",
            "data",
            "foo",
            "bar",
        ];
        for name in unknown {
            assert!(!is_canonical_collection(name));
            assert!(!is_allowed_collection(name));
            assert_eq!(resolve_allowed_collection(name), None);
        }
    }

    #[test]
    fn test_allowlist_rejects_legacy_snake_case_directly() {
        // Direct API endpoint requires camelCase; snake_case without resolution is not allowed
        assert_eq!(resolve_allowed_collection("chat_messages"), None);
        assert_eq!(resolve_allowed_collection("global_steering"), None);
        assert_eq!(resolve_allowed_collection("training_skills"), None);
    }

    #[test]
    fn test_allowlist_rejects_path_traversal() {
        let attacks = [
            "../squads",
            "../../etc/passwd",
            "../../../evil",
            "squads/../../evil",
            "./squads",
            ".",
            "..",
            "/squads",
            "squads\0.json",
            "squads/test",
            "squads.json",
        ];
        for attack in attacks {
            assert!(!is_allowed_collection(attack), "failed for {attack}");
            assert_eq!(
                resolve_allowed_collection(attack),
                None,
                "resolved traversal {attack}"
            );
        }
    }

    #[test]
    fn test_validate_payload_accepts_valid_arrays() {
        assert!(validate_collection_payload(b"[]").is_ok());
        assert!(validate_collection_payload(b"[1, 2, 3]").is_ok());
        assert!(validate_collection_payload(b"[{\"id\": \"item-1\"}]").is_ok());
        assert!(validate_collection_payload(b"  [ {\"name\": \"alpha\"} ]  \n").is_ok());
    }

    #[test]
    fn test_validate_payload_rejects_non_arrays() {
        let non_arrays: &[&[u8]] = &[
            b"{\"id\": \"item-1\"}",
            b"\"just a string\"",
            b"12345",
            b"true",
            b"false",
            b"null",
            b"",
            b"   ",
        ];
        for item in non_arrays {
            let res = validate_collection_payload(item);
            assert!(matches!(res, Err(FleetError::NotAJsonArray)));
        }
    }

    #[test]
    fn test_validate_payload_rejects_invalid_json() {
        let invalid: &[&[u8]] = &[b"[1, 2, ", b"[{foo: bar}]", b"not json at all", b"[\0]"];
        for item in invalid {
            let res = validate_collection_payload(item);
            assert!(matches!(res, Err(FleetError::NotAJsonArray)));
        }
    }

    #[test]
    fn test_validate_payload_rejects_oversized_payload() {
        // 1 MiB + 1 byte
        let oversized = vec![b' '; FLEET_MAX_BODY + 1];
        let res = validate_collection_payload(&oversized);
        assert!(matches!(res, Err(FleetError::PayloadTooLarge { .. })));
    }
}
