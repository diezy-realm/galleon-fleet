# Steering: Galleon Architecture & Codebase Navigation

This steering document governs how AI agents architect, implement, query, and navigate the Galleon Fleet AI codebase. It applies to all agent sessions (Antigravity, Claude Code, subagents).

## Fundamental Normative Standard: `standard-docs/tech-standart/`

The **prescriptive Single Source of Truth (SSOT)** for architecture, boundaries, language conventions, and contracts is located in [`standard-docs/tech-standart/`](../standard-docs/tech-standart/):
- **Architecture Overview:** [`standard-docs/tech-standart/architecture/00-overview.md`](../standard-docs/tech-standart/architecture/00-overview.md)
- **Strict Boundary Policy:** [`standard-docs/tech-standart/architecture/01-boundary-policy.md`](../standard-docs/tech-standart/architecture/01-boundary-policy.md)
- **Rust Conventions (2024 / MSRV 1.82+):** [`standard-docs/tech-standart/tech-stack/rust-conventions.md`](../standard-docs/tech-standart/tech-stack/rust-conventions.md)
- **Go Conventions (1.25+):** [`standard-docs/tech-standart/tech-stack/go-conventions.md`](../standard-docs/tech-standart/tech-stack/go-conventions.md)
- **Tauri Conventions (v2):** [`standard-docs/tech-standart/tech-stack/tauri-conventions.md`](../standard-docs/tech-standart/tech-stack/tauri-conventions.md)
- **Web Conventions (React 19 / TS 5.7+ / Tailwind 4):** [`standard-docs/tech-standart/tech-stack/web-conventions.md`](../standard-docs/tech-standart/tech-stack/web-conventions.md)
- **gRPC Contract (:50052 Rust):** [`standard-docs/tech-standart/api-contracts/grpc-system-gateway.md`](../standard-docs/tech-standart/api-contracts/grpc-system-gateway.md)
- **HTTP Engine API (:9090 Go):** [`standard-docs/tech-standart/api-contracts/http-engine-api.md`](../standard-docs/tech-standart/api-contracts/http-engine-api.md)

Related rules & skills:
- Rust Standards: [`.gemini/rules/rust-standards.md`](rules/rust-standards.md)
- Graph-First Codebase Navigation: [`.gemini/skills/graphify/SKILL.md`](skills/graphify/SKILL.md)

---

## 1. 4-Tier Layering Governance & Strict Boundary Policy

All implementations and refactoring MUST adhere to the following separation of concerns:

| Tier | Runtime | Port / Interface | Tanggung Jawab Resmi | Hal Dilarang (Forbidden) |
|---|---|---|---|---|
| **Desktop Shell** (`apps/tauri-2/`) | Tauri v2 (Rust) | IPC (`invoke`) | Window lifecycle, system tray, OS dialogs, packaging desktop `.exe`. | Dilarang memuat business logic, API logic, persistence, atau fake data (ADR 0003). |
| **System Core** (`crates/`) | Rust (2024 edition, 25 crates) | gRPC Server (:50052) `SystemGateway` | Security microkernel (sandbox, vault AES-256-GCM), native tools (fs/shell), LLM execution SSOT (ADR 0001: OpenAI, Gemini, Bedrock, Anthropic), WASI runtime, channels, mDNS. | Dilarang mengelola fleet governance, crew turn orchestration, atau RAG memory (milik Go). Rust = leaf service. |
| **AI Orchestrator** (`engine/`) | Go 1.25+ | gRPC (:50051) & HTTP (:9090) | Agent brain (`StartTurn`), fleet governance (crews, policies, risk tiers), vector memory RAG, SQLite/Postgres persistence. | Dilarang direct LLM API calling atau native tool exec langsung (wajib delegasi ke Rust gRPC :50052). |
| **UI Layer** (`web-2/`) | React 19 + TS + Tailwind 4 | HTTP Proxy ke Go (:9090) | Visual rendering, view-models (`useFleetStore`), client-bridge (`apiClient.ts`). | Zero backend logic (ADR 0002). Dilarang membypass Go untuk memanggil Rust gRPC :50052 langsung. |

### Core Invariants & SSOT Directives:
1. **Frontend → Go → Rust (Never Frontend → Rust):** Go engine adalah gateway penegak kebijakan (*governance*). Frontend dilarang memanggil Rust gRPC secara langsung.
2. **LLM SSOT = Rust `clawcrew-providers` (ADR 0001):** Semua vendor LLM dijalankan di Rust. Go murni bertindak sebagai orkestrator konteks turn.
3. **Secrets SSOT = Rust Vault:** Disimpan di `~/.config/galleon/vault/`. Diakses via gRPC `GetSecret`/`SetSecret`. Wajib credential scrubbing di log.
4. **Policy SSOT = Go Engine:** Aturan approval dan kalkulasi risk-tier hanya ada di Go (`fleet/policies.go`). Tauri/UI dilarang hardcoding policy.
5. **No Shared Database:** Tiap tier mengisolasi storage miliknya sendiri (Tauri: `window.json`, Rust: `vault/`, Go: `engine/data/*.db`).
6. **Rust Code Standards:** Crate baru memakai prefix `galleon-*`, shared dependencies di root `[workspace.dependencies]`, trait-driven (`Provider`, `ToolExecutor`), error handling dengan `thiserror` dan propagasi `?` (**strictly no `unwrap()` / `expect()` in production paths**).
7. **Mandatory Rust Test Runner (`cargo-nextest`):** Seluruh eksekusi test Rust WAJIB menggunakan `cargo-nextest` (`cargo nextest run -p <crate>`, `cargo nextest run --workspace`) untuk isolasi proses dan eksekusi paralel cepat (ref: https://nexte.st/docs/installation/from-source/).

---

## 2. Graph-First Codebase Navigation Principle

**Graph first, grep second, manual browsing never.**

All codebase queries — symbol lookup, path discovery, dependency tracing, module understanding, call-flow tracing, and architecture questions — MUST follow this resolution order:

1. **Graphify indexing** → Refresh or build the knowledge graph first.
2. **Graphify query** → Query `graphify-out/` with `query`, `path`, or `explain`.
3. **Grep** → Use targeted `grep`/`Select-String` only for line-level detail after Graphify has identified the target file(s) and symbol(s).
4. **Direct file read** → Open specific files only after the Graphify query and targeted search have identified them.

For every query/path/flow investigation, run `graphify update .` when
`graphify-out/graph.json` exists. If it does not exist, run `graphify .` first.
This indexing step is mandatory even when the requested lookup appears simple.
VS Code Copilot follows the project-scoped rules in `.github/copilot-instructions.md`.

> **NEVER** start with broad recursive grep, `find`, or directory walks to understand architecture, locate symbols, or discover relationships. Use the graph.

## Graphify Output Structure

```
graphify-out/
├── graph.html        # Interactive graph visualization
├── GRAPH_REPORT.md   # God nodes, surprising connections, suggested questions
├── graph.json        # Persistent knowledge graph (query without re-reading)
├── obsidian/         # Open as Obsidian vault for visual exploration
├── wiki/             # Wikipedia-style articles for agent navigation
└── cache/            # SHA256 cache — re-runs only process changed files
    ├── ast/          # Per-file AST analysis
    ├── semantic/     # Semantic relationship extraction
    └── stat-index.json  # File metadata and hash index
```

## Agent Query Protocol

### Step 1: Check graph availability

Before any codebase query, verify that `graphify-out/` exists:

```powershell
# Windows (pwsh)
Test-Path graphify-out/graph.json

# If missing, generate the graph first:
graphify .
# Or if graphify CLI is unavailable:
python -m graphify .
```

If `graphify-out/` does not exist or `graph.json` is missing, **generate the graph before proceeding**. Do not skip this step.

### Step 2: Query the graph

Use graphify's query capabilities to answer questions:

```powershell
# Semantic query — find concepts and relationships
graphify query "what modules handle task cancellation?"

# Path query — find connections between concepts
graphify path "TaskRecord" "SessionLifecycle"

# For structural queries, read graph.json or the wiki/
```

For agents without CLI access to `graphify`, read these files in order:

1. `graphify-out/GRAPH_REPORT.md` — Architecture overview, god nodes, key connections.
2. `graphify-out/wiki/` — Per-module and per-concept articles.
3. `graphify-out/graph.json` — Raw graph data for programmatic queries.
4. `graphify-out/cache/stat-index.json` — File index with sizes, hashes, and word counts.

### Step 3: Targeted grep for detail

After graphify identifies the relevant file(s), use `Select-String` for line-level precision:

```powershell
# GOOD: graphify told us TaskRecord is in task_registry.rs
Select-String -Pattern "pub struct TaskRecord" crates/clawcrew-runtime/src/control_plane/task_registry.rs

# BAD: blind recursive search without checking the graph
Get-ChildItem -Recurse -Filter "*.rs" | Select-String "TaskRecord"
```

### Step 4: Read file content

Open only the specific files and line ranges that step 2 and 3 identified.

## When to Re-index

Run `graphify . --update` (incremental) when:

- New modules or crates are added.
- Major refactors change module boundaries.
- Before starting a new architectural task (P0, P1, P2, etc.).

Run `graphify .` (full re-index) when:

- The graph is more than a week old.
- The codebase has undergone a large merge or rebase.

## Anti-Patterns (Prohibited)

| ❌ Don't | ✅ Do instead |
|---|---|
| `Get-ChildItem -Recurse \| Select-String "pattern"` as first step | `graphify query "what handles pattern?"` |
| Browsing random directories to "understand the codebase" | Read `GRAPH_REPORT.md` for architecture overview |
| Opening 10+ files to find a symbol definition | Query the graph, then open the one identified file |
| Guessing file paths from memory | Check `stat-index.json` or the graph for canonical paths |
| Skipping graph when it exists | Always check graph first, even for "simple" lookups |

## Subagent Directive

All subagents (research, implementation, review) inherit this steering. When spawning subagents:

- Include "Use graphify-out/ for codebase navigation per steering.md" in the subagent prompt.
- Subagents MUST set their working directory to the repository root before any graphify or file operation.

## Compatibility

This steering is designed to work with:

- **Antigravity** (this platform) — via `.gemini/steering.md`
- **Claude Code** — via `.claude/CLAUDE.md` reference
- **Other AI agents** — via `AGENTS.md` reference

The canonical source of truth for this policy is this file: `.gemini/steering.md`.

