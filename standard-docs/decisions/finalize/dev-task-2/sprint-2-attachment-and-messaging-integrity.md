---
sprint: 2
title: Attachment & Messaging Integrity — Debounce Bug, Multi-Format, 2-Layer Validation
status: planned
depends_on: Sprint 0 §C (BL-14 harus merge dulu — file overlap orchestrator/mod.rs)
---

# Sprint 2 — Attachment & Messaging Integrity

> **PRASYARAT KERAS:** BL-14 (Sprint 0) harus sudah merge sebelum sprint ini menyentuh `clawcrew-channels/src/orchestrator/mod.rs`. Jangan mulai BL-52 di file ini sebelum BL-14 selesai — dua crew menulis file yang sama secara paralel menghasilkan merge conflict/regresi silang.

## A. Bug Data-Loss (PRIORITASKAN — P0, mempengaruhi semua channel messaging hari ini)

- [x] **BL-52 (P0 — bug data-loss nyata, PRIORITASKAN di atas semua item lain sprint ini)** Perbaiki bug attachment hilang di `MessageDebouncer`. **Masalah terverifikasi sampai baris kode:** saat pesan rapid digabung, hanya `turn` PERTAMA yang dipertahankan strukturnya (komentar kode sendiri: *"the combined turn keeps the place of the sender's first message"*), attachment dari pesan ke-2/3/dst HILANG diam-diam — hanya teksnya yang ikut digabung. File: `crates/clawcrew-infra/src/debounce.rs`, `crates/clawcrew-channels/src/orchestrator/mod.rs:1370-1372` (titik overwrite `turn.turn.msg.content = combined`). Status: COMPLETED (`debounced_messages_preserve_all_attachments` passed).

#### Detail Eksekusi — BL-52

**Koreksi baris (verifikasi 2026-10-06):** Titik overwrite `turn.turn.msg.content = combined` ada di `crates/clawcrew-channels/src/orchestrator/mod.rs:1662` (bergeser dari `1370-1372` di dokumen lama — kode telah berubah sejak dicatat), di dalam fungsi `route_inbound_slot`. Verifikasi akar masalah LENGKAP (bukan hanya titik overwrite, tapi seluruh jalur):

1. `crates/clawcrew-infra/src/debounce.rs` — `MessageDebouncer::debounce_with_window(sender_key: &str, message: &str, window: Duration)` HANYA menerima `message: &str` (teks polos). `DebouncerEntry.messages: Vec<String>` menyimpan teks saja. `fire_debounced()` menggabung via `entry.messages.join("\n")` — SATU `String`, tidak ada tempat untuk attachment sama sekali di level debouncer ini.
2. `crates/clawcrew-channels/src/orchestrator/mod.rs` sekitar baris 11316 (fungsi pemanggil, nama fungsi tempatnya belum dikonfirmasi persis — baca ~11250-11320 untuk nama fungsi pembungkus): panggilan `ctx.debouncer.debounce_with_window(&debounce_key, &msg.content, debounce_window)` hanya mengirim `&msg.content` (String) — **`msg.attachments` (field `Vec<MediaAttachment>` di `clawcrew-api/src/channel.rs:423`) TIDAK PERNAH disertakan dalam pemanggilan ini**. Attachment pesan ke-2/3/dst genuinely tidak pernah disalurkan ke mana pun, bukan "ditimpa" — ini presisi penting untuk desain fix.
3. `InboundTurn { turn: Box<PendingTurn> }` dibuat SEKALI saat bucket debounce PERTAMA kali dibuka (`msg` lengkap dengan attachment pesan pertama tersimpan di `PendingTurn.msg`). Pesan susulan dalam window yang sama HANYA memperbarui bucket teks di debouncer (langkah 1), tidak pernah menyentuh `PendingTurn` yang sudah dibuat.
4. Di `route_inbound_slot` (baris 1662), saat window selesai: `turn.turn.msg.content = combined` — mengganti teks `PendingTurn` pertama dengan gabungan teks SEMUA pesan, TAPI `turn.turn.msg.attachments` tetap attachment pesan PERTAMA saja (field ini tidak pernah disentuh ulang).

**Spesifikasi API — perubahan diperlukan di 2 layer:**
```rust
// crates/clawcrew-infra/src/debounce.rs — tambah payload generik, bukan String murni.
// Opsi desain (pilih salah satu, DIPUTUSKAN saat implementasi berdasarkan dampak minimal):

// OPSI A (lebih kecil diff): debouncer tetap String-only untuk teks,
// orchestrator ikut menyimpan Vec<MediaAttachment> secara TERPISAH per
// debounce_key, lalu digabung manual di route_inbound_slot setelah
// `content.await` resolve. Debouncer TIDAK berubah sama sekali.

// OPSI B (lebih bersih, breaking change internal ke debounce.rs):
pub enum DebounceResult<T = String> {
    Pending { rx: tokio::sync::oneshot::Receiver<T>, extended: bool },
    Passthrough(T),
}
// DebouncerEntry.messages: Vec<(String, Vec<MediaAttachment>)>
// fire_debounced() menggabung teks DAN meng-concat semua Vec<MediaAttachment>
// dari seluruh entry.messages, bukan hanya join String.
```
**ASUMSI (perlu konfirmasi arsitektur sebelum eksekusi — ini keputusan desain, bukan detail bebas pilih):** Rekomendasi saya adalah **OPSI A** (minimal diff, tidak mengubah kontrak publik `MessageDebouncer` yang dipakai banyak test existing di `debounce.rs` — 9 test akan tetap hijau tanpa modifikasi). Attachment accumulator terpisah di orchestrator:
```rust
// Di titik pemanggilan debounce (orchestrator/mod.rs ~baris 11316), SEBELUM
// memanggil debouncer, akumulasi attachment ke map terpisah yang sudah
// dipegang orchestrator (pola sama dengan `debounce_buckets: HashMap<String, ...>`
// yang sudah ada untuk forwarder — baca definisi `debounce_buckets` dulu untuk
// menyamakan lifetime/locking pattern):
let mut pending_attachments = debounce_attachments.entry(debounce_key.clone())
    .or_insert_with(Vec::new);
pending_attachments.extend(msg.attachments.drain(..));

// Di route_inbound_slot, SETELAH content.await resolve (baris 1662):
InboundSlot::Debounced { mut turn, content, attachments_key } => match content.await {
    Ok(combined) => {
        turn.turn.msg.content = combined;
        turn.turn.msg.attachments = take_accumulated_attachments(&attachments_key); // BARU
        turn
    }
    Err(_) => return,
},
```

**Konvensi penamaan:** `attachments_key`/`take_accumulated_attachments` — nama contoh, ikuti pola existing `debounce_key`/`debounce_buckets` di file yang sama persis gaya snake_case Rust yang sudah dipakai.

**Langkah pengerjaan:**
1. Baca `crates/clawcrew-channels/src/orchestrator/mod.rs` baris 11250-11450 PENUH (bagian yang memanggil `debouncer.debounce_with_window` dan menangani `DebounceResult`) — pahami variable `debounce_buckets`, `debounce_bucket_owners` yang sudah ada sebagai pola accumulator serupa yang bisa ditiru untuk attachment.
2. **TUNGGU BL-14 (Sprint 0) merge dulu** — PRASYARAT KERAS sudah tercatat di header file ini, file yang sama (`orchestrator/mod.rs`) sedang disentuh BL-14.
3. Tambah struktur akumulasi attachment terpisah (pola di atas, OPSI A) — JANGAN ubah `MessageDebouncer` di `debounce.rs` kecuali OPSI A terbukti tidak mungkin saat implementasi (baru pindah ke OPSI B dengan konfirmasi Lead).
4. Update titik `turn.turn.msg.content = combined` (baris 1662) menjadi juga mengisi `attachments`.
5. Pastikan attachment di-drain (bukan di-clone lalu attachment asli masih ada di `msg` yang sudah tidak dipakai) — hindari duplikasi memory untuk attachment besar (foto/video).

**Acceptance criteria testable (WAJIB ada skenario ini):**
```rust
#[tokio::test]
async fn debounced_messages_preserve_all_attachments() {
    // Kirim 3 pesan rapid dalam window yang sama, masing-masing dengan
    // 1 attachment berbeda (gunakan MediaAttachment sintetis/mock).
    // Setelah window selesai, assert turn.msg.attachments.len() == 3
    // (bukan hanya attachment pesan pertama).
}
```
Plus test regresi: pesan TANPA attachment yang didebounce tetap bekerja seperti sebelumnya (tidak ada attachment kosong yang menyebabkan panic/unwrap).

**Dependency:** BL-14 (Sprint 0, hard prerequisite — file overlap). BL-55 (bagian B di bawah) depend pada tiket ini (hard — perlu `msg.attachments` lengkap sebelum bisa menghitung total bytes per bucket).

**Edge case:** Pesan ke-2/3 datang SETELAH bucket sudah mulai proses `fire_debounced` (race antara timer fire dan pesan baru) — pola `extended` flag yang sudah ada di `debounce.rs` (`DebounceResult::Pending { extended: bool }`) menangani ini untuk teks; akumulator attachment baru HARUS mengikuti pola locking yang sama (di bawah lock yang sama dengan `debounce_buckets`) supaya tidak race antara "bucket baru dibuka" vs "attachment ditambah ke bucket lama yang sudah fire".

**JANGAN DIUBAH:** `MessageDebouncer` public API (`debounce`/`debounce_with_window`/`DebounceResult`) — 9 unit test existing di `debounce.rs` harus tetap hijau tanpa modifikasi assertion, kecuali Lead mengkonfirmasi OPSI B. `PendingTurn`/`InboundTurn` struct shape untuk field selain `attachments` tidak berubah.

## B. Perluasan Skema & Layer Validasi (paralel dengan A setelah A mulai, TIDAK sebelum)

- [ ] **BL-53 (LOW-MEDIUM, paralel dengan A)** Perluas `MediaKind` dengan `Document`/`Archive`/`Link` (saat ini hanya `Audio|Image|Video|Unknown`). File: `clawcrew-api/src/media.rs`. Referensi skema: Claude `source.type: base64|url|file` (seragam, lebih bersih ditiru) vs Gemini union-field-per-jenis.

#### Detail Eksekusi — BL-53

**Verifikasi:** `crates/clawcrew-api/src/media.rs:27` — `pub enum MediaKind { Audio, Image, Video, Unknown }`, dengan `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` — PENTING: enum ini `Copy`, jadi penambahan variant TIDAK memerlukan perubahan derive, tapi SEMUA `match` exhaustive terhadap `MediaKind` di seluruh codebase akan gagal compile sampai ditambah arm baru (ini FITUR bukan bug — compiler akan menunjukkan tepat semua titik yang perlu diupdate).

**File & simbol spesifik:**
- Ubah: `crates/clawcrew-api/src/media.rs:27` — tambah variant `Document`, `Archive`, `Link`.
- WAJIB cari semua consumer: jalankan `code` tool `find_references` pada `MediaKind` SEBELUM menambah variant, untuk inventarisasi semua `match` yang akan pecah — JANGAN tambah variant lalu "fix compile error satu-satu tanpa rencana", baca dulu SEMUA titik match untuk desain respons yang konsisten (misal: apakah `Document`/`Archive` punya perlakuan sama dengan `Unknown` di beberapa titik, atau genuinely baru).

**Spesifikasi API:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Audio,
    Image,
    Video,
    Document,  // BARU — PDF, DOCX, XLSX, dll.
    Archive,   // BARU — ZIP, TAR, dll.
    Link,      // BARU — URL murni tanpa file bytes (preview link)
    Unknown,
}
```

**Langkah pengerjaan:**
1. `find_references` ke `MediaKind` — catat SEMUA file yang match exhaustive terhadapnya.
2. Tambah 3 variant baru.
3. Compiler akan menunjukkan setiap `match` yang pecah — perbaiki satu per satu dengan perlakuan yang MASUK AKAL secara semantik (bukan sekadar `_ => default` yang menyembunyikan kasus baru).
4. Tentukan heuristik klasifikasi MIME/ekstensi baru (`Document`: `.pdf`,`.docx`,`.xlsx`,`.pptx`,`.txt`; `Archive`: `.zip`,`.tar`,`.gz`,`.7z`; `Link`: deteksi dari struktur URL, bukan MIME — ini kategori berbeda sifatnya, verifikasi dulu bagaimana `Link` seharusnya terdeteksi, kemungkinan BUKAN dari MIME type sama sekali melainkan dari context pesan yang berisi URL tanpa file terlampir — ini butuh klarifikasi desain, catat sebagai ASUMSI).

**ASUMSI:** Variant `Link` kemungkinan tidak cocok dengan model klasifikasi MIME-based yang dipakai 3 variant lain — perlu keputusan desain terpisah (apakah `Link` adalah `MediaKind` sama sekali, atau field terpisah di `MediaAttachment`). Verifikasi ke Lead sebelum eksekusi kalau ambiguitas ini signifikan.

**Dependency:** Tidak hard-block tiket lain, tapi referensi skema Claude/Gemini (`source.type: base64|url|file`) menyiratkan `Link` mungkin lebih cocok sebagai variant TERPISAH dari klasifikasi MIME (`base64`/`url`/`file` adalah SOURCE TYPE, bukan MEDIA KIND) — pertimbangkan apakah ini butuh dimensi baru (`AttachmentSource` enum terpisah) alih-alih menumpuk ke `MediaKind`.

**JANGAN DIUBAH:** 4 variant existing (`Audio`,`Image`,`Video`,`Unknown`) — hanya tambah, tidak reorder/rename (reorder aman untuk enum non-`#[repr]` tapi tetap hindari untuk minimize diff review).

- [ ] **BL-54 (MEDIUM, paralel, tersebar per-channel)** Layer 1: validasi hard-cap 10 attachment di Front App entry point (desktop/web/messaging) — SEBELUM bytes dikirim ke Core.

#### Detail Eksekusi — BL-54

**TIDAK DIVERIFIKASI sesi ini** — "Front App entry point" mencakup banyak permukaan (desktop Tauri, web React, tiap channel messaging) yang di luar budget grep sesi ini. **Langkah pertama WAJIB bagi tim eksekusi:** inventarisasi semua titik upload/attach di `apps/tauri/`, `web/src/`, dan tiap `crates/galleon-channel-*`/`clawcrew-channels/src/` yang menerima attachment dari user sebelum menulis kode — tiket ini genuinely tersebar, bukan satu titik.

**Acceptance criteria testable:** Upload percobaan ke-11 attachment dalam satu pesan → ditolak di level Front App (UI menampilkan pesan error), request TIDAK terkirim ke Core sama sekali (verifikasi via network trace/mock, bukan hanya UI state).

**Dependency:** Tidak hard-block BL-52 (bisa paralel), tapi BL-55 (Layer 2) adalah DEFENSE-IN-DEPTH terhadap tiket ini — kalau BL-54 genuinely menutup semua Front App, BL-55 masih perlu ada untuk kasus BL-54 di-bypass (klien custom, API langsung).

**ASUMSI:** Angka "10" adalah hard-cap yang SUDAH diputuskan (bukan nilai yang perlu didesain ulang) — dokumen tidak menyebut dari mana angka ini berasal; kalau ini bisa dikonfigurasi per-tier (lihat `entitlement.QuotaPolicy`/ENT-01 Sprint 4), ini mungkin perlu field tambahan di sana — TAPI saya TIDAK merekomendasikan menambah ini ke ENT-01 tanpa keputusan eksplisit Owner (10 attachment per message sepertinya limit teknis/UX, bukan limit tier bisnis).

- [ ] **BL-55 (MEDIUM, SETELAH BL-52 — hard dependency, bukan paralel)** Layer 2: Core count+bytes enforcement SETELAH agregasi debounce. Menutup celah user kirim banyak pesan rapid untuk mengakali total 10 per-pesan, plus resource-gate total BYTES per bucket (jawab kekhawatiran over-memory).

#### Detail Eksekusi — BL-55

**File & simbol spesifik:** Titik enforcement harus di SETELAH `turn.turn.msg.attachments` terisi lengkap hasil fix BL-52 (`route_inbound_slot`, `orchestrator/mod.rs:1662` + perubahan BL-52) — enforcement SEBELUM titik ini akan menghitung attachment yang belum lengkap (bug yang sama dengan BL-52 kalau titik cek salah).

**Spesifikasi API:**
```rust
// Setelah blok `turn.turn.msg.attachments = ...;` (hasil fix BL-52), tambah:
const MAX_ATTACHMENTS_PER_TURN: usize = 10; // sinkron dengan BL-54 Front App
const MAX_ATTACHMENT_BYTES_PER_TURN: u64 = /* nilai TBD, lihat ASUMSI */;

fn enforce_attachment_limits(msg: &mut ChannelMessage) -> Result<(), AttachmentLimitError> {
    if msg.attachments.len() > MAX_ATTACHMENTS_PER_TURN {
        return Err(AttachmentLimitError::TooManyAttachments {
            count: msg.attachments.len(),
            max: MAX_ATTACHMENTS_PER_TURN,
        });
    }
    let total_bytes: u64 = msg.attachments.iter().map(|a| a.byte_size()).sum(); // cek method byte_size() ada di MediaAttachment, verifikasi dulu
    if total_bytes > MAX_ATTACHMENT_BYTES_PER_TURN {
        return Err(AttachmentLimitError::TooManyBytes { total_bytes, max: MAX_ATTACHMENT_BYTES_PER_TURN });
    }
    Ok(())
}
```

**Langkah pengerjaan:**
1. **Tunggu BL-52 selesai dan merge** (hard dependency — tidak bisa mulai lebih awal, enforcement butuh attachment yang sudah lengkap).
2. Cek apakah `MediaAttachment` (`clawcrew-api/src/media.rs` atau `channel.rs`) sudah punya method untuk ukuran byte (`byte_size()`/`len()`/sejenis) — kalau belum, tambahkan sebagai bagian tiket ini.
3. Tentukan titik pemanggilan `enforce_attachment_limits` — SETELAH `route_inbound_slot` mengisi attachment lengkap, SEBELUM turn diteruskan ke lane admission (`turn.registration`/`enqueue` yang sudah ada).
4. Response ke user saat limit terlampaui: pesan error yang jelas (bukan silent drop) — konsisten pola `send_notice_with_timeout`/busy notice yang sudah ada di file ini untuk notifikasi best-effort.

**Edge case (disebutkan eksplisit dokumen):** User kirim 5 pesan rapid, masing-masing 3 attachment (total 15) — SETIAP pesan individual lolos cek Front App (BL-54, maks 10 per pesan), tapi gabungan debounce 15 attachment HARUS ditolak di Layer 2 ini. Test WAJIB mensimulasikan skenario persis ini.

**Acceptance criteria testable:**
```rust
#[tokio::test]
async fn debounced_bucket_rejects_when_combined_attachments_exceed_cap() {
    // 5 pesan x 3 attachment = 15 total, masing-masing pesan individual <= 10.
    // Assert: turn ditolak dengan AttachmentLimitError::TooManyAttachments,
    // BUKAN diteruskan dengan attachment terpotong diam-diam.
}
```

**ASUMSI:** Nilai `MAX_ATTACHMENT_BYTES_PER_TURN` belum ditentukan dokumen manapun — perlu keputusan Lead/Owner (contoh: 50MB total per turn?). JANGAN pilih angka sembarangan tanpa konfirmasi — tulis sebagai konstanta bernama jelas yang mudah diubah, dan tandai PR dengan catatan "nilai butuh approval sebelum merge ke production config".

**Dependency:** BL-52 (hard). Opsional selaras dengan BL-54 untuk angka "10" yang sama (hindari dua konstanta berbeda nilai di dua layer).

**JANGAN DIUBAH:** Layer 1 (BL-54) tidak dihapus/digantikan — ini defense-in-depth DUA layer, keduanya tetap ada.

## C. Redaksi & Scrub Tool I/O (independen, file berbeda, bisa paralel penuh dengan A/B)

- [ ] **BL-37 Tahap 1 (P1, HIGH — bug wiring, bukan desain, PRIORITASKAN)** Sambungkan `capture_tool_input`/`capture_tool_output` yang sudah dirancang benar tapi NOL dipanggil produksi. Ganti `call_arguments`/`r.output` mentah di `tool_execution.rs:166-262`. File: `crates/clawcrew-runtime/src/agent/tool_execution.rs`.

#### Detail Eksekusi — BL-37 Tahap 1

**Koreksi lokasi (penting):** `capture_tool_input`/`capture_tool_output` **TIDAK ada** di `tool_execution.rs` — definisinya di crate yang BERBEDA: `crates/clawcrew-log/src/tool_io.rs:23` (`capture_tool_input(policy: &ResolvedPolicy, tool: &str, redacted: &str) -> Option<ToolIoCapture>`) dan `:32` (`capture_tool_output`, signature identik). Dokumen lama benar bahwa fungsi ini "dirancang benar tapi nol dipanggil produksi" — grep `tool_execution.rs` untuk nama fungsi ini = 0 match, mengkonfirmasi gap wiring.

**Koreksi baris:** Baris persis di `tool_execution.rs` tempat raw value ditulis ke log (padanan dari klaim dokumen `166-262`) adalah **baris ~259** (`"input": call_arguments` di dalam `::clawcrew_log::record!(DEBUG, ...)`, fungsi `execute_one_tool`) — nomor baris SUDAH BERGESER dari dokumen lama, verifikasi ulang nomor persis saat eksekusi (`grep -n '"input": call_arguments'`) karena file terus berubah.

**Signature `capture_tool_input`/`capture_tool_output` butuh `redacted: &str`** — ARTINYA caller WAJIB men-scrub/leak-scan INPUT terlebih dahulu SEBELUM memanggil fungsi ini (dokumentasi fungsi sendiri: *"the input string AFTER the runtime has scanned it for credential leaks (using `clawcrew_runtime::security::LeakDetector`)"*). Ini urutan 2 langkah: (1) leak-scan via `LeakDetector` (`crates/clawcrew-runtime/src/security/` — cari exact path, kemungkinan dekat `landlock.rs`/`seatbelt.rs` di modul `security`), (2) `capture_tool_input(policy, tool, redacted)` untuk truncation+denylist.

**File & simbol spesifik:**
- Baca: `crates/clawcrew-log/src/tool_io.rs` (sudah dibaca penuh sesi ini) — `ToolIoCapture { text, original_bytes, truncated }`, `ResolvedPolicy`, `ToolIoPolicy::{Off,Full,Redacted}`.
- Cari: `crates/clawcrew-runtime/src/security/` untuk `LeakDetector` exact path dan signature method scan-nya (BELUM diverifikasi sesi ini — WAJIB sebagai langkah pertama tim eksekusi).
- Ubah: `crates/clawcrew-runtime/src/agent/tool_execution.rs` baris ~259 (titik `"input": call_arguments`) dan titik serupa untuk output (grep `r.output`/`result.output` di sekitar baris ini untuk titik kedua).

**Spesifikasi API (pola integrasi):**
```rust
// Sebelum (saat ini, tool_execution.rs ~baris 254-261):
::clawcrew_log::record!(
    DEBUG,
    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Invoke)
        .with_category(::clawcrew_log::EventCategory::Tool)
        .with_attrs(::serde_json::json!({
            "tool": call_name,
            "tool_call_id": tool_call_id,
            "input": call_arguments, // RAW, tidak di-scan/truncate
        })),
    format!("tool call: {call_name}")
);

// Sesudah:
let leak_scanned_input = security::LeakDetector::scan(&full_args); // verifikasi API persis sebelum pakai
let captured_input = clawcrew_log::capture_tool_input(&resolved_policy, call_name, &leak_scanned_input);
::clawcrew_log::record!(
    DEBUG,
    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Invoke)
        .with_category(::clawcrew_log::EventCategory::Tool)
        .with_attrs(::serde_json::json!({
            "tool": call_name,
            "tool_call_id": tool_call_id,
            "input": captured_input.map(|c| c.text), // None kalau policy Off -> field tidak muncul
        })),
    format!("tool call: {call_name}")
);
```

**Konvensi penamaan:** `resolved_policy`/`leak_scanned_input`/`captured_input` — snake_case lokal, konsisten pola existing di file (`full_args`, `tool_call_id_owned`).

**Langkah pengerjaan:**
1. Temukan `LeakDetector` exact path+signature (`code` tool `search_symbols` untuk "LeakDetector" — WAJIB sebelum menulis kode).
2. Temukan sumber `ResolvedPolicy` yang sudah tersedia di scope `execute_one_tool` (parameter `dispatch: ToolDispatchContext` kemungkinan membawa ini — baca definisi `ToolDispatchContext` untuk field policy).
3. Ganti titik `"input": call_arguments` (baris ~259) dengan pola di atas.
4. Cari titik PARALEL untuk output (`result`/`r.output` — grep ulang persis karena belum terverifikasi sesi ini) dan terapkan `capture_tool_output` dengan pola sama.
5. Test: aktifkan `ToolIoPolicy::Redacted` di config test, panggil tool dengan argumen mengandung string yang menyerupai API key (`sk-xxxx...`), assert log event TIDAK mengandung string asli (hanya placeholder/truncated).

**Acceptance criteria testable:** Test baru `tool_io_capture_redacts_and_truncates_in_debug_log` — simulasikan `ToolIoPolicy::Redacted`, assert field `input`/`output` di log event event ter-redacted, BUKAN raw `call_arguments.to_string()`.

**Dependency:** BL-37 Tahap 2 (di bawah) SETELAH tahap ini. BL-38 (independen, file config berbeda — bisa paralel, lihat catatan di bawah).

**JANGAN DIUBAH:** `ToolIoCapture`/`capture_tool_input`/`capture_tool_output`/`capture_with_policy` di `tool_io.rs` — API ini sudah benar sesuai dokumentasinya sendiri, tiket ini murni WIRING caller, bukan mengubah fungsi itu.

- [ ] **BL-37 Tahap 2 (SETELAH Tahap 1)** Pisah axis policy: technical tool I/O (default truncated) vs conversational tool I/O (default `Off`). File: `crates/clawcrew-config/src/schema.rs`, `crates/clawcrew-log/src/{tool_io.rs,config.rs}`.

#### Detail Eksekusi — BL-37 Tahap 2

**File & simbol spesifik:** `ToolIoPolicy` enum (dipakai di `tool_io.rs`, didefinisikan kemungkinan di `clawcrew-log/src/config.rs` — BELUM diverifikasi sesi ini, WAJIB cek sebelum eksekusi) saat ini SATU axis (`Off|Full|Redacted`) berlaku untuk SEMUA tool. Tiket ini minta DUA axis terpisah: tool teknis (file-write, shell, dll — default `Redacted`/truncated) vs tool conversational (kirim pesan ke user/channel — default `Off`, karena isi percakapan user adalah data sensitif yang berbeda kelas dari argumen command teknis).

**Langkah pengerjaan:**
1. Baca `crates/clawcrew-config/src/schema.rs` untuk struktur `ResolvedPolicy`/`ToolIoPolicy` existing persis.
2. Tentukan cara membedakan "tool teknis" vs "tool conversational" — kemungkinan via klasifikasi existing di tool registry (`ToolManifest`? — cek apakah ada field kategori tool yang sudah ada, JANGAN buat klasifikasi baru kalau sudah ada yang bisa direuse).
3. Tambah field policy baru (`technical_tool_io: ToolIoPolicy`, `conversational_tool_io: ToolIoPolicy`) dengan default berbeda, atau skema lain yang konsisten dengan pola existing — baca dulu bagaimana field policy lain di `schema.rs` didefinisikan (default value, serde attribute) untuk konsistensi.
4. Update `capture_with_policy`/callsite BL-37 Tahap 1 untuk memilih axis yang tepat berdasarkan kategori tool.

**ASUMSI:** Klasifikasi "tool teknis vs conversational" belum ada precedent di kode (perlu verifikasi `ToolManifest` saat eksekusi) — ini mungkin perlu field baru di tool registry, bukan hanya di config policy. Scope size (SETELAH Tahap 1, tidak diberi size letter di dokumen asli) mungkin lebih besar dari asumsi awal kalau klasifikasi tool genuinely belum ada.

**Dependency:** BL-37 Tahap 1 (hard, prasyarat).

- [ ] **BL-38 (P1, MEDIUM, independen dari BL-37, bisa paralel)** Terapkan `scrub_credentials` di jalur `record!` (`tool_execution.rs:234-266`) — gap terpisah dari BL-37, soal kredensial bukan konten percakapan.

#### Detail Eksekusi — BL-38

**Verifikasi:** `scrub_credentials` SUDAH diimport di `tool_execution.rs:16` (`use super::loop_::{..., scrub_credentials};`) dan SUDAH dipakai minimal 1 titik: baris ~96 (`result: Some(scrub_credentials(&reason))` — di cabang "Unknown tool" error path). **Ini KOREKSI terhadap dokumen**: `scrub_credentials` BUKAN nol dipakai — sudah dipakai di jalur ERROR, tapi (sesuai klaim dokumen) kemungkinan BELUM dipakai konsisten di jalur SUCCESS/semua `record!` call lain di file ini. Perlu inventarisasi SEMUA titik `record!`/`ObserverEvent` di `tool_execution.rs` yang membawa data mentah (`full_args`, hasil tool, dll.) dan cek mana yang SUDAH vs BELUM di-scrub.

**Langkah pengerjaan:**
1. `grep -n "record!\|ObserverEvent::" crates/clawcrew-runtime/src/agent/tool_execution.rs` — list SEMUA titik emit log/observer event.
2. Untuk tiap titik, cek apakah data yang dibawa (`full_args`, `result`, `output`) sudah lewat `scrub_credentials` — baris 96 sudah benar (contoh pola yang ditiru), titik lain (termasuk baris ~259 yang jadi fokus BL-37 Tahap 1 — KOORDINASIKAN dua tiket ini, mereka menyentuh titik yang SAMA) kemungkinan belum.
3. Tambahkan `scrub_credentials(...)` di setiap titik yang belum, konsisten pola baris 96.

**Catatan koordinasi dengan BL-37 Tahap 1 (BUKAN hard dependency, tapi file-overlap erat):** Kedua tiket menyentuh baris yang berdekatan/sama di `tool_execution.rs`. BL-37 Tahap 1 menambah `capture_tool_input`/`capture_tool_output` (truncation+denylist policy), BL-38 menambah `scrub_credentials` (deteksi+redaksi kredensial, orthogonal ke truncation). Urutan yang MASUK AKAL: `scrub_credentials` dulu (BL-38, deteksi kredensial di raw string), BARU `capture_tool_input` (BL-37, truncate hasil yang sudah di-scrub) — ini KONSISTEN dengan dokumentasi `capture_tool_input` sendiri yang mengharapkan parameter `redacted: &str` SUDAH scrubbed. **Rekomendasi: kerjakan BL-38 SEBELUM BL-37 Tahap 1 pada titik yang overlap**, meskipun dokumen asli menandai keduanya independen — ini koreksi urutan berdasarkan ketergantungan data (output BL-38 adalah input BL-37), beri tahu Lead kalau urutan sprint perlu disesuaikan.

**Acceptance criteria testable:** Panggil tool dengan argumen mengandung string menyerupai kredensial (`AKIA...`, `sk-...`, dll — pola yang sudah dipakai test `scrub_credentials` kalau ada, cek `loop_.rs` untuk test existing) di SEMUA jalur (bukan hanya error path baris 96) → log event tidak mengandung string asli.

**JANGAN DIUBAH:** `scrub_credentials` function itu sendiri (`super::loop_::scrub_credentials`) — tiket ini murni WIRING ke titik tambahan, bukan mengubah logic scrub.

- [ ] **BL-39 (P2, MEDIUM)** Proteksi ACL file log Windows (`writer.rs:403-420` hanya punya branch `#[cfg(unix)]`).

#### Detail Eksekusi — BL-39

**TIDAK DIVERIFIKASI sesi ini** (file `writer.rs` di luar budget baca sesi ini — kemungkinan `crates/clawcrew-log/src/writer.rs`, belum dikonfirmasi). **Langkah pertama WAJIB:** baca file ini, konfirmasi baris persis `#[cfg(unix)]` yang disebut (nomor baris bisa sudah bergeser seperti ditemukan di tiket-tiket lain sprint ini), dan cek APA yang dilakukan branch unix (chmod 600? ACL POSIX?) sebagai referensi perilaku yang harus disetarakan di Windows.

**Spesifikasi API (pola umum, verifikasi API Windows persis saat eksekusi):**
```rust
#[cfg(windows)]
fn restrict_log_file_acl(path: &Path) -> std::io::Result<()> {
    // Windows: gunakan crate `windows-acl` atau panggilan API SetNamedSecurityInfo
    // untuk membatasi akses file log hanya ke current user (setara chmod 600 unix).
    // VERIFIKASI dulu apakah crate ini sudah ada di Cargo.toml dependencies —
    // JANGAN tambah dependency baru tanpa cek dulu apakah ada alternatif
    // yang sudah dipakai crate lain di workspace (ponytail: reuse > tambah).
}
```

**Langkah pengerjaan:**
1. Baca `writer.rs` (lokasi exact path perlu dikonfirmasi — grep `#[cfg(unix)]` di `crates/clawcrew-log/`).
2. Pahami perilaku unix branch persis (permission mode apa yang di-set).
3. Cek dependency Cargo.toml workspace untuk Windows ACL crate yang sudah terpasang (`windows`/`windows-sys` crate sudah dipakai luas di codebase ini untuk fitur Windows lain — kemungkinan besar reuse, bukan dependency baru).
4. Implementasi branch `#[cfg(windows)]` setara.

**Acceptance criteria testable:** Test yang jalan HANYA di Windows CI (`#[cfg(windows)] #[test]`) memverifikasi file log yang baru dibuat punya ACL yang membatasi akses ke current user saja.

**ASUMSI:** Severity P2/MEDIUM mengasumsikan ini bukan vulnerability aktif-dieksploitasi (file log di Windows mungkin sudah terlindungi oleh permission folder parent) — kalau verifikasi langkah 1 menunjukkan file log benar-benar world-readable di Windows hari ini, severity ini mungkin perlu dinaikkan, catat temuan ke Lead.

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
