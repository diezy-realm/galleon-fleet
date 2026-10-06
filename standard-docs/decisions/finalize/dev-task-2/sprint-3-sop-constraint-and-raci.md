---
sprint: 3
title: SOP Engine — Do's/Don'ts, RACI, Quest, Constraint Layer (Separation of Duties)
status: planned
depends_on: none (additive, risiko regresi rendah)
---

# Sprint 3 — SOP Engine: Do's/Don'ts, RACI, Constraint Layer

## A. Flow Layer — Do's/Don'ts, RACI, Role Template, Quest (existing design-spec)

- [ ] **BL-06 (dari `sop-do-dont-raci-role-templates-design-spec.md`)** Field Do's/Don'ts terstruktur di `SopStep`. **Catatan generik:** contoh role (Backend/Frontend/QA) HARUS diperluas eksplisit mencakup minimal 1 contoh non-engineering (Marketing: Copywriter/Content Creator) — fundamental produk Galleon adalah AI Companion lintas departemen, bukan hanya engineering.
- [ ] **BL-07** Field RACI lengkap di `Sop` struct.
- [ ] **BL-08** Template role per-Crew — **diperluas generik** (lihat catatan BL-06), bukan hanya Backend/Frontend/QA.
- [ ] **BL-09** Matangkan `Quest` (Go) + `mission.Service` baru.

## B. Constraint Layer — Separation of Duties (BARU, two-layer design)

> **Temuan riset kunci:** diagram arrow (BPMN/n8n) TIDAK BISA merepresentasikan larangan berbasis role — bahkan Camunda/jBPM (workflow engine enterprise) tidak native punya SoD. Solusi: Layer Flow (`SopStep`, existing, TIDAK diubah) + Layer Constraint (baru, `SopConstraint`) terpisah. Enforcement via hook `preToolUse` yang infrastrukturnya SUDAH ADA (`HookResult::Cancel`).

- [ ] **BL-57 (LOW, additive, tidak ada risiko regresi)** Tipe `SopConstraint`/`ConstraintKind` di `sop::types`. Pola: `Sop.constraints: Vec<SopConstraint>` meniru `Sop.steps: Vec<SopStep>` yang sudah ada.
- [ ] **BL-58 (MEDIUM, SETELAH BL-57)** Enforcement `RoleRestriction` via hook `preToolUse` existing. Contoh konkret: role "Lead" dilarang `ToolKind::FileWrite`/`Shell` di MANA PUN dalam SOP run.
- [ ] **BL-59 (MEDIUM, SETELAH BL-58)** Enforcement `DistinctPerformer` (Dynamic Separation of Duties).
- [ ] **BL-60 (LOW, SETELAH BL-58)** Contoh SOP nyata "Engineering Feature Delivery" — SEKALIGUS jadi test-case pertama untuk evaluation/telemetry loop (lihat `decisions/sop-eval-telemetry-loop-design.md`, advisory bukan blocking).
- [ ] **BL-61 (LOW, SETELAH BL-60)** Contoh SOP non-engineering (Marketing) — bukti `SopConstraint` genuinely department-agnostic tanpa ubah kode `ConstraintKind`.

## C. Evaluation/Telemetry Loop (advisory, divalidasi lewat BL-60)

- [ ] **Eval-01 (dari `decisions/sop-eval-telemetry-loop-design.md`)** Sambungkan `clawcrew-eval` (harness Replay/Live yang sudah matang, CLI sudah ada) sebagai check ADVISORY di CI — TIDAK blocking merge. Dijalankan Lead→Backend→Principle (bukan Owner harian).
- [ ] **Eval-02** Surface `SopCounters`/`RunSnapshot` (metrics SOP yang sudah matang tapi nol di-expose) ke endpoint read-only gateway/dashboard.

## D. SOP Execution Model — Delivery Separation & Cost Cap (dari `automation-cron-webhook-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)

- [ ] **BL-13 (LOW-MEDIUM, MULAI DENGAN VERIFIKASI, bukan langsung kode)** Cost cap eksplisit per cron-job/SOP-run. **Verifikasi dulu**: `SecurityPolicy.max_actions_per_hour`/`record_action()`/`is_rate_limited()` (`crates/clawcrew-config/src/policy.rs:349`) sudah dipakai luas di `cron/scheduler.rs:898,1272` dan `subagent/mod.rs` — kemungkinan BESAR task ini SUDAH tercakup existing mechanism, bukan gap baru. Definition of Done tahap ini: dokumentasikan apakah `max_actions_per_hour` sudah cukup sebagai cost-cap level cron-job/SOP-run, atau genuinely butuh field tambahan (mis. cap biaya USD, bukan cuma jumlah aksi).
- [ ] **BL-12 (LOW, EVALUASI dulu — bukan implementasi langsung)** Pemisahan eksekusi vs delivery untuk SOP/cron trigger eksternal (pola `gh-aw` safe-outputs). **Bukan vulnerability aktif** — `ContentSafety::screen_event` (BL-14, Sprint 0) sudah menyaring input SOP. Ini defense-in-depth tambahan: evaluasi apakah perlu tahap "proposed output" terpisah sebelum `SopRunAction::ExecuteStep` menerapkan delivery (permission lebih sempit). Definition of Done tahap ini: design-spec baru yang memutuskan perlu/tidak, JANGAN langsung ubah `SopRunAction` yang sudah punya invariant ketat.

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
