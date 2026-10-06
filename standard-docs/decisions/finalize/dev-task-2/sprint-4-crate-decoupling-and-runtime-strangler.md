---
sprint: 4
title: Crate Decoupling & Runtime Strangler — gateway/runtime Decoupling, Go Agent-Loop Migration
status: planned
depends_on: Sprint 0-1 (BG-C1 untuk auth; persistensi Go untuk BG-D-series)
---

# Sprint 4 — Crate Decoupling & Runtime Strangler

> Sprint PALING BESAR (XL, berseri) — ini jantung migrasi arsitektur `clawcrew-runtime` (286k LOC) → Go. BL-18 (resource-gate sub-agent) diangkat ke sprint ini karena dependency-nya (BG-D1/BG-D4) akhirnya terjadwal di sprint yang sama.

## A. Dekoplasi Crate Rust (lanjutan BR-C1..C4 yang sudah selesai)

- [ ] **BR-C5 · R-14 · PR-61 (P1, L)** ⚑ `gateway` tidak bergantung ke `runtime` (426 ref → 0). Dep: BR-D1 (lihat §C — ini saling bergantung, koordinasikan urutan nyata saat eksekusi, dev-tasks mencatat BR-C5 depends BR-D1 TAPI BR-D1 juga di sprint ini).
- [ ] **BR-C6 · R-15 · PR-63 (P2, XL, berseri)** Pecah `channels` (198k LOC) → `galleon-channel-<nama>` (sudah: core, imessage, signal, bluesky). Dep: BR-C4 (selesai).
- [ ] **BR-C7 · R-17 · PR-60 (P2, M)** Pecah file terbesar yang tetap di Rust.
- [ ] **BR-C8 · R-18 · PR-97 (P3, M)** Audit `tools`/`config`/`providers` — tanpa pemecahan spekulatif.

## B. Protokol Klien (prasyarat BG-D-series butuh ini untuk streaming)

- [ ] **BG-E1 · G-36 · PR-51 (P1, M)** Spesifikasi Galleon Client Protocol v1.
- [ ] **BG-E2 · G-36/G-27 · PR-52 (P1, L)** Server Client Protocol (WS JSON-RPC 2.0). Dep: BG-E1, BG-C1 (Sprint 0), BG-D1 (§C sprint ini — sirkular, koordinasikan: BG-E2 butuh BG-D1 ADA, bukan selesai total; verifikasi urutan nyata saat eksekusi).

## C. Otak Agent di Go — Strangler dari `clawcrew-runtime`

> **BL-05 BARU DITAMBAHKAN (CRITICAL, dari `skills-steering-lessons-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05 ini)** — celah keamanan privilege-escalation AKTIF: `engine/src/crew/dispatcher.go` masih mendaftarkan DUA jalur delegasi sub-agent paralel ke LLM — `spawn_subagent` (`dispatcher.go:216`, TANPA ceiling-check, langsung `gateway.ExecuteTurn`) vs `DelegateTaskTool`/`delegate_task` (`delegation.go:116-180`, PUNYA ceiling-check teruji `TestDelegationPolicyCeiling`, tapi ternyata TIDAK benar-benar eksekusi — cuma validasi+return JSON). LLM bebas pilih jalur mana, dan `spawn_subagent` melewati privilege-ceiling sepenuhnya. **Scope BL-05 (satukan jadi satu jalur, hapus/alias `spawn_subagent`) HARUS selesai SEBELUM atau BERSAMAAN dengan BG-D1** — kalau BG-D1 dibangun di atas `spawn_subagent` yang belum diperbaiki, resource-gate baru (eks-BL-18 di atas) akan menjaga pintu yang salah.

- [ ] **BL-05 (CRITICAL, PRIORITASKAN di §C ini — sebelum BG-D1 mulai menyentuh `dispatcher.go`)** Satukan dua jalur delegasi sub-agent: hapus entri `spawn_subagent` dari `standardTools` di `dispatcher.go`, ATAU alias ke `DelegateTaskTool` logic. Klarifikasi juga: apakah `DelegateTaskTool` yang "tidak eksekusi apa-apa" itu disengaja (lapis approval sebelum eksekusi nyata di tempat lain) atau fitur belum selesai — perlu jawaban sebelum memutuskan bentuk penyatuan. File: `engine/src/crew/dispatcher.go` (satu file utama, isolated change). **AC tambahan (2026-10-06, Owner):** tambah depth-limit (bukan hanya capability-ceiling) — `ExecutionContext.DelegationDepth`/`MaxDelegationDepth` baru (`tool/dto.go`), ditegakkan di `delegation.go` `ExecuteWithContext`, nilai dari `entitlement.QuotaPolicy.MaxDelegationDepth()` (ENT-01 di §E bawah, Community=1/Team=2/Dockyard=3). Lihat `standard-docs/backlog/skills-steering-lessons-backlog.md` BL-05 untuk AC lengkap.
- [ ] **BG-D1 · G-30 · PR-47 (P1, L)** Agent loop/turn: tool-call loop, kompaksi, batas iterasi di atas `ExecuteTurn` Rust. *Acceptance:* paritas dengan `eval` replay. **Acceptance criteria tambahan (eks-BL-18, DITUNDA → DIANGKAT sprint ini):** jalur masuk `ExecuteTurn` dari Go `spawn_subagent` HARUS punya resource-gate (cap jumlah/memori sub-agent) sebelum BG-D1 dianggap selesai — jangan ulangi gap yang sama yang sudah ditutup di sisi Rust (BL-15, Sprint 5). **Titik pemasangan konkret (trace 2026-10-06):** `crew/services.go:187-189` (goroutine spawn sebelum `s.runSubagent`) — gate harus cek SEBELUM `subagentWg.Add(1)`, bukan di dalam `runSubagent` sendiri (`services.go:235-264`), supaya penolakan tidak memakan slot goroutine. **Catatan scope:** `runSubagent` saat ini HANYA menangani `TurnToken_Token` (teks), tidak ada `case *pb.TurnToken_ToolCall` — sub-agent belum punya tool-access, jadi resource-gate di sini murni membatasi *jumlah* sub-agent paralel, BUKAN kedalaman rantai (depth-limit itu scope BL-05, berlaku saat tool-access ditambahkan nanti). **Dep tambahan: BL-05 di atas** — kalau `spawn_subagent` dihapus/dialias BL-05, resource-gate ini dipasang di jalur pengganti (`DelegateTaskTool`→`runSubagent` yang sudah tersambung), bukan jalur lama.
- [ ] **BG-D2 · G-31 · PR-48 (P1, L)** SOP engine + approval gate. Dep: BG-D1.
- [ ] **BG-D3 · G-32 · PR-49 (P2, M)** Skills & scheduler (cron) terikat Squad/Crew. Dep: BG-D1.
- [ ] **BG-D4 · G-33 · PR-49 (P2, M)** Sub-agent / handoff / A2A sebagai Artifact reference. Dep: BG-D1. **Acceptance criteria tambahan (eks-BL-18, bagian 2):** resource-gate dari BG-D1 di atas HARUS diwariskan ke jalur handoff/A2A ini — titik masuk sub-agent kedua yang perlu gate sama.
- [ ] **BG-D5 · G-34 · PR-50 (P2, M)** Padanan `control_plane` & `approval`; cek duplikasi dengan `fleet/policies.go` (policy tetap SSOT di sana).

## D. Ramping Runtime (hanya setelah C punya parity)

- [ ] **BR-D1 · R-16 · PR-60 (P2, XL, berseri)** ⚑ Ramping runtime per modul (urutan R-00 §3). Syarat tiap modul: parity Go + eval hijau.
- [ ] **BR-D2 · R-37 · PR-56 (P2, M)** Retarget `galleon-eval` ke jalur Go; pertahankan fixture trace. Dep: BG-D1.
- [ ] **BR-D3 · R-38 · PR-59 (P2, S)** ⚑ Hapus `verifiable_intent` (tag `archive/pre-galleon` dulu; ODR-21).
- [ ] **BR-D4 · R-41 · PR-62 (P2, L)** ⚑ Hapus `rpc` + REST/WS/A2A lama setelah parity. Dep: BR-E3 (Sprint 6), BR-C5 (§A sprint ini).

## E. Observabilitas & Organisasi (dep ke persistensi Sprint 1)

> **BL-80/81/82 BARU DITAMBAHKAN (tier/pricing & entitlement, dari `product-research/tier-pricing-and-entitlement-final.md`, keputusan Owner 2026-10-06)** — tangga tier final (Community/Team/Dockyard/Private Dockyard: Ships 1/3/6/custom, Squads total 2/6/12/custom, Crew per Squad 6/6/8/custom, Crew per Ship 8/22/100/custom, Concurrent Voyages 2/4/8/custom) perlu berbadan kode di `entitlement/` (saat ini hanya `// TODO`, nol interface). **BL-81 BUKAN tiket terpisah yang menunggu BG-F1 selesai — ia adalah bagian scope BG-F1 itu sendiri** ("model organisasi persisten dari nol"), ditambahkan sebagai AC tambahan di baris BG-F1 di bawah. Lihat `standard-docs/backlog/tier-entitlement-pricing-backlog.md` untuk AC lengkap per tiket.

- [ ] **BG-F1 · G-23 · PR-53 (P1, L)** Model organisasi persisten Quartermaster→Ship→Squad→Crew. Dep: BG-B2 (selesai). **Acceptance criteria tambahan (BL-81, prasyarat guard kuota bermakna — 5 bug diverifikasi 2026-10-06):** (1) ID Ship unik, bukan `"shp_"+fleetID` (`ship/services.go:31`, kolisi Ship ke-2 pada Fleet sama); (2) satu sumber hitung Ship/Squad ditetapkan (`countCollection("ships")`/`countCollection("squads")` interim — JANGAN `Fleet.ActiveShips`, yang selalu `[]string{}` dan tidak pernah di-append); (3) `squad.Squad` struct baru dari nol (`squad/interfaces.go` saat ini kosong total — `package squad` + 1 baris TODO); (4) `ship.Ship` tambah field `CrewIDs []string` untuk Crew-langsung-di-Ship (non-Squad, role Supervisor/Security — lihat riset §1.2); (5) guard kuota dari `entitlement.QuotaPolicy` (BL-80 di bawah) dipasang DI `handleCollections` (`fleet/delivery.go`) untuk koleksi `ships`/`squads`/`crew` — BUKAN hanya di `ship.Service`/`squad.Service` yang jalur nyatanya belum tersambung (`ship.Service.CreateShip` nol pemanggil, jalur tulis nyata = `POST /api/collections/{name}` → `SaveCollection` menerima JSON bebas, melewati guard service manapun); (6) cek+insert atomik di bawah lock yang sama (`fleetService.mu`); (7) entitas di atas plafon (lapse/downgrade/impor) tetap bisa dibaca & diekspor, hanya *create* baru ditolak (grandfathering).
- [ ] **ENT-01 (BL-80, P2, S)** `QuotaPolicy` interface + `CommunityPolicy`/`TeamPolicy`/`DockyardPolicy` di `engine/src/entitlement/` (saat ini `interfaces.go` hanya `package entitlement` + TODO). Method: `MaxFleets()`, `MaxShips()`, `MaxSquadsTotal()`, `MaxCrewsPerSquad()`, `MaxCrewsPerShip()`, `MaxConcurrentVoyages()`, **`MaxDelegationDepth()`** (`*int`, nil = unlimited/custom). Nilai final: Community `{1,1,2,6,8,2,1}`, Team `{1,3,6,6,22,4,2}`, Dockyard `{1,6,12,8,100,8,3}`. Endpoint `GET /api/v1/entitlement/usage`. **Dep: BL-81 di atas** untuk AC "guard menolak create ke-N+1" — tanpa itu guard dekoratif. **Konsumen `MaxDelegationDepth()`: BL-05 (§C) — penegakan depth-limit rantai delegasi sub-agent**, ditambahkan 2026-10-06 setelah analisa pola KiroCrew (platform agen sesi ini membatasi rantai Crew→Crew via `max_delegation_depth` statis). **Jangan taruh logika verifikasi lisensi Ed25519 di package publik ini** — `TeamPolicy`/`DockyardPolicy` hanya nilai fallback/default-saat-aktif; verifier Ed25519 Team/Dockyard adalah modul Licensing privat terpisah (repo lain, di luar scope sprint ini — dicatat ongoing).
- [ ] **GO-MOD-RENAME (BL-82, P3, S)** `engine/go.mod` module path `github.com/diezy-labs/claw-crew/engine` → `github.com/diezy-realm/galleon-fleet/engine` (remote git repo sudah pindah, module path belum ikut). **Kerjakan sebelum modul Licensing privat (Q5 Owner) mulai mengimpor package `entitlement`** — menunda ke Sprint 8 (RB-02) berarti modul privat itu harus refactor import path ulang begitu rebrand jalan. Isolated, mekanis (ponytail: byte-level find-replace, verifikasi dengan `go build ./...` bukan grep manual).
- [ ] **BG-F2 · G-25 · PR-53 (P2, M)** Kontrak rute QM two-layer + test.
- [ ] **BG-F3 · G-26 · PR-54 (P2, M)** Audit trail & policy SSOT read-only ke UI/Tauri.
- [ ] **BG-F5 · G-28 · PR-96 (P3, M)** Observabilitas: zap + request-id Go→Rust; scrubbing.

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
