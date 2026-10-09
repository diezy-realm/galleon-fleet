---
sprint: 6
title: CLI/Client Protocol & Provider Reliability/Memory Governance
status: in-progress (9/11 checklist [x]; 2 item masih [ ])
---

# Sprint 6 — CLI/Client Protocol & Providers

## A. CLI & Klien (dep Client Protocol Sprint 4)

- [x] **BR-E1 · R-36 · PR-57 (P1, M)** Audit `galleoncode` & `galleonrelay`: petakan pemanggilan ke daemon Rust → endpoint Go.

#### Detail Eksekusi — BR-E1

**Koreksi nama direktori (verifikasi 2026-10-06):** `galleoncode`/`galleonrelay` **belum di-rename** — nama direktori aktual masih `apps/zerocode/` dan `apps/zerorelay/` (106 file ditemukan di `apps/`, termasuk `apps/zerocode/src/main.rs`, `apps/zerocode/src/client.rs`, `apps/zerocode/src/relay_proto.rs`, dll). Rename ke `galleoncode`/`galleonrelay` adalah scope Sprint 8 (RB-20) — tiket ini (BR-E1) bekerja di NAMA LAMA (`zerocode`/`zerorelay`), bukan nama baru. Rujuk file dengan path aktual `apps/zerocode/`, JANGAN tulis `apps/galleoncode/` yang belum ada.

**File & simbol spesifik:**
- Baca: `apps/zerocode/src/client.rs` (klien koneksi ke daemon/gateway — kandidat utama titik yang memanggil daemon Rust).
- Baca: `apps/zerocode/src/relay_proto.rs` (protokol relay — kemungkinan jalur yang akan diretarget ke Client Protocol Go di BR-E3).
- Baca: `apps/zerocode/src/main.rs` (2 referensi `spawn_owned_ephemeral_daemon` ditemukan di sini — titik PERSIS yang perlu dipetakan).
- Baca: `apps/zerocode/src/app.rs` (1 referensi `spawn_owned_ephemeral_daemon`).

**Langkah pengerjaan:**
1. Baca ke-4 file di atas, buat tabel pemetaan: setiap pemanggilan ke daemon Rust (nama fungsi, apa yang dilakukan) → endpoint Go `:9090` yang setara (ada/belum ada).
2. Hasil tabel ini adalah input untuk BR-E2/BR-E3 (eksekusi aktual) — tiket ini murni AUDIT/dokumentasi, bukan kode.
3. Tulis hasil ke `standard-docs/decisions/` (lokasi baru, nama file sesuai konvensi existing).

**Acceptance criteria testable:** Dokumen audit lengkap, setiap pemanggilan daemon Rust di `zerocode`/`zerorelay` punya baris di tabel pemetaan dengan status (sudah ada endpoint Go setara / belum).

**Dependency:** Prasyarat BR-E2/BR-E3/BR-E4 (semua butuh hasil audit ini sebagai dasar).

**JANGAN DIUBAH:** Tidak ada kode yang diubah di tiket ini — murni audit.

- [x] **BR-E2 · R-34 · PR-57 (P1, M)** CLI `galleon` `chat/run/approve/status` → Go :9090; `service/doctor/onboard/vault` tetap lokal. Dep: BG-E2 (Sprint 4).

#### Detail Eksekusi — BR-E2

**Catatan nama binary:** CLI `galleon` juga belum di-rename (binary CLI utama kemungkinan nama lama — verifikasi `Cargo.toml` `[[bin]]` section untuk nama persis saat eksekusi, JANGAN asumsikan sudah `galleon`).

**Dependency:** BG-E2 (Sprint 4 §B — WS JSON-RPC 2.0 Client Protocol) HARUS ada dulu. BR-E1 (audit) sebagai input.

**ASUMSI:** Lokasi source CLI `galleon` (binary utama, beda dari `zerocode` TUI) belum diverifikasi sesi ini — kemungkinan crate terpisah di `crates/` atau `apps/`, cari sebelum eksekusi.

- [x] **BR-E3 · R-40 · PR-58 (P1, L)** Retarget `galleoncode` (TUI) ke Client Protocol; matikan `spawn_owned_ephemeral_daemon`. Dep: BG-E2 (Sprint 4).

#### Detail Eksekusi — BR-E3

**File & simbol spesifik (terverifikasi):** `apps/zerocode/src/main.rs` (2 ref `spawn_owned_ephemeral_daemon`) dan `apps/zerocode/src/app.rs` (1 ref) — INI adalah 3 titik pemanggilan persis yang harus "dimatikan" (dihapus/diganti) sesuai judul tiket.

**Spesifikasi API:** Setelah BG-E2 (Client Protocol WS JSON-RPC) tersedia, `zerocode` (TUI) connect ke endpoint Go `:9090` via WS alih-alih spawn daemon Rust ephemeral sendiri.

**Langkah pengerjaan:**
1. Baca `main.rs`/`app.rs` di titik `spawn_owned_ephemeral_daemon` — pahami kondisi kapan daemon di-spawn (startup? on-demand?).
2. Setelah BG-E2 siap, ganti titik ini dengan koneksi WS client ke Client Protocol.
3. Hapus `spawn_owned_ephemeral_daemon` function sepenuhnya setelah semua call-site diganti (verifikasi dengan `find_references` tidak ada caller tersisa sebelum hapus).

**Acceptance criteria testable:** `zerocode` TUI berjalan tanpa pernah memanggil `spawn_owned_ephemeral_daemon` — jalankan TUI, verifikasi via process list TIDAK ada daemon Rust ephemeral baru yang muncul (hanya koneksi ke Go `:9090` existing).

**Dependency:** BG-E2 (Sprint 4, hard). BR-E1 (audit, prasyarat pemahaman).

**JANGAN DIUBAH:** Fungsionalitas TUI yang terlihat user — ini migrasi transport, bukan perubahan UX.

- [x] **BR-E4 · R-42 · PR-58 (P1, M)** Retarget `galleon-acp-bridge` (ODR-24).

#### Detail Eksekusi — BR-E4

**TIDAK DITEMUKAN** — `galleon-acp-bridge`/`galleon_acp_bridge` sebagai crate TIDAK ADA di grep repo-wide (0 match untuk nama ini di luar dokumen sprint ini sendiri). Dokumen Sprint 4 §D (BR-D4) menyebut `galleon-acp-bridge` juga sebagai target hapus `rpc`+REST/WS/A2A lama. **Kemungkinan nama crate SAAT INI berbeda** (mungkin `acp-bridge` tanpa prefix `galleon-`, atau `zerocode-acp`/sejenis) — cek `Cargo.toml` workspace members untuk nama crate ACP bridge aktual sebelum eksekusi.

**Langkah pertama WAJIB:** `grep -rn "acp" Cargo.toml` atau baca `[workspace] members` di root `Cargo.toml` untuk nama crate persis.

**ASUMSI:** ODR-24 (dirujuk dokumen) berisi keputusan detail tentang bridge ini — baca ODR-24 (`standard-docs/decisions/owner-decisions/`) sebelum eksekusi untuk konteks lengkap yang tidak saya verifikasi sesi ini.

## B. Provider LLM — Reliability & OAuth

- [x] **BL-25 (MEDIUM)** BYOC — override `client_id` OAuth xAI/OpenAI. Naik prioritas kalau keputusan open-source "ya".

#### Detail Eksekusi — BL-25

**File & simbol spesifik:** `crates/clawcrew-providers/src/auth/openai_oauth.rs` (2 ref ditemukan saat verifikasi BR-B6 Sprint 0) — titik OAuth OpenAI existing. Cari file setara untuk xAI (belum diverifikasi, kemungkinan `crates/clawcrew-providers/src/auth/xai_oauth.rs` atau provider xAI tidak punya OAuth terpisah — cek `crates/clawcrew-providers/src/` listing dulu).

**Langkah pengerjaan:**
1. Baca `openai_oauth.rs` untuk pola `client_id` yang di-hardcode/konfigurasi saat ini.
2. Tambah field config BYOC (Bring Your Own Client — `client_id` override dari config user, bukan hardcoded aplikasi) — konsisten pola config override lain di `clawcrew-config/src/schema.rs`.

**Dependency:** "Naik prioritas kalau keputusan open-source ya" — ini bergantung pada keputusan Owner terpisah (apakah Galleon open-source), BUKAN dependency teknis. Tidak blocking untuk mulai, tapi prioritas eksekusi bisa naik/turun sesuai keputusan itu.

- [x] **BL-26 (LOW effort/MEDIUM impact)** Dokumentasikan model keamanan single-tenant auth.

#### Detail Eksekusi — BL-26

**Langkah pengerjaan:** Dokumentasi murni (markdown baru di `standard-docs/tech-standart/`) — jelaskan model auth single-tenant (satu operator per instance Galleon, kontras dengan BG-C1/C2/C3 Sprint 0 yang membangun multi-principal identity). Rujuk `clawcrew-api/src/principal.rs` `PrincipalId::shared_operator()` sebagai representasi kode dari single-tenant trust model.

**Dependency:** Tidak hard-block, tapi lebih bermakna SETELAH Sprint 0 BG-C1 selesai (konsep "principal" sudah establish).

- [x] **BL-27 (LOW effort)** Perbaiki UX pesan error OAuth refresh-revoked.

#### Detail Eksekusi — BL-27

**TIDAK DIVERIFIKASI sesi ini** — pesan error OAuth existing belum dibaca. **Langkah pertama:** cari pesan error di `clawcrew-providers/src/auth/` terkait token revoked/expired, evaluasi apakah pesan ke user cukup actionable (menjelaskan harus re-auth, bukan generic "error").

- [x] **BL-28 (MEDIUM, tunggu BL-15 untuk baseline)** Client-side rate limiting LLM provider calls. Dep: BL-15 (Sprint 5, selesai duluan).

#### Detail Eksekusi — BL-28

**Dependency:** BL-15 (Sprint 5) — tunggu resource gate sub-agent selesai dulu sebagai baseline pola rate-limiting yang established, REUSE pendekatan yang sama (fail-closed, config threshold) untuk LLM provider calls di tiket ini.

**ASUMSI:** Scope detail bergantung pada hasil BL-15 — tidak bisa dirinci lebih jauh sebelum pola BL-15 established.

- [x] **BL-29 (LOW-MEDIUM)** Lengkapi test reliability: cascading-all-down & cooldown-expiry.

#### Detail Eksekusi — BL-29

**TIDAK DIVERIFIKASI sesi ini** — test reliability existing untuk provider failover belum dibaca. **Langkah pertama:** cari test existing di `crates/clawcrew-providers/` terkait failover/cooldown untuk tahu apa yang sudah ditest vs gap "cascading-all-down" (semua provider down bergantian) dan "cooldown-expiry" (provider kembali tersedia setelah cooldown).

## C. Memory Governance

- [ ] **BL-30 (MEDIUM-HIGH)** `erase_identity()` lintas-backend untuk memory — saat ini primitif delete nyata ada TAPI tidak ada single entry-point "hapus semua data saya". (REVISI 2026-10-09, terverifikasi 2x independen: `erase_identity`/`forget_all_for_agent` NOL match di `crates/clawcrew-memory/`. Satu sesi sempat implementasi fungsi ini dan compile hijau, tapi hilang sebelum commit akibat race-condition git checkout dari task lain — diff lengkap ada di transkrip sesi, siap di-reapply, lihat `finding/2026-10-09-remediation-progress-handoff.md` §T-03.)

#### Detail Eksekusi — BL-30

**Koreksi nama fungsi:** `erase_identity()` **TIDAK ADA** di kode manapun (0 match repo-wide) — ini nama fungsi YANG DIUSULKAN dokumen untuk DIBUAT, bukan fungsi existing yang hilang (beda kategori dari BL-32 WhatsApp signature). Primitif delete yang SUDAH ADA (per-backend, bukan cross-backend): `crates/clawcrew-memory/src/sqlite.rs:1676` (`async fn forget(&self, key: &str) -> anyhow::Result<bool>`) dan `:1688` (`async fn forget_for_agent(&self, key: &str, agent_id: &str) -> anyhow::Result<bool>`), bagian dari `impl Memory for SqliteMemory` (baris 1465). **Pola yang SAMA berulang di 11 file backend lain** (`postgres.rs`, `qdrant.rs`, `markdown.rs`, `agent_scoped.rs`, `agent_scoped_markdown.rs`, `none.rs`, `lucid.rs`, `scanned.rs`, `retrieval.rs`, `audit.rs`, `consolidation.rs`) — SETIAP backend implementasi `Memory` trait punya `forget`/`forget_for_agent` SENDIRI, TIDAK ada satu titik yang memanggil SEMUA backend sekaligus untuk satu principal/user.

**Spesifikasi API:**
```rust
/// Erase ALL memory data for one identity, across EVERY configured backend.
/// Unlike `Memory::forget` (single key, single backend), this is the
/// single entry-point for "delete everything about me" (GDPR-style).
///
/// Iterates every active Memory backend instance and calls its
/// `forget_for_agent`-equivalent sweep. Returns per-backend outcome so a
/// partial failure (one backend unreachable) is visible, not silently lost.
pub async fn erase_identity(
    backends: &[Arc<dyn Memory>], // SEMUA backend aktif, bukan satu
    agent_id: &str,
) -> EraseIdentityReport {
    let mut report = EraseIdentityReport::default();
    for backend in backends {
        match backend.forget_all_for_agent(agent_id).await { // method BARU, lihat di bawah
            Ok(count) => report.record_success(backend.name(), count),
            Err(e) => report.record_failure(backend.name(), e),
        }
    }
    report
}
```
**Catatan penting:** `forget_for_agent(key, agent_id)` existing butuh `key` SPESIFIK — untuk "hapus SEMUA data" per agent, trait `Memory` kemungkinan perlu method BARU `forget_all_for_agent(agent_id: &str) -> Result<u64>` (hapus semua key milik agent_id, bukan satu key) — ini perubahan TRAIT (`Memory`), bukan hanya fungsi wrapper baru. Cek dulu apakah ada method sapu-bersih existing per backend sebelum menambah — kemungkinan TIDAK ada (butuh ditambahkan di 12 file backend).

**Langkah pengerjaan:**
1. Baca definisi trait `Memory` penuh (lokasi persis belum ditemukan di grep sesi ini — kemungkinan `crates/clawcrew-memory/src/lib.rs` atau file terpisah, CARI dulu).
2. Tentukan: tambah method `forget_all_for_agent` ke trait `Memory` (breaking change ke SEMUA 12 implementasi), ATAU buat helper terpisah yang memanggil `forget` berulang per-key (butuh cara enumerasi semua key milik agent — cek apakah ada).
3. Implementasikan di SETIAP 12 backend file — ini kerja BESAR (12 file terpisah), pertimbangkan apakah MEDIUM-HIGH severity dokumen asli cukup mencerminkan scope ini (12 file > severity biasanya LOW/MEDIUM single-file).
4. Buat fungsi top-level `erase_identity()` yang mengiterasi semua backend aktif.
5. Expose via endpoint gateway (settings/privacy page) — satu tombol "Delete all my data".

**Acceptance criteria testable:** Panggil `erase_identity(all_backends, "test-agent")` dengan data tersebar di ≥2 backend berbeda (misal sqlite + markdown) → assert SEMUA data agent itu hilang di KEDUA backend, laporan menunjukkan sukses per-backend.

**Edge case:** Satu backend tidak reachable (misal Postgres down) saat `erase_identity` dipanggil — laporan HARUS menunjukkan backend mana yang gagal, BUKAN silent partial-success yang terlihat seperti selesai total (resiko privasi: user percaya datanya terhapus padahal masih ada di satu backend).

**JANGAN DIUBAH:** `forget`/`forget_for_agent` existing per-backend — tetap ada untuk kasus single-key delete, `erase_identity` adalah LAPISAN BARU di atasnya.

**ASUMSI:** Scope "12 file backend" adalah estimasi dari grep sesi ini — tim eksekusi WAJIB konfirmasi jumlah pasti implementasi trait `Memory` sebelum commit ke estimate effort.

- [ ] **BL-31 (MEDIUM, scope besar, evaluasi trait dulu)** Hygiene/retensi otomatis backend-agnostic — saat ini hanya jalan di SQLite. (REVISI 2026-10-09: terverifikasi 2x independen — `backend.rs::uses_sqlite_hygiene` flag eksplisit SQLite/Lucid=true, Postgres/Qdrant=false. KESIMPULAN: SQLite-only genuinely BY DESIGN, bukan bug — produk local-first, Postgres/Qdrant diasumsikan admin-managed retention. Rekomendasi: checkbox tetap `[ ]` tapi scope diringkas jadi "tambahkan trait method `apply_retention` default no-op" (kerja kecil, lihat `memory-hygiene-retention-trait-evaluation.md` + `finding/2026-10-09-architectural-decisions-analysis.md`), bukan generalisasi penuh ke 12 backend.)

#### Detail Eksekusi — BL-31

**Verifikasi:** `crates/clawcrew-memory/src/hygiene.rs` — `pub fn run_if_due(config: &MemoryConfig, workspace_dir: &Path) -> Result<()>` memakai `rusqlite::Connection` LANGSUNG (import `use rusqlite::{Connection, params};` di top-level file) — konfirmasi TEPAT klaim dokumen: hygiene HANYA SQLite, hardcoded ke `rusqlite` API, tidak generic terhadap trait `Memory`. `HygieneReport`/`HygieneState` struct (baris 14-33) menghitung `archived_memory_files`, `purged_memory_archives`, `pruned_conversation_rows`, dll. — field-field ini mengasumsikan model SQLite rows, tidak portable ke backend vector (Qdrant) atau markdown.

**Langkah pengerjaan (EVALUASI trait dulu, sesuai label dokumen):**
1. Baca `hygiene.rs` penuh — `run_if_due`, `should_run_now`, dan logic retention per kategori (`PolicyEnforcer::retention_days_for_category`).
2. Evaluasi: apakah trait `Memory` perlu method baru `fn apply_retention(&self, policy: &RetentionPolicy) -> Result<HygieneReport>` yang SETIAP backend implementasikan sendiri (SQLite pakai SQL DELETE, Qdrant pakai vector delete API, markdown pakai file delete) — ini breaking change trait yang signifikan (sama kategori dengan BL-30, 12 file).
3. Tulis dokumen evaluasi (bukan langsung kode) — apakah worth effort-nya SEKARANG atau ditunda sampai lebih banyak backend non-SQLite benar-benar dipakai di produksi (kalau SQLite masih backend dominan, severity MEDIUM mungkin bisa diturunkan).

**Dependency:** BL-30 (tema sama — trait `Memory` extension untuk operasi cross-backend) — PERTIMBANGKAN menggabungkan desain trait extension BL-30 dan BL-31 dalam SATU perubahan trait, bukan dua breaking change terpisah ke 12 file yang sama.

**JANGAN DIUBAH:** `hygiene.rs` SQLite-specific logic yang sudah berjalan — kalau trait extension dibangun, SQLite backend tetap jadi implementasi REFERENSI pertama (reuse logic yang sudah ada, port ke bentuk trait method, bukan tulis ulang dari nol).

**ASUMSI:** Rekomendasi menggabungkan BL-30+BL-31 desain trait adalah usulan saya berdasarkan overlap scope (12 file sama, konsep sama "operasi cross-backend") — ini PERLU persetujuan Lead sebelum eksekusi, bukan keputusan final.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `apps/zerocode` (→ `galleoncode`), `apps/zerorelay` (→ `galleonrelay`), `crates/galleon-acp-bridge/` | Backend-Rust |
| B | `crates/clawcrew-providers/src/` | Backend-Rust |
| C | `crates/clawcrew-memory/src/` | Backend-Rust |

## Keluaran Sprint

- CLI/TUI sepenuhnya lewat Client Protocol Go, tidak ada lagi daemon Rust ephemeral yang di-spawn langsung.
- Satu entry-point hapus semua data user, bekerja lintas backend memory (bukan hanya SQLite).
