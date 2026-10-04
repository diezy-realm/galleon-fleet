# Naming Map — Lama → Galleon (draf untuk sign-off Owner)

> **Status:** DRAF (N-00). Berlaku setelah Owner menyetujui. Basis: ODR-16, ODR-17, ODR-18.
> Fakta diambil dari kode 2026-10-04. Kolom *Strategi* menentukan apakah perlu migrasi data.

## 1. Binary & aplikasi

| Lama | Baru | Strategi |
|---|---|---|
| `clawcrew` (root bin/package) | package `galleon-fleet`, perintah `galleon` | **tanpa alias** (ODR-19) |
| `clawcrew-acp-bridge` (`src/bin/`) | `galleon-acp-bridge` | rename |
| `zerocode` (`apps/zerocode`, TUI CLI) | `galleoncode` | ODR-17; env `ZEROCODE_*`→`GALLEONCODE_*` |
| `zerorelay` (`apps/zerorelay`) | `galleonrelay` | ODR-17; env `ZERORELAY_*`→`GALLEONRELAY_*` |
| `fleet-ai-desktop` (`apps/tauri`) | `galleon-desktop` | rename paket |
| `react-example` (`web/`) | `galleon-web` | rename paket |
| `xtask`, `tools/fill-translations` | tetap | hanya ubah string merek di dalamnya |

## 2. Crate Rust (27)

Aturan: `clawcrew-X` → `galleon-X`. Crate yang sudah `galleon-*` tetap. Urutan eksekusi bawah→atas (N-21).

| Lama | Baru | Catatan |
|---|---|---|
| `clawcrew-api` `-infra` `-log` `-macros` `-tls` `-dist` `-spawn` `-commands` `-hardware` | `galleon-api` `-infra` `-log` `-macros` `-tls` `-dist` `-spawn` `-commands` `-hardware` | rename langsung |
| `clawcrew-config` `-memory` `-providers` `-tools` `-tool-call-parser` `-routing` `-eval` `-sop-graph` | `galleon-*` | `sop-graph` & `eval` menunggu hasil R-00 (mungkin dihapus/dipindah) |
| `clawcrew-channels` | `galleon-channels` | lalu dipecah per channel (R-15) menjadi `galleon-channel-<nama>` |
| `clawcrew-runtime` | `galleon-runtime` | **rename ditunda sampai R-00/R-16** (isi akan menyusut/dipindah) |
| `clawcrew-gateway` | `galleon-gateway` | setelah R-14 |
| `clawcrew-plugins` | `galleon-plugins` | |
| `clawcrew-relay-proto` | `galleon-relay-proto` | ikut `galleonrelay` |
| `galleon-channel-{core,signal,bluesky,imessage}`, `galleon-i18n` | tetap | sudah sesuai |

Identifier Rust: `clawcrew_*` → `galleon_*`; tipe `ClawCrew*` → `Galleon*`.

## 3. Environment variable

| Lama (kemunculan di kode) | Baru | Strategi |
|---|---|---|
| `CLAWCREW_*` (±400; mis. `WORKSPACE`, `CONFIG_DIR`, `AUDIT_SIGNING_KEY`, `DATA_DIR`, `SOCKET`, `SLACK_BOT_TOKEN`, `ACP_BRIDGE_TOKEN`) | `GALLEON_*` | **ganti langsung, tanpa fallback** (ODR-19). Satu-satunya pengecualian: migrasi data pengguna (§4). **Jangan log nilai.** |
| `ZEROCODE_*` (20), `ZERORELAY_*` (7) | `GALLEONCODE_*`, `GALLEONRELAY_*` | ganti langsung |
| `GALLEON_*` (3, sudah ada) | tetap | |
| `CLAWCREW_TEST_*`, `CLAWCREW_APT_TEST_CALLS`, `CLAWCREW_PARALLEL_TEST_RUNS` | `GALLEON_TEST_*` dst. | khusus test, tanpa fallback |

## 4. Path & data pengguna (butuh migrasi)

| Lama | Baru | Strategi |
|---|---|---|
| `~/.clawcrew/` (+ `config`) | `~/.config/galleon/` (Windows: `%APPDATA%\galleon\`) | pada boot: bila baru kosong & lama ada → salin atomik + tanda `.migrated`; **tidak menghapus** lama; vault sudah di `~/.config/galleon/vault/` |
| `.clawcrew_config`, `.clawcrew_plugin_channel`, `.clawcrew_plugin_memory`, `.clawcrew_pane` (nama file/dir internal/fixture) | `.galleon_*` | rename; cek apakah on-disk persisten (butuh migrasi) atau hanya test |
| Socket/IPC (`CLAWCREW_SOCKET`) | `GALLEON_SOCKET` | rename + fallback |

## 5. Kontrak & identitas teknis

| Lama | Baru | Catatan |
|---|---|---|
| WIT `clawcrew:plugin@0.1.0` (v0, beku) | v0 tetap; **v1 = `galleon:plugin@1.0.0`** (`wit/v1/`) | ODR-18, N-24, R-35 |
| Go module `github.com/diezy-labs/claw-crew/engine` | `github.com/diezy-realm/galleon-fleet/engine` | ubah semua import + `wire` regen |
| gRPC package/service di `proto/agent_service.proto` | tetap `SystemGateway`/`AgentEngine`; ganti nama package proto bila memuat merek lama | perubahan wire-compatible; regen kedua sisi dalam satu PR |
| Header HTTP/WS, prefix metrik | `X-Galleon-*`, `galleon_*` | |
| Tauri `productName` "Fleet AI", title "Fleet AI - Autonomous Fleet" | "Galleon" | `identifier` `com.galleon.sovereign` sudah sesuai — pertahankan |
| Docker image `ghcr.io/clawcrew-labs/clawcrew` | `ghcr.io/diezy-realm/galleon-fleet` | org = `diezy-realm` (ODR-19) |
| Nama service OS (systemd/launchd/Windows Service) | `galleon` | unit lama dihapus/dimigrasi oleh `galleon service install` |

## 6. Teks & istilah

| Lama | Baru |
|---|---|
| ClawCrew, Claw Crew, claw-crew, zeroclaw | Galleon |
| Fleet AI, FleetAI, fleet-ai | **Galleon Fleet** (nama produk); slug `galleon-fleet` |
| Istilah lore (Ship, Quartermaster, Squad, Crew) | tetap untuk UX; kode netral (`fleet/ship/squad/crew`) |

## 7. Pengecualian (JANGAN diubah)

- `standard-docs/decisions/**` (ADR/ODR immutable), `CHANGELOG`, riwayat git.
- Atribusi lisensi upstream & `NOTICE`.
- Fixture bertanda tangan/hash tetap, data uji historis.
- WIT v0 (`clawcrew:plugin@0.1.0`) — beku.
- Format on-disk lama selama jalur migrasi belum ada.

## 8. Aturan branding (ODR-19)

| Konteks | Bentuk |
|---|---|
| Nama produk (UI, docs, window title, installer) | **Galleon Fleet** |
| Slug identitas (repo, org path, Go module, image, paket root, README) | `galleon-fleet` (org `diezy-realm`) |
| Prefix teknis pendek (crate `galleon-*`, env `GALLEON_*`, perintah `galleon`, `galleoncode`, `galleonrelay`, path `galleon/`) | `galleon` |
| Dilarang di kode/aset baru | `clawcrew`, `zeroclaw`, `claw-crew`, `FleetAI`, `Fleet AI`, `fleet-ai` |

Tidak ada alias/fallback nama lama. Satu-satunya jembatan: migrasi data pengguna `~/.clawcrew` → `~/.config/galleon` (§4).

## 9. Keputusan terselesaikan (berbasis fakta 2026-10-04)

| Pertanyaan | Hasil | Bukti |
|---|---|---|
| Alias lama? | Tidak ada (ODR-19) | Owner |
| Org GHCR | `diezy-realm`, produk `galleon-fleet` | Owner; `git remote` = `diezy-realm/galleon-fleet` |
| `sop-graph` & `eval` | **Tetap; hanya rename → `galleon-sop-graph`, `galleon-eval`** | lihat di bawah |

**Fakta `clawcrew-sop-graph`:** 442 LOC, tipe proyeksi serde murni; satu-satunya pemakai kode = `runtime/src/sop/graph.rs` (+ fitur `schema-export` runtime). Tidak dipakai `apps/zerocode` (deskripsi crate usang). Ikut nasib modul `sop` di R-00 (bila SOP pindah ke Go, tipe wire ini menjadi kontrak/proto, bukan hilang).
**Fakta `clawcrew-eval`:** 2.470 LOC, dipakai CLI root (`src/commands/eval.rs`, `src/main.rs`) dan `tests/support/trace.rs`; `runner.rs` membangun `clawcrew_runtime::agent::agent::Agent` untuk replay deterministik trace LLM. **Ketergantungan keras pada otak agent Rust** → bila `agent` pindah ke Go (R-00), harness harus diarahkan ulang ke Go (replay lewat `StartTurn` + mock `ExecuteTurn`). Bukan alasan menghapus; jadi sub-tiket R-37.