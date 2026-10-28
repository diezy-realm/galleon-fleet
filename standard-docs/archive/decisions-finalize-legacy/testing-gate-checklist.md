# Testing Gate Checklist — Finalize (Phase 2 → Phase 3 → Phase 4)

> **Pembaruan 2026-10-04:** gerbang G0–G8 untuk task yang sudah [x] belum dicentang Principle (GV-12). Gerbang untuk pekerjaan baru ada di 
oadmap/PR-plan.md (GW-0..GW-9). G6 (T-6.5..6.10) ditahan sampai keputusan CO-09.

- **Status:** Siap pakai — berlaku untuk SETIAP PR dari `task-breakdown.md` (Grup 0–9), branch `feat/refactoring-rust`.
- **Author:** squad-lead
- **Tanggal:** 2026-10-04
- **Relasi:** Pola dokumen ini mengikuti `RF-C-channel-crate-split-self-register.md` ("Principle Review Checklist") dan `RF-D-principle-gateway-checklist.md` ("Gate D-1..D-5"), diadaptasi untuk scope multi-stack (Go + Rust + Web/Tauri) di `standard-docs/decisions/finalize/`. Gate umum di Bagian 0 TETAP BERLAKU untuk semua PR; Bagian 1–9 menambah gate KHUSUS per Grup yang tidak ada di checklist RF-C/RF-D (keduanya murni Rust channel/orchestrator).
- **Sumber acceptance criteria:** `analysis-conclusion.md` (temuan gap) + `task-breakdown.md` (task & acceptance per Grup 0–8 + checkpoint Grup 9).

---

## Bagian 0 — Gate Umum (WAJIB untuk SEMUA PR, sebelum cek gate spesifik Grup)

- [ ] **Satu task = satu PR kecil.** Branch non-`master`, PR target `master`. Tolak PR yang mencampur task dari dua Grup berbeda tanpa alasan eksplisit di description.
- [ ] **SSOT:** tidak ada dua sumber kebenaran hidup bersamaan untuk fakta yang sama (contoh: risk-tier, metrics, secret) setelah PR merge — kode lama yang digantikan benar-benar dihapus/dirute ulang, bukan di-comment-out atau dibiarkan sebagai dead path.
- [ ] **Tidak ada `unwrap()`/`expect()`/`panic!` baru** di production path Rust, dan tidak ada `panic`/unchecked type-assertion baru di Go/TypeScript pada path yang dieksekusi runtime (bukan test).
- [ ] **Tracking wajib dipenuhi** sesuai aturan `task-breakdown.md`: task yang diklaim selesai punya tanggal, file yang disentuh, command verifikasi + exit code/durasi, dan ringkasan perubahan arsitektur. PR tanpa ini **ditolak langsung**, terlepas dari kualitas kode.
- [ ] **Command verifikasi dijalankan ULANG oleh Principle sendiri** di worktree — jangan percaya klaim "sudah lolos" di PR description tanpa re-run minimal command fast-loop yang relevan.
- [ ] **CI hijau** sebelum APPROVE (semua job: Go, Rust, Web yang applicable untuk file yang diubah).

**Reject jika:** salah satu di atas gagal, atau PR tidak bisa dipetakan ke task ID manapun di `task-breakdown.md`.

### Command verifikasi baseline (reuse dari `task-breakdown.md` §"Standar Verifikasi")

```powershell
# Go
cd engine; go vet ./...; go test ./...; cd ..

# Rust
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo nextest run -p galleon-architecture-tests   # SSOT & invariant gate

# Web
cd web; npm run lint; npm run build; cd ..
```

---

## Gate G0: Quick Win — ADR 0003 Tauri Real Metrics (T-0.1)

- [ ] `get_system_metrics()` di `apps/tauri/src/main.rs` memanggil `engine_get(...)` — grep untuk konstanta lama (`gateway_latency_ms: 8`, `memory_db_mb: 18.4`) harus **KOSONG** di file tersebut.
- [ ] Ada fallback graceful eksplisit (bukan `panic`/`unwrap`) saat engine Go tidak terjangkau.
- [ ] `cargo check -p <tauri-crate>` hijau.

**Reject jika:** fallback mengembalikan konstanta palsu baru alih-alih status error yang jujur ("engine unreachable").

---

## Gate G1: Frontend QA Blocker — Buka Jalan C3 (T-1.1 – T-1.3)

- [ ] Tipe `ToolDefinition`/`ProcessItem` diimpor dari sumber tunggal (`apiClient.ts`), tidak didefinisikan ulang duplikat di komponen (`HarborView.tsx`, `EngineRoomView.tsx`).
- [ ] `npm run lint` (`tsc --noEmit`) hijau — **0 error type**, bukan cuma berkurang.
- [ ] `seedData.ts` tidak lagi diimpor oleh `fleetStore.ts` (grep `seedData` di `web/src/store/` = kosong), store hidrasi dari `apiClient.getCollection()`/endpoint nyata.
- [ ] `npm run build` (`vite build`) sukses, exit code 0.

**Reject jika:** lint hijau dicapai dengan `@ts-ignore`/`any` baru alih-alih shape mapping yang benar.

---

## Gate G2: ADR 0002 — Backend Logic Pindah dari web ke Go (T-2.1 – T-2.3)

- [ ] Endpoint `/api/network/qrcode` dan `/api/system/metrics` **ter-registrasi di Go** (`engine/src/fleet/delivery.go` atau setara) dan punya unit test (`TestHandleNetworkQRCode`, `TestHandleSystemMetrics`) yang lolos.
- [ ] `grep "qrcode"` dan `grep "os.totalmem\|os.cpus"` di `web/server.ts` = **0 match** setelah PR. Principle re-run grep ini sendiri, jangan percaya klaim.
- [ ] `web/src/utils/apiClient.ts` menambahkan method pemanggil endpoint baru (`getNetworkQrCode(url)`), bukan memanggil Node API langsung dari browser/server.ts.
- [ ] `go test ./...` dan `npm run build` keduanya hijau pasca-pemindahan (bukti tidak ada regresi fungsional, QR code & metrics tetap tampil di UI).

**Reject jika:** `web/server.ts` masih memuat logika generate QR/metrics dalam bentuk apapun (termasuk versi "disederhanakan" yang tidak full delegate ke Go) — ADR 0002 melarang **semua** backend logic di web, bukan hanya sebagian.

---

## Gate G3: Phase 2 Lane C Sisa — Route Contract & Learning-by-Consent (T-3.1, T-3.2)

- [ ] **T-3.1 (C4):** PR menyertakan peta/dokumentasi setiap call di `apiClient.ts` vs route terdaftar di `*/delivery.go`; mismatch/404 potensial ditandai dan di-fix dalam PR yang sama atau dilacak sebagai task baru (bukan didiamkan).
- [ ] **T-3.2 (C5):** `MemoryProposal{scope, rule, versioned, reversible}` adalah tipe Artifact yang **benar-benar pending approval** — ada test lifecycle: submit → `pending` → approve (Captain) → ditulis scoped ke memori + reversible → reject tercatat.
- [ ] Tidak ada tulis-langsung ke memori tanpa melalui approval flow untuk koreksi user (melanggar prinsip "learning by consent").

**Reject jika:** T-3.2 menulis langsung ke memori saat submit (skip approval), atau "reversible" hanya klaim tanpa test yang membuktikan revert benar-benar mengembalikan state sebelumnya.

---

## Gate G4: Lane D — Sandbox Tool Execution Fail-Closed (T-4.1 – T-4.3)

> **Kebijakan terkunci:** Rust adalah Core, Go adalah orkestrator. **Error Reject (Fail-Closed)** — dilarang fallback eksekusi lokal di Go.

- [ ] **T-4.1 (D1):** Grep setiap `builtin_*.go` (`builtin_code.go`, `builtin_git.go`, `builtin_web.go`, `builtin_workspace.go`) untuk eksekusi OS langsung (`os/exec`, file I/O langsung tanpa melalui gateway) — HARUS **tidak ada** path yang jalan tanpa melalui `SystemGateway` saat gateway tersedia.
- [ ] Jika `execCtx.Gateway == nil` atau gRPC gagal → fungsi mengembalikan **error eksplisit**, bukan fallback diam-diam ke eksekusi in-process. Principle re-run `go test ./engine/src/tool/...` sendiri dan baca test `direct_dispatch_denial_test.go` untuk konfirmasi assertion-nya benar menguji rejection, bukan sekadar "tidak error".
- [ ] **T-4.2 (D2):** Integration test round-trip Go→Rust untuk minimal 1 tool nyata (`bash`/`read_file`) — log Rust menunjukkan eksekusi RIIL terjadi di proses Rust (bukan di-mock keduanya).
- [ ] **T-4.3:** Path traversal test (`../../etc/passwd`) dikirim via **gRPC call sungguhan** (bukan panggil fungsi Go langsung) dan ditolak Rust dengan error containment yang jelas.

**Reject jika:** fallback in-process apa pun tersisa di `builtin_*.go` saat gateway tersedia, atau test traversal memanggil fungsi lokal Go alih-alih benar-benar lewat gRPC ke Rust (testnya jadi tidak membuktikan boundary nyata).

---

## Gate G5: ADR 0001 — LLM SSOT Rust (T-5.1 – T-5.6) — Implementasi Terbesar

- [ ] **T-5.1 (Proto):** `ExecuteTurn` RPC + pesan `ExecuteTurnRequest`/`TurnToken` (oneof `TokenChunk | ToolCall | TurnComplete | TurnError`) ada di `proto/agent_service.proto`; `cargo build -p galleon-gateway` DAN regenerasi Go pb (`go generate ./engine/pkg/pb`) dua-duanya hijau.
- [ ] **T-5.2 (Rust handler):** `execute_turn` mendelegasikan ke `galleon-providers` (bukan reimplement vendor call baru di gateway); test `test_execute_turn_success` dengan mock provider lolos, meliputi urutan event streaming (`TokenChunk` → `TurnComplete`).
- [ ] **T-5.3:** Integration test Go client memanggil `ExecuteTurn` dan menerima stream token — dijalankan, bukan hanya ditulis sebagai TODO.
- [ ] **T-5.4 (Go delegation):** `grep "llmProvider llm.Provider" engine/src/crew/services.go` = **0 match** pasca-PR. `StartTurn` memanggil `systemGateway.ExecuteTurn()`, bukan `llmProvider.StreamChat()`.
- [ ] **T-5.5:** Test crew tidak lagi menggunakan `mock_provider.go` Go — diganti mock/test-server gRPC Rust.
- [ ] **T-5.6 (Cleanup):** Setelah T-5.4/T-5.5 hijau, file `engine/src/llm/{provider,multi_provider,mock_provider,interfaces,wire,dispatcher}.go` benar-benar **terhapus** (bukan dikosongkan isinya/di-comment). `go build ./...` dan `go test ./...` tetap hijau setelah penghapusan.
- [ ] **API key tidak pernah di-log** di Go maupun Rust pada seluruh alur `ExecuteTurn` — Principle grep log statement di sekitar kode yang menyentuh token/secret.

**Reject jika:** Phase Cleanup (T-5.6) dieksekusi SEBELUM Phase 2 (T-5.4/T-5.5) terverifikasi hijau di CI — urutan dependency wajib dihormati, jangan hapus Go LLM code duluan lalu "perbaiki nanti". Juga reject jika `ExecuteTurn` di Rust mengimplementasikan ulang HTTP call vendor sendiri alih-alih reuse `galleon-providers` (duplikasi SSOT yang baru).

---

## Gate G6: Phase 3 — DynamicRouter Integration (T-6.1 – T-6.10)

> **Prasyarat non-negotiable:** 3 poin sign-off Principle (Gap 1 tipe `PluginCapability`, Gap 2 defer crate-split, Gap 5 jumlah handler & urutan fase) harus tercatat di PR pertama Grup ini sebelum lanjut ke task berikutnya.

- [ ] **T-6.1:** `capability()` di `galleon-routing` mengembalikan `Option<PluginCapability>` (reuse enum `galleon-plugins`), **tidak** mendefinisikan enum capability baru yang duplikat.
- [ ] **T-6.2:** `galleon-gateway` meng-import `galleon-routing` dan memasang `DynamicRouter` sebagai `.fallback()` — sebelumnya 0 import, sesudahnya minimal 1 integration test membuktikan request lewat router ini.
- [ ] **T-6.3 (5 core handler):** Setiap handler (`api_config`, `api_logs`, `auth_rate_limit`, `api_webauthn`, `api_audit`) jadi **thin wrapper** `RouteHandler` — logic bisnis TIDAK ditulis ulang. Ada test collision: plugin yang coba registrasi path yang sama (`/api/config`) ditolak dengan error, bukan silent-override.
- [ ] **T-6.4 (streaming regression):** Test `Body::from_stream(...)` lolos di suite `galleon-routing` SEBELUM Fase 3 (T-6.9/T-6.10) dimulai. Ini adalah gate blocking — PR Fase 3 tanpa bukti T-6.4 hijau **ditolak**.
- [ ] **T-6.5 – T-6.8 (Fase 2, 15 handler):** Setiap batch migrasi: `cargo test -p galleon-gateway` hijau, endpoint terkait tetap merespons benar, test collision T-6.3 masih hijau (regresi dicek ulang, bukan diasumsikan tetap jalan).
- [ ] **T-6.9 – T-6.10 (Fase 3, protocol handler):** SSE/WebSocket upgrade tetap berfungsi setelah migrasi — Principle minta demonstrasi manual atau test otomatis koneksi tidak terputus, bukan hanya `cargo build` hijau.

**Reject jika:** handler Fase 2/3 dimulai sebelum 5 core handler (T-6.3) dan streaming regression (T-6.4) terbukti hijau — urutan fase di `task-breakdown.md` adalah urutan risiko, bukan saran.

---

## Gate G7: ADR-001 — Channel Distribution WASI Pilot (T-7.1 – T-7.3)

- [ ] **T-7.1:** 2 channel Tier C kandidat (`line`/`lark` atau sejenis, inbound-first/webhook) dibangun sebagai `.wasm` plugin menggunakan fixture template yang ADA (`crates/galleon-plugins/tests/fixtures/channel-fixture/`), bukan membangun loader WASI baru dari nol.
- [ ] Round-trip polling pesan masuk teruji di integration test — bukan hanya "plugin ter-load".
- [ ] **T-7.2:** `wit/v0/channel.wit` dibekukan (`.frozen` marker) HANYA setelah pilot T-7.1 hijau; gate CI `wit-breaking-change-check` aktif dan menolak perubahan breaking pada contract yang sudah dibekukan.
- [ ] **T-7.3 (Lapis B):** Crate `galleon-channel-<name>` baru dapat di-build **mandiri** (`cargo check -p galleon-channel-<name>`) DAN gateway inti terukur mengecil (bandingkan `cargo build --timings` sebelum/sesudah, jangan klaim tanpa angka).
- [ ] Terapkan Gate D-1 s/d D-4 dari `RF-D-principle-gateway-checklist.md` untuk setiap sub-PR crate-split di Grup ini — dokumen itu TETAP BERLAKU dan tidak diulang di sini.

**Reject jika:** T-7.2 dieksekusi sebelum pilot T-7.1 terbukti hijau (membekukan contract yang belum tervalidasi), atau candidate channel yang dipilih untuk Tier C ternyata berbagi governance primitive dengan Tier A (ulangi grep governance-coupling dari Gate D-2 RF-D sebelum APPROVE).

---

## Gate G8: Vault Secret SSOT (T-8.1, T-8.2)

- [ ] **T-8.1:** `get_decrypted_secret` di `grpc_system_gateway.rs` tidak lagi `Status::unimplemented`; ada test simpan (`SetSecret`) → ambil (`GetSecret`) yang lolos; kunci yang tidak ada mengembalikan `NOT_FOUND`, bukan panic/500 generik.
- [ ] **Secret TIDAK PERNAH tercatat di log** — Principle grep log statement di sekitar handler ini secara eksplisit, termasuk log level debug/trace.
- [ ] **T-8.2:** `crew.StartTurn` (hasil Gate G5) mengambil API key via vault Rust, `grep "os.Getenv" engine/src/crew/` untuk credential LLM = kosong setelah PR ini.

**Reject jika:** vault handler mengembalikan secret di error message (contoh: menyertakan nilai secret di pesan `NOT_FOUND`/log error untuk debugging) — ini kebocoran credential, bukan sekadar bug kecil.

---

## Gate G9 (Checkpoint Akhir) — End-to-End Proof (T-9.1)

Sebelum task ini ditandai `[x]`, Principle WAJIB memverifikasi siklus penuh berjalan, bukan hanya membaca laporan:

> `Objective diterima Quartermaster` → `FleetOrderProposal awaiting approval` → `Pirate King approve` → `crew.StartTurn via Rust LLM gRPC` → `tools berjalan di Rust sandbox` → `Artifact disimpan ke DiskStore` → `Metrics riil tampil di Tauri & Web` → `Restart tidak kehilangan data`

- [ ] Setiap langkah di siklus di atas punya bukti konkret (log/screen-recording/test otomatis) — **tidak ada langkah yang "diasumsikan sudah jalan karena task individunya hijau"**. Integrasi lintas-Grup sering gagal meski unit per-Grup lolos.
- [ ] Tidak ada fallback palsu, konstanta hardcoded, atau mock yang lolos ke jalur ini (cross-check ulang Gate G0, G4, G5, G8 — semua harus APPROVED sebelum G9 dimulai).
- [ ] Restart proses (Go engine dan/atau Rust gateway) di tengah/sesudah siklus **tidak menghilangkan data** — diuji dengan benar-benar mematikan dan menyalakan ulang proses, bukan hanya membaca kode penyimpanan.
- [ ] `AUDIT.md` diperbarui dengan status akhir siklus end-to-end ini.

**Reject jika:** G9 diklaim selesai berdasarkan laporan tiap Grup tanpa demonstrasi siklus end-to-end yang benar-benar dijalankan ulang oleh Principle.

---

## Ringkasan Urutan Review PR

1. Jalankan **Bagian 0 (Gate Umum)** dulu untuk SEMUA PR tanpa kecuali.
2. Pilih Gate G0–G9 sesuai Grup task yang dicakup PR (lihat ID task: `T-0.x` → G0, `T-1.x` → G1, dst).
3. Untuk PR yang menyentuh channel/crate-split Rust di Grup G7, tambahkan Gate D-1 s/d D-4 dari `RF-D-principle-gateway-checklist.md` — jangan duplikasi isinya di sini, cukup rujuk.
4. **APPROVE hanya kalau SEMUA gate relevan (Bagian 0 + Gate Grup yang applicable) lulus.** Kalau ragu salah satu item, default ke REJECT dan minta klarifikasi/perbaikan, bukan "approve dengan catatan".
5. G9 (checkpoint akhir) hanya dibuka untuk review setelah G0–G8 yang relevan ber-status APPROVED — jangan mulai demonstrasi end-to-end di atas fondasi yang belum solid.

## Definition of Done (Finalize — keseluruhan)

Folder `finalize/` dinyatakan selesai ketika: seluruh task `task-breakdown.md` Grup 0–8 berstatus `[x]` dengan bukti verifikasi konkret per Gate di atas, **dan** checkpoint G9 (End-to-End Proof) berhasil didemonstrasikan Principle secara langsung tanpa fallback/mock yang tersembunyi di jalur utama.
