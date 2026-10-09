---
title: Backlog — Token/Credit Efficiency (Cache Coordination & Visibility)
status: backlog
date: 2026-10-05
author: squad-lead
depends_on: research/token-efficiency-codebase-audit.md
roadmap_refs: R-88, R-89, R-90, R-91 (standard-docs/roadmap/R-rust.md)
---

# Backlog — Token/Credit Efficiency

> Diturunkan dari `research/token-efficiency-codebase-audit.md`. Nomor lanjutan dari BL-44 (`payment-lockout-gate-backlog.md`). **Catatan penting**: dua rekomendasi awal dari riset eksternal (cache_passthrough toggle, defer_loading MCP) DICABUT setelah audit kode membuktikan keduanya bukan gap nyata untuk arsitektur galleon-fleet — lihat §1 dan §3 dokumen riset. Backlog ini hanya memuat gap yang terverifikasi file:baris. **Diangkat ke roadmap resmi 2026-10-05 sebagai R-88 s.d. R-91** (`standard-docs/roadmap/R-rust.md`, section "Token/Credit Efficiency"). BL-45 dipecah jadi BL-45a (observasi/R-88) dan BL-45b (mitigasi/R-89, kondisional) agar tidak membangun solusi tanpa data pengukuran dulu.

---

## BL-45 — Koordinasikan Cache Breakpoint dengan Titik Potong Trim History (P1, MEDIUM)

**Rujukan audit:** §2 Gap A (`research/token-efficiency-codebase-audit.md`)

**Mekanisme terverifikasi (detail matang, 2026-10-05):** breakpoint cache Anthropic TIDAK disimpan sebagai state persisten di history — ia diterapkan FRESH setiap request lewat `apply_cache_breakpoints` (`compatible/mod.rs:1088`, native setara di `anthropic/mod.rs`), dengan strategi "rolling breakpoint": (1) breakpoint TETAP di system prompt setiap request (`merge_system_into_user` aware), (2) breakpoint KEDUA bergeser ke pesan non-system TERAKHIR yang punya teks (`last message when it carries text, otherwise the nearest earlier non-system message that does`). Strategi ini BENAR untuk kasus normal: selama riwayat hanya BERTAMBAH (append-only), prefix [system...pesan-sebelum-terakhir] tetap identik byte-demi-byte antar-request, sehingga cache HIT berlanjut walau breakpoint kedua terus bergeser maju.

**Masalah konkret:** `trim_to_recent_turns_with_crumb` (`crates/clawcrew-runtime/src/agent/history_trim.rs`) menghapus turn-turn LAMA dari TENGAH riwayat — turn yang dibuang berada DI ANTARA breakpoint system dan breakpoint rolling-terakhir. Begitu turn tengah dibuang, prefix [system...pesan-sebelum-terakhir] pada request BERIKUTNYA secara byte sudah BERBEDA dari yang di-cache sebelumnya (turn yang hilang membuat seluruh urutan setelah titik potong bergeser), sehingga Anthropic server tidak bisa mencocokkan prefix lama → cache MISS total untuk request pasca-trim, dan harus membangun cache baru dari nol (kena cache-WRITE premium 1.25-2x, bukan cache-READ diskon 90%).

**Scope:** Idealnya trim JARANG terjadi (threshold `DEFAULT_MAX_HISTORY_MESSAGES=50`), sehingga biaya cache-miss satu kali ini sudah "harga wajar" membersihkan riwayat — TAPI saat ini TIDAK ADA pengukuran yang mengonfirmasi berapa sering trim actually terjadi di sesi nyata, dan tidak ada perbandingan biaya (apakah penalti cache-miss sekali itu lebih mahal dari terus menyimpan riwayat panjang tanpa trim). Scope BL-45 diperjelas jadi DUA bagian:

1. **BL-45a (observasi dulu, S)**: tambah logging/metric yang mencatat setiap kali `trimmed=true` dari `TrimResult` DAN cache_read_input_tokens pada request pasca-trim berikutnya — supaya ada data nyata (bukan asumsi) soal seberapa sering & seberapa mahal pola ini terjadi sebelum membangun mitigasi.
2. **BL-45b (mitigasi, M, dependen BL-45a jika data menunjukkan dampak signifikan)**: pertimbangkan trim yang align dengan breakpoint — misal: trim HANYA pada boundary yang sejajar dengan posisi breakpoint sebelumnya, atau terima cache-miss satu kali per-trim sebagai trade-off yang disengaja (dan dokumentasikan sebagai keputusan sadar, bukan silent cost).

**File ownership:** Backend-Rust — `crates/clawcrew-runtime/src/agent/turn/context_recovery.rs` (titik trim + logging baru), `crates/clawcrew-providers/src/dispatch/accounting.rs` (korelasi cache_read dengan event trim terdekat).

**Definition of Done (BL-45a):** Log/metric baru mencatat pasangan (trim_event_timestamp, next_request_cache_read_tokens) untuk minimal 1 sesi uji nyata; keputusan BL-45b (apakah layak dikerjakan) diambil berdasar data ini, bukan asumsi.

**Dependency:** Tidak ada untuk BL-45a. BL-45b dependen hasil BL-45a.

**Risiko/Severity:** MEDIUM — murni optimisasi biaya, tidak mempengaruhi kebenaran fungsional. Diturunkan dari estimasi awal karena threshold trim yang jarang (50 pesan) mungkin membuat dampak sebenarnya kecil — perlu DIUKUR, bukan diasumsikan signifikan.

---

## BL-46 — Surface Cache Hit-Rate ke Dashboard/CLI Operator (P2, LOW-MEDIUM)

**Rujukan audit:** §2 Gap B

**Scope:** `cache_read_input_tokens`/`cache_creation_input_tokens` sudah di-track penuh end-to-end sampai `clawcrew-config/src/cost/types.rs` — field ini TIDAK PERLU dibangun dari nol. Yang dibutuhkan: titik tampilan (dashboard web atau output CLI `doctor`/`cost` command) yang menyurfaceкan rasio cache-hit (`cache_read / (cache_read + input_tokens_non_cached)`) per sesi, supaya operator bisa memverifikasi klaim komentar kode sendiri (`schema.rs:982-984`: *"verify cache_read_input_tokens > 0"*) tanpa membaca log mentah.

**File ownership:** Frontend (kalau dashboard) ATAU Backend-Rust (`crates/clawcrew-runtime/src/doctor/mod.rs` kalau lewat CLI) — scope final tergantung mana yang lebih murah diimplementasi, perlu dicek lebih dulu field apa yang sudah di-expose lewat `clawcrew-api`.

**Definition of Done:** Operator bisa melihat (lewat dashboard ATAU `kirocrew doctor`/`kirocrew cost` command) angka cache-hit-rate per sesi/per hari tanpa membaca file log mentah.

**Dependency:** Tidak ada hard dependency — tapi lebih bernilai dikerjakan SETELAH BL-45 (supaya angka yang disurfaceкan merepresentasikan sistem yang sudah dikoordinasikan, bukan angka rendah palsu akibat Gap A yang belum diperbaiki).

**Risiko/Severity:** LOW-MEDIUM — pure observability, tidak ada risiko fungsional.

---

## BL-47 — Dokumentasikan Prinsip "Steering Stabil = Cache-Friendly" sebagai Keputusan Sadar (P3, LOW)

**Rujukan audit:** §2 Gap C

**Scope:** Steering file (`crew-automation.md`, `ponytail.md`, dll dengan `inclusion: always`) saat ini SUDAH cache-friendly secara struktural (dimuat stabil di posisi awal prompt), tapi ini efek samping arsitektur, bukan prinsip operasional yang didokumentasikan sadar. Tambahkan satu paragraf di `AGENTS.md` atau dokumen operator-facing yang setara, menyatakan eksplisit: "jangan ubah steering file inti di tengah sesi aktif yang sedang berjalan — mengubahnya membatalkan prompt cache untuk seluruh sesi tersebut", supaya kebetulan arsitektur ini tidak dirusak tanpa sadar oleh perubahan konfigurasi di masa depan.

**File ownership:** Dokumentasi — `AGENTS.md` atau `standard-docs/` yang relevan.

**Definition of Done:** Satu paragraf eksplisit tersimpan, menyebut mekanisme prompt-cache byte-exact-prefix-match sebagai alasan.

**Dependency:** Tidak ada.

**Risiko/Severity:** LOW — murni dokumentasi, tidak ada perubahan kode.

---

## Rekomendasi yang DICABUT (Bukan Backlog, untuk Transparansi)

| Rekomendasi awal (dari riset eksternal) | Alasan dicabut |
|---|---|
| "Aktifkan `cache_passthrough` di config" | Field ini HANYA untuk compatible-gateway provider (LiteLLM dst). Native Anthropic provider SUDAH menempatkan cache marker tanpa syarat field ini — tidak ada toggle yang perlu diaktifkan untuk kasus penggunaan standar. |
| "Implementasikan `defer_loading` untuk MCP tools (niru Claude Code)" | MCP server di galleon-fleet dimuat statis sekali per-proses (`McpClient::connect_all`), tidak ada hot-reload/connect-disconnect di tengah sesi. Masalah yang `defer_loading` selesaikan (tool list berubah dinamis di tengah sesi) tidak terjadi secara struktural di arsitektur ini. |
| "Bangun cache-safe forking untuk compaction (niru Claude Code)" | Compaction galleon-fleet adalah drop-oldest-whole-turns, BUKAN LLM-summarize call terpisah. Masalah yang cache-safe-forking selesaikan (compaction call sendiri membatalkan cache) tidak applicable — gap sebenarnya adalah BL-45 (koordinasi breakpoint-trim), bukan forking pattern. |

## Urutan Eksekusi yang Disarankan

1. **BL-45a** dulu — observasi murah (logging), menjawab apakah BL-45b layak dikerjakan dengan DATA, bukan asumsi.
2. **BL-45b** hanya jika BL-45a menunjukkan dampak signifikan.
3. **BL-46** setelah BL-45a (minimal) — supaya angka cache-hit-rate yang disurfaceкan ke operator sudah termasuk korelasi trim-event, bukan angka telanjang tanpa konteks.
4. **BL-47** kapan saja, independen, biaya nol.

## Catatan Proses

Severity keseluruhan: LOW-MEDIUM — tidak ada bug fungsional (semua jalur tetap menghasilkan respons benar), murni soal efisiensi biaya. Prioritaskan di bawah BL-32 (bug nyata WhatsApp signature) dan BL-14 (prompt-injection gap), sejalan dengan urutan prioritas yang sudah berlaku di backlog lain.