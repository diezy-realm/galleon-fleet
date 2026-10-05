---
title: Audit Mendalam — Token/Credit Efficiency di Codebase galleon-fleet
status: research
date: 2026-10-05
author: squad-lead
depends_on:
  - research/payment-lockout-gate-industry-research.md (metodologi serupa)
feeds_into: backlog/token-efficiency-backlog.md
---

# Audit Mendalam: Apa yang Sebenarnya Terjadi di Kode (Bukan Asumsi dari Riset Eksternal)

Dokumen ini **mengoreksi** kesimpulan awal dari sintesis riset eksternal (`riset-teknik-penghematan-token-ai-coding-assistant` artifact) setelah membaca implementasi NYATA, bukan hanya nama field. Prinsip yang sama dipakai di audit channel-signature sesi sebelumnya: menemukan field/nama fungsi TIDAK CUKUP, harus ditelusuri siapa yang memanggilnya dan dalam kondisi apa.

## 1. Koreksi Penting: `cache_passthrough` BUKAN Gap untuk Native Anthropic

**Kesimpulan sebelumnya (SALAH/tidak lengkap)**: "cache_passthrough default false — prompt caching tidak aktif otomatis."

**Fakta setelah membaca kode (`crates/clawcrew-providers/src/anthropic/mod.rs:792,804,825,857,1568`)**: native Anthropic provider menempatkan `cache_control` marker **TANPA mengecek `cache_passthrough` sama sekali** — field itu dicek hanya di `compatible/mod.rs::apply_cache_breakpoints` (`crates/clawcrew-providers/src/compatible/mod.rs:1096`), yang jalur kodenya KHUSUS untuk provider "OpenAI-compatible gateway" (LiteLLM, vLLM, proxy custom) yang menerjemahkan antara format OpenAI Chat Completions dan Anthropic Messages API.

Komentar kode sendiri eksplisit (`schema.rs:980-985`):
> *"Only gateways that translate between OpenAI Chat Completions and the Anthropic Messages API forward these breakpoints (e.g. LiteLLM)... The native Anthropic provider always places them, compatible providers only behind `cache_passthrough`."*

**Artinya**: kalau Achmad memakai Anthropic API langsung (bukan lewat gateway compatible seperti LiteLLM), **prompt caching SUDAH aktif otomatis sejak awal, tanpa toggle apa pun**. `cache_passthrough` hanya relevan untuk topologi deployment spesifik (self-hosted gateway di depan Anthropic) — bukan gap umum yang mempengaruhi semua pengguna.

**Yang justru PERLU dicek** (bukan toggle config, tapi observasi runtime): apakah cache HIT benar-benar terjadi. Komentar kode sendiri menyarankan ini (`schema.rs:982-984`): *"verify the configured route serves cache reads: an immediate repeat of a cache-creating request must report `cache_read_input_tokens > 0`."* Tracking ini **sudah ada end-to-end** — `cache_read_input_tokens`/`cache_creation_input_tokens` di-parse dari response Anthropic (`anthropic/mod.rs:570,574,1990-1991`), diteruskan ke `dispatch/accounting.rs:533-547`, dan disimpan di `clawcrew-config/src/cost/types.rs:11-23`. **Belum diverifikasi di sesi ini**: apakah angka ini SURFACE ke UI/dashboard yang bisa dilihat Achmad, atau hanya tersimpan di internal cost tracker tanpa representasi visual.

## 2. Koreksi: Compaction yang Ada BUKAN "LLM Summarize", Jadi "Cache-Safe Forking" Tidak Applicable Langsung

**Kesimpulan sebelumnya (perlu nuansa)**: "belum ada cache-safe forking seperti Claude Code."

**Fakta setelah membaca `crates/clawcrew-runtime/src/agent/turn/context_recovery.rs:95-115`**: mekanisme compaction yang ada adalah **drop-oldest-whole-turns** (`trim_to_recent_turns_with_crumb`) — bukan LLM-generated summary sama sekali. Komentar kode eksplisit: *"One rule: drop oldest whole turns until we are under a budget forced below the current size... Whole turns or nothing."*

Ini artinya rekomendasi "cache-safe forking" dari riset Claude Code **tidak langsung applicable** — teknik itu menjawab masalah "compaction call sendiri membatalkan cache karena system prompt berbeda", tapi galleon-fleet TIDAK melakukan compaction call terpisah sama sekali. Pendekatan drop-oldest-turns justru punya properti cache-friendly yang berbeda: karena HANYA pesan lama yang dibuang dan prefix tidak diubah (sistem prompt + breadcrumb tetap di tempatnya), prefix yang tersisa (sistem prompt + beberapa turn awal yang masih dipertahankan) **berpotensi tetap cache-hit** — asalkan breakpoint cache diletakkan SEBELUM titik potong drop, bukan di akhir riwayat.

**Gap nyata yang teridentifikasi** (bukan "harus niru Claude Code", tapi gap spesifik untuk arsitektur drop-oldest ini): tidak ditemukan bukti bahwa breakpoint `apply_cache_control` di `compatible/mod.rs` ATAU logic marker Anthropic native memperhitungkan TITIK POTONG trim — breakpoint diletakkan berdasar posisi pesan dalam array SAAT REQUEST DIKIRIM (`apply_cache_breakpoints` dipanggil per-request), bukan dikoordinasikan dengan kapan `trim_to_recent_turns_with_crumb` baru saja memotong riwayat. Trim yang terjadi MENGUBAH offset semua pesan setelah titik potong, yang **invalidate cache untuk SELURUH prefix setelah titik potong** (karena breakpoint sebelumnya menunjuk ke pesan yang sekarang sudah tidak ada / sudah bergeser index). Ini bukan bug fungsional (hasil tetap benar), tapi efisiensi: setiap kali trim terjadi, cache_read akan jatuh ke 0 untuk request berikutnya sampai cache ter-build ulang.

## 3. Konfirmasi: `defer_loading` MCP Tidak Relevan untuk Arsitektur Ini

**Fakta setelah membaca `crates/clawcrew-tools/src/mcp_client.rs:326,890`**: MCP server dimuat sekali lewat `McpClient::connect_all(configs)` dari config statis saat sesi/proses dimulai. Tidak ditemukan fungsi `reload`/`hot_reload`/`disconnect_server` yang bisa dipanggil DI TENGAH sesi yang sedang berjalan.

**Implikasi**: masalah yang `defer_loading` Claude Code pecahkan (tool list berubah di tengah sesi karena user connect/disconnect MCP server secara interaktif) **tidak terjadi secara struktural di galleon-fleet** — karena topologi MCP-nya statis per-proses, bukan dinamis per-sesi. Rekomendasi backlog untuk `defer_loading` **DICABUT** dari rencana — bukan gap, karena masalahnya tidak ada untuk arsitektur ini.

## 4. Gap yang Benar-Benar Nyata (Hasil Audit Terkoreksi)

Setelah tiga koreksi di atas, gap nyata yang tersisa untuk ditindaklanjuti:

### Gap A — Cache breakpoint tidak dikoordinasikan dengan titik potong trim (lihat §2)
Setiap `trim_to_recent_turns_with_crumb` berjalan, cache yang terbentuk sebelumnya kemungkinan langsung invalid karena pergeseran offset pesan. Ini BUKAN soal "menambah fitur niru kompetitor" — ini soal MENGHUBUNGKAN dua mekanisme yang sudah ada (cache breakpoint + trim) supaya tidak saling meniadakan efisiensi satu sama lain.

### Gap B — Visibilitas cache hit-rate ke operator tidak terverifikasi
Data `cache_read_input_tokens`/`cache_creation_input_tokens` sudah di-track penuh di cost accounting, tapi BELUM dicek sesi ini apakah ada endpoint/UI dashboard yang menyurfaceкan rasio cache-hit secara eksplisit (bukan hanya biaya total). Tanpa visibilitas ini, operator (Achmad) tidak punya cara mudah memverifikasi klaim komentar kode sendiri ("verify cache_read_input_tokens > 0") tanpa membaca log mentah.

### Gap C — Tidak ada dokumentasi operator-facing soal urutan prompt yang cache-friendly
Steering file (`crew-automation.md`, `ponytail.md`, dll) kebetulan SUDAH cache-friendly secara struktural (dimuat stabil di awal via `inclusion: always`), tapi tidak ada dokumentasi eksplisit yang menyatakan "jangan ubah steering file di tengah sesi aktif" sebagai prinsip operasional yang sadar — ini saat ini berhasil karena kebetulan arsitektur, bukan karena keputusan sadar yang didokumentasikan.

## Kesimpulan

Audit mendalam ini MENGOREKSI 2 dari 3 rekomendasi awal (cache_passthrough bukan gap untuk native Anthropic; defer_loading tidak relevan untuk arsitektur MCP statis), dan menemukan 1 gap baru yang lebih presisi dan actionable (koordinasi cache breakpoint dengan trim offset) dibanding rekomendasi awal yang hanya "niru fitur kompetitor" tanpa verifikasi apakah masalah yang sama benar-benar ada di sini.

Pelajaran proses (konsisten dengan lesson sesi-sesi sebelumnya): riset eksternal memberi KOSAKATA dan HIPOTESIS gap yang baik, tapi setiap hipotesis WAJIB diverifikasi ke implementasi nyata sebelum ditulis sebagai backlog — kalau tidak, backlog akan menyuruh "membangun fitur yang sudah ada" (cache_passthrough) atau "menyelesaikan masalah yang tidak ada" (defer_loading).

## Sumber (File:Baris Terverifikasi)

- `crates/clawcrew-providers/src/anthropic/mod.rs:792,804,825,857,1568,570,574,1990-1991` — native cache marker placement + usage parsing.
- `crates/clawcrew-providers/src/compatible/mod.rs:1088-1140` — `apply_cache_breakpoints`, scope terbatas ke compatible gateway.
- `crates/clawcrew-config/src/schema.rs:826-847,975-1010` — `CacheTtl`, `cache_passthrough`, `cache_ttl` dengan komentar developer yang eksplisit soal scope.
- `crates/clawcrew-runtime/src/agent/turn/context_recovery.rs:80-180` — `try_recover_context_overflow`, drop-oldest-whole-turns, bukan LLM-summarize.
- `crates/clawcrew-runtime/src/agent/history.rs:13,451` — `DEFAULT_MAX_HISTORY_MESSAGES`, `trim_history`.
- `crates/clawcrew-tools/src/mcp_client.rs:326,890` — `connect`/`connect_all`, pemuatan statis, nol hot-reload.
- `crates/clawcrew-providers/src/dispatch/accounting.rs:533-547`, `crates/clawcrew-config/src/cost/types.rs:11-23` — jalur akumulasi cache token ke cost tracker.
- Riset eksternal (artifact `riset-teknik-penghematan-token-ai-coding-assistant`) — sumber hipotesis awal yang DIKOREKSI oleh audit ini.
