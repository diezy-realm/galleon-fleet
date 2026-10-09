# Galleon Fleet AI: Architecture & Layering Specification

> **Status:** Canonical Architecture Reference  
> **Source of Truth:** [`standard-docs/tech-standart/architecture/00-overview.md`](../standard-docs/tech-standart/architecture/00-overview.md) & [`01-boundary-policy.md`](../standard-docs/tech-standart/architecture/01-boundary-policy.md)  
> **Replaces:** Legacy ClawCrew 3-tier model with the official Galleon 4-Tier Model.

---

## 1. High-Level 4-Tier Architecture

Galleon Fleet is strictly structured into four distinct operational tiers:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TIER 4: UI LAYER (`web/`)                         │
│                    Web App & Desktop Frontend (React 19)                    │
│                                                                             │
│   • React 19 + TypeScript + Tailwind CSS 4                                  │
│   • State Management: Zustand (`useFleetStore`)                             │
│   • Client Bridge: `apiClient.ts` (HTTP / SSE proxy ke Go :9090)            │
│   • STRICT RULE: Pure presentation. Zero backend logic. Zero direct LLM SDK.│
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ HTTP :9090 (REST / SSE)
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      TIER 3: AI ORCHESTRATOR (`engine/`)                    │
│                            Go 1.27 (Google Wire DI)                         │
│                                                                             │
│   • High-level Agent Engine & Single Brain (`StartTurn`)                    │
│   • Fleet Governance, Missions, Ships, Squads, Crew Roster                  │
│   • Policies & Approval Gate SSOT (Risk-Tiers, Audit Log)                   │
│   • Memory Indexing, RAG, & Persistent Storage (SQLite / Postgres)          │
│   • Multi-surface Gateway: WebAuthn, QR Device Pairing                      │
│   • Services: gRPC (:50051 Internal), HTTP (:9090 Frontend-facing)          │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ gRPC :50052 (`SystemGatewayService`)
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       TIER 2: SYSTEM CORE (`crates/`)                       │
│                        Rust 2024 Edition (27 Crates)                        │
│                                                                             │
│   • Security Microkernel: OS Sandboxing (Landlock / Seatbelt / AppContainer)│
│   • LLM Execution SSOT (OpenAI, Gemini, Bedrock, Anthropic via providers)   │
│   • Native Tools Execution (Shell, File I/O, Git, Sysinfo)                  │
│   • Secret Vault (AES-256-GCM encrypted in ~/.config/galleon/vault/)        │
│   • WASI Plugin Host (Wasmtime, signed components)                          │
│   • Channel Adapters (Tier A: Discord, Slack, WhatsApp, Telegram, Matrix)   │
│   • STRICT RULE: Leaf service. Zero crew orchestration. Zero unwrap().      │
└─────────────────────────────────────────────────────────────────────────────┘
                                       ▲
                                       │ Tauri IPC (`invoke`)
┌──────────────────────────────────────┴──────────────────────────────────────┐
│                    TIER 1: DESKTOP SHELL (`apps/tauri/`)                    │
│                        Tauri v2 (`galleon-desktop`)                         │
│                                                                             │
│   • Window Lifecycle, System Tray, OS Dialogs, Native Notifications         │
│   • Desktop App Packaging (`.exe`, `.dmg`, `.AppImage`)                     │
│   • Thin Proxy to Go :9090. Zero business logic. Zero fake metrics.         │
│   • FORBIDDEN: Direct terminal execution (wajib lewat Go policy + Rust).    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. In-Depth Comparison: Tiers & Boundaries

| Dimensi | Tier 1: Desktop Shell (`apps/tauri/`) | Tier 2: System Core (`crates/`) | Tier 3: AI Orchestrator (`engine/`) | Tier 4: UI Layer (`web/`) |
|---|---|---|---|---|
| **Peran Utama** | Desktop Window Shell & OS Wrapper | Security Microkernel & Execution Leaf | AI Brain & Workflow Orchestrator | Presentation & User Interaction |
| **Bahasa & Runtime** | Rust (Tauri v2) | Rust 2024 (Tokio async) | Go 1.27 (Google Wire DI) | TypeScript, React 19, Tailwind 4 |
| **Lokasi Kode** | `apps/tauri/` | `crates/*` | `engine/src/`, `engine/cmd/` | `web/src/` |
| **Tugas Riil** | 1. Membungkus UI `web/` jadi native desktop window.<br>2. Tray icon & system menu.<br>3. Dialog OS native.<br>4. Forwarding IPC ke Go engine :9090. | 1. Sandboxing OS (Landlock LSM).<br>2. Eksekusi native tools (fs, shell, git).<br>3. **LLM Provider Execution SSOT** (ADR 0001).<br>4. AES-256-GCM Vault.<br>5. WASI plugin runtime & channel adapters. | 1. Keputusan AI & Turn state machine (`StartTurn`).<br>2. Fleet governance: Ships, Squads, Crews.<br>3. Policy & Approval gates.<br>4. Vector memory & persistent storage (SQLite/Postgres).<br>5. WebAuthn & QR device pairing. | 1. Render UI components.<br>2. Client-side routing.<br>3. State management via Zustand (`useFleetStore`).<br>4. HTTP / SSE client via `apiClient.ts`. |
| **Inbound Interface** | Tauri IPC `invoke('cmd')` | gRPC `:50052` (`SystemGatewayService`) | HTTP `:9090`, gRPC `:50051` | Browser DOM & User Events |
| **Outbound Interface** | HTTP ke Go :9090 | Return gRPC stream/response ke Go | gRPC client ke Rust :50052, SSE stream ke UI | HTTP request ke Go :9090 |
| **DILARANG Keras** | Business logic, direct terminal exec (ODR-07), fake metrics (ADR 0003). | Crew orchestration, RAG memory, fleet governance. | Shell/fs exec lokal tanpa Rust (wajib fail-closed), direct vendor LLM calls. | Backend server logic (Express `server.ts`), direct LLM SDK (`@google/genai`). |

---

## 3. Communication & Data Flow

### A. Crew Turn Execution Flow
```
1. User mengirim prompt di UI (`web/`)
   ↓ HTTP POST /api/fleet/crews/{id}/run
2. Go HTTP Handler (`engine/src/fleet/`)
   ↓ memanggil crew.Services.StartTurn()
3. Go Orchestrator (`engine/src/crew/services.go`)
   ↓ Menyusun turn context (history pesan, tools allowlist, memory)
   ↓ Membuka gRPC stream ke Rust SystemGateway.ExecuteTurn (ADR 0001)
4. Rust Turn Executor (`crates/clawcrew-providers/`)
   ↓ Memanggil vendor LLM (OpenAI / Gemini / Bedrock / Anthropic)
   ↓ Streaming tokens kembali ke Go melalui gRPC
5. Go Engine
   ↓ Memvalidasi tool calls terhadap policy & approval gate
   ↓ Jika tool aman/disetujui → panggil Rust ExecuteNativeTool (sandboxed)
   ↓ Menyimpan hasil ke SQLite/Postgres & memori RAG
   ↓ Meneruskan event stream via SSE ke UI
6. UI memperbarui tampilan chat secara real-time
```

### B. Terminal Execution Flow (ODR-07 Anti-Injection)
```
UI / Tauri ──► HTTP POST /api/engine/execute (Go :9090)
                    │
                    ▼
           Go Policy & Approval Gate (fleet/policies.go)
           [Cek Risk Tier & Izin User / Approvals]
                    │
                    ▼ (Jika Disetujui)
           gRPC ExecuteNativeTool ke Rust Gateway (:50052)
                    │
                    ▼
           Rust OS Sandbox (Landlock / Seatbelt)
                    │
                    ▼ (Kembalikan Output)
           Rust ──► Go Audit Log ──► UI Response
```

---

## 4. Coding & Quality Standards

- **Rust Standards:**
  - Wajib zero `unwrap()` dan `expect()` di jalur produksi.
  - Poison recovery pada Mutex: `poisoned.into_inner()` + log warning.
  - Standar ukuran file $\le 800$ baris, fungsi $\le 60$ baris.
  - Gunakan `cargo nextest run` untuk testing.
  - Prefix crate baru: `galleon-*`.
- **Go Standards:**
  - Clean architecture: `engine/src/<domain>/` (`services.go`, `store.go`, `handlers.go`).
  - Google Wire DI di `engine/app/wire.go`.
  - Fail-closed: Jika Rust gateway offline, tolak tool execution dengan error; dilarang fallback lokal.
  - Structured logging dengan zap/slog dan error wrapping `%w`.
- **UI / Frontend Standards:**
  - TypeScript strict mode, zero `any`.
  - Zero backend logic di `web/` (hapus Express `server.ts`).
  - Hapus `@google/genai` dan artefak AI Studio dari `web/`.
