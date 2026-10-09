---
title: Dev Task 2 — Sprint Breakdown Terkonsolidasi (dev-tasks/ + backlog/ research)
status: active
date: 2026-10-05
author: squad-lead
supersedes_view: ../dev-tasks/ (tetap dibaca untuk histori PR-selesai; lihat §Reconciliation)
---

# Dev Task 2 — Sprint Breakdown

> **Kenapa folder baru, bukan edit `dev-tasks/`:** `dev-tasks/` adalah working-doc tim dev eksternal (coding di mesin lain) — dikecualikan dari auto-maintenance per lesson 2026-10-04. Folder ini (`dev-task-2/`) adalah **lapisan rekonsiliasi** yang digabungkan oleh Lead Squad: tugas asli `dev-tasks/01..08` (121 tiket roadmap, basis tim dev) + seluruh backlog riset `standard-docs/backlog/` (BL-01 s.d. BL-61, basis Lead/Principle/squad). Tujuan: satu urutan sprint tunggal supaya BE/FE/Platform **tidak mengerjakan hal yang sama dua kali** dari dua sumber berbeda.
>
> **⚠️ Update 2026-10-09:** `dev-tasks/` (tanpa -2) sudah dipindah seutuhnya ke [`../../archive/dev-tasks/`](../../archive/dev-tasks/) — tim dev eksternal sudah selesai/sinkron, bukan lagi working-doc aktif. Dibaca sebagai histori, tidak diedit.
>
> **Aturan baca:** `dev-task-2/sprint-*.md` adalah SSOT untuk **urutan kerja ke depan** — gabungan tiket `BG-*`/`BR-*`/`FW-*`/`FD-*`/`PL-*`/`RB-*`/`GV-*` yang masih `[ ]` + seluruh `BL-*` backlog riset, disusun ulang per-sprint lintas sumber. 4 dari 9 sprint (1,2,3,7) sudah genuinely 100% `[x]` dan dipindah ke [`../../archive/dev-task-2-completed-sprints/`](../../archive/dev-task-2-completed-sprints/) — 5 sprint yang tersisa di sini (0,4,5,6,8) masih punya item `[ ]` terbuka.
>
> **⚠️ WAJIB BACA SEBELUM MENGERJAKAN TASK APA PUN:** [`RULES.md`](RULES.md) — 8 aturan kerja (verifikasi, dokumentasi, CI gate, serialisasi task beririsan) yang lahir dari insiden nyata di sesi 2026-10-08/09. Setiap aturan punya bukti insiden konkret, bukan preferensi gaya.
>
> **Status terbaru per 2026-10-09 malam:** PR #18 (T-01 s/d T-08 feature remediation) masih **OPEN, belum merge** ke `testing` — jangan anggap checkbox terkait (mis. BR-B5 vault) sudah tuntas sampai genuinely merged. 4 masalah CI/governance-tooling yang ditemukan saat PR #18 diinvestigasi (false-positive Principle Gate unwrap-filter — ternyata ada di 2 workflow, bukan 1 — sccache runner, Gate D-2 overbroad regex, dependency conflict `web/`) sudah **TUNTAS dan terverifikasi** di [PR #19](https://github.com/diezy-realm/galleon-fleet/pull/19) (`fix/ci-infra-gates` → `testing`) — lihat [`ci-infra-fix/README.md`](ci-infra-fix/README.md).

## Status Verifikasi Dasar (2026-10-05, branch `feat/refactoring-rust` @ `a296dc23`)

Sebelum menyusun sprint, status `dev-tasks/` diverifikasi langsung ke kode (bukan asumsi dari dokumen) — lihat percakapan sesi ini untuk jejak file:baris. Ringkasan: **Sprint 0 dan sebagian besar Sprint 1 ASLI sudah selesai nyata di kode** (bukan klaim kosong) — BG-A1/A2/A3/A4/A5/A6, BR-A1-A4, BR-B1/B2, BR-C1-C4, FD-A1-A4, FW-A0/A1/A2a/A5/B1, PL-A1-A4, RB-10/11/13/14/15, GV-02-10, BG-B1/B2 semua terverifikasi nyata (implementasi substantif, bukan stub) via spot-check 6 klaim kritis. Satu temuan baru dicatat: **approval-gate endpoint terminal (`BG-A4`) belum punya verifikasi identitas pemanggil** — lihat Sprint 2 item baru `S2-SEC-01`.

Item dev-tasks yang BELUM selesai (`[ ]`) + seluruh BL-01..61 backlog digabung di sprint di bawah.

## Peta Sprint

| File | Fokus | Sumber Gabungan |
|---|---|---|
| [`sprint-0-security-foundation.md`](sprint-0-security-foundation.md) | P0 keamanan sisa + gap approval-identity baru ditemukan | BG-A1b, BG-C1..C3, BR-B3..B6, BL-32, BL-14, BL-19 |
| [`sprint-1-persistence-and-quality-gates.md`](../../archive/dev-task-2-completed-sprints/sprint-1-persistence-and-quality-gates.md) — **ARSIP, 63/63 [x]** | Persistensi Go (SQLite lanjutan/Postgres/RAG), burn-down kualitas Rust, **skills/lessons persistence** | BG-B3..B5, BR-A5..A6, BL-45a/45b/46/47 (token-efficiency), BL-48..50, **BL-01/03/04 (skills-lessons, §E baru)** |
| [`sprint-2-attachment-and-messaging-integrity.md`](../../archive/dev-task-2-completed-sprints/sprint-2-attachment-and-messaging-integrity.md) — **ARSIP, 8/8 [x]** | Bug data-loss attachment + content-safety channel | BL-52..55, BL-14, BL-37, BL-38, BL-39 |
| [`sprint-3-sop-constraint-and-raci.md`](../../archive/dev-task-2-completed-sprints/sprint-3-sop-constraint-and-raci.md) — **ARSIP, 13/13 [x]** | SOP engine: Do's/Don'ts, RACI, Constraint layer (SoD), **delivery-separation & cost-cap** | BL-06..09, BL-57..61, **BL-12/13 (§D baru)** |
| [`sprint-4-crate-decoupling-and-runtime-strangler.md`](sprint-4-crate-decoupling-and-runtime-strangler.md) | Dekoplasi crate besar + strangler runtime→Go, **satukan jalur delegasi sub-agent** | BR-C5..C8, BR-D1..D4, BG-D1..D5, BL-18 (↑ diangkat), **BL-05 (§C, CRITICAL — baru)** |
| [`sprint-5-mcp-plugin-and-subagent-resource.md`](sprint-5-mcp-plugin-and-subagent-resource.md) | Sandbox MCP, resource-gate sub-agent, plugin governance | BL-15..17, BL-19..23, BL-24 (tetap exploratory) |
| [`sprint-6-cli-client-protocol-and-providers.md`](sprint-6-cli-client-protocol-and-providers.md) | CLI/Client Protocol, provider reliability/OAuth, memory governance | BG-E1/E2, BR-E1..E4, BL-25..31 |
| [`sprint-7-product-features-and-org-model.md`](../../archive/dev-task-2-completed-sprints/sprint-7-product-features-and-org-model.md) — **ARSIP, 23/23 [x]** | Fitur produk FE (organisasi, approval UI), payment-lockout preventif, **toolbox parity KiroCrew (BL-62/68/69/71/72/74/75/76/77)**, **steering end-user** | BG-F1..F6, FW-C1..C5, BL-40..44, BL-62/68/69 (`ui-nice-to-have-backlog.md`), BL-71/72/74/75/76/77 (`toolbox-parity-backlog.md`), **BL-02 (§F baru)**, **BL-73/78/79 dicatat REFERENSI EXPLORATORY saja di §D (bukan tiket aktif)** |
| [`sprint-8-rebrand-and-release.md`](sprint-8-rebrand-and-release.md) | Rename massal, WIT v1, CI pangkas, rilis/installer, **service installer & state migration** | RB-02, RB-12, RB-16/17, RB-20..41, BR-F1..F5, PL-A5, PL-B2, PL-C1/C2, PL-D1/D2, GV-01/11/12, **BL-10/11 (§D)** |

## Reconciliation — BL yang Digabung ke Dev-Task Existing (Bukan Duplikat)

Beberapa BL sudah punya **tiket dev-tasks yang persis sama scope-nya** — ini BUKAN kerja baru, hanya diberi nomor silang supaya terlihat satu sumber kebenaran:

| BL | Tiket dev-tasks yang sama | Keputusan |
|---|---|---|
| BL-18 (resource gate `ExecuteTurn` jalur Go `spawn_subagent`) | `BG-D1`/`BG-D4` (agent loop + sub-agent/handoff di Go) | **DITUNDA → sekarang DIANGKAT** di Sprint 4, karena BG-D1/BG-D4 sudah dijadwalkan di sprint yang sama — gate ditempel sebagai acceptance criteria tambahan BG-D1, bukan tiket terpisah |
| BL-51 (vector memory Go per-scope, koordinasi RAG) | `BG-B4` (memori vektor/RAG per scope Fleet/Ship/Squad/Crew/Run) | **DITUNDA → sekarang DIANGKAT** di Sprint 1, BL-51 jadi acceptance criteria tambahan BG-B4 (bukan tiket terpisah) |
| BL-37/BL-38/BL-39 (redaksi tool I/O, scrub, permission log) | Tidak ada tiket dev-tasks setara — murni temuan Lead | Tetap tiket BL mandiri di Sprint 2, TIDAK bentrok file dengan BG/BR manapun |
| BL-14 (ContentSafety ke channel orchestrator biasa) | Tidak ada tiket dev-tasks setara | Tiket BL mandiri, tapi **irisan file** dengan BL-52 (`clawcrew-channels/src/orchestrator/mod.rs`) — digabung ke sprint yang sama (Sprint 2) agar satu PR, bukan dua PR beririsan file |
| BR-C5 (`gateway` lepas dari `runtime`) | Dep: `BR-D1` cron (dev-tasks sendiri) | Tetap di Sprint 4 bersama BR-D1, urutan dari dev-tasks dipertahankan |
| **BR-F1** (errata ADR-001: WhatsApp Tier A, WIT v0 beku, bluesky) | **`GV-07`** — SUDAH `[x]` selesai PR-21, file `standard-docs/decisions/refactoring-rust/phase-2/ADR-001-errata.md` **sudah ada dan isinya persis mencakup ketiga poin BR-F1** (diverifikasi baca file langsung) | **BR-F1 DITANDAI SUDAH TERPENUHI oleh GV-07 — TIDAK dijadwalkan ulang di sprint manapun.** Ini duplikat murni di dalam `dev-tasks/02-be-rust-core.md` sendiri (tiket `[ ]` yang kerjanya sudah dilakukan tapi dicentang di bawah nomor tiket lain) — perlu diberitahu ke tim dev agar BR-F1 dicentang `[x]` juga, bukan dikerjakan ulang |
| BL-33 (Windows Authenticode signing) | `PL-D1` (Pipeline rilis desktop MSI/NSIS, **signing**) | BL-33 adalah detail acceptance-criteria dari `PL-D1`, BUKAN tiket sejajar baru — digabung sebagai sub-item `PL-D1` di Sprint 8 |
| BL-34 (`tauri.bundled.conf.json` hilang) | `PL-D1`/`FD-C2` (bundling Windows) | Sub-item verifikasi `PL-D1`/`FD-C2` di Sprint 8, bukan tiket mandiri |
| **BL-71/72/74/75/76/77** (toolbox parity KiroCrew — approval-mode toggle, sambung ApprovalForm ke API, hapus context placeholder, chat-to-entity, @/slash autocomplete, bar project/workspace) | Tidak ada tiket dev-tasks setara (murni temuan audit UI/UX Lead) | **DIANGKAT langsung** ke Sprint 7 §D sebagai kerja aktif — BL-77 MENYATU dengan BL-62/68/69 (§E), bukan tiket sejajar baru |
| BL-73 (Quest-loop nudge-interval) & BL-78 (Optimize prompt) | Tidak ada tiket dev-tasks setara | **TETAP EXPLORATORY** — tidak dimasukkan sprint manapun, scope besar/definisi belum jelas, tunggu keputusan Owner eksplisit |
| BL-80/81/82 (tier/pricing & entitlement — `QuotaPolicy`, 5 bug prasyarat guard kuota, module path rename) | `BG-F1` (model organisasi persisten) | **DIANGKAT langsung** ke Sprint 4 §E (2026-10-06, keputusan Owner Q2/Q4 riset `tier-pricing-and-entitlement-final.md`) — BL-81 MENYATU sebagai AC tambahan `BG-F1` (bukan tiket terpisah menunggu BG-F1 selesai, karena scope BG-F1 sendiri memang "dari nol"); BL-80 jadi `ENT-01` dan BL-82 jadi `GO-MOD-RENAME`, keduanya berdiri sendiri tapi dep ke BL-81/BG-F1 untuk makna guard. Sumber: `standard-docs/backlog/tier-entitlement-pricing-backlog.md`. |

## Audit Menyeluruh 2026-10-05 — BL yang TERLEWAT dari Seluruh Sprint (Ditemukan & Diperbaiki)

> Permintaan Achmad: audit lintas `research/`+`backlog/`+`dev-task-2/` untuk pastikan tidak ada gap tercatat tapi lupa diangkat jadi tiket. Metodologi: ekstrak semua ID tiket dari 9 file sprint, bandingkan dengan `backlog/README.md` (peta lengkap BL-01..78). Hasil: **9 BL ditemukan belum pernah masuk sprint manapun** sejak backlog itu dibuat — bukan DITUNDA/EXPLORATORY yang disengaja, genuinely terlewat.

| BL | Judul | Severity | Sprint Baru | Alasan Penempatan |
|---|---|---|---|---|
| BL-05 | Satukan 2 jalur delegasi sub-agent (`spawn_subagent` vs `DelegateTaskTool`) | **CRITICAL** (celah privilege-escalation aktif) | Sprint 4 §C | File-scope sama (`dispatcher.go`) dengan BG-D1/BG-D4; HARUS selesai sebelum BG-D1 menyentuh file ini |
| BL-01 | Jembatan gRPC: persist Lesson/Learning Go→Rust vector store | HIGH (bug fungsional nyata) | Sprint 1 §E (baru) | Tema identik §A/§D sprint ini (jembatan Go-Rust storage persisten) |
| BL-03 | Dokumentasikan ekosistem `open-skills` existing | LOW | Sprint 1 §E (baru) | Paralel BL-01, dokumentasi murni |
| BL-04 | Test skill/steering inheritance sub-agent | MEDIUM | Sprint 1 §E (baru) | Lebih bermakna setelah BL-01 |
| BL-02 | GAP-01: rancang steering produk Galleon end-user | MEDIUM (perancangan) | Sprint 7 §F (baru) | Menyentuh pengalaman end-user produk, bukan backend internal |
| BL-10 | Service installer setara `kirocrew service install` | MEDIUM | Sprint 8 §D | Tema deployment sama dengan `PL-D1`/`PL-D2` existing |
| BL-11 | State migration tooling (cron/memory/config/skills) | MEDIUM (risiko data-loss) | Sprint 8 §D | Paralel BL-10, tema deployment |
| BL-12 | Pemisahan eksekusi vs delivery SOP/cron trigger eksternal | LOW (evaluasi dulu) | Sprint 3 §D (baru) | Menyentuh `SopRunAction::ExecuteStep`, tema sama dengan Constraint Layer |
| BL-13 | Cost cap eksplisit per cron-job/SOP-run | LOW-MEDIUM (verifikasi dulu — mungkin sudah tercakup `SecurityPolicy.max_actions_per_hour`) | Sprint 3 §D (baru) | Paralel BL-12 |

**Catatan tambahan dari audit ini**: `research/README.md` dan `backlog/README.md` masih merujuk "BL-01 s.d. BL-39" di beberapa tempat (stale sejak backlog bertambah ke BL-78) — sudah diperbaiki. Dua dokumen riset baru (`ui-right-rail-journal-newchat-audit.md`, `toolbox-parity-audit.md`) ditambahkan ke index `research/README.md` Tema 15 (sebelumnya tidak terindeks).

**Prinsip gabungan**: kalau 2+ tiket (dari sumber manapun) menyentuh file yang sama atau punya hard dependency, mereka **masuk sprint yang sama** dan diberi catatan "serialkan penulis" di file sprint — sesuai aturan assignment Lead (hindari dua crew menulis file sama).

## Urutan & Ketergantungan Antar-Sprint

```
Sprint 0 (keamanan) ──┬─→ Sprint 2 (attachment/messaging) ──→ Sprint 3 (SOP constraint)
                      │
                      ├─→ Sprint 1 (persistence/quality) ──→ Sprint 6 (CLI/providers)
                      │
                      └─→ Sprint 4 (crate decoupling/strangler) ──→ Sprint 5 (MCP/subagent)
                                                               ──→ Sprint 7 (product features)
                                                                        ↓
                                                              Sprint 8 (rebrand/release) — SELALU TERAKHIR
```

Sprint 8 (rename massal) secara desain **tidak boleh mulai** sebelum Sprint 0-7 selesai — ini aturan existing dev-tasks (`RB-2x` hanya setelah hapus/pecah selesai) yang dipertahankan, bukan aturan baru.

## DoD & Checklist PR

Lihat [`dod/`](dod/) — satu file checklist per sprint, format konsisten dengan `roadmap/PR-plan.md` §1 (gerbang GW-0..9) + Definition-of-Done per-tim dari `dev-tasks/01..08`. Checklist DoD TIDAK mengubah gerbang yang sudah ada — hanya mengelompokkannya per-sprint supaya Principle punya satu daftar per PR-batch (sesuai pola kerja "PR batch-by-phase" yang sudah berjalan).

## Verifikasi Tambahan (2026-10-05) — `web-2`/`apps/tauri-2` Migration Status

Dicek langsung ke filesystem (bukan asumsi dari dokumen), karena dev-tasks menyatakan "`web/` = bekas `web-2/`" dan "`apps/tauri/` = bekas `apps/tauri-2/`" — perlu dipastikan ini migrasi TUNTAS, bukan dua direktori hidup berdampingan.

| Direktori lama | Status | Isi | Risiko |
|---|---|---|---|
| `web-2/` | **Masih ada di disk** | Hanya `dist/` (build output) + `node_modules/` (gitignored) — **TIDAK ada `src/`** | `web-2/dist/` (11 file) **masih ter-commit di git** (`git ls-files web-2` mengembalikan hasil) — sampah build artifact lama, bukan source code aktif |
| `apps/tauri-2/` | **Masih ada di disk** | Hanya `target/` (build output Rust) | **0 file ter-tracked git** (`git ls-files apps/tauri-2` kosong) — sepenuhnya lokal/gitignored, aman diabaikan |

**Simpulan:** migrasi SOURCE sudah tuntas nyata — `web/` dan `apps/tauri/` berisi source lengkap (`src/`, `Cargo.toml`, dst), `web-2/`/`apps/tauri-2/` HANYA menyisakan build-output lokal. **Satu temuan kecil nyata:** `web-2/dist/` punya 11 file yang masih ter-commit ke git sejak sebelum rename — ini sampah repo, bukan source yang belum dimigrasikan. **Tindakan:** ditambahkan sebagai item baru `RB-16b` di Sprint 8 (`git rm -r web-2/dist`), BUKAN item migrasi baru karena migrasi sumber sudah selesai (`FD-A1`/`FW-A0`/`FW-A1` sudah `[x]`).

## Konvensi Commit & PR (WAJIB, berlaku semua sprint)

### Template Commit & Judul PR

```
feat({nomor tiket}): {deskripsi singkat}
```

- `{nomor tiket}` = ID tiket PERSIS seperti di sprint file (`BG-A4`, `BL-52`, `S2-SEC-01`, dst) — **bukan** nomor PR dev-tasks lama (`PR-15`), **bukan** nomor roadmap (`G-04`). Satu commit/PR yang menuntaskan beberapa tiket sekaligus menulis semua ID dipisah koma: `feat(BL-52,BL-14): perbaiki debounce attachment loss + content-safety channel`.
- `{deskripsi singkat}` — present tense, bahasa bebas (ID/EN), tanpa titik di akhir, maks ±70 karakter total judul (konsisten aturan PR title existing).
- Prefix SELALU `feat(...)` untuk pekerjaan sprint ini — **bukan** `fix(*)`/`hotfix(*)`/`phase(*)` yang sudah dipakai konvensi lama. Kalau tiket genuinely bug-fix murni (misal `BL-32`, `BL-52`), tetap pakai `feat({tiket}): ...` supaya satu konvensi penomoran berlaku rata — jangan campur `fix(BL-52)` dan `feat(BL-14)` dalam sprint yang sama.
- Contoh valid:
  - `feat(BG-A1b): satukan kontrak native tool Go-Rust via proto`
  - `feat(S2-SEC-01): verifikasi identitas pada approval-gate terminal`
  - `feat(BL-57): tambah tipe SopConstraint di sop::types`

### Checklist Commit/PR (ditambahkan ke SETIAP file `dod/dod-sprint-*.md`)

- [ ] Judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis — **tidak ada pengecualian prefix lain untuk pekerjaan sprint ini**.
- [ ] `{tiket}` yang ditulis cocok PERSIS dengan ID di sprint file terkait (case-sensitive, tanpa spasi) — PR dengan ID yang typo/tidak ketemu di sprint file manapun DITOLAK Principle sampai diperbaiki.
- [ ] Kalau commit/PR menuntaskan >1 tiket, SEMUA ID ditulis dipisah koma tanpa spasi setelah koma (`BL-52,BL-14`, bukan `BL-52, BL-14` atau `BL-52 & BL-14`).
- [ ] PR description menyebutkan sprint asal (`Sprint 2`) secara eksplisit di baris pertama body — memudahkan Principle mencocokkan ke file DoD yang benar.

> **Kenapa ini ditambahkan sekarang:** audit checklist DoD existing (`dod/dod-sprint-*.md`) ternyata TIDAK ada satu pun yang eksplisit mengecek penamaan commit/PR — gap severity MEDIUM (bukan HIGH, karena tidak merusak kode, tapi menyulitkan traceability tiket↔commit yang jadi tujuan utama `dev-task-2/`). Checklist ini disisipkan ke SETIAP file DoD sebagai bagian "Gerbang Wajib", bukan hanya didokumentasikan di sini saja.

## Kebijakan Branch (WAJIB, berlaku semua sprint)

**SATU branch fitur untuk seluruh `dev-task-2/` — JANGAN buat sub-branch per sprint/per-tiket.**

- Branch kerja saat ini: **`feat/refactoring-finalize-01`** (dikonfirmasi ada di `origin`). Semua commit Sprint 0-8 masuk ke branch ini, bukan `feat/sprint-0-xxx` atau `feat/BL-52-xxx` terpisah.
- **Catatan posisi sesi ini:** working branch sesi ini saat ini adalah `feat/refactoring-rust` (bukan `feat/refactoring-finalize-01`) — kalau `feat/refactoring-finalize-01` dimaksudkan sebagai branch kerja BARU untuk `dev-task-2/`, pastikan di-checkout secara eksplisit sebelum mulai Sprint 0, atau konfirmasi apakah `feat/refactoring-rust` akan di-rename/merge ke branch itu.
- **Pengecualian darurat:** kalau genuinely terdesak membuat sub-branch (misal eksperimen berisiko yang perlu diisolasi), WAJIB:
  1. Branch dari tip `feat/refactoring-finalize-01` terkini (bukan dari commit lama).
  2. Rebase/merge balik ke `feat/refactoring-finalize-01` SESEGERA mungkin setelah selesai — jangan biarkan sub-branch hidup lebih dari durasi satu tiket.
  3. **TIDAK ADA commit yang tertinggal** — sebelum sub-branch dihapus, jalankan `git log feat/refactoring-finalize-01..<sub-branch>` untuk pastikan nol commit unik yang belum masuk base branch.
  4. Principle memverifikasi poin 3 di checklist review sebelum approve merge manapun yang berasal dari sub-branch.

## Catatan Non-Negosiasi (dipertahankan dari dev-tasks asli)

1. Sumber kebenaran TIKET tetap `roadmap/*.md` (R-nn/G-nn/U-nn/N-nn/P-nn/D-nn) — `dev-task-2/` adalah tampilan kerja gabungan, bukan pengganti.
2. `dev-tasks/` asli TIDAK diedit oleh Lead — biarkan tim dev update sendiri; `dev-task-2/` murni referensi tambahan di sebelahnya.
3. **Wajib Menandai & Memberi Deskripsi:** Centang `[x]` di `dev-task-2/sprint-*.md` HANYA setelah PR merge + gerbang relevan lulus + Principle approve (sama seperti aturan `dev-tasks/`). **WAJIB menyertakan keterangan/deskripsi konkret penyelesaian** pada tiket tersebut (mencakup hash commit, file yang diubah/dibuat, hasil test/verifikasi, dan ringkasan solusi). Jangan biarkan tiket selesai tanpa bukti deskripsi.
4. BL yang masih **DITUNDA/EXPLORATORY** (BL-24) TIDAK dimasukkan ke sprint manapun sebagai kerja aktif — tetap tercatat di `backlog/README.md`, tunggu keputusan Owner eksplisit.
5. **Commit/PR WAJIB pakai template `feat({tiket}): {deskripsi}`** (lihat §Konvensi Commit & PR di atas) — severity MEDIUM yang baru ditambahkan, berlaku untuk commit yang BELUM di-push (tidak perlu rewrite history commit yang sudah di-push).
6. **Satu branch fitur (`feat/refactoring-finalize-01`) untuk seluruh dev-task-2** — tidak ada sub-branch per sprint/tiket kecuali darurat (lihat §Kebijakan Branch).
