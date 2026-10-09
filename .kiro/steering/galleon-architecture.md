---
title: Galleon Fleet — architecture & layering
inclusion: always
---

# Galleon Fleet — project rules

Workspace steering untuk repo `galleon-fleet` (monorepo 4-tier: Tauri shell + Rust core + Go engine + React UI). Sumber kanonikal: `standard-docs/`, `.gemini/steering.md`, `standard-docs/tech-standart/architecture/00-overview.md`.

## Branding
- Project di-rebrand dari `zeroclaw`/`clawcrew`/`Fleet AI` → **Galleon Fleet** (`galleon-fleet`). Binary: `galleon`, TUI: `galleoncode`, relay: `galleonrelay`.
- Crate/file/modul/dokumen BARU wajib pakai `galleon-*` (mis. `galleon-channel-*`).
- Env var baru: `GALLEON_*` (tanpa fallback, ODR-19).

## 4-tier layering (STRICT — jaga boundary)
| Tier | Lokasi | Runtime | Tugas | Komunikasi |
|---|---|---|---|---|
| Desktop Shell | `apps/tauri/` | Rust (Tauri v2) | window, tray, dialog OS, packaging desktop | Tauri IPC (`invoke`) |
| System Core | `crates/` | Rust 2024 (27 crates) | security microkernel, sandbox (Landlock), native tools, LLM SSOT (`clawcrew-providers`), vault, WASI | gRPC `SystemGateway` :50052 |
| AI Orchestrator | `engine/` | Go 1.27 | otak agen tunggal (`StartTurn`), fleet governance, memori persisten (SQLite/Postgres), pairing/QR | gRPC `AgentEngine` :50051 + HTTP :9090 |
| Interface / UI | `web/` | TS, React 19 + Tailwind 4 | rendering, view-model, `apiClient.ts`. **Zero backend logic.** | HTTP ke Go :9090 |

Aturan turunan:
- **UI tidak boleh berisi logika backend atau direct LLM call.** Hapus `@google/genai` dan pindahkan logika Express `server.ts` ke Go.
- **Eksekusi terminal dari Tauri wajib lewat Go** (ODR-07) untuk penegakan policy dan anti-injeksi: Tauri → Go (policy) → Rust (sandbox).
- **LLM Provider SSOT di Rust:** Go murni orkestrator context turn dan mendelegasikan LLM streaming ke Rust via gRPC `ExecuteTurn` (ADR 0001).

## Single Source Of Truth (dari AGENTS.md — gate review, bukan gaya)
- Sebelum menambah field/config/schema/cache/lookup paralel, identifikasi sumber kanonikal. Jika fakta sudah ada, resolve dari sana saat dipakai — jangan snapshot/duplikat.
- Tidak ada database bersama lintas tier.
- Zero `unwrap()` dan `expect()` di jalur produksi Rust.

## Alur kerja & Navigasi
- Navigasi wajib **Graphify-First** (`graphify-out/` query/path/explain) sebelum grep atau baca file.
- Branch non-`master`, PR ke `master` / `testing`. Conventional commits tanpa footer bot/AI.
- Validasi: `cargo nextest run -p <crate>`, `cargo clippy -p <crate> -- -D warnings`, `go test -race ./...`, `npm run lint`.
