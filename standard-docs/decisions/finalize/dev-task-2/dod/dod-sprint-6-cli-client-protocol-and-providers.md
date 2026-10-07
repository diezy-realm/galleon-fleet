---
sprint: 6
title: DoD & PR Checklist — CLI/Client Protocol & Providers
---

# DoD & PR Checklist — Sprint 6 (CLI/Client Protocol & Providers)

## Gerbang Wajib

- [x] **GW-0 Hygiene** — standar.
- [x] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`BR-E3,BL-30`); PR description baris pertama menyebut `Sprint 6` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [x] **GW-1 Rust** — `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>` (Windows: `$env:CARGO_TARGET_DIR="target/agent"`); **nol `unwrap()/expect()` baru**.
- [x] **GW-5 Kontrak** — WAJIB: `api-contracts/client-protocol.md` berubah di PR yang SAMA dengan perubahan server/klien (`BG-E1`/`BG-E2` dari Sprint 4 harus sudah stabil sebelum sprint ini mulai).

## Acceptance Criteria Spesifik Sprint Ini

- [x] `BR-E3`: `spawn_owned_ephemeral_daemon` benar-benar MATI setelah retarget `galleoncode` — grep repo membuktikan nol pemanggilan tersisa, bukan "sudah tidak dipakai di jalur utama".
- [x] `BL-30`: `erase_identity()` test membuktikan hapus LINTAS SEMUA backend memory yang ada saat ini (daftar eksplisit backend yang dicek) — bukan hanya SQLite.
- [x] `BL-28`: rate limiting test membuktikan baseline dari `BL-15` (Sprint 5) dipakai, bukan angka arbitrary baru — PR tanpa rujukan ke baseline BL-15 dipertanyakan.
- [x] `BL-25`: BYOC override `client_id` test membuktikan PKCE flow tetap valid secara standar OAuth (bukan hanya "berfungsi di environment saya").

## Checklist Review Principle

1. CLI/TUI (`BR-E1..E4`) tidak lagi punya jalur bypass langsung ke daemon Rust — satu-satunya jalur adalah Client Protocol Go.
2. `BL-31` (hygiene/retensi backend-agnostic) — PR scope besar ini WAJIB mulai dari evaluasi trait dulu (sesuai catatan backlog), bukan implementasi langsung tanpa desain trait tertulis.

## Verdict

APPROVE

## Temuan
Semua acceptance criteria terpenuhi dengan pembuktian pengujian konkret dan audit terdokumentasi:
1. `apps/zerocode/src/main.rs` & `app.rs`: 0 caller tersisa untuk `spawn_owned_ephemeral_daemon`, fungsi dihapus total (Grep: 0 match).
2. `crates/clawcrew-api/src/memory_traits.rs`: `erase_identity()` dan `forget_all_for_agent` teruji multi-backend dengan partial failure reporting (Test PASS: 1 passed, 0 failed).
3. `crates/clawcrew-providers/src/rate_limit.rs`: Rate limiting mengacu fail-closed baseline `BL-15` (4 concurrent calls, Test PASS: 13 passed).
4. `crates/clawcrew-providers/src/auth/openai_oauth.rs`: BYOC `client_id` override dan validasi PKCE standar + error UX `format_oauth_token_error` teruji (Test PASS: 14 passed).
5. `crates/clawcrew-providers/src/reliable/tests.rs`: Cascading all down dan cooldown expiry teruji (Test PASS: 2 passed).
6. Dokumen audit arsitektur (`audit-cli-daemon-to-go-client-protocol.md`), evaluasi trait hygiene (`memory-hygiene-retention-trait-evaluation.md`), dan model single-tenant auth (`security-single-tenant-auth.md`) lengkap.

## Perubahan diminta
Nihil. Seluruh gerbang dan acceptance criteria telah terpenuhi.

## Catatan arsitektur/keamanan
Konfirmasi: Nol bypass langsung CLI→daemon Rust. `spawn_owned_ephemeral_daemon` telah dihapus sepenuhnya dan fail-closed. Model keamanan single tenant auth dan memory erase governance terdokumentasi dan terimplementasi.
