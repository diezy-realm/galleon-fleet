---
title: Orders, Quest, SOP & Workflow — Role, Peran, Do's/Don'ts — Audit vs Kompetitor
status: research
date: 2026-10-04
author: squad-lead
depends_on: capabilities-skills-lessons-steering-audit.md
---

# Orders, Quest, SOP & Workflow — Role, Peran, Do's/Don'ts

> **Konteks:** Owner meminta analisis konkret: di dunia kerja nyata tiap crew punya role/peran masing-masing
> dengan SOP (Standard Operating Procedure) kerja, Do's dan Don'ts eksplisit. Dokumen ini mengaudit apa yang
> SUDAH diterapkan di `galleon-fleet` (SOP engine Rust, Workflow/Quest Go) dan membandingkan dengan n8n
> (workflow automation) serta praktik SOP/RACI dunia kerja nyata.
>
> **Status:** Riset murni. Tidak mengubah `decisions/` atau `dev-tasks/`.

---

## 1. Temuan Utama: SOP Engine Rust Sudah SANGAT Matang — Lebih dari Workflow Tool Biasa

Audit `crates/clawcrew-runtime/src/sop/` menemukan sistem SOP yang levelnya setara/lebih kaya dari n8n untuk sebagian aspek, karena didesain khusus untuk agent LLM (bukan integrasi API biasa):

| Konsep SOP dunia kerja | Implementasi Galleon (Rust `sop::types`) | Kematangan |
|---|---|---|
| **Trigger/pemicu prosedur** | `SopTrigger`: Mqtt, Webhook, Cron, Peripheral, Filesystem, Calendar, Channel, Manual, Amqp — 9 jenis | ✅ Sangat lengkap, lebih banyak dari trigger n8n standar |
| **Tingkat otonomi eksekusi** | `SopExecutionMode`: Auto, Supervised (default), StepByStep, PriorityBased, Deterministic | ✅ Granular — bisa diatur per-SOP, bahkan per-step (`SopStep.mode` override) |
| **Checkpoint/approval manusia** | `SopStepKind::Checkpoint`, `requires_confirmation`, `gate_prompt`, `edit` (approver bisa amend field sebelum lanjut) | ✅ Lebih kaya dari sekadar "pause" — ada quorum policy (`SopStep.policy` → `[sop.approval].policies`) |
| **Role/peran per-langkah** | `SopStep.agent: Option<String>` — override agent per-step, `effective_agent()` resolve step-override lalu parent | ✅ Ada, dan ENFORCED secara arsitektur (lihat §1a) |
| **Retry/gagal-tangani** | `StepFailure` (on_failure), `SopRunStatus::Failed` dengan `failure_reason` tercatat | ✅ Ada |
| **Concurrency control** | `SopAdmissionPolicy`: Parallel, Hold, Coalesce, Drop — plus `max_concurrent`, `cooldown_secs` | ✅ Lebih eksplisit dari n8n (yang defaultnya tidak punya opsi admission policy granular ini) |
| **Audit trail/credential-safety** | `SopRunDetail::from_run()` men-scrub kredensial dari SETIAP field sebelum keluar — step output, tool args, trigger topic | ✅ Defense-in-depth, konsisten temuan riset permission sebelumnya |

### 1a. Role Enforcement per-Step — Bukan Sekadar Label, Tapi Dipaksa Arsitektur

Komentar kode di `SopRunAction::ExecuteStep` (sop/types.rs) adalah salah satu invariant paling tegas yang saya temukan di seluruh audit sesi ini:

> *"Consumers MUST run the step AS that effective agent... The step runs on an EXPLICIT child transcript... the parent turn's conversation is never sent to the step agent's provider... A driver path that does NOT carry it MUST fail the cross-agent step CLOSED rather than run it with the parent's context... a cross-agent step never executes with a broader scope than its own agent — it is re-assembled or it is refused."*

Ini **persis** prinsip yang dibahas di dua dokumen riset sebelumnya (ceiling permission, sub-agent inheritance) — tapi diterapkan di level SOP step, bukan cuma delegasi tool. Kalau step SOP ditugaskan ke "Backend-Rust" role, step itu **tidak bisa** diam-diam jalan dengan context/tool-scope milik agent lain — fail-closed, bukan fail-open.

### 1b. Yang Belum Ada: Template Role/Persona per-Crew (Hanya Ada 1 Template Generik)

`crates/clawcrew-runtime/src/agent/personality_templates/AGENTS.md` adalah template Do's/Don'ts yang **sudah ada dan bagus**, tapi untuk **satu persona generik** ("Personal Assistant"), bukan per-role Crew seperti pola `squad-backend`/`squad-frontend`/`squad-qa` yang dipakai di platform KiroCrew-dashboard (sesi kerja ini sendiri).

Kutipan nyata dari template yang sudah ada (format Do's/Don'ts eksplisit, persis yang Owner minta):

```markdown
## Safety
- Don't exfiltrate private data. Ever.
- Don't run destructive commands without asking.
- `trash` > `rm` (recoverable beats gone forever)
- When in doubt, ask.

## External vs Internal
**Safe to do freely:** Read files, explore, organize, learn, search the web.
**Ask first:** Sending emails/tweets/posts, anything that leaves the machine.
```

**Gap konkret:** direktori `personality_templates/` hanya punya 1 set file (`AGENTS.md`, `SOUL.md`, `USER.md`, `MEMORY.md`, `IDENTITY.md`, `TOOLS.md`, `HEARTBEAT.md`) — tidak ada template per-role (`AGENTS.backend.md`, `AGENTS.reviewer.md`, dst.) yang mendefinisikan Do's/Don'ts SPESIFIK untuk peran teknis (Backend Engineer, Code Reviewer, QA, dst.), berbeda dari persona asisten personal.

### 1c. Quest (Go `mission` package) — Skeleton, Bukan Sistem Matang

```go
// engine/src/mission/interfaces.go — SELURUH isi file
type Quest struct {
    ID           string `json:"id"`
    Status       string `json:"status"`
    TargetShipID string `json:"target_ship_id"`
    WorkflowID   string `json:"workflow_id"`
}
```

Dibandingkan `SopRun` di Rust (puluhan field: step_results, failure_reason, revision tracking, waiting_since, dll), `Quest` Go **baru skeleton 4 field**. `Quest.WorkflowID` menunjukkan quest dimaksudkan sebagai pembungkus organisasi (Fleet/Ship level) di atas `WorkflowTemplate` (`engine/src/workflow/dto.go`), tapi implementasinya belum menyusul.

`WorkflowTemplate.StepTemplate.AgentRole` (Go) **sudah** punya field role eksplisit (`AgentRole string`) — ini jawaban langsung untuk "role per step" dari sisi Go, konsisten dengan `SopStep.agent` di Rust. Tapi karena `Quest` masih skeleton, belum jelas bagaimana `AgentRole` ini di-resolve ke Crew/Squad nyata di hierarki Fleet→Ship→Squad→Crew.

---

## 2. Perbandingan Konkret dengan n8n

Sumber: `n8n.io` blog "15 best practices for deploying AI agents in production", `docs.n8n.io/flow-logic/error-handling/`, komunitas n8n.

### 2.1 Error Handling — n8n Punya Satu Pola Terpusat, Galleon Punya Field per-Step

| Pola n8n | Pola Galleon (`sop::types`) | Perbandingan |
|---|---|---|
| **Error Trigger node** terpisah — satu workflow "Error Handler" didaftarkan sebagai `Error Workflow` di settings; SEMUA workflow lain reroute ke sana saat gagal | `StepFailure` (`on_failure`) per-step, `SopRunStatus::Failed` dengan `failure_reason` | n8n = terpusat (satu error-workflow untuk banyak workflow); Galleon = terdistribusi (per-step policy). **Galleon TIDAK punya pola terpusat setara "Error Trigger"** — kalau beberapa SOP butuh respons gagal yang sama (notifikasi Slack, log ke sistem eksternal), itu harus diulang per-SOP, bukan satu handler yang dipakai bersama. |
| **Continue on Fail** (toggle per-node) — node mengembalikan objek error, workflow lanjut jalan | Tidak ditemukan ekuivalen eksplisit — `on_failure` default-nya "fail the run" | Galleon condong ke fail-strict by default (konsisten prinsip fail-closed di seluruh sistem), n8n punya opsi "lanjut meski gagal" eksplisit untuk API non-kritis. Ini PERBEDAAN FILOSOFI yang mungkin disengaja (Galleon lebih konservatif), bukan kekurangan. |
| **Idempotency via unique request_id + Upsert** — n8n secara eksplisit menyarankan ini untuk mencegah duplikasi saat recovery | `SopRun.run_id`, `DeterministicRunState` (resume dari `last_completed_step`) | Galleon SUDAH mendukung resume-dari-checkpoint (lebih baik dari sekadar idempotency key) — `DeterministicRunState` persis mekanisme recovery yang n8n sarankan dibangun manual. |

### 2.2 Role per-Node — n8n Tidak Punya Konsep Ini Secara Native

n8n node adalah "trigger" atau "action" generik — tidak ada konsep built-in "node ini dijalankan sebagai role X dengan permission Y". Role/permission di n8n biasanya ditangani di level kredensial API eksternal (siapa punya API key), bukan di level workflow-engine. **Galleon's `SopStep.agent` + ceiling enforcement (§1a) adalah KELEBIHAN nyata** dibanding n8n untuk kasus "setiap langkah dikerjakan role berbeda dengan scope berbeda" — ini relevan karena Galleon memang didesain untuk organisasi AI, bukan sekadar integrasi API.

### 2.3 Deployment & Monitoring — n8n Lebih Matang untuk Observability Produksi

n8n best-practices (`blog.n8n.io`) menekankan: queue mode untuk scaling, monitoring execution metrics, structured logging per-eksekusi. Galleon's `SopRun`/`SopRunDetail` sudah punya audit trail detail (step-by-step, tool-calls, durasi), tapi saya belum menemukan ekuivalen "queue mode" untuk SOP run yang sangat banyak bersamaan — `max_concurrent` + `SopAdmissionPolicy` menangani SATU SOP, belum tentu scaling LINTAS SOP di level sistem.

---

## 3. Perbandingan dengan Praktik SOP/RACI Dunia Kerja Nyata

Sumber: Atlassian/Asana/Smartsheet RACI chart guides, glitter.io "SOP Checklist: 15 Required Sections".

### 3.1 RACI (Responsible, Accountable, Consulted, Informed) — Standar Universal yang BELUM Direplikasi Eksplisit

RACI adalah matriks assignment tanggung jawab paling umum dipakai di organisasi nyata:
- **Responsible** — yang mengerjakan tugas
- **Accountable** — yang bertanggung jawab atas hasil akhir (biasanya satu orang per tugas, "the buck stops here")
- **Consulted** — diminta pendapat sebelum keputusan
- **Informed** — diberi tahu setelah keputusan/selesai

**Pemetaan ke konsep Galleon yang sudah ada:**

| RACI | Ekuivalen Galleon saat ini | Gap |
|---|---|---|
| Responsible | `SopStep.agent` (effective_agent) — jelas, satu agent per step | ✅ Ada |
| Accountable | ❓ Tidak eksplisit — SOP tidak punya field "siapa accountable kalau run ini gagal total" | **GAP** — berguna untuk SOP kritis (Fleet-level), bukan semua SOP butuh ini |
| Consulted | `SopStepKind::Checkpoint` + `gate_prompt` — mirip tapi bukan "consulted sebelum", lebih ke "approve sebelum lanjut" | ⚠️ Mirip tapi beda semantik — Checkpoint = gate/approval, bukan sekadar minta pendapat tanpa blocking |
| Informed | Audit trail (`SopRunDetail`) + event (`PublishEvent` di Go) — pihak lain BISA tahu, tapi tidak ada "notify X secara eksplisit" sebagai bagian definisi SOP | ⚠️ Mekanisme ada (event system), tapi "siapa harus diberi tahu" tidak dideklarasikan di `Sop` struct |

**Rekomendasi konkret:** Pola Galleon saat ini condong ke "Responsible + Checkpoint-sebagai-Accountable-implisit" — cukup untuk SOP teknis sehari-hari, tapi untuk SOP kritis Fleet-level (sesuai hierarki Quartermaster→Ship→Squad→Crew yang jadi fokus Anda), menambahkan field eksplisit `accountable_role` dan `informed_roles: Vec<String>` di `Sop` struct akan membuat SOP benar-benar merepresentasikan RACI penuh, bukan cuma Responsible.

### 3.2 Struktur SOP Dunia Nyata (15 Bagian Wajib, glitter.io) vs `Sop` Struct Galleon

| Bagian SOP standar dunia kerja | Ekuivalen Galleon | Status |
|---|---|---|
| Judul & tujuan | `Sop.name`, `Sop.description` | ✅ |
| Scope (kapan SOP berlaku) | `SopTrigger` (kapan dipicu) | ✅ — malah lebih presisi (trigger teknis, bukan deskripsi scope naratif) |
| Role & tanggung jawab (RACI) | `SopStep.agent` saja (Responsible) | ⚠️ Parsial, lihat §3.1 |
| Langkah-langkah bernomor | `SopStep.number`, `.title`, `.body` | ✅ |
| **Do's dan Don'ts eksplisit** | ❌ Tidak ada field terstruktur — kalau mau Do/Don't, harus ditulis manual di `SopStep.body` (markdown prosa) | **GAP** — ini jawaban langsung untuk pertanyaan Owner |
| Kondisi pengecualian/edge-case | `StepRouting` (conditional routing), `on_failure` | ✅ |
| Versi & riwayat perubahan | `Sop.version` (semver manual) | ✅ tapi manual, tidak ada changelog terstruktur |
| Kriteria sukses/selesai | `SopStepStatus` (Completed/Failed/Skipped) — tapi ini status TEKNIS, bukan kriteria bisnis "apa artinya step ini berhasil" | ⚠️ Parsial |

**Temuan paling relevan untuk pertanyaan Anda:** `galleon-fleet` **belum punya field terstruktur untuk Do's/Don'ts** di level SOP. Saat ini kalau mau menulis aturan "jangan lakukan X, selalu lakukan Y" untuk suatu prosedur, satu-satunya tempat adalah menulis bebas di `SopStep.body` (markdown prosa yang dibaca LLM) — bukan field terpisah yang bisa divalidasi/ditampilkan terstruktur di UI, atau di-enforce sebagai constraint eksplisit (beda dari sekadar instruksi dalam teks).

---

## 4. Sintesis — Apa yang Sudah Benar vs Apa yang Jadi Gap

**Sudah benar dan matang (jangan diubah):**
- SOP engine Rust (`sop::types`) adalah pondasi solid — role-per-step enforcement sudah fail-closed by design, lebih baik dari n8n untuk kasus multi-role-agent.
- Checkpoint/approval dengan quorum policy sudah ada.
- Audit trail dengan credential-scrubbing konsisten di semua lapis (sudah terbukti berulang di seluruh sesi riset ini).

**Gap konkret (bahan backlog):**
1. **Tidak ada field Do's/Don'ts terstruktur** di `Sop`/`SopStep` — hanya bisa ditulis prosa di `body`. (GAP paling relevan untuk pertanyaan Anda)
2. ~~**Tidak ada template role/persona per-Crew**~~ — **DIRANCANG 2026-10-04**, lihat §5.
3. ~~**RACI belum lengkap**~~ — **DIRANCANG 2026-10-04**, lihat §5.
4. ~~**`Quest` (Go) masih skeleton**~~ — **DIRANCANG 2026-10-04**, lihat §5.
5. **Tidak ada "Error Workflow" terpusat** setara n8n — penanganan gagal terdistribusi per-SOP, bukan satu handler yang dipakai bersama lintas-SOP. **MASIH TERBUKA**, belum masuk design-spec (lihat §5 catatan).

---

## 5. STATUS: Design-Spec Sudah Dibuat (2026-10-04)

Empat dari lima gap di §4 sudah dirancang konkret (bukan lagi "rekomendasi terbuka") di:

> **`standard-docs/decisions/sop-do-dont-raci-role-templates-design-spec.md`**

Isinya: kode Rust/Go siap-tempel untuk (1) field `StepConduct{dos,donts}` di `SopStep`, (2) field RACI (`accountable_role`, `informed_roles`) di `Sop` — reuse `SopStep.policy` yang sudah ada untuk "Consulted", (3) tiga file template role (`AGENTS.backend.md`/`AGENTS.frontend.md`/`AGENTS.qa.md`), (4) `Quest` Go diperluas dengan status enum + lifecycle timestamp + `SopRunIDs`, plus catatan bahwa `Quest` butuh `mission.Service` baru (scope terpisah, belum ada sama sekali).

Task-breakdown eksekusi dari design-spec ini ada di `standard-docs/backlog/` (lihat `sop-raci-role-quest-backlog.md`), bukan diulang di sini — dokumen riset ini tetap jadi rujukan ANALISIS.

**Yang masih terbuka (belum dirancang):** item #5 (Error Workflow terpusat setara n8n) — dinilai belum urgent untuk skala SOP saat ini, dicatat sebagai item masa depan, bukan diabaikan.

---

## 6. Sumber

- Kode diaudit: `crates/clawcrew-runtime/src/sop/types.rs` (seluruh file — `Sop`, `SopStep`, `SopRun`, `SopTrigger`, `SopExecutionMode`, `SopAdmissionPolicy`), `crates/clawcrew-runtime/src/sop/capability/registry.rs`, `crates/clawcrew-runtime/src/agent/personality_templates/AGENTS.md`, `engine/src/mission/interfaces.go` (`Quest`), `engine/src/workflow/dto.go` (`WorkflowTemplate`, `StepTemplate.AgentRole`), `crates/clawcrew-config/src/schema.rs:4077` (`AliasedAgentConfig`)
- n8n error handling: https://docs.n8n.io/flow-logic/error-handling/, https://blog.n8n.io/best-practices-for-deploying-ai-agents-in-production/
- RACI chart: https://www.atlassian.com/work-management/project-management/raci-chart, https://asana.com/resources/raci-chart
- SOP checklist standar: https://www.glitter.io/blog/process-documentation/sop-checklist-essentials
- Dokumen terkait: `capabilities-skills-lessons-steering-audit.md`, `agent-subagent-ship-squad-permission-scheme.md`
