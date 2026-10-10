---
sprint: 4
title: Crate Decoupling & Runtime Strangler — gateway/runtime Decoupling, Go Agent-Loop Migration
status: in-progress (17/26 checklist [x]; 9 item masih [ ] -- sprint terbesar, berseri)
---

# Sprint 4 — Crate Decoupling & Runtime Strangler

> Sprint PALING BESAR (XL, berseri) — ini jantung migrasi arsitektur `clawcrew-runtime` (286k LOC) → Go. BL-18 (resource-gate sub-agent) diangkat ke sprint ini karena dependency-nya (BG-D1/BG-D4) akhirnya terjadwal di sprint yang sama.

## A. Dekoplasi Crate Rust (lanjutan BR-C1..C4 yang sudah selesai)

- [ ] **BR-C5 · R-14 · PR-61 (P1, L)** ⚑ `gateway` tidak bergantung ke `runtime` (426 ref → 0). Dep: BR-D1 (lihat §C — ini saling bergantung, koordinasikan urutan nyata saat eksekusi, dev-tasks mencatat BR-C5 depends BR-D1 TAPI BR-D1 juga di sprint ini).

#### Detail Eksekusi — BR-C5

**TIDAK DIVERIFIKASI PENUH sesi ini** (426 referensi `gateway`→`runtime` tidak di-grep ulang — klaim dari dokumen lama, verifikasi angka ini sebagai langkah pertama). **Langkah pertama WAJIB:** `grep -rn "clawcrew_runtime::" crates/clawcrew-gateway/src/ | wc -l` untuk konfirmasi angka 426 masih akurat atau sudah berubah (kode terus bergerak, seperti ditemukan di Sprint 0-2 — banyak nomor baris/jumlah referensi dokumen lama sudah bergeser).

**Langkah pengerjaan:**
1. Jalankan grep di atas untuk ground-truth jumlah referensi saat ini.
2. Kategorikan tiap referensi: (a) tipe data yang perlu dipindah ke crate bersama (`clawcrew-api` kemungkinan), (b) fungsi yang perlu di-reimplementasi di `gateway` sendiri, (c) referensi yang genuinely perlu `runtime` tetap ada (kalau ada, catat sebagai pengecualian yang perlu didiskusikan).
3. Koordinasikan dengan BR-D1 (§D, ramping runtime per modul) — urutan NYATA (bukan urutan tiket di dokumen) adalah BG-D1 (Go agent loop) harus cukup matang dulu sebelum modul Rust tertentu bisa "dipensiunkan" dari sisi `gateway`.

**Dependency:** BR-D1 (sirkular, lihat §Catatan Sirkularitas di bawah).

**ASUMSI:** Scope detail per-referensi tidak bisa saya rinci tanpa membaca 426 titik referensi (di luar budget sesi ini) — tim eksekusi WAJIB melakukan kategorisasi langkah 2 sebagai pekerjaan AWAL sebelum estimasi effort final.

- [ ] **BR-C6 · R-15 · PR-63 (P2, XL, berseri)** Pecah `channels` (198k LOC) → `galleon-channel-<nama>` (sudah: core, imessage, signal, bluesky). Dep: BR-C4 (selesai).

#### Detail Eksekusi — BR-C6

**Verifikasi parsial:** `galleon-channel-core`, `galleon-channel-imessage`, `galleon-channel-signal`, `galleon-channel-bluesky` disebut "sudah" di dokumen — konsisten dengan lesson project ini (RF-B1 pattern sudah established untuk `BlueskyChannel`). Channel YANG BELUM dipecah (tersisa di `clawcrew-channels` monolith) perlu diinventarisasi.

**Langkah pengerjaan:**
1. `read` direktori `crates/` untuk daftar `galleon-channel-*` yang SUDAH ada vs channel yang masih di `clawcrew-channels/src/` (WhatsApp, Telegram, Discord, dll — belum diverifikasi mana yang sudah/belum).
2. Untuk SETIAP channel yang belum dipecah, ikuti pola RF-B1 yang SUDAH established (lesson project): (a) verifikasi satu-satunya consumer eksternal tetap resolve via re-export, (b) verifikasi public surface baru identik (struct/constructor/impl Channel), (c) wire feature `dep:galleon-channel-x`, (d) HANYA setelah itu hapus kode monolith lama.
3. **PRIORITASKAN channel yang "Tier A" (produksi aktif)** dulu sebelum Tier B/C (regional/WASI, Sprint 5 BR-F3/F4/F5) — menghindari pecah channel yang jarang dipakai duluan sementara channel utama masih monolith besar.

**JANGAN DIUBAH:** Policy allowlist bersama — tetap di `galleon-channel-core`, JANGAN direimplementasi per-channel (lesson project: "shared policy lives in galleon-channel-core re-exporting clawcrew_config::schema SSOT, never reimplemented per-channel").

**Dependency:** BR-C4 (selesai, prasyarat terpenuhi).

- [ ] **BR-C7 · R-17 · PR-60 (P2, M)** Pecah file terbesar yang tetap di Rust.

#### Detail Eksekusi — BR-C7

**File & simbol spesifik (terverifikasi dari Sprint 1 BR-A5):** `crates/clawcrew-gateway/src/api.rs` (374 referensi `unwrap/expect` disebut Sprint 1 — indikasi file BESAR), `api_config.rs` (164 ref), `ws.rs` (122 ref), `api_sections.rs` (104 ref) — kandidat kuat untuk "file terbesar yang tetap di Rust" setelah BR-C5 (`gateway` decoupling).

**Langkah pengerjaan:**
1. Setelah BR-C5 selesai (gateway lepas dari runtime), ukur ulang file terbesar yang TERSISA (`wc -l crates/*/src/*.rs | sort -rn | head -20`).
2. Pecah file >2000 baris (threshold umum, konfirmasi dengan Lead kalau beda) menjadi submodule — pola SAFE RF-A (file-split WITHIN satu crate, `pub(crate) use` menjaga path, zero dependency change) yang sudah established di lesson project ini.

**Dependency:** BR-C5 (file ukuran berubah signifikan setelah decoupling, ukur SETELAH itu bukan sebelum).

- [x] **BR-C8 · R-18 · PR-97 (P3, M)** Audit `tools`/`config`/`providers` — tanpa pemecahan spekulatif.

#### Detail Eksekusi — BR-C8

**Langkah pengerjaan:** AUDIT murni (bukan kode) — baca `crates/clawcrew-tools/` (atau lokasi tools, verifikasi nama crate persis), `crates/clawcrew-config/`, `crates/clawcrew-providers/` untuk ukuran/kompleksitas, tulis temuan apakah PERLU dipecah atau TIDAK — **"tanpa pemecahan spekulatif"** berarti JANGAN pecah crate ini kalau tidak ada bukti konkret ukuran/kompleksitas bermasalah (ponytail: YAGNI, jangan refactor tanpa kebutuhan nyata).

**JANGAN DIUBAH:** Jangan pecah ketiga crate ini sampai audit menunjukkan kebutuhan nyata — ini P3 (prioritas rendah) justru karena sifatnya audit-dulu.

## B. Protokol Klien (prasyarat BG-D-series butuh ini untuk streaming)

- [x] **BG-E1 · G-36 · PR-51 (P1, M)** Spesifikasi Galleon Client Protocol v1.

#### Detail Eksekusi — BG-E1

**Langkah pengerjaan:** Tulis spesifikasi (dokumen, bukan kode) di `api-contracts/client-protocol.md` (path sesuai tabel File Ownership dokumen ini) — WS JSON-RPC 2.0 method untuk `chat`/`run`/`approve`/`status` (sesuai kebutuhan BR-E2 Sprint 6 yang akan dipetakan ke protokol ini). Definisikan: method names, params shape, response shape, error codes, event/notification shape untuk streaming (tool-call events, turn progress).

**Dependency:** Prasyarat BG-E2 (implementasi server), BR-E2/E3 (Sprint 6, consumer CLI/TUI).

**ASUMSI:** JSON-RPC 2.0 dipilih dokumen (bukan keputusan saya) — spesifikasi harus comply JSON-RPC 2.0 spec standar (method/params/id/result/error shape baku), bukan varian custom.

- [x] **BG-E2 · G-36/G-27 · PR-52 (P1, L)** Server Client Protocol (WS JSON-RPC 2.0). Dep: BG-E1, BG-C1 (Sprint 0), BG-D1 (§C sprint ini — sirkular, koordinasikan: BG-E2 butuh BG-D1 ADA, bukan selesai total; verifikasi urutan nyata saat eksekusi).

#### Detail Eksekusi — BG-E2

**Langkah pengerjaan:**
1. Implementasikan WS server Go sesuai spec BG-E1.
2. Pasang middleware `identity.RequireAuthenticated` (BG-C1, Sprint 0) di titik WS upgrade — koneksi WS harus authenticated sebelum menerima method call apa pun (fail-closed, konsisten pola `handleTerminalExec` Sprint 0).
3. Method `chat`/`run` meneruskan ke BG-D1 agent loop (begitu BG-D1 cukup matang — lihat §Catatan Sirkularitas).

**Dependency:** BG-E1 (hard), BG-C1 (hard, Sprint 0), BG-D1 (sirkular — mulai duluan tapi tidak perlu selesai total, lihat catatan sirkularitas dokumen ini).

**JANGAN DIUBAH:** `identity.RequireAuthenticated` middleware — reuse langsung dari BG-C1, jangan buat varian auth terpisah untuk WS.

## D. Ramping Runtime (hanya setelah C punya parity)

- [ ] **BR-D1 · R-16 · PR-60 (P2, XL, berseri)** ⚑ Ramping runtime per modul (urutan R-00 §3). Syarat tiap modul: parity Go + eval hijau.

#### Detail Eksekusi — BR-D1

**TIDAK DIVERIFIKASI sesi ini** — urutan modul "R-00 §3" (dirujuk dokumen) belum dibaca. **Langkah pertama WAJIB:** baca `standard-docs/roadmap/R-rust.md` §R-00 bagian 3 untuk urutan modul persis yang harus di-ramping (strangler pattern — matikan modul Rust satu per satu setelah Go punya parity).

**Langkah pengerjaan (per modul, BERSERI — jangan paralel 2 modul yang saling import):**
1. Pilih modul berikutnya sesuai urutan R-00 §3.
2. Verifikasi Go agent loop (BG-D1 dkk) punya PARITY fungsional (test `clawcrew-eval` replay hijau untuk skenario yang mencakup modul ini).
3. Matikan modul Rust (feature flag off, atau hapus kalau sudah confirmed tidak ada regresi).
4. Ulangi untuk modul berikutnya.

**Dependency:** BG-D1-D5 (§C, parity harus established lebih dulu per modul).

- [ ] **BR-D2 · R-37 · PR-56 (P2, M)** Retarget `galleon-eval` ke jalur Go; pertahankan fixture trace. Dep: BG-D1.

#### Detail Eksekusi — BR-D2

**Koreksi nama kemungkinan:** `galleon-eval` disebut Sprint 3 Eval-01 sebagai `clawcrew-eval` (crate harness Replay/Live) — KEMUNGKINAN nama sama, belum di-rename (`galleon-eval` adalah nama PASCA-rename Sprint 8). Verifikasi nama crate aktual (`crates/clawcrew-eval/` kemungkinan) sebelum eksekusi — JANGAN cari direktori `galleon-eval` yang mungkin belum ada.

**Langkah pengerjaan:** Setelah BG-D1 jalan, retarget harness eval (`clawcrew-eval`) untuk replay fixture terhadap jalur Go (bukan lagi langsung ke `clawcrew-runtime` Rust) — PERTAHANKAN fixture trace yang sudah ada (data test tidak diubah, hanya target eksekusi yang berubah).

**Dependency:** BG-D1 (hard).

- [x] **BR-D3 · R-38 · PR-59 (P2, S)** ⚑ Hapus `verifiable_intent` (tag `archive/pre-galleon` dulu; ODR-21). (SELESAI: tag `archive/pre-galleon` dibuat pada commit `95d346eb0`, seluruh referensi fungsional dihapus bersih di commit `0e197e110`, merged ke `testing` commit `3af7290f9` via PR #20.)

#### Detail Eksekusi — BR-D3

**TIDAK DIVERIFIKASI sesi ini** — lokasi modul `verifiable_intent` belum dicari (disebut di `archive/research-closed-loop/verifiable-intent-assessment.md` menurut Sprint 7 §B BG-F6, kemungkinan terkait). **Langkah pertama WAJIB:** `grep -rn "verifiable_intent" crates/` untuk lokasi persis, dan baca ODR-21 (`standard-docs/decisions/owner-decisions/`) untuk konteks keputusan sebelum menghapus apa pun.

**Langkah pengerjaan:** Git tag `archive/pre-galleon` pada commit SEBELUM penghapusan (preservasi history, bukan hard-delete tanpa jejak) — pola yang sama dengan praktik archival lain di project ini — BARU hapus modul.

**JANGAN DIUBAH:** Jangan hapus tanpa tag archive dulu — ini destructive action yang perlu jejak rollback.

- [ ] **BR-D4 · R-41 · PR-62 (P2, L)** ⚑ Hapus `rpc` + REST/WS/A2A lama setelah parity. Dep: BR-E3 (Sprint 6), BR-C5 (§A sprint ini).

#### Detail Eksekusi — BR-D4

**Verifikasi parsial:** `crates/clawcrew-runtime/src/rpc/dispatch/mod.rs` ditemukan saat grep Sprint 5 (1 ref `loop_detector`-adjacent) — konfirmasi modul `rpc` ADA di lokasi ini.

**Langkah pengerjaan:** Setelah BR-E3 (Sprint 6, CLI/TUI retargeted ke Client Protocol) DAN BR-C5 (gateway lepas runtime) selesai — hapus `crates/clawcrew-runtime/src/rpc/` dan REST/WS/A2A lama yang digantikan Client Protocol (BG-E2). Tag archive dulu (pola sama BR-D3) sebelum hapus.

**Dependency:** BR-E3 (Sprint 6, hard), BR-C5 (§A, hard) — JANGAN mulai sebelum KEDUA selesai, menghapus jalur lama sebelum penggantinya terbukti jalan = outage risk.

## E. Observabilitas & Organisasi (dep ke persistensi Sprint 1)

> **BL-05 BARU DITAMBAHKAN (CRITICAL, dari `skills-steering-lessons-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05 ini)** — celah keamanan privilege-escalation AKTIF: `engine/src/crew/dispatcher.go` masih mendaftarkan DUA jalur delegasi sub-agent paralel ke LLM — `spawn_subagent` (`dispatcher.go:216`, TANPA ceiling-check, langsung `gateway.ExecuteTurn`) vs `DelegateTaskTool`/`delegate_task` (`delegation.go:116-180`, PUNYA ceiling-check teruji `TestDelegationPolicyCeiling`, tapi ternyata TIDAK benar-benar eksekusi — cuma validasi+return JSON). LLM bebas pilih jalur mana, dan `spawn_subagent` melewati privilege-ceiling sepenuhnya. **Scope BL-05 (satukan jadi satu jalur, hapus/alias `spawn_subagent`) HARUS selesai SEBELUM atau BERSAMAAN dengan BG-D1** — kalau BG-D1 dibangun di atas `spawn_subagent` yang belum diperbaiki, resource-gate baru (eks-BL-18 di atas) akan menjaga pintu yang salah.

- [x] **BL-05 (CRITICAL, PRIORITASKAN di §C ini — sebelum BG-D1 mulai menyentuh `dispatcher.go`)** Satukan dua jalur delegasi sub-agent: hapus entri `spawn_subagent` dari `standardTools` di `dispatcher.go`, ATAU alias ke `DelegateTaskTool` logic. Klarifikasi juga: apakah `DelegateTaskTool` yang "tidak eksekusi apa-apa" itu disengaja (lapis approval sebelum eksekusi nyata di tempat lain) atau fitur belum selesai — perlu jawaban sebelum memutuskan bentuk penyatuan. File: `engine/src/crew/dispatcher.go` (satu file utama, isolated change). **AC tambahan (2026-10-06, Owner):** tambah depth-limit (bukan hanya capability-ceiling) — `ExecutionContext.DelegationDepth`/`MaxDelegationDepth` baru (`tool/dto.go`), ditegakkan di `delegation.go` `ExecuteWithContext`, nilai dari `entitlement.QuotaPolicy.MaxDelegationDepth()` (ENT-01 di §E bawah, Community=1/Team=2/Dockyard=3). Lihat `standard-docs/backlog/skills-steering-lessons-backlog.md` BL-05 untuk AC lengkap.
- [x] **BG-D1 · G-30 · PR-47 (P1, L)** Agent loop/turn: tool-call loop, kompaksi, batas iterasi di atas `ExecuteTurn` Rust. *Acceptance:* paritas dengan `eval` replay. **Acceptance criteria tambahan (eks-BL-18, DITUNDA → DIANGKAT sprint ini):** jalur masuk `ExecuteTurn` dari Go `spawn_subagent` HARUS punya resource-gate (cap jumlah/memori sub-agent) sebelum BG-D1 dianggap selesai — jangan ulangi gap yang sama yang sudah ditutup di sisi Rust (BL-15, Sprint 5). **Titik pemasangan konkret (trace 2026-10-06):** `crew/services.go:187-189` (goroutine spawn sebelum `s.runSubagent`) — gate harus cek SEBELUM `subagentWg.Add(1)`, bukan di dalam `runSubagent` sendiri (`services.go:235-264`), supaya penolakan tidak memakan slot goroutine. **Catatan scope:** `runSubagent` saat ini HANYA menangani `TurnToken_Token` (teks), tidak ada `case *pb.TurnToken_ToolCall` — sub-agent belum punya tool-access, jadi resource-gate di sini murni membatasi *jumlah* sub-agent paralel, BUKAN kedalaman rantai (depth-limit itu scope BL-05, berlaku saat tool-access ditambahkan nanti). **Dep tambahan: BL-05 di atas** — kalau `spawn_subagent` dihapus/dialias BL-05, resource-gate ini dipasang di jalur pengganti (`DelegateTaskTool`→`runSubagent` yang sudah tersambung), bukan jalur lama.
- [x] **BG-D2 · G-31 · PR-48 (P1, L)** SOP engine + approval gate. Dep: BG-D1.
- [x] **BG-D3 · G-32 · PR-49 (P2, M)** Skills & scheduler (cron) terikat Squad/Crew. Dep: BG-D1.
- [x] **BG-D4 · G-33 · PR-49 (P2, M)** Sub-agent / handoff / A2A sebagai Artifact reference. Dep: BG-D1. **Acceptance criteria tambahan (eks-BL-18, bagian 2):** resource-gate dari BG-D1 di atas HARUS diwariskan ke jalur handoff/A2A ini — titik masuk sub-agent kedua yang perlu gate sama.
- [x] **BG-D5 · G-34 · PR-50 (P2, M)** Padanan `control_plane` & `approval`; cek duplikasi dengan `fleet/policies.go` (policy tetap SSOT di sana).

## D. Ramping Runtime (hanya setelah C punya parity)

- [ ] **BR-D1 · R-16 · PR-60 (P2, XL, berseri)** ⚑ Ramping runtime per modul (urutan R-00 §3). Syarat tiap modul: parity Go + eval hijau.
- [ ] **BR-D2 · R-37 · PR-56 (P2, M)** Retarget `galleon-eval` ke jalur Go; pertahankan fixture trace. Dep: BG-D1.
- [x] **BR-D3 · R-38 · PR-59 (P2, S)** ⚑ Hapus `verifiable_intent` (tag `archive/pre-galleon` dulu; ODR-21). (SELESAI: tag `archive/pre-galleon` dibuat pada commit `95d346eb0`, seluruh referensi fungsional dihapus bersih di commit `0e197e110`, merged ke `testing` commit `3af7290f9` via PR #20.)
- [ ] **BR-D4 · R-41 · PR-62 (P2, L)** ⚑ Hapus `rpc` + REST/WS/A2A lama setelah parity. Dep: BR-E3 (Sprint 6), BR-C5 (§A sprint ini).

## E. Observabilitas & Organisasi (dep ke persistensi Sprint 1)

> **BL-80/81/82 BARU DITAMBAHKAN (tier/pricing & entitlement, dari `product-research/tier-pricing-and-entitlement-final.md`, keputusan Owner 2026-10-06)** — tangga tier final (Community/Team/Dockyard/Private Dockyard: Ships 1/3/6/custom, Squads total 2/6/12/custom, Crew per Squad 6/6/8/custom, Crew per Ship 8/22/100/custom, Concurrent Voyages 2/4/8/custom) perlu berbadan kode di `entitlement/` (saat ini hanya `// TODO`, nol interface). **BL-81 BUKAN tiket terpisah yang menunggu BG-F1 selesai — ia adalah bagian scope BG-F1 itu sendiri** ("model organisasi persisten dari nol"), ditambahkan sebagai AC tambahan di baris BG-F1 di bawah. Lihat `standard-docs/backlog/tier-entitlement-pricing-backlog.md` untuk AC lengkap per tiket.

- [x] **BG-F1 · G-23 · PR-53 (P1, L)** Model organisasi persisten Quartermaster→Ship→Squad→Crew. Dep: BG-B2 (selesai). **Acceptance criteria tambahan (BL-81, prasyarat guard kuota bermakna — 5 bug diverifikasi 2026-10-06):** (1) ID Ship unik, bukan `"shp_"+fleetID` (`ship/services.go:31`, kolisi Ship ke-2 pada Fleet sama); (2) satu sumber hitung Ship/Squad ditetapkan (`countCollection("ships")`/`countCollection("squads")` interim — JANGAN `Fleet.ActiveShips`, yang selalu `[]string{}` dan tidak pernah di-append); (3) `squad.Squad` struct baru dari nol (`squad/interfaces.go` saat ini kosong total — `package squad` + 1 baris TODO); (4) `ship.Ship` tambah field `CrewIDs []string` untuk Crew-langsung-di-Ship (non-Squad, role Supervisor/Security — lihat riset §1.2); (5) guard kuota dari `entitlement.QuotaPolicy` (BL-80 di bawah) dipasang DI `handleCollections` (`fleet/delivery.go`) untuk koleksi `ships`/`squads`/`crew` — BUKAN hanya di `ship.Service`/`squad.Service` yang jalur nyatanya belum tersambung (`ship.Service.CreateShip` nol pemanggil, jalur tulis nyata = `POST /api/collections/{name}` → `SaveCollection` menerima JSON bebas, melewati guard service manapun); (6) cek+insert atomik di bawah lock yang sama (`fleetService.mu`); (7) entitas di atas plafon (lapse/downgrade/impor) tetap bisa dibaca & diekspor, hanya *create* baru ditolak (grandfathering).
- [x] **ENT-01 (BL-80, P2, S)** `QuotaPolicy` interface + `CommunityPolicy`/`TeamPolicy`/`DockyardPolicy` di `engine/src/entitlement/` (saat ini `interfaces.go` hanya `package entitlement` + TODO). Method: `MaxFleets()`, `MaxShips()`, `MaxSquadsTotal()`, `MaxCrewsPerSquad()`, `MaxCrewsPerShip()`, `MaxConcurrentVoyages()`, **`MaxDelegationDepth()`** (`*int`, nil = unlimited/custom). Nilai final: Community `{1,1,2,6,8,2,1}`, Team `{1,3,6,6,22,4,2}`, Dockyard `{1,6,12,8,100,8,3}`. Endpoint `GET /api/v1/entitlement/usage`. **Dep: BL-81 di atas** untuk AC "guard menolak create ke-N+1" — tanpa itu guard dekoratif. **Konsumen `MaxDelegationDepth()`: BL-05 (§C) — penegakan depth-limit rantai delegasi sub-agent**, ditambahkan 2026-10-06 setelah analisa pola KiroCrew (platform agen sesi ini membatasi rantai Crew→Crew via `max_delegation_depth` statis). **Jangan taruh logika verifikasi lisensi Ed25519 di package publik ini** — `TeamPolicy`/`DockyardPolicy` hanya nilai fallback/default-saat-aktif; verifier Ed25519 Team/Dockyard adalah modul Licensing privat terpisah (repo lain, di luar scope sprint ini — dicatat ongoing).
- [x] **GO-MOD-RENAME (BL-82, P3, S)** `engine/go.mod` module path `github.com/diezy-labs/claw-crew/engine` → `github.com/diezy-realm/galleon-fleet/engine` (remote git repo sudah pindah, module path belum ikut). **Kerjakan sebelum modul Licensing privat (Q5 Owner) mulai mengimpor package `entitlement`** — menunda ke Sprint 8 (RB-02) berarti modul privat itu harus refactor import path ulang begitu rebrand jalan. Isolated, mekanis (ponytail: byte-level find-replace, verifikasi dengan `go build ./...` bukan grep manual).

#### Detail Eksekusi — GO-MOD-RENAME

**Verifikasi 2026-10-06:** `engine/go.mod:1` — persis `module github.com/diezy-labs/claw-crew/engine` (terkonfirmasi akurat, belum berubah sejak dicatat).

**Langkah pengerjaan:**
1. Ubah baris 1 `engine/go.mod` ke `module github.com/diezy-realm/galleon-fleet/engine`.
2. Find-replace SEMUA import path `github.com/diezy-labs/claw-crew/engine/...` → `github.com/diezy-realm/galleon-fleet/engine/...` di SELURUH file `.go` di `engine/` (termasuk `engine/pkg/`, `engine/core/`).
3. `go build ./...` dari root `engine/` — ini verifikasi WAJIB (bukan grep manual) karena compiler akan menangkap import yang terlewat.
4. `go vet ./...` dan jalankan test suite existing untuk konfirmasi nol regresi.

**Acceptance criteria testable:** `go build ./...` dan `go test ./...` keduanya hijau setelah rename, nol sisa referensi `diezy-labs/claw-crew` di file `.go` manapun (`grep -rn "diezy-labs/claw-crew" engine/ --include=*.go` = 0 match).

**JANGAN DIUBAH:** Logic kode apa pun — ini MURNI rename path, perilaku program harus identik sebelum/sesudah.

- [x] **BG-F2 · G-25 · PR-53 (P2, M)** Kontrak rute QM two-layer + test.
- [x] **BG-F3 · G-26 · PR-54 (P2, M)** Audit trail & policy SSOT read-only ke UI/Tauri.
- [x] **BG-F5 · G-28 · PR-96 (P3, M)** Observabilitas: zap + request-id Go→Rust; scrubbing. (SELESAI: tracing.StartSpan tersambung di HTTP handler delivery.go, trace_id konsisten lintas log slog HTTP, gRPC client metadata x-request-id, dan grpc_system_gateway.rs di Rust. Uji Go TestRunHTTPDelivery_TracingAndRequestIDSynchronization dan Rust go_callable_grpc_surface_propagates_request_id_tracing_metadata lulus 100%, merged ke testing commit 3af7290f9 via PR #20.)

#### Detail Eksekusi — BG-F5

**Scope final (bukan "adopsi zap" seperti klaim asli):** sambungkan `tracing.StartSpan`/`End` (sudah ada, `engine/src/core/tracing/tracing.go`) ke jalur HTTP request di `delivery.go`, supaya `trace_id` yang sama muncul di: (a) log `slog` HTTP, (b) log gRPC interceptor ke Rust, (c) `x-request-id` yang sudah dikirim ke Rust. Saat ini request-id dan tracing span adalah dua potongan yang tidak tersambung.

**Langkah pengerjaan:**
1. Baca `core/tracing/tracing.go` (`StartSpan`/`End`, field `TraceID`/`SpanID`/`ParentSpanID`) dan `run/delivery.go` untuk titik masuk HTTP request.
2. Panggil `tracing.StartSpan` di awal handler `delivery.go`, gunakan `TraceID` yang dihasilkan sebagai (atau disinkronkan dengan) `x-request-id` yang sudah dipropagasi ke gRPC metadata (`system_gateway.go:17-20`).
3. Pastikan `slog` logger di jalur ini menyertakan `trace_id` sebagai field terstruktur (bukan hanya di tracing span, juga di log line).
4. Scrubbing kredensial Go-side SUDAH ADA (`RedactAttr`/`RedactString` di `core/logger/logger.go`) — tidak perlu kerja baru, hanya verifikasi redaksi ini jalan juga untuk field `trace_id` kalau kebetulan membawa data sensitif (seharusnya tidak, tapi cek).

**DoD:** satu request HTTP end-to-end menghasilkan `trace_id` yang sama di log `slog` HTTP, log gRPC interceptor, dan metadata yang diterima Rust — dibuktikan dengan 1 test/log sample, bukan hanya baca kode.

**Dependency:** Tidak hard-block tiket lain. Prinsip konsistensi redaksi dengan BL-38 (Sprint 2, Rust `scrub_credentials`) sudah terpenuhi secara independen di masing-masing sisi — tidak perlu kerja tambahan untuk itu.

## Catatan Sirkularitas (dari dev-tasks asli, dipertahankan apa adanya)

Dev-tasks asli mencatat `BR-C5 dep BG-D1`(cron) dan `BG-E2 dep BG-D1`. Karena BR-C5/BG-E2/BG-D1 semua di sprint yang sama, urutan EKSEKUSI nyata (bukan urutan tiket) harus: **BG-D1 mulai dulu** (tidak perlu selesai total) → BR-C5 dan BG-E2 bisa mulai begitu BG-D1 cukup matang untuk dependency-nya terpenuhi. Lead memverifikasi titik "cukup matang" ini saat sprint berjalan, bukan di dokumen ini.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `crates/clawcrew-{gateway,channels}/`, `crates/galleon-channel-*/` | Backend-Rust (berseri per-crate) |
| B | `api-contracts/client-protocol.md`, Go WS server baru | Backend-Go |
| C | `engine/src/` agent-loop module baru | Backend-Go |
| D | `crates/clawcrew-runtime/src/` (per-modul, R-00 §3 urutan) | Backend-Rust (berseri, JANGAN paralel 2 modul yang saling import) |
| E | `engine/src/organization/` (baru), `engine/src/fleet/` | Backend-Go |

## Keluaran Sprint

- `gateway` dan `channels` lepas total dari `clawcrew-runtime`.
- Agent-loop berjalan di Go dengan parity Rust (`eval` replay hijau) DAN resource-gate sub-agent aktif sejak awal (bukan ditambal belakangan).
- Model organisasi Quartermaster→Ship→Squad→Crew persisten di Go.
