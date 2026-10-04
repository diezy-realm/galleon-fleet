# Lane P — Platform, CI, Deploy, Rilis

- [x] **P-01 (P0, S) Pastikan workspace Cargo hijau** [Selesai PR-03] `exclude = ["apps/tauri"]` ditambahkan di root Cargo.toml, `frontendDist` diarahkan ke `../../web/dist`. `cargo metadata --no-deps` dan `cargo check` hijau (0 error).
- [ ] **P-02 (P1, M) Dockerfile\*:** sesuaikan stub `apps/tauri/{src/main.rs,build.rs}` dan copy `web/dist`; pertimbangkan target server (Go + Rust gateway + statis UI) terpisah dari desktop. Dep: U-01.
- [ ] **P-03 (P1, S) `Cargo.toml` `include`** `/web/dist/**/*`: putuskan — UI di-embed ke binary server atau disajikan Nginx; ADR kecil.
- [ ] **P-04 (P1, M) CI `web/`:** install, typecheck, lint, test, build. Dep: U-12.
- [ ] **P-05 (P1, M) CI desktop:** perbarui `desktop-check.yml` dan `tests/architecture/desktop_release.rs` untuk `fleet-ai-desktop`.
- [ ] **P-06 (P1, S) CI engine:** pastikan `engine-ci.yml` menjalankan `go test -race`, `go vet`, `staticcheck`, cek `wire_gen`.
- [x] **P-07 (P1, M) Gerbang boundary tier** di `tests/architecture/` [Selesai PR-11] Modul `tests/architecture/tier_boundaries.rs` ditambahkan mengunci isolasi 4-tier arsitektur secara otomatis (UI tanpa backend, Tauri tanpa shell, Go tanpa exec/vendor LLM); test lolos 100%. Dep: G-05, U-02.
- [ ] **P-08 (P2, M) Pangkas workflow CI warisan** (39 workflow) yang tidak relevan; tandai required checks.
- [ ] **P-10 (P1, L) Nginx + TLS:** konfigurasi reverse proxy ke Go :9090 (+WebSocket/SSE), HSTS, rate-limit, header keamanan; contoh compose untuk VPS. Dep: G-21.
- [ ] **P-11 (P2, M) Panduan deploy** lokal ↔ VPS/cloud (certbot/Caddy alternatif), model ancaman, hardening.
- [ ] **P-12 (P2, L) Pipeline rilis:** desktop Windows (MSI/NSIS, tanda tangan), server (Docker image, checksum, SBOM). Dep: U-09.
- [ ] **P-13 (P3, M) Installer pilih Tier B** (R-33).
