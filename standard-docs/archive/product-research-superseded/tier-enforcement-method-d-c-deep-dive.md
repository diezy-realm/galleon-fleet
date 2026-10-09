---
title: Riset Mendalam — Enforcement Tier Community (Metode D sebagai primitive, C sebagai UX layer)
status: superseded
superseded_by: standard-docs/product-research/tier-pricing-and-entitlement-final.md
date: 2026-10-05
author: squad-lead
depends_on:
  - standard-docs/product-research/community-tier-licensing-enforcement.md
  - standard-docs/product-research/license-provenance-attribution-research.md
---

> **DIGANTIKAN (2026-10-06):** Q1-Q5 di Bagian 7 dokumen ini sudah dijawab final oleh Owner dan dikonsolidasikan ke `tier-pricing-and-entitlement-final.md` §3.1. Metode D murni (tanpa token) diperluas jadi D (pemisah paket) + P2 token-di-repo-terpisah, karena kebutuhan full-local/offline. **5 temuan bug arsitektur di Bagian 1 TETAP BERLAKU** — diverifikasi ulang dan dikutip utuh di dokumen baru §3.3. Dipertahankan di bawah sebagai catatan historis verifikasi.

> **Keputusan Owner (diterima):** Metode **D** = enforcement primitive, Metode **C** = UX layer, Metode **A dan B dibuang**. Dokumen ini tidak membuka ulang keputusan itu. Tujuannya: memastikan D+C benar-benar bisa ditegakkan di kode yang ada, dan menandai tempat di mana rancangan sebelumnya tidak akan bekerja.
>
> Tidak ada kode atau tiket diubah oleh dokumen ini. Semua angka kode diverifikasi langsung (Go `engine/`), klaim eksternal diverifikasi hari ini (2026-10-05) atau ditandai *[tidak terverifikasi]*.

## 1. Ringkasan Temuan (yang mengubah rancangan)

| # | Temuan | Dampak |
|---|---|---|
| 1 | **Guard `ship.Service.CreateShip` tidak menegakkan apa pun hari ini.** `ship.NewService` dan `CreateShip` tidak punya pemanggil di repo (`git grep`/`Select-String`: hanya definisinya di `engine/src/ship/services.go:19,25`). | ENT-01 seperti ditulis = guard dekoratif (pola *configuration-lies* yang sama dengan temuan sesi 2026-10-04/05) |
| 2 | **Jalur tulis Ship/Squad yang nyata = `POST /api/collections/{name}`** (`fleet/delivery.go:40,215` → `fleet/services.go:480 SaveCollection`) yang menimpa seluruh array JSON (`ships`, `squads`) dengan isi bebas. | Chokepoint kuota harus di sini (interim) atau di model BG-F1 (final). Guard di service lain bisa dilewati lewat endpoint ini |
| 3 | `Fleet.ActiveShips` **tidak pernah diisi** (hanya `[]string{}` di `fleet/services.go:125`). Rumus di dokumen lama `len(fleet.ActiveShips) >= max` akan selalu `0`. | Kuota Ship tidak akan pernah terpicu. Sumber hitung harus ditetapkan dulu (lihat §4.2) |
| 4 | Tiga "sumber kebenaran" jumlah Ship/Squad bersaing: `Fleet.ActiveShips`, `ship.Service` (in-memory, mati), dan koleksi JSON `ships`/`squads` yang dihitung `countCollection` (`fleet/services.go:152,158`). | Pelanggaran SSOT (AGENTS.md). BG-F1 wajib menetapkan satu |
| 5 | `ship.CreateShip` membuat ID `"shp_"+fleetID`: dua Ship pada satu Fleet **saling menimpa** (`ship/services.go:31`). | Batas "2 Ship" tak bisa diuji sebelum ID diperbaiki |
| 6 | `entitlement.json` + HMAC di §3.3 dokumen lama = **Metode A**, yang sudah dibuang. | Dihapus dari rancangan; Community tidak butuh file entitlement (§4.3) |
| 7 | Dua rujukan preseden di dokumen lama keliru: **Sentry FSL** dan **Supabase** bukan capability-gating (§2). | Dikoreksi; preseden D yang valid lebih sempit |
| 8 | Modul Go `github.com/diezy-labs/claw-crew/engine` (`engine/go.mod:1`) ≠ remote `diezy-realm/galleon-fleet`. | Modul tertutup `galleon-pro-*` tidak bisa `require` engine sebelum path final diputuskan |
| 9 | Spesifikasi berbenturan: "Crew **unlimited**" vs "1 Squad maks **6** Crew". | Perlu keputusan Owner sebelum ODR (§7-Q1) |

## 2. Verifikasi Eksternal (2026-10-05)

| Klaim di dokumen lama | Hasil verifikasi | Status |
|---|---|---|
| GitLab CE/EE = capability gating via modul tertutup | **Sebagian benar.** CE (MIT) tidak memuat kode `ee/`. Tapi EE *source-available* di repo yang sama (`ee/`, lisensi proprietary) dan diaktifkan dengan **license file** (`.gitlab-license`). GitLab sendiri menulis: *"We don't obfuscate the EE codebase... nothing stops evil users to grep the codebase for `license` and modify the code"* ([gitlab-org/gitlab#2952](https://gitlab.com/gitlab-org/gitlab/-/issues/2952)). | Dikoreksi: GitLab = D di batas paket CE + lisensi hukum + license key di sisi EE |
| Sentry FSL memakai pola D | **Salah.** FSL = lisensi *source-available non-compete* yang berubah ke Apache-2.0/MIT setelah 2 tahun ([InfoQ](https://www.infoq.com/news/2023/12/functional-source-license), [sentry.io/open/licensing](https://open.sentry.io/licensing/)). Sentry eksplisit menolak model open-core ("no open-core model"). Itu enforcement hukum, bukan capability gating | **Dihapus sebagai preseden D** |
| Supabase memakai pola D | *[tidak terverifikasi di sesi ini]*; sepengetahuan saya Supabase memonetisasi layanan hosted, bukan gating fitur. Jangan dijadikan preseden sampai dicek | Dihapus |
| Obsidian: lisensi komersial opsional | Benar, tetapi tanggalnya **Februari 2025**, bukan 2026 ([obsidian.md/blog/free-for-work](https://obsidian.md/blog/free-for-work/)). Obsidian itu closed-source: tidak bisa di-fork, jadi bukan analog langsung | Dikoreksi tanggal; jangan dipakai sebagai bukti untuk repo MIT |
| Home Assistant Cloud $6,50/bln | Harga itu berlaku sejak Feb 2022 ([nabucasa.com](https://www.nabucasa.com/more-info/subscriptions-feb-22/)). Per 2026-10-02 layanan diganti nama **Home Assistant Link**, halaman produk kini menampilkan **$65,00/tahun** ([nabucasa.com/link](https://www.nabucasa.com/link/), [home-assistant.io blog 2026-10-02](https://www.home-assistant.io/blog/2026/10/02/big-tech-ruined-the-cloud-so-were-renaming-ours/)). "Ratusan ribu pelanggan" = *[tidak terverifikasi]* | Dikoreksi harga; jumlah pelanggan jangan dikutip |
| n8n (tidak disebut sebelumnya) | File ber-`.ee.`/direktori `.ee` ada di repo publik tetapi **di luar** Sustainable Use License; perlu Enterprise License untuk memakainya ([n8n LICENSE.md](https://github.com/n8n-io/n8n/blob/master/LICENSE.md)) | Pola *source-available + lisensi hukum*: alternatif yang **tidak** kita pilih; relevan hanya untuk kode baru non-turunan |

Kesimpulan eksternal: **tidak ada preseden yang membuktikan bahwa gating teknis mencegah bypass di source terbuka.** Yang terbukti: (1) pemisahan paket (CE tanpa kode EE), (2) lisensi hukum, (3) nilai layanan (Home Assistant Link = remote access + backup, yang butuh server mereka). D melindungi *paket Community*, bukan nilai bisnis.

## 3. Apa yang Sebenarnya Dijamin D

- **Dijamin:** binary Community buatan kita tidak memuat kode Pro, tidak ada `LicensedPolicy` untuk "dihidupkan". Tidak ada `if licensed {}` yang bisa di-comment-out.
- **Tidak dijamin:** siapa pun bisa mengubah `CommunityPolicy{2,3,6}` → `{999,999,999}` dan build ulang. Ini residu yang **sudah diterima Owner**. Basis Rust/Go turunan ZeroClaw berlisensi MIT/Apache: kita tidak punya dasar hukum melarangnya (lihat `license-provenance-attribution-research.md` §3).
- **Pelindung sebenarnya di luar kode:** (a) merek "Galleon" — fork harus berganti nama (perlu clearance, bukan sekadar klaim), (b) EULA per organisasi untuk modul Pro, (c) nilai yang tak bisa di-fork (Relay, Pack terkurasi, dukungan).

### 3.1 Celah logis "buang B": sisi Pro

D hanya menjawab "apa isi binary Community". Tapi binary/modul **Pro** harus tahu *siapa yang berhak*, kapan langganan habis, dan bagaimana membatalkan. Pilihan untuk Pro (keputusan nanti, bukan sekarang; YAGNI sampai ada pelanggan):

| Opsi | Cara | Catatan |
|---|---|---|
| P1 | Distribusi modul Pro hanya via registry/akun berbayar; tanpa token runtime | Sederhana; tidak bisa kedaluwarsa/dicabut; kebocoran binary hanya bisa dilawan hukum |
| P2 | Token bertanda tangan (verifikasi offline) **di dalam modul tertutup** | Ini bukan Metode B di source terbuka: kode verifikasinya tidak ada di repo publik, jadi tak bisa di-grep dan di-comment-out. Inilah yang GitLab lakukan di sisi EE |

Rekomendasi: tetapkan **P2 hanya sebagai detail internal `galleon-pro-*`** saat modul itu dibangun. "Buang B" berlaku penuh untuk Community; saya tidak melihat konflik dengan keputusan Anda, tapi ini harus tercatat eksplisit di ODR agar tidak dibaca sebagai "Pro tak pernah butuh token".

## 4. Rancangan D yang Bisa Ditegakkan

### 4.1 Antarmuka (di repo publik, `engine/src/entitlement/`)

```go
// QuotaPolicy menyatakan batas per Fleet; nil = tanpa batas.
type QuotaPolicy interface {
    MaxShips() *int
    MaxSquadsTotal() *int // total lintas semua Ship, bukan per-Ship
    MaxCrewsPerSquad() *int
}
```

- `CommunityPolicy` = konstanta kompilasi `{2, 3, 6}` (atau `{2,3,nil}` jika Q1 dijawab "unlimited").
- `entitlement.Set` (Wire provider set) menyediakan `QuotaPolicy`. `engine/app/wire.go` sudah memakai pola `*.Set` (`fleet.Set`, `crew.Set`, ...), jadi tambahan `entitlement.Set` cocok dengan konvensi.
- Pro: modul privat `galleon-pro-*` memiliki `main`/komposisi sendiri yang mengimpor paket engine publik dan mengganti provider `QuotaPolicy`. Syarat teknis: tidak ada direktori `internal/` di `engine/` (saya cek: tidak ada) dan path modul final (§1-#8).

### 4.2 Titik penegakan dan sumber hitung (SSOT)

Aturan yang di-enforce: `totalShips ≤ 2` **dan** `totalSquads ≤ 3` (dua integer independen; komposisi `{1+2}+{1+1}` hanyalah contoh valid).

1. **Tetapkan satu sumber hitung dulu** — bagian dari BG-F1 (model organisasi persisten). Guard membaca jumlah dari sumber itu saat dipakai (AGENTS.md: *resolve from canonical source, jangan snapshot*).
2. **Guard di agregat yang memiliki data**, bukan di service terpisah. Hitung Ship dan Squad dari dua service berbeda dengan lock berbeda menimbulkan TOCTOU (dua request bersamaan sama-sama lolos cek). Cek + insert harus atomik: di bawah satu lock (`fleetService.mu` sudah ada, `fleet/services.go:27`) atau satu transaksi SQLite (setelah BG-B2).
3. **Interim (sebelum BG-F1 selesai)**: jika ingin kuota efektif lebih awal, satu-satunya chokepoint adalah `SaveCollection` untuk nama `ships`/`squads` (hitung panjang array JSON sebelum `os.WriteFile`). Ini sementara dan harus dihapus saat BG-F1 mengganti penulisan koleksi. Saya **tidak** merekomendasikan interim ini kecuali Owner butuh kuota aktif sebelum Sprint 4 §E; risikonya: menegakkan aturan pada bentuk data sementara.
4. Perbaiki ID Ship (`"shp_"+fleetID` → ID unik) sebagai bagian BG-F1; tanpa itu tes "Ship ke-3 ditolak" tak bermakna.
5. Guard harus di **service layer** supaya HTTP/CLI/gRPC otomatis tercakup (keputusan Lead sebelumnya tetap benar), tetapi hanya efektif jika service itu memang jalur tulis tunggal (temuan #2).

### 4.3 Tidak perlu file entitlement untuk Community

Kuota Community = konstanta kompilasi. `entitlement.json` + HMAC (dokumen lama §3.3) tidak memberi apa pun di Community dan merupakan Metode A. **Hapus dari rancangan.** File tier/token baru dibahas hanya saat modul Pro ada (§3.1-P2).

### 4.4 Semantik penurunan tier dan data pengguna

Prinsip produk: data, memori, artifact milik user dan portabel. Maka kuota **memblokir pembuatan**, tidak pernah menghapus/menyembunyikan/mengunci data yang sudah ada. Pengguna yang melebihi batas (mis. dari build Pro lama atau koleksi hasil edit) tetap bisa membaca, mengekspor, dan menjalankan; hanya *create* baru yang ditolak. Hitung hanya entitas aktif (bukan yang diarsipkan), dan tetapkan definisi "aktif" di BG-F1.

### 4.5 Rust

Tidak ada kuota Ship/Squad di Rust (data organisasi ada di Go). Jangan membuat salinan kuota di Rust (SSOT). Catatan nama: `ClientSessionQuota` di `crates/clawcrew-runtime/src/rpc/wss.rs:184` adalah admission koneksi WSS, tidak berhubungan dengan tier; beri nama berbeda agar tidak tertukar.

## 5. Lapisan C (UX)

Tujuan C: pengguna jujur memahami batas tanpa ditipu atau dinag. Server tetap satu-satunya penegak; UI hanya menjelaskan.

| Komponen | Spesifikasi |
|---|---|
| Kontrak error | `403` + `{"error":"quota_exceeded","resource":"ship\|squad\|crew","limit":N,"current":N}`. *(Rekomendasi saya; tidak diverifikasi ke standar. `402` saya hindari karena semantik "Payment Required" belum terstandarisasi di HTTP dan tidak cocok untuk tier gratis.)* `upgrade_url` dihapus dari kontrak engine: URL harga adalah konfigurasi UI, bukan urusan engine; engine tidak perlu tahu halaman harga |
| Endpoint baca penggunaan | `GET /api/v1/entitlement/usage` → `{ships:{limit,current}, squads:{...}, crewPerSquad:{limit}}` dibaca UI saat render. **Satu sumber angka**: menggantikan 5 nilai tak konsisten yang hardcoded: `ShipyardView.tsx:42` ("1 Active Ship / 6 Crew Berths"), `:53` ("1 of 3 Ships"), `:58` ("15 Berths"), `FlagBridgeView.tsx:685` ("Pro ... 5 Ships"), `CrewView.tsx:252` ("/ 15 Berths") |
| Tahap UX | (1) di bawah batas: meter netral, tanpa prompt; (2) di batas: tombol *Create* dinonaktifkan dengan penjelasan ("2 of 2 Ships in use") dan tautan ke halaman paket; (3) upaya tetap mengirim: tampilkan error terstruktur, bukan toast generik |
| Larangan | Tidak ada pop-up berulang, timer, atau degradasi kinerja sengaja (itu dark pattern dan merusak klaim "milik user"). Tidak ada phone-home untuk menghitung; telemetri hanya opt-in |
| i18n | Teks lewat `web/src/lib/i18n.ts` (AGENTS.md), bukan literal |
| Istilah tier | Dokumen lama memakai "Team"; `FlagBridgeView.tsx:685` dan `web/docs/fundamental/06` memakai "Pro". Satu istilah harus dipilih (§7-Q3) sebelum string UI dibuat |

## 6. Rencana Verifikasi (tes yang gagal bila logika rusak)

1. **Tes kuota**: tabel-driven; buat Ship ke-1, ke-2 lolos, ke-3 → `ErrQuotaExceeded{resource:"ship",limit:2,current:2}`; Squad ke-4 ditolak walau tersebar di Ship berbeda.
2. **Tes konkurensi** (`go test -race`): N goroutine membuat Ship bersamaan; hasil akhir tepat 2 (membuktikan cek+insert atomik).
3. **Tes jalur**: tulis Ship/Squad lewat *setiap* jalur tulis yang ada (HTTP collection, service, gRPC jika ada) dan buktikan semuanya menolak. Mencegah guard dekoratif (temuan #1).
4. **Tes ketiadaan (absence test)** untuk klaim D: CI memeriksa bahwa build Community tidak memuat simbol/paket Pro (`go list -deps` + `grep` pada binary), supaya klaim "tidak ada kode untuk dibuka" tetap benar seiring waktu.
5. **Tes grandfathering**: dataset melebihi batas tetap bisa dibaca/diekspor; hanya *create* yang ditolak.

## 7. Pertanyaan Terbuka untuk Owner (menghalangi ODR)

| # | Pertanyaan | Mengapa penting |
|---|---|---|
| Q1 | "Crew unlimited" atau "1 Squad maks 6 Crew"? Dua-duanya tidak bisa benar bersamaan: 6 per Squad × 3 Squad = maks 18 Crew. Jika "6 per Squad", maka Crew **dibatasi** (lewat Squad), dan `MaxCrewsPerSquad` perlu guard di tempat Crew dibuat. Jika "unlimited", hapus `MaxCrewsPerSquad` | Menentukan bentuk `QuotaPolicy` dan AC ENT-01 |
| Q2 | Kuota interim via `SaveCollection` sebelum BG-F1 (§4.2-3): dipakai atau ditunggu BG-F1? | Menentukan apakah ada enforcement nyata sebelum Sprint 4 §E |
| Q3 | Nama tier berbayar: **Pro** atau **Team**? | Dokumen fundamental/UI = Pro; dokumen enforcement = Team |
| Q4 | Path modul Go final (`github.com/<org>/galleon-fleet/engine`)? | Prasyarat modul Pro tertutup; terkait Sprint 8 rebrand |
| Q5 | Pro-side: P1 (distribusi berbayar) atau P2 (token di modul tertutup)? | Boleh ditunda; cukup dicatat sebagai keputusan tertunda di ODR |

## 8. Masukan untuk ODR (belum ditulis)

ODR bersifat *immutable* (`ODR-2026-10-04-product-direction.md`: perubahan = ODR baru yang men-*supersede*). Karena itu saya tidak menulisnya sampai Q1 dan Q3 dijawab. Isi yang sudah siap:

- **Keputusan 1 (siap):** Metode D = enforcement primitive; C = UX layer; A dan B dibuang untuk Community. Pro-side ditunda (Q5).
- **Keputusan 2 (siap):** Batas Community = `totalShips ≤ 2`, `totalSquads ≤ 3` (total lintas Ship). Crew per Squad = menunggu Q1.
- **Keputusan 3 (siap):** Kuota memblokir *create*, tidak pernah menghapus/menyembunyikan data (§4.4).
- **Keputusan 4 (siap):** Satu sumber hitung ditetapkan di BG-F1; tidak ada kuota di Rust.
- **Mensupersede:** teks "Community = 1 Ship aktif, ≤5 Crew, 2 Voyage, delegation depth 1" di `.kiro/steering/galleon-product-fundamentals.md:43`. Dokumen yang ikut selaras: `web/docs/fundamental/03-operating-model.md:213` ("one active Squad per Ship"), `05-collaboration-and-memory.md:111` (depth 1), `06-business-and-ux.md:85,94`, serta 5 angka hardcoded UI di §5.
- **Nomor:** ODR terakhir = ODR-25 (`ODR-2026-10-04-addendum-4.md`); usulan berkas `ODR-2026-10-05-community-tier-and-license-provenance.md`, ID mulai ODR-26.
- **Tambahan legal (dari riset provenance):** keputusan memulihkan atribusi ZeroClaw dan larangan klaim merek ZeroClaw/ClawCrew.

## 9. Dampak ke Tiket (usulan, belum diterapkan)

| Tiket | Perubahan usulan |
|---|---|
| ENT-01 (Sprint 4 §E) | AC ditambah: guard di jalur tulis yang nyata; tes jalur (§6-3); tes konkurensi; tidak ada `entitlement.json`/HMAC |
| BG-F1 (Sprint 4 §E) | AC ditambah: tetapkan satu sumber hitung Ship/Squad; perbaiki ID Ship; ganti `SaveCollection` untuk `ships`/`squads`; definisi "aktif" |
| FW-C1 (Sprint 7 §A) | AC: baca `GET /api/v1/entitlement/usage`; hapus 5 angka hardcoded; teks via i18n |
| Sprint 8 rebrand | Tambah: path modul Go final; CI guard `clawcrew-labs/clawcrew` (lihat riset provenance §5-b) |
| CI (baru, kecil) | Absence test build Community (§6-4) |

## 10. Sumber

- Internal: `engine/src/ship/services.go`, `engine/src/fleet/{interfaces,services,delivery}.go`, `engine/src/entitlement/interfaces.go`, `engine/src/squad/interfaces.go`, `engine/app/wire.go`, `engine/go.mod`, ODR-2026-10-04 (+addendum-4), `.kiro/steering/galleon-product-fundamentals.md`, `web/src/components/features/{ShipyardView,FlagBridgeView,CrewView}.tsx`, `web/docs/fundamental/*`.
- Eksternal: GitLab ([single codebase](https://about.gitlab.com/blog/a-single-codebase-for-gitlab-community-and-enterprise-edition/), [licensing](https://docs.gitlab.com/development/licensing/), [#2952](https://gitlab.com/gitlab-org/gitlab/-/issues/2952)); Sentry ([FSL](https://blog.sentry.io/introducing-the-functional-source-license-freedom-without-free-riding), [InfoQ](https://www.infoq.com/news/2023/12/functional-source-license)); Obsidian ([free for work](https://obsidian.md/blog/free-for-work/)); Nabu Casa ([Link](https://www.nabucasa.com/link/)); n8n ([LICENSE.md](https://github.com/n8n-io/n8n/blob/master/LICENSE.md)).
