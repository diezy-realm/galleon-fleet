---
sprint: 8
title: Rebrand Massal, WIT v1, CI Cleanup, Release/Installer
status: planned
depends_on: Sprint 0-7 SELURUHNYA — aturan non-negosiasi dev-tasks asli (hapus/pecah SEBELUM rename massal)
---

# Sprint 8 — Rebrand & Release (SELALU TERAKHIR)

> **Aturan keras dari dev-tasks asli, dipertahankan:** "Hapus/pecah **sebelum** rename massal." Sprint ini TIDAK BOLEH mulai sebelum Sprint 0-7 selesai — rename menyentuh identifier di seluruh kode, kalau dijalankan lebih awal setiap sprint lain akan bentrok merge terus-menerus.

## A. Persiapan & Hapus Legacy Sisa

- [x] **RB-02 · N-02 · PR-80 (P1, M)** Skrip rename + `brand_gate.sh`.

#### Detail Eksekusi — RB-02

**Langkah pengerjaan:** Tulis skrip (shell/Python di `scripts/`) yang melakukan find-replace sistematis `clawcrew`→`galleon` (case-sensitive variants: `ClawCrew`→`GalleonFleet` PascalCase, `CLAWCREW`→`GALLEON` uppercase, `clawcrew`→`galleon` lowercase, `claw-crew`→`galleon-fleet` kebab-case — VERIFIKASI pola kapitalisasi dari `naming-map.md` existing sebelum menulis regex, JANGAN asumsikan hanya satu casing). `brand_gate.sh` adalah CI check yang gagal kalau menemukan sisa string `clawcrew`/`claw-crew` setelah rename selesai (dirujuk RB-40 sebagai bagian gerbang akhir).

**File & simbol spesifik:** `standard-docs/tech-standart/naming/naming-map.md` (dikonfirmasi ADA — ditemukan di grep sebelumnya) adalah SSOT pemetaan nama lama→baru, baca PENUH sebelum menulis skrip rename.

**JANGAN DIUBAH:** Isi semantik kode — ini MURNI rename identifier/string, perilaku program harus identik sebelum/sesudah (eksplisit di judul §B "mekanis, tidak campur perilaku").

- [ ] **RB-12 · N-12 · PR-62 (P1, M)** ⚑ Hapus kode warisan tak terpakai hasil R-00 (REST/WS/A2A, hardware/peripheral). Dep: `BR-D4` (Sprint 4, hapus `rpc`).

#### Detail Eksekusi — RB-12

**Dependency:** BR-D4 (Sprint 4, HARD — `rpc`+REST/WS/A2A lama harus sudah dihapus dulu di sana, tiket ini adalah CLEANUP SISA setelah itu, bukan penghapusan pertama).

**Langkah pengerjaan:** Verifikasi dengan `find_references`/grep bahwa modul yang ditandai "tak terpakai" GENUINELY nol caller tersisa (bukan asumsi dari nama) — pola bug "configuration-lies" sudah tercatat berulang di lesson project ini (jangan simpulkan dead-code dari nama/komentar saja).

**JANGAN DIUBAH:** Jangan hapus tanpa tag `archive/pre-galleon` dulu (pola sama BR-D3 Sprint 4) — preservasi history sebelum destructive delete.

- [ ] **RB-16 · N-16 · PR-19 (P2, S)** Fixture & contoh tak relevan.
- [ ] **RB-17 · N-17 · PR-19 (P2, S)** Terjemahan tak terpakai; regenerasi `.ftl/.po`.

#### Detail Eksekusi — RB-16/RB-17

**Langkah pengerjaan:** Audit fixture test (`tests/`) dan file lokalisasi (`.ftl` Fluent — dikonfirmasi dipakai AGENTS.md "`clawcrew_runtime::i18n::get_required_cli_string`") untuk entry yang genuinely tidak dirujuk kode manapun — verifikasi dengan grep referensi key Fluent sebelum hapus (key `.ftl` yang tidak match dengan panggilan `get_required_cli_string` manapun adalah kandidat hapus).

## B. Rename Massal (Gelombang 3 — mekanis, tidak campur perilaku)

- [ ] **RB-20 · N-20 · PR-83 (P1, M)** Binary: `clawcrew`→`galleon`, `zerocode`→`galleoncode`, `zerorelay`→`galleonrelay`, `acp-bridge`→`galleon-acp-bridge`.

#### Detail Eksekusi — RB-20

**Verifikasi path aktual:** `apps/zerocode/` dan `apps/zerorelay/` dikonfirmasi ada (106 file ditemukan Sprint 6) — rename direktori INI (bukan nama yang sudah berubah). `acp-bridge` TIDAK ditemukan dengan nama persis ini di repo (lihat Sprint 6 BR-E4 — nama crate ACP bridge aktual perlu dicek `Cargo.toml` workspace members).

**Langkah pengerjaan:**
1. `git mv apps/zerocode apps/galleoncode`, `git mv apps/zerorelay apps/galleonrelay` — PRESERVASI history git (`git mv`, bukan delete+create).
2. Update SEMUA referensi path internal (`Cargo.toml` `[[bin]]` names, import path, CI workflow yang merujuk path lama) — SETELAH `git mv`, jalankan `cargo build --workspace` untuk menangkap referensi yang terlewat.
3. Binary CLI utama (`galleon` — verifikasi nama binary saat ini, BR-E2 Sprint 6 menyebut CLI `galleon` tapi belum dikonfirmasi apakah ini nama SUDAH ada atau nama TARGET rename).

**Dependency:** Sprint 0-7 SELURUHNYA (aturan non-negosiasi dokumen ini) — JANGAN mulai sebelum semua sprint lain selesai.

- [ ] **RB-21 · N-21 · PR-82 (P1, L, berseri)** Crate `clawcrew-*`→`galleon-*` (bawah→atas; `runtime` setelah R-16/Sprint 4 selesai).

#### Detail Eksekusi — RB-21

**Dependency:** BR-D1 (Sprint 4, "R-16" — ramping runtime HARUS selesai sebelum crate `clawcrew-runtime` di-rename, karena rename `runtime` di tengah proses strangler akan membingungkan referensi "sedang dimigrasi dari crate mana").

**Langkah pengerjaan (BERSERI, bawah→atas dependency graph):**
1. Urutkan rename berdasarkan `cargo metadata` dependency graph — crate TANPA dependency internal lain (daun) di-rename DULU, crate yang banyak di-depend (`clawcrew-api`, `clawcrew-config`) di-rename TERAKHIR.
2. Setiap crate: `git mv crates/clawcrew-x crates/galleon-x`, update `Cargo.toml` nama package DAN semua `Cargo.toml` lain yang depend padanya, `cargo check --workspace` setelah SETIAP crate (bukan batch semua lalu check sekali — rename crate adalah kasus di mana verifikasi per-langkah lebih aman daripada edit-batch-build-sekali, KARENA setiap rename crate mengubah import path yang dirujuk crate lain, risiko salah lebih tinggi dari refactor file-split biasa).
3. `clawcrew-runtime`→`galleon-runtime` PALING TERAKHIR (dependency Sprint 4 BR-D1).

**JANGAN DIUBAH:** Urutan ketergantungan antar-crate — rename HARUS mengikuti graph dependency, bukan urutan alfabetis sembarangan.

- [ ] **RB-22 · N-22 · PR-82 (P1, M)** Identifier & string dalam kode.
- [ ] **RB-23 · N-23 · PR-81 (P1, M)** Env `CLAWCREW_*`→`GALLEON_*`, `~/.clawcrew`→`~/.config/galleon` + migrasi data (tanpa alias).

#### Detail Eksekusi — RB-22/RB-23

**Langkah pengerjaan RB-23 (migrasi data, LEBIH BERISIKO dari RB-22 identifier murni):**
1. Tulis skrip migrasi: copy/move `~/.clawcrew/*` → `~/.config/galleon/*` saat binary baru pertama jalan DAN mendeteksi direktori lama ada tapi baru belum.
2. **"Tanpa alias"** — berarti SETELAH migrasi berjalan, binary TIDAK membaca `~/.clawcrew/` lagi sama sekali (bukan dual-read permanen) — pola migrasi SATU KALI, bukan symlink/fallback selamanya.
3. Koordinasikan dengan BL-11 (Sprint 8 §D, state migration tooling) — kalau BL-11 sudah membangun tooling migrasi state generik, REUSE untuk RB-23, jangan tulis migrasi terpisah.

**Dependency:** BL-11 (§D sprint ini sendiri — overlap tema, lihat tabel File Ownership untuk serialisasi).

- [x] **RB-24 · N-24 · PR-86 (P1, M)** WIT v1 `wit/v1` `galleon:plugin@1.0.0`; v0 beku lolos `.frozen`. Pasangan `BR-F2` (Sprint 5).
- [x] **RB-25 · N-25 · PR-84 (P1, M)** Go module → `github.com/diezy-realm/galleon-fleet/engine`; regen wire/proto.
- [x] **RB-26 · N-26 · PR-85 (P1, S)** UI `web/` (lihat `FW-D1`).
- [x] **RB-27 · N-27 · PR-85 (P1, S)** Desktop (lihat `FD-C1`).
- [x] **RB-28 · N-28 · PR-87 (P1, M)** CI/scripts/xtask/Docker/Make; image `ghcr.io/diezy-realm/galleon-fleet`.
- [x] **RB-29 · N-29 · PR-87 (P2, S)** Metadata repo; pertahankan atribusi lisensi upstream.

#### Detail Eksekusi — RB-26/27/28/29

**RB-26/27:** Lihat detail di `FW-D1`/`FD-C1` di bawah (baris sendiri, dengan AC lebih spesifik) — tiket ini adalah REFERENSI silang, bukan kerja terpisah.

**RB-29 (atribusi lisensi — PENTING, perlu kehati-hatian):** `LICENSE-MIT`/`NOTICE`/`README` sudah punya atribusi ZeroClaw yang DIPULIHKAN per lesson tercatat (sesi 2026-10-05, "memulihkan atribusi ZeroClaw di LICENSE-MIT/NOTICE/README"). **JANGAN hapus/kurangi atribusi ini saat rename metadata repo** — eksplisit judul tiket "pertahankan atribusi lisensi upstream". Verifikasi SETELAH rename bahwa atribusi masih utuh (baca `LICENSE-MIT`/`NOTICE` setelah perubahan metadata, bandingkan dengan versi sebelum rename).

**JANGAN DIUBAH:** Atribusi lisensi upstream di `LICENSE-MIT`/`NOTICE` — ini bukan string "clawcrew" yang perlu di-rename, ini legal attribution yang HARUS tetap merujuk project asal.

- [ ] **RB-30 · N-30 · PR-87 (P2, M)** Dokumen normatif non-ADR (`AGENTS.md`, `CLAUDE.md`, copilot-instructions, tech-standart, foundation, **dan `finalize/`**).

#### Detail Eksekusi — RB-30

**Catatan khusus `finalize/`:** Direktori ini (termasuk `dev-task-2/` yang SEDANG diperluas oleh dokumen ini sendiri) akan ikut terkena rename massal — PASTIKAN rename ini TIDAK menghapus/merusak struktur tiket yang sudah ditulis sepanjang Sprint 0-7 (string `clawcrew`→`galleon` di CONTOH KODE dalam dokumen sprint ini sendiri perlu diputuskan: apakah contoh kode historis dibiarkan sebagai "snapshot nama lama" atau diupdate — REKOMENDASI: biarkan contoh kode di sprint file APA ADANYA karena itu snapshot akurat dari kondisi kode SAAT tiket ditulis, update HANYA kalau kode asli sudah benar-benar di-rename duluan).

**JANGAN DIUBAH:** ADR (`standard-docs/decisions/owner-decisions/ODR-*`) — eksplisit dikecualikan ("non-ADR"), ADR tidak pernah di-rename/dipindah, hanya di-supersede.

- [x] **FW-D1 · N-26 · PR-85 (P1, S)** `react-example`→`galleon-web`, "Fleet AI"→"Galleon Fleet", migrasi key localStorage.

#### Detail Eksekusi — FW-D1

**Langkah pengerjaan:**
1. Rename direktori/package `web/` (kalau nama internal masih "react-example" — verifikasi `package.json` nama field).
2. Find-replace string tampilan "Fleet AI"→"Galleon Fleet" di SELURUH `web/src/` (UI text, title tag, dll).
3. **Migrasi key localStorage:** kalau ada key localStorage browser yang memakai prefix lama (`fleet-ai-*`/`clawcrew-*`), tulis migrasi JS yang membaca key lama SEKALI saat load, copy ke key baru, hapus key lama — supaya user existing tidak kehilangan preference/state tersimpan browser.

**Acceptance criteria testable:** User dengan localStorage key lama yang sudah ada sebelum rename, setelah update ke versi baru, preference-nya TETAP ADA (bukan reset ke default).

- [x] **FD-C1 · N-27 · PR-85 (P1, S)** `fleet-ai-desktop`→`galleon-desktop`, `productName` "Galleon Fleet". Pertahankan `identifier = com.galleon.sovereign` (verifikasi).

#### Detail Eksekusi — FD-C1

**Langkah pertama WAJIB:** Verifikasi `identifier` di `apps/tauri/tauri.conf.json` (file dikonfirmasi ADA) — cek apakah SUDAH `com.galleon.sovereign` atau masih nilai lama. Tiket ini bilang "pertahankan" yang menyiratkan SUDAH benar — kalau ternyata BELUM, ini bukan "pertahankan" tapi "ubah", eskalasi perbedaan ini ke Lead.

**JANGAN DIUBAH:** `identifier` di `tauri.conf.json` kalau sudah benar — ini field krusial untuk OS (macOS bundle ID, Windows app identity) yang TIDAK BOLEH berubah antar versi rilis (user existing install akan dianggap app berbeda kalau identifier berubah).

## C. CI Cleanup & Platform

- [x] **PL-A5 · P-08 · PR-93 (P2, M)** Pangkas workflow warisan (39) & tandai required checks.

#### Detail Eksekusi — PL-A5

**TIDAK DIVERIFIKASI sesi ini** — jumlah workflow `.github/workflows/` (39, menurut dokumen) belum di-grep ulang. **Langkah pertama:** `read` direktori `.github/workflows/` untuk inventarisasi, cek mana yang masih relevan vs warisan (nama file lama yang menyebut `clawcrew`/branch yang sudah tidak dipakai).

- [x] **PL-B2 · P-02 + P-03 · PR-90 (P1, M)** Dockerfile & `include` `/web/dist`: sesuaikan stub `apps/tauri`; putuskan UI di-embed vs disajikan Nginx (ADR kecil).

#### Detail Eksekusi — PL-B2

**Catatan terkait temuan Sprint README:** `web-2/dist/` (11 file masih ter-commit git, item `RB-16b` disebut README §Verifikasi Tambahan) adalah SAMPAH REPO terpisah dari `/web/dist` yang relevan untuk tiket ini — JANGAN campur dua pembersihan ini, `RB-16b` (`git rm -r web-2/dist`) adalah tiket TERSENDIRI yang belum punya baris eksplisit di sprint manapun (perlu ditambahkan — lihat catatan di akhir file ini).

**Langkah pengerjaan:** Tulis ADR kecil (keputusan UI di-embed Docker image vs disajikan Nginx terpisah) SEBELUM mengubah Dockerfile — ini keputusan arsitektur kecil tapi perlu didokumentasikan, bukan diputuskan diam-diam dalam PR.

## D. Deploy & Rilis

- [x] **PL-C1 · P-10 · PR-91 (P1, L)** Nginx + TLS: reverse proxy ke Go :9090 (WS/SSE), HSTS, rate-limit, header keamanan. Dep: `BG-C3` (Sprint 0).

#### Detail Eksekusi — PL-C1

**Dependency:** BG-C3 (Sprint 0, hard — autentikasi sesi/device middleware harus ada sebelum expose publik via Nginx; reverse proxy TANPA auth layer di belakangnya adalah exposure risk).

- [x] **PL-C2 · P-11 · PR-91 (P2, M)** Panduan deploy lokal↔VPS/cloud, model ancaman, hardening; tetapkan peran `galleonrelay` vs Nginx.

#### Detail Eksekusi — PL-C2

**Dependency:** PL-C1 (hard, prasyarat teknis), BR-E4 (Sprint 6, `galleon-acp-bridge`/relay retargeted — perlu tahu peran final relay sebelum menulis panduan).

- [x] **FD-B1 · U-09 · PR-92 (P1, M)** Sidecar desktop: jalankan Go engine + Rust gateway (lifecycle, health, shutdown).
- [x] **FD-B2 · (ODR-01)** System tray, dialog file native, window — fitur OS shell, tiap command dicatat di allowlist.

#### Detail Eksekusi — FD-B1/FD-B2

**FD-B2 catatan keamanan:** "Tiap command dicatat di allowlist" — ini prinsip keamanan EKSPLISIT (ODR-01), BUKAN opsional. Setiap OS shell command baru (dialog file, dll.) yang ditambahkan HARUS didaftarkan ke allowlist existing (`apps/tauri/capabilities/default.json` dikonfirmasi ADA — cek struktur allowlist di file ini) SEBELUM dipanggil dari kode, bukan ditambah lalu allowlist menyusul.

**JANGAN DIUBAH:** Prinsip default-closed allowlist (`AGENTS.md`: "New external surfaces default closed").

- [x] **PL-D1 · P-12 · PR-93 (P2, L)** Pipeline rilis: desktop (MSI/NSIS, signing), server (Docker image, checksum, SBOM). Dep: FD-B1. **Sub-item eks-BL-33 (Windows Authenticode signing):** `signtool sign /fd SHA256 /tr <timestamp-url> /td SHA256` di job `build-desktop-windows`, graceful-degrade sama seperti macOS (skip kalau secret absen). Dep: pengadaan sertifikat (non-teknis, bisa jadi blocker waktu).

#### Detail Eksekusi — PL-D1

**Edge case penting (eksplisit di tiket):** Signing HARUS graceful-degrade (skip signing, build tetap lanjut TANPA sign) ketika secret sertifikat belum ada — JANGAN membuat CI gagal keras kalau sertifikat belum diadakan (ini blocker NON-TEKNIS, pengadaan sertifikat, bukan bug kode).

**Dependency:** FD-B1 (hard), pengadaan sertifikat Windows Authenticode (eksternal, non-teknis — Lead perlu melacak status pengadaan ini terpisah dari progress kode).

- [x] **FD-C2 · P-12 · PR-93 (P2, L)** Bundling Windows (MSI/NSIS, signing) bersama Platform. **Sub-item eks-BL-34 (`tauri.bundled.conf.json` hilang):** verifikasi SEGERA (murah) apakah file ini genuinely hilang atau di-generate runtime oleh `scripts/desktop/prepare-kernel.sh` — kalau hilang, 3 job desktop build berisiko gagal CI diam-diam.

#### Detail Eksekusi — FD-C2

**Verifikasi:** `scripts/desktop/prepare-kernel.sh` DIKONFIRMASI ADA (6 referensi `tauri.bundled.conf.json` di file ini sendiri) — INDIKASI KUAT file config ini DIGENERATE oleh script, bukan genuinely hilang. **Langkah pertama WAJIB tetap:** baca `prepare-kernel.sh` penuh untuk konfirmasi PERSIS apakah script ini membuat `tauri.bundled.conf.json` sebelum build (artinya "hilang" di git adalah NORMAL, by-design) — kalau konfirmasi, tiket ini TERTUTUP tanpa kerja kode (hanya dokumentasikan temuan). Juga cek `.github/workflows/release-stable-manual.yml` (6 ref juga) untuk konfirmasi job CI benar-benar memanggil `prepare-kernel.sh` sebelum build Windows.

**Acceptance criteria testable:** Baca `prepare-kernel.sh` + `release-stable-manual.yml`, tulis temuan definitif (file digenerate script vs genuinely missing) — TIDAK perlu kode baru kalau konfirmasi "digenerate by design".

- [x] **PL-D2 · P-13 · PR-94 (P3, M)** Installer pilih Tier B.

#### Detail Eksekusi — PL-D2

**Dependency:** BR-F4 (Sprint 5, "Pilih Tier B saat instalasi" — scope SAMA, kemungkinan PL-D2 adalah bagian installer dari BR-F4, koordinasikan supaya tidak dikerjakan dua kali oleh dua crew berbeda).

- [x] **BL-10 (MEDIUM, dari `automation-cron-webhook-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)** Service installer setara `kirocrew service install` — command `galleon service install/status/uninstall`, generate unit systemd (Linux)/plist launchd (macOS), sudo-scope dibatasi (hanya langkah instalasi, daemon jalan sebagai user biasa). Independen dari PL-D1/D2 (beda OS-integration layer), tapi cocok dikerjakan bersamaan karena temanya sama (deployment convenience).

#### Detail Eksekusi — BL-10

**Langkah pengerjaan:**
1. Tambah subcommand `service install/status/uninstall` ke CLI `galleon`.
2. Linux: generate unit file systemd (`~/.config/systemd/user/galleon.service` — USER unit, bukan system-wide, konsisten "daemon jalan sebagai user biasa").
3. macOS: generate plist launchd setara (`~/Library/LaunchAgents/`).
4. **Sudo-scope dibatasi:** HANYA langkah instalasi (menulis unit file ke lokasi yang mungkin perlu privilege) yang minta elevated permission — daemon ITU SENDIRI jalan sebagai user biasa setelahnya, JANGAN jalankan daemon sebagai root/sistem.

**JANGAN DIUBAH:** Prinsip "daemon jalan sebagai user biasa" — ini keputusan keamanan eksplisit di judul tiket, jangan "simplifikasi" dengan menjalankan sebagai root untuk convenience.

- [x] **BL-11 (MEDIUM, risiko data-loss operasional)** State migration tooling — dokumentasi/script untuk pindahkan state Galleon (cron `jobs.db`, memory SQLite/vector, config, skills) ke host baru. **Perhatian khusus:** verifikasi dulu apakah `clawcrew-memory` pakai SQLite WAL mode — kalau ya, sync WAJIB include `.db-wal`/`.db-shm` atau data terbaru hilang diam-diam.

#### Detail Eksekusi — BL-11

**Langkah pertama WAJIB:** `grep -rn "journal_mode\|WAL" crates/clawcrew-memory/src/sqlite.rs` untuk konfirmasi mode WAL aktif — kalau ya, SEMUA skrip migrasi/backup state HARUS copy `.db-wal`/`.db-shm` BERSAMA file `.db` utama (checkpoint WAL dulu sebelum copy adalah alternatif lebih aman — `PRAGMA wal_checkpoint(TRUNCATE)` sebelum copy, menghindari kebutuhan copy 3 file sekaligus).

**Dependency:** RB-23 (§B sprint ini sendiri, overlap tema migrasi — REUSE tooling yang sama, jangan tulis migrasi state terpisah untuk RB-23 vs BL-11).

**Edge case:** Migrasi saat daemon MASIH BERJALAN di host lama (hot migration) — WAL file sedang aktif ditulis, copy tanpa checkpoint berisiko data loss. Dokumentasi HARUS eksplisit: "stop daemon dulu, ATAU checkpoint WAL dulu" sebagai langkah wajib sebelum copy.

## E. QA/Integrasi Akhir & Gerbang Akhir

- [ ] **PL-E1 · PR-plan §4 (GW-6)** Branch `integration/wave-N`: `cargo nextest run --workspace` + `go test -race ./...` + `npm run build`.

#### Detail Eksekusi — PL-E1

**Langkah pengerjaan:** Buat branch integrasi, jalankan SEMUA 3 command (Rust nextest, Go race-detector test, npm build) — pola "verifikasi BATCH" dari lesson project ini (`cargo check --workspace` SEKALI, bukan per-crate serial) berlaku juga di sini: jalankan `cargo nextest run --workspace` SEKALI untuk SELURUH hasil Sprint 0-8, bukan per-sprint berulang.

**Acceptance criteria testable:** Ketiga command hijau tanpa error.

- [ ] **PL-E2** Smoke e2e: Engine + gateway + UI/desktop; uji negatif fail-closed.

#### Detail Eksekusi — PL-E2

**Langkah pengerjaan:** Jalankan Go engine + Rust gateway + UI/desktop bersamaan (end-to-end nyata, bukan mock), lakukan uji NEGATIF (bukan hanya happy path): request tanpa kredensial ke endpoint yang di-gate (S2-SEC-01, BG-C1, dll Sprint 0) → konfirmasi DITOLAK, bukan lolos diam-diam.

**Dependency:** SELURUH Sprint 0-7 (gate yang diuji negatif di sini adalah hasil sprint-sprint sebelumnya).

- [x] **PL-E3 · N-41 · PR-88** Smoke test upgrade data lama (`~/.clawcrew`) → baru.

#### Detail Eksekusi — PL-E3

**Dependency:** RB-23 (§B, migrasi data `~/.clawcrew`→`~/.config/galleon`) — test ini VERIFIKASI hasil RB-23 bekerja, bukan membangun migrasi baru.

**Acceptance criteria testable:** Setup instance lama dengan data di `~/.clawcrew/`, jalankan binary baru, konfirmasi data termigrasi ke `~/.config/galleon/` dengan isi IDENTIK (checksum/count data sebelum-sesudah sama).

- [ ] **PL-E4 · T-9.1 / CO-07** End-to-end demo manual: UI/Tauri → Go → Rust, tanpa fallback/konstanta palsu, bukti di `finalize/e2e-proof.md`.

#### Detail Eksekusi — PL-E4

**Catatan penting:** "Tanpa fallback/konstanta palsu" — ini prinsip verifikasi yang SAMA dengan lesson project ini berulang kali ("configuration-lies" bug pattern) — demo manual ini HARUS menunjukkan jalur NYATA (bukan data mock/hardcoded yang terlihat seperti berfungsi tapi sebenarnya tidak tersambung, pola BL-74 Sprint 7 placeholder yang sama).

**Langkah pengerjaan:** Dokumentasikan di `finalize/e2e-proof.md` setiap langkah demo dengan BUKTI (screenshot/log output nyata), bukan klaim naratif tanpa bukti — konsisten pola "verifikasi secara konkret" yang sudah ditegaskan berulang di lesson project ini.

- [x] **CO-05 (dipersempit) · T-6.9** Handler protokol stateful yang BERTAHAN (`sse`, `ws`, `ws_approval`, `ws_sop_runs`, `acp`) → `RouteHandler`. Hanya untuk handler yang tidak dihapus (lihat `CO-09` keputusan Owner).
- [x] **CO-06 (dipersempit) · T-6.10** Handler transport lanjutan yang BERTAHAN (`webhook_ingress`, `tls`, `version`, `node_tool`, `hardware_context`, dst). Hanya untuk handler yang tidak dihapus.

#### Detail Eksekusi — CO-05/CO-06

**Verifikasi:** `webhook_ingress` dikonfirmasi ADA dan MATANG (`crates/clawcrew-gateway/src/webhook_ingress.rs`, dibaca penuh Sprint 0 BL-32) — handler ini TERMASUK yang "bertahan" (tidak dihapus BR-D4/RB-12), konsisten tiket ini.

**Dependency:** Keputusan Owner `CO-09` (dirujuk tiket ini) menentukan handler mana yang dihapus vs bertahan — baca `CO-09` SEBELUM memutuskan handler mana yang perlu di-port ke `RouteHandler` di sini.

**JANGAN DIUBAH:** Handler yang SUDAH diputuskan dihapus (scope BR-D4/RB-12) — tiket ini HANYA untuk handler yang BERTAHAN.

- [x] **CO-08** Hapus artefak Phase 2 tertinggal: komentar/doc lama yang menyebut `web-2`.

#### Detail Eksekusi — CO-08

**Temuan terkait (dari README §Verifikasi Tambahan):** `web-2/dist/` (11 file build-artifact) masih ter-commit git — item ini dicatat README sebagai `RB-16b` TAPI **belum ada baris `[ ]` eksplisit di sprint manapun** (termasuk sprint ini). **Tim eksekusi WAJIB menambahkan baris `RB-16b` (`git rm -r web-2/dist`) sebagai bagian CO-08 ini** (temanya identik: bersihkan sisa `web-2`) — eksekusi `git rm -r web-2/dist` sebagai bagian tiket CO-08, bukan ditunda tanpa tiket.

**Langkah pengerjaan:**
1. `grep -rn "web-2" --include=*.md --include=*.rs --include=*.go` untuk inventarisasi SEMUA komentar/dokumen yang masih menyebut `web-2`.
2. `git rm -r web-2/dist` (RB-16b, digabung ke tiket ini).
3. Verifikasi `web-2/` direktori SELURUHNYA bisa dihapus total (dikonfirmasi README: hanya `dist/`+`node_modules/` tersisa, `node_modules/` sudah gitignored) — `git rm -r web-2/` kalau genuinely tidak ada source code tersisa.

- [ ] **RB-40 · N-40 · PR-88 (P1, M)** Gerbang akhir: `brand_gate` hijau, nextest workspace, `go test -race`, `npm run build`, build desktop.
- [ ] **RB-41 · N-41 · PR-88 (P2, S)** Catat sisa pengecualian di `naming-map.md`.

#### Detail Eksekusi — RB-40/RB-41

**Dependency:** SEMUA tiket Sprint 8 §A/B/C/D (gerbang akhir memverifikasi hasil SEMUA tiket rename/CI/deploy sudah selesai).

**RB-41:** Kalau ada string `clawcrew` yang TIDAK BISA di-rename (misal atribusi lisensi upstream — RB-29, atau nama upstream dependency eksternal yang bukan milik project ini) — catat PENGECUALIAN ini eksplisit di `naming-map.md`, JANGAN diam-diam dibiarkan sebagai `brand_gate` failure yang di-ignore tanpa dokumentasi.

- [ ] **GV-01 · D-01 · PR-00 (P0, S)** Commit terpisah per concern lalu PR dari `feat/refactoring-rust`.

#### Detail Eksekusi — GV-01

**Catatan branch (lihat README §Kebijakan Branch):** Dokumen ini sendiri (README `dev-task-2/`) mencatat branch kerja resmi adalah `feat/refactoring-finalize-01`, BUKAN `feat/refactoring-rust` yang disebut tiket GV-01 asli (nama lama dari dev-tasks sebelum konsolidasi `dev-task-2/`). **Verifikasi branch resmi SAAT INI** (cek `git branch`/README terbaru) sebelum PR — gunakan branch yang BENAR SAAT EKSEKUSI, bukan nama yang tercatat di tiket lama ini.

- [ ] **GV-11** ⚑ Sign-off Owner untuk PR bertanda ⚑ sebelum merge.
- [x] **GV-12** Principle re-verify semua `[x]` di `finalize/task-breakdown.md` memakai `testing-gate-checklist.md` (gate G0–G8) sebelum ditutup resmi — **termasuk re-verify `BR-F1` yang perlu dicentang `[x]` manual (lihat README §Reconciliation)**.

#### Detail Eksekusi — GV-11/GV-12

**GV-12 — tindakan konkret BR-F1:** README §Reconciliation sudah mengidentifikasi `BR-F1` sebagai duplikat `GV-07` yang sudah `[x]` selesai PR-21 — Principle WAJIB mencentang `BR-F1` `[x]` secara MANUAL di `dev-tasks/02-be-rust-core.md` (file asli, BUKAN `dev-task-2/`) sebagai bagian re-verify GV-12 ini, bukan membiarkannya tetap `[ ]` tak tercentang selamanya.

**JANGAN DIUBAH:** `dev-tasks/` asli secara UMUM TIDAK diedit oleh Lead (per README §Catatan Non-Negosiasi poin 2) — PENGECUALIAN SATU-SATUNYA adalah mencentang `[x]` BR-F1 ini, karena itu adalah KOREKSI faktual (kerja sudah selesai di tiket lain), bukan perubahan scope/kerja baru.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A/B | Seluruh repo (mekanis, skrip rename) — `RB-21` berseri bawah→atas | Lintas-tim, dikoordinasi Platform/DevOps |
| C | `.github/workflows/`, `Dockerfile` | Platform/DevOps |
| D | `nginx/`, `apps/tauri/`, `.github/workflows/release-stable-manual.yml` | Platform/DevOps + Backend-Rust (sidecar) |
| E | `tests/architecture/`, `finalize/e2e-proof.md` | QA + Lead (sign-off) |

## Keluaran Sprint

- Rename `clawcrew`→`galleon` tuntas di SELURUH kode tanpa alias (ODR-19).
- Release Windows+macOS+Linux ter-sign, installer siap distribusi.
- `brand_gate` + nextest workspace + `go test -race` + `npm run build` semua hijau sebagai gerbang akhir sebelum rilis resmi.
