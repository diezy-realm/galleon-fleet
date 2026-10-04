# 04 — FE-Desktop (`apps/tauri/`: Tauri v2, shell OS saja)

Konvensi: `tech-stack/tauri-conventions.md`, ADR 0003. Perintah IPC: `async fn` → `Result<T, String>`, proxy ke Go `:9090`. Gerbang: GW-3, GW-4, GW-8.
(`apps/tauri/` = bekas `apps/tauri-2/`.)

## A. P0 — Perbaikan Rusak & Keamanan

- [ ] **FD-A1 · U-01 + P-01 · PR-03 (P0, S) Perbaiki `frontendDist`** (kini `../../web-2/dist`, rusak) → `../../web/dist`.
- [ ] **FD-A1b · P-01 · PR-03 (P0, S) ⚑ Perbaiki workspace Cargo.** `cargo metadata` di root gagal: *multiple workspace roots found* — `apps/tauri` ada di `members` root, tetapi `apps/tauri/Cargo.toml` punya `[workspace]` sendiri. Pilih satu: (a) standalone → `exclude = ["apps/tauri"]` di root; (b) member → hapus `[workspace]` di Tauri. Rekomendasi (a): Tauri butuh toolchain/dep GUI berbeda dan boleh di-build terpisah. Juga `edition="2021"` → 2024, komentar "tauri-2 is a standalone app" usang, nama paket `fleet-ai-desktop` → `galleon-desktop`. *Acceptance:* `cargo metadata` hijau di root dan di `apps/tauri`; `cargo check` Tauri sukses; verifikasi PR-plan memakai perintah yang valid.
- [ ] **FD-A2 · U-06 · PR-04 (P0, S) ⚑ Hapus `execute_terminal_command`** — saat ini bukan shell nyata tetapi **simulasi output palsu** (`fleet status`, `echo`, dst.) yang melanggar ADR 0003. Terminal lewat Go (BG-A4); terminal UI dinonaktifkan sementara.
- [x] **FD-A3a · T-0.1 `get_system_metrics`** sudah memanggil `engine_get("system/metrics")`.
- [ ] **FD-A3b · U-07 · PR-04 (P0, S) Hapus koleksi lokal berlogika** (`get_collection/save_collection` membaca/menulis file di `app_data_dir`) dan `get_engine_processes` (3 proses hardcode palsu); proxy ke Go (`/api/collections/*`, `/api/engine/processes`). Temuan keamanan: `name` dari frontend dipakai langsung di `format!("{}.json", name)` → **path traversal** saat menulis, plus `unwrap()` di `main.rs:~70`. Selain itu `apiClient.getCollection` dalam mode Tauri membaca file lokal dulu, sehingga SSOT ganda dengan Go.
- [ ] **FD-A3c · U-17 · PR-22 (P1, S) Cabut plugin Tauri yang tak terpakai.** `main.rs` menginisialisasi `shell`, `dialog`, `fs`, `notification` dan `capabilities/default.json` memberi `shell:allow-open`, `fs:default`, dst., padahal frontend tidak mengimpor satupun `@tauri-apps/plugin-*`. Deny-by-default: cabut sampai ada pemakai nyata. Rapikan `mobile.json` ("ClawCrew"), `productName`/title "Fleet AI" → Galleon Fleet.
- [ ] **FD-A4 · U-08 · PR-22 (P1, S) CSP & capabilities ketat:** hapus `unsafe-eval`, `connect-src` hanya `http://127.0.0.1:9090` (kini `https: ipc: ws:`), scope fs `$APPDATA/galleon/*`.

## B. Fungsi Shell

- [ ] **FD-B1 · U-09 · PR-92 (P1, M) Sidecar desktop:** jalankan Go engine + Rust gateway (lifecycle, health, shutdown), tanpa logika bisnis.
- [ ] **FD-B2 · (ODR-01) System tray, dialog file native, window** — hanya fitur OS shell; tiap command dicatat di allowlist.

## C. Rebrand & Rilis

- [ ] **FD-C1 · N-27 · PR-85 (P1, S) `fleet-ai-desktop`→`galleon-desktop`**, `productName` "Galleon Fleet", window/tray. Pertahankan `identifier = com.galleon.sovereign` (verifikasi).
- [ ] **FD-C2 · P-12 · PR-93 (P2, L) Bundling Windows (MSI/NSIS, signing)** bersama Platform.

## Definition of Done (FE-Desktop)

- [ ] `cargo check -p <desktop>` + `cargo tauri build` · [ ] tidak ada shell/fs/koleksi lokal berlogika, tidak ada output palsu · [ ] semua IPC proxy ke Go · [ ] allowlist deny-by-default
