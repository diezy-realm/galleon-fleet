---
sprint: 8
title: DoD & PR Checklist — Rebrand & Release
---

# DoD & PR Checklist — Sprint 8 (Rebrand & Release — SELALU TERAKHIR)

## Gerbang Wajib

- [x] **GW-0 Hygiene** — WAJIB ketat: PR rename massal (N-2x) HANYA mekanis (skrip + laporan), **TIDAK bercampur perubahan perilaku**. PR yang menyisipkan logic fix di tengah rename DITOLAK — pisah jadi PR terpisah.
- [x] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR) walau untuk PR rename massal — gunakan ID tiket rename yang relevan (`RB-21`, dst); multi-tiket dipisah koma tanpa spasi; PR description baris pertama menyebut `Sprint 8` eksplisit; commit di branch `feat/refactoring-finalize-01` saja (lihat README §Kebijakan Branch).
- [ ] **GW-6 Integrasi** — WAJIB untuk `PL-E1`/`PL-E2`: `cargo nextest run --workspace` + `go test -race ./...` + `npm run build` SEKALIGUS dalam satu branch `integration/wave-N`.
- [x] **GW-9 Rollback** — WAJIB untuk `RB-23` (migrasi `~/.clawcrew`→`~/.config/galleon`): jalur mundur tertulis dan DIUJI (bukan hanya didokumentasikan).

## Acceptance Criteria Spesifik Sprint Ini

- [x] **Prasyarat keras**: PR pertama sprint ini HARUS menyertakan konfirmasi Sprint 0-7 seluruhnya `[x]` — Principle MENOLAK PR rename apa pun kalau ada sprint sebelumnya yang belum tuntas (aturan "hapus/pecah sebelum rename massal").
- [ ] `RB-21`: crate rename berseri bawah→atas — PR yang rename crate level atas (`runtime`) SEBELUM crate di bawahnya selesai DITOLAK.
- [x] `PL-D1`/`FD-C2` (termasuk eks-`BL-33`/`BL-34`): signing graceful-degrade — build TIDAK gagal kalau secret absen, tapi status signing jelas di release notes (ter-sign vs tidak).
- [x] `GV-12`: Principle re-verify SEMUA `[x]` di `finalize/task-breakdown.md` DAN `dev-tasks/` memakai `testing-gate-checklist.md` — termasuk `BR-F1` yang perlu dicentang manual (duplikat `GV-07`, lihat README §Reconciliation) sebelum sprint ini ditutup resmi.
- [ ] `PL-E4`/`CO-07`: bukti end-to-end demo manual (`finalize/e2e-proof.md`) TIDAK mengandung fallback/konstanta palsu — Principle membaca bukti ini baris per baris, bukan percaya ringkasan "demo berhasil".

## Checklist Review Principle

1. `brand_gate.sh` hijau untuk SELURUH repo — bukan sampel sebagian file.
2. Tidak ada `CLAWCREW_*`/`clawcrew` tersisa di kode PRODUKSI (dokumentasi historis `docs/_archive` sudah dihapus di Sprint sebelumnya, jadi tidak relevan di sini).
3. Atribusi lisensi upstream dipertahankan di `RB-29` — Principle cross-check `LICENSE`/`NOTICE` tidak terhapus tidak sengaja oleh skrip rename.
4. `RB-30` menyinggung `finalize/` juga — pastikan `dev-tasks/` dan `dev-task-2/` (folder ini sendiri) ikut diperbarui penamaan brand-nya kalau relevan, TANPA mengubah isi tiket/status.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: konfirmasi Sprint 0-7 tuntas + GV-12 re-verify selesai termasuk BR-F1>
```

## Gerbang Akhir Rilis (RB-40, sebelum `prerelease→main`)

- [ ] `brand_gate` hijau
- [ ] `cargo nextest run --workspace` hijau
- [ ] `go test -race ./...` hijau
- [ ] `npm run build` hijau
- [ ] Build desktop (Windows/macOS/Linux) sukses
- [ ] Approval Achmad eksplisit (`prerelease→main` — satu-satunya langkah yang BUKAN wewenang Principle/Lead)
