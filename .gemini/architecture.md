# Galleon Fleet AI: Architecture & Layering Specification

> **Status:** Canonical Architecture Reference  
> **Source of Truth:** [`standard-docs/tech-standart/architecture/00-overview.md`](../standard-docs/tech-standart/architecture/00-overview.md), [`01-boundary-policy.md` v2](../standard-docs/tech-standart/architecture/01-boundary-policy.md), & [`standard-docs/decisions/0018-rust-single-language-architecture.md`](../standard-docs/decisions/0018-rust-single-language-architecture.md) (ADR-0018)  
> **Replaces:** Legacy ClawCrew 3-tier model and temporary 4-tier model with the official Galleon 3-Tier Model (Rust Single-Language Architecture).

---

## 1. High-Level 3-Tier Architecture

Galleon Fleet is strictly structured into three distinct operational tiers:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        TIER 3: UI LAYER (`web/`)                            │
│                 Web App & Desktop Frontend (React 19)                       │
│                                                                             │
│   • React 19 + TypeScript 5.7+ + Tailwind CSS 4                             │
│   • State Management: Zustand (`useFleetStore`)                             │
│   • Client Bridge: `apiClient.ts` (HTTP / WS proxy ke Gateway :42617)       │
│   • STRICT RULE: Pure presentation. Zero backend logic. Zero direct LLM SDK.│
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ HTTP / WebSocket :42617
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    TIER 2: SYSTEM CORE & GATEWAY (`crates/`, `src/`)        │
│                        Rust 2024 Edition                                    │
│                                                                             │
│   • Gateway: HTTP/WS :42617 (`galleon-gateway`)                             │
│   • Security Microkernel: OS Sandboxing (Landlock / Seatbelt / AppContainer)│
│   • LLM Execution SSOT (OpenAI, Gemini, Bedrock, Anthropic via providers)   │
│   • Native Tools Execution (Shell, File I/O, Git, Sysinfo)                  │
│   • Secret Vault (AES-256-GCM encrypted in ~/.config/galleon/vault/)        │
│   • Agent Runtime & Fleet Core (`galleon-runtime`, `galleon-fleet-core`)     │
│   • WASI Plugin Host (Wasmtime, signed components)                          │
│   • Channel Adapters (Discord, Slack, WhatsApp, Telegram, Matrix, etc.)     │
│   • STRICT RULE: No direct frontend bypass. Zero unwrap() in production.    │
└──────────────────────────────────────▲──────────────────────────────────────┘
                                       │ Tauri IPC (`invoke`) / HTTP Client
┌──────────────────────────────────────┴──────────────────────────────────────┐
│                 TIER 1: CLIENT / DESKTOP APPS (`apps/tauri/`, CLI)          │
│                        Tauri v2 (`galleon-desktop`) & CLI (`galleon`)       │
│                                                                             │
│   • Window Lifecycle, System Tray, OS Dialogs, Native Notifications         │
│   • Desktop App Packaging (`.exe`, `.dmg`, `.AppImage`)                     │
│   • Thin Client/Proxy ke Gateway :42617. Zero business logic. Zero fake.    │
│   • Sandboxed terminal via PTY & native tools in System Core.               │
└─────────────────────────────────────────────────────────────────────────────┘
```

> **Catatan:** Penulisan kode Go / engine Go dilarang di core repository ini; arsitektur backend dikonsolidasikan penuh ke Rust sesuai ADR-0018 dan 01-boundary-policy v2.

---

## 2. In-Depth Comparison: Tiers & Boundaries

| Dimensi | Tier 1: Client/Desktop Apps (`apps/tauri/`, CLI) | Tier 2: System Core & Gateway (`crates/`, `src/`) | Tier 3: UI Layer (`web/`) |
|---|---|---|---|
| **Peran Utama** | Desktop Window Shell, OS Wrapper & CLI Client | Gateway, Security Microkernel & Agent Core Execution | Presentation & User Interaction |
| **Bahasa & Runtime** | Rust (Tauri v2 / CLI) | Rust 2024 (Tokio async) | TypeScript, React 19, Tailwind 4 |
| **Lokasi Kode** | `apps/tauri/`, `src/` | `crates/*` | `web/src/` |
| **Tugas Riil** | 1. Membungkus UI `web/` jadi native desktop window.<br>2. Tray icon & system menu.<br>3. Dialog OS native.<br>4. Forwarding IPC / HTTP ke Gateway :42617. | 1. Gateway HTTP/WS :42617 (`galleon-gateway`).<br>2. Sandboxing OS (Landlock LSM).<br>3. Eksekusi native tools (fs, shell, git).<br>4. **LLM Provider Execution SSOT** (ADR 0001).<br>5. AES-256-GCM Vault.<br>6. Agent runtime, WASI host & channels. | 1. Render UI components.<br>2. Client-side routing.<br>3. State management via Zustand (`useFleetStore`).<br>4. HTTP / WS client via `apiClient.ts`. |
| **Inbound Interface** | Tauri IPC `invoke('cmd')`, CLI flags | HTTP / WS `:42617` | Browser DOM & User Events |
| **Outbound Interface** | HTTP / WS ke Gateway :42617 | Return HTTP response / WS event stream ke client | HTTP request / WS connection ke Gateway :42617 |
| **DILARANG Keras** | Business logic, direct terminal exec tanpa sandbox, fake metrics (ADR 0003). | Bypass security boundaries, unwrap di jalur produksi. | Backend server logic (Express `server.ts`), direct LLM SDK (`@google/genai`). |

---

## 3. Communication & Data Flow

### A. Turn Execution Flow
```
1. User mengirim prompt di UI (`web/`)
   ↓ HTTP/WS POST /api/v1/agent/run
2. Gateway Handler (`crates/galleon-gateway/`)
   ↓ Memvalidasi auth & request context
3. Runtime Engine (`crates/galleon-runtime/`)
   ↓ Menyusun turn context (history pesan, tools allowlist, memory)
   ↓ Memanggil provider execution (ADR 0001)
4. Turn Executor (`crates/galleon-providers/`)
   ↓ Memanggil vendor LLM (OpenAI / Gemini / Bedrock / Anthropic)
   ↓ Streaming tokens kembali ke runtime
5. Runtime & Sandbox
   ↓ Memvalidasi tool calls terhadap policy & approval gate
   ↓ Jika tool aman/disetujui → panggil ExecuteNativeTool (sandboxed Landlock)
   ↓ Menyimpan hasil ke SQLite & memori
   ↓ Meneruskan event stream via WebSocket / SSE ke UI
6. UI memperbarui tampilan chat secara real-time
```

### B. Terminal Execution Flow (Anti-Injection & Sandboxed)
```
UI / Tauri ──► HTTP/WS /api/terminal (Gateway :42617)
                    │
                    ▼
           Policy & Approval Gate (`galleon-runtime`)
           [Cek Risk Tier & Izin User / Approvals]
                    │
                    ▼ (Jika Disetujui)
           ExecuteNativeTool / PTY Session
                    │
                    ▼
           Rust OS Sandbox (Landlock / Seatbelt)
                    │
                    ▼ (Kembalikan Output)
           Audit Log ──► UI Response
```

---

## 4. Coding & Quality Standards

- **Rust Standards:**
  - Wajib zero `unwrap()` dan `expect()` di jalur produksi.
  - Poison recovery pada Mutex: `poisoned.into_inner()` + log warning.
  - Standar ukuran file $\le 800$ baris, fungsi $\le 60$ baris.
  - Gunakan `cargo nextest run` untuk testing.
  - Prefix crate baru: `galleon-*`.
- **Larangan Go Engine:**
  - Penulisan kode Go / engine Go dilarang di core repository ini; arsitektur backend dikonsolidasikan penuh ke Rust sesuai ADR-0018 dan 01-boundary-policy v2.
- **UI / Frontend Standards:**
  - TypeScript strict mode, zero `any`.
  - Zero backend logic di `web/` (hapus Express `server.ts`).
  - Hapus `@google/genai` dan artefak AI Studio dari `web/`.
