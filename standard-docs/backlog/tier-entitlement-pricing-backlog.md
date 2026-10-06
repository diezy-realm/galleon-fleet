---
title: Backlog Tier Entitlement & Pricing (BL-80 s.d. BL-82)
status: active
date: 2026-10-06
author: squad-lead
source: standard-docs/product-research/tier-pricing-and-entitlement-final.md
---

# Backlog Tier Entitlement & Pricing (BL-80 s.d. BL-82)

> Diturunkan dari `product-research/tier-pricing-and-entitlement-final.md` (sesi 2026-10-06, menggantikan `community-tier-licensing-enforcement.md` + `tier-enforcement-method-d-c-deep-dive.md`). Semua klaim kode di tiket ini diverifikasi langsung ke `engine/src/{ship,fleet,squad,entitlement}/*.go` pada 2026-10-06 — lihat riset §3.3 untuk bukti file:baris. Dipetakan ke `dev-task-2/sprint-4-crate-decoupling-and-runtime-strangler.md` §E (lihat riset §4) — BUKAN tiket yang menunggu BG-F1 selesai dulu, karena scope BG-F1 di sprint yang sama memang membangun model organisasi dari nol.

| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-80 | `QuotaPolicy` interface + `CommunityPolicy` di `entitlement/` (angka final: 1 Ship/2 Squad/8 CrewPerShip/2 Voyage) | MEDIUM | Aktif — siap eksekusi, nol dependency kode (package `entitlement/` sudah ada, isinya hanya TODO) |
| BL-81 | Perbaiki 5 bug arsitektur yang membuat guard kuota apa pun dekoratif (ID Ship kolisi, jalur tulis ganda, `ActiveShips` mati, 3-sumber-kebenaran, `squad.Squad` kosong) | **HIGH** (prasyarat BL-80 bermakna) | Aktif — PRIORITASKAN sebelum BL-80 AC "guard menolak" dianggap lolos; masuk sebagai AC tambahan BG-F1 |
| BL-82 | `engine/go.mod` module path rename `claw-crew`→`galleon-fleet` sebelum modul Licensing privat mulai mengimpornya | LOW (murah, tapi urutan penting) | Aktif — kerjakan sebelum modul Licensing (repo terpisah) dibangun, untuk hindari kerja ganda saat RB-02/Sprint 8 |

---

## BL-80 — `QuotaPolicy` interface + `CommunityPolicy`

**Severity:** MEDIUM | **Size:** S | **Sprint:** 4 §E, bersama BG-F1

**Masalah:** `engine/src/entitlement/interfaces.go` saat ini (`read` diverifikasi 2026-10-06) berisi persis:
```go
package entitlement

// TODO: Define entitlement domain interfaces
```
Nol interface, nol implementasi. Tidak ada dimensi kuota apa pun di kode — angka tier yang disepakati Owner (lihat tabel di bawah) sepenuhnya belum berbadan kode.

**Scope & file ownership:** `engine/src/entitlement/interfaces.go` (baru), `engine/src/entitlement/service.go` (baru), `engine/src/entitlement/wire.go` (baru, pola `var Set = wire.NewSet(...)` konsisten dengan `fleet.Set`/`ship` — lihat `engine/src/fleet/wire.go`). Backend-Go. Tidak overlap file dengan BL-81/BL-82.

**Input siap:** tabel tier final (`tier-pricing-and-entitlement-final.md` §1.1):

| Dimensi | Community | Team | Dockyard |
|---|---|---|---|
| Fleets | 1 | 1 | 1 |
| Ships | 1 | 3 | 6 |
| Squads total | 2 | 6 | 12 |
| Crew per Squad | 6 | 6 | 8 |
| Crew per Ship (plafon utama) | 8 | 22 | 100 |
| Concurrent Voyages | 2 | 4 | 8 |
| **Max Delegation Depth** (baru, 2026-10-06) | **1** | **2** | **3** |

> **AC TAMBAHAN (2026-10-06, keputusan Owner):** Dimensi `MaxDelegationDepth` ditambahkan setelah analisa pola KiroCrew (platform agen sesi ini membatasi rantai delegasi Crew→Crew via `max_delegation_depth` statis per preset, `clawcrew-config/src/presets.rs:238`) — `galleon-fleet` perlu padanan yang sama, karena model Crew-tersubordinasi-juga-bisa-spawn (dikonfirmasi Owner) berarti rantai delegasi tak terbatas adalah vektor resource-exhaustion/privilege-chain yang tidak bisa diaudit tanpa batas kedalaman. Ini terpisah dari BL-05 (yang menyatukan 2 jalur delegasi + memasang depth-check) — ENT-01 hanya menyediakan NILAI-nya, BL-05 yang menegakkannya.

**Acceptance criteria:**
1. Interface `QuotaPolicy` dengan method: `MaxFleets()`, `MaxShips()`, `MaxSquadsTotal()`, `MaxCrewsPerSquad()`, `MaxCrewsPerShip()`, `MaxConcurrentVoyages()`, **`MaxDelegationDepth()`** — semua `*int` (nil = unlimited, untuk Private Dockyard custom).
2. `CommunityPolicy` konstanta hardcoded `{1,1,2,6,8,2,1}`, publik (bisa diimpor modul Licensing nanti sebagai nilai fallback saat lisensi Team/Dockyard tidak aktif/trial habis).
3. `TeamPolicy`/`DockyardPolicy` struct ADA di package publik dengan nilai dari tabel di atas (`{1,3,6,6,22,4,2}` / `{1,6,12,8,100,8,3}`) — tapi ini bentuk DEFAULT/fallback-saat-aktif, BUKAN tempat logika verifikasi lisensi (itu di modul Licensing privat, lihat BL-83/scope terpisah di luar backlog ini).
4. `entitlement.Set` (Wire provider) menyediakan `QuotaPolicy` lewat `NewService`, pola sama dengan `fleet.Set`.
5. Endpoint baru `GET /api/v1/entitlement/usage` → `{fleets:{limit,current}, ships:{...}, squadsTotal:{...}, crewPerSquad:{limit}, crewPerShip:{...}, concurrentVoyages:{limit,current}, delegationDepth:{limit}}`. Satu sumber angka untuk FW-C1 (menggantikan 5 nilai hardcoded tidak konsisten di UI).
6. Unit test tabel-driven untuk tiap method ketiga policy, termasuk `MaxDelegationDepth()`.
7. **BLOCKED oleh BL-81** untuk AC "guard menolak create ke-N+1" — lihat BL-81 kenapa guard di `ship.Service`/`squad.Service` murni akan dekoratif tanpa perbaikan jalur tulis dulu.
8. **Konsumen `MaxDelegationDepth()`: BL-05** (`skills-steering-lessons-backlog.md`) — ENT-01 hanya menyediakan nilai, tidak menegakkan sendiri (tidak ada logic depth-counting di package `entitlement`).

**Definition of done:** `go test ./engine/src/entitlement/...` hijau; `GET /api/v1/entitlement/usage` mengembalikan angka benar untuk tier Community (termasuk `delegationDepth.limit=1`); code review Lead (clean code, SOLID — interface kecil, tidak over-abstraksi untuk Private Dockyard yang custom).

---

## BL-81 — Perbaiki 5 bug arsitektur (prasyarat guard kuota bermakna)

**Severity:** HIGH | **Size:** M | **Sprint:** 4 §E, AC tambahan BG-F1 (bukan tiket berdiri sendiri — menyatu dengan scope BG-F1 "model organisasi persisten")

**Masalah (diverifikasi langsung ke kode, 2026-10-06):**

| # | Bug | File:bukti | Dampak jika tidak diperbaiki |
|---|---|---|---|
| 1 | `ship.Service.CreateShip` nol pemanggil di seluruh repo | `engine/src/ship/services.go:19`; `fleet/delivery.go` tidak `import` package `ship` | Guard kuota di `ship.Service` tidak pernah tereksekusi lewat jalur HTTP nyata — dekoratif |
| 2 | Jalur tulis Ship/Squad nyata = `POST /api/collections/{name}` → `SaveCollection`, menerima JSON array bebas | `fleet/delivery.go` (`handleCollections`) → `fleet/services.go` (`SaveCollection`) | Guard apapun di service lain dilewati total via endpoint ini |
| 3 | `Fleet.ActiveShips` diisi `[]string{}` saat `CreateFleet`, tidak pernah di-append | `fleet/services.go` (`CreateFleet`) | `len(fleet.ActiveShips) >= max` selalu `0` — kuota Ship tidak pernah terpicu lewat field ini |
| 4 | ID Ship = `"shp_" + fleetID`, tanpa komponen unik lain | `ship/services.go:31` | Ship ke-2 pada Fleet yang sama menimpa key map yang sama; tes "Ship ke-2 ditolak" tidak bermakna sebelum ini diperbaiki |
| 5 | `squad/interfaces.go` isinya `package squad` + `// TODO: Define squad domain interfaces` (2 baris total) | `engine/src/squad/interfaces.go` | Nol representasi domain Squad; kuota Squad hari ini hanya bisa dihitung dari `countCollection("squads")` (file JSON mentah), bukan model |

**Scope & file ownership:** `engine/src/ship/services.go` (perbaiki ID generation + sambung ke jalur tulis nyata ATAU hapus kalau digantikan model BG-F1), `engine/src/fleet/services.go` (ganti `ActiveShips` jadi sumber hitung yang benar — `countCollection` interim atau skema persisten final), `engine/src/squad/interfaces.go` (definisikan `Squad` struct + `Service` dari nol), `engine/src/fleet/delivery.go` (titik guard kuota di `handleCollections` untuk `ships`/`squads`, interim sampai jalur tulis final BG-F1 menggantikannya). Backend-Go. **Serialkan dengan BL-80**: BL-81 harus menyediakan sumber hitung yang benar sebelum BL-80 AC-7 (guard menolak) bisa diuji lolos.

**Acceptance criteria:**
1. ID Ship unik (bukan `"shp_"+fleetID`) — mis. `"shp_"+fleetID+"_"+ulid()` atau counter atomik per-Fleet.
2. Satu sumber hitung Ship/Squad ditetapkan dan didokumentasikan (interim: `countCollection("ships")`/`countCollection("squads")`, yang SUDAH nyata dipakai `GetMetrics` — jangan pakai `Fleet.ActiveShips` yang mati).
3. `squad.Squad` struct baru: minimal `{ID, ShipID, CrewIDs []string}`, konsisten pola `ship.Ship`.
4. `ship.Ship` tambah field `CrewIDs []string` untuk menampung Crew-langsung-di-Ship (non-Squad) — lihat model §1.2 riset.
5. Guard kuota (dari `entitlement.QuotaPolicy`, BL-80) dipasang DI `handleCollections` untuk nama koleksi `ships`/`squads`/`crew` — bukan hanya di `ship.Service`/`squad.Service` yang jalur nyatanya belum tersambung.
6. Cek+insert atomik di bawah lock yang sama (`fleetService.mu` sudah ada) — tidak ada TOCTOU dua request bersamaan sama-sama lolos.
7. Entitas di atas plafon (dari lapse/downgrade/impor) tetap bisa dibaca & diekspor; hanya *create* baru yang ditolak (grandfathering, §2 riset).

**Definition of done:** 4 tes di riset §4 (Bagian "Tes yang membuktikan logika benar") hijau: tes kuota tabel-driven, tes jalur ganda (bypass via `/api/collections` langsung tetap ditolak), tes konkurensi `-race`, tes grandfathering. Code review Lead — verifikasi independen bahwa guard benar-benar di titik tulis nyata, bukan di service yang terbukti tidak terpanggil.

---

## BL-82 — Rename module path Go sebelum modul Licensing privat dibangun

**Severity:** LOW (murah) tapi **urutan penting** (sebelum Licensing module mulai, bukan setelah) | **Size:** S | **Sprint:** 4 §E

**Masalah:** `engine/go.mod` saat ini:
```
module github.com/diezy-labs/claw-crew/engine
```
Nama organisasi dan repo (`diezy-labs/claw-crew`) sudah usang — remote git repo ini sekarang `github.com/diezy-realm/galleon-fleet` (dikonfirmasi lesson learned sebelumnya: "Remote is now https://github.com/diezy-realm/galleon-fleet.git"). Modul Licensing privat (repo terpisah, scope BL-83/di luar backlog ini — lihat §2.1/§3.2 riset) akan mengimpor package publik `entitlement` dari `engine` ini sejak modul itu mulai dibangun. Menunda rename ke Sprint 8 (RB-02) berarti modul privat harus di-refactor import path-nya begitu rebrand jalan — kerja ganda yang bisa dihindari dengan urutan yang benar.

**Scope & file ownership:** `engine/go.mod` (module path), seluruh file yang mengimpor path lama — minimal terkonfirmasi: `engine/src/fleet/services.go`, `engine/src/fleet/delivery.go` (via `github.com/diezy-labs/claw-crew/engine/pkg/{client,pb}` dan `.../core/metrics`); grep ulang wajib saat eksekusi karena daftar ini belum lengkap (hanya 2 file yang dibaca penuh sesi ini). Backend-Go, isolated change (satu commit mekanis, bukan refactor logika).

**Acceptance criteria:**
1. `module github.com/diezy-realm/galleon-fleet/engine` di `go.mod`.
2. Seluruh `import "github.com/diezy-labs/claw-crew/engine/..."` diganti ke path baru — verifikasi dengan `go build ./...` hijau (bukan grep manual yang bisa miss).
3. Tidak ada perubahan logika — murni mekanis (ponytail: byte-level find-replace per lesson "pure byte-copy untuk perubahan mekanis", bukan retype manual).

**Definition of done:** `go build ./engine/...` dan `go vet ./engine/...` hijau; `git grep "diezy-labs/claw-crew"` di `engine/` nol hasil.

---

## Catatan Non-Negosiasi

1. **BL-80/81/82 TIDAK menunggu BG-F1 "selesai" dulu** — BL-81 (prasyarat 5 bug) adalah bagian scope BG-F1 itu sendiri ("model organisasi persisten dari nol"), bukan dependency eksternal yang mendahuluinya. Keputusan Owner 2026-10-06 (Q2 riset): masuk `dev-task-2` sekarang.
2. **Modul Licensing (verifier Ed25519 Team/Dockyard, repo privat terpisah)** TIDAK masuk backlog ini — dicatat scope "ongoing" di riset §2.1/§5, menunggu keputusan Owner kapan diangkat jadi tiket aktif dengan nomor sendiri.
3. **Jangan bangun logika verifikasi Ed25519 di package publik `entitlement`** — `TeamPolicy`/`DockyardPolicy` (BL-80 AC-3) hanya nilai fallback, bukan tempat cek signature/expiry. Itu melanggar keputusan Q5 (P2, token di repo berbeda).

## Lihat Juga

- `standard-docs/product-research/tier-pricing-and-entitlement-final.md` — riset lengkap, sumber tiket ini.
- `standard-docs/backlog/README.md` — index backlog BL-01 s.d. BL-79; dokumen ini memperkenalkan BL-80 s.d. BL-82 (nomor pertama kali terpakai, bukan rujukan ke tiket yang sudah ada sebelumnya).
- `standard-docs/decisions/finalize/dev-task-2/sprint-4-crate-decoupling-and-runtime-strangler.md` — BG-F1 §E, lokasi integrasi.
