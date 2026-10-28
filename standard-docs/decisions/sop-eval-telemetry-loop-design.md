---
title: Desain — Evaluation/Telemetry Loop Formal untuk SOP Engine Galleon (Advisory, Divalidasi via Squad KiroCrew)
status: proposal
date: 2026-10-05
author: squad-lead
depends_on:
  - research/agentic-ai-fundamentals-and-lifecycle-phase.md
context: >
  Koreksi Achmad (2026-10-05) atas desain awal: (1) hierarki eksekusi Lead/Principle/Quartermaster
  yang menjalankan, Owner hanya cek kesesuaian; (2) mode ADVISORY bukan blocking, mencegah
  false-positive "dikira bug"; (3) scope SOP yang benar = struktur SopRun/SopStep di SOP ENGINE
  GALLEON (crates/clawcrew-runtime/src/sop/), BUKAN dokumen markdown -- divalidasi lewat skenario
  kerja Lead/Principle/Backend squad KiroCrew sebagai test-case nyata.
---

# Desain: Evaluation/Telemetry Loop Formal untuk SOP Engine Galleon

## 1. Verifikasi Kode — Infrastruktur yang SUDAH Ada (Bukan Dibangun dari Nol)

**`galleon-eval`** (`crates/galleon-eval/src/`): harness evaluation matang — mode `Replay` (deterministik, scripted LLM response, zero-cost) dan `Live`, dengan `grader.rs`, `report.rs` (`CaseReport`/`SuiteReport`), CLI command `eval` sudah terhubung ke binary utama (`src/commands/eval.rs`).

**`clawcrew-runtime/src/sop/metrics.rs`**: `MetricCounters`/`RunSnapshot` sudah menghitung metrik SOP end-to-end — runs completed/failed/cancelled, steps executed/failed/skipped, human approvals, windowed aggregation (ring buffer 1000 run, ~90 hari).

**Gap konkret (terverifikasi grep, bukan asumsi)**: `galleon-eval` nol dipanggil di `.github/workflows/`; `SopCounters` nol disurfaceкan ke `galleon-gateway/src/`. Infrastruktur ADA, tidak tersambung.

**Koreksi penting soal struktur SOP**: `SopRun`/`SopStep`/`SopExecutionMode` (`crates/clawcrew-runtime/src/sop/types.rs`) adalah struktur GENERIK (priority, execution mode Auto/Supervised/StepByStep/PriorityBased/Deterministic). `ApprovalPrincipal` (`sop/approval/principal.rs`) hanya mencatat WHO/transport sebuah approval gate diselesaikan (Agent/Cli/Ws/Http/System/Channel) — **TIDAK ADA konsep native "role Lead/Principle/Quartermaster"** di SOP engine. Jadi validasi lewat skenario squad berarti MEMODELKAN flow kerja nyata sebagai `SopRun` konkret, bukan mencari fitur role yang sudah ada.

## 2. Desain Loop (3 Koreksi Diterapkan)

### Hierarki Eksekusi (Koreksi #1)

```
Lead      -> Tulis/pilih SOP test-case yang merepresentasikan flow kerja nyata
             (misal: "security-fix-flow" sebagai SopRun berisi step
             spec-review -> implement -> principle-gate -> merge)
Backend   -> Eksekusi SOP test-case lewat galleon-eval mode Replay
             (deterministik, tidak butuh LLM call nyata/biaya)
Principle -> Review report (CaseReport/SuiteReport) + SopCounters hasil run,
             putuskan: SOP engine berperilaku sesuai ekspektasi atau tidak
Quartermaster -> (jika scope mencakup koordinasi lintas-squad) agregasi hasil
             dari beberapa SOP test-case, laporkan ringkasan ke Owner
Owner (Achmad) -> HANYA menerima ringkasan "sudah dijalankan sesuai / ada
             penyimpangan yang perlu diputuskan" -- tidak menjalankan tool
             eval atau membaca report mentah secara rutin
```

### Mode Advisory, Bukan Blocking (Koreksi #2)

- `galleon-eval` dijalankan di CI sebagai **check TERPISAH, non-blocking** (status "neutral"/report comment di PR, BUKAN required-status-check yang mencegah merge).
- Alasan eksplisit: SOP engine dan skenario kerja squad masih berkembang — gate yang terlalu strict di awal berisiko membuat kegagalan eval (yang mungkin karena skenario test belum matang, bukan bug nyata) disalahartikan sebagai bug produksi. Advisory memberi ruang kalibrasi dulu.
- **Kriteria kapan boleh naik jadi blocking** (keputusan masa depan, BUKAN sekarang): setelah minimal 1 siklus penuh (misal 1 bulan) eval berjalan advisory tanpa false-positive signifikan, Principle/Lead bisa mengusulkan upgrade ke blocking untuk SOP test-case yang sudah terbukti stabil.

### Scope SOP yang Benar (Koreksi #3)

Eval case TIDAK dibuat dari dokumen `.md` manapun. Eval case dibuat sebagai **`SopRun` nyata** yang:
1. Step-nya memodelkan tahapan kerja squad yang sudah established (lesson: Lead→Backend→Principle→merge, atau Lead→Design→Lead→FE untuk kasus design request).
2. Dieksekusi lewat `galleon-eval` mode `Replay` — scripted LLM response, supaya deterministik dan bisa dijalankan berulang tanpa biaya/variabilitas model.
3. Hasil (`SopCounters` + `CaseReport`) diperiksa Principle untuk konfirmasi SOP ENGINE (bukan squad manusia) berperilaku benar — misal: step approval gate benar-benar menunggu principal yang tepat, step yang gagal benar-benar tercatat sebagai `steps_failed`, dst.

## 3. Rencana Kerja Konkret (Draft untuk Backlog)

1. **Definisikan 2-3 SOP test-case awal** yang memodelkan flow squad paling established (Lead→Backend→Principle→merge testing-PR; Lead→Design→Lead→FE). Lead menulis struktur `SopRun`/`SopStep` untuk ini.
2. **Hubungkan `galleon-eval` ke CI sebagai advisory check** — satu job baru di `.github/workflows/`, tidak masuk required-status-check.
3. **Expose `SopCounters` ke satu endpoint read-only** di `galleon-gateway` (minimal: JSON dump periodik, BUKAN dashboard UI penuh di tahap awal — itu over-scope untuk kebutuhan advisory).
4. **Principle menjalankan siklus pertama** dan melaporkan ke Quartermaster/Lead: SOP test-case lolos/gagal, dengan alasan konkret kalau gagal.
5. **Owner (Achmad) menerima ringkasan satu-baris per siklus** — "SOP eval: 2/2 lolos" atau "SOP eval: 1 penyimpangan, [detail singkat], menunggu keputusan" — bukan laporan teknis panjang.

## 4. Yang SENGAJA Tidak Dikerjakan Dulu (Sejalan Prinsip Advisory + YAGNI)

- **Dashboard UI penuh untuk `SopCounters`** — endpoint read-only cukup di awal; UI visual hanya kalau terbukti dibutuhkan setelah beberapa siklus.
- **Blocking CI gate** — ditunda sampai siklus advisory terbukti stabil (lihat §2).
- **Eval case untuk SEMUA SOP yang mungkin ada** — mulai dari 2-3 yang paling representatif, bukan cakupan penuh di awal.

## Sumber

- `crates/galleon-eval/src/{lib,runner,case,grader,report}.rs`, `src/commands/eval.rs`.
- `crates/clawcrew-runtime/src/sop/{types,metrics,approval/principal}.rs`.
- Verifikasi grep: `.github/workflows/` (nol `galleon-eval`), `galleon-gateway/src/` (nol `SopCounters`).
- `research/agentic-ai-fundamentals-and-lifecycle-phase.md` §6 — kerangka maturity model yang memotivasi loop ini.
