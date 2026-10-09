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

> **Update 2026-10-09 — ditemukan WORKFLOW KEDUA dengan bug yang sama:** investigasi tambahan terhadap 2 check yang sebelumnya belum dianalisis (`Principle Checklist & Standards Gate` dan `SOP Eval Replay`) menemukan root cause T-CI-01 dan T-CI-02 masing-masing muncul di **dua file workflow**, bukan satu:
> - **T-CI-01** (unwrap-filter): bug identik juga ada di `.github/workflows/pr-testing-gate.yml` (job `Principle Checklist & Standards Gate`), bukan hanya `principle-review-trigger.yml`. Karena job itu fail duluan di gate unwrap, Gate D-1/D-2/test suite di workflow yang sama ikut `skipped` — Gate D-2 di `pr-testing-gate.yml` belum pernah genuinely dijalankan untuk PR #18.
> - **T-CI-02** (sccache): root cause identik juga muncul di job `SOP Eval Replay (Advisory Non-Blocking)`, step `cargo metadata`. Job ini **dikonfirmasi non-blocking merge** (advisory, Owner bisa bypass) — tapi root cause sccache globalnya tetap relevan diperbaiki karena dipakai banyak job lain yang genuinely blocking.
> - Detail lengkap + command yang sudah diverifikasi: lihat update di masing-masing file `T-CI-01-unwrap-filter-false-positive.md` dan `T-CI-02-sccache-runner-failure.md`.
> - **Implikasi untuk eksekusi:** memperbaiki SATU file workflow saja (misal hanya `principle-review-trigger.yml`) TIDAK CUKUP membuat PR #18 hijau — `pr-testing-gate.yml` punya bug sama dan harus diperbaiki bersamaan di task yang sama (T-CI-01), bukan task terpisah.

**Jangan centang checkbox manapun di bawah hanya karena logic/script sudah diubah** — DoD setiap task mensyaratkan PR TEST nyata yang membuktikan behavior lama vs baru, karena ini task memperbaiki *gate itu sendiri* (kalau gate-nya salah pakai cara yang sama untuk verifikasi, risiko bug berulang).

## Peta task

| Task | Dampak | Prioritas | File |
|---|---|---|---|
| [`T-CI-01-unwrap-filter-false-positive.md`](T-CI-01-unwrap-filter-false-positive.md) | Setiap PR Rust besar — salah-flag inline `#[cfg(test)] mod tests` sebagai production unwrap. **2 file workflow terdampak:** `principle-review-trigger.yml` + `pr-testing-gate.yml` | **Tinggi** (blocking, luas) | 2 Principle Gate workflows |
| [`T-CI-02-sccache-runner-failure.md`](T-CI-02-sccache-runner-failure.md) | Setiap job CI yang menjalankan `cargo` tanpa install sccache dulu. **Terdampak:** Desktop Check (3 OS, blocking) + SOP Eval Replay (advisory, non-blocking) | **Tinggi** (blocking, luas) | Desktop Check + SOP Eval workflows |
| [`T-CI-03-gate-d2-regex-overbroad.md`](T-CI-03-gate-d2-regex-overbroad.md) | Setiap PR yang menyentuh `clawcrew-channels/src/util.rs` dengan cfg-gate per-channel baru. Berlaku di kedua Principle Gate workflow setelah gate unwrap-nya lolos | Menengah | 2 Principle Gate workflows |
| [`T-CI-04-galleon-web-dependency-conflict.md`](T-CI-04-galleon-web-dependency-conflict.md) | Setiap PR yang menyentuh `web/` — blocking total, nol terkait kode | **Tinggi** (blocking total, tapi fix sederhana) | `web/package.json` |

## Kenapa dipisah dari PR #18 / sprint dev-task-2

PR #18 berisi kerja FITUR (vault rotate, loop-detector wiring, tracing, dll). Ke-4 masalah ini adalah TOOLING yang menilai PR tersebut — mencampur keduanya dalam satu PR akan membuat review Principle (manusia) sulit membedakan "bug kode" vs "bug alat ukur kode". Rekomendasi: PR #18 lanjut ke review manusia dengan catatan manual override untuk 4 check ini (sudah diverifikasi false-positive/infra, lihat investigasi di `2026-10-09-remediation-progress-handoff.md`), sementara 4 task CI-fix ini dikerjakan di PR terpisah yang menyasar `.github/workflows/` dan `web/package.json`.

## Urutan eksekusi yang disarankan

Tidak saling dependen secara teknis — bisa paralel. Tapi serialkan T-CI-01 dan T-CI-03 (keduanya edit file workflow yang sama, `principle-review-trigger.yml` DAN `pr-testing-gate.yml`) untuk hindari race-condition edit seperti yang sudah tercatat di lesson project. T-CI-01 harus menyentuh KEDUA file workflow dalam satu PR/commit yang sama — menyelesaikan satu lalu lupa yang lain akan membuat PR tetap merah di gate yang belum diperbaiki.
