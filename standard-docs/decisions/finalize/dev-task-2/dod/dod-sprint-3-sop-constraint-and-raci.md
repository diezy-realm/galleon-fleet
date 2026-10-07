---
sprint: 3
title: DoD & PR Checklist — SOP Constraint & RACI
---

# DoD & PR Checklist — Sprint 3 (SOP Constraint & RACI)

## Gerbang Wajib

- [x] **GW-0 Hygiene** — standar.
- [x] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`BL-57,BL-58`); PR description baris pertama menyebut `Sprint 3` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [x] **GW-1 Rust** — `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>` (Windows: `$env:CARGO_TARGET_DIR="target/agent"`); **nol `unwrap()/expect()` baru**; `BL-57` (`SopConstraint` baru) harus additive murni — test existing `SopStep`/`SopRun` TIDAK berubah hasil (regresi nol).
- [x] **GW-5 Kontrak** — kalau `sop::types` punya breaking change struktur serialisasi (JSON/proto), dokumen kontrak terkait diperbarui di PR yang sama.

## Acceptance Criteria Spesifik Sprint Ini

- [x] `BL-06`/`BL-08`: contoh role di PR description WAJIB menyertakan minimal 1 role non-engineering (Marketing/Finance/dst) — PR hanya dengan contoh Backend/Frontend/QA DITOLAK sebagai tidak memenuhi fundamental produk "AI Companion lintas departemen".
- [x] `BL-58`: test membuktikan "Lead" (atau role apa pun yang di-restrict) BENAR-BENAR diblokir hook `preToolUse` saat mencoba `ToolKind::FileWrite`/`Shell` — assert `HookResult::Cancel` dikembalikan, bukan hanya warning log.
- [x] `BL-60`: contoh SOP "Engineering Feature Delivery" dieksekusi via `clawcrew-eval` mode `Replay`, hasil tersimpan sebagai fixture — ini jadi test-case pertama Eval-01.
- [x] `BL-61`: contoh SOP Marketing dieksekusi dengan `ConstraintKind` YANG SAMA tanpa modifikasi kode — PR yang menambah special-case kode untuk Marketing DITOLAK (melanggar department-agnostic).
- [x] `Eval-01`: check CI baru untuk `clawcrew-eval` terkonfirmasi ADVISORY (non-blocking) — PR yang membuatnya blocking tanpa persetujuan eksplisit Owner DITOLAK (lihat `decisions/sop-eval-telemetry-loop-design.md`).

## Checklist Review Principle

1. Layer Flow (`SopStep`) tidak disentuh oleh perubahan Layer Constraint — diff yang menyentuh KEDUA layer di satu PR dipertanyakan (seharusnya terpisah sesuai desain two-layer).
2. Enforcement constraint reuse hook existing (`preToolUse`) — PR yang membangun mekanisme enforcement baru dari nol dipertanyakan (ponytail: reuse > tambah).
3. `SopCounters`/`RunSnapshot` (Eval-02) expose read-only — tidak ada mutasi state SOP lewat endpoint metrics baru.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi contoh non-engineering ADA di PR BL-06/08/61>
```
