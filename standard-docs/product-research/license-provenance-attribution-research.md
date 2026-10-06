---
title: Riset Provenance & Atribusi ZeroClaw — Akar Masalah, Kewajiban Lisensi, Pemulihan
status: research
date: 2026-10-05
author: squad-lead
depends_on:
  - standard-docs/product-research/community-tier-licensing-enforcement.md
---

> **Bukan nasihat hukum.** Dokumen ini membaca teks lisensi dan bukti repo. Sebelum rilis publik, minta review advokat/ahli IP. Klaim yang tidak bisa saya verifikasi ditandai *[tidak terverifikasi]*.

## 1. Hasil Singkat

| Item | Status |
|---|---|
| `LICENSE-MIT` memuat notice hak cipta ZeroClaw Labs | **Dipulihkan** (2026-10-05) |
| `NOTICE` memuat atribusi upstream + disclaimer non-afiliasi | **Dipulihkan** (ditulis ulang, lihat §4) |
| `README.md` bagian License/Credits | **Dipulihkan**; klaim "only official ClawCrew repository" dihapus |
| Temuan lanjutan di luar 3 file (CI, go.mod, Cargo.toml) | **Belum diubah**, lihat §5 |

Perubahan belum di-commit (menunggu instruksi Anda).

## 2. Akar Masalah (terverifikasi di git)

- Commit pertama repo = `05cb353f` "feat: initial release — ZeroClaw v0.1.0" (2026-02-13, penulis upstream). Repo ini adalah derivative dari upstream, bukan karya dari nol.
- Commit `97bc6eb4` (2026-09-27) "chore: rebrand zeroclaw to clawcrew" menjalankan `rebrand.py` yang melakukan `str.replace('ZeroClaw','ClawCrew')` dan varian huruf kecil/besar **ke seluruh file tanpa pengecualian** (`rebrand.py:11-14`).
- Efeknya di berkas legal (dari `git diff 97bc6eb4^ 97bc6eb4`): `LICENSE-MIT` 1 baris berubah, `NOTICE` 14 baris berubah. `LICENSE-APACHE` tidak berubah (teks Apache tidak memuat nama pemegang hak cipta).
- Jadi ini **bukan penghapusan sengaja**; ini efek samping search-and-replace. Tetap pelanggaran syarat lisensi sampai diperbaiki, apa pun niatnya.
- Baris asli sebelum rebrand: `Copyright (c) 2025 ZeroClaw Labs`. Cocok dengan upstream saat ini (`raw.githubusercontent.com/zeroclaw-labs/zeroclaw/master/LICENSE-MIT`, diambil hari ini).

## 3. Kewajiban Lisensi (dibaca langsung dari teks)

| Lisensi | Klausa | Kewajiban | Status sekarang |
|---|---|---|---|
| MIT | paragraf kondisi | Notice hak cipta + teks izin disertakan di semua salinan/porsi substansial | **Terpenuhi** setelah pemulihan |
| Apache-2.0 | §4(a) | Berikan salinan lisensi ke penerima | Terpenuhi (`LICENSE-APACHE` ada, tak berubah) |
| Apache-2.0 | §4(b) | File yang diubah ditandai perubahan | Terpenuhi via git history + pernyataan di `NOTICE`; tidak ada header per-file (lihat §5-f) |
| Apache-2.0 | §4(c) | Pertahankan notice hak cipta/atribusi dari Source | Terpenuhi setelah pemulihan |
| Apache-2.0 | §4(d) | Jika Work punya `NOTICE`, derivative wajib memuat salinan notice atribusinya | **Terpenuhi** setelah `NOTICE` ditulis ulang; "notice yang tak relevan" boleh dikecualikan |
| Apache-2.0 | §6 | Tidak ada hak memakai nama dagang Licensor, kecuali untuk menjelaskan asal karya | Terpenuhi: nama ZeroClaw hanya dipakai menyebut asal |

Catatan penting:
1. Karena dual-license `MIT OR Apache-2.0`, penerima boleh memilih salah satu. Kita wajib memenuhi **keduanya** karena kita mendistribusikan di bawah keduanya.
2. Hak cipta upstream tidak hanya milik "ZeroClaw Labs". Upstream `NOTICE` menyatakan kontributor mempertahankan hak cipta atas kontribusinya. Konvensi fork: pertahankan baris upstream apa adanya, tambahkan baris sendiri. Itu yang saya lakukan.
3. MIT/Apache **tidak** mewajibkan kode turunan tetap terbuka, dan modul baru yang bukan turunan boleh berlisensi lain (lihat riset enforcement). Yang tidak boleh: menghapus atau menyamarkan atribusi upstream.
4. Versi upstream berpindah lisensi beberapa kali (Apache → MIT → dual, lihat `git log --follow LICENSE-MIT`: `5418f66c`, `b09e77c8`, `f3dcbc99`). Repo kita mewarisi keadaan dual. Pernyataan "dual MIT OR Apache-2.0" sudah konsisten dengan upstream hari ini.

## 4. Apa yang Saya Ubah dan Mengapa

**`LICENSE-MIT`**: baris hak cipta jadi dua: `Copyright (c) 2025 ZeroClaw Labs` (dipulihkan verbatim) dan `Copyright (c) 2026 Galleon Fleet contributors` (baru, untuk modifikasi kita).

**`NOTICE`** (ditulis ulang, bukan tambal):
- Dipertahankan dari upstream: baris "includes software developed at ZeroClaw Labs (https://github.com/zeroclaw-labs)", tautan daftar kontributor upstream, pernyataan kontributor memegang hak cipta, dan bagian Verifiable Intent pihak ketiga.
- **Sengaja dibuang**: "The only official ClawCrew repository is github.com/clawcrew-labs/clawcrew", "Any other repository claiming to be ClawCrew is unauthorized", rujukan `CLA.md` dan `TRADEMARK.md`.
  - Klaim repositori resmi itu salah untuk repo ini (remote sebenarnya `github.com/diezy-realm/galleon-fleet`).
  - `CLA.md`, `TRADEMARK.md` dan `docs/book/src/contributing/cla.md` **tidak ada** di repo ini (`Test-Path` = False); rujukan ke file yang tidak ada menyesatkan.
  - Upstream sendiri tidak punya `TRADEMARK.md` di root master (HTTP 404 saat saya ambil; lokasi lain tidak saya periksa); klaim merek ZeroClaw tercantum di README upstream: "The ZeroClaw name and logo are trademarks of ZeroClaw Labs".
- Ditambahkan: asal karya, tanggal commit awal, pernyataan modifikasi, disclaimer "tidak berafiliasi/di-endorse ZeroClaw Labs", nama ZeroClaw hanya untuk menyebut asal.

**`README.md`**: bagian "Official repository & impersonation notice" dihapus (klaim tak benar); bagian License dan Credits ditulis ulang agar menyebut ZeroClaw sebagai sumber, pemegang merek, dan kredit kreator/lead upstream yang sebelumnya dihapus rebrand.

## 5. Temuan Lanjutan (TIDAK diubah; perlu keputusan Anda)

| # | Temuan | Bukti | Dampak |
|---|---|---|---|
| a | Nama "ClawCrew Labs" tidak punya dasar. Rebrand membuat entitas yang tidak saya temukan buktinya; klaim merek "ClawCrew ... trademarks of ClawCrew Labs" juga datang dari situ | `rebrand.py`; hasil `git grep` | Klaim merek oleh entitas yang mungkin tidak ada = menyesatkan. Sudah dihapus dari README/NOTICE; perlu dicek sisa di docs lain |
| b | ~40 file masih merujuk `clawcrew-labs/clawcrew` | `git grep`: `.github/workflows/*` (guard `if: github.repository == 'clawcrew-labs/clawcrew'` di `codeql.yml:45`, `docker-publish.yml:58`, `daily-audit.yml:80`), `.github/ISSUE_TEMPLATE/config.yml`, `.claude/skills/*`, `CONTRIBUTING.md`, `SECURITY.md`, `.gitmodules` | Job CI dengan guard itu **tidak akan pernah jalan** di `diezy-realm/galleon-fleet`. Ini bug fungsional, bukan hanya kosmetik |
| c | Module Go `github.com/diezy-labs/claw-crew/engine` (ketiga nama org berbeda) | `engine/go.mod:1` | Harus diputuskan sebelum modul tertutup `galleon-pro-*` bisa `require` engine (lihat riset enforcement §4) |
| d | `Cargo.toml` `repository`/`homepage` ke `clawcrew-labs/clawcrew`, `www.clawcrew.com` | `Cargo.toml:10-11` | Metadata crate menunjuk ke milik orang lain / tak ada |
| e | README masih bermerek ClawCrew (judul, badge, Discord, install `curl ... clawcrew-labs/...`) | `README.md:1-60` | Bagian Sprint 8 rebrand; badge dan skrip install menunjuk repo yang bukan milik Anda |
| f | Tidak ada header hak cipta per-file di `*.rs`/`*.go`/`*.ts` (hanya `node_modules`) | `git grep -i copyright` | Tidak wajib untuk MIT/Apache; pernyataan modifikasi di `NOTICE` + git history cukup. Tidak perlu tindakan |
| g | `NOTICE` masih memuat bagian Verifiable Intent, padahal ODR-21 memutuskan modul itu dihapus (R-38) | `ODR-2026-10-04-addendum-4.md`; `crates/clawcrew-config/src/*` masih merujuk `verifiable_intent` | Bagian itu boleh dihapus **setelah** R-38 selesai (Apache §4(d): notice yang tak relevan dikecualikan). Jangan dihapus lebih awal |
| h | Trademark "Galleon" belum di-clearance | `community-tier-licensing-enforcement.md` §2.3 | Tetap tindakan Owner + ahli IP |

## 6. Koreksi atas Dokumen `community-tier-licensing-enforcement.md`

Dua klaim di dokumen itu ternyata keliru setelah saya cek sumber eksternal (detail di `tier-enforcement-method-d-c-deep-dive.md` §2). Koreksinya sudah saya terapkan di dokumen tersebut.

## 7. Pertanyaan Terbuka untuk Owner

1. Nama hukum untuk baris `Copyright (c) 2026 ...`: apakah "Galleon Fleet contributors" cukup, atau pakai nama Anda / badan usaha?
2. Apakah ada entitas "ClawCrew Labs" yang nyata? Jika tidak, semua rujukan harus dibersihkan di sprint rebrand (§5-a,b,d,e).
3. Perbaikan guard CI (§5-b) dikerjakan sekarang (bug fungsional) atau ikut Sprint 8?

## 8. Sumber

- Internal: `git log --follow LICENSE-MIT NOTICE`; `git show 97bc6eb4^:NOTICE`; `git diff 97bc6eb4^ 97bc6eb4`; `rebrand.py`.
- Upstream (diambil 2026-10-05): LICENSE-MIT, NOTICE, README di `github.com/zeroclaw-labs/zeroclaw`.
- Teks Apache License 2.0 §4, §6 (`apache.org/licenses/LICENSE-2.0.txt`).
