---
title: Dev Task 2 — Sprint Breakdown Terkonsolidasi (dev-tasks/ + backlog/ research)
status: active
date: 2026-10-05
author: squad-lead
supersedes_view: ../dev-tasks/ (tetap dibaca untuk histori PR-selesai; lihat §Reconciliation)
---

# Dev Task 2 — Sprint Breakdown

> **Kenapa folder baru, bukan edit `dev-tasks/`:** `dev-tasks/` adalah working-doc tim dev eksternal (coding di mesin lain) — dikecualikan dari auto-maintenance per lesson 2026-10-04. Folder ini (`dev-task-2/`) adalah **lapisan rekonsiliasi** yang digabungkan oleh Lead Squad: tugas asli `dev-tasks/01..08` (121 tiket roadmap, basis tim dev) + seluruh backlog riset `standard-docs/backlog/` (BL-01 s.d. BL-61, basis Lead/Principle/squad). Tujuan: satu urutan sprint tunggal supaya BE/FE/Platform **tidak mengerjakan hal yang sama dua kali** dari dua sumber berbeda.
>
> **Aturan baca:** `dev-tasks/*.md` tetap SSOT untuk histori PR yang SUDAH selesai (jangan diedit, jangan dipindah). `dev-task-2/sprint-*.md` adalah SSOT untuk **urutan kerja ke depan** — gabungan tiket `BG-*`/`BR-*`/`FW-*`/`FD-*`/`PL-*`/`RB-*`/`GV-*` yang masih `[ ]` + seluruh `BL-*` backlog riset, disusun ulang per-sprint lintas sumber.

## Status Verifikasi Dasar (2026-10-05, branch `feat/refactoring-rust` @ `a296dc23`)

Sebelum menyusun sprint, status `dev-tasks/` diverifikasi langsung ke kode (bukan asumsi dari dokumen) — lihat percakapan sesi ini untuk jejak file:baris. Ringkasan: **Sprint 0 dan sebagian besar Sprint 1 ASLI sudah selesai nyata di kode** (bukan klaim kosong) — BG-A1/A2/A3/A4/A5/A6, BR-A1-A4, BR-B1/B2, BR-C1-C4, FD-A1-A4, FW-A0/A1/A2a/A5/B1, PL-A1-A4, RB-10/11/13/14/15, GV-02-10, BG-B1/B2 semua terverifikasi nyata (implementasi substantif, bukan stub) via spot-check 6 klaim kritis. Satu temuan baru dicatat: **approval-gate endpoint terminal (`BG-A4`) belum punya verifikasi identitas pemanggil** — lihat Sprint 2 item baru `S2-SEC-01`.

Item dev-tasks yang BELUM selesai (`[ ]`) + seluruh BL-01..61 backlog digabung di sprint di bawah.

## Peta Sprint

| File | Fokus | Sumber Gabungan |
|---|---|---|
| [`sprint-0-security-foundation.md`](sprint-0-security-foundation.md) | P0 keamanan sisa + gap approval-identity baru ditemukan | BG-A1b, BG-C1..C3, BR-B3..B6, BL-32, BL-14, BL-19 |
| [`sprint-1-persistence-and-quality-gates.md`](sprint-1-persistence-and-quality-gates.md) | Persistensi Go (SQLite lanjutan/Postgres/RAG), burn-down kualitas Rust | BG-B3..B5, BR-A5..A6, BL-45a/45b/46/47 (token-efficiency), BL-48..50 |
| [`sprint-2-attachment-and-messaging-integrity.md`](sprint-2-attachment-and-messaging-integrity.md) | Bug data-loss attachment + content-safety channel | BL-52..55, BL-14, BL-37, BL-38, BL-39 |
| [`sprint-3-sop-constraint-and-raci.md`](sprint-3-sop-constraint-and-raci.md) | SOP engine: Do's/Don'ts, RACI, Constraint layer (SoD) | BL-06..09, BL-57..61 |
| [`sprint-4-crate-decoupling-and-runtime-strangler.md`](sprint-4-crate-decoupling-and-runtime-strangler.md) | Dekoplasi crate besar + strangler runtime→Go | BR-C5..C8, BR-D1..D4, BG-D1..D5, BL-18 (↑ diangkat, dependency terpenuhi di sprint ini) |
| [`sprint-5-mcp-plugin-and-subagent-resource.md`](sprint-5-mcp-plugin-and-subagent-resource.md) | Sandbox MCP, resource-gate sub-agent, plugin governance | BL-15..17, BL-19..23, BL-24 (tetap exploratory) |
| [`sprint-6-cli-client-protocol-and-providers.md`](sprint-6-cli-client-protocol-and-providers.md) | CLI/Client Protocol, provider reliability/OAuth, memory governance | BG-E1/E2, BR-E1..E4, BL-25..31 |
| [`sprint-7-product-features-and-org-model.md`](sprint-7-product-features-and-org-model.md) | Fitur produk FE (organisasi, approval UI), payment-lockout preventif | BG-F1..F6, FW-C1..C5, BL-40..44 |
| [`sprint-8-rebrand-and-release.md`](sprint-8-rebrand-and-release.md) | Rename massal, WIT v1, CI pangkas, rilis/installer | RB-02, RB-12, RB-16/17, RB-20..41, BR-F1..F5, PL-A5, PL-B2, PL-C1/C2, PL-D1/D2, GV-01/11/12 |

## Reconciliation — BL yang Digabung ke Dev-Task Existing (Bukan Duplikat)

Beberapa BL sudah punya **tiket dev-tasks yang persis sama scope-nya** — ini BUKAN kerja baru, hanya diberi nomor silang supaya terlihat satu sumber kebenaran:

| BL | Tiket dev-tasks yang sama | Keputusan |
|---|---|---|
| BL-18 (resource gate `ExecuteTurn` jalur Go `spawn_subagent`) | `BG-D1`/`BG-D4` (agent loop + sub-agent/handoff di Go) | **DITUNDA → sekarang DIANGKAT** di Sprint 4, karena BG-D1/BG-D4 sudah dijadwalkan di sprint yang sama — gate ditempel sebagai acceptance criteria tambahan BG-D1, bukan tiket terpisah |
| BL-51 (vector memory Go per-scope, koordinasi RAG) | `BG-B4` (memori vektor/RAG per scope Fleet/Ship/Squad/Crew/Run) | **DITUNDA → sekarang DIANGKAT** di Sprint 1, BL-51 jadi acceptance criteria tambahan BG-B4 (bukan tiket terpisah) |
| BL-37/BL-38/BL-39 (redaksi tool I/O, scrub, permission log) | Tidak ada tiket dev-tasks setara — murni temuan Lead | Tetap tiket BL mandiri di Sprint 2, TIDAK bentrok file dengan BG/BR manapun |
| BL-14 (ContentSafety ke channel orchestrator biasa) | Tidak ada tiket dev-tasks setara | Tiket BL mandiri, tapi **irisan file** dengan BL-52 (`clawcrew-channels/src/orchestrator/mod.rs`) — digabung ke sprint yang sama (Sprint 2) agar satu PR, bukan dua PR beririsan file |
| BR-C5 (`gateway` lepas dari `runtime`) | Dep: `BR-D1` cron (dev-tasks sendiri) | Tetap di Sprint 4 bersama BR-D1, urutan dari dev-tasks dipertahankan |
| **BR-F1** (errata ADR-001: WhatsApp Tier A, WIT v0 beku, bluesky) | **`GV-07`** — SUDAH `[x]` selesai PR-21, file `standard-docs/decisions/refactoring-rust/phase-2/ADR-001-errata.md` **sudah ada dan isinya persis mencakup ketiga poin BR-F1** (diverifikasi baca file langsung) | **BR-F1 DITANDAI SUDAH TERPENUHI oleh GV-07 — TIDAK dijadwalkan ulang di sprint manapun.** Ini duplikat murni di dalam `dev-tasks/02-be-rust-core.md` sendiri (tiket `[ ]` yang kerjanya sudah dilakukan tapi dicentang di bawah nomor tiket lain) — perlu diberitahu ke tim dev agar BR-F1 dicentang `[x]` juga, bukan dikerjakan ulang |
| BL-33 (Windows Authenticode signing) | `PL-D1` (Pipeline rilis desktop MSI/NSIS, **signing**) | BL-33 adalah detail acceptance-criteria dari `PL-D1`, BUKAN tiket sejajar baru — digabung sebagai sub-item `PL-D1` di Sprint 8 |
| BL-34 (`tauri.bundled.conf.json` hilang) | `PL-D1`/`FD-C2` (bundling Windows) | Sub-item verifikasi `PL-D1`/`FD-C2` di Sprint 8, bukan tiket mandiri |

**Prinsip gabungan**: kalau 2+ tiket (dari sumber manapun) menyentuh file yang sama atau punya hard dependency, mereka **masuk sprint yang sama** dan diberi catatan "serialkan penulis" di file sprint — sesuai aturan assignment Lead (hindari dua crew menulis file sama).

## Urutan & Ketergantungan Antar-Sprint

```
Sprint 0 (keamanan) ──┬─→ Sprint 2 (attachment/messaging) ──→ Sprint 3 (SOP constraint)
                      │
                      ├─→ Sprint 1 (persistence/quality) ──→ Sprint 6 (CLI/providers)
                      │
                      └─→ Sprint 4 (crate decoupling/strangler) ──→ Sprint 5 (MCP/subagent)
                                                               ──→ Sprint 7 (product features)
                                                                        ↓
                                                              Sprint 8 (rebrand/release) — SELALU TERAKHIR
```

Sprint 8 (rename massal) secara desain **tidak boleh mulai** sebelum Sprint 0-7 selesai — ini aturan existing dev-tasks (`RB-2x` hanya setelah hapus/pecah selesai) yang dipertahankan, bukan aturan baru.

## DoD & Checklist PR

Lihat [`dod/`](dod/) — satu file checklist per sprint, format konsisten dengan `roadmap/PR-plan.md` §1 (gerbang GW-0..9) + Definition-of-Done per-tim dari `dev-tasks/01..08`. Checklist DoD TIDAK mengubah gerbang yang sudah ada — hanya mengelompokkannya per-sprint supaya Principle punya satu daftar per PR-batch (sesuai pola kerja "PR batch-by-phase" yang sudah berjalan).

## Catatan Non-Negosiasi (dipertahankan dari dev-tasks asli)

1. Sumber kebenaran TIKET tetap `roadmap/*.md` (R-nn/G-nn/U-nn/N-nn/P-nn/D-nn) — `dev-task-2/` adalah tampilan kerja gabungan, bukan pengganti.
2. `dev-tasks/` asli TIDAK diedit oleh Lead — biarkan tim dev update sendiri; `dev-task-2/` murni referensi tambahan di sebelahnya.
3. Centang `[x]` di `dev-task-2/sprint-*.md` HANYA setelah PR merge + gerbang relevan lulus + Principle approve (sama seperti aturan `dev-tasks/`).
4. BL yang masih **DITUNDA/EXPLORATORY** (BL-24) TIDAK dimasukkan ke sprint manapun sebagai kerja aktif — tetap tercatat di `backlog/README.md`, tunggu keputusan Owner eksplisit.
