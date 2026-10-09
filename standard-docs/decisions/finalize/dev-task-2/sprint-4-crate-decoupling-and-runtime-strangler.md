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

- [ ] **BG-D1 · G-30 · PR-47 (P1, L)** Agent loop/turn: tool-call loop, kompaksi, batas iterasi di atas `ExecuteTurn` Rust. *Acceptance:* paritas dengan `eval` replay. **Acceptance criteria tambahan (eks-BL-18, DITUNDA → DIANGKAT sprint ini):** jalur masuk `ExecuteTurn` dari Go `spawn_subagent` HARUS punya resource-gate (cap jumlah/memori sub-agent) sebelum BG-D1 dianggap selesai — jangan ulangi gap yang sama yang sudah ditutup di sisi Rust (BL-15, Sprint 5).
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

- [ ] **BG-F1 · G-23 · PR-53 (P1, L)** Model organisasi persisten Quartermaster→Ship→Squad→Crew. Dep: BG-B2 (selesai).
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
