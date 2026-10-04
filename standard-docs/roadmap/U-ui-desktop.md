# Lane U — UI (`web/`) & Desktop (`apps/tauri/`)

Ref: `tech-standart/tech-stack/{web,tauri}-conventions.md`, ADR 0002/0003. UI hanya bicara ke Go :9090.

- [x] **U-01 (P0, S) Perbaiki `frontendDist`** [Selesai PR-03] `frontendDist` diubah ke `../../web/dist`; root `Cargo.toml` ditambahkan `exclude = ["apps/tauri"]`; `cargo metadata` dan `cargo check` hijau.
- [ ] **U-02 (P1, M) Hapus `web/server.ts` Express** dan skrip `dev=tsx watch server.ts` → `vite`. Dep: G-20, G-22. Acceptance: `npm run dev` hanya Vite; proxy `/api`→:9090.
- [ ] **U-03 (P1, M) QR pairing di UI** memakai endpoint Go (G-20); tanpa paket `qrcode` di server. Dep: G-20.
- [ ] **U-04 (P1, M) Telemetri OS dari Go** (G-22), hapus dummy/derivasi lokal di UI.
- [x] **U-05 (P1, S) Bersihkan artefak AI Studio:** [Selesai PR-14] `@google/genai`, `better-sqlite3`, `qrcode`, `@types/qrcode` dicopot; `metadata.json`, `bun.lock`, `web/.github/`, `web/Dockerfile` dihapus via `git rm`; QR code di `RemoteAccessModal.tsx` menggunakan `apiClient.getNetworkQrCode()`; nama paket diganti `react-example`→`galleon-web`. `npm run lint` & `npm run build` hijau.
- [x] **U-06 (P0, S) Hapus `execute_terminal_command`** [Selesai PR-04] Simulasi palsu `execute_terminal_command` dihapus dari `apps/tauri/src/main.rs`. Terminal UI dikunci pending Go Policy Gateway (PR-15).
- [x] **U-07 (P0, S) Hapus `get_system_metrics` palsu** & koleksi lokal [Selesai PR-04] `get_store_path` dihapus; `get_collection`/`save_collection` dan `get_engine_processes` mem-proxy ke Go :9090 dengan sanitasi path.
- [x] **U-08 (P1, S) CSP & capabilities ketat:** [Selesai PR-22] `'unsafe-eval'` dihapus dari CSP, `connect-src` dikunci ke `'self' http://127.0.0.1:9090 http://localhost:9090 ipc:`, `productName` dan window title diselaraskan ke Galleon Fleet.
- [ ] **U-09 (P1, M) Sidecar desktop:** Tauri menjalankan Go engine + Rust gateway (lifecycle, health, shutdown), tanpa logika bisnis. Dep: P-lane build sidecar.
- [ ] **U-10 (P2, M) Tipe E2:** `HarborTool`, `ProcessItem` dll. diselaraskan dengan kontrak API Go; generate dari OpenAPI bila ada.
- [ ] **U-11 (P2, M) `apiClient.ts` tunggal bertipe**; larang `fetch` langsung di komponen (lint).
- [ ] **U-12 (P1, S) Skrip kualitas:** `typecheck`, `lint` (eslint), `test` (vitest) di `web/package.json`; strict TS tanpa `any`.
- [ ] **U-13 (P2, M) Layar organisasi:** Quartermaster/Ship/Squad/Crew, memori, approval — sesuai G-23/G-24.
- [ ] **U-14 (P3, M) i18n & aksesibilitas dasar** mengikuti `web/src/lib/i18n.ts` bila ada.
- [x] **U-15 (P1, S) Deklarasikan `@tauri-apps/api` di `web/package.json`** [Selesai PR-14] `@tauri-apps/api: ^2.0.0` dideklarasikan di `web/package.json`; invoke Tauri dimuat dinamis (dynamic import `invokeTauri`), bundle mandiri dan tidak membocorkan inisialisasi IPC ke runtime browser.
- [x] **U-16 (P0, S) ⚑ Matikan `POST /api/engine/execute` di `web/server.ts`** [Selesai PR-04] Endpoint RCE shell dan middleware dihapus dari `web/server.ts`. Route duplikat `/api/providers/ollama/*` dan `/api/engine/processes` dihapus.
- [x] **U-17 (P1, S) Cabut plugin Tauri tak terpakai** [Selesai PR-22] Plugin `shell`, `dialog`, `fs`, `notification` dicabut dari `main.rs`, `Cargo.toml`, dan `capabilities/default.json` (deny-by-default, hanya `core:default`); `mobile.json` diselaraskan ke Galleon Fleet.
