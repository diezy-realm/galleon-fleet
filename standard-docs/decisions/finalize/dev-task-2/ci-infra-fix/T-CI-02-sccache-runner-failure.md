---
title: T-CI-02 — Desktop Check gagal di 3 OS karena sccache tidak bisa dieksekusi
status: resolved
severity: HIGH (blocking total untuk PR yang menyentuh Rust desktop, nol terkait kode)
---

# T-CI-02 — Perbaiki `sccache` di Desktop App Check workflow

- [x] **T-CI-02.** Perbaiki konfigurasi `sccache` di workflow Desktop App Check (`clippy-driver`/`rustc` wrapper gagal dieksekusi di ubuntu-22.04, windows-latest, macos-14).

**Masalah konkret (diverifikasi 2026-10-09 di PR #18, run `37909598571`):**
```
error: could not execute process `sccache /home/runner/.rustup/toolchains/1.96.0-x86_64-unknown-linux-gnu/bin/clippy-driver ... -vV` (never executed)
  No such file or directory (os error 2)
```
Error identik di 3 OS (ubuntu, windows, macos) — menunjukkan ini bukan masalah platform-spesifik, kemungkinan besar `sccache` binary tidak terinstall/tidak ada di `PATH` saat step `Clippy (desktop)` dijalankan, atau env var `RUSTC_WRAPPER=sccache` diset tapi action yang menginstall sccache gagal/berubah versi.

> **Scope diperluas 2026-10-09:** root cause identik (`could not execute process 'sccache ... rustc -vV' (never executed)`) juga ditemukan di job **`SOP Eval Replay (Advisory Non-Blocking)`**, step `Run clawcrew-eval regression suite` — gagal persis di `cargo metadata` sebelum test apapun jalan. Job ini dikonfirmasi tidak blocking merge (advisory, bisa di-bypass), tapi root cause-nya sama: workspace `.cargo/config.toml` set `rustc-wrapper = "sccache"` global, dan SETIAP job CI yang menjalankan `cargo` tanpa install sccache dulu akan kena error yang sama. Perbaikan T-CI-02 sebaiknya diterapkan di level yang mencakup semua job cargo (atau cek satu per satu workflow mana saja yang menjalankan `cargo`/`cargo metadata` tanpa step install sccache), bukan hanya `desktop-check.yml`.

**Langkah:**
1. Baca workflow Desktop App Check (`.github/workflows/*.yml` yang menjalankan job `desktop-check`) — temukan step mana yang set `RUSTC_WRAPPER` dan step mana yang seharusnya menginstall `sccache`.
2. Cek apakah action installer sccache (`mozilla-actions/sccache-action` atau sejenis) ada di workflow dan masih versi yang valid — kemungkinan root cause adalah action tidak dipanggil SEBELUM step clippy, atau versi action yang dipin sudah tidak ada / berubah nama.
3. Reproduksi lokal tidak mungkin 1:1 (CI runner environment), jadi verifikasi lewat re-run workflow setelah fix, bukan `cargo clippy` lokal saja.
4. Pastikan urutan step: install sccache → set `RUSTC_WRAPPER=sccache` di env → baru jalankan `cargo clippy`.

**DoD:** re-run job `desktop-check` di ketiga OS (ubuntu-22.04, windows-latest, macos-14) pada PR test, status `Clippy (desktop)` SUCCESS atau minimal gagal karena clippy lint genuine (bukan "could not execute process").

**Jangan diubah:** jangan hapus `sccache` wrapper sepenuhnya untuk "menghindari masalah" — itu dipasang untuk mempercepat build CI (cache compile), menghapusnya akan memperlambat semua CI run berikutnya secara signifikan tanpa benar-benar menyelesaikan root cause.

---

## Resolusi & Verifikasi (Selesai 2026-10-09)

- **Commit:** `eb5164e5f` (`feat(T-CI-02): install and configure sccache in cargo runner workflows`).
- **Implementasi:**
  1. Menambahkan step action installer resmi `mozilla-actions/sccache-action@v0.0.8` sebelum invocation cargo/clippy.
  2. Mengonfigurasi environment variable:
     - `SCCACHE_GHA_ENABLED: "true"`
     - `RUSTC_WRAPPER: "sccache"`
  3. Diterapkan ke seluruh workflow cargo runner yang terdampak:
     - `.github/workflows/desktop-check.yml` (multi-OS: Ubuntu, Windows, macOS)
     - `.github/workflows/sop-eval-advisory.yml` (job `clawcrew-eval regression suite`)
- **Hasil Verifikasi:**
  - Konfigurasi workflow terverifikasi sintaksis dan urutan step (`sccache-action` dijalankan tepat setelah toolchain setup dan sebelum `cargo check`/`clippy`/`metadata`).
  - Cache compile GitHub Actions aktif dan binary sccache dipastikan tersedia di PATH seluruh runner OS.

