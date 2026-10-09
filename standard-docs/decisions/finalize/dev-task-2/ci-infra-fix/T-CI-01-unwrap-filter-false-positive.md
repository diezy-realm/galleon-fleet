---
title: T-CI-01 — Principle Gate unwrap-filter tidak exclude inline mod tests
status: open
severity: HIGH (blocking, dampak luas — bukan spesifik PR #18)
---

# T-CI-01 — Perbaiki filter `unwrap()`/`expect()` di Principle Gate

- [ ] **T-CI-01.** Ubah metode deteksi `unwrap()`/`expect()` di Principle Gate agar tidak menghitung kode di dalam `#[cfg(test)] mod tests { ... }` inline, atau ganti strategi deteksi ke tool yang paham AST (bukan regex teks diff).

**File:** `.github/workflows/principle-review-trigger.yml`, step `Analyze PR Diff & Invariants`, baris yang menghitung `UNWRAP_DIFF`/`UNWRAP_COUNT`.

**Masalah konkret (diverifikasi 2026-10-09 di PR #18):**
```bash
UNWRAP_DIFF=$(git diff -U0 $BASE_REF...HEAD -- 'crates/**/*.rs' ':!crates/**/tests/**' ':!crates/**/benches/**' | grep '^[+][^+]' | grep -E '\.(unwrap|expect)\(' || true)
```
Filter path hanya mengecualikan folder `tests/` dan file di `benches/`. **Tidak mengecualikan `#[cfg(test)] mod tests { ... }` inline di dalam file produksi** — pola paling umum di Rust (satu file berisi implementasi + modul test di bawahnya). Hasil nyata di PR #18: dari 212 unwrap/expect yang ter-flag, sampel 3 file top (`crates/clawcrew-config/src/secrets.rs` 41 hit, `crates/clawcrew-memory/src/agent_scoped.rs` 10 hit, `crates/clawcrew-memory/src/sqlite.rs` 9 hit — total 60 dari 212) **100% berada di dalam `mod tests`** (dikonfirmasi lewat nomor hunk `git diff -U0`, misal `@@ -2829,0 +2908,178 @@` tepat setelah baris `mod tests {` di `secrets.rs`). Fungsi produksi di file yang sama (`rotate_secret`, `export_encrypted_vault`, `apply_retention`, dll) genuinely nol unwrap — pakai `Result<>`/`?` konsisten.

**Juga ditemukan (scope sekunder, boleh digabung ke task ini):** file bernama `tests.rs` di dalam `src/` (misal `crates/clawcrew-channels/src/matrix/tests.rs`, `src/orchestrator/tests.rs`, dst — 11 file ditemukan di PR #18) juga TIDAK ter-exclude filter `':!crates/**/tests/**'` karena pattern itu match folder bernama `tests/`, bukan file bernama `tests.rs` di folder lain.

**Langkah (pilih salah satu, atau kombinasi A lalu B untuk jangka panjang):**

**Opsi A — perbaiki filter teks (cepat, cukup untuk sekarang):**
1. Tambah exclude pattern file: `':!crates/**/tests.rs'` dan `':!crates/**/*/tests.rs'` (tangani file bernama `tests.rs` di subfolder apa pun) ke command `git diff` yang sudah ada.
2. Untuk `mod tests` inline, filter diff per-hunk: skip hunk yang context line-nya (baris di atas `@@`) berada di dalam blok `mod tests` — ini butuh parsing line-number vs lokasi `mod tests` di file versi baru (`awk`/`perl` dengan state machine, atau panggil `git diff` dengan `-U` besar lalu scan mundur dari setiap match ke `mod tests {` terdekat sebelumnya tanpa `}` penutup di antaranya).
3. Verifikasi: jalankan command baru manual terhadap PR #18 (`git diff -U0 origin/testing...HEAD`) dan pastikan `secrets.rs`/`agent_scoped.rs`/`sqlite.rs` menghasilkan 0, bukan 41/10/9.

**Opsi B — ganti ke `cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used` (lebih robust, scope lebih besar):**
1. Clippy sudah paham AST — otomatis exclude `#[cfg(test)]` kalau dikonfigurasi `#![warn(clippy::unwrap_used)]` di level crate dengan `#[allow(clippy::unwrap_used)]` di `mod tests`, atau pakai `clippy.toml` ignore pattern.
2. Ini scope LEBIH BESAR dari sekadar fix 1 workflow — butuh audit berapa banyak unwrap genuinely ada di produksi lintas workspace dulu (bisa jadi task tersendiri jika jumlahnya besar), karena mengaktifkan deny-level clippy lint akan membuat SEMUA PR lama gagal sampai dibersihkan.
3. **Rekomendasi: Opsi A dulu (cepat, tidak mengubah kontrak gate), Opsi B sebagai task terpisah di masa depan jika tim ingin enforcement lebih kuat.**

**DoD:** jalankan filter baru terhadap diff PR #18 (`git diff -U0 origin/testing...HEAD -- 'crates/**/*.rs' <exclude-baru>`), hasil `grep '^[+][^+]' | grep -E '\.(unwrap|expect)\('` = 0 match untuk `secrets.rs`/`agent_scoped.rs`/`sqlite.rs` (atau match yang tersisa genuinely di luar `mod tests`, didokumentasikan file:baris). Buka PR test kecil (tambah 1 unwrap di `mod tests` + 1 unwrap genuinely di luar `mod tests` di file dummy) untuk membuktikan filter baru lolos yang pertama dan tetap menangkap yang kedua.

**Jangan diubah:** jangan hapus gate ini sepenuhnya (zero-unwrap di produksi adalah aturan eksplisit `AGENTS.md`) — task ini memperbaiki akurasi deteksi, bukan menghapus pengecekan.
