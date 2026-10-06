---
title: Audit — Turn Resilience (Crash/Disconnect/Token-Habis) & Steer vs Queue
status: draft
date: 2026-10-05
author: squad-lead
---

# Audit: Resiliency Turn/Sub-Agent & Steer vs Queue

> **Konteks pertanyaan Owner (2026-10-05):**
> 1. Ketika Agent sedang menjalankan task dan sudah spawn sub-agent, namun terhenti karena koneksi internet buruk/crash/token habis — apa sudah di-handle?
> 2. Ketika Agent sedang proses query lalu user mengajukan chat tambahan — apa sudah di-handle? Ekspektasi: user bisa pilih 2 opsi (olah pesan baru / antrian).
>
> Scope audit: **kode produk galleon-fleet** (`engine/` Go + `crates/clawcrew-runtime` Rust) dibandingkan dengan **platform KiroCrew resmi** (dashboard yang menjalankan sesi ini) dan **Google Antigravity**, sebagai referensi metode penanganan industri.

## Cek Riset Sebelumnya — Tidak Ada Duplikasi

Grep terhadap `standard-docs/backlog/` dan `standard-docs/research/` untuk `resume|checkpoint|crash|interrupt|steer|queue|in-flight`: **tidak ada riset sebelumnya yang menyentuh topik spesifik ini.** Hit terdekat adalah `subagent-resource-management-backlog.md` (BL-15/16/17) — tapi itu soal **resource-gate crash OOM saat spawn ~20 sub-agent sekaligus**, bukan soal **crash recovery setelah sub-agent berjalan** atau **steer/queue pesan baru**. Dua topik ini genuinely baru, bukan perluasan dari backlog yang ada.

---

## 1. Crash/Disconnect/Token-Habis — Status di galleon-fleet (Verifikasi Kode)

### Yang SUDAH ada (verified, bukan asumsi)

**`engine/src/run/interfaces.go` + `engine/pkg/store/sqlite/run_store.go`** — ada mekanisme `ResumableStore`:

```go
// ResumableStore is an optional capability: a disk-backed Store recovers runs
// left mid-flight by an engine restart. In-memory stores do not implement it.
type ResumableStore interface {
	ResumeInterruptedRuns(ctx context.Context) ([]*Run, error)
}
```

`RunStore.ResumeInterruptedRuns` (SQLite, `run_store.go:131`) query `WHERE status IN ('queued', 'planning', 'running', 'waiting_for_input', 'cancelling')` — jadi KALAU **engine Go-nya sendiri** yang restart/crash (proses dibunuh, server reboot), run yang levelnya belum terminal akan ditemukan lagi saat boot dan di-emit event `run.resumed` (`services.go` `ResumeInterrupted`). Ini **real dan teruji** (`run_store.go` + `disk_store_test.go` ada test), bukan placeholder.

### Yang BELUM ada — gap konkret, bukan asumsi

**Gap #1 — tidak ada apapun untuk disconnect koneksi LLM provider di tengah turn.** Dibaca langsung dari `engine/src/crew/services.go::StartTurn` (baris 38-232):

- Stream `ExecuteTurn` ke Rust gateway di-`Recv()` dalam loop blocking (baris ~108). Kalau koneksi provider (OpenAI/Anthropic/dll, via `clawcrew-providers` di Rust) putus di tengah, `stream.Recv()` mengembalikan error non-`io.EOF`, lalu fungsi langsung `return appErrors.Wrap(err, ...)` — **turn mati total, tidak ada retry otomatis, tidak ada checkpoint token yang sudah di-generate sejauh itu untuk dilanjutkan.**
- Tidak ada state SOP/token-progress yang disimpan per-chunk — `TurnEvent` hanya dikirim ke `eventCh` (channel in-memory), **tidak pernah ditulis ke `RunStore`/SQLite**. Begini: `ResumableStore` di atas mencatat STATUS run (`running`/`queued`/dst), bukan ISI percakapan yang sudah di-generate. Jadi "resume" yang ada hari ini artinya **run-nya ditemukan lagi dan di-flag perlu perhatian**, BUKAN "lanjutkan dari token terakhir" — itu beda level granularitas.
- **Token habis (rate limit/quota)**: grep `clawcrew-providers` untuk retry/backoff — ada retry logic provider-level (lihat `dispatch/accounting.rs`), tapi ini untuk **transient error single-request**, bukan untuk "budget token sesi habis di tengah SOP multi-step". Tidak ada mekanisme checkpoint SOP-step yang menyimpan "step mana yang terakhir sukses" supaya SOP run bisa di-lanjut dari situ setelah token top-up — `SopRun`/`SopStep` (lihat `crates/clawcrew-runtime/src/sop/types.rs`) PUNYA field status per-step, tapi saya belum verifikasi apakah ada endpoint "resume dari step N" yang dipakai operator (perlu dicek terpisah, di luar scope audit kali ini — DITANDAI sebagai follow-up, lihat §Rekomendasi).

**Gap #2 — sub-agent (goroutine) TIDAK punya persistence record sendiri.** Dibaca langsung dari `services.go`:

```go
subagentWg.Add(1)
go func(subID, subTask string) {
    defer subagentWg.Done()
    s.runSubagent(turnCtx, subID, subTask, eventCh)
}(res.SubagentID, res.SubagentTask)
```

Grep `SubagentID`/`subagent_id` di seluruh `engine/`: **hanya muncul di `services.go`, `dispatcher.go`, `delegation.go`, dan proto `agent_service.pb.go` — nol tabel SQLite, nol struct yang mem-persist status sub-agent.** Konsekuensi konkret:
- Sub-agent adalah goroutine in-process (`go func(...)`), persis sama dengan temuan `subagent-resource-management-backlog.md` BL-15 soal `DelegateTool`/`clawcrew_spawn::spawn!` — **kalau proses Go mati (crash/OOM/restart), SEMUA goroutine sub-agent yang sedang jalan hilang total, tanpa jejak status apa pun untuk di-resume.** `ResumeInterruptedRuns` di atas hanya tahu Run level (`StatusRunning`), tidak tahu sub-agent mana yang sedang jalan di dalamnya atau progressnya.
- Tidak ada field "parent run tetap running, 2 dari 3 sub-agent re-attachable" — begitu proses mati, hubungan parent-run↔sub-agent putus total, Run yang di-resume akan start ulang dari awal (bukan resume sub-agent yang sempat jalan).
- `subagentWg.Wait()` di `defer` HANYA menjamin graceful-shutdown kalau proses MASIH hidup (misal context dibatalkan via `CancelRun`) — ini tidak membantu kasus proses benar-benar mati.

**Kesimpulan Gap #2**: ini sama persis dengan pola "configuration-lies" versi lain yang beberapa kali ditemukan sesi-sesi sebelumnya — `ResumableStore` ADA dan BEKERJA (bukan klaim kosong), tapi cakupannya **cuma Run, tidak turun ke granularitas sub-agent**. Siapa pun yang baca nama `ResumeInterruptedRuns` tanpa membaca isi fungsinya bisa salah asumsi "sub-agent juga di-resume" — padahal tidak.

### Perbandingan dengan platform KiroCrew (dashboard yang menjalankan sesi ini)

KiroCrew (produk `kirodotdev/Kiro` + Crew, bukan kode galleon-fleet) **punya cakupan lebih luas** untuk kasus yang sama:

| Aspek | KiroCrew (platform resmi) | galleon-fleet (kode produk, saat ini) |
|---|---|---|
| Proses induk/gateway mati | **Durable queue**: "Crew writes accepted subagent work into a durable task queue before acknowledging it. Queued work survives a Gateway restart and re-enters scheduling when capacity is available... reconciled conservatively so work with uncertain side effects is not silently repeated." | `ResumableStore` di level Run SAJA — tidak ada queue durable di level sub-agent, tidak ada "reconciled conservatively" (begitu proses mati, state goroutine sub-agent hilang, tidak ada mekanisme anti-dobel-eksekusi saat resume) |
| Sub-agent gagal/crash sendiri (bukan proses induk) | **Retry button** eksplisit per sub-agent row — "spawns a fresh attempt with the same task" (bukan resume state lama, tapi RE-RUN task yang sama dengan hasil kerja sebelumnya dibuang) | **Nol** — tidak ada state "failed" untuk sub-agent sama sekali (karena tidak di-persist), apalagi tombol retry |
| Koneksi/stream provider putus mid-turn | Tidak eksplisit didokumentasikan sebagai "resume dari token terakhir" juga (konsisten dengan temuan galleon-fleet: ini genuinely sulit secara arsitektur LLM-stateless) — tapi **stall detection**: "a row that stops producing activity is marked stalled so you can inspect or stop it; Crew does not treat silence alone as proof that useful work ended" | Tidak ada stall-detection — proses yang hang tanpa error eksplisit (bukan `io.EOF`/error) akan terus menunggu di `stream.Recv()` tanpa batas waktu (tidak ada context timeout eksplisit terlihat di `StartTurn`, hanya `turnCtx.Done()` yang dipicu CANCEL manual, bukan oleh deteksi stall otomatis) |
| Token/context habis mid-session | **Auto-compaction**: "automatically summarizing older conversation history as your session grows... only the verbose middle gets compressed" — trigger otomatis saat context window penuh | Tidak ditemukan mekanisme setara di `engine/`/`crates/clawcrew-runtime` untuk SOP run — perlu verifikasi terpisah apakah `clawcrew-providers`/`agent/loop_` punya compaction (DITANDAI follow-up) |
| User berhenti paksa (bukan crash) | "Stopping is neutral — partial output is preserved and marked as 'stopped by user,' not as a failure." | `CancelRun` ADA dan bekerja (`StatusCancelling`→`StatusCancelled`, context cancel dipicu) — ini SETARA, galleon-fleet sudah benar di titik ini |

### Perbandingan dengan Google Antigravity

Riset eksternal Antigravity (Agent Manager, produk Google) menunjukkan **kelas masalah yang SAMA justru jadi bug paling sering dilaporkan** — bukan metode yang terbukti sudah solved dengan elegan:
- "Agent terminated due to error" — generic failure message untuk crash runtime, solusi yang didokumentasikan user: mulai task/thread BARU (bukan resume) — **tidak lebih baik dari galleon-fleet**, bahkan tanpa durable-queue sekelas KiroCrew.
- Kasus nyata index SQLite corrupt setelah Windows force-restart: "Antigravity's sidebar index — a cached protobuf inside a SQLite database — never got a chance to flush before Windows pulled the plug. When Antigravity came back up, it restored a stale copy... your recent conversations simply don't exist" — ini JUSTIFIKASI kuat untuk galleon-fleet TIDAK meniru pendekatan cache-protobuf-tanpa-flush-guarantee Antigravity; SQLite `RunStore` galleon-fleet pakai `INSERT ... ON CONFLICT` per-event (lebih aman), tapi perlu dipastikan ada `fsync`/WAL mode yang benar (DITANDAI follow-up, bukan klaim pasti).
- "Completed Agent terminal tasks remain Running and leave uncancellable high-CPU log processes" — mirip pola stall yang BELUM ter-cover di galleon-fleet (tidak ada stall-detection, lihat tabel atas).

**Simpulan perbandingan**: KiroCrew (platform resmi) adalah referensi yang LEBIH baik untuk ditiru dibanding Antigravity pada topik ini — Antigravity sendiri terbukti dari laporan user justru sering gagal di kelas masalah yang sama. KiroCrew punya 3 pola konkret yang applicable: **durable queue + reconcile-conservatively**, **retry-as-fresh-attempt (bukan state resume palsu)**, **stall detection terpisah dari error detection**.

---

## 2. Steer vs Queue — Status di galleon-fleet (Verifikasi Kode)

### Yang ditemukan: NOL mekanisme

Dibaca langsung `StartTurn` (`engine/src/crew/services.go`) baris demi baris — fungsi ini adalah **loop blocking tunggal**:
```go
for {
    token, err := stream.Recv()
    ...
}
```
Tidak ada channel masuk apa pun selain `turnCtx.Done()` (yang hanya dipicu `CancelRun`, bukan pesan baru). Grep proto `clawcrew-api` untuk `SteerTurn`/`InjectMessage`/`AppendMessage`: **nol hasil.** `pb.ExecuteTurnRequest` (request awal ke Rust gateway) dikirim SEKALI di awal (`protoReq`) — tidak ada RPC terpisah atau field apa pun di proto untuk mengirim pesan tambahan ke stream yang sedang berjalan.

**Konsekuensi konkret**: hari ini, kalau user kirim chat baru saat Agent (dalam arsitektur galleon-fleet: satu `Run`/`StartTurn`) masih jalan, SATU-SATUNYA jalur yang ada secara struktural adalah:
1. **Chat baru ditolak/diblokir** sampai turn selesai (kalau UI mencegah input ganda) — ini perilaku "paksa queue tanpa pilihan", ATAU
2. **Chat baru memicu `CreateRun` BARU yang terpisah** (karena `CreateRun` tidak mengecek apakah sesi punya run aktif) — ini berarti **DUA run berjalan paralel tanpa koordinasi**, bukan steer (pesan masuk ke turn yang sama) dan bukan queue (pesan tertata rapi menunggu). Ini perlu dicek lebih jauh di layer UI/web (`web/src/`) untuk tahu mana yang benar-benar terjadi — DITANDAI follow-up, karena ini di luar `engine/` murni.

**Tidak ada OPSI untuk user pilih "steer" atau "queue" sama sekali** — pertanyaan Owner soal "user dapat memilih 2 opsi" belum punya satu pun jalur struktural di kode produk saat ini (baik UI maupun backend).

### Bagaimana platform KiroCrew menangani ini (jawaban LANGSUNG untuk ekspektasi Anda)

Ini **PERSIS** mekanisme yang Anda bayangkan — sudah berjalan secara resmi di platform KiroCrew (bukan hipotesis, dikonfirmasi dari dokumentasi resmi `kiro.dev/docs/cli/chat/queue-steering/` dan `kiro.dev/docs/crew/chat/message-controls/`):

| Mode | Perilaku | Kapan dipakai |
|---|---|---|
| **Steer** (default) | Pesan tambahan dikirim KE TURN YANG SAMA, "picked up at the next safe point in its work — without interrupting an action that's already in progress". Kalau user kirim beberapa pesan steer sebelum ada "safe point", semua dikonkatenasi `\n\n` jadi SATU pesan gabungan. | Koreksi arah real-time ("fokus ke X dulu") tanpa buang progress yang sedang jalan |
| **Queue** | Pesan DIBUFFER lokal (terlihat di "activity tray", label `Queue (N)`), dikirim sebagai TURN BARU setelah turn saat ini selesai total. Multi-pesan di-queue dikirim SATU PER SATU berurutan (bukan di-batch jadi satu). | Numpuk beberapa follow-up tanpa menyela kerja yang sedang jalan |
| **Cancel + redirect** | `Ctrl+C`/`Esc` saat ada pesan steer tertunda = batalkan turn SEKARANG + otomatis kirim pesan yang tertunda sebagai turn baru ("stop that and do this instead") | User berubah pikiran total, bukan sekadar menambah arahan |

Poin penting dari dokumentasi resmi yang relevan untuk desain galleon-fleet:
- **Toggle, bukan dialog per-pesan**: `Ctrl+S` switch mode SEKALI untuk sesi itu — user tidak diminta pilih setiap kali kirim pesan (lebih ringan secara UX daripada modal "steer atau queue?" tiap chat).
- **Steer punya SATU slot pesan** — bukan antrian tak terbatas; ini mencegah steer-flood yang membingungkan agent di tengah kerja.
- **"If a steer cannot reach an active turn, Crew demotes it to a visible queue card instead of dropping it"** — fallback aman: steer yang gagal terkirim (misal turn sudah selesai duluan) TIDAK hilang, otomatis jadi entri queue.
- Pembeda jelas dari "Injected messages" (cron/subagent-completion/hook) — pesan user manual (steer/queue) secara UI DIBEDAKAN dari pesan otomatis sistem, supaya user tidak bingung asal sebuah instruksi.

### Bagaimana Antigravity menangani ini

Riset tidak menemukan dokumentasi resmi Antigravity untuk pola setara steer/queue dalam SATU task/thread. Yang ada adalah **paralelisme di level task terpisah** (bukan steer ke task yang sama): "you can have the AI perform up to three tasks in parallel... While leaving the first task to it, you can throw in a second and third" — ini artinya Antigravity's answer ke "user mau kirim kerjaan baru saat agent sibuk" adalah **selalu mulai task/thread BARU paralel**, TIDAK pernah menyuntik ke task yang sedang jalan. Beda filosofi dari KiroCrew:
- KiroCrew: 1 turn aktif + steer/queue untuk MEMPENGARUHI turn itu atau turn selanjutnya di SESI YANG SAMA.
- Antigravity: tasks independen paralel, tidak ada mekanisme "pengaruhi task yang sedang jalan" — kalau mau ubah arah, task lama dibiarkan/dibunuh, task baru dibuat.

**Simpulan**: pola Antigravity TIDAK menjawab ekspektasi Anda ("2 opsi: olah pesan baru / antrian" DALAM SATU alur kerja yang sama) — itu lebih dekat ke "subagent paralel" yang galleon-fleet SUDAH punya analognya sendiri (`DelegateTool`/sub-agent). Pola **KiroCrew (steer vs queue) adalah jawaban yang tepat** untuk pertanyaan Anda, karena memang didesain untuk kasus "satu alur kerja yang sedang jalan, user mau pengaruhi tanpa membuang progress".

---

## Rekomendasi Konkret (bukan tindakan — ini audit, Anda putuskan)

Tiga gap diurutkan dari yang paling murah/aman diimplementasikan ke yang paling besar:

1. **Steer vs Queue untuk `StartTurn`** (gap #baru, Bagian 2) — perlu: (a) field proto baru di `ExecuteTurnRequest`/RPC terpisah untuk mengirim pesan tambahan ke `turnCtx` yang masih hidup, (b) `runService` menyimpan reference channel/mutex per `SessionID` yang aktif (mirip `activeTurns map[string]context.CancelFunc` yang SUDAH ada — bisa diperluas jadi `activeTurns map[string]*ActiveTurnHandle` berisi cancel + inject-channel), (c) UI toggle mode (steer/queue) seperti `Ctrl+S` KiroCrew. **Estimasi effort: medium** — pattern `activeTurns` sudah ada, ini extend bukan bangun dari nol.
2. **Stall detection terpisah dari error detection** (Gap #1 bagian stream-putus) — `StartTurn` loop `stream.Recv()` butuh timeout/heartbeat eksplisit (context dengan deadline yang di-reset setiap token masuk), supaya hang ≠ dianggap sehat selamanya. **Estimasi effort: kecil** — tambah `context.WithTimeout` yang di-refresh per-token.
3. **Sub-agent persistence record** (Gap #2) — ini yang paling besar: butuh tabel SQLite baru (`subagent_runs` dengan `parent_run_id`, `subagent_id`, `task`, `status`, mirip `RunStore` yang sudah ada) + `ResumeInterruptedRuns` diperluas untuk juga query sub-agent yang statusnya non-terminal. **Estimasi effort: besar** — menyentuh `run/interfaces.go`, SQLite schema baru, dan `crew/services.go::runSubagent`. Ini **irisan langsung** dengan BL-15/16/17 (resource-gate sub-agent, sudah di Sprint 5 `dev-task-2`) — kalau dikerjakan, SATU PR sebaiknya menggabungkan persistence + resource-gate, bukan dua PR terpisah yang berisiko konflik file (`crew/services.go` disentuh keduanya).

**Follow-up verifikasi yang BELUM dilakukan di audit ini** (disebut eksplisit, bukan diklaim selesai):
- Apakah `SopStep`/`SopRun` punya endpoint "resume dari step N" yang benar-benar dipakai (bukan hanya field status pasif).
- Apakah `clawcrew-providers`/`agent/loop_` punya mekanisme compaction otomatis setaraf token-habis seperti KiroCrew.
- Perilaku UI `web/src/` yang SEBENARNYA terjadi saat user kirim chat ke sesi yang masih punya run aktif (run baru paralel vs ditolak vs antre) — ini perlu baca kode frontend, di luar scope `engine/`+`crates/` yang diaudit sesi ini.

Tidak ada backlog BL baru dibuat di dokumen ini — menunggu keputusan Anda apakah 3 gap ini masuk `dev-task-2/` sebagai sprint tambahan atau tetap riset murni dulu.

---

## 3. Follow-up (2026-10-05, lanjutan diskusi) — Graceful Degradation Sebelum Limit & Resume Setelah App Ditutup Total

> Owner setuju adopsi pendekatan KiroCrew (steer/queue dkk di atas), dan menambah 2 pertanyaan baru: (a) metode "graceful shutdown" saat kredit/token mendekati batas (early-warning di sisa 0,1%, bukan nunggu 0%), (b) metode resume setelah proses benar-benar **ditutup** (bukan sekadar crash sementara) — apakah KiroCrew punya pola serupa.

### 3a. Early-warning budget (0,1% tersisa) — Status di galleon-fleet

**Temuan paling penting: infrastrukturnya SUDAH ADA, tapi DIBUANG di titik enforcement — ini bug "configuration-lies" kelas baru, bukan feature-gap murni.**

Bukti konkret, dibaca langsung:

1. `crates/clawcrew-config/src/cost/tracker.rs::check_budget()` (baris 95-152) menghasilkan **3 varian** `BudgetCheck`: `Allowed`, `Warning { current_usd, limit_usd, period }`, `Exceeded { .. }`. `Warning` dipicu di `warn_threshold = config.warn_at_percent / 100.0` — **persis mekanisme early-threshold yang Anda minta**, dan defaultnya `warn_at_percent = 80` (`schema.rs:7559`, `fixtures/v1.toml:622`) — konsisten dengan KiroCrew (notifikasi di 80% usage, bukan 100%). Field ini **sudah configurable** — operator bisa set ke 99.9 kalau mau persis "sisa 0,1%" sesuai permintaan Anda.
2. **TAPI** satu-satunya pemanggil produksi, `enforce_tool_loop_budget()` (`crates/clawcrew-runtime/src/agent/turn/provider_call.rs:116-144`), memakai `if let Some(BudgetCheck::Exceeded { .. }) = check_tool_loop_budget()` — pattern-match ini **HANYA menangkap varian `Exceeded`**. Varian `Warning` yang dikembalikan `check_budget()` **tidak pernah match pola ini, jadi dibuang diam-diam** (`if let` yang tidak match = tidak terjadi apa-apa, bukan error). Tidak ada log, tidak ada event ke UI, tidak ada graceful-degrade apa pun saat budget baru menyentuh 80%/99.9%/berapa pun ambang `warn_at_percent`.
3. Konsekuensi langsung: **hari ini galleon-fleet PERSIS melakukan apa yang Anda tidak inginkan** — berhenti keras begitu `Exceeded` (budget 100% terlampaui, request ditolak via `anyhow::bail!`), **tanpa fase peringatan sebelumnya sama sekali** — walau mesin peringatannya sendiri sudah dibangun dan dikonfigurasi aktif di `fixtures/v1.toml`.

**Apa yang perlu ditambahkan (bukan dibangun dari nol) untuk pola "graceful shutdown ala KiroCrew 80%-warning":**
- Tangani cabang `BudgetCheck::Warning` di `enforce_tool_loop_budget()` (atau titik baru terpisah) — minimal: emit event/notifikasi ke UI (mirip `TurnEvent` di Go, atau hook `postToolUse`), BUKAN `bail!` (request tetap jalan, hanya warning).
- "Graceful" versi lebih dalam dari sekadar notifikasi (ini bagian yang BELUM ada infrastrukturnya sama sekali, perlu dibangun): begitu `Warning` tersentuh, SOP/turn yang sedang jalan **menyelesaikan step/tool-call yang sedang in-flight dulu, baru berhenti menerima tool-call BARU** — berbeda dari `Exceeded` yang langsung `bail!` di tengah apa pun. Ini pola "drain, don't kill" standar graceful-shutdown (SIGTERM vs SIGKILL) — belum ada analognya di `provider_call.rs` saat ini (baik `Warning` maupun `Exceeded` sama-sama tidak membedakan "lagi di tengah tool-call" vs "mau mulai tool-call baru").
- Threshold `warn_at_percent` di config SUDAH bisa diset ke nilai mendekati 100 (misal 99.9) oleh operator kalau mau meniru "sisa 0,1%" — tidak perlu field baru untuk itu, cukup dokumentasi + default yang lebih masuk akal untuk kasus "jangan sampai mendadak mati" (dua ambang terpisah: `warn_at_percent` untuk notifikasi dini seperti sekarang, ditambah ambang baru `graceful_drain_at_percent` lebih tinggi, misal 99, khusus untuk memicu "stop menerima tool-call baru" tanpa membunuh yang sedang jalan).

### 3b. Resume Setelah App Ditutup Total — Perbandingan dengan KiroCrew

Pertanyaan Anda membedakan dua level berbeda yang penting dipisahkan secara eksplisit:

| Level | Definisi | Yang ditanya di Bagian 1 (audit sebelumnya) | Yang ditanya sekarang |
|---|---|---|---|
| **Crash sementara** | Proses mati tak terduga (OOM, kill -9, network drop) TAPI data di disk masih utuh dan konsisten | `ResumableStore`/`ResumeInterruptedRuns` (Go, sudah ada, scope Run-level) | — |
| **Close total/sengaja** | User/OS menutup aplikasi (quit, restart OS, uninstall-reinstall) — proses tidak ada sampai dinyalakan ulang secara eksplisit | — | **Pertanyaan baru ini** |

KiroCrew punya **pola berbeda** untuk level ini, BUKAN perluasan `ResumableStore` — melainkan dua mekanisme yang saling melengkapi:

1. **Checkpoints & Rewind** (`kiro.dev/docs/checkpoints/`) — ini jawaban untuk "resume kerja yang sedang berjalan" dalam SATU proses yang masih hidup, bukan untuk "app sudah ditutup". Checkpoint men-snapshot FILE setiap kali agent memodifikasinya (bukan setiap token/step) — kalau direstore, file DAN conversation context sama-sama di-rewind ke titik itu. **Ini tidak otomatis survive proses mati** — checkpoint tersimpan di disk per-sesi, tapi dokumentasi tidak menyebutnya sebagai mekanisme crash-recovery; ini mekanisme UNDO, bukan RESUME-AFTER-CLOSE.
2. **Sessions (persist per surface)** — "Sessions open as tabs... The open set persists per surface, so closing and reopening the dashboard restores your tabs." Ini levelnya **daftar sesi + riwayat chat**, bukan "lanjutkan turn yang terputus di tengah tool-call". Kalau app ditutup saat turn SEDANG jalan, yang di-restore saat dibuka lagi adalah riwayat percakapan sampai titik terakhir yang tersimpan — **turn yang terputus TIDAK otomatis lanjut jalan sendiri**, user harus kirim prompt baru untuk melanjutkan secara manual (ini konsisten dengan sifat LLM yang stateless — tidak ada "proses LLM" yang bisa di-resume, hanya CONTEXT yang bisa dipakai ulang).
3. **Snapshot & Restore** (`kiro.dev/docs/crew/features/snapshot/`) — ini levelnya PALING DEKAT dengan "resume setelah app ditutup total", tapi scope-nya BUKAN "lanjutkan turn", melainkan **"pulihkan SELURUH STATE aplikasi"**: memory, crons, config, skills, workspace tree — satu file tarball portable. Pola pentingnya:
   - **Replace-mode recovery** — sebelum menimpa state lama, Crew memindahkan state lama ke folder `pre-restore-<timestamp>/` dulu. Kalau restore GAGAL di tengah, Crew **otomatis mencoba kembalikan state sebelumnya** ("Crew attempts to restore the previous state automatically"). Ini pola **transactional restore** (write-ahead-ish) — bukan destructive overwrite langsung.
   - **Auto-detect mode** (Replace vs Merge) dari keberadaan `memory.db` di destination — mencegah "restore tanpa sadar menimpa data yang sudah ada".
   - **Gateway harus di-stop dulu** sebelum restore (`kirocrew restore` menolak jalan kalau Gateway masih listening) — mencegah race-condition tulis-ganda ke file state yang sama.

**Simpulan paling tajam untuk pertanyaan Anda**: KiroCrew **TIDAK punya** mekanisme "turn yang terputus otomatis lanjut sendiri setelah app dibuka lagi" — baik untuk crash sementara MAUPUN close total. Yang ADA dan TERBUKTI adalah **state/context dipertahankan supaya USER bisa melanjutkan dengan mudah** (checkpoint untuk file+context sesi aktif, snapshot untuk seluruh instalasi), bukan auto-resume tanpa interaksi. Ini bukan kekurangan KiroCrew — ini **batasan arsitektur fundamental LLM**: tidak ada "proses inferensi" yang bisa di-pause-dan-resume seperti thread OS: begitu stream provider putus, context yang sudah terkirim ke LLM provider hilang dari sisi provider (provider tidak punya state), satu-satunya yang bisa dipertahankan adalah SALINAN LOKAL dari apa yang sudah terjadi, untuk dipakai ulang sebagai prompt baru.

**Analoginya ke galleon-fleet:**

| Kebutuhan | Pola KiroCrew yang relevan | Status galleon-fleet saat ini |
|---|---|---|
| State SELURUH APLIKASI bisa di-backup+restore (termasuk setelah close total/pindah mesin) | **Snapshot & Restore** (tarball, transactional replace, pre-restore folder) | Grep awal (bagian 1) tidak menemukan ini — **follow-up verifikasi baru**: perlu cek apakah `engine/` atau `crates/` punya CLI `galleon snapshot`/`galleon restore` setara. Kalau tidak ada, ini backlog BARU (bukan perluasan `ResumableStore` yang sudah ada), karena scope-nya beda total (seluruh app state vs satu Run). |
| Turn yang terputus (crash sementara, proses MASIH bisa boot ulang cepat) | `ResumableStore`/`ResumeInterruptedRuns` (Go) — setara, SUDAH ada di galleon-fleet | ADA, tapi scope-nya cuma Run-level, BELUM sub-agent-level (lihat Gap #2, Bagian 1) |
| User tutup app sengaja, buka lagi besok, mau lanjutkan dari TITIK TERAKHIR tanpa ngulang prompt | **Sessions persist** (riwayat chat dipertahankan, user re-trigger manual) — BUKAN auto-resume | Perlu verifikasi terpisah: apakah `web/src/` menyimpan riwayat chat per-sesi yang reload otomatis saat app dibuka lagi (DITANDAI follow-up, di luar `engine/`+`crates/` yang sudah diaudit) |

### Rekomendasi Tambahan (3a + 3b)

4. **Sambungkan `BudgetCheck::Warning` ke jalur nyata** (bukan dibuang) di `enforce_tool_loop_budget()` — PALING MURAH dari semua rekomendasi di dokumen ini, karena tracker-nya SUDAH lengkap, cuma caller yang kurang satu cabang match. **Estimasi effort: kecil.**
5. **Tambah ambang kedua `graceful_drain_at_percent`** (lebih tinggi dari `warn_at_percent`, misal 99) khusus untuk memicu "stop terima tool-call baru, selesaikan yang in-flight" — beda semantik dari `Warning` (sekadar notifikasi) dan `Exceeded` (bail paksa). **Estimasi effort: medium** — butuh state baru "draining" di `TurnCtx`.
6. **Verifikasi apakah galleon-fleet punya setara `snapshot`/`restore` CLI** untuk seluruh state aplikasi (bukan hanya Run) — kalau tidak ada, ini backlog baru terpisah dari Gap #2/#3 di Bagian 1, karena levelnya "seluruh instalasi" bukan "satu Run/sub-agent". **DITANDAI follow-up, belum diverifikasi di audit ini.**
7. **Jangan desain "auto-resume turn tanpa interaksi user setelah app ditutup total"** — berdasarkan riset, ini bukan pola yang terbukti ada bahkan di KiroCrew (platform paling matang yang diriset), karena bertentangan dengan sifat stateless LLM. Yang realistis dan terbukti adalah **pastikan state lokal (progress SOP, hasil tool-call, context percakapan) tersimpan rapi di disk SEBELUM app ditutup**, supaya saat dibuka lagi, USER (atau Lead/Principle secara eksplisit) bisa memilih "lanjutkan dari sini" dengan satu prompt — ini levelnya UX-assisted-resume, bukan silent-auto-resume.

### 3c. Follow-up (2026-10-05, lanjutan lagi) — Snapshot-Tarball vs Replace-In-Place: Mana yang Tidak Menumpuk?

> Owner menanyakan: apakah snapshot (tarball penuh) tidak memakan storage berlebih/menumpuk, atau lebih baik skema **replace pada workspace aktif**?

**Jawaban langsung: kekhawatiran Anda benar secara teknis, dan jawabannya BUKAN salah satu/salah lainnya — dua mekanisme ini menjawab kebutuhan BERBEDA, bukan pengganti satu sama lain.**

**Bukti bahwa snapshot tarball MEMANG berisiko menumpuk (diverifikasi, bukan asumsi):**
- Dokumentasi resmi `kirocrew snapshot` sendiri HARUS menyediakan `--keep N` (prune eksplisit) — kalau tidak berisiko menumpuk, parameter ini tidak perlu ada.
- Setiap `kirocrew snapshot` adalah **FULL DUMP, bukan incremental** — tidak ada bukti deduplikasi/delta antar-snapshot di dokumentasi (`memory.db`, `workspace/`, `plan_memory/` dicopy utuh tiap kali).
- Laporan nyata dari produk sejenis (`kirodotdev/Kiro` IDE, arsitektur mirip): storage menumpuk ke **5.4GB** (breakdown: ~4.8GB file sesi JSON @2-3MB/sesi + ~564MB index vector) dan kasus lain **13GB+** tanpa cleanup — root cause tepat seperti kekhawatiran Anda: "no built-in cleanup mechanism... after several months of use."

**Bukti bahwa KiroCrew SENDIRI tidak memakai snapshot untuk resume rutin** (`kiro.dev/docs/crew/system/` §Related): *"Snapshot and restore: back up state **before a large cleanup**"* — frasa ini eksplisit menempatkan snapshot sebagai operasi **point-in-time terencana** (sebelum migrasi/cleanup besar), BUKAN mekanisme yang jalan otomatis tiap crash/close. Untuk kerja harian, KiroCrew pakai jalur **berbeda**: dua pola yang SUDAH berbasis replace-in-place, bukan dump:
1. **Durable queue** (dibahas Bagian 1) — "writes accepted subagent work into a durable task queue **before** acknowledging it" — ini row/entry yang di-UPDATE status-nya, bukan file baru per-crash.
2. **Storage screen** (`kiro.dev/docs/crew/system/`) — kelola sesi AKTIF satu-per-satu (trash/restore per-sesi), bukan dump-seluruh-state.

**Simpulan untuk galleon-fleet — jawaban "mana yang lebih baik" adalah: pakai KEDUANYA, untuk kasus yang berbeda:**

| Kasus | Mekanisme tepat | Alasan | Status di galleon-fleet |
|---|---|---|---|
| **Resume rutin setelah crash/close** (topik asli pertanyaan Anda — Bagian 1 & 3b) | **Replace-in-place pada live state** — satu row/file selalu di-`UPDATE`, mewakili state TERBARU saja | Tidak ada akumulasi — jumlah data di disk TIDAK bertambah seiring waktu/jumlah crash, karena setiap resume menimpa row yang sama | **SUDAH benar secara desain**: `RunStore.Save`/`UpdateStatus` (`engine/pkg/store/sqlite/run_store.go`) pakai `INSERT ... ON CONFLICT` ke SATU row per `run.ID` — ini **replace-in-place**, bukan snapshot. Gap #2 (sub-agent persistence, Bagian 1) HARUS mengikuti pola yang SAMA — tabel `subagent_runs` dengan baris di-`UPDATE`, **BUKAN** tabel riwayat/snapshot yang terus bertambah. |
| **Backup sebelum operasi berisiko** (migrasi besar, pindah mesin, Sprint 8 rebrand massal) | **Snapshot/tarball point-in-time**, WAJIB dengan retensi `--keep N` | Ini genuinely butuh salinan BEKU di titik waktu tertentu (kalau migrasi gagal, perlu titik rollback) — tapi harus dibatasi jumlahnya, bukan dibiarkan tak terbatas | Belum diverifikasi apakah ada setara di galleon-fleet (follow-up #6, masih terbuka) — kalau dibangun, WAJIB sertakan retention/prune dari awal desain (jangan ulangi pola `kirodotdev/Kiro` IDE yang baru dapat keluhan storage setelah "several months of use" tanpa cleanup bawaan). |

**Rekomendasi tambahan (menggantikan sebagian rekomendasi #6 sebelumnya):**

6. (Revisi) **Kalau snapshot/tarball dibangun untuk galleon-fleet** (kebutuhan "pindah mesin"/backup pra-migrasi, BUKAN resume rutin): wajib desain retensi (`--keep N` ATAU TTL berbasis waktu) SEJAK AWAL — jangan tunda sampai jadi masalah storage seperti kasus `kirodotdev/Kiro` IDE. Pertimbangkan kompresi (gzip tarball, bukan raw tar) karena `workspace/`+`memory.db` yang di-dump berulang sangat compressible (teks/JSON/SQLite punya redundansi tinggi).
8. **Sub-agent persistence (Gap #2, Bagian 1) ditegaskan ulang HARUS replace-in-place**, bukan event-log yang terus bertambah — desain tabel `subagent_runs` dengan `UPDATE ... WHERE subagent_id = ?` per transisi status, meniru `RunStore` yang sudah ada, BUKAN `INSERT` baris baru per event (itu levelnya `RunEvent`/`AppendEvent` yang terpisah dan MEMANG dirancang bertambah — perlu retensi/pruning SENDIRI, lihat `run_store.go` `AppendEvent`/`ReplayEvents`, DITANDAI follow-up: perlu verifikasi apakah tabel event ini punya retention policy atau berpotensi menumpuk tanpa batas juga).

---

## 4. Follow-up (2026-10-05, verifikasi mendalam) — Retention `AppendEvent`/`ReplayEvents`: KONFIRMASI, Ini Memang Sumber Penumpukan Nyata

> Owner meminta analisa konkret: apakah tabel event history (`AppendEvent`/`ReplayEvents`) punya retention policy, mengingat sifatnya append-only yang DIRANCANG bertambah — berbeda dari status row (`runs` table) yang replace-in-place.

### Jawaban: TIDAK ADA retention sama sekali — terverifikasi di DUA implementasi backend, bukan satu

Galleon-fleet punya **dua implementasi berbeda** untuk `run.Store`/event-log, dan **keduanya nol retensi**:

**(a) `engine/pkg/store/sqlite/run_store.go` (SQLite backend):**
```sql
CREATE TABLE IF NOT EXISTS run_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    error_str TEXT DEFAULT '',
    timestamp DATETIME NOT NULL,
    FOREIGN KEY(run_id) REFERENCES runs(id) ON DELETE CASCADE
);
```
(`engine/pkg/store/sqlite/schema.go`) — `AppendEvent` murni `INSERT` (bukan `ON CONFLICT`/upsert), `ReplayEvents` murni `SELECT`. Grep seluruh `engine/` untuk `DELETE FROM run_events`/`VACUUM`/`prune`/`retention`: **nol hasil**. Ada `ON DELETE CASCADE` di foreign key — tapi ini TIDAK PERNAH TERPAKAI karena tidak ada satu baris kode pun yang melakukan `DELETE FROM runs` (verifikasi: grep `DELETE FROM runs` juga nol hasil). Jadi cascade itu infrastruktur mati, bukan mekanisme cleanup aktif.

**(b) `engine/src/persistence/disk_store.go` (DiskStore/JSONL backend — INI YANG AKTIF DI PRODUKSI):**
```go
func (s *DiskStore) AppendEvent(ctx context.Context, event *run.RunEvent) error {
    ...
    f, err := os.OpenFile(s.eventsFile(event.RunID), os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
    ...
    f.Write(append(data, '\n'))
}
```
Satu file `events.jsonl` PER RUN (`runs/<runID>/events.jsonl`), di-append terus tanpa batas ukuran, tanpa rotasi, tanpa TTL. **Verifikasi wiring produksi**: `engine/app/wire_gen.go` memanggil `NewDiskStore` — ini backend yang BENAR-BENAR DIPAKAI saat engine jalan, bukan `sqlite.RunStore` (yang grep menunjukkan hanya dipanggil dari file `_test.go`). Artinya **analisis dampak storage HARUS fokus ke `DiskStore`/JSONL, bukan SQLite** — ini temuan baru yang mengoreksi asumsi implisit audit Bagian 1 (yang membahas `run_store.go` SQLite seolah itu jalur produksi utama).

### Dampak Konkret — Kenapa Ini Benar-Benar Sumber Penumpukan

Dari `crew/services.go::StartTurn` (dibaca di Bagian 1), `TurnEvent` diterbitkan **per-token/per-delta streaming**:
```go
case *pb.TurnToken_Token:
    ...
    eventCh <- &TurnEvent{Type: EventTextChunk, Content: ev.Token.Delta}
```
Satu turn LLM biasa menghasilkan RATUSAN hingga RIBUAN token/delta. **TAPI** — poin penting yang perlu diluruskan: `eventCh` (yang menerima tiap token) adalah jalur **in-memory** ke SSE/UI (`MemoryEventHub.Publish`), BUKAN otomatis jalur yang sama dengan `DiskStore.AppendEvent`. Dari grep sebelumnya (Bagian 1 audit), `AppendEvent` di jalur produksi (`run/services.go::PublishEvent`) **TIDAK dipanggil sama sekali** — hanya `MemoryEventHub.Publish` yang jalan. Ini berarti:

- **Temuan POSITIF (mengurangi severity dari ekspektasi awal)**: `run_events`/`events.jsonl` secara LITERAL **belum pernah ditulis di jalur produksi hari ini** — `run/services.go::PublishEvent` cuma publish ke in-memory hub, tidak pernah memanggil `s.store.AppendEvent`/`DiskStore.AppendEvent`. Jadi secara FAKTA SAAT INI, tidak ada file `events.jsonl` yang bertambah dari trafik normal — fitur replay/append ini ADA (kode lengkap, teruji di `_test.go`), tapi **tidak terhubung ke jalur event emission yang nyata jalan** (`StartTurn`/`PublishEvent`).
- **Ini justru gap BARU yang lebih mendasar dari "retention policy"**: pertanyaan "apakah event log punya retention" jadi agak hipotetis untuk HARI INI, karena event log itu sendiri **tidak terisi** dari jalur produksi. Begitu `AppendEvent` DISAMBUNGKAN ke `PublishEvent` (yang logically seharusnya terjadi — fitur `ReplayEvents`/"replay for SSE reconnect" tidak ada gunanya kalau tidak pernah diisi), SAAT ITU risiko penumpukan yang Anda khawatirkan jadi nyata — dan karena granularitasnya PER-TOKEN (bukan per-turn), volumenya jauh lebih besar daripada `runs` table (1 row per Run vs potensi ribuan baris/baris JSONL per Run).
- Estimasi kasar skala masalah KALAU disambungkan tanpa retensi: 1 turn ~500-2000 token delta × ukuran JSON overhead per event (~150-300 byte termasuk `id`,`run_id`,`event_type`,`timestamp` field) = **75KB-600KB per turn** hanya untuk event log (di luar `run.json` yang jauh lebih kecil, cuma status). Kalau SOP dengan ratusan turn/hari, ini bisa jadi puluhan-ratusan MB/hari per instalasi aktif — pola pertumbuhan PERSIS seperti laporan nyata `kirodotdev/Kiro` IDE (4.8GB dari file sesi JSON) yang dikutip di Bagian 3c.

### Kesimpulan — Dua Temuan Terpisah, Jangan Digabung

1. **Temuan A (lebih mendasar)**: `AppendEvent` tidak tersambung ke jalur produksi (`PublishEvent`) — fitur replay-event untuk SSE reconnect/resume granular SECARA FUNGSIONAL BELUM AKTIF, terlepas dari soal retention. Ini perlu diputuskan dulu: apakah mau disambungkan (untuk kebutuhan replay SSE setelah reconnect browser, BEDA dari `ResumeInterruptedRuns` yang levelnya Run-status) — KALAU iya, Temuan B di bawah jadi WAJIB dikerjakan BERSAMAAN, bukan menyusul.
2. **Temuan B (yang ditanyakan Owner)**: KALAU/KETIKA Temuan A disambungkan, `run_events`/`events.jsonl` **nol retention** — tidak ada TTL, tidak ada cap ukuran file, tidak ada prune-by-age, tidak ada kompaksi (misal: setelah run `Completed`, event per-token tidak perlu dipertahankan granular, cukup ringkasan). Pola `ON DELETE CASCADE` yang ada di schema SQLite adalah sinyal bahwa DESAINER AWAL sadar `run_events` terikat ke lifecycle `runs` — tapi implementasinya belum pernah memicu delete sama sekali (karena `runs` row juga tidak pernah di-DELETE, hanya di-UPDATE status).

### Rekomendasi Tambahan (menggantikan sebagian poin #8 sebelumnya)

9. **KEPUTUSAN (bukan opsi terbuka): JANGAN sambungkan `AppendEvent` untuk replay granular per-token.** `ResumeInterruptedRuns` (Run-status level, SUDAH ada dan SUDAH teruji) menjawab kebutuhan fungsional yang SEBENARNYA diminta saat reconnect/resume — "apa status terakhir, apa hasil akhirnya" — BUKAN "tampilkan ulang tiap token seolah baru diketik". Replay token-demi-token hanya bernilai estetika UI (efek "mengetik ulang"), yang justru kontraproduktif setelah reconnect (user ingin lihat hasil JADI, bukan animasi ulang). Biaya (75KB-600KB/turn, berpotensi puluhan-ratusan MB/hari, plus kompleksitas desain retensi yang harus benar sejak awal) tidak sepadan dengan manfaat murni-estetika ini — pola "lazy senior dev": kalau kebutuhan nyata bisa dipenuhi tanpa fitur, jangan bangun fiturnya.
   - **Yang cukup untuk kebutuhan resume**: tambahkan field ringkasan hasil akhir (`FinalOutput string`/`Summary string`) ke `run.Run` (disimpan di `run.json` yang SUDAH ada, replace-in-place, TIDAK menambah entitas baru) — ini memberi "apa hasilnya" tanpa riwayat token granular.
   - **`AppendEvent`/`ReplayEvents`/`events.jsonl` yang SUDAH ada di kode: biarkan TIDAK AKTIF (status quo) atau hapus sebagai dead-code** (konsisten dengan `AGENTS.md` §Safety: "Do not suppress unused production code... remove it, connect it, or track it") — JANGAN disambungkan ke `PublishEvent` tanpa kebutuhan konkret yang menjustifikasi biaya storage-nya. Keputusan "remove vs track as backlog-tertunda" diserahkan ke Anda; saya REKOMENDASIKAN "track sebagai backlog DITUNDA" (bukan hapus langsung) karena ini infrastruktur yang sudah ditulis+teruji, mungkin berguna untuk kebutuhan BERBEDA di masa depan (lihat poin 10).
10. **Kalau SUATU SAAT ada kebutuhan BERBEDA yang genuinely butuh event-log** (bukan "replay visual UI", tapi contoh konkret: audit-trail tool-call untuk compliance/forensik — "tool apa yang dipanggil, kapan, oleh siapa"), maka jawabannya BUKAN simpan semua token, tapi simpan **hanya event level tool-call** (`EventToolCallStarted`/`EventToolCallFinished`/`EventError`/`EventSubagentSpawned`) — ini SUDAH jauh lebih jarang secara alami (puluhan per turn, bukan ribuan seperti `EventTextChunk`/`EventThoughtChunk`), dan punya nilai audit nyata yang berbeda dari sekadar scroll-back UI. `EventTextChunk`/`EventThoughtChunk` (streaming delta) harus TETAP TIDAK PERNAH dipersist dalam skenario apa pun — itu murni transport-layer untuk UI real-time, disposable begitu ditampilkan, baik untuk kebutuhan resume MAUPUN audit.
11. **Scope sub-agent persistence (Gap #2, Bagian 1) TETAP berdiri sendiri, TIDAK perlu digabung dengan event-log** — koreksi dari rekomendasi sebelumnya: karena keputusan #9 di atas berarti event-log TIDAK dibangun sekarang, maka tidak ada lagi alasan "satu PR karena filosofi desain saling terkait". Gap #2 (tabel `subagent_runs` dengan `UPDATE` per status, meniru `RunStore`) berdiri sendiri sebagai pekerjaan yang jelas scope-nya: replace-in-place murni, TANPA komponen event-log sama sekali.

**Ringkasan keputusan untuk Anda bawa ke Lead/Backend**: TIDAK ada backlog baru dari topik event-log/replay di sesi riset ini — justru sebaliknya, REKOMENDASI adalah TIDAK membangun apa pun di arah ini, dan opsional membersihkan `AppendEvent`/`ReplayEvents`/`events.jsonl` yang sudah ditulis tapi tidak terhubung (dead-code candidate, bukan technical-debt yang perlu diselesaikan — hanya perlu keputusan: hapus sekarang, atau catat sebagai "tertunda, backend sudah siap kalau kebutuhan audit-trail tool-call muncul nanti").




