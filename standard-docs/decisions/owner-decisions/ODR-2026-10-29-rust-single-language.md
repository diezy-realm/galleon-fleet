# ODR-2026-10-29 — Rust sebagai Bahasa Tunggal Produk Core `galleon-fleet`

- **Status:** Ratified
- **Tanggal:** 2026-10-29
- **Otoritas:** Owner (Achmad)
- **Konteks:** Galleon Fleet Platform — repository `diezy-realm/galleon-fleet` (branch kerja `testing`)
- **Terkait:** Supersedes sebagian `ODR-2026-10-04-product-direction` (ODR-02 klausa "Go sebagai gerbang", ODR-05 implikasi store Go, ODR-07, asumsi A1/A3/A4), `ODR-2026-10-04-addendum-1` (ODR-11, ODR-12, ODR-14), `ODR-2026-10-04-addendum-3` (implikasi ODR-20 tentang retarget eval ke Go), `ODR-2026-10-04-addendum-4` (ODR-22, ODR-23), `ODR-2026-10-05-delegated-authority-chain` (§ pemisahan peran Go), `ODR-2026-10-09-dev-task-2-remediation-scope` (butir khusus Go: T-02 logging Go, bagian Go di T-08). ADR-0018 (arsitektur bahasa tunggal), PRD-002, US-009.

> Sifat: immutable. Perubahan = ODR baru yang menyatakan `Supersedes ODR-2026-10-29`.
> Dokumen ini disusun oleh asisten AI atas instruksi langsung Owner tanggal 2026-10-29; isinya adalah keputusan Owner, bukan keputusan agent.

---

## 1. Latar Belakang & Masalah

Arah 2026-10-04 menempatkan Go sebagai "otak" (ODR-03/ODR-11) dan Rust sebagai system gateway. Audit 2026-10-29 (`board/research/RES-2026-10-29-rust-single-language-baseline.md`) menemukan fakta berikut di `testing@3f3a729`:

1. **Kontrak Go↔Rust drift tanpa terdeteksi.** Rust `execute_turn` mengabaikan `system_prompt`, `tools`, `max_tokens`; tidak pernah mengirim `TurnToolCall`; Go menandai turn "completed successfully" saat stream berakhir walau Rust tidak mengirim `Complete`. Ini terjadi karena dua bahasa memegang dua salinan kontrak dan ownership modul berpindah-pindah.
2. **Go tidak dikirim di image produksi.** `Dockerfile` hanya build Rust; sidecar Tauri hanya menjalankan Rust daemon. Dashboard web/desktop menunjuk Go `:9090`, sehingga tidak berfungsi tanpa Go dijalankan manual.
3. **gRPC SystemGateway Rust tidak pernah dijalankan** (`start_grpc_server` tidak dipanggil).
4. **Separuh paket Go tidak terhubung** ke graf produksi (squad, ship, mission, schedule, protocol, policy, treasury, store sqlite, learning handler, pairing routes).
5. **Celah keamanan nyata di Go fleet API** (bind `:9090` semua interface, `/api/terminal/exec` tanpa auth untuk tier EXECUTE, bypass kuota koleksi, git-status sukses palsu).
6. **Router produksi Rust gateway terkikis** oleh refactor `db18a3745` (152 path → ±23), termasuk rute yang masih dipanggil CLI (`/admin/*`, `/pair`). Penghapusan itu dilakukan atas asumsi A3/ODR-14 tanpa inventaris pemakai yang disyaratkan ODR-14.

Owner menilai biaya mempertahankan dua bahasa di produk core lebih besar dari manfaatnya, dan memutuskan migrasi total ke Rust.

## 2. Pernyataan Keputusan (Mandat Mutlak)

1. **ODR-26 — Bahasa tunggal.** Seluruh kode produksi repository `galleon-fleet` ditulis dalam **Rust** (backend, CLI, daemon, gateway, desktop shell Tauri) dan **TypeScript** hanya untuk UI `web/`. Go **tidak** boleh ada di repository ini setelah Sprint 13 selesai (`engine/`, `go.work`, `proto/` untuk Go, workflow Go dihapus).
2. **ODR-27 — Satu permukaan API produk.** Rust gateway (`galleon-gateway`, port default 42617, bind default `127.0.0.1`) adalah **satu-satunya** API untuk Web, Desktop, CLI, dan QR/pairing. Menggantikan ODR-14 dan asumsi A3/A4: rute REST Rust **adalah** permukaan produk. Rute dipasang hanya bila punya konsumen yang tercatat di `api-contracts/consumer-inventory.md`.
3. **ODR-28 — Satu otak.** Agent loop, orkestrasi, SOP, subagent, cron, approval, identitas/pairing, dan eksekusi terminal ada di Rust (`galleon-runtime`, `galleon-gateway`, crate terkait). Menggantikan ODR-11, ODR-12 (CLI Rust tidak lagi klien ke Go), ODR-22, dan klausa "Go sebagai gerbang" di ODR-02/ODR-07. Eksekusi terminal dari Tauri/Web melewati Rust gateway → `SecurityPolicy` allowlist → `ApprovalManager` → sandbox tool Rust.
4. **ODR-29 — Kontrak dikunci oleh test, bukan oleh bahasa ganda.** Setiap rute yang dikonsumsi UI/Desktop/CLI wajib punya test kontrak (snapshot JSON + test 401/403/4xx) di crate Rust. Perubahan kontrak tanpa update snapshot = CI merah.
5. **ODR-30 — Add-on Go di repository terpisah.** Kebutuhan tambahan yang ingin ditulis dalam Go dibuat di **repository terpisah** di org `diezy-realm` dan hanya boleh berintegrasi lewat kontrak publik yang sudah ada: HTTP API gateway (dengan token pairing), MCP, atau plugin WASM `galleon:plugin@1.0.0` (`wit/v1/`). Repository core **tidak** memuat SDK, proto, atau kode Go milik add-on.
6. **ODR-31 — YAGNI pada port.** Yang di-port hanya perilaku Go yang **terpasang dan punya konsumen**. Paket Go yang tidak terhubung tidak di-port; bila produk butuh (mis. Squad/Ship/Mission), dibuat sebagai story baru dengan desain Rust.
7. **ODR-32 — Persistensi.** Arah ODR-05 (SQLite lokal) tetap. Untuk data dashboard fleet, format file JSON yang ada **dipertahankan** selama migrasi agar data pengguna tidak perlu dikonversi; ADR-005 (desain store Go) dinyatakan Superseded. Pergantian ke SQLite untuk data fleet = story terpisah.
8. **ODR-33 — Ukuran task.** Setiap task eksekusi di board dirancang selesai **≤ 1 jam** oleh satu agent, dengan satu branch `feat/...` dan satu PR ke `testing`.

Keputusan yang **tetap berlaku** tanpa perubahan: ODR-01, ODR-04, ODR-06, ODR-08, ODR-09, ODR-10, ODR-13, ODR-15, ODR-16, ODR-17, ODR-18, ODR-19, ODR-21, ODR-24, ODR-25, ODR-20 (kecuali implikasi retarget eval ke Go yang kini tidak relevan), ODR-03 (fokus produk "Organisasi AI" tetap; implikasi "otak Go" diganti ODR-28), dan Delegated Authority Chain sebagai **gate wajib** (pemeriksa principal pindah ke Rust gateway).

## 3. Batasan & Larangan (Guardrails)

- **Dilarang:** menambah file `*.go`, `go.mod`, `go.sum`, `go.work`, atau workflow Go baru di `galleon-fleet`. Satu-satunya pengecualian sampai Sprint 13: perbaikan keamanan kontainmen Go yang disetujui di SPEC-011 (bind loopback, menutup `/api/terminal/exec`).
- **Dilarang:** menambah gRPC/protobuf internal Rust↔Rust. Komunikasi antar crate = panggilan fungsi Rust.
- **Dilarang:** memasang rute tanpa `require_auth`, kecuali daftar publik di `01-boundary-policy.md` v2 §4.
- **Dilarang:** mengembalikan respons sukses palsu (default `clean:true`, `evaluated-pass`, "completed successfully") saat operasi gagal.
- **Dilarang:** mem-port paket Go yang tidak terhubung atau stub (ODR-31).
- **Dilarang:** menambah dependensi crate baru untuk port fleet API bila fungsi setara sudah ada di `Cargo.lock`.
- **Diharuskan:** setiap PR port menyertakan test kontrak (ODR-29) dan bukti `cargo nextest run -p <crate>` + `cargo clippy -p <crate> -- -D warnings` di laporan RPT.
- **Diharuskan:** crate baru menulis `[lints] workspace = true` dan `#![forbid(unsafe_code)]`.
- **Diharuskan:** Go baru boleh dipakai hanya di repository add-on terpisah (ODR-30).

## 4. Dampak Arsitektur & Operasional

Model 4-tier diganti model 3-tier (lihat `01-boundary-policy.md` v2):

- **Tier 1 (Desktop — `apps/tauri/`):** shell tipis; memanggil Rust gateway lokal (42617) dengan token pairing. `GALLEON_ENGINE_URL`/`:9090` dihapus. CSP `connect-src` ke gateway.
- **Tier 2 (Rust Core — `crates/*`, `src/`):** satu-satunya backend. Menyerap fleet dashboard API (crate baru `galleon-fleet-core` + adapter `galleon-gateway/src/api_fleet/`). `strangler_runner.rs`, `grpc_system_gateway.rs`, `start_grpc_server`, `build.rs` proto dihapus.
- **Tier 3 (Go Engine):** **dihapus** dari repository ini. Add-on Go hidup di repository lain (ODR-30).
- **Tier 4 (UI Web — `web/`):** proxy dev menunjuk Rust gateway; zero backend logic tetap berlaku. Nginx upstream ke gateway Rust.

Operasional: satu binary (`galleon`) + UI statis. CI kehilangan job Go (`engine-ci.yml`, deteksi path Go di `pr-testing-gate.yml`).

## 5. Rencana Pelaksanaan & Verifikasi

- [ ] Sprint 09 — keputusan & aturan masuk repo, steering diperbarui, kontainmen keamanan Go (SPEC-011).
- [ ] Sprint 10 — pemulihan router Rust yang punya konsumen + layer keamanan + inventaris konsumen (SPEC-012).
- [ ] Sprint 11 — port fleet dashboard API ke Rust dengan test kontrak (SPEC-013).
- [ ] Sprint 12 — repoint Web/Desktop/Nginx ke Rust gateway, smoke E2E (SPEC-014).
- [ ] Sprint 13 — dekomisioning Go + proto + strangler + guard CI "no Go" (SPEC-015).
- [ ] Sprint 14 — higiene Rust: pewarisan lint dengan ratchet, doctest lane, sccache kondisional, drift tooling, Sonar kondisional (SPEC-016).
- [ ] Sprint 15 — pemulihan wiring serve gateway yang hilang di `db18a3745` (TLS/mTLS, peringatan public bind, `path_prefix`, tunnel) + ekstraksi `crates/galleon-tunnel` (SPEC-017). Boleh diaktifkan tepat setelah Sprint 10 (RULES amendment §A4).
- [ ] Verifikasi akhir: `rg --files -g '*.go' -g 'go.mod' -g 'go.work'` di repo = kosong; `rg -n ':9090|GALLEON_ENGINE_URL|ENGINE_URL'` di kode produksi = kosong; seluruh rute di `consumer-inventory.md` punya test kontrak hijau.
