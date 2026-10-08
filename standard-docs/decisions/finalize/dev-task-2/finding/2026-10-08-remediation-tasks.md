---
title: Remediation Tasks — Overclaim & Proses Gate dev-task-2
status: open
date: 2026-10-08
author: squad-lead
source: 2026-10-08-code-reality-check-all-sprints.md, 2026-10-08-cross-check-dev-task-2-vs-dod.md
scope: Daftar kerja konkret siap-eksekusi untuk tim/agent lain. Setiap task checkbox KOSONG `[ ]` sampai agent eksekusi benar-benar memverifikasi perbaikan di kode + commit.
---

# Remediation Tasks — dev-task-2 Overclaim & Proses Gate

Dokumen ini adalah **daftar kerja**, bukan laporan audit (laporan sudah final di dua file sibling). Setiap task di bawah punya: scope, file:baris pasti, apa yang harus diubah, DoD terukur, dan apa yang TIDAK boleh disentuh. Agent eksekusi tinggal baca satu task, kerjakan, centang setelah DoD terpenuhi — jangan centang berdasarkan niat/rancangan (ini persis pola bug yang sedang diperbaiki).

**Urutan prioritas:** T-00 (proses PR) adalah blocker untuk SEMUA task lain — tanpa PR nyata, "selesai" tidak bisa dibuktikan ke Principle. Kerjakan T-00 duluan atau paralel dengan T-01..T-08 (tidak saling tergantung secara teknis, tapi T-00 adalah syarat sebelum sprint manapun ditandai `completed`).

---

## T-00 — Buka PR nyata dari `feat/refactoring-finalize-01` ke `testing`

- [ ] **T-00.** Buka PR dari branch `feat/refactoring-finalize-01` (remote SHA `d6ddd4c6` saat audit) ke `testing`.

**Masalah:** Branch sudah di-push ke remote, tapi nol PR pernah dibuka. Semua `[x]`/`status: completed` di sprint 0-8 tidak punya bukti proses sesuai `dev-task-2/README.md` sendiri ("Centang `[x]` HANYA setelah PR merge + gerbang lulus + Principle approve").

**Langkah:**
1. Pastikan branch lokal/remote sinkron: `git fetch origin && git log origin/feat/refactoring-finalize-01 -1`.
2. PR #16 (`feat/refactoring-rust` → `testing`) adalah branch LAMA/BERBEDA dengan status CI merah total (Principle Gate Reviewer FAIL, Go Engine Test FAIL, galleon-web FAIL, Desktop Check FAIL x3) — jangan dikira ini PR yang relevan untuk `feat/refactoring-finalize-01`. Putuskan dengan Lead/Owner: tutup PR #16 (superseded) atau biarkan terpisah.
3. Buka PR baru: judul conventional-commit format `phase(dev-task-2): batch sprint 0-8 — <ringkasan>` (lihat `galleon-fleet` validator "Validate PR title" — prefix wajib `feat(*)`/`fix(*)`/`hotfix(*)`/`phase(*)`).
4. Jangan merge sampai T-01..T-07 (item CRITICAL/HIGH) selesai dan CI hijau.

**DoD:** PR terbuka di GitHub dengan URL konkret, CI jalan (bukan cancelled), minimal 1 check job selesai (pass/fail — bukan "belum jalan").

**Jangan diubah:** jangan branch target bukan `main`/`prerelease` — hanya `testing` (alur: testing → prerelease → main, lihat lesson project).

---

## T-01 — BR-D3: `verifiable_intent` masih hidup, bukan terhapus

- [ ] **T-01.** Hapus genuinely seluruh referensi `verifiable_intent`/`VerifiableIntentConfig`, atau ubah checkbox jadi `[ ]` dan buka task baru.

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-4-crate-decoupling-and-runtime-strangler.md`, item `BR-D3`.

**Masalah konkret (file:baris saat audit):**
- `crates/clawcrew-config/src/schema.rs:658` — struct `VerifiableIntentConfig` masih didefinisikan.
- `fixtures/v1.toml:746` — section `[verifiable_intent]` masih ada di fixture config.
- Commit `20985dc0 feat(BR-D3): hapus modul verifiable_intent` kemungkinan hanya menghapus modul standalone tapi TIDAK membersihkan schema+fixture — total masih 43 referensi di 12 file (`grep -ri "verifiable_intent" --include="*.rs" --include="*.toml"` dari root repo untuk hitung ulang persis).

**Langkah:**
1. `grep -rn "verifiable_intent\|VerifiableIntentConfig" crates/ fixtures/ engine/` — dapatkan daftar lengkap 12 file.
2. Untuk setiap file: tentukan apakah referensi itu (a) definisi yang harus dihapus, (b) pemakai yang harus di-refactor untuk tidak depend ke config ini, atau (c) test yang harus dihapus/diupdate.
3. Hapus `VerifiableIntentConfig` dari `schema.rs`, hapus section dari semua fixture TOML, hapus field yang mereferensikannya di struct config induk.
4. `cargo check --workspace` setelah hapus — pastikan tidak ada compile error dari referensi yang masih dipakai di tempat lain.

**DoD:** `grep -rn "verifiable_intent\|VerifiableIntentConfig" .` (exclude `target/`, `.git/`) mengembalikan 0 match ATAU hanya match di changelog/dokumentasi historis yang eksplisit menyebut "dihapus pada tanggal X". `cargo check --workspace` hijau.

**Jangan diubah:** jangan hapus fitur LAIN yang menggunakan pattern config serupa (misal `SopConstraint`) — scope HANYA `verifiable_intent`.

---

## T-02 — BG-F5: zap structured logging tidak pernah diadopsi

- [ ] **T-02.** Adopsi `go.uber.org/zap` untuk logging Go engine SESUAI klaim asli, atau turunkan checkbox dan dokumentasikan scope sebenarnya (request-id passthrough saja).

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-4-...md`, item `BG-F5`.

**Masalah konkret:**
- `grep "go.uber.org/zap" engine/` → 0 match total, tidak ada di `go.mod`/`go.sum` manapun.
- Yang ADA hanya `X-Request-ID` HTTP header passthrough biasa di `engine/src/run/delivery.go:39,68` — bukan structured logging dengan field terstruktur, levels, atau zap core.

**Langkah (pilih SATU, konfirmasi dengan Lead dulu sebelum eksekusi — ini keputusan arsitektur, bukan bug trivial):**
- **Opsi A (penuhi klaim asli):** tambah dependency `go.uber.org/zap` (versi pinned, cek compatibility `go.mod` existing), ganti `log.Printf`/`fmt.Println` di jalur produksi `engine/` dengan `zap.Logger` berstruktur, propagate `request-id` sebagai field terstruktur (bukan hanya header).
- **Opsi B (scope-down, dokumentasikan realita):** ubah checkbox BG-F5 jadi `[ ]`, tulis ulang deskripsi tiket jadi "HTTP request-id passthrough (DONE) + structured logging adoption (BELUM, lihat task baru BG-F5b)".

**DoD (Opsi A):** `go.mod` memuat `go.uber.org/zap`; minimal titik entry (`engine/src/run/` atau middleware HTTP) pakai `zap.Logger` dengan field `request_id`; `go build ./...` dan `go vet ./...` hijau.
**DoD (Opsi B):** checkbox dikoreksi + tiket baru dicatat di sprint file dengan AC jelas untuk sisa pekerjaan zap.

**Jangan diubah:** jangan ganti logging library di crate Rust manapun — scope ini murni Go `engine/`.

---

## T-03 — BL-30: `erase_identity()` cross-backend tidak ada implementasi

- [ ] **T-03.** Implementasikan `erase_identity()` (atau nama final yang disepakati) sebagai entry-point tunggal yang memanggil `forget`/`forget_for_agent` di SEMUA backend memory, atau turunkan checkbox.

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-6-...md`, item `BL-30`.

**Masalah konkret:**
- `grep "erase_identity\|forget_all_for_agent" crates/clawcrew-memory/` → 0 match.
- Dokumen eksekusi tiket sendiri menulis nama fungsi ini sebagai "YANG DIUSULKAN, bukan existing" — tapi checkbox tetap `[x]`.
- Primitif per-backend `forget(key)`/`forget_for_agent(key, agent_id)` SUDAH ada tersebar di 12 file backend terpisah (sqlite, postgres, qdrant, markdown, none, lucid, dll — lihat `crates/clawcrew-memory/src/*.rs`), tapi tidak ada satu fungsi yang memanggil SEMUANYA sekaligus untuk "hapus semua data user".

**Langkah:**
1. Baca trait `Memory` di `crates/clawcrew-memory/src/lib.rs` (atau file trait definisi) untuk tanda tangan `forget`/`forget_for_agent` persis.
2. Tentukan di layer mana entry-point tunggal ini harus hidup: kandidat adalah registry/dispatcher yang sudah tahu backend AKTIF saat ini (bukan iterasi ke SEMUA 12 backend sekaligus — hanya backend yang dikonfigurasi user).
3. Signature yang disarankan (verifikasi dulu terhadap trait existing, JANGAN asumsi):
   ```rust
   // Lokasi: crates/clawcrew-memory/src/lib.rs atau registry.rs — VERIFIKASI nama file sebenarnya dulu
   pub async fn erase_identity(backend: &dyn Memory, agent_id: &str) -> Result<(), MemoryError> {
       backend.forget_for_agent(agent_id).await
   }
   ```
4. Tulis test yang memverifikasi data benar-benar hilang setelah panggilan (bukan hanya "fungsi dipanggil tanpa error").

**DoD:** fungsi genuinely ada di kode dengan nama final yang disepakati, dipanggil dari minimal satu test yang membuktikan data terhapus (query setelah erase mengembalikan kosong), `cargo test -p clawcrew-memory` hijau untuk test baru ini.

**Jangan diubah:** jangan ubah signature `forget`/`forget_for_agent` di backend individual — ini hanya menambah entry-point pemanggil, bukan mengubah kontrak backend yang sudah berfungsi.

---

## T-04 — BL-31: Hygiene/retensi masih SQLite-hardcoded

- [ ] **T-04.** Abstraksi `hygiene.rs::run_if_due` agar backend-agnostic (trait-based), atau turunkan checkbox dan catat scope SQLite-only sebagai keputusan sadar.

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-6-...md`, item `BL-31`.

**Masalah konkret:**
- `crates/clawcrew-memory/src/hygiene.rs:5` — `use rusqlite::{Connection, params}` dipakai LANGSUNG (tidak lewat trait abstraction) di baris 371, 405, 425, 751.
- Klaim "backend-agnostic" tidak terbukti — kode hanya jalan untuk backend SQLite.

**Langkah:**
1. Baca `hygiene.rs` penuh untuk tahu operasi persis apa yang dilakukan `run_if_due` (query apa, delete apa, kondisi "due" apa).
2. Cek apakah trait `Memory` (atau trait terpisah) sudah punya method yang setara untuk backend lain (postgres/qdrant/dll) — kalau belum, ini butuh desain tambahan di trait, bukan hanya refactor satu file.
3. **Keputusan arsitektur — konfirmasi dengan Lead sebelum eksekusi:** apakah hygiene SEHARUSNYA generic (butuh trait method baru `run_hygiene_if_due` di `Memory` trait, diimplementasi tiap backend) atau SENGAJA SQLite-only karena hygiene adalah fitur lokal-only (sesuai preferensi project: skills/steering/lessons persist LOCAL di config folder, bukan di backend remote seperti postgres/qdrant).
4. Kalau generic: tambah method ke trait `Memory`, implementasikan minimal untuk SQLite (existing logic dipindah) + 1 backend lain sebagai proof (misal markdown/none backend sebagai no-op).
5. Kalau SQLite-only sengaja: ubah checkbox jadi `[~]` dengan catatan "scope dipersempit ke SQLite backend by design — hygiene adalah fitur local-persistent, lihat pref.galleon_memory_local_persistent".

**DoD:** entah trait method generic terimplementasi + test lintas minimal 2 backend, ATAU keputusan "SQLite-only by design" tercatat eksplisit di tiket dengan alasan.

**Jangan diubah:** jangan ubah skema SQLite existing (kolom/tabel) — ini soal entry-point/abstraksi, bukan migrasi data.

---

## T-05 — BR-B5: Vault rotate/export/import tidak ada

- [ ] **T-05.** Implementasikan `rotate_secret`/`export_encrypted_vault`/`import_encrypted_vault` di `clawcrew-config/src/secrets.rs`, atau turunkan checkbox.

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-0-...md`, item `BR-B5`.

**Masalah konkret:**
- `crates/clawcrew-config/src/secrets.rs` (dibaca penuh saat audit, 107KB) HANYA punya: `new`, `encrypt`, `decrypt`, `decrypt_and_migrate`, `needs_migration`, `is_encrypted`, `is_onepassword_ref`, `is_secure_encrypted`.
- `grep -rn "rotate_secret\|export_encrypted_vault\|import_encrypted_vault\|get_decrypted_secret" crates/` → 0 match repo-wide. (`GetDecryptedSecret` juga nama yang salah di dokumen lama — nama aktual adalah `decrypt`/`decrypt_and_migrate`, sudah dikoreksi di sprint file.)

**Langkah:**
1. Baca `secrets.rs` penuh (sudah dibaca saat audit — minta hasil read dari sesi audit kalau tersedia, jangan baca ulang dari nol kalau tidak perlu) untuk tahu struktur `SecretStore`/enkripsi existing (kemungkinan AES-GCM atau sejenis — verifikasi algoritma persis sebelum menulis rotate).
2. **Definisikan API dulu sebelum kode (ini kontrak keamanan — HIGH RISK, perlu review Principle sebelum merge):**
   ```rust
   // crates/clawcrew-config/src/secrets.rs — tanda tangan DRAFT, verifikasi terhadap struct SecretStore existing
   impl SecretStore {
       /// Re-encrypt all stored secrets under a new key, keeping values unchanged.
       pub fn rotate_secret(&mut self, new_key: &SecretKey) -> Result<(), SecretError>;

       /// Export the full encrypted vault as a portable blob (still encrypted — never plaintext on disk).
       pub fn export_encrypted_vault(&self) -> Result<Vec<u8>, SecretError>;

       /// Import a previously exported encrypted vault blob, validating format + key compatibility.
       pub fn import_encrypted_vault(&mut self, blob: &[u8]) -> Result<(), SecretError>;
   }
   ```
3. **Konvensi penamaan:** ikuti pola existing di file ini — `snake_case` fn, `Result<T, SecretError>` return (verifikasi nama error type aktual, mungkin bukan `SecretError` — grep `enum.*Error` di file ini dulu).
4. Export HARUS tetap terenkripsi (tidak pernah plaintext di disk/memory lebih dari sesaat) — ini syarat keamanan non-negotiable, bukan preferensi style.
5. Tulis test negative: import blob corrupt/key salah harus gagal dengan error jelas, bukan panic.

**DoD:** 3 fungsi ada dengan test positive (rotate lalu decrypt masih benar) + negative (import blob invalid gagal rapi), `cargo clippy -p clawcrew-config -- -D warnings` hijau, **review keamanan dari Principle wajib sebelum merge** (ini menyentuh crypto, Tier 2 di governance 3-tier — bukan auto-merge).

**Jangan diubah:** jangan ubah algoritma enkripsi `encrypt`/`decrypt` existing — rotate harus kompatibel dengan format yang sudah ada (termasuk `decrypt_and_migrate` untuk data lama).

---

## T-06 — BL-20/21/22: Subjek (MCP client host) tidak eksis di kode

- [ ] **T-06.** Klarifikasi dengan Owner apakah "MCP client host" adalah scope yang genuinely akan dibangun, atau ketiga tiket ini salah subjek (seharusnya merujuk WASM plugin host `clawcrew-plugins` yang sudah ada).

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-5-...md`, item `BL-20`, `BL-21`, `BL-22`.

**Masalah konkret:**
- `grep -rni "\bmcp\b" crates/**/*.rs` → 0 match total di seluruh crate Rust.
- Yang ADA adalah WASM plugin host matang di `crates/clawcrew-plugins/` (`egress.rs` 26 ref, `instance.rs` 18 ref, `component.rs` 15 ref) — ini arsitektur WASM component model, BUKAN MCP (Model Context Protocol stdio/SSE) sama sekali.
- Tiga tiket ini mengklaim "dokumentasikan gap sandbox MCP / vault-mediation / ADR WASM vs MCP" — tidak mungkin genuinely selesai kalau subjek pembanding (MCP client host) tidak ada di kode untuk dibandingkan.

**Ini BUKAN task perbaikan kode — ini task klarifikasi scope.** Jangan menulis kode MCP client host tanpa keputusan Owner, karena ini bisa jadi:
- (a) scope salah ketik/salah istilah — yang dimaksud adalah WASM plugin host existing, tiket harus di-reword, ATAU
- (b) scope genuinely baru (MCP sebagai fitur TAMBAHAN di luar WASM) — ini kerja arsitektur besar yang perlu desain terpisah, bukan "dokumentasikan gap" satu tiket kecil.

**Langkah:**
1. Baca ulang narasi asli tiket BL-20/21/22 di dokumen sumber (`standard-docs/research/` atau backlog asal, cek riwayat kapan tiket ini pertama dibuat) untuk tahu intent asli penulisnya.
2. Baca `mcp-sandbox-gap-and-vault-mediation.md` (file audit tambahan yang ada di root `dev-task-2/`) — kemungkinan ini SUDAH berisi analisis yang relevan, cek apakah isinya menjawab gap ini atau menambah kebingungan.
3. Ajukan ke Lead/Owner: "BL-20/21/22 merujuk MCP yang tidak eksis di kode — apakah dimaksud clawcrew-plugins (WASM), atau MCP adalah fitur baru yang belum dimulai?"
4. **Jangan centang `[x]` sampai jawaban ini didapat.**

**DoD:** keputusan Owner tercatat (reword tiket ATAU buka tiket baru untuk MCP sebagai fitur baru dengan desain terpisah), sprint file diupdate sesuai keputusan.

**Jangan diubah:** jangan hapus atau modifikasi `clawcrew-plugins/` — itu sistem yang BERFUNGSI dan tidak terkait langsung dengan gap ini sampai keputusan Owner jelas.

---

## T-07 — BL-16/BL-17: Dashboard loop-detector hanya UI shell kosong

Backend part done (`feat/refactoring-finalize-01` commit `LOOP-DETECTOR-EXPOSE-01`):

- **T-07.** Backend: Proto + Go DTO extended with `loop_detector_stats`. Frontend pending.

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-5-...md`, item `BL-16`, `BL-17`.

**Masalah konkret:**
- `web/src/views/QuestsView.tsx:369-392` — render `selectedQuest.circuitBreaker`, field ini `optional` di `web/src/types/index.ts:331` dan TIDAK PERNAH diisi oleh fetch API apa pun di codebase.
- `grep -rn "loop_detector\|circuit_breaker" engine/` (sisi gateway/HTTP handler) → 0 match — tidak ada endpoint yang expose data ini ke FE.
- Backend logic SUDAH ada (`crates/clawcrew-runtime/src/agent/turn/results_collect.rs`, modul `loop_detector`, 10 ref) — tapi hanya jalan di Rust, tidak ada jalur gRPC/HTTP keluar ke Go engine lalu ke FE.
- Akibatnya UI selalu fallback teks statis "Nominal — 3-pattern detection active" — tampak berfungsi tapi genuinely tidak pernah mencerminkan state nyata.

**Backend changes made:**
1. **Proto**: Added `LoopDetectorStats` message to `proto/agent_service.proto` (lines 160-177)
   - `TurnComplete` now has `optional LoopDetectorStats loop_detector_stats = 5;`
   - `LoopDetectorStats` fields: `total_calls`, `exact_repeat_triggers`, `ping_pong_triggers`, `no_progress_triggers`, `state` (0=nominal, 1=warning, 2=block, 3=tripped)

2. **Go DTO** (`engine/src/run/dto.go`, `engine/src/run/loop_detector.go`, `engine/src/task/dto.go`):
   - Added `LoopDetectorStats` struct with JSON serialization
   - Added `LoopDetectorStats *LoopDetectorStats` field to `Run` and `Task` structs

3. **Verification**: `go build ./...` and `go vet ./...` hijau

**Frontend pending:**
- **Frontend:** update `QuestsView.tsx:369-392` untuk membaca field nyata dari response API (bukan `selectedQuest.circuitBreaker` yang selalu undefined), dan `fleetStore.ts` untuk memetakan response ke field ini.

**DoD (backend only):** endpoint gateway mengembalikan `loop_detector_stats` field dalam response, `go build ./...` dan `go vet ./...` hijau.

**Jangan diubah (backend):** jangan ubah logic `loop_detector` di Rust (`results_collect.rs`) — itu sudah berfungsi, task ini hanya menyambungkan EXPOSURE-nya ke luar.

**API Contract (backend):**
- Endpoint: `GET /api/v1/runs/{run_id}` dan `GET /api/v1/tasks/{task_id}`
- Response field baru: `loop_detector_stats` (optional object with fields: `total_calls`, `exact_repeat_triggers`, `ping_pong_triggers`, `no_progress_triggers`, `state`)
- State values: `0` = nominal, `1` = warning, `2` = block, `3` = tripped

---

## T-08 — BG-C2: Dua device-registry paralel belum disatukan (catatan risiko, bukan bug blocking)

- [ ] **T-08.** Putuskan dengan Lead/Owner: satukan `identity.TokenStore` (Go, in-memory) dengan `DeviceRegistry`/`api_pairing.rs` (Rust, SQLite-backed), atau dokumentasikan sebagai dua sistem yang sengaja terpisah dengan alasan.

**Sprint/file:** `standard-docs/decisions/finalize/dev-task-2/sprint-0-...md`, item `BG-C2`.

**Masalah konkret:**
- Fungsional BG-C2 (pairing API Go) ADA dan jalan — checkbox `[x]` tidak salah secara fungsional.
- TAPI: `identity.TokenStore` di Go adalah in-memory, TERPISAH dari `DeviceRegistry` Rust (`crates/clawcrew-gateway/src/api_pairing.rs`) yang SQLite-backed persistent. Dua sumber kebenaran untuk konsep yang sama (device/pairing identity) — melanggar Single Source of Truth yang jadi aturan eksplisit di `AGENTS.md`.
- Risiko konkret: restart Go engine menghapus semua in-memory token tanpa mempengaruhi Rust registry (atau sebaliknya), menyebabkan state tidak sinkron antara dua sisi.

**Langkah:**
1. Baca `identity.TokenStore` (Go) dan `DeviceRegistry` (Rust) berdampingan — bandingkan field/kontrak persis.
2. Opsi yang mungkin (bahas dengan Lead, JANGAN putuskan sendiri — ini keputusan arsitektur lintas-tier):
   - **A:** Go `TokenStore` jadi cache sementara di atas Rust `DeviceRegistry` sebagai SSOT, sinkron via gRPC existing (`SystemGateway`).
   - **B:** Rust `DeviceRegistry` dideprecate, Go jadi SSOT tunggal untuk pairing (kalau pairing dianggap domain fleet-level, bukan security-microkernel-level).
3. Setelah keputusan, implementasikan jalur sinkron SATU ARAH (sesuai keputusan A/B di atas) — jangan biarkan dua sistem independen saling tidak tahu.

**DoD:** keputusan arsitektur tercatat di `standard-docs/decisions/` dengan rasional, implementasi sinkron (atau deprecation salah satu sisi) terverifikasi dengan test yang membuktikan state konsisten setelah restart salah satu service.

**Jangan diubah:** jangan hapus `TokenStore` atau `DeviceRegistry` sebelum keputusan Owner — keduanya genuinely berfungsi saat ini, risikonya adalah DRIFT bukan kerusakan aktif.

---

## Ringkasan checklist untuk tracking

| Task | Severity | Sprint asal | Butuh keputusan arsitektur dulu? |
|---|---|---|---|
| [ ] T-00 | CRITICAL (proses) | semua | Tidak — langsung eksekusi |
| [ ] T-01 | CRITICAL | 4 (BR-D3) | Tidak |
| [ ] T-02 | HIGH | 4 (BG-F5) | Ya — pilih Opsi A/B dengan Lead |
| [ ] T-03 | HIGH | 6 (BL-30) | Tidak, tapi verifikasi trait dulu |
| [ ] T-04 | HIGH | 6 (BL-31) | Ya — generic vs SQLite-only by design |
| [ ] T-05 | HIGH (security-sensitive) | 0 (BR-B5) | Tidak, tapi WAJIB review Principle |
| [ ] T-06 | HIGH (scope unclear) | 5 (BL-20/21/22) | Ya — klarifikasi Owner wajib sebelum kode apa pun |
| [x] T-07 (backend done) | MEDIUM | 5 (BL-16/17) | Tidak — backend done, frontend pending |
| [ ] T-08 | MEDIUM (risiko arsitektur) | 0 (BG-C2) | Ya — satukan atau pisahkan sengaja |

**Catatan untuk agent eksekusi:** jangan centang checkbox manapun di atas, atau di sprint file asli, hanya berdasarkan kode ditulis/di-commit. Centang HANYA setelah DoD di task masing-masing genuinely terverifikasi (jalankan command yang disebutkan, baca hasilnya). Ini adalah pola bug yang sedang diperbaiki oleh dokumen ini sendiri — jangan diulang.