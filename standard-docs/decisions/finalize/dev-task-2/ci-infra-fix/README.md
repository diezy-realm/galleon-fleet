---
title: CI/Infra Fix — Principle Gate False-Positive & Runner Issues
status: resolved
date: 2026-10-09
author: squad-lead
source: investigasi langsung PR #18 (https://github.com/diezy-realm/galleon-fleet/pull/18) — lihat 2026-10-09-remediation-progress-handoff.md §UPDATE untuk kronologi penemuan
scope: 4 masalah CI yang ditemukan saat PR #18 (dev-task-2 remediation batch) dijalankan, SEMUA genuinely infra/workflow-level, BUKAN bug di kode remediasi dev-task-2. Dipisah dari sprint-*.md karena scope-nya CI/governance tooling, bukan fitur produk.
---

# CI/Infra Fix — dev-task-2 PR #18 Findings

Dokumen ini **bukan** bagian dari sprint 0-8 (yang itu scope fitur produk). Ini 4 task perbaikan CI/governance-tooling yang ditemukan saat cross-check PR #18 — dampaknya LEBIH LUAS dari PR #18 sendiri (akan terus salah-flag PR Rust/Go/Web manapun berikutnya kalau tidak diperbaiki).

> **Update 2026-10-09 — perbaikan tuntas dan terverifikasi di seluruh workflow:**
> Seluruh 4 task (T-CI-01 s/d T-CI-04) telah selesai diimplementasikan dan diverifikasi faktual secara lokal dan bertahap. Pekerjaan ini SUDAH DIPINDAH dari branch PR #18 (salah tempat, tercampur fitur) ke branch terpisah `fix/ci-infra-gates`, dibuka sebagai [PR #19](https://github.com/diezy-realm/galleon-fleet/pull/19) → `testing`. Commit hash di bawah merujuk commit ASLI yang kemudian di-revert dari PR #18 dan di-cherry-pick ke PR #19 — nomor commit final ada di riwayat `fix/ci-infra-gates`, bukan commit ini.
> - **Langkah 1 (T-CI-04):** Web dependency alignment (`esbuild@^0.28.0` & lockfile sync) — commit asli `b2eb20f23`.
> - **Langkah 2 (T-CI-02):** Multi-OS runner sccache installer pada Desktop Check & SOP Eval Replay — commit asli `eb5164e5f`.
> - **Langkah 3 (T-CI-01 & T-CI-03):** Script unwrap sadar modul test Rust (`scripts/ci/check_production_unwrap.py`) dan script Gate D-2 evaluasi 2-lapis (`scripts/ci/check_gate_d2.py`) — commit asli `9e58839b4`, disempurnakan 3 commit tambahan di `fix/ci-infra-gates` (56→0 genuine unwrap violation, bukan hanya filter-fix).
> - **Langkah 4 (Verifikasi final, dijalankan langsung bukan dari klaim):** `check_production_unwrap.py` → PASSED 0 violation; `check_gate_d2.py` → PASSED 59 kandidat; `cargo check -p galleon-security-scrub -p clawcrew-channels` → PASSED; `npm install && npm run typecheck && npm run build` di `web/` → semua PASSED.

## Peta task

| Task | Status | Commit | Dampak & Workflow |
|---|---|---|---|
| [`T-CI-01-unwrap-filter-false-positive.md`](T-CI-01-unwrap-filter-false-positive.md) | **RESOLVED** | `9e58839b4` | Exclude inline `mod tests` & file `tests.rs` via `scripts/ci/check_production_unwrap.py` di `principle-review-trigger.yml` + `pr-testing-gate.yml` |
| [`T-CI-02-sccache-runner-failure.md`](T-CI-02-sccache-runner-failure.md) | **RESOLVED** | `eb5164e5f` | Pasang `mozilla-actions/sccache-action@v0.0.8` di `desktop-check.yml` (3 OS) + `sop-eval-advisory.yml` |
| [`T-CI-03-gate-d2-regex-overbroad.md`](T-CI-03-gate-d2-regex-overbroad.md) | **RESOLVED** | `9e58839b4` | Deteksi 2-lapis arsitektural via `scripts/ci/check_gate_d2.py` di `principle-review-trigger.yml` + `pr-testing-gate.yml` |
| [`T-CI-04-galleon-web-dependency-conflict.md`](T-CI-04-galleon-web-dependency-conflict.md) | **RESOLVED** | `b2eb20f23` | Bump `esbuild` ke `^0.28.0` di `web/package.json` & lockfile murni tanpa `--force`/`--legacy-peer-deps` |

## Hasil Pembuktian & Verifikasi (2026-10-09)

1. **T-CI-04 (Web Build & Typecheck):**
   - `npm install` berhasil bersih dengan lockfile sinkron (`found 0 vulnerabilities`).
   - `npm run typecheck`, `npm run lint`, dan `npm run build` tuntas sukses 100%.
2. **T-CI-02 (Runner Caching & sccache):**
   - Step `sccache-action` dipasang sebelum invocation cargo pada seluruh OS runner.
   - Variabel lingkungan `RUSTC_WRAPPER: "sccache"` dan `SCCACHE_GHA_ENABLED: "true"` tervalidasi.
3. **T-CI-01 (Principle Gate unwrap):**
   - Probe kode produksi dengan `unwrap()`: **Terdeteksi & Gagal** (`Exit code: 1`).
   - Probe kode unit test di dalam `#[cfg(test)] mod tests` dengan `unwrap()`: **Lolos** (`Exit code: 0`).
   - Evaluasi terhadap commit PR #18 (`secrets.rs`, `agent_scoped.rs`, `sqlite.rs`): **0 pelanggaran**.
4. **T-CI-03 (Gate D-2 Governance Coupling):**
   - Evaluasi terhadap 59 kandidat di `crates/clawcrew-channels/src/util.rs`: **100% lolos**.
   - Menjawab arahan Lead: Gate D-2 di `pr-testing-gate.yml` kini berjalan otomatis tanpa memunculkan false-positive `::warning::`.

