# Lane U — UI (`web/`) & Desktop (`apps/tauri/`)

Ref: `tech-standart/tech-stack/{web,tauri}-conventions.md`, ADR 0002/0003. UI hanya bicara ke Go :9090.

- [ ] **U-01 (P0, S) Perbaiki `frontendDist`** di `apps/tauri/tauri.conf.json` → `../../web/dist`; verifikasi `cargo tauri build`/dev. Acceptance: window menampilkan UI.
- [ ] **U-02 (P1, M) Hapus `web/server.ts` Express** dan skrip `dev=tsx watch server.ts` → `vite`. Dep: G-20, G-22. Acceptance: `npm run dev` hanya Vite; proxy `/api`→:9090.
- [ ] **U-03 (P1, M) QR pairing di UI** memakai endpoint Go (G-20); tanpa paket `qrcode` di server. Dep: G-20.
- [ ] **U-04 (P1, M) Telemetri OS dari Go** (G-22), hapus dummy/derivasi lokal di UI.
- [ ] **U-05 (P1, S) Bersihkan artefak AI Studio:** `@google/genai`, `better-sqlite3`, `metadata.json`, `bun.lock`, `web/.github/`, `web/Dockerfile`, `docker-compose.yml`, `nginx.conf`, `DEPLOYMENT.md`, `web/data/`, `web/docs/`; ganti nama paket `react-example`→`galleon-web`. Audit pemakaian dulu (pastikan tidak ada panggilan LLM langsung). Acceptance: `npm ci && npm run build` hijau.
- [ ] **U-06 (P0, S) Hapus `execute_terminal_command`** dari `apps/tauri/src/main.rs`; ganti perintah IPC `async fn → Result<T,String>` yang memanggil Go G-04. Dep: G-04 (hapus dulu, UI terminal dinonaktifkan sementara bila G-04 belum siap).
- [ ] **U-07 (P0, S) Hapus `get_system_metrics` palsu** & koleksi lokal (`get_collection/save_collection`, `get_engine_processes` yang berlogika); proxy ke Go.
- [ ] **U-08 (P1, S) CSP & capabilities ketat:** hapus `unsafe-eval`, batasi `connect-src` ke `http://127.0.0.1:9090`, scope fs `$APPDATA/galleon/*`.
- [ ] **U-09 (P1, M) Sidecar desktop:** Tauri menjalankan Go engine + Rust gateway (lifecycle, health, shutdown), tanpa logika bisnis. Dep: P-lane build sidecar.
- [ ] **U-10 (P2, M) Tipe E2:** `HarborTool`, `ProcessItem` dll. diselaraskan dengan kontrak API Go; generate dari OpenAPI bila ada.
- [ ] **U-11 (P2, M) `apiClient.ts` tunggal bertipe**; larang `fetch` langsung di komponen (lint).
- [ ] **U-12 (P1, S) Skrip kualitas:** `typecheck`, `lint` (eslint), `test` (vitest) di `web/package.json`; strict TS tanpa `any`.
- [ ] **U-13 (P2, M) Layar organisasi:** Quartermaster/Ship/Squad/Crew, memori, approval — sesuai G-23/G-24.
- [ ] **U-14 (P3, M) i18n & aksesibilitas dasar** mengikuti `web/src/lib/i18n.ts` bila ada.
