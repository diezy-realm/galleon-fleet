use galleon_config::config_migration::{default_legacy_config_dir, migrate_config_directory};
use std::fs;
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
    assert!(legacy.exists(), "legacy directory must be preserved");
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

#[test]
fn test_config_migration_legacy_dir_resolution() {
    let _dir = default_legacy_config_dir();
}
