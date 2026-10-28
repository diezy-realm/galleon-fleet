# Owner Decision Record — 2026-10-09: Scope Final T-02/T-04/T-06/T-08 (dev-task-2 remediation)

> **Status:** Accepted — rekomendasi squad-lead diterapkan sebagai keputusan final, tidak perlu konfirmasi ulang.
> **Konteks:** 4 item di `standard-docs/decisions/finalize/dev-task-2/finding/2026-10-08-remediation-tasks.md` ditandai "butuh keputusan arsitektur Owner" sebelum eksekusi. Investigasi fakta mendalam (2026-10-09) menghasilkan rekomendasi konkret untuk tiap item — dicatat di sini sebagai keputusan final supaya tim eksekusi tidak menunggu konfirmasi lebih lanjut.

## T-02 — BG-F5 (Go structured logging)

**Keputusan: Opsi B.** Tidak adopsi `zap`. Go `engine/` sudah pakai `log/slog` + modul tracing custom OpenTelemetry-compatible yang matang (JSON structured, log rotation, auto-redaksi kredensial) — zap tidak perlu dan tidak lebih baik.

**Scope task yang tersisa (bukan dibatalkan, hanya diperkecil):** sambungkan `tracing.StartSpan` ke jalur HTTP (`delivery.go`) supaya `trace_id` konsisten lintas HTTP→gRPC log→metadata ke Rust. Detail di `sprint-4-crate-decoupling-and-runtime-strangler.md` §BG-F5.

## T-04 — BL-31 (Memory hygiene backend-agnostic)

**Keputusan: Opsi B.** SQLite-only genuinely BY DESIGN, bukan bug — `backend.rs::uses_sqlite_hygiene` sudah berupa flag eksplisit (SQLite/Lucid=true, Postgres/Qdrant=false), konsisten dengan positioning produk local-first (`pref.galleon_memory_local_persistent`).

**Scope task yang tersisa:** tambahkan trait method `apply_retention` dengan default no-op implementation (sesuai `memory-hygiene-retention-trait-evaluation.md`) — kerja kecil (1 method + default), bukan generalisasi penuh ke 12 backend. Memberi upgrade-path untuk Qdrant/Postgres di masa depan tanpa memaksa kerja sekarang. Detail di `sprint-6-cli-client-protocol-and-providers.md` §BL-31.

## T-06 — BL-20/21/22 (MCP client host)

**Keputusan: Bukan keputusan arsitektur — audit awal yang salah.** MCP client host genuinely ADA di `crates/galleon-tools/src/mcp_*.rs` (10 file), terpisah total dari WASM plugin host (`galleon-plugins/`). Dokumen gap-analysis-nya (`mcp-sandbox-gap-and-vault-mediation.md`) sudah ditulis solid 2026-10-07, ADR WASM-vs-MCP (`decisions/adr-wasm-plugin-vs-mcp.md`) sudah `Accepted`.

**Tindakan:** BL-20/21/22 checkbox dipertahankan `[x]` di `sprint-5-mcp-plugin-and-subagent-resource.md`. **T-06 dihapus dari daftar kerja — tidak ada yang perlu dieksekusi tim.**

## T-08 — BG-C2 (dua device-registry paralel: Go `TokenStore` vs Rust `DeviceRegistry`)

**Keputusan: Opsi A.** Rust `DeviceRegistry` (`crates/clawcrew-gateway/src/api_pairing.rs`, SQLite-persistent) jadi Single Source of Truth. Go `memoryPairingService`/`TokenStore` (`engine/src/identity/`, in-memory) menjadi cache sinkron di atas Rust via gRPC `SystemGateway` existing — bukan sumber kebenaran independen.

**Rasional:** field keduanya hampir identik (DeviceID↔id, PairedAt↔paired_at, dst), Rust sudah persistent (survive restart), Go in-memory tidak (restart Go = device hilang dari sisi Go meski masih ada di Rust) — risiko drift nyata di produksi.

**Scope task:** implementasi jalur sinkron satu arah (Rust SSOT → Go cache), bukan desain dua arah. Detail di `sprint-0-security-foundation.md` §BG-C2.

---

**Catatan proses:** keempat rekomendasi ini sudah tertulis lengkap dengan bukti file:baris di `standard-docs/decisions/finalize/dev-task-2/finding/2026-10-09-architectural-decisions-analysis.md` sebelum ODR ini dibuat. ODR ini murni mengangkat status dari "rekomendasi" ke "keputusan final" agar tidak ditanyakan ulang.
