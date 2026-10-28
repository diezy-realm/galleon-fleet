# apps/tauri/ — standar eksekusi Desktop Shell (agy)

Berkas ini aktif otomatis saat kamu (agy) bekerja di tier `apps/tauri/` (Rust,
Tauri v2). Kamu adalah **eksekutor Desktop Shell**: tugas tier ini HANYA window,
tray, dialog OS, packaging, dan IPC (`invoke`) — **bukan** logika bisnis/domain.

Standar arsitektur 3-tier (sumber tunggal, dipakai KiroCrew & agy):

@[Arsitektur & layering galleon](../../.kiro/steering/galleon-architecture.md)

Aturan WAJIB tier ini:
- **Shell tipis.** Tauri = klien/proxy ke Rust Gateway HTTP/WS (`http://127.0.0.1:42617`), bukan tempat
  memutuskan fakta fleet. Jangan definisikan ulang risk-tier/policy/reply di
  sini dengan nilai berbeda — gateway/runtime Rust adalah SSOT. Penulisan kode Go dilarang sesuai ADR-0018 dan 01-boundary-policy v2.
- **Terminal/eksekusi shell:** Gunakan Tauri IPC + portable-pty atau sandboxed tools di System Core.
  Larangan penulisan/pemanggilan Go engine sesuai ADR-0018.
- **Rust idiom (sama seperti `crates/`):** zero `.unwrap()`/`.expect()` di jalur
  produksi; propagasi error (`Result`/`?`); borrow sebelum clone; command Tauri
  (`#[tauri::command]`) tipis, delegasi ke runtime/gateway Rust.
- **Branding:** modul/berkas BARU pakai `galleon-`, binary `galleon`.
- **Verifikasi:** `cargo check -p <crate tauri>` + `cargo nextest run` untuk yang
  diubah. Jangan push langsung ke `testing`/`master`.
