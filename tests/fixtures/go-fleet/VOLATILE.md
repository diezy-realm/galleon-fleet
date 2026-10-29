# Volatile Keys in Go Fleet Contract Fixtures

> **Status:** Ratified Reference for Contract Testing (TASK-065, SPEC-012, BL-087)  
> **Target Consumer:** TASK-081 (`feat/fleet-core-mount-contract-tests`) & Galleon Contract Suite

Berkas ini mencatat daftar field/kunci yang bernilai **volatil** (berubah antar-eksekusi atau bergantung pada runtime host, waktu, dan PID proses) pada respons JSON fixture di direktori `tests/fixtures/go-fleet/`.

Saat melakukan pengujian kepatuhan kontrak (contract testing di TASK-081 / Sprint 11), test suite **WAJIB mengabaikan (mask/ignore)** kunci-kunci di bawah ini saat membandingkan respons gateway Rust terhadap fixture rekaman Go ini.

---

## 1. Daftar Kategori Kunci Volatil Global

| Kategori | Nama Kunci / Pola | Contoh Nilai Aktual | Alasan Volatilitas |
|---|---|---|---|
| **Waktu / Tanggal** | `timestamp`, `createdAt`, `generatedAt`, `date` | `"2026-10-29T20:39:09+07:00"`, `"Thursday, 29 October 2026"` | Waktu eksekusi jam dinding saat request dipanggil |
| **Durasi & Uptime** | `uptime`, `duration` | `"1.92893577s"`, `"0h 0m"`, `"99.704µs"` | Uptime daemon Go atau durasi pengukuran eksekusi perintah |
| **Identitas Proses & OS** | `pid`, `activeProcesses`, `active_threads` | `3065152`, `7`, `4` | PID proses daemon pada OS host dan jumlah goroutine/thread |
| **Penggunaan Memori Host** | `memoryUsedMB`, `memoryUsage.usedMB`, `memoryUsage.percent`, `memory_db_mb`, `memory` | `1.185997`, `2` | Alokasi heap/RSS memori dinamis OS saat request ditangani |
| **ID Transaksi / Entitas Dinamis** | `id` (pada snapshot & audit), `auditId` | `"snap-1793281022"`, `"audit-ae8d73954896bec3"` | ID acak atau berbasis timestamp yang di-generate baru per request |
| **Checksum Turunan** | `checksum` (pada snapshot POST) | `"sha256:76f224e545"` | Hash SHA-256 yang memuat metadata snapshot ber-timestamp |
| **Host/Port URL QR Code** | `url`, `dataUrl` | `"http://127.0.0.1:9092"`, `"data:image/png;base64,..."` | Port binding dinamis saat daemon berjalan |
| **Path Sistem Sementara** | `detail` (pada diagnostic item snap) | `"... (/tmp/galleon-contract-data-X2GfV0/data)"` | Path direktori sementara (`mktemp`) data engine |

---

## 2. Pemetaan Kunci Volatil per Endpoint Fixture

### `GET_api_fleet_metrics.json`
- `memoryUsedMB`: Angka float penggunaan memori runtime Go.

### `GET_api_system_metrics.json`
- `uptime`: String format durasi uptime daemon Go (`"Xh Ym"`).
- `activeProcesses`: Jumlah goroutine runtime aktif (`runtime.NumGoroutine()`).
- `memoryUsage.usedMB`: Memori terpakai dalam MB.
- `memoryUsage.percent`: Persentase memori.
- `memory_db_mb`: Nilai alokasi memori internal.

### `GET_api_system_executive_briefing.json`
- `generatedAt`: Timestamp RFC3339 pembuatan laporan eksekutif.
- `date`: String representasi tanggal lokal (mis. `"Thursday, 29 October 2026"`).

### `GET_api_network_qrcode.json`
- `url`: URL target yang mencantumkan port binding daemon.
- `dataUrl`: Base64 PNG QR code yang mengkodekan `url` di atas.

### `GET_api_diagnostics.json`
- `[].detail`: Pada elemen komponen Disaster Recovery Daemon (`id: "d-5"`), field `detail` memuat path absolut direktori sementara (`/tmp/.../data`).

### `GET_api_snapshots.json`
- `[].createdAt`: Waktu relatif saat snapshot diinisialisasi terhadap waktu sekarang (`time.Now().Add(...)`).

### `POST_api_snapshots.json`
- `id`: ID snapshot yang dihasilkan menggunakan timestamp detik (`snap-<unix_seconds>`).
- `createdAt`: Timestamp ISO/RFC3339 snapshot baru dibuat.
- `checksum`: SHA-256 turunan dari payload snapshot.

### `GET_api_engine_processes.json`
- `[].pid`: PID OS dari proses Go Agent Engine saat berjalan.
- `[].memory`: Penggunaan memori proses dalam MB.

### `POST_api_terminal_exec.json`
- `duration`: Waktu durasi eksekusi terminal (mis. `"99.704µs"`).
- `auditId`: UUID/hash audit transaksi unik (mis. `"audit-ae8d73954896bec3"`).

### `GET_api_v1_audit_trail.json`
- `[].id`: ID audit log unik per kejadian eksekusi.
- `[].timestamp`: Timestamp RFC3339 pencatatan audit log.
- `[].duration`: Durasi eksekusi perintah yang diaudit.
