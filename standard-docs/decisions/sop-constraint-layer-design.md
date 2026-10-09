---
title: Riset & Desain — SOP Kompleks dengan Role-Based Constraint (Two-Layer: Flow + Constraint)
status: proposal
date: 2026-10-05
author: squad-lead
depends_on:
  - research/orders-quest-sop-workflow-role-audit.md
  - standard-docs/decisions/sop-do-dont-raci-role-templates-design-spec.md
context: >
  Contoh konkret Achmad: "Lead membuat Task/Architecture/diagram, Lead mengkoordinasikan ke
  Backend/Frontend sesuai task masing-masing, Lead TIDAK BOLEH coding (hanya BE/FE boleh),
  Lead mengecek semua hasil BE/FE." SOP dunia nyata "berisi aturan ketat dan panjang", lebih
  kompleks dari diagram arrow linear ala n8n.
---

# Riset & Desain: Merepresentasikan SOP Kompleks dengan Larangan Berbasis Role

## 1. Temuan Riset Paling Penting — Diagram Arrow TIDAK BISA Merepresentasikan Larangan

**BPMN (standar formal business process) sendiri tidak punya notasi untuk "role X dilarang melakukan Y".** Swimlane BPMN hanya POSITIVE assignment (siapa MELAKUKAN apa) — tidak ada primitive deny/prohibition native. Dikonfirmasi lintas sumber (Oracle BPM docs, Visual Paradigm). Solusi akademiknya (Wolter & Schaad, BPM 2007) adalah **menempelkan authorization-constraint artifact TERPISAH** ke diagram, dikompilasi ke XACML untuk enforcement runtime — bukan bagian native BPMN.

**Bahkan workflow engine enterprise (Camunda, jBPM) TIDAK punya Separation-of-Duty (SoD) built-in.** Dikonfirmasi langsung dari forum Camunda: `candidateGroups` cuma assignment/visibility, bukan constraint engine. Tim harus hand-roll via custom header (`ExcludeUsersFromActivities`) dicek lewat gateway expression. **Camunda 8.8 bahkan menghapus task-visibility enforcement bawaan**, mendorong lebih jauh ke authorization service eksternal.

**Konklusi langsung untuk kasus Anda**: mencoba encode "Lead tidak boleh coding" sebagai **arrow yang hilang di DAG adalah representasi yang SALAH secara struktural** — BPMN sendiri tidak bisa, Camunda/jBPM juga tidak built-in. Pola yang BENAR secara konsisten di seluruh literatur (BPMN-extension, XACML, Petri-net RBAC, paper agent 2026) adalah **two-layer design**.

## 2. Istilah Formal: Separation of Duties (SoD)

Ini adalah istilah akademik/industri standar untuk persis kasus Anda (Nash & Poland 1990; Botha & Eloff 2001 — "canonical workflow treatment"). Dua jenis:

- **Static SoD (SSoD)**: user tidak boleh memegang DUA role yang konflik secara bersamaan — dicek saat ASSIGNMENT role, independen dari instance workflow mana pun. (Contoh: satu Crew tidak boleh didaftarkan sebagai "Lead" DAN "Backend" sekaligus untuk Ship yang sama.)
- **Dynamic SoD (DSoD)**: user BOLEH memegang dua role, tapi tidak boleh MENGERJAKAN dua tugas yang konflik dalam SATU case/instance yang sama — dicek saat eksekusi terhadap riwayat instance itu. (Contoh: kalau SATU agent dikasih role ganda, dia tetap tidak boleh coding DAN approve codingnya sendiri di SATU SopRun yang sama.)

**Representasi struktural (bukan prosa)** yang konsisten dipakai lintas standar:

```rust
enum Constraint {
    /// SoD: dua aksi tidak boleh terjadi dalam case/instance yang sama.
    NotCoExistence(ActivityRef, ActivityRef),
    /// Dependency urutan tanpa memaksa satu jalur rigid.
    Precedence(ActivityRef, ActivityRef),
    /// Cardinality -- misal "review gate harus terjadi tepat 1x".
    Existence { activity: ActivityRef, min: u32, max: u32 },
    /// Larangan eksplisit: role ini TIDAK BOLEH melakukan aktivitas-aktivitas ini.
    RoleRestriction { role: RoleId, forbidden_activities: Vec<ActivityRef> },
}
```

Ini mengambil pola dari **DECLARE** (van der Aalst/Pesic — alternatif DEKLARATIF dari pemodelan proses imperatif/arrow): bukan menggambar jalur yang DIIZINKAN, tapi mendeklarasikan SET CONSTRAINT — trace manapun yang memenuhi semua constraint adalah valid (open-world). Ini **secara struktural adalah lawan dari arrow n8n**, dan jauh lebih cocok untuk "aturan ketat dan panjang" yang Anda maksud, karena constraint bisa ditambah terus tanpa menggambar ulang seluruh diagram.

## 3. Riset Terkini (2025-2026) — Khusus untuk AI Agent Organization

Tiga paper terbaru yang LANGSUNG relevan untuk Galleon (organisasi AI agent, bukan workflow manusia biasa):

- **"Constrained Process Maps for Multi-Agent Generative AI Workflows"** (arXiv:2602.02034, Feb 2026) — role separation adalah STRUKTURAL: setiap role = satu node keputusan MDP dengan transition set sendiri, bukan tempelan aturan teks.
- **"A Layered Organizational Design Framework for Agent-Native Organizations"** (arXiv:2608.08516) — memisahkan tegas **Permission** (tool/info apa yang bisa diakses) dari **Privilege** (state change apa yang boleh diotorisasi), dikompilasi PER-TASK jadi scoped "world" tempat agent beroperasi. **Ini analog PALING DEKAT dengan "Lead tidak boleh coding"**: bukan properti arrow workflow, tapi capability scope yang di-resolve PER ROLE sebelum task dimulai.
- **"Dynamic Capability Scoping for Enterprise AI Agents"** (arXiv:2607.22445) — arsitektur permission 3-sumber: role-based ceiling + task-context classifier + aturan "combination prohibition" eksplisit (persis deny-rule XACML, tapi dihitung dinamis dari konteks task).

## 4. Desain Two-Layer untuk SOP Engine Galleon

### Layer 1 — Flow (yang SUDAH ada, TIDAK diubah)

`SopStep`/`SopRun`/`StepRouting` di `crates/clawcrew-runtime/src/sop/types.rs` — DAG/urutan step, siapa EKSEKUSI apa (`SopStep.agent`), kapan (`SopTrigger`), gimana (`SopExecutionMode`). **Ini layer yang sudah matang, cocok untuk "siapa harus melakukan apa secara positif" — JANGAN diubah atau dipaksa menangkap larangan.**

### Layer 2 — Constraint (BARU, terpisah, menjawab "Lead tidak boleh coding")

Tambah struct terpisah yang di-attach ke `Sop` (bukan ke `SopStep` individual — constraint berlaku LINTAS step dalam satu run):

```rust
/// Constraint lintas-step dalam satu SOP -- menjawab "role X dilarang Y",
/// berbeda axis dari SopStep.agent (yang menjawab "role X MENGERJAKAN Y").
pub struct SopConstraint {
    pub kind: ConstraintKind,
}

pub enum ConstraintKind {
    /// SoD: role ini dilarang mengerjakan step dengan tool/kind tertentu.
    /// Contoh: RoleRestriction { role: "Lead", forbidden_tool_kinds: [Write, Shell] }
    RoleRestriction { role: String, forbidden_tool_kinds: Vec<ToolKind> },
    /// Dua step tidak boleh dieksekusi oleh agent yang SAMA dalam SATU run
    /// (Dynamic SoD) -- misal step "implement" dan step "approve" harus beda performer.
    DistinctPerformer { step_a: StepRef, step_b: StepRef },
    /// Step harus terjadi persis N kali (cardinality/review-gate wajib).
    RequiredOccurrence { step: StepRef, min: u32, max: u32 },
}
```

**Titik enforcement**: sejalan desain hook `preToolUse` yang sudah dibahas sebelumnya — SEBELUM sebuah step/tool dieksekusi, `SopEngine` mengecek `SopConstraint` milik run tersebut terhadap (role yang akan eksekusi, tool/kind yang akan dipanggil, riwayat step yang sudah terjadi di run itu). Kalau melanggar `RoleRestriction` atau `DistinctPerformer`, step DITOLAK sebelum jalan — persis exit-code-2 `preToolUse` Kiro, bukan sekadar warning di teks steering.

### Mengapa Dua Layer, Bukan Satu

Mencoba memasukkan larangan ke `SopStep.agent` (Layer 1) akan membuat satu field menjawab dua pertanyaan berbeda ("siapa HARUS" vs "siapa TIDAK BOLEH") — ini PERSIS kesalahan yang riset buktikan BPMN/Camunda hindari dengan sengaja (constraint selalu artifact terpisah, dikompilasi ke policy enforcement sendiri). Memisahkan juga membuat `SopConstraint` reusable LINTAS Ship/Squad — constraint "Lead tidak boleh coding" (Squad Engineering) dan "Copywriter tidak boleh publish tanpa approval" (Squad Marketing) adalah instance `RoleRestriction` yang SAMA strukturnya, cuma beda role/forbidden_tool_kinds — konsisten dengan koreksi Anda soal generic department-agnostic.

## 5. Contoh Konkret: Mengkodekan Kasus Anda

```rust
// SOP: "Engineering Feature Delivery"
Sop {
    name: "engineering-feature-delivery",
    steps: vec![
        SopStep { number: 1, title: "Architecture & Task Breakdown", agent: Some("Lead"), .. },
        SopStep { number: 2, title: "Implement Backend", agent: Some("Backend"), .. },
        SopStep { number: 3, title: "Implement Frontend", agent: Some("Frontend"), .. },
        SopStep { number: 4, title: "Review & Approve", agent: Some("Lead"), .. },
    ],
    constraints: vec![
        // "Lead tidak boleh coding" -- larangan tool, bukan larangan step tertentu
        SopConstraint { kind: RoleRestriction {
            role: "Lead".into(),
            forbidden_tool_kinds: vec![ToolKind::FileWrite, ToolKind::Shell],
        }},
        // Step 1 (buat arsitektur) dan step 4 (approve) boleh sama2 Lead -- TIDAK
        // perlu DistinctPerformer di sini, karena constraint-nya bukan soal
        // performer yang sama, tapi soal TOOL yang dipanggil.
    ],
}
```

Kalau model yang berperan sebagai "Lead" mencoba memanggil tool `file_write` di step mana pun dalam SOP ini, `SopEngine` menolaknya SEBELUM eksekusi — bukan karena stepnya salah, tapi karena ROLE-nya dilarang tool itu, di MANA PUN dalam run.

## 6. Rekomendasi Implementasi

1. **Jangan desain ulang Layer 1** (`SopStep`/flow) — sudah matang dan benar untuk tugasnya.
2. **Tambah `SopConstraint` sebagai layer terpisah** di `Sop` struct — scope awal: `RoleRestriction` dan `DistinctPerformer` (dua pola paling umum dari contoh Anda dan riset SoD), tunda `RequiredOccurrence` (cardinality) sampai ada kasus nyata yang butuh.
3. **Enforcement lewat hook `preToolUse`** yang sudah ada infrastrukturnya (`crates/clawcrew-runtime/src/hooks/`, `HookResult::Cancel`) — SOP engine mendaftarkan hook dinamis per-run yang mengecek `SopConstraint` sebelum tool dieksekusi.
4. **Generic lintas departemen**: `ToolKind`/`role: String` bebas, sama seperti `CrewMember.Role` di Go — tidak hardcode ke istilah engineering.

## Sumber

Lihat sitasi arXiv/DOI/URL inline di tiap bagian (Wolter & Schaad BPM 2007, Ahn & Sandhu ACM TISSEC 2000, XACML RBAC Profile OASIS, Basin et al. ETH Zürich, arXiv:2602.02034, arXiv:2608.08516, arXiv:2607.22445, arXiv:2412.10152 DECLARE-to-ASP). Riset dilakukan subagent 2026-10-05, diverifikasi via web_search ke dokumentasi/paper resmi.
