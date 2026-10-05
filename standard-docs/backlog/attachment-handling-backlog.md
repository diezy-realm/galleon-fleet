---
title: Backlog — Attachment Handling (Bug Fix, Multi-Format, 2-Layer Validation)
status: backlog
date: 2026-10-05
author: squad-lead
depends_on:
  - research/attachment-debounce-bug-and-layer-design.md
  - research/document-volume-concurrency-readiness-audit.md
---

# Backlog — Attachment Handling

> Diturunkan dari `research/attachment-debounce-bug-and-layer-design.md`. Nomor lanjutan dari BL-51 (`knowledge-base-citation-backlog.md`). Keputusan Achmad (2026-10-05): approve semua rekomendasi — bug fix diprioritaskan, limit count 10/attachment, desain 2-layer (Front Apps validasi awal + Core agregasi/resource-gate).

---

## BL-52 — PERBAIKI BUG: Attachment Hilang Diam-Diam Saat `MessageDebouncer` Menggabungkan Pesan Rapid (P0, M)

**Rujukan:** `research/attachment-debounce-bug-and-layer-design.md` §1 — bug terverifikasi sampai baris implementasi, BUKAN asumsi.

**Scope:** `MessageDebouncer` (`crates/clawcrew-infra/src/debounce.rs`) saat ini hanya mengakumulasi teks (`DebouncerEntry.messages: Vec<String>`, dikirim via `oneshot::Sender<String>`). Ubah agar menyimpan pasangan `(text, attachments: Vec<MediaAttachment>)` per pesan yang masuk window, dan hasil akhir berupa `DebouncedResult { combined_text: String, combined_attachments: Vec<MediaAttachment> }`. Di sisi pemanggil (`crates/clawcrew-channels/src/orchestrator/mod.rs`, fungsi `route_inbound_slot` baris ~1370 dan titik pembuatan `InboundSlot::Debounced` baris ~11670), `turn.msg.attachments` di-extend dengan `combined_attachments`, bukan hanya `turn.msg.content` yang di-overwrite.

**File ownership:** Backend-Rust — `crates/clawcrew-infra/src/debounce.rs` (struct + API), `crates/clawcrew-channels/src/orchestrator/mod.rs` (titik pemanggilan).

**Definition of Done:** Test yang mensimulasikan 3 pesan rapid berturut-turut dari sender yang sama, masing-masing dengan 1 attachment berbeda — setelah window debounce selesai, turn hasil gabungan punya SEMUA 3 attachment (tidak hanya attachment pesan pertama) DAN teks tergabung benar seperti sebelumnya (regresi nol untuk kasus teks-saja).

**Dependency:** Tidak ada — bug fix isolated ke dua file yang sudah teridentifikasi presisi.

**Risiko/Severity:** **P0 — bug data-loss nyata**, mempengaruhi SEMUA channel messaging yang pakai debounce (WhatsApp, Telegram, dll) hari ini, bukan hipotetis. Prioritaskan di atas item lain di backlog ini.

---

## BL-53 — Perluas `MediaKind` dengan `Document`/`Archive`/`Link` (P1, S)

**Rujukan:** §2 — `MediaKind` saat ini hanya `Audio|Image|Video|Unknown`, tidak ada kategori untuk dokumen/ZIP/URL eksternal.

**Scope:** Tambah varian `Document` (PDF/office), `Archive` (ZIP — diterima sebagai SATU attachment utuh, ekstraksi adalah tahap terpisah, lihat BL-56), `Link` (URL eksternal sebagai referensi teks, BUKAN fetch otomatis — isinya diambil nanti oleh tool `web_fetch` yang sudah ada saat agent benar-benar butuh, bukan saat ingest). Pertimbangkan juga `AttachmentSource` enum terpisah (`Base64 { mime_type, data } | Url { url } | FileRef { file_id }`) mengikuti pola `source.type` Claude API yang seragam — ini infrastruktur untuk BL-54/55 juga, bukan hanya kategorisasi.

**File ownership:** Backend-Rust — `crates/clawcrew-api/src/media.rs` (`MediaKind` enum), titik klasifikasi MIME yang sudah ada di `crates/clawcrew-tools/src/file_upload.rs` (reuse tabel MIME yang sudah ada, jangan duplikat).

**Definition of Done:** Test: file PDF/docx terklasifikasi `Document`; file `.zip` terklasifikasi `Archive`; teks yang terdeteksi sebagai URL (regex sederhana) terklasifikasi `Link`. Tidak ada regresi pada klasifikasi `Audio|Image|Video` yang sudah ada.

**Dependency:** Tidak ada hard dependency ke BL-52, tapi secara logis melengkapi (BL-52 memperbaiki ALUR agregasi, BL-53 memperluas JENIS yang bisa diagregasi).

**Risiko/Severity:** LOW-MEDIUM — penambahan tipe data, backward-compatible (varian baru di enum, tidak mengubah varian lama).

---

## BL-54 — Layer 1: Validasi Hard-Cap 10 Attachment di Setiap Front App Entry Point (P1, M)

**Rujukan:** §3 Layer 1 — keputusan Achmad: 10 attachment maksimal, berlaku di titik masuk SEBELUM bytes dikirim ke Core.

**Scope:** Setiap entry point channel (web upload widget, Tauri desktop file-picker, dan tiap channel messaging — WhatsApp/Telegram/Discord/Slack/dst di `crates/clawcrew-channels/`) menolak submit dengan pesan error eksplisit saat jumlah attachment dalam SATU submit/prompt > 10. Ini validasi PRESENTATION-LAYER — tidak menyentuh logic Core, murni gate di titik masuk masing-masing channel/app.

**File ownership:** Frontend (web upload widget), FE-Desktop (Tauri file-picker), Backend-Rust per-channel (`crates/clawcrew-channels/src/{whatsapp*,telegram,discord,slack,...}.rs` — titik validasi sebelum attachment diteruskan ke orchestrator).

**Definition of Done:** Test per-channel: submit dengan 11 attachment ditolak dengan pesan jelas ke user; submit dengan 10 attachment diterima.

**Dependency:** Tidak ada hard dependency — bisa paralel dikerjakan per-channel oleh squad berbeda (file ownership terpisah per channel, aman diserialkan ke beberapa developer).

**Risiko/Severity:** MEDIUM — perubahan tersebar di banyak file channel, tapi masing-masing kecil dan independen.

---

## BL-55 — Layer 2: Core Count+Bytes Enforcement Setelah Agregasi Debounce (P1, M)

**Rujukan:** §3 Layer 2 — menutup celah "user kirim banyak pesan rapid untuk mengakali total 10 per-pesan", dan resource-gate total bytes (kekhawatiran "over memory" Achmad).

**Scope:** SETELAH BL-52 (bug fix agregasi) selesai, tambah DUA pengecekan di titik `route_inbound_slot` saat debounce resolve: (1) **count enforcement**: kalau `combined_attachments.len() > 10` setelah gabungan dari beberapa pesan rapid, tolak/potong dengan pesan jelas ke user (bukan silent truncate) — ini menutup celah Layer 1 yang hanya validasi PER-PESAN, bukan per-TOTAL-WINDOW. (2) **bytes-cap per bucket**: tambah field `max_bucket_bytes` di `MessageDebouncer`/`DebouncerEntry`, akumulasi ukuran total attachment yang masuk window, tolak pesan baru yang akan membuat total bucket melebihi cap (misal default 200MB) — mencegah 10× file 50MB datang rapid bersamaan menahan 500MB di memori.

**File ownership:** Backend-Rust — `crates/clawcrew-infra/src/debounce.rs` (bytes-cap), `crates/clawcrew-channels/src/orchestrator/mod.rs` (count enforcement di `route_inbound_slot`).

**Definition of Done:** Test: 4 pesan rapid masing-masing 3 attachment (total 12) → ditolak/dipotong di 10 dengan pesan jelas, bukan diteruskan 12 ke agent. Test: pesan rapid dengan total ukuran lampiran melebihi `max_bucket_bytes` → pesan terbaru ditolak dengan pesan jelas, bucket yang sudah terkumpul tetap diproses (tidak membuang yang sudah valid).

**Dependency:** SETELAH BL-52 (butuh agregasi attachment yang sudah benar sebagai prasyarat untuk menghitung total).

**Risiko/Severity:** MEDIUM — titik integrasi ke jalur debounce yang sudah diperbaiki BL-52, perlu test regresi memastikan tidak merusak alur normal (di bawah cap).

---

## BL-56 — Ekstraksi ZIP dengan Zip-Bomb Protection (P3, EXPLORATORY, L)

**Rujukan:** §4 — tidak ada vendor API besar yang auto-extract ZIP; kalau galleon-fleet butuh ini, perlu resource-gate sendiri karena isi ZIP bisa jauh lebih besar dari ukuran terkompresi.

**Scope:** DITUNDA sengaja (YAGNI) — terima ZIP sebagai `MediaKind::Archive` utuh dulu (BL-53) tanpa ekstraksi otomatis. Item ini HANYA diangkat ke aktif kalau ada kebutuhan nyata (user melaporkan butuh agent membaca ISI ZIP, bukan hanya tahu ada ZIP). Kalau diaktifkan nanti: wajib ada cap rasio ekstraksi (compressed:decompressed) dan cap total ukuran hasil ekstraksi, mengikuti prinsip anti-zip-bomb standar (deteksi rasio kompresi anomali, batasi kedalaman nested-zip, batasi jumlah entry).

**File ownership:** TBD saat diaktifkan.

**Dependency:** BL-53 (butuh `MediaKind::Archive` ada dulu sebagai basis).

**Risiko/Severity:** LOW saat ini (tidak dikerjakan) — berpotensi HIGH kalau nanti dibangun tanpa zip-bomb protection yang benar.

---

## Urutan Eksekusi yang Disarankan

1. **BL-52** dulu, tanpa kecuali — bug data-loss nyata, prioritas P0.
2. **BL-53** paralel dengan BL-52 (file berbeda, independen) — infrastruktur tipe yang dibutuhkan BL-55/56.
3. **BL-54** paralel dengan BL-52/53 — independen, tersebar per-channel.
4. **BL-55** SETELAH BL-52 selesai (hard dependency).
5. **BL-56** tetap DITUNDA/EXPLORATORY — jangan assign ke squad kecuali ada kebutuhan nyata yang dikonfirmasi.

## Catatan Proses

Severity keseluruhan backlog ini LEBIH TINGGI dari backlog riset-eksternal lain di sesi ini (payment-lockout, token-efficiency, knowledge-base) karena BL-52 adalah **bug nyata yang sedang terjadi di produksi** (bukan preventif/opsional) — sejajar prioritas dengan BL-32 (WhatsApp signature) sebagai bug data-integrity yang harus ditangani segera.
