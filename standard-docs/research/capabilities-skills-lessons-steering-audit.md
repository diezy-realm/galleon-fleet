---
title: Capabilities, Skills, Lesson-Learned & Steering — Audit Implementasi vs Kompetitor
status: research
date: 2026-10-04
author: squad-lead (diskusi dengan Achmad)
depends_on: agent-subagent-permission-scoping-survey.md, agent-subagent-ship-squad-permission-scheme.md
---

# Capabilities, Skills, Lesson-Learned & Steering — Audit Implementasi vs Kompetitor

> **Konteks:** Owner (Achmad) menilai pengetahuan umum (skills, lesson-learned, steering, global steering)
> harus TERBAWA ketika Agent berpindah workspace — bukan sekadar file config yang ditulis di repo, tapi
> persisten di local storage. Secara eksplisit menyebut suka pola Kiro Crew karena ini. Dokumen ini
> mengaudit APA YANG SUDAH ADA di `galleon-fleet` dan membandingkan dengan pola kompetitor (Kiro Crew
> sebagai acuan utama, GitHub Copilot sebagai kontras arsitektur).
>
> **Status:** Riset murni. Tidak mengubah `decisions/` atau `dev-tasks/`. Temuan V10 (dua jalur delegasi
> paralel, dari dokumen sebelumnya) dicatat ulang di §4 sebagai titik riset, BUKAN ditulis ke drift register
> — sesuai arahan eksplisit Owner.

---

## 1. Temuan Utama: Galleon Sudah Punya Fondasi Jauh Lebih Matang dari Perkiraan

Audit `crates/clawcrew-runtime/src/skills/` dan `crates/clawcrew-memory/` menemukan infrastruktur yang **sudah mengimplementasikan konsep inti** yang Owner sukai dari Kiro Crew — tapi tersebar dan belum didokumentasikan sebagai satu sistem koheren.

| Komponen | Lokasi kode | Kematangan |
|---|---|---|
| **Skill loading berlapis** (workspace → `open-skills` global di `home_dir()` → plugin WASM) | `crates/clawcrew-runtime/src/skills/mod.rs:563-760` (`load_skills`, `load_skills_for_agent`, `load_skills_with_open_skills_config`) | ✅ Sudah ada, mendukung per-agent skill bundle + shadowing audit (`ShadowedSkill`, `DroppedSkill`) |
| **Skill registry lengkap** (create/review/audit/cache/suggest/improve) | `crates/clawcrew-runtime/src/skills/{creator,review,audit,cache,suggestions,improver}.rs` | ✅ Sangat matang — `creator.rs` 71k baris, `mod.rs` 188k baris |
| **Memory lintas-scope dengan grant eksplisit antar-agent** | `crates/clawcrew-memory/src/agent_scoped.rs` (`AgentScopedMemory`, `AgentMemoryGrant`) | ✅ Sudah ada — agent bisa diberi akses baca memori agent lain via allowlist kategori, mirip semantik "Fleet/Ship/Squad/Crew" scope yang disebut `product/galleon-product-definition.md` |
| **Memory backend lengkap** (SQLite, Postgres, Qdrant, knowledge graph, consolidation, decay, dedup, hygiene) | `crates/clawcrew-memory/src/{sqlite,postgres,qdrant,knowledge_graph,consolidation,decay,dedup,hygiene}.rs` | ✅ Sangat matang — `sqlite.rs` 196k baris |
| **Lokasi penyimpanan di luar repo** (`install_root_dir()` = folder config, bukan folder project) | `crates/clawcrew-config/src/schema.rs:5090` | ✅ Sudah lokal-persisten, bukan file config di repo — **ini TEPAT prinsip yang Owner sukai** |

**Yang BELUM ada / belum ditemukan di audit ini (DIKOREKSI 2026-10-04, lihat §1a):**
- ~~Lesson-learned sebagai first-class concept di kode Rust/Go Galleon sendiri~~ — **KOREKSI: SALAH.** Audit lanjutan menemukan `engine/src/memory/proposal.go` (Go) sudah mengimplementasikan "Learning by Consent" penuh: `MemoryProposal` dengan lifecycle Pending→Approved/Rejected/Reverted, scope eksplisit `Fleet/Workspace/Project/Ship/Crew/Run`, versioned (auto-increment untuk rule+scope sama), endpoint `POST /api/v1/learning/proposals` dengan test `TestLearningGovernance_Lifecycle` lulus. Lihat §1a untuk detail.
- **Steering file loader eksplisit** untuk produk Galleon sendiri (`galleon`/`galleoncode` CLI) — **masih benar belum ditemukan**, bukan salah asumsi. `.kiro/steering/` yang ada di root repo ini adalah steering **KiroCrew-dashboard** (platform yang menjalankan sesi kerja ini), BUKAN steering produk Galleon untuk end-user. Dua hal berbeda, jangan tercampur. Lihat §1b.
- **Vector store produksi di Go hanya in-memory placeholder** (temuan baru, bukan dari audit awal) — lihat §1c.

### 1a. Lesson-Learned di Go — Sudah Matang, Scope-nya Persis Fleet/Ship/Squad/Crew/Run

`engine/src/memory/proposal.go` adalah jawaban definitif untuk "di mana logic lesson-learned harus ditambahkan: Rust atau Go?" — **jawabannya Go, dan sudah ada di sana.**

```go
// Komentar asli di kode:
// Learning-by-consent (docs/finalize/01, fundamentals): a correction becomes a
// rule only AFTER the Pirate King approves it — never auto-written.
// MemoryProposal ... scoped (Fleet/Workspace/Project/Ship/Crew/Run), versioned, and reversible
```

Ini **arsitektur yang benar**: lesson/learning adalah keputusan KEBIJAKAN (apa yang dipercaya sistem, di scope mana ia berlaku) — bukan eksekusi teknis. Go = otak orkestrasi + policy SSOT, jadi Go adalah tempat yang tepat. Rust tidak perlu (dan tidak boleh) punya logic ini sendiri.

**Celah yang masih ada:** `ProposalStore.Approve()` menulis ke `VectorStore` yang di-inject — tapi implementasi default (`engine/src/memory/store.go`, `NewVectorStore()`) adalah `inMemoryVectorStore`: **hilang saat proses Go restart.** Vector store PRODUKSI (SQLite/Qdrant/Postgres persisten) ada di **Rust** (`crates/clawcrew-memory/src/{sqlite,qdrant,postgres}.rs`), bukan di Go. Ini berarti ada DUA implementasi `VectorStore` yang terpisah tanpa jalur penyatuan jelas — mirip pola V8 (tool contract Go↔Rust) tapi untuk memory. Pertanyaan arsitektur terbuka: apakah `ProposalStore` Go seharusnya memanggil Rust via gRPC untuk persist ke vector-store produksi, konsisten dengan boundary ADR-0001 (Go=kebijakan, Rust=storage/eksekusi)?

### 1a-i. KONFIRMASI (diskusi 2026-10-04): Persistence Go Benar-Benar Nol, Bukan Asumsi

Verifikasi literal: `engine/src/memory/wire.go` menyambungkan `NewVectorStore()` (in-memory) langsung ke `NewService` tanpa opsi backend lain; grep `os.Open|os.Create|sql.Open|bolt.|badger.` di seluruh `engine/src/memory/*.go` = **nol hasil**. `inMemoryVectorStore` murni `map[string]*Document` di heap Go — restart proses engine menghapus SEMUA proposal yang sudah di-approve. Fitur "Learning by Consent" (invariant produk F3-3) **secara praktis tidak persistent sama sekali** di kondisi kode saat ini.

Proto `SystemGateway` (`proto/agent_service.proto`) hanya punya 3 RPC: `ExecuteTurn`, `ExecuteNativeTool`, `GetDecryptedSecret` — **tidak ada RPC untuk memory/vector/skill apa pun.** Go `engine/src/memory/` dan Rust `clawcrew-memory` adalah dua sistem yang hidup berdampingan TANPA jalur sambung sama sekali saat ini.

### 1a-ii. Rekomendasi: Pisahkan Kebijakan (Go) dari Storage (Rust) — Jangan Pindahkan Semua Logic

Owner lebih suka lesson-learned persistent dan sejalan dengan skills/steering (Rust). **Setuju untuk bagian STORAGE, tidak untuk bagian KEPUTUSAN KONSENSUS** — dua hal ini tercampur di `proposal.go` saat ini dan perlu dipisah:

| Bagian | Tetap di mana | Alasan |
|---|---|---|
| **Kebijakan/konsensus** — siapa approve, scope mana berlaku (Fleet/Ship/Squad/Crew/Run), aturan status lifecycle | **Go** | Ini domain organisasi, sudah SSOT di `fleet/policies.go`. Memindahkan ke Rust akan mengulang pola kesalahan yang sedang diperbaiki Phase-5 R-00 (`clawcrew-runtime` yang salah tempat memegang agent-orchestration-logic) |
| **Storage & retrieval vector** — tulis/baca embedding, SQLite/Qdrant/Postgres | **Rust** | `clawcrew-memory` sudah matang untuk ini; kerja I/O berat + native lebih cocok System Core, bukan duplikasi di Go |
| **Jembatan** | gRPC `SystemGateway`, tambah RPC baru (pola sama seperti `ExecuteTurn`) | Pola sudah terbukti jalan, jangan buat protokol baru |

**Pola konkret yang diusulkan** (generalisasi ADR-0001, bukan arsitektur baru):
```
Go ProposalStore.Approve()
   → tetap di Go: validasi status, scope, versioning, siapa approve
   → PANGGIL Rust via gRPC baru (mis. "MemoryGateway.StoreLearnedRule") untuk PERSIST
Rust clawcrew-memory
   → terima rule+scope dari Go, simpan ke backend aktif (SQLite/Qdrant/Postgres)
   → retrieval juga lewat Rust (embedding/vector-search sudah ada)
```

### 1a-iii. Hierarki Go vs Rust yang Relevan — Ditarik dari Pola yang Sudah Ada, Bukan Preferensi Baru

Berdasarkan 3 bukti konkret yang sudah established di repo ini (ADR-0001, `galleon-architecture.md` boundary, `fleet/policies.go` SSOT):

**Go = kebijakan/keputusan/hierarki organisasi. Rust = eksekusi/storage/retrieval teknis.** Ini bukan usul baru — ini penerapan konsisten dari batas yang SUDAH diputuskan untuk LLM (ADR-0001) dan tool execution (`ExecuteNativeTool`), diterapkan sama untuk memory/lesson/skill/steering supaya seluruh sistem punya SATU pola boundary, bukan pola berbeda-beda per subsistem.

### 1b. Steering — Dua Sistem Berbeda, Jangan Tercampur

`.kiro/steering/*.md` yang ada SEKARANG di root `galleon-fleet` (`galleon-architecture.md`, `galleon-product-fundamentals.md`, dll — terkonfirmasi masih ada, terakhir diupdate 2026-10-04) adalah steering untuk **KiroCrew-dashboard** (platform yang menjalankan sesi kerja Anda dengan saya) — BUKAN steering untuk produk Galleon sendiri (CLI `galleon`/`galleoncode` yang dipakai end-user Galleon).

Dua sistem yang mudah tertukar:
1. **KiroCrew-dashboard steering** (`galleon-fleet/.kiro/steering/`) — workspace-scoped, repo-tracked, dibaca saat agent (saya) bekerja DI REPO INI. Ini yang Anda generate manual sebelumnya; masih aktif dan terupdate.
2. **Steering produk Galleon sendiri** (untuk end-user CLI) — **memang belum ditemukan implementasinya** di `crates/` manapun. Ini bukan salah tempat cari — kemungkinan besar memang belum dibangun. Scope saat ini baru sebatas `standard-docs/` (aturan proyek, repo-tracked, untuk tim developer Galleon), BUKAN pengetahuan lintas-proyek untuk Agent/Crew Galleon sendiri yang dipakai end-user (meniru `~/.kiro/steering/` Kiro Crew).

**Ini gap produk nyata**, bukan kekurangan audit saya. Perlu jadi item roadmap tersendiri kalau Owner mau Galleon punya steering lintas-workspace untuk end-user-nya sendiri.

### 1b-i. GAP RESEARCH RESMI (dikonfirmasi 2026-10-04): Steering Produk Galleon Belum Ada

Owner mengonfirmasi fokus produk Galleon adalah ini — dicatat sebagai gap riset resmi, bukan sekadar observasi:

> **GAP-01 — Steering/global-steering end-user Galleon belum terimplementasi.** Produk Galleon (CLI `galleon`/`galleoncode`) saat ini TIDAK punya mekanisme setara `~/.kiro/steering/` Kiro Crew (persistent, lintas-workspace, workspace-overrides-global). `standard-docs/` yang ada hanya untuk tim developer Galleon sendiri (repo-tracked, aturan proyek) — bukan untuk end-user Galleon yang memakai produk jadi. Kalau produk Galleon ingin replikasi pengalaman "Agent ingat preferensi saya lintas-proyek" yang disukai Owner dari Kiro Crew, ini perlu dirancang dari nol: lokasi fisik (`~/.config/galleon/steering/`? — konsisten `install_root_dir()` yang sudah ada), format file, aturan resolusi konflik (workspace vs global), dan SIAPA yang membaca steering ini (Rust `galleoncode` CLI langsung, atau lewat Go orchestrator untuk konsistensi dengan hierarki §1a-iii: Go pegang kebijakan "steering mana yang aktif", Rust eksekusi baca-file).

### 1c. Vector Store Fleksibel (Local + Cloud) — Sudah Ada di Rust, Terpecah dari Go

Dikonfirmasi: `crates/clawcrew-memory` sudah mendukung penuh fleksibilitas storage yang disebut Owner — SQLite (lokal), Postgres (server/cloud), Qdrant (vector performa tinggi), plus `knowledge_graph.rs`/`knowledge_graph_pg.rs` (graph-based retrieval). Backend ini **matang dan lengkap**, bukan sekadar skeleton.

**Masalahnya bukan ketersediaan fitur, tapi jalur integrasi:** Go `engine/src/memory/service.go` punya `VectorStore` interface sendiri yang **tidak** terhubung ke backend Rust manapun — implementasi defaultnya cuma in-memory (§1a). Ini perlu diputuskan sebagai keputusan arsitektur formal (level ADR), bukan dibiarkan ambigu: apakah Go WAJIB panggil Rust via gRPC untuk semua operasi vector-store (konsisten pola `ExecuteTurn`/`ExecuteNativeTool`), atau Go akan punya driver native sendiri untuk sebagian backend (duplikasi yang disengaja dengan alasan performa/latency)?

---

## 2. Perbandingan Konkret dengan Kompetitor

### 2.1 Kiro Crew — Memory 6 Lapis (acuan utama, sesuai preferensi Owner)

Sumber: `kiro.dev/docs/crew/features/memory/`.

Enam lapis memory, **masing-masing punya tujuan, lokasi fisik, dan aturan hidup/mati berbeda** — bukan satu blob memori generik:

| Lapis | Lokasi fisik (lokal) | Isi | Siklus hidup |
|---|---|---|---|
| **Preferences** | `~/.kiro/crew/workspace/memory/preferences.md` | Kebiasaan kerja user | Diganti UTUH tiap 30 pesan (bukan append-only), cap 4.250 char |
| **Projects** | `~/.kiro/crew/workspace/memory/projects.md` | Konteks kerja aktif (CR, package, branch) | Siklus sama dengan preferences |
| **Recent history** | `~/.kiro/crew/workspace/memory/history/` | Ringkasan harian dengan **tiered decay** | 0-13 hari penuh → 14-60 hari ringkas → 61-180 hari tanggal saja → 181-364 hari tidak di-load (tetap di disk) → 365+ hari dihapus |
| **Semantic memory** | SQLite `semantic_memory` table + FAISS index opsional | Fakta key-value terstruktur (`pref.*`, `project.*`, `user.*`) | Selalu aktif; **confidence-gating** — tulisan LLM butuh confidence ≥0.8, tulisan eksplisit user selalu menang |
| **Episodic memory** | SQLite `episodic_memories` table + FAISS | Potongan kejadian masa lalu yang bisa dicari | Dedup via cosine similarity >0.88; max 10.000 entri, importance-based pruning |
| **Lessons** | Semantic entries `lesson.<md5hash>` (confidence 1.0) | **Koreksi yang diajarkan user, override default behavior** | Dedup via substring+topic overlap (>50% keyword→replace); cap 50 lesson, 37.250 char |

**Poin kunci yang relevan untuk Galleon:** lesson BUKAN kategori memori generik — ia punya mekanisme pembuatan spesifik (`learn_add` tool ATAU terdeteksi otomatis dari pola koreksi dalam percakapan), confidence tertinggi (1.0, selalu menang), dan di-inject sebagai blok terpisah `[Learned corrections]` — bukan dicampur dengan fakta biasa. Ini **persis** pola yang dipakai sesi ini sendiri (lihat `[Learned corrections]` di konteks percakapan Anda).

### 2.2 Kiro Crew — Steering: Workspace vs Global, Resolusi Konflik Eksplisit

Sumber: `kiro.dev/docs/steering/`.

| Scope | Lokasi | Berlaku untuk | Prioritas saat konflik |
|---|---|---|---|
| **Workspace steering** | `.kiro/steering/` (di root project) | Hanya workspace itu | **Menang** saat bertentangan dengan global |
| **Global steering** | `~/.kiro/steering/` (home directory) | SEMUA workspace | Kalah dari workspace steering, tapi tetap jadi default di workspace baru |
| **Team steering** | Variasi dari Global — didorong via MDM/Group Policy atau di-download dari repo pusat ke `~/.kiro/steering/` | Seluruh tim | Sama seperti global |

**File steering standar** yang Kiro sediakan sebagai starting point: `product.md` (tujuan produk), `tech.md` (stack teknis), `structure.md` (organisasi file/konvensi) — **ini mirip 1:1 dengan apa yang `standard-docs/foundation/` dan `tech-standart/` di Galleon coba capai**, hanya saja Galleon menaruhnya di repo (`standard-docs/`) bukan di `~/.kiro/steering/` (home directory lokal).

**Perbedaan arsitektur penting untuk didiskusikan:** steering Kiro **default-nya bukan git-tracked** — ia hidup di home directory, terpisah dari repo. Galleon's `standard-docs/` justru **git-tracked di repo**. Ini bukan salah/benar, tapi dua model berbeda:
- Model Kiro: pengetahuan "cara saya suka kerja" (preference personal) terpisah dari "aturan proyek ini" (repo-tracked `AGENTS.md`/`.kiro/steering` project-level yang MEMANG di-commit).
- Model Galleon saat ini: `standard-docs/` di repo = aturan proyek (benar, ini memang harus di-commit supaya tim lain lihat). Tapi **pertanyaan Owner adalah soal lapis yang BERBEDA** — bukan aturan proyek, tapi pengetahuan Agent/Crew individual yang terbawa LINTAS workspace/proyek (preference, lesson, skill yang dipelajari dari pengalaman).

### 2.3 Kiro Crew — Sub-Agent: Apa yang Diwarisi vs Diisolasi

Sumber: `kiro.dev/docs/custom-agents/subagents/`.

| Diwariskan dari parent (SHARED) | Diisolasi per sub-agent |
|---|---|
| Steering files | Riwayat percakapan |
| MCP servers | Context window |
| Akses file workspace | State Spec |
| **Konfigurasi permission** | Trigger hook |

**Ini jawaban langsung untuk desain Galleon** (dibahas di dokumen sebelumnya §2 — dua jalur delegasi paralel): pola yang BENAR bukan "sub-agent dapat scope baru dari nol" atau "sub-agent mewarisi semua tanpa batas" — tapi **pemisahan jelas antara apa yang dibagi (steering, MCP, permission config) vs apa yang diisolasi (context, history, state)**. `DelegateTaskTool` Galleon sudah benar untuk sisi "permission ceiling diwariskan" — yang belum ada eksplisit adalah "steering/skill apa yang ikut diwariskan ke sub-agent" karena memang belum ada steering-loader yang teraudit di `crates/`.

### 2.4 GitHub Copilot — Kontras Arsitektur (bukan untuk ditiru, untuk dihindari)

Sumber: `docs.github.com/en/copilot/concepts/agents/copilot-memory`, startdebugging.net.

**Copilot Memory disimpan di SERVER GitHub, scoped per-repository** — bukan lokal. Dikelola lewat *Repository → Settings → Code & automation → Copilot → Memory*, bukan file di disk. "Memories created by one part of Copilot can be used by another part" — tapi semuanya hidup di infrastruktur GitHub, pengguna tidak punya salinan lokal yang portable.

Ada **TIGA sistem terpisah yang gampang disalah-pahami sebagai satu**: Copilot Memory (server-side, per-repo), Repository Custom Instructions (`.github/copilot-instructions.md`, repo-tracked), dan `AGENTS.md` (konvensi lintas-tool, repo-tracked) — ketiganya punya precedence berbeda dan tidak semua agent membaca ketiganya secara konsisten (`startdebugging.net`: "Copilot's choice between conflicting instructions is non-deterministic" kalau ada overlap).

**Ini justru mengkonfirmasi alasan Owner menyukai pola Kiro Crew**: Copilot Memory TIDAK portable (terkunci di akun GitHub + repo tertentu), tidak transparan (user tidak bisa baca file mentahnya), dan punya ambiguitas resolusi konflik. Kiro Crew sebaliknya: file markdown biasa di disk lokal, bisa dibaca/diedit manual, resolusi konflik dijelaskan eksplisit (workspace menang atas global).

---

## 3. KESIMPULAN FINAL (dikonfirmasi Achmad, 2026-10-04): Split Architecture

> **Keputusan:** Kebijakan/konsensus/hierarki organisasi (Fleet→Ship→Squad→Crew, siapa approve, scope mana berlaku) tetap di **Go**. Storage & eksekusi teknis (skill parsing, vector embedding, baca/tulis SQLite/Qdrant/Postgres) tetap di **Rust** — TIDAK dipindah ke Go. Keduanya dihubungkan via gRPC `SystemGateway`, mengikuti pola yang sudah terbukti di ADR-0001 (`ExecuteTurn`)/`ExecuteNativeTool`.

Ini **bukan** "pindahkan logic skills/steering ke Go" — justru kebalikannya: Rust **tetap** memegang bagian yang sudah matang (188k baris `skills/mod.rs`, `clawcrew-memory` dengan SQLite/Qdrant/Postgres), karena membangun ulang itu di Go akan membuang kerja yang sudah berfungsi dan melanggar prinsip "reuse > tambah". Yang perlu dibangun BARU hanyalah **jembatan gRPC** antara `ProposalStore` (Go, kebijakan) dan `clawcrew-memory` (Rust, storage) — bukan arsitektur baru dari nol.

Ringkasan tabel kebutuhan vs status (final, pasca-keputusan):

| Kebutuhan Owner | Status | Keputusan |
|---|---|---|
| Skill terbawa lintas-workspace | ✅ Ada (`load_skills_with_open_skills_config`, Rust) | **TETAP di Rust** — matang, jangan dipindah |
| Lesson-learned persisten lokal | ⚠️ Logic kebijakan ada di Go (`proposal.go`), tapi storage cuma in-memory | **Kebijakan tetap Go, storage pindah ke Rust via gRPC baru** (lihat BL-01 di `backlog/`) |
| Global steering lintas-workspace (produk Galleon) | ❌ Belum ada sama sekali (GAP-01) | **Perlu dibangun baru** — pembagian: Go pegang "steering mana aktif" (kebijakan), Rust eksekusi baca-file (konsisten §1a-iii) |
| Resolusi konflik workspace vs global | ❌ Belum ada (turunan dari GAP-01) | Rancang bersamaan dengan GAP-01, pola workspace-menang seperti Kiro Crew §2.2 |
| Sub-agent mewarisi pengetahuan umum | ⚠️ Permission ceiling sudah benar (`DelegateTaskTool`), skill/steering inheritance belum eksplisit | Perlu test baru, lihat BL-04 di `backlog/` |

Task breakdown konkret dari kesimpulan ini ada di `standard-docs/backlog/` (lihat §5 revisi di bawah).

---

## 4. Catatan Riset (BUKAN Drift Register) — V10 dari Dokumen Sebelumnya

Sesuai arahan eksplisit Owner ("catat di point research saja, agar kedepan kita bisa buatkan task breakdown dari itu"), temuan berikut **TETAP di level riset**, belum ditulis ke `foundation/04-drift-register-and-hygiene.md`:

> **Dua jalur delegasi sub-agent paralel** — `DelegateTaskTool` (`engine/src/tool/delegation.go`, punya ceiling-check teruji) vs `spawn_subagent` (`engine/src/crew/dispatcher.go`, tanpa ceiling-check). Lihat `agent-subagent-ship-squad-permission-scheme.md` §2 untuk detail lengkap. **Keputusan menunggu task-breakdown terpisah**, tidak dieksekusi dari dokumen riset ini.

---

## 5. Task-Breakdown — Dipindah ke `standard-docs/backlog/`

Rekomendasi di versi riset sebelumnya sudah dikonkretkan jadi task-breakdown bernomor di `standard-docs/backlog/skills-steering-lessons-backlog.md` (BL-01 s.d. BL-05), mengikuti keputusan split architecture di §3. Dokumen ini tetap jadi rujukan ANALISIS; `backlog/` adalah rujukan EKSEKUSI.

---

## 6. Sumber

- Kode diaudit: `crates/clawcrew-runtime/src/skills/mod.rs` (`load_skills_for_agent_audited`, `load_skills_with_open_skills_config`), `crates/clawcrew-memory/src/agent_scoped.rs` (`AgentScopedMemory`, `AgentMemoryGrant`), `crates/clawcrew-config/src/schema.rs:5090` (`install_root_dir`)
- Kiro Crew Memory: https://kiro.dev/docs/crew/features/memory/
- Kiro Steering: https://kiro.dev/docs/steering/
- Kiro Sub-Agents (apa yang diwariskan): https://kiro.dev/docs/custom-agents/subagents/
- Kiro Agent Skills: https://kiro.dev/docs/skills/, https://kiro.dev/docs/crew/capabilities/skills/
- Kiro Configuration Scopes (`KIRO_HOME`): https://kiro.dev/docs/configuration/
- GitHub Copilot Memory (kontras arsitektur): https://docs.github.com/en/copilot/concepts/agents/copilot-memory
- Copilot Memory vs Custom Instructions vs AGENTS.md: https://startdebugging.net/2026/09/copilot-memory-vs-repository-custom-instructions-vs-agents-md/
- Dokumen terkait: `agent-subagent-permission-scoping-survey.md`, `agent-subagent-ship-squad-permission-scheme.md`
