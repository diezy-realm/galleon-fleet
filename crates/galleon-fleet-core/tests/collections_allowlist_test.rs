use galleon_fleet_core::{
    is_allowed_collection, is_canonical_collection, resolve_allowed_collection,
    FleetError, FleetStore, CANONICAL_COLLECTIONS, FLEET_MAX_BODY, POLICY_DATA_COLLECTIONS,
};

#[test]
fn test_allowlist_15_canonical_collections() {
    assert_eq!(CANONICAL_COLLECTIONS.len(), 15);
    let expected = [
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

    for name in expected {
        assert!(
            CANONICAL_COLLECTIONS.contains(&name),
            "missing canonical collection: {name}"
        );
        assert!(is_canonical_collection(name));
        assert!(is_allowed_collection(name));
        assert_eq!(resolve_allowed_collection(name), Some(name));
    }
}

#[test]
fn test_allowlist_policy_data_collections() {
    assert_eq!(POLICY_DATA_COLLECTIONS.len(), 2);
    for name in ["policies", "riskTiers"] {
        assert!(is_allowed_collection(name));
        assert_eq!(resolve_allowed_collection(name), Some(name));
    }
}

#[test]
fn test_unknown_collection_rejected_with_404_equivalent() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = FleetStore::new(tmp.path());

    let unknown_names = [
        "notes",
        "random",
        "users",
        "credentials",
        "secrets",
        "settings",
        "squads_backup",
        "chat_messages", // must be camelCase chatMessages
    ];

    for name in unknown_names {
        assert!(!is_allowed_collection(name));
        assert_eq!(resolve_allowed_collection(name), None);

        let get_res = store.get_collection(name);
        assert!(
            matches!(get_res, Err(FleetError::UnknownCollection(_))),
            "expected UnknownCollection for {name}, got {get_res:?}"
        );

        let save_res = store.save_collection_blocking(name, b"[]");
        assert!(
            matches!(save_res, Err(FleetError::UnknownCollection(_))),
            "expected UnknownCollection for {name}, got {save_res:?}"
        );
    }
    Ok(())
}

#[test]
fn test_path_traversal_impossible() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = FleetStore::new(tmp.path());

    let traversal_attempts = [
        "../etc/passwd",
        "../../shadow",
        "../../../etc/hosts",
        "../squads",
        "squads/../../evil",
        "squads/../ships",
        "./squads",
        ".",
        "..",
        "/etc/passwd",
        "\\..\\..\\windows\\system32",
        "squads\0.json",
    ];

    for attempt in traversal_attempts {
        assert!(
            !is_allowed_collection(attempt),
            "traversal string {attempt} was mistakenly allowed"
        );
        assert_eq!(
            resolve_allowed_collection(attempt),
            None,
            "traversal string {attempt} resolved"
        );

        let res_get = store.get_collection(attempt);
        assert!(
            matches!(res_get, Err(FleetError::UnknownCollection(_))),
            "get_collection failed to reject traversal {attempt}"
        );

        let res_save = store.save_collection_blocking(attempt, b"[]");
        assert!(
            matches!(res_save, Err(FleetError::UnknownCollection(_))),
            "save_collection failed to reject traversal {attempt}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_body_must_be_json_array() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = FleetStore::new(tmp.path());

    // Valid arrays:
    store.save_collection("squads", b"[]").await?;
    store
        .save_collection("ships", br#"[{"id":"ship-alpha"}]"#)
        .await?;

    // Invalid bodies (not an array):
    let invalid_bodies: &[&[u8]] = &[
        br#"{"name":"not an array"}"#,
        br#""just a string""#,
        br#"12345"#,
        br#"true"#,
        br#"false"#,
        br#"null"#,
        br#""#,
        br#"{}"#,
    ];

    for body in invalid_bodies {
        let res = store.save_collection("squads", body).await;
        assert!(
            matches!(res, Err(FleetError::NotAJsonArray)),
            "expected NotAJsonArray, got {res:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_fleet_max_body_quota() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = FleetStore::new(tmp.path());

    // Exact 1 MiB array of spaces around brackets is valid
    let mut exact_1mib = Vec::with_capacity(FLEET_MAX_BODY);
    exact_1mib.push(b'[');
    exact_1mib.resize(FLEET_MAX_BODY - 1, b' ');
    exact_1mib.push(b']');
    assert_eq!(exact_1mib.len(), FLEET_MAX_BODY);
    store.save_collection("quests", &exact_1mib).await?;

    // Exceeding 1 MiB by 1 byte rejected
    let mut oversized = Vec::with_capacity(FLEET_MAX_BODY + 1);
    oversized.push(b'[');
    oversized.resize(FLEET_MAX_BODY, b' ');
    oversized.push(b']');
    assert_eq!(oversized.len(), FLEET_MAX_BODY + 1);

    let res = store.save_collection("quests", &oversized).await;
    assert!(
        matches!(res, Err(FleetError::PayloadTooLarge { size, limit }) if size == FLEET_MAX_BODY + 1 && limit == FLEET_MAX_BODY),
        "expected PayloadTooLarge, got {res:?}"
    );
    Ok(())
}

#[tokio::test]
async fn test_all_fifteen_collections_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = FleetStore::new(tmp.path());

    for &col in CANONICAL_COLLECTIONS {
        assert_eq!(store.get_collection(col)?, None);

        let payload = format!(r#"[{{"collection":"{col}","status":"ok"}}]"#);
        store.save_collection(col, payload.as_bytes()).await?;

        let loaded = store
            .get_collection(col)?
            .ok_or_else(|| format!("collection {col} not found"))?;
        assert_eq!(loaded, payload.as_bytes());
    }
    Ok(())
}
