---
title: Backlog — Skills, Steering, Lessons & Sub-Agent Delegation
status: backlog
date: 2026-10-04
author: squad-lead
derived_from:
  - research/agent-subagent-permission-scoping-survey.md
  - research/agent-subagent-ship-squad-permission-scheme.md
  - research/capabilities-skills-lessons-steering-audit.md
---

# Backlog — Skills, Steering, Lessons & Sub-Agent Delegation

> **Tujuan folder ini:** menjembatani `standard-docs/research/` (analisis, belum keputusan eksekusi) ke
> task-breakdown yang BISA langsung dibagi ke squad-backend/squad-frontend. Setiap item di sini punya ID,
> keputusan arsitektur yang mendasarinya, scope, dan dependency — siap diangkat jadi tiket roadmap resmi
> setelah Lead Squad review.
>
> **Belum ada di `roadmap/` resmi** — folder ini adalah tahap antara riset→roadmap. Jangan dieksekusi
> langsung tanpa diangkat ke `roadmap/*.md` dulu (sumber kebenaran tiket tetap `roadmap/`, sesuai aturan
> `dev-tasks/README.md`).
>
> **TIDAK menyentuh `decisions/finalize/dev-tasks/`** — area itu masih dikerjakan tim lain di mesin lain.

---

## Prinsip Arsitektur yang Mendasari Semua Item (Jangan Dilanggar)

Dikonfirmasi Achmad 2026-10-04 (lihat `capabilities-skills-lessons-steering-audit.md` §3):

> **Kebijakan/konsensus/hierarki organisasi (Fleet→Ship→Squad→Crew) = Go. Storage & eksekusi teknis (skill parsing, vector embedding, baca/tulis SQLite/Qdrant/Postgres) = Rust.** Dihubungkan via gRPC `SystemGateway`, pola sama seperti `ExecuteTurn`/`ExecuteNativeTool` yang sudah ada. JANGAN pindahkan skills/vector-memory yang sudah matang di Rust ke Go — itu kebalikan dari keputusan ini.

---

## BL-01 — Jembatan gRPC: Persist Lesson/Learning dari Go ke Rust Vector Store

**Masalah:** `engine/src/memory/proposal.go` (Go) punya logic "Learning by Consent" lengkap (lifecycle, scope Fleet/Ship/Squad/Crew/Run, versioning) tapi menulis ke `inMemoryVectorStore` — hilang total saat restart proses. Vector store produksi (SQLite/Qdrant/Postgres) ada di Rust `clawcrew-memory`, tanpa jalur sambung.

**Keputusan arsitektur:** Go tetap pegang kebijakan (siapa approve, scope, versioning). Rust tetap pegang storage. Tambah RPC baru di proto, JANGAN bangun ulang vector-store di Go.

**Scope:**
- `proto/agent_service.proto`: tambah RPC baru di service `SystemGateway`, mis. `StoreLearnedRule(LearnedRuleRequest) returns (LearnedRuleResponse)` dan `RetrieveLearnedRules(RetrieveRequest) returns (stream LearnedRuleResult)`.
- Rust `crates/clawcrew-gateway/src/grpc_system_gateway.rs`: implementasi RPC baru, delegasikan ke `clawcrew-memory` yang sudah ada (reuse embedding/vector-search, JANGAN tulis ulang).
- Go `engine/src/memory/proposal.go`: `Approve()` dan `Revert()` panggil RPC baru via `client.SystemGatewayClient` (reuse pattern yang sama dengan `dispatcher.go`'s `ExecuteNativeTool`), BUKAN tulis ke `inMemoryVectorStore` lagi.
- Go `engine/src/memory/store.go`: `inMemoryVectorStore` boleh tetap ada untuk unit test, tapi wiring produksi (`wire.go`) harus pakai implementasi yang memanggil Rust gateway.

**File ownership:** `proto/agent_service.proto` (Backend-Rust lead, Backend-Go review), `crates/clawcrew-gateway/src/grpc_system_gateway.rs` (Backend-Rust), `engine/src/memory/{proposal,store,wire}.go` (Backend-Go). Serialkan: proto dulu, baru implementasi dua sisi paralel.

**Definition of Done:** `TestLearningGovernance_Lifecycle` (sudah ada) tetap hijau; tambah test baru yang memverifikasi `Approve()` lalu restart proses Go (simulasi) lalu `List()` masih mengembalikan rule yang di-approve (karena sekarang persisten di Rust).

**Dependency:** Tidak ada — bisa mulai langsung, proto change kecil (2 RPC tambahan, tidak mengubah 3 RPC existing).

**Risiko/Severity:** HIGH — ini bug nyata (fitur inti produk "learning by consent" tidak persistent), bukan sekadar technical debt kosmetik.

---

## BL-02 — GAP-01: Rancang Steering Produk Galleon (End-User, Bukan Dashboard KiroCrew)

**Masalah:** Produk Galleon (`galleon`/`galleoncode` CLI) tidak punya mekanisme setara `~/.kiro/steering/` Kiro Crew. `standard-docs/` di repo ini HANYA untuk tim developer Galleon, bukan untuk end-user produk jadi.

**Keputusan arsitektur yang perlu diambil (belum diputuskan, ini scoping task):**
1. Lokasi fisik: `~/.config/galleon/steering/` (konsisten `install_root_dir()` yang sudah ada di `clawcrew-config`)?
2. Format file: markdown bebas seperti Kiro, atau terstruktur (frontmatter + body)?
3. Resolusi konflik: workspace-menang-atas-global (pola Kiro Crew, sudah terbukti) — adopsi langsung atau ada pertimbangan lain?
4. **Siapa membaca:** sesuai prinsip arsitektur di atas — Go pegang KEPUTUSAN "steering mana yang aktif untuk sesi ini" (karena itu kebijakan/konteks organisasi), Rust `galleoncode` CLI EKSEKUSI baca file fisik dan suntik ke prompt (karena itu I/O teknis). Perlu RPC baru atau cukup Rust baca langsung dari disk tanpa lewat Go (karena ini CLI standalone, beda dari skills yang dipakai server/engine)?

**Scope:** Ini task PERANCANGAN dulu (bikin ADR/design-spec baru di `decisions/`), BUKAN implementasi langsung — karena melibatkan keputusan one-way-door (format file yang dipakai user, begitu dirilis sulit diubah tanpa migrasi).

**File ownership:** squad-lead menulis design-spec, lalu assign ke Backend-Rust (karena `galleoncode` CLI = Rust) untuk feedback implementability sebelum spec final.

**Definition of Done (untuk task perancangan ini):** Design-spec baru `standard-docs/decisions/` berisi 4 keputusan di atas + diagram alur baca-steering saat `galleoncode` start.

**Dependency:** Tidak ada untuk tahap desain. Implementasi menunggu desain disetujui Owner (karena ini mengubah pengalaman end-user produk jadi, bukan internal tooling).

**Risiko/Severity:** MEDIUM — bukan bug, tapi gap fitur yang disebut eksplisit sebagai preferensi Owner.

---

## BL-03 — Dokumentasikan Ekosistem `open-skills` yang Sudah Ada

**Masalah:** `crates/clawcrew-runtime/src/skills/mod.rs` punya skill-loading 3 lapis (workspace → `open-skills` global di home directory → plugin WASM) yang sudah matang dan BERFUNGSI, tapi tidak terdokumentasi di `standard-docs/` manapun. Risiko: tim developer baru/task breakdown mendatang menganggap ini belum ada dan membangun ulang (duplikasi SSOT).

**Scope:** Tulis `standard-docs/tech-standart/plugin-standard/open-skills-ecosystem.md` (atau tambahkan sub-bagian di `plugin-codebase-standard.md` yang sudah ada) yang menjelaskan: 3 lapis loading, `ShadowedSkill`/`DroppedSkill` audit trail, bagaimana `load_skills_for_agent` berbeda per Crew/agent alias.

**File ownership:** squad-lead (dokumentasi, bukan kode) — bisa delegasi ke Tech-Lead/Docs lane kalau squad itu aktif.

**Definition of Done:** Dokumen baru mengutip fungsi + baris kode nyata (seperti pola `tech-standart/` lain), bukan deskripsi generik.

**Dependency:** Tidak ada. Low-risk, bisa dikerjakan paralel dengan BL-01/BL-02.

**Risiko/Severity:** LOW — dokumentasi murni, tidak mengubah kode.

---

## BL-04 — Test Baru: Skill/Steering Inheritance untuk Sub-Agent

**Masalah:** `TestDelegationPolicyCeiling` (sudah ada) membuktikan permission ceiling diwariskan dengan benar ke sub-agent. Tapi belum ada test yang membuktikan skill/steering/pengetahuan umum JUGA diwariskan — padahal ini concern eksplisit Owner ("Agent sudah memiliki pengetahuan umum yang bisa dipakai" saat berpindah workspace/delegasi).

**Rujukan pola yang benar** (dari `agent-subagent-permission-scoping-survey.md` §2.3, Kiro Crew): sub-agent **mewarisi** steering files + MCP + permission config dari parent, tapi **isolasi** context/history/state. Pola ini harus diverifikasi juga berlaku di `DelegateTaskTool`.

**Scope:** Tambah test baru di `engine/src/tool/phase3_tool_test.go` (atau file test baru `delegation_knowledge_test.go`) yang memverifikasi: sub-agent yang dibuat `DelegateTaskTool` menerima skill-bundle yang sama dengan parent (bukan skill-set kosong atau skill-set berbeda tanpa alasan eksplisit).

**File ownership:** Backend-Go (test ada di `engine/src/tool/`), tapi mungkin perlu kerja sama Backend-Rust kalau skill-bundle resolution perlu dipanggil dari Go side.

**Definition of Done:** Test baru lulus, DAN kalau ternyata skill TIDAK diwariskan saat ini, dicatat sebagai bug baru (bukan asumsi default "pasti sudah benar").

**Dependency:** BL-01 tidak wajib selesai dulu, tapi akan lebih bermakna kalau lesson-persistence (BL-01) sudah jalan — supaya test ini juga bisa cek lesson diwariskan, bukan cuma skill.

**Risiko/Severity:** MEDIUM — ini verifikasi, bisa jadi menemukan bug baru atau mengonfirmasi sudah benar.

---

## BL-05 — V10: Satukan Dua Jalur Delegasi Sub-Agent Paralel

**Masalah (dari `agent-subagent-ship-squad-permission-scheme.md` §2):** Ada DUA jalur delegasi sub-agent — `DelegateTaskTool` (`engine/src/tool/delegation.go`, PUNYA ceiling-check teruji) vs `spawn_subagent` (`engine/src/crew/dispatcher.go`, TANPA ceiling-check). LLM bebas pilih salah satu; kalau pilih `spawn_subagent`, privilege-ceiling tidak berlaku sama sekali.

> **KOREKSI PENTING (2026-10-05, verifikasi lanjutan untuk cross-check BL-15/R-50):** Audit kode membuktikan kedua tool ini TIDAK simetris seperti anggapan awal — severity CRITICAL yang tercatat di bawah masih valid, tapi dengan nuansa baru:
> - **`spawn_subagent` (`dispatcher.go:216`) BENAR-BENAR mengeksekusi** — dia men-spawn goroutine Go (`s.runSubagent`, `services.go:235`) yang memanggil **`s.gateway.ExecuteTurn(...)` — gRPC call nyata ke Rust `SystemGateway`** dengan `SessionId: "subagent-<id>"`. Ini sub-agent yang BENAR-BENAR hidup dan memakan resource Rust (LLM call, tool loop, dst.) — dan karena cara masuknya TANPA ceiling-check, sub-agent ini juga TIDAK melalui gate resource apa pun di sisi Rust.
> - **`DelegateTaskTool`/`delegate_task` (`delegation.go:116-180`) TIDAK mengeksekusi apa pun** — fungsinya cuma validasi `RequestedCapabilities` lalu return JSON `{"status": "delegated", ...}`. Dia TIDAK memanggil `gateway.ExecuteTurn` atau apapun yang membuat sub-agent benar-benar jalan. Ceiling-check yang "teruji" (`TestDelegationPolicyCeiling`) menguji tool yang **tidak benar-benar melakukan apa yang namanya sarankan** — perlu klarifikasi ke Backend-Go/Principle apakah `delegate_task` ini memang dimaksudkan sebagai "approval/intent record" (lapis otorisasi SEBELUM eksekusi nyata di tempat lain) atau ini bug/kode belum selesai (fitur delegasi yang dibangun tapi belum disambung ke eksekusi).
>
> **Dampak ke BL-15/R-50 (gate resource di Rust `DelegateTool`):** Sub-agent yang di-spawn via jalur Go `spawn_subagent` MASUK ke Rust lewat `ExecuteTurn` (RPC berbeda dari `ExecuteNativeTool` yang dipakai tool-dispatch biasa), bukan lewat Rust `DelegateTool` (`tools/delegate/mod.rs`) yang jadi scope BL-15. **Ini berarti gate resource yang dirancang BL-15/R-50 (yang hanya menjaga titik masuk `DelegateTool`) TIDAK otomatis melindungi sub-agent yang masuk lewat `ExecuteTurn` langsung dari Go** — ini jalur masuk resource KETIGA yang terlewat dari audit `subagent-resource-management-audit.md` sebelumnya (yang hanya mengaudit `DelegateTool` Rust-side). Lihat BL-18 (baru, di `subagent-resource-management-backlog.md`) untuk follow-up.

**Keputusan arsitektur:** Satukan ke `DelegateTaskTool` sebagai SSOT (karena sudah teruji), hapus atau alias `spawn_subagent` di `dispatcher.go`'s `standardTools` supaya LLM hanya punya satu jalur delegasi.

**Scope:**
- `engine/src/crew/dispatcher.go`: hapus entri `spawn_subagent` dari `standardTools`, ATAU ubah jadi alias yang mendelegasikan ke `DelegateTaskTool` logic (supaya nama tool tetap familiar untuk LLM yang sudah dilatih memanggilnya, tapi lewat jalur aman).
- Pastikan tidak ada test existing yang bergantung pada `spawn_subagent` berperilaku tanpa ceiling-check (kalau ada, itu test yang salah dan perlu diperbaiki, bukan dipertahankan).

> **AC TAMBAHAN (2026-10-06, keputusan Owner — depth-limit, bukan hanya capability-ceiling):** Analisa pola KiroCrew (platform agen yang menjalankan sesi ini) menunjukkan rantai delegasi (Crew A→B→C→...) dibatasi DUA lapis: capability-ceiling (sudah ada di `delegate_task`) DAN depth-limit statis per tier (`max_delegation_depth` di `clawcrew-config/src/presets.rs:238`, preset `tight_runtime()`=1). `galleon-fleet` saat ini NOL depth-tracking di kedua jalur — gap ini belum tercakup scope BL-05 asli di atas.
>
> **Tambahan field:** `ExecutionContext` (`engine/src/tool/dto.go:55`) tambah `DelegationDepth int` (0 = top-level turn, increment per hop) dan `MaxDelegationDepth int` (diisi dari `entitlement.QuotaPolicy.MaxDelegationDepth()` saat context pertama dibentuk di entry-point — `tool/services.go:324`, `tool/delivery.go:154`).
>
> **Tambahan logic di `delegation.go` `ExecuteWithContext`:** setelah capability-ceiling check, sebelum return — hitung `nextDepth := execCtx.DelegationDepth + 1`, tolak (`CodePermissionDenied`) kalau `nextDepth > execCtx.MaxDelegationDepth`.
>
> **Dependency baru:** butuh `entitlement.QuotaPolicy.MaxDelegationDepth() *int` (lihat AC tambahan ENT-01/BL-80 di `tier-entitlement-pricing-backlog.md`) — nilai final Owner: Community=1, Team=2, Dockyard=3, Private Dockyard=custom (nil). **Juga depends pada eksekusi nyata subagent benar-benar membentuk `ExecutionContext` child** (lihat koreksi 2026-10-05 di atas: `spawn_subagent` sudah eksekusi nyata via `gateway.ExecuteTurn`, tapi path itu TANPA ceiling-check maupun depth-tracking; `delegate_task` punya ceiling-check tapi tidak eksekusi nyata — penyatuan jalur HARUS menghasilkan satu jalur yang punya ketiganya: eksekusi nyata + capability-ceiling + depth-limit).

**File ownership:** Backend-Go, `engine/src/crew/dispatcher.go` + `engine/src/tool/delegation.go` + `engine/src/tool/dto.go` (field baru) + `engine/src/tool/services.go`/`delivery.go` (isi `MaxDelegationDepth` dari `entitlement.QuotaPolicy` saat context dibentuk). Isolated change, tidak overlap file dengan BL-80/81/82 (entitlement/ sendiri ditulis BL-80).

**Definition of Done:** Hanya satu tool delegasi sub-agent terdaftar untuk LLM, dan jalur itu benar-benar eksekusi (`gateway.ExecuteTurn` atau penggantinya); `TestDelegationPolicyCeiling` tetap hijau; tambah test baru yang membuktikan (1) tool lama tidak lagi bisa dipanggil TANPA ceiling-check, DAN (2) rantai delegasi melebihi `MaxDelegationDepth` tertolak dengan error yang jelas (test tabel-driven per tier: Community gagal di hop ke-2, Team gagal di hop ke-3, dst).

**Dependency:** `entitlement.QuotaPolicy.MaxDelegationDepth()` (ENT-01/BL-80, AC tambahan 2026-10-06) untuk nilai per tier — selain itu independent, bisa duluan dari semua item lain di backlog ini.

> **LAMPIRAN — Trace lengkap jalur eksekusi nyata (2026-10-06, verifikasi file:baris langsung, bukan asumsi dari koreksi 2026-10-05 sebelumnya):**
>
> Jalur nyata: `dispatcher.go:216` (`Dispatch`, deteksi `call.Name == "spawn_subagent"`) → return `IsSubagentAction:true` → dikonsumsi di `crew/services.go:177` (`StartTurn`, loop event turn utama) → `services.go:187-189` spawn goroutine `subagentWg.Add(1); go func(){ s.runSubagent(...) }` → `runSubagent` (`services.go:235-264`) memanggil **`s.gateway.ExecuteTurn(ctx, &pb.ExecuteTurnRequest{SessionId: "subagent-<id>", Messages: [...]})`** — gRPC call nyata ke Rust `SystemGateway`, bukan simulasi. Nol ceiling-check di sepanjang jalur ini (dikonfirmasi: tidak ada pemanggilan `DelegateTaskTool`/capability-check apa pun antara `dispatcher.go` dan `runSubagent`).
>
> **Temuan baru yang mengubah pembacaan severity (belum tercatat di audit 2026-10-05):** `runSubagent`'s receive-loop (`services.go:248-263`) HANYA menangani `case *pb.TurnToken_Token:` (teks delta) — **tidak ada `case *pb.TurnToken_ToolCall:`**. Sub-agent yang di-spawn lewat jalur ini **tidak punya tool-access sama sekali**, sehingga **tidak bisa memanggil `spawn_subagent` lagi** — rantai delegasi rekursif (A→B→C) secara teknis TIDAK MUNGKIN terjadi hari ini, bukan karena ada gate yang mencegahnya, tapi karena tool-dispatch loop untuk sub-agent belum diimplementasikan.
>
> **Konsekuensi untuk AC depth-limit di atas:** depth-limit TETAP perlu dipasang sekarang (bersamaan BL-05) sebagai pencegahan — begitu `runSubagent` suatu saat diberi tool-access (kebutuhan wajar untuk sub-agent berguna), rantai rekursif langsung jadi mungkin tanpa gate. Lebih murah pasang sebelum kemampuan itu ada daripada menambal setelahnya. Severity CRITICAL BL-05 yang tercatat tetap valid, tapi bentuk eksploitasi hari ini adalah **"capability-ceiling hop-pertama dilewati total"** (sub-agent jalan bebas tanpa warisan batasan parent, dan tanpa gate resource Rust karena masuk lewat `ExecuteTurn` langsung, bukan lewat `DelegateTool` Rust yang jadi scope BL-15) — bukan "rantai tak terbatas", karena rantai itu belum mungkin secara teknis.
>
> **Implikasi langkah pertama eksekusi BL-05:** `DelegateTaskTool.ExecuteWithContext` (`delegation.go:116-180`) saat ini TIDAK TERHUBUNG ke `runSubagent` atau eksekusi apa pun — dia hanya `json.Marshal` lalu return. Menjadikannya SSOT (keputusan arsitektur yang sudah diputuskan) MENGHARUSKAN langkah eksplisit: sambungkan `ExecuteWithContext` ke `s.runSubagent` (atau penggantinya) SEBELUM capability-ceiling dan depth-limit yang sudah dirancang punya efek nyata — kalau tidak disambung, BL-05 hanya memindahkan "tool yang tidak eksekusi apa-apa" ke posisi SSOT tanpa benar-benar menutup celahnya.

**Risiko/Severity:** CRITICAL — ini celah keamanan nyata yang sudah teridentifikasi (privilege escalation path yang masih hidup), bukan technical debt kosmetik. Prioritaskan di atas BL-01..04.

---

## Urutan Eksekusi yang Disarankan

1. **BL-05** dulu (celah keamanan aktif, scope kecil, tidak ada dependency)
2. **BL-01** (bug fungsional nyata — fitur "learning by consent" tidak persistent)
3. **BL-03** (dokumentasi, low-risk, bisa paralel dengan 1-2)
4. **BL-04** (verifikasi, lebih bermakna setelah BL-01 selesai)
5. **BL-02** (perancangan GAP-01 — scope produk besar, perlu diskusi Owner lebih dulu sebelum masuk sprint)

---

## Catatan Proses

- Setiap item di sini perlu diangkat ke `roadmap/*.md` (lane R untuk Rust, G untuk Go, atau lane baru) sebelum di-assign ke squad — folder `backlog/` ini bukan pengganti `roadmap/`, hanya staging area riset→roadmap.
- Setelah diangkat ke roadmap dan dieksekusi, pindahkan status item ini (atau hapus dari `backlog/`) agar tidak dua tempat mencatat hal yang sama — selaras prinsip SSOT yang sama dipakai di seluruh `standard-docs/`.
