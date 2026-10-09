---
sprint: 1
title: DoD & PR Checklist — Persistence & Quality Gates
---

# DoD & PR Checklist — Sprint 1 (Persistence & Quality Gates)

## Gerbang Wajib

- [ ] **GW-0 Hygiene** — sama seperti standar.
- [ ] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`BG-B3,BL-51`); PR description baris pertama menyebut `Sprint 1` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [ ] **GW-1 Rust** — `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>` (Windows: `$env:CARGO_TARGET_DIR="target/agent"`); **nol `unwrap()/expect()` baru**.
- [ ] **GW-2 Go** — `go vet ./...`; `go test -race ./...`; migrasi data (BG-B3 Postgres) punya jalur rollback tertulis.
- [ ] **GW-9 Rollback** — WAJIB untuk `BG-B3`/`BG-B4`/`BG-B5` (semua menyentuh persistensi) — PR migrasi data memuat jalur mundur eksplisit.

## Acceptance Criteria Spesifik Sprint Ini

- [ ] `BG-B3`: test container Postgres lolos; migrasi dari SQLite (kalau ada) idempoten.
- [ ] `BG-B4`: desain RAG/vector Go-Rust TIDAK menciptakan sistem paralel kedua tanpa jembatan — PR harus eksplisit menyebut bagaimana ini berkoordinasi dengan `knowledge_graph.rs` Rust.
- [ ] `BR-A5`: progres burn-down `unwrap/expect` terukur per-PR (jumlah hit sebelum/sesudah dicatat di deskripsi PR), bukan klaim "sudah dikurangi" tanpa angka.
- [ ] `R-88`: data korelasi trim-history vs cache-miss ADA dan dibaca sebelum `R-89` di-assign — PR `R-89` TANPA rujukan data `R-88` ditolak.
- [ ] `R-92`: node Lesson/Decision baru WAJIB punya field citation terisi — test baru menolak node tanpa citation.

## Checklist Review Principle

1. Query SQL ter-parameterized (no injection) — relevan untuk `BG-B3` Postgres.
2. Index/execution plan dipertimbangkan untuk query baru (3NF kecuali ada alasan denormalisasi eksplisit, dicatat).
3. Tidak ada `unwrap/expect` baru walau di area "quality burn-down" — regresi di area yang sedang diperbaiki adalah CHANGES-REQUESTED otomatis.
4. Vector storage (`BG-B4`) punya isolasi per-scope (Fleet/Ship/Squad/Crew/Run) — bukan satu index global.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi R-88 data ADA sebelum R-89 di-approve>
```
