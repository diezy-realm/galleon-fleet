# BL-12: Evaluasi & Desain Safe-Outputs Delivery Separation

**Status**: Evaluated & Specified  
**Tanggal**: 2026-10-07  
**Area**: Sprint 3 — Area D: Execution Model  
**Referensi**:
- `standard-docs/decisions/finalize/dev-task-2/sprint-3-sop-constraint-and-raci.md`
- `crates/clawcrew-runtime/src/sop/types.rs`
- `crates/clawcrew-runtime/src/sop/engine/mod.rs`
- GitHub Actions Workspace (`gh-aw`) Safe-Outputs Pattern

---

## 1. Latar Belakang & Konsep Safe-Outputs

Pola **Safe-Outputs** (dipopulerkan oleh arsitektur agent sandbox seperti `gh-aw`) memisahkan dua domain keamanan kritis:
1. **Domain Generasi / Penalaran**: Agent LLM berinteraksi dengan data dan tools yang berpotensi terpapar input tidak tepercaya (*untrusted input / prompt injection*).
2. **Domain Pengiriman (*Delivery*)**: Pengiriman hasil ke luar sistem (misal: posting ke Slack/Discord/Telegram, commit/push ke Git repository, publishing release, dispatch webhook).

Jika agent yang berada di Domain 1 memegang langsung kredensial pengiriman (delivery token / webhook secret), sebuah serangan prompt injection dapat membajak agent untuk mengeksfiltrasi rahasia atau mengirimkan payload berbahaya secara langsung ke audiens eksternal.

Safe-outputs mewajibkan pemisahan tegas:
- **Agent step** hanya diizinkan memproduksi data / artefak (*produce structured outputs*).
- **Delivery step** dieksekusi secara terpisah dalam konteks deterministik tanpa campur tangan LLM.

---

## 2. Evaluasi Terhadap `SopRunAction`

Pertanyaan kunci dalam DoD Sprint 3:
> *"Tentukan apakah `SopRunAction` perlu `ExecuteStep` terpisah untuk output delivery."*

### 2.1. Analisis Struktur `SopRunAction` Saat Ini
Saat ini `SopRunAction` mendefinisikan aksi transisi state:
```rust
pub enum SopRunAction {
    ExecuteStep { run_id: String, step: SopStep, context: String },
    WaitApproval { run_id: String, step: SopStep, context: String },
    DeterministicStep { run_id: String, step: SopStep, input: Value },
    CheckpointWait { run_id: String, step: SopStep, state_file: PathBuf },
    Pending { run_id: String, sop_name: String, step: u32, reason: String },
    Completed { run_id: String, sop_name: String },
    Cancelled { run_id: String, sop_name: String },
    Failed { run_id: String, sop_name: String, step: u32, error: String },
}
```

### 2.2. Keputusan Desain: Tidak Menambah Variant Baru
**Diputuskan: TIDAK PERLU menambahkan variant baru ke `SopRunAction`.**

**Alasan Arsitektural:**
1. **Dukungan Built-in via Pipeline**: `SopEngine` telah mendukung rantai eksekusi step bertahap (*piped step inputs*). Hasil eksekusi `SopStepResult` pada step $N$ secara otomatis dialirkan menjadi input bagi step $N+1$.
2. **Deterministic Capability Steps**: `SopStepKind::Capability` bersama `SopRunAction::DeterministicStep` mengeksekusi aksi secara deterministik melalui `SopCapabilityRegistry` tanpa pemanggilan LLM.
3. **Minimilisasi Kompleksitas (YAGNI / Single Source of Truth)**: Menambahkan variant seperti `DeliverOutputStep` pada `SopRunAction` akan menduplikasi semantik `DeterministicStep` dan merusak exhaustive match handling pada seluruh executor runtime, driver, dan test suite yang ada.

---

## 3. Pola Authored Safe-Outputs yang Direkomendasikan

Pemisahan delivery harus diimplementasikan pada tingkat **Authoring SOP** menggunakan kombinasi tipe step:

```
┌─────────────────────────────────┐
│ Step 1: Execute (Agent)         │
│ - Menghasilkan draf rilis / pesan│
│ - Output: JSON / Markdown string │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│ Step 2: Checkpoint (Gate Review)│
│ - Operator / Reviewer memverifikasi│
│ - edit: body (opsional)         │
└────────────────┬────────────────┘
                 │ (Approved)
                 ▼
┌─────────────────────────────────┐
│ Step 3: Capability (Delivery)   │
│ - kind: capability              │
│ - capability: "publish_release" │
│ - with: { input: "{{step.1}}" } │
│ - Dijalankan deterministik 100% │
└─────────────────────────────────┘
```

### Manfaat Keamanan:
1. Agent pada Step 1 tidak memiliki akses ke secret pengiriman atau izin jaringan untuk mem-publish.
2. Step 2 memastikan verifikasi manusia (HITL) terhadap konten yang digenerate sebelum publikasi.
3. Step 3 menjalankan pengiriman melalui capability internal yang terisolasi dan teraudit.

---

## 4. Kesimpulan

Spesifikasi desain ini memvalidasi bahwa arsitektur SOP saat ini sudah sepenuhnya siap mengakomodasi pola safe-outputs tanpa memerlukan modifikasi refactor yang memecah kontrak API pada `SopRunAction`.
