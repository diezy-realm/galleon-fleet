---
title: Galleon — Tangga Tier, Pricing, dan Entitlement (FINAL, menggantikan dokumen lama)
status: final
date: 2026-10-06
author: squad-lead
supersedes:
  - standard-docs/product-research/community-tier-licensing-enforcement.md
  - standard-docs/archive/product-research-superseded/tier-enforcement-method-d-c-deep-dive.md
depends_on:
  - standard-docs/product/galleon-product-definition.md
  - standard-docs/product-research/market-strategic-report-oct2026.md
---

> **Lingkup dokumen ini:** menetapkan tangga tier final (Community/Team/Dockyard/Private Dockyard), model harga flat, mekanisme entitlement (Metode D untuk pemisahan paket + Metode P2 — token Ed25519 di modul tertutup repo terpisah — untuk Team/Dockyard), dan hasil cross-check konkret ke kode `engine/` yang menentukan urutan eksekusi nyata.
>
> **Kenapa dokumen ini men-supersede dua dokumen lama:** `community-tier-licensing-enforcement.md` (status: final, 2026-10-05) menetapkan angka Community lama (≤2 Ship, ≤3 Squad, tanpa dimensi `CrewPerShip`/`ConcurrentVoyages`/harga) yang sudah dimatangkan ulang di sesi 2026-10-06 bersama Owner + crew Product Owner (2x spawn, `5f60c799`/`05839013`). `tier-enforcement-method-d-c-deep-dive.md` menetapkan Metode D murni (tanpa token) sebagai primitif utama — ini diperluas di sesi ini karena kebutuhan **full-local/offline** (Owner, 2026-10-06) mengharuskan tier Team/Dockyard bisa dibuka di binary yang sama tanpa rebuild/redownload. Lima temuan bug arsitektur di dokumen lama (§1) **tetap berlaku dan diverifikasi ulang** di dokumen ini — tidak dibatalkan oleh angka baru.
>
> **Catatan verifikasi:** setiap klaim kode di dokumen ini dibaca langsung dari `engine/src/{fleet,ship,squad,entitlement}/` pada 2026-10-06 oleh squad-lead. Tidak ada klaim dari asumsi atau dari nama file/registry semata.

---

## Bagian 1 — Tangga Tier Final

### 1.1 Tabel (disetujui Owner, 2026-10-06)

| Dimensi | Community | Team | Dockyard | Private Dockyard |
|---|---|---|---|---|
| **Harga** | Gratis | **$15/bulan flat** | **$28/bulan flat + per seat** | Custom, by request |
| Target perangkat (acuan, bukan hard-limit) | 8 GB | 16 GB | server 32 GB | by request |
| Fleets | 1 | 1 | 1 | custom |
| Ships | 1 | 3 | 6 | custom |
| Squads total (lintas semua Ship) | 2 | 6 | 12 | custom |
| Crew per Squad | 6 | 6 | 8 | custom |
| **Crew tertugaskan per Ship** (plafon utama) | 8 | 22 | 100 | custom |
| Concurrent Voyages | 2 | 4 | 8 | custom |

### 1.2 Model Crew per Ship: slot Squad + slot langsung

`Crew tertugaskan per Ship` adalah **plafon utama**, bukan hasil kali `Squad × CrewPerSquad`. Crew di dalam satu Ship terbagi dua kelompok yang sama-sama ditarik dari plafon ini:

- **Crew dalam Squad** — anggota salah satu Squad milik Ship itu.
- **Crew langsung-di-Ship (non-Squad)** — tidak tergabung Squad apa pun, dipakai untuk peran lintas-Squad seperti Supervisor/Security (pola pengawasan middleware, bukan tipe skema baru — Crew biasa dengan `squadID` kosong dan `shipID` terisi).

Contoh Community: 1 Squad penuh (6 Crew) + 2 Crew langsung-di-Ship = 8, pas plafon. Formula:

```
CrewInSquads(shipID) + CrewDirect(shipID) = CrewPerShip (plafon)
```

Tidak ada field "jumlah slot non-Squad" tersendiri — itu residu otomatis dari `plafon - CrewInSquads`. Konsekuensi: `SquadsTotal × CrewPerSquad` boleh **lebih kecil** dari `CrewPerShip` (mis. Team: 6×6=36 kapasitas-Squad vs 3 Ship×22=66 plafon — selisihnya adalah slot Crew-langsung yang disengaja, bukan kekurangan).

### 1.3 Concurrent Voyages

**Voyage** = satu eksekusi/run dari sebuah **Quest** (SOP/workflow hidup). **Concurrent Voyages** = jumlah Voyage yang boleh berjalan bersamaan lintas seluruh Fleet — ini pembatas RAM/CPU paling langsung karena satu Voyage aktif berarti minimal satu Crew sedang memanggil LLM/tool saat itu; Crew yang terdaftar tapi idle nyaris tidak memakan resource.

Angka Community (2) sudah final sejak dokumen produk `web/docs/fundamental/06-business-and-ux.md:103` ("2 concurrent Voyages") — **terverifikasi tertulis di dokumen, TAPI nol implementasi kode**: `grep` repo-wide untuk `ConcurrentVoyage`/`VoyageLane`/`MaxConcurrent` di `engine/` nol hasil; satu-satunya kemunculan adalah contoh nilai statis di `web/docs/fundamental/07-technical-architecture.md:189` (dokumen arsitektur, bukan kode berjalan). Angka Team (4) dan Dockyard (8) adalah **usulan Lead**, diturunkan dari rasio RAM target perangkat (2x dan 4x Community), bukan dari dokumen atau kode existing.

Catatan RAM: ini plafon **produk**, bukan pengganti resource governor (BL-15) yang menyesuaikan ke RAM bebas runtime aktual. Jika mesin Dockyard sebenarnya hanya 16GB, BL-15 yang menahan, bukan angka 8 ini.

---

## Bagian 2 — Model Harga dan Siklus Langganan

| Aspek | Keputusan |
|---|---|
| Model | **Flat per tier** (bukan per-Ship/per-unit). Team selalu $15 baik dipakai untuk 1 Ship atau 3 Ship. Dockyard $28 + biaya per seat (jumlah seat belum diisi PO). |
| Scope harga | **Global**, satu harga untuk semua wilayah (bukan regional) — sesuai posisi "community base". |
| Periode default | Bulanan, bisa dibatalkan. |
| Trial | **Maksimal 14 hari**, semua fitur tier Team/Dockyard terbuka; hari ke-15 Ship/Squad/Crew di atas plafon Community diarsipkan. |
| Sidik jari trial | Email akun + UUID instalasi acak (dibuat saat instalasi pertama) — bukan MAC address (berubah di Docker/VM, mudah dipalsukan, data pribadi). |
| "Rent a Ship" | Nama kerja untuk prinsip "bangun kapasitas saat butuh, bayar berjangka" — **bukan** sewa satu Ship individual di luar tier. Diwujudkan sebagai lisensi berjangka pendek (7/30 hari) untuk upgrade sementara ke Team/Dockyard. |
| Lapse (langganan berhenti) | Tenggang 30 hari → kembali ke batas Community; entitas di atas plafon jadi arsip (tidak dihapus). Baca dan ekspor tetap terbuka (`web/docs/fundamental/06-business-and-ux.md:405` — larangan mengunci Artifact/memory di balik paywall). |
| Impor/restore | Data kelebihan di atas plafon diterima masuk sebagai arsip, tidak ditolak — mencegah bypass tanpa build ulang sekaligus menghormati kepemilikan data user. Berlaku untuk tiga jalur: restore snapshot lokal, impor file ekspor Ship, migrasi data lama. |

### 2.1 Licensing Web (komponen terpisah, ongoing)

**Status: dicatat sebagai scope ongoing, belum dirancang.** Repo terpisah dari `galleon-fleet`. Alasan pemisahan: engine lokal (`crates/`+`engine/`) adalah local-first dan tidak boleh menyimpan logika billing (melanggar boundary "UI zero backend logic" dan domain Fleet/Ship/Squad tidak mencakup billing). Pembayaran, akun, dan penerbitan lisensi harus terjadi di komponen online terpisah.

Pola alur (generate-online, verify-offline — sejalan [Keygen air-gapped activation](https://github.com/keygen-sh/air-gapped-activation-example)):
1. User bayar di Licensing Web → terima file lisensi bertanda tangan Ed25519 (plafon + expiry).
2. Import manual ke Galleon (lokal) → engine verifikasi dengan public key ter-embed, 100% offline, tidak perlu phone-home rutin.
3. Perpanjangan = import file baru (otomatis jika online, manual jika offline total).

Tidak dirancang detail (arsitektur Licensing Web, jalur pembayaran) di dokumen ini — menunggu keputusan Owner kapan diangkat jadi scope aktif.

---

## Bagian 3 — Metode Enforcement: D (pemisah paket) + P2 (token repo-terpisah)

### 3.1 Keputusan final (Q1-Q5, dibuka ulang dan dijawab 2026-10-06)

| # | Pertanyaan (dari dokumen lama) | Jawaban final |
|---|---|---|
| Q1 | Crew unlimited atau dibatasi 6/Squad? | **Dibatasi** lewat `CrewPerShip` (plafon utama: 8/22/100) — bukan murni `Squad × 6`. Crew-dalam-Squad dan Crew-langsung-di-Ship sama-sama ditarik dari plafon ini (§1.2). `MaxCrewsPerSquad` tetap ada sebagai batas per-Squad individual. |
| Q2 | Kuota interim via `SaveCollection` sebelum BG-F1, atau tunggu? | **Tidak menunggu** — dimasukkan ke `dev-task-2` sekarang (lihat §4), karena BG-F1 di sprint yang sama scope-nya memang membangun model organisasi dari nol. 5 bug arsitektur (§3.3) jadi AC wajib BG-F1, bukan tiket terpisah yang mendahuluinya. |
| Q3 | Nama tier: Pro atau Team? | **Team** (solo/profesional), **Dockyard** (multi-user/Organization), **Private Dockyard** (custom by-request). "Pro"/"Growth"/"Squad" sebagai nama tier dibuang — bentrok dengan entitas domain. |
| Q4 | Path modul Go final? | `engine/go.mod` saat ini `github.com/diezy-labs/claw-crew/engine` — **nama lama**, berbeda dari remote git (`diezy-realm/galleon-fleet`). Harus di-rename **sekarang** (bukan ditunda ke Sprint 8/RB-02), karena modul Licensing privat (Q5) akan mengimpor path ini sejak awal dibangun — menunda berarti kerja ganda. |
| Q5 | Pro-side: P1 (distribusi tanpa token) atau P2 (token di modul tertutup)? | **P2, modul tertutup di REPO BERBEDA** (bukan di dalam binary publik). Verifier Ed25519 untuk Team/Dockyard adalah bagian dari Licensing Web/module privat (§2.1), mengimpor package publik `entitlement` dari `engine` tapi private key dan logika penerbitan tidak pernah ada di repo publik. |

### 3.2 Mengapa D + P2-repo-terpisah, bukan D murni atau Ed25519-di-binary-publik

- **D murni** (dokumen lama): fitur Team/Dockyard tidak ada sama sekali di binary Community — upgrade butuh build/download modul baru. Ini **tidak cocok** untuk prinsip full-local: user offline tidak punya jalur mendapat modul baru tanpa koneksi di titik upgrade, dan trial 14-hari jadi sulit (harus swap binary).
- **Ed25519 di binary publik** (usulan interim Lead, DIKOREKSI oleh Owner): salah — Owner menegaskan Q5 = token harus di modul tertutup **repo terpisah**, bukan verifier di source publik. Ini dipertahankan.
- **Keputusan final:** binary Community (`engine/`, open source) **tidak memuat kode verifikasi Ed25519 sama sekali**. Verifier dan penerbit lisensi hidup di repo privat terpisah (Licensing module). Binary Community hanya tahu `QuotaPolicy` interface (publik) dan `CommunityPolicy` (konstanta hardcoded, publik). Saat user upgrade ke Team/Dockyard, modul privat yang terpasang (bagaimana dipasangnya — binary tambahan, plugin, atau proses terpisah — adalah detail desain Licensing module, dicatat ongoing di §2.1) menyediakan implementasi `QuotaPolicy` lain lewat Wire provider override, dan modul itulah yang memverifikasi file lisensi Ed25519.
- Residu yang diterima (konsisten dengan residu Community yang sudah Owner terima sejak awal): seseorang bisa menulis `CommunityPolicy` palsu dengan nilai besar dan build ulang binary Community sendiri — itu harga open source. Modul Licensing privat tidak bisa di-bypass dengan cara ini karena source-nya tidak publik.

### 3.3 Lima temuan bug arsitektur (diverifikasi ulang 2026-10-06, tetap berlaku)

Dibaca langsung dari `engine/src/{ship,fleet,squad,entitlement}/*.go` pada tanggal dokumen ini:

| # | Temuan | Bukti (file:baris) | Konsekuensi |
|---|---|---|---|
| 1 | `ship.Service.CreateShip` nol pemanggil di seluruh repo. | `engine/src/ship/services.go:19` (definisi), `fleet/delivery.go` tidak meng-import package `ship` | Guard kuota di `ship.Service` adalah guard dekoratif — tidak pernah dieksekusi lewat jalur HTTP nyata |
| 2 | Jalur tulis Ship/Squad yang nyata dipakai hari ini adalah `POST /api/collections/{name}` → `SaveCollection`, menerima JSON array bebas tanpa validasi kuota apa pun. | `fleet/delivery.go:40,215` (`handleCollections`) → `fleet/services.go` (`SaveCollection`, akhir file) | Guard apa pun di service lain bisa dilewati total lewat endpoint ini |
| 3 | `Fleet.ActiveShips` diisi `[]string{}` saat `CreateFleet` dan tidak pernah di-append di manapun. | `fleet/services.go` (`CreateFleet`) | Rumus `len(fleet.ActiveShips) >= max` akan selalu `0` — kuota Ship tidak akan pernah terpicu lewat field ini |
| 4 | ID Ship dibentuk `"shp_" + fleetID` — tanpa komponen unik lain. | `ship/services.go:31` (`CreateShip`) | Ship kedua pada Fleet yang sama menimpa key map yang sama; "Ship ke-2 ditolak karena limit" tidak bisa diuji sebelum ID diperbaiki |
| 5 | `squad/interfaces.go` isinya hanya `package squad` + komentar `// TODO: Define squad domain interfaces` — nol struct, nol service. | `engine/src/squad/interfaces.go` (2 baris total) | Squad belum punya representasi Go sama sekali; semua kuota Squad hari ini hanya bisa dihitung dari `countCollection("squads")` (file JSON), bukan dari domain model |

**Temuan tambahan (koreksi atas asumsi Lead sendiri di sesi ini):** `entitlement/interfaces.go` **ada** (bukan hilang), tapi isinya persis `package entitlement` + `// TODO: Define entitlement domain interfaces` (70 byte total, diverifikasi `read`) — sama kosongnya dengan `squad`. Tiket `BL-80`/`BL-81` yang dirujuk berulang kali di sintesis-sintesis sesi ini (chat sebelumnya) **tidak pernah benar-benar dibuat** — itu adalah kekeliruan penamaan oleh Lead sendiri; nomor backlog riil berikutnya yang tersedia dan belum terpakai di `standard-docs/backlog/README.md` adalah mulai **BL-80** ke atas (index sampai BL-79), jadi nomor itu kini dipakai benar-benar untuk pertama kali di §5 dokumen ini.

---

## Bagian 4 — Dampak Konkret ke `dev-task-2/` (sesuai Q2/Q4: masuk sekarang, tidak ditunda)

| Tiket | Sprint | Perubahan |
|---|---|---|
| **ENT-01** (baru) | Sprint 4 §E | `QuotaPolicy` interface + `CommunityPolicy`/`TeamPolicy`/`DockyardPolicy` di `engine/src/entitlement/`, nilai dari tabel §1.1 (bukan `{2,3,6}` versi lama). `TeamPolicy`/`DockyardPolicy` ada di package publik sebagai bentuk DEFAULT (dipakai saat tidak ada lisensi aktif/trial habis → fallback ke Community), bukan tempat logika entitlement Team/Dockyard yang sesungguhnya (itu di modul Licensing privat, §2.1/§3.2). |
| **BG-F1** (existing, AC diperluas) | Sprint 4 §E | AC tambahan WAJIB sebelum tiket ini selesai: (a) perbaiki ID Ship jadi unik; (b) tetapkan satu sumber hitung Ship/Squad (interim: `countCollection`, final: skema persisten baru); (c) tambah field `CrewIDs []string` ke `ship.Ship` untuk Crew-langsung-di-Ship; (d) definisikan `squad.Squad` dari nol (saat ini kosong total); (e) guard kuota harus menutup jalur `SaveCollection` DAN jalur service baru — bukan hanya salah satu. |
| **GO-MOD-RENAME** (baru, kecil) | Sprint 4 §E | `engine/go.mod` module path `github.com/diezy-labs/claw-crew/engine` → `github.com/diezy-realm/galleon-fleet/engine`; update seluruh import yang mereferensikannya (`fleet/services.go`, `fleet/delivery.go`, dan kemungkinan file lain — perlu grep ulang saat eksekusi). Prasyarat agar modul Licensing privat (repo lain) bisa mengimpor path yang benar sejak awal, menghindari kerja ganda saat RB-02/Sprint 8 rebrand jalan. |
| **FW-C1** (existing) | Sprint 7 §A | AC: baca `GET /api/v1/entitlement/usage` (endpoint baru, bagian ENT-01) untuk tampilkan plafon tier; tampilkan dimensi baru (`CrewPerShip`, `ConcurrentVoyages`); hapus 5 angka hardcoded tidak konsisten (`ShipyardView.tsx:42,53,58`, `FlagBridgeView.tsx:685`, `CrewView.tsx:252`); teks lewat `web/src/lib/i18n.ts`. |
| **LICENSE-MODULE** (baru, scope dicatat, TIDAK dieksekusi sekarang) | — (ongoing, luar `dev-task-2`) | Repo privat terpisah: verifier + penerbit Ed25519 untuk Team/Dockyard, Licensing Web (akun/bayar/generate file). Detail arsitektur menunggu keputusan Owner kapan diangkat jadi scope aktif. |

### Tes yang membuktikan logika benar (ponytail: non-trivial logic butuh 1 check)

1. **Tes kuota tabel-driven**: Ship ke-1 (Community) lolos, Ship ke-2 ditolak `ErrShipQuotaExceeded{limit:1,current:1}` — HARUS gagal sebelum BG-F1-(a) ID-fix diterapkan (membuktikan bug #4 nyata), lolos sesudahnya.
2. **Tes jalur ganda**: tulis Ship lewat `POST /api/collections/ships` langsung (bypass service) — harus tetap ditolak kalau guard dipasang di titik yang benar (lapisan `handleCollections`, bukan hanya `ship.Service`). Mencegah guard dekoratif berulang (bug #1-#2).
3. **Tes konkurensi** (`go test -race`): N goroutine membuat Ship bersamaan pada Fleet yang sama; hasil akhir tepat sesuai plafon tier (membuktikan cek+insert atomik, bukan TOCTOU).
4. **Tes grandfathering**: entitas di atas plafon (dari lapse/downgrade) tetap bisa dibaca dan diekspor; hanya operasi *create* baru yang ditolak.

---

## Bagian 5 — Status Pending

| Item | Siapa | Catatan |
|---|---|---|
| Biaya dasar/jumlah seat Dockyard di atas $28 flat | PO | Belum diisi — perlu dikomunikasikan eksplisit ke PO (lihat chat sesi sebelumnya) |
| Jalur pembayaran global dari basis Indonesia | Lead (riset belum mulai) | Stripe Indonesia invite-only + tidak dukung lintas-negara; perlu merchant-of-record alternatif |
| Validasi harga $15/$28 | Owner | Lewat wawancara privat/beta tertutup (rilis publik ditahan, `market-strategic-report-oct2026.md:124`) |
| Arsitektur Licensing module (repo terpisah) | Lead, menunggu keputusan Owner | Dicatat ongoing (§2.1), tidak dirancang detail di dokumen ini |
| Clearance trademark "Galleon" | Owner (konsultasi ahli IP) | Carry-over dari dokumen lama, belum berubah |
| Atribusi ZeroClaw di LICENSE-MIT/NOTICE/README | Owner | Carry-over dari dokumen lama — status "selesai 2026-10-05, belum di-commit" per `license-provenance-attribution-research.md` |

---

## Referensi

- `standard-docs/decisions/finalize/dev-task-2/sprint-4-crate-decoupling-and-runtime-strangler.md` — BG-F1 §E, lokasi ENT-01 baru
- `standard-docs/archive/dev-task-2-completed-sprints/sprint-7-product-features-and-org-model.md` — FW-C1
- `standard-docs/product-research/license-provenance-attribution-research.md` — provenance ZeroClaw, tidak berubah oleh dokumen ini
- `web/docs/fundamental/06-business-and-ux.md:94-128,405` — batas Community lama (digantikan §1), larangan paywall-lock data (tetap berlaku)
- `web/docs/fundamental/07-technical-architecture.md:189` — satu-satunya kemunculan `MaxConcurrentVoyages` di repo, nilai contoh bukan kode
- Kode diverifikasi langsung: `engine/src/ship/services.go`, `engine/src/fleet/{services,delivery}.go`, `engine/src/squad/interfaces.go`, `engine/src/entitlement/interfaces.go`, `engine/src/fleet/wire.go`, `engine/go.mod`
- Riset lisensi offline: [Keygen offline licenses](https://keygen.sh/docs/choosing-a-licensing-model/offline-licenses/), [Keygen air-gapped activation](https://github.com/keygen-sh/air-gapped-activation-example)
