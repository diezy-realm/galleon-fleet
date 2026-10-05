# 05 — Platform, DevOps, QA

Gerbang: GW-0, GW-6 (integrasi), GW-8. Prinsip: Nginx hanya reverse proxy + TLS (ODR-15); local-first, dapat dipasang di VPS/cloud.

## A. CI & Gerbang Boundary

- [x] **PL-A1 · P-04 · PR-12 (P1, M) CI `web/`:** [Selesai PR-12] Workflow `.github/workflows/web-check.yml` dibuat; skrip `typecheck`, `lint`, `build` diverifikasi hijau 100%. Dep: FW-B1.
- [x] **PL-A2 · P-05 · PR-12 (P1, M) CI desktop:** [Selesai PR-12] Workflow `.github/workflows/desktop-check.yml` diperbarui menggunakan path manifest `apps/tauri/Cargo.toml` dan target `desktop-check`.
- [x] **PL-A3 · P-06 · PR-13 (P1, S) CI engine:** [Selesai PR-13] Workflow `.github/workflows/engine-ci.yml` diperbarui dengan `go mod verify`, `go vet ./...`, pengecekan kompilasi Wire injection (`wire ./app`), `staticcheck`, dan test dengan race detector (`go test -race ./...`); perbaikan injeksi redundan `run/wire.go`.
- [x] **PL-A4 · P-07 · PR-11 (P1, M) Gerbang boundary tier** di `tests/architecture/` [Selesai PR-11] Modul `tests/architecture/tier_boundaries.rs` ditambahkan mengunci isolasi 4-tier secara otomatis di suite arsitektur: UI tier terisolasi tanpa import Tauri IPC/backend direct, Desktop tier tanpa eksekusi shell/terminal langsung di IPC app handler, dan Go Engine tanpa import `os/exec` maupun direct SDK vendor LLM. CLI root struct & method visibilitas dibersihkan. Test `tier_boundaries` lulus 100%. Dep: BG-A3, FW-A2.
- [ ] **PL-A5 · P-08 · PR-93 (P2, M) Pangkas workflow warisan** (39) & tandai required checks.

## B. Build, Docker, Workspace

- [x] **PL-B1 · P-01 · PR-03 (P0, S) Workspace Cargo hijau** [Selesai PR-03] `exclude = ["apps/tauri"]` ditambahkan di root Cargo.toml, `frontendDist` diarahkan ke `../../web/dist`. `cargo metadata --no-deps` dan `cargo check` hijau.
- [ ] **PL-B2 · P-02 + P-03 · PR-90 (P1, M) Dockerfile & `include` `/web/dist`:** sesuaikan stub `apps/tauri`; putuskan UI di-embed vs disajikan Nginx (ADR kecil); target server Go + Rust gateway.

## C. Deploy Remote

- [ ] **PL-C1 · P-10 · PR-91 (P1, L) Nginx + TLS:** reverse proxy ke Go :9090 (WS/SSE), HSTS, rate-limit, header keamanan; contoh compose VPS. Dep: BG-C3.
- [ ] **PL-C2 · P-11 · PR-91 (P2, M) Panduan deploy** lokal↔VPS/cloud, model ancaman, hardening; tetapkan peran `galleonrelay` vs Nginx (ODR-25).

## D. Rilis & Installer

- [ ] **PL-D1 · P-12 · PR-93 (P2, L) Pipeline rilis:** desktop (MSI/NSIS, signing), server (Docker image, checksum, SBOM). Dep: FD-B1.
- [ ] **PL-D2 · P-13 · PR-94 (P3, M) Installer pilih Tier B.**

## E. QA / Integrasi (per gelombang)

- [ ] **PL-E1 · PR-plan §4 (GW-6) Branch `integration/wave-N`:** `cargo nextest run --workspace` + `go test -race ./...` + `npm run build`.
- [ ] **PL-E2 · Smoke e2e:** Engine + gateway + UI/desktop; uji negatif fail-closed (Rust mati, token salah, injeksi perintah).
- [ ] **PL-E3 · N-41 · PR-88 Smoke test upgrade** data lama (`~/.clawcrew`) → baru.
- [ ] **PL-E4 · T-9.1 End-to-end demo** (lihat 08 §A) — bukti manual tanpa fallback/konstanta palsu, direkam di `finalize/`.
