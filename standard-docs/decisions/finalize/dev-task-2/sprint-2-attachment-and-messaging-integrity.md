---
sprint: 2
title: Attachment & Messaging Integrity — Debounce Bug, Multi-Format, 2-Layer Validation
status: planned
depends_on: Sprint 0 §C (BL-14 harus merge dulu — file overlap orchestrator/mod.rs)
---

# Sprint 2 — Attachment & Messaging Integrity

> **PRASYARAT KERAS:** BL-14 (Sprint 0) harus sudah merge sebelum sprint ini menyentuh `clawcrew-channels/src/orchestrator/mod.rs`. Jangan mulai BL-52 di file ini sebelum BL-14 selesai — dua crew menulis file yang sama secara paralel menghasilkan merge conflict/regresi silang.

## A. Bug Data-Loss (PRIORITASKAN — P0, mempengaruhi semua channel messaging hari ini)

- [ ] **BL-52 (P0 — bug data-loss nyata, PRIORITASKAN di atas semua item lain sprint ini)** Perbaiki bug attachment hilang di `MessageDebouncer`. **Masalah terverifikasi sampai baris kode:** saat pesan rapid digabung, hanya `turn` PERTAMA yang dipertahankan strukturnya (komentar kode sendiri: *"the combined turn keeps the place of the sender's first message"*), attachment dari pesan ke-2/3/dst HILANG diam-diam — hanya teksnya yang ikut digabung. File: `crates/clawcrew-infra/src/debounce.rs`, `crates/clawcrew-channels/src/orchestrator/mod.rs:1370-1372` (titik overwrite `turn.turn.msg.content = combined`).

## B. Perluasan Skema & Layer Validasi (paralel dengan A setelah A mulai, TIDAK sebelum)

- [ ] **BL-53 (LOW-MEDIUM, paralel dengan A)** Perluas `MediaKind` dengan `Document`/`Archive`/`Link` (saat ini hanya `Audio|Image|Video|Unknown`). File: `clawcrew-api/src/media.rs`. Referensi skema: Claude `source.type: base64|url|file` (seragam, lebih bersih ditiru) vs Gemini union-field-per-jenis.
- [ ] **BL-54 (MEDIUM, paralel, tersebar per-channel)** Layer 1: validasi hard-cap 10 attachment di Front App entry point (desktop/web/messaging) — SEBELUM bytes dikirim ke Core.
- [ ] **BL-55 (MEDIUM, SETELAH BL-52 — hard dependency, bukan paralel)** Layer 2: Core count+bytes enforcement SETELAH agregasi debounce. Menutup celah user kirim banyak pesan rapid untuk mengakali total 10 per-pesan, plus resource-gate total BYTES per bucket (jawab kekhawatiran over-memory).

## C. Redaksi & Scrub Tool I/O (independen, file berbeda, bisa paralel penuh dengan A/B)

- [ ] **BL-37 Tahap 1 (P1, HIGH — bug wiring, bukan desain, PRIORITASKAN)** Sambungkan `capture_tool_input`/`capture_tool_output` yang sudah dirancang benar tapi NOL dipanggil produksi. Ganti `call_arguments`/`r.output` mentah di `tool_execution.rs:166-262`. File: `crates/clawcrew-runtime/src/agent/tool_execution.rs`.
- [ ] **BL-37 Tahap 2 (SETELAH Tahap 1)** Pisah axis policy: technical tool I/O (default truncated) vs conversational tool I/O (default `Off`). File: `crates/clawcrew-config/src/schema.rs`, `crates/clawcrew-log/src/{tool_io.rs,config.rs}`.
- [ ] **BL-38 (P1, MEDIUM, independen dari BL-37, bisa paralel)** Terapkan `scrub_credentials` di jalur `record!` (`tool_execution.rs:234-266`) — gap terpisah dari BL-37, soal kredensial bukan konten percakapan.
- [ ] **BL-39 (P2, MEDIUM)** Proteksi ACL file log Windows (`writer.rs:403-420` hanya punya branch `#[cfg(unix)]`).

## File Ownership & Serialization

| Area | File | Penulis | Catatan serialisasi |
|---|---|---|---|
| A (BL-52) | `clawcrew-infra/src/debounce.rs`, `clawcrew-channels/src/orchestrator/mod.rs` | Backend-Rust | **Tunggu BL-14 (Sprint 0) merge dulu** |
| B (BL-53/54/55) | `clawcrew-api/src/media.rs`, Front App entry point per-channel, Core agregasi | Backend-Rust + Frontend (per-channel) | BL-55 tunggu BL-52 merge |
| C (BL-37/38/39) | `clawcrew-runtime/src/agent/tool_execution.rs`, `clawcrew-config/src/schema.rs`, `clawcrew-log/src/*` | Backend-Rust (crew berbeda dari A/B, file tidak beririsan) | Bisa paralel penuh dengan A/B |

## Keluaran Sprint

- Attachment pesan ke-2/3/dst tidak lagi hilang diam-diam saat debounce.
- Limit 10 attachment ditegakkan di 2 layer (Front App + Core), termasuk resource-gate bytes.
- `log_tool_io=redacted` benar-benar redacted (bukan lagi silent no-op).
