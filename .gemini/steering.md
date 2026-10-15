# Steering: Galleon Fleet AI Workspace & Codebase Navigation

> **Status:** Canonical Workspace Steering (Normative)  
> **Repository:** `diezy-realm/galleon-fleet`  
> **Brand Identity:** Galleon Fleet (`galleon-fleet`) | CLI: `galleon`  
> **Highest Authority:** [`standard-docs/`](../standard-docs/) (ODR-01 s/d ODR-25, Tech Standards, PR Plan)  
> **Applies to:** Semua AI agent (Antigravity, Claude Code, subagents, developer tools).

---

## 1. Otoritas Tertinggi & Hierarki Keputusan

Jika terdapat kontradiksi antara dokumen mana pun di repo ini, urutan otoritas penentu kebenaran (Single Source of Truth) adalah:
1. **Owner Decision Records (ODR):** [`standard-docs/decisions/owner-decisions/`](../standard-docs/decisions/owner-decisions/) (ODR-01 s/d ODR-25). Bersifat mutlak & *immutable*.
2. **Definisi Produk Resmi:** [`standard-docs/product/galleon-product-definition.md`](../standard-docs/product/galleon-product-definition.md).
3. **Standar Teknis & Batasan Arsitektur:** [`standard-docs/tech-standart/`](../standard-docs/tech-standart/).
4. **Roadmap & Master PR Plan:** [`standard-docs/roadmap/PR-plan.md`](../standard-docs/roadmap/PR-plan.md).
5. **Steering Dokumen ini (`.gemini/steering.md`):** Panduan operasional harian agent.

---

## 2. Arsitektur Kanonikal 4-Tier & Strict Boundary Policy

Galleon beroperasi dalam **4 tier ketat** dengan batas tanggung jawab tegas ([`01-boundary-policy.md`](../standard-docs/tech-standart/architecture/01-boundary-policy.md)):

```
┌────────────────────────────────────────────────────────────────────────┐
│                        TIER 4: UI LAYER (`web/`)                       │
│                   React 19 + TypeScript + Tailwind CSS 4               │
│         Zero backend logic. Zero fake data. Zero direct LLM call.      │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ HTTP :9090 (REST/SSE)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                   TIER 3: AI ORCHESTRATOR (`engine/`)                  │
│                     Go 1.27 (Google Wire DI, slog/zap)                 │
│       Otak Tunggal: Turn Lifecycle, DAG, Fleet Governance, Memory      │
└───────────────────┬────────────────────────────────┬───────────────────┘
                    │ gRPC :50052                    ▲ IPC (proxy ke :9090)
                    ▼ (SystemGateway)                │
┌────────────────────────────────────────┐  ┌────────────────────────────┐
│      TIER 2: SYSTEM CORE (`crates/`)   │  │ TIER 1: DESKTOP (`apps/    │
│            Rust 2024 (27 crates)       │  │          tauri/`)          │
│ Sandbox, Vault AES-256, LLM SSOT, WASI │  │ Tauri v2 OS Shell & Window │
└────────────────────────────────────────┘  └────────────────────────────┘
```

### Matriks Tanggung Jawab & Larangan

| Tier | Path Aktif | Port / Protokol | Tanggung Jawab Resmi (Owned) | Hal Terlarang (FORBIDDEN) |
|---|---|---|---|---|
| **Tier 1: Desktop Shell** | `apps/tauri/` | Tauri IPC (`invoke`) | Window lifecycle, tray icon, dialog native OS, bundling installer (`.exe`/`.dmg`). | ❌ Business logic, eksekusi terminal langsung (ODR-07), data palsu (ADR 0003). |
| **Tier 2: System Core** | `crates/` | gRPC Server `:50052` (`SystemGateway`) | Security microkernel, sandbox (Landlock/Seatbelt), **LLM Execution SSOT** (`clawcrew-providers`), secrets Vault (AES-256-GCM), native tools, WASI host, channel adapters. | ❌ Fleet governance, turn orchestration, RAG vector memory (milik Go). Rust = leaf service. |
| **Tier 3: AI Orchestrator** | `engine/` | gRPC Server `:50051`, HTTP `:9090` | **Agent Brain**: turn state machine (`StartTurn`), task graph, fleet policies & approval gate, memori persisten (SQLite/Postgres), pairing/QR gateway. | ❌ Eksekusi shell/fs lokal tanpa Rust (wajib fail-closed), direct vendor LLM call (wajib delegasi ke Rust via gRPC). |
| **Tier 4: UI Layer** | `web/` | HTTP proxy ke Go `:9090` | Rendering komponen, view model Zustand, client routing. | ❌ Backend logic di Express (`server.ts` harus dibersihkan, ADR 0002), direct LLM SDK (`@google/genai` dilarang, ODR-08). |

### Invarian Keras (Hard Invariants):
1. **Frontend → Go → Rust (Dilarang Frontend → Rust):** Go engine adalah satu-satunya gerbang kebijakan (*governance & anti-injection*). Frontend dilarang memanggil Rust gRPC `:50052` langsung.
2. **LLM SSOT = Rust `clawcrew-providers` (ADR 0001):** Semua vendor LLM dijalankan di Rust. Go engine menyusun konteks pesan dan mendelegasikan inferensi ke Rust via gRPC `ExecuteTurn`.
3. **Eksekusi Terminal Terproteksi (ODR-07):** Alur terminal wajib: `Tauri/Web` $\rightarrow$ `Go Engine (Policy & Approval)` $\rightarrow$ `Rust SystemGateway (Sandbox)`.
4. **Secrets SSOT = Rust Vault:** API key dan kredensial tersimpan di `~/.config/galleon/vault/` (AES-256-GCM terenkripsi). Wajib credential scrubbing di log.
5. **No Shared Database:** Tiap tier mengisolasi storage-nya sendiri. Komunikasi lintas tier hanya via API.
6. **Migrasi Legacy Path Tuntas (ODR-01, PR-01, CO-08):** Path kanonikal resmi adalah `web/` dan `apps/tauri/`. Direktori legacy `web-2/` dan `apps/tauri-2/` telah selesai dimigrasi dan dilarang dibuat, direferensikan, atau dimasukkan ke git tracking. Setiap residu lokal `web-2` atau `apps/tauri-2` wajib dihapus.

---

## 3. Empat Permukaan Akses & Channel Messaging

- **4 Permukaan Akses Produk (ODR-02):**
  - **CLI:** Binary Rust `galleon` sebagai klien tipis ke Go engine `:9090` (ODR-12).
  - **Desktop:** Tauri v2 di `apps/tauri/` (hanya proxy ke Go + fitur OS).
  - **Web:** Dashboard React 19 di `web/`.
  - **QR Pairing:** Otentikasi dan login perangkat tambahan terikat token via Nginx/Go.
- **Distribusi Channel Messaging (ODR-04):**
  - **Tier A (Embed/Default):** Slack, WhatsApp, Discord, Telegram, Matrix (ikut instalasi default).
  - **Tier B (Compiled Feature-Crates `galleon-channel-<name>`):** WeChat, QQ, Signal, iMessage, AMQP, IRC.
  - **Tier C (WASI Plugin Sandbox):** Channel niche inbound/webhook (Nostr, Twitch, dll.) dengan verifikasi signature.

---

## 4. Standar Kode & Kualitas Produksi

### A. Standar Rust (Rust 2024 / MSRV 1.82+)
- **Zero `unwrap()` / `expect()` di Jalur Produksi:** Wajib menggunakan error propagation `?` dengan `thiserror` atau `Result<T>`. Pengecualian hanya untuk test (`#[cfg(test)] mod tests`, file `tests.rs`, folder `tests/`) atau invarian terdokumentasi `// INVARIANT: ...`. Seluruh test helper mock/fixture yang memakai `unwrap()` dilarang berada di luar scope `#[cfg(test)]`.
- **Mutex Poison Recovery:** Tangani mutex lock dengan `poisoned.into_inner()` + log warning, jangan `.expect("lock poisoned")`.
- **Standar Ukuran File & Fungsi (Ratchet Rule):**
  - File non-test: target $\le 800$ baris (warning $> 2.000$, gagal CI $> 5.000$).
  - Fungsi: target $\le 60$ baris.
  - Crate: target $\le 30\text{k}$ LOC.
- **Mandatory Fast Test Runner (`cargo-nextest`):**
  ```powershell
  # Test spesifik per crate
  cargo nextest run -p <crate-name>
  # Test spesifik fungsi test
  cargo nextest run -p <crate-name> -E 'test(<test_fn>)'
  # Workspace test
  cargo nextest run --workspace
  ```
- **Targeted Checking (Cepat & Tanpa Lock Contention di Windows):**
  ```powershell
  $env:CARGO_TARGET_DIR="target/agent"; cargo check -p <crate-name>
  cargo clippy -p <crate-name> -- -D warnings
  ```

### B. Standar Go (Go 1.27)
- **Dependency Injection:** Google Wire (`engine/app/wire.go` $\rightarrow$ `wire_gen.go`).
- **Fail-Closed Policy:** Delegasi tool ke Rust via `ExecuteNativeTool`. Jika Rust offline, kembalikan error tertolak (fail-closed), dilarang fallback mengeksekusi shell/fs lokal di Go.
- **Logging & Errors:** Structured logging via `zap`/`slog`. Error pembungkus wajib `%w`.
- **Pengujian:** `go test -race ./...` di folder `engine/`.

### C. Standar Web & UI (React 19 + TypeScript + Tailwind 4)
- **Zero Backend Logic:** Pindahkan logika Express di `server.ts` ke Go engine.
- **Zero Client LLM Calls:** Hapus `@google/genai` dari `web/package.json`.
- **Type Safety:** TypeScript strict mode, zero `any`. Gunakan `apiClient.ts` untuk komunikasi dengan Go `:9090`.
- **Peer Dependency & Lockfile Hygiene:** Dilarang menggunakan bypass `--force` atau `--legacy-peer-deps` di CI workflow maupun instalasi lokal. Versi `vite` dan `esbuild` di `web/package.json` wajib selaras, dan `package-lock.json` wajib ter-commit sinkron.

### D. Standar CI/Infra & Quality Gates (Ref: `.gemini/rules/ci-infra-standards.md`)
Berdasarkan investigasi konkrit CI PR #18 (`ci-infra-fix/`), 4 pilar tata kelola CI/Infra berikut mengikat seluruh agent dan PR:
1. **Zero-Unwrap Invariant Scoping:** Pengecekan CI diff wajib mengecualikan file `tests.rs` (`':!crates/**/tests.rs'`, `':!crates/**/*/tests.rs'`) dan blok `#[cfg(test)] mod tests`. Dilarang menandai helper unit test sebagai pelanggaran produksi.
2. **Compiler Toolchain & sccache Isolation:** Karena `rustc-wrapper = "sccache"` aktif di `.cargo/config.toml`, setiap job CI yang memanggil `cargo` wajib menginstal binary `sccache` di runner PATH terlebih dahulu (misal via `mozilla-actions/sccache-action`), atau meng-override dengan `RUSTC_WRAPPER=""`. Dilarang menghapus konfigurasi caching dari `.cargo/config.toml`.
3. **Pemisahan Channel Transport vs Governance (Gate D-2):** Conditional compilation `#[cfg(feature = "channel-*")]` pada fungsi utilitas murni di `crates/clawcrew-channels/src/util.rs` (HTTP limiter, string formatter, breadcrumbs) adalah SAH. **Koreksi 2026-10-09:** `util.rs` genuinely berisi `PendingApproval`/`resolve_pending_approval`/`new_approval_token` (bukan tidak ada sama sekali seperti klaim versi steering sebelumnya) — tapi ini SAH karena murni I/O/transport (generate token, tunggu balasan teks user per-channel), terbukti `clawcrew-channels` tidak depend ke `clawcrew-runtime` tempat logic kebijakan sungguhan (`approval/mod.rs`, `security/playbook.rs`) berada. Yang tetap dilarang keras: menaruh KEPUTUSAN kebijakan (risk-tier threshold, RACI, auto-approve criteria) atau dependency langsung ke `clawcrew-runtime`/Go policy di utilitas channel. Script gate D-2 wajib mendeteksi coupling tata kelola nyata (simbol kebijakan + dependency graph), bukan sekadar grep nama fitur channel ATAU grep nama simbol approval I/O semata.
4. **Clean Web Packaging:** Seluruh dependensi web wajib resolvable dengan `npm ci` / `npm install` murni tanpa konflik `ERESOLVE`.

---

## 5. Protokol Navigasi Codebase (Graphify-First)

> **Graph first, grep second, manual browsing never.**

Semua pencarian simbol, penelusuran dependensi, pemahaman arsitektur, dan flow panggilan HARUS mengikuti urutan ini:

1. **Cek ketersediaan Graphify (`graphify-out/`):**
   ```powershell
   Test-Path graphify-out/graph.json
   ```
   Jika belum ada, bangun dengan `graphify .`. Jika sudah ada perubahan besar, perbarui dengan `graphify update .`.
2. **Kueri Knowledge Graph:**
   - Gunakan `graphify query "konsep/fitur"` untuk pencarian semantik.
   - Gunakan `graphify path "A" "B"` untuk menelusuri relasi antar komponen.
   - Baca `graphify-out/GRAPH_REPORT.md` untuk gambaran god nodes dan klaster modul.
3. **Targeted Grep:** Gunakan `Select-String` baris demi baris HANYA setelah file target diidentifikasi oleh Graphify:
   ```powershell
   # BENAR: Graphify menemukan file, lakukan pencarian presisi
   Select-String -Pattern "pub struct SystemGateway" crates/clawcrew-gateway/src/grpc_system_gateway.rs
   ```
4. **Direct File Read:** Buka file hanya pada baris yang relevan dari hasil langkah 2 & 3.
5. **Dilarang keras:** Melakukan recursive grep buta (`Get-ChildItem -Recurse | Select-String`) ke seluruh repo tanpa memanfaatkan graph.

---

## 6. Model Otomatisasi & Tata Kelola 3-Tier

Pengembangan dan review PR diatur dengan model eskalasi 3-Tier ([`3-TIER-AUTOMATION-MODEL.md`](../standard-docs/decisions/governance/3-TIER-AUTOMATION-MODEL.md)):

```
PR Diajukan ke Testing
         │
         ▼
┌───────────────────────────────┐
│ Cek Severity & Kata Kunci?    │
└───────────────────────────────┘
         │
         ├─► [TIER 1: NORMAL/MINOR] ──► Principle Bot AUTO-APPROVE & Auto-Merge (0 min Owner)
         │
         ├─► [TIER 2: CRITICAL/HIGH] ──► Lead Squad Analisis (3 Opsi) ──► Owner Memilih (5-15 min)
         │
         └─► [TIER 3: STRATEGIS/SCOPE] ─► Diskusi Lead + PO ──────────► Owner Memutuskan (10-30 min)
```

### Konvensi Git & PR:
- **Disiplin Konvensi Commit & PR (WAJIB):** Pastikan setiap commit baru wajib mengikuti format tiket kanonikal:
  ```
  feat({tiket}): {deskripsi}
  ```
  *(Contoh: `feat(BL-32): pulihkan verifikasi whatsapp signature`)*
  - `{tiket}` = ID tiket persis dari sprint/backlog (`BG-A4`, `BL-52`, `S2-SEC-01`, dst.). Jika menuntaskan >1 tiket: `feat(BL-52,BL-14): ...`.
  - Prefix SELALU `feat(...)` untuk seluruh pekerjaan sprint `dev-task-2` agar penomoran tiket seragam (bukan dicampur `fix(...)`/`chore(...)`).
  - Judul PR DAN judul commit wajib memakai format template ini.
- Gunakan branch non-`master` / non-`main` (branch fitur aktif: `feat/refactoring-finalize-01`, dilarang sub-branch liar).
- Target PR ke `main` / `testing`.
- Pisahkan commit per concern (ODR-10).
- Dilarang menyertakan bot/AI attribution footer di PR description atau commit body.
- Deklarasikan dependensi tugas dengan `Depends on #...` dan `Supersedes #...`.

### Penandaan & Pelaporan Penyelesaian Tugas (DoD Tracking):
- **Wajib Tandai `[x]` & Beri Deskripsi:** Setiap kali suatu tiket atau task selesai dikerjakan dan diverifikasi:
  - Wajib mengubah tanda checkbox dari `[ ]` menjadi `[x]` pada berkas sprint terkait di [`standard-docs/decisions/finalize/dev-task-2/sprint-*.md`](../standard-docs/decisions/finalize/dev-task-2/).
  - Wajib menambahkan **keterangan/deskripsi konkret penyelesaian** tepat di bawah atau di samping tiket tersebut (mencakup: commit hash, file yang diubah/dibuat, ringkasan solusi, dan bukti hasil test/verifikasi yang dijalankan).
  - Jangan biarkan tiket yang sudah selesai tanpa centang `[x]` atau tanpa deskripsi pembuktian.

---

## 7. Referensi Cepat Roadmap & PR Plan

Untuk eksekusi task, rujuk ke tabel master 121 tiket di [`standard-docs/roadmap/PR-plan.md`](../standard-docs/roadmap/PR-plan.md):
- **Gelombang 0 (P0 Dasar & Keamanan):** PR-00 s/d PR-06 (R-00 inventory, U-01 frontendDist, U-06 terminal removal, G-01 fail-closed).
- **Gelombang 1 (Pembersihan & Gate CI):** PR-10 s/d PR-22 (Zero unwrap gate, boundary gates, AI studio artifact cleanup, Ollama probe removal).
- **Gelombang 2 (Persistensi & Pemisahan Engine):** PR-30 s/d PR-72 (SQLite store, QR pairing, Agent loop ke Go, Channel decoupling).
- **Gelombang 3 (Rebranding Galleon):** PR-80 s/d PR-88 (Rename crates, env vars, binaries `galleon`/`galleoncode`/`galleonrelay`).
- **Gelombang 4 (Platform & Release):** PR-90 s/d PR-99 (Nginx TLS, Tauri sidecar, Release pipeline).
