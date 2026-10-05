---
title: Backlog — Automation, Cron, Webhook Gaps
status: backlog
date: 2026-10-05
author: squad-lead
derived_from:
  - research/automation-cron-webhook-audit.md
---

# Backlog — Automation, Cron, Webhook Gaps

> **Tujuan folder ini:** sama seperti backlog lain — menjembatani `research/` ke task-breakdown siap-bagi.
> **Belum ada di `roadmap/` resmi.** **TIDAK menyentuh `decisions/finalize/dev-tasks/`.**
>
> **Catatan (DIKOREKSI 2026-10-05):** Pertanyaan keamanan "apakah payload webhook eksternal di-sanitasi dari
> prompt-injection" semula dijawab "sudah terjawab tuntas" — setelah verifikasi lanjutan ("apakah channel
> non-SOP juga pakai `screen_event` yang sama"), ditemukan **GAP KRITIS**: mekanisme `ContentSafety` HANYA
> berlaku untuk jalur SOP (`sop/dispatch.rs`), SEDANGKAN pesan chat biasa dari channel (Telegram/Discord/dll,
> `clawcrew-channels` orchestrator) TIDAK melewati sanitasi apa pun. Lihat BL-14 di bawah — ini sekarang
> item PRIORITAS TERTINGGI di backlog ini, bukan catatan selesai.

---

## BL-14 — Terapkan `ContentSafety` ke Jalur Channel Orchestrator Biasa (PRIORITAS TERTINGGI)

**Rujukan riset:** `automation-cron-webhook-audit.md` §3.2c

**Masalah (terverifikasi, bukan asumsi):** `ContentSafety::screen_event`/`sanitize_untrusted` HANYA dipanggil di `crates/clawcrew-runtime/src/sop/{dispatch.rs,engine/mod.rs}`. Grep menyeluruh di `crates/clawcrew-channels/` (orchestrator yang menangani SEMUA pesan chat Telegram/Discord/Slack/dll) = 0 hasil untuk sanitasi/guard apa pun. `msg.content` dipakai mentah sebagai prompt agent di beberapa titik (`process_channel_message_body`, baris ~8854/9684/10118 — `history_user_content`, `user_msg`, `query`).

**Dampak:** Prompt-injection via pesan chat biasa (bukan via webhook/SOP trigger) TIDAK difilter sama sekali. Anggota grup Telegram/Discord manapun (bahkan yang legit/sudah lolos allowlist channel) bisa mengirim teks berisi instruksi berbahaya yang sampai mentah ke agent — "sudah lolos allowlist channel" TIDAK SAMA DENGAN "aman dari prompt-injection", karena kedua ancaman itu independen (satu soal siapa pengirim, satu soal apa isi pesannya).

**Scope dikonfirmasi (2026-10-05):**
- ✅ `crates/clawcrew-gateway/src/api_sop_webhook.rs` — **AMAN**, tidak terpengaruh. Endpoint ini "SOP-only", request yang tidak match SOP langsung 404 dan tidak pernah masuk jalur chat/LLM. Selalu lewat `dispatch_untrusted_fan_in` yang pakai `ContentSafety`.
- ❌ **SEMUA channel kena**, bukan cuma Telegram/Discord — dikonfirmasi langsung di `qq.rs` dan `wechat.rs` (Tier B/C): keduanya membangun `ChannelMessage` generik lalu kirim ke mpsc channel yang sama dipakai semua channel lain, tanpa jalur sanitasi sendiri. Bug ini ada di **satu titik tunggal** (`process_channel_message_body` di orchestrator), bukan tersebar per-channel.
- **Kabar baik:** karena bug-nya di satu titik, **satu perbaikan menutup gap untuk SEMUA channel sekaligus** (Tier A: Slack/WhatsApp/Discord/Telegram; Tier B/C: QQ/WeChat/Lark/DingTalk/dst.) — tidak perlu patch satu-satu per channel.

**Keputusan yang HARUS diambil dulu sebelum implementasi** (bukan langsung coding):
1. **Klarifikasi apakah ini disengaja atau oversight** — apakah pesan dari peer yang sudah lolos allowlist channel-level dianggap "trusted by definition" secara sadar? Kalau ya, kenapa SOP channel trigger (yang levelnya sama, lewat channel yang sama) tetap di-scan? Inkonsistensi ini perlu dijelaskan, bukan dibiarkan.
2. **Tentukan scope penerapan** — apakah SEMUA pesan channel perlu di-scan (berat secara performa, tiap pesan kena overhead regex+scan), atau hanya pesan dari sumber tertentu (grup vs DM, member baru vs lama, dst.)?

**Scope implementasi (setelah keputusan di atas):** Reuse `ContentSafety` yang SUDAH ADA (jangan bangun modul baru, SSOT) — panggil `sanitize_untrusted`/`screen_event` di titik `msg.content` sebelum dipakai sebagai prompt di `process_channel_message_body`, dengan `SopTriggerSource::Channel` yang sudah ada sebagai source type.

**File ownership:** Backend-Rust — `crates/clawcrew-channels/src/orchestrator/mod.rs` (titik pemakaian) + kemungkinan `crates/clawcrew-config/src/schema.rs` (field config baru kalau butuh toggle on/off per-channel, menyusul pola `SopConfig.untrusted_input_guard` yang sudah ada).

**Definition of Done:** Test baru menyusul pola `screen_event_blocks_untrusted_injection_when_configured` yang sudah ada di SOP — pesan channel berisi `"ignore all previous instructions"` di-block/di-sanitize sebelum jadi prompt, DAN pesan normal tetap lewat tanpa distorsi.

**Dependency:** Tidak ada teknis, tapi WAJIB keputusan arsitektur (poin 1-2 di atas) sebelum eksekusi — ini bukan task "tambah baris kode", ini keputusan keamanan produk.

**Risiko/Severity:** **HIGH** — ini bukan technical debt kosmetik, ini jalur prompt-injection yang terbuka nyata di kode produksi untuk kasus penggunaan paling umum (chat biasa), lebih luas dampaknya dari gap SOP webhook yang sudah tertutup.

---

## BL-10 — Service Installer Setara `kirocrew service install`

**Rujukan riset:** `automation-cron-webhook-audit.md` §2.1, §4

**Masalah:** Belum ditemukan ekuivalen registrasi systemd/launchd otomatis untuk daemon Galleon (gateway + scheduler). Kiro Crew punya `kirocrew service install` dengan sudo-scope dibatasi eksplisit (hanya `tee`+`systemctl`, bukan proses itu sendiri jalan sebagai root), auto-restart on crash, auto-start on boot.

**Scope:** Tambah command `galleon service install/status/uninstall` (atau nama setara) di binary CLI utama — generate unit file systemd (Linux) / plist launchd (macOS), dengan prinsip sudo-scope sama (hanya langkah instalasi unit butuh elevated privilege, proses daemon jalan sebagai user biasa).

**File ownership:** Backend-Rust (CLI/binary layer, kemungkinan `apps/zerocode` atau binary root `galleon`).

**Definition of Done:** `galleon service install` berhasil register unit, daemon survive SSH disconnect + restart setelah reboot (test manual di Linux VM minimal), `galleon service uninstall` bersih tanpa sisa.

**Dependency:** Tidak ada. Independen.

**Risiko/Severity:** MEDIUM — bukan bug, tapi gap deployment yang menghambat adopsi produk sebagai daemon 24/7 untuk end-user non-technical.

---

## BL-11 — State Migration Tooling (Cron DB + Memory DB + Config + Skills)

**Rujukan riset:** §2.1 (tabel path Kiro Crew)

**Masalah:** Belum ada dokumentasi atau tool untuk memindahkan state Galleon (cron jobs di SQLite, memory, config, skills) ke host baru. Kiro Crew punya tabel path eksplisit + script `sync-to-remote.sh` yang menangani atomic SQLite backup (termasuk WAL files) + port custom + config patching.

**Scope:**
1. Identifikasi SEMUA path state Galleon yang perlu disinkronkan (cron `jobs.db`, `clawcrew-memory` SQLite/vector files, `config.toml`, skill directories, vault).
2. Tulis dokumentasi di `standard-docs/` (tabel path + alasan, menyusul pola Kiro Crew) ATAU script `scripts/sync-to-remote.sh` setara (pilih sesuai keputusan Lead — dokumentasi dulu lebih murah, script lebih robust).
3. **Perhatian khusus WAL files** — kalau `clawcrew-memory` pakai SQLite dengan WAL mode (perlu verifikasi), sync harus include `.db-wal`/`.db-shm` atau data terbaru hilang (lesson langsung dari insiden yang Kiro Crew sendiri dokumentasikan).

**File ownership:** squad-lead (dokumentasi) atau Backend-Rust (script) — tentukan saat diangkat ke roadmap.

**Definition of Done:** Minimal dokumentasi tabel path lengkap di `standard-docs/`; idealnya script teruji migrasi end-to-end (cron job + memory tetap utuh di host baru).

**Dependency:** Tidak ada. Bisa paralel dengan BL-10.

**Risiko/Severity:** MEDIUM — gap operasional, bisa menyebabkan silent data loss (lesson/memory/cron hilang) kalau user migrasi host tanpa panduan.

---

## BL-12 — Pemisahan Eksekusi vs Delivery untuk SOP/Cron Trigger Eksternal

**Rujukan riset:** §3.2 (perbandingan `gh-aw` safe-outputs)

**Masalah:** Saat ini agent eksekusi DAN delivery output terjadi di proses yang sama (`execute_job_with_retry` → `deliver_and_classify_run_result` berurutan di fungsi yang sama). `gh-aw` memisahkan agent-eksekusi dari job-penerap-output (permission lebih sempit) sebagai salah satu dari 6 layer defense-in-depth mereka.

**Catatan penting:** Ini **bukan vulnerability konkret yang ditemukan** — `ContentSafety::screen_event` (BL terkait: sudah selesai, lihat catatan di atas) sudah menyaring input sebelum masuk agent. Ini adalah **pola arsitektur tambahan**, bukan perbaikan atas celah aktif.

**Scope:** Evaluasi dulu (bukan langsung implementasi) — apakah untuk SOP/cron yang dipicu trigger eksternal (webhook, channel), perlu tahap tambahan: agent generate output → simpan sebagai "proposed output" → proses TERPISAH (permission lebih sempit, mis. hanya boleh kirim ke channel yang didaftarkan, tidak bisa eksekusi tool lain) yang menerapkan delivery. Ini perubahan arsitektur SOP execution model yang cukup besar — perlu keputusan Lead/Owner sebelum masuk implementasi, karena menyentuh `SopRunAction::ExecuteStep` yang sudah punya invariant ketat (lihat riset SOP sebelumnya).

**File ownership:** squad-lead (evaluasi/design-spec dulu) sebelum assign ke Backend-Rust.

**Definition of Done (untuk tahap evaluasi):** Design-spec baru yang memutuskan: perlu/tidak perlu pemisahan ini untuk Galleon, dan kalau perlu, SOP mana yang wajib pakai (hanya trigger eksternal? semua?).

**Dependency:** Tidak ada untuk evaluasi. Implementasi (kalau diputuskan perlu) tergantung hasil evaluasi.

**Risiko/Severity:** LOW (saat ini) — defense-in-depth tambahan, bukan menutup celah aktif. Jangan over-prioritaskan di atas gap yang lebih konkret (BL-10/BL-11).

---

## BL-13 — Cost Cap Eksplisit per Cron-Job/SOP-Run

**Rujukan riset:** §3.3

**Masalah:** `warn_if_high_frequency_agent_job` hanya WARN (log), tidak membatasi. `gh-aw` punya cap eksplisit (per-run DAN per-hari per-workflow) yang MENOLAK eksekusi kalau budget terlampaui, plus integrasi OTLP untuk observability cost-vs-outcome.

**Scope:** Tambah field cap biaya di level job/SOP (mis. `max_runs_per_day`, atau integrasi dengan budget yang mungkin sudah ada di level lain — perlu dicek dulu apakah `fleet/policies.go` atau `clawcrew-config` punya konsep budget yang bisa di-reuse, BUKAN bikin sistem budget baru kalau sudah ada).

**File ownership:** TBD — tergantung hasil cek apakah ada sistem budget existing untuk di-reuse (Backend-Go kalau budget-nya kebijakan Fleet-level, Backend-Rust kalau murni di scheduler).

**Definition of Done:** Job yang melebihi cap ditolak dieksekusi (bukan cuma warn), dengan pesan jelas kenapa.

**Dependency:** **Verifikasi dulu** apakah sistem budget/quota sudah ada di tempat lain (`fleet/policies.go`, `SecurityPolicy.record_action()` yang sudah ditemukan punya `max_actions_per_hour` — ini MUNGKIN sudah cukup, perlu dicek sebelum nambah field baru yang duplikat).

**Risiko/Severity:** LOW-MEDIUM — kemungkinan sebagian sudah tercakup `SecurityPolicy` existing (`is_rate_limited()`, `record_action()` ditemukan di scheduler); task ini mungkin lebih kecil dari perkiraan setelah verifikasi.

---

## Urutan Eksekusi yang Disarankan

1. **BL-14 dulu, di atas semua** — gap keamanan nyata di jalur paling umum dipakai (chat biasa). Mulai dengan klarifikasi keputusan (poin 1-2 di BL-14), bukan langsung coding.
2. **BL-13** — mulai dengan VERIFIKASI apakah `SecurityPolicy.max_actions_per_hour` sudah cukup sebagai cost-cap (bisa jadi task ini selesai hanya dengan dokumentasi "sudah ada", bukan kode baru).
3. **BL-11** — gap operasional dengan risiko data-loss, lebih mendesak dari BL-10.
4. **BL-10** — deployment convenience, penting untuk adopsi produk tapi tidak urgent secara keamanan/data.
5. **BL-12** — evaluasi dulu, implementasi (kalau ada) paling akhir karena scope arsitektur besar.

## Riwayat Koreksi

- 2026-10-05: BL-14 ditambahkan setelah verifikasi lanjutan menemukan gap kritis (jalur channel biasa tidak pakai `ContentSafety`). Entri sebelumnya yang menyatakan "integrity filtering sudah selesai" dikoreksi — itu hanya benar untuk jalur SOP, tidak untuk jalur channel biasa.

## Catatan Proses

Sama seperti backlog lain: setiap item perlu diangkat ke `roadmap/*.md` sebelum di-assign ke squad.
