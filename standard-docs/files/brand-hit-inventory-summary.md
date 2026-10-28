# Ringkasan Inventaris Brand Hit (Legacy -> Galleon)

**Tanggal:** 2026-10-04  
**Status:** Baseline Selesai (PR-06 / N-01 / RB-01)  
**Sumber Data:** `standard-docs/files/brand-hit-inventory.csv` (1.709 file, 40,422 hits)  
**Pola Token Dicari:** `zeroclaw`, `zerocode`, `zerorelay`, `clawcrew` / `claw-crew` / `claw_crew` / `claw crew`, `fleet-ai` / `fleet_ai` / `fleet ai`.

---

## 1. Distribusi per Kategori Tindakan

| Kategori | Jumlah File | Total Kemunculan | Strategi & Waktu Eksekusi |
|---|---|---|---|
| **`rename`** | 1,400 | 33,537 | Diubah secara mekanis ke Galleon pada **Gelombang 3 (PR-80 s/d PR-88)** setelah pembersihan legacy |
| **`delete`** | 246 | 4,335 | Dihapus langsung pada **Gelombang 1 (PR-17, PR-18, PR-19)** sebelum rename massal dimulai |
| **`keep`** | 43 | 381 | Dipertahankan tanpa diubah: ADR/ODR historis immutable, lisensi upstream (Apache-2.0/MIT), WIT v0 beku |
| **`needs-migration`** | 20 | 2,169 | Membutuhkan migrasi data on-disk saat runtime boot (`~/.clawcrew` -> `~/.config/galleon`) pada **PR-81** |
| **Total** | **1,709** | **40,422** | |

---

## 2. Distribusi per Komponen Terbesar

| Komponen | Jumlah File | Total Kemunculan | Kategori Utama |
|---|---|---|---|
| `crates/clawcrew-channels` | 79 | 9,123 | `rename` / `delete` |
| `crates/clawcrew-runtime` | 245 | 8,868 | `rename` / `delete` |
| `docs` | 238 | 3,836 | `rename` / `delete` |
| `crates/clawcrew-providers` | 44 | 2,223 | `rename` / `delete` |
| `crates/clawcrew-tools` | 88 | 2,101 | `rename` / `delete` |
| `crates/clawcrew-gateway` | 53 | 1,758 | `rename` / `delete` |
| `src` | 125 | 1,508 | `rename` / `delete` |
| `crates/clawcrew-config` | 34 | 1,290 | `rename` / `delete` |
| `root` | 42 | 1,222 | `rename` / `delete` |
| `apps/zerocode` | 33 | 1,067 | `rename` / `delete` |
| `crates/galleon-i18n` | 12 | 785 | `rename` / `delete` |
| `scripts` | 60 | 671 | `rename` / `delete` |
| `standard-docs` | 72 | 660 | `rename` / `delete` |
| `tests` | 64 | 657 | `rename` / `delete` |
| `crates/galleon-hardware` | 33 | 547 | `rename` / `delete` |

---

## 3. Rencana Eksekusi Berdasarkan Inventaris

1. **Gelombang 1 (Pembersihan Sebelum Rename):**
   - Hapus seluruh cluster berkategori `delete` (arsip `docs/_archive/`, workflow ClawCrew usang, skill usang, packaging usang).
   - Tindakan ini langsung memotong ratusan file dan ribuan hit dari beban rename.
2. **Gelombang 2 (Kontrak & Identitas Teknis):**
   - Tetapkan WIT v1 (`wit/v1/galleon:plugin@1.0.0`) sambil mempertahankan v0 beku (`keep`).
   - Siapkan skrip rename `scripts/rename/` dan gerbang `scripts/ci/brand_gate.sh`.
3. **Gelombang 3 (Rename Mekanis & Migrasi Data):**
   - Eksekusi `needs-migration` (PR-81): salin atomik folder data on-disk `~/.clawcrew` ke `~/.config/galleon` saat boot.
   - Eksekusi `rename` per crate (PR-82) dari bawah ke atas, binary (PR-83), Go engine (PR-84), UI/Desktop (PR-85), dan CI/docs (PR-87).
   - Pastikan entri `keep` (ADR/ODR di `standard-docs/decisions/` dan lisensi) tetap utuh tanpa modifikasi.
