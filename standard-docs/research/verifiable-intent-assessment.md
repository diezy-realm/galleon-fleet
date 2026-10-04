# Penilaian `verifiable_intent` — Relevansi untuk Pengembangan Galleon

> Status: Final (2026-10-04). Dibuat agar pengetahuan dari modul `clawcrew-runtime/src/verifiable_intent` tidak hilang ketika kodenya dihapus (R-38). Kode dipulihkan dari tag `archive/pre-galleon` bila dibutuhkan.

## 1. Apa yang sebenarnya diimplementasikan (fakta kode)

| Berkas | LOC | Isi |
|---|---|---|
| `types.rs` | 1.319 | Model data: `Jwk` (EC P-256, `kid` di dalam JWK), `Cnf`, `MandateMode {Immediate, Autonomous}`, `PaymentInstrument`, `Entity` (merchant/payee: id→name→website), mandat checkout/payment (open/final, L3), `Constraint`/`KnownConstraint`, `CredentialChain` |
| `crypto.rs` | 1.220 | `jws_sign`, `sd_hash`, `create_disclosure`, `serialize_sd_jwt` — SD-JWT dengan `ring` (ECDSA P-256) |
| `issuance.rs` | 717 | Penerbitan **L2** (mode Immediate: user mengunci nilai akhir; mode Autonomous: user menetapkan batasan) dan **L3** (agent bertindak di dalam batasan) |
| `verification.rs` | 1.258 | Pemeriksaan constraint (`StrictnessMode::{Strict,Permissive}`: unknown constraint pada mandat *open* selalu ditolak), integritas binding. **`VerifiedCredentialChain` tidak bisa dibuat oleh kode mana pun** — "verifier penuh ditunda ke tahap upstream" |
| `error.rs` | 95 | `ViError`/`ViErrorKind` |

**Konsep:** rantai kredensial 3 lapis (L1 penerbit/credential provider → L2 mandat user → L3 eksekusi agent) berbasis SD-JWT. User menandatangani **batasan** (jumlah, merchant allowlist, instrumen pembayaran, line item) lalu agent boleh bertindak mandiri **hanya di dalam batasan itu**, dan pihak ketiga (merchant/PSP) dapat memverifikasi bukti bahwa tindakan agent sah.

**Status:** penerbitan + kripto + tipe ada; verifier rantai belum; tool `vi_verify` disembunyikan; satu-satunya efek runtime = peringatan konfigurasi. Dependen: hanya `src/main.rs`; daftar nama `vi_verify` di `channels/orchestrator` (kategori "execute"). Tidak ada di Go, `web/`, atau dokumen produk.

## 2. Kecocokan dengan Galleon

| Aspek | Penilaian |
|---|---|
| Domain (pembayaran/checkout/merchant) | **Tidak cocok.** Galleon: BYOK/BYOM, Treasury read-only, tanpa perantara tagihan/komersial (F1). Tidak ada fitur commerce di PRD |
| Pola "Autonomous: user menetapkan batas, agent bertindak sendiri" | **Cocok secara konsep** dengan *autonomy under command*: approval, risk-tier, Fleet Code/Charter, budget cap |
| Bukti kriptografis yang dapat diverifikasi pihak ketiga | Bermanfaat bila kelak ada **remote device / multi-org / audit eksternal**; saat ini audit internal (Go audit trail + audit signing Rust) mencukupi |
| Kematangan | Setengah jadi; bergantung spesifikasi eksternal yang masih berkembang dan verifier upstream |
| Biaya pemeliharaan | 4,6k LOC kripto sensitif harus diaudit/diperbarui tanpa pemakai |

**Kesimpulan:** implementasi saat ini **legacy komersial** dan tidak dipakai → **hapus**. Namun *ide* yang bernilai ada: **rantai mandat terbatas yang ditandatangani** untuk pendelegasian wewenang.

## 3. Ide yang dipertahankan untuk masa depan: *Delegated Authority Chain* (DAC)

Pemetaan konsep ke hierarki Galleon:

| VI | Galleon (usulan) |
|---|---|
| L1 penerbit / kunci terpercaya | Quartermaster (Owner) device key |
| L2 mandat user (batasan) | **Charter / Fleet Code** yang ditandatangani Owner: tool yang boleh, budget USD, scope memori, jam kerja, risk-tier |
| L3 eksekusi agent dalam batasan | **Mission/Run token** per Ship→Squad→Crew yang *mempersempit* (attenuate) wewenang induk |
| Constraint: amount, allowlist, instrumen | Constraint: tool allowlist, path/host allowlist, budget cap, kedaluwarsa, jumlah iterasi |
| Open vs Final mandate | Autonomous (batasan) vs Immediate (satu aksi yang di-approve) — sama dengan alur approval |
| `StrictnessMode`; unknown constraint pada mandat open = tolak | **Pelajaran desain:** constraint yang tidak dikenal pada wewenang terbuka harus fail-closed |
| `kid` di dalam JWK, `cnf` mengikat kunci | binding kredensial ke kunci perangkat (selaras QR pairing G-20) |

**Manfaat bila dibangun (di Go, bukan Rust — policy SSOT):** wewenang sub-agent tidak bisa melebihi induk (bukti kriptografis, bukan hanya cek runtime); audit yang dapat diverifikasi offline; delegasi antar-perangkat tanpa mempercayai transport.
**Kapan layak:** setelah G-35 (identitas/akses), G-23 (model organisasi), G-24 (consent), dan kebutuhan multi-device/remote nyata. Gunakan token sederhana (mis. macaroon/biscuit-style atau JWT dengan batas) lebih dulu; SD-JWT penuh hanya bila ada verifier pihak ketiga.
**Aset yang layak dipakai ulang sebagai referensi:** pola `ring` ECDSA P-256 + JWS, desain disclosure selektif, uji constraint strict vs permissive. Salinan ada di tag arsip.

## 4. Keputusan & tindak lanjut

- **Hapus** modul, tool `vi_*`, `VerifiableIntentConfig`, peringatan (R-38), setelah membuat tag `archive/pre-galleon`.
- **Catat** konsep DAC sebagai kandidat ADR masa depan (G-37, P3) — bukan komitmen implementasi.
- Konfigurasi pengguna yang masih memuat `[verifiable_intent]` diabaikan dengan peringatan satu kali.
