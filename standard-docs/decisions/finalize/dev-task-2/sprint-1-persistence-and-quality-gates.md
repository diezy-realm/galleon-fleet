---
sprint: 1
title: Persistence & Quality Gates — Postgres, RAG/Vector, Rust Burn-down, Token/Knowledge Efficiency
status: planned
depends_on: BG-B1/BG-B2 (sudah selesai, lihat dev-tasks/01)
---

# Sprint 1 — Persistence & Quality Gates

## A. Persistensi Go Lanjutan

- [ ] **BG-B3 · G-12 · PR-42 (P2, L)** Store PostgreSQL + test container. Dep: BG-B1 (selesai).
- [ ] **BG-B4 · G-13 · PR-43 (P1, M)** Memori vektor/RAG per scope Fleet/Ship/Squad/Crew/Run. Dep: BG-B2 (selesai). **Acceptance criteria tambahan (eks-BL-51, DITUNDA → DIANGKAT sprint ini karena dependency terpenuhi):** desain koordinasi lintas Go-Rust untuk RAG/vector mengikuti pola yang sama dengan `knowledge_graph.rs` Rust (lihat §C) — JANGAN bangun 2 sistem vector paralel tanpa jembatan.
- [ ] **BG-B5 · G-14 · PR-43 (P2, M)** Retensi, ekspor, hapus (privasi) per scope. Dep: BG-B2 (selesai).

## B. Burn-down Kualitas Rust

- [ ] **BR-A5 · R-02 · PR-30 (P1, M, berseri per crate)** Burn-down `unwrap/expect` produksi (±400 hit; `gateway`, `channels`, `runtime`). Dep: BR-A1 (selesai).
- [ ] **BR-A6 · R-05 · PR-20 (P2, S)** Selaraskan `rust-conventions.md` (MSRV, jumlah crate).

## C. Token/Cache Efficiency (diangkat ke roadmap sebagai R-88..91)

- [ ] **R-88 (eks-BL-45a, P1, MEDIUM, PRIORITAS — murah)** Ukur korelasi trim-history vs cache-miss sebelum mitigasi apa pun. Logging-only, menjawab apakah R-89 layak dengan data nyata.
- [ ] **R-89 (eks-BL-45b, KONDISIONAL)** Align titik potong trim dengan rolling cache-breakpoint. **Hanya dikerjakan jika R-88 konfirmasi dampak signifikan** — jangan mulai sebelum data R-88 ada.
- [ ] **R-90 (eks-BL-46, LOW-MEDIUM)** Surface cache hit-rate ke dashboard/CLI operator. Setelah R-88 minimal selesai.
- [ ] **R-91 (eks-BL-47, LOW, independen, biaya nol)** Dokumentasikan prinsip "steering stabil = cache-friendly".

## D. Knowledge-Base / Citation (diangkat ke roadmap sebagai R-92..95)

- [ ] **R-92 (eks-BL-48, MEDIUM)** Field citation + verifikasi pada node Lesson/Decision di `knowledge_graph.rs` — mitigasi bug "configuration-lies" (`query_by_similarity` nama vector tapi impl FTS5 keyword).
- [ ] **R-93 (eks-BL-49, MEDIUM-HIGH)** Aktifkan Knowledge Library untuk `standard-docs/` — setelah R-92 untuk koherensi (tidak hard-block).
- [ ] **R-94 (eks-BL-50, LOW, murni dokumentasi)** Query decomposition + domain-filtering sebagai SOP riset (pola dari metodologi Perplexity, bukan kode baru).

## E. Skills/Steering/Lessons Persistence (dari `skills-steering-lessons-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)

> Tema sama dengan §A/§D di atas (jembatan Go-Rust untuk storage vektor/persisten) — BL-01 digabung ke sini karena pola arsitekturnya identik: Go pegang kebijakan, Rust pegang storage, gRPC `SystemGateway` sebagai jembatan.

- [ ] **BL-01 (HIGH, bug fungsional nyata)** Jembatan gRPC: persist Lesson/Learning dari Go (`engine/src/memory/proposal.go`, "Learning by Consent" lifecycle lengkap) ke Rust vector store (`clawcrew-memory`) — saat ini `Approve()`/`Revert()` menulis ke `inMemoryVectorStore` Go yang hilang total saat restart proses. Scope: RPC baru `StoreLearnedRule`/`RetrieveLearnedRules` di proto `SystemGateway`, implementasi di `clawcrew-gateway/src/grpc_system_gateway.rs` (reuse `clawcrew-memory`, JANGAN tulis ulang vector-store). **Serialisasi: proto dulu, baru implementasi Go+Rust paralel.**
- [ ] **BL-03 (LOW, dokumentasi murni, bisa paralel kapan saja)** Dokumentasikan ekosistem `open-skills` yang sudah matang (`crates/clawcrew-runtime/src/skills/mod.rs`, 3-lapis loading workspace→global→plugin WASM) — risiko duplikasi SSOT kalau tim lain menganggap belum ada. Tulis `standard-docs/tech-standart/plugin-standard/open-skills-ecosystem.md`.
- [ ] **BL-04 (MEDIUM, verifikasi — lebih bermakna SETELAH BL-01 selesai)** Test baru: skill/steering inheritance untuk sub-agent (`DelegateTaskTool`) — verifikasi sub-agent mewarisi skill-bundle yang sama dengan parent (bukan skill-set kosong tanpa alasan). File: `engine/src/tool/phase3_tool_test.go` atau test baru.

> **BL-02 (GAP-01, perancangan steering produk Galleon end-user) TIDAK dimasukkan sprint ini** — scope-nya adalah ADR/design-spec (keputusan one-way-door format file untuk end-user), bukan kerja kode. Lebih cocok sebagai item mandiri di Sprint 7 (produk FE) karena menyentuh pengalaman end-user `galleon`/`galleoncode` CLI, bukan backend internal — lihat Sprint 7 §F.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `engine/pkg/store/postgres/`, `engine/pkg/store/vector/` (baru) | Backend-Go |
| B | `crates/clawcrew-{gateway,channels,runtime}/src/**` (per-crate berseri, JANGAN paralel di crate yang sama) | Backend-Rust |
| C | `crates/clawcrew-runtime/src/context/{trim,cache}.rs` (lokasi perlu konfirmasi saat eksekusi) | Backend-Rust |
| D | `crates/clawcrew-runtime/src/knowledge_graph.rs`, `clawcrew-api/src/knowledge.rs` | Backend-Rust |
| E | `proto/agent_service.proto`, `crates/clawcrew-gateway/src/grpc_system_gateway.rs`, `engine/src/memory/{proposal,store,wire}.go`, `standard-docs/tech-standart/plugin-standard/` (BL-03) | Backend-Go + Backend-Rust (BL-01 serial: proto dulu) |

## Keluaran Sprint

- RAG/vector Go-Rust tidak lagi dua sistem paralel tanpa jembatan.
- Burn-down `unwrap/expect` turun signifikan dari ±400 hit baseline.
- Data konkret ada/tidaknya korelasi trim-history vs cache-miss (R-88) — keputusan R-89 berbasis data, bukan hipotesis.
- Node Lesson/Decision punya citation terverifikasi, bukan klaim tanpa sumber.
- "Learning by Consent" (Go) persisten lintas-restart lewat jembatan gRPC ke Rust vector store — bukan lagi `inMemoryVectorStore` yang hilang saat proses mati.
