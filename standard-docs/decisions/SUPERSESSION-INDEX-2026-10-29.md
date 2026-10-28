# Indeks Supersesi — ODR-2026-10-29 (Rust Single Language)

**Tujuan:** satu tempat untuk mengetahui dokumen mana yang tidak lagi berlaku, sebagian berlaku, atau tetap. ODR lama **tidak diedit dan tidak diberi banner** (immutable); statusnya dibaca dari indeks ini. Banner hanya dipasang pada ADR berstatus Superseded (ADR-005, §2) dan pada dokumen normatif Go (§3, oleh TASK-092). ADR-0002 (Diamandemen) mendapat satu catatan amandemen; ADR berstatus Tetap tidak diubah.

## 1. Owner Decision Records

| Dokumen | Butir | Status baru | Diganti oleh |
|---|---|---|---|
| ODR-2026-10-04-product-direction | ODR-02 (klausa "Go sebagai gerbang" QR) | Superseded sebagian | ODR-28 (pairing di Rust gateway) |
| | ODR-03 | Tetap (fokus produk); implikasi "otak Go" Superseded | ODR-28 |
| | ODR-05 | Tetap (arah SQLite); implikasi store Go Superseded | ODR-32 |
| | ODR-07 | Superseded | ODR-28 (terminal via Rust gateway + SecurityPolicy + ApprovalManager) |
| | A1, A3, A4 | Superseded | ODR-27, ODR-28 |
| | ODR-01, 04, 06, 08, 09, 10 | Tetap | — |
| ODR-2026-10-04-addendum-1 | ODR-11, ODR-12, ODR-14 | Superseded | ODR-27, ODR-28 |
| | ODR-13, 15, 16 | Tetap | — |
| ODR-2026-10-04-addendum-2 | ODR-17, ODR-18 | Tetap | — |
| ODR-2026-10-04-addendum-3 | ODR-19 | Tetap | — |
| | ODR-20 | Tetap; catatan "retarget eval ke Go" tidak relevan | ODR-28 |
| ODR-2026-10-04-addendum-4 | ODR-21, 24, 25 | Tetap | — |
| | ODR-22, ODR-23 | Superseded | ODR-28 (identitas/pairing/WebAuthn di Rust; Client Protocol server tidak dibangun di Go — kosakata JSON-RPC Rust yang ada tetap) |
| ODR-2026-10-05-delegated-authority-chain | DAC sebagai gate wajib | Tetap | — |
| | Penempatan pemeriksa principal di Go | Superseded | ODR-28 (Rust gateway) |
| ODR-2026-10-09-dev-task-2-remediation-scope | T-02 (logging Go) | Tidak relevan setelah Go dihapus | ODR-26 |
| | T-08 (DeviceRegistry SQLite SSOT) | Tetap untuk Rust; bagian cache Go Superseded | ODR-28 |

## 2. ADR

| Dokumen | Status baru | Catatan |
|---|---|---|
| `0001-llm-ssot-rust.md` | Tetap | Pemanggil Go hilang; SSOT tetap Rust |
| `0002-web2-backend-policy.md` | Diamandemen | "backend = Go API" dibaca "backend = Rust gateway" |
| `0003-tauri-metrics-real-api.md` | Tetap | Sumber metrik = `/api/system/metrics` di Rust gateway |
| `finalize/ADR-005-persistence-store-design.md` | Superseded | ODR-32 |
| `adr-wasm-plugin-vs-mcp.md` | Tetap | Jalur integrasi add-on (ODR-30) |
| `0018-rust-single-language-architecture.md` | **Baru, Accepted** | — |

## 3. Dokumen Normatif yang Diberi Banner "Superseded" (TASK-092)

Tambahkan di baris paling atas:

```markdown
> [!WARNING]
> **Superseded oleh ODR-2026-10-29 / ADR-0018.** Dokumen ini dipertahankan sebagai arsip. Jangan dijadikan acuan implementasi.
```

| File | Tindakan |
|---|---|
| `standard-docs/tech-standart/tech-stack/go-conventions.md` | Banner + pindah ke `standard-docs/archive/` |
| `standard-docs/tech-standart/api-contracts/grpc-system-gateway.md` | Banner + arsip |
| `standard-docs/tech-standart/api-contracts/http-engine-api.md` | Banner; isi rute dipindah ke `api-contracts/consumer-inventory.md` (TASK-058) |
| `standard-docs/roadmap/G-go-engine.md` | Banner + arsip |
| `standard-docs/tech-standart/architecture/00-overview.md` | Ganti diagram 4-tier → 3-tier (rujuk `01-boundary-policy.md` v2) |
| `standard-docs/tech-standart/architecture/01-boundary-policy.md` | Diganti penuh dengan v2 (paket ini) |
| `standard-docs/templates/template-odr.md`, `template-adr.md` | Lihat `TEMPLATES-AMENDMENT` di paket board |

## 4. File Steering Agent & Referensi Go yang Wajib Diperbarui (TASK-053, TASK-054, TASK-092)

Steering agent: `.github/copilot-instructions.md` (16 hit), `.github/agents/principle.agent.md` (8), `.github/agents/technical-lead.agent.md` (5), `.gemini/steering.md` (20), `.gemini/architecture.md` (20), `.kiro/steering/galleon-architecture.md` (7), `.kiro/steering/galleon-web-ts.md` (1), `apps/tauri/AGENTS.md`, `engine/AGENTS.md` (dihapus bersama `engine/`). Hit = `rg -c -i '\bgo\b|:9090|golang'` @3f3a729. `AGENTS.md` dan `CLAUDE.md` root **tidak** memuat referensi Go (0 hit) — tidak perlu diubah.

Daftar lengkap 76 file non-`engine/` yang memuat `:9090|go engine|golang|ENGINE_URL` ada di `board/research/RES-2026-10-29-go-reference-inventory.txt`. Yang berupa **kode/test** (bukan dokumen) dan wajib ditangani:

| File | Tindakan | Task |
|---|---|---|
| `tests/architecture/tier_boundaries.rs`, `tests/architecture/no_duplicate_risk_tier.rs` | Tulis ulang aturan arsitektur ke model 3-tier (risk-tier SSOT = Rust) | TASK-091 |
| `apps/galleoncode/src/config/mod.rs` `[engine]` section (`enabled=false`, `:9090`, `:50051`), `apps/galleoncode/tests/contract_test.rs` | Hapus section & test kontrak Go | TASK-089 |
| `crates/galleon-eval/src/runner.rs` (`GoTurnRequest`, doc "Go engine") | Rename tipe/doc; runner sudah memakai `galleon_runtime::agent::Agent` | TASK-089 |
| `crates/galleon-api/src/sop.rs:4`, `crates/galleon-gateway/Cargo.toml:32`, `apps/tauri/Cargo.toml:20` | Perbarui komentar | TASK-088 / TASK-085 |
| `scripts/migrations/extract-seed.mjs` | Arahkan output ke seed `galleon-fleet-core` atau hapus bila tidak dipakai | TASK-070 |
| `web/src/utils/apiClient.ts`, `web/vite.config.ts`, `web/src/components/features/*`, `web/src/components/layout/RightRail.tsx` | Repoint ke gateway | TASK-082, TASK-083, TASK-084 |
| `apps/tauri/src/main.rs`, `apps/tauri/tauri.conf.json`, `nginx/galleon.conf` | Repoint ke gateway | TASK-085, TASK-086 |

Teks wajib di setiap file steering:

```markdown
## Bahasa & Boundary (ODR-2026-10-29)
- Produk core `galleon-fleet` = Rust (+ TypeScript untuk `web/`). Jangan menulis atau mengusulkan kode Go di repo ini.
- Satu API produk: Rust gateway (`galleon-gateway`, default 127.0.0.1:42617). Tidak ada `:9090`, tidak ada gRPC internal.
- Add-on Go = repository terpisah, integrasi lewat HTTP API / MCP / WASM `galleon:plugin@1.0.0`.
- Acuan: `standard-docs/tech-standart/architecture/01-boundary-policy.md` v2.
```
