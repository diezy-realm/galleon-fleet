# BL-13: Verifikasi Cost Cap & max_actions_per_hour

**Status**: Verified Sufficient  
**Tanggal**: 2026-10-07  
**Area**: Sprint 3 — Area D: Execution Model  
**Referensi**:
- `crates/galleon-config/src/policy.rs:349-350`
- `crates/clawcrew-runtime/src/agent/cost.rs`
- `crates/clawcrew-runtime/src/tools/delegate/mod.rs`
- `standard-docs/decisions/finalize/dev-task-2/sprint-3-sop-constraint-and-raci.md`

---

## 1. Latar Belakang & Pertanyaan Evaluasi

Task `BL-13` pada Sprint 3 meminta verifikasi apakah `max_actions_per_hour` yang sudah ada di `SecurityPolicy` cukup sebagai cost cap / safety bound, atau apakah diperlukan per-run cost cap berbasis LLM cost (cents).

Tujuan:
1. Mencegah runaway loops (infinite tool invocation / agent spinning).
2. Membatasi pengeluaran finansial (API token spend).
3. Memastikan kepatuhan terhadap prinsip *Single Source of Truth* tanpa menambah duplikasi state atau dependensi yang tidak diperlukan.

---

## 2. Temuan Arsitektur & Fakta Implementasi

Berdasarkan audit faktual codebase pada repositori:

### 2.1. `max_actions_per_hour` (Rate & Frequency Cap)
- **Definisi**: Dideklarasikan pada `crates/galleon-config/src/policy.rs:349`:
  ```rust
  pub max_actions_per_hour: u32,
  pub max_cost_per_day_cents: u32,
  ```
- **Penegakan**:
  - Ditegakkan per-sender dan per-agent pada runtime loop.
  - Nilai `0` diinterpretasikan sebagai strict zero budget (hard deny).
  - Pada hirarki delegasi (`crates/clawcrew-runtime/src/tools/delegate/mod.rs`), target policy di-clamp secara fail-closed:
    ```rust
    target_policy.max_actions_per_hour = target_policy
        .max_actions_per_hour
        .min(self.security.max_actions_per_hour);
    ```
    Hal ini menjamin bahwa subagent atau delegasi tidak dapat digunakan untuk membypass batas aksi per jam.

### 2.2. `max_cost_per_day_cents` & `CostTracker` (Financial Spend Cap)
- **Definisi**: Didefinisikan pada `SecurityPolicy` dan terintegrasi dengan subsistem biaya (`crates/clawcrew-runtime/src/agent/cost.rs`).
- **Penegakan**:
  - `CostTracker` menghitung biaya riil berdasarkan pricing tabel per model (`ModelRates`: prompt, completion, cache hit/miss).
  - Di dalam `crates/clawcrew-runtime/src/tools/delegate/mod.rs`, pemanggilan delegasi mengevaluasi:
    ```rust
    if Self::exceeds_cost_cap(summary.daily_cost_usd, max_cost_per_day_cents) {
        return self.cost_budget_refusal(target_alias, max_cost_per_day_cents);
    }
    ```
  - `ToolLoopCostTrackingContext` mengakumulasikan pemakaian token per turn (`TurnUsage`) secara task-local tanpa race condition.

---

## 3. Kesimpulan & Keputusan

**Keputusan: Verified Sufficient.**

1. **Dual-Layer Protection**:
   - `max_actions_per_hour` secara efektif menangani ancaman *velocity* (loop tak terbatas, high-frequency tool abuse).
   - `max_cost_per_day_cents` bersama `CostTracker` menangani ancaman *financial spend* (pemakaian token berlebih).
2. **Kesesuaian dengan SOP**:
   - Step eksekusi SOP dijalankan melalui agent yang tunduk pada `SecurityPolicy` dan `CostTracker`.
   - Tidak diperlukan per-run cost cap field tambahan pada `SopStep` atau `SopRunAction` karena budget sudah dikontrol secara terpusat pada level agent risk profile dan delegasi.
3. **Single Source of Truth**:
   - Menghindari penambahan field duplikat `cost_cap_cents` di dalam `SopManifest` yang akan berbenturan dengan `SecurityPolicy`.

Dengan demikian, task `BL-13` terpenuhi secara menyeluruh tanpa memerlukan perubahan schema baru.
