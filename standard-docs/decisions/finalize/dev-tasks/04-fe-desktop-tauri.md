# 04 — FE-Desktop (`apps/tauri/`: Tauri v2, shell OS saja)

Konvensi: `tech-stack/tauri-conventions.md`, ADR 0003. Perintah IPC: `async fn` → `Result<T, String>`, proxy ke Go `:9090`. Gerbang: GW-3, GW-4, GW-8.
(`apps/tauri/` = bekas `apps/tauri-2/`.)

## A. P0 — Perbaikan Rusak & Keamanan

- [ ] **FD-A1 · U-01 + P-01 · PR-03 (P0, S) Perbaiki `frontendDist`** (kini `../../web-2/dist`, rusak) → `../../web/dist`; `cargo check -p fleet-ai-desktop`; workspace Cargo hijau, `Cargo.lock` konsisten.
- [ ] **FD-A2 · U-06 · PR-04 (P0, S) ⚑ Hapus `execute_terminal_command`** — saat ini bukan shell nyata tetapi **simulasi output palsu** (`fleet status`, `echo`, dst.) yang melanggar ADR 0003. Terminal lewat Go (BG-A4); terminal UI dinonaktifkan sementara.
- [x] **FD-A3a · T-0.1 `get_system_metrics`** sudah memanggil `engine_get("system/metrics")`.
- [ ] **FD-A3b · U-07 · PR-04 (P0, S) Hapus koleksi lokal berlogika** (`get_collection/save_collection` membaca/menulis file lokal) dan logika `get_engine_processes`; proxy ke Go (`/api/collections/*`, `/api/engine/processes`).
- [ ] **FD-A4 · U-08 · PR-22 (P1, S) CSP & capabilities ketat:** hapus `unsafe-eval`, `connect-src` hanya `http://127.0.0.1:9090` (kini `https: ipc: ws:`), scope fs `$APPDATA/galleon/*`.

## B. Fungsi Shell

- [ ] **FD-B1 · U-09 · PR-92 (P1, M) Sidecar desktop:** jalankan Go engine + Rust gateway (lifecycle, health, shutdown), tanpa logika bisnis.
- [ ] **FD-B2 · (ODR-01) System tray, dialog file native, window** — hanya fitur OS shell; tiap command dicatat di allowlist.

## C. Rebrand & Rilis

- [ ] **FD-C1 · N-27 · PR-85 (P1, S) `fleet-ai-desktop`→`galleon-desktop`**, `productName` "Galleon Fleet", window/tray. Pertahankan `identifier = com.galleon.sovereign` (verifikasi).
- [ ] **FD-C2 · P-12 · PR-93 (P2, L) Bundling Windows (MSI/NSIS, signing)** bersama Platform.

## Definition of Done (FE-Desktop)

- [ ] `cargo check -p <desktop>` + `cargo tauri build` · [ ] tidak ada shell/fs/koleksi lokal berlogika, tidak ada output palsu · [ ] semua IPC proxy ke Go · [ ] allowlist deny-by-default
