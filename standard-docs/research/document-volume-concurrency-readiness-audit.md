---
title: Audit Mendalam — Kesiapan Teknis File/Dokumen Volume Tinggi & Concurrency Tool-Call untuk Tim Profesional
status: research
date: 2026-10-05
author: squad-lead
context: Achmad membayangkan user adalah tim profesional yang setiap hari menangani banyak dokumen/file besar/screenshot/foto, dan dalam 1 query chat banyak pekerjaan berjalan bersamaan. Pertanyaan mengerucut dari topik RAG/knowledge-base sebelumnya ke SOP/Quest/Workflow.
---

# Audit: Kesiapan Teknis untuk Tim Profesional (Volume Dokumen Tinggi + Concurrency)

## Ringkasan Jawaban Langsung

**Sebagian siap, dengan DUA gap arsitektural nyata yang pola dasarnya SAMA dengan BL-15/18 (resource gate sub-agent) yang sudah ditemukan sesi sebelumnya**: gap resource-gating muncul berulang di level yang berbeda-beda (sub-agent spawn SUDAH digate, tool-call biasa BELUM).

## 1. Handling Dokumen/File Besar/Screenshot/Foto — Audit Internal

**Yang SUDAH matang:**
- Limit ukuran file per-channel sudah ada dan dikonfigurasi granular: WeCom 20MB (`max_file_size_mb`), QQ 10MB hardcoded, `file_read.rs` tool 10MB, `file_upload.rs`/`file_upload_bundle.rs` dengan `max_file_size_bytes` configurable.
- Deteksi MIME matang (`infer` crate + fallback filename-extension table, cover 20+ format termasuk gambar/dokumen/office/arsip).
- `image_info.rs`: limit 20MB, deteksi format via magic bytes.

**Gap nyata #1 — Tidak ada pola "upload-once-reference-by-id" (Files API ala Anthropic):**
Setiap kali agent butuh isi file yang sama, kemungkinan besar isinya di-embed ULANG ke context (perlu verifikasi lebih lanjut jalur `file_upload.rs` → konsumsi tool lain, TAPI tidak ditemukan konsep `file_id` persisten yang bisa di-reference lintas tool-call dalam satu sesi). Studi kasus riset eksternal (kindatechnical.com): tanpa pola ini, kontrak 80 halaman di-upload ulang 400x oleh satu user sehari sampai kuota egress jebol — risiko struktural yang SAMA berpotensi terjadi di galleon-fleet kalau dokumen besar dipakai berulang dalam satu task panjang (misal review kontrak multi-turn).

**Gap nyata #2 — SOP engine belum punya konsep "artifact/attachment" first-class:**
`crates/clawcrew-runtime/src/sop/` punya modul lengkap (`approval`, `capability`, `graph`, `step_contract`, `trigger_registry`, dst) TAPI nol modul `document`/`attachment`/`artifact`. Untuk tim profesional yang workflow-nya "upload dokumen → proses → approve → arsip", SOP saat ini tidak punya cara native mengikat sebuah file ke step tertentu secara terlacak — kemungkinan ditangani ad-hoc via path string biasa di `BindingContext`, bukan tipe data terstruktur.

## 2. Dev-Tasks — TIDAK Ada Pembahasan

Grep menyeluruh ke `standard-docs/decisions/finalize/dev-tasks/` untuk kata kunci file-besar/upload/concurrency/multimodal/screenshot: **nol hasil relevan**. Satu-satunya match (`CO-03`, soal `api_upload` handler migrasi ke RouteHandler) sudah **SUPERSEDED** — soal migrasi REST handler lama, bukan soal kapasitas/concurrency file. **Ini topik yang genuinely belum pernah dibahas tim dev-tasks.**

## 3. Keterkaitan dengan Riset Sebelumnya (Knowledge-Base, Indexing, Token Efficiency)

- **Token efficiency (R-88..91)**: trim-history coordination relevan LANGSUNG — dokumen besar yang di-embed penuh ke context akan jadi kandidat pertama yang kena trim, dan setiap trim+reload dokumen besar = biaya ganda (cache-miss + re-embed ulang isi file). Files-API pattern (gap #1 di atas) justru MENGURANGI tekanan ini — kalau file di-reference by ID bukan di-embed penuh, context jadi jauh lebih ringan, otomatis membantu masalah token-efficiency juga.
- **Knowledge Library (R-93, BL-49)**: pipeline chunk→index→embed yang direncanakan untuk `standard-docs/` sebenarnya adalah mekanisme YANG SAMA yang dibutuhkan untuk dokumen besar harian tim (PDF, laporan) — kalau R-93 dibangun dengan baik, ia bisa generalisasi jadi solusi untuk KEDUA kebutuhan (knowledge base internal DAN dokumen kerja harian tim), bukan dua sistem terpisah.
- **Citation verification (R-92, BL-48)**: relevan untuk dokumen kerja juga — kalau tim menyimpan "ringkasan laporan X" sebagai Lesson/Decision node, sitasi ke file aslinya perlu diverifikasi tetap valid (file tidak dihapus/diganti).

## 4. Concurrency Multi-Tool-Call dalam 1 Query — Audit Internal + Riset Industri

**Temuan internal**: `execute_tools_parallel()` (`crates/clawcrew-runtime/src/agent/tool_execution.rs:527-563`) SUDAH ADA dan matang secara SAFETY LOGIC — ada guard sadar untuk race condition (`tool_search` dipaksa sequential), approval-gated calls dipaksa sequential. **TAPI nol resource gate** — `join_all` menjalankan SEMUA tool call dalam batch sekaligus tanpa cap jumlah/memori, dan `parallel_tools` default `false` di config (`clawcrew-config/src/schema.rs:3935`).

**Riset industri mengonfirmasi ini bukan kelemahan unik galleon-fleet — ini gap industri luas:**
- **Anthropic (resmi)**: *"Independent, read-only operations are usually safe to run in parallel... Tools with side effects, shared state, or ordering requirements might be better run sequentially."* — TIDAK memberi angka cap resmi, tanggung jawab diserahkan ke implementasi aplikasi.
- **CrewAI**: hanya punya `max_rpm` (rate per MENIT), bukan concurrency-count cap.
- **LangGraph**: TIDAK punya semaphore/pool resmi — model "super-step" membiarkan semua node terjadwal jalan paralel, concurrency sesungguhnya diserahkan ke runtime async.
- **AutoGen**: punya pola "Concurrent Agents" tapi tanpa resource cap resmi.
- **KiroCrew sendiri** (observasi tool description langsung): punya adaptive concurrency cap untuk SUB-AGENT SPAWN (`spawn_run`, ceiling 6, adaptif via `resource_status`), TAPI **nol cap untuk tool-call biasa dalam satu turn**. `resource_status` sendiri eksplisit menyebut dirinya *"advisory only — does NOT reserve or enforce anything."*

**Insiden lapangan nyata** (laporan independen, bukan vendor resmi): tim engineering melaporkan corrupted shared state 4x akibat parallel tool calls tanpa gate — *"parallel tool calling amounts to concurrent writes against shared state, and nothing in the framework will tell you that."*

## Rekomendasi Konkret (Prioritas)

1. **Bungkus `execute_tools_parallel` dengan `tokio::sync::Semaphore`** — cap awal 4-8 concurrent tool-exec per turn, bisa disesuaikan setelah data nyata (pola sama dengan BL-45a: ukur dulu sebelum mitigasi permanen).
2. **Pisahkan tool read-only vs side-effect secara eksplisit** di `should_run_parallel()` (`tool_execution.rs`) — tool yang menulis file/state (upload, write) dipaksa sequential; tool read-only tetap paralel tanpa gate. Pola ini sudah terbukti di Anthropic computer-use/browser-use.
3. **Adopsi pola Files-API (upload-once-reference-by-id)** untuk dokumen besar — generate `file_id` internal persisten per sesi, tool lain reference by ID bukan re-embed. Ini mengurangi BEBAN GANDA: biaya token (trim-history R-88..91) dan biaya I/O berulang.
4. **Tambahkan tipe `Attachment`/`DocumentRef` ke SOP binding (`crates/clawcrew-runtime/src/sop/binding.rs`)** supaya workflow "upload→proses→approve→arsip" punya jalur terstruktur, bukan path string ad-hoc.
5. **Cap jumlah file paralel per batch upload** (bukan hanya ukuran) — referensi non-presisi dari M365 Copilot (20 file/percakapan) sebagai starting point yang reasonable.

## Catatan Kejujuran Riset

Tidak ada vendor manapun (Anthropic/OpenAI/CrewAI/LangGraph/AutoGen) yang mempublikasikan angka cap concurrency "aman" secara resmi — semua rekomendasi angka di atas adalah inferensi dari pola analog (rate-limit RPM, file-count-per-percakapan), bukan kutipan langsung. Keputusan angka final adalah keputusan kapasitas/kebijakan, bukan murni teknis.

## Sumber

- Internal: `crates/clawcrew-tools/src/{file_upload,file_upload_bundle,file_download,image_info}.rs`, `crates/clawcrew-channels/src/{wecom_ws,qq}.rs`, `crates/clawcrew-runtime/src/agent/tool_execution.rs:527-563`, `crates/clawcrew-config/src/schema.rs:3778-3935,4763`, `crates/clawcrew-runtime/src/sop/mod.rs`.
- Dev-tasks: grep menyeluruh `standard-docs/decisions/finalize/dev-tasks/` — nol hasil relevan selain `CO-03` (superseded).
- Eksternal (subagent, 2026-10-05): Notion/Slack/M365 Copilot file limits resmi, Claude Files API (`platform.claude.com/docs/en/build-with-claude/files`), GitHub Copilot CLI context management (spill-to-disk pattern), Anthropic parallel tool use docs resmi, CrewAI/LangGraph/AutoGen concurrency docs.
