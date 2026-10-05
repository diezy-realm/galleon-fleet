---
title: Backlog — SopConstraint (Layer Larangan Berbasis Role untuk SOP Engine)
status: backlog
date: 2026-10-05
author: squad-lead
depends_on:
  - decisions/sop-constraint-layer-design.md
  - decisions/sop-do-dont-raci-role-templates-design-spec.md
---

# Backlog — SopConstraint

> Diturunkan dari `decisions/sop-constraint-layer-design.md`. Nomor lanjutan dari BL-56 (`attachment-handling-backlog.md`). Keputusan Achmad (2026-10-05): setuju desain two-layer (Flow existing + Constraint baru). Scope awal: `RoleRestriction` + `DistinctPerformer`, `RequiredOccurrence` ditunda sampai ada kasus nyata.

---

## BL-57 — Tipe `SopConstraint`/`ConstraintKind` di `sop::types` (P1, M)

**Rujukan:** `decisions/sop-constraint-layer-design.md` §4.

**Scope:** Tambah struct baru di `crates/clawcrew-runtime/src/sop/types.rs`, mengikuti konvensi penamaan/dokumentasi field yang sudah ada di file ini (lihat `Sop`/`SopStep`/`SopTrigger`):

```rust
/// Constraint lintas-step dalam satu SOP -- menjawab "role X dilarang Y",
/// axis berbeda dari SopStep.agent (yang menjawab "role X MENGERJAKAN Y").
/// Diperiksa oleh hook preToolUse SEBELUM eksekusi, bukan properti step individual.
pub struct SopConstraint {
    pub kind: ConstraintKind,
}

pub enum ConstraintKind {
    /// SoD: role ini dilarang memanggil tool dengan kind berikut, DI MANA PUN
    /// dalam run -- bukan larangan per-step, larangan per-tool lintas seluruh SOP.
    RoleRestriction { role: String, forbidden_tool_kinds: Vec<ToolKind> },
    /// Dynamic SoD: dua step tidak boleh dieksekusi oleh agent/role yang SAMA
    /// dalam SATU run (misal step "implement" dan step "approve" harus beda performer).
    DistinctPerformer { step_a: StepRef, step_b: StepRef },
}
```

Tambah `constraints: Vec<SopConstraint>` (default kosong, `#[serde(default)]`) di struct `Sop` — field baru PERSIS mengikuti pola `steps: Vec<SopStep>` yang sudah ada (`crates/clawcrew-runtime/src/sop/types.rs:458` area).

**File ownership:** Backend-Rust — `crates/clawcrew-runtime/src/sop/types.rs`.

**Definition of Done:** Test serialisasi/deserialisasi `SopConstraint` round-trip; `Sop` dengan `constraints: []` (default kosong) tidak mengubah perilaku SOP existing (backward-compatible); test unit untuk kedua varian `ConstraintKind` sebagai data struct murni (belum ada enforcement — itu BL-58).

**Dependency:** Tidak ada — field baru, additive, tidak menyentuh logic eksekusi existing.

**Risiko/Severity:** LOW — penambahan tipe data, backward-compatible by design (default kosong = perilaku lama).

---

## BL-58 — Enforcement `RoleRestriction` via Hook `preToolUse` (P1, M)

**Rujukan:** §4-5 — titik enforcement memakai infrastruktur hook yang SUDAH ADA (`crates/clawcrew-runtime/src/hooks/`, `HookResult::Cancel`), bukan membangun mekanisme baru.

**Scope:** Saat `SopEngine` memulai run untuk `Sop` yang punya `constraints` non-kosong, daftarkan hook `preToolUse` dinamis (scoped ke run tersebut) yang: (1) resolve role aktif dari `effective_agent()` step yang sedang berjalan, (2) cek apakah role itu punya `RoleRestriction` di `Sop.constraints`, (3) kalau tool yang akan dipanggil masuk `forbidden_tool_kinds`, return `HookResult::Cancel` dengan pesan jelas (misal: *"Role 'Lead' dilarang memanggil tool kind 'FileWrite' pada SOP 'engineering-feature-delivery'"*) — tool TIDAK dieksekusi, LLM menerima pesan penolakan sebagai hasil tool call (bukan error sistem).

**File ownership:** Backend-Rust — `crates/clawcrew-runtime/src/sop/engine/mod.rs` (titik registrasi hook per-run), `crates/clawcrew-runtime/src/hooks/` (hook handler baru, kemungkinan `hooks/builtin/sop_constraint.rs`).

**Definition of Done:** Test end-to-end: SOP dengan `RoleRestriction { role: "Lead", forbidden_tool_kinds: [FileWrite] }`, step yang `effective_agent() == "Lead"` mencoba memanggil tool `write` → ditolak dengan pesan jelas, step TIDAK lanjut sampai agent memilih aksi lain atau run gagal sesuai `on_failure`. SOP TANPA constraint (default kosong) berjalan identik seperti sebelum BL-57/58 (regresi nol).

**Dependency:** SETELAH BL-57 (butuh tipe `SopConstraint` ada dulu).

**Risiko/Severity:** MEDIUM — titik integrasi ke jalur eksekusi step yang sudah berjalan di produksi; perlu test regresi memastikan SOP tanpa constraint tidak terpengaruh sama sekali.

---

## BL-59 — Enforcement `DistinctPerformer` (Dynamic SoD) (P2, M)

**Rujukan:** §2 — Dynamic SoD: role boleh sama, tapi tidak boleh mengerjakan DUA step yang konflik dalam SATU run yang sama.

**Scope:** Setelah BL-58 (hook infrastructure sudah ada), tambah pengecekan kedua: sebelum step `step_b` dieksekusi, cek riwayat step yang sudah selesai di run yang sama (`SopRun` punya `step_results` — lihat riset sebelumnya) — kalau `step_a` dari pasangan `DistinctPerformer` sudah dieksekusi oleh performer yang SAMA dengan yang akan eksekusi `step_b`, tolak dengan pesan jelas.

**File ownership:** Backend-Rust — titik yang sama dengan BL-58 (hook handler), plus baca `SopRun.step_results` untuk riwayat performer.

**Definition of Done:** Test: SOP dengan `DistinctPerformer { step_a: "implement", step_b: "approve" }`, kalau SATU agent (role ganda, hipotetis) mengerjakan step "implement" lalu dijadwalkan juga untuk "approve" di run yang SAMA → ditolak. Kalau performer berbeda → lolos normal.

**Dependency:** SETELAH BL-58 (reuse hook infrastructure + pola enforcement yang sama).

**Risiko/Severity:** MEDIUM — kasus pakai lebih niche dari `RoleRestriction` (butuh skenario role ganda), tapi melengkapi cakupan SoD penuh (Static vs Dynamic) sesuai riset.

---

## BL-60 — Contoh SOP Nyata: "Engineering Feature Delivery" sebagai Test-Case + Dokumentasi (P2, S)

**Rujukan:** §5 — contoh konkret kasus Achmad ("Lead tidak boleh coding").

**Scope:** Buat SATU SOP definition nyata (bukan hanya unit test abstrak) yang merepresentasikan flow: Lead (architecture+task breakdown) → Backend (implement) → Frontend (implement) → Lead (review+approve), dengan `RoleRestriction { role: "Lead", forbidden_tool_kinds: [FileWrite, Shell] }`. Ini dipakai SEKALIGUS sebagai: (1) dokumentasi contoh pemakaian `SopConstraint` untuk Lead/Owner, (2) test-case untuk `clawcrew-eval` (lihat `decisions/sop-eval-telemetry-loop-design.md` — evaluation/telemetry loop yang sudah didesain sebelumnya bisa langsung memakai SOP ini sebagai salah satu case awal).

**File ownership:** Backend-Rust/Lead — definisi SOP (format sesuai `Sop` struct, lokasi penyimpanan SOP mengikuti konvensi yang sudah ada), dokumentasi pemakaian.

**Definition of Done:** SOP ini bisa dijalankan via `clawcrew-eval` mode `Replay`, hasilnya membuktikan constraint bekerja (Lead yang "mencoba" memanggil tool write ditolak dalam skenario scripted).

**Dependency:** SETELAH BL-58 (butuh enforcement nyata untuk dites, bukan hanya tipe data).

**Risiko/Severity:** LOW — murni contoh + dokumentasi, tidak ada kode produksi baru di luar yang sudah dibangun BL-57/58.

---

## BL-61 — Generalisasi: Contoh SOP Non-Engineering (Squad Marketing) sebagai Bukti Department-Agnostic (P3, S)

**Rujukan:** Koreksi Achmad soal Galleon sebagai AI Companion yang mengadopsi struktur Perusahaan nyata (bukan hanya engineering) — `research/agentic-ai-fundamentals-and-lifecycle-phase.md` dan diskusi hierarki Fleet=Perusahaan/Ship=Departemen/Squad=Tim/Crew=individu.

**Scope:** Buat SATU SOP contoh TAMBAHAN untuk Ship/Squad non-engineering (misal "Marketing Content Approval": Copywriter menulis draft → Content Lead review → **Copywriter dilarang publish langsung tanpa approval** sebagai `RoleRestriction`). Tujuannya membuktikan `SopConstraint` genuinely department-agnostic (`role: String` bebas, bukan enum tertutup ke istilah engineering) — bukan hanya diklaim generic tapi hanya pernah dites dengan skenario Backend/Frontend/Lead.

**File ownership:** Lead — definisi SOP contoh kedua.

**Definition of Done:** SOP ini berjalan dengan `SopConstraint` yang SAMA strukturnya dengan BL-60, hanya beda nilai `role`/`forbidden_tool_kinds` — tanpa perlu mengubah kode `SopConstraint`/`ConstraintKind` sama sekali (pembuktian genericity).

**Dependency:** SETELAH BL-60 (reuse pola yang sudah terbukti, bukan desain ulang).

**Risiko/Severity:** LOW — murni contoh pembuktian, tidak ada kode produksi baru.

---

## Urutan Eksekusi yang Disarankan

1. **BL-57** dulu — tipe data, tidak ada risiko regresi.
2. **BL-58** setelah BL-57 — enforcement `RoleRestriction`, use-case paling langsung dari contoh Achmad.
3. **BL-59** setelah BL-58 — enforcement `DistinctPerformer`, melengkapi cakupan SoD.
4. **BL-60** setelah BL-58 (bisa paralel dengan BL-59) — contoh nyata + sambung ke evaluation loop yang sudah didesain.
5. **BL-61** setelah BL-60 — bukti department-agnostic, prioritas rendah tapi penting untuk validasi filosofi produk.

## Catatan Proses

Severity keseluruhan MEDIUM — ini bukan bug, tapi fitur fundamental yang menutup gap arsitektural nyata (BPMN/Camunda juga tidak native punya ini, dikonfirmasi riset). BL-60/61 sengaja dirancang untuk terhubung ke `decisions/sop-eval-telemetry-loop-design.md` yang sudah ada — SOP contoh ini JUGA menjadi test-case pertama untuk evaluation loop advisory yang sudah didesain, bukan dua inisiatif terpisah.
