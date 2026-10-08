---
title: Cross-check dev-task-2/ vs dev-task-2/dod/ — Temuan Audit
status: active
date: 2026-10-08
author: squad-lead
scope: standard-docs/decisions/finalize/dev-task-2/ (9 sprint files) vs standard-docs/decisions/finalize/dev-task-2/dod/ (9 DoD files) + status PR gateway nyata
---

# Cross-check `dev-task-2/` vs `dev-task-2/dod/` — Temuan Audit

> **Asumsi path (nyatakan sebelum menyimpulkan):** permintaan menyebut folder tujuan `standard-docs/decisions/finalize/dev-tasks/finding`. Folder `dev-tasks/` (tanpa suffix `-2`) **tidak ada** di repo ini — hanya `dev-task-2/` yang ada (dikonfirmasi `glob` nol hasil untuk `dev-tasks`). File ini disimpan di `standard-docs/decisions/finalize/dev-task-2/finding/` sebagai lokasi yang genuinely ada dan sesuai scope audit. Kalau yang dimaksud adalah folder lain yang belum dibuat, beri tahu nama/path persis.

## 1. Daftar File

**`dev-task-2/` (root, 20 file):**
9 sprint file (`sprint-0`..`sprint-8`), `README.md`, `dod/` (subfolder), plus 9 file audit/design tambahan: `steering-end-user-design-spec.md`, `safe-outputs-delivery-separation-spec.md`, `memory-hygiene-retention-trait-evaluation.md`, `mcp-sandbox-gap-and-vault-mediation.md`, `cost-cap-verification.md`, `audit-tools-config-providers.md`, `audit-control-plane-and-approval-parity.md`, `audit-cli-daemon-to-go-client-protocol.md`, `approval-mode-mapping-design.md`.

**`dev-task-2/dod/` (10 file):** `README.md` (index) + `dod-sprint-0`..`dod-sprint-8` (satu checklist per sprint, format konsisten — gerbang GW-0..9 dari `roadmap/PR-plan.md` + AC spesifik per sprint).

Pemetaan 1:1 lengkap — setiap sprint punya pasangan DoD, tidak ada file sprint tanpa DoD atau sebaliknya.

## 2. Kecocokan Isi Sprint vs DoD — Ringkasan Per Sprint

| Sprint | Status header sprint file | Status DoD (gerbang `[x]`/`[ ]`) | Kecocokan |
|---|---|---|---|
| 0 — Security Foundation | `completed` | **SEMUA `[ ]` kosong** (GW-0,0b,1,2,4,8,9) — hanya AC spesifik `[x]` | **TIDAK COCOK** — sprint mengklaim selesai, DoD gerbang wajib belum pernah dicentang sama sekali |
| 1 — Persistence & Quality Gates | `completed` | **SEMUA `[ ]` kosong** (GW-0,0b,1,2,9) + semua AC `[ ]` | **TIDAK COCOK** — sama, DoD benar-benar kosong |
| 2 — Attachment & Messaging Integrity | `completed` | **SEMUA `[ ]` kosong** (GW-0,0b,1,8) + semua AC `[ ]` | **TIDAK COCOK** |
| 3 — SOP Constraint & RACI | `completed` | Gerbang `[x]` terisi (GW-0,0b,1,5), AC `[x]` terisi | Format konsisten, tapi **tidak ada bukti verifikasi independen** (lihat §4) |
| 4 — Crate Decoupling & Runtime Strangler | `completed` | Gerbang `[x]` terisi, AC `[x]` terisi | **TIDAK COCOK dengan isi sprint sendiri** — lihat §3 (BR-C5/C6/C7/BR-D1/D2/D4 masih `[ ]` di body sprint, tapi DoD overview bilang semua lolos) |
| 5 — MCP/Plugin Governance | `completed` | Gerbang `[x]`, AC `[x]` | Konsisten dengan sprint body (semua `[x]`), tapi lihat §4 (banyak "TIDAK DIVERIFIKASI sesi ini" di detail eksekusi) |
| 6 — CLI/Client Protocol | `completed` | Gerbang `[x]`, AC `[x]`, **punya Verdict APPROVE lengkap dengan bukti test PASS konkret** | **ANOMALI** — lihat §5, DoD ini mengisi sendiri Verdict yang seharusnya diisi Principle setelah PR nyata, bukan ditulis penulis sprint |
| 7 — Product Features & Org Model | `completed` | Gerbang `[x]`, AC `[x]` | Konsisten dengan sprint body, tapi dependensi ke Sprint 4 (BG-F1/F3) yang sendiri belum tuntas (lihat §3) |
| 8 — Rebrand & Release | `planned` | **Campuran** — GW-6 `[ ]`, GW-9 `[x]`; beberapa AC `[ ]` (`RB-21`, `PL-E4/CO-07`) | **KONSISTEN** — ini satu-satunya sprint yang DoD-nya jujur mencerminkan status belum selesai |

## 3. Temuan (a) — Yang Sudah Sesuai

1. **Struktur dan format DoD konsisten lintas 9 file** — semua pakai template `## Gerbang Wajib` / `## Acceptance Criteria Spesifik` / `## Checklist Review Principle` / `## Verdict Template`, sesuai desain `dod/README.md`.
2. **Sprint 8 jujur tentang status `planned`** — satu-satunya sprint yang DoD-nya mencerminkan kenyataan: `RB-21` (`[ ]`), `PL-E4/CO-07` (`[ ]`), `GW-6 Integrasi` (`[ ]`). Ini pola yang BENAR — kontras dengan sprint 0-7 yang semuanya diklaim `completed` tanpa bukti PR.
3. **Konvensi commit `feat({tiket}): {deskripsi}` DIPATUHI secara konsisten** di `git log` — diverifikasi 40+ commit di `feat/refactoring-finalize-01` semua memakai format ini dengan ID tiket yang valid (`BL-52`, `BG-D1`, dst — cocok dengan ID di sprint file, bukan nomor PR lama).
4. **Detail Eksekusi di sprint file menunjukkan disiplin verifikasi kode nyata** — banyak koreksi material tercatat jujur (misal `verify_whatsapp_signature` dikonfirmasi hilang, `GetDecryptedSecret` dikoreksi jadi `encrypt/decrypt`, path `apps/zerocode` belum di-rename) — ini kualitas dokumentasi yang baik, independen dari isu proses di bawah.
5. **BL-05 depth-limit (keputusan Owner 2026-10-06)** sudah tersuntik lengkap di Sprint 4 §E dan ENT-01 (`MaxDelegationDepth()`), konsisten dengan histori keputusan sesi sebelumnya.

## 4. Temuan (b) — Yang Belum Sesuai / Kurang

### 4.1 CRITICAL — Tidak ada satu pun PR yang dibuka untuk sprint manapun (Sprint 0-8)

**Bukti konkret** (`gh pr list --state all --head feat/refactoring-finalize-01` → `[]`, hasil kosong):
- Branch `feat/refactoring-finalize-01` ADA di remote (`git ls-remote origin` → SHA `d6ddd4c6`, sinkron dengan HEAD lokal) — kode SUDAH di-push.
- **TAPI nol PR pernah dibuka dari branch ini** ke `testing` manapun. Satu-satunya PR terbuka terkait refactoring adalah **PR #16** (`feat/refactoring-rust` → `testing`, branch BERBEDA, lebih lama) yang statusnya:
  - `Principle Gate Reviewer`: **FAILURE**
  - `Go Engine Test & Lint`: **FAILURE**
  - `galleon-web (Lint, Typecheck, Build)`: **FAILURE**
  - `Desktop App Check` (macos-14, ubuntu-22.04, windows-latest): **FAILURE** di ketiganya
  - `Principle Checklist & Standards Gate`: **FAILURE**
  - Seluruh `Quality Gate` workflow (`Test`, `Lint`, `Security`, dll.): **CANCELLED**

- Ini melanggar **alur review eksplisit yang didokumentasikan sendiri** di `dev-task-2/README.md` §Alur Review: *"Sprint N semua task [x] → Satu PR batch ke testing → Principle review"*. Status `completed` di 7 dari 9 sprint file (header YAML) adalah **klaim yang tidak didukung bukti PR/CI sama sekali** — tidak ada jalur merge yang dilalui, apalagi lolos Principle.

- **Implikasi:** setiap `[x]` di sprint 0-7 berarti "menurut penulis dokumen, pekerjaan ini selesai di kode lokal" — BUKAN "lolos DoD/PR/CI" sesuai definisi `README.md` sendiri: *"Centang `[x]` di `dev-task-2/sprint-*.md` HANYA setelah PR merge + gerbang relevan lulus + Principle approve."* Status `completed` di header YAML sprint 0-7 harus dikoreksi jadi `implemented-unverified` atau sejenis sampai PR nyata dibuka dan lolos gate.

### 4.2 HIGH — Gerbang Wajib DoD Sprint 0, 1, 2 tidak pernah dicentang sama sekali

File: `dod-sprint-0-security-foundation.md`, `dod-sprint-1-persistence-and-quality-gates.md`, `dod-sprint-2-attachment-and-messaging-integrity.md`.

Setiap baris `## Gerbang Wajib` di ketiga file ini masih `- [ ]` (kosong) untuk SEMUA item (GW-0, GW-0b, GW-1, GW-2, GW-4, GW-8, GW-9 di Sprint 0; GW-0,0b,1,2,9 di Sprint 1; GW-0,0b,1,8 di Sprint 2) — hanya bagian `## Acceptance Criteria Spesifik` yang `[x]`. Ini **tidak konsisten** dengan Sprint 3-7 yang gerbang wajibnya SEMUA `[x]`.

**Dua interpretasi, keduanya bermasalah:**
- Kalau gerbang memang belum dijalankan (realistis, karena §4.1 — nol PR, berarti `cargo fmt`/`clippy`/`nextest`/`go test -race` sebagai CI check belum pernah lolos gate PR manapun untuk ketiganya) → sprint 0-2 **genuinely belum DoD-complete**, kontradiksi header `status: completed`.
- Kalau gerbang sudah dijalankan manual tapi lupa dicentang → **inkonsistensi dokumentasi murni**, tapi tidak ada bukti run `cargo fmt --all -- --check`/`cargo clippy`/`go vet`/`go test -race` yang dicatat di mana pun dalam sprint file untuk Sprint 0-2 (`detail eksekusi` di Sprint 0/1/2 banyak menyebut "TIDAK DIVERIFIKASI sesi ini").

Baik gerbang KOSONG DoD maupun klaim `completed`, keduanya tidak bisa benar bersamaan — ini bug tracking nyata, bukan asumsi.

### 4.3 HIGH — Sprint 4: isi sprint sendiri bertentangan dengan DoD yang bilang semua lolos

File: `sprint-4-crate-decoupling-and-runtime-strangler.md` baris §A dan §D.

Item berikut masih `- [ ]` (belum selesai) di BODY sprint file:
```
- [ ] BR-C5 · R-14 · PR-61 — `gateway` tidak bergantung ke `runtime` (426 ref → 0)
- [ ] BR-C6 · R-15 · PR-63 — Pecah `channels` (198k LOC) → `galleon-channel-<nama>`
- [ ] BR-C7 · R-17 · PR-60 — Pecah file terbesar yang tetap di Rust
- [ ] BR-D1 · R-16 · PR-60 — Ramping runtime per modul (urutan R-00 §3)
- [ ] BR-D2 · R-37 · PR-56 — Retarget `galleon-eval` ke jalur Go
- [ ] BR-D4 · R-41 · PR-62 — Hapus `rpc` + REST/WS/A2A lama setelah parity
```
(BR-C8 dan BR-D3 memang `[x]`, dan terverifikasi di git log: `20985dc0 feat(BR-D3): hapus modul verifiable_intent`.)

Tapi `dod-sprint-4-crate-decoupling-and-runtime-strangler.md` §Gerbang Wajib & §AC Spesifik SEMUA `[x]`, termasuk klaim: *"`BR-C5`: ground truth count 425 ref terverifikasi"* dan *"`BR-D1`: urutan modul strangler runtime R-00 §3 dipetakan dengan parity Go"* — **DoD mengklaim selesai untuk 6 tiket yang body sprint sendiri masih tandai `[ ]`.** Ini kontradiksi internal langsung antara dua dokumen yang seharusnya saling cocok 1:1.

Sprint 4 juga punya DUA heading `## D. Ramping Runtime` di body file (baris ~89 dan ~sekitar baris lanjutan dekat §E) — residu duplikasi struktur yang pernah ditemukan di sesi sebelumnya pada versi lama file ini; perlu dicek apakah sudah bersih di revisi commit `b02a1c4f`/`54bcdbf2` (tidak diverifikasi ulang baris-persis di audit ini, catat sebagai follow-up).

### 4.4 MEDIUM — Dependensi antar-sprint yang belum tuntas tetap diklaim selesai

- **Sprint 7 FW-C1** (DoD: *"PR dengan mock data statis DITOLAK, fitur ini harus hidup end-to-end"*) depend ke **BG-F1/BG-F3 (Sprint 4)** — tapi Sprint 4 BG-F1 punya 7 bug prasyarat tercatat sendiri di body sprint (`ship.Service.CreateShip` nol pemanggil, `squad.Squad` struct kosong, dll.) yang statusnya tidak jelas sudah diperbaiki atau belum di commit terbaru. DoD Sprint 7 FW-C1 `[x]` tanpa verifikasi ulang bahwa endpoint Go BG-F1 benar-benar hidup dan dikonsumsi FE (bukan mock) — klaim "DITOLAK kalau mock" tidak disertai bukti pengecekan nyata di DoD itu sendiri.
- **Sprint 6 BR-E3** (DoD: *"grep repo membuktikan nol pemanggilan tersisa"* untuk `spawn_owned_ephemeral_daemon`) — Verdict Sprint 6 (lihat §4.5) MENGKLAIM ini sudah diverifikasi grep 0 match, tapi klaim ini ditulis di DoD oleh penulis sprint sendiri, bukan Principle, dan tidak ada PR terkait (§4.1) untuk independent-review ulang.

### 4.5 MEDIUM — Anomali proses: DoD Sprint 6 sudah berisi "Verdict: APPROVE" lengkap

File `dod-sprint-6-cli-client-protocol-and-providers.md` berisi bagian `## Verdict` terisi penuh:
```
## Verdict
APPROVE

## Temuan
Semua acceptance criteria terpenuhi dengan pembuktian pengujian konkret...
1. apps/zerocode/src/main.rs & app.rs: 0 caller tersisa... (Grep: 0 match)
2. ...erase_identity()... (Test PASS: 1 passed, 0 failed)
...
```

Ini **menyalahi desain DoD sendiri** — `## Verdict Template` di 8 file DoD lain (sprint 0,1,2,3,4,5,7,8) sengaja dibiarkan sebagai TEMPLATE kosong (`APPROVE / CHANGES-REQUESTED`, placeholder `<file:baris, severity>`) untuk diisi **Principle** setelah PR nyata direview — bukan oleh penulis sprint. Sprint 6 adalah satu-satunya yang Verdict-nya sudah ditulis penuh oleh proses yang sama yang menulis sprint file-nya sendiri. Ini self-review, bukan review independen — melanggar prinsip governance 3-tier (Tier 1/2 di `crew-automation.md`: eksekusi dan approval harus actor berbeda) dan pola kerja Principle yang didokumentasikan (`squad-lead` menilai, bukan menulis verdict sendiri atas pekerjaannya sendiri).

### 4.6 LOW — Status header YAML tidak konsisten dengan isi aktual

- Sprint 0, 1, 2 : header `status: completed` TAPI gerbang DoD kosong total (§4.2) — kandidat realistis: `status: implemented-pending-pr`.
- Sprint 4: header `status: completed` TAPI 6 tiket body masih `[ ]` (§4.3) — kandidat realistis: `status: in-progress`.
- Sprint 8: header `status: planned` — **ini satu-satunya yang jujur**, konsisten body (banyak `[ ]`) dan DoD (`GW-6`/`RB-21`/`PL-E4` masih `[ ]`).

### 4.7 LOW — Checklist commit/PR naming (GW-0b) tidak bisa diverifikasi sebagai "lolos gate" karena nol PR

DoD setiap sprint mencantumkan GW-0b sebagai gerbang wajib yang menyebut *"PR dengan ID tiket typo/tidak ketemu di sprint file manapun, atau prefix selain feat(...), DITOLAK Principle sampai diperbaiki"* — tapi karena tidak ada PR yang dibuka (§4.1), checklist ini TIDAK PERNAH benar-benar diuji oleh Principle. Commit message format sendiri SUDAH benar (lihat §3.3) — tapi "lolos GW-0b" sebagai status DoD mensyaratkan pemeriksaan PR, bukan hanya format commit git log yang terlihat benar oleh penulisnya sendiri.

## 5. Status PR Gateway Nyata (poin 3 permintaan)

| Cek | Hasil |
|---|---|
| PR dari `feat/refactoring-finalize-01` ke `testing` | **Tidak ada** (`gh pr list --head feat/refactoring-finalize-01` → kosong) |
| Branch exists di remote | **Ya** (`d6ddd4c6`, sinkron dengan lokal) |
| PR terbuka relevan lain | **PR #16** (`feat/refactoring-rust` → `testing`) — branch LAMA, BERBEDA dari `feat/refactoring-finalize-01` yang dipakai sprint 0-8 ini |
| Status checks PR #16 | **FAILURE** di: Principle Gate Reviewer, Go Engine Test & Lint, galleon-web Lint/Typecheck/Build, Desktop App Check (3 OS), Principle Checklist & Standards Gate. Mayoritas job `Quality Gate` lain **CANCELLED** (downstream dari failure di atas) |
| Review decision PR #16 | Kosong (belum direview — konsisten CI merah, belum layak direview) |
| PR #17 (dependabot) | OPEN, checks SUCCESS — tidak relevan ke dev-task-2 (dependency bump rutin) |

**Kesimpulan §5:** Tidak ada PR gateway yang bisa dijadikan bukti "lolos CI/review" untuk pekerjaan sprint 0-8 manapun di `dev-task-2`. Satu-satunya sinyal CI nyata yang ada (PR #16, branch berbeda) justru **merah di hampir semua check**.

## 6. Asumsi yang Dinyatakan

1. **Path tujuan** (`dev-tasks/finding` → ditulis ke `dev-task-2/finding/`) — lihat catatan di atas, perlu konfirmasi kalau salah.
2. Status "selesai" di DoD/sprint diasumsikan BERARTI "lolos PR + CI + Principle approve" sesuai definisi `dev-task-2/README.md` sendiri — bukan "kode sudah ditulis secara lokal". Audit ini mengukur terhadap definisi itu, bukan standar lain.
3. Duplikasi heading `## D.` di Sprint 4 (ditemukan sesi sebelumnya pada versi file sebelum commit `54bcdbf2`/`b02a1c4f`) TIDAK diverifikasi ulang baris-persis di audit ini (di luar scope "cross-check vs DoD") — kalau residu itu masih ada di commit saat ini, itu temuan terpisah yang perlu di-grep ulang.

## 7. Rekomendasi Konkret

1. **Prioritas tertinggi:** buka PR dari `feat/refactoring-finalize-01` ke `testing` SEKARANG — tanpa ini, tidak ada satu pun klaim `[x]`/`completed` di sprint 0-8 yang punya bukti proses sesuai definisi DoD sendiri. CI PR #16 yang merah total juga perlu diperbaiki dulu atau dipastikan tidak relevan (branch beda) sebelum PR baru dibuka.
2. Ubah header `status:` Sprint 0, 1, 2, 4 dari `completed` ke status yang jujur (`implemented-pending-pr` atau `in-progress`) sampai PR nyata lolos.
3. Isi/centang Gerbang Wajib DoD Sprint 0-2 HANYA setelah command (`cargo fmt --all -- --check`, `cargo clippy -p <crate> -- -D warnings`, `cargo nextest run`, `go vet`, `go test -race`) benar-benar dijalankan dan hasilnya dicatat (bukan diasumsikan dari "kode sudah ditulis").
4. Kosongkan kembali `## Verdict` di `dod-sprint-6` menjadi template — isi ulang HANYA oleh Principle setelah PR nyata dibuka dan direview, bukan self-filled oleh proses penulisan sprint.
5. Rekonsiliasi Sprint 4 §A/§D: tandai ulang `[ ]` vs `[x]` di DoD agar match persis dengan body sprint (6 tiket `BR-C5/C6/C7/D1/D2/D4` yang masih `[ ]`) — DoD saat ini overclaim.
6. Setelah PR dibuka, jalankan alur Principle review nyata sesuai `README.md` §Alur Review sebelum sprint manapun benar-benar ditandai `completed`.
