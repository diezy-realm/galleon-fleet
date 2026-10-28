# Galleon State Migration & Backup Guide (BL-11 / RB-23)

Panduan pemindahan state Galleon Fleet (konfigurasi, basis data SQLite memori, jobs cron, dan skills) antar host atau migrasi dari direktori warisan `~/.clawcrew` ke `~/.config/galleon`.

---

## 1. Peringatan Keamanan Kritis: SQLite WAL Mode

Seluruh database memori di `crates/galleon-memory` (seperti `knowledge.db`, `audit.db`, `response_cache.db`) beroperasi dengan **`PRAGMA journal_mode = WAL;`**.

> [!CAUTION]
> **JANGAN PERNAH** menyalin hanya file `.db` saat transaksi aktif masih ada di file `-wal`! 
> Menyalin `.db` tanpa file `-wal` dan `-shm` akan menyebabkan data mutakhir hilang diam-diam (*silent data loss*) atau basis data mengalami *corruption*.

### Prosedur Aman Sebelum Backup / Migrasi:
1. **Hentikan daemon** (`galleon` / `clawcrew`) sebelum memindahkan file.
2. Jika melakukan *hot backup* saat daemon berjalan, lakukan *checkpoint* terlebih dahulu:
   ```sql
   PRAGMA wal_checkpoint(TRUNCATE);
   ```
3. Salin ketiga file secara atomik: `<name>.db`, `<name>.db-wal`, dan `<name>.db-shm`.

---

## 2. Penggunaan Skrip Migrasi Otomatis

Skrip migrasi disediakan di `scripts/migrations/migrate_state.py`:

```bash
# Uji coba kering (dry-run) tanpa menulis disk
python scripts/migrations/migrate_state.py --dry-run

# Migrasi otomatis dari ~/.clawcrew ke ~/.config/galleon
python scripts/migrations/migrate_state.py

# Migrasi ke direktori target kustom
python scripts/migrations/migrate_state.py --source /path/to/old --target /path/to/new --force
```

Skrip ini secara otomatis:
- Menjalankan `PRAGMA wal_checkpoint(TRUNCATE)` pada setiap database SQLite yang ditemukan.
- Menyalin `config.toml`, seluruh database `.db` beserta `-wal` & `-shm`.
- Menyalin folder `skills/`, `shared/`, dan `vault/`.
- Menulis file penanda `.migrated` di folder tujuan.
