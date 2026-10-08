# Code Reality Check — Semua Checkbox `[x]` di `dev-task-2/` vs Kode Nyata

**Tanggal:** 2026-10-08
**Metode:** 4 subagent read-only paralel (grep + read + jalankan test/build nyata di repo), ditambah verifikasi langsung oleh squad-lead untuk item kritis (BL-52, BL-01). Tidak ada perubahan dibuat ke `dev-task-2/` — ini laporan audit, bukan edit.
**Scope:** Setiap item `[x]` di sprint-0 s.d. sprint-8, dicek terhadap definisi+pemanggil nyata di kode (bukan nama/registry saja — sesuai lesson "configuration-lies" project ini).
**Catatan:** Satu subagent (Sprint 1+2, `cargo test -p clawcrew-channels`) mengalami cold-compile >21 menit yang tidak sempat selesai sebelum laporan ini disusun — item yang bergantung pada hasil test itu ditandai "verifikasi grep saja, runtime test belum selesai" di bawah. Verifikasi grep langsung squad-lead mengkonfirmasi BL-52 dan BL-01 solid secara kode.

## Ringkasan eksekutif

Dari ±115 item `[x]` yang diperiksa lintas 9 sprint:
- **Mayoritas (~75%) genuinely SESUAI** — kode ada, terhubung ke jalur produksi, test lulus.
- **~10 item adalah overclaim nyata** (`[x]` tapi kode nol/sebagian) — pola "configuration-lies" yang sudah berulang kali tercatat di lesson project ini.
- **Sprint 4 punya kontradiksi internal terkonfirmasi**: 6 item body sendiri `[ ]` (BR-C5/C6/C7/D1/D2/D4) — ini SUDAH BENAR, bukan bug. Tapi 2 item lain yang ditandai `[x]` (**BR-D3**, **BG-F5**) TERBUKTI SALAH oleh kode.

## Temuan CRITICAL/HIGH — checkbox `[x]` tidak cocok kode

| ID | Sprint | Klaim | Kenyataan kode | Bukti |
|---|---|---|---|---|
| **BR-D3** | 4 | `[x]` "Hapus `verifiable_intent`" | **TIDAK DIHAPUS** — masih 43 referensi di 12 file | `crates/clawcrew-config/src/schema.rs:658` struct `VerifiableIntentConfig` masih ada; `fixtures/v1.toml:746` section `[verifiable_intent]` masih ada. Commit `20985dc0` yang diklaim "hapus" kemungkinan hanya hapus modul standalone tanpa bersihkan config schema, atau di-revert. |
| **BG-F5** | 4 | `[x]` "zap + request-id Go→Rust; scrubbing" | **zap TIDAK PERNAH diadopsi** | `grep "go.uber.org/zap"` di `engine/` = 0 match total, tidak ada di `go.mod` manapun. Hanya `X-Request-ID` HTTP header passthrough biasa yang ada (`run/delivery.go:39,68`) — bukan structured logging. |
| **BL-30** | 6 | `[x]` "`erase_identity()` lintas-backend" | **TIDAK ADA SAMA SEKALI** | `grep "erase_identity\|forget_all_for_agent" crates/clawcrew-memory/` = 0 match. Dokumen eksekusi sendiri menulis "nama fungsi YANG DIUSULKAN, bukan existing" — tapi checkbox tetap `[x]`. |
| **BL-31** | 6 | `[x]` "Hygiene backend-agnostic" | **Masih SQLite-only** | `hygiene.rs:5` `use rusqlite::{Connection, params}` dipakai langsung di baris 371/405/425/751. Tidak ada trait abstraction. |
| **BR-B5** | 0 | `[x]` "Vault rotasi/ekspor/migrasi, `GetDecryptedSecret` auth gate" | **TIDAK ADA** | `secrets.rs` (107KB, dibaca penuh) hanya punya `new/encrypt/decrypt/decrypt_and_migrate/needs_migration/is_encrypted/is_onepassword_ref/is_secure_encrypted`. Grep repo-wide `rotate_secret\|export_encrypted_vault\|import_encrypted_vault\|get_decrypted_secret` = 0 match. |
| **BL-20/21/22** | 5 | `[x]` "Dokumentasikan gap sandbox MCP / vault-mediation / ADR WASM vs MCP" | **Subjeknya (MCP client host) tidak eksis** | Grep repo-wide `crates/**/*.rs` untuk `\bmcp\b` (case-insensitive) = 0 match total. Tidak mungkin ketiga tiket ini genuinely selesai kalau subjek pembandingnya tidak ada di kode. |
| **BL-16/BL-17** | 5 | `[x]` "Surface `loop_detector` ke Tasks-view dashboard" | **UI shell kosong, nol backend wiring** | `QuestsView.tsx:369-392` render `selectedQuest.circuitBreaker` — field optional di `types/index.ts:331`, TIDAK PERNAH diisi fetch API apa pun. Gateway grep `loop_detector\|circuit_breaker` = 0 match. UI selalu fallback teks statis "Nominal — 3-pattern detection active". |

## Temuan MEDIUM — sebagian/deviasi arsitektur (bukan bug, tapi perlu dicatat)

| ID | Sprint | Klaim | Kenyataan |
|---|---|---|---|
| BR-C5 | 4 | Body `[ ]` sudah benar — gateway masih 425 referensi `clawcrew_runtime::` (klaim lama 426, hampir tidak berubah) | Konsisten, body sprint sendiri sudah akurat |
| BR-C6 | 4 | Body `[ ]` benar — narasi dokumen "sudah: core/imessage/signal/bluesky" **tidak terbukti**, 0 crate `galleon-channel-*` ditemukan | Klaim narasi di luar checkbox juga perlu dikoreksi |
| BG-C2 | 0 | `[x]` tapi menciptakan **dua device-registry paralel** — `identity.TokenStore` Go (in-memory) TIDAK terhubung ke Rust `DeviceRegistry`/`api_pairing.rs` seperti direncanakan | Risiko SSOT yang diperingatkan dokumen sendiri sebagai "jangan diputuskan diam-diam" — terjadi juga |
| BR-B6 | 0 | `[x]` tapi lokasi salah — loopback provider ada di **Go** (`identity/provider.go`), bukan Rust `auth_provider/` seperti diklaim | Fungsional benar, lokasi/bahasa klaim salah |
| S2-SEC-01 | 0 | `[x]`, fungsional benar (test lulus) tapi arsitektur beda — guard inline di handler+service (2 layer duplikat), bukan middleware route-wrapper tunggal seperti dirancang | Tidak bug, tapi risiko drift config ke depan |
| BR-F5 | 5 | `[x]` "e2e penuh (host/signature/capability)" — nyatanya cuma 1 test function (`loads_and_exposes_webhook_ingress`), load-proof minimal | Jauh dari "e2e penuh" |
| BR-F2 | 5 | `[x]` WIT v1 dual-host — baru `types.wit`, 7 file interface lain (channel/config/tool/dll) belum porting | Scaffolding awal saja |
| BG-F1 (AC2) | 4 | `[x]`, 5/7 AC genuinely fixed (termasuk AC1 ID Ship kolisi — bug lama BENAR sudah diperbaiki) tapi AC7 (grandfathering) tidak terverifikasi mendalam | Perbaikan signifikan, 1 AC perlu klarifikasi |

## Temuan SOLID — konfirmasi positif penting

- **BL-05 (CRITICAL privilege-escalation)**: genuinely closed. `dispatcher.go:241` satu jalur unified (`spawn_subagent`/`delegate_task` sama-sama lewat satu handler), `tool/dto.go:67-68` `DelegationDepth`/`MaxDelegationDepth` ada, `delegation.go:131-135` enforcement nyata + test. Ini temuan keamanan paling penting dan terbukti solid.
- **BL-52 (bug data-loss attachment)**: terverifikasi langsung oleh squad-lead — `orchestrator/mod.rs:1696` `turn.turn.msg.attachments = attachments` benar mengumpulkan attachment dari seluruh pesan debounce, plus enforcement limit di baris 1666-1672 (BL-54/55).
- **BL-01 (persistence bridge Go↔Rust)**: `grpc_system_gateway.rs:642,701` `store_learned_rule`/`retrieve_learned_rules` ada dan terimplementasi.
- **BG-D1 (resource-gate subagent Go)**: `services.go:187-197` gate SEBELUM `subagentWg.Add(1)` — persis titik yang diklaim.
- **ENT-01/BL-80**: nilai tier persis cocok spesifikasi (`policies.go:8` Community `{1,1,2,6,8,2,1}`).
- **GO-MOD-RENAME/BL-82**: `engine/go.mod:1` sudah benar, meski `go build ./...` belum dijalankan sesi ini untuk konfirmasi hijau penuh.
- **BL-23**: `PluginPermission::FileRead/FileWrite` ternyata genuinely enforced (match tunggal yang dicurigai dokumen justru adalah guard check yang benar), bukan dead-permission seperti dikhawatirkan.
- **BL-37/38/39, BL-53/54/55**: semua terverifikasi sesuai (tool I/O capture, scrub, ACL Windows, MediaKind, attachment caps).

## Perubahan diminta (ke dokumen `dev-task-2/`, bukan ke kode)

1. **BR-D3** (sprint-4): ubah `[x]` → `[ ]` atau `[~]`, catat bahwa `VerifiableIntentConfig` masih hidup di schema+fixture.
2. **BG-F5** (sprint-4): ubah `[x]` → `[ ]`, catat "request-id HTTP header ada, zap structured logging nol".
3. **BL-30** (sprint-6): ubah `[x]` → `[ ]`, catat fungsi genuinely belum dibangun.
4. **BL-31** (sprint-6): ubah `[x]` → `[ ]`, catat masih SQLite-hardcoded.
5. **BR-B5** (sprint-0): ubah `[x]` → `[ ]`, catat rotate/export/import vault belum ada.
6. **BL-20/21/22** (sprint-5): ubah `[x]` → `[ ]`, catat MCP client host tidak eksis sebagai blocker struktural.
7. **BL-16/17** (sprint-5): ubah `[x]` → `[ ]` atau tandai "UI shell only, no backend wiring".
8. **BG-C2** (sprint-0): tidak perlu ubah checkbox (fungsional ada), tapi tambah catatan risiko "dua device-registry paralel belum disatukan — keputusan Lead/Owner diperlukan".

## Rekomendasi

Jangan buka PR batch apa pun ke `testing` untuk sprint 0/4/5/6 sebelum 7 item di atas diselaraskan — Principle akan menolak begitu cross-check, membuang satu putaran review (konsisten temuan audit DoD 2026-10-08 sebelumnya yang sudah menunjukkan pola serupa di level dokumen, kini terkonfirmasi di level kode baris-per-baris).

Pola "configuration-lies" (checkbox `[x]` mendahului implementasi nyata, kemungkinan ditulis optimis berdasarkan desain bukan hasil) terjadi di SETIAP sprint yang diperiksa kecuali sprint-2 dan sprint-3 (keduanya 100% bersih). Rekomendasikan ke Owner: proses penulisan dev-task sprint berikutnya memisahkan jelas "desain ditulis" vs "kode diverifikasi berjalan" sebelum checkbox dicentang — bukan mencentang berdasarkan niat/rancangan.
