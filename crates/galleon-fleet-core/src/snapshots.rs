//! Fleet snapshot models and operations.
//!
//! Under SPEC-013 §Snapshot (default Q2 PRD):
//! - `POST /api/snapshots {label}` copies collection files to `root/snapshots/<id>/`
//!   and calculates real `sizeMB`, `entityCount`, and SHA-256 `checksum`.
//! - `GET /api/snapshots` reads the directory, returning items sorted newest-first.
//! - Enforces `MAX_SNAPSHOTS = 20` capacity limit by deleting oldest snapshots.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::collections::{CANONICAL_COLLECTIONS, POLICY_DATA_COLLECTIONS};
use crate::error::FleetError;
use crate::store::{FleetStore, current_timestamp_iso8601};

/// Maximum number of snapshots preserved in the store before oldest are pruned.
pub const MAX_SNAPSHOTS: usize = 20;

/// A point-in-time snapshot of the fleet collection state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotItem {
    /// Unique snapshot identifier formatted as `snap-<timestamp>`.
    pub id: String,
    /// ISO 8601 / RFC 3339 creation timestamp.
    pub created_at: String,
    /// Human-readable snapshot label.
    pub label: String,
    /// Combined disk size in megabytes of all included collections.
    #[serde(rename = "sizeMB")]
    pub size_mb: f64,
    /// Total count of entities across all included collections.
    pub entity_count: usize,
    /// SHA-256 checksum formatted as `sha256:<hex>` derived from collection names and content.
    pub checksum: String,
    /// List of collection names included in this snapshot.
    pub includes: Vec<String>,
}

/// Request payload for creating a snapshot.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CreateSnapshotRequest {
    /// Preferred label for the snapshot.
    pub label: Option<String>,
    /// Fallback title accepted for web/desktop backward compatibility.
    pub title: Option<String>,
}

impl FleetStore {
    /// Return the path to the snapshots directory (`<root>/snapshots`).
    pub fn snapshots_dir(&self) -> PathBuf {
        self.root.join("snapshots")
    }

    /// Asynchronously create a snapshot of all collection files currently in `<root>`.
    ///
    /// Copies collection files to `<root>/snapshots/<id>/`, computes real metrics,
    /// writes `snapshot.json` metadata, and prunes older snapshots beyond `MAX_SNAPSHOTS`.
    pub async fn create_snapshot(&self, label: &str) -> Result<SnapshotItem, FleetError> {
        self.ensure_dir()?;
        let _guard = self.write_lock.lock().await;
        self.create_snapshot_inner(label)
    }

    /// Synchronously create a snapshot of all collection files currently in `<root>`.
    pub fn create_snapshot_blocking(&self, label: &str) -> Result<SnapshotItem, FleetError> {
        self.ensure_dir()?;
        let _guard = self.write_lock.blocking_lock();
        self.create_snapshot_inner(label)
    }

    fn create_snapshot_inner(&self, label: &str) -> Result<SnapshotItem, FleetError> {
        let snaps_dir = self.snapshots_dir();
        if !snaps_dir.exists() {
            std::fs::create_dir_all(&snaps_dir).map_err(FleetError::Io)?;
        }

        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();
        let nanos = duration.subsec_nanos();
        let mut id = format!("snap-{secs}{nanos:09}");
        let mut target_dir = snaps_dir.join(&id);

        let mut counter = 1;
        while target_dir.exists() {
            id = format!("snap-{secs}{nanos:09}-{counter}");
            target_dir = snaps_dir.join(&id);
            counter += 1;
        }

        std::fs::create_dir_all(&target_dir).map_err(FleetError::Io)?;

        // Find candidate collection files in self.root
        let mut candidate_collections: Vec<&str> = CANONICAL_COLLECTIONS
            .iter()
            .chain(POLICY_DATA_COLLECTIONS.iter())
            .copied()
            .collect();
        candidate_collections.sort_unstable();
        candidate_collections.dedup();

        let mut includes = Vec::new();
        let mut total_bytes: u64 = 0;
        let mut entity_count: usize = 0;
        let mut hasher = Sha256::new();

        for col_name in candidate_collections {
            let col_file = self.root.join(format!("{col_name}.json"));
            if col_file.is_file() {
                let data = std::fs::read(&col_file).map_err(FleetError::Io)?;
                let dest_file = target_dir.join(format!("{col_name}.json"));
                std::fs::write(&dest_file, &data).map_err(FleetError::Io)?;

                total_bytes += data.len() as u64;
                hasher.update(col_name.as_bytes());
                hasher.update(&data);
                includes.push(col_name.to_string());

                if let Ok(serde_json::Value::Array(arr)) =
                    serde_json::from_slice::<serde_json::Value>(&data)
                {
                    entity_count += arr.len();
                } else if let Ok(serde_json::Value::Object(_)) =
                    serde_json::from_slice::<serde_json::Value>(&data)
                {
                    entity_count += 1;
                }
            }
        }

        let mb = total_bytes as f64 / 1_048_576.0;
        let size_mb = (mb * 10.0).round() / 10.0;
        let checksum = format!("sha256:{:x}", hasher.finalize());

        let snapshot = SnapshotItem {
            id,
            created_at: current_timestamp_iso8601(),
            label: label.to_string(),
            size_mb,
            entity_count,
            checksum,
            includes,
        };

        // Persist snapshot.json metadata atomically in target_dir
        let meta_bytes = serde_json::to_vec_pretty(&snapshot).map_err(FleetError::Json)?;
        let dest_meta = target_dir.join("snapshot.json");
        let mut temp = tempfile::NamedTempFile::new_in(&target_dir).map_err(FleetError::Io)?;
        use std::io::Write;
        temp.write_all(&meta_bytes).map_err(FleetError::Io)?;
        temp.as_file().sync_all().map_err(FleetError::Io)?;
        temp.persist(&dest_meta)
            .map_err(|e| FleetError::Io(e.error))?;

        // Prune older snapshots if exceeding MAX_SNAPSHOTS
        let _ = self.prune_old_snapshots_inner();

        Ok(snapshot)
    }

    /// List all snapshots in the store, sorted newest-first.
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotItem>, FleetError> {
        let snaps_dir = self.snapshots_dir();
        if !snaps_dir.exists() {
            return Ok(Vec::new());
        }

        let mut items = Vec::new();
        let entries = std::fs::read_dir(&snaps_dir).map_err(FleetError::Io)?;

        for entry in entries {
            let entry = entry.map_err(FleetError::Io)?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let meta_file = path.join("snapshot.json");
            let maybe_item = std::fs::read(&meta_file)
                .ok()
                .and_then(|data| serde_json::from_slice::<SnapshotItem>(&data).ok());
            if let Some(item) = maybe_item {
                items.push(item);
                continue;
            }

            let maybe_reconstructed = path
                .file_name()
                .and_then(|s| s.to_str())
                .filter(|s| s.starts_with("snap-"))
                .and_then(|dir_name| self.reconstruct_snapshot_from_dir(&path, dir_name).ok());
            if let Some(item) = maybe_reconstructed {
                items.push(item);
            }
        }

        items.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });

        Ok(items)
    }

    /// Retrieve a specific snapshot by its ID.
    pub fn get_snapshot(&self, id: &str) -> Result<Option<SnapshotItem>, FleetError> {
        let snap_dir = self.snapshots_dir().join(id);
        if !snap_dir.is_dir() {
            return Ok(None);
        }

        let meta_file = snap_dir.join("snapshot.json");
        if meta_file.is_file() {
            let data = std::fs::read(&meta_file).map_err(FleetError::Io)?;
            let item = serde_json::from_slice::<SnapshotItem>(&data).map_err(FleetError::Json)?;
            return Ok(Some(item));
        }

        let item = self.reconstruct_snapshot_from_dir(&snap_dir, id)?;
        Ok(Some(item))
    }

    /// Prune older snapshots if exceeding `MAX_SNAPSHOTS` (20).
    pub fn prune_old_snapshots(&self) -> Result<usize, FleetError> {
        let _guard = self.write_lock.blocking_lock();
        self.prune_old_snapshots_inner()
    }

    fn prune_old_snapshots_inner(&self) -> Result<usize, FleetError> {
        let snaps_dir = self.snapshots_dir();
        if !snaps_dir.exists() {
            return Ok(0);
        }

        let items = self.list_snapshots()?;
        if items.len() <= MAX_SNAPSHOTS {
            return Ok(0);
        }

        let mut pruned = 0;
        for item in &items[MAX_SNAPSHOTS..] {
            let dir = snaps_dir.join(&item.id);
            if dir.exists() && std::fs::remove_dir_all(&dir).is_ok() {
                pruned += 1;
            }
        }

        Ok(pruned)
    }

    fn reconstruct_snapshot_from_dir(
        &self,
        dir_path: &Path,
        id: &str,
    ) -> Result<SnapshotItem, FleetError> {
        let mut candidate_collections: Vec<&str> = CANONICAL_COLLECTIONS
            .iter()
            .chain(POLICY_DATA_COLLECTIONS.iter())
            .copied()
            .collect();
        candidate_collections.sort_unstable();
        candidate_collections.dedup();

        let mut includes = Vec::new();
        let mut total_bytes: u64 = 0;
        let mut entity_count: usize = 0;
        let mut hasher = Sha256::new();

        for col_name in candidate_collections {
            let col_file = dir_path.join(format!("{col_name}.json"));
            if col_file.is_file() {
                let data = std::fs::read(&col_file).map_err(FleetError::Io)?;
                total_bytes += data.len() as u64;
                hasher.update(col_name.as_bytes());
                hasher.update(&data);
                includes.push(col_name.to_string());

                if let Ok(serde_json::Value::Array(arr)) =
                    serde_json::from_slice::<serde_json::Value>(&data)
                {
                    entity_count += arr.len();
                } else if let Ok(serde_json::Value::Object(_)) =
                    serde_json::from_slice::<serde_json::Value>(&data)
                {
                    entity_count += 1;
                }
            }
        }

        let mb = total_bytes as f64 / 1_048_576.0;
        let size_mb = (mb * 10.0).round() / 10.0;
        let checksum = format!("sha256:{:x}", hasher.finalize());

        Ok(SnapshotItem {
            id: id.to_string(),
            created_at: current_timestamp_iso8601(),
            label: id.to_string(),
            size_mb,
            entity_count,
            checksum,
            includes,
        })
    }
}
