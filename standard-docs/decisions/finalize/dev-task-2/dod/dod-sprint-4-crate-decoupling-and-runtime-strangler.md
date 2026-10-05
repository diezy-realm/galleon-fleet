---
sprint: 4
title: DoD & PR Checklist — Crate Decoupling & Runtime Strangler
---

# DoD & PR Checklist — Sprint 4 (Crate Decoupling & Runtime Strangler)

## Gerbang Wajib

- [ ] **GW-1 Rust** — standar, plus: `BR-C5`/`BR-C6`/`BR-D1` adalah PR berseri (XL) — setiap PR dalam seri HARUS lolos `cargo metadata` dependency-direction gate (`BR-A3`, sudah terkunci) sendiri, bukan menunggu seri selesai total.
- [ ] **GW-2 Go** — `BG-D1`..`BG-D5` adalah fitur BARU di Go, bukan port langsung — `go test -race ./...` WAJIB untuk setiap PR, tidak ada pengecualian "akan ditest nanti".
- [ ] **GW-6 Integrasi** — WAJIB untuk sprint ini (XL, lintas-tier): smoke e2e start engine Go + gateway Rust → `StartTurn` stream → `ExecuteNativeTool` fail-closed saat Rust mati → UI memuat; hasil tertulis di deskripsi PR.

## Acceptance Criteria Spesifik Sprint Ini

- [ ] `BG-D1`: acceptance "paritas dengan `eval` replay" dibuktikan dengan angka konkret (berapa % test case fixture yang match), bukan klaim "sudah paritas".
- [ ] `BG-D1`/`BG-D4` (eks-`BL-18`): test resource-gate pada jalur Go `spawn_subagent`→`ExecuteTurn` — kirim N spawn request melebihi cap, assert DITOLAK/DI-QUEUE (bukan dieksekusi semua tanpa batas).
- [ ] `BR-C5`: `cargo metadata` membuktikan `gateway` 426 ref → 0 ke `runtime` — angka final HARUS 0, bukan "berkurang signifikan".
- [ ] `BR-D1`: setiap modul yang di-ramping dari `clawcrew-runtime` punya PR terpisah dengan acceptance "parity Go + eval hijau" dicek SATU PER SATU, bukan di-batch.
- [ ] `BR-D4`: `rpc` + REST/WS/A2A lama dihapus HANYA setelah `BR-E3` (Sprint 6) dan `BR-C5` (sprint ini) keduanya `[x]` — PR `BR-D4` tanpa referensi dua dependency ini di description DITOLAK.

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
