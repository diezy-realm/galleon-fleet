# Galleon Fleet AI — Copilot Instructions & Steering Rules

**Single Source of Truth (SSOT):** [`standard-docs/tech-standart/`](../standard-docs/tech-standart/)  
This document governs AI agent behavior, architecture compliance, coding standards, and codebase navigation across all sessions.

---

## 1. Graph-First Codebase Navigation Principle

**Rule:** Graph first, grep second, manual browsing never.

For every query involving architecture, symbols, call sites, dependencies, or paths:
1. **Check & Index:** Verify `graphify-out/graph.json` exists (`Test-Path graphify-out/graph.json`). Run `graphify update .` (or `graphify .` if missing).
2. **Query the Graph:** Use `graphify query "..."`, `graphify path "..." "..."`, or read `graphify-out/GRAPH_REPORT.md` / `wiki/`.
3. **Targeted Search:** Only after Graphify identifies the target file/symbol, use targeted grep (`Select-String` / ripgrep) for specific lines.
4. **Direct Read:** Open only the specific files identified. Do not do blind recursive directory searches or scan files blindly.

---

## 2. 3-Tier Layering & Strict Boundary Policy (`standard-docs/tech-standart/architecture/01-boundary-policy.md`)

All changes MUST preserve the separation of concerns (penulisan kode Go/engine Go dilarang; seluruh arsitektur backend dikonsolidasikan ke Rust sesuai [ADR-0018](../standard-docs/decisions/0018-rust-single-language-architecture.md) dan [`01-boundary-policy.md` v2](../standard-docs/tech-standart/architecture/01-boundary-policy.md)):

| Tier | Runtime & Port | Tanggung Jawab Resmi (Owns) | Hal Dilarang Keras (Forbidden) |
|---|---|---|---|
| **Tier 1: Desktop Shell** (`apps/tauri/`) | Tauri v2 (Rust IPC) | OS window lifecycle, system tray, native file dialogs, desktop bundling (`.exe`, `.dmg`). | **NO business logic**, NO API handlers, NO direct persistence, NO fake metrics (ADR 0003). |
| **Tier 2: System Core** (`crates/`, `src/`) | Rust 2024 (HTTP/WS :42617 `galleon-gateway`) | Gateway HTTP/WS, security microkernel (sandbox, vault AES-256-GCM), native tools (fs/shell), **LLM execution SSOT (ADR 0001)**, agent runtime, WASI plugin host, channels, fleet dashboard domain (`galleon-fleet-core`). | **NO gRPC/protobuf internal**, handler HTTP tanpa auth (kecuali rute publik). |
| **Tier 3: UI Layer** (`web/`) | React 19 + TS 5.7+ (Vite + Tailwind 4) | View rendering, Tailwind components, view-models, client-bridge (`apiClient.ts`). | **Zero backend logic (ADR 0002)**, NO QR code/OS telemetry generation in UI server, memanggil LLM langsung (ODR-08). |

### Data Flow Invariants:
1. **Flow Hierarchy:** `Frontend (web / Tauri) → Rust Gateway HTTP/WS (:42617)`. Direct frontend bypass ke internal crate dilarang.
2. **LLM SSOT (ADR 0001):** All LLM vendor calls (OpenAI, Gemini, Bedrock, Anthropic) live in Rust `galleon-providers`.
3. **Secrets SSOT:** Stored in Rust Vault (`~/.config/galleon/vault/` AES-256-GCM).
4. **Policy SSOT:** Rules for auto-approval and risk-tiers exist in Rust (`galleon-runtime` / `galleon-gateway`).
5. **No Shared Database:** Each tier strictly isolates storage (Tauri: `window.json`, Rust: `vault/`, `DeviceRegistry` SQLite, `galleon-fleet-core` JSON).
6. **Larangan Go Engine:** Penulisan kode Go / engine Go dilarang di core repository ini; arsitektur backend dikonsolidasikan penuh ke Rust sesuai [ADR-0018](../standard-docs/decisions/0018-rust-single-language-architecture.md) dan [`01-boundary-policy.md` v2](../standard-docs/tech-standart/architecture/01-boundary-policy.md).

---

## 3. Rust Standards (`standard-docs/tech-standart/tech-stack/rust-conventions.md`)
- **Edition & MSRV:** Rust 2024, MSRV 1.82+.
- **Branding:** All crates MUST use prefix `galleon-` (e.g. `galleon-channel-matrix`, `galleon-tools`).
- **Dependencies:** Declare centrally in root `Cargo.toml` (`[workspace.dependencies]`). Use `dep.workspace = true`.
- **Error Handling:**
  - Define domain errors with `thiserror`. Define `pub type Result<T> = std::result::Result<T, Error>;`.
  - **STRICTLY NO `unwrap()` OR `expect()` in production paths.** Propagate with `?`.
- **Concurrency & Observability:** Tokio async standard (`tokio::select!`, handle background tasks). Tracing via `#[instrument(skip(self))]`. **Mandatory credential/secret scrubbing in logs.**
- **Testing & Verification:**
  - **Mandatory Fast Test Runner:** WAJIB pakai `cargo-nextest` (`cargo nextest run -p <crate>`, `cargo nextest run --workspace`).
  - Targeted checking: `cargo check -p <crate>` and `cargo clippy -p <crate> -- -D warnings`.
  - Windows target contention: `$env:CARGO_TARGET_DIR="target/agent"; cargo check -p <crate>`.

---

## 4. Larangan Go Engine (`standard-docs/tech-standart/architecture/01-boundary-policy.md`)
Penulisan kode Go / engine Go dilarang di core repository ini; arsitektur backend dikonsolidasikan penuh ke Rust sesuai [ADR-0018](../standard-docs/decisions/0018-rust-single-language-architecture.md) dan [`01-boundary-policy.md` v2](../standard-docs/tech-standart/architecture/01-boundary-policy.md).

---

## 5. Desktop Shell Standards (`standard-docs/tech-standart/tech-stack/tauri-conventions.md`)
- **Tauri Version:** Tauri v2.
- **IPC Commands:** All commands are `async fn`, return `Result<T, String>`, and communicate with Rust Gateway HTTP (`http://127.0.0.1:42617`).
- **Security:** Deny-by-default allowlist in `tauri.conf.json`. Scoped filesystem to `$APPDATA/galleon/*`.
- **Platform Separation:** Use `#[cfg(target_os = "...")]` for OS-specific shell commands or file operations.

---

## 6. Web & UI Standards (`standard-docs/tech-standart/tech-stack/web-conventions.md`)
- **Stack:** React 19, TypeScript 5.7+ (Strict mode: no `any`), Vite 6, Tailwind CSS 4.
- **API Client:** Use typed `apiClient.ts` pointing to Rust Gateway (`http://localhost:42617` / Vite proxy `/api`).
- **Zero Backend Logic:** No express API business logic in `web/`. View rendering and view-models only.
- **Styling & Components:** Tailwind utilities (no inline `style` objects). Custom theme values from design system.
