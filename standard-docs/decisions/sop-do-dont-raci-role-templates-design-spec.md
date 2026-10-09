---
title: Design Spec — SOP Do's/Don'ts, RACI, Role Templates per-Crew, Quest Maturity
status: proposed
date: 2026-10-04
author: squad-lead
depends_on: ../research/orders-quest-sop-workflow-role-audit.md
reviewers_needed: Backend-Rust (sop::types impact), Backend-Go (Quest/Workflow impact), Principle (gate check)
---

# Design Spec — SOP Do's/Don'ts, RACI, Role Templates, Quest Maturity

> **Status:** PROPOSED — belum diimplementasi. Ini design-spec siap-eksekusi, bukan kode final. 211 konstruksi
> `SopStep{}`/`Sop{}` tersebar di 30 file (`crates/clawcrew-runtime/src/sop/`), jadi perubahan struct harus
> lewat `#[serde(default)]` + `..Default::default()` (pola yang SUDAH dipakai konsisten di `types.rs`) agar
> tidak merusak call-site yang ada. Jangan eksekusi langsung tanpa Principle gate review.
>
> **Tidak menyentuh** `decisions/finalize/dev-tasks/` (masih dikerjakan tim lain).

---

## 1. Field Do's/Don'ts Terstruktur untuk `SopStep` (Rust)

### Keputusan desain

Tambah field baru, BUKAN pindahkan konten dari `body` (markdown prosa) yang sudah ada — Do's/Don'ts terstruktur melengkapi `body`, bukan menggantikannya. Alasan: `body` adalah instruksi naratif untuk LLM (tetap perlu), field baru ini untuk constraint yang bisa divalidasi/ditampilkan terpisah di UI Blueprint editor (sesuai gap yang ditemukan: SOP dunia nyata punya Do's/Don'ts sebagai bagian checklist terpisah, bukan tercampur prosa).

### Perubahan struct

```rust
// crates/clawcrew-runtime/src/sop/types.rs — tambahan ke SopStep

/// Explicit Do's and Don'ts for this step, surfaced separately from `body`
/// so they can be rendered as a checklist in the Blueprint editor and
/// validated independently of the narrative instruction. Non-authoritative
/// over `scope`/`routing`/`on_failure` (those remain the enforced
/// mechanisms) — this is operator-facing guidance injected into the
/// step prompt alongside `body`, not a new enforcement layer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct StepConduct {
    /// Explicit rules the step MUST follow. Rendered as "Do:" checklist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dos: Vec<String>,
    /// Explicit rules the step MUST NOT violate. Rendered as "Don't:" checklist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub donts: Vec<String>,
}

// Di dalam SopStep struct, tambahkan field:
    /// Explicit Do's/Don'ts for this step. Absent = no additional conduct
    /// rules beyond `body` + `scope`/`on_failure`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conduct: Option<StepConduct>,
```

### Dampak ke `SopStep::default()`

Tambah `conduct: None` ke `impl Default for SopStep` (baris yang sudah ada di `types.rs`). Karena `#[serde(default, skip_serializing_if = "Option::is_none")]`, ini **tidak** memengaruhi 211 konstruksi existing yang pakai `..SopStep::default()` — mereka otomatis dapat `conduct: None`.

### Dampak ke prompt-injection (agent/turn)

`SopRunAction::ExecuteStep` yang membawa `step: SopStep` ke agent — perlu update titik yang merender `body` ke prompt (lokasi: cari `step.body` di `crates/clawcrew-runtime/src/agent/turn/` atau `sop/executor.rs`) agar `conduct.dos`/`conduct.donts` ikut disuntik, format seperti:
```
## Do
- <dos[0]>
- <dos[1]>

## Don't
- <donts[0]>
```
Ini **generalisasi** dari pola template `personality_templates/AGENTS.md` yang sudah terbukti bagus (`## Safety` dengan bullet "Don't..."), diterapkan terstruktur per-step bukan hanya di persona-level.

### Verifikasi/test

Tambah test di `types.rs` (ikuti pola `default_step_contract_fields_do_not_serialize` yang sudah ada): `conduct: None` tidak serialize; `conduct: Some(StepConduct{ dos: vec![], donts: vec![] })` juga tidak serialize field kosong (`skip_serializing_if` berlaku rekursif per Vec).

---

## 2. Field RACI Lengkap di `Sop` Struct (Rust)

### Keputusan desain

**Reuse `approval.policies` yang sudah ada untuk Accountable/Consulted** (jangan bangun sistem approver-group baru — `SopStep.policy: Option<String>` sudah merujuk `[sop.approval].policies` dengan quorum). RACI lengkap:

| RACI | Field baru/reuse | Alasan |
|---|---|---|
| Responsible | `SopStep.agent` (SUDAH ADA) | Tidak diubah |
| Accountable | `Sop.accountable_role: Option<String>` (BARU, di level `Sop` bukan per-step — accountable biasanya satu per prosedur, bukan per-langkah) | Field baru minimal, tidak reuse approval policy karena accountable adalah pernyataan ORGANISASI ("siapa pemilik SOP ini"), bukan gate approval |
| Consulted | `SopStep.policy` (SUDAH ADA, reuse) | Quorum-approval SUDAH berfungsi sebagai "harus dikonsultasikan sebelum lanjut" — tidak perlu field baru, cukup DOKUMENTASIKAN ulang semantik `policy` mencakup ini |
| Informed | `Sop.informed_roles: Vec<String>` (BARU) | Belum ada mekanisme "siapa diberi tahu setelah selesai" — perlu field baru + hook di `SopRunStatus::Completed`/`Failed` transition untuk emit notifikasi ke role ini |

### Perubahan struct

```rust
// crates/clawcrew-runtime/src/sop/types.rs — tambahan ke struct Sop

    /// Organizational owner accountable for this procedure's outcome — NOT
    /// an approval gate (see `SopStep.policy` for that). Informational: who
    /// answers for this SOP succeeding or failing. `None` = no designated
    /// owner beyond the parent `agent`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accountable_role: Option<String>,
    /// Role aliases notified when a run reaches a terminal state
    /// (Completed/Failed/Cancelled). Delivery mechanism is the existing
    /// run-event system (`PublishEvent`-equivalent on the Rust side) — this
    /// field only declares WHO, not HOW the notification is delivered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub informed_roles: Vec<String>,
```

Tambahkan juga ke `SopMeta` (TOML manifest struct) dan `SopManifest::from_sop()` mapping — ikuti pola existing `agent: Option<String>` yang sudah dipetakan di kedua tempat.

### Dampak eksekusi

Saat `SopRun` mencapai status terminal (`Completed`/`Failed`/`Cancelled`), titik transisi state (kemungkinan di `sop/engine/mod.rs` atau `sop/dispatch.rs`) perlu memanggil emit-event untuk setiap `informed_roles` — **reuse channel notifikasi yang sudah ada** (lihat bagaimana approval request sudah emit `tool.approval_required` via `PublishEvent` di Go — cari ekuivalen Rust-side event emission untuk SOP run completion).

### Verifikasi/test

Tambah test: SOP dengan `informed_roles: ["qa-lead"]` yang selesai (`Completed`) menghasilkan satu event bertipe baru (mis. `sop.run_completed_notify`) dengan `informed_roles` sebagai payload — supaya konsumen (Go orchestrator atau notification channel) bisa subscribe tanpa SOP engine Rust tahu MEKANISME pengiriman (konsisten boundary Go=kebijakan-notifikasi, Rust=eksekusi-state-machine).

---

## 3. Template Role per-Crew (Backend/Frontend/QA)

### Keputusan desain

Ikuti pola file yang SUDAH ada di `personality_templates/` (AGENTS.md generik) — buat 3 file baru sejenis, bukan mekanisme baru. Setiap file mengikuti struktur yang sudah terbukti (Safety, External vs Internal, dst.) tapi isi Do's/Don'ts spesifik per peran teknis.

### File baru: `crates/clawcrew-runtime/src/agent/personality_templates/AGENTS.backend.md`

```markdown
# AGENTS.md — Backend Crew (Go Orchestrator + Rust System Core)

## Role
Kamu Backend Crew. Tugasmu implementasi literal dari spec yang Lead Squad tulis — bukan menulis spec sendiri.

## Do
- Baca modul yang akan diubah + test adjacent SEBELUM edit (AGENTS.md root: "Read the owning module... before editing").
- Validasi di boundary (input dari luar tier), bukan di tengah logic.
- Propagate error di jalur produksi Rust — jangan `unwrap()`/`expect()` kecuali invariant yang didokumentasikan membuat panic mustahil.
- Go: wrap error dengan `%w`, gunakan `zap` untuk structured logging.
- Satu writer per crate/file yang beririsan — serialkan, jangan paralel di scope sama.
- Verifikasi batch (`cargo check --workspace` sekali) untuk pembuktian akhir, bukan per-file berulang.

## Don't
- Jangan negosiasi ulang spec saat coding — spec sudah difinalkan Lead; lapor balik kalau ada blocker teknis, jangan diam-diam ubah scope.
- Jangan tambah dependency berat tanpa use-case konkret.
- Jangan campur refactor luas dengan perubahan fungsional di PR yang sama.
- Jangan commit kecuali diminta eksplisit; jangan `git add .` di repo ini (node_modules/package-lock tidak di .gitignore).
- Jangan push ke `main`/`master` langsung.

## Boundary Spesifik Repo Ini
- Go = kebijakan/orkestrasi/hierarki organisasi (Fleet→Ship→Squad→Crew). Rust = eksekusi/storage/native tools.
- UI tidak pernah panggil Rust gRPC langsung — selalu lewat Go :9090.
```

### File baru: `AGENTS.frontend.md`

```markdown
# AGENTS.md — Frontend Crew (React 19 + TypeScript + Tailwind)

## Role
Kamu Frontend Crew. Implementasi UI sesuai design-spec dari Lead/Design — zero backend logic.

## Do
- Semua panggilan API lewat `apiClient.ts` (generic `get<T>`/`post<T>`), jangan `fetch` langsung di komponen.
- TypeScript strict — tanpa `any`.
- Validasi input di boundary form sebelum kirim ke API.
- Komponen functional, Tailwind utility classes — hindari inline style.

## Don't
- Jangan panggil Rust gRPC atau LLM provider langsung dari UI — itu pelanggaran boundary (ADR 0001/0002).
- Jangan simpan logika bisnis di komponen — kalau ada aturan domain, itu milik Go.
- Jangan tambah dependency AI/LLM client (`@google/genai` dst.) di `web/` — sudah pernah jadi insiden ADR 0002.

## Boundary Spesifik Repo Ini
- UI → HTTP Go :9090 saja. Tauri IPC → Go HTTP (bukan Rust gRPC :50052 langsung).
```

### File baru: `AGENTS.qa.md`

```markdown
# AGENTS.md — QA Crew

## Role
Kamu QA Crew. Verifikasi hasil Backend/Frontend sebelum merge, plus bantu build-efficiency saat compile/test lama.

## Do
- Jalankan test yang relevan dengan scope perubahan, bukan full-suite tiap kali kecuali diminta.
- Laporkan temuan bug dengan severity jelas (minor/medium/critical/high) — pola 3-tier yang dipakai Principle.
- Verifikasi test count SEBELUM/SESUDAH refactor file-split sama (bukti pure-move, bukan test hilang).

## Don't
- Jangan approve PR yang CI-nya belum hijau.
- Jangan tandai task selesai tanpa verifikasi — report crew ≠ Definition of Done.

## Boundary Spesifik Repo Ini
- Build/test lama (>30 menit) → eskalasi, jangan terus tunggu pasif.
```

### Dampak ke loader

Perlu verifikasi: apakah `load_skills_for_agent`/konfigurasi `AliasedAgentConfig` punya cara memilih file `AGENTS.<role>.md` mana yang dipakai per-agent-alias, atau saat ini hanya satu `AGENTS.md` statis yang selalu dipakai? **Ini pertanyaan verifikasi lanjutan sebelum file baru ini punya efek nyata** — kalau loader-nya hardcode satu nama file, perlu tambah logic pemilihan berdasarkan role/alias dulu (scope terpisah, bukan bagian dari task menulis template ini).

---

## 4. Matangkan `Quest` (Go) Menyusul `SopRun` (Rust)

### Keputusan desain

**Jangan samakan `Quest` dengan `SopRun`** — `Quest` adalah pembungkus ORGANISASI (Fleet/Ship level, bisa membungkus BANYAK `SopRun`), `SopRun` adalah eksekusi TEKNIS satu prosedur. Menambah field ke `Quest` harus mencerminkan perbedaan level ini, bukan sekadar duplikasi field `SopRun`.

### Perubahan struct

```go
// engine/src/mission/interfaces.go — Quest diperluas

type QuestStatus string

const (
	QuestPending   QuestStatus = "pending"
	QuestActive    QuestStatus = "active"
	QuestCompleted QuestStatus = "completed"
	QuestFailed    QuestStatus = "failed"
	QuestCancelled QuestStatus = "cancelled"
)

type Quest struct {
	ID           string      `json:"id"`
	Status       QuestStatus `json:"status"` // diketatkan dari string bebas ke enum
	TargetShipID string      `json:"target_ship_id"`
	WorkflowID   string      `json:"workflow_id"`

	// BARU — menyusul field yang sudah terbukti perlu di SopRun:
	StartedAt   time.Time  `json:"started_at"`
	CompletedAt *time.Time `json:"completed_at,omitempty"`
	// FailureReason, mirror SopRun.failure_reason — kenapa quest gagal di
	// level organisasi (beda dari failure_reason per-SopRun yang teknis).
	FailureReason string `json:"failure_reason,omitempty"`
	// SopRunIDs — Quest membungkus satu/lebih SopRun; ini daftar run yang
	// Quest ini orkestrasikan (lintas Rust SOP engine via gRPC).
	SopRunIDs []string `json:"sop_run_ids,omitempty"`
	// AccountableRole / InformedRoles — RACI di level Quest (Fleet/Ship),
	// SELARAS dengan field baru di Sop (Rust) §2, tapi Quest adalah level
	// organisasi jadi field ini independen (satu Quest bisa membungkus
	// beberapa Sop dengan accountable_role berbeda-beda).
	AccountableRole string   `json:"accountable_role,omitempty"`
	InformedRoles   []string `json:"informed_roles,omitempty"`
}
```

### Dampak lifecycle

`Quest.Status` saat ini `string` bebas (tidak type-safe) — ganti ke `QuestStatus` enum menyusul pola `SopRunStatus` di Rust (type-safe, `Display` implementasi). **Perlu cek dulu**: apakah ada kode existing yang set `Quest.Status` dengan string sembarang yang akan pecah saat tipe diketatkan? (Verifikasi sebelum eksekusi — `git grep "\.Status = "` di file yang pakai `Quest`.)

### Dampak ke `mission` service layer

`engine/src/mission/interfaces.go` saat ini HANYA berisi struct — tidak ada `Service` interface seperti `fleet.Service` atau `memory.Service`. Perlu tambah `mission.Service` dengan method minimal: `CreateQuest`, `GetQuest`, `UpdateQuestStatus`, `ListQuestsByShip` — mengikuti pola service layer yang konsisten dipakai domain lain (`fleet/interfaces.go`, `memory/interfaces.go`). Ini scope TERPISAH dari task "tambah field", perlu task sendiri kalau mau Quest benar-benar operasional (bukan cuma struct kaya tanpa service).

### Verifikasi/test

Test baru: `Quest` lifecycle (Pending→Active→Completed/Failed) mirror pola test `SopRun` yang sudah ada di Rust (`sop_run_serde_roundtrip`), plus test bahwa `SopRunIDs` terisi saat Quest mengorkestrasi lebih dari satu SOP run.

---

## Urutan Eksekusi yang Disarankan

1. **§1 (Do's/Don'ts `SopStep`)** — scope kecil, isolated, langsung actionable.
2. **§3 (template role)** — independent dari §1/§2/§4, bisa paralel, tapi perlu verifikasi loader dulu (lihat catatan di §3).
3. **§2 (RACI `Sop`)** — scope sedikit lebih besar (TOML manifest mapping + event emission), setelah §1 karena sama-sama menyentuh `types.rs`.
4. **§4 (Quest Go)** — paling besar scope-nya (perlu service layer baru), kerjakan terakhir atau sebagai task terpisah dari 3 lainnya.

## File Ownership (untuk Lead assign ke squad)

| Item | File utama | Owner | Review |
|---|---|---|---|
| §1 | `crates/clawcrew-runtime/src/sop/types.rs` + titik render prompt | Backend-Rust | Principle |
| §2 | `crates/clawcrew-runtime/src/sop/types.rs` (manifest mapping) + event emission point | Backend-Rust | Principle |
| §3 | `crates/clawcrew-runtime/src/agent/personality_templates/AGENTS.{backend,frontend,qa}.md` (baru) + verifikasi loader | Backend-Rust (loader) | squad-lead (isi konten) |
| §4 | `engine/src/mission/interfaces.go` + service baru `engine/src/mission/service.go` | Backend-Go | Principle |

Serialkan §1/§2 (satu file `types.rs`), §3/§4 independen bisa paralel.
