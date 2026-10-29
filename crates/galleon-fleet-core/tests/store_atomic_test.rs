use galleon_fleet_core::FleetStore;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn test_atomic_write_simulation_of_interrupted_write()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = FleetStore::new(tmp.path().join("fleet"));
    store.ensure_dir()?;

    // Existing good file
    let good_payload = br#"[{"id":"good-1"}]"#;
    store.write_atomic("ships.json", good_payload).await?;

    // Simulate an interrupted write: create a named temp file in root but do NOT persist
    {
        let mut temp = tempfile::NamedTempFile::new_in(store.root())?;
        use std::io::Write;
        temp.write_all(b"corrupted partial data")?;
        // Dropping temp without persist cleans it up
    }

    // Target file must remain untouched
    let read_back = store.read_file("ships.json")?;
    assert_eq!(read_back, good_payload);
    Ok(())
}

#[tokio::test]
async fn test_concurrent_atomic_writes() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = Arc::new(FleetStore::new(tmp.path().join("fleet")));
    store.ensure_dir()?;

    let f1 = async { store.write_atomic("file_1.json", b"[1]").await };
    let f2 = async { store.write_atomic("file_2.json", b"[2]").await };
    let f3 = async { store.write_atomic("file_3.json", b"[3]").await };

    let (r1, r2, r3) = tokio::join!(f1, f2, f3);
    r1?;
    r2?;
    r3?;

    assert_eq!(store.read_file("file_1.json")?, b"[1]");
    assert_eq!(store.read_file("file_2.json")?, b"[2]");
    assert_eq!(store.read_file("file_3.json")?, b"[3]");

    Ok(())
}

#[test]
fn test_legacy_import_candidate_order() -> Result<(), Box<dyn std::error::Error>> {
    let missing_candidate = PathBuf::from("/non/existent/path/for/fleet/data");
    let actual_source = tempfile::tempdir()?;
    let store_dir = tempfile::tempdir()?;
    let store = FleetStore::new(store_dir.path().join("fleet"));

    // Populate actual source
    let quests_file = actual_source.path().join("quests.json");
    std::fs::write(&quests_file, br#"[{"id":"quest-42"}]"#)?;

    let candidates = vec![missing_candidate, actual_source.path().to_path_buf()];
    let report = store.import_legacy_from_candidates(&candidates)?;
    assert!(report.is_some());
    let rep = report.ok_or("expected report")?;
    assert_eq!(rep.source, actual_source.path());
    assert_eq!(rep.files_copied, vec!["quests.json".to_string()]);

    assert_eq!(store.read_file("quests.json")?, br#"[{"id":"quest-42"}]"#);
    assert!(quests_file.exists()); // Source preserved
    assert!(store.is_imported());

    Ok(())
}

#[test]
fn test_legacy_import_maps_snake_case_collections() -> Result<(), Box<dyn std::error::Error>> {
    let src = tempfile::tempdir()?;
    let dst = tempfile::tempdir()?;
    let store = FleetStore::new(dst.path().join("fleet"));

    // Legacy snake_case names from Go data
    std::fs::write(src.path().join("global_steering.json"), b"[]")?;
    std::fs::write(src.path().join("steering_directives.json"), b"[]")?;
    std::fs::write(src.path().join("training_hooks.json"), b"[]")?;
    std::fs::write(src.path().join("journal_sessions.json"), b"[]")?;
    std::fs::write(src.path().join("treasury_ledger.json"), b"[]")?;
    std::fs::write(src.path().join("policies.json"), b"[]")?;
    std::fs::write(src.path().join("riskTiers.json"), b"[]")?;

    let report = store.import_from_dir(src.path())?;
    assert!(
        report
            .files_copied
            .contains(&"globalSteering.json".to_string())
    );
    assert!(
        report
            .files_copied
            .contains(&"steeringDirectives.json".to_string())
    );
    assert!(
        report
            .files_copied
            .contains(&"trainingHooks.json".to_string())
    );
    assert!(
        report
            .files_copied
            .contains(&"journalSessions.json".to_string())
    );
    assert!(
        report
            .files_copied
            .contains(&"treasuryLedger.json".to_string())
    );
    assert!(report.files_copied.contains(&"policies.json".to_string()));
    assert!(report.files_copied.contains(&"riskTiers.json".to_string()));

    // Verify all source files still exist intact
    assert!(src.path().join("global_steering.json").exists());
    assert!(src.path().join("steering_directives.json").exists());
    assert!(src.path().join("training_hooks.json").exists());
    assert!(src.path().join("journal_sessions.json").exists());
    assert!(src.path().join("treasury_ledger.json").exists());
    assert!(src.path().join("policies.json").exists());
    assert!(src.path().join("riskTiers.json").exists());

    Ok(())
}
