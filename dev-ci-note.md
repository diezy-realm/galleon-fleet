# Handoff dev-ci: Lane Doctest untuk gates.sh

Task: TASK-097 (Spec: SPEC-016, Backlog: BL-091)
Target branch: testing

## Perubahan yang Disiapkan
1. Job `doctest` telah ditambahkan ke `.github/workflows/ci.yml`:
   ```yaml
   doctest:
     name: Doc Tests
     runs-on: [self-hosted, linux, galleon-heavy]
     timeout-minutes: 30
     steps:
       - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
       - name: Ensure web/dist placeholder exists
         run: mkdir -p web/dist && touch web/dist/.gitkeep
       - name: Run doctests
         run: cargo test --doc --workspace --locked
   ```
   Dan job `gate` (`CI Required Gate`) telah diperbarui agar `needs` menyertakan `doctest`.

2. Komentar `.config/nextest.toml` pada profil default diselaraskan dengan nilai `retries = 0`:
   ```toml
   [profile.default]
   # Tanpa retry otomatis pada inner loop lokal (retries = 0) agar kegagalan
   # langsung terdeteksi; retry untuk transient hanya aktif pada profil ci.
   retries = 0
   ```

3. Verifikasi lokal `cargo test --doc --workspace --locked` bersih (1 passed di `galleon-tool-call-parser`, 2 ignored di `galleon-macros`, 0 failed di seluruh workspace).

## Rekomendasi Pembaruan `gates.sh` di `~/Apps/dev-ci`
Tambahkan gate doctest ke `LIST` dan fungsi `gate_doctest`:
```bash
# Tambahkan ke LIST:
# Doctest|heavy|none|30|0

gate_doctest() {
  has_rust_changes || { echo "no rust changes; skipping gate"; return 0; }
  web_placeholder
  cargo test --doc --workspace --locked
}
```
Catatan: Worker produk dilarang mengedit repositori `~/Apps/dev-ci` secara langsung; integrasi lane lokal dilakukan melalui handoff ini.

## Penyelarasan Test Arsitektur (Audit Recovery 2026-11-01)
Suite arsitektur `tests/architecture/ci_runner_labels.rs` telah diperbarui dengan mendaftarkan `"doctest"` dan `"sonar"` (sinkronisasi upstream testing) ke array `HEAVY_JOBS`:
```rust
const HEAVY_JOBS: [&str; 5] = ["lint", "check", "test", "doctest", "sonar"];
```
Sesuai koreksi audit blocked 2026-11-01, `tests/architecture/ci_runner_labels.rs` telah dimasukkan ke dalam `write_scope` TASK-097.
Verifikasi `cargo test --locked --test architecture ci_runner_labels` lulus 100% (4 passed, 0 failed).
