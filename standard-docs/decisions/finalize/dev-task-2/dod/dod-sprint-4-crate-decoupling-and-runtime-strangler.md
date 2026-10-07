---
sprint: 4
title: DoD & PR Checklist — Crate Decoupling & Runtime Strangler
---

# DoD & PR Checklist — Sprint 4 (Crate Decoupling & Runtime Strangler)

## Gerbang Wajib

- [x] **GW-0 Hygiene** — standar.
- [x] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`BR-C5,BG-D1`); PR description baris pertama menyebut `Sprint 4` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [x] **GW-1 Rust** — `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>` (Windows: `$env:CARGO_TARGET_DIR="target/agent"`); **nol `unwrap()/expect()` baru**; plus: `BR-C5`/`BR-C6`/`BR-D1` adalah PR berseri (XL) — setiap PR dalam seri HARUS lolos `cargo metadata` dependency-direction gate (`BR-A3`, sudah terkunci) sendiri, bukan menunggu seri selesai total.
- [x] **GW-2 Go** — `BG-D1`..`BG-D5` adalah fitur BARU di Go, bukan port langsung — `go test -race ./...` WAJIB untuk setiap PR, tidak ada pengecualian "akan ditest nanti".
- [x] **GW-6 Integrasi** — WAJIB untuk sprint ini (XL, lintas-tier): smoke e2e start engine Go + gateway Rust → `StartTurn` stream → `ExecuteNativeTool` fail-closed saat Rust mati → UI memuat; hasil tertulis di deskripsi PR.

## Acceptance Criteria Spesifik Sprint Ini

- [x] `BG-D1`: acceptance "paritas dengan `eval` replay" didukung struktur turn token gRPC stream dan tool loop parity.
- [x] `BG-D1`/`BG-D4` (eks-`BL-18`): test resource-gate pada jalur Go `spawn_subagent`→`ExecuteTurn` dan `HandoffSubagent` — kirim spawn request melebihi cap, terbukti DITOLAK via test `resource_gate_test.go`.
- [x] `BR-C5`: ground truth count 425 ref terverifikasi di `crates/clawcrew-gateway/src/` dan didokumentasikan di `audit-control-plane-and-approval-parity.md`.
- [x] `BR-D1`: urutan modul strangler runtime R-00 §3 dipetakan dengan parity Go.
- [x] `BR-D3`: modul `verifiable_intent` terarsip `archive/pre-galleon` dan dihapus bersih (5,663 LOC).
- [x] `BR-D4`: penghapusan `rpc` lama dipagari oleh prasyarat `BR-E3` (Sprint 6) dan `BR-C5`.

## Checklist Review Principle

1. Urutan sirkular `BG-D1`/`BR-C5`/`BG-E2` (dicatat di `sprint-4-*.md` §Catatan Sirkularitas) — Principle memverifikasi urutan EKSEKUSI nyata yang diklaim PR cocok dengan yang Lead tetapkan, bukan diasumsikan dari urutan file.
2. Setiap crate split baru (`galleon-channel-*`, dst) punya `Cargo.toml` dengan dependency MINIMAL — tidak membawa dependency `clawcrew-runtime` "untuk jaga-jaga".
3. Tidak ada `#[allow(dead_code)]` baru selama proses strangler — kode yang belum terpakai dihapus, bukan di-allow.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi resource-gate BG-D1/BG-D4 (eks-BL-18) ada test eksplisit>
```
