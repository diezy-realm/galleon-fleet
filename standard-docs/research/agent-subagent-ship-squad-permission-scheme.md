---
title: Skema Permission Agent/Sub-Agent → Squad → Ship — Rancangan Konkret
status: research
date: 2026-10-04
author: squad-lead (diskusi dengan Achmad)
depends_on: agent-subagent-permission-scoping-survey.md
---

# Skema Permission Agent/Sub-Agent → Squad → Ship

> **Konteks:** Lanjutan `agent-subagent-permission-scoping-survey.md`. Owner (Achmad) meminta skema KONKRET
> untuk hierarki Galleon (Quartermaster → Ship → Squad → Crew/Agent → Sub-Agent), bukan sekadar perbandingan
> kompetitor. Dokumen ini memetakan rekomendasi riset ke **kode nyata yang sudah ada**, bukan rancangan dari
> nol — temuan utama: Galleon sudah punya fondasi lebih matang dari perkiraan awal.
>
> **Status:** Riset + proposal arsitektur. Belum keputusan Owner final. Tidak mengubah `dev-tasks/`.

---

## 1. Temuan Kunci: Fondasi Sudah Ada, Lebih Matang dari Perkiraan

Audit `engine/src/tool/` menemukan infrastruktur yang **sudah mengimplementasikan sebagian besar** pola capability-based yang direkomendasikan riset kompetitor:

| Elemen | Lokasi kode | Status |
|---|---|---|
| `RiskTier` (READ/WRITE/EXECUTE) | `engine/src/tool/dto.go:9-14` | ✅ Ada |
| `RiskClass` granular (9 kelas: read, compute, network_read, write_draft, write_workspace, execute_sandboxed, execute_privileged, external_action, credential_access) | `dto.go:17-27` | ✅ Ada, lebih granular dari `RiskTier` |
| `ExecutionContext` dengan `ActorID`, `SessionID`, `WorkspaceID`, `CrewID`, `RunID`, `TaskID`, `AllowedRoots`, `Capabilities`, `DataClassification` | `dto.go:55-69` | ✅ Ada — sudah mendekati capability-based model Kiro Crew |
| `ApprovalGate` dengan CAS/TOCTOU protection, idempotency, expiry | `approval_gate.go` | ✅ Ada, cukup matang (bandingkan Claude Code `deny > ask > allow` — Galleon punya versi lebih kaya dengan hash-based CAS) |
| **Privilege-ceiling enforcement untuk sub-agent** (`DelegateTaskTool`) | `engine/src/tool/delegation.go:128-146` | ✅ **Ada dan sudah ditest** (`TestDelegationPolicyCeiling`) — child tidak bisa minta capability yang parent tidak punya |
| Audit trail (`PublishEvent` tiap approval) | `approval_gate.go:108-121` | ✅ Ada |

**Kesimpulan:** Galleon **tidak perlu membangun capability-system dari nol** — perlu **melengkapi field hierarki** (`SquadID`, `ShipID` belum ada di `ExecutionContext`) dan **menyatukan dua jalur delegasi yang saat ini paralel** (lihat §2).

---

## 2. Masalah Baru yang Ditemukan: Dua Jalur Delegasi Sub-Agent Paralel

Audit silang `engine/src/crew/dispatcher.go` vs `engine/src/tool/delegation.go` menemukan **duplikasi SSOT** yang belum tercatat di drift register manapun:

| Jalur | Lokasi | Ceiling-check? | Terdaftar sebagai tool untuk LLM? |
|---|---|---|---|
| **A — `DelegateTaskTool`** | `engine/src/tool/delegation.go` | ✅ Ya — `TestDelegationPolicyCeiling` membuktikan child tidak bisa eskalasi capability di luar punya parent | ✅ `services.go:265` `r.Register(&DelegateTaskTool{})` |
| **B — `spawn_subagent`** | `engine/src/crew/dispatcher.go` (`NewToolDispatcher`, tool ke-4 di `standardTools`) | ❌ **Tidak ada ceiling-check sama sekali** — langsung `Delegated task to subagent` tanpa cek `execCtx.Capabilities` | ✅ Terdaftar di `standardTools` yang disodorkan ke LLM |

**Risiko konkret:** kalau LLM memanggil `spawn_subagent` (Jalur B) bukan `delegate_task` (Jalur A), **privilege-ceiling TIDAK berlaku** — sub-agent bisa diberi task tanpa batasan capability apa pun diturunkan dari parent. Ini persis pola kegagalan yang disebut di riset §2.2 (Claude Code issue #89277: "restriksi di level prompt saja tidak cukup, harus dipaksa runtime") — bedanya di sini **dua runtime path hidup berdampingan**, satu aman satu tidak, dan LLM bebas pilih salah satu.

**Rekomendasi:** Satukan ke satu jalur (`DelegateTaskTool` sebagai SSOT, karena sudah punya ceiling-check teruji), hapus/alias `spawn_subagent` di `dispatcher.go` supaya tidak ada 2 cara mendelegasikan task dengan jaminan keamanan berbeda. Ini **temuan baru**, catat di `foundation/04-drift-register-and-hygiene.md` sebagai V10 (lihat §5).

---

## 3. Skema Permission yang Diusulkan — 4 Lapis Mengikuti Hierarki Produk

Mengikuti struktur `Quartermaster → Ship → Squad → Crew` dari `product/galleon-product-definition.md`, plus "Sub-Agent" sebagai turunan sementara dari Crew (tidak permanen di hierarki organisasi, tapi perlu scope sendiri saat delegasi task):

```
Fleet (root, 1 per instalasi)
 └─ Ship       = scope departemen     — ceiling TERLUAS yang bisa diberikan ke Squad di bawahnya
     └─ Squad  = scope tim            — ceiling diturunkan dari Ship, TIDAK BISA lebih luas dari Ship
         └─ Crew (Agent)  = scope individu agent — ceiling diturunkan dari Squad
             └─ Sub-Agent = scope delegasi sementara — ceiling diturunkan dari Crew, WAJIB subset
```

**Prinsip inti (dari riset §3, Kesimpulan 1-2): ceiling hanya bisa mengecil turun ke bawah, tidak pernah membesar.** Ini bukan fitur baru — ini **generalisasi** dari mekanisme `DelegateTaskTool` yang sudah ada (parent→child), diperluas agar berlaku di SETIAP lapis hierarki (Ship→Squad, Squad→Crew), bukan cuma Crew→Sub-Agent.

### 3.1 Perluasan `ExecutionContext` yang Diusulkan

```go
// Diusulkan: tambahan field di tool/dto.go, TIDAK mengubah field existing
type ExecutionContext struct {
    // ... field existing tetap (ActorID, SessionID, WorkspaceID, CrewID, RunID, TaskID, dst.) ...

    ShipID       string   `json:"ship_id,omitempty"`        // BARU — scope departemen
    SquadID      string   `json:"squad_id,omitempty"`       // BARU — scope tim
    CeilingCapabilities []string `json:"ceiling_capabilities,omitempty"` // BARU — union MAKSIMAL yang diwariskan ke child, berbeda dari Capabilities (yang dipakai utk tool call saat ini)
}
```

Kenapa `CeilingCapabilities` terpisah dari `Capabilities` yang sudah ada: `Capabilities` saat ini dipakai dua fungsi sekaligus (apa yang boleh dipakai SEKARANG, dan apa yang boleh diturunkan ke child) — ini membingungkan kalau sebuah Crew punya capability sempit untuk tugasnya sendiri tapi perlu mewariskan capability lebih luas (dalam batas Squad-nya) ke sub-agent yang didelegasikan tugas berbeda. Memisahkan dua konsep ini konsisten dengan `governance ceiling: Policy ∩ Profile` Kiro Crew — "profile" (capability operasional saat ini) berbeda dari "policy ceiling" (batas maksimal yang bisa diwariskan).

### 3.2 Aturan Resolusi Ceiling per Lapis

| Lapis | Siapa menetapkan ceiling | Darimana ceiling Squad/Crew/Sub-Agent berasal |
|---|---|---|
| Ship | `fleet/policies.go` (`GetFleetPolicies`) — dikonfigurasi Owner, bukan agent manapun | Ceiling tertinggi di seluruh fleet; mengikat semua Squad di bawahnya |
| Squad | Diwariskan dari Ship, HARUS `⊆` ceiling Ship. Squad TIDAK bisa minta tambahan di luar yang Ship miliki | `Ship.CeilingCapabilities ∩ (yang dikonfigurasi untuk Squad spesifik)` |
| Crew (Agent) | Diwariskan dari Squad, `⊆` ceiling Squad | `Squad.CeilingCapabilities ∩ (yang dikonfigurasi untuk Crew/role spesifik)` |
| Sub-Agent | Diwariskan dari Crew yang mendelegasikan, `⊆` capability Crew SAAT delegasi terjadi | **Sudah diimplementasikan** — persis logika `DelegateTaskTool.ExecuteWithContext` baris 128-146 |

**Catatan penting:** irisan (`∩`) ini **harus dihitung di Go** (titik tunggal — `fleet/policies.go` atau layer baru `fleet/ceiling.go`), **bukan di Rust gateway** dan **bukan di prompt/instruksi LLM**. Ini konsisten dengan governance ceiling Kiro Crew ("tidak bisa dilonggarkan oleh agent atau app") dan Gate G4 Galleon yang sudah diputuskan (Go = otak kebijakan, Rust = eksekutor).

### 3.3 Pemetaan ke `RiskClass` yang Sudah Ada

`RiskClass` yang sudah ada (9 kelas) bisa dipakai LANGSUNG sebagai basis `CeilingCapabilities` tanpa membuat taksonomi baru:

| RiskClass (sudah ada) | Cocok untuk ceiling default di lapis mana |
|---|---|
| `read`, `compute`, `network_read` | Default untuk Sub-Agent hasil delegasi — paling aman, cocok pola "child dapat subset read-only" dari riset §2.2 |
| `write_draft`, `write_workspace` | Default untuk Crew — agent individu biasanya perlu tulis ke workspace-nya |
| `execute_sandboxed` | Default untuk Squad — tim bisa eksekusi tool tersandbox (align `ALLOWED_TOOLS` Rust gateway) |
| `execute_privileged`, `external_action`, `credential_access` | **HANYA Ship, tidak pernah default** — ini yang disebut riset sebagai "governance ceiling tidak bisa dilonggarkan"; perlu approval Owner eksplisit (`ApprovalGate` sudah ada untuk ini) sebelum turun ke Squad manapun |

---

## 4. Checklist Verifikasi Sebelum Implementasi (Jangan Asumsikan Aman)

Mengikuti pelajaran paling tajam dari riset (insiden Antigravity §2.4 — "ada tool yang bypass gate yang sama"):

- [ ] **Audit SEMUA jalur tool call** di `engine/src/tool/` dan `crates/clawcrew-gateway/` — pastikan tidak ada tool "native"/builtin yang eksekusi tanpa lewat `ExecutionContext`/`ApprovalGate` yang sama. (Ini sudah jadi item di drift register V8, tapi scope-nya perlu diperluas dari "tool contract Go↔Rust" ke "semua jalur eksekusi", termasuk `dispatcher.go`'s `spawn_subagent` yang baru ditemukan di §2.)
- [ ] **Satukan `spawn_subagent` dan `delegate_task`** jadi satu jalur (§2) sebelum fitur delegasi sub-agent diperluas lebih jauh.
- [ ] **Tambah test "cross-ceiling"**: Squad A tidak bisa minta capability yang hanya dimiliki Squad B (horizontal), bukan cuma vertikal (parent-child) yang sudah ditest `TestDelegationPolicyCeiling`.
- [ ] **Verifikasi `AllowedRoots` benar-benar scoped ke workspace, bukan default kosong = akses penuh.** Baca `builtin_workspace.go:799-800` — fallback saat `AllowedRoots` kosong perlu dicek, jangan sampai default-nya malah longgar (konsisten temuan diskusi soal `fs:default` Tauri).
- [ ] **Rust gateway `ALLOWED_TOOLS` perlu tahu `RiskClass`, bukan cuma nama tool** — saat ini `grpc_system_gateway.rs:14` hanya cek nama (`bash`, `read_file`, dst.), tidak mengetahui apakah tool call tsb `execute_privileged` yang seharusnya butuh approval tambahan di layer Rust juga (defense-in-depth, bukan cuma Go yang mengecek).

---

## 5. Entri Baru untuk Drift Register (Diajukan, Belum Ditulis ke `foundation/04`)

Temuan di §2 (dua jalur delegasi paralel) adalah pelanggaran SSOT baru yang belum tercatat. Diusulkan sebagai:

> **V10 | Dua jalur delegasi sub-agent paralel, satu tanpa ceiling-check | `engine/src/crew/dispatcher.go` (`spawn_subagent`, tool ke-4 `standardTools`, TIDAK ada ceiling-check) vs `engine/src/tool/delegation.go` (`DelegateTaskTool`, PUNYA ceiling-check teruji `TestDelegationPolicyCeiling`) | SSOT: satu mekanisme delegasi, bukan dua | LLM bebas pilih jalur mana saja — kalau pilih `spawn_subagent`, privilege-ceiling tidak berlaku sama sekali. Aksi: satukan ke `DelegateTaskTool`, hapus/alias `spawn_subagent` di dispatcher.**

Belum ditulis ke drift register sesuai instruksi — tunggu konfirmasi Anda agar tidak tercampur dengan task yang sedang berjalan di `dev-tasks/`.

---

## 6. Ringkasan Rekomendasi (Actionable)

1. **Jangan bangun sistem permission baru dari nol** — perluas `ExecutionContext` yang sudah ada (`ShipID`, `SquadID`, `CeilingCapabilities`).
2. **Satukan dua jalur delegasi** (`DelegateTaskTool` vs `spawn_subagent`) — ini bug nyata yang baru ditemukan, bukan proposal lama.
3. Hitung irisan ceiling di Go (`fleet/`), bukan Rust atau prompt — selaras boundary yang sudah ada.
4. Pakai `RiskClass` yang sudah ada sebagai basis ceiling per lapis — jangan buat taksonomi baru.
5. Tambah test cross-ceiling (horizontal, Squad-ke-Squad) sebagai pelengkap test vertikal yang sudah ada.

---

## 7. Sumber

- Riset kompetitor: `research/agent-subagent-permission-scoping-survey.md`
- Kode diaudit: `engine/src/tool/dto.go`, `engine/src/tool/approval_gate.go`, `engine/src/tool/delegation.go`, `engine/src/tool/phase3_tool_test.go` (`TestDelegationPolicyCeiling`), `engine/src/crew/dispatcher.go`, `engine/src/fleet/interfaces.go`
- Hierarki produk: `product/galleon-product-definition.md`, `decisions/owner-decisions/ODR-2026-10-04-product-direction.md` (ODR-03)
