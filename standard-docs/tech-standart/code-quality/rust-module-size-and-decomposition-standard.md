# Rust Module Size & Decomposition Standard

> **Status:** Normatif (Proposed → berlaku setelah sign-off Lead + PO) · **Dasar bukti:** [`phase-5-production-hardening`](../../decisions/refactoring-rust/phase-5-production-hardening/README.md)
> Melengkapi `tech-stack/rust-conventions.md`. Bila bertentangan, dokumen ini menang untuk topik ukuran/pemecahan.

## 1. Tujuan

Mencegah kembalinya "bload code" (file 10k–30k baris, crate 200k+ baris) dengan batas objektif yang bisa dicek CI, bukan selera.

## 2. Batas ukuran

| Unit | Target | Peringatan | Gagal CI (file/crate baru atau yang diubah) |
|---|---:|---:|---:|
| File sumber (non-test) | ≤ 800 baris | > 2.000 | > 5.000 |
| File test (`tests.rs`) | ≤ 2.000 | > 5.000 | > 10.000 → pecah per fitur |
| Fungsi | ≤ 60 baris | > 150 | > 400 (mis. `async_main_inner` ±3.7k = pelanggaran) |
| `lib.rs` / `mod.rs` | hanya `mod`, `pub use`, tipe publik inti; **bukan** implementasi | > 300 baris logic | > 1.000 |
| Crate | ≤ 30k LOC | > 60k | > 100k tanpa ADR pengecualian |
| Jumlah `pub mod` di satu `lib.rs` | ≤ 20 | > 30 | > 45 |
| Dependency internal per crate | ≤ 6 | > 10 | > 14 |

Aturan *ratchet*: file/crate eksisting yang melebihi batas **tidak boleh bertambah** besar; PR yang menyentuhnya sebaiknya mengurangi. Pengecualian wajib dicatat di `scripts/ci/size-exceptions.toml` dengan alasan + tanggal tinjau ulang.

## 3. Cara memecah file (body-split)

1. **Pure-move dulu.** PR hanya memindahkan kode; tanpa perubahan logika, nama, atau signature. Diff harus bisa diverifikasi dengan `git diff --color-moved`.
2. Pecah **berdasarkan tanggung jawab**, bukan berdasarkan ukuran: `types.rs`, `error.rs`, `config.rs`, `service.rs`, `store.rs`, `handlers/<fitur>.rs`.
3. Test inline → `tests.rs` sibling (`#[cfg(test)] mod tests;`). Sudah dilakukan untuk file jumbo (RF-A0/A2–A7).
4. Visibilitas minimum: `pub(super)` / `pub(crate)`; jangan menaikkan ke `pub` hanya agar kompilasi lolos.
5. `mod.rs` tetap sebagai fasad dengan `pub use` agar path publik tidak berubah.
6. Satu fungsi raksasa dipecah terakhir, setelah ada karakterisasi test; ekstrak fase (parse → validasi → eksekusi → render), bukan potong acak.

## 4. Kapan memecah menjadi crate (compile-time win)

Pecah file **tidak** mempercepat build; pecah crate yang mempercepat. Sebuah potongan layak jadi crate bila **semua** terpenuhi:

- [ ] Dependency keluarnya satu arah dan **tidak mengimpor induknya** (tidak ada siklus; jika ada, ekstrak kontrak dulu).
- [ ] API publik kecil dan stabil (trait/DTO), ≤ ~30 item publik.
- [ ] Fan-in nyata (dipakai ≥ 2 crate) **atau** biaya kompilasinya terukur besar (`cargo build --timings`).
- [ ] Punya test sendiri yang jalan tanpa crate induk.
- [ ] Bukan fragmentasi prematur: jangan buat crate < 500 baris tanpa alasan boundary (keamanan, platform, feature opsional).

Penamaan: crate baru **wajib** `galleon-<domain>`; crate legacy `clawcrew-*` tidak di-rename massal.

## 5. Arah dependency (dikunci CI)

```
galleon-api (leaf: trait + DTO)
   ↑
log · spawn · infra · config-types
   ↑
config · providers · security-core · i18n
   ↑
memory · tools · cron · observability
   ↑
runtime (agent loop, SOP, hooks)          galleon-channel-core
   ↑                                              ↑
gateway · eval · bin (wiring/DI)          galleon-channel-<name>
```

Aturan:
- Panah hanya naik. Crate di bawah **tidak boleh** mengimpor crate di atasnya.
- **Channel (`galleon-channel-*`, `galleon-channels`) tidak boleh mengimpor `galleon-runtime`.** Channel bergantung pada trait di crate kontrak; runtime di-inject saat wiring di bin/gateway.
- `galleon-api` adalah leaf: tidak ada dependency internal.
- `galleon-plugins` hanya boleh bergantung ke `api`, `infra`, `log`.
- Composition root (DI) hanya di bin dan `gateway`; library crate tidak membuat global singleton.

Gerbang: test `tests/architecture/` berbasis `cargo metadata` dengan daftar edge terlarang.

## 6. Kualitas produksi (dikunci CI)

- `[workspace.lints]` + `[lints] workspace = true` di setiap crate: `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic` (`warn` → ratchet → `deny` per-crate). `clippy.toml`: `allow-unwrap-in-tests = true`.
- Pengecualian `expect` hanya dengan komentar `// INVARIANT: <alasan tak mungkin gagal>` dan `#[expect(clippy::expect_used, reason = "...")]`.
- Release memakai `panic = "abort"` → panic produksi = proses mati; error harus dipropagasi (`thiserror` + `Result<T>`).
- Test runner: `cargo nextest run -p <crate>`; lint: `cargo clippy -p <crate> -- -D warnings`.
- Dependency: `cargo-deny` (advisories, licenses, bans) di CI; versi dideklarasikan sekali di `[workspace.dependencies]`.

## 7. Pengukuran

Setiap PR pemecahan crate melampirkan: `cargo build --timings` (clean & incremental edit 1 baris) sebelum/sesudah, ukuran binary bila relevan. Tanpa angka = tidak diklaim sebagai optimasi build.
