---
title: Audit & Desain — Attachment Loss di MessageDebouncer + Skema Multi-Format + Layer Validasi
status: proposal
date: 2026-10-05
author: squad-lead
depends_on: research/document-volume-concurrency-readiness-audit.md
feeds_into: backlog/attachment-handling-backlog.md
---

# Audit & Desain: Penanganan Attachment Lintas-Pesan, Multi-Format, dan Layer Validasi

## 1. Bug Terverifikasi: Attachment Hilang Saat `MessageDebouncer` Menggabungkan Pesan Rapid

**Status: BUG NYATA, terverifikasi sampai implementasi — bukan asumsi.**

Jejak verifikasi (`crates/clawcrew-channels/src/orchestrator/mod.rs`):

1. `MessageDebouncer` (`crates/clawcrew-infra/src/debounce.rs`) mengakumulasi **HANYA TEKS**: `struct DebouncerEntry { messages: Vec<String>, ... }`, hasil akhir dikirim lewat `oneshot::Sender<String>` — tidak ada field attachment sama sekali di tipe ini.
2. Saat pesan rapid pertama tiba, `InboundTurn` dibuat dari `msg` pesan **PERTAMA** (baris ~11670: `let inbound = InboundTurn { turn: Box::new(PendingTurn { ctx, msg, ... }), ... }`) dan disimpan sebagai carrier di `InboundSlot::Debounced { turn, content }`.
3. Komentar kode sendiri eksplisit mengonfirmasi desain ini (baris 11602-11604): *"the combined turn keeps the place of the **sender's first message**"* — pesan kedua, ketiga, dst dalam window yang sama HANYA menyumbang teksnya ke debouncer, `msg` aslinya (termasuk attachment-nya) tidak pernah disimpan.
4. Saat window debounce selesai (`route_inbound_slot`, baris 1370-1372): `turn.turn.msg.content = combined` — HANYA field `.content` (String) di-overwrite dengan teks gabungan. Field attachment di `turn.msg` tetap persis punya pesan PERTAMA.

**Konsekuensi konkret**: kalau user WhatsApp mengirim 3 pesan rapid berturut-turut — "ini laporannya" + [lampirkan file1.pdf] lalu "dan ini juga" + [lampirkan file2.pdf] — setelah debounce window selesai, teks tergabung jadi "ini laporannya\ndan ini juga", TAPI attachment yang sampai ke agent **HANYA file1.pdf** (dari pesan pertama). **file2.pdf hilang secara diam-diam, tanpa error apa pun.**

Ini pola bug "configuration-lies" ke-4 yang ditemukan sesi riset ini (setelah WhatsApp signature, capture_tool_input, query_by_similarity) — mekanisme (debounce) terlihat berfungsi dari luar (teks tergabung benar), tapi diam-diam membuang data (attachment) yang user kirim.

## 2. Skema Data Saat Ini vs Kebutuhan

**`MediaAttachment`/`MediaKind` saat ini** (`crates/clawcrew-api/src/media.rs`): `MediaKind` hanya punya varian `Audio | Image | Video | Unknown` — **tidak ada `Document`, `Archive`, atau `Link`** sebagai kategori attachment. `MarkerKind::Document` ada, tapi itu untuk disposisi RENDER teks (`[Document: name] <path>`), bukan kategori sumber attachment.

**Referensi skema industri (Gemini API + Claude API, riset 2026-10-05):**

| Vendor | Pola | Field Kunci |
|---|---|---|
| Gemini `Part` | Union: `inlineData` (base64) vs `fileData` (reference via Files API) | `Blob{mimeType, data}` / `FileData{mimeType, fileUri}` |
| Claude content block | `source.type`: `"base64" \| "url" \| "file"` — field SERAGAM untuk `image` dan `document` block | `{type, media_type, data\|url\|file_id}` |
| URL eksternal | Claude: didukung HANYA untuk `image` (`source.type:"url"`), TIDAK untuk `document`. Gemini: tidak ada field URL di `Part` sama sekali — URL ditangani via tool terpisah `url_context` (fetch server-side, bukan attachment schema) | — |
| Folder/ZIP | **Tidak ada dukungan native di API manapun** — keduanya single-file-per-upload; extract/split selalu di level APLIKASI (klien extract ZIP dulu, upload file individual) | — |

**Insight desain paling penting**: pola Claude (`source.type` seragam base64/url/file untuk SEMUA jenis attachment) lebih bersih daripada pola Gemini (union field berbeda nama per jenis data). Untuk Rust, pola Claude lebih mudah dipetakan ke satu `enum AttachmentSource { Base64 { mime_type, data }, Url { url }, FileRef { file_id } }` yang dipakai seragam lintas `MediaKind`.

## 3. Desain Layer (Sesuai Arahan Achmad)

**Prinsip**: dua layer terpisah dengan tanggung jawab berbeda — Front Apps (presentation) validasi count SEBELUM data masuk; Core Layer (`clawcrew-infra`) menjamin agregasi/integritas SETELAH data masuk. Front App yang "nakal" tidak bisa membebani Core; Core tidak bergantung pada disiplin Front App manapun (defense in depth).

### Layer 1 — Front Apps (Desktop/Web/Messaging): Validasi Count di Titik Masuk

- **Setiap entry point channel** (web upload widget, Tauri desktop file-picker, tiap channel messaging — WhatsApp/Telegram/Discord/dst) menerapkan **hard cap 10 attachment per single submit/prompt** — konsisten dengan klaster angka industri terverifikasi (Gemini App 10, dekat Claude.ai/Copilot ~20 tapi 10 dipilih sebagai "angka aman" sesuai keputusan Achmad).
- Validasi ini SEBELUM bytes dikirim ke Core — mencegah payload besar menyentuh backend sama sekali kalau sudah jelas melebihi batas di titik asal.
- **Pesan error eksplisit ke user** saat lewat batas (bukan silent drop) — konsisten pola industri yang semuanya reject-keras-dengan-pesan-jelas (bukan auto-drop diam-diam seperti bug debounce di atas).

### Layer 2 — Core (`clawcrew-infra`/`clawcrew-channels`): Agregasi & Integritas

- **Perbaiki `MessageDebouncer` agar mengakumulasi attachment, bukan hanya teks.** Ubah `DebouncerEntry.messages: Vec<String>` jadi menyimpan pasangan `(text, attachments)` per pesan, dan hasil akhir yang dikirim via oneshot adalah `DebouncedResult { combined_text: String, combined_attachments: Vec<MediaAttachment> }` bukan `String` polos. `route_inbound_slot` lalu meng-extend `turn.msg.attachments` dengan hasil gabungan, bukan hanya overwrite `.content`.
- **Core-level count enforcement SETELAH agregasi**: kalau gabungan dari beberapa pesan debounce ternyata total attachment-nya > 10 (misal user kirim 4 pesan rapid, masing-masing 3 file = 12 total), Core menolak/memotong dengan pesan jelas — ini mencegah celah "front app validasi per-pesan 10, tapi user kirim banyak pesan rapid untuk mengakali total jadi 50".
- **Perluas `MediaKind`** menjadi `Audio | Image | Video | Document | Archive | Link | Unknown` — `Document` untuk PDF/office, `Archive` untuk ZIP (ditangani sebagai SATU attachment utuh di level Core — ekstraksi isi ZIP, kalau diperlukan, adalah tahapan terpisah SETELAH diterima, bukan bagian dari validasi count awal), `Link` untuk URL eksternal yang dikirim user sebagai referensi (bukan file bytes).
- **Resource gate saat agregasi** (mencegah "over memory" seperti kekhawatiran Achmad): batasi total BYTES akumulasi per debounce bucket (bukan hanya count) — kalau user kirim 10 file masing-masing 50MB dalam window rapid yang sama, itu 500MB yang harus ditahan di memori sampai window selesai. Perlu cap total bytes per bucket, terpisah dari cap per-file yang sudah ada.

## 4. Mengapa ZIP dan Link Diperlakukan Beda (Berdasar Riset)

- **ZIP**: TIDAK ada vendor besar yang auto-extract di level API — pola yang benar adalah terima ZIP sebagai SATU attachment (`MediaKind::Archive`), validasi ukuran total di count-cap yang SAMA (1 ZIP = 1 slot dari 10, bukan dihitung sejumlah file di dalamnya saat validasi awal), lalu ekstraksi jadi tahap pemrosesan terpisah yang punya resource-gate sendiri (karena isi ZIP bisa jauh lebih besar dari ukuran terkompresi — risiko "zip bomb" harus diantisipasi kalau nanti dibangun).
- **Link/URL**: Claude mendukung ini HANYA untuk image (`source.type:"url"`), Gemini menanganinya sebagai tool terpisah (`url_context`, fetch server-side) BUKAN sebagai bagian dari attachment biasa. Untuk galleon-fleet: `MediaKind::Link` sebaiknya tidak langsung di-fetch isi URL-nya sebagai bagian dari agregasi attachment (beda tanggung jawab) — cukup disimpan sebagai referensi teks yang nanti **tool `web_fetch` yang sudah ada** (bukan tool baru) yang mengambil isinya saat benar-benar dibutuhkan agent.

## Rekomendasi Prioritas

1. **BUG FIX segera**: perbaiki attachment-loss di `MessageDebouncer` — ini bug data-loss nyata yang mempengaruhi SEMUA channel messaging yang pakai debounce (WhatsApp, Telegram, dll), bukan edge-case jarang.
2. **Layer 1 (Front Apps)**: validasi count=10 di titik masuk setiap channel — perubahan kecil per-channel, bisa paralel dikerjakan.
3. **Layer 2 (Core)**: perluas `MediaKind`, tambah resource-gate bytes-per-bucket di debouncer SETELAH bug fix #1.
4. **Opsional lanjutan**: ekstraksi ZIP dengan zip-bomb protection — tahap terpisah, didesain nanti kalau genuinely dibutuhkan (YAGNI untuk sekarang, cukup terima ZIP sebagai attachment utuh dulu).

## Sumber

- Internal: `crates/clawcrew-infra/src/debounce.rs`, `crates/clawcrew-channels/src/orchestrator/mod.rs` (baris 752-768, 1370-1372, 11590-11690), `crates/clawcrew-api/src/media.rs`.
- Eksternal (subagent, 2026-10-05): Gemini API `Content.Part`/`Blob`/`FileData` schema (ai.google.dev/api/generate-content), Claude API content block `source.type` schema (platform.claude.com/docs/en/build-with-claude/{vision,files}), konfirmasi nol dukungan native ZIP/folder di kedua API.
- Riset sebelumnya: `research/document-volume-concurrency-readiness-audit.md` (limit count 10-20 lintas produk).
