---
sprint: 5
title: MCP/Plugin Governance & Sub-Agent Resource Management
status: planned
depends_on: BL-19 (Sprint 0, MCP sandbox dasar)
---

# Sprint 5 — MCP/Plugin Governance & Sub-Agent Resource Management

## A. Resource Management Sub-Agent (akar masalah crash yang dilaporkan Owner)

- [ ] **BL-15 (P1, HIGH, PRIORITAS TERTINGGI file ini)** Resource gate SEBELUM spawn sub-agent. Nol gate saat ini adalah penyebab langsung crash yang sudah dilaporkan.
- [ ] **BL-16 (MEDIUM)** Surface `loop_detector_circuit_breaker` ke Tasks-view — 3-pola deteksi sudah matang tapi hasil tidak pernah sampai ke dashboard.
- [ ] **BL-17 (LOW, exploratory, dep BL-16)** Agregasi lintas-sub-agent untuk fan-out besar.

> **BL-18 TIDAK di sprint ini** — sudah diangkat ke Sprint 4 sebagai acceptance-criteria tambahan BG-D1/BG-D4 (jalur Go `spawn_subagent`→`ExecuteTurn`), karena dependency aslinya ada di sana.

## B. Plugin & MCP Architecture (lanjutan BL-19 Sprint 0)

- [ ] **BL-20 (MEDIUM)** Dokumentasikan gap sandbox MCP (interim) — sambil BL-19 (Sprint 0) berjalan.
- [ ] **BL-21 (MEDIUM, evaluasi desain dulu)** Vault-mediation untuk secret MCP env/headers.
- [ ] **BL-22 (MEDIUM)** ADR pembanding WASM Plugin vs MCP.
- [ ] **BL-23 (MEDIUM)** Verifikasi wiring `PluginPermission::FileRead/Write`.

> **BL-24 TETAP TIDAK DIJADWALKAN** — exploratory, butuh keputusan arsitektur Owner eksplisit (RPC proto baru vs config distribution, menyentuh boundary Go/Rust yang sudah one-way-door). Tercatat di `backlog/README.md`, bukan di sprint manapun sampai Owner memutuskan.

## C. Plugin & Channel Tier (dev-tasks asli, scope berdekatan)

- [ ] **BR-F2 · R-35 · PR-86 (P1, M)** Host WIT v1 (dual v0/v1; v0 tetap lolos `.frozen`). Pasangan `RB-24` (Sprint 8).
- [ ] **BR-F3 · R-32 · PR-64 (P2, L)** Channel regional Tier B (WeChat/QQ…) + feature flag.
- [ ] **BR-F4 · R-33 · PR-94 (P2, M)** Pilih Tier B saat instalasi.
- [ ] **BR-F5 · R-31 · PR-95 (P2, M)** Tier C WASI: pilot `channel-nostr-pilot` + `.frozen` v0 sudah ada; tinggal e2e penuh (host, signature, capability) dan ukur batas `wasi:http` outbound.

> **BR-F1 TIDAK dijadwalkan** — duplikat `GV-07` yang sudah `[x]` selesai PR-21 (lihat `README.md` §Reconciliation). Tidak ada kerja baru di sini, hanya perlu dicentang `[x]` di dev-tasks asli.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `crates/clawcrew-runtime/src/agent/spawn.rs` (atau setara), dashboard Tasks-view (`web/`) | Backend-Rust + Frontend |
| B | MCP client host (`crates/clawcrew-plugins/` atau setara — SAMA file dengan BL-19 Sprint 0, koordinasikan) | Backend-Rust |
| C | `wit/v1/`, `crates/clawcrew-channels/` Tier B/C | Backend-Rust |

## Keluaran Sprint

- Sub-agent spawn punya cap jumlah/memori — root cause crash tertutup.
- MCP governance (vault-mediation, permission wiring) terverifikasi, bukan asumsi.
- WIT v1 hidup berdampingan v0 tanpa breaking existing plugin.
