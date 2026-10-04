# 02 — BE-Rust (System Core, `crates/`, gRPC :50052, CLI)

Konvensi: `tech-stack/rust-conventions.md`, `code-quality/rust-module-size-and-decomposition-standard.md`. Gerbang: GW-1 (+GW-5 kontrak, GW-8 keamanan).
Prinsip: **verifikasi per modul (R-00 §4) wajib sebelum modul runtime dipindah/dihapus**; Go memiliki parity lebih dulu (strangler).

## A. Gerbang Kualitas

- [ ] **BR-A1 · R-01 · PR-10 (P1, S) `[workspace.lints]`** (`unwrap_used`/`expect_used` = warn non-test, `unsafe_code` terkontrol); crate mewarisi `lints.workspace = true`.
- [ ] **BR-A2 · R-03 · PR-10 (P1, S) Gerbang ukuran file/crate** di `tests/architecture/`; daftar pengecualian bertanggal.
- [ ] **BR-A3 · R-04 · PR-10 (P1, S) Gerbang arah dependensi** (`cargo metadata`): larang `plugins→runtime`, `channels→runtime` baru.
- [ ] **BR-A4 · R-06 · PR-10 (P2, S) Baseline waktu build/test** (cold/warm, nextest) → `foundation/03`.
- [ ] **BR-A5 · R-02 · PR-30 (P1, M, berseri per crate) Burn-down `unwrap/expect` produksi** (±400 hit; `gateway`, `channels`, `runtime`). Dep: BR-A1.
- [ ] **BR-A6 · R-05 · PR-20 (P2, S) Selaraskan `rust-conventions.md`** (MSRV, jumlah crate).

## B. Kontrak gRPC & Keamanan Inti

- [ ] **BR-B1 · R-20 · PR-35 (P1, M) `execute_task` nyata** (`grpc_system_gateway.rs`) atau hapus dari proto — tanpa stub sukses palsu (GW-5).
- [ ] **BR-B2 · R-21 · PR-35 (P1, M) Test integrasi Go↔Rust lengkap:** `ExecuteTurn` stream (T-5.3 ✔), `GetDecryptedSecret` (handler ✔ T-8.1), `ExecuteNativeTool`; tambahkan skenario loopback-only/mTLS dan kegagalan.
- [ ] **BR-B3 · R-22 · PR-36 (P1, M) Sandbox:** test Landlock/Seatbelt/AppContainer per OS; deny-by-default terbukti.
- [ ] **BR-B4 · R-23 · PR-36 (P2, S) Scrubbing kredensial di log** — test tangkap-log `ExecuteTurn`/vault.
- [ ] **BR-B5 · R-24 · PR-36 (P2, M) Vault:** rotasi/ekspor/migrasi; `GetDecryptedSecret` hanya untuk caller terotorisasi.
- [ ] **BR-B6 · R-39 · PR-55 (P1, M) ⚑ Auth pemanggil Go:** satu provider (loopback token/peer-uid/mTLS) menggantikan `principal_resolver`/`auth_provider`/`webauthn` Rust. Dep: BG-C1.

## C. Dekoplasi Crate (pecah kode besar)

- [ ] **BR-C1 · R-10 · PR-31 (P1, M) Selesaikan ekstraksi `i18n`.** *Status: parsial* — crate `galleon-i18n` sudah ada (RF-B, ±2.160 LOC) tetapi `clawcrew-channels` masih 87 ref ke `runtime::i18n`. Repoint ke `galleon-i18n`, hapus duplikasi di runtime.
- [ ] **BR-C2 · R-11 · PR-32 (P1, L) Ekstrak `security::scrub`** (36 ref).
- [ ] **BR-C3 · R-12 · PR-33 (P1, L) tools-scoped / memory_strategy / hooks / sop::types → trait di `clawcrew-api`.**
- [ ] **BR-C4 · R-13 · PR-34 (P1, L) `channels` tidak bergantung ke `runtime`** (677 ref → 0); gerbang BR-A3 mengunci. Dep: BR-C1..C3.
- [ ] **BR-C5 · R-14 · PR-61 (P1, L) ⚑ `gateway` tidak bergantung ke `runtime`** (426 ref → 0). Dep: BR-D1 (cron). Terkait keputusan 08 §C.
- [ ] **BR-C6 · R-15 · PR-63 (P2, XL, berseri) Pecah `channels` (198k LOC)** → `galleon-channel-<nama>` (sudah: core, imessage, signal, bluesky). Dep: BR-C4.
- [ ] **BR-C7 · R-17 · PR-60 (P2, M) Pecah file terbesar** yang tetap di Rust.
- [ ] **BR-C8 · R-18 · PR-97 (P3, M) Audit `tools`/`config`/`providers`** — tanpa pemecahan spekulatif.

## D. Ramping `clawcrew-runtime` (286k LOC) — Strangler ke Go

- [ ] **BR-D1 · R-16 · PR-60 (P2, XL, berseri) ⚑ Ramping runtime per modul** (urutan R-00 §3). Syarat tiap modul: parity Go + eval hijau.
- [ ] **BR-D2 · R-37 · PR-56 (P2, M) Retarget `galleon-eval`** ke jalur Go; pertahankan fixture trace. Dep: BG-D1.
- [ ] **BR-D3 · R-38 · PR-59 (P2, S) ⚑ Hapus `verifiable_intent`** (tag `archive/pre-galleon` dulu; ODR-21). Dep: PR-17.
- [ ] **BR-D4 · R-41 · PR-62 (P2, L) ⚑ Hapus `rpc` + REST/WS/A2A lama** setelah parity. Dep: BR-E3, BR-C5.

## E. CLI & Klien

- [ ] **BR-E1 · R-36 · PR-57 (P1, M) Audit `galleoncode` & `galleonrelay`:** petakan pemanggilan ke daemon Rust → endpoint Go.
- [ ] **BR-E2 · R-34 · PR-57 (P1, M) CLI `galleon`** `chat/run/approve/status` → Go :9090; `service/doctor/onboard/vault` tetap lokal. Dep: BG-E2.
- [ ] **BR-E3 · R-40 · PR-58 (P1, L) Retarget `galleoncode` (TUI)** ke Client Protocol; matikan `spawn_owned_ephemeral_daemon`. Dep: BG-E2.
- [ ] **BR-E4 · R-42 · PR-58 (P1, M) Retarget `galleon-acp-bridge`** (ODR-24).

## F. Plugin & Channel (Tier A/B/C)

- [ ] **BR-F1 · R-30 · PR-21 (P1, S) Errata ADR-001** (WhatsApp Tier A, WIT v0 beku, bluesky) + `plugin-codebase-standard.md`.
- [ ] **BR-F2 · R-35 · PR-86 (P1, M) Host WIT v1** (dual v0/v1; v0 tetap lolos `.frozen`). Pasangan RB-24.
- [ ] **BR-F3 · R-32 · PR-64 (P2, L) Channel regional Tier B** (WeChat/QQ…) + feature flag.
- [ ] **BR-F4 · R-33 · PR-94 (P2, M) Pilih Tier B saat instalasi.**
- [ ] **BR-F5 · R-31 · PR-95 (P2, M) Tier C WASI:** pilot `channel-nostr-pilot` + `.frozen` v0 sudah ada (T-7.1/7.2 ✔); tinggal e2e penuh (host, signature, capability) dan ukur batas `wasi:http` outbound.

## Selesai (arsip)

- [x] T-5.1..5.3 proto + handler `execute_turn` + test · [x] T-6.1..6.4 routing: capability enum, wiring `.fallback()`, 5 core handler, regresi streaming · [x] T-7.1..7.3 pilot Tier C + freeze WIT v0 + crate `galleon-channel-*` · [x] T-8.1 `get_decrypted_secret`

## Definition of Done (BE-Rust)

- [ ] `cargo fmt --check`, `clippy -D warnings`, `nextest` per crate · [ ] nol `unwrap/expect` & `allow(dead_code)` baru · [ ] edge dependensi terlarang tidak muncul · [ ] kontrak proto/WIT sinkron · [ ] log bebas rahasia
