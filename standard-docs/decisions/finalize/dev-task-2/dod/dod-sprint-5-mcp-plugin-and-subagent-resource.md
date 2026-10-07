---
sprint: 5
title: DoD & PR Checklist — MCP/Plugin Governance & Sub-Agent Resource
---

# DoD & PR Checklist — Sprint 5 (MCP/Plugin Governance & Sub-Agent Resource)

## Gerbang Wajib

- [x] **GW-0 Hygiene** — standar.
- [x] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`BL-19,BL-23`); PR description baris pertama menyebut `Sprint 5` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [x] **GW-1 Rust** — `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>` (Windows: `$env:CARGO_TARGET_DIR="target/agent"`); **nol `unwrap()/expect()` baru**.
- [x] **GW-8 Keamanan** — WAJIB untuk `BL-19`/`BL-20`/`BL-21`/`BL-23`: threat-note eksplisit per PR (permukaan baru default tertutup); test negatif (akses di luar scope → ditolak).

## Acceptance Criteria Spesifik Sprint Ini

- [x] `BL-15`: test spawn sub-agent melebihi cap, assert DITOLAK/DI-QUEUE dengan pesan jelas (bukan crash/OOM) — ini akar masalah crash yang dilaporkan Owner, PR tanpa test reproduksi crash ASLI yang sekarang tertutup dipertanyakan.
- [x] `BL-16`: metrics `loop_detector_circuit_breaker` TERLIHAT di Tasks-view — screenshot/demo di PR description, bukan hanya endpoint API tanpa UI.
- [x] `BL-19`: MCP stdio child process test coba akses file/network di luar scope, assert DITOLAK — setara level sandboxing dengan WASM plugin yang sudah ada (dicek berdampingan, bukan asumsi "sudah cukup").
- [x] `BL-23`: `PluginPermission::FileRead/Write` wiring diverifikasi dengan test yang BENAR-BENAR mencoba read/write di luar permission granted, assert DITOLAK.
- [x] `BR-F2`: WIT v1 dan v0 hidup BERDAMPINGAN — test membuktikan plugin v0 existing TIDAK breaking setelah v1 ditambahkan.

## Checklist Review Principle

1. `BL-24` TIDAK muncul di PR manapun sprint ini — kalau ada PR yang mengimplementasikan governance MCP fleet-wide, Principle menolak dan eskalasi ke Lead (butuh keputusan Owner eksplisit dulu, exploratory status belum berubah).
2. Rigor keamanan MCP (`BL-19`) dibandingkan eksplisit dengan WASM plugin sandboxing di PR description — tidak boleh ada celah "MCP lebih longgar karena alasan teknis" tanpa justifikasi risk-based tertulis.
3. Resource-gate (`BL-15`) punya angka cap konkret dan rationale-nya (kenapa angka ini, bukan angka lain) di PR description.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi BL-24 TIDAK diimplementasikan tanpa keputusan Owner>
```
