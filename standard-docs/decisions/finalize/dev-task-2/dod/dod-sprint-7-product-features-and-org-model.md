---
sprint: 7
title: DoD & PR Checklist — Product Features & Org Model
---

# DoD & PR Checklist — Sprint 7 (Product Features & Org Model)

## Gerbang Wajib

- [x] **GW-0 Hygiene** — standar.
- [x] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`FW-C1,BL-43`); PR description baris pertama menyebut `Sprint 7` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [x] **GW-3 UI/Desktop** — `npm ci && npm run lint && npm run build` hijau; TS strict tanpa `any`; tidak ada `fetch` langsung ke Rust/komponen.
- [x] **GW-2 Go** — standar untuk `BG-F4`/`BG-F6`.
- [x] **GW-8 Keamanan** — WAJIB untuk seluruh `BL-40..44` (payment-lockout): ini preventif untuk kelas insiden Comet/Replit, threat-note dan test negatif tidak boleh dilewati meski "belum ada tool pembayaran nyata".

## Acceptance Criteria Spesifik Sprint Ini

- [x] `FW-C1`: layar organisasi menampilkan DATA NYATA dari `BG-F1`/`BG-F3` (Sprint 4) — PR dengan mock data statis DITOLAK, fitur ini harus hidup end-to-end.
- [x] `FW-C3`: prompt approval terminal UI terhubung ke `S2-SEC-01` (Sprint 0) — test membuktikan approval TIDAK bisa di-klik tanpa principal terverifikasi.
- [x] `BL-43`: UI Settings "Allow Agents to Make Payments" menampilkan STATE MENTAH dari backend (misal: `locked: true, unlock_expires_at: null`) — bukan ringkasan teks yang digenerate agent ("agen mengatakan sudah unlocked"). Ini acceptance OWASP ASI09, PR yang menampilkan ringkasan agent DITOLAK.
- [x] `BL-41`/`BL-42`: test eksplisit mencoba memanggil tool berkategori `payment_category` TANPA unlock, assert fail-closed DITOLAK saat registrasi tool (bukan hanya saat eksekusi).
- [x] `BL-44`: audit log Locked→Unlocked test membuktikan setiap transisi tercatat dengan timestamp + principal yang melakukan (bukan log generik tanpa identitas).

## Checklist Review Principle

1. **File-overlap check WAJIB**: `BL-41`/`BL-42` menyentuh `tool_execution.rs` yang sama dengan `BL-37`/`BL-38` (Sprint 2) — Principle memverifikasi Sprint 2 sudah merge SEBELUM approve PR Sprint 7 §C.
2. `unlock_expires_at` punya default behavior yang fail-closed (expired = locked), bukan fail-open.
3. UI organisasi (`FW-C1`) tidak expose data sensitif (budget/memory scope Crew individu) ke role yang tidak berhak — authz dicek per-endpoint.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi BL-43 state mentah (bukan ringkasan agent) + Sprint 2 sudah merge>
```
