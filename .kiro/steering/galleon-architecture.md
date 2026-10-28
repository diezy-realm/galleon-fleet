---
title: Galleon Fleet — architecture & layering
inclusion: always
---

# Galleon Fleet — project rules

Workspace steering untuk repo `galleon-fleet` (monorepo 3-tier: Tauri/Client shell + Rust core/gateway + React UI). Sumber kanonikal: `standard-docs/`, `.gemini/steering.md`, `standard-docs/tech-standart/architecture/00-overview.md`, `standard-docs/tech-standart/architecture/01-boundary-policy.md` (v2), dan `standard-docs/decisions/0018-rust-single-language-architecture.md` (ADR-0018).

## Branding
- Project di-rebrand dari `zeroclaw`/`clawcrew`/`Fleet AI` → **Galleon Fleet** (`galleon-fleet`). Binary: `galleon`, TUI: `galleoncode`, relay: `galleonrelay`.
- Crate/file/modul/dokumen BARU wajib pakai `galleon-*` (mis. `galleon-channel-*`).
- Env var baru: `GALLEON_*` (tanpa fallback, ODR-19).

## 3-tier layering (STRICT — jaga boundary)
| Tier | Lokasi | Runtime | Tugas | Komunikasi |
|---|---|---|---|---|
| Tier 1: Client/Desktop Apps | `apps/tauri/`, CLI | Rust 2024 (Tauri v2 / CLI binary) | Window shell, tray, dialog OS native, client transport ke gateway | Tauri IPC (`invoke`) |
| Tier 2: System Core | `crates/`, `src/` | Rust 2024 (HTTP/WS :42617 `galleon-gateway`) | Gateway HTTP/WS, security microkernel (sandbox, vault AES-256-GCM), native tools (fs/shell), **LLM execution SSOT (ADR 0001)**, agent runtime, WASI plugin host, channels, fleet dashboard domain (`galleon-fleet-core`) | HTTP/WS :42617 |
| Tier 3: UI Layer | `web/` | TS, React 19 + Tailwind 4 | rendering, view-model, client-bridge (`apiClient.ts`). **Zero backend logic.** | HTTP ke Gateway :42617 |

Aturan turunan:
- **UI tidak boleh berisi logika backend atau direct LLM call.** Hapus `@google/genai` dan pindahkan logika backend ke System Core / Gateway Rust (ADR 0002).
- **Larangan Go Engine:** Penulisan kode Go / engine Go dilarang di core repository ini; arsitektur backend dikonsolidasikan penuh ke Rust sesuai ADR-0018 dan 01-boundary-policy v2.
- **LLM Provider SSOT di Rust:** Semua vendor LLM dijalankan di Rust `galleon-providers` (ADR 0001).

## Single Source Of Truth (dari AGENTS.md — gate review, bukan gaya)
- Sebelum menambah field/config/schema/cache/lookup paralel, identifikasi sumber kanonikal. Jika fakta sudah ada, resolve dari sana saat dipakai — jangan snapshot/duplikat.
- Tidak ada database bersama lintas tier.
- Zero `unwrap()` dan `expect()` di jalur produksi Rust.

## Alur kerja & Navigasi
- Navigasi wajib **Graphify-First** (`graphify-out/` query/path/explain) sebelum grep atau baca file.
- Branch non-`master`, PR ke `master` / `testing`. Conventional commits tanpa footer bot/AI.
- Validasi: `cargo nextest run -p <crate>`, `cargo clippy -p <crate> -- -D warnings`, `npm run lint`.

## Landasan vs pekerjaan berjalan (SSOT dipisah)
- **`standard-docs/`** = landasan baku saja (arsitektur, standar teknis, arah produk/ODR). Onboarding: baca `standard-docs/ONBOARDING.md` dulu.
- **Pekerjaan berjalan** (riset, backlog, task, sprint, report, governance) ada di papan **`~/Apps/board/`** — aturannya di steering `board-coordination.md` + `~/Apps/board/RULES.md`.
- `roadmap/` + `decisions/finalize/dev-task-2|3` **tetap** di `standard-docs/` (SSOT tiket aktif; dev-task-2/3 dikerjakan tim lain — jangan sentuh tanpa konfirmasi).
