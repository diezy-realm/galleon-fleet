# 04 — FE-Desktop (`apps/tauri/`: Tauri v2, shell OS saja)

Konvensi: `tech-stack/tauri-conventions.md`, ADR 0003. Perintah IPC: `async fn` → `Result<T, String>`, proxy ke Go `:9090`. Gerbang: GW-3, GW-4, GW-8.
(`apps/tauri/` = bekas `apps/tauri-2/`.)

## A. P0 — Perbaikan Rusak & Keamanan

- [x] **FD-A1 · U-01 + P-01 · PR-03 (P0, S) Perbaiki `frontendDist`** [Selesai PR-03] `tauri.conf.json` diperbarui: `"frontendDist": "../../web/dist"`. Build frontend `npm run build` dan packaging sinkron.
- [x] **FD-A1b · P-01 · PR-03 (P0, S) ⚑ Perbaiki workspace Cargo.** [Selesai PR-03] Opsi (a) diterapkan: `exclude = ["apps/tauri"]` pada root `Cargo.toml`. `cargo metadata --no-deps` hijau di root dan `apps/tauri`; `cargo check` di `apps/tauri` sukses (0 error, 0 warning).
- [x] **FD-A2 · U-06 · PR-04 (P0, S) ⚑ Hapus `execute_terminal_command`** [Selesai PR-04] Simulasi palsu `execute_terminal_command` dihapus dari `apps/tauri/src/main.rs` dan handler `invoke_handler!`. Terminal dialihkan ke Go (PR-15), sementara UI dikunci.
- [x] **FD-A3a · T-0.1 `get_system_metrics`** sudah memanggil `engine_get("system/metrics")`.
- [x] **FD-A3b · U-07 · PR-04 (P0, S) Hapus koleksi lokal berlogika** [Selesai PR-04] `get_store_path` dihapus. `get_collection` dan `save_collection` diubah mem-proxy ke Go `/api/collections/{name}` dengan sanitasi nama koleksi. `get_engine_processes` mem-proxy ke Go `/api/engine/processes`. SSOT tunggal Go ditegakkan.
- [x] **FD-A3c · U-17 · PR-22 (P1, S) Cabut plugin Tauri yang tak terpakai.** [Selesai PR-22] Plugin `shell`, `dialog`, `fs`, `notification` dicabut dari `apps/tauri/src/main.rs`, `Cargo.toml`, dan `capabilities/default.json` (deny-by-default, hanya `core:default`). `mobile.json` dan branding diselaraskan ke Galleon Fleet.
- [x] **FD-A4 · U-08 · PR-22 (P1, S) CSP & capabilities ketat:** [Selesai PR-22] `tauri.conf.json` diperketat: `'unsafe-eval'` dihapus dari CSP, `connect-src` dikunci ke `'self' http://127.0.0.1:9090 http://localhost:9090 ipc:`, `productName` dan window title diselaraskan ke "Galleon Fleet". `cargo check` di `apps/tauri/` hijau 100%.

## B. Fungsi Shell

- [ ] **FD-B1 · U-09 · PR-92 (P1, M) Sidecar desktop:** jalankan Go engine + Rust gateway (lifecycle, health, shutdown), tanpa logika bisnis.
- [ ] **FD-B2 · (ODR-01) System tray, dialog file native, window** — hanya fitur OS shell; tiap command dicatat di allowlist.

## C. Rebrand & Rilis

- [ ] **FD-C1 · N-27 · PR-85 (P1, S) `fleet-ai-desktop`→`galleon-desktop`**, `productName` "Galleon Fleet", window/tray. Pertahankan `identifier = com.galleon.sovereign` (verifikasi).
- [ ] **FD-C2 · P-12 · PR-93 (P2, L) Bundling Windows (MSI/NSIS, signing)** bersama Platform.

## Definition of Done (FE-Desktop)

- [ ] `cargo check -p <desktop>` + `cargo tauri build` · [ ] tidak ada shell/fs/koleksi lokal berlogika, tidak ada output palsu · [ ] semua IPC proxy ke Go · [ ] allowlist deny-by-default
