# Lane R — Rust (System Core)

Ref: `decisions/refactoring-rust/phase-5-production-hardening/README.md` (W0–W5), `tech-standart/code-quality/`, `tech-standart/plugin-standard/`.
Build Windows: `$env:CARGO_TARGET_DIR="target/agent"`; test: `cargo nextest run -p <crate>`.

## R-0x Klasifikasi & gerbang kualitas

- [ ] **R-00 (P0 — PRIORITAS UTAMA, ODR-11, M) Inventaris `clawcrew-runtime` vs boundary.** Konteks terukur: `agent` 77k, `sop` 38k, `rpc` 32k, `skills` 12k, `cron` 10k, `control_plane` 7k LOC masih dipakai `src/main.rs` dan `gateway/{api,api_config,ws,a2a}.rs` (`foundation/05`). Keluaran per modul: keputusan + daftar pemanggil + tiket tujuan (G-30..G-33 bila pindah ke Go). Per modul (agent, sop, subagents, cron, memory_strategy, skills, hooks, tools, security, i18n, observability, rpc…) tetapkan: *keep-in-Rust* / *move-to-Go* / *delete* / *feature-gate*, dengan bukti pemakai (siapa memanggil). Output: tabel di `decisions/refactoring-rust/phase-5-production-hardening/R-00-runtime-inventory.md`. **Tidak ada penghapusan tanpa sign-off Owner.** Acceptance: tiap top-level modul punya keputusan + alasan; daftar dependen terukur. Out: eksekusi pemindahan.
- [ ] **R-01 (P1, S) `[workspace.lints]`** di root `Cargo.toml` (`clippy::unwrap_used`/`expect_used` = warn di non-test, `unsafe_code` terkontrol); crate mewarisi `lints.workspace = true`. Dep: –. Acceptance: `cargo clippy --workspace -- -D warnings` tetap hijau dengan baseline/allow terlacak.
- [ ] **R-02 (P1, M) Burn-down `unwrap/expect` produksi** (±400 hit; runtime 237). Per crate, mulai `gateway`, `channels`. Dep: R-01. Acceptance: nol hit di jalur produksi atau invariant terdokumentasi.
- [ ] **R-03 (P1, S) Gerbang ukuran file/crate** di `tests/architecture/` sesuai `rust-module-size-and-decomposition-standard.md`; daftar pengecualian bertanggal. Acceptance: CI gagal bila file baru melewati batas.
- [ ] **R-04 (P1, S) Gerbang arah dependency antar-crate** (`cargo metadata`); larang `plugins→runtime`, `channels→runtime` baru, dst. Acceptance: test gagal pada edge terlarang.
- [ ] **R-05 (P2, S) Selaraskan `rust-conventions.md`** (MSRV 1.96, jumlah crate 27). Dep: –.
- [ ] **R-06 (P2, S) Ukur waktu build/test baseline** (cold/warm, `nextest`) dan simpan di `foundation/03`.

## R-1x Putus kopling & pecah crate (setelah R-00)

- [ ] **R-10 (P1, L) Ekstrak `i18n`** dari runtime (87 ref dari channels) ke crate kecil. Dep: R-00, R-04.
- [ ] **R-11 (P1, L) Ekstrak `security::scrub`** (36 ref) ke crate/ke `clawcrew-infra`. Dep: R-00.
- [ ] **R-12 (P1, L) Pisahkan kontrak tools-scoped / memory_strategy / hooks / sop::types** yang dipakai channels menjadi trait di `clawcrew-api`. Dep: R-00.
- [ ] **R-13 (P1, L) `channels` tidak bergantung ke `runtime`** (677 ref → 0). Dep: R-10..R-12. Acceptance: edge dihapus di `cargo metadata`; gerbang R-04 mengunci.
- [ ] **R-14 (P1, L) `gateway` tidak bergantung ke `runtime`; hapus REST/WS/A2A lama bila tak ada pemakai (ODR-14)** (426 ref; cron, observability). Dep: R-00. Bila cron pindah ke Go: koordinasi G-lane.
- [ ] **R-15 (P2, XL→dipecah) Pecah `channels` (198k LOC)** per tier/protokol: crate `galleon-channel-<nama>` untuk channel baru, bertahap untuk lama. Dep: R-13.
- [ ] **R-16 (P2, XL→dipecah) Pecah/ramping `runtime` (286k LOC)** sesuai hasil R-00 (move/delete/gate). Dep: R-00, sign-off Owner.
- [ ] **R-17 (P2, M) Pecah file terbesar** yang tetap tinggal di Rust (daftar dari R-03).
- [ ] **R-18 (P3, M) `tools` (85k) & `config` (85k) & `providers` (84k):** audit pemisahan sub-domain; jangan pecah spekulatif (YAGNI).

## R-2x Hardening & kontrak

- [ ] **R-20 (P1, M) Implementasi nyata `execute_task`** di `grpc_system_gateway.rs` atau hapus dari proto bila tidak dipakai (keputusan dengan G-lane). Acceptance: tidak ada stub yang menjawab sukses palsu.
- [ ] **R-21 (P1, M) Test integrasi gRPC Go↔Rust** (`ExecuteTurn` stream, `GetDecryptedSecret`, `ExecuteNativeTool`) termasuk mTLS/loopback-only binding. Dep: –.
- [ ] **R-22 (P1, M) Sandbox:** test Landlock/Seatbelt/AppContainer per OS; kebijakan deny-by-default terbukti.
- [ ] **R-23 (P2, S) Scrubbing kredensial di log** — test tangkap-log untuk `ExecuteTurn`/vault.
- [ ] **R-24 (P2, M) Vault:** uji rotasi/ekspor/migrasi; `GetDecryptedSecret` hanya untuk caller terotorisasi.

## R-3x Plugin & channel tier

- [ ] **R-30 (P1, S) Errata ADR-001** (WhatsApp → Tier A, WIT v0 beku, bluesky) — lihat D-04; perbarui `plugin-codebase-standard.md`.
- [ ] **R-31 (P2, L) Pilot channel Tier C WASI** end-to-end (host, signature, capability) pada fixture; ukur batas `wasi:http` outbound.
- [ ] **R-32 (P2, S) WeChat/QQ/dst. = Tier B** (ODR-13, regional): buat crate `galleon-channel-<nama>` + feature flag; catat di errata ADR-001.
- [ ] **R-33 (P2, M) Pasang mekanisme pilih-Tier-B saat instalasi** (feature flags / installer). Dep: P-lane installer.
- [ ] **R-34 (P1, M) CLI = binary Rust, klien Go untuk orkestrasi (ODR-12)** — `chat/run/approve/status` memanggil Go :9090 (SSE/WS); `service/doctor/onboard/vault` tetap lokal Rust; tanpa agent loop lokal. Desain di `foundation/05`. Dep: R-00, G-lane API.
