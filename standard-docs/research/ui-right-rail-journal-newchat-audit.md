---
title: Audit UI/UX — Right Rail (Git/Subagent), Captain's Journal Workspace, New Chat Options
status: draft
date: 2026-10-05
author: squad-lead
---

# Audit UI/UX: 3 Fitur Nice-to-Have (Referensi KiroCrew)

> Konteks: Owner melampirkan 3 screenshot dashboard KiroCrew (right-sidebar Git panel, left-sidebar session list, "+ New Chat" dropdown dengan opsi spesifik) dan meminta analisis kesenjangan + kompetitor untuk 3 topik di `web/` galleon-fleet.

## 1. Right Sidebar — Git Tracking, Sub-Agent Process, Docs (ala KiroCrew)

### Status di galleon-fleet (verifikasi kode)

`web/src/components/layout/RightRail.tsx` **sudah ada dan sudah di-wire** (toggle `isFleetPulseOpen`, responsive desktop-rail + mobile-drawer) — TAPI isinya murni **fleet-governance widgets**: Treasury (BYOK spend), Captain's Approval (pending), Active Voyages (quest progress %), Quick Actions. **Nol konten Git, nol subagent-process, nol docs panel.** Ini bukan "belum ada sidebar kanan" — strukturnya ADA, hanya kontennya beda total dari yang di-screenshot (Git diff/commits, Session tabs, File tree).

Screenshot pertama Anda (KiroCrew) menunjukkan: tab-switcher (`Git` | file lain) di atas, lalu di bawahnya **Changes** (list file modified dengan path lengkap + ikon status M/A/D) dan **Commits** (hash pendek + message conventional-commit + author + waktu relatif + badge `HEAD`). Ini levelnya **source-control panel VS Code-native** (`SCM view`), bukan widget ringkasan governance.

### Analisis kompetitor (riset eksternal)

Pola "right-rail untuk git+subagent+docs" bukan fitur eksotis — ini **gap yang terdokumentasi eksplisit sebagai pain point** di komunitas coding-agent:
- Issue nyata di repo agent CLI besar: *"There's no way to glance at token spend, git branch, todo items, or subagent activity without opening overlay panels"* — permintaan *"Persistent right-side dashboard panel"*. Ini PERSIS kebutuhan yang Anda gambarkan.
- Beberapa tool pihak-ketiga lahir KHUSUS untuk mengisi gap ini karena IDE/CLI aslinya tidak punya: `agtop` ("top-style TUI... tracks every session: spend, token usage, context pressure, CPU load, tool invocations"), `ccmux` ("hard to keep track of which session is idle, which is waiting for permission... jump to the one that needs you"), `agent-dashboard` (circular usage meters + GitHub contribution heatmap).
- Cursor punya pendekatan BERBEDA: bukan sidebar real-time, tapi **atribusi line-level** (deteksi baris mana yang ditulis AI vs manusia, dibandingkan ke commit history) — ini levelnya analytics pasca-fakta, bukan live-tracking.

**Kesimpulan kompetitor**: tidak ada SATU pola dominan "right-rail standar" — KiroCrew (native VS Code-style SCM tab) dan tool pihak-ketiga (`agtop`/`ccmux`, TUI terpisah) menyelesaikan masalah yang SAMA dengan cara BERBEDA. Sinyal pentingnya: **kebutuhannya nyata dan sudah validated oleh banyak proyek independen** — bukan preferensi estetika semata.

### Rekomendasi konkret untuk galleon-fleet

Jangan desain dari nol — **perluas `RightRail.tsx` jadi tab-switcher**, bukan ganti isinya:
1. Tambah tab-switcher di atas `RightRail` (mirip screenshot 1): `Fleet Pulse` (konten existing) | `Git` (baru) | `Sub-Agents` (baru).
2. **Tab Git**: butuh data real dari backend — grep awal menunjukkan galleon-fleet BELUM punya endpoint git-status di `engine/`/`crates/` (ini perlu diverifikasi terpisah, DITANDAI follow-up: cek apakah Tauri shell punya akses `git2`/shell git langsung, karena desktop app BISA baca git lokal tanpa lewat Go/Rust gateway — sesuai layering ODR-07 "eksekusi terminal dari Tauri wajib lewat Go", TAPI baca-status git read-only mungkin beda kelas risiko dari eksekusi command, perlu keputusan Lead).
3. **Tab Sub-Agents**: data sumbernya SUDAH ADA secara konsep — `TurnEvent` (`EventSubagentSpawned`, Bagian 1 audit sebelumnya) sudah membawa `SubagentID`+task. Yang belum ada adalah ENDPOINT yang expose sub-agent AKTIF (bukan historical event) ke frontend — ini **beririsan langsung dengan Gap #2 (sub-agent persistence)** dari audit `turn-resilience-and-steer-queue-audit.md`: kalau tabel `subagent_runs` dibangun (rekomendasi sebelumnya), endpoint untuk tab ini otomatis punya sumber data. **Jangan bangun endpoint sub-agent-status terpisah sebelum Gap #2 diputuskan** — satu pekerjaan backend, dua konsumen (resume-logic + UI tab ini).
4. **Docs**: paling murah — kalau maksudnya "dokumen terkait task aktif" (bukan git), ini bisa jadi tab ketiga yang murni membaca `standard-docs/`/artifact yang sudah di-reference di quest/journal aktif — tidak perlu backend baru, cukup query file-list dari context yang sudah ada di state.

---

## 2. Left Sidebar Session/Workspace — Captain's Journal Perlu Rapi Workspace

### Status di galleon-fleet (verifikasi kode)

`CaptainsJournalView.tsx` **sudah ada dan cukup matang** — search, pin/unpin, archive, convert-to-Artifact, convert-to-Quest, privacy footer ("Journal vs Logbook"). Tapi struktur groupingnya **FLAT**: hanya "Pinned Discussions" vs "Recent Sessions" — TIDAK ADA grouping per-workspace sama sekali di level UI, walau DATA-nya sudah punya `workspaceId` (`fleetStore.ts`: `selectedWorkspace`, `workspaces: string[]`, `createJournalSession(title, workspaceId)`).

Bandingkan ke screenshot 2 (KiroCrew sidebar kiri): grouping jelas per **folder project** — `Galleon` (7 sessions) lalu `Auto-Improve - diezy-realm...` (1 session) sebagai section terpisah, masing-masing collapsible. Ini levelnya **folder/project grouping asli**, bukan tag pin/unpin.

**Akar masalah "berantakan" yang Anda rasakan**: `workspaces: string[]` di `fleetStore.ts` saat ini (`'Product Platform', 'Autonomous Agents', 'Core Infrastructure'`) adalah **daftar nama statis flat** — bukan struktur folder (tidak ada nesting, tidak ada relasi ke path lokal/URL repo). `createJournalSession` menerima `workspaceId` sebagai string bebas, TAPI `CaptainsJournalView.tsx` tidak pernah mengelompokkan render berdasarkan field ini — field-nya tersimpan tapi tidak dipakai untuk grouping visual (`activeSession.workspaceId` hanya ditampilkan sebagai badge kecil di top-bar, BUKAN dipakai sebagai section header di sidebar kiri).

### Perbandingan konkret dengan screenshot KiroCrew

| Aspek | KiroCrew (screenshot 2) | galleon-fleet `CaptainsJournalView` saat ini |
|---|---|---|
| Grouping sidebar | Per-folder/project (`Galleon`, `Auto-Improve - diezy-realm...`), collapsible, dengan COUNT per grup | Flat: hanya Pinned vs Recent, TIDAK ada section per-workspace |
| Identitas folder | Terhubung ke REPO ASLI (`diezy-realm/galleon-fleet`, terlihat dari nama folder = org/repo) | `workspaceId` cuma label bebas (`'Product Platform'`), TIDAK terhubung ke path lokal atau URL repo manapun |
| Status/tag per sesi | `Planned`/`Autopilot` badge per-sesi, warna beda per state | Tidak ada tag status di level sesi journal (hanya pin/archive boolean) |
| New chat DI DALAM folder yang sama | Ada opsi eksplisit "New chat in folder" (lihat screenshot 3) — submenu pilih folder tujuan | `createJournalSession` menerima `workspaceId` sebagai parameter, TAPI TIDAK ADA UI untuk memicu "buat sesi baru DI workspace yang sedang dibuka" — tombol `+ New Entry` yang ada hanya buka form judul, workspace yang dipakai otomatis `selectedWorkspace` GLOBAL (dari state lain, bisa saja berbeda dari workspace yang sedang di-browse user) |

**Konfirmasi akar "berantakan"**: `handleCreateNew` di `CaptainsJournalView.tsx` memanggil `createJournalSession(newTitlePrompt.trim(), selectedWorkspace)` — `selectedWorkspace` ini VARIABEL GLOBAL dari store, BUKAN "workspace dari sesi yang sedang di-lihat/filter saat ini". Kalau user sedang scroll-lihat sesi-sesi di bawah grup "Autonomous Agents" tapi `selectedWorkspace` global masih `'Product Platform'`, entry baru akan masuk ke workspace yang SALAH — ini PERSIS pola bug yang membuat workspace "berantakan" (entry nyasar ke grup yang tidak diharapkan user), bukan cuma soal tampilan visual.

### Rekomendasi konkret

5. **Tambah grouping visual per-`workspaceId`** di `CaptainsJournalView.tsx` — ganti struktur render "Pinned vs Recent" (flat) jadi: Pinned (lintas-workspace, tetap di atas) → lalu PER-WORKSPACE sebagai section collapsible (meniru pola KiroCrew screenshot 2) dengan count seperti `Galleon (7)`. Field `workspaceId` SUDAH ADA di data — ini **perubahan render/grouping murni, BUKAN perubahan schema data**. Estimasi effort: kecil-medium (satu komponen, logic group-by yang sudah lazim di React).
6. **Perbaiki bug "entry baru nyasar workspace"** — ganti referensi `selectedWorkspace` (global) di `handleCreateNew` jadi context lokal: workspace yang terakhir di-klik/di-filter DI DALAM `CaptainsJournalView` itu sendiri, bukan state global yang bisa saja sedang dipakai halaman lain (`mission-board`, dll yang juga baca `selectedWorkspace` sesuai grep `questData.workspaceId || get().selectedWorkspace` di baris 603). Ini **bug fix, bukan feature baru** — root cause: satu variabel global dipakai sebagai default di DUA tempat berbeda (quest creation DAN journal creation) tanpa isolasi konteks.
7. **"Create chat baru DI workspace yang sama"** (permintaan eksplisit Anda) — tambahkan tombol "+ New Entry" KEDUA yang kontekstual: muncul di header SETIAP section-workspace (bukan hanya satu tombol global di `PageHeader`), yang otomatis mem-passing `workspaceId` dari section itu — meniru pola "New chat in folder" di screenshot 3 KiroCrew (submenu pilih folder, atau tombol inline per-folder yang workspace-nya sudah implisit dari section).
8. **Keputusan desain terbuka (tanya Owner)**: apakah `workspaceId` SEBAIKNYA mulai terhubung ke entitas nyata (path lokal `[PROJECT]` aktif atau URL repo Git), bukan string bebas seperti sekarang? Ini relevan karena permintaan Anda menyebut "antar dokumen (local / url repo)" — kalau workspace journal mau benar-benar terikat ke repo yang sedang dibuka (seperti folder KiroCrew yang namanya persis `org/repo`), ini perubahan model data (`workspaces: string[]` → objek dengan `{id, label, kind: 'local'|'git-url', path/url}`), BUKAN sekadar perubahan tampilan — scope lebih besar dari rekomendasi #5-7, perlu keputusan eksplisit sebelum dikerjakan.

---

## 3. "+ New Chat" dengan Opsi Spesifik (ala KiroCrew V3)

### Status di galleon-fleet (verifikasi kode)

Grep `AppSidebar.tsx` untuk `New Chat`/`new chat`/dropdown/Menu: **nol hasil.** Tidak ada tombol "+ New Chat" dengan dropdown opsi di sidebar utama galleon-fleet sama sekali hari ini — navigasi murni daftar menu tetap (`COMMAND`/`FLEET`/`OPERATIONS`/`CONTROL` sections, masing-masing item adalah `NavigationTab` statis seperti `quarterdeck`/`realm`/`captains-journal`). Tidak ada konsep "mulai percakapan baru dengan mode tertentu" di level sidebar global — yang ada hanya `createJournalSession` (khusus di dalam Captain's Journal) dan `createQuest` (khusus di Mission Board/Right Rail).

### Detail opsi KiroCrew (dari screenshot 3 + dokumentasi resmi)

Dropdown "+ New Chat" KiroCrew punya **4 opsi berbeda secara eksplisit**, bukan satu tombol generik:
1. **New chat** — percakapan biasa, default.
2. **New autopilot chat** — "One job, done in steps — it plans first, you approve, then it runs stage by stage." Ini mode EKSEKUSI terstruktur (plan→approve→execute per-stage), beda dari chat eksplorasi biasa.
3. **New ephemeral chat** — sesi sekali-pakai, kemungkinan tidak tersimpan permanen di riwayat (meniru konsep "incognito").
4. **New chat in folder** — submenu PILIH folder tujuan (`Galleon` / `Auto-Improve - diezy-realm/galleon-fleet` di screenshot), chat baru otomatis masuk ke grup itu.

Plus item navigasi terkait: **Crew Members** ("Opens the Crew Members page — each member is a standing agent with its own thread") dan **New folder** — ini levelnya organisasi SESI, bukan sekadar mulai percakapan.

### Analisis — Kenapa Ini Relevan untuk galleon-fleet

Galleon-fleet SUDAH punya KONSEP yang setara untuk 2 dari 4 opsi KiroCrew, hanya BELUM disatukan sebagai satu entry-point:

| Opsi KiroCrew | Konsep setara di galleon-fleet (SUDAH ADA, tersebar) | Lokasi |
|---|---|---|
| New chat (biasa) | `createJournalSession` tanpa tujuan spesifik | `CaptainsJournalView.tsx` |
| **New autopilot chat** (plan→approve→execute per-stage) | **Quest** (`createQuest`) — "Living SOPs & workflow maps", punya `activeVoyageProgress` (stage-based progress) | `RightRail.tsx` Quick Actions "Launch New Quest", `useFleetStore` |
| New ephemeral chat | **Tidak ditemukan** — grep tidak menemukan konsep "sesi tidak tersimpan" di `fleetStore.ts` | — |
| New chat in folder | Setara `workspaceId` parameter yang SUDAH ada di `createJournalSession`, tapi belum ada UI pemicu (lihat rekomendasi #7 di atas) | `fleetStore.ts` |

**Insight paling penting**: Anda TIDAK perlu membangun 4 fitur baru dari nol — **2 dari 4 sudah ada sebagai KONSEP TERPISAH** (Journal = chat biasa, Quest = autopilot staged-execution), yang hilang hanyalah **satu entry-point UI terpadu** yang menyatukan pilihannya di level sidebar global, PERSIS seperti KiroCrew menyatukan "chat biasa" dan "autopilot" di satu dropdown yang sama alih-alih dua halaman terpisah tanpa penghubung.

### Rekomendasi konkret

9. **Tambah tombol "+ New" di `AppSidebar.tsx`** (saat ini tidak ada tombol aksi apa pun di sidebar, hanya daftar nav statis) dengan dropdown 3 opsi yang MEMETAKAN ke konsep yang SUDAH ADA, bukan membangun backend baru:
   - "New Journal Entry" → navigasi ke `captains-journal` + trigger `setIsCreating(true)` (state yang sudah ada di `CaptainsJournalView.tsx`, tinggal diakses dari luar via prop/store).
   - "New Quest (Autopilot)" → `createQuest(...)` + navigasi ke `mission-board` (persis tombol "Launch New Quest" yang SUDAH ada di `RightRail.tsx` — cukup DUPLIKASI entry-point-nya ke sidebar kiri, bukan logic baru).
   - "New Journal in Workspace..." → submenu pilih dari `workspaces: string[]` yang sudah ada, memanggil `createJournalSession(title, pickedWorkspaceId)` dengan workspace EKSPLISIT dari pilihan user (ini SEKALIGUS memperbaiki rekomendasi #6 — kalau workspace selalu dipilih eksplisit di titik pembuatan, bug "nyasar ke `selectedWorkspace` global" otomatis tidak relevan lagi untuk jalur ini).
10. **"Ephemeral chat" — TIDAK direkomendasikan dibangun sekarang** (YAGNI): tidak ada kebutuhan eksplisit yang disebut Anda, dan tidak ada analog existing di galleon-fleet untuk ditambahkan — kalau nanti genuinely dibutuhkan (misal "coba prompt cepat tanpa simpan riwayat permanen"), ini fitur terpisah yang butuh keputusan privasi/retensi sendiri (apakah benar-benar tidak disimpan sama sekali, atau disimpan tapi auto-expire) — jangan bangun duluan sebelum ada use-case nyata.
11. **"New folder"/grouping folder generik untuk SEMUA jenis sesi (bukan hanya Journal)** — ini scope LEBIH BESAR dari rekomendasi #5 (yang scope-nya cuma Journal): kalau Owner mau folder yang sama dipakai lintas Journal+Quest+lainnya (bukan `workspaceId` per-fitur yang terpisah), ini butuh desain ulang konsep "workspace/folder" jadi entitas SATU SUMBER KEBENARAN yang dipakai semua fitur — relevan dengan prinsip `AGENTS.md` §Single Source Of Truth. **Keputusan terbuka**: apakah folder grouping ini scope-nya PER-FITUR (Journal punya grouping sendiri, Quest punya sendiri) atau SATU konsep folder lintas-fitur? Ini mempengaruhi besar-kecil scope implementasi #5 dan #9.

---

## Ringkasan Prioritas & Keterkaitan Antar-Rekomendasi

```
#5 (grouping visual Journal per-workspace)
   └─→ #7 (tombol New Entry kontekstual per-section) butuh #5 selesai dulu
#6 (bugfix selectedWorkspace nyasar) ── independen, bisa duluan, PALING MURAH
#8 (workspace→repo nyata?) ── keputusan Owner, menentukan ulang scope #5/#11
#9 (dropdown "+New" sidebar) butuh konsep #2(Quest)+#7(Journal-in-workspace) sebagai target navigasi
#11 (folder lintas-fitur?) ── keputusan Owner, menentukan apakah #5 di-generalize atau tetap Journal-only

Tab Git (Bagian 1) ── independen, butuh keputusan Lead soal akses git read-only dari Tauri
Tab Sub-Agents (Bagian 1) ── BERGANTUNG pada Gap #2 (sub-agent persistence) dari audit turn-resilience sebelumnya
```

**Paling murah untuk dikerjakan duluan (tidak butuh keputusan Owner lebih dulu)**: #6 (bugfix workspace nyasar — ini genuinely bug, bukan feature) dan #5 (grouping visual Journal, data sudah ada tinggal render ulang).

**Butuh keputusan Owner SEBELUM dikerjakan**: #8 (apakah workspace terhubung ke repo nyata), #11 (folder per-fitur vs lintas-fitur), dan akses-git-dari-Tauri (Bagian 1).

---

## Keputusan Final (2026-10-05, Achmad)

Tiga keputusan terbuka di atas sudah dijawab eksplisit:

**#5/#6/#7 — APPROVED.** Tambah 1 level grouping per-`workspaceId` di atas Pinned/Recent di `CaptainsJournalView.tsx`, perbaiki bug `selectedWorkspace` global yang salah konteks, tambah tombol "New Entry" kontekstual per-section.

**#8 — APPROVED dengan desain spesifik: PISAHKAN `Project` dari `Workspace`, JANGAN satu entitas lintas-fitur (#11 ditolak).**

Analisis yang membawa ke keputusan ini (bukan preferensi, berbasis fakta kode):
- `Quest.workspaceId` dan `Quest.projectId` **sudah coexist sebagai 2 field berbeda** di `types/index.ts` — governance-grouping dan repo-identity sudah dibedakan di model Quest yang sudah jalan. Memaksakan satu entitas lintas-fitur (opsi #11) berarti migrasi skema yang sudah di produksi tanpa bukti kebutuhan nyata (YAGNI).
- `ContextBar.tsx` sudah punya 2 sumbu state terpisah: `workspaces`/`selectedWorkspace` DAN `projects`/`selectedProject` — confirming dua konsep berbeda sudah hidup berdampingan di UI top bar, bukan satu yang perlu disatukan.
- Git/repo context (Tab Git di Right Rail, Bagian 1) secara alami melekat ke **direktori proyek konkret** (`local_path`/`git_url`), bukan ke label governance macam `'Product Platform'`.

**Model data final:**
```typescript
// BARU — entity repo nyata, dipakai Tab Git (Right Rail) + opsional Journal/Quest
export interface Project {
  id: string;
  name: string;
  sourceType: 'local_path' | 'git_url';
  path?: string;          // untuk local_path
  gitRemoteUrl?: string;  // untuk git_url
  defaultBranch?: string;
  lastSyncedAt?: string;
  status: 'connected' | 'disconnected' | 'error';
}

// UPGRADE minimal — tetap label governance, hanya diberi stable id (dari string[] jadi {id,name}[])
export interface Workspace {
  id: string;
  name: string;
}

export interface JournalSession {
  // ...existing fields unchanged...
  workspaceId: string;   // tetap — grouping sidebar (governance, TIDAK berubah)
  projectId?: string;    // BARU, opsional — FK ke Project nyata kalau sesi terkait repo
}
```

**#3 (boundary git ops) — APPROVED: logic Go/Rust wajib, exception hanya fetch API eksternal murni.**
- Operasi git read/write (`status`/`branch`/`log`, resolve `local_path`) **wajib lewat Go** sebagai policy gate — sejalan `galleon-architecture.md` ODR-07 ("eksekusi terminal dari Tauri wajib lewat Go"). Git CLI invocation adalah kategori eksekusi-proses-eksternal yang sama, bukan exception baru.
- Alasan security: `exec git` langsung dari Tauri/Rust dengan `path`/`gitRemoteUrl` yang asalnya input user adalah command-injection/path-traversal risk yang sama persis dengan kasus RCE `/api/engine/execute` yang sudah pernah ditutup — harus lewat gate (`terminal_policy.go`/`EvaluateTerminalCommand`) yang sudah ada, bukan jalur baru tanpa review.
- Exception yang DIIZINKAN: fetch metadata read-only via GitHub/GitLab REST API (nama repo, default branch, last commit) — ini HTTP murni, tidak ada proses eksternal yang di-exec, boleh dari layer manapun.
- Rust tetap SSOT untuk LLM Provider (tidak berubah oleh keputusan ini) — resolve path lokal dan eksekusi git command bukan domain LLM, tetap domain Go sesuai tabel 4-tier.

## Koreksi (2026-10-05, Achmad) — `Workspace` adalah milik Captain's Journal, BUKAN field lintas-fitur

Setelah keputusan #8 di atas ditulis, Achmad mengoreksi: *"tetap per-fitur (Journal punya sendiri, Quest punya `projectId`/`suggestedShipId` sendiri) harusnya `workspace` itu masuk ke dalam Captain's Journal, karena berisi riwayat chat/session."*

**Verifikasi kode membenarkan koreksi ini.** `Quest.workspaceId` ternyata **vestigial** — grep konkret menunjukkan `MissionBoard.tsx` memfilter quest pakai `q.projectId` dan `q.suggestedShipId`/`q.assignedShipId` (baris 81/83/141/191), **TIDAK PERNAH** pakai `q.workspaceId` untuk query/filter apa pun. Satu-satunya pemakaian `workspaceId` di jalur Quest adalah sebagai default fallback saat create (`QuestsView.tsx:99`, `QuestBrainstormModal.tsx:222`, `fleetStore.ts:603`: `questData.workspaceId || get().selectedWorkspace`) — field yang di-assign tapi tidak pernah dibaca kembali untuk logic nyata.

**Keputusan final (menggantikan sebagian keputusan #8 sebelumnya):**
1. `Workspace` grouping/entity **scoped ke `JournalSession` saja** — bukan field generik yang numpang di `Quest`. Ini levelnya "kelompok riwayat chat/session", domain murni Journal.
2. **`Quest.workspaceId` DIHAPUS** dari `types/index.ts` (bukan dipertahankan sebagai field basi) — field ini terbukti tidak dipakai untuk filter/query apa pun di produksi. Quest tetap berdiri di atas sumbu kerjanya sendiri: `projectId` + `suggestedShipId`/`assignedShipId`.
3. `selectedWorkspace`/`setSelectedWorkspace` di `fleetStore.ts` **dipersempit jadi state milik Journal** (tidak lagi dibaca `QuestsView.tsx`/`QuestBrainstormModal.tsx`/`MissionBoard.tsx`). Ini sekaligus menghapus AKAR bug BL-65 ("entry nyasar workspace") — bukan workaround UI yang menambal gejala, tapi menghilangkan state-sharing lintas-fitur yang jadi sumber bug.
4. `ContextBar.tsx` top-bar: dropdown Workspace yang saat ini tampil global **dipindah ke dalam Captain's Journal saja** (tidak relevan tampil di semua tab lagi kalau konsepnya scoped Journal). Top bar global cukup pakai `Project`/`selectedProject` (relevan lintas-fitur: Quest, Git tab, dkk — TIDAK berubah oleh koreksi ini).

**Model data final (menggantikan draft sebelumnya):**
```typescript
// Project — TETAP seperti sebelumnya, entity lintas-fitur (Git tab, dll)
export interface Project {
  id: string;
  name: string;
  sourceType: 'local_path' | 'git_url';
  path?: string;
  gitRemoteUrl?: string;
  defaultBranch?: string;
  lastSyncedAt?: string;
  status: 'connected' | 'disconnected' | 'error';
}

// Workspace — scoped ke Journal, bukan lagi field global dipakai Quest
export interface Workspace {
  id: string;
  name: string;
}

export interface JournalSession {
  // ...existing fields unchanged...
  workspaceId: string;   // tetap — kini eksplisit domain Journal, bukan "label governance generik"
  projectId?: string;    // opsional — FK ke Project nyata kalau sesi terkait repo
}

// Quest — workspaceId DIHAPUS, tetap pakai projectId + suggestedShipId/assignedShipId
export interface Quest {
  // workspaceId: string;  <-- DIHAPUS
  projectId: string;      // tetap, sumbu kerja Quest yang genuinely dipakai
  suggestedShipId: string;
  assignedShipId?: string;
  // ...field lain tidak berubah...
}
```

Backlog implementasi dari seluruh 11 rekomendasi + keputusan ini (termasuk koreksi scope Workspace) ditulis di `standard-docs/backlog/ui-nice-to-have-backlog.md`.
