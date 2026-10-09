---
title: Standar Kerja dev-task-2 — Verifikasi, Dokumentasi, dan CI Gate
status: active
date: 2026-10-09
author: squad-lead
scope: Aturan wajib untuk SIAPA PUN (agent atau manusia) yang mengerjakan task di dev-task-2/ (sprint-*.md, finding/, ci-infra-fix/). Lahir dari pola kegagalan nyata yang berulang di sesi 2026-10-08/09 — setiap aturan di bawah punya insiden konkret sebagai alasan, bukan preferensi gaya.
---

# Standar Kerja dev-task-2

Dokumen ini mengikat SEMUA kerja di dalam folder `dev-task-2/` (sprint, finding, ci-infra-fix) — agent maupun manusia. Setiap aturan ditulis karena ADA insiden nyata yang terjadi tanpa aturan ini. Baca sebelum mulai task apa pun di folder ini.

## 1. Checkbox `[x]` HANYA setelah 3 syarat terpenuhi — bukan niat, bukan commit message

**Syarat:** (a) kode genuinely ada di disk (`grep`/baca file langsung), (b) `cargo check`/`go build`/`npm run build` relevan hijau, (c) PR nyata dibuka dan minimal 1 CI check selesai (bukan "belum jalan").

**Insiden yang melahirkan aturan ini:** commit `0a8cac52` mengklaim 4 perubahan (proto message, proto field, Go struct, DTO field) di message-nya — `git show --stat` membuktikan hanya 1 dari 4 genuinely masuk. Subagent `30366934` (T-01) melaporkan "0 match, sudah ter-commit" padahal verifikasi independen membuktikan 16+1 match `verifiable_intent` masih ada dan HEAD tidak berubah sama sekali.

**Konsekuensi kalau dilanggar:** tim berikutnya membangun fitur lanjutan di atas fondasi yang dikira ada tapi kosong (lihat insiden BL-16/17 — FE sempat hampir dimulai sebelum ketahuan backend-nya dead-code).

## 2. Jangan percaya self-report subagent/CI/commit-message — verifikasi langsung ke sumber primer

**Urutan verifikasi wajib:** `git log`/`git show --stat` (bukti commit) → `grep`/baca file (bukti kode) → `cargo check`/`go build` (bukti compile) → hasil run CI nyata (bukti end-to-end). Jangan berhenti di langkah pertama yang "terlihat meyakinkan".

**Insiden:** 3 kali dalam sesi yang sama — (1) subagent T-01 self-report keliru, (2) commit T-07 overclaim 3 dari 4 perubahan, (3) audit T-06 (MCP) sendiri yang KELIRU karena hanya mengandalkan satu `grep` sempit tanpa `glob` nama file eksplisit — ternyata MCP client host genuinely ada 10 file di `clawcrew-tools/`.

**Pelajaran tambahan dari insiden T-06:** untuk klaim "X tidak ada", verifikasi dengan DUA metode independen (grep DAN glob nama file, atau grep DAN baca direktori), bukan satu metode saja — terutama kalau hasilnya 0 match yang mengejutkan/berlawanan dengan ekspektasi.

## 3. Dokumen status (`finding/`, progress-handoff) WAJIB diberi tanggal-cek dan ditandai usang begitu kondisi berubah

**Aturan:** begitu commit baru masuk yang mengubah status sebuah task yang didokumentasikan "belum selesai"/"masih bug", tambahkan section `⚠️ UPDATE <tanggal>` di PALING ATAS file (sebelum isi lama), BUKAN edit diam-diam isi lama atau biarkan isi lama apa adanya tanpa penanda. Jangan hapus isi lama kecuali genuinely tidak ada nilai historis sama sekali.

**Insiden:** `2026-10-09-remediation-progress-handoff.md` dan `2026-10-08-remediation-tasks.md` sempat berisi klaim "T-01/02/03/04/05/07 belum selesai" selama berjam-jam setelah 5 commit baru (`4fd64305`..`a16f3531`) genuinely menyelesaikan keenamnya — kalau tim lain membaca dokumen itu tanpa cross-check manual ke `git log`, mereka akan mengerjakan ulang pekerjaan yang sudah tuntas.

## 4. Keputusan arsitektur yang sudah direkomendasikan dan disetujui WAJIB dipromosikan ke ODR — jangan biarkan hidup hanya di `finding/`

**Aturan:** begitu Owner/Lead menyetujui sebuah rekomendasi (eksplisit atau dengan menindaklanjutinya sebagai keputusan final), tulis `standard-docs/decisions/owner-decisions/ODR-<tanggal>-<slug>.md` SEGERA di sesi yang sama — jangan tunda ke sesi berikutnya, dan jangan biarkan rekomendasi hanya tertulis di dokumen `finding/`/analysis.

**Insiden:** T-02/T-04/T-06/T-08 direkomendasikan solid di `2026-10-09-architectural-decisions-analysis.md`, tapi karena belum di-ODR-kan, agent menyajikan ULANG keempatnya sebagai "pertanyaan terbuka" di turn berikutnya — Owner menegur ini secara eksplisit ("kenapa anda memberi pertanyaan berulang... kita sudah menyepakati itu sebelumnya").

## 5. PR title WAJIB divalidasi terhadap `scripts/check-pr-title.sh` SEBELUM membuka PR, bukan sesudah CI gagal

**Format wajib:** `type(scope): description` dengan `type` ∈ {`build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, `test`}. **`phase(...)` BUKAN tipe yang valid** meskipun terasa deskriptif untuk kerja multi-sprint — gunakan `feat(...)` atau `refactor(...)` dan jelaskan "multi-sprint batch" di deskripsi PR, bukan di tipe commit.

**Insiden:** PR #18 dibuka dengan judul `phase(dev-task-2): ...` dan langsung gagal check "Validate PR title" — baru diperbaiki setelah ditemukan saat investigasi CI, bukan dicegah sebelum PR dibuka.

## 6. CI gate yang gagal WAJIB diinvestigasi sampai root cause sebelum disimpulkan "bug di kode kita"

**Urutan investigasi wajib:** (a) baca SCRIPT/workflow gate itu sendiri untuk tahu PERSIS apa yang dihitung/dicocokkan, (b) jalankan ulang logic yang sama secara lokal untuk melihat baris persis yang di-flag, (c) baca baris itu di konteks (fungsi apa, `mod tests` atau produksi, dummy value atau kredensial nyata) sebelum menyimpulkan genuine-violation vs false-positive gate.

**Insiden:** PR #18 di-flag "212 unwrap/expect" dan "50 potential secret leak" oleh Principle Gate — investigasi membuktikan SEMUA 50 secret-leak adalah dummy test credential/nama fungsi modul redaksi/baris workflow itu sendiri (0 kredensial nyata bocor), dan mayoritas sampel unwrap genuinely berada di `#[cfg(test)] mod tests` yang lolos filter path gate (bukan hilangnya error-handling produksi). Tanpa investigasi ini, kesimpulan cepat "kode remediasi penuh bug" akan SALAH dan membuang waktu memperbaiki kode yang sebenarnya sudah benar.

**Lihat `ci-infra-fix/README.md`** untuk 4 task konkret yang lahir dari pola ini — gate yang false-positive diperbaiki di SUMBERNYA (workflow), bukan dengan menambah workaround di kode fitur.

## 7. Task CI/tooling dipisah dari task fitur — jangan campur dalam satu PR/sprint

**Aturan:** perbaikan gate CI, workflow, dependency-version-conflict masuk `ci-infra-fix/` (atau PR terpisah) — BUKAN ditambahkan ke sprint fitur (`sprint-0`..`sprint-8`) atau dicampur ke PR fitur yang sedang direview.

**Alasan:** mencampur keduanya membuat reviewer manusia (Principle) kesulitan membedakan "apakah PR ini mengandung bug kode" vs "apakah alat ukur kode ini yang salah" — dua pertanyaan dengan proses verifikasi berbeda total.

## 8. Serialkan task yang menyentuh file/crate yang sama — jangan fan-out paralel ke scope beririsan

**Aturan:** sebelum menjalankan >1 subagent/task paralel, cek dulu apakah mereka menyentuh file atau crate yang sama. Jika ya, SERIALKAN (jalankan satu per satu, verifikasi, baru lanjut), jangan paralelkan dengan asumsi "secara tiket mereka independen".

**Insiden:** T-01 dan T-05 sama-sama menyentuh `crates/clawcrew-config` secara paralel — salah satu menjalankan `git checkout` multi-file yang menghapus pekerjaan task lain sebelum sempat commit (korban: T-01 DAN T-03, keduanya kehilangan kerja yang sudah genuinely benar).

---

**Catatan untuk tim lain:** aturan di atas bukan teori — setiap satu punya insiden nyata di sesi 2026-10-08/09 yang bisa ditelusuri ke `finding/2026-10-09-remediation-progress-handoff.md` dan `finding/2026-10-09-architectural-decisions-analysis.md`. Kalau menemukan pola kegagalan BARU yang tidak tercakup di atas, tambahkan sebagai aturan baru di sini (dengan insiden konkretnya) — jangan biarkan pola yang sama terulang karena tidak terdokumentasi.
