---
title: Backlog — Toolbox Parity (8 Fitur KiroCrew vs galleon-fleet) + Smart Recommendation
status: active
date: 2026-10-05
author: squad-lead
---

# Backlog Toolbox Parity (BL-71 s.d. BL-79)

> Sumber: `standard-docs/research/toolbox-parity-audit.md` (audit 8 fitur toolbox KiroCrew vs galleon-fleet, API vs UI terverifikasi file:baris). **Fitur 3 (Attachment) TIDAK diberi BL baru** — bug-nya persis BL-52..55 yang sudah ada di `attachment-handling-backlog.md`/`dev-task-2/sprint-2`, hanya dikonfirmasi ulang masih terbuka. BL baru di sini HANYA untuk gap yang belum tercatat di backlog manapun.

## Peta Backlog

| BL | Fitur | Judul | File Scope | Severity | Status |
|---|---|---|---|---|---|
| BL-71 | 1 (Approval mode) | Toggle approval-mode per-chat (Normal/Reads/Trust/YOLO) terhubung ke `RiskTier`/`RiskClass` API yang sudah ada | `web/src/components/layout/ContextBar.tsx` (baru: dropdown dekat composer), `engine/src/tool/approval_gate.go` (baca, tidak diubah) | MEDIUM-HIGH | Aktif |
| BL-72 | 1 (Approval mode) | Sambungkan `ApprovalForm.tsx`/`fleetStore.ts::handleApproval` ke endpoint nyata (`POST /api/v1/tool-executions/{id}/approve\|deny`) — saat ini mock state lokal | `web/src/components/ApprovalForm.tsx`, `web/src/store/fleetStore.ts` | MEDIUM | Aktif, independen dari BL-71 |
| BL-73 | 2 (Quest-loop) | Desain mekanisme nudge-interval + cycle-limit + stop-file untuk `Quest` (ADR dulu, bukan langsung kode — scope besar, bersinggungan Gap #2 sub-agent persistence) | `engine/src/mission/` (baru), proto `TurnRequest` (tambah field), `standard-docs/decisions/` (ADR) | HIGH (scope besar) | **EXPLORATORY — butuh keputusan Owner sebelum eksekusi**, bukan langsung dikerjakan |
| BL-74 | 4 (Context bar) | Hapus/ganti context-window placeholder hardcoded (`EngineRoomView.tsx:335`) dengan data real ATAU hapus kalau belum bisa ditrack | `web/src/components/features/EngineRoomView.tsx` | LOW-MEDIUM (configuration-lies fix) | Aktif, murah |
| BL-75 | 5 (Chat to entity) | Tombol "Chat" di `CrewView.tsx`/`SquadsView.tsx`/`ShipsView.tsx` yang membuka sesi Journal terikat entitas (pakai `JournalSession.workspaceId`/`projectId` dari BL-68) | `web/src/components/features/CrewView.tsx`, `SquadsView.tsx`, `ShipsView.tsx` | MEDIUM | Aktif, **setelah BL-68** (butuh model Journal-scoping final) |
| BL-76 | 6 (@/slash) | Autocomplete `@` (mention file/folder dari `Project`, BL-62) dan `/` (slash command skill) di composer Journal | `web/src/components/features/CaptainsJournalView.tsx` | MEDIUM-HIGH (fitur baru, bukan kecil) | Aktif, **setelah BL-62** (butuh `Project` entity untuk `@`-mention) |
| BL-77 | 7 (Project/workspace bar) | Pindahkan switcher Workspace dari `QuestsView.tsx` sidebar ke bentuk yang benar: Workspace scoped Journal (sesuai BL-66/68), Project (BL-62) jadi bar global di `ContextBar.tsx` yang BENAR-BENAR dirender (bukan hanya diimpor) | `web/src/components/layout/ContextBar.tsx`, `web/src/components/features/QuestsView.tsx` (hapus switcher Workspace dari sini jika BL-68 menghapus `Quest.workspaceId`) | MEDIUM | Aktif, **menyatu dengan BL-68/BL-69** (`ui-nice-to-have-backlog.md`) — jangan kerjakan terpisah, lihat §Reconciliation |
| BL-78 | 8 (Optimize prompt) | **Spawn sub-agent "Prompt Engineer"** (task kecil, scope sempit) untuk mempertajam intent user SEBELUM turn utama — bukan fungsi rewrite di tengah pipeline. Hasil tampil **editable di composer, tidak auto-send**. Reuse infrastruktur delegasi `clawcrew-runtime/src/subagent/mod.rs` (BL-05) — **DIPUTUSKAN** (bukan "kemungkinan" lagi), satu mekanisme delegasi dipakai bersama BL-79 | `crates/clawcrew-runtime/src/subagent/` (reuse), modul baru untuk logic Prompt Engineer (lokasi tepat ditentukan saat implementasi — kandidat: `clawcrew-runtime/src/agent/prompt_engineer.rs` atau submodul `subagent/`), `web/src/components/features/CaptainsJournalView.tsx` (tombol + editable composer) | MEDIUM-HIGH (fitur baru) | **ON DEV (2026-10-05, keputusan Achmad)** — metode final: lihat `research/toolbox-parity-audit.md` §Penajaman Metode + §Klarifikasi Flow. Kriteria DoD: hasil sub-agent HARUS tampil editable di composer, TIDAK auto-send |
| BL-79 | Baru (top-bar "smart rekomendasi") | Post-response: **spawn sub-agent sejenis BL-78** (bukan LLM utama) untuk tarik kesimpulan cepat dari response terakhir + array maks 5 pertanyaan lanjutan (hybrid instruksi-prompt + hard-cap `slice(0,5)`). Klik 1 item → teks muncul di composer (editable, tidak auto-send). Klik item ke-2+ → gabung dengan separator `", "` (koma+spasi) | `web/src/components/layout/ContextBar.tsx` (baru: render chip rekomendasi), reuse modul sub-agent yang SAMA dengan BL-78 (satu mekanisme delegasi, dua titik panggil: pre-turn untuk BL-78, post-turn untuk BL-79) | MEDIUM-HIGH (fitur baru) | **ON DEV (2026-10-05, keputusan Achmad)** — pertanyaan terbuka "LLM utama vs sub-agent" DIPUTUSKAN: sub-agent, satu mekanisme dengan BL-78. Kriteria DoD: array ≤5 (hard-cap wajib aktif meski instruksi-prompt sudah dipasang), klik gabung composer dengan separator `, ` |

## Reconciliation — Hindari Duplikasi dengan Backlog/Dev-Task Existing

| Temuan sesi ini | Sudah tercatat di | Keputusan |
|---|---|---|
| Fitur 3 (Attachment) bug Debouncer masih terbuka | `attachment-handling-backlog.md` BL-52..55, `dev-task-2/sprint-2-attachment-and-messaging-integrity.md` §A | **TIDAK ada BL baru** — ini re-konfirmasi BL-52 masih berlaku, bukan temuan baru. Naikkan urgensi: sudah 2 sesi berturut dikonfirmasi belum diperbaiki. |
| Fitur 7 (bar project/workspace) | `ui-nice-to-have-backlog.md` BL-62 (Project entity), BL-68 (Workspace scoping Journal), BL-69 (dropdown sidebar) | BL-77 di atas **menyatu** dengan ketiga BL ini, bukan tiket independen — scope-nya persis "render switcher yang benar" yang sudah didesain BL-62/68/69, hanya ditambah observasi `QuestsView.tsx` sidebar perlu dibersihkan setelah `Quest.workspaceId` dihapus. |
| Fitur 1 (Approval mode API) | Sudah lengkap (`approval_gate.go`), tidak ada gap backend | Tidak ada BL baru untuk sisi API — BL-71/72 murni UI/wiring. |

## Urutan Eksekusi Disarankan

```
BL-74 (context placeholder fix) — paling murah, independen, kerjakan duluan
BL-72 (sambungkan ApprovalForm ke API nyata) — independen, murah-medium

BL-62 (Project entity, dari ui-nice-to-have-backlog) ──→ BL-76 (@ mention)
BL-68 (Workspace scoping, dari ui-nice-to-have-backlog) ──┬─→ BL-75 (Chat to entity)
                                                            └─→ BL-77 (project/workspace bar, MENYATU BL-69)

BL-71 (toggle approval-mode UI) — independen, tapi butuh keputusan desain dulu (lihat Catatan)

BL-78 (Prompt Engineer sub-agent) ──→ BL-79 (Smart rekomendasi, REUSE mekanisme sub-agent BL-78)
   — ON DEV, serialkan: bangun mekanisme delegasi+UI-editable di BL-78 dulu, BL-79 reuse modulnya untuk titik panggil post-turn

BL-73 (Quest-loop) — TETAP EXPLORATORY, tunggu keputusan Owner, scope besar (TIDAK termasuk promosi ON DEV 2026-10-05)
```

## Catatan Keputusan Terbuka (Butuh Owner)

1. **BL-71**: apakah toggle Normal/Reads/Trust/YOLO ala KiroCrew cocok dipetakan ke model governance galleon-fleet (Ship/Squad/Crew `authority: read_only|draft_only|gated_write`), atau perlu bentuk UI berbeda? Jangan tiru UI KiroCrew mentah-mentah tanpa cek kesesuaian model data terlebih dahulu.
2. **BL-73 (Quest-loop)**: TETAP EXPLORATORY — scope besar, perlu ADR dulu — apakah loop ini jalan di Go (`orchestrator`) atau butuh komponen scheduler baru? Bersinggungan dengan Gap #2 (sub-agent persistence) dari `turn-resilience-and-steer-queue-audit.md`. Tiga pertanyaan tambahan dari riset metode 2026-10-05: (a) Go atau Rust yang pegang loop, (b) stop-file literal atau sinyal in-memory, (c) bagaimana cycle-limit menghindari konflik dengan `loop_detector.rs` existing yang bisa salah-tangkap re-injection goal sebagai stuck-pattern.

**BL-78 dan BL-79 DIPROMOSIKAN ke ON DEV (2026-10-05, keputusan Achmad)** — tidak lagi EXPLORATORY. Pertanyaan terbuka sebelumnya (siapa jalankan tarik-kesimpulan BL-79) sudah dijawab sebagai bagian dari keputusan promosi ini: **sub-agent, SATU mekanisme delegasi dipakai bersama BL-78** (bukan LLM utama, bukan dua mekanisme terpisah). Detail implementasi (lokasi modul tepat, nama tool/fungsi) ditentukan saat eksekusi — bukan lagi butuh keputusan Owner tambahan.

## Belum Diangkat ke Roadmap

Menunggu konfirmasi eksplisit Achmad sebelum diangkat jadi tiket `R-nn` resmi di `standard-docs/roadmap/`.
