---
title: Resource Management saat Spawn Sub-Agent — Audit Gap Kritis vs Kiro Crew & Antigravity
status: research
date: 2026-10-05
author: squad-lead
depends_on: agent-subagent-ship-squad-permission-scheme.md
---

# Resource Management saat Spawn Sub-Agent — Audit Gap Kritis

> **Konteks:** Owner melaporkan pengalaman nyata: Agent (termasuk sesi KiroCrew-dashboard yang menjalankan
> saya sendiri) kadang spawn sub-agent dalam jumlah besar (sampai ~20) tanpa cek memori lebih dulu, menyebabkan
> aplikasi crash. Owner juga mengamati Kiro Crew dan Antigravity sudah punya memory management lumayan baik,
> TAPI keduanya masih punya kelemahan: tidak bisa auto-kill sub-agent yang loop/zombie — harus manual oleh
> user, dan user tidak tahu sub-agent mana yang bermasalah.
>
> **Status:** Riset murni. Tidak mengubah `decisions/` atau `dev-tasks/`.

---

## 1. Temuan Utama: Galleon TIDAK Punya Resource-Aware Spawning Sama Sekali

Audit `crates/clawcrew-runtime/src/tools/delegate/mod.rs` (jalur spawn sub-agent via `DelegateTool`) dan modul terkait menemukan **kebalikan dari yang Owner kira** — Galleon bukan "sudah ada tapi kurang", tapi **belum ada mekanisme ini sama sekali**:

| Mekanisme yang dicari | Status di Galleon |
|---|---|
| Cek RAM/CPU tersedia SEBELUM spawn sub-agent | ❌ **Tidak ada.** `DelegateTool` struct tidak punya field/referensi ke `process_stats`/`sysinfo` sama sekali. |
| Cap jumlah sub-agent PARALEL (lebar, bukan kedalaman) | ❌ **Tidak ada.** Config hanya punya `max_delegation_depth` (kedalaman RANTAI delegasi — A delegasi ke B delegasi ke C), tidak ada field untuk jumlah sub-agent yang jalan BERSAMAAN. |
| Queue/admission control saat resource penuh | ❌ **Tidak ada.** Tidak ditemukan semaphore atau queue yang menahan spawn baru saat kapasitas penuh. |
| Auto-detect & kill sub-agent zombie/infinite-loop | ⚠️ **Parsial, dan BUKAN untuk kasus yang Owner maksud.** `control_plane::reaper` ada (`reaper_loop`, interval 60 detik, `DEFAULT_MAX_RUNTIME_SECS` 3600 detik), TAPI ini mendeteksi task yang **heartbeat-nya berhenti** (proses OS yang menjalankannya sudah MATI dari boot sebelumnya) — bukan task yang masih hidup tapi stuck infinite-loop MEMAKAN resource. `reconcile_timed_out` hanya "mark a stale-heartbeat task timed out" (comment asli di kode) — update status DB, BUKAN membunuh task yang sedang berjalan. |
| Visibility ke user: sub-agent mana yang bermasalah | ⚠️ **Ada infrastruktur dasar** (`ProcessStats::sample()` di `process_stats.rs` — RSS+CPU% keseluruhan proses, dipakai dashboard), tapi ini angka AGREGAT satu proses, **TIDAK per-sub-agent**. Tidak ada breakdown "sub-agent #7 memakai 2GB RAM, sub-agent #12 stuck 40 menit". |

**Penjelasan arsitektur penting:** sub-agent di Galleon (`clawcrew_spawn::spawn!`) adalah **tokio task in-process**, bukan subprocess OS terpisah. Artinya SEMUA sub-agent berbagi satu proses OS dan satu alokasi memori — satu sub-agent yang leak/infinite-loop bisa menghabiskan RAM proses induk sampai OS mem-kill seluruh proses (persis skenario crash yang Owner laporkan). Ini juga berarti "kill sub-agent individual" secara teknis berarti `JoinHandle::abort()` di level tokio — **mekanisme ini SECARA TEKNIS MEMUNGKINKAN** (tokio punya API-nya), tapi **tidak ditemukan dipakai** untuk kasus auto-detect-zombie di audit ini — hanya dipakai di `DelegateAction::CancelTask` yang WAJIB dipicu AGENT secara sadar via tool call, bukan otomatis oleh sistem.

**Kesimpulan paling tajam:** laporan Owner ("Agent spawn banyak sub-agent sekaligus tanpa melihat memory") bukan persepsi — ini **gap arsitektur nyata yang terverifikasi di kode**. Tidak ada satu baris kode pun di jalur `DelegateTool` yang mengecek `ProcessStats::sample()` atau menolak spawn karena resource penuh.

---

## 2. Perbandingan dengan Kiro Crew — INI YANG OWNER SEBUT "SUDAH CUKUP BAGUS"

Sumber: `kiro.dev/docs/crew/system/`.

### 2.1 Kiro Crew PUNYA Mekanisme yang Galleon Tidak Punya — Dikonfirmasi

> *"The subagent limit shown here reflects available host memory and CPU — it can be lower than the configured maximum when headroom is tight. The `resource_status` tool exposes the same operational view to an agent so it can choose a narrower path before starting a large fan-out or build."*

> *"Crew applies one aggregate ceiling across concurrent agents. When memory becomes critically low, scheduled work defers and new work remains queued or is refused according to the caller's contract, rather than pushing the host into sustained memory pressure."*

> *"Configure aggregate limits with `resource_limits.max_total_memory_mb` and `resource_limits.max_total_processes` in `config.json`."*

**Ini PERSIS tool `resource_status` yang saya pakai sendiri di sesi ini** — bukan dokumentasi abstrak, ini mekanisme yang benar-benar saya alami: setiap kali saya mau spawn banyak sub-agent, saya dianjurkan cek `resource_status` dulu, dan hasilnya memberi tahu cap aktual (bisa LEBIH RENDAH dari config maksimal kalau RAM/CPU sedang ketat).

**Elemen konkret yang terbukti bekerja (dari pengalaman + dokumentasi):**
1. **Aggregate ceiling, bukan per-sub-agent** — satu batas gabungan untuk SEMUA sub-agent aktif, bukan cap per-sub-agent individual.
2. **Adaptive, bukan statis** — cap efektif turun otomatis saat headroom sempit, naik lagi saat lega.
3. **Durable queue** — kerja yang ditolak admit TIDAK hilang, tapi `survives a Gateway restart and return to scheduling when the host can admit them` (dari dokumentasi Tasks view).
4. **Transparansi lewat Tasks view** — dashboard "lists active sessions, subagents, scheduled work... with their current state and resource use. Use it to find a session holding memory."

### 2.2 TAPI Owner Benar — Kiro Crew TIDAK Auto-Kill Zombie Sub-Agent

Dokumentasi yang sama **tidak menyebutkan** mekanisme otomatis "deteksi sub-agent infinite-loop, kill otomatis". Mekanisme yang ada:
- `resource_limits` mencegah spawn BARU saat penuh (preventif) — tapi TIDAK membunuh sub-agent yang SUDAH jalan dan stuck.
- `CancellationToken` dkk kemungkinan ada (pola umum tokio), tapi dokumentasi publik tidak menyebut trigger OTOMATIS berbasis "sub-agent ini sudah jalan X menit tanpa progress, matikan".

**Ini mengonfirmasi pengamatan Owner secara presisi**: Kiro Crew unggul di PENCEGAHAN (jangan biarkan spawn baru kalau resource penuh), tapi tidak di PENYEMBUHAN (sub-agent yang sudah jalan dan stuck tetap harus di-handle manual). Lesson yang SUDAH tersimpan di memori sesi ini sendiri (`"spawned squad crews can VANISH with zero trace... though some eventually completed after 1+hr"`) adalah bukti langsung dari pengalaman nyata — bahkan sesi saya sendiri pernah mengalami sub-agent yang "hilang" tanpa cara pasti untuk tahu apakah dia stuck atau sedang progress lambat.

---

## 3. Perbandingan dengan Google Antigravity

Sumber: riset kompetitor sesi sebelumnya (`agent-subagent-permission-scoping-survey.md` §2.4) + pencarian tambahan.

Riset sebelumnya sudah menemukan Antigravity punya **Secure Mode** dengan sandbox virtual dan pembatasan network — tapi fokus riset itu adalah KEAMANAN (tool bypass gate), bukan RESOURCE MANAGEMENT. Owner sekarang menyebut aspek berbeda: Antigravity "sudah cukup bagus" soal memory tapi sama-sama tidak bisa auto-kill zombie sub-agent.

**Pola yang konsisten dengan temuan Antigravity sebelumnya:** mode "Turbo" Antigravity (mematikan Terminal Sandbox demi kecepatan) adalah ANTI-PATTERN yang didokumentasikan Google sendiri — ini menunjukkan trade-off yang SAMA dengan yang Galleon hadapi: resource-awareness vs kecepatan/fleksibilitas. Tidak ditemukan dokumentasi publik Antigravity yang merinci mekanisme auto-kill zombie subagent secara spesifik — konsisten dengan laporan Owner bahwa ini memang kelemahan umum, bukan sesuatu yang Galleon tertinggal secara unik.

---

## 4. Sintesis — Peta Gap Konkret

| Kemampuan | Galleon | Kiro Crew | Antigravity (dari riset sebelumnya + laporan Owner) |
|---|---|---|---|
| Cek resource SEBELUM spawn | ❌ Tidak ada | ✅ `resource_status` tool + aggregate ceiling adaptif | ✅ (menurut Owner, "cukup bagus") |
| Cap jumlah sub-agent paralel | ❌ Tidak ada (hanya depth, bukan width) | ✅ `max_total_processes` | ✅ (menurut Owner) |
| Queue saat resource penuh | ❌ Tidak ada | ✅ Durable queue, survive restart | Tidak terverifikasi |
| Auto-kill sub-agent zombie/infinite-loop | ❌ Tidak ada (reaper hanya untuk proses MATI, bukan stuck-tapi-hidup) | ❌ Tidak ada (dikonfirmasi Owner + tidak di dokumentasi) | ❌ Tidak ada (dikonfirmasi Owner) |
| Visibility per-sub-agent (mana yang bermasalah) | ⚠️ Agregat saja, bukan per-sub-agent | ✅ Tasks view (per-session breakdown) | Tidak terverifikasi, tapi Owner bilang user tetap tidak tahu mana yang stuck |

**Insight kunci:** "Auto-kill zombie sub-agent" adalah masalah yang **belum terpecahkan di industri**, bukan cuma gap Galleon. Tapi "cek resource sebelum spawn" + "cap jumlah paralel" + "queue" **SUDAH terpecahkan** oleh Kiro Crew dan terbukti bekerja — ini bagian yang paling murah untuk Galleon tutup duluan karena ada contoh konkret yang sudah berfungsi (bahkan tool `resource_status` sudah tersedia untuk saya pakai sendiri sebagai referensi pola API-nya).

---

## 5. Rekomendasi (Bahan Diskusi, Bukan Keputusan Final)

### Prioritas tinggi — bagian yang sudah terbukti bisa diselesaikan (menyusul Kiro Crew)

1. **Tambah field `resource_limits.max_total_memory_mb` dan `max_total_processes`** di `clawcrew-config` — menyusul pola Kiro Crew persis. Reuse `process_stats::sample()` yang SUDAH ADA sebagai sumber data (RSS/CPU), jangan bangun sensor baru.
2. **Gate di `DelegateTool` sebelum spawn** — sebelum `clawcrew_spawn::spawn!` dipanggil untuk delegate baru, cek `ProcessStats::sample().rss_bytes` terhadap `max_total_memory_mb` DAN hitung jumlah task `Running` berkind `Delegate` di `control_plane` terhadap `max_total_processes`. Tolak/queue kalau terlampaui — jangan silent-allow.
3. **Tool `resource_status` setara** — kalau belum ada tool MCP serupa untuk Galleon sendiri (bukan tool dashboard KiroCrew yang saya pakai saat ini), bangun satu supaya AGENT (bukan cuma dashboard) bisa cek sebelum fan-out besar — ini generalisasi langsung dari pola yang Owner sendiri alami manfaatnya di sesi ini.
4. **Durable queue untuk delegate yang ditolak admit** — reuse `control_plane` task registry yang SUDAH ADA (status `Pending`/`Queued` kemungkinan sudah ada sebagai enum, perlu dicek) — jangan buat sistem queue terpisah.

### Prioritas sedang — belum terpecahkan di industri, tapi layak coba

5. **Heuristik "stuck tapi hidup"** (BUKAN cuma heartbeat mati) — misal: sub-agent yang sudah jalan >N menit TANPA tool-call baru (bukan cuma tanpa heartbeat) ditandai "possibly stuck" dan di-surface ke user untuk keputusan manual (bukan auto-kill langsung — risiko membunuh kerja lambat tapi valid). Ini mengisi gap yang Owner identifikasi SEBAGAI MASALAH INDUSTRI, bukan cuma gap Galleon — kalau berhasil, ini keunggulan nyata dibanding Kiro Crew/Antigravity.
6. **Breakdown resource PER-sub-agent**, bukan cuma agregat proses — perlu riset lebih lanjut apakah mungkin diukur akurat untuk tokio task in-process (lebih sulit dari subprocess OS terpisah yang punya PID sendiri) — kemungkinan butuh instrumentasi manual (counter per-task-id) bukan `sysinfo` biasa.

---

## 6. KOREKSI PENTING (riset lanjutan) — Galleon SUDAH Punya Heuristik "Stuck tapi Hidup" yang Matang

> Bagian §5 poin 5 di atas menyimpulkan "stuck tapi hidup" belum terpecahkan dan Galleon belum punya mekanismenya.
> **Audit kode lanjutan membuktikan kesimpulan itu SALAH untuk Galleon** — berikut koreksinya, mengikuti aturan
> "setiap klaim harus diverifikasi ke kode, bukan asumsi."

### 6.1 `agent/loop_detector.rs` — Loop detector 3-pattern, sudah matang dan sudah DIPASANG

File `crates/clawcrew-runtime/src/agent/loop_detector.rs` (test suite besar, termasuk reference-oracle untuk hash) mengimplementasikan **persis** heuristik "stuck tapi hidup" yang dicari — bukan dari heartbeat/timer, tapi dari **analisis pola tool-call nyata**:

1. **Exact repeat** — tool yang sama + argumen identik dipanggil berulang. Eskalasi 3 tingkat: `max_repeats` → `Warning`, `+1` → `Block`, `+2` → `Break` (circuit breaker, turn dihentikan).
2. **Ping-pong** — dua tool bergantian (A→B→A→B) selama ≥4 siklus penuh. Eskalasi serupa.
3. **No-progress** — tool yang sama dipanggil berulang dengan ARGUMEN BERBEDA tapi HASIL IDENTIK (`no_progress_min_calls`, default 5× → `Warning`, lebih → `Block`, **tidak pernah `Break`** karena hasil identik dari argumen berbeda ambigu — bisa jadi pencarian kosong yang valid, bukan pasti stuck). Window sliding 20 entri, deteksi non-consecutive (interleaved call lain tidak mereset streak) — komentar kode menyebut kasus nyata yang memotivasi ini: *"the stuck agent ran 43 near-duplicate shell calls returning byte-identical output, interleaved with other tools."* Ini bukti bahwa pola persis yang Owner laporkan **sudah pernah terjadi dan sudah ditangani**.

**Wiring (dikonfirmasi, bukan asumsi):** dipasang di `agent/turn/mod.rs:1147` dan dieksekusi di `agent/turn/results_collect.rs` SETIAP tool-call result diproses — ini generic untuk **setiap turn**, bukan spesifik agent utama. Karena sub-agent (`DelegateTool`) pada akhirnya menjalankan mesin turn/loop yang SAMA (`agent/loop_/mod.rs`), loop detector ini **otomatis berlaku juga untuk sub-agent**, tanpa kode tambahan.

**Yang terjadi saat `Break` dipicu** (`results_collect.rs:118`): `anyhow::bail!("Agent loop aborted by loop detector: {msg}")` — ini benar-benar menghentikan turn via Rust error propagation, BUKAN sekadar log. Untuk sub-agent, error ini akan naik ke `DelegateTool`'s task wrapper dan menandai task tersebut `Failed` di `control_plane` — tokio task dilepas, resource (memori history, dsb.) di-reclaim oleh Rust ownership/drop semantics secara otomatis.

**Independen dari `max_tool_iterations`:** loop detector ini adalah lapis TAMBAHAN di atas `max_tool_iterations` (hard cap default 10, lihat `agent/turn/mod.rs:109`). Komentar kode menegaskan keduanya saling melengkapi: *"A genuinely stuck agent still terminates at `max_tool_iterations`, where the graceful summary runs"* — artinya ada 2 lapis pertahanan: loop detector (deteksi pola dini, bisa lebih cepat dari iterasi max) + hard iteration cap (jaring pengaman terakhir kalau pola tidak terdeteksi).

### 6.2 Revisi Peta Gap (menggantikan tabel §4 untuk baris "Auto-kill zombie")

| Kemampuan | Galleon (REVISI) | Kiro Crew | Antigravity |
|---|---|---|---|
| Deteksi stuck berbasis POLA perilaku (bukan cuma timeout) | ✅ **ADA & matang** — `LoopDetector` 3-pattern, auto-terminate turn pada `Break` | Tidak terverifikasi di dokumentasi publik | Tidak terverifikasi |
| Auto-kill PROSES OS yang zombie (resource masih dipakai walau terdeteksi stuck) | ⚠️ Untuk sub-agent in-process: `Break` sudah melepas tokio task-nya (lihat 6.1) — TAPI untuk kasus hang di LUAR tool-call loop (misal stuck di satu panggilan LLM API yang tidak pernah response) loop detector TIDAK membantu karena dia bereaksi ANTARA tool call, bukan selama satu call berlangsung. | Sama-sama tidak ada mekanisme publik untuk kasus ini | Sama |
| Visibility ke user: sub-agent mana yang terdeteksi stuck | ⚠️ Loop detector menulis ke structured log (`clawcrew_log::record!` dengan `trace_id`), TAPI tidak dikonfirmasi ada surfacing ke dashboard/UI secara real-time — perlu verifikasi lanjutan apakah log ini sampai ke Tasks-view setara Galleon. | Tasks view per-session (dari §2) | Tidak terverifikasi |

### 6.3 Gap yang MASIH valid setelah koreksi ini

Loop detector menutup kasus "stuck KARENA pola tool-call berulang" — tapi TIDAK menutup:
- **Hang di satu panggilan LLM/tool call tunggal yang tidak pernah selesai** (network stuck, provider API hang) — loop detector baru bereaksi SETELAH sebuah call selesai dan hasilnya tercatat; kalau call itu sendiri tidak pernah kembali, tidak ada mekanisme eskalasi dari sisi ini. Ini kemungkinan domain `delegate_config`'s default timeout values (disebut di struct `DelegateTool`, field `delegate_config: DelegateToolConfig`) — **belum diverifikasi** apakah ada timeout per-tool-call yang membungkus ini.
- **Memory leak gradual tanpa pola tool-call berulang** (misal: history turn yang terus bertambah tanpa pruning efektif) — loop detector tidak mendeteksi pertumbuhan memori, hanya pola PERILAKU tool-call.
- **Agregasi di level banyak sub-agent sekaligus** — loop detector bekerja PER-TURN (per sub-agent individual), tidak ada korelasi "5 dari 20 sub-agent paralel semuanya menunjukkan pola stuck yang sama" yang di-surface sebagai satu insight ke user.

### 6.4 Rekomendasi Baru (menggantikan §5 poin 5)

~~5. Heuristik "stuck tapi hidup"~~ **SUDAH ADA, cabut dari backlog "belum terpecahkan."** Ganti rekomendasi dengan:

5. **Verifikasi timeout per-tool-call tunggal** di `DelegateToolConfig` — pastikan ada batas waktu keras untuk SATU tool-call/LLM-request yang tidak pernah response, independen dari loop detector (yang hanya bereaksi antar-call).
6. **Surface `loop_detector_circuit_breaker` log event ke dashboard/Tasks-view** — saat ini hanya tercatat di structured log dengan `trace_id`; kalau belum ada UI yang menampilkan ini secara real-time per sub-agent, ini adalah perbaikan murah (reuse infrastruktur ada) untuk langsung menjawab keluhan Owner "user tidak tahu mana sub-agent yang bermasalah."
7. **Reuse `LoopDetector` pattern untuk agregasi level-fleet** — kalau Fleet/Ship punya banyak sub-agent paralel, pertimbangkan korelasi lintas-sub-agent (bukan cuma per-sub-agent individual) sebagai lapis observability tambahan — ini genuinely baru, belum ada precedent di Kiro Crew/Antigravity sejauh diriset.

---

## 8. Verifikasi Lanjutan — Timeout Per-Call (KONFIRMASI POSITIF) & Dashboard Surfacing (KONFIRMASI GAP)

### 8.1 Timeout per-tool-call/per-run — SUDAH ADA dan LEBIH BAIK dari asumsi §6.3

`DelegateToolConfig` (`crates/clawcrew-config/src/schema.rs:3731`) punya 2 field timeout, keduanya dengan default dan bisa di-override per-agent (`[agents.<name>]`):

| Field | Default | Lingkup |
|---|---|---|
| `timeout_secs` | 120s | Sub-agent **non-agentic** (satu model-provider call tunggal) |
| `agentic_timeout_secs` | 300s | Sub-agent **agentic** (loop tool-call penuh) |

**Wiring terverifikasi** (`tools/delegate/mod.rs`):
- Baris 2305: `tokio::time::timeout(Duration::from_secs(timeout_secs), ...)` — membungkus jalur non-agentic.
- Baris 4001: `tokio::time::timeout(Duration::from_secs(agentic_timeout_secs), run_tool_call_loop(ToolLoop { ... }))` — membungkus **seluruh** `run_tool_call_loop`, bukan cuma satu call di dalamnya.

**Ini menjawab langsung kekhawatiran di §6.3**: `tokio::time::timeout` di Rust membatalkan (drop) future yang dibungkus begitu durasi terlampaui — termasuk kalau future itu sedang STUCK menunggu response LLM API yang tidak pernah datang. Jadi hang di dalam satu LLM call tunggal **sudah tertangani**, bukan gap. Kedua lapis (`timeout_secs`/`agentic_timeout_secs`) ini independen dari `LoopDetector` (§6.1) — satu melindungi dari "hang diam" (tidak ada aktivitas sama sekali), satu melindungi dari "sibuk tapi tidak maju" (ada aktivitas tool-call tapi berulang/tidak produktif). Kombinasi keduanya menutup kedua sisi masalah "stuck tapi hidup."

**Revisi §6.3**: poin pertama ("hang di satu panggilan LLM/tool call tunggal") **dicabut dari daftar gap** — sudah tertutup `agentic_timeout_secs`/`timeout_secs`.

### 8.2 Surfacing `loop_detector_circuit_breaker` ke dashboard — KONFIRMASI GAP NYATA

Audit menyeluruh membuktikan **tidak ada jalur** dari event ini ke mana pun selain file log lokal:

1. **Hanya 1 occurrence** dari string `"loop_detector_circuit_breaker"` di SELURUH codebase (`agent/turn/results_collect.rs:118`) — tidak ada consumer, tidak ada query, tidak ada test yang mencarinya kembali.
2. **Tujuan log**: `clawcrew_log::record!` → macro `tracing::event!` → `writer::record_event()` (`clawcrew-log/src/writer.rs:553`) → ditulis ke file **`runtime-trace.jsonl`** (rotated by date/size, murni file lokal di disk).
3. **Tidak ada reader**: grep `runtime-trace` di seluruh `crates/` hanya muncul di `writer.rs` itu sendiri (penulisan + testnya) — tidak ada modul lain yang membaca file ini kembali untuk ditampilkan.
4. **Go engine (dashboard/API utama produk) nol pengetahuan**: grep `loop_detector|circuit_breaker|CircuitBreaker|LoopDetection` di seluruh `engine/` = 0 hasil. Dashboard Go tidak punya field, endpoint, atau konsep apa pun untuk event ini.
5. Hit awal di `rpc/types.rs` dan `gateway/ws.rs` dari pencarian gabungan ternyata **false-positive** (regex match ke term lain, bukan term loop-detector) — dikonfirmasi ulang dengan pencarian spesifik, hasil nihil di kedua file.

**Kesimpulan tegas**: event sirkuit-breaker ini terjebak sebagai **baris JSON di file lokal yang tidak pernah dibaca ulang oleh sistem apa pun**. Ini PERSIS gap yang menjelaskan keluhan asli Owner — *"user tidak tahu mana sub agent yang terkendala"* — walau mesin deteksinya sendiri (`LoopDetector`) sudah matang, hasil deteksinya tidak pernah sampai ke mata user. Ini bukan masalah "mekanisme deteksi lemah", tapi murni **masalah observability/wiring** yang terisolasi dan relatif murah diperbaiki (reuse `control_plane::TaskRegistry` yang sudah ada: tambahkan field `last_loop_detector_event: Option<String>` pada `TaskRecord`, tulis saat `Break` terjadi di `handle_delegate`/jalur agentic, baca dari situ di Tasks-view).

---

## 9. Sumber

- Kode diaudit §8 (verifikasi lanjutan): `crates/clawcrew-config/src/schema.rs:3731` (`DelegateToolConfig`, `timeout_secs`, `agentic_timeout_secs`), `crates/clawcrew-runtime/src/tools/delegate/mod.rs:2301-2320,3985-4120` (wiring `tokio::time::timeout` non-agentic & agentic), `crates/clawcrew-log/src/writer.rs:553,1035` (`record_event`, rotasi `runtime-trace.jsonl`), `crates/clawcrew-log/src/macro.rs` (`record!`)
- Kode diaudit §6 (koreksi loop detector): `crates/clawcrew-runtime/src/agent/loop_detector.rs` (full read — `LoopDetector`, `LoopDetectorConfig`, `LoopDetectionResult`, 3 detector), `crates/clawcrew-runtime/src/agent/turn/results_collect.rs:60-120` (wiring + `anyhow::bail!` pada `Break`), `crates/clawcrew-runtime/src/agent/turn/mod.rs:109,1147`, `crates/clawcrew-runtime/src/agent/history_pruner.rs:215`
- Kode diaudit §1-5 (temuan awal): `crates/clawcrew-runtime/src/tools/delegate/mod.rs` (`DelegateTool`, `handle_cancel_task`, `DelegateAction`), `crates/clawcrew-runtime/src/process_stats.rs`, `crates/clawcrew-runtime/src/control_plane/reaper.rs`, `crates/clawcrew-runtime/src/control_plane/task_registry.rs`, `crates/clawcrew-config/src/schema.rs:14316` (`max_delegation_depth`)
- Kiro Crew System & Storage: https://kiro.dev/docs/crew/system/
- Kiro Crew Subagents: https://kiro.dev/docs/crew/features/subagents/
- Dokumen terkait: `agent-subagent-permission-scoping-survey.md` (§2.4 Antigravity Secure Mode)
- Pengalaman langsung sesi ini: tool `resource_status` (dipakai sebelum spawn besar), lesson tersimpan soal sub-agent yang "vanish" tanpa jejak jelas
