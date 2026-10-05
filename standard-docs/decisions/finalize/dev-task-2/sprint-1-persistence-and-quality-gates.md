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

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `engine/pkg/store/postgres/`, `engine/pkg/store/vector/` (baru) | Backend-Go |
| B | `crates/clawcrew-{gateway,channels,runtime}/src/**` (per-crate berseri, JANGAN paralel di crate yang sama) | Backend-Rust |
| C | `crates/clawcrew-runtime/src/context/{trim,cache}.rs` (lokasi perlu konfirmasi saat eksekusi) | Backend-Rust |
| D | `crates/clawcrew-runtime/src/knowledge_graph.rs`, `clawcrew-api/src/knowledge.rs` | Backend-Rust |

## Keluaran Sprint

- RAG/vector Go-Rust tidak lagi dua sistem paralel tanpa jembatan.
- Burn-down `unwrap/expect` turun signifikan dari ±400 hit baseline.
- Data konkret ada/tidaknya korelasi trim-history vs cache-miss (R-88) — keputusan R-89 berbasis data, bukan hipotesis.
- Node Lesson/Decision punya citation terverifikasi, bukan klaim tanpa sumber.
