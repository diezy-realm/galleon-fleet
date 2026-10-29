//! Tests for Fleet snapshots (TASK-069, SPEC-013).

use std::fs;
use std::path::Path;

use galleon_fleet_core::{FleetError, FleetStore, MAX_SNAPSHOTS, SnapshotItem};

#[tokio::test]
async fn test_snapshot_checksum_changes_when_collection_changes() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    // 1. Initial collection state
    store
        .save_collection("ships", b"[{\"id\":\"ship-1\",\"name\":\"Galleon A\"}]")
        .await?;

    let snap1 = store.create_snapshot("Initial Fleet State").await?;
    assert_eq!(snap1.label, "Initial Fleet State");
    assert_eq!(snap1.entity_count, 1);
    assert_eq!(snap1.includes, vec!["ships"]);
    assert!(snap1.checksum.starts_with("sha256:"));

    // 2. Modifying the collection must produce a different checksum
    store
        .save_collection(
            "ships",
            b"[{\"id\":\"ship-1\",\"name\":\"Galleon A Modified\"}]",
        )
        .await?;

    let snap2 = store.create_snapshot("Modified Fleet State").await?;
    assert_eq!(snap2.label, "Modified Fleet State");
    assert_eq!(snap2.entity_count, 1);
    assert_ne!(
        snap1.checksum, snap2.checksum,
        "Checksum must change when collection content changes"
    );

    // 3. Adding another collection changes checksum and entityCount
    store
        .save_collection(
            "crew",
            b"[{\"id\":\"c-1\",\"name\":\"First Mate\"},{\"id\":\"c-2\",\"name\":\"Navigator\"}]",
        )
        .await?;

    let snap3 = store.create_snapshot("State With Crew").await?;
    assert_eq!(snap3.entity_count, 3); // 1 ship + 2 crew
    assert_eq!(snap3.includes, vec!["crew", "ships"]);
    assert_ne!(
        snap2.checksum, snap3.checksum,
        "Checksum must change when new collection is added"
    );

    Ok(())
}

#[tokio::test]
async fn test_snapshot_files_copied_to_snapshot_dir() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    store
        .save_collection("squads", b"[{\"id\":\"sq-1\",\"name\":\"Alpha\"}]")
        .await?;
    store
        .save_collection("quests", b"[{\"id\":\"q-1\",\"title\":\"Explore\"}]")
        .await?;

    let snap = store.create_snapshot("Multi-Collection Backup").await?;
    let snap_dir = store.snapshots_dir().join(&snap.id);

    assert!(snap_dir.is_dir(), "Snapshot directory must exist");
    assert!(
        snap_dir.join("squads.json").is_file(),
        "squads.json must be copied"
    );
    assert!(
        snap_dir.join("quests.json").is_file(),
        "quests.json must be copied"
    );
    assert!(
        snap_dir.join("snapshot.json").is_file(),
        "snapshot.json metadata must exist"
    );

    // Verify snapshot.json contains identical metadata
    let meta_content =
        fs::read_to_string(snap_dir.join("snapshot.json")).map_err(FleetError::Io)?;
    let parsed: SnapshotItem = serde_json::from_str(&meta_content).map_err(FleetError::Json)?;
    assert_eq!(parsed, snap);

    // Verify get_snapshot retrieves the same snapshot
    let retrieved = store.get_snapshot(&snap.id)?;
    assert_eq!(retrieved, Some(snap));

    Ok(())
}

#[tokio::test]
async fn test_snapshot_pruning_enforces_max_capacity() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    store
        .save_collection("artifacts", b"[{\"id\":\"art-1\"}]")
        .await?;

    // Create 23 snapshots (MAX_SNAPSHOTS is 20)
    let total_to_create = MAX_SNAPSHOTS + 3;
    let mut created_ids = Vec::new();

    for i in 0..total_to_create {
        let snap = store
            .create_snapshot(&format!("Snapshot batch {i}"))
            .await?;
        created_ids.push(snap.id);
    }

    let all_snaps = store.list_snapshots()?;
    assert_eq!(
        all_snaps.len(),
        MAX_SNAPSHOTS,
        "Snapshot store must retain exactly MAX_SNAPSHOTS ({MAX_SNAPSHOTS}) entries"
    );

    // Check that the newest snapshot is present and at the front (index 0)
    assert_eq!(all_snaps[0].id, created_ids[total_to_create - 1]);

    // Check that the oldest snapshots were pruned from disk
    let oldest_id = &created_ids[0];
    assert!(
        !store.snapshots_dir().join(oldest_id).exists(),
        "Oldest snapshot directory {oldest_id} must have been deleted"
    );

    let second_oldest_id = &created_ids[1];
    assert!(
        !store.snapshots_dir().join(second_oldest_id).exists(),
        "Second oldest snapshot directory {second_oldest_id} must have been deleted"
    );

    Ok(())
}

#[tokio::test]
async fn test_snapshot_json_shape_matches_go_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let store = FleetStore::new(tmp.path());

    store
        .save_collection("ships", b"[{\"id\":\"s-1\"}]")
        .await?;
    let snap = store.create_snapshot("Genesis Commissioning State").await?;

    let snap_val = serde_json::to_value(&snap)?;

    // Read Go fixture POST_api_snapshots.json
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent);
    if let Some(base) = base_dir {
        let fixture_path = base.join("tests/fixtures/go-fleet/POST_api_snapshots.json");
        if fixture_path.exists() {
            let fixture_bytes = fs::read(&fixture_path)?;
            let fixture_val: serde_json::Value = serde_json::from_slice(&fixture_bytes)?;

            // Verify all keys from fixture are present in Rust JSON
            let fixture_obj = fixture_val
                .as_object()
                .ok_or("fixture must be a JSON object")?;
            let snap_obj = snap_val
                .as_object()
                .ok_or("snapshot must be a JSON object")?;

            for key in fixture_obj.keys() {
                assert!(
                    snap_obj.contains_key(key),
                    "Rust snapshot JSON must contain key '{key}' from Go fixture"
                );
            }

            // Verify non-volatile types:
            // - label: string
            assert!(snap_obj["label"].is_string());
            // - sizeMB: number
            assert!(snap_obj["sizeMB"].is_number());
            // - entityCount: number
            assert!(snap_obj["entityCount"].is_number());
            // - includes: array
            assert!(snap_obj["includes"].is_array());

            // Verify volatile field shape:
            // - id: string starting with "snap-"
            let id_str = snap_obj["id"].as_str().ok_or("id must be a string")?;
            assert!(id_str.starts_with("snap-"));
            // - createdAt: valid ISO8601 string
            assert!(snap_obj["createdAt"].is_string());
            // - checksum: string starting with "sha256:"
            let checksum_str = snap_obj["checksum"]
                .as_str()
                .ok_or("checksum must be a string")?;
            assert!(checksum_str.starts_with("sha256:"));
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_empty_collection_snapshot() -> Result<(), FleetError> {
    let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
    let store = FleetStore::new(tmp.path());

    let snap = store.create_snapshot("Empty Initial State").await?;
    assert_eq!(snap.entity_count, 0);
    assert_eq!(snap.size_mb, 0.0);
    assert!(snap.includes.is_empty());
    assert!(snap.checksum.starts_with("sha256:"));

    let list = store.list_snapshots()?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, snap.id);

    Ok(())
}
