# ADR-005 — Desain Store Persistensi Terpadu Go Engine (SQLite, PostgreSQL, Vektor, Migrasi, Enkripsi)

- **Status:** Accepted (Ratifikasi ODR-05)
- **Tanggal:** 2026-10-04
- **Pengusul:** Galleon Core Architecture / BE-Go Lane
- **Konteks:** Tier Go Engine (`engine/`), penggantian DiskStore flat file JSON/JSONL
- **Terkait:** ODR-05, ODR-07, GW-2, GW-6, GW-7, G-10..G-14, PR-40

---

## 1. Konteks & Problem Statement

Pada arsitektur awal, Go Engine menggunakan `engine/src/disk_store.go` yang menyimpan status agregat (Ships, Squads, Crews, Policies, Terminal Audit) ke dalam direktori file flat (`.json` dan `.jsonl`) di `$GALLEON_DATA_DIR` atau `engine/data`.

Keterbatasan kritis dari model flat-file ini meliputi:
1. **Kurangnya Garansi Transaksional (ACID):** Pembaruan concurrent rentan terhadap race condition, partial write, dan korupsi data bila proses mati tiba-tiba.
2. **Keterbatasan Query & Relasi:** Agregasi multi-tabel, pencarian status, dan penyaringan run/turn memerlukan full memory scan.
3. **Memori Vektor & RAG:** Flat-file tidak mendukung pengindeksan vektor berdimensi tinggi untuk retrieval augmented generation (RAG) per scope organisasi (Fleet/Ship/Squad/Crew/Run).
4. **Keamanan & Kepatuhan Privasi:** Tidak ada mekanisme bawaan untuk enkripsi at-rest, retensi berbasis waktu, atau penghapusan data terisolasi.

ODR-05 menetapkan arah produk: **SQLite (lokal-default) / PostgreSQL (server-opsional)** sebagai fondasi persistensi memori permanen.

---

## 2. Pilihan Desain & Evaluasi

### A. Pola Abstraksi: Interface Store per Agregat
- **Pilihan:** Pemisahan interface per aggregate root (`FleetStore`, `ShipStore`, `SquadStore`, `CrewStore`, `RunStore`, `AuditStore`, `VectorStore`).
- **Rasional:** Memungkinkan dependensi modular via Google Wire, mempermudah mocking pada unit test tanpa memerlukan instance basis data fisik, dan mengisolasi dialek DB di balik layer repository.

### B. Pemilihan Dialek Basis Data
1. **SQLite (Default / Local-first):**
   - Mode WAL (`PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON;`).
   - Pure-Go driver (`modernc.org/sqlite`) direkomendasikan agar kompilasi silang (cross-compilation) Go di Windows, Linux, dan macOS tetap zero-CGO dan mudah di-embed ke dalam binary desktop Tauri.
   - Vektor: Penyimpanan embedding sebagai `BLOB` (float32 array) dengan k-NN cosine distance in-memory atau ekstensi `sqlite-vec`.
2. **PostgreSQL (Server / Multi-user, Opsional):**
   - Driver `pgx/v5` dengan pool connection terkelola.
   - Ekstensi `pgvector` untuk pencarian vektor native menggunakan indeks HNSW / IVFFlat.

### C. Strategi Migrasi Idempoten dari DiskStore Legacy
- Saat engine melakukan bootstrap (`InitStore`):
  1. Jalankan skema DDL berurutan (`schema_migrations` table).
  2. Periksa keberadaan file flat legacy di `$GALLEON_DATA_DIR/collections/` dan `audit.jsonl`.
  3. Lakukan pembacaan rekursif dan insert transaksi ke SQLite/PostgreSQL dengan `ON CONFLICT (id) DO NOTHING`.
  4. Tandai file legacy dengan memindahkannya ke `$GALLEON_DATA_DIR/archive_legacy_<timestamp>/`.
  5. Catat log audit migrasi. Bila terjadi error, rollback transaksi basis data dan pertahankan file legacy tanpa perubahan (fail-closed).

### D. Enkripsi At-Rest & Keamanan Kredensial
- Kredensial rahasia (API keys, authorization tokens) **tidak disimpan** di basis data Go engine dalam bentuk teks polos.
- Rahasia utama dikelola oleh Vault Rust melalui gRPC SystemGateway `GetDecryptedSecret` (T-8 / ODR-05).
- Data sensitif yang wajib tersimpan di Go dienkripsi menggunakan AEAD (ChaCha20-Poly1305 / AES-256-GCM) dengan key derivation dari master machine key.

### E. Memori Vektor & Hirarki RAG per Scope
- Memori terisolasi dalam 5 tingkat hierarki:
  1. `ScopeFleet`: Pengetahuan universal seluruh armada.
  2. `ScopeShip`: Konteks spesifik repository/proyek.
  3. `ScopeSquad`: Konteks kelompok spesialisasi tugas.
  4. `ScopeCrew`: Preferensi dan memori spesifik persona agent.
  5. `ScopeRun`: Konteks percakapan/turn yang sedang berlangsung.
- Setiap entri vektor menyimpan `(id, scope_type, scope_id, text, embedding, metadata_json, created_at)`.
- Query RAG wajib menyertakan filter scope ketat untuk mencegah kebocoran konteks antar-tenant atau antar-kapal.

---

## 3. Keputusan Arsitektur

1. **Paket & Struktur Folder:**
   - `engine/pkg/store/`: Interface utama dan tipe agregat domain persistensi.
   - `engine/pkg/store/sqlite/`: Implementasi SQLite driver, DDL, dan migrasi.
   - `engine/pkg/store/postgres/`: Implementasi PostgreSQL driver dan query pgvector.
   - `engine/pkg/store/migrator/`: Logika migrasi satu arah dari DiskStore legacy.
2. **Pola Transaksi:**
   - Mengharuskan `context.Context` pada seluruh signature method `Store`.
   - Menggunakan pattern Unit of Work (`WithTx(ctx, func(tx Store) error)`) untuk operasi yang melibatkan pembaruan beberapa agregat sekaligus.
3. **Penyimpanan Audit:**
   - `TerminalAuditStore` dan `SecurityAuditStore` disimpan append-only, dengan indeks pada `timestamp`, `risk_tier`, dan `caller_identity`.
4. **Backup Online:**
   - SQLite memanfaatkan API `VACUUM INTO ?` untuk pencadangan berkala tanpa downtime.

---

## 4. Konsekuensi

- **Positif:**
  - Jaminan konsistensi ACID penuh pada status kapal, kru, dan audit ledger.
  - Skalabilitas siap pakai dari single-user desktop (SQLite) hingga server armada (PostgreSQL).
  - Fondasi kokoh untuk fitur RAG multi-scope, memori jangka panjang agent, dan pembersihan kepatuhan privasi (GDPR).
- **Negatif / Kompleksitas:**
  - Penambahan dependensi driver database pada Go Engine.
  - Perlunya pengelolaan migrasi skema basis data versi demi versi.
- **Mitigasi:**
  - Menggunakan pure-Go driver SQLite untuk menjaga zero CGO requirement dan portabilitas CI/CD.
  - Menyertakan pengujian container terisolasi untuk suite PostgreSQL di CI.

---

## 5. Kepatuhan Batasan (Boundary Compliance)

- **Go Engine Tier (GW-2):** Memegang Single Source of Truth (SSOT) untuk state persistensi, memori agent, dan audit ledger.
- **Rust Gateway (GW-1):** Tetap memegang SSOT untuk Secrets Vault, eksekusi OS, dan sandboxing.
- **Fail-Closed:** Bila engine gagal menginisialisasi atau memverifikasi integritas database store, engine menolak booting dan menghentikan serving port :9090.
