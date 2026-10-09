---
title: CI/Infra Fix — Principle Gate False-Positive & Runner Issues
status: open
date: 2026-10-09
author: squad-lead
source: investigasi langsung PR #18 (https://github.com/diezy-realm/galleon-fleet/pull/18) — lihat 2026-10-09-remediation-progress-handoff.md §UPDATE untuk kronologi penemuan
scope: 4 masalah CI yang ditemukan saat PR #18 (dev-task-2 remediation batch) dijalankan, SEMUA genuinely infra/workflow-level, BUKAN bug di kode remediasi dev-task-2. Dipisah dari sprint-*.md karena scope-nya CI/governance tooling, bukan fitur produk.
---

# CI/Infra Fix — dev-task-2 PR #18 Findings

Dokumen ini **bukan** bagian dari sprint 0-8 (yang itu scope fitur produk). Ini 4 task perbaikan CI/governance-tooling yang ditemukan saat cross-check PR #18 — dampaknya LEBIH LUAS dari PR #18 sendiri (akan terus salah-flag PR Rust/Go/Web manapun berikutnya kalau tidak diperbaiki).

**Jangan centang checkbox manapun di bawah hanya karena logic/script sudah diubah** — DoD setiap task mensyaratkan PR TEST nyata yang membuktikan behavior lama vs baru, karena ini task memperbaiki *gate itu sendiri* (kalau gate-nya salah pakai cara yang sama untuk verifikasi, risiko bug berulang).

## Peta task

| Task | Dampak | Prioritas | File |
|---|---|---|---|
| [`T-CI-01-unwrap-filter-false-positive.md`](T-CI-01-unwrap-filter-false-positive.md) | Setiap PR Rust besar — salah-flag inline `#[cfg(test)] mod tests` sebagai production unwrap | **Tinggi** (blocking, luas) | Principle Gate workflow |
| [`T-CI-02-sccache-runner-failure.md`](T-CI-02-sccache-runner-failure.md) | Setiap PR yang menyentuh `apps/tauri/`+Rust di 3 OS (Desktop Check) | **Tinggi** (blocking, luas) | Desktop App Check workflow |
| [`T-CI-03-gate-d2-regex-overbroad.md`](T-CI-03-gate-d2-regex-overbroad.md) | Setiap PR yang menyentuh `clawcrew-channels/src/util.rs` dengan cfg-gate per-channel baru | Menengah | Principle Gate workflow |
| [`T-CI-04-galleon-web-dependency-conflict.md`](T-CI-04-galleon-web-dependency-conflict.md) | Setiap PR yang menyentuh `web/` — blocking total, nol terkait kode | **Tinggi** (blocking total, tapi fix sederhana) | `web/package.json` |

## Kenapa dipisah dari PR #18 / sprint dev-task-2

PR #18 berisi kerja FITUR (vault rotate, loop-detector wiring, tracing, dll). Ke-4 masalah ini adalah TOOLING yang menilai PR tersebut — mencampur keduanya dalam satu PR akan membuat review Principle (manusia) sulit membedakan "bug kode" vs "bug alat ukur kode". Rekomendasi: PR #18 lanjut ke review manusia dengan catatan manual override untuk 4 check ini (sudah diverifikasi false-positive/infra, lihat investigasi di `2026-10-09-remediation-progress-handoff.md`), sementara 4 task CI-fix ini dikerjakan di PR terpisah yang menyasar `.github/workflows/` dan `web/package.json`.

## Urutan eksekusi yang disarankan

Tidak saling dependen secara teknis — bisa paralel. Tapi serialkan T-CI-01 dan T-CI-03 (keduanya edit file workflow yang sama, `principle-review-trigger.yml`) untuk hindari race-condition edit seperti yang sudah tercatat di lesson project.
