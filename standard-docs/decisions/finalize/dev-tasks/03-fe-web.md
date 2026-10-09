# 03 — FE-Web (`web/`: React 19, TS 5.7+ strict, Vite, Tailwind 4)

Konvensi: `tech-stack/web-conventions.md`, ADR 0002 (zero backend logic). Gerbang: GW-3. Semua data lewat Go `:9090`.
(`web/` = bekas `web-2/`; path lama di dokumen historis dibaca sebagai `web/`.)

## A. Bersihkan Artefak & Server Lama

- [x] **FW-A0 · U-15 · PR-14 (P1, S) Deklarasikan `@tauri-apps/api` di `web/package.json`.** [Selesai PR-14] `@tauri-apps/api: ^2.0.0` dideklarasikan langsung di `web/package.json`; invoke Tauri dimuat dinamis (dynamic import `invokeTauri`), browser bundle tidak memuat `@tauri-apps/api` saat startup.
- [x] **FW-A1 · U-05 · PR-14 (P1, S) Bersihkan artefak AI Studio:** [Selesai PR-14] `@google/genai`, `better-sqlite3`, `qrcode`, `@types/qrcode` dicopot dari `package.json` & lockfile; `metadata.json`, `bun.lock`, `web/.github/`, `web/Dockerfile` dihapus via `git rm`; QR code di `RemoteAccessModal.tsx` menggunakan `apiClient.getNetworkQrCode()`; nama paket diganti ke `galleon-web`.
- [x] **FW-A2a · U-16 · PR-04 (P0, S) ⚑ Matikan `POST /api/engine/execute` di `server.ts` sekarang.** [Selesai PR-04] RCE endpoint `POST /api/engine/execute` beserta middleware token/gate dihapus dari `web/server.ts`. Route duplikat `/api/providers/ollama/*` dan `/api/engine/processes` dihapus (sekarang di-proxy langsung ke Go :9090). Eksekusi terminal di `apiClient.ts` dikunci pending Go Policy Gateway (PR-15).
- [ ] **FW-A2 · U-02 · PR-70 (P1, M) Hapus `web/server.ts` (Express)** (masih menghitung memori via `os.totalmem`); skrip `dev` `tsx watch server.ts` → `vite`. Dep: BG-C4b. *Acceptance:* `npm run dev` hanya Vite.
- [x] **FW-A3 · U-03 (sebagian) · T-2.3 QR dihapus dari server Express;** UI memakai `apiClient.getNetworkQrCode()` → Go.
- [ ] **FW-A4 · U-03/U-04 · PR-70 (P1, M) QR pairing berotorisasi & telemetri OS dari Go;** hapus derivasi lokal tersisa. Dep: BG-C2, BG-C4b.
- [x] **FW-A5 · (turunan T-1.3) · PR-14 (P2, S) Hapus `src/utils/seedData.ts` yatim** [Selesai PR-14] File `src/utils/seedData.ts` dihapus via `git rm`; `hydrateFleet` ditambahkan di `fleetStore.ts` dengan alias backward `hydrateSeedData`.

## B. Kualitas & Kontrak

- [x] **FW-B1 · U-12 · PR-12 (P1, S) Skrip kualitas:** [Selesai PR-12] Skrip `typecheck` (`tsc --noEmit`), `test` (`node --test`), `lint`, `build` ditambahkan di `web/package.json`; TS strict mode; CI workflow `web-check.yml` dibuat.
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
