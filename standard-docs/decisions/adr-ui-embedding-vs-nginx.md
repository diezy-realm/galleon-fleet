# ADR: Keputusan Strategi Penyajian UI — Embedded Server vs Nginx (PL-B2)

- **Status:** DITERIMA
- **Tanggal:** 2026-10-08
- **Konteks:** Tiket PL-B2 (Sprint 8) — Menentukan apakah frontend dashboard Galleon (`web/dist`) harus di-embed ke dalam binary container/server atau disajikan oleh Nginx terpisah.

---

## 1. Konteks Masalah
Galleon Fleet memiliki beberapa target distribusi:
1. **Desktop Shell** (`apps/tauri`): Aplikasi native offline-first.
2. **Single-Node Container / Developer Server**: `docker run` mandiri tanpa kebutuhan setup reverse proxy rumit.
3. **Production Cloud Deployment**: Deployment multi-node dengan load balancing, domain publik, dan proteksi TLS.

---

## 2. Keputusan Arsitektural (Hybrid Approach)

1. **Standalone Container (`Dockerfile` / Server Binary)**:
   - Dashboard statis (`web/dist`) **DI-EMBED** ke dalam container server pada port `:9090` sebagai fallback bawaan.
   - **Rasional**: Memungkinkan single-container deployment (`docker run -p 9090:9090 ghcr.io/diezy-realm/galleon-fleet`) langsung menyajikan dashboard operasional tanpa ketergantungan pada container Nginx terpisah.

2. **Production Enterprise / Multi-Tenant**:
   - Nginx diletakkan di layer terdepan (`nginx/galleon.conf`) untuk terminasi TLS, HTTP/2, HSTS, rate-limiting, dan kompresi gzip/brotli.
   - Nginx melakukan proxy pass ke loopback backend (`127.0.0.1:9090`).

3. **Desktop (Tauri v2)**:
   - Menggunakan aset lokal `web/dist` langsung via IPC webview Tauri (`apps/tauri/tauri.conf.json`).

---

## 3. Konsekuensi
- Ukuran binary container bertambah ~1-2MB karena menyertakan aset statis terkompresi `web/dist`.
- Developer lokal dan operator homelab mendapatkan pengalaman "one-click deploy" tanpa konfigurasi reverse proxy manual.
- Operator produksi tetap dapat mengoptimalkan serving aset melalui Nginx reverse proxy.
