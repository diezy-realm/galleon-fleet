---
title: Backlog — Sub-Agent Resource Management & Observability Gaps
status: backlog
date: 2026-10-05
author: squad-lead
depends_on: subagent-resource-management-audit.md
---

# Backlog — Sub-Agent Resource Management & Observability Gaps

> Diturunkan dari `research/subagent-resource-management-audit.md` (§5, §6.4, §8.2). Konteks: Owner melaporkan
> aplikasi crash karena Agent spawn ~20 sub-agent sekaligus tanpa cek memori, dan user tidak bisa tahu/kill
> sub-agent yang stuck. Audit kode membuktikan sebagian "sudah ada tapi tidak nyambung ke user" (loop detector,
> timeout), sebagian lagi benar-benar tidak ada (resource gate sebelum spawn). Nomor lanjutan dari BL-14
> (`automation-cron-webhook-backlog.md`).

---

## BL-15 — Resource Gate Sebelum Spawn Sub-Agent (PRIORITAS TERTINGGI)

**Rujukan riset:** `subagent-resource-management-audit.md` §1, §5 poin 1-4

**Masalah (terverifikasi):** `DelegateTool` tidak punya referensi apa pun ke `process_stats`/`sysinfo`. Tidak ada field cap jumlah sub-agent PARALEL di config (`max_delegation_depth` hanya kedalaman rantai, bukan lebar). Tidak ada queue/admission-control. Sub-agent adalah tokio task in-process (bukan subprocess OS) — satu sub-agent leak/infinite-loop bisa menghabiskan RAM proses induk sampai OS mem-kill seluruh aplikasi. Ini PERSIS skenario crash yang Owner laporkan, bukan asumsi.

**Pola rujukan yang SUDAH terbukti bekerja:** Kiro Crew — `resource_limits.max_total_memory_mb`/`max_total_processes` di config, aggregate ceiling (bukan per-sub-agent) yang ADAPTIF (turun otomatis saat headroom sempit), tool `resource_status` untuk agent cek sendiri sebelum fan-out besar, durable queue yang survive restart.

**Scope:**
1. Tambah `resource_limits.max_total_memory_mb` dan `max_total_processes` di `clawcrew-config` (schema baru, serde default supaya tidak merusak config existing — menyusul pola `#[serde(default, ...)]` yang sudah konsisten dipakai di `types.rs`/`schema.rs`).
2. Gate di `DelegateTool` SEBELUM `clawcrew_spawn::spawn!` dipanggil untuk delegate baru: cek `process_stats::sample().rss_bytes` terhadap `max_total_memory_mb` DAN hitung task `Running` berkind `Delegate` di `control_plane::TaskRegistry` terhadap `max_total_processes`. Tolak/queue, jangan silent-allow.
3. Reuse `control_plane` task registry untuk queue (cek dulu apakah status `Pending`/`Queued` sudah ada sebagai enum `TaskStatus` sebelum menambah varian baru).

**File ownership:** Backend-Rust — `crates/clawcrew-config/src/schema.rs` (field baru), `crates/clawcrew-runtime/src/tools/delegate/mod.rs` (gate logic), `crates/clawcrew-runtime/src/control_plane/` (queue, kalau perlu varian status baru).

**Definition of Done:** Test baru yang membuktikan: (a) spawn ditolak/di-queue saat `max_total_memory_mb`/`max_total_processes` terlampaui, bukan silent-allow; (b) task yang di-queue tetap tereksekusi begitu kapasitas tersedia lagi; (c) aggregate ceiling, bukan per-sub-agent — satu sub-agent besar tidak lolos cek individual tapi tetap kena cap gabungan.

**Dependency:** Tidak ada — bisa langsung dikerjakan, infrastruktur dasarnya (`process_stats::sample()`, `control_plane::TaskRegistry`) sudah ada.

**Risiko/Severity:** **HIGH** — ini akar masalah crash nyata yang dilaporkan Owner, bukan technical debt kosmetik.

---

## BL-16 — Surface `loop_detector_circuit_breaker` ke Dashboard/Tasks-View

**Rujukan riset:** `subagent-resource-management-audit.md` §6.1, §8.2

**Masalah (terverifikasi):** `LoopDetector` (`agent/loop_detector.rs`) sudah matang — mendeteksi 3 pola stuck (exact-repeat, ping-pong, no-progress) dan men-terminate turn via `anyhow::bail!` saat `Break`. TAPI event ini hanya tercatat sebagai 1 baris log (`loop_detector_circuit_breaker`) ke file lokal `runtime-trace.jsonl` — **tidak ada consumer**. Grep `loop_detector|circuit_breaker` di SELURUH `engine/` (Go, dashboard utama) = 0 hasil. User tidak punya cara tahu sub-agent mana yang kena circuit-breaker tanpa membaca file log mentah secara manual.

**Dampak:** Ini PERSIS yang menjelaskan keluhan Owner "user tidak tahu mana sub agent yang terkendala" — mesin deteksinya sudah bekerja, tapi hasilnya tidak pernah sampai ke mata user.

**Scope:** Reuse `control_plane::TaskRegistry` yang SUDAH ADA (SSOT, jangan bikin sistem terpisah):
1. Tambah field `last_loop_detector_event: Option<String>` (atau struct kecil berisi pattern+pesan+timestamp) pada `TaskRecord`, dengan `#[serde(default)]` supaya backward-compatible.
2. Di titik `Break` (`agent/turn/results_collect.rs` sekitar baris 118), SEBELUM `anyhow::bail!`, tulis event ini ke `TaskRecord` milik delegate task yang bersangkutan (butuh thread task_id ke scope ini — cek apakah `turn_id`/`trace_id` sudah bisa dipetakan ke `control_plane` task_id, atau perlu parameter baru).
3. Expose field ini di endpoint/RPC yang sudah dipakai Tasks-view (govern oleh Go — cek `engine/` endpoint mana yang serve data task ke dashboard, reuse, jangan bikin endpoint baru).

**File ownership:** Backend-Rust (field + write point) — `crates/clawcrew-runtime/src/control_plane/task_registry.rs`, `crates/clawcrew-runtime/src/agent/turn/results_collect.rs`. Backend-Go (expose ke API dashboard) — endpoint Tasks-view existing, TBD lokasi pasti (perlu grep lanjutan).

**Definition of Done:** Sub-agent yang terkena `Break` dari loop detector muncul di Tasks-view dengan alasan spesifik (misal "circuit breaker: tool 'shell' called 5 times with identical arguments"), bukan cuma status "Failed" generik.

**Dependency:** Tidak ada blocker teknis. Perlu riset lanjutan kecil: lokasi endpoint Go yang serve Tasks-view saat ini (belum diverifikasi di sesi riset).

**Risiko/Severity:** **MEDIUM** — bukan celah keamanan, tapi langsung menjawab keluhan UX/observability yang Owner laporkan sendiri. Biaya implementasi rendah (reuse infrastruktur ada), manfaat langsung terasa.

---

## BL-17 — Agregasi Lintas-Sub-Agent untuk Fan-Out Besar (Fleet-Level Insight)

**Rujukan riset:** `subagent-resource-management-audit.md` §6.3, §6.4 poin 7

**Masalah:** `LoopDetector` bekerja PER-TURN (per sub-agent individual). Tidak ada korelasi "5 dari 20 sub-agent paralel semuanya menunjukkan pola stuck yang sama" sebagai satu insight ke user. Saat fan-out besar (kasus Owner: ~20 sub-agent sekaligus), user harus memeriksa satu-satu alih-alih mendapat sinyal agregat "banyak sub-agent di batch ini stuck, kemungkinan root cause sama."

**Scope:** Baru dirancang SETELAH BL-16 selesai (butuh data per-sub-agent tersurface dulu). Mekanisme: agregator level `control_plane` yang membaca `last_loop_detector_event` dari semua task dalam satu "batch"/parent delegation, deteksi kalau >N% sub-agent dalam batch yang sama kena pola serupa, surface sebagai satu notifikasi/summary bukan N notifikasi terpisah.

**File ownership:** TBD — kemungkinan Backend-Rust (`control_plane`) untuk agregasi + Backend-Go/Frontend untuk presentasi.

**Definition of Done:** Belum ditentukan — ini item EXPLORATORY, perlu design-spec terpisah sebelum breakdown task konkret.

**Dependency:** BL-16 (butuh data individual per-sub-agent tersurface dulu, baru bisa diagregasi).

**Risiko/Severity:** LOW — genuinely baru, belum ada precedent di Kiro Crew/Antigravity. Nilai tambah nyata tapi tidak urgent dibanding BL-15/BL-16. Jangan dikerjakan sebelum BL-15/BL-16 selesai.

---

## BL-18 — Jalur Masuk Resource KETIGA: `spawn_subagent` (Go) → `ExecuteTurn` (Rust) Melewati Gate `DelegateTool` (CRITICAL, BARU)

**Rujukan riset:** Verifikasi cross-check BL-05 vs BL-15/R-50, 2026-10-05. Lihat juga koreksi di `backlog/skills-steering-lessons-backlog.md` BL-05.

**Masalah (terverifikasi langsung ke kode):** Audit sebelumnya (`subagent-resource-management-audit.md`) HANYA mengaudit jalur Rust `DelegateTool` (`tools/delegate/mod.rs`, dipicu dari dalam satu turn Rust yang sudah berjalan). Verifikasi lanjutan menemukan jalur KETIGA yang terlewat:

`spawn_subagent` (Go, `dispatcher.go:216`) → goroutine `runSubagent` (`services.go:235`) → **`s.gateway.ExecuteTurn(ctx, &pb.ExecuteTurnRequest{SessionId: "subagent-<id>", ...})`** — gRPC call LANGSUNG ke Rust `SystemGateway`, membuka SESI TURN BARU di Rust, independen dari `DelegateTool`.

**Dampak:** BL-15/R-50 (resource gate yang dirancang di `tools/delegate/mod.rs`, sebelum `clawcrew_spawn::spawn!`) **TIDAK melindungi jalur ini sama sekali** — `ExecuteTurn` kemungkinan punya jalur masuk resource yang berbeda total di sisi Rust (perlu diverifikasi: apakah `ExecuteTurn` RPC handler punya resource accounting sendiri, atau sama sekali tanpa gate seperti `DelegateTool` sebelum BL-15 diimplementasi). Ini juga berarti: skenario crash yang Owner laporkan (~20 sub-agent sekaligus) BISA SAJA berasal dari jalur `spawn_subagent` Go, bukan dari `DelegateTool` Rust — audit asal mula KEDUA jalur perlu dilakukan sebelum menganggap BL-15 saja cukup menutup akar masalah.

**Scope investigasi (BUKAN langsung implementasi):**
1. ~~Verifikasi: apakah `ExecuteTurn` RPC handler Rust punya resource accounting apa pun~~ **SELESAI DIVERIFIKASI (2026-10-05): TIDAK ADA SAMA SEKALI.** `SystemGatewayService::execute_turn` (`crates/clawcrew-gateway/src/grpc_system_gateway.rs:412`) — implementasi gRPC produksi nyata (bukan stub/test; punya `vault_dir`/`secret_store` sungguhan, listen `:50052`) — body-nya `clawcrew_spawn::spawn!(async move {...})` LANGSUNG, nol semaphore, nol cek `process_stats`, nol cek jumlah sesi turn aktif. Dibandingkan `rpc/wss.rs` yang PUNYA `session_permits`/`handshake_permits` (`tokio::sync::Semaphore`) — tapi itu untuk jalur WSS client (UI/CLI), BUKAN untuk `SystemGateway.ExecuteTurn` yang dipanggil Go. **Ini gap yang LEBIH PARAH dari `DelegateTool`** (yang setidaknya punya `timeout_secs`/`agentic_timeout_secs`) — `ExecuteTurn` tidak punya pelindung apa pun.
2. Verifikasi: apakah `spawn_subagent` (Go) punya pembatas jumlah paralel di SISI GO (sebelum RPC dikirim ke Rust) — **masih perlu dicek**, belum diverifikasi di sesi ini.
3. Setelah investigasi: desain gate yang KONSISTEN untuk SEMUA jalur masuk sub-agent (DelegateTool Rust, ExecuteTurn dari spawn_subagent Go) — kemungkinan solusinya ada di Rust-side (gate di level `SystemGateway`/`ExecuteTurn` handler itu sendiri, SSOT untuk semua sesi turn baru, bukan duplikasi gate di tiap titik masuk).

**Konteks penting dari `finalize/dev-tasks/01-be-go-engine.md` (dibaca read-only, TIDAK diedit — folder itu di luar scope sesi ini):** Ada **BG-D1** (`G-30`, P1, L — "Agent loop/turn: tool-call loop, kompaksi, batas iterasi DI ATAS `ExecuteTurn` Rust") dan **BG-D4** (`G-33`, P2, M — "Sub-agent/handoff/A2A sebagai Artifact reference", depends on BG-D1), keduanya status `[ ]` BELUM dikerjakan. Ini bagian strategi "strangler dari `clawcrew-runtime`" — rencana jangka-menengah memindahkan agent-loop/orkestrasi sub-agent dari Rust ke Go, membuat `ExecuteTurn` murni jadi primitive eksekusi LLM sementara Go pegang logic loop/iterasi/sub-agent. **Implikasi:** BL-18 adalah gap DARURAT untuk KEADAAN SEKARANG (sebelum BG-D1/BG-D4 dikerjakan) — gate yang dibangun sekarang perlu didesain supaya tidak harus dibongkar total saat BG-D1/BG-D4 nanti berjalan (kemungkinan: taruh gate di level RPC/transport Rust yang akan tetap jadi titik masuk APAPUN siapa pemanggil-nya di masa depan, bukan di logic yang akan dipindah ke Go).

**File ownership:** Backend-Rust (gate di `grpc_system_gateway.rs::execute_turn` — titik paling tepat karena akan tetap jadi RPC boundary apa pun hasil BG-D1/BG-D4) + Backend-Go (verifikasi sisa poin 2 di atas, koordinasi timing dengan rencana BG-D1/BG-D4 di `dev-tasks/` supaya tidak dibangun 2x).

**Definition of Done:** (a) Dokumen temuan poin 2 (pembatas sisi Go); (b) gate resource terpasang di `execute_turn` handler Rust (reuse pola `resource_limits.max_total_memory_mb`/`max_total_processes` yang sama dirancang di BL-15, SATU config SSOT dipakai kedua titik masuk — `DelegateTool` DAN `execute_turn` — bukan dua config terpisah); (c) catatan ke Backend-Go/Principle bahwa gate ini DISENGAJA ditaruh di titik yang akan tetap jadi RPC boundary meski BG-D1/BG-D4 nanti memindah logic loop ke Go.

**Dependency:** Tidak ada untuk investigasi (sudah selesai sebagian). Implementasi gate SEBAIKNYA dikoordinasikan dengan tim yang memegang `finalize/dev-tasks/` (BG-D1/BG-D4) supaya tidak bentrok/duplikasi kerja — **tanyakan ke Achmad timing yang tepat** sebelum mulai coding.

> **KEPUTUSAN ACHMAD (2026-10-05): TUNGGU BG-D1/BG-D4 dulu.** Tim dev sedang mengerjakan BG-D1/BG-D4 (strangler `clawcrew-runtime` → Go) lebih dulu — BL-18 BELUM diimplementasi sekarang. Status item ini: **riset + backlog matang, bukan tiket eksekusi aktif.** Tujuannya supaya saat tim dev-tasks sampai ke bagian `ExecuteTurn`/sub-agent (BG-D1/BG-D4), temuan dan rekomendasi gate di dokumen ini sudah siap jadi rujukan — bukan ditemukan dari nol lagi. **Jangan assign ke squad manapun sebelum BG-D1/BG-D4 bergerak** — re-cek status dev-tasks secara berkala (tanpa mengedit folder itu) untuk tahu kapan waktu yang tepat mengangkat BL-18 dari backlog ke eksekusi nyata.

**Risiko/Severity:** **CRITICAL** secara teknis (nol pelindung resource di `ExecuteTurn`), **TAPI eksekusi DITUNDA secara sengaja** oleh keputusan Achmad di atas — bukan diabaikan, murni soal urutan kerja supaya tidak tumpang-tindih dengan BG-D1/BG-D4 yang sedang berjalan.

---

## Urutan Eksekusi yang Disarankan

1. **BL-15 dulu** — akar masalah crash nyata (resource gate di `DelegateTool`). Tanpa ini, BL-16/BL-17 cuma memperbaiki visibility atas masalah yang masih bisa terjadi.
2. **BL-16** — biaya rendah, manfaat langsung, reuse infrastruktur yang sudah ada (`TaskRegistry`). Bisa paralel dengan BL-15 kalau file ownership tidak tumpang tindih (BL-15 di `tools/delegate/mod.rs` + config; BL-16 di `control_plane/task_registry.rs` + `agent/turn/results_collect.rs` — overlap kecil, serialkan kalau 1 penulis).
3. **BL-18 — DITUNDA (keputusan Achmad 2026-10-05).** Status: riset+backlog matang, BUKAN tiket aktif. Tunggu tim dev-tasks menyelesaikan BG-D1/BG-D4 (strangler `clawcrew-runtime` → Go) lebih dulu. Re-cek status BG-D1/BG-D4 secara berkala; angkat BL-18 ke eksekusi begitu BG-D1/BG-D4 bergerak atau selesai, supaya gate didesain pas dengan bentuk final RPC boundary pasca-migrasi, bukan dibangun 2x.
4. **BL-17 terakhir** — exploratory, depends on BL-16, perlu design-spec dulu sebelum masuk sprint.

## Riwayat Keputusan

- 2026-10-05: Achmad memutuskan BL-18 ditunda sampai BG-D1/BG-D4 (`finalize/dev-tasks/01-be-go-engine.md`) bergerak — item ini tetap didokumentasikan matang di `research`/`backlog` sebagai rujukan siap-pakai untuk tim dev-tasks nanti, bukan dieksekusi sekarang.

## Catatan Proses

Sama seperti backlog lain: setiap item perlu diangkat ke `roadmap/*.md` sebelum di-assign ke squad. Lihat `roadmap/R-rust.md` R-5x (baru) untuk pengangkatan resmi BL-15/BL-16/BL-17. **BL-18/R-53 SENGAJA TIDAK diangkat sebagai tiket aktif** — tetap tercatat di roadmap dengan catatan tunda, supaya terlihat sebagai rencana, bukan hilang dari radar.
