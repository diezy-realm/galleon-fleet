---
title: Backlog — Channel Signature Bug, CI/CD Windows Signing, Observability Content PII
status: backlog
date: 2026-10-05
author: squad-lead
depends_on: channel-signature-verification-audit.md, cicd-supplychain-audit-partial.md, observability-content-pii-audit.md
---

# Backlog — Channel Signature, CI/CD, Observability

> Diturunkan dari 3 dokumen riset. Semua melalui verifikasi berlapis (termasuk koreksi atas kesimpulan awal yang keliru) — lihat masing-masing dokumen untuk riwayat lengkap. Nomor lanjutan dari BL-31 (`backlog/providers-memory-governance-backlog.md`).

---

## BL-32 — Perbaiki/Pulihkan `verify_whatsapp_signature` dan `optional_channel_routes` yang Hilang (P0, HIGH)

**Rujukan riset:** `channel-signature-verification-audit.md`.

**Masalah (terverifikasi independen, grep seluruh repo):** `fn verify_whatsapp_signature` (dirujuk 9+ test di `clawcrew-gateway/src/tests.rs:5811-5956,7492`) dan `fn optional_channel_routes` (dirujuk drift-guard test `webhook_ingress.rs:959`) **tidak ada definisinya di manapun di codebase**. Drift-guard test `every_channel_webhook_route_is_classified_by_the_registry` akan panic (`.expect()` gagal) karena mencari teks fungsi yang tidak ada di `lib.rs`.

**Dampak:** Tidak diketahui pasti apakah WhatsApp HMAC verification (`X-Hub-Signature-256`) benar-benar berjalan di produksi — jaring pengaman otomatis yang seharusnya membuktikan ini sendiri rusak/tidak bisa compile.

**Scope (urutan wajib):**
1. Jalankan `cargo test -p clawcrew-gateway --features channel-whatsapp-cloud` untuk ground-truth pass/fail — ini HARUS dijawab dengan compiler, bukan baca kode lagi.
2. Cari riwayat git (`git log -p`/`git blame`) untuk tahu kapan/mengapa fungsi ini hilang — regresi refactor tidak lengkap, atau file salah dihapus.
3. Pulihkan/perbaiki implementasi, pastikan drift-guard benar-benar PASS (bukan cuma compile).
4. Tambah test integrasi level HTTP (POST ke route Axum WhatsApp dengan signature salah, assert 401) — bukan cuma unit-level.

**File ownership:** Backend-Rust — `crates/clawcrew-gateway/src/{tests.rs,webhook_ingress.rs,lib.rs}`.

**Definition of Done:** `cargo test -p clawcrew-gateway` lolos penuh termasuk drift-guard dan WhatsApp signature test; test integrasi HTTP baru membuktikan 401 untuk signature salah.

**Dependency:** Tidak ada — bisa langsung dikerjakan, ini verifikasi+fix, bukan desain baru.

**Risiko/Severity:** **HIGH** — bukan karena ada eksploitasi aktif terbukti, tapi karena jaring pengaman otomatis yang seharusnya mencegah regresi serupa (route baru lolos tanpa auth) saat ini tidak berfungsi.

---

## BL-33 — Windows Authenticode Code Signing untuk Release Desktop (P1, HIGH)

**Rujukan riset:** `cicd-supplychain-audit-partial.md`.

**Masalah:** `build-desktop-windows` job build `.msi`/`-setup.exe` lalu langsung upload — tidak ada `signtool`, tidak ada secret `WINDOWS_CERT*`, tidak ada field signing di `tauri.conf.json`. Berbeda dari macOS yang sudah punya signing+notarization lengkap (dengan graceful degrade kalau secret absen).

**Dampak:** Binary Windows unsigned → SmartScreen/Defender warning untuk SETIAP pengguna yang install — blocker UX adopsi nyata, terutama relevan kalau target produk end-user non-developer (sesuai arah produk Galleon) dan/atau open-source.

**Scope:** Tambah step signing di `build-desktop-windows` job (`release-stable-manual.yml`) — `WINDOWS_CERTIFICATE`(base64 PFX)+`WINDOWS_CERTIFICATE_PASSWORD` secrets, `signtool sign /fd SHA256 /tr <timestamp-url> /td SHA256` terhadap `.msi`/`-setup.exe`, dengan pola graceful-degrade SAMA seperti macOS (skip kalau secret absen, jangan gate build).

**File ownership:** Platform/DevOps — `.github/workflows/release-stable-manual.yml`.

**Definition of Done:** Release Windows ter-sign kalau secret tersedia; build tidak gagal kalau secret absen (graceful degrade); dokumentasi status signing di release notes.

**Dependency:** Perlu akuisisi/pengadaan sertifikat Authenticode (code signing certificate) — ini bisa jadi blocker non-teknis (biaya/waktu pengadaan), bukan blocker kode.

**Risiko/Severity:** HIGH untuk UX adopsi, tapi TIDAK urgent secara keamanan aktif — bisa dijadwalkan setelah BL-32.

---

## BL-34 — Verifikasi & Perbaiki Referensi `tauri.bundled.conf.json` yang Hilang (P1, MEDIUM)

**Rujukan riset:** `cicd-supplychain-audit-partial.md`.

**Masalah:** File ini dirujuk 3× (`release-stable-manual.yml:537,635,686`, `TESTING.md:56`, `prepare-kernel.sh:29`) tapi TIDAK DITEMUKAN di working tree pada 2 audit independen (sub-agent CI/CD dan sub-agent channel). Kalau genuinely hilang, 3 job desktop build (`build-desktop`, `build-desktop-linux`, `build-desktop-windows`) berisiko GAGAL di CI saat ini.

**Scope:** Baca `scripts/desktop/prepare-kernel.sh` lengkap untuk konfirmasi apakah file ini di-generate runtime sebelum build. Kalau tidak ada generator, buat file statis dengan `bundle.windows`/`nsis` config + slot signing (untuk BL-33).

**File ownership:** Platform/DevOps — `scripts/desktop/prepare-kernel.sh`, `apps/tauri/tauri.bundled.conf.json` (kalau perlu dibuat).

**Definition of Done:** Konfirmasi definitif file ini ada/di-generate dengan benar; 3 job desktop build terverifikasi tidak gagal karena file hilang.

**Dependency:** Sebaiknya sebelum/bersamaan BL-33 (kalau ini tempat signing config seharusnya berada).

**Risiko/Severity:** MEDIUM — bisa jadi CI sudah gagal diam-diam atau file di-generate dengan cara yang belum diverifikasi.

---

## BL-35 — Pin Git Dependency `esp-idf-*` ke Rev Spesifik (P2, MEDIUM)

**Rujukan riset:** `cicd-supplychain-audit-partial.md`.

**Masalah:** `firmware/esp32/Cargo.toml:21-26` — `esp-idf-sys`/`esp-idf-hal`/`esp-idf-svc` di-pin ke `git = "..."` TANPA `rev`/`branch`, mengambil HEAD default branch tiap resolve ulang. Supply-chain risk: upstream bisa push commit kompromais yang ter-pull otomatis.

**Scope:** Tambah `rev = "<commit-sha>"` spesifik untuk ketiga dependency.

**File ownership:** Backend-Rust/Firmware — `firmware/esp32/Cargo.toml`.

**Definition of Done:** Ketiga dependency terpin ke commit spesifik, build firmware tetap sukses.

**Dependency:** Tidak ada.

**Risiko/Severity:** MEDIUM — scope terbatas (firmware ESP32 saja, bukan desktop/core), tapi perbaikan murah.

---

## BL-36 — Cakupan `cargo deny` untuk Firmware (`esp32`/`nucleo`) (P2, MEDIUM)

**Rujukan riset:** `cicd-supplychain-audit-partial.md`.

**Masalah:** `deny.toml` root tidak mengaudit `firmware/esp32`/`firmware/nucleo` karena masing-masing punya `Cargo.lock` TERPISAH (bukan workspace member root) — blind-spot audit.

**Scope:** Buat `deny.toml` scoped untuk firmware ATAU extend CI `daily-audit.yml` dengan `--manifest-path firmware/esp32/Cargo.toml` dan `firmware/nucleo/Cargo.toml`.

**File ownership:** Platform/DevOps — `.github/workflows/daily-audit.yml`, kemungkinan `firmware/*/deny.toml` baru.

**Definition of Done:** Firmware crate tercakup audit dependency yang sama dengan workspace utama.

**Dependency:** Tidak ada.

**Risiko/Severity:** MEDIUM — blind-spot audit, bukan vulnerability langsung.

---

## BL-37 — Sambungkan API Redaksi Tool I/O yang Sudah Ada (Bug Wiring, Bukan Kekurangan Desain) (P1, HIGH)

**Rujukan riset:** `observability-content-pii-audit.md` (dikonfirmasi independen oleh 2 sub-agent audit terpisah).

**Masalah:** `capture_tool_input`/`capture_tool_output` (`clawcrew-log/src/tool_io.rs:24-61`) adalah API redaksi yang DIRANCANG BENAR — menghormati `log_tool_io` policy + truncate 40KB + denylist. **Tapi grep seluruh `crates/` membuktikan fungsi ini NOL dipanggil di produksi** — hanya dipakai di unit test crate itu sendiri. `tool_execution.rs:166-262` menulis `call_arguments`/`r.output` MENTAH ke `attrs` via `record!`, dan komentar kode SENDIRI (baris 166-168) eksplisit menyatakan niat "emit full input/output" — kontradiksi langsung dengan API redaksi yang dibangun terpisah dan tidak pernah disambungkan.

**Dampak:** Ini kelas bug "konfigurasi berbohong" — operator yang membaca `log_tool_io: redacted` di config percaya ada redaksi aktif, padahal fungsi yang menegakkannya tidak pernah dipanggil. LEBIH berbahaya dari tidak ada redaksi sama sekali karena menciptakan false sense of security. Isi percakapan user/LLM (lewat tool `send_message`, `artifact_post_comment`, `memory_recall`, dll) ikut ter-log penuh (truncated 40KB) ke `runtime-trace.jsonl`.

**Scope (2 tahap, urutan wajib):**
1. **[WAJIB DULU]** Ganti `call_arguments`/`r.output` mentah di `tool_execution.rs:166-262` dengan hasil `capture_tool_input`/`capture_tool_output` — ini bug wiring sederhana, PRIORITASKAN sebelum redesign apa pun. Perlu threading `ResolvedPolicy` ke call site yang saat ini tidak menerimanya.
2. **[SETELAH #1]** Pertimbangkan pisah dua axis policy jangka menengah: (a) technical tool I/O (shell, file ops) tetap default truncated; (b) conversational tool I/O (tool dengan parameter free-text percakapan) default `Off`, sejajar pola `log_llm_request_payload` yang sudah benar. Reuse `log_tool_io_denylist` yang sudah ada (`clawcrew-log/src/config.rs:24`) tapi defaultnya kosong.

**File ownership:** Backend-Rust — `crates/clawcrew-runtime/src/agent/tool_execution.rs:166-262` (tahap 1), `crates/clawcrew-config/src/schema.rs` (`LogToolIo` enum, tahap 2), `crates/clawcrew-log/src/{tool_io.rs,config.rs}` (tahap 2).

**Definition of Done:** Tahap 1 — `capture_tool_input`/`capture_tool_output` benar-benar dipanggil di `tool_execution.rs`, test baru membuktikan `log_tool_io=redacted` benar-benar truncate/redact sesuai policy (bukan lagi silent no-op). Tahap 2 — tool conversational default tidak simpan konten penuh.

**Dependency:** Tahap 1 tidak ada dependency, bisa langsung dikerjakan. Tahap 2 setelah tahap 1 selesai.

**Risiko/Severity:** **HIGH** — bug wiring nyata (bukan sekadar gap desain) yang membuat satu baris konfigurasi keamanan tidak berfungsi sama sekali, dikonfirmasi independen 2 kali.

---

## BL-38 — Terapkan `scrub_credentials` di Jalur `record!` Tool I/O (P1, MEDIUM)

**Rujukan riset:** `observability-content-pii-audit.md`.

**Masalah:** Scrub kredensial (`scrub_credentials`) saat ini HANYA diterapkan di jalur observer/model-visible, BUKAN di jalur `record!` yang menulis ke `runtime-trace.jsonl` (`tool_execution.rs:234-266`). Ini gap TERPISAH dari BL-37 (yang soal konten percakapan) — ini soal kredensial yang mungkin kebetulan ikut dalam tool input/output yang BUKAN tool conversational.

**Scope:** Terapkan `scrub_credentials` (yang sudah ada, `turn/redact.rs`) di titik `record!` sebelum `attrs` ditulis ke disk, independen dari BL-37.

**File ownership:** Backend-Rust — `crates/clawcrew-runtime/src/agent/tool_execution.rs:234-266`.

**Definition of Done:** Kredensial yang kebetulan ada di tool input/output ter-scrub sebelum tulis ke disk, test baru membuktikan.

**Dependency:** Tidak ada, independen dari BL-37 (bisa paralel).

**Risiko/Severity:** MEDIUM — melengkapi BG-F5/BR-B4 (dev-tasks) yang scope-nya kemungkinan tidak menyentuh titik spesifik ini.

---

## BL-39 — Proteksi Permission File Log untuk Windows (P2, MEDIUM)

**Rujukan riset:** `observability-content-pii-audit.md`.

**Masalah:** `writer.rs:403-420` set `0o600` HANYA di `#[cfg(unix)]` — tidak ada branch `#[cfg(windows)]` yang set ACL eksplisit. File `runtime-trace.jsonl` di Windows mewarisi permission default direktori induk.

**Scope:** Tambah `#[cfg(windows)]` branch yang set ACL agar hanya current user yang bisa baca (via `windows-acl` crate atau `std::os::windows::fs`), paralel dengan branch Unix yang sudah ada.

**File ownership:** Backend-Rust — `crates/clawcrew-log/src/writer.rs:403-420`.

**Definition of Done:** File log di Windows punya proteksi ACL setara `0o600` Unix; test baru (platform-conditional) membuktikan.

**Dependency:** Tidak ada.

**Risiko/Severity:** MEDIUM — relevan khusus untuk konteks pengguna Windows (shared machine/multi-user profile), risiko kecil di laptop single-user.

---

## Urutan Eksekusi yang Disarankan

1. **BL-32 dulu** — ini bukan "mungkin ada gap", ini bug nyata di jaring pengaman otomatis yang sudah terverifikasi independen. Jalankan test dulu untuk ground-truth sebelum apa pun lain.
2. **BL-37 kedua** — gap privasi konten percakapan, dampak langsung ke kepercayaan pengguna, perbaikan berdampak sedang (tidak butuh desain ulang besar).
3. **BL-34** — perlu dikonfirmasi SEGERA karena implikasinya "CI mungkin sudah gagal diam-diam" — murah untuk dicek.
4. **BL-38 dan BL-39** — bisa paralel dengan di atas, scope kecil masing-masing.
5. **BL-33** — penting untuk UX adopsi tapi butuh pengadaan sertifikat (non-teknis), jadwalkan tapi tidak blocking item lain.
6. **BL-35 dan BL-36** — firmware-scope, lower priority, bisa dikerjakan kapan saja tanpa dependency.

## Catatan Proses

Setiap item perlu diangkat ke `roadmap/*.md` sebelum di-assign ke squad. BL-32 khususnya — karena melibatkan verifikasi "apakah CI benar-benar hijau hari ini" — sebaiknya diprioritaskan sebagai pertanyaan yang dijawab SEBELUM item lain di backlog ini dikerjakan, supaya tim tidak membangun di atas asumsi yang mungkin salah.
