# apps/tauri/ — standar eksekusi Desktop Shell (agy)

Berkas ini aktif otomatis saat kamu (agy) bekerja di tier `apps/tauri/` (Rust,
Tauri v2). Kamu adalah **eksekutor Desktop Shell**: tugas tier ini HANYA window,
tray, dialog OS, packaging, dan IPC (`invoke`) — **bukan** logika bisnis/domain.

Standar arsitektur 4-tier (sumber tunggal, dipakai KiroCrew & agy):

@[Arsitektur & layering galleon](../../.kiro/steering/galleon-architecture.md)

Aturan WAJIB tier ini:
- **Shell tipis.** Tauri = proxy ke Go engine (HTTP :9090), bukan tempat
  memutuskan fakta fleet. Jangan definisikan ulang risk-tier/policy/reply di
  sini dengan nilai berbeda — engine adalah SSOT; Tauri cukup teruskan.
- **Terminal/eksekusi shell WAJIB lewat Go (ODR-07):** Tauri → Go (policy) →
  Rust (sandbox). JANGAN `/api/engine/execute` untuk terminal interaktif;
  pakai Tauri IPC + portable-pty bila membangun terminal embed.
- **Rust idiom (sama seperti `crates/`):** zero `.unwrap()`/`.expect()` di jalur
  produksi; propagasi error (`Result`/`?`); borrow sebelum clone; command Tauri
  (`#[tauri::command]`) tipis, delegasi ke engine.
- **Branding:** modul/berkas BARU pakai `galleon-`, binary `galleon`.
- **Verifikasi:** `cargo check -p <crate tauri>` + `cargo nextest run` untuk yang
  diubah. Jangan push langsung ke `testing`/`master`.
