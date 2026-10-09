---
title: T-CI-04 — galleon-web npm ERESOLVE (vite vs esbuild dependency conflict)
status: open
severity: HIGH (blocking total untuk semua PR yang menyentuh web/, tapi fix sederhana)
---

# T-CI-04 — Perbaiki konflik dependency `vite`/`esbuild` di `web/`

- [ ] **T-CI-04.** Selaraskan versi `esbuild` dengan yang diminta `vite@8.3.1` (atau downgrade `vite`) di `web/package.json`.

**Masalah konkret (diverifikasi 2026-10-09 di PR #18, run `37909598782`):**
```
npm error ERESOLVE could not resolve
npm error While resolving: vite@8.3.1
npm error Found: esbuild@0.25.12
npm error Could not resolve dependency:
npm error peerOptional esbuild@"^0.27.0 || ^0.28.0" from vite@8.3.1
npm error Conflicting peer dependency: esbuild@0.28.2
```
`web/package.json` memin `esbuild@^0.25.0` secara eksplisit, tapi `vite@8.3.1` (dan `@tailwindcss/vite@4.3.3` secara transitif) butuh `esbuild@^0.27.0 || ^0.28.0`. **Tidak terkait commit remediasi dev-task-2 manapun** — pre-existing version mismatch di `package.json`/lockfile, kemungkinan `vite` baru saja di-bump tanpa menyesuaikan pin `esbuild`.

**Langkah:**
1. Baca `web/package.json` — cek versi exact `vite`, `esbuild`, `@tailwindcss/vite`, `@vitejs/plugin-react` yang dipin.
2. Tentukan arah fix (pilih salah satu, verifikasi compatibility matrix masing-masing):
   - **Opsi A:** naikkan pin `esbuild` ke `^0.27.0` atau `^0.28.0` (sesuai yang vite minta) — cek breaking changes esbuild 0.25→0.27/28 sebelum naikkan.
   - **Opsi B:** downgrade `vite` ke versi yang kompatibel dengan `esbuild@^0.25.0` — cek apakah fitur vite 8.3.1 yang baru dipakai di codebase (kalau ada) akan hilang.
3. Jalankan `npm install` lokal di `web/` setelah pin diubah — pastikan lockfile (`package-lock.json`) ikut terupdate dan ter-commit.
4. Jalankan `npm run lint && npm run typecheck && npm run build` di `web/` untuk pastikan tidak ada regresi dari perubahan versi.

**DoD:** `npm install` di `web/` sukses tanpa `--force`/`--legacy-peer-deps` (keduanya menyembunyikan masalah, bukan menyelesaikannya), `npm run build` hijau, job `galleon-web (Lint, Typecheck, Build)` SUCCESS di PR test.

**Jangan diubah:** jangan pakai `--force` atau `--legacy-peer-deps` sebagai fix permanen di CI config — itu menyembunyikan ketidakcocokan versi, bukan menyelesaikannya, dan berisiko lolos ke runtime behavior yang tidak terduga.
