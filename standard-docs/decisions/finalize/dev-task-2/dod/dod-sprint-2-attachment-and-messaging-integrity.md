---
sprint: 2
title: DoD & PR Checklist — Attachment & Messaging Integrity
---

# DoD & PR Checklist — Sprint 2 (Attachment & Messaging Integrity)

## Gerbang Wajib

- [ ] **GW-0 Hygiene** — standar.
- [ ] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`BL-52,BL-14`); PR description baris pertama menyebut `Sprint 2` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [ ] **GW-1 Rust** — `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>` (Windows: `$env:CARGO_TARGET_DIR="target/agent"`); **nol `unwrap()/expect()` baru**; plus: perubahan di `clawcrew-channels/src/orchestrator/mod.rs` HARUS menyertakan diff yang jelas terhadap commit BL-14 (Sprint 0) — reviewer memverifikasi tidak ada revert tidak sengaja terhadap ContentSafety yang baru ditambahkan.
- [ ] **GW-8 Keamanan** — khusus `BL-37`/`BL-38`: test tangkap-log membuktikan redaksi/scrub BENAR-BENAR terjadi (baca file log hasil test, assert konten sensitif TIDAK ADA), bukan hanya assert fungsi dipanggil.

## Acceptance Criteria Spesifik Sprint Ini

- [ ] `BL-52`: test baru mengirim 3 pesan rapid dengan attachment berbeda per-pesan, assert SEMUA attachment (bukan hanya pesan pertama) muncul di `turn.msg.attachments` setelah debounce.
- [ ] `BL-55`: test mengirim 15 pesan rapid 1-attachment masing-masing (total 15 > limit 10), assert count-enforcement menolak setelah agregasi — BUKAN hanya menolak di Layer 1 per-pesan (yang bisa diakali kirim banyak pesan kecil).
- [ ] `BL-37` Tahap 1: `log_tool_io=redacted` di config test, assert `tool_execution.rs` memanggil `capture_tool_input`/`capture_tool_output` (bukan `call_arguments` mentah) — grep assert di test, bukan baca manual.
- [ ] `BL-38`: kredensial sengaja disisipkan di tool input/output test, assert hilang di `runtime-trace.jsonl` hasil test.
- [ ] `BL-39`: test platform-conditional (`#[cfg(windows)]`) membuktikan ACL file log Windows setara `0o600` Unix.

## Checklist Review Principle

1. **File-overlap check WAJIB**: PR ini menyentuh `clawcrew-channels/src/orchestrator/mod.rs` — Principle memverifikasi BL-14 (Sprint 0) sudah merge dan tidak ada konflik logic ContentSafety vs debounce fix.
2. `MediaKind` baru (`BL-53`) tidak breaking existing attachment serialization — test regresi untuk `Audio|Image|Video` existing tetap lolos.
3. Resource-gate bytes (`BL-55`) punya angka konkret di PR description (berapa MB cap per bucket, bukan "reasonable limit").

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi BL-14 Sprint 0 sudah merge SEBELUM approve PR ini>
```
