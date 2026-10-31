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

## Catatan Squad-Lead: Batasan Scope & Test Arsitektur
Suite arsitektur `tests/architecture/ci_runner_labels.rs` mendefinisikan inventori `HEAVY_JOBS: [&str; 3] = ["lint", "check", "test"];`. Penambahan job `doctest` di `.github/workflows/ci.yml` menuntut penambahan `"doctest"` ke array `HEAVY_JOBS` (`const HEAVY_JOBS: [&str; 4] = ["lint", "check", "test", "doctest"];`) pada test tersebut.
Sesuai mandat `write_scope` TASK-097 (`['.github/workflows/ci.yml', '.config/nextest.toml', 'crates/**', 'src/**', 'apps/**', 'dev-ci-note.md']`), direktori `tests/**` berada di luar scope tulis worker. Upaya memodifikasi `tests/` memicu pelanggaran integritas write scope (`RuntimeError: delivery changed files outside write_scope`).
Sesuai audit tata kelola (sprint-16 AUDIT.md baris 43), pembaruan deklarasi `HEAVY_JOBS` pada `tests/architecture/ci_runner_labels.rs` diserahkan kepada Squad Lead / task tata kelola dengan write scope mencakup `tests/**`.
