---
title: Backlog — UI Nice-to-Have (Right Rail Git/Sub-Agent, Journal Workspace Grouping, New Chat Entry-Point)
status: active
date: 2026-10-05
author: squad-lead
---

# Backlog UI Nice-to-Have (BL-62 s.d. BL-70)

> Sumber: `standard-docs/research/ui-right-rail-journal-newchat-audit.md` (audit kode `web/src/` + riset kompetitor KiroCrew/Antigravity + 3 keputusan Achmad 2026-10-05). Scope: 3 topik UI — (1) Right Rail tab Git + Sub-Agents, (2) Captain's Journal workspace grouping + bugfix, (3) "+ New Chat" entry-point terpadu. Semua item mengikuti template/layouting EXISTING (`RightRail.tsx`, `CaptainsJournalView.tsx`, `AppSidebar.tsx`) — tidak ada komponen baru dari nol kecuali disebutkan eksplisit.

## Keputusan Yang Mengikat Seluruh Backlog Ini

1. **`Project` ≠ `Workspace`** — dua entity terpisah, tidak dilebur. `Project` = repo nyata (`local_path`/`git_url`), dipakai Tab Git (lintas-fitur). `Workspace` = **scoped ke Captain's Journal saja** (riwayat chat/session) — BUKAN field generik yang numpang di `Quest`.
2. **Koreksi 2026-10-05**: `Quest.workspaceId` terbukti vestigial by grep (`MissionBoard.tsx` filter pakai `projectId`/`suggestedShipId`/`assignedShipId`, `workspaceId` di Quest tidak pernah dibaca untuk query/filter nyata) — field ini **DIHAPUS** dari `Quest`, bukan dipertahankan. `selectedWorkspace`/`setSelectedWorkspace` di `fleetStore.ts` dipersempit jadi state milik Journal, tidak lagi dibaca `QuestsView.tsx`/`QuestBrainstormModal.tsx`/`MissionBoard.tsx`. Ini sekaligus menghapus akar bug BL-65.
3. **Git ops wajib lewat Go** sebagai policy gate (ODR-07 extended) — Tauri/Rust TIDAK boleh `exec git` langsung. Exception hanya fetch metadata read-only via REST API eksternal (GitHub/GitLab).
4. Komponen existing (`RightRail.tsx`, `CaptainsJournalView.tsx`, `AppSidebar.tsx`) **diperluas**, bukan diganti — pola tab-switcher dan grouping harus konsisten styling Tailwind yang sudah dipakai (lihat kelas `rounded-xl border border-neutral-200 dark:border-neutral-800`, dsb).

---

## Topik 1 — Right Rail: Tab Git + Sub-Agents

| BL | Judul | File Scope | Severity | Dependency | Status |
|---|---|---|---|---|---|
| BL-62 | `Project` entity + Go endpoint `GET /api/project/git-status` (branch, dirty/clean, ahead/behind) | `engine/src/fleet/` (baru: `project_git.go`), `web/src/types/index.ts` (`Project`), `web/src/components/layout/RightRail.tsx` | MEDIUM-HIGH | Tidak ada — bisa duluan | Aktif |
| BL-63 | Tab-switcher di `RightRail.tsx`: `Fleet Pulse` \| `Git` \| `Sub-Agents` | `web/src/components/layout/RightRail.tsx` | MEDIUM | Setelah BL-62 (Tab Git butuh data) | Aktif |
| BL-64 | Tab Sub-Agents — endpoint status sub-agent aktif | `engine/src/fleet/services.go`, tabel `subagent_runs` (SQLite) | MEDIUM | **BERGANTUNG pada Gap #2 (sub-agent persistence) dari `turn-resilience-and-steer-queue-audit.md`** — jangan bangun endpoint terpisah sebelum tabel ini ada. Satu backend, dua konsumen (resume-logic + tab ini) | **DITUNDA** — tunggu keputusan eksekusi Gap #2 |

**Catatan keamanan BL-62**: endpoint git-status read-only (`status`/`branch`/`log`) tetap WAJIB melalui `EvaluateTerminalCommand`/policy gate yang sama dengan terminal eksekusi (`terminal_policy.go`) — bukan karena read-only otomatis aman, tapi karena `path`/`gitRemoteUrl` sumbernya input user dan harus divalidasi sebelum di-exec sebagai argumen command.

---

## Topik 2 — Captain's Journal: Workspace Grouping + Bugfix

| BL | Judul | File Scope | Severity | Dependency | Status |
|---|---|---|---|---|---|
| BL-65 | Bugfix: `handleCreateNew` pakai `selectedWorkspace` GLOBAL (state lintas-fitur, dipakai juga Mission Board) alih-alih konteks section yang sedang dibuka di Journal | `web/src/components/features/CaptainsJournalView.tsx` baris ~70 | **HIGH (bug nyata — entry nyasar workspace)** | Tidak ada — PALING MURAH, kerjakan duluan | Aktif — PRIORITASKAN |
| BL-66 | Grouping visual sidebar kiri per-`workspaceId`: Pinned (lintas-workspace, tetap atas) → section per-workspace collapsible dengan count (meniru pola KiroCrew `Galleon (7)`) | `web/src/components/features/CaptainsJournalView.tsx` | MEDIUM | Setelah BL-65 (hindari grouping yang masih mewarisi bug salah-konteks) | Aktif |
| BL-67 | Tombol "+ New Entry" kontekstual per-section workspace (bukan hanya 1 tombol global di `PageHeader`) — otomatis passing `workspaceId` dari section yang diklik | `web/src/components/features/CaptainsJournalView.tsx` | LOW-MEDIUM | Setelah BL-66 (butuh section sudah ter-render) | Aktif |
| BL-68 | Upgrade `workspaces: string[]` → `Workspace{id,name}[]` **scoped ke Journal saja**, hapus `Quest.workspaceId` (vestigial, terbukti tidak dipakai filter/query), persempit `selectedWorkspace`/`setSelectedWorkspace` jadi state milik Journal (tidak lagi dibaca `QuestsView.tsx`/`QuestBrainstormModal.tsx`/`MissionBoard.tsx`), tambah `JournalSession.projectId?: string` (FK ke `Project` dari BL-62) | `web/src/store/fleetStore.ts`, `web/src/types/index.ts`, `web/src/components/features/QuestsView.tsx`, `web/src/components/features/QuestBrainstormModal.tsx`, `web/src/components/features/MissionBoard.tsx` (hapus referensi `workspaceId`), `web/src/components/layout/ContextBar.tsx` (pindahkan dropdown Workspace ke dalam Journal, top bar global cukup `Project`) | **MEDIUM-HIGH** (menyentuh 6 file, termasuk hapus field dari Quest) | Paralel dengan BL-62 (beda file utama) — tapi BL-65/66/67 harus TUNGGU BL-68 selesai dulu karena state `selectedWorkspace` yang mereka pakai berubah bentuk | Aktif — naik prioritas karena membereskan akar bug BL-65 |

---

## Topik 3 — "+ New Chat" Entry-Point Terpadu

| BL | Judul | File Scope | Severity | Dependency | Status |
|---|---|---|---|---|---|
| BL-69 | Tombol "+ New" di `AppSidebar.tsx` dengan dropdown 3 opsi: "New Journal Entry" (→ `captains-journal` + trigger `setIsCreating`), "New Quest (Autopilot)" (→ `createQuest` + `mission-board`, duplikasi entry-point `RightRail.tsx` yang sudah ada), "New Journal in Workspace..." (submenu pilih dari `Workspace[]`, eksplisit set `workspaceId` — sekaligus menghindari bug BL-65 untuk jalur ini) | `web/src/components/layout/AppSidebar.tsx` | MEDIUM | Setelah BL-68 (butuh `Workspace[]` sudah ber-id) | Aktif |
| BL-70 | "Ephemeral chat" (sesi tidak tersimpan) | — | LOW | — | **TIDAK DIREKOMENDASIKAN dibangun (YAGNI)** — tidak ada use-case eksplisit dari Achmad; kalau nanti dibutuhkan, perlu keputusan privasi/retensi terpisah dulu (betul-betul tidak disimpan vs auto-expire) |

---

## Urutan Eksekusi Disarankan

```
BL-68 (hapus Quest.workspaceId, persempit selectedWorkspace jadi state Journal) — PRIORITAS, membereskan akar bug
  └─→ BL-65 (bugfix create-entry, sekarang trivial karena state sudah scoped benar)
        └─→ BL-66 (grouping, butuh BL-65)
              └─→ BL-67 (tombol kontekstual, butuh section BL-66)

BL-62 (Project entity + Go endpoint) ──→ BL-63 (tab-switcher Git)

BL-69 (dropdown "+New" sidebar) butuh BL-68 (Workspace[] ber-id) + BL-62 (Project untuk opsi lain)
BL-64 (Tab Sub-Agents) — DITUNDA, tunggu keputusan Gap #2 (lihat turn-resilience-and-steer-queue-audit.md)
BL-70 — TIDAK dibangun sekarang (YAGNI)
```

**Serialisasi file untuk hindari dua crew menulis file sama**: `BL-68` (`fleetStore.ts`/`types/index.ts`/`QuestsView.tsx`/`QuestBrainstormModal.tsx`/`MissionBoard.tsx`/`ContextBar.tsx`) **harus selesai dan di-merge dulu** sebelum `CaptainsJournalView.tsx` (BL-65/66/67) dimulai — bukan paralel, karena BL-65/66/67 membaca bentuk `selectedWorkspace`/`Workspace[]` yang baru dari BL-68. `RightRail.tsx` (BL-62/63) independen, boleh paralel dengan BL-68 (file beda, tidak ada ketergantungan data). `AppSidebar.tsx` (BL-69) menunggu BL-68 (Workspace ber-id) DAN BL-62 (Project untuk opsi lain) selesai.

## Belum Diangkat ke Roadmap

Mengikuti pola backlog lain — menunggu konfirmasi eksplisit Achmad sebelum diangkat jadi tiket `R-nn` resmi di `standard-docs/roadmap/`.
