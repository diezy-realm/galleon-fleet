# Galleon Fleet — Panduan Deployment & Hardening Produksi (PL-C2)

Panduan operasional dan arsitektur deployment untuk Galleon Fleet pada infrastruktur lokal dan VPS/Cloud.

---

## 1. Peran Arsitektur: Nginx vs `galleonrelay`

| Komponen | Lingkup & Peran Utama | Protokol / Target |
|---|---|---|
| **Nginx** | Public Ingress Gateway & TLS Termination | HTTP/HTTPS (443), WebSockets, SSE ke Go Engine (`127.0.0.1:9090`) |
| **`galleonrelay`** | Peer-to-Peer / Cross-Host Encrypted Bridge | E2EE Transport, tunnel komunikasi antarmesin yang terisolasi di balik NAT |

- **Nginx** digunakan saat menyajikan antarmuka pengguna Web Dashboard, autentikasi berbasis browser, dan terminasi sertifikat HTTPS publik (Let's Encrypt).
- **`galleonrelay`** digunakan untuk menghubungkan sub-agent, node worker, atau perangkat mobile pendamping tanpa mengekspos port database atau gateway secara publik.

---

## 2. Model Ancaman & Hardening (Threat Model)

1. **Prinsip Least Privilege**:
   - Daemon `galleon` dan engine Go **TIDAK BOLEH** dijalankan sebagai user `root` atau `Administrator`. Gunakan dedicated service user (misal: `galleon`).
2. **Default-Closed Firewall**:
   - Hanya port 80 dan 443 yang boleh dibuka ke internet publik.
   - Port internal `:9090` (Go engine) dan `:8080` (Rust gateway) **HARUS** di-bind ke loopback interface `127.0.0.1`.
3. **Proteksi Autentikasi**:
   - Semua panggilan terminal dan mutasi tool execution wajib melewati header otentikasi Bearer Token (`GALLEON_API_TOKEN`).
   - Rate limiting ketat diterapkan di Nginx untuk rute `/api/pairing` dan `/api/auth` (maks 5 request/detik).
4. **Sandboxing Eksekusi Tool**:
   - Tool dengan risiko `EXECUTE` dibatasi oleh `terminal_policy.go` (allowlist perintah baca, blocking `rm -rf`, `sudo`, `format`).

---

## 3. Langkah Deployment VPS / Linux

### Langkah 1: Siapkan User dan Direktori
```bash
sudo useradd -r -s /bin/false -m -d /var/lib/galleon galleon
sudo mkdir -p /etc/galleon /var/log/galleon
sudo chown -R galleon:galleon /var/lib/galleon /var/log/galleon
```

### Langkah 2: Konfigurasi Service Systemd
Salin template `nginx/galleon.conf` ke `/etc/nginx/sites-available/galleon.conf` dan buat symlink ke `sites-enabled`.
Jalankan service manager:
```bash
python3 scripts/service/galleon_service.py install
```

### Langkah 3: Verifikasi Status
```bash
python3 scripts/service/galleon_service.py status
curl -i https://your-domain.example.com/api/health
```
