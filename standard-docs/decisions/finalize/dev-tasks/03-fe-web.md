# 03 — FE-Web (`web/`: React 19, TS 5.7+ strict, Vite, Tailwind 4)

Konvensi: `tech-stack/web-conventions.md`, ADR 0002 (zero backend logic). Gerbang: GW-3. Semua data lewat Go `:9090`.
(`web/` = bekas `web-2/`; path lama di dokumen historis dibaca sebagai `web/`.)

## A. Bersihkan Artefak & Server Lama

- [ ] **FW-A0 · U-15 · PR-14 (P1, S) Deklarasikan `@tauri-apps/api` di `web/package.json`.** Saat ini hanya di `package.json` root (ter-hoist; `npm run build` dan `lint` hijau), sehingga web tidak mandiri. Pindahkan ke dependensi `web` dan pastikan `tauriBridge` dimuat dinamis agar mode browser tidak membawanya.
- [ ] **FW-A1 · U-05 · PR-14 (P1, S) Bersihkan artefak AI Studio:** `@google/genai`, `better-sqlite3`, `qrcode`, `@types/qrcode` (`package.json`), `metadata.json`, `bun.lock`, `web/.github/`, `web/Dockerfile`. Audit (2026-10-04): `qrcode` hanya dipakai `RemoteAccessModal.tsx` (QR kini dari Go → ganti `<img>`/data URL, lalu hapus dep); `@google/genai` dan `better-sqlite3` tidak diimpor di `src/` maupun `server.ts`.
- [ ] **FW-A2a · U-16 · PR-04 (P0, S) ⚑ Matikan `POST /api/engine/execute` di `server.ts` sekarang.** Menjalankan `powershell -Command`/`sh -c` dari body request; gerbangnya menerima header `x-tauri-environment: true` yang bisa dipalsukan siapa pun (bypass token). Lebih berbahaya daripada simulasi Tauri. Hapus endpoint (terminal lewat Go BG-A4). Hapus pula `/api/providers/ollama/*` dan `/api/engine/processes` di `server.ts` (duplikat milik Go/Rust).
- [ ] **FW-A2 · U-02 · PR-70 (P1, M) Hapus `web/server.ts` (Express)** (masih menghitung memori via `os.totalmem`); skrip `dev` `tsx watch server.ts` → `vite`. Dep: BG-C4b. *Acceptance:* `npm run dev` hanya Vite.
- [x] **FW-A3 · U-03 (sebagian) · T-2.3 QR dihapus dari server Express;** UI memakai `apiClient.getNetworkQrCode()` → Go.
- [ ] **FW-A4 · U-03/U-04 · PR-70 (P1, M) QR pairing berotorisasi & telemetri OS dari Go;** hapus derivasi lokal tersisa. Dep: BG-C2, BG-C4b.
- [ ] **FW-A5 · (turunan T-1.3) · PR-14 (P2, S) Hapus `src/utils/seedData.ts` yatim** (store sudah hydrate dari API, file tidak diimpor); pastikan `hydrateSeedData` diganti nama netral (`hydrateFleet`).

## B. Kualitas & Kontrak

- [ ] **FW-B1 · U-12 · PR-12 (P1, S) Skrip kualitas:** saat ini hanya `lint = tsc --noEmit`; tambah `typecheck`, `eslint`, `test` (vitest); TS strict tanpa `any`.
- [ ] **FW-B2 · U-11 · PR-71 (P2, M) `apiClient.ts` tunggal bertipe;** lint larang `fetch` langsung di komponen.
- [ ] **FW-B3 · U-10 · PR-71 (P2, M) Tipe E2** (`HarborTool`, `ProcessItem`, …) selaras kontrak API Go; sinkron `finalize/route-map.md`. (T-1.1/1.2 sudah memperbaiki tipe dasar.)

## C. Fitur Produk

- [ ] **FW-C1 · U-13 · PR-72 (P2, L) Layar organisasi:** Quartermaster/Ship/Squad/Crew, memori, approval, audit/policy read-only. Dep: BG-F1, BG-F3.
- [ ] **FW-C2 · U-14 · PR-98 (P3, M) i18n & aksesibilitas dasar.**
- [ ] **FW-C3 · (ODR-07) Terminal UI** memakai `POST /api/terminal/exec` setelah BG-A4; tampilkan prompt approval.
- [ ] **FW-C4 · (G-20) Halaman perangkat terhubung:** daftar perangkat, revoke, QR pairing.
- [ ] **FW-C5 · (G-24) UI persetujuan `MemoryProposal`** (learning by consent). Dep: BG-F4.

## D. Rebrand UI

- [ ] **FW-D1 · N-26 · PR-85 (P1, S) `react-example`→`galleon-web`**, "Fleet AI"→"Galleon Fleet", migrasi key localStorage.

## Selesai (arsip)

- [x] T-1.1 tipe `ToolDefinition` · [x] T-1.2 shape `ProcessItem` · [x] T-1.3 store hydrate dari API · [x] T-3.1 peta kontrak (`route-map.md`)

## Definition of Done (FE-Web)

- [ ] `npm ci && npm run lint && npm run build` hijau · [ ] TS strict, tanpa `any` · [ ] tidak ada `fetch` langsung ke Rust/komponen · [ ] tanpa style inline (Tailwind)
