# 03 — Build, Test, CI, Rilis, Deployment

> Sumber: `Justfile`, `Makefile`, `.cargo/`, `.config/nextest.toml`, `.github/workflows/`, `tests/`, `deny.toml`, `clippy.toml`. Perintah adalah yang terdokumentasi di repo; **tidak dijalankan** pada observasi ini.

## 1. Perintah kerja harian

| Tujuan | Perintah | Catatan |
|---|---|---|
| Cek cepat crate | `cargo check -p <crate>` | Windows: `$env:CARGO_TARGET_DIR="target/agent"` |
| Lint crate | `cargo clippy -p <crate> -- -D warnings` | |
| Test | `cargo nextest run -p <crate>` / `--workspace` | Standar repo; `just test` = `cargo test --locked` |
| Format | `cargo fmt --all -- --check`; `taplo` untuk TOML | |
| Pra-PR penuh | `./dev/ci.sh all` / `just ci` | fmt + clippy + test |
| Audit | `just audit`, `just deny` | `deny.toml`, `.cargo/audit.toml` |
| Go engine | `go test -race ./...` di `engine/`; `go generate` untuk Wire | |
| web | `npm run dev` / `build` / `lint` di `web/` | Express dev server (`server.ts`); tidak ada skrip test terdeteksi **[belum diverifikasi]** |
| tauri | `cargo tauri dev/build` di `apps/tauri` | anggota workspace root (paket `fleet-ai-desktop`) |
| web (warisan) | `npm run typecheck`, `npm test`, `npm run build` | |
| Docs-only | `scripts/ci/docs_quality_gate.sh`, `docs_links_gate.sh` | |

Akselerasi: `sccache` + `rust-lld` (`.cargo/config.toml`), nextest (`.config/nextest.toml`).

## 2. Lapisan test

| Lapisan | Lokasi | Tujuan |
|---|---|---|
| Unit | `#[cfg(test)]`/`tests.rs` sibling (Rust); `*_test.go` (±41) | logika lokal |
| Architecture gate | `tests/architecture/` | `no_duplicate_state`, `no_duplicate_risk_tier`, `config_save_isolation`, `cli_fluent_coverage`, `ci_runner_labels`, `container_release`, `desktop_release`, `publish_contract`, `release_workflow` |
| Component / Integration / System | `tests/component|integration|system/` | CLI, gateway, provider, plugin, security; agent loop, channel, memori; E2E first-run & multi-agent |
| Live / Manual | `tests/live/`, `tests/manual/` | butuh provider nyata / prosedur manual |
| Fuzz / Bench / Eval | `fuzz/`, `benches/`, `evals/regression` | validasi command/config/provider/webhook, benchmark, regresi loop tool |

**Celah:** (a) `unwrap_used`, (b) ukuran file/crate, (c) arah dependency antar-crate, (d) boundary tier (Go tidak shell/fs; Tauri hanya proxy; UI tanpa backend), (e) test & CI untuk `web/` dan `apps/tauri`. (a–c) diusulkan di Phase 5 H0.1–H0.4; (d–e) di 04 backlog.

## 3. Pagar kualitas yang ada

- `clippy.toml`: larang `tokio::spawn` langsung (pakai `clawcrew_spawn::spawn!`), macro logging langsung (pakai `record!`), `anyhow!` polos.
- `deny.toml`: tolak vulnerability, yanked, registry/git tak dikenal, wildcard, lisensi tak disetujui, versi duplikat (skip bertanggal).
- `panic = "abort"` di release; `release_workflow.rs` menguji workflow rilis.
- i18n: teks CLI via Fluent (`cli_fluent_coverage`).

## 4. CI (`.github/workflows/`, 39 workflow)

| Kelompok | Workflow |
|---|---|
| Inti | `ci.yml`, `ci-code-analysis.yml`, `engine-ci.yml`, `platform-tests.yml`, `windows-tests.yml`, `desktop-check.yml`, `cross-platform-clippy.yml` |
| Keamanan | `codeql.yml`, `daily-audit.yml`, `daily-npm-audit.yml`, `trivy-scheduled.yml`, `ci-sbom.yml`, `npm-deps-review.yml`, `monthly-outdated.yml` |
| Build/Rilis | `cross-platform-build-manual.yml`, `docker-image-pr.yml`, `docker-publish.yml`, `release-gate-approval.yml`, `release-stable-manual.yml`, `testing-to-prerelease-pr.yml`, `pub-crates.yml`, `pub-aur.yml`, `pub-scoop.yml`, `scoop-bucket-canary.yml`, `discord-release.yml`, `tweet-release.yml` |
| Proyek/Docs | `docs-deploy.yml`, validasi terjemahan, labeler PR, dashboard proyek, workflow principle, notifikasi persetujuan tier |

`engine-ci.yml` = gerbang Go **[cakupan belum diverifikasi]**. `desktop-check.yml` kemungkinan menargetkan `apps/tauri` (warisan) **[belum diverifikasi]**.

## 5. Versioning & rilis

- Versi tunggal workspace `0.8.5`; `release-plz.toml` (changelog + tag, semver-check off).
- Alur: `CHANGELOG-next.md` → `release-gate-approval` → `release-stable-manual` → build lintas platform → Docker + crates.io + AUR + Scoop → pengumuman.
- Konvensi: Conventional Commits; branch non-`master`; PR ke `master`; tanpa footer atribusi bot.
- **Rilis hari ini hanya mencakup artefak warisan.** Menjadikan `web/` + `apps/tauri` + engine + gateway sebagai artefak rilis adalah pekerjaan fondasi yang belum ada (04 P1).

## 6. Deployment

| Target | Aset | Catatan |
|---|---|---|
| Docker/Compose | `docker-compose.yml` | gateway loopback `42617`, volume, healthcheck, non-root |
| K8s/OpenShift | `deploy-k8s/` | secret di-`gitignore`; state ephemeral kecuali PVC |
| Nix | `flake.nix`, `nix/module.nix` | multi-instance systemd, sandboxing |
| FreeBSD / RPi | `dist/freebsd`, `scripts/deploy-rpi.sh` | |
| Desktop Galleon | Tauri bundler dari `apps/tauri` | belum ada pipeline; menjalankan Go engine + Rust gateway sebagai sidecar belum terdokumentasi **[belum diverifikasi]** |

## 7. Definition of Done umum

- [ ] `cargo fmt --check`, `cargo clippy -p <crate> -- -D warnings`, `cargo nextest run -p <crate>` hijau.
- [ ] `go test -race ./...` bila menyentuh `engine/`; lint/typecheck/build bila menyentuh `web/`.
- [ ] Kontrak berubah → proto/WIT + `api-contracts/` dalam satu PR.
- [ ] Tidak ada `unwrap()/expect()` baru di jalur produksi; tidak ada secret di diff.
- [ ] Docs terkait diperbarui; ADR tidak diedit (errata/ADR baru).

## 8. Baseline Waktu Build & Test Terukur (R-06)

> Diukur secara konkret pada lingkungan Windows x86_64 per 2026-10-04.

| Operasi | Target / Scope | Profil | Waktu Terukur | Keterangan |
|---|---|---|---:|---|
| **Full Workspace Check** | `cargo check --workspace` (27 crates) | `dev [unoptimized]` | **7m 02s** (~423s) | Baseline kompilasi penuh cold/dirty |
| **Incremental Warm Check** | `cargo check --workspace` | `dev [unoptimized]` | **3.78s** | Cache hangat incremental dev loop |
| **Architecture Gate: Module Size** | `cargo test --test architecture module_size_limits` | `test` | **0.53s** | Scanner ukuran seluruh file Rust |
| **Architecture Gate: Dependency Direction** | `cargo test --test architecture dependency_direction` | `test` | **< 0.05s** | Verifikasi isolasi leaf & channel |
| **Go Engine Unit Tests** | `go test ./...` di `engine/` | `cached/local` | **1.2s** | Seluruh modul Go Engine |

