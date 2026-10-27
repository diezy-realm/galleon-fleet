//! Tests for SQLite WAL mode checkpointing, database backup, state migration,
//! and zero-data-loss verification (SPEC-008 / T3-21 / BL-11).

use clawcrew_memory::{
    Memory, MemoryCategory, SqliteMemory, WalCheckpointMode, backup_database_from_connection,
    backup_database_with_wal_checkpoint, checkpoint_wal, checkpoint_wal_truncate,
    compute_table_checksum, ensure_wal_mode, is_wal_mode, migrate_database_file,
    verify_database_integrity,
};
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_wal_mode_confirmed_for_memory_and_cron() {
    let tmp = TempDir::new().unwrap();
    let workspace = tmp.path();

    // 1. Verify memory SQLite database (brain.db) initializes in WAL mode
    let memory = SqliteMemory::new("test-agent", workspace).unwrap();
    memory
        .store("key-1", "wal-content-1", MemoryCategory::Core, None)
        .await
        .unwrap();

    let brain_path = workspace.join("memory").join("brain.db");
    assert!(brain_path.exists(), "brain.db must exist");

    let brain_conn = Connection::open(&brain_path).unwrap();
    assert!(
        is_wal_mode(&brain_conn).unwrap(),
        "brain.db must be in WAL journal mode"
    );
    let journal_mode: String = brain_conn
        .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        journal_mode.to_lowercase(),
        "wal",
        "brain.db journal_mode must be 'wal'"
    );

    // 2. Verify cron SQLite database (jobs.db) initializes in WAL mode
    let cron_dir = workspace.join("cron");
    fs::create_dir_all(&cron_dir).unwrap();
    let jobs_path = cron_dir.join("jobs.db");

    let cron_conn = Connection::open(&jobs_path).unwrap();
    cron_conn
        .execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS cron_jobs (
                id TEXT PRIMARY KEY,
                expression TEXT NOT NULL,
                command TEXT NOT NULL
             );",
        )
        .unwrap();

    assert!(
        is_wal_mode(&cron_conn).unwrap(),
        "jobs.db must be in WAL journal mode"
    );
    let cron_journal_mode: String = cron_conn
        .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        cron_journal_mode.to_lowercase(),
        "wal",
        "jobs.db journal_mode must be 'wal'"
    );
}

#[test]
fn test_wal_checkpoint_truncate_procedure_clears_log() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test_wal.db");
    let wal_path = tmp.path().join("test_wal.db-wal");

    let conn = Connection::open(&db_path).unwrap();
    ensure_wal_mode(&conn).unwrap();

    // Disable autocheckpoint so all written pages remain strictly in the WAL file
    conn.execute_batch("PRAGMA wal_autocheckpoint = 0;")
        .unwrap();

    conn.execute_batch(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT);
         INSERT INTO items (value) VALUES ('val-1'), ('val-2'), ('val-3');",
    )
    .unwrap();

    // WAL file should exist and have frames
    assert!(
        wal_path.exists(),
        "WAL sidecar file must exist after writes"
    );
    let wal_size_before = fs::metadata(&wal_path).unwrap().len();
    assert!(
        wal_size_before > 0,
        "WAL file must be > 0 bytes with autocheckpoint disabled"
    );

    // Test PASSIVE checkpoint first
    let passive = checkpoint_wal(&conn, WalCheckpointMode::Passive).unwrap();
    assert!(!passive.busy, "Passive checkpoint must not be busy");

    // Run WAL checkpoint TRUNCATE to flush all frames and truncate the WAL file
    let checkpoint = checkpoint_wal_truncate(&conn).unwrap();
    assert!(!checkpoint.busy, "Checkpoint TRUNCATE must not be busy");

    // Checkpoint TRUNCATE truncates the WAL file to 0 bytes
    let wal_size_after = fs::metadata(&wal_path).unwrap().len();
    assert_eq!(
        wal_size_after, 0,
        "WAL file must be truncated to 0 bytes after PRAGMA wal_checkpoint(TRUNCATE)"
    );

    // Verify all rows are readable from the main database file
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3, "All rows must remain accessible after checkpoint");
}

#[tokio::test]
async fn test_wal_active_daemon_edge_case_backup_zero_data_loss() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("active_daemon.db");
    let backup_path = tmp.path().join("backup").join("active_daemon_backup.db");

    // Daemon opens connection and keeps it open
    let daemon_conn = Connection::open(&db_path).unwrap();
    ensure_wal_mode(&daemon_conn).unwrap();
    daemon_conn
        .execute_batch(
            "PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE daemon_state (
                key TEXT PRIMARY KEY,
                val TEXT NOT NULL,
                timestamp TEXT NOT NULL
             );",
        )
        .unwrap();

    // Daemon writes rows (held in WAL)
    for i in 1..=100 {
        daemon_conn
            .execute(
                "INSERT INTO daemon_state (key, val, timestamp) VALUES (?1, ?2, datetime('now'))",
                rusqlite::params![format!("daemon-key-{}", i), format!("daemon-val-{}", i)],
            )
            .unwrap();
    }

    // Backup procedure executes while daemon connection is active
    let report = backup_database_from_connection(&daemon_conn, &db_path, &backup_path).unwrap();
    assert!(
        !report.checkpoint.busy,
        "Checkpoint must succeed without busy lock"
    );
    assert!(backup_path.exists(), "Backup file must be created on disk");

    // Open backup database independently without any WAL sidecar
    let backup_conn = Connection::open(&backup_path).unwrap();
    verify_database_integrity(&backup_conn).unwrap();

    let backup_rows: i64 = backup_conn
        .query_row("SELECT COUNT(*) FROM daemon_state", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        backup_rows, 100,
        "All 100 rows must be present in the backup without data loss"
    );

    // Prove 0 data loss: compare source and backup checksums
    let daemon_checksum = compute_table_checksum(&daemon_conn, "daemon_state").unwrap();
    let backup_checksum = compute_table_checksum(&backup_conn, "daemon_state").unwrap();
    assert_eq!(
        daemon_checksum, backup_checksum,
        "Checksums must be strictly identical between active daemon DB and backup"
    );
}

#[test]
fn test_wal_stopped_daemon_edge_case_with_pending_wal() {
    let tmp = TempDir::new().unwrap();
    let crash_dir = tmp.path().join("crashed_daemon");
    fs::create_dir_all(&crash_dir).unwrap();
    let source_path = crash_dir.join("stopped_daemon.db");
    let wal_path = crash_dir.join("stopped_daemon.db-wal");
    let target_path = tmp
        .path()
        .join("migrated")
        .join("stopped_daemon_migrated.db");

    // 1. Simulate daemon running, writing uncheckpointed data into WAL
    {
        let live_db = tmp.path().join("live.db");
        let live_wal = tmp.path().join("live.db-wal");
        let conn = Connection::open(&live_db).unwrap();
        ensure_wal_mode(&conn).unwrap();
        conn.execute_batch(
            "PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE pending_jobs (id INTEGER PRIMARY KEY, task TEXT);
             INSERT INTO pending_jobs (task) VALUES ('task-A'), ('task-B'), ('task-C');",
        )
        .unwrap();

        // Snapshot abruptly stopped daemon state while transactions are pending in WAL
        fs::copy(&live_db, &source_path).unwrap();
        fs::copy(&live_wal, &wal_path).unwrap();
    }

    // Pending WAL file exists on disk
    assert!(
        wal_path.exists(),
        "WAL file should exist after daemon shutdown"
    );
    let wal_len = fs::metadata(&wal_path).unwrap().len();
    assert!(
        wal_len > 0,
        "WAL file should contain uncheckpointed transactions"
    );

    // 2. Migration tooling handles stopped daemon state
    let report = migrate_database_file(&source_path, &target_path).unwrap();
    assert!(
        !report.checkpoint.busy,
        "Migration checkpoint must complete cleanly"
    );

    // 3. Verify destination database has all rows and 0 data loss
    let dest_conn = Connection::open(&target_path).unwrap();
    verify_database_integrity(&dest_conn).unwrap();

    let count: i64 = dest_conn
        .query_row("SELECT COUNT(*) FROM pending_jobs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3, "All 3 pending jobs must survive migration");

    assert_eq!(
        report.pre_migration_checksums, report.post_migration_checksums,
        "Pre-migration and post-migration checksums must match identically"
    );
}

#[test]
fn test_wal_checksum_table_identical_multi_table() {
    let tmp = TempDir::new().unwrap();
    let source_path = tmp.path().join("fleet.db");
    let dest_path = tmp.path().join("backup_fleet.db");

    let conn = Connection::open(&source_path).unwrap();
    ensure_wal_mode(&conn).unwrap();

    conn.execute_batch(
        "CREATE TABLE agents (
            id TEXT PRIMARY KEY,
            alias TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
         );
         CREATE TABLE memories (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL,
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'core',
            embedding BLOB,
            agent_id TEXT REFERENCES agents(id),
            UNIQUE (agent_id, key)
         );
         INSERT INTO agents VALUES ('uuid-1', 'lead', '2026-10-28T00:00:00Z');
         INSERT INTO agents VALUES ('uuid-2', 'backend', '2026-10-28T00:00:01Z');
         INSERT INTO memories VALUES ('mem-1', 'sys.prompt', 'You are senior BE', 'core', X'01020304', 'uuid-2');
         INSERT INTO memories VALUES ('mem-2', 'sys.config', '{\"wal\": true}', 'config', NULL, 'uuid-1');",
    )
    .unwrap();

    let report = backup_database_with_wal_checkpoint(&source_path, &dest_path).unwrap();
    assert_eq!(
        report.source_checksums.len(),
        2,
        "Both tables must be checksummed"
    );
    assert_eq!(
        report.source_checksums, report.destination_checksums,
        "Source and destination checksums must be 100% identical"
    );

    let dest_conn = Connection::open(&dest_path).unwrap();
    verify_database_integrity(&dest_conn).unwrap();

    let agents_checksum = compute_table_checksum(&dest_conn, "agents").unwrap();
    assert_eq!(agents_checksum.row_count, 2);

    let memories_checksum = compute_table_checksum(&dest_conn, "memories").unwrap();
    assert_eq!(memories_checksum.row_count, 2);
}

#[test]
fn test_wal_cron_jobs_database_checkpoint_and_migration() {
    let tmp = TempDir::new().unwrap();
    let jobs_db = tmp.path().join("jobs.db");
    let backup_jobs_db = tmp.path().join("jobs.db.backup");

    let conn = Connection::open(&jobs_db).unwrap();
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE cron_jobs (
            id TEXT PRIMARY KEY,
            expression TEXT NOT NULL,
            command TEXT NOT NULL,
            schedule TEXT,
            enabled INTEGER NOT NULL DEFAULT 1
         );
         CREATE TABLE cron_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL REFERENCES cron_jobs(id) ON DELETE CASCADE,
            started_at TEXT NOT NULL,
            finished_at TEXT NOT NULL,
            status TEXT NOT NULL
         );
         INSERT INTO cron_jobs VALUES ('job-1', '0 * * * *', 'galleon sync', 'hourly', 1);
         INSERT INTO cron_runs (job_id, started_at, finished_at, status)
         VALUES ('job-1', '2026-10-28T01:00:00Z', '2026-10-28T01:00:05Z', 'success');",
    )
    .unwrap();

    let report = backup_database_with_wal_checkpoint(&jobs_db, &backup_jobs_db).unwrap();
    assert_eq!(report.source_checksums, report.destination_checksums);

    let dest_conn = Connection::open(&backup_jobs_db).unwrap();
    let jobs_count: i64 = dest_conn
        .query_row("SELECT COUNT(*) FROM cron_jobs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(jobs_count, 1);

    let runs_count: i64 = dest_conn
        .query_row("SELECT COUNT(*) FROM cron_runs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(runs_count, 1);
}

#[test]
fn test_wal_edge_case_empty_database_and_idempotent_recheckpoint() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("empty.db");
    let backup_path = tmp.path().join("empty_backup.db");

    let conn = Connection::open(&db_path).unwrap();
    ensure_wal_mode(&conn).unwrap();

    // Idempotent checkpoint on empty DB
    let res1 = checkpoint_wal_truncate(&conn).unwrap();
    assert!(!res1.busy);
    let res2 = checkpoint_wal_truncate(&conn).unwrap();
    assert!(!res2.busy);

    let report = backup_database_with_wal_checkpoint(&db_path, &backup_path).unwrap();
    assert_eq!(report.source_checksums, report.destination_checksums);
    assert_eq!(report.source_checksums.len(), 0, "No tables on empty DB");
}

#[test]
fn test_wal_edge_case_corrupt_database_detection() {
    let tmp = TempDir::new().unwrap();
    let corrupt_path = tmp.path().join("corrupt.db");
    let dest_path = tmp.path().join("should_fail.db");

    // Write invalid data
    fs::write(&corrupt_path, b"not a valid sqlite file contents at all").unwrap();

    let result = backup_database_with_wal_checkpoint(&corrupt_path, &dest_path);
    assert!(
        result.is_err(),
        "Corrupted database must fail integrity check or opening"
    );
    assert!(
        !dest_path.exists(),
        "Destination must not be created if source fails integrity check"
    );
}
