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

## 2. 4-Tier Layering & Strict Boundary Policy (`standard-docs/tech-standart/architecture/01-boundary-policy.md`)

All changes MUST preserve the separation of concerns:

| Tier | Runtime & Port | Tanggung Jawab Resmi (Owns) | Hal Dilarang Keras (Forbidden) |
|---|---|---|---|
| **Tier 1: Desktop Shell** (`apps/tauri-2/`) | Tauri v2 (Rust IPC) | OS window lifecycle, system tray, native file dialogs, desktop bundling (`.exe`, `.dmg`). | **NO business logic**, NO API handlers, NO direct persistence, NO fake metrics (ADR 0003). |
| **Tier 2: System Core** (`crates/`) | Rust 2024 (gRPC :50052 `SystemGateway`) | Security microkernel (sandbox, vault AES-256-GCM), native tools (fs/shell), **LLM execution SSOT (ADR 0001)**, WASI plugin host, channels. | **NO fleet governance**, NO crew turn orchestration, NO RAG memory. Rust is a **leaf service** (does not call Go/Tauri). |
| **Tier 3: AI Orchestrator** (`engine/`) | Go 1.27+ (gRPC :50051 & HTTP :9090) | Agent brain (`StartTurn`), fleet governance, policies (risk-tiers, approvals), vector memory RAG, SQLite/Postgres persistence. | **NO direct LLM vendor API calls** (delegated to Rust ADR 0001), NO direct shell/fs tool exec (delegated to Rust). |
| **Tier 4: UI Layer** (`web-2/`) | React 19 + TS 5.7+ (Vite + Tailwind 4) | View rendering, Tailwind components, view-models, client-bridge (`apiClient.ts`). | **Zero backend logic (ADR 0002)**, NO QR code/OS telemetry generation in UI server, **NEVER bypass Go to call Rust gRPC directly**. |

### Data Flow Invariants:
1. **Flow Hierarchy:** `Frontend (web-2 / Tauri) → Go HTTP (:9090) → Rust gRPC (:50052)`. Direct Frontend $\rightarrow$ Rust is forbidden.
2. **LLM SSOT (ADR 0001):** All LLM vendor calls (OpenAI, Gemini, Bedrock, Anthropic) live in Rust `clawcrew-providers`. Go purely orchestrates turn context and streams via gRPC.
3. **Secrets SSOT:** Stored in Rust Vault (`~/.config/galleon/vault/` AES-256-GCM). Retrieved via gRPC `GetSecret`/`SetSecret`.
4. **Policy SSOT:** Rules for auto-approval and risk-tiers exist ONLY in Go (`fleet/policies.go`). Tauri and UI must query Go, never hardcode policies.
5. **No Shared Database:** Each tier strictly isolates storage (Tauri: `window.json`, Rust: `vault/`, Go: `engine/data/*.db`).

---

## 3. Rust Standards (`standard-docs/tech-standart/tech-stack/rust-conventions.md`)
- **Edition & MSRV:** Rust 2024, MSRV 1.82+.
- **Branding:** New crates MUST use prefix `galleon-` (e.g. `galleon-channel-matrix`). Legacy crates keep `clawcrew-*`.
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

## 4. Go Engine Standards (`standard-docs/tech-standart/tech-stack/go-conventions.md`)
- **Version:** Go 1.27+.
- **Package Layout:** Clean architecture (`handlers.go` $\rightarrow$ `services.go` $\rightarrow$ `store.go`). Never circular import.
- **Dependency Injection:** Compile-time DI using Google Wire (`wire.go` $\rightarrow$ `wire_gen.go`).
- **gRPC Integration:** Go acts as gRPC client to Rust `SystemGatewayClient` (:50052) for LLM (`ExecuteTurn`) and Tools (`ExecuteTool`).
- **HTTP Routing:** Gorilla Mux on `:9090`. All routes accept `context.Context` and propagate downstream.
- **Error Handling & Logging:** Wrap errors with `fmt.Errorf("...: %w", err)`. Structured logging with `go.uber.org/zap`.

---

## 5. Desktop Shell Standards (`standard-docs/tech-standart/tech-stack/tauri-conventions.md`)
- **Tauri Version:** Tauri v2.
- **IPC Commands:** All commands are `async fn`, return `Result<T, String>`, and proxy to Go HTTP `:9090`.
- **Security:** Deny-by-default allowlist in `tauri.conf.json`. Scoped filesystem to `$APPDATA/galleon/*`.
- **Platform Separation:** Use `#[cfg(target_os = "...")]` for OS-specific shell commands or file operations.

---

## 6. Web & UI Standards (`standard-docs/tech-standart/tech-stack/web-conventions.md`)
- **Stack:** React 19, TypeScript 5.7+ (Strict mode: no `any`), Vite 6, Tailwind CSS 4.
- **API Client:** Use typed `apiClient.ts` pointing to `http://localhost:9090`.
- **Zero Backend Logic:** No express API business logic in `web-2/`. View rendering and view-models only.
- **Styling & Components:** Tailwind utilities (no inline `style` objects). Custom theme values from design system.
