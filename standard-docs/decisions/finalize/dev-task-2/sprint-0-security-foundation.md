---
sprint: 0
title: Security Foundation — Identity, Approval-Gate, Sandbox
status: planned
depends_on: none (sudah bisa mulai sekarang)
---

# Sprint 0 — Security Foundation

> Fokus: menutup gap identitas/otorisasi yang masih terbuka setelah Sprint 0/1 ASLI (dev-tasks) selesai. Verifikasi sesi 2026-10-05 menemukan satu gap baru (`S2-SEC-01`) yang tidak ada di dev-tasks asli — approval-gate endpoint terminal belum punya pemeriksaan identitas pemanggil.

## A. Identity & Access (prasyarat semua gate approval)

- [ ] **BG-C1 · G-35 · PR-44 (P1, L)** Identity & akses di Go: principal resolver (deny-by-default), WebAuthn, pairing. *Acceptance:* tanpa kredensial → ditolak; test negatif. Ref `R-00a`. File: `engine/src/` (modul baru identity).
- [ ] **S2-SEC-01 (BARU, P0, S)** ⚑ **Verifikasi identitas pada `req.Approved` endpoint terminal.** Ditemukan sesi 2026-10-05: `engine/src/fleet/services.go:713` menerima `req.Approved` sebagai boolean self-reported dari body HTTP tanpa pemeriksaan identitas apa pun — siapa pun yang punya akses network ke `:9090` bisa kirim `{"command":"rm -rf /","approved":true}` dan lolos approval gate HIGH_RISK. **Acceptance:** `handleTerminalExec` menolak `req.Approved=true` kecuali request membawa principal terverifikasi dari `BG-C1`. **Dep: BG-C1 (hard — tidak bisa dikerjakan sebelum principal resolver ada).**
- [ ] **BR-B6 · R-39 · PR-55 (P1, M)** ⚑ Auth pemanggil Go: satu provider (loopback token/peer-uid/mTLS) menggantikan `principal_resolver`/`auth_provider`/`webauthn` Rust. Dep: BG-C1.
- [ ] **BG-C2 · G-20 · PR-45 (P1, L)** Pairing perangkat via QR berotorisasi: token sekali pakai terikat perangkat, expiry, daftar, revoke, rate-limit; `/api/pairing/*`. Dep: BG-C1.
- [ ] **BG-C3 · G-21 · PR-45 (P1, M)** Autentikasi sesi/perangkat (middleware) untuk akses remote. Dep: BG-C2.

## B. Sandbox & Credential Hygiene (paralel, tidak bergantung ke A)

- [ ] **BR-B3 · R-22 · PR-36 (P1, M)** Sandbox: test Landlock/Seatbelt/AppContainer per OS; deny-by-default terbukti.
- [ ] **BR-B4 · R-23 · PR-36 (P2, S)** Scrubbing kredensial di log — test tangkap-log `ExecuteTurn`/vault.
- [ ] **BR-B5 · R-24 · PR-36 (P2, M)** Vault: rotasi/ekspor/migrasi; `GetDecryptedSecret` hanya untuk caller terotorisasi.

## C. Bug Nyata Jaring Pengaman (independen, bisa paralel, PRIORITASKAN dalam sprint ini)

- [x] **BL-32 (P0, HIGH — bug nyata, bukan desain)** Perbaiki/pulihkan `verify_whatsapp_signature` + `optional_channel_routes` yang hilang. **Urutan wajib:** (1) `cargo test -p clawcrew-gateway --features channel-whatsapp-cloud` untuk ground-truth, (2) `git log -p`/`git blame` cari kapan hilang, (3) pulihkan implementasi, (4) tambah test integrasi HTTP level (signature salah → 401). File: `crates/clawcrew-gateway/src/{tests.rs,webhook_ingress.rs,lib.rs}`.
  - **Status Selesai:** Selesai direstorasi & diverifikasi 100% lulus test.
  - **Root Cause Faktual:** Terpotong 4,927 baris di commit `db18a3745` (*"feat: refactoring rust"*) saat transisi parsial router.
  - **Restorasi:** `crates/clawcrew-gateway/src/lib.rs` (`verify_whatsapp_signature`, `optional_channel_routes`, `sop_webhook_routes`, handlers webhook channel), `crates/clawcrew-gateway/src/webhook_ingress.rs` (type fix, callsite pin update, 11 unit tests HMAC-SHA256, dan 5 integration tests Axum HTTP-level 401/404/200).
  - **Bukti Uji:** 19/19 tests lulus hijau (`cargo test -p clawcrew-gateway --features channel-whatsapp-cloud -- webhook_ingress`).
- [x] **BL-19 (P1, HIGH)** Sandbox MCP stdio child process — gap keamanan di jalur yang sudah dipakai produksi (nol sandboxing OS-level untuk proses stdio server, kontras dengan WASM plugin yang sangat ketat).
  - **Status Selesai:** Selesai diterapkan & diverifikasi 100% lulus test.
  - **Solusi Arsitektur:** Menambahkan konfigurasi `sandbox_profile: Option<String>` dan `allowed_paths: Vec<String>` dengan deny-by-default helper `is_sandboxed()` pada `McpServerConfig` (`crates/clawcrew-config/src/schema.rs`).
  - **Implementasi:** Di `crates/clawcrew-tools/src/mcp_transport.rs`, mendefinisikan trait `McpCommandBoundary`, setter/getter global, serta fallback `apply_default_mcp_sandbox` (sanitasi environment dari token rahasia CWE-200, isolasi working directory ke whitelist path, dan fail-closed untuk profil terblokir). Menghubungkan pembungkus sandbox pada `StdioTransport::spawn`. Di `crates/clawcrew-runtime/src/tools/mod.rs`, menghubungkan sandbox runtime kanonikal (`RuntimeMcpCommandBoundary`) ke registry MCP.
  - **Bukti Uji:** 6/6 unit tests MCP stdio sandbox lulus hijau (`mcp_stdio_default_config_is_sandboxed`, `mcp_stdio_opt_out_profiles_are_not_sandboxed`, `mcp_stdio_default_sandbox_strips_sensitive_environment`, `mcp_stdio_default_sandbox_blocks_deny_all_profile`, `mcp_stdio_sandbox_boundary_rejects_out_of_scope_access`, `mcp_stdio_sandbox_opt_out_bypasses_boundary`), 228/228 mcp tests di `clawcrew-tools` lulus. Akses di luar scope terbukti DITOLAK.
- [x] **BL-14 (P1, HIGH)** Terapkan `ContentSafety` ke jalur channel orchestrator biasa (non-SOP) — prompt-injection terbuka di semua channel saat ini, satu titik perbaikan menutup semua. **Catatan file-overlap:** menyentuh `clawcrew-channels/src/orchestrator/mod.rs` — file yang SAMA dengan BL-52 (Sprint 2). Serialkan: BL-14 (sprint ini) selesai dan merge dulu SEBELUM BL-52 mulai menyentuh file ini, supaya tidak dua crew menulis file sama secara paralel.
  - **Status Selesai:** Selesai diterapkan & diverifikasi 100% lulus test.
  - **Solusi Arsitektur:** Mengekstrak `ContentSafety` & `PromptGuard` ke crate `galleon-security-scrub` (menghormati invariant R-04 bahwa channel crates tidak boleh depend ke `clawcrew-runtime`).
  - **Implementasi:** Di `crates/clawcrew-channels/src/orchestrator/mod.rs:8757`, seluruh pesan inbound dari semua channel disaring melalui `content_safety.screen_content(&msg.content)`. Zero-width, control tokens (`<|im_start|>`), dan homoglyphs dinetralkan; injection dengan mode Block ditolak dini dan direply notice peringatan aman.
  - **Bukti Uji:** 58/58 tests di `galleon-security-scrub` lulus, 4/4 tests `content_safety` di `clawcrew-channels` lulus (`test result: ok. 4 passed`). Invariant `dependency_direction` R-04 tetap utuh.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A (Identity) | `engine/src/` identity module baru | Backend-Go |
| A (S2-SEC-01) | `engine/src/fleet/services.go` | Backend-Go — **setelah** BG-C1 merge |
| B (Sandbox/Vault) | `crates/clawcrew-gateway/src/{sandbox,vault}.rs` | Backend-Rust |
| C (BL-32) | `crates/clawcrew-gateway/src/{tests.rs,webhook_ingress.rs,lib.rs}` | Backend-Rust |
| C (BL-19) | MCP client host (`crates/clawcrew-plugins/` atau setara) | Backend-Rust |
| C (BL-14) | `crates/clawcrew-channels/src/orchestrator/mod.rs` | Backend-Rust — **harus selesai sebelum Sprint 2/BL-52 menyentuh file ini** |

## Keluaran Sprint

- Principal resolver Go hidup + dipakai approval-gate terminal (menutup `S2-SEC-01`).
- `cargo test -p clawcrew-gateway` ground-truth soal WhatsApp signature terjawab definitif (lolos atau gagal — keduanya hasil valid, yang penting bukan lagi tidak diketahui).
- MCP stdio sandboxed setara WASM plugin.
- `ContentSafety` aktif di seluruh channel non-SOP.
