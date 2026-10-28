# 01 — Peta Repositori

> Fakta per 2026-10-04. Jumlah file = perkiraan (tanpa `node_modules`, `target`, `.git`).
> **Tier:** T1 Desktop Shell, T2 Rust Core, T3 Go Orchestrator, T4 UI (lihat `tech-standart/architecture/00-overview.md`).
> **Status:** ✅ aktif Galleon · 🕰 warisan ClawCrew (dipelihara, bukan target) · 🔧 tooling.

## 1. Kode produk

| Path | File | Tier | Isi & peran | Status |
|---|---:|---|---|---|
| `web/` | 152 | T4 | (ODR-01; dulu `web-2/`) React 19 + TS + Vite + Tailwind 4 + Zustand. UI Galleon (Harbor, Treasury, Shipyard, EngineRoom, …). Memuat `server.ts` Express (QR, telemetri OS) = pelanggaran ADR 0002; artefak AI Studio (lihat 02 M5) | ✅ |
| `apps/tauri/` | ~10 | T1 | (ODR-01; dulu `apps/tauri-2/`) Shell Tauri v2 "Fleet AI", paket `fleet-ai-desktop`; `frontendDist` masih `../../web-2/dist` (rusak, U-01). Anggota workspace Cargo | ✅ |
| `engine/` | 155 | T3 | Modul Go mandiri: `cmd/agent-engine`, `app` (Wire), `core` (config, logger, metrics, interceptors), `pkg/client` (klien Rust), `pkg/pb`, `src/<domain>` (±99 file), `data/` (17 seed JSON), `tests/` | ✅ |
| `proto/` | 1 | T2↔T3 | `agent_service.proto` — kontrak gRPC Go↔Rust | ✅ |
| `crates/` | 942 | T2 | 27 crate Rust (§2) | ✅ sedang di-refactor |
| `wit/` | 12 | T2 | `wit/v0/*.wit` kontrak plugin WASI (`clawcrew:plugin@0.1.0`), `.frozen`, `VERSIONING.md` | ✅ beku v0 |
| `src/` | 126 | T2 | Binary root `clawcrew`: `main.rs`, `main_cli_args.rs`, subcommand CLI (`quickstart`, `agent`, `gateway`, `daemon`, `doctor`, `cron`, `channel`, `sop`, `plugin`, `desktop`, `eval`, `locales`, …) | 🕰/✅ host bagi crate Rust |
| `apps/zerocode/` | — | T2 | TUI/runtime agen lokal Rust | 🕰 |
| `apps/zerorelay/` | — | T2 | Relay jarak jauh | 🕰 |
| `web/` | 193 | T4 | Dashboard ClawCrew; di-embed ke binary root lewat `/web/dist` | 🕰 |
| `apps/tauri/` | — | T1 | Shell Tauri ClawCrew + supervisor daemon (`clawcrew-desktop`, anggota workspace) | 🕰 |
| `firmware/` | 36 | — | Firmware perangkat/periferal | pendukung hardware |
| `xtask/`, `tools/fill-translations/` | 48 / 2 | — | Otomasi build & terjemahan (anggota workspace) | 🔧 |

## 2. Crate Rust (`crates/`)

Ukuran dan arah dependency: Phase 5 §1 (`cargo metadata`).

| Kelompok | Crate | Peran |
|---|---|---|
| Kontrak/leaf | `clawcrew-api` | Trait + tipe bersama (tanpa dep internal) |
| | `galleon-macros`, `-commands`, `-relay-proto`, `-sop-graph`, `-dist` | Macro config, katalog slash-command, protokol relay, proyeksi graf SOP, registri target rilis |
| Fondasi | `clawcrew-log`, `-spawn`, `-infra`, `-tls` | Logging terstruktur, spawn ber-atribusi, backend sesi/watchdog, TLS/mTLS |
| | `clawcrew-config` (85k LOC, hub), `galleon-i18n` | Skema config & secret; Fluent i18n |
| Domain | `clawcrew-providers` | **SSOT LLM** (ADR 0001) |
| | `-memory`, `-tools`, `-tool-call-parser`, `-hardware` | Memori & embedding, tool agen, parser tool-call, perangkat |
| | `clawcrew-runtime` (286k LOC) | Agent loop, SOP, hooks, cron, security, observability, skills, RPC, TUI, subagents |
| Tepi | `clawcrew-channels` (198k LOC) | ±35 adapter channel + orchestrator |
| | `galleon-channel-{core,signal,bluesky,imessage}` | Pola feature-crate channel (Tier B) |
| | `clawcrew-plugins` | Host WASI/Wasmtime, manifest, signature, catalog, egress |
| Komposisi | `clawcrew-gateway` | HTTP/WS gateway (`api_*`), **gRPC `SystemGatewayService`** (`grpc_system_gateway.rs`), vault, pairing |
| | `galleon-routing` | `DynamicRouter`/`RouteHandler` (kini dependency gateway; belum di-commit) |
| | `clawcrew-eval` | Harness evaluasi trace |

Fixture (anggota workspace): `crates/clawcrew-plugins/tests/fixtures/*` (channel, nostr-pilot, egress, tool-*), `crates/clawcrew-log/tests/fixtures/attribution-macro-*`.

## 3. Domain Go (`engine/src/`)

| Kelompok | Package | Catatan |
|---|---|---|
| Eksekusi | `crew`, `run`, `task`, `workflow`, `tool`, `orchestrator` | `crew`/`orchestrator` memanggil Rust `ExecuteTurn`; `tool` masih punya eksekusi in-process (04 V3) |
| Governance | `policy`, `approval`, `audit`, `entitlement`, `treasury`, `charter` | Sebagian interface/stub (mis. `treasury`) |
| Organisasi | `fleet`, `ship`, `squad`, `mission`, `navigator`, `briefing`, `schedule`, `integration` | `fleet` memuat Quartermaster chat, diagnostics, QR/metrik sistem |
| Data | `memory`, `artifact`, `persistence` | `persistence` = DiskStore JSON/JSONL |

Port: gRPC `:50051`, HTTP `:9090` (`/metrics`, `/healthz`, `/api/v1/*`, `/api/turn`, `/api/query`, fleet routes), klien ke Rust `localhost:50052`.

## 4. Test, kualitas, ekosistem

| Path | File | Isi |
|---|---:|---|
| `tests/` | 87 | `architecture/`, `component/`, `integration/`, `system/`, `live/`, `manual/`, `support/` |
| `fuzz/`, `benches/`, `evals/`, `fixtures/` | 6 / 1 / 9 / 9 | Fuzz, benchmark, regresi evaluasi, fixture JSON |
| `dev/`, `scripts/` | 20 / 101 | Harness lokal & CI (`dev/ci.sh`), skrip gate/rilis/deploy |
| `.github/` | 73 | 39 workflow, template PR, agent (`principle`, `product-owner`, `technical-lead`), `copilot-instructions.md` |

## 5. Distribusi & deployment

`Dockerfile{,.alpine,.debian,.ci}`, `Containerfile`, `docker-compose*.yml`, `Makefile` (Podman), `deploy-k8s/`, `nix/` + `flake.nix`, `dist/` (AUR/Scoop/FreeBSD), `install.sh`/`install.ps1`/`setup.bat`. Seluruhnya dibangun untuk produk warisan (🕰); belum disesuaikan untuk `web/` + `apps/tauri/` Galleon (lihat 02 M2–M3).

## 6. Dokumentasi & konfigurasi agen AI

| Path | Peran |
|---|---|
| `standard-docs/` | **Otoritas** keputusan, standar, kontrak, PRD, arsip eksekusi, fondasi |
| `docs/` (341) | Warisan upstream: analisis, `launch-gate/`, `maintainers/`, `security/`, `assets/`, `_archive/`; `docs/refactoring/` kosong |
| `AGENTS.md`, `CLAUDE.md` | Aturan bersama agent |
| `.gemini/`, `.kiro/`, `.claude/`, `.agents/`, `.copilot/` | Steering per-tool; harus selaras dengan `standard-docs/` (bentrok → `standard-docs/` menang) |
| `.pr-evidence/` (40) | Bukti/screenshot PR |

## 7. File root

Konfigurasi sah: `Cargo.toml/lock`, `clippy.toml`, `rustfmt.toml`, `deny.toml`, `taplo.toml`, `release-plz.toml`, `locales.toml`, `Justfile`, `.editorconfig`, `.nvmrc`, `package.json`.
Kandidat dirapikan (04 §3): `rebrand.py`, `rebrand_icons.py`, `create_logo.py`, `clean_docs.py`, `update_docs.py`, `PR_PHASE2B_TEMPLATE.md`, `CHANGELOG_FIXES.md`, `base_logo.png`.
