# 05 — Analisis Penempatan Agent Orchestrator (Go vs Rust)

> Observasi 2026-10-04. Menjawab pertanyaan Owner: apakah orchestrator sebaiknya di Go, dan apakah CLI harus Rust.

## 1. Bukti

| Fakta | Sumber |
|---|---|
| Aturan normatif: Go = "agent brain (`StartTurn`), fleet governance, RAG"; Rust = leaf service, dilarang crew orchestration | `tech-standart/architecture/01-boundary-policy.md` |
| Go **sudah** memanggil Rust untuk LLM: `StartTurn` menyusun `ExecuteTurnRequest` dan membuka stream ke Rust | `engine/src/crew/services.go:38,81,100` |
| Rust `execute_turn` memakai `ModelProvider` (LLM SSOT) | `crates/galleon-gateway/src/grpc_system_gateway.rs:24,412` |
| **Tetapi** Rust punya otak kedua: `runtime/agent` 77k LOC, `sop` 38k, `rpc` 32k, `skills` 12k, `cron` 10k, `control_plane` 7k | ukuran modul runtime |
| Otak Rust itu **masih dipakai**: `src/main.rs` (CLI), `gateway/{api,api_config,ws,a2a}.rs` | grep `agent::` |

## 2. Kesimpulan

1. **Ya, orchestrator seharusnya di Go.** Alasan: (a) sudah ditetapkan boundary & ODR-03; (b) memori persisten, hierarki Quartermaster→Ship→Squad→Crew, policy/approval, dan DB berada di Go — agent loop yang terpisah akan membaca/menulis state ganda (melanggar SSOT); (c) Go adalah satu pintu keamanan (ODR-07).
2. Hierarki sudah *tertulis*, tetapi **kodenya belum mengikuti**: ±180k LOC otak Rust masih aktif lewat CLI dan REST/WS gateway. Itu utang terbesar; memecah crate sebelum klasifikasi akan memperbanyak kode yang nantinya dibuang.
3. **Rust tetap benar untuk:** LLM executor (`providers`), tool native + sandbox, vault, channel, WASI host, hardware, i18n, scrub, CLI binary.

## 3. CLI di Rust — pendapat

Memilih Rust untuk **binary CLI** tepat (satu executable, startup cepat, mudah didistribusi, sudah ada `src/commands`, `apps/zerocode`). Yang dipisahkan adalah **tanggung jawab**, bukan bahasa:

| Tanggung jawab | Pemilik |
|---|---|
| Parsing perintah, UX terminal/TUI, streaming output | **CLI Rust** |
| `service`/daemon, `doctor`, `onboard`, instalasi, vault lokal, hardware | **CLI Rust** (urusan host lokal) |
| Chat / run quest / approval / memori / fleet | **Go** — CLI memanggil HTTP :9090 (SSE/WS) |
| LLM call, tool native | **Rust gateway** (dipanggil Go, bukan CLI langsung) |

Mode: `galleon` (CLI) menjalankan/menemukan engine Go sebagai sidecar lokal, lalu menjadi klien. Tanpa fallback "agent loop lokal" — sesuai fail-closed.

## 4. Konsekuensi untuk tiket

- R-00 diprioritaskan; tambah sub-keputusan per modul `agent`, `sop`, `rpc`, `skills`, `cron`, `control_plane`, `verifiable_intent`.
- G-lane harus menyerap fungsi yang dipindahkan (SOP, skills, cron) — tiket G-30..G-33.
- R-34 diubah: CLI Rust = klien Go untuk orkestrasi (bukan klien tipis untuk semua).
- REST/WS/A2A gateway Rust lama dihapus bila tidak ada pemakai (R-14).
