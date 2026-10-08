#!/usr/bin/env python3
"""
Galleon Fleet — State Migration & Backup Tooling (BL-11 / RB-23)
================================================================
Handles migration and backup of Galleon state:
- Configuration (config.toml, credentials)
- Memory databases (knowledge.db, audit.db, response_cache.db, etc.)
- Cron / job scheduler databases (jobs.db)
- Custom skills and extensions

CRITICAL SAFETY NOTICE FOR SQLITE WAL MODE:
Every database in `crates/clawcrew-memory` runs with `PRAGMA journal_mode = WAL;`.
Copying only the `.db` file while transactions exist in `-wal` will cause
silent data loss. This tool executes `PRAGMA wal_checkpoint(TRUNCATE)` before
copying, or copies `.db`, `.db-wal`, and `.db-shm` atomically together.
"""

import argparse
import os
import shutil
import sqlite3
import sys
from pathlib import Path


def get_default_source_dir() -> Path:
    """Returns ~/.clawcrew or OS equivalent for legacy state."""
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if appdata:
            legacy = Path(appdata) / "clawcrew"
            if legacy.exists():
                return legacy
    home = Path.home()
    legacy = home / ".clawcrew"
    return legacy


def get_default_target_dir() -> Path:
    """Returns ~/.config/galleon or %APPDATA%/galleon for new state."""
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if appdata:
            return Path(appdata) / "galleon"
    xdg_config = os.environ.get("XDG_CONFIG_HOME")
    if xdg_config:
        return Path(xdg_config) / "galleon"
    return Path.home() / ".config" / "galleon"


def checkpoint_sqlite_db(db_path: Path) -> bool:
    """Runs PRAGMA wal_checkpoint(TRUNCATE) on a SQLite database."""
    if not db_path.is_file():
        return False
    try:
        conn = sqlite3.connect(str(db_path))
        cursor = conn.cursor()
        cursor.execute("PRAGMA wal_checkpoint(TRUNCATE);")
        res = cursor.fetchall()
        conn.close()
        print(f"[CHECKPOINT] Checkpointed {db_path.name} successfully (status: {res})")
        return True
    except Exception as err:
        print(f"[WARN] Failed to checkpoint {db_path.name}: {err}", file=sys.stderr)
        return False


def copy_db_with_wal(src_db: Path, dest_dir: Path, checkpoint: bool = True):
    """Copies .db, and if present .db-wal and .db-shm files."""
    dest_dir.mkdir(parents=True, exist_ok=True)
    if checkpoint:
        checkpoint_sqlite_db(src_db)

    # Copy main db
    dest_db = dest_dir / src_db.name
    shutil.copy2(src_db, dest_db)
    print(f"[COPY] {src_db.name} -> {dest_db}")

    # Copy wal & shm if they exist
    wal_file = src_db.with_name(src_db.name + "-wal")
    if wal_file.exists():
        shutil.copy2(wal_file, dest_dir / wal_file.name)
        print(f"[COPY] {wal_file.name} -> {dest_dir / wal_file.name}")

    shm_file = src_db.with_name(src_db.name + "-shm")
    if shm_file.exists():
        shutil.copy2(shm_file, dest_dir / shm_file.name)
        print(f"[COPY] {shm_file.name} -> {dest_dir / shm_file.name}")


def migrate_state(source: Path, target: Path, dry_run: bool = False, force: bool = False):
    print(f"=== Galleon State Migration ===")
    print(f"Source: {source}")
    print(f"Target: {target}")

    if not source.exists():
        print(f"[ERROR] Source directory does not exist: {source}", file=sys.stderr)
        return False

    if target.exists() and not force:
        target_files = list(target.iterdir())
        if target_files:
            print(f"[WARN] Target directory is not empty. Use --force to overwrite.", file=sys.stderr)
            return False

    if dry_run:
        print("[DRY-RUN] Scanning source state...")
        for item in source.rglob("*"):
            if item.is_file():
                rel = item.relative_to(source)
                print(f"  Will copy: {rel}")
        return True

    target.mkdir(parents=True, exist_ok=True)

    # 1. Migrate config file
    config_src = source / "config.toml"
    if config_src.is_file():
        dest_cfg = target / "config.toml"
        shutil.copy2(config_src, dest_cfg)
        print(f"[CONFIG] Migrated config.toml to {dest_cfg}")

    # 2. Migrate data / databases
    data_src = source / "data"
    search_dirs = [data_src] if data_src.exists() else [source]
    for sdir in search_dirs:
        for db_file in sdir.glob("*.db"):
            rel = db_file.parent.relative_to(source)
            dest_subdir = target / rel
            copy_db_with_wal(db_file, dest_subdir, checkpoint=True)

    # 3. Migrate skills and shared assets
    for folder_name in ["skills", "shared", "vault"]:
        f_src = source / folder_name
        if f_src.exists() and f_src.is_dir():
            f_dest = target / folder_name
            shutil.copytree(f_src, f_dest, dirs_exist_ok=True)
            print(f"[DIR] Copied {folder_name} -> {f_dest}")

    # Create migration marker
    marker = target / ".migrated"
    marker.write_text(f"Migrated from {source} by Galleon State Migration Tool\n")
    print(f"[SUCCESS] Migration completed. Marker written at {marker}")
    return True


def main():
    parser = argparse.ArgumentParser(description="Galleon State Migration & Backup Tool")
    parser.add_argument("--source", type=Path, default=None, help="Source state dir (default: legacy ~/.clawcrew)")
    parser.add_argument("--target", type=Path, default=None, help="Target state dir (default: ~/.config/galleon)")
    parser.add_argument("--dry-run", action="store_true", help="Scan and list files without copying")
    parser.add_argument("--force", action="store_true", help="Overwrite non-empty target directory")
    args = parser.parse_args()

    source = args.source or get_default_source_dir()
    target = args.target or get_default_target_dir()

    success = migrate_state(source, target, dry_run=args.dry_run, force=args.force)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
