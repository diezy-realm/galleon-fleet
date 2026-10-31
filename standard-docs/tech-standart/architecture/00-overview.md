# Architecture Overview

**Status:** Living document  
**Last updated:** 2026-10-31  
**Replaces:** `.gemini/architecture.md` (merged here for single SSOT)  
**Normative Boundary Policy:** [`01-boundary-policy.md`](01-boundary-policy.md) v2 (ODR-2026-10-29 / ADR-0018)

---

## System Identity

**Galleon** is a 3-tier AI agent platform with strict boundary enforcement (Rust Single Language, per ODR-2026-10-29 / ADR-0018):

| Tier | Runtime | Port/Interface | Responsibility |
|------|---------|----------------|----------------|
| **Tier 1: Desktop Shell** | Tauri v2 (Rust, `apps/tauri/`) | IPC | OS integration (window, tray, dialog, packaging), daemon lifecycle |
| **Tier 2: System Core** | Rust 2024 (`crates/*`, `src/`) | HTTP/WS :42617 (`galleon-gateway`) | Single backend: gateway, agent loop, orchestration, LLM (ADR-0001), tools, vault, fleet domain, identity/pairing |
| **Tier 3: UI Layer** | React 19 + TS (`web/`) | HTTP/WS client | Rendering, view-model, client-side routing, **zero backend logic** |

---

> [!NOTE]
> **Catatan Alias Kanonikal (ODR-01) & Arsitektur 3-Tier (ODR-2026-10-29):** Antarmuka web aktif berada di **`web/`** dan desktop di **`apps/tauri/`** (`web-2/` dan `apps/tauri-2/` adalah alias historis). Seluruh backend digabungkan ke Tier 2 Rust (`galleon-gateway` default port 42617); tier Go engine telah dihapus dari repositori ini (add-on non-core dikembangkan di repositori terpisah, ODR-30). Selengkapnya: [`01-boundary-policy.md`](01-boundary-policy.md).

## Tier Boundaries (STRICT)

### 1. Desktop Shell (`apps/tauri/`, Rust)
- **Owned:** Window lifecycle, system tray, OS dialogs, native notifications, `.exe`/packaging, start/stop sidecar `galleon daemon`
- **Forbidden:** Business logic, API handler logic, persistence outside Tauri directory, crew state, secret management, direct terminal execution
- **Protocol:** HTTP ke Rust gateway lokal `http://127.0.0.1:<gateway.port>` (default 42617) dengan bearer token pairing

### 2. System Core (`crates/*`, `src/`, Rust)
- **Owned (Single Backend):**
  - Gateway HTTP/WS (`galleon-gateway`): semua API untuk Web, Desktop, CLI, QR/pairing
  - Agent loop, orkestrasi, SOP, subagent, cron, approval (`galleon-runtime`)
  - LLM execution (ADR-0001), channel plugins, plugin WASI, memory, security microkernel, vault
  - Fleet dashboard domain (`galleon-fleet-core`)
  - Identitas, pairing, WebAuthn, DAC gate
- **Forbidden:**
  - gRPC/protobuf internal antar crate Rust
  - Rute HTTP tanpa auth kecuali daftar rute publik
  - Logika bisnis di handler HTTP
- **Protocol:** Inbound HTTP/WS di gateway (default `127.0.0.1:42617`); komunikasi antar-crate adalah pemanggilan fungsi langsung

### 3. UI Layer (`web/`, TypeScript)
- **Owned:** React components, routing klien, `apiClient.ts`, view model
- **Forbidden:** Logika backend, generate QR, telemetri OS, menyimpan secret, memanggil LLM langsung (ODR-08)
- **Protocol:** HTTP/WS ke Rust gateway (dev: proxy Vite `/api` ke port gateway 42617; prod: disajikan oleh gateway atau reverse-proxy Nginx)

---

## Data Flow (Concrete Example: Crew Turn Execution)

```
1. User clicks "Run crew" in UI
   ↓ HTTP POST /api/fleet/crews/{id}/run (Bearer token)
2. Rust Gateway router (crates/galleon-gateway)
   ↓ verifies bearer auth & delegates to runtime
3. Rust crew orchestrator & agent loop (crates/galleon-runtime)
   ↓ builds turn context (messages, tools, memory)
   ↓ dispatches to LLM provider
4. Rust provider execution (crates/galleon-providers, ADR-0001 SSOT)
   ↓ streams tokens back via WebSocket/SSE
5. Gateway streams completion to UI
   ↓ persists state (galleon-fleet-core / SQLite)
   ↓ HTTP 200 / WebSocket response
6. UI updates chat messages
```

**Key invariant:** UI berkomunikasi hanya dengan Rust Gateway :42617 dengan otentikasi bearer pairing.

---

## Persistence Layer

| Data | Owner | Storage | Access |
|------|-------|---------|--------|
| Fleet config (crews, policies) | Rust Core | `~/.config/galleon/fleet/` (atomic JSON, 0600) | `galleon-fleet-core` |
| Pairing & Device Registry | Rust Core | `~/.config/galleon/devices.db` (SQLite) | `galleon-gateway` |
| Secrets (API keys, tokens) | Rust Core | `~/.config/galleon/vault/` (encrypted) | `galleon-gateway/vault.rs` |
| Tauri window state | Desktop Shell | `~/.config/galleon/window.json` | Tauri `apps/tauri/main.rs` |

---

## Network Ports

| Port | Service | Access |
|------|---------|--------|
| :42617 (default) | Rust HTTP/WS Gateway (`galleon-gateway`) | Frontend-facing (Web, Tauri, CLI) |
| :5173 | Vite dev server (`web/`) | Dev only (prod = static bundle in gateway/Tauri) |

---

## Crate Structure (Rust)

See `docs/book/src/architecture/crates.md` (archived KiroCrew upstream) for detailed dependency graph. Key abstractions:

- `galleon-gateway`: HTTP and WebSocket server, vault, secret management, device pairing
- `galleon-runtime`: Agent turn execution, tool dispatch, message streaming
- `galleon-providers`: LLM vendor APIs (OpenAI, Gemini, Bedrock, Anthropic) - **SSOT (ADR 0001)**
- `galleon-channel-*`: Channel plugins (Discord, Telegram, Slack, etc.)
- `galleon-tools`: Native tool implementations (filesystem, shell, network)
- `galleon-fleet-core`: Fleet domain model and atomic JSON persistence

**Naming:** All crates use `galleon-*` prefix (SPEC-009 / brand consistency).

---

## Module Structure (Go - Diarsipkan)

> [!NOTE]
> Seluruh modul Go (`engine/`) telah diarsipkan ke `standard-docs/archive/go-engine/` dan dihapus dari repositori inti per ODR-2026-10-29 / ADR-0018. Add-on Go dikembangkan di repositori terpisah (ODR-30).

---

## Communication Protocols

### Internal (Antar Crate Rust)

Pemanggilan fungsi langsung / in-process dispatch (tanpa overhead RPC internal).

### Frontend & Klien Eksternal

**HTTP REST & WebSocket:** Rust `galleon-gateway` (:42617 default)
- `/api/fleet/*` - Fleet management (crews, policies, roster)
- `/api/crews/*` - Crew operations (start turn, history, artifacts)
- `/api/system/*` - System metrics, health, telemetry
- `/ws/chat` - Real-time message streaming (LLM tokens, tool events)

---

## Plugin Architecture

**WASI Components** (Rust `galleon-runtime/plugin/`):
- Tools, channels, and memory plugins as `.wasm` modules
- Sandboxed execution (WASI preview2, no host filesystem access by default)
- Discovery: `~/.config/galleon/plugins/` at runtime

See `docs/book/src/plugins/` (archived upstream) for plugin authoring guide.

---

## Security Model

### Secrets (ADR: Rust = SSOT)

- API keys stored in `~/.config/galleon/vault/` (AES-256-GCM encrypted)
- Rust `VaultService` = single accessor
- Secret scrubbing in logs (credential regex filter, exfiltration URL detection)

### Sandboxing

- Tauri IPC: allowlist-only (commands explicitly registered in `tauri.conf.json`)
- WASI plugins: no network, no filesystem by default (explicit capability grants)
- Gateway HTTP: CORS restricted to `localhost` in dev, pairing bearer required

---

## Testing Strategy

| Layer | Test Type | Location | CI Gate |
|-------|-----------|----------|---------|
| Rust crates | Unit + integration | `crates/*/tests/` | `cargo nextest run --locked` |
| Architecture | Tier & boundary invariants | `tests/architecture/` | `cargo test --test architecture` |
| Tauri commands | E2E (WebDriver) | `apps/tauri/tests/` | `cargo test --features e2e` |
| UI components | Vitest + React Testing Library | `web/src/**/*.test.tsx` | `npm test` |

**Full CI:** `./dev/ci.sh all` atau `gates.sh` lokal.

---

## Deployment Targets

- **Desktop:** `.exe` (Windows), `.dmg` (macOS), `.AppImage` (Linux) via Tauri bundler
- **Server / Self-Hosted:** Binary tunggal Rust release (`galleon` / `galleon-gateway`), systemd services

---

## Performance Characteristics

| Metric | Target | Measured (2026-10-03) |
|--------|--------|----------------------|
| Cold start (Tauri) | <3s | 2.1s (M3 Mac, 1.8s Windows 11) |
| HTTP latency (Gateway) | <50ms | 18ms p50, 42ms p99 |
| LLM streaming (Rust) | First token <500ms | 320ms (OpenAI gpt-4), 180ms (Gemini) |

**Bottleneck:** LLM API latency (vendor-dependent, not system overhead).

---

## Observability

- **Logs:** Structured JSON (`tracing` in Rust)
- **Metrics:** Prometheus-compatible `/metrics` endpoint di Rust gateway
- **Tracing:** OpenTelemetry spans (optional, `OTEL_EXPORTER_OTLP_ENDPOINT` env var)

---

## Related Documents

- [Boundary Policy (v2)](./01-boundary-policy.md) - Kebijakan boundary 3-tier normatif
- [ADR-0018: Rust Single Language Architecture](../decisions/0018-rust-single-language-architecture.md) - Arsitektur bahasa tunggal Rust
- [ADR 0001: LLM SSOT Rust](../decisions/0001-llm-ssot-rust.md) - Rust mengeksekusi panggilan LLM
- [ADR 0002: web-2 Backend Policy](../decisions/0002-web2-backend-policy.md) - Zero logic di frontend
- [ADR 0003: Tauri Metrics Real API](../decisions/0003-tauri-metrics-real-api.md) - Sumber telemetri real API
- [Tech Stack: Rust Conventions](../tech-stack/rust-conventions.md) - Cargo workspace, trait boundaries
- [API Contracts: Consumer Inventory](../api-contracts/consumer-inventory.md) - SSOT inventaris rute API
- [Arsip: Go Conventions](../../archive/go-engine/go-conventions.md) - Dokumen historis
- [Arsip: gRPC System Gateway](../../archive/go-engine/grpc-system-gateway.md) - Dokumen historis
- [Arsip: HTTP Engine API](../api-contracts/http-engine-api.md) - Dokumen historis berbanner
