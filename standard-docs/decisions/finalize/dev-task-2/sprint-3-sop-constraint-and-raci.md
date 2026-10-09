---
sprint: 3
title: SOP Engine — Do's/Don'ts, RACI, Quest, Constraint Layer (Separation of Duties)
status: completed
depends_on: none (additive, risiko regresi rendah)
---

# Sprint 3 — SOP Engine: Do's/Don'ts, RACI, Constraint Layer

## A. Flow Layer — Do's/Don'ts, RACI, Role Template, Quest (existing design-spec)

- [x] **BL-06 (dari `sop-do-dont-raci-role-templates-design-spec.md`)** Field Do's/Don'ts terstruktur di `SopStep`. **Catatan generik:** contoh role (Backend/Frontend/QA) HARUS diperluas eksplisit mencakup minimal 1 contoh non-engineering (Marketing: Copywriter/Content Creator) — fundamental produk Galleon adalah AI Companion lintas departemen, bukan hanya engineering.

#### Detail Eksekusi — BL-06

**Verifikasi:** `crates/clawcrew-runtime/src/sop/types.rs:311` — `struct SopStep` sudah punya ~15 field (`number`, `title`, `body`, `suggested_tools`, `requires_confirmation`, `kind`, `schema`, `scope`, `routing`, `on_failure`, `mode`, `calls`, `pos`, `agent`, `capability`, `capability_input`, `policy`, `gate_prompt`, dll.) — **TIDAK ADA field Do's/Don'ts** di struct ini saat ini (dibaca penuh baris 300-390). Konfirmasi: tiket ini genuinely menambah field baru, bukan memperbaiki yang sudah ada.

**File & simbol spesifik:** Ubah `crates/clawcrew-runtime/src/sop/types.rs`, `struct SopStep` (baris 311).

**Spesifikasi API:**
```rust
pub struct SopStep {
    // ... field existing tidak diubah ...

    /// Structured guidance for the agent executing this step: things it
    /// SHOULD do and things it must NOT do. Rendered into the step prompt
    /// alongside `body`. Empty vecs = no additional guidance (backward compatible).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dos: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub donts: Vec<String>,
}
```
Pola `#[serde(default, skip_serializing_if = "Vec::is_empty")]` — IKUTI PERSIS pola field `calls: Vec<PlannedToolCall>` yang sudah ada di struct yang sama (baris ~352) untuk konsistensi serde behavior (backward compatible: SOP.md lama tanpa field ini tetap valid).

**Contoh pengkodean (format SOP.md, cek parser existing untuk field lain seperti `requires_confirmation`/`gate_prompt` sebagai referensi sintaks markdown-to-struct):**
```markdown
- do: Validasi input sebelum tulis file
- do: Gunakan context Project yang sudah di-load
- dont: Jangan hardcode credential di step body
```

**Langkah pengerjaan:**
1. Baca parser SOP.md (file yang mem-parsing markdown ke `SopStep` — kemungkinan di `crates/clawcrew-runtime/src/sop/parser.rs` atau sejenis, PERLU DICARI, belum diverifikasi sesi ini) untuk pola bullet existing (`- do:`/`- dont:` konsisten `- edit:`/`- prompt:` yang sudah dipakai `gate_prompt`).
2. Tambah field `dos`/`donts` ke `SopStep`.
3. Update parser untuk mengenali bullet `- do:`/`- dont:`.
4. **Perluas contoh role WAJIB mencakup minimal 1 non-engineering** — saat menulis dokumentasi/contoh SOP untuk tiket ini, sertakan contoh Marketing (Copywriter/Content Creator role) bukan hanya Backend/Frontend/QA.
5. Rendering ke prompt step: cari titik di mana `step.body` dirender ke prompt agent (kemungkinan `agent/turn/mod.rs` atau `sop/engine/mod.rs`), tambahkan `dos`/`donts` ke template prompt.

**Acceptance criteria testable:** SOP.md dengan bullet `- do:`/`- dont:` ter-parse ke `SopStep.dos`/`donts` dengan benar; SOP.md TANPA bullet ini tetap parse sukses (backward compat, vec kosong).

**Dependency:** BL-07 (RACI, di bawah) — serialkan di `sop/types.rs` kalau menyentuh struct yang sama, sudah dicatat di tabel File Ownership.

**JANGAN DIUBAH:** Field existing `SopStep` lainnya — hanya tambah 2 field baru.

**ASUMSI:** Lokasi file parser SOP.md belum diverifikasi — tim eksekusi WAJIB `search_symbols`/grep untuk fungsi parsing markdown-to-SopStep sebagai langkah pertama.

- [x] **BL-07** Field RACI lengkap di `Sop` struct.

#### Detail Eksekusi — BL-07

**Verifikasi:** `crates/clawcrew-runtime/src/sop/types.rs:458` — `struct Sop` punya field `name`, `description`, `version`, `priority`, `execution_mode`, `triggers`, `steps`, `cooldown_secs`, `max_concurrent`, `location`, `deterministic`, `admission_policy`, dll. — **TIDAK ADA field RACI** (Responsible/Accountable/Consulted/Informed) saat ini.

**Spesifikasi API:**
```rust
pub struct Sop {
    // ... field existing ...

    /// RACI assignment for this procedure: who is Responsible (executes),
    /// Accountable (owns outcome), Consulted (input before/during), Informed
    /// (notified after). Values are role/crew identifiers (free-form string,
    /// consistent with `SopStep.agent` alias format). Absent = no RACI
    /// tracking (backward compatible with existing SOP.md files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raci: Option<RaciAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct RaciAssignment {
    pub responsible: Vec<String>,
    pub accountable: String, // Accountable harus TUNGGAL (prinsip RACI klasik: satu accountable)
    #[serde(default)]
    pub consulted: Vec<String>,
    #[serde(default)]
    pub informed: Vec<String>,
}
```

**Konvensi penamaan:** `RaciAssignment` (PascalCase, pola `SopAdmissionPolicy`/`SopExecutionMode` existing), field `raci: Option<RaciAssignment>` di `Sop`.

**Langkah pengerjaan:**
1. Tambah struct `RaciAssignment` + field `raci` ke `Sop`.
2. Update parser SOP.md untuk section `[sop.raci]` (TOML-style, konsisten pola `[sop.approval]` yang sudah disebut di komentar `policy` field `SopStep`).
3. Validasi: `accountable` harus non-empty string (satu orang/role), bukan Vec — ini BEDA dari `responsible`/`consulted`/`informed` yang Vec. Tambahkan validasi ini di titik parse/save.

**Acceptance criteria testable:** SOP.md dengan `[sop.raci]` section ter-parse benar; SOP tanpa section ini tetap valid (`raci: None`).

**Edge case:** `accountable` kosong string — harus ditolak di validasi save, bukan diterima silently sebagai "tidak ada accountable" (melanggar prinsip RACI).

**Dependency:** BL-06 (file overlap `sop/types.rs`, serialkan).

**JANGAN DIUBAH:** Field existing `Sop` — hanya tambah 1 field opsional.

- [x] **BL-08** Template role per-Crew — **diperluas generik** (lihat catatan BL-06), bukan hanya Backend/Frontend/QA.

#### Detail Eksekusi — BL-08

**TIDAK DIVERIFIKASI sesi ini** — lokasi "template role per-Crew" existing (kalau ada) belum dicari. **Langkah pertama WAJIB:** grep/`search_symbols` untuk "RoleTemplate"/"CrewTemplate" untuk tahu apakah ini genuinely baru atau perluasan dari struct existing.

**Langkah pengerjaan:**
1. Cari struct/konsep "role template" existing (kemungkinan terkait `SopStep.agent` alias resolution, atau konsep terpisah di `engine/src/crew/`).
2. Perluas minimal 1 role non-engineering (Marketing: Copywriter/Content Creator) sebagai contoh konkret dalam template — BUKAN hanya menyebutnya di dokumentasi, tapi sebagai entry template nyata yang bisa dipakai.

**Dependency:** BL-06/BL-07 (tema sama, role-based SOP authoring).

**ASUMSI:** Scope tiket ini perlu diperjelas oleh Lead sebelum eksekusi — "template role" bisa berarti struct data baru, atau sekadar dokumentasi contoh SOP per role. Dokumen asli tidak cukup spesifik untuk saya tentukan sendiri tanpa menebak.

- [x] **BL-09** Matangkan `Quest` (Go) + `mission.Service` baru.

#### Detail Eksekusi — BL-09

**TIDAK DIVERIFIKASI sesi ini** — `Quest` (Go) disebut di README (`BG-D3` "Skills & scheduler (cron) terikat Squad/Crew" dan konteks dashboard Kiro Crew "Quest-loop" BL-73) tapi lokasi struct Go `Quest`/package `mission` belum dicari. **Langkah pertama WAJIB:** `grep -rn "type Quest" engine/src/` untuk lokasi struct existing (kalau ada) sebelum menulis `mission.Service` baru.

**ASUMSI:** "Matangkan" menyiratkan struct `Quest` SUDAH ada dalam bentuk minimal — verifikasi ini sebelum asumsi dari-nol.

## B. Constraint Layer — Separation of Duties (BARU, two-layer design)

> **Temuan riset kunci:** diagram arrow (BPMN/n8n) TIDAK BISA merepresentasikan larangan berbasis role — bahkan Camunda/jBPM (workflow engine enterprise) tidak native punya SoD. Solusi: Layer Flow (`SopStep`, existing, TIDAK diubah) + Layer Constraint (baru, `SopConstraint`) terpisah. Enforcement via hook `preToolUse` yang infrastrukturnya SUDAH ADA (`HookResult::Cancel`).

- [x] **BL-57 (LOW, additive, tidak ada risiko regresi)** Tipe `SopConstraint`/`ConstraintKind` di `sop::types`. Pola: `Sop.constraints: Vec<SopConstraint>` meniru `Sop.steps: Vec<SopStep>` yang sudah ada.

#### Detail Eksekusi — BL-57

**File & simbol spesifik:** Ubah `crates/clawcrew-runtime/src/sop/types.rs` — tambah struct baru DI DEKAT `struct Sop` (baris 458) dan `struct SopStep` (baris 311), ikuti pola `#[derive(Debug, Clone, Serialize, Deserialize)] #[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]` yang konsisten di SETIAP struct lain di file ini.

**Spesifikasi API:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SopConstraint {
    pub kind: ConstraintKind,
    /// Human-readable reason shown when the constraint blocks an action
    /// (surfaced via HookResult::Cancel message).
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstraintKind {
    /// A named role may never use the listed tool kinds, anywhere in this SOP run.
    RoleRestriction { role: String, forbidden_tools: Vec<String> },
    /// Two named steps must be executed by different performers (Dynamic SoD).
    DistinctPerformer { step_a: u32, step_b: u32 },
}
```
Tambah ke `Sop` struct: `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub constraints: Vec<SopConstraint>,` — field BARU, backward compatible.

**Langkah pengerjaan:**
1. Tambah `SopConstraint`/`ConstraintKind` enum ke `sop/types.rs` (ATAU file baru `sop/constraint.rs` sesuai tabel File Ownership dokumen — PILIH file baru `constraint.rs` untuk memisahkan concern dari `types.rs` yang sudah besar, re-export via `sop/mod.rs`).
2. Tambah field `constraints: Vec<SopConstraint>` ke `Sop`.
3. Update parser SOP.md untuk section `[[sop.constraints]]` (TOML array-of-table).

**Acceptance criteria testable:** SOP.md dengan `[[sop.constraints]]` ter-parse ke `Vec<SopConstraint>`; SOP tanpa section ini tetap valid (vec kosong, nol perilaku berubah — INI SEBABNYA severity LOW/additive).

**Dependency:** BL-58/BL-59 (enforcement) depend pada tipe ini.

**JANGAN DIUBAH:** `SopStep`/flow layer existing — constraint adalah layer TERPISAH, tidak menggantikan apa pun.

- [x] **BL-58 (MEDIUM, SETELAH BL-57)** Enforcement `RoleRestriction` via hook `preToolUse` existing. Contoh konkret: role "Lead" dilarang `ToolKind::FileWrite`/`Shell` di MANA PUN dalam SOP run.

#### Detail Eksekusi — BL-58

**TIDAK DIVERIFIKASI sesi ini** — `HookResult::Cancel` dan infrastruktur hook `preToolUse` disebut dokumen tapi lokasi file persis belum dicari (kemungkinan `crates/clawcrew-runtime/src/hooks/` sesuai tabel File Ownership). **Langkah pertama WAJIB:** baca `crates/clawcrew-runtime/src/hooks/` penuh untuk signature `preToolUse` hook dan `HookResult::Cancel` variant persis sebelum menulis enforcement logic.

**Spesifikasi API (pola, verifikasi signature persis saat eksekusi):**
```rust
fn enforce_role_restriction(
    constraint: &ConstraintKind, // varian RoleRestriction
    current_role: &str,
    tool_kind: ToolKind,
) -> Option<HookResult> {
    if let ConstraintKind::RoleRestriction { role, forbidden_tools } = constraint {
        if role == current_role && forbidden_tools.contains(&tool_kind.to_string()) {
            return Some(HookResult::Cancel {
                reason: format!("role '{role}' is restricted from using {tool_kind:?} in this SOP run"),
            }); // verifikasi shape persis HookResult::Cancel
        }
    }
    None
}
```

**Langkah pengerjaan:**
1. Baca hook infrastructure `preToolUse` penuh.
2. Cari titik di SOP run loop di mana `current_role`/performer step saat ini diketahui (kemungkinan dari `SopStep.agent`/`effective_agent()` yang sudah ada, baris ~400-405 di `types.rs`).
3. Loop semua `Sop.constraints`, filter `RoleRestriction`, panggil enforcement SEBELUM tool dieksekusi (titik `preToolUse` yang sudah ada).
4. Test: SOP run dengan constraint "Lead dilarang Shell" + step yang di-assign ke Lead mencoba panggil tool shell → `HookResult::Cancel`, tool TIDAK dieksekusi.

**Dependency:** BL-57 (hard, tipe harus ada dulu).

**JANGAN DIUBAH:** Hook `preToolUse` infrastructure — tiket ini HANYA menambah satu enforcement rule baru yang memakai hook existing.

- [x] **BL-59 (MEDIUM, SETELAH BL-58)** Enforcement `DistinctPerformer` (Dynamic Separation of Duties).

#### Detail Eksekusi — BL-59

**Langkah pengerjaan:**
1. Setelah BL-58 pattern established, tambah enforcement untuk `ConstraintKind::DistinctPerformer { step_a, step_b }` — cek saat step B akan dieksekusi, apakah performer-nya (resolved via `effective_agent`) SAMA dengan performer step A yang sudah selesai. Kalau sama → `HookResult::Cancel`.
2. Titik pemasangan: SAMA dengan BL-58 (hook `preToolUse`, atau titik step-transition kalau constraint ini perlu dicek di level step bukan tool — VERIFIKASI mana yang tepat, karena "distinct performer" adalah constraint ANTAR-STEP, bukan antar-tool-call, mungkin perlu hook berbeda `preStepStart` kalau ada, atau logic terpisah di SOP engine loop).

**Edge case:** Step A dan B dijalankan oleh agent yang SAMA karena SOP hanya punya 1 agent terdaftar (tidak ada agent lain yang bisa ditugaskan) — ini harus menghasilkan error konfigurasi yang jelas saat SOP di-save/validasi, BUKAN silent failure saat run.

**Dependency:** BL-58 (pola enforcement serupa).

**ASUMSI:** Titik pemasangan enforcement (`preToolUse` vs titik lain) perlu verifikasi arsitektur SOP engine loop saat eksekusi — saya tidak bisa memastikan `preToolUse` cukup granular untuk constraint antar-step tanpa membaca SOP engine loop penuh (di luar budget sesi ini).

- [x] **BL-60 (LOW, SETELAH BL-58)** Contoh SOP nyata "Engineering Feature Delivery" — SEKALIGUS jadi test-case pertama untuk evaluation/telemetry loop (lihat `decisions/sop-eval-telemetry-loop-design.md`, advisory bukan blocking).

#### Detail Eksekusi — BL-60

**Langkah pengerjaan:** Tulis SOP.md nyata (bukan kode Rust) yang mendemonstrasikan Do's/Don'ts (BL-06) + RACI (BL-07) + Constraint RoleRestriction (BL-58) dalam satu procedure "Engineering Feature Delivery" — pola FE→Lead→Backend→Principle yang sudah established di lesson project ini. File: kemungkinan di direktori SOP contoh/template (lokasi belum diverifikasi — cari `*.sop.md`/direktori `sop-examples/` existing).

**Dependency:** BL-58 (constraint enforcement harus berfungsi untuk SOP ini punya makna, bukan hanya field kosong).

- [x] **BL-61 (LOW, SETELAH BL-60)** Contoh SOP non-engineering (Marketing) — bukti `SopConstraint` genuinely department-agnostic tanpa ubah kode `ConstraintKind`.

#### Detail Eksekusi — BL-61

**Langkah pengerjaan:** Tulis SOP.md kedua untuk workflow Marketing (Copywriter→Reviewer→Publish, atau pola serupa) menggunakan `ConstraintKind` YANG SAMA PERSIS tanpa modifikasi kode — ini acceptance criteria utamanya: kalau perlu ubah kode `ConstraintKind` untuk SOP ini, BL-57/58 desainnya belum genuinely generic dan perlu direvisi.

**Dependency:** BL-60 (pola yang sama, domain berbeda).

## C. Evaluation/Telemetry Loop (advisory, divalidasi lewat BL-60)

- [x] **Eval-01 (dari `decisions/sop-eval-telemetry-loop-design.md`)** Sambungkan `clawcrew-eval` (harness Replay/Live yang sudah matang, CLI sudah ada) sebagai check ADVISORY di CI — TIDAK blocking merge. Dijalankan Lead→Backend→Principle (bukan Owner harian).

#### Detail Eksekusi — Eval-01

**TIDAK DIVERIFIKASI sesi ini** — lokasi `clawcrew-eval` crate dan CLI-nya belum dibaca. **Langkah pertama WAJIB:** baca `crates/clawcrew-eval/` struktur dan CLI entry point.

**Langkah pengerjaan:**
1. Baca `clawcrew-eval` CLI command existing.
2. Tambah job `.github/workflows/` baru yang menjalankan eval harness terhadap BL-60 (SOP "Engineering Feature Delivery") sebagai test-case — job TIDAK memblokir merge (`continue-on-error: true` atau pola non-required-check yang sudah dipakai workflow lain, cek `.github/workflows/` existing untuk konvensi non-blocking check).
3. Hasil eval di-post sebagai PR comment/annotation (advisory), bukan status check yang gagal.

**Dependency:** BL-60 (butuh SOP contoh untuk dijadikan test-case eval).

- [x] **Eval-02** Surface `SopCounters`/`RunSnapshot` (metrics SOP yang sudah matang tapi nol di-expose) ke endpoint read-only gateway/dashboard.

#### Detail Eksekusi — Eval-02

**TIDAK DIVERIFIKASI sesi ini** — lokasi `SopCounters`/`RunSnapshot` belum dicari. **Langkah pertama WAJIB:** `grep -rn "SopCounters\|RunSnapshot" crates/` untuk lokasi struct ini (disebut dokumen "sudah matang" — kemungkinan ada di `sop/` module, perlu dikonfirmasi).

**Langkah pengerjaan:**
1. Temukan struct `SopCounters`/`RunSnapshot` dan API yang sudah mengisinya.
2. Tambah endpoint read-only baru di `clawcrew-gateway/src/` (pola REST GET, konsisten endpoint metrics lain yang sudah ada seperti disebut R-90 Sprint 1 `clawcrew_cache_hit_rate`).

**Dependency:** Tidak hard-block tiket lain di sprint ini.

## D. SOP Execution Model — Delivery Separation & Cost Cap (dari `automation-cron-webhook-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)

- [x] **BL-13 (LOW-MEDIUM, MULAI DENGAN VERIFIKASI, bukan langsung kode)** Cost cap eksplisit per cron-job/SOP-run. **Verifikasi dulu**: `SecurityPolicy.max_actions_per_hour`/`record_action()`/`is_rate_limited()` (`crates/clawcrew-config/src/policy.rs:349`) sudah dipakai luas di `cron/scheduler.rs:898,1272` dan `subagent/mod.rs` — kemungkinan BESAR task ini SUDAH tercakup existing mechanism, bukan gap baru. Definition of Done tahap ini: dokumentasikan apakah `max_actions_per_hour` sudah cukup sebagai cost-cap level cron-job/SOP-run, atau genuinely butuh field tambahan (mis. cap biaya USD, bukan cuma jumlah aksi).

#### Detail Eksekusi — BL-13

**Langkah pengerjaan (VERIFIKASI, bukan kode):**
1. Baca `crates/clawcrew-config/src/policy.rs` sekitar baris 349 (`SecurityPolicy.max_actions_per_hour`) penuh — pahami apa yang dihitung sebagai "action" (setiap tool call? setiap SOP step? setiap turn?).
2. Baca pemanggilan di `cron/scheduler.rs:898,1272` dan `subagent/mod.rs` — konfirmasi baris persis (dokumen lama, nomor bisa bergeser — verifikasi ulang).
3. Tulis dokumen temuan singkat (markdown, di `standard-docs/decisions/` atau sejenis) menjawab: apakah ini CUKUP sebagai cost-cap, atau butuh dimensi baru (biaya USD aktual dari token LLM, bukan hitungan aksi).
4. KEPUTUSAN: kalau cukup, tiket ini SELESAI tanpa kode baru (dokumentasi saja). Kalau tidak cukup, tulis AC lanjutan sebagai tiket terpisah untuk ditambahkan (butuh approval Lead/Owner karena menambah dimensi baru ke `SecurityPolicy`).

**JANGAN DIUBAH:** `SecurityPolicy`/`max_actions_per_hour` di tahap verifikasi ini — murni membaca dan mendokumentasikan.

- [x] **BL-12 (LOW, EVALUASI dulu — bukan implementasi langsung)** Pemisahan eksekusi vs delivery untuk SOP/cron trigger eksternal (pola `gh-aw` safe-outputs). **Bukan vulnerability aktif** — `ContentSafety::screen_event` (BL-14, Sprint 0) sudah menyaring input SOP. Ini defense-in-depth tambahan: evaluasi apakah perlu tahap "proposed output" terpisah sebelum `SopRunAction::ExecuteStep` menerapkan delivery (permission lebih sempit). Definition of Done tahap ini: design-spec baru yang memutuskan perlu/tidak, JANGAN langsung ubah `SopRunAction` yang sudah punya invariant ketat.

#### Detail Eksekusi — BL-12

**TIDAK DIVERIFIKASI sesi ini** — `SopRunAction::ExecuteStep` belum dibaca. **Langkah pertama WAJIB:** cari definisi `SopRunAction` enum (kemungkinan di `sop/types.rs` dekat struct lain yang sudah dibaca, atau `sop/engine/mod.rs`).

**Langkah pengerjaan:**
1. Baca `SopRunAction` dan invariant yang dimaksud dokumen ("invariant ketat" — perlu dipahami apa yang membuatnya ketat sebelum mengevaluasi perubahan).
2. Tulis design-spec baru (markdown) mengevaluasi: apakah pola `gh-aw` safe-outputs (proposed-output lalu delivery terpisah) applicable di sini, dengan trade-off dijelaskan.
3. KEPUTUSAN: kalau Lead/Owner setuju perlu, buat tiket implementasi BARU (bukan di sini). Kalau tidak perlu, tutup tiket ini dengan dokumentasi alasan.

**JANGAN DIUBAH:** `SopRunAction` — tahap ini murni evaluasi/dokumentasi.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `crates/clawcrew-runtime/src/sop/types.rs` (field baru), `engine/src/mission/` (Quest Go) | Backend-Rust + Backend-Go |
| B | `crates/clawcrew-runtime/src/sop/{types.rs,constraint.rs}` (baru), `crates/clawcrew-runtime/src/hooks/` (reuse existing) | Backend-Rust — **serialkan A dan B pada `sop/types.rs` kalau bersinggungan** |
| C | `crates/clawcrew-eval/`, `.github/workflows/` (check baru, non-blocking), `clawcrew-gateway/src/` (endpoint metrics baru) | Backend-Rust + Platform/DevOps |
| D | `crates/clawcrew-config/src/policy.rs` (baca, verifikasi BL-13), `standard-docs/decisions/` (design-spec baru BL-12) | squad-lead (evaluasi/dokumentasi dulu, bukan kode) |

## Keluaran Sprint

- SOP engine punya Do's/Don'ts + RACI generik lintas departemen (terbukti via contoh Marketing, bukan hanya Engineering).
- Larangan berbasis role ("Lead tidak boleh coding") ditegakkan struktural via hook, bukan instruksi teks yang bisa diabaikan model.
- `clawcrew-eval` tersambung CI sebagai advisory check; `SopCounters` terlihat operator tanpa baca log mentah.
