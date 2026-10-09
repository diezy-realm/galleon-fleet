---
title: T-CI-02 — Desktop Check gagal di 3 OS karena sccache tidak bisa dieksekusi
status: open
severity: HIGH (blocking total untuk PR yang menyentuh Rust desktop, nol terkait kode)
---

# T-CI-02 — Perbaiki `sccache` di Desktop App Check workflow

- [ ] **T-CI-02.** Perbaiki konfigurasi `sccache` di workflow Desktop App Check (`clippy-driver`/`rustc` wrapper gagal dieksekusi di ubuntu-22.04, windows-latest, macos-14).

**Masalah konkret (diverifikasi 2026-10-09 di PR #18, run `37909598571`):**
```
error: could not execute process `sccache /home/runner/.rustup/toolchains/1.96.0-x86_64-unknown-linux-gnu/bin/clippy-driver ... -vV` (never executed)
  No such file or directory (os error 2)
```
Error identik di 3 OS (ubuntu, windows, macos) — menunjukkan ini bukan masalah platform-spesifik, kemungkinan besar `sccache` binary tidak terinstall/tidak ada di `PATH` saat step `Clippy (desktop)` dijalankan, atau env var `RUSTC_WRAPPER=sccache` diset tapi action yang menginstall sccache gagal/berubah versi.

**Langkah:**
1. Baca workflow Desktop App Check (`.github/workflows/*.yml` yang menjalankan job `desktop-check`) — temukan step mana yang set `RUSTC_WRAPPER` dan step mana yang seharusnya menginstall `sccache`.
2. Cek apakah action installer sccache (`mozilla-actions/sccache-action` atau sejenis) ada di workflow dan masih versi yang valid — kemungkinan root cause adalah action tidak dipanggil SEBELUM step clippy, atau versi action yang dipin sudah tidak ada / berubah nama.
3. Reproduksi lokal tidak mungkin 1:1 (CI runner environment), jadi verifikasi lewat re-run workflow setelah fix, bukan `cargo clippy` lokal saja.
4. Pastikan urutan step: install sccache → set `RUSTC_WRAPPER=sccache` di env → baru jalankan `cargo clippy`.

**DoD:** re-run job `desktop-check` di ketiga OS (ubuntu-22.04, windows-latest, macos-14) pada PR test, status `Clippy (desktop)` SUCCESS atau minimal gagal karena clippy lint genuine (bukan "could not execute process").

**Jangan diubah:** jangan hapus `sccache` wrapper sepenuhnya untuk "menghindari masalah" — itu dipasang untuk mempercepat build CI (cache compile), menghapusnya akan memperlambat semua CI run berikutnya secara signifikan tanpa benar-benar menyelesaikan root cause.
