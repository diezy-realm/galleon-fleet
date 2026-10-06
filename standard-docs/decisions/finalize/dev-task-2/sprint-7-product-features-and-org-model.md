---
sprint: 7
title: Product Features (FE Organization/Approval UI) & Payment-Lockout Preventif
status: planned
depends_on: Sprint 4 §E (BG-F1/F3 org model), Sprint 1 §A (BG-B4 untuk FW-C5)
---

# Sprint 7 — Product Features & Payment-Lockout Preventif

## A. Fitur Produk FE (dep org model Sprint 4)

- [ ] **FW-C1 · U-13 · PR-72 (P2, L)** Layar organisasi: Quartermaster/Ship/Squad/Crew, memori, approval, audit/policy read-only. Dep: BG-F1, BG-F3 (Sprint 4).
- [ ] **FW-C2 · U-14 · PR-98 (P3, M)** i18n & aksesibilitas dasar.
- [ ] **FW-C3 · (ODR-07)** Terminal UI memakai `POST /api/terminal/exec` setelah `BG-A4` (sudah selesai) + `S2-SEC-01` (Sprint 0, approval-identity) — tampilkan prompt approval yang BENAR-BENAR terverifikasi identitas, bukan checkbox kosong.
- [ ] **FW-C4 · (G-20)** Halaman perangkat terhubung: daftar perangkat, revoke, QR pairing. Dep: `BG-C2` (Sprint 0).
- [ ] **FW-C5 · (G-24)** UI persetujuan `MemoryProposal` (learning by consent). Dep: `BG-F4` (Go, lihat §B di bawah).

## B. Learning by Consent & ADR Lanjutan

- [ ] **BG-F4 · G-24 · PR-96 (P2, L)** Learning by consent lanjutan: tipe `MemoryProposal` sudah ada (T-3.2 ✔); tinggal alur umpan balik → persetujuan Owner → versi/rollback di atas store baru. Dep: `BG-B4` (Sprint 1).
- [ ] **BG-F6 · G-37 · PR-99 (P3, S)** ADR opsional Delegated Authority Chain (rujukan `research/verifiable-intent-assessment.md` §3c — di-upgrade jadi gate WAJIB sebelum desain fitur commerce, bukan lagi opsional, per keputusan Owner 2026-10-05).

## C. Payment-Lockout Gate (Preventif — nol tool pembayaran eksis saat ini, pasang gate SEBELUM kapabilitas dibangun)

> Desain+riset sudah lengkap (`decisions/payment-lockout-gate-scheme.md`, diperkuat 4 temuan riset: AP2 v0.2, insiden Comet/Replit, OWASP Agentic ASI03/09/10). Ditunda Owner 2026-10-05 (fokus BL-32 dulu) — sekarang BL-32 sudah di Sprint 0, jadi sprint ini adalah titik eksekusi wajar.

- [ ] **BL-40 (MEDIUM-importance/LOW-urgency, tidak ada dependency)** `PaymentLockConfig` schema (termasuk `unlock_expires_at`) di `clawcrew-config`.
- [ ] **BL-41 (MEDIUM, SETELAH BL-40)** `PaymentLockManager` + titik gate di `tool_execution.rs` (termasuk expiry check).
- [ ] **BL-42 (MEDIUM-HIGH, paralel dengan BL-41)** Field `payment_category` di `ToolManifest` + fail-closed waktu registrasi.
- [ ] **BL-43 (MEDIUM, SETELAH BL-40/41)** UI Settings "Allow Agents to Make Payments" — **state mentah, bukan ringkasan agent** (OWASP ASI09: state harus dari sistem, bukan laporan agent yang bisa salah/menipu).
- [ ] **BL-44 (LOW-MEDIUM, paralel dengan BL-43)** Audit log perubahan mode Locked→Unlocked.

## D. Toolbox Parity — KiroCrew vs Galleon (dari `toolbox-parity-backlog.md`, audit 2026-10-05)

> `BL-73` (Quest-loop), `BL-78` (Optimize prompt), dan `BL-79` (Smart rekomendasi top-bar, baru) **TIDAK dimasukkan sebagai tiket `[ ]` aktif di sini** — ketiganya tetap EXPLORATORY di `backlog/toolbox-parity-backlog.md`, menunggu keputusan Owner, sesuai §Catatan Non-Negosiasi poin 4. Dicatat sebagai REFERENSI saja supaya tim tahu metodenya sudah dipertajam (2026-10-05) tanpa disalahartikan sudah siap eksekusi:
> - **BL-78**: metode sekarang "spawn sub-agent Prompt Engineer sebelum turn utama" (bukan rewrite sederhana), hasil editable tidak auto-send, kemungkinan reuse infrastruktur delegasi `clawcrew-runtime/src/subagent/mod.rs` (BL-05, §C di atas). Lihat `research/toolbox-parity-audit.md` §Penajaman Metode untuk detail flow lengkap.
> - **BL-79 (baru)**: top-bar "smart rekomendasi" — post-response tarik kesimpulan + array maks 5 pertanyaan lanjutan (hybrid instruksi-prompt + hard-cap), klik item → teks masuk composer (editable), klik item ke-2+ → gabung separator `", "`. Satu pertanyaan terbuka tersisa: siapa menjalankan tarik-kesimpulan (LLM utama vs sub-agent terpisah) — keputusan ini mempengaruhi apakah BL-78 dan BL-79 berbagi SATU mekanisme delegasi atau dua mekanisme terpisah.

- [ ] **BL-74 (LOW-MEDIUM, PALING MURAH, kerjakan duluan)** Hapus context-window placeholder hardcoded (`EngineRoomView.tsx:335` — teks statis "8,192 tokens... 68%" tidak terhubung sesi nyata) — ganti data real atau hapus.
- [ ] **BL-72 (MEDIUM, independen)** Sambungkan `ApprovalForm.tsx`/`fleetStore.ts::handleApproval` ke endpoint nyata (`POST /api/v1/tool-executions/{id}/approve|deny`, sudah ada di `approval_gate.go` — saat ini UI mock state lokal).
- [ ] **BL-71 (MEDIUM-HIGH, butuh keputusan desain dulu)** Toggle approval-mode per-chat (Normal/Reads/Trust/YOLO ala KiroCrew) terhubung ke `RiskTier`/`RiskClass` API yang sudah ada di `approval_gate.go`/`policy.go`. **Keputusan terbuka**: apakah mode ini dipetakan ke `authority: read_only|draft_only|gated_write` existing di `CrewMember`, atau butuh bentuk baru — jangan tiru UI KiroCrew mentah tanpa cek kesesuaian model.
- [ ] **BL-75 (MEDIUM, SETELAH BL-68 §B di bawah)** Tombol "Chat" di `CrewView.tsx`/`SquadsView.tsx`/`ShipsView.tsx` yang membuka sesi Journal terikat entitas.
- [ ] **BL-76 (MEDIUM-HIGH, SETELAH BL-62 §B di bawah)** Autocomplete `@` (mention `Project`) + `/` (slash skill) di composer `CaptainsJournalView.tsx`.
- [ ] **BL-77 (MEDIUM, MENYATU dengan BL-62/68/69 §B — jangan kerjakan terpisah)** Pindahkan switcher Workspace dari sidebar `QuestsView.tsx` ke bentuk benar: Workspace scoped Journal, Project jadi bar global `ContextBar.tsx` yang BENAR-BENAR dirender (saat ini hanya diimpor, tidak pernah muncul di JSX).

## E. UI Nice-to-Have (dari `ui-nice-to-have-backlog.md`, prasyarat §D di atas)

> BL-62/68/69 sudah dirancang lengkap di `backlog/ui-nice-to-have-backlog.md` dengan urutan eksekusi dan serialisasi file sendiri — rujuk file itu untuk detail. Diringkas di sini karena jadi dependency langsung §D:

- [ ] **BL-68 (MEDIUM-HIGH, PRIORITAS)** Hapus `Quest.workspaceId` (vestigial), scope `selectedWorkspace`/`Workspace[]` ke Captain's Journal saja, tambah `JournalSession.projectId?`.
- [ ] **BL-62 (MEDIUM-HIGH)** `Project` entity + Go endpoint `GET /api/project/git-status` — wajib lewat policy gate (`terminal_policy.go`), bukan `exec git` langsung dari Tauri.
- [ ] **BL-65/66/67 (lihat `ui-nice-to-have-backlog.md`)** Bugfix + grouping visual + tombol kontekstual Captain's Journal, SETELAH BL-68.
- [ ] **BL-69 (MEDIUM)** Dropdown "+ New" di `AppSidebar.tsx`, SETELAH BL-68.

## F. GAP-01 — Perancangan Steering Produk Galleon End-User (dari `skills-steering-lessons-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)

> Scope PERANCANGAN (ADR/design-spec), bukan implementasi langsung — keputusan one-way-door format file untuk end-user `galleon`/`galleoncode` CLI. Ditempatkan di sprint ini (bukan Sprint 1) karena menyentuh pengalaman end-user produk jadi, konsisten dengan tema Sprint 7 (fitur produk).

- [ ] **BL-02 (MEDIUM, perancangan dulu — implementasi menunggu desain disetujui Owner)** Rancang mekanisme steering produk Galleon end-user setara `~/.kiro/steering/` Kiro Crew (saat ini TIDAK ADA — `standard-docs/` hanya untuk tim developer, bukan end-user produk jadi). 4 keputusan yang perlu dijawab design-spec: (1) lokasi fisik file (`~/.config/galleon/steering/`?), (2) format (markdown bebas vs frontmatter terstruktur), (3) resolusi konflik (workspace-menang-atas-global, pola Kiro Crew), (4) siapa membaca — Go pegang keputusan "steering mana aktif" (kebijakan), Rust `galleoncode` CLI eksekusi baca+suntik ke prompt (I/O teknis). Output: design-spec baru di `standard-docs/decisions/`. File: `web/src/pages/settings/` (opsional, kalau butuh UI toggle) — murni Lead (dokumentasi) di tahap ini, belum ada file kode.

## Catatan File-Overlap Lintas-Sprint

`BL-41`/`BL-42` menyentuh `tool_execution.rs` yang SAMA dengan `BL-37`/`BL-38` (Sprint 2). **Pastikan Sprint 2 sudah merge sebelum Sprint 7 §C mulai** — kalau belum, koordinasikan dengan Lead untuk serialisasi tambahan di luar urutan sprint normal.

## Keluaran Sprint

- Layar organisasi FE hidup, menampilkan hierarki Fleet=Perusahaan/Ship=Departemen/Squad=Tim/Crew=individu secara nyata (bukan mock).
- Payment-lockout gate terpasang SEBELUM ada tool pembayaran nyata dibangun — mencegah kelas insiden Comet/Replit sebelum terjadi di Galleon.
- Context-window placeholder tidak lagi menampilkan data palsu; approval-queue UI tersambung endpoint nyata; Workspace/Project switcher muncul di bentuk yang benar (Journal-scoped + bar global), bukan tersembunyi/salah lokasi.
