# ADR-0018 — Arsitektur Bahasa Tunggal Rust untuk `galleon-fleet`

- **Status:** Accepted
- **Tanggal:** 2026-10-29
- **Pengusul:** Senior Lead Engineer (disusun asisten AI atas mandat Owner)
- **Konteks:** `galleon-fleet` — transisi dari 4-tier (Tauri / Rust / Go / Web) ke 3-tier (Tauri / Rust / Web)
- **Terkait:** ODR-2026-10-29 (ODR-26…ODR-33), ADR-0001 (LLM SSOT Rust — tetap), ADR-0002 (diamandemen: backend = Rust gateway), ADR-0003 (tetap: metrik nyata, sumber kini Rust), ADR-005 (Superseded), `01-boundary-policy.md` v2

> Nomor 0018 dipilih karena ADR-006…ADR-017 pernah dipakai di arsip ClawCrew (`files/brand-hit-inventory.csv`) dan ID tidak boleh dipakai ulang.

---

## 1. Konteks & Problem Statement

Fakta di `testing@3f3a729` (rincian: `board/research/RES-2026-10-29-rust-single-language-baseline.md`):

- Kontrak gRPC `proto/agent_service.proto` dipegang dua bahasa; Rust mengabaikan field 3/5/7 `ExecuteTurnRequest` dan tidak pernah mengirim `TurnToolCall`. Tidak ada test kontrak lintas bahasa yang menangkapnya.
- Server gRPC Rust (`start_grpc_server`) tidak pernah dipanggil; Go tidak dikirim di `Dockerfile`; sidecar Tauri tidak menjalankan Go. Jalur Go↔Rust tidak berjalan di produksi.
- Dashboard (web + Tauri + nginx) menunjuk Go `:9090`.
- Router produksi Rust (`gateway.rs:134 build_gateway_router`) hanya ±23 rute setelah refactor `db18a3745`; rute yang dipanggil CLI (`/admin/shutdown`, `/admin/paircode`, `/admin/sop/pending`, `/pair`) tidak terpasang; layer body-limit/timeout/security-headers hilang.
- Agent loop Rust (`Agent::turn_streamed*`) sudah dipakai `/ws/chat`; kapabilitas yang dibutuhkan port (policy perintah, ApprovalManager, DeviceRegistry, sysinfo, qrcode, rusqlite, insta) sudah ada.

Masalah: mempertahankan Go menambah satu bahasa, satu toolchain, satu kontrak protobuf, dan satu proses — untuk ±21k baris produksi yang separuhnya tidak terhubung — sementara perilaku yang benar-benar dipakai bisa ditampung crate Rust yang ada.

## 2. Pilihan Solusi yang Dipertimbangkan

1. **Opsi A — Pertahankan Go sebagai otak (status quo ODR-11).**
   - Kelebihan: tidak ada pekerjaan port.
   - Kekurangan: agent loop harus dipindah dari Rust ke Go (pekerjaan jauh lebih besar dari port dashboard); kontrak dua bahasa tetap rawan drift; Go belum dikirim di produksi.
2. **Opsi B — Go sebagai BFF dashboard saja, Rust sisanya.**
   - Kelebihan: port lebih sedikit.
   - Kekurangan: tetap dua bahasa dan dua proses; masalah kontrak dan distribusi tetap; Go fleet API justru sumber celah keamanan (bind semua interface, exec tanpa auth).
3. **Opsi C — Rust bahasa tunggal; port hanya rute Go yang terpasang & dikonsumsi; add-on Go di repo terpisah.**
   - Kelebihan: satu kontrak (tipe Rust + snapshot test), satu binary, satu toolchain CI; reuse komponen Rust yang ada; menutup celah keamanan sekaligus.
   - Kekurangan: perlu port ±20 rute dashboard dan pemulihan router; risiko regresi perilaku UI.
4. **Opsi D — Rust tunggal + rewrite besar (`galleon-runtime` dipecah total, storage ganti SQLite, Squad/Ship/Mission dibangun).**
   - Kelebihan: arsitektur "ideal".
   - Kekurangan: melanggar YAGNI; risiko tinggi; tidak dibutuhkan untuk menghapus Go.

## 3. Keputusan

**Opsi C.** Bentuk teknis:

```
apps/tauri (Tier 1) ──HTTP+token──▶ galleon-gateway (Tier 2, :42617, 127.0.0.1)
web/ (Tier 3 UI)   ──HTTP/WS───────▶        │
CLI galleon        ──in-process / HTTP admin┘
                                    ├── api_fleet/ (adapter HTTP, tipis)
                                    │       └──▶ galleon-fleet-core (domain + store JSON atomik, tanpa axum/runtime)
                                    ├── api_pairing.rs / DeviceRegistry (SQLite)
                                    ├── ws/ (Agent::turn_streamed)
                                    └──▶ galleon-runtime (agent, approval, security, sop, cron, …)
```

Aturan crate:

- `galleon-fleet-core`: dependensi hanya `galleon-config`, `galleon-infra`, `serde`, `serde_json`, `thiserror`, `tempfile`, `sysinfo`, `qrcode` (semua sudah di `Cargo.lock`). `#![forbid(unsafe_code)]`, `[lints] workspace = true`.
- `galleon-gateway/src/api_fleet/`: satu file per kelompok rute, setiap handler memanggil `require_auth` lalu fungsi `galleon-fleet-core`. Tidak ada logika bisnis di handler.
- Penghapusan: `strangler_runner.rs`, `grpc_system_gateway.rs`, `start_grpc_server`, `TurnApiHandler`, `build.rs` + `proto/agent_service.proto`, dependensi `tonic`/`prost` bila tidak dipakai crate lain (dicek `cargo tree -i tonic`).
- Ekstraksi dari `galleon-runtime` dibatasi pada modul coupling rendah. Program ini hanya mengekstrak `tunnel` (`crates/galleon-tunnel`, TASK-105) setelah wiring-nya dipulihkan (TASK-104); `approval` ditunda (YAGNI).

## 4. Konsekuensi

- **Positif:** satu sumber kontrak; Go hilang dari supply chain dan CI; dashboard berjalan dengan binary yang sama di Docker/Tauri; celah S-01…S-07 tertutup; CLI admin/pairing kembali berfungsi.
- **Negatif / Risiko:**
  - R1 — Perbedaan perilaku halus antara handler Go dan Rust (format JSON, kode status).
  - R2 — Memulihkan rute lama dapat menghidupkan kembali kode yang belum diuji ulang sejak 2026-10-03.
  - R3 — Pewarisan lint memicu ribuan peringatan (`-D warnings`).
  - R4 — Data dashboard pengguna yang sudah ada di direktori data Go.
- **Mitigasi:**
  - R1 → fixture kontrak direkam dari kode Go **sebelum** Go dihapus (Sprint 10–11), snapshot `insta` di Rust.
  - R2 → hanya rute berkonsumen yang dipulihkan; setiap kelompok rute punya test 401 + happy path; sisanya tetap tidak terpasang dan dicatat sebagai dormant.
  - R3 → ratchet `#![allow(...)]` per crate dengan baseline angka (SPEC-016).
  - R4 → format file JSON dipertahankan; nama koleksi kanonik camelCase (yang dipakai web, RES F-16). Lokasi baru `<Config.data_dir>/fleet/` (Rust sudah punya `data_dir`, `galleon-config/src/schema.rs:86–110`). Impor sekali dari lokasi Go (`FLEET_DATA_DIR`, `GALLEON_DATA_DIR`, `<cwd>/engine/data`; `services.go:57–69`) tanpa menghapus sumber, ditandai file `.imported`. Diverifikasi di TASK-067.

## 5. Kepatuhan Batasan (Boundary Compliance)

- **Tanggung jawab Tier:** sesuai `standard-docs/tech-standart/architecture/01-boundary-policy.md` **v2** (3-tier).
- **Protokol Komunikasi:** REST/JSON + WebSocket ke `galleon-gateway`; Tauri IPC hanya untuk OS integration; panggilan fungsi Rust antar crate. gRPC internal **tidak** dipakai.
