//! Tooling for SQLite WAL mode checkpointing, database backup, state migration,
//! and verification of zero data loss (SPEC-008 / T3-21 / BL-11).

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Checkpoint mode corresponding to SQLite `PRAGMA wal_checkpoint(...)` modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalCheckpointMode {
    /// Checkpoint as many frames as possible without waiting for readers/writers.
    Passive,
    /// Wait for readers to finish, checkpoint all frames, but do not truncate.
    Full,
    /// Like Full, but also wait for readers so that new writers restart from start of WAL.
    Restart,
    /// Checkpoint all frames and truncate the WAL file to zero bytes.
    /// Recommended before backup, file transfer, or migration.
    Truncate,
}

impl WalCheckpointMode {
    pub fn as_pragma_str(&self) -> &'static str {
        match self {
            Self::Passive => "PASSIVE",
            Self::Full => "FULL",
            Self::Restart => "RESTART",
            Self::Truncate => "TRUNCATE",
        }
    }
}

/// Result of executing `PRAGMA wal_checkpoint(...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalCheckpointResult {
    /// True if the checkpoint was blocked from completing (e.g., active readers).
    pub busy: bool,
    /// Total number of WAL frames in the log file, or -1 if not in WAL mode.
    pub log_frames: i64,
    /// Number of WAL frames successfully checkpointed back into the database file.
    pub checkpointed_frames: i64,
}

/// Checksum and row count summary for a table, ensuring zero data loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableChecksum {
    pub table_name: String,
    pub row_count: u64,
    pub sha256_hex: String,
}

/// Detailed report from a database backup operation with WAL checkpoint.
#[derive(Debug, Clone)]
pub struct WalBackupReport {
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub checkpoint: WalCheckpointResult,
    pub source_checksums: Vec<TableChecksum>,
    pub destination_checksums: Vec<TableChecksum>,
    pub bytes_copied: u64,
}

/// Detailed report from a database state migration operation with WAL checkpoint.
#[derive(Debug, Clone)]
pub struct WalMigrationReport {
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub checkpoint: WalCheckpointResult,
    pub pre_migration_checksums: Vec<TableChecksum>,
    pub post_migration_checksums: Vec<TableChecksum>,
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Executes a WAL checkpoint on an open SQLite connection.
pub fn checkpoint_wal(conn: &Connection, mode: WalCheckpointMode) -> Result<WalCheckpointResult> {
    let pragma_sql = format!("PRAGMA wal_checkpoint({});", mode.as_pragma_str());
    let (busy, log, checkpointed): (i32, i64, i64) = conn
        .query_row(&pragma_sql, [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .with_context(|| format!("Failed to execute {}", pragma_sql))?;

    Ok(WalCheckpointResult {
        busy: busy != 0,
        log_frames: log,
        checkpointed_frames: checkpointed,
    })
}

/// Executes `PRAGMA wal_checkpoint(TRUNCATE)` on an open SQLite connection.
pub fn checkpoint_wal_truncate(conn: &Connection) -> Result<WalCheckpointResult> {
    checkpoint_wal(conn, WalCheckpointMode::Truncate)
}

/// Returns true if the database connection is currently operating in WAL mode.
pub fn is_wal_mode(conn: &Connection) -> Result<bool> {
    let mode: String = conn
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .context("Failed to query journal_mode")?;
    Ok(mode.eq_ignore_ascii_case("wal"))
}

/// Ensures the database is set to WAL journal mode.
pub fn ensure_wal_mode(conn: &Connection) -> Result<()> {
    if !is_wal_mode(conn)? {
        let new_mode: String = conn
            .query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))
            .context("Failed to set PRAGMA journal_mode = WAL")?;
        if !new_mode.eq_ignore_ascii_case("wal") {
            bail!(
                "Requested WAL journal mode, but SQLite returned {}",
                new_mode
            );
        }
    }
    Ok(())
}

/// Verifies database integrity via `PRAGMA integrity_check`.
pub fn verify_database_integrity(conn: &Connection) -> Result<()> {
    let status: String = conn
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .context("Failed to execute PRAGMA integrity_check")?;
    if status != "ok" {
        bail!("Database integrity check failed: {}", status);
    }
    Ok(())
}

/// Computes a canonical, deterministic SHA-256 checksum and row count for a table.
pub fn compute_table_checksum(conn: &Connection, table_name: &str) -> Result<TableChecksum> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?1",
            [table_name],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !exists {
        bail!("Table '{}' not found in database", table_name);
    }

    let escaped_name = table_name.replace('"', "\"\"");
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", escaped_name))?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if columns.is_empty() {
        return Ok(TableChecksum {
            table_name: table_name.to_string(),
            row_count: 0,
            sha256_hex: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_string(),
        });
    }

    let quoted_cols = columns
        .iter()
        .map(|c| format!("quote(\"{}\")", c.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT {} FROM \"{}\" ORDER BY rowid",
        quoted_cols, escaped_name
    );

    let mut stmt = conn.prepare(&query)?;
    let col_count = columns.len();
    let mut rows = stmt.query([])?;

    let mut hasher = Sha256::new();
    let mut count: u64 = 0;

    while let Some(row) = rows.next()? {
        count += 1;
        for i in 0..col_count {
            let val: String = row.get(i)?;
            hasher.update(val.as_bytes());
            hasher.update(b"|");
        }
        hasher.update(b"\n");
    }

    let hash_result = hasher.finalize();
    let sha256_hex = hex_encode(&hash_result);

    Ok(TableChecksum {
        table_name: table_name.to_string(),
        row_count: count,
        sha256_hex,
    })
}

/// Computes SHA-256 checksums for all user tables in the database.
pub fn compute_all_tables_checksums(conn: &Connection) -> Result<Vec<TableChecksum>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master 
         WHERE type='table' 
           AND name NOT LIKE 'sqlite_%' 
         ORDER BY name",
    )?;
    let tables: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut checksums = Vec::with_capacity(tables.len());
    for table in tables {
        checksums.push(compute_table_checksum(conn, &table)?);
    }
    Ok(checksums)
}

/// Backups a SQLite database file to destination_path, enforcing WAL checkpoint TRUNCATE
/// before file copy and verifying checksums match with zero data loss.
pub fn backup_database_with_wal_checkpoint(
    source_path: &Path,
    destination_path: &Path,
) -> Result<WalBackupReport> {
    if !source_path.exists() {
        bail!("Source database does not exist: {}", source_path.display());
    }

    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create destination directory: {}",
                parent.display()
            )
        })?;
    }

    let source_conn = Connection::open_with_flags(
        source_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("Failed to open source database: {}", source_path.display()))?;

    source_conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

    let checkpoint = checkpoint_wal_truncate(&source_conn)?;
    if checkpoint.busy {
        bail!(
            "WAL checkpoint was blocked (busy=true). Active readers or writers on {}",
            source_path.display()
        );
    }

    verify_database_integrity(&source_conn)?;
    let source_checksums = compute_all_tables_checksums(&source_conn)?;

    let bytes_copied = fs::copy(source_path, destination_path).with_context(|| {
        format!(
            "Failed to copy database from {} to {}",
            source_path.display(),
            destination_path.display()
        )
    })?;

    let dest_conn = Connection::open_with_flags(
        destination_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| {
        format!(
            "Failed to open destination database: {}",
            destination_path.display()
        )
    })?;

    verify_database_integrity(&dest_conn)?;
    let destination_checksums = compute_all_tables_checksums(&dest_conn)?;

    if source_checksums != destination_checksums {
        bail!(
            "Zero-data-loss validation failed! Source checksums ({:?}) do not match destination checksums ({:?})",
            source_checksums,
            destination_checksums
        );
    }

    Ok(WalBackupReport {
        source_path: source_path.to_path_buf(),
        destination_path: destination_path.to_path_buf(),
        checkpoint,
        source_checksums,
        destination_checksums,
        bytes_copied,
    })
}

/// Backups a SQLite database using an existing active connection.
pub fn backup_database_from_connection(
    conn: &Connection,
    source_path: &Path,
    destination_path: &Path,
) -> Result<WalBackupReport> {
    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create destination directory: {}",
                parent.display()
            )
        })?;
    }

    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    let checkpoint = checkpoint_wal_truncate(conn)?;
    if checkpoint.busy {
        bail!(
            "WAL checkpoint was blocked (busy=true) on {}",
            source_path.display()
        );
    }

    verify_database_integrity(conn)?;
    let source_checksums = compute_all_tables_checksums(conn)?;

    let bytes_copied = fs::copy(source_path, destination_path).with_context(|| {
        format!(
            "Failed to copy database from {} to {}",
            source_path.display(),
            destination_path.display()
        )
    })?;

    let dest_conn = Connection::open_with_flags(
        destination_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| {
        format!(
            "Failed to open destination database: {}",
            destination_path.display()
        )
    })?;

    verify_database_integrity(&dest_conn)?;
    let destination_checksums = compute_all_tables_checksums(&dest_conn)?;

    if source_checksums != destination_checksums {
        bail!(
            "Zero-data-loss validation failed! Source checksums ({:?}) do not match destination checksums ({:?})",
            source_checksums,
            destination_checksums
        );
    }

    Ok(WalBackupReport {
        source_path: source_path.to_path_buf(),
        destination_path: destination_path.to_path_buf(),
        checkpoint,
        source_checksums,
        destination_checksums,
        bytes_copied,
    })
}

/// Migrates a SQLite database state from source_path to target_path, ensuring WAL checkpoint TRUNCATE
/// is executed and proving zero data loss with identical table checksums.
pub fn migrate_database_file(source_path: &Path, target_path: &Path) -> Result<WalMigrationReport> {
    if !source_path.exists() {
        bail!("Source database does not exist: {}", source_path.display());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create target directory: {}", parent.display()))?;
    }

    let conn = Connection::open_with_flags(
        source_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("Failed to open source database: {}", source_path.display()))?;

    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    let checkpoint = checkpoint_wal_truncate(&conn)?;
    if checkpoint.busy {
        bail!(
            "WAL checkpoint was blocked (busy=true) during migration of {}",
            source_path.display()
        );
    }

    verify_database_integrity(&conn)?;
    let pre_checksums = compute_all_tables_checksums(&conn)?;
    drop(conn);

    fs::copy(source_path, target_path).with_context(|| {
        format!(
            "Failed to transfer database from {} to {}",
            source_path.display(),
            target_path.display()
        )
    })?;

    let dest_conn = Connection::open_with_flags(
        target_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| {
        format!(
            "Failed to open migrated database: {}",
            target_path.display()
        )
    })?;

    verify_database_integrity(&dest_conn)?;
    let post_checksums = compute_all_tables_checksums(&dest_conn)?;

    if pre_checksums != post_checksums {
        bail!(
            "Migration data loss detected! Pre-migration checksums ({:?}) do not match post-migration checksums ({:?})",
            pre_checksums,
            post_checksums
        );
    }

    Ok(WalMigrationReport {
        source_path: source_path.to_path_buf(),
        target_path: target_path.to_path_buf(),
        checkpoint,
        pre_migration_checksums: pre_checksums,
        post_migration_checksums: post_checksums,
    })
}
