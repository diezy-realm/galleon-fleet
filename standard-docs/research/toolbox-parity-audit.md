---
title: Audit Status 8 Fitur Toolbox KiroCrew vs galleon-fleet (API vs UI)
status: done
date: 2026-10-05
author: squad-lead
---

# Audit: 8 Fitur Toolbox KiroCrew — Status Implementasi galleon-fleet

> Konteks: Owner melampirkan 5 screenshot toolbox KiroCrew (dashboard yang menjalankan sesi ini) dan meminta riset status 8 fitur setaranya di PRODUK galleon-fleet (`engine/src` Go, `crates/` Rust, `web/src` TS/TSX) — dipisah tegas API vs UI/UX. Verifikasi dilakukan sampai implementasi nyata (file:baris) + test, bukan tebakan dari nama/registry, sesuai lesson "configuration-lies" yang berulang ditemukan sesi-sesi sebelumnya.

## Metodologi

4 subagent paralel (2 Backend untuk `engine/src`+`crates/`, 2 Frontend untuk `web/src`) membagi 7 dari 8 fitur berdasarkan kedekatan file-scope. Fitur 7 (bar project/workspace) diverifikasi langsung oleh Lead karena konteks mendalam sudah ada dari audit sesi sebelumnya (`ui-right-rail-journal-newchat-audit.md`).

## Ringkasan Tabel

| # | Fitur | API | UI | Overall |
|---|---|---|---|---|
| 1 | Approval mode | **SELESAI** | **PARSIAL salah-bentuk, mock** | Mismatch |
| 2 | Set goals / quest (loop) | **BELUM ADA** | *(tidak relevan tanpa API)* | Belum ada |
| 3 | Attachment | **PARSIAL (ada bug)** | **BELUM ADA** | Backend ada+bug, FE nol |
| 4 | Bar context & LLM model | — | **PARSIAL (model nyata, context placeholder)** | Separuh nyata |
| 5 | Chat to crew/squad/ships | — | **BELUM ADA** | Belum ada |
| 6 | Tag `@`/`/` | — | **BELUM ADA** | Belum ada |
| 7 | Bar project/workspace | — | **PARSIAL, salah lokasi** | Ada tapi scope salah |
| 8 | Optimize prompt | **BELUM ADA** | **BELUM ADA** | Belum ada |

## Detail Per Fitur

### 1. Approval Mode

**API — SELESAI.** `engine/src/tool/approval_gate.go` (461 baris): `MemoryApprovalGate` dengan `RequestApproval`/`RequestApprovalWithDetails`, channel-blocking timeout 10 menit, CAS validation. Enforcement nyata di `policy.go` (baris 40-55): trigger otomatis berdasarkan `RiskTier`/`RiskClass`. Dipanggil di `services.go:417` — tool execution DIBLOKIR sampai approved. Endpoint HTTP: `POST /api/v1/approvals/{id}/resolve`, `GET /api/v1/approvals/{id}`, `POST /api/v1/tool-executions/{id}/approve|deny`. Dibuktikan test (`approval_enforcement_test.go`, `phase3_tool_test.go`).

**UI — PARSIAL, bentuk salah + mock.** `ApprovalForm.tsx`/`ApprovalsView.tsx`/`ApprovalPage.tsx` adalah approval-QUEUE (list item approve/reject) — BUKAN toggle mode per-chat (Normal/Reads/Trust/YOLO) seperti referensi KiroCrew. `handleSubmit` di `ApprovalForm.tsx:48` hanya callback lokal; `handleApproval` di `fleetStore.ts` manipulasi state lokal, **nol `fetch`/`axios` ke endpoint approval API yang sudah ada**.

### 2. Set Goals / Quest (Loop Otomatis)

**API — BELUM ADA.** `engine/src/quest/`, `engine/src/mission/` eksekusi: tidak ditemukan. `orchestrator.Service.IntakeObjective` mengembalikan proposal-only (`ProposalStatusAwaitingApproval`), tidak pernah eksekusi. Proto `TurnRequest` nol field `cycleLimit`/`maxCycles`/`stopFile`/`goal`/`interval`. Nol match untuk `monitor`/`autonudge`/`goal.*loop` di seluruh backend.

**Kesimpulan**: "Quest" di kode adalah metafora data model (`Quest.mapSteps`, `activeVoyageProgress`) — bukan mekanisme nudge-interval+cycle-limit+stop-file yang nyata jalan otomatis.

### 3. Attachment

**API — PARSIAL dengan bug nyata.** Infrastruktur matang: `crates/clawcrew-api/src/media.rs` (`MediaAttachment`, `MediaKind: Audio|Image|Video|Unknown`), endpoint `POST /api/upload` (`clawcrew-gateway/src/api_upload.rs`, limit 20MB), `MediaPipeline` trait per-channel (`clawcrew-channels/src/orchestrator/media_pipeline.rs`). **BUG TERKONFIRMASI ULANG** (sudah tercatat BL-52 sesi sebelumnya, BELUM diperbaiki): `MessageDebouncer::fire_debounced()` (`clawcrew-infra/src/debounce.rs:176-185`) cuma `entry.messages.join("\n")` — attachment pesan ke-2+ hilang diam-diam saat pesan rapid digabung. Test `multiple_messages_concatenated` hanya memverifikasi string concat, tidak menyentuh attachment sama sekali.

**UI — BELUM ADA sama sekali.** Nol `<input type="file">`/drag-drop di composer manapun: `CaptainsJournalView.tsx` (composer plain `<input type="text">`), `QuartermasterOffice.tsx`, `VoiceQuartermasterModal.tsx` (voice-only). Nol koneksi `fetch`/`axios` ke `/api/upload` yang sudah ada di backend.

### 4. Bar Context & LLM Model

**UI — PARSIAL, satu nyata satu placeholder.** Model selector FUNGSIONAL: `QuarterdeckView.tsx` baris 46-56 (`AIModelOption`, `AI_MODELS` array: Claude 3.7 Sonnet, Gemini 2.5 Pro/Flash, DeepSeek-R1), dropdown nyata baris 736-749. **Context window placeholder PALSU**: `EngineRoomView.tsx:335` — teks hardcoded *"Context window allocated: 8,192 tokens. Local GPU memory at 68% threshold."* — string statis, tidak terhubung sesi nyata manapun. `TreasuryView.tsx:129` punya `tokensUsed` dari `QuestLedgerEntry` (data backend asli) tapi ini ledger biaya, bukan context-window live.

**Kesimpulan**: pola configuration-lies baru — terlihat seperti monitoring real-time, sebenarnya dekorasi.

### 5. Chat to Crew/Squad/Ships

**UI — BELUM ADA sama sekali.** Nol tombol "Chat"/"Message"/navigasi ke sesi chat terikat entitas di `CrewView.tsx`, `SquadsView.tsx`, `ShipsView.tsx`. Chat hanya ada generik di `QuarterdeckView.tsx` dan privat per-sesi di `CaptainsJournalView.tsx` — tidak ada yang terikat ke satu CrewMember/Squad/Ship spesifik.

### 6. Tag `@` Dokumen/Folder, `/` Skills

**UI — BELUM ADA sama sekali.** Satu-satunya match `/` adalah field generik `shortcut?: string` di `SearchBar.tsx:13` (tidak relevan). Nol autocomplete trigger `@`/`/` di composer manapun — input selalu plain-text tanpa file-picker/skill-suggester.

### 7. Bar Project/Workspace

**UI — ADA tapi scope SALAH.** Switcher Workspace/Project nyata DIRENDER (bukan placeholder) di `web/src/components/features/QuestsView.tsx:166-200` sebagai sidebar kiri form create-Quest — TAPI ini BUKAN bar global persisten seperti referensi screenshot. `ContextBar.tsx` (top bar global) MENGIMPOR `selectedWorkspace`/`selectedProject`/setter-nya dari store, tapi **tidak pernah merender dropdown-nya di JSX** — hanya dipakai di Account-menu "Realm & Workspace" yang navigasi ke halaman Settings, bukan switcher langsung.

**Relevansi dengan koreksi sesi ini**: temuan ini konsisten dengan koreksi Achmad bahwa `Workspace` seharusnya scoped ke Captain's Journal (BL-68), bukan tersebar sebagai state global yang numpang di `Quest`/`ContextBar` tanpa bentuk UI yang jelas di manapun.

### 8. Optimize Prompt

**API — BELUM ADA.** Nol hit `optimize_prompt`/`prompt_rewrite`/`PromptOptimizer` di `engine/src/` maupun `crates/` (grep case-insensitive repo-wide = 0 match produksi). Satu-satunya "prompt handling" adalah system-prompt STATIS per-channel (`clawcrew-channels/src/orchestrator/mod.rs::build_channel_system_prompt`, `engine/src/orchestrator/services.go:95-98`) — ini template-filling (datetime/sender), bukan optimasi dinamis.

**UI — BELUM ADA.** Ikon Sparkles muncul di banyak komponen (`QuartermasterOffice.tsx`, `RightRail.tsx`, `ContextBar.tsx`, `ThemeCustomizer.tsx`, `CommandPalette.tsx`) tapi SEMUA dekoratif untuk fitur lain (bukan optimize-prompt) — nol tombol "optimize"/"enhance"/"improve prompt" ditemukan.

## Insight Lintas-Fitur

1. **Bukan pola seragam "UI tertinggal backend"** — tiga pola berbeda: (a) backend matang + UI salah-bentuk (Fitur 1, 7), (b) backend ada-dengan-bug + UI nol (Fitur 3), (c) nol total di kedua sisi (Fitur 2, 5, 6, 8).
2. **Bug Attachment Debouncer paling urgent** — satu-satunya temuan level DATA-LOSS di produksi (bukan "fitur belum ada"), sudah tercatat BL-52, masih terbuka.
3. **Context-window placeholder** (`EngineRoomView.tsx:335`) adalah configuration-lies instance baru — direkomendasikan jadi tiket kecil terpisah.
4. **4 fitur nol-total** (Quest-loop, Chat-to-entity, @/slash, Optimize-prompt) adalah scope FITUR BARU, bukan bugfix — perlu keputusan prioritas Owner, Quest-loop khususnya besar (bersinggungan dengan Gap #2 sub-agent persistence dari audit resiliency sebelumnya).

Backlog implementasi: `standard-docs/backlog/toolbox-parity-backlog.md` (BL-71 s.d. BL-78). Reconciliation ke `dev-task-2/`: lihat README `dev-task-2` §Reconciliation (update berikutnya).

## Penajaman Metode (2026-10-05, Achmad) — Masih EXPLORATORY, Belum Jadi Tiket

Setelah riset metode lanjutan (lihat §Lanjutan di bawah untuk hasil penelusuran mekanisme existing), Achmad memberi arah desain lebih spesifik untuk dua topik. Ini MEMPERJELAS metode, bukan instruksi implementasi — BL-78 tetap EXPLORATORY, top-bar-recommendation belum jadi BL formal.

### BL-78 — Optimize Prompt: Pendekatan "Prompt Engineer Sub-Agent", Bukan Rewrite Langsung

Arahan Achmad: pendekatan optimize-prompt yang lebih matang untuk menghindari **U-turn** (hasil akhir LLM besar menyimpang dari maksud user) adalah dengan **spawn sub-agent kecil berperan "Prompt Engineer"** — bukan fungsi rewrite/template sederhana. Tujuannya memastikan hasil akhir dari LLM utama "mutlak dan sesuai keinginan user", dengan sub-agent khusus yang tugasnya memperjelas/mempertajam intent sebelum sampai ke LLM besar.

**Flow yang diminta:**
1. User klik tombol "Optimize" di composer.
2. Sub-agent "Prompt Engineer" memproses prompt mentah user (scope kecil, task tunggal — bukan LLM besar yang juga menjawab).
3. Hasil **TIDAK auto-send** — tetap tampil di composer sebagai teks **editable**, user masih bisa ubah sebelum kirim.
4. Format hasil memakai separator **koma + spasi (`, `)** antar klausa/poin yang disempurnakan.

**Konsekuensi desain yang perlu dicatat (bukan keputusan final, poin diskusi lanjutan):**
- Ini mengubah posisi BL-78 dari "satu fungsi transform di pipeline backend" (opsi arsitektur dari riset sebelumnya: insersi setelah history-trim) menjadi **pola delegasi sub-agent terpisah** yang berjalan SEBELUM turn utama dimulai — bukan di titik tengah pipeline existing. Ini kemungkinan TIDAK memakai jalur `clawcrew-channels/src/orchestrator/mod.rs` yang sudah dipetakan (karena itu pipeline turn utama, bukan pre-processing terpisah).
- "Editable, tidak auto-send" berarti ini murni **UI-side interaction pattern** (mirip draft/preview), bukan transformasi backend yang transparan ke user — user HARUS melihat dan menyetujui hasil sebelum terkirim.
- Pola "spawn sub-agent kecil untuk tugas spesifik sebelum delegasi ke agent utama" ini konsisten dengan pola `DelegateTaskTool`/sub-agent scoping yang SUDAH ada di `clawcrew-runtime/src/subagent/mod.rs` (BL-05, Sprint 4) — kemungkinan bisa reuse infrastruktur delegasi yang sama, bukan membangun mekanisme spawn baru. Ini butuh verifikasi lanjutan sebelum ADR ditulis.
- Definisi "optimize" masih belum terjawab tuntas (pertanyaan asli backlog) — tapi sudah lebih sempit: bukan "rewrite grammar generik", melainkan "pertajam intent agar LLM besar tidak U-turn". Ini kriteria evaluasi yang lebih konkret untuk desain sub-agent Prompt Engineer nanti.

### Top-Bar Recommendation — Flow Kesimpulan Cepat + Array Pertanyaan Lanjutan (Maks 5)

Arahan Achmad: secara flow, rekomendasi ini menarik beberapa **kesimpulan cepat** dari response terakhir, lalu menyusun **feedback pertanyaan lanjutan** sebagai **array maksimal 5 item** dari hasil response tersebut.

**Konsekuensi desain yang perlu dicatat:**
- Ini levelnya "post-response analysis" — berjalan SETELAH LLM utama selesai merespons, bukan sebelum (beda arah dari BL-78 yang pre-processing). Sumber datanya adalah response yang BARU SELESAI, bukan prompt yang akan dikirim.
- Field `Discovery.type: 'recommendation'` dan pola render `suggestedActions` (ditemukan di riset sebelumnya, `QuartermasterOffice.tsx:278-300`) adalah kandidat fondasi data model — tapi keduanya saat ini TIDAK membatasi jumlah ke maksimal 5, dan tidak eksplisit "array pertanyaan lanjutan" (lebih ke arah "aksi" seperti `create_quest`/`save_artifact`). Perlu field/struktur baru atau perluasan, bukan reuse murni.
- **Sama seperti arahan BL-78**: hasil rekomendasi/pertanyaan ini juga harus **editable** (konsisten dengan prinsip "jangan auto-commit keputusan AI tanpa user approve" yang sudah jadi pola berulang di produk — approval-gate, draft-journal, dst).
- Pola "tarik kesimpulan dari response lalu ajukan balik ke user sebagai pertanyaan" ini secara konsep MIRIP dengan apa yang sedang saya lakukan di akhir setiap turn (opsi `[OPTIONS: ... | ... | ...]`) — tapi itu levelnya convention UI dashboard KiroCrew, bukan mekanisme produk Galleon. Perlu dibedakan jelas: apakah top-bar-recommendation ini meniru pola itu sebagai fitur produk end-user Galleon (`web/`), bukan sekadar steering/convention level agent.

**Pertanyaan terbuka baru yang muncul dari penajaman ini:**
1. Siapa yang menjalankan "tarik kesimpulan cepat" untuk top-bar-recommendation — LLM utama di akhir turn (tambahan step), atau sub-agent terpisah (pola sama dengan Prompt Engineer BL-78)? Kalau pola delegasi sub-agent dipakai konsisten di kedua fitur, ini argumen kuat untuk desain SATU mekanisme delegasi-kecil yang dipakai ulang untuk keduanya (pre-processing untuk BL-78, post-processing untuk recommendation) — bukan dua mekanisme terpisah.
2. ~~"Maksimal 5" — apakah ini hard-cap UI (truncate kalau lebih) atau LLM/sub-agent diinstruksikan eksplisit menghasilkan ≤5?~~ **DIJAWAB 2026-10-05** — lihat §Klarifikasi Flow di bawah.
3. ~~Separator koma+spasi untuk BL-78 — apakah ini format untuk MENAMPILKAN hasil, atau format output sub-agent?~~ **DIJAWAB 2026-10-05** — separator berlaku untuk top-bar-recommendation (gabung multi-klik di composer), BUKAN format output BL-78. Lihat §Klarifikasi Flow.

## Klarifikasi Flow (2026-10-05, Achmad) — Hybrid Instruksi-Prompt + Hard-Cap, dan Flow Multi-Klik

### "Maksimal 5" = Instruksi Prompt (embed) + Hard-Cap sebagai Pengaman, Bukan Salah Satu Saja

Achmad mengusulkan pendekatan: **prompt tambahan di-embed** ke LLM/sub-agent (instruksi eksplisit "tarik kesimpulan + maksimal 5 pertanyaan lanjutan"), lalu **response ditangkap (capture)** sebagai array hasil. Ini levelnya **instruksi-prompt** (LLM diminta patuh ke limit, tapi tidak terjamin matematis — bisa hasilkan 4 atau 7).

Rekomendasi Lead: gabungkan dengan **hard-cap** (`array.slice(0, 5)` di layer render/kode) sebagai pengaman di belakang instruksi prompt — bukan menggantikan pendekatan Achmad, melainkan jaring pengaman kalau LLM tidak patuh persis ke limit:

```
Response LLM utama selesai
        ↓
Embed prompt tambahan ("tarik kesimpulan + maks 5 pertanyaan lanjutan") — INSTRUKSI PROMPT (usulan Achmad)
        ↓
Response ditangkap sebagai array
        ↓
Hard-cap slice(0,5) — PENGAMAN, hanya aktif kalau LLM melebihi limit
        ↓
Tampil ke UI
```

Analogi konkret yang sudah Achmad alami langsung: pola `[OPTIONS: pilihan1 | pilihan2 | pilihan3]` di akhir setiap turn dashboard ini — itu instruksi-prompt ("maksimal 4 options") dengan kemungkinan hard-cap sebagai pengaman di sisi platform. Pola yang sama diusulkan untuk top-bar-recommendation.

### Flow Multi-Klik "Smart Rekomendasi" — Separator Koma+Spasi untuk Gabung di Composer, Bukan Format Output Sub-Agent

Klarifikasi flow lengkap dari Achmad:

```
Response LLM utama selesai
        ↓
Tampil sebagai tombol/chip "Smart Rekomendasi" (hasil tangkapan array, maks 5 item)
        ↓
User klik SATU item → teks item itu MUNCUL sebagai teks tambahan di toolbox/composer chat
        (BUKAN auto-send — konsisten prinsip editable yang sudah ditetapkan untuk BL-78)
        ↓
User klik item KEDUA (sebelum kirim) → teks baru digabung ke composer existing
        dengan separator ", " (koma+spasi) antar kalimat/klausa
        ↓
User bebas edit manual, baru kirim
```

**Ini menjawab tegas pertanyaan terbuka #3 sebelumnya**: separator koma+spasi berlaku untuk **top-bar-recommendation** (menggabungkan beberapa pilihan rekomendasi yang diklik berurutan di composer), BUKAN format output internal sub-agent Prompt Engineer BL-78 — dua konteks terpisah yang kebetulan sama-sama disebut Achmad di pesan yang sama, sudah diklarifikasi tidak tertukar.
