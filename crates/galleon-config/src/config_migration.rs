//! First-boot user data migration from legacy `~/.clawcrew` to canonical `~/.config/galleon`.
//!
//! Per SPEC-009 / ODR-19 (T3-18 / RB-23):
//! 1. Copies all files and subdirectories from `~/.clawcrew` into `~/.config/galleon` atomically.
//! 2. Leaves a `.migrated` marker file in both `~/.clawcrew/.migrated` and `~/.config/galleon/.migrated`.
//! 3. Preserves the legacy directory (does not delete it).
//! 4. Does not perform runtime dual-read: once migrated, Galleon operates strictly from `~/.config/galleon`.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Returns the legacy config directory `~/.clawcrew` (or Windows `%USERPROFILE%\.clawcrew`).
pub fn default_legacy_config_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let home = home.trim();
        if !home.is_empty() {
            return Some(PathBuf::from(home).join(".clawcrew"));
        }
    }
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let profile = profile.trim();
            if !profile.is_empty() {
                return Some(PathBuf::from(profile).join(".clawcrew"));
            }
        }
    }
    directories::UserDirs::new().map(|u| u.home_dir().join(".clawcrew"))
}

/// Recursively copies directory contents atomically into `target_dir` using a temporary directory.
///
/// Returns `Ok(true)` if migration was performed, or `Ok(false)` if migration was skipped
/// (e.g., target already populated, legacy dir does not exist or was already migrated).
pub fn migrate_config_directory(legacy_dir: &Path, target_dir: &Path) -> Result<bool> {
    if !legacy_dir.exists() || !legacy_dir.is_dir() {
        return Ok(false);
    }

    // Check if legacy directory is already marked as migrated
    let legacy_marker = legacy_dir.join(".migrated");
    if legacy_marker.exists() {
        return Ok(false);
    }

    // Check if legacy directory has any files to migrate
    let legacy_entries = match fs::read_dir(legacy_dir) {
        Ok(entries) => entries
            .filter_map(std::result::Result::ok)
            .collect::<Vec<_>>(),
        Err(_) => return Ok(false),
    };
    if legacy_entries.is_empty() {
        return Ok(false);
    }

    // Check target directory: if it already exists and is not empty, do not overwrite
    if target_dir.exists() {
        let target_entries = match fs::read_dir(target_dir) {
            Ok(entries) => entries
                .filter_map(std::result::Result::ok)
                .filter(|e| e.file_name() != ".migrated")
                .collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        };
        if !target_entries.is_empty() {
            return Ok(false);
        }
    }

    // Ensure target's parent directory exists
    let target_parent = target_dir
        .parent()
        .context("Target directory has no parent")?;
    fs::create_dir_all(target_parent)
        .with_context(|| format!("Failed to create parent dir {}", target_parent.display()))?;

    // Create a temporary staging directory in the same filesystem/parent
    let tmp_dir_name = format!(
        ".galleon_migration_tmp_{}_{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    );
    let tmp_dir = target_parent.join(tmp_dir_name);
    fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("Failed to create staging dir {}", tmp_dir.display()))?;

    // Perform recursive copy into tmp_dir
    if let Err(e) = copy_dir_all(legacy_dir, &tmp_dir) {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(e).context("Failed copying legacy directory to staging");
    }

    // Write .migrated marker in staging dir
    let marker_content = format!(
        "migrated = true\nsource = \"{}\"\ntarget = \"{}\"\ntimestamp = \"{}\"\n",
        legacy_dir.display(),
        target_dir.display(),
        chrono::Utc::now().to_rfc3339()
    );
    let tmp_marker = tmp_dir.join(".migrated");
    if let Err(e) = fs::write(&tmp_marker, &marker_content) {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(e).context("Failed writing marker in staging directory");
    }

    // If target_dir exists (empty), remove it before rename
    if target_dir.exists() {
        let _ = fs::remove_dir_all(target_dir);
    }

    // Atomically move tmp_dir to target_dir
    if let Err(e) = fs::rename(&tmp_dir, target_dir) {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(e).with_context(|| {
            format!(
                "Failed to atomically rename {} to {}",
                tmp_dir.display(),
                target_dir.display()
            )
        });
    }

    // Mark legacy directory with .migrated as well
    let _ = fs::write(legacy_marker, marker_content);

    ::galleon_log::record!(
        INFO,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
            .with_outcome(::galleon_log::EventOutcome::Success)
            .with_attrs(::serde_json::json!({
                "source": legacy_dir.display().to_string(),
                "target": target_dir.display().to_string(),
            })),
        "Migrated legacy configuration from ~/.clawcrew to ~/.config/galleon"
    );

    Ok(true)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        if file_name == ".migrated" {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&file_name);

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "Failed to copy file {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_migration_atomic_copy_from_legacy_dir() {
        let temp = tempdir().unwrap();
        let legacy = temp.path().join(".clawcrew");
        let target = temp.path().join(".config").join("galleon");

        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("config.toml"), "[gateway]\nport = 8080\n").unwrap();
        fs::create_dir_all(legacy.join("data")).unwrap();
        fs::write(legacy.join("data").join("state.db"), "mock db bytes").unwrap();

        let migrated = migrate_config_directory(&legacy, &target).expect("migration runs");
        assert!(migrated, "migration should have succeeded");

        assert!(target.exists(), "target directory must exist");
        assert_eq!(
            fs::read_to_string(target.join("config.toml")).unwrap(),
            "[gateway]\nport = 8080\n"
        );
        assert_eq!(
            fs::read_to_string(target.join("data").join("state.db")).unwrap(),
            "mock db bytes"
        );
        assert!(target.join(".migrated").is_file());
        assert!(legacy.join(".migrated").is_file());
        // Original legacy directory is preserved (not deleted)
        assert!(legacy.exists());
    }

    #[test]
    fn test_config_migration_skip_if_already_migrated() {
        let temp = tempdir().unwrap();
        let legacy = temp.path().join(".clawcrew");
        let target = temp.path().join(".config").join("galleon");

        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("config.toml"), "version = 1\n").unwrap();
        fs::write(legacy.join(".migrated"), "migrated = true\n").unwrap();

        let migrated = migrate_config_directory(&legacy, &target).expect("migration runs");
        assert!(!migrated, "should skip migration if already migrated");
        assert!(!target.exists(), "target should not be created");
    }

    #[test]
    fn test_config_migration_skip_if_target_has_content() {
        let temp = tempdir().unwrap();
        let legacy = temp.path().join(".clawcrew");
        let target = temp.path().join(".config").join("galleon");

        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("config.toml"), "legacy = true\n").unwrap();

        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("config.toml"), "existing = true\n").unwrap();

        let migrated = migrate_config_directory(&legacy, &target).expect("migration runs");
        assert!(!migrated, "should not overwrite non-empty target");
        assert_eq!(
            fs::read_to_string(target.join("config.toml")).unwrap(),
            "existing = true\n"
        );
    }

    #[test]
    fn test_config_migration_handles_missing_legacy_dir() {
        let temp = tempdir().unwrap();
        let legacy = temp.path().join("does_not_exist");
        let target = temp.path().join(".config").join("galleon");

        let migrated = migrate_config_directory(&legacy, &target).expect("runs cleanly");
        assert!(!migrated);
        assert!(!target.exists());
    }

    #[test]
    fn test_config_migration_handles_empty_legacy_dir() {
        let temp = tempdir().unwrap();
        let legacy = temp.path().join(".clawcrew");
        let target = temp.path().join(".config").join("galleon");
        fs::create_dir_all(&legacy).unwrap();

        let migrated = migrate_config_directory(&legacy, &target).expect("runs cleanly");
        assert!(!migrated);
    }
}
