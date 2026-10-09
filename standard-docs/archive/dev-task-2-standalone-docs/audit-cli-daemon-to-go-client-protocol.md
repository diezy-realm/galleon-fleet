# Audit: CLI, Client Protocol, dan Pemetaan Daemon Rust ke Go Gateway :9090 (BR-E1, BR-E2, BR-E3, BR-E4)

**Tickets:** BR-E1 (Audit `zerocode`/`zerorelay`), BR-E2 (CLI Command Routing), BR-E3 (TUI Retarget & Matikan `spawn_owned_ephemeral_daemon`), BR-E4 (Retarget ACP Bridge ODR-24)  
**Status:** Approved / Selesai  
**Date:** 2026-10-08  
**Prasyarat:** Sprint 4 BG-E1/BG-E2 (`api-contracts/client-protocol.md`)  

---

## 1. Executive Summary

Audit ini memetakan seluruh interaksi klien CLI, TUI (`apps/zerocode`), relay forwarder (`apps/zerorelay`), dan ACP bridge (`src/bin/clawcrew-acp-bridge.rs`) yang sebelumnya berinteraksi dengan daemon Rust via local IPC socket/named-pipe atau ephemeral process, untuk sepenuhnya dialihkan ke **Go Client Protocol** gateway pada port `:9090` (WS/JSON-RPC 2.0).

---

## 2. Pemetaan Pemanggilan Daemon Rust → Go Client Protocol :9090 (BR-E1)

Berdasarkan audit langsung pada `apps/zerocode/src/client.rs`, `apps/zerocode/src/relay_proto.rs`, `apps/zerocode/src/main.rs`, dan `apps/zerocode/src/app.rs`:

| Modul/File | Method Wire / Pemanggilan Rust Lama | Deskripsi / Peran | Status di Go Gateway `:9090` (Client Protocol) | Catatan Transisi |
| :--- | :--- | :--- | :--- | :--- |
| `client.rs:88` | `initialize` | Handshake awal sesi & UID | **Tersedia** (WS connection handshake) | Menggunakan header auth bearer / token pairing |
| `client.rs:115` | `session/new` | Inisiasi agent session baru | **Tersedia** (`session.new` / `chat`) | Dimetakan ke Session Manager di Go |
| `client.rs:116` | `session/prompt` | Mengirim prompt chat ke agent | **Tersedia** (`chat` / `run`) | Streaming token lewat WS message |
| `client.rs:117` | `session/configure` | Mengubah runtime config sesi | **Tersedia** (`session.configure`) | Diteruskan ke subagent/runtime core |
| `client.rs:118` | `session/cancel` | Membatalkan eksekusi task/agent | **Tersedia** (`session.cancel`) | Abort signal pada goroutine/task |
| `client.rs:121` | `session/approve` | Menyetujui/menolak approval gate | **Tersedia** (`approve`) | Sinkron dengan approval parity (BG-A1/A2) |
| `client.rs:125` | `status` | Mengambil status runtime/daemon | **Tersedia** (`status`) | Mengembalikan uptime, load, agent list |
| `client.rs:126` | `health` | Liveness check | **Tersedia** (`GET :9090/healthz` / RPC) | Health endpoint standar gateway |
| `client.rs:127` | `doctor/run` | Diagnostik lokal host/lingkungan | **Lokal** (Tetap dijalankan di host CLI) | Tidak memerlukan gateway Go |
| `client.rs:136` | `memory/list` & `memory/search` | Query memori/konteks percakapan | **Tersedia** via Go Storage / Memory bridge | Diteruskan ke Memory backend |
| `client.rs:148-160` | `sops/*` | Manajemen & eksekusi SOP | **Tersedia** via SOP Engine `:9090` | Terintegrasi dengan ODR-09 |
| `relay_proto.rs` | Protocol blind forwarding | Terowongan TLS outer + inner mTLS | **Tersedia** via outer TLS WS proxy Go | Forwarder ciphertext blind |
| `main.rs:968` | `spawn_owned_ephemeral_daemon` | Auto-spawn daemon Rust lokal | **DIMATIKAN (BR-E3)** | **Nol pemanggilan**; fail-closed ke prompt :9090 |
| `app.rs:1472` | `spawn_owned_ephemeral_daemon` | Auto-respawn daemon lokal | **DIMATIKAN (BR-E3)** | Hanya retry reconnect ke target yang ada |

---

## 3. CLI Routing Policy (BR-E2)

Perintah CLI `galleon` / `clawcrew` dipisahkan secara tegas antara operasi control-plane/runtime (dialihkan ke Go gateway `:9090`) dan operasi utility lokal:

```
[CLI User Command]
        │
        ├── chat / run / approve / status ──► [Go Client Protocol :9090 (WebSocket / JSON-RPC)]
        │
        └── service / doctor / onboard / vault ──► [Local Host Execution (Direct CLI Engine)]
```

### Rincian Routing:
1. **Remote / Go Gateway `:9090`:**
   - `chat`: Membuka sesi interaktif real-time dengan agent via WS.
   - `run`: Mengirim headless prompt / execution batch.
   - `approve`: Memberikan keputusan operator (approve/deny) ke checkpoint runtime.
   - `status`: Memeriksa kondisi daemon, running tasks, dan antrean.
2. **Local Host Execution:**
   - `service`: Manajemen service manager OS (systemd, Windows service, launchd) untuk start/stop daemon.
   - `doctor`: Diagnostik filesystem lokal, permissions, PATH, dan ketersediaan tool.
   - `onboard`: Setup wizard lokal (file konfigurasi, pembuatan direktori awal, key generation).
   - `vault`: Akses keystore/OS keyring lokal untuk enkripsi kredensial host.

---

## 4. Eliminasi `spawn_owned_ephemeral_daemon` (BR-E3)

### Masalah Arsitektur Lama:
Sebelumnya, jika `zerocode` TUI gagal terhubung ke daemon socket lokal, ia memanggil `spawn_owned_ephemeral_daemon(&config_dir, socket)`, yang mengeksekusi sub-proses daemon Rust sendiri secara diam-diam. Hal ini menciptakan split-brain dengan gateway Go `:9090` dan menyembunyikan status kegagalan service yang sebenarnya.

### Solusi & Implementasi:
1. **Hapus Definisi & Call-Site:** Seluruh referensi `spawn_owned_ephemeral_daemon` di `main.rs` dan `app.rs` dihapus secara permanen.
2. **Fail-Closed dengan Pesan Actionable:** Jika koneksi ke socket/gateway gagal, TUI langsung menampilkan error instruktif kepada operator untuk memastikan daemon / gateway Go di port `:9090` sudah berjalan.
3. **Reconnect Semantics:** Reconnect loop pada TUI hanya mencoba menyambung kembali ke target yang telah ditentukan tanpa memicu pembuatan proses baru.

---

## 5. Retargeting ACP Bridge (BR-E4 & ODR-24)

### Status Aktual Source:
- ACP Bridge berada di `src/bin/clawcrew-acp-bridge.rs` (bagian dari binary package dengan feature `acp-bridge`).
- Bridge menghubungkan IO `stdin`/`stdout` ke WebSocket endpoint `/acp`.
- **Target Endpoint:** Diarahkan secara default ke URL WebSocket Go gateway pada `http://127.0.0.1:9090/acp` (atau `ws://127.0.0.1:9090/acp`).
- Skema autentikasi menggunakan token bearer yang diperoleh dari pairing code (`/pair` dan `/pair/code`).

---

## 6. Verifikasi & Kesimpulan

1. Tidak ada bypass langsung CLI/TUI ke internal engine Rust — seluruh komunikasi interaktif melewati Go Client Protocol contract.
2. `spawn_owned_ephemeral_daemon` dieliminasi total dari kode sumber.
3. Kepatuhan penuh terhadap `api-contracts/client-protocol.md` dan ODR-24.
