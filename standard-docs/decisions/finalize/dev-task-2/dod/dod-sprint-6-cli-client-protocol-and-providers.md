---
sprint: 6
title: DoD & PR Checklist — CLI/Client Protocol & Providers
---

# DoD & PR Checklist — Sprint 6 (CLI/Client Protocol & Providers)

## Gerbang Wajib

- [ ] **GW-1 Rust** — standar.
- [ ] **GW-5 Kontrak** — WAJIB: `api-contracts/client-protocol.md` berubah di PR yang SAMA dengan perubahan server/klien (`BG-E1`/`BG-E2` dari Sprint 4 harus sudah stabil sebelum sprint ini mulai).

## Acceptance Criteria Spesifik Sprint Ini

- [ ] `BR-E3`: `spawn_owned_ephemeral_daemon` benar-benar MATI setelah retarget `galleoncode` — grep repo membuktikan nol pemanggilan tersisa, bukan "sudah tidak dipakai di jalur utama".
- [ ] `BL-30`: `erase_identity()` test membuktikan hapus LINTAS SEMUA backend memory yang ada saat ini (daftar eksplisit backend yang dicek) — bukan hanya SQLite.
- [ ] `BL-28`: rate limiting test membuktikan baseline dari `BL-15` (Sprint 5) dipakai, bukan angka arbitrary baru — PR tanpa rujukan ke baseline BL-15 dipertanyakan.
- [ ] `BL-25`: BYOC override `client_id` test membuktikan PKCE flow tetap valid secara standar OAuth (bukan hanya "berfungsi di environment saya").

## Checklist Review Principle

1. CLI/TUI (`BR-E1..E4`) tidak lagi punya jalur bypass langsung ke daemon Rust — satu-satunya jalur adalah Client Protocol Go.
2. `BL-31` (hygiene/retensi backend-agnostic) — PR scope besar ini WAJIB mulai dari evaluasi trait dulu (sesuai catatan backlog), bukan implementasi langsung tanpa desain trait tertulis.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi nol bypass langsung CLI→daemon Rust>
```
