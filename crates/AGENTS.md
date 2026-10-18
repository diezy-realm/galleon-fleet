# crates/ — standar eksekusi Rust (agy)

Berkas ini aktif otomatis saat kamu (agy) bekerja di tier `crates/` (Rust 2024,
25+ crate, System Core). Kamu adalah **eksekutor Backend-Rust**: patuhi idiom +
error-handling + SSOT + workflow build di bawah. Standar lengkap (sumber tunggal,
dipakai KiroCrew & agy):

@[Cargo workflow galleon](../.kiro/steering/galleon-cargo-workflow.md)

Ringkas yang WAJIB:
- **Error handling:** jalur produksi WAJIB propagasi (`Result`/`Option`,
  `?`-operator, `thiserror`/`anyhow` sesuai pola crate). **Zero `.unwrap()` /
  `.expect()`** di jalur produksi kecuali invariant terdokumentasi — gate CI
  memeriksanya (`check_production_unwrap.py`).
- **Idiom:** borrow sebelum clone; iterator over index-loop; `impl Trait` di
  arg bila cukup; `#[derive(...)]` daripada impl manual; modul kecil fokus.
  Jangan `#[allow(dead_code)]`/underscore untuk sembunyikan kode mati — hapus,
  sambungkan, atau track.
- **Foldering/SSOT:** satu fakta satu sumber kanonikal; jangan duplikat
  state/cache paralel. Crate/modul BARU pakai prefix `galleon-`, bukan
  `clawcrew-`. Definisi pindah ke leaf crate SSOT (pola T3-02 di
  `reference/dev-task-3/README.md`).
- **Verifikasi:** `cargo check -p <crate>` (bukan `--workspace` kecuali perlu) +
  `cargo nextest run -p <crate>` (BUKAN `cargo test`). sccache opsional per
  mesin via `~/.cargo/config.toml` (tidak diset di config repo agar CI/Docker
  tidak bergantung padanya). Batch edit dulu, build+test SEKALI di akhir.
