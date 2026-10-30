//! Storage engine for Galleon Fleet collections, snapshots, and legacy imports.
//!
//! Provides atomic file writes via temporary files and one-time migration import
//! from legacy Go data directories.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::FleetError;

/// Maximum payload size for collection files (1 MiB = 1,048,576 bytes).
pub const FLEET_MAX_BODY: usize = 1_048_576;

/// Canonical collection names (camelCase) supported by web dashboard and fleet core.
pub use crate::collections::CANONICAL_COLLECTIONS;

/// Resolves a filename stem (canonical or legacy snake_case) to its canonical camelCase name.
pub fn resolve_collection_name(file_stem: &str) -> Option<&'static str> {
    for &col in CANONICAL_COLLECTIONS {
        if file_stem == col {
            return Some(col);
        }
    }
    match file_stem {
        "chat_messages" => Some("chatMessages"),
        "global_steering" => Some("globalSteering"),
        "journal_sessions" => Some("journalSessions"),
        "steering_directives" => Some("steeringDirectives"),
        "training_hooks" => Some("trainingHooks"),
        "training_skills" => Some("trainingSkills"),
        "treasury_ledger" => Some("treasuryLedger"),
        "policies" => Some("policies"),
        "riskTiers" => Some("riskTiers"),
        _ => None,
    }
}

/// Details of a completed legacy import operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    /// Source directory from which files were copied.
    pub source: PathBuf,
    /// List of file names copied to the root directory.
    pub files_copied: Vec<String>,
    /// UTC timestamp of the import in ISO 8601 format.
    pub timestamp: String,
}

/// Atomic file store for fleet collections and snapshots.
#[derive(Debug, Clone)]
pub struct FleetStore {
    /// Root directory for fleet storage (`<Config.data_dir>/fleet`).
    pub root: PathBuf,
    /// Mutex for serializing writes across concurrent callers.
    pub(crate) write_lock: Arc<Mutex<()>>,
}

impl FleetStore {
    /// Create a new `FleetStore` rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Resolve the root directory with explicit environment/path parameters (pure and testable).
    pub fn resolve_root_with(
        galleon_data_dir: Option<&str>,
        data_dir_override: Option<&Path>,
        galleon_config_dir: Option<&str>,
        home_dir: Option<&str>,
    ) -> PathBuf {
        if let Some(data_dir) = galleon_data_dir {
            let trimmed = data_dir.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("fleet");
            }
        }

        if let Some(data_dir) = data_dir_override {
            return data_dir.join("fleet");
        }

        if let Some(config_dir) = galleon_config_dir {
            let trimmed = config_dir.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("data").join("fleet");
            }
        }

        if let Some(home) = home_dir {
            let trimmed = home.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed)
                    .join(".config")
                    .join("galleon")
                    .join("data")
                    .join("fleet");
            }
        }

        PathBuf::from("data").join("fleet")
    }

    /// Resolve the root directory, honoring `GALLEON_DATA_DIR` if set.
    ///
    /// Precedence:
    /// 1. `GALLEON_DATA_DIR` environment variable (`<GALLEON_DATA_DIR>/fleet`)
    /// 2. `data_dir_override` (`<data_dir_override>/fleet`)
    /// 3. `GALLEON_CONFIG_DIR` environment variable (`<GALLEON_CONFIG_DIR>/data/fleet`)
    /// 4. `HOME` (`~/.config/galleon/data/fleet`)
    /// 5. Fallback `./data/fleet`
    pub fn resolve_root(data_dir_override: Option<&Path>) -> PathBuf {
        let env_data = std::env::var("GALLEON_DATA_DIR").ok();
        let env_cfg = std::env::var("GALLEON_CONFIG_DIR").ok();
        let env_home = std::env::var("HOME").ok();
        Self::resolve_root_with(
            env_data.as_deref(),
            data_dir_override,
            env_cfg.as_deref(),
            env_home.as_deref(),
        )
    }

    /// Construct a `FleetStore` from a data directory, respecting `GALLEON_DATA_DIR`.
    pub fn from_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self::new(Self::resolve_root(Some(data_dir.as_ref())))
    }

    /// Construct a `FleetStore` from environment configuration.
    pub fn from_env() -> Self {
        Self::new(Self::resolve_root(None))
    }

    /// Construct and initialize a `FleetStore`, ensuring its directory exists
    /// and running the one-time legacy import if necessary.
    pub fn init(root: impl Into<PathBuf>) -> Result<Self, FleetError> {
        let store = Self::new(root);
        store.ensure_dir()?;
        let _ = store.import_legacy_if_needed()?;
        Ok(store)
    }

    /// Access the root path of the store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Ensure the root directory exists.
    pub fn ensure_dir(&self) -> Result<(), FleetError> {
        if !self.root.exists() {
            std::fs::create_dir_all(&self.root).map_err(FleetError::Io)?;
        }
        Ok(())
    }

    /// Path to a specific file in the store.
    pub fn file_path(&self, filename: &str) -> PathBuf {
        self.root.join(filename)
    }

    /// Path to a collection file in the store (`<root>/<name>.json`).
    pub fn collection_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.json"))
    }

    /// Check if a specific file exists in the store.
    pub fn exists(&self, filename: &str) -> bool {
        self.file_path(filename).exists()
    }

    /// Atomically write bytes to `<root>/<filename>` using a NamedTempFile in the same directory.
    ///
    /// Serialized via `write_lock`.
    /// Rejects payloads exceeding `FLEET_MAX_BODY` (1 MiB).
    pub async fn write_atomic(&self, filename: &str, data: &[u8]) -> Result<(), FleetError> {
        if data.len() > FLEET_MAX_BODY {
            return Err(FleetError::PayloadTooLarge {
                size: data.len(),
                limit: FLEET_MAX_BODY,
            });
        }

        self.ensure_dir()?;
        let dest_path = self.root.join(filename);

        let _guard = self.write_lock.lock().await;

        let mut temp = tempfile::NamedTempFile::new_in(&self.root).map_err(FleetError::Io)?;
        use std::io::Write;
        temp.write_all(data).map_err(FleetError::Io)?;
        temp.as_file().sync_all().map_err(FleetError::Io)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = temp
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600));
        }

        temp.persist(&dest_path)
            .map_err(|e| FleetError::Io(e.error))?;
        Ok(())
    }

    /// Synchronously and atomically write bytes to `<root>/<filename>`.
    pub fn write_atomic_blocking(&self, filename: &str, data: &[u8]) -> Result<(), FleetError> {
        if data.len() > FLEET_MAX_BODY {
            return Err(FleetError::PayloadTooLarge {
                size: data.len(),
                limit: FLEET_MAX_BODY,
            });
        }

        self.ensure_dir()?;
        let dest_path = self.root.join(filename);

        let _guard = self.write_lock.blocking_lock();

        let mut temp = tempfile::NamedTempFile::new_in(&self.root).map_err(FleetError::Io)?;
        use std::io::Write;
        temp.write_all(data).map_err(FleetError::Io)?;
        temp.as_file().sync_all().map_err(FleetError::Io)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = temp
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600));
        }

        temp.persist(&dest_path)
            .map_err(|e| FleetError::Io(e.error))?;
        Ok(())
    }

    /// Atomically write a collection JSON file.
    pub async fn write_collection(&self, name: &str, data: &[u8]) -> Result<(), FleetError> {
        let filename = format!("{name}.json");
        self.write_atomic(&filename, data).await
    }

    /// Read raw bytes of `<root>/<filename>`.
    pub fn read_file(&self, filename: &str) -> Result<Vec<u8>, FleetError> {
        let path = self.root.join(filename);
        if !path.exists() {
            return Err(FleetError::NotFound(filename.to_string()));
        }
        std::fs::read(&path).map_err(FleetError::Io)
    }

    /// Read a collection file if it exists.
    pub fn read_collection(&self, name: &str) -> Result<Option<Vec<u8>>, FleetError> {
        let filename = format!("{name}.json");
        let path = self.root.join(filename);
        if !path.exists() {
            return Ok(None);
        }
        std::fs::read(path).map(Some).map_err(FleetError::Io)
    }

    /// Read a collection by name with allowlist validation.
    ///
    /// Rejects names not in the allowlist with `FleetError::UnknownCollection`.
    /// Resolves canonical static name to prevent any path traversal.
    /// Returns `Ok(None)` if the file does not exist yet.
    pub fn get_collection(&self, name: &str) -> Result<Option<Vec<u8>>, FleetError> {
        let canonical = crate::collections::resolve_allowed_collection(name)
            .ok_or_else(|| FleetError::UnknownCollection(name.to_string()))?;
        self.read_collection(canonical)
    }

    /// Save a collection with allowlist and body validation.
    ///
    /// - Rejects names not in the allowlist with `FleetError::UnknownCollection`.
    /// - Rejects bodies that exceed `FLEET_MAX_BODY` with `FleetError::PayloadTooLarge`.
    /// - Rejects bodies that are not valid JSON arrays with `FleetError::NotAJsonArray`.
    /// - Resolves canonical static name to prevent any path traversal.
    /// - Atomically writes data to `<root>/<canonical>.json`.
    pub async fn save_collection(&self, name: &str, data: &[u8]) -> Result<(), FleetError> {
        let canonical = crate::collections::resolve_allowed_collection(name)
            .ok_or_else(|| FleetError::UnknownCollection(name.to_string()))?;
        crate::collections::validate_collection_payload(data)?;
        self.write_collection(canonical, data).await
    }

    /// Synchronously save a collection with allowlist and body validation.
    pub fn save_collection_blocking(&self, name: &str, data: &[u8]) -> Result<(), FleetError> {
        let canonical = crate::collections::resolve_allowed_collection(name)
            .ok_or_else(|| FleetError::UnknownCollection(name.to_string()))?;
        crate::collections::validate_collection_payload(data)?;
        let filename = format!("{canonical}.json");
        self.write_atomic_blocking(&filename, data)
    }

    /// Count the number of entries in a stored collection.
    ///
    /// If the collection does not exist, cannot be read, or is not a valid JSON array,
    /// returns 0 without panicking.
    pub fn count_collection(&self, name: &str) -> usize {
        match self.get_collection(name) {
            Ok(Some(bytes)) => {
                if let Ok(serde_json::Value::Array(arr)) = serde_json::from_slice(&bytes) {
                    arr.len()
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Count entries in a stored collection whose `"status"` property
    /// matches any of the given `wanted_statuses`.
    ///
    /// If the collection does not exist, cannot be read, or is not a valid JSON array,
    /// returns 0 without panicking.
    pub fn count_by_status(&self, name: &str, wanted_statuses: &[&str]) -> usize {
        match self.get_collection(name) {
            Ok(Some(bytes)) => {
                if let Ok(serde_json::Value::Array(arr)) = serde_json::from_slice(&bytes) {
                    arr.iter()
                        .filter(|item| {
                            item.get("status")
                                .and_then(|v| v.as_str())
                                .map(|s| wanted_statuses.contains(&s))
                                .unwrap_or(false)
                        })
                        .count()
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Calculate a fleet metrics snapshot from this store with optional RSS memory usage.
    pub fn get_metrics(&self, memory_used_mb: Option<f64>) -> crate::metrics::FleetMetrics {
        crate::metrics::calculate_fleet_metrics(self, memory_used_mb)
    }

    /// Calculate an executive briefing report snapshot from this store with given date and timestamp.
    pub fn get_executive_briefing(
        &self,
        date: String,
        generated_at: String,
    ) -> crate::briefing::ExecutiveBriefing {
        crate::briefing::calculate_executive_briefing(self, date, generated_at)
    }

    /// Retrieve fleet policies and risk tiers from this store.
    ///
    /// Reads `<root>/policies.json` and `<root>/riskTiers.json` if they exist in the
    /// store, falling back to the canonical embedded seed definitions if absent.
    pub fn get_policies(&self) -> Result<crate::policies::FleetPoliciesResponse, FleetError> {
        let policies = match self.read_collection("policies")? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => crate::policies::load_seed_policies()?,
        };
        let risk_tiers = match self.read_collection("riskTiers")? {
            Some(bytes) => serde_json::from_slice(&bytes)?,
            None => crate::policies::load_seed_risk_tiers()?,
        };
        Ok(crate::policies::FleetPoliciesResponse {
            policies,
            risk_tiers,
        })
    }

    /// Check if the store has already been marked as imported.
    pub fn is_imported(&self) -> bool {
        self.root.join(".imported").exists()
    }

    /// Run the one-time legacy import using candidate sources in order:
    /// 1. `FLEET_DATA_DIR`
    /// 2. `GALLEON_DATA_DIR`
    /// 3. `<cwd>/engine/data` (or `./engine/data`)
    ///
    /// Skips import if `.imported` marker exists.
    pub fn import_legacy_if_needed(&self) -> Result<Option<ImportReport>, FleetError> {
        if self.is_imported() {
            return Ok(None);
        }

        let mut candidates = Vec::new();
        if let Ok(dir) = std::env::var("FLEET_DATA_DIR") {
            let p = PathBuf::from(dir.trim());
            if !p.as_os_str().is_empty() {
                candidates.push(p);
            }
        }
        if let Ok(dir) = std::env::var("GALLEON_DATA_DIR") {
            let p = PathBuf::from(dir.trim());
            if !p.as_os_str().is_empty() {
                candidates.push(p);
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("engine").join("data"));
        }
        candidates.push(PathBuf::from("engine").join("data"));

        self.import_legacy_from_candidates(&candidates)
    }

    /// Run the one-time legacy import checking the given candidates in order.
    pub fn import_legacy_from_candidates(
        &self,
        candidates: &[PathBuf],
    ) -> Result<Option<ImportReport>, FleetError> {
        if self.is_imported() {
            return Ok(None);
        }

        let Some(source_dir) = candidates.iter().find(|p| p.is_dir()) else {
            return Ok(None);
        };

        self.import_from_dir(source_dir).map(Some)
    }

    /// Import collections from a specific source directory.
    ///
    /// - Copies `<collection>.json` top-level files without deleting source files.
    /// - Only copies files if they do not already exist in `root`.
    /// - Writes `<root>/.imported` marker containing source path and UTC ISO 8601 timestamp.
    /// - Returns `Err` if `.imported` marker is already present.
    pub fn import_from_dir(&self, source_dir: &Path) -> Result<ImportReport, FleetError> {
        if self.is_imported() {
            return Err(FleetError::Custom(
                "import already performed (.imported marker exists)".to_string(),
            ));
        }

        self.ensure_dir()?;
        let mut copied = Vec::new();

        let entries = std::fs::read_dir(source_dir).map_err(FleetError::Io)?;
        for entry in entries {
            let entry = entry.map_err(FleetError::Io)?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some(canonical) = resolve_collection_name(stem) else {
                continue;
            };

            let dest_name = format!("{canonical}.json");
            let dest = self.root.join(&dest_name);
            if !dest.exists() {
                std::fs::copy(&path, &dest).map_err(FleetError::Io)?;
                copied.push(dest_name);
            }
        }

        // Sort copied list for deterministic reports
        copied.sort();

        let now_str = current_timestamp_iso8601();
        let marker_content = format!(
            "source: {}\nimported_at: {}\n",
            source_dir.display(),
            now_str
        );

        // Atomically write the .imported marker
        let marker_path = self.root.join(".imported");
        let mut temp = tempfile::NamedTempFile::new_in(&self.root).map_err(FleetError::Io)?;
        use std::io::Write;
        temp.write_all(marker_content.as_bytes())
            .map_err(FleetError::Io)?;
        temp.as_file().sync_all().map_err(FleetError::Io)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = temp
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600));
        }

        temp.persist(&marker_path)
            .map_err(|e| FleetError::Io(e.error))?;

        Ok(ImportReport {
            source: source_dir.to_path_buf(),
            files_copied: copied,
            timestamp: now_str,
        })
    }
}

/// Helper converting current system time to a UTC ISO 8601 formatted string without external crates.
pub(crate) fn current_timestamp_iso8601() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    let days = (secs / 86400) as i64;
    let rem_secs = (secs % 86400) as u32;

    let hour = rem_secs / 3600;
    let minute = (rem_secs % 3600) / 60;
    let second = rem_secs % 60;

    let (year, month, day) = days_to_civil(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Civil day conversion from epoch days using the Howard Hinnant algorithm.
fn days_to_civil(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1024 + doe / 1461 - doe / 142456) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_atomic_write_and_read() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = FleetStore::new(tmp.path().join("fleet"));

        let payload = br#"[{"id":"ship-1","name":"Black Pearl"}]"#;
        store.write_atomic("ships.json", payload).await?;

        let read_back = store.read_file("ships.json")?;
        assert_eq!(read_back, payload);

        let collection_read = store.read_collection("ships")?;
        assert_eq!(collection_read, Some(payload.to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn test_atomic_write_exceeds_limit() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = FleetStore::new(tmp.path().join("fleet"));

        let oversized = vec![b'a'; FLEET_MAX_BODY + 1];
        let res = store.write_atomic("ships.json", &oversized).await;
        match res {
            Err(FleetError::PayloadTooLarge { size, limit }) => {
                assert_eq!(size, FLEET_MAX_BODY + 1);
                assert_eq!(limit, FLEET_MAX_BODY);
            }
            other => panic!("expected PayloadTooLarge, got: {other:?}"),
        }

        assert!(!store.exists("ships.json"));
        Ok(())
    }

    #[test]
    fn test_atomic_write_blocking() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = FleetStore::new(tmp.path().join("fleet"));

        let payload = br#"[{"id":"crew-1","name":"Jack Sparrow"}]"#;
        store.write_atomic_blocking("crew.json", payload)?;

        let read_back = store.read_file("crew.json")?;
        assert_eq!(read_back, payload);
        Ok(())
    }

    #[test]
    fn test_legacy_import_copies_and_preserves_source() -> Result<(), Box<dyn std::error::Error>> {
        let src_tmp = tempfile::tempdir()?;
        let dst_tmp = tempfile::tempdir()?;
        let store = FleetStore::new(dst_tmp.path().join("fleet"));

        // Setup source directory with canonical, legacy snake_case, and non-collection files
        let crew_src = src_tmp.path().join("crew.json");
        std::fs::write(&crew_src, br#"[{"id":"c1"}]"#)?;

        let chat_src = src_tmp.path().join("chat_messages.json");
        std::fs::write(&chat_src, br#"[{"id":"m1"}]"#)?;

        let skills_src = src_tmp.path().join("training_skills.json");
        std::fs::write(&skills_src, br#"[{"id":"s1"}]"#)?;

        let unrelated = src_tmp.path().join("unrelated.txt");
        std::fs::write(&unrelated, b"not a collection")?;

        let report = store.import_from_dir(src_tmp.path())?;
        assert_eq!(report.source, src_tmp.path());
        assert!(report.files_copied.contains(&"crew.json".to_string()));
        assert!(
            report
                .files_copied
                .contains(&"chatMessages.json".to_string())
        );
        assert!(
            report
                .files_copied
                .contains(&"trainingSkills.json".to_string())
        );

        // Check destination files exist
        assert_eq!(store.read_file("crew.json")?, br#"[{"id":"c1"}]"#);
        assert_eq!(store.read_file("chatMessages.json")?, br#"[{"id":"m1"}]"#);
        assert_eq!(store.read_file("trainingSkills.json")?, br#"[{"id":"s1"}]"#);
        assert!(!store.exists("unrelated.txt"));

        // CRITICAL: Verify source files were NOT deleted or moved
        assert!(crew_src.exists());
        assert!(chat_src.exists());
        assert!(skills_src.exists());
        assert!(unrelated.exists());

        // Verify .imported marker
        assert!(store.is_imported());
        let marker = store.read_file(".imported")?;
        let marker_text = String::from_utf8(marker)?;
        assert!(marker_text.contains(&src_tmp.path().display().to_string()));
        assert!(marker_text.contains("imported_at: "));
        Ok(())
    }

    #[test]
    fn test_legacy_import_does_not_repeat_and_preserves_existing()
    -> Result<(), Box<dyn std::error::Error>> {
        let src_tmp = tempfile::tempdir()?;
        let dst_tmp = tempfile::tempdir()?;
        let store = FleetStore::new(dst_tmp.path().join("fleet"));
        store.ensure_dir()?;

        // Pre-create chatMessages.json in store root
        store.write_atomic_blocking("chatMessages.json", br#"[{"id":"existing"}]"#)?;

        // Source has chat_messages.json with older/different content
        let chat_src = src_tmp.path().join("chat_messages.json");
        std::fs::write(&chat_src, br#"[{"id":"legacy"}]"#)?;

        let report = store.import_from_dir(src_tmp.path())?;
        // Since chatMessages.json already existed, it should not have been overwritten
        assert!(
            !report
                .files_copied
                .contains(&"chatMessages.json".to_string())
        );
        assert_eq!(
            store.read_file("chatMessages.json")?,
            br#"[{"id":"existing"}]"#
        );

        // Second import attempt should be skipped
        assert!(store.is_imported());
        let second_result = store.import_legacy_from_candidates(&[src_tmp.path().to_path_buf()])?;
        assert_eq!(second_result, None);
        Ok(())
    }

    #[test]
    fn test_galleon_data_dir_resolution() {
        let custom_dir = "/tmp/test_custom_galleon_data_dir";
        let root = FleetStore::resolve_root_with(Some(custom_dir), None, None, None);
        assert_eq!(root, PathBuf::from(custom_dir).join("fleet"));

        let override_dir = Path::new("/var/override");
        let root_override = FleetStore::resolve_root_with(None, Some(override_dir), None, None);
        assert_eq!(root_override, override_dir.join("fleet"));

        let root_precedence =
            FleetStore::resolve_root_with(Some(custom_dir), Some(override_dir), None, None);
        assert_eq!(root_precedence, PathBuf::from(custom_dir).join("fleet"));
    }

    #[test]
    fn test_timestamp_iso8601_format() {
        let ts = current_timestamp_iso8601();
        assert_eq!(ts.len(), 20); // "YYYY-MM-DDTHH:MM:SSZ"
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[10..11], "T");
    }

    #[tokio::test]
    async fn test_store_get_and_save_collection() -> Result<(), FleetError> {
        let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
        let store = FleetStore::new(tmp.path());

        // Initially, valid collection does not exist
        assert_eq!(store.get_collection("squads")?, None);

        // Unknown collection rejected with UnknownCollection
        assert!(matches!(
            store.get_collection("notes"),
            Err(FleetError::UnknownCollection(_))
        ));

        // Save valid collection
        let sample = br#"[{"id":"squad-1","name":"Core Software Delivery"}]"#;
        store.save_collection("squads", sample).await?;

        // Read back
        let data = store.get_collection("squads")?;
        assert_eq!(data, Some(sample.to_vec()));

        // Save with non-array rejected
        assert!(matches!(
            store
                .save_collection("squads", br#"{"object":"not-array"}"#)
                .await,
            Err(FleetError::NotAJsonArray)
        ));

        // Save with path traversal rejected
        assert!(matches!(
            store.save_collection("../../evil", sample).await,
            Err(FleetError::UnknownCollection(_))
        ));

        Ok(())
    }

    #[test]
    fn test_store_save_collection_blocking() -> Result<(), FleetError> {
        let tmp = tempfile::tempdir().map_err(FleetError::Io)?;
        let store = FleetStore::new(tmp.path());

        let sample = br#"[{"id":"ship-1"}]"#;
        store.save_collection_blocking("ships", sample)?;

        let data = store.get_collection("ships")?;
        assert_eq!(data, Some(sample.to_vec()));
        Ok(())
    }
}
