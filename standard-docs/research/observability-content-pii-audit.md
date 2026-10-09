---
title: Audit Observability — Isi Percakapan User Ikut Ter-log via Tool I/O (Bukan Kredensial)
status: research
date: 2026-10-05
author: squad-lead
depends_on: null
---

# Audit Observability — Konten Percakapan di Luar Kredensial

> Fokus BERBEDA dari BG-F5/BR-B4 (dev-tasks, soal scrubbing kredensial) — ini soal apakah ISI PERCAKAPAN user/LLM ikut ter-log secara default.

## Verdict: CHANGES-REQUESTED — Gap Genuine, Severity HIGH

## Temuan

### 1. LLM Request Payload — Default OFF, Desain SUDAH BENAR

`log_llm_request_payload` default `Off` (`clawcrew-config/src/schema.rs:13407`). `LlmRequestPayloadPolicy::from_raw` resolve nilai kosong/unknown ke `Off` secara eksplisit — **fail-closed**, bukan fail-open. Event `llm_request` default hanya metadata (`iteration`, `messages_count`, `model`, `trace_id`), prompt mentah TIDAK ikut kecuali eksplisit opt-in. Ini **privacy-by-design yang benar** — jadikan rujukan pola untuk perbaikan gap di bawah.

### 2. Tool I/O — Default `Redacted`, TAPI Nama Menyesatkan: Hanya Truncate, Bukan Content-Redaction

**Ini masalah utamanya, dan dikonfirmasi GANDA oleh 2 sub-agent independen.** `log_tool_io` default `Redacted` (`schema.rs:13403`). API redaksi yang BENAR sudah ADA dan dirancang dengan baik: `capture_tool_input`/`capture_tool_output` (`clawcrew-log/src/tool_io.rs:24-61`) menghormati `log_tool_io` + truncate 40960 byte + denylist.

**Tapi grep seluruh `crates/` membuktikan `capture_tool_input`/`capture_tool_output` NOL dipanggil di produksi — hanya dipakai di unit test `tool_io.rs` sendiri.** Ini bukan sekadar "nama menyesatkan" — ini **API yang dirancang benar tapi tidak pernah di-wire ke call site produksi**, kelas bug yang sub-agent kedua sebut dengan tepat sebagai *"konfigurasi berbohong"*: operator yang membaca config `log_tool_io: redacted` akan percaya ada redaksi aktif, padahal fungsi yang seharusnya menegakkannya tidak pernah dipanggil. Ini LEBIH BERBAHAYA daripada tidak ada redaksi sama sekali, karena menciptakan false sense of security.

`clawcrew-runtime/src/agent/tool_execution.rs:166-262` — komentar kode SENDIRI eksplisit menyatakan niat: *"emit Start with full input... emit Complete with full output"* — ini mengonfirmasi bahwa logging penuh BUKAN kecelakaan, tapi desain yang kontradiktif dengan API redaksi yang sudah dibangun di crate lain dan tidak pernah disambungkan. `record!` dipanggil dengan `attrs` berisi `"input": call_arguments` dan `"output": r.output` — nilai mentah, hanya `scrub_credentials` (kredensial, bukan konten) yang diterapkan, dan bahkan itu tidak konsisten di semua field (`with_attrs` "output" di baris ~257 tidak selalu dapat scrub yang sama).

**Implikasi langsung**: tool seperti `send_message`, `artifact_post_comment`, `memory_recall` membawa **teks percakapan/konten user atau respons LLM** sebagai argumen/hasil — ikut masuk ke `runtime-trace.jsonl` tersimpan penuh (truncated 40KB), bukan di-redact isinya. Ini BERBEDA dari kredensial (yang sudah ditangani BG-F5/BR-B4) — ini soal konten personal percakapan.

### 2b. Response Text LLM — Terpisah dari Tool I/O, Terkonfirmasi Tidak Ada

Sub-agent kedua (konfirmasi independen) memeriksa titik terpisah: `stream_consume.rs:346-358` hanya log pada kegagalan (`has_reasoning`, `protocol_suppressed` — boolean saja, tanpa teks). **Tidak ditemukan event `llm_response` yang menaruh teks jawaban model** — ini aman, konsisten dengan temuan #1.

### 3. Permission File Log — 0o600 Unix, TIDAK ADA Equivalent di Windows

`clawcrew-log/src/writer.rs:403-420`: `#[cfg(unix)] options.mode(0o600)` — **hanya berlaku Unix**. Tidak ada `#[cfg(windows)]` branch yang set ACL eksplisit — file `runtime-trace.jsonl` mewarisi permission default direktori induk di Windows. Severity MEDIUM (relevan untuk konteks pengguna Windows) — risiko riil kecil di laptop single-user, tapi gap nyata dibanding proteksi Unix.

### 4. Lokasi & Shipping — Murni Lokal, Akses via Gateway Butuh Token

Path default: `state/runtime-trace.jsonl` relatif ke `workspace_dir`. Endpoint `/api/logs` wajib `require_auth` (bearer token pairing) — gateway default bind `127.0.0.1`, hanya reachable localhost kecuali user sengaja expose. **Tidak ada log shipping ke server terpusat secara default.** OTel exporter (jalur ekspor eksternal satu-satunya) punya privacy gate SENDIRI (`scrub_for_export`, terpisah dari JSONL writer) dan butuh endpoint eksplisit — default tetap localhost yang kemungkinan tidak listening.

**Kesimpulan akses**: log murni lokal, tapi gateway lokal bisa diakses proses lain di mesin yang sama kalau mereka berhasil mencuri bearer token pairing — bukan ancaman jaringan, tapi ancaman "proses lain di mesin yang sama."

## Rekomendasi

1. **Sambungkan API redaksi yang SUDAH ADA** — `capture_tool_input`/`capture_tool_output` (`clawcrew-log/src/tool_io.rs:24-61`) dirancang benar (menghormati `log_tool_io` + truncate + denylist) tapi tidak pernah dipanggil. Ganti `call_arguments`/`r.output` mentah di `tool_execution.rs:166-262` dengan hasil kedua fungsi ini — ini PERTAMA sebelum pertimbangan desain lain, karena ini bug "wiring hilang", bukan kekurangan desain.
2. **Setelah #1, pertimbangkan pisah dua axis policy tool I/O** (rekomendasi arsitektur jangka menengah): (a) *technical tool I/O* (shell, file ops) tetap default truncated seperti sekarang; (b) *conversational tool I/O* (tool yang parameternya berisi free-text percakapan — `send_message`, `artifact_post_comment`, dll) default ke `Off`, sejajar dengan pola `log_llm_request_payload` yang sudah benar.
3. **Terapkan `scrub_credentials` secara KONSISTEN** di jalur `record!` (`tool_execution.rs:234-247,252-266`) — saat ini scrub tidak konsisten diterapkan ke semua field (`with_attrs` "output" tidak selalu dapat scrub yang sama dengan "input").
4. **Dokumentasikan eksplisit** bahwa `log_tool_io = redacted` saat ini TIDAK berfungsi sebagai redaksi (sampai #1 selesai) — supaya user tidak berasumsi "redacted = aman" dari membaca nama config saja.
5. **Tambah proteksi permission Windows** di `writer.rs` (`open_active_file`, `rotate_active`, `trim_to_last_entries`) — paralel dengan branch `#[cfg(unix)]` yang sudah ada di ketiga fungsi tersebut (dikonfirmasi oleh kedua sub-agent, grep `cfg(windows)`/ACL/DACL = nol hasil).

**Catatan penting**: desain `LlmRequestPayloadPolicy` (temuan #1) adalah CONTOH YANG BENAR — jangan diubah, jadikan pola rujukan. Gap tool I/O BUKAN kekurangan desain (API redaksinya sudah dibangun dengan baik) — ini bug wiring: komentar kode SENDIRI di `tool_execution.rs:166-168` eksplisit menyatakan niat "emit full input/output", kontradiksi langsung dengan API redaksi yang dibangun terpisah dan tidak pernah disambungkan. Ini dikonfirmasi independen oleh 2 sub-agent audit terpisah.

---

## Sumber

- Audit independen ganda (2 sub-agent, `509f1796` dan `299cc2f8` — temuan konsisten, saling memperkuat): `crates/clawcrew-config/src/schema.rs:13403,13407,13411,7644,7798` (`LogToolIo`, `LogLlmRequestPayload`, truncate bytes, bind address default)
- `crates/clawcrew-log/src/tool_io.rs:24-65` (`capture_tool_input`/`capture_tool_output` — API redaksi dirancang benar, NOL call-site produksi, hanya dipakai di unit test sendiri), `writer.rs:369-475,700` (`open_active_file`/`rotate_active`/`trim_to_last_entries`, permission Unix-only), `config.rs:40,47,143-146,231-240` (path default, policy resolution)
- `crates/clawcrew-runtime/src/agent/turn/provider_call.rs:34-96` (`announce_llm_request`, contoh desain benar, dibuktikan test `provider_call.rs:588-654`), `tool_execution.rs:166-262` (gap: komentar kode sendiri menyatakan niat full-logging, `record!` tanpa redaksi konsisten)
- `crates/clawcrew-runtime/src/agent/turn/stream_consume.rs:346-358` (konfirmasi: tidak ada event `llm_response` dengan teks jawaban model)
- `crates/clawcrew-runtime/src/observability/otel.rs:63-66` (OTel privacy gate terpisah, `ObservabilityBackend::None` default, `otel_genai_content`/`otel_tool_io` default `Off`)
- `crates/clawcrew-gateway/src/api_logs.rs:84-93`, `api.rs:45-73` (`require_auth`)
