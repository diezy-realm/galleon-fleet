---
title: Galleon — Community Tier, Lisensi & Enforcement
status: superseded
superseded_by: standard-docs/product-research/tier-pricing-and-entitlement-final.md
date: 2026-10-05
author: squad-product-owner
reviewed_by: squad-lead (sesi 2026-10-05)
depends_on:
  - standard-docs/product/galleon-product-definition.md
  - standard-docs/product-research/market-strategic-report-oct2026.md
  - standard-docs/product-research/gtm-segmentation-risk-research.md
---

> **DIGANTIKAN (2026-10-06):** angka tier, model harga, dan metode enforcement di dokumen ini sudah digantikan oleh `tier-pricing-and-entitlement-final.md` setelah pematangan lanjutan dengan Owner (tangga tier baru dengan dimensi `CrewPerShip`/`ConcurrentVoyages`, harga flat, Metode D+P2-repo-terpisah). Bagian provenance lisensi ZeroClaw (Bagian 2) TIDAK digantikan — tetap berlaku, lihat `license-provenance-attribution-research.md`. Dipertahankan di bawah hanya sebagai catatan historis.

> **Lingkup dokumen ini:** menetapkan batas Community tier secara final, menjelaskan kewajiban lisensi dari provenance fork ZeroClaw, dan mendefinisikan strategi enforcement (Metode D — capability gating via modul tertutup). Dokumen ini adalah acuan PO + Lead untuk Sprint 4 ENT-01 dan Sprint 7 FW-C1.
>
> **Catatan verifikasi:** semua klaim teknis di dokumen ini diverifikasi oleh squad-lead (read-only, sesi 2026-10-05) sebelum ditulis. Klaim yang belum dapat diverifikasi ditandai *[tidak terverifikasi]*. Jangan menandai item action sebagai selesai tanpa bukti implementasi.

---

## Bagian 1 — Batas Community Tier (FINAL)

### 1.1 Definisi Batas

| Dimensi | Batas Community | Catatan |
|---|---|---|
| Ship (departemen) | ≤ 2 total per Fleet | Dihitung dari `len(Fleet.ActiveShips)` |
| Squad (tim) | ≤ 3 total lintas semua Ship | Dihitung dengan iterasi semua Ship aktif, `sum(len(ship.SquadIDs))` — bukan per-Ship |
| Crew (karyawan AI) | Tidak dibatasi | Dikendalikan cukup lewat batas Squad |
| Crew per Squad | ≤ 6 | Soft ceiling — menjaga Squad tetap fokus |

**Aturan enforcement:** dua perbandingan integer independen — `totalShips <= 2` DAN `totalSquads <= 3`. Tidak ada validasi distribusi per-Ship; contoh `{Ship A: 2 Squad} + {Ship B: 1 Squad}` adalah komposisi valid, tapi tidak ada kombinasi khusus yang diwajibkan atau diblokir selain kedua total terpenuhi.

### 1.2 Alasan Desain

**Mengapa batas Ship dan Squad, bukan Crew?**
Ship dan Squad adalah unit organisasi/governance (departemen dan tim), bukan unit eksekusi agent. Membatasi Crew tidak memberi nilai diferensiasi karena OpenClaw, ZeroClaw, dan Hermes semuanya sudah unlimited agent — bersaing di sana tidak relevan. Batas Community justru membatasi *struktur organisasi* yang dapat dikelola, bukan *jumlah pekerjaan yang bisa diselesaikan*.

**Mengapa 2 Ship dan 3 Squad?**
Ini cukup untuk eksplorasi nyata. Dua departemen (misalnya Engineering dan Marketing) dengan total tiga tim memberikan gambaran hierarki Galleon yang sesungguhnya: Quartermaster → Ship → Squad → Crew. Pengguna yang butuh lebih dari ini biasanya sudah butuh fitur governance multi-user (tier Team), bukan sekadar lebih banyak departemen.

**Mengapa Crew unlimited?**
Pengguna bisa menyusun ulang Squad dan Crew kapan pun. Membatasi Crew per Fleet akan menghukum eksperimentasi yang justif menghasilkan feedback produk. Batas Squad sudah cukup mengontrol total Crew yang dapat aktif secara bersamaan (maks 18 Crew aktif: 3 Squad × 6 Crew/Squad).

**Koreksi dari versi sebelumnya:** `galleon-product-fundamentals.md` saat ini masih menyatakan "Community = 1 Ship aktif, ≤5 Crew, 2 Voyage, delegation depth 1" — ini **sudah usang** dan harus diperbarui dengan ODR baru sebelum Sprint 4 mulai.

---

## Bagian 2 — Kewajiban Lisensi dan Provenance Fork

### 2.1 Status Provenance

Galleon adalah derivative dari **ZeroClaw** (`github.com/zeroclaw-labs/zeroclaw`, dual-licensed MIT OR Apache-2.0). Bukti: commit pertama repo ini adalah `ZeroClaw v0.1.0` (2026-02-13). ZeroClaw Labs memegang trademark "ZeroClaw" dan "ZeroClaw Labs".

**Gap yang harus ditindaklanjuti (BLOCKER sebelum rilis publik apa pun):**

- `LICENSE-MIT` saat ini mencantumkan Copyright "ClawCrew Labs 2025" — ZeroClaw Labs tidak disebut sama sekali.
- `NOTICE` menyebut `clawcrew-labs/clawcrew` sebagai "the only official repository" — ini klaim yang keliru dan berpotensi menyesatkan.
- `README.md:178` mereplikasi kalimat lisensi upstream dengan "ClawCrew" sebagai pengganti nama asli.

**Kewajiban MIT/Apache-2.0 yang harus dipenuhi:**

1. Pemberitahuan hak cipta asli ZeroClaw Labs harus dipertahankan (MIT §1).
2. File yang diubah harus ditandai sebagai perubahan (Apache-2.0 §4b).
3. Klaim trademark ZeroClaw dilarang (Apache-2.0 §6).

**Tindakan yang diperlukan (bukan nasihat hukum):**

```
LICENSE-MIT  → tambah "Based on ZeroClaw by ZeroClaw Labs (https://github.com/zeroclaw-labs/zeroclaw)"
NOTICE       → ganti klaim "only official repository" milik ClawCrew Labs
             → tambah "Portions of this software derived from ZeroClaw, Copyright 2025 ZeroClaw Labs"
README.md    → perbarui baris lisensi agar tidak mengklaim trademark pihak lain
```

### 2.2 Implikasi Model Open Core

Galleon menggunakan model **Open Core**: basis warisan MIT/Apache tetap permisif, modul baru (`galleon-pro-*`) bisa dilisensikan komersial. MIT/Apache tidak mewajibkan Rust dibuka — itu pilihan strategis. Yang wajib hanyalah atribusi dan notasi perubahan.

Modul `galleon-pro-*` (closed source, repo terpisah) **tidak melanggar** MIT/Apache selama:
- Binary Community tidak menyertakan kode Pro.
- Kode warisan yang dicopy ke Pro tetap menyertakan notice asli.
- Pro tidak mengklaim kompatibilitas atau asal dari ZeroClaw tanpa izin.

**Yang tidak bisa dilakukan:** mengklaim bahwa "metode Workspace/SOP/RACI" adalah hak kekayaan intelektual eksklusif Galleon. RACI adalah teknik manajemen umum; SoD adalah prinsip keamanan yang sudah dikenal luas. Yang bisa dilindungi: implementasi spesifik (kode, hak cipta), nama merek (trademark — perlu clearance, lihat §2.3), dan konten pack terkurasi (EULA per organisasi).

### 2.3 Clearance Nama Merek "Galleon"

Pencarian awal (web, bukan penelusuran USPTO resmi) menemukan: `galleon.io` (perangkat lunak AI real estat), Galleon Technologies Inc. (registrasi USPTO), sengketa Galleon Guild Ltd. v. partnerships (Florida, 2026). Ini bukan temuan hukum konklusif, tetapi cukup sebagai sinyal bahwa clearance resmi diperlukan sebelum investasi merek lebih lanjut.

**Action item:** lakukan clearance trademark "Galleon" di Indonesia dan pasar sasaran sebelum material publik apa pun menggunakan merek ini.

---

## Bagian 3 — Strategi Enforcement: Metode D

### 3.1 Mengapa Metode D

Empat metode dievaluasi dalam sesi riset ini:

| Metode | Deskripsi | Efektivitas vs bypass | Direkomendasikan? |
|---|---|---|---|
| A — HMAC embedded key | Key simetris di binary | Nol (trivially extractable, source terbuka) | Tidak |
| B — Ed25519 asymmetric | Public key di binary, private di server | Rendah untuk open source (bisa comment-out di source) | Tidak sebagai enforcement utama |
| C — Soft limit / degradasi UX | Warning, rate-limit, upgrade prompt | Nol untuk bypass intentional; tinggi untuk UX | Ya, sebagai UX layer di atas D |
| D — Capability gating via modul tertutup | Fitur Pro tidak ada di binary Community | Tertinggi — tidak ada yang bisa dihapus dari source Community | Ya, sebagai enforcement primitive |

**Alasan memilih D:** untuk repo MIT/Apache yang bisa di-fork, tidak ada metode teknis yang benar-benar mencegah bypass. Metode D tidak mencoba mencegah — melainkan memastikan binary Community tidak pernah punya kode yang membuka batas, sehingga tidak ada yang bisa "dihapus". Seseorang yang mau hack bisa mengubah `2` menjadi `999` di source dan build ulang — itu harga yang dibayar untuk menjadi open source, dan itu acceptable. Model bisnis tidak bergantung pada pencegahan teknis ini.

Preseden yang valid lebih sempit dari yang semula ditulis: GitLab CE/EE memisahkan paket CE dari kode `ee/` (tetapi EE tetap source-available + license key), dan Home Assistant membuktikan honor system + manfaat nyata (Link/Relay) bisa berjalan. **Koreksi 2026-10-05:** Sentry FSL dan Supabase bukan capability gating dan dihapus dari daftar preseden; lihat `tier-enforcement-method-d-c-deep-dive.md` §2.

### 3.2 Desain Teknis Metode D

**Interface QuotaPolicy (ENT-01, Sprint 4, sebelum BG-F1):**

```go
// engine/src/entitlement/interfaces.go (saat ini: // TODO — belum ada skeleton)
// QuotaPolicy menetapkan batas organisasi per Fleet.
// nil berarti tidak ada batas (tier Team dan atas).
type QuotaPolicy interface {
    MaxShips() *int           // nil = unlimited
    MaxSquadsTotal() *int     // nil = unlimited; dihitung lintas semua Ship
    MaxCrewsPerSquad() *int   // nil = unlimited
}

type Service interface {
    GetPolicy(ctx context.Context, fleetID string) (QuotaPolicy, error)
}
```

**CommunityPolicy (hardcoded, Community build — open source):**

```go
type CommunityPolicy struct{}

func (p CommunityPolicy) MaxShips() *int        { v := 2; return &v }
func (p CommunityPolicy) MaxSquadsTotal() *int  { v := 3; return &v }
func (p CommunityPolicy) MaxCrewsPerSquad() *int { v := 6; return &v }
```

**Guard di service layer (bukan di HTTP handler):**

```go
// Dipanggil dari ship.Service.CreateShip sebelum append ke ActiveShips
quota, _ := e.entitlement.GetPolicy(ctx, fleetID)
if max := quota.MaxShips(); max != nil && len(fleet.ActiveShips) >= *max {
    return nil, ErrShipQuotaExceeded
}

// Dipanggil dari squad.Service.CreateSquad — butuh agregasi lintas Ship
totalSquads := 0
for _, s := range allShips { totalSquads += len(s.SquadIDs) }
if max := quota.MaxSquadsTotal(); max != nil && totalSquads >= *max {
    return nil, ErrSquadQuotaExceeded
}
```

**Catatan implementasi kritis (dari verifikasi Lead):**

1. Agregasi total Squad lintas Ship (`sum(len(ship.SquadIDs))`) **belum ada** di arsitektur saat ini — harus dibangun, bukan "sudah tersedia". `engine/src/ship/services.go` saat ini adalah in-memory map tanpa kuota atau agregasi.

2. ENT-01 (definisi interface `QuotaPolicy`) harus masuk **sebelum** BG-F1, karena BG-F1 membangun model organisasi persisten dan skema data Fleet/Ship/Squad harus dirancang dari awal untuk mendukung agregasi kuota.

3. Dua approval gate paralel (`tool/approval_gate.go` di Go dan `sop/approval/broker.rs` di Rust) adalah risiko konsistensi yang sudah diketahui. Kuota yang di-enforce dari satu sisi bisa tidak terlihat dari sisi lain. Hingga `clawcrew-runtime` selesai distrangler ke Go (Sprint 4), ini adalah trade-off yang diterima.

4. `engine/src/entitlement/interfaces.go` saat ini hanya `// TODO` satu baris — belum ada skeleton sama sekali.

### 3.3 File Entitlement Lokal (Mekanisme Offline) — DIBATALKAN 2026-10-05

> **Dibatalkan:** rancangan `entitlement.json` + HMAC di bawah adalah Metode A yang sudah dibuang Owner. Community memakai konstanta kompilasi `CommunityPolicy`, tanpa file. Sisi Pro ditunda. Lihat `tier-enforcement-method-d-c-deep-dive.md` §3.1 dan §4.3. Teks asli dipertahankan di bawah hanya sebagai catatan historis.

Galleon adalah local-first. Enforcement kuota tidak boleh butuh koneksi server. Mekanisme:

```json
// ~/.config/galleon/entitlement.json (di-generate otomatis saat instalasi pertama)
{
  "tier": "community",
  "issued_at": "2026-10-05T00:00:00Z",
  "hmac": "<stdlib crypto/hmac, key embed di binary — tamper-detection, bukan DRM>"
}
```

Untuk Community: file ini di-generate otomatis, kuota hardcoded di binary. HMAC hanya tamper-detection ringan, bukan security. Untuk Team: license key diterbitkan saat berlangganan, diverifikasi offline via Ed25519 (public key di binary, private key hanya di server). Masa tenggang 30 hari jika key kedaluwarsa.

### 3.4 UX Layer (Metode C sebagai pelengkap D)

Saat kuota terlampaui, respons terstruktur:

```json
{
  "error": "quota_exceeded",
  "resource": "squad",
  "limit": 3,
  "current": 3,
  "upgrade_url": "/settings/plans"
}
```

UI menampilkan upgrade prompt, bukan error generik. Implementasi di Sprint 7 §A FW-C1.

---

## Bagian 4 — Pemetaan ke dev-task-2

### Tiket Baru: ENT-01

**Sprint:** 4 §E — sebelum atau bersamaan awal BG-F1
**Severity:** MEDIUM | **Size:** S

Definisikan `QuotaPolicy` interface dan `CommunityPolicy` hardcoded di `engine/src/entitlement/interfaces.go`. Tidak ada dependency luar.

**Acceptance criteria:**
- `entitlement/interfaces.go` berisi interface dengan tiga method.
- `CommunityPolicy` mengimplementasi dengan nilai `{2, 3, 6}`.
- Unit test ketiga method.
- Guard di `ship.Service.CreateShip` memanggil policy dan mengembalikan `ErrShipQuotaExceeded`.

### Acceptance Criteria Tambahan untuk Tiket Existing

| Tiket | AC tambahan |
|---|---|
| BG-F1 (Sprint 4 §E) | Skema data persisten mendukung query `totalSquads` lintas Fleet; `CreateSquad` memanggil guard kuota ENT-01 |
| FW-C1 (Sprint 7 §A) | Tampilkan upgrade prompt saat `quota_exceeded`; hapus angka hardcoded tidak konsisten di `ShipyardView.tsx` |

---

## Bagian 5 — Status dan Tindakan Tertunda

| Item | Prioritas | Siapa |
|---|---|---|
| Pulihkan atribusi ZeroClaw di LICENSE-MIT, NOTICE, README | ~~BLOCKER~~ **SELESAI 2026-10-05** (belum di-commit); temuan lanjutan di `license-provenance-attribution-research.md` §5 | Owner |
| Clearance trademark "Galleon" | TINGGI — sebelum investasi merek | Owner (konsultasi ahli IP) |
| ODR baru: gantikan batas Community lama di `galleon-product-fundamentals.md` | TINGGI — sebelum BG-F1 mulai | PO → Owner |
| ENT-01 — `QuotaPolicy` interface | Sprint 4, sebelum BG-F1 | Backend-Go |
| AC kuota di BG-F1 dan FW-C1 | Sprint 4 + Sprint 7 | Backend-Go + Frontend |
| Evaluasi `galleon-pro-*` closed module | Setelah ada permintaan Pro nyata (YAGNI) | Lead + Owner |

---

## Referensi

- `standard-docs/product/galleon-product-definition.md`
- `standard-docs/decisions/finalize/dev-task-2/sprint-4-crate-decoupling-and-runtime-strangler.md` — BG-F1 §E
- `standard-docs/archive/dev-task-2-completed-sprints/sprint-7-product-features-and-org-model.md` — FW-C1
- `standard-docs/decisions/sop-constraint-layer-design.md`
- Riset eksternal: DotScramble Ed25519 license system (Hashnode, 2026-06-29)
- ZeroClaw upstream: `github.com/zeroclaw-labs/zeroclaw`
