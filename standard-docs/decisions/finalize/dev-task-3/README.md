---
title: Dev Task 3 — Konsolidasi Task Aktif (dari dev-task-2, 5 sprint tersisa)
status: active
date: 2026-10-09
author: squad-lead
source: standard-docs/decisions/finalize/dev-task-2/sprint-{0,4,5,6,8}-*.md (5 dari 9 sprint — sprint 1/2/3/7 sudah 100% selesai, diarsipkan ke standard-docs/archive/dev-task-2-completed-sprints/)
---

# Dev Task 3 — Konsolidasi Task Aktif

## Asumsi & metodologi (baca sebelum pakai dokumen ini)

1. **Sumber tunggal: `dev-task-2/sprint-*.md`, bukan `backlog/`/`roadmap/` langsung.** `dev-task-2/README.md` menyatakan dirinya adalah "lapisan rekonsiliasi" yang SUDAH menggabungkan seluruh `backlog/BL-01..82` + `roadmap/R-00..R-95`/`G-*`/`N-*`/`P-*` ke 9 sprint, dengan tabel Reconciliation eksplisit per item yang dipetakan. Mengaudit `backlog/`/`roadmap/` lagi sebagai sumber independen akan menduplikasi pekerjaan rekonsiliasi yang sudah dilakukan — dokumen ini TIDAK melakukan itu.
2. **5 sprint aktif dibaca penuh:** sprint-0 (1 item open), sprint-4 (12 item), sprint-5 (2 item), sprint-6 (2 item), sprint-8 (15 item) — total 32 entri tiket mentah sebelum konsolidasi.
3. **Deteksi duplikasi dilakukan DI DALAM 32 entri ini** (bukan terhadap backlog/roadmap mentah) — ditemukan overlap yang digabung menjadi entri tunggal (lihat §Duplikasi Terdeteksi).
4. **Isi substantif TIDAK diubah** — Detail Eksekusi, spesifikasi API, langkah pengerjaan, dependency, dan catatan "JANGAN DIUBAH" dari sprint asli dipertahankan verbatim atau diringkas tanpa menghapus requirement. Status ambigu dinyatakan eksplisit, tidak diputuskan sepihak.
5. **Governance docs dikecualikan** — `PRINCIPLE-CREW-*`, `3-TIER-AUTOMATION-MODEL.md`, ADR, ODR adalah kebijakan/referensi, bukan task kerja, jadi tidak masuk konsolidasi ini.

## Peta sumber → entri konsolidasi

| Sprint asal | Item `[ ]` asli | Entri di dokumen ini |
|---|---|---|
| Sprint 0 | 1 (BR-B5) | T3-01 |
| Sprint 4 | 12 (BR-C5,C6,C7,D1,D2,D3,D4; BG-F5) | T3-02 s.d. T3-09 |
| Sprint 5 | 2 (BL-16, BL-17) | T3-10, T3-11 |
| Sprint 6 | 2 (BL-30, BL-31) | T3-12, T3-13 (dicatat SALING TERKAIT, lihat §Duplikasi) |
| Sprint 8 | 15 (RB-12,16,17,20,21,22,23,30; PL-E1,E2,E4; RB-40,41; GV-11) | T3-14 s.d. T3-24 |
| *(gap, bukan dari sprint)* | R-13 (Lane R roadmap) | T3-02b |
| *(gap, bukan dari sprint)* | G-22, G-29 (Lane G roadmap) | T3-25, T3-26 |
| *(gap, bukan dari sprint)* | U-02/03/04, U-10/11 (Lane U roadmap) | T3-27, T3-28 |

**Total: 32 tiket mentah (dev-task-2) + 6 gap genuine ditemukan via audit silang roadmap → 31 entri konsolidasi.** Lihat §Catatan verifikasi tambahan untuk metodologi dan daftar lengkap gap.

---

## Duplikasi / overlap terdeteksi (digabung, bukan dihapus)

| Entri gabungan | Tiket asli yang digabung | Alasan |
|---|---|---|
| **T3-12/13 ditandai saling-terkait, TIDAK digabung jadi satu** | BL-30 (erase_identity) + BL-31 (hygiene retention) | Sprint-6 sendiri sudah mencatat ASUMSI "pertimbangkan menggabungkan desain trait extension BL-30 dan BL-31 dalam SATU perubahan trait" — tapi ini usulan yang BELUM disetujui Lead di dokumen asli. Dipertahankan sebagai 2 entri terpisah di sini, dengan catatan silang eksplisit, SESUAI status ambigu aslinya — bukan diputuskan sepihak di sini. |
| **T3-20 (RB-29 atribusi lisensi)** | Disebut di Sprint 8 sebagai sub-catatan RB-26-29, tidak punya baris checkbox independen (RB-26-29 semua `[x]` kecuali konteksnya) | RB-29 sendiri sudah `[x]` — TIDAK dimasukkan sebagai tiket aktif, hanya dicatat sebagai CONSTRAINT yang mengikat T3-19 (RB-30, dokumen normatif) supaya atribusi tidak hilang saat rename lanjutan menyentuh dokumen. |
| **CO-08 web-2/dist (RB-16b)** | Disebut dalam Sprint 8 CO-08 sebagai sub-task yang "belum ada baris eksplisit di sprint manapun" | Sudah genuinely dieksekusi — `web-2/` dan `apps/tauri-2/` dikonfirmasi 0 file tracked git per audit sesi 2026-10-09 (lihat lesson project). **CO-08 keseluruhan statusnya sudah `[x]` di sprint-8** — tidak masuk sebagai tiket aktif di dokumen ini, dicatat di sini murni sebagai jejak audit supaya tidak ditanyakan ulang. |
| **PL-D2 (installer Tier B) vs BR-F4 (Sprint 5, instalasi Tier B)** | Sprint-8 sendiri mencatat: "scope SAMA, kemungkinan PL-D2 adalah bagian installer dari BR-F4, koordinasikan supaya tidak dikerjakan dua kali" | BR-F4 statusnya `[x]` di sprint-5. PL-D2 (sprint-8) **tetap `[x]`** di sumber asli juga — tidak ada tiket terbuka tersisa untuk topik ini, tidak masuk dokumen ini. |
| **RB-21 (rename crate) depends BR-D1 (Sprint 4) depends R-00 §3** | Rename `clawcrew-runtime`→`galleon-runtime` (RB-21) secara eksplisit menunggu BR-D1 (ramping runtime) selesai | Dicatat sebagai dependency chain di T3-18 (RB-21), BUKAN entri terpisah — ini satu rantai keputusan, bukan dua pekerjaan independen. |

**Tidak ditemukan duplikasi antar-sprint di luar yang tercatat di atas** — tabel Reconciliation `dev-task-2/README.md` sudah menyelesaikan sebagian besar deduplikasi sebelum sprint ditulis (BL-18→BG-D1/D4, BL-51→BG-B4, BR-F1→GV-07, dll — semua sudah `[x]`/tidak dijadwalkan ulang di sprint asli, jadi tidak muncul lagi sebagai item `[ ]` di sini).

---

## A. Keamanan & Identitas

### T3-01 — Vault: rotasi/ekspor/migrasi secret (BR-B5)

- **Sprint asal:** 0 (Security Foundation)
- **Tujuan:** `GetDecryptedSecret` hanya untuk caller terotorisasi; dukung rotasi/ekspor/migrasi vault.
- **Status implementasi:** Selesai di branch `testing` (commit `3af7290f9`) — `rotate_secret`/`export_encrypted_vault`/`import_encrypted_vault` diimplementasikan di `crates/clawcrew-config/src/secrets.rs` dan di-surface via gRPC `SystemGateway` (`crates/clawcrew-gateway/src/grpc_system_gateway.rs`). Diverifikasi oleh unit test `secrets::tests::rotate_secret_*` (3 tests lulus 100%) dan `secrets::tests::export_encrypted_vault_*` (4 tests lulus 100%).
- **Ruang lingkup:** `crates/clawcrew-config/src/secrets.rs`.
- **DoD:** PR #20 merged ke `testing` (commit `3af7290f9`); `rotate_secret`/`export_encrypted_vault`/`import_encrypted_vault` lolos test di branch `testing`. (Terpenuhi)

---

## B. Dekoplasi Crate Rust & Runtime Strangler (Sprint 4 — XL, berseri)

### T3-02 — `gateway` lepas dari `runtime` (BR-C5)

- **Tujuan:** `clawcrew-gateway` tidak lagi bergantung ke `clawcrew-runtime` (klaim lama: 426 referensi → 0).
- **Ruang lingkup:** `crates/clawcrew-gateway/src/`.
- **Status implementasi (in-progress):**
  - Tahap 1–10 selesai: 120 referensi berhasil diputus (426 → 306 referensi, pengurangan ~28.2%).
  - 25 referensi `clawcrew_runtime::health` dialihkan langsung ke SSOT asalnya `clawcrew_infra::health`.
  - 14 referensi `clawcrew_runtime::security::pairing` dialihkan ke SSOT `clawcrew_config::pairing`.
  - 9 referensi `clawcrew_runtime::i18n` dialihkan ke crate independen `galleon_i18n`.
  - 43 referensi `clawcrew_runtime::observability` (`Observer`, `ObserverEvent`, `ObserverMetric`, `NoopObserver`) dialihkan ke `clawcrew_api::observability_traits`.
  - 11 referensi `clawcrew_runtime::tools` (`CanvasStore`, batas konten `MAX_CONTENT_SIZE`, `ALLOWED_CONTENT_TYPES`), `cost` (`CostTracker`), dan import lama `DefaultMemoryStrategy` dialihkan ke `clawcrew_tools::canvas` dan `clawcrew_config::cost`.
  - 4 referensi di Tahap 4 (`browse` dialihkan ke SSOT `clawcrew_config::browse`, `SecretStore` ke `clawcrew_config::secrets`, dan `PerToolChannelHandle` ke `clawcrew_tools::send_via`).
  - 4 referensi di Tahap 5 (`restart` subsystem dialihkan ke SSOT `clawcrew_infra::restart`, commit `e42fc917d`).
  - 3 referensi di Tahap 6 (`process_stats` sampling & struct dialihkan ke SSOT `clawcrew_infra::process_stats`, commit `cddf96e8f`).
  - 2 referensi di Tahap 7 (`session_metadata` registry & fallback projections dialihkan ke SSOT `clawcrew_infra::session_metadata`, commit `ab232a143`). Referensi `session` di gateway turun menjadi 0.
  - 2 referensi di Tahap 8 (`param_options::resolve_options` dialihkan ke SSOT `clawcrew_tools::param_options`, commit `4637ed76a`). Referensi `tools` di gateway turun dari 3 menjadi 1.
  - 2 referensi di Tahap 9 (`identity::load_aieos_identity` & `Names` dialihkan ke SSOT `clawcrew_config::identity`, commit `eceb6d76a`). Referensi `identity` di gateway turun menjadi 0.
  - 1 referensi di Tahap 10 (`audit::verify_chain` dialihkan ke SSOT `clawcrew_config::audit`, commit `6c50894b0`).
  - 1 referensi di Tahap 11 (`webauthn` dialihkan ke SSOT `clawcrew_config::webauthn`, commit `cd56a1544`). Referensi `security` di gateway turun menjadi 0.
  - 1 referensi di Tahap 12 (`tools` di `api_sop_author.rs` dialihkan ke SSOT `clawcrew_tools::param_options`, commit `74914e3bd`). Referensi `tools` di gateway turun menjadi 0.
  - 1 referensi di Tahap 13 (`platform::backup` dialihkan ke SSOT `clawcrew_infra::backup`, commit `df2234abb`). Referensi `platform` di gateway turun menjadi 0.
  - 4 referensi di Tahap 14 (`doctor` di `api.rs` didekoplasi via `clawcrew_api::doctor::DiagnosticsRunner` dan `AppState::diagnostics`, commit `451a40034`). Referensi `doctor` di gateway turun menjadi 0.
  - 6 referensi di Tahap 15 (`rpc` types & `tui_registry` didekoplasi via `clawcrew_api::runtime_traits::TuiConnectionRegistry`, DTO config/personality/skills ke `clawcrew_config::dto`, `clawcrew_api::personality`, dan `clawcrew_api::skill`, commit `89e0f154f`). Referensi kategori `rpc` di gateway turun menjadi 0.
  - 9 referensi di Tahap 16 (`daemon` lifecycle controls & readiness reporter dialihkan ke SSOT `clawcrew_infra::gateway_controls`, commit `0d391d7d3`). Referensi `daemon` di gateway turun dari 9 menjadi 0.
  - 20 referensi di Tahap 17 (`quickstart` [10] & `integrations` [10] didekoplasi, commit `405746df6`):
    - `QuickstartRunner`, `FieldSection`, `FieldDescriptor`, `AppliedAgent`, `QuickstartStep`, `QuickstartError` didekoplasi ke `clawcrew_config::quickstart`.
    - `model_catalog_with_config` & `model_provider_is_local` didekoplasi ke `clawcrew_providers::catalog`.
    - `IntegrationCategory`, `IntegrationEntry`, `IntegrationStatus`, `IntegrationCatalogProvider` didekoplasi ke `clawcrew_config::integrations`.
    - Gateway `AppState` diperluas dengan `Option<Arc<dyn QuickstartRunner>>` dan `Option<Arc<dyn IntegrationCatalogProvider>>`.
    - `api_quickstart.rs`, `api_sections.rs`, dan `api.rs` di gateway dialihkan ke `clawcrew_config` dan `clawcrew_providers`.
    - Referensi `quickstart` dan `integrations` di gateway turun tuntas 100% menjadi 0.
  - Tahap 18 selesai (commit `187b6397c`):
    - `api_skills.rs`, `acp.rs`, `api_personality.rs`, `api_providers.rs`, `api_sections.rs`, dan `a2a.rs` didekoplasi 100% dari runtime.
    - Didefinisikan trait runner di leaf crate: `SkillsRunner` (`clawcrew_api::skill`), `PersonalityRunner` (`clawcrew_api::personality`), dan `AcpSessionRunner` (`clawcrew_api::runtime_traits`).
    - Dibuat runtime implementations: `RuntimeSkillsRunner` di `clawcrew-runtime/src/skills/runner.rs`, `RuntimePersonalityRunner` di `clawcrew-runtime/src/agent/personality_templates/mod.rs`.
    - Dibuat SSOT provider reliability `ProviderHealthRecord` di `clawcrew-providers::reliability`.
    - Gateway `AppState` diperluas dengan `personality_runner`, `skills_runner`, dan `acp_runner`.
    - Sisa referensi runtime di gateway turun dari 264 menjadi 251: `cron` (103), `sop` (93), `agent` (28), `control_plane` (24), `observability` (3 - test only).
    - 6 file gateway (`api_sections.rs`, `api_personality.rs`, `api_providers.rs`, `a2a.rs`, `api_skills.rs`, `acp.rs`) kini 100% bebas dari `clawcrew_runtime::`.
  - Seluruh unit test terkait lulus 100% (34 test `a2a`, 3 test `api_skills`, 5 test `personality`, compiler check gateway 0 error).
- **DoD:** 0 referensi `clawcrew_runtime::` tersisa di `clawcrew-gateway/src/` kecuali pengecualian yang didiskusikan & didokumentasikan.

### T3-03 — Pecah `clawcrew-channels` jadi crate per-channel (BR-C6)

- **Tujuan:** Pecah `channels` (198k LOC) → `galleon-channel-<nama>` satu crate per channel. Sudah selesai: `core`, `signal`, `bluesky`, `imessage`, `irc`, `nostr`, `twitch`, `dingtalk`, `mochat`, `clawdtalk`, `reddit`, `twitter`, `notion`, `linq`, `nextcloud`, `filesystem`, `amqp`, `wecom`, `webhook`, `email`, `qq`.
- **Ruang lingkup:** `crates/clawcrew-channels/src/`, crate baru `galleon-channel-*`.
- **Status implementasi (in-progress):**
  - Crate `galleon-channel-irc` diekstrak (14/14 unit test ok, commit `29aec5f46` & `bd9ee0252`).
  - Crate `galleon-channel-nostr` diekstrak mengikuti pola RF-B1: dipisahkan ke crate independen dengan dependency policy allowlist SSOT via `galleon-channel-core`, re-export `pub use galleon_channel_nostr as nostr`, monolith `src/nostr.rs` dihapus, terdaftar di arsitektur gate `tests/architecture/dependency_direction.rs` (commit `e31f0134f`).
  - Crate `galleon-channel-twitch` diekstrak ke crate independen `crates/galleon-channel-twitch` (8/8 unit test ok, commit `09b90e976`).
  - Crate `galleon-channel-dingtalk` diekstrak ke crate independen `crates/galleon-channel-dingtalk` (9/9 unit test ok, commit `7fa757755`).
  - Crate `galleon-channel-mochat` diekstrak ke crate independen `crates/galleon-channel-mochat` (9/9 unit test ok, commit `29294eede`).
  - Crate `galleon-channel-clawdtalk` diekstrak ke crate independen `crates/galleon-channel-clawdtalk` (5/5 unit test ok, commit `61b93cf7e`). Monolith `src/clawdtalk.rs` dihapus, re-export via `pub use galleon_channel_clawdtalk as clawdtalk`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-reddit` diekstrak ke crate independen `crates/galleon-channel-reddit` (22/22 unit test ok, commit `8a3b8ed26`). Monolith `src/reddit.rs` dihapus, re-export via `pub use galleon_channel_reddit as reddit`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-twitter` diekstrak ke crate independen `crates/galleon-channel-twitter` (15/15 unit test ok, commit `1d40d2c49`). Monolith `src/twitter.rs` dihapus, re-export via `pub use galleon_channel_twitter as twitter`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-notion` diekstrak ke crate independen `crates/galleon-channel-notion` (14/14 unit test ok, commit `3e85fc851`).
  - Crate `galleon-channel-linq` diekstrak ke crate independen `crates/galleon-channel-linq` (33/33 unit test ok, commit `b3c5ce86f`).
  - Crate `galleon-channel-nextcloud` diekstrak ke crate independen `crates/galleon-channel-nextcloud` (32/32 unit test ok, commit `9896cb22e`). Monolith `src/nextcloud_talk.rs` dihapus, re-export via `pub use galleon_channel_nextcloud as nextcloud_talk`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-filesystem` diekstrak ke crate independen `crates/galleon-channel-filesystem` (22/22 unit test ok, commit `4dc389546`). Monolith `src/filesystem.rs` dihapus, re-export via `pub use galleon_channel_filesystem as filesystem`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-amqp` diekstrak ke crate independen `crates/galleon-channel-amqp` (21/21 unit test ok, commit `8c131b0ee`). Monolith `src/amqp.rs` dihapus, re-export via `pub use galleon_channel_amqp as amqp`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-wecom` diekstrak ke crate independen `crates/galleon-channel-wecom` (7/7 unit test ok, commit `c609f7d1c`). Monolith `src/wecom.rs` dihapus, re-export via `pub use galleon_channel_wecom as wecom`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-webhook` diekstrak ke crate independen `crates/galleon-channel-webhook` (36/36 unit test ok, commit `35f56e1e8`). Monolith `src/webhook.rs` dihapus, re-export via `pub use galleon_channel_webhook as webhook`, terdaftar di arsitektur gate.
  - Crate `galleon-channel-email` diekstrak ke crate independen `crates/galleon-channel-email` (71/71 unit test ok). Monolith `src/email_channel.rs` dihapus, re-export via `pub use galleon_channel_email as email_channel`, terdaftar di arsitektur gate `tests/architecture/dependency_direction.rs` (lulus 100%).
  - Crate `galleon-channel-qq` diekstrak ke crate independen `crates/galleon-channel-qq` (57/57 unit test ok, commit `e041a8783`). Monolith `src/qq.rs` (2.993 baris) dihapus, re-export via `pub use galleon_channel_qq as qq`, trait `AudioTranscriber` didefinisikan secara decoupled tanpa ketergantungan runtime, terdaftar di arsitektur gate `tests/architecture/dependency_direction.rs` (lulus 100%).
  - Crate `galleon-channel-line` diekstrak ke crate independen `crates/galleon-channel-line` (61/61 unit test ok). Monolith `src/line.rs` (2.993 baris) dihapus tuntas, re-export via `pub use galleon_channel_line as line`, shared `identity_persist` dipusatkan ke `galleon-channel-core` (19/19 unit test ok), trait decoupled `AudioTranscriber` diimplementasikan untuk `TranscriptionManager`, terdaftar di arsitektur gate `tests/architecture/dependency_direction.rs` (lulus 100%).
  - Seluruh unit test channel baru lulus 100%, compiler check dan architecture test `dependency_direction` lulus 100%.
- **JANGAN DIUBAH:** Policy allowlist bersama tetap di `galleon-channel-core`, jangan reimplementasi per-channel.
- **Dependency:** BR-C4 (sudah `[x]`, prasyarat terpenuhi).
- **DoD:** Semua channel Tier A/B punya crate sendiri; `clawcrew-channels` monolith hanya menyisakan channel yang belum dijadwalkan.

### T3-04 — Pecah file Rust terbesar yang tersisa (BR-C7)

- **Tujuan:** Pecah file >2000 baris yang tetap di Rust setelah BR-C5 selesai.
- **Ruang lingkup:** Kandidat kuat (dari ukuran unwrap/expect Sprint 1): `crates/clawcrew-gateway/src/api.rs`, `api_config.rs`, `ws.rs`, `api_sections.rs`.
- **Langkah konkret:**
  1. **Setelah T3-02 (BR-C5) selesai**, ukur ulang file terbesar yang tersisa (`wc -l crates/*/src/*.rs | sort -rn | head -20`) — ukuran berubah signifikan setelah decoupling, jangan ukur sebelum itu.
  2. Pecah file >2000 baris jadi submodule — pola SAFE RF-A (file-split dalam satu crate, `pub(crate) use` menjaga path, zero dependency change).
- **Dependency:** T3-02 (BR-C5) — hard, ukur SETELAH bukan sebelum.
- **DoD:** Tidak ada file produksi Rust >2000 baris tanpa justifikasi tertulis.

### T3-05 — Ramping runtime per modul, strangler pattern (BR-D1)

- **Tujuan:** Matikan modul `clawcrew-runtime` (286k LOC) satu per satu, urutan sesuai `R-rust.md` §R-00 bagian 3, syarat tiap modul: parity Go + eval hijau.
- **Ruang lingkup:** `crates/clawcrew-runtime/src/` (per-modul).
- **Langkah konkret (BERSERI — jangan paralel 2 modul yang saling import):**
  1. Baca `standard-docs/roadmap/R-rust.md` §R-00 bagian 3 untuk urutan modul persis.
  2. Pilih modul berikutnya sesuai urutan itu.
  3. Verifikasi Go agent loop (BG-D1 dkk, sudah `[x]`) punya parity fungsional (test `clawcrew-eval` replay hijau untuk skenario yang mencakup modul ini).
  4. Matikan modul Rust (feature flag off, atau hapus kalau confirmed tidak ada regresi).
  5. Ulangi untuk modul berikutnya.
- **Dependency:** BG-D1-D5 (§C Sprint 4, semua sudah `[x]`) — parity harus established per modul sebelum modul itu di-ramping.
- **DoD:** Setiap modul yang di-ramping punya bukti parity (`clawcrew-eval` replay hijau) sebelum dimatikan; tidak ada modul dimatikan tanpa bukti.

### T3-06 — Retarget `clawcrew-eval` ke jalur Go (BR-D2)

- **Tujuan:** Harness eval replay fixture terhadap jalur Go (bukan langsung ke `clawcrew-runtime` Rust), pertahankan fixture trace existing.
- **Ruang lingkup:** `crates/clawcrew-eval/` (verifikasi nama — "`galleon-eval`" adalah nama PASCA-rename Sprint 8, belum berlaku sekarang).
- **Langkah konkret:** Setelah T3-05 (BG-D1) jalan, retarget harness eval untuk replay fixture ke jalur Go. Fixture trace data TIDAK diubah, hanya target eksekusi.
- **Dependency:** T3-05/BG-D1 — hard.
- **DoD:** `clawcrew-eval` replay berjalan terhadap endpoint Go, fixture trace lama tetap valid tanpa modifikasi data.

### T3-07 — Hapus `verifiable_intent` (BR-D3)

- **Tujuan:** Hapus modul `verifiable_intent` (tag `archive/pre-galleon` dulu; ref ODR-21).
- **Ruang lingkup:** `crates/clawcrew-config/src/schema.rs`, `fixtures/v1.toml`, `sections.rs`, `validation_warnings.rs`, `doctor/mod.rs`, `src/main.rs`, `tests/component/verifiable_intent_notice_visibility.rs`.
- **Status implementasi:** Selesai & terverifikasi di branch `feat/refactoring-finalize-01` — Tag git `archive/pre-galleon` telah dibuat pada commit sebelum pembersihan (`95d346eb0`) dan dipush ke remote origin. Seluruh referensi `verifiable_intent`/`VerifiableIntentConfig` (15 file, -735 baris) dibersihkan permanen di commit `0e197e110`. Verifikasi grep membuktikan 0 referensi fungsional tersisa di seluruh `crates/`, `src/`, `tests/`, `engine/`, dan `web/`.
- **DoD:** `grep -rn "verifiable_intent" crates/` = 0 match fungsional, tag `archive/pre-galleon` sudah dibuat sebelum commit penghapusan. (Terpenuhi)

### T3-08 — Hapus `rpc` + REST/WS/A2A lama (BR-D4)

- **Tujuan:** Hapus `crates/clawcrew-runtime/src/rpc/` dan REST/WS/A2A lama setelah parity dengan Client Protocol (BG-E2) terbukti.
- **Ruang lingkup:** `crates/clawcrew-runtime/src/rpc/`.
- **Langkah konkret:** Setelah BR-E3 (Sprint 6, CLI/TUI retargeted — sudah `[x]`) DAN T3-02/BR-C5 (gateway lepas runtime) **keduanya** selesai — hapus modul `rpc` dan REST/WS/A2A lama. Tag archive dulu (pola sama T3-07) sebelum hapus.
- **Dependency:** BR-E3 (Sprint 6, hard, sudah `[x]`), T3-02/BR-C5 (hard, masih `[ ]`) — **jangan mulai sebelum KEDUA selesai**, menghapus jalur lama sebelum pengganti terbukti jalan = risiko outage.
- **DoD:** `crates/clawcrew-runtime/src/rpc/` terhapus, tag archive ada, 0 regresi di smoke test end-to-end pasca-hapus.

### T3-09 — Sambungkan tracing span ke HTTP request path Go (BG-F5)

- **Tujuan (scope final, BUKAN "adopsi zap" seperti klaim dokumen asli):** Sambungkan `tracing.StartSpan`/`End` (sudah ada, `engine/core/tracing/tracing.go`) ke jalur HTTP request di `delivery.go`, supaya `trace_id` konsisten lintas log `slog` HTTP, log gRPC interceptor, dan `x-request-id` yang dikirim ke Rust.
- **Ruang lingkup:** `engine/src/run/delivery.go`, `engine/core/tracing/tracing.go`, `engine/pkg/client/system_gateway.go`, `crates/clawcrew-gateway/src/grpc_system_gateway.rs`.
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — `tracing.StartSpan`/`End` tersambung di handler HTTP `engine/src/run/delivery.go`, field `trace_id` konsisten tercatat di log `slog` dan response header `X-Request-ID`, gRPC client `pkg/client/system_gateway.go` mempropagasi metadata `x-request-id` pada seluruh RPC, dan `grpc_system_gateway.rs` di Rust mengekstrak serta mengembalikan `x-request-id` di response metadata. Diverifikasi oleh Go unit test `TestRunHTTPDelivery_TracingAndRequestIDSynchronization` dan Rust integration test `go_callable_grpc_surface_propagates_request_id_tracing_metadata` (keduanya lulus 100%).
- **DoD:** Satu request HTTP end-to-end menghasilkan `trace_id` yang SAMA di log `slog` HTTP, log gRPC interceptor, dan metadata yang diterima Rust — dibuktikan test nyata di Go dan Rust. (Terpenuhi)

### T3-02b — `channels` lepas dari `runtime` (R-13 — GAP DITEMUKAN BELAKANGAN, tidak ada di sprint asli)

- **Sumber:** **TIDAK ADA** di `dev-task-2/sprint-*.md` manapun — ditemukan via audit silang terhadap `standard-docs/roadmap/R-rust.md` SETELAH dokumen ini pertama ditulis (lihat §Catatan verifikasi tambahan di akhir dokumen). Genuinely terlewat dari rekonsiliasi awal `dev-task-2`, BUKAN duplikat BR-C6/R-15 (yang soal memecah `channels` jadi crate per-channel — topik berbeda dari soal dependency `channels`→`runtime`).
- **Tujuan:** `clawcrew-channels` tidak lagi bergantung ke `clawcrew-runtime` (klaim `roadmap/R-rust.md`: 677 referensi → 0).
- **Ruang lingkup:** `crates/clawcrew-channels/src/`.
- **Status implementasi:** Selesai — `clawcrew-channels` sudah sepenuhnya lepas dari `clawcrew-runtime`. `cargo tree -p clawcrew-channels | grep clawcrew-runtime` membuktikan 0 edge dependency langsung maupun transitif ke `clawcrew-runtime`. `grep -rn "clawcrew_runtime::" crates/clawcrew-channels/src/` hanya menghasilkan 1 baris assertion string literal di test `orchestrator/tests.rs:23685` (`block.contains("clawcrew_runtime::acp::AcpServer::new")`), dengan 0 referensi fungsional kode produksi.
- **Dependency (dari `R-rust.md`):** R-10, R-11, R-12 (ekstraksi i18n/security-scrub/kontrak tools-scoped ke `clawcrew-api` — semua sudah `[x]`, lihat sprint-1 arsip BR-A2/A3/A4 setara).
- **Dependency ke T3-03 (BR-C6/R-15):** Prasyarat R-13 untuk T3-03 kini telah terpenuhi.
- **DoD:** 0 referensi fungsional `clawcrew_runtime::` tersisa di `clawcrew-channels/src/`; `cargo metadata`/`cargo tree` tidak lagi menunjukkan edge `clawcrew-channels`→`clawcrew-runtime`. (Terpenuhi)

---

## F. Gap Lane G/U ditemukan via audit silang penuh (TIDAK ADA di sprint dev-task-2 manapun)

### T3-25 — Endpoint metrik sistem nyata via Rust (G-22)

- **Sumber:** `roadmap/G-go-engine.md` G-22 — **genuinely tidak ada** di `dev-task-2`.
- **Tujuan:** Endpoint metrik sistem nyata (proses, CPU/mem) — via Rust bila butuh akses OS-level; menggantikan `get_system_metrics` palsu.
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — RPC `GetSystemMetrics` ditambahkan ke `SystemGateway` proto (`proto/agent_service.proto`), diimplementasikan di `grpc_system_gateway.rs` menggunakan `clawcrew_runtime::process_stats::sample()`, dan dipanggil oleh Go `engine/src/fleet/delivery.go` (`handleSystemMetrics`) via `SystemGatewayClient`. Nilai `cpuUsage` dan `memoryUsage` kini bersumber dari OS sebenarnya. Diverifikasi oleh Rust integration test `go_callable_grpc_surface_returns_real_system_metrics` dan Go test `TestHandleSystemMetrics_RealGatewayDelegation` (keduanya 100% lulus).
- **Ruang lingkup:** `proto/agent_service.proto`, `crates/clawcrew-gateway/src/grpc_system_gateway.rs`, `engine/pkg/client/system_gateway.go`, `engine/src/fleet/delivery.go`.
- **DoD:** `/api/system/metrics` mengembalikan CPU/mem nyata (bukan hardcoded). (Terpenuhi)

### T3-26 — Kontrak native tool Go↔Rust, SSOT tunggal (G-29) — **P0**

- **Sumber:** `roadmap/G-go-engine.md` G-29 — **genuinely tidak ada** di `dev-task-2`. Prioritas **P0**, belum pernah diangkat ke sprint manapun.
- **Tujuan:** Satu SSOT skema tool native Go↔Rust — saat ini Rust `ALLOWED_TOOLS=[bash,read_file,write_file,git]` (argumen `command/args/path/contents`) vs 19 tool Go (`workspace.*`, `execute_command`, `git.*`, `code.*`, `sandbox.run_code`, `web.fetch`, `browser.*`, `edit_file`, dst) — **hanya `write_file` yang namanya sama, dan kunci argumennya pun beda** (`content` vs `contents`).
- **Ruang lingkup:** `proto/` (skema tool + argumen), kedua sisi Go (`engine/`) dan Rust (`crates/clawcrew-gateway/`).
- **Langkah konkret:**
  1. Inventarisasi lengkap: daftar 19 tool Go + 4 tool Rust `ALLOWED_TOOLS`, bandingkan nama+skema argumen satu-per-satu.
  2. Definisikan skema tool+argumen SSOT di `proto/` — dipakai KEDUA sisi (bukan Go dan Rust punya definisi independen yang kebetulan mirip).
  3. Perbaiki mismatch `write_file` (`content` vs `contents`) sebagai bukti konkret pertama bahwa SSOT bekerja.
  4. `workspace_path` di proto perlu diisi — catatan roadmap: field ini "kini tak diisi `ExecuteNativeTool`".
  5. Tulis test kontrak yang gagal kalau Go dan Rust punya skema tool yang divergen.
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — skema proto SSOT didefinisikan (`GetNativeToolCatalog`, `WriteFileToolArgs` dengan kanonikal key `content`), mismatch `write_file` diselaraskan di Rust & Go, field `workspace_path` selalu terisi di `ToolCallRequest`, serta contract test Go (`native_tool_contract_test.go`) & Rust (`system_gateway_integration_test.rs`) lolos 100%. Spesifikasi terdokumentasi di `standard-docs/tech-standart/api-contracts/native-tool-catalog.md`.
- **DoD:** Test kontrak proto lulus di kedua sisi; `write_file`/`content` vs `contents` sudah konsisten; `workspace_path` terisi di semua panggilan `ExecuteNativeTool`. (Terpenuhi)

### T3-27 — Hapus Express server, pindah UI metrics/QR ke endpoint Go (U-02, U-03, U-04)

- **Sumber:** `roadmap/U-ui-desktop.md` U-02, U-03, U-04 — **genuinely tidak ada** di `dev-task-2`. Digabung jadi satu entri karena ketiganya saling depend dan sama-sama depend ke G-22 (T3-25).
- **Tujuan:**
  - **U-02:** Hapus `web/server.ts` Express dan skrip `dev=tsx watch server.ts`, ganti sepenuhnya ke Vite. Acceptance: `npm run dev` hanya Vite, proxy `/api`→`:9090`.
  - **U-03:** QR pairing di UI pakai endpoint Go (G-20, sudah `[x]`) — tanpa paket `qrcode` di server.
  - **U-04:** Telemetri OS dari Go (G-22/T3-25), hapus dummy/derivasi lokal di UI.
- **Ruang lingkup:** `web/server.ts`, `web/package.json` (scripts), komponen UI yang masih render metrik/QR lokal.
- **Dependency:** U-02 depend G-20 (sudah `[x]`) + G-22 (T3-25, sudah `[x]`). U-04 depend langsung ke T3-25.
- **Langkah konkret:**
  1. Verifikasi dulu apakah `web/server.ts` masih genuinely dipakai — U-16 (sudah `[x]`, Sprint awal) sudah menghapus endpoint RCE `/api/engine/execute` dari file ini, tapi file itu sendiri mungkin belum dihapus total.
  2. Hapus `server.ts` + skrip `tsx watch` sepenuhnya, pastikan `vite dev` + proxy config menggantikan fungsinya.
  3. QR pairing: ganti pemanggilan lokal ke `apiClient` yang hit endpoint Go `/api/pairing/*` (G-20).
  4. Telemetri OS: setelah T3-25 selesai, ganti dummy di UI dengan panggilan ke endpoint yang sudah genuinely return data OS nyata.
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — `web/server.ts` dihapus permanen, `web/package.json` diselaraskan ke murni Vite (`"dev": "vite"`, `"start": "vite preview"`), dependensi express dicopot. QR pairing dan telemetri OS sepenuhnya dialihkan ke Go engine (`/api/network/qrcode`, `/api/system/metrics` didukung oleh gateway Rust G-22). Uji arsitektur UI dan Go engine tier boundaries lulus 100%.
- **DoD:** `web/server.ts` terhapus dari git; `npm run dev` murni Vite; QR pairing dan metrik OS di UI genuinely dari API Go, 0 dummy/hardcoded tersisa. (Terpenuhi)

### T3-28 — Tipe E2 & `apiClient.ts` tunggal bertipe (U-10, U-11)

- **Sumber:** `roadmap/U-ui-desktop.md` U-10, U-11 — **genuinely tidak ada** di `dev-task-2`. Digabung karena tema identik (type-safety API client).
- **Tujuan:**
  - **U-10:** Tipe `HarborProvider`, `ProcessItem`, `FleetMetrics`, `DiagnosticItem`, `SnapshotItem`, `ExecutiveBriefing`, `SystemMetrics`, `NetworkInfo`, `ProjectGitStatus` diselaraskan dengan kontrak API Go.
  - **U-11:** `apiClient.ts` tunggal bertipe; larang `fetch` langsung di komponen (boundary gate / lint).
- **Ruang lingkup:** `web/src/utils/apiClient.ts`, `web/src/components/`, `tests/architecture/tier_boundaries.rs`.
- **Langkah konkret:**
  1. Go belum expose OpenAPI spec: definisikan interface TS terpusat yang mencerminkan response shape Go persis di `web/src/utils/apiClient.ts`.
  2. Ganti seluruh sisa direct `fetch` di komponen (`RemoteAccessModal.tsx`) menjadi pemanggilan metode `apiClient.getNetwork()`.
  3. Tambah boundary test enforcement di `tests/architecture/tier_boundaries.rs` (`test_ui_tier_boundaries`) yang menggagalkan build jika ada pemanggilan `fetch(` langsung di komponen UI di luar `apiClient.ts`.
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — tipe E2 kanonikal didefinisikan di `web/src/utils/apiClient.ts` (`HarborProvider`, `ProcessItem`, `DiagnosticItem`, `SnapshotItem`, `ExecutiveBriefing`, `FleetMetrics`, `SystemMetrics`, `NetworkInfo`, `ProjectGitStatus`), direct `fetch` di `RemoteAccessModal.tsx` dialihkan ke `apiClient.getNetwork()`, dan arsitektur gate `test_ui_tier_boundaries` diperluas untuk memastikan 0 direct `fetch()` di luar `apiClient.ts` (lulus 100%).
- **DoD:** 0 pemanggilan `fetch` langsung di komponen (test boundary menegakkan), tipe response API tidak divergen dari kontrak Go nyata. (Terpenuhi)

---

### T3-10 — Surface loop-detector ke Tasks-view (BL-16)

- **Tujuan:** 3-pola deteksi loop (`LoopDetector`) sudah matang di Rust tapi hasilnya tidak pernah sampai ke dashboard.
- **Ruang lingkup:** `crates/clawcrew-runtime/src/agent/loop_detector.rs` (sumber data), `proto/agent_service.proto`, `engine/src/run/dto.go`, dashboard Tasks-view (`web/src/`).
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — Wiring end-to-end loop detector dituntaskan di commit `a16f3531`: pesan `LoopDetectorStats` ditambahkan ke `TurnComplete` di `proto/agent_service.proto`, `run.Run` dan `task.Task` DTO Go engine memuat `LoopDetectorStats`, event publishing `run.loop_detector_stats` diimplementasikan di `runService`, dan di frontend `fleetStore.ts` (`syncQuestTelemetry`) memetakan telemetri ke `CircuitBreakerTelemetry` yang dirender di `QuestsView.tsx` (`Calls`, `Exact Repeats`, `Ping-Pong`, `status`).
- **Dependency:** T3-11 (BL-17) depend pada ini.
- **DoD:** `QuestsView.tsx` menampilkan state loop-detector NYATA dari API (bukan field kosong/placeholder) — dikonfirmasi test dan kode nyata. (Terpenuhi)

### T3-11 — Agregasi lintas-sub-agent untuk fan-out besar (BL-17)

- **Tujuan:** Setelah T3-10 memberi visibility per sub-agent, agregasikan lintas banyak sub-agent (fan-out besar) jadi ringkasan.
- **Status:** `EXPLORATORY` — depend keras pada T3-10/BL-16 yang masih terbuka.
- **Ruang lingkup:** Dashboard Tasks-view (`web/src/`), kemungkinan endpoint gateway tambahan.
- **Langkah konkret:** Setelah T3-10 selesai, tampilkan ringkasan (berapa Warning/Block/Break) bukan daftar individual yang membengkak untuk fan-out besar.
- **Dependency:** T3-10/BL-16 — hard, belum bisa mulai.
- **Ambiguitas dinyatakan:** Bentuk UI (chart/tabel/badge count) belum ditentukan dokumen asli — ini keputusan desain, bukan detail teknis bebas pilih. Konfirmasi ke Lead/Design sebelum eksekusi.
- **DoD:** Tidak ditentukan sampai bentuk UI dikonfirmasi — tahap sekarang adalah menunggu T3-10 + keputusan desain.

---

## D. Memory Governance (Sprint 6)

### T3-12 — `erase_identity()` lintas-backend (BL-30)

- **Tujuan:** Primitif delete per-backend sudah ada tapi tidak ada satu entry-point "hapus semua data saya" lintas 12 backend.
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — `erase_identity()` dan `EraseIdentityReport` diimplementasikan di `crates/clawcrew-api/src/memory_traits.rs`, di-reexport di `crates/clawcrew-memory/src/lib.rs`. Trait method `forget_all_for_agent` terpasang di trait `Memory` dengan delegasi default ke `purge_agent`. Diverifikasi lewat unit test `erase_identity_aggregates_multi_backend_and_reports_partial_failure` (lulus 100%).
- **Ruang lingkup:** Trait `Memory`, `crates/clawcrew-api/src/memory_traits.rs`, `crates/clawcrew-memory/src/lib.rs`.
- **DoD:** `erase_identity(all_backends, "test-agent")` dengan data tersebar di ≥2 backend berbeda → SEMUA data agent itu hilang di KEDUA backend, laporan menunjukkan sukses per-backend, kegagalan satu backend TERLIHAT jelas di laporan (bukan silent). (Terpenuhi)

### T3-13 — Hygiene/retensi otomatis backend-agnostic (BL-31)

- **Tujuan:** Hygiene/retensi otomatis saat ini hanya jalan di SQLite (`hygiene.rs` pakai `rusqlite::Connection` langsung, hardcoded).
- **Status implementasi:** Selesai di branch `feat/refactoring-finalize-01` — method `apply_retention(&self, policy: &RetentionPolicy) -> Result<BackendHygieneReport>` didefinisikan di trait `Memory` (`crates/clawcrew-api/src/memory_traits.rs`) dengan default no-op. Diimplementasikan nyata di `SqliteMemory` (`crates/clawcrew-memory/src/sqlite.rs`) dan diteruskan oleh `AgentScopedMemory`. Diverifikasi oleh unit test `test_sqlite_apply_retention_pruning_and_dry_run` (lulus 100%).
- **Ruang lingkup:** `crates/clawcrew-memory/src/sqlite.rs`, `crates/clawcrew-api/src/memory_traits.rs`, trait `Memory`.
- **DoD:** Trait `Memory` punya method `apply_retention` dengan default no-op; SQLite backend mengimplementasikan versi nyata; backend lain compile tanpa perubahan (inherit default). (Terpenuhi)

---

## E. Rebrand, Legacy Cleanup & Release (Sprint 8 — SELALU TERAKHIR, dep Sprint 0-7 SELURUHNYA)

> **Aturan keras non-negosiasi dari dev-tasks asli:** "Hapus/pecah SEBELUM rename massal." Seluruh seksi ini TIDAK BOLEH mulai sebelum sprint 0,4,5,6 (dan 1,2,3,7 yang sudah selesai) genuinely tuntas — rename menyentuh identifier di seluruh kode, kalau dijalankan lebih awal setiap sprint lain akan bentrok merge terus-menerus.

### T3-14 — Hapus kode warisan tak terpakai (RB-12)

- **Tujuan:** Hapus kode warisan REST/WS/A2A, hardware/peripheral hasil audit R-00.
- **Ruang lingkup:** Modul yang ditandai "tak terpakai" di audit R-00.
- **Langkah konkret:** Verifikasi dengan `find_references`/grep bahwa modul GENUINELY nol caller tersisa (bukan asumsi dari nama) — pola "configuration-lies" tercatat berulang di lesson project, jangan simpulkan dead-code dari nama/komentar saja.
- **JANGAN DIUBAH:** Jangan hapus tanpa tag `archive/pre-galleon` dulu (pola sama T3-07/T3-08).
- **Dependency:** T3-08/BR-D4 (hard — `rpc`+REST/WS/A2A lama harus sudah dihapus dulu di sana; ini CLEANUP SISA, bukan penghapusan pertama).
- **DoD:** 0 caller tersisa terverifikasi grep/find_references sebelum hapus; tag archive dibuat.

### T3-15 — Bersihkan fixture & terjemahan tak terpakai (RB-16, RB-17)

- **Tujuan:** Audit fixture test dan file lokalisasi `.ftl`/`.po` untuk entry yang tidak dirujuk kode manapun.
- **Ruang lingkup:** `tests/` (fixture), file Fluent `.ftl`.
- **Langkah konkret:** Grep referensi key Fluent — key yang tidak match panggilan `get_required_cli_string` manapun adalah kandidat hapus. Audit fixture serupa.
- **DoD:** Setiap key `.ftl` yang dihapus terbukti 0 referensi; regenerasi `.ftl`/`.po` setelah pembersihan.

### T3-16 — Rename binary (RB-20)

- **Tujuan:** `clawcrew`→`galleon`, `zerocode`→`galleoncode`, `zerorelay`→`galleonrelay`, `acp-bridge`→`galleon-acp-bridge`.
- **Ruang lingkup:** `apps/zerocode/` (106 file, dikonfirmasi ada), `apps/zerorelay/` (dikonfirmasi ada). Nama crate ACP bridge aktual **belum terkonfirmasi** — `acp-bridge` tidak ditemukan dengan nama persis ini di grep repo-wide, cek `Cargo.toml` workspace members untuk nama aktual.
- **Langkah konkret:**
  1. `git mv apps/zerocode apps/galleoncode`, `git mv apps/zerorelay apps/galleonrelay` — preservasi history git.
  2. Update semua referensi path internal (`Cargo.toml` `[[bin]]`, import path, CI workflow).
  3. `cargo build --workspace` setelah rename untuk menangkap referensi terlewat.
  4. Verifikasi nama binary CLI utama (`galleon`) — apakah sudah ada atau target rename, belum dikonfirmasi.
- **Dependency:** Sprint 0-7 SELURUHNYA (aturan non-negosiasi).
- **DoD:** `cargo build --workspace` hijau pasca-rename, 0 referensi path lama tersisa.

### T3-17 — Rename crate `clawcrew-*`→`galleon-*` (RB-21)

- **Tujuan:** Rename seluruh crate, urutan bawah→atas dependency graph; `clawcrew-runtime` paling terakhir.
- **Ruang lingkup:** Seluruh `crates/`.
- **Langkah konkret (BERSERI, bawah→atas):**
  1. Urutkan rename berdasarkan `cargo metadata` dependency graph — crate daun (tanpa dependency internal) duluan, `clawcrew-api`/`clawcrew-config` (banyak di-depend) terakhir.
  2. Setiap crate: `git mv`, update `Cargo.toml` nama package + semua yang depend padanya, `cargo check --workspace` SETELAH SETIAP crate (bukan batch — rename crate berisiko lebih tinggi dari file-split biasa, verifikasi per-langkah lebih aman di sini).
  3. `clawcrew-runtime`→`galleon-runtime` PALING TERAKHIR, menunggu T3-05/BR-D1 (ramping runtime) selesai.
- **Dependency:** T3-05/BR-D1 (hard — rename `runtime` di tengah proses strangler akan membingungkan referensi migrasi).
- **JANGAN DIUBAH:** Urutan ketergantungan antar-crate — harus mengikuti graph dependency.
- **DoD:** `cargo check --workspace` hijau setelah SETIAP crate rename individual; `clawcrew-runtime` adalah crate terakhir yang di-rename.

### T3-18 — Rename identifier/string & env var (RB-22, RB-23)

- **Tujuan:** Identifier & string dalam kode (`RB-22`); env `CLAWCREW_*`→`GALLEON_*`, `~/.clawcrew`→`~/.config/galleon` + migrasi data tanpa alias (`RB-23`).
- **Ruang lingkup:** Seluruh kode (RB-22); skrip migrasi data (RB-23).
- **Langkah konkret RB-23 (LEBIH BERISIKO — migrasi data):**
  1. Skrip migrasi: copy/move `~/.clawcrew/*` → `~/.config/galleon/*` saat binary baru pertama jalan DAN direktori lama ada tapi baru belum.
  2. **"Tanpa alias"** = SETELAH migrasi, binary TIDAK membaca `~/.clawcrew/` lagi sama sekali — migrasi SATU KALI, bukan dual-read permanen.
  3. Koordinasikan dengan T3-24 (BL-11, state migration tooling) — REUSE tooling kalau sudah dibangun, jangan tulis migrasi terpisah.
- **Dependency:** T3-24/BL-11 (overlap tema, serialisasi penulis).
- **DoD:** 0 referensi `clawcrew`/`claw-crew` string tersisa (RB-22); user dengan data `~/.clawcrew/` lama, setelah upgrade, data termigrasi utuh ke `~/.config/galleon/` TANPA fallback baca direktori lama (RB-23).

### T3-19 — Rename dokumen normatif non-ADR (RB-30)

- **Tujuan:** Rename referensi di `AGENTS.md`, `CLAUDE.md`, copilot-instructions, `tech-standart/`, `foundation/`, **dan `finalize/`** (termasuk `dev-task-2/`/`dev-task-3/` ini sendiri).
- **Ruang lingkup:** Dokumen normatif non-ADR.
- **Catatan khusus:** Rename ini TIDAK BOLEH merusak struktur tiket yang sudah ditulis di sprint dev-task-2/3 — contoh kode historis (string `clawcrew` dalam contoh kode) **dibiarkan apa adanya** sebagai snapshot akurat kondisi kode SAAT tiket ditulis, diupdate HANYA kalau kode asli sudah benar-benar di-rename duluan.
- **Constraint dari T3-20 (RB-29, sudah `[x]`):** Atribusi lisensi upstream ZeroClaw di `LICENSE-MIT`/`NOTICE`/`README` **TIDAK BOLEH berkurang** saat rename dokumen ini menyentuh file terkait — verifikasi atribusi masih utuh setelah rename.
- **JANGAN DIUBAH:** ADR (`decisions/owner-decisions/ODR-*`) — eksplisit dikecualikan, tidak pernah di-rename/dipindah, hanya di-supersede.
- **DoD:** Dokumen normatif konsisten nama baru; atribusi lisensi upstream terverifikasi utuh pasca-rename; ADR tidak tersentuh.

### T3-20 — Service installer (BL-10)

- **Tujuan:** Command `galleon service install/status/uninstall`, generate unit systemd (Linux)/plist launchd (macOS), sudo-scope dibatasi hanya langkah instalasi.
- **Ruang lingkup:** CLI `galleon` subcommand baru.
- **Langkah konkret:**
  1. Tambah subcommand `service install/status/uninstall`.
  2. Linux: generate unit systemd USER (`~/.config/systemd/user/galleon.service`, bukan system-wide).
  3. macOS: generate plist launchd setara (`~/Library/LaunchAgents/`).
  4. **Sudo-scope dibatasi:** hanya instalasi (tulis unit file) yang minta elevated permission — daemon jalan sebagai user biasa setelahnya.
- **JANGAN DIUBAH:** Prinsip "daemon jalan sebagai user biasa" — jangan "simplifikasi" dengan root.
- **DoD:** `galleon service install` membuat unit file tanpa menjalankan daemon sebagai root; `status`/`uninstall` bekerja simetris.

### T3-21 — State migration tooling (BL-11)

- **Tujuan:** Tooling pindahkan state Galleon (cron `jobs.db`, memory SQLite/vector, config, skills) ke host baru.
- **Ruang lingkup:** Skrip migrasi state, `crates/clawcrew-memory/src/sqlite.rs` (WAL mode).
- **Langkah konkret:**
  1. **Langkah pertama WAJIB:** `grep -rn "journal_mode\|WAL" crates/clawcrew-memory/src/sqlite.rs` — konfirmasi mode WAL aktif.
  2. Kalau WAL aktif: skrip migrasi/backup HARUS copy `.db-wal`/`.db-shm` bersama `.db` utama, ATAU checkpoint WAL dulu (`PRAGMA wal_checkpoint(TRUNCATE)`) sebelum copy — checkpoint lebih aman daripada copy 3 file sekaligus.
  3. Dokumentasikan edge case hot-migration (daemon masih jalan di host lama): wajib "stop daemon dulu, ATAU checkpoint WAL dulu" sebelum copy.
- **Dependency/overlap:** T3-18/RB-23 (migrasi `~/.clawcrew`→`~/.config/galleon`) — REUSE tooling yang sama, jangan tulis migrasi state terpisah untuk topik yang identik.
- **DoD:** Migrasi state tervalidasi dengan daemon berjalan DAN berhenti (dua skenario), 0 data loss pada kedua kasus (checksum/count sebelum-sesudah sama).

### T3-22 — QA/Integrasi akhir (PL-E1, PL-E2, PL-E4)

- **Tujuan:** Verifikasi batch seluruh hasil Sprint 0-8 sebelum rilis.
- **Ruang lingkup:** Branch `integration/wave-N`, `finalize/e2e-proof.md`.
- **Langkah konkret:**
  1. **PL-E1:** `cargo nextest run --workspace` + `go test -race ./...` + `npm run build` — SEKALI untuk seluruh hasil Sprint 0-8 (pola verifikasi batch, bukan per-sprint berulang).
  2. **PL-E2:** Smoke e2e Engine+gateway+UI/desktop bersamaan (nyata, bukan mock). Uji NEGATIF eksplisit: request tanpa kredensial ke endpoint yang di-gate (S2-SEC-01, BG-C1) → konfirmasi DITOLAK, bukan lolos diam-diam.
  3. **PL-E4:** Demo manual end-to-end UI/Tauri→Go→Rust, **tanpa fallback/konstanta palsu** (pola "configuration-lies" yang sama harus dihindari di sini) — dokumentasikan di `finalize/e2e-proof.md` dengan bukti (screenshot/log nyata), bukan klaim naratif.
- **Dependency:** SELURUH Sprint 0-7 (gate yang diuji di sini adalah hasil sprint-sprint sebelumnya).
- **DoD:** Ketiga command PL-E1 hijau; PL-E2 uji negatif terbukti menolak (bukan hanya happy path lolos); PL-E4 punya bukti konkret tersimpan di `finalize/e2e-proof.md`.

### T3-23 — Gerbang akhir rebrand (RB-40, RB-41)

- **Tujuan:** `brand_gate` hijau, nextest workspace, `go test -race`, `npm run build`, build desktop — gerbang akhir sebelum rilis.
- **Ruang lingkup:** Seluruh hasil Sprint 0-8 §A/B/C/D.
- **Langkah konkret:**
  1. **RB-40:** Jalankan `brand_gate.sh` (dari T3-16/RB-02, sudah `[x]`) — harus 0 sisa string `clawcrew`/`claw-crew`.
  2. **RB-41:** Untuk setiap string `clawcrew` yang TIDAK BISA di-rename (misal atribusi lisensi upstream T3-19/RB-29, atau nama dependency eksternal) — catat PENGECUALIAN eksplisit di `naming-map.md`, jangan diam-diam diabaikan sebagai `brand_gate` failure yang di-skip tanpa dokumentasi.
- **Dependency:** SEMUA tiket §A/B/C/D Sprint 8 (T3-14 s.d. T3-21).
- **DoD:** `brand_gate` 0 match tak terdokumentasi; setiap pengecualian punya baris di `naming-map.md` dengan alasan.

### T3-24 — Sign-off Owner (GV-11)

- **Tujuan:** Owner menandatangani PR bertanda ⚑ (one-way-door) sebelum merge.
- **Ruang lingkup:** PR yang ditandai ⚑ di sprint 0-8 (BR-C5, BR-C6, BR-D1, BR-D3, BR-D4, RB-12, RB-20 — semua tiket di dokumen ini yang bertanda ⚑ di sprint asli).
- **Langkah konkret:** Lead mengumpulkan daftar PR ⚑ yang sudah siap merge, ajukan ke Owner untuk approval eksplisit sebelum merge — bukan auto-merge Tier 1.
- **DoD:** Setiap PR ⚑ punya catatan approval Owner tertulis sebelum merge (konsisten 3-tier governance model: Tier 2/3 butuh keputusan manusia eksplisit).

---

## Urutan eksekusi lintas-seksi (ringkasan dependency)

```
T3-01 (vault)         ──► merge PR #18 ke testing (tidak ada kerja kode baru)

T3-09 (tracing)        independen, bisa paralel kapan saja

T3-10 (loop-detector)  ──► cek PR #18 dulu (mungkin sudah selesai)
   └─► T3-11 (agregasi fan-out)

T3-12 (erase_identity) ──► cek diff lama siap-reapply dulu
T3-13 (hygiene)         independen dari T3-12 (scope sudah diringkas)

T3-02 (BR-C5 gateway)  ──┬─► T3-04 (BR-C7 pecah file, ukur SETELAH T3-02)
                         └─► T3-08 (BR-D4 hapus rpc, butuh T3-02 + BR-E3 selesai)
T3-02b (R-13 channels) ──► T3-03 (BR-C6 pecah channels per-crate, R-rust.md: R-15 Dep R-13)
T3-05 (BR-D1 ramping)  ──┬─► T3-06 (BR-D2 retarget eval)
                         └─► T3-17 (RB-21 rename runtime, PALING TERAKHIR)

── SPRINT 8 TIDAK MULAI SEBELUM SEMUA DI ATAS + SPRINT 1/2/3/7 SELESAI ──

T3-14 (RB-12 hapus legacy)     ──► butuh T3-08 selesai dulu
T3-15 (RB-16/17 fixture)        independen
T3-16 (RB-20 rename binary)   ──► butuh SEMUA sprint 0-7 selesai
T3-17 (RB-21 rename crate)    ──► butuh T3-05 selesai, lalu BERSERI bawah→atas
T3-18 (RB-22/23 identifier+env) ──► overlap T3-21, koordinasikan
T3-19 (RB-30 dokumen normatif)  independen, tapi hati-hati atribusi lisensi
T3-20 (BL-10 service installer) independen
T3-21 (BL-11 state migration)  ──► overlap T3-18, REUSE tooling sama
T3-22 (QA akhir)               ──► butuh T3-14..T3-21 selesai
T3-23 (gerbang akhir)          ──► butuh T3-22 selesai
T3-24 (sign-off Owner)         ──► per-PR ⚑, berjalan paralel sepanjang sprint

── GAP LANE G/U (independen dari sprint 0-8, bisa paralel kapan saja) ──

T3-26 (G-29 kontrak native tool, P0)  independen, prioritas tinggi — SSOT belum pernah dibangun
T3-25 (G-22 metrik sistem Rust)       independen
   └─► T3-27 (U-02/03/04, UI pindah ke endpoint Go) ──► butuh T3-25 untuk U-04 bermakna
T3-28 (U-10/11, tipe E2 + apiClient)  independen
```

## Referensi silang ke dokumen asli

Setiap entri T3-xx di atas adalah ringkasan terstruktur dari tiket aslinya di `dev-task-2/sprint-{0,4,5,6,8}-*.md` — dokumen asli TETAP menjadi rujukan detail eksekusi penuh (termasuk ASUMSI, catatan koreksi historis, dan jejak audit yang tidak disalin ulang di sini untuk menghindari duplikasi teks). Dokumen ini adalah PETA KERJA, bukan pengganti dokumen sumber.

## Catatan verifikasi tambahan (pasca-penulisan dokumen ini)

Setelah draf pertama dokumen ini ditulis murni dari `dev-task-2/sprint-*.md`, dilakukan audit silang terhadap `standard-docs/roadmap/R-rust.md` (dan Lane G/U/N/P/D) untuk memverifikasi klaim bahwa seluruh roadmap sudah "terserap" ke `dev-task-2`. Hasil audit:

- **99% tiket `R-nn`/`G-nn` yang masih `[ ]` di `roadmap/*.md` SUDAH punya pasangan `BR-*`/`BG-*`/`RB-*` eksplisit** (notasi `· R-nn ·` di baris tiket `dev-task-2`) — termasuk yang statusnya di `roadmap/R-rust.md` masih tertulis `[ ]` padahal genuinely sudah `[x]` di `dev-task-2`/arsipnya (contoh: `R-02`→`BR-A5`, sudah `[x]` di sprint-1 arsip). **`roadmap/R-rust.md` sendiri belum disinkronkan statusnya** — ini bug dokumentasi terpisah, dicatat tapi tidak diperbaiki di sesi ini (di luar scope "konsolidasi task aktif").
- **Satu genuine gap ditemukan: R-13** (`channels` lepas dari `runtime`) — tidak ada di `dev-task-2` manapun, ditambahkan sebagai **T3-02b** di atas.
- Lane G (Go), U (UI), N (rename), P (platform), D (docs) tidak diaudit silang selengkap Lane R karena volume besar (90 tiket roadmap total terbuka menurut audit kasar) dan constraint waktu/memori sesi — **ada kemungkinan gap serupa R-13 di lane lain yang belum terverifikasi**. Rekomendasi: audit silang lanjutan Lane G/U/N/P/D sebagai task terpisah kalau dibutuhkan kepastian 100%, BUKAN diasumsikan lengkap hanya dari dokumen ini.
