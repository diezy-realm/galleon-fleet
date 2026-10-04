# Finalize — Ringkasan Eksekusi Galleon Fleet

> Folder ini berisi kesimpulan analisis konkret dan rencana eksekusi terakhir berdasarkan observasi langsung terhadap kode di branch `feat/refactoring-rust`.

## Isi Folder

| File | Isi |
|------|-----|
| [`analysis-conclusion.md`](./analysis-conclusion.md) | Kesimpulan observasi konkret per area: Phase 2 (Lane A–F + ADR-001), Phase 3 (DynamicRouter), Phase 4 (ADR 0001/0002/0003). Memuat tabel perbandingan "status dokumen vs kode aktual". |
| [`task-breakdown.md`](./task-breakdown.md) | Rincian task dengan checkbox `[ ]`, dikelompokkan per Grup (0–9), lengkap dengan: scope file, dependency, acceptance criteria, dan referensi dokumen normative. |
| [`dev-tasks/`](./dev-tasks/README.md) | **Task breakdown tim BE/FE (aktif):** 8 file per tim/kategori (BE-Go, BE-Rust, FE-Web, FE-Desktop, Platform/QA, Rebrand, Governance, Carryover) dengan checkbox `[ ]`, tiket roadmap, nomor PR, dan keputusan terbuka. Sumber kerja harian; menggantikan sisa pekerjaan di `task-breakdown.md`. |
| [`testing-gate-checklist.md`](./testing-gate-checklist.md) | Checkpoint testing gate (Gate G0–G9 + Gate Umum) untuk Principle me-review setiap PR dari `task-breakdown.md`, mengikuti pola `RF-C-channel-crate-split-self-register.md` dan `RF-D-principle-gateway-checklist.md` di `refactoring-rust/`. |

## Lanjutan (2026-10-04)

Asesmen crate Rust, roadmap pemecahan kode besar, dan standar plugin ada di
[`../refactoring-rust/phase-5-production-hardening/README.md`](../refactoring-rust/phase-5-production-hardening/README.md).
Standar normatif baru: [`code-quality/`](../../tech-standart/code-quality/rust-module-size-and-decomposition-standard.md) dan
[`plugin-standard/`](../../tech-standart/plugin-standard/plugin-codebase-standard.md).
Koreksi drift: Phase 4 berada di `decisions/refactoring-rust/phase-4/`; `clawcrew-routing` kini sudah masuk dependency gateway (uncommitted); `galleon-channel-{core,imessage,signal,bluesky}` sudah ada.

## Scope Observasi

```
standard-docs/decisions/refactoring-rust/phase-2/   → SSOT Migration (Lane A–F)
standard-docs/decisions/refactoring-rust/phase-3/   → DynamicRouter Design Spec
standard-docs/decisions/phase-4/                    → ADR 0001/0002/0003 (Accepted)
```

## Status Snapshot (diperbarui 2026-10-04, diverifikasi dari kode)

> Path `web-2`/`tauri-2` sudah menjadi `web/`/`apps/tauri/` (ODR-01).

| Area | Progress |
|------|----------|
| Phase 2 Lane A/B/F | ✅ Selesai |
| Phase 2 Lane C (C1–C5) | ✅ Selesai (`MemoryProposal` ada di `engine/src/memory`; peta rute `route-map.md`) |
| Phase 2 Lane D (fail-closed) | ⚠️ `builtin_*.go` via gateway ✔; sisa `os/exec` di `tool/{services,sandbox,plugin,mcp_client}.go` → BG-A1 |
| Phase 2 Lane E | ✅ Tipe/de-mock selesai; sisa `seedData.ts` yatim (FW-A5) |
| ADR-001 Channel 3-Tier | ⚠️ Pilot Tier C + WIT v0 beku + `galleon-channel-*` ✔; errata & WIT v1 menunggu (BR-F1, BR-F2) |
| Phase 3 DynamicRouter | ⚠️ T-6.1–6.4 ✔; T-6.5–6.10 tertahan keputusan Owner (CO-09) |
| ADR 0001 (LLM SSOT Rust) | ✅ `ExecuteTurn` + Go delegasi; `engine/src/llm` dihapus |
| ADR 0002 (web zero logic) | ⚠️ QR/metrics di Go ✔; `web/server.ts` + dependensi lama masih ada (FW-A1/A2) |
| ADR 0003 (Tauri real API) | ⚠️ `get_system_metrics` ✔; `execute_terminal_command` simulasi palsu, koleksi lokal, `frontendDist` rusak, CSP longgar (FD-A1..A4) |
| Vault secret SSOT | ✅ `get_decrypted_secret` terimplementasi |
| Roadmap baru (R/G/U/P/D/N, 121 tiket) | 📋 Lihat `../../roadmap/PR-plan.md`; belum ada PR dieksekusi |
## Keputusan Arsitektur (Status Approval)

| # | Poin Keputusan | Status | Rincian Keputusan User |
|---|---------------|--------|------------------------|
| 1 | Sign-off ADR-001 Channel 3-Tier Model | ⏳ Antrean | Mengikuti tahapan WASI pilot Tier C |
| 2 | Approval A2 2-Brain Quartermaster (Routing 2-Layer) | ✅ **APPROVED** | Disetujui: routing 2-layer untuk memfilter pertanyaan umum vs koordinasi crew/ships/squad |
| 3 | Fail-Closed policy untuk Lane D1/D2 | ✅ **APPROVED** | Disetujui: **Error Reject (Fail-Closed)** — Rust adalah Core sistem, Go hanya orkestrator; dilarang fallback eksekusi lokal di Go |
| 4 | ADR 0001 LLM SSOT via Rust gRPC streaming | ✅ **APPROVED** | Disetujui: Rust SSOT untuk LLM provider, Go mendelegasikan lewat gRPC |
| 5 | Compile & Test Optimization (sccache & rust-lld) | ✅ **ACTIVE** | `sccache 0.18.0` terpasang & aktif di `.cargo/config.toml`, targeted check/test standard aktif |

