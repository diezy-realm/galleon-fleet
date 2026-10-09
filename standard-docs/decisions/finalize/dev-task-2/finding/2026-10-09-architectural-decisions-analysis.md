---
title: Analisis Fakta — T-02/T-04/T-06/T-08 (Item Butuh Keputusan)
status: analysis-complete
date: 2026-10-09
author: squad-lead
scope: Investigasi mendalam ke kode nyata untuk 4 item yang digating di 2026-10-08-remediation-tasks.md. Setiap klaim diverifikasi file:baris langsung, bukan diulang dari audit permukaan sebelumnya.
---

# Analisis Fakta — Item yang Butuh Keputusan

Dokumen ini adalah hasil investigasi mendalam (bukan audit ulang) untuk T-02, T-04, T-06, T-08. **Dua dari empat ternyata overturned sepenuhnya oleh fakta baru** — audit permukaan sebelumnya (grep sempit) melewatkan konteks yang mengubah kesimpulan secara material. Rekomendasi final per item di bawah.

---

## T-02 — BG-F5 (zap logging) — REKOMENDASI: Opsi B, tapi dengan temuan baru yang memperbaiki scope

**Fakta baru yang mengubah gambaran:** audit awal benar bahwa `zap` nol diadopsi — tapi **Go `engine/` genuinely SUDAH punya structured logging yang matang**, hanya berbeda library:

- `log/slog` (stdlib Go) dipakai konsisten di 9 file inti: `app.go`, `main.go`, `interceptors.go`, `core/logger/logger.go`, `core/tracing/tracing.go`, `system_gateway.go`, `dispatcher.go`, `services.go`.
- `core/logger/logger.go` — JSON structured logging (`slog.NewJSONHandler`), log rotation (`lumberjack`), **auto-redaction kredensial** (regex untuk API key, Bearer token, GitHub token, private key) di `RedactAttr`/`RedactString`. Ini infrastruktur produksi matang, bukan stub.
- `core/tracing/tracing.go` — modul tracing custom OpenTelemetry-compatible (`Span` dengan `TraceID`/`SpanID`/`ParentSpanID`, `StartSpan`/`End`).
- `core/interceptors/interceptors.go` — gRPC interceptor yang log setiap unary/stream call dengan `method`, `duration_sec`, `status_code` via `slog`.
- `pkg/client/system_gateway.go:17-20` — komentar eksplisit bertanda `(BG-F5)`: `x-request-id` DARI HTTP header SUDAH dipropagasi ke outgoing gRPC metadata ke Rust.

**Gap genuine yang tersisa (bukan "zap hilang", tapi ini):** modul tracing (`StartSpan`) **tidak pernah dipanggil dari HTTP handler** (`delivery.go`) — `grep StartSpan` di luar file testnya sendiri = 0 match. Request-id dari HTTP header dan sistem tracing span adalah dua potongan terpisah yang tidak tersambung. request-id berhasil nyambung ke Rust (gRPC metadata), tapi TIDAK pernah masuk ke `Span`/trace_id di sisi Go sendiri.

**Rekomendasi:** **Opsi B (tidak perlu adopsi zap)** — infrastruktur logging-nya sudah ada dan lebih cocok (stdlib, lebih ringan dari zap, sudah redaksi kredensial). TAPI tambahkan task baru yang lebih presisi dari BG-F5 asli: **sambungkan `tracing.StartSpan` ke HTTP request path di `delivery.go`** sehingga `trace_id` konsisten antara HTTP, gRPC log, dan request-id yang dikirim ke Rust. Checkbox BG-F5 turunkan ke `[ ]` dengan deskripsi baru: "zap tidak diadopsi (keputusan sadar — slog+tracing custom sudah ada dan lebih sesuai); gap genuine = StartSpan belum dipanggil dari delivery.go, trace_id tidak konsisten lintas HTTP->gRPC log."

---

## T-04 — BL-31 (Hygiene backend-agnostic) — REKOMENDASI: Opsi B (SQLite-only by design), DENGAN upgrade path tervalidasi

**Fakta yang mengonfirmasi "by design":**
- `crates/clawcrew-memory/src/backend.rs` sudah punya klasifikasi EKSPLISIT `uses_sqlite_hygiene`/`sqlite_based` per backend — SQLite dan Lucid = `true`; Postgres, Qdrant, Markdown, None = `false`. Ini bukan kecelakaan arsitektur, ini **flag yang sudah ditulis dengan sengaja**.
- Produk Galleon diposisikan "local-first" (`product/galleon-product-definition.md`, `market-strategic-report-oct2026.md`) — SQLite/Markdown untuk desktop single-user, Postgres/Qdrant untuk skenario server/Dockyard multi-tenant yang retention-nya diharapkan dikelola ADMIN DI LEVEL DATABASE (pg_cron, Qdrant collection TTL), bukan oleh aplikasi.
- Verifikasi langsung `postgres.rs`/`qdrant.rs`: nol method TTL/retention — konsisten dengan "retention didesain untuk dikelola di luar aplikasi untuk backend ini."

**Fakta tambahan — SUDAH ADA dokumen keputusan tertulis** (`memory-hygiene-retention-trait-evaluation.md`, 2026-10-08): mengusulkan trait method `apply_retention` dengan **default no-op implementation** di trait `Memory` — artinya backend yang tidak butuh hygiene (Postgres/Qdrant saat ini) otomatis inherit no-op TANPA kerja tambahan, SQLite dapat implementasi nyata (reuse `hygiene.rs`), dan baru upgrade Qdrant ke real retention "saat production scale" (fase masa depan, bukan sekarang).

**Rekomendasi:** **Opsi B dengan tambahan konkret** — SQLite-only SAAT INI sudah benar secara desain (bukan bug), TAPI implementasikan trait method `apply_retention` (default no-op) dari dokumen evaluasi yang sudah ada — ini kerja KECIL (satu method trait + default), bukan generalisasi penuh ke 12 backend. Ini memberi upgrade path tanpa memaksa kerja besar sekarang. Checkbox BL-31 turunkan ke `[ ]` dengan deskripsi: "SQLite-only genuinely by design (lihat backend.rs::uses_sqlite_hygiene + evaluasi 2026-10-08), tambahkan trait method apply_retention (default no-op) agar backend lain bisa upgrade nanti tanpa refactor besar."

---

## T-06 — BL-20/21/22 (MCP client host) — REKOMENDASI: Opsi A TERBALIK — audit lama SALAH, bukan tiketnya

**TEMUAN KRITIS — audit sebelumnya genuinely keliru, bukan sekadar ambigu:**

MCP client host **GENUINELY ADA** di `crates/clawcrew-tools/src/`: `mcp_transport.rs`, `mcp_client.rs`, `mcp_deferred.rs`, `mcp_protocol.rs`, `mcp_context.rs`, `mcp_prompt.rs`, `mcp_prompts_tool.rs`, `mcp_resource.rs`, `mcp_resources_tool.rs`, `mcp_tool.rs` — **10 file, dikonfirmasi ada di disk** (`glob` langsung, bukan grep lama yang keliru scope ke `crates/**/*.rs` dengan pattern `\bmcp\b` yang entah kenapa menghasilkan nol).

Dokumen `mcp-sandbox-gap-and-vault-mediation.md` (2026-10-07, SUDAH ADA, ditulis SEBELUM audit saya) sudah mengonfirmasi ini dan menjelaskan dengan tepat:
- MCP Client Host = `clawcrew-tools` (JSON-RPC via stdio/SSE/HTTP streaming) — protokol MCP standar industri.
- WASM plugin host = `clawcrew-plugins` (wasmtime, Component Model) — **arsitektur BERBEDA TOTAL**, bukan nama lain untuk hal yang sama.
- Gap sandbox MCP vs WASM sudah dianalisis: MCP stdio process TIDAK punya `McpCommandBoundary` (Landlock/Bubblewrap/Seatbelt/AppContainer) setara dengan isolasi WASM — proses MCP subprocess berjalan dengan privilege host penuh kalau sandbox profile tidak diset `deny-all`/`blocked`.
- Desain vault-mediation untuk secret MCP sudah dirancang (JIT decrypt dari `secrets::decrypt`, format `$clawvault$v1$...`).

**Rekomendasi:** **BUKAN** Opsi A/B seperti saya tawarkan sebelumnya — pertanyaannya sendiri salah premis. BL-20/21 **genuinely SUDAH dikerjakan dengan benar** (dokumen analisis gap sudah ditulis, lengkap, akurat) — audit permukaan saya sebelumnya yang keliru karena scope grep terlalu sempit/sudah stale sebelum dokumen 2026-10-07 ditulis. **Checkbox BL-20/BL-21 BISA naik ke `[x]`** (dokumentasi gap sudah ada dan solid) — TAPI BL-22 (disebut "ADR WASM vs MCP") perlu dicek terpisah apakah ADR formal genuinely ditulis atau baru di level working-doc seperti ini. **Hapus T-06 dari daftar "butuh keputusan Owner"** — tidak ada keputusan arsitektur yang perlu diambil, hanya perlu verifikasi checkbox vs dokumen yang sudah ada.

---

## T-08 — BG-C2 (dua device-registry paralel) — REKOMENDASI: Opsi A, dikonfirmasi SSOT violation nyata

**Fakta yang mengonfirmasi (bukan asumsi):**

Perbandingan field-by-field membuktikan ini genuinely SATU ENTITY yang disimpan dua kali:

| | Go `PairedDevice` (`engine/src/identity/pairing.go`) | Rust `DeviceInfo` (`clawcrew-gateway/src/api_pairing.rs`) |
|---|---|---|
| Identitas | `DeviceID` | `id` |
| Nama | `DeviceName` | `name` |
| Platform/tipe | `Platform` | `device_type` |
| Waktu pairing | `PairedAt` | `paired_at` |
| Terakhir aktif | `LastSeenAt` | `last_seen` |
| Status | `Revoked` (bool) | (tidak ada — row dihapus/diupdate) |
| Storage | **in-memory** (`map[string]*PairedDevice`, hilang saat restart Go) | **SQLite** (`devices.db`, persistent) |

**Keduanya genuinely live di production**, bukan satu dipakai satu tidak:
- Go: `engine/src/fleet/delivery.go:28-30` — `identity.NewMemoryTokenStore()` + `identity.NewPairingService(tokenStore)` dipanggil di jalur produksi (bukan hanya test).
- Rust: `crates/clawcrew-gateway/src/api.rs:5612,5923` — `DeviceRegistry::new(&data_dir)` dipanggil di jalur produksi gateway.

**Catatan penting — bukan false-positive seperti T-06:** nama `DeviceRegistry::new()` TANPA argumen juga muncul 14x di `clawcrew-hardware/src/device.rs` — **itu struct BERBEDA, domain hardware/GPIO, tidak terkait T-08**. Hanya `DeviceRegistry::new(&data_dir)` (dengan path argumen) di `clawcrew-gateway` yang relevan — sudah saya pisahkan secara eksplisit saat verifikasi agar tidak salah hitung seperti kemungkinan kekeliruan grep T-06 sebelumnya.

**Risiko konkret (bukan hipotetis):** restart Go engine (`memoryTokenStore`/`memoryPairingService` in-memory) menghapus SEMUA sesi pairing device dan daftar `PairedDevice` yang baru di-claim, SEMENTARA Rust `DeviceRegistry` (SQLite) tetap menyimpan device yang sama dari sisi lain. Device yang di-pair lewat Go akan "hilang" dari sudut pandang Go setelah restart meski Rust masih punya recordnya — user harus pairing ulang padahal device-nya sudah pernah terdaftar di Rust.

**Rekomendasi:** **Opsi A** — jadikan Rust `DeviceRegistry` (SQLite, persistent) sebagai SSOT. Go `PairingService`/`TokenStore` tetap untuk session-token sementara (itu memang domain Go — token sesi HTTP/WS, bukan device record), TAPI `PairedDevice` record (hasil `Claim()`) harus ditulis ke Rust `DeviceRegistry` via gRPC (ada jalur `SystemGateway` existing yang bisa diperluas), bukan hanya `map` in-memory Go. Ini bukan "pisahkan sepenuhnya" (Opsi B) karena keduanya genuinely merepresentasikan fakta yang sama (device yang di-pair) — harus satu sumber kebenaran.

---

## Ringkasan rekomendasi final

| Item | Rekomendasi | Alasan singkat |
|---|---|---|
| T-02 | **Opsi B** + task baru (sambungkan tracing ke HTTP) | slog+tracing custom sudah lebih baik dari zap; gap sebenarnya di wiring, bukan library |
| T-04 | **Opsi B** + tambahan kecil (trait no-op) | SQLite-only genuinely by design (flag eksplisit di backend.rs), upgrade path murah sudah didesain |
| T-06 | **BUKAN keputusan — audit lama salah** | MCP host genuinely ada, dokumen gap sudah ditulis solid, BL-20/21 bisa `[x]`, cek BL-22 ADR terpisah |
| T-08 | **Opsi A** | Field-by-field match persis, dua jalur produksi live bersamaan, risiko restart-data-loss konkret, bukan hipotetis |

**Catatan proses:** T-06 adalah pelajaran penting — audit grep permukaan (`\bmcp\b` di `crates/**/*.rs`) menghasilkan 0 match yang KELIRU, kemungkinan karena pattern regex atau scope path yang salah, dan ini nyaris menyebabkan tiket yang sudah benar dikerjakan ditandai sebagai "scope tidak jelas." Pelajaran: untuk klaim "X tidak ada", verifikasi dengan `glob` nama file eksplisit DAN grep, jangan grep saja — terutama kalau hasilnya 0 match yang mengejutkan.
