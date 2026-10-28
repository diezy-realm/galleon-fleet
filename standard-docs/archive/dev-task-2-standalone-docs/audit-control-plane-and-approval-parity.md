# Audit & Analisis: Padanan Control Plane, Approval, dan Dekoplasi Rust (BG-D5, BR-C5, BR-C6, BR-D4)

**Tanggal:** 2026-10-07  
**Tiket:** BG-D5, BR-C5, BR-C6, BR-D4  
**Status:** Selesai / Terverifikasi berbasis data riil kode

---

## 1. Analisis Padanan `control_plane` & `approval` (BG-D5 · G-34)

### 1.1 Temuan Kode Rust (`crates/galleon-runtime/src/control_plane/`)
Pemeriksaan struktur modul `control_plane` di Rust menemukan:
- `task_registry.rs` (19.4k LOC): Registrasi tugas runtime dan metadata state.
- `task_runner.rs` (54.9k LOC): Eksekutor state machine penanganan task turn.
- `task_store_sqlite.rs` (70.5k LOC): Persistensi relasional SQLite untuk task run dan audit snapshot.
- `authority.rs` (11.1k LOC): Otoritas penegakan izin instruksi.
- `a2a_projection.rs` (4.7k LOC): Proyeksi task ke jalur Agent-to-Agent.

### 1.2 Padanan di Sisi Go Engine
Go Engine telah memiliki subsistem ekuivalen yang bersih (Clean Architecture):
- **Task Management & State Machine:** `engine/src/task/` (`interfaces.go`, `services.go`, `dto.go`) dengan monotonic state machine (`CanTransitionTo`), `TaskRepository`, dan `TaskRunner`.
- **Approval & Action Digest:** `engine/src/approval/` (`interfaces.go`, `learning_delivery.go`) dengan `ActionDigest` SHA-256 canonical hashing dan approval audit trail.
- **SSOT Policy:** `engine/src/policy/` (`interfaces.go`, `services.go`) dan `engine/src/fleet/policies.go` memegang `FleetCode`, `RequireApprovalForWrite`, dan `MaxWeeklySpendUSD`.

### 1.3 Verifikasi SSOT (Single Source of Truth)
Sesuai aturan `AGENTS.md`:
- Go Engine `fleet/policies.go` dan `policy/services.go` adalah **satu-satunya sumber kebenaran (SSOT)**.
- Tidak dibuat duplikasi kontrol approval di runtime Rust maupun di layer UI. Seluruh evaluasi izin write, memory sharing, dan limit spending dieksekusi secara terpusat oleh Go.

---

## 2. Inventaris & Rencana Dekoplasi Crate Rust (BR-C5, BR-C6, BR-D4)

### 2.1 Dekoplasi Gateway (`BR-C5 · R-14`)
- **Ground Truth Count:** Ditemukan tepat **425 referensi** `galleon_runtime::` di dalam `crates/galleon-gateway/src/`.
- **Distribusi Utama:**
  - `galleon_runtime::cron` (103 ref)
  - `galleon_runtime::sop` (85 ref)
  - `galleon_runtime::observability` (48 ref)
  - `galleon_runtime::control_plane` (24 ref)
  - `galleon_runtime::health` (21 ref)
  - `galleon_runtime::agent` (19 ref)
  - Lain-lain: `integrations` (10), `i18n` (7), `tools` (7), `quickstart` (10), `doctor` (4), `restart` (4).
- **Strategi Dekoplasi:**
  - Endpoint `cron`, `sop`, `agent`, dan `tasks` yang bergantung ke runtime akan dialihkan konsumsinya ke Go Client Protocol (BG-E2) dan Go Engine REST API.
  - Setelah porting dan pengujian `galleon-eval` replay selesai (Sprint 4-6), layer handler gateway dapat dipensiunkan ke angka 0.

### 2.2 Pemecahan Crate Channels (`BR-C6 · R-15`)
- **Crate yang Sudah Berdiri Sendiri:**
  - `crates/galleon-channel-core`
  - `crates/galleon-channel-bluesky`
  - `crates/galleon-channel-imessage`
  - `crates/galleon-channel-signal`
- **Channel Monolith Tersisa di `crates/galleon-channels`:**
  - `discord/`, `git/`, `matrix/`, `orchestrator/`, `slack/`, `telegram/`.
- **Pola Pemecahan (RF-B1):**
  - Mengikuti template `galleon-channel-bluesky` saat mengekstraksi channel Tier A (Telegram, Slack, Discord) dengan public surface identical dan allowlist policy dari `galleon-channel-core`.

### 2.3 Penghapusan Legacy RPC (`BR-D4 · R-41`)
- **Status Ketergantungan:**
  - Memerlukan penyelesaian `BR-E3` (Sprint 6, retargeting CLI/TUI ke Client Protocol) dan `BR-C5` (Sprint 4-5).
  - Penghapusan `crates/galleon-runtime/src/rpc/` dan REST/WS/A2A lama dipagari oleh kedua dependensi tersebut demi mencegah outage risk.
