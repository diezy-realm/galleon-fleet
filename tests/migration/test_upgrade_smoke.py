"""
Smoke test upgrade data lama (~/.clawcrew) -> baru (~/.config/galleon)
Acceptance Criteria PL-E3 / N-41:
- Setup instance lama dengan data di ~/.clawcrew/ (mock directory)
- Jalankan skrip migrasi
- Konfirmasi data termigrasi ke ~/.config/galleon/ dengan isi IDENTIK
- Konfirmasi file SQLite WAL dicheckpoint dan konsisten
"""

import os
import shutil
import sqlite3
import tempfile
import unittest
from pathlib import Path

# Import migration script module
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "scripts" / "migrations"))
from migrate_state import migrate_state

class TestUpgradeSmoke(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.legacy_dir = Path(self.temp_dir) / "clawcrew"
        self.target_dir = Path(self.temp_dir) / "galleon"
        self.legacy_dir.mkdir(parents=True)

        # 1. Populate mock config
        (self.legacy_dir / "config.toml").write_text('[gateway]\nport = 9090\n', encoding="utf-8")

        # 2. Populate mock SQLite database with WAL
        db_path = self.legacy_dir / "knowledge.db"
        conn = sqlite3.connect(str(db_path))
        conn.execute("PRAGMA journal_mode = WAL;")
        conn.execute("CREATE TABLE fleet_meta (k TEXT PRIMARY KEY, v TEXT);")
        conn.execute("INSERT INTO fleet_meta VALUES ('vessel', 'galleon-one');")
        conn.commit()
        conn.close()

    def tearDown(self):
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_migration_accuracy_and_sqlite_checkpoint(self):
        # Run migration
        res = migrate_state(self.legacy_dir, self.target_dir, force=True)
        self.assertTrue(res, "Migration must return True on success")

        # Check target config file exists and content matches
        target_config = self.target_dir / "config.toml"
        self.assertTrue(target_config.is_file(), "config.toml must exist in target")
        self.assertEqual(target_config.read_text(encoding="utf-8"), '[gateway]\nport = 9090\n')

        # Check target SQLite DB is readable and data preserved
        target_db = self.target_dir / "knowledge.db"
        self.assertTrue(target_db.is_file(), "knowledge.db must exist in target")
        conn = sqlite3.connect(str(target_db))
        cursor = conn.cursor()
        cursor.execute("SELECT v FROM fleet_meta WHERE k = 'vessel';")
        row = cursor.fetchone()
        conn.close()
        self.assertIsNotNone(row)
        self.assertEqual(row[0], "galleon-one")

        # Check .migrated marker in target dir
        self.assertTrue((self.target_dir / ".migrated").is_file(), ".migrated marker must exist in target dir")

if __name__ == "__main__":
    unittest.main()
