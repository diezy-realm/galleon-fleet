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

## 3b. Rujukan Standar Industri — DAC BUKAN Ide Terisolasi (riset lanjutan 2026-10-05)

Verifikasi lanjutan membuktikan modul `verifiable_intent` BUKAN rekayasa internal Galleon — ini implementasi dari standar industri nyata, dan usulan DAC di atas SEJALAN dengan upaya standardisasi yang SEDANG berjalan paralel di beberapa badan standar. Ini mengubah DAC dari "ide internal yang terdengar bagus" menjadi "arah yang selaras dengan ke mana industri bergerak".

**"Verifiable Intent" adalah merek/standar resmi Mastercard** (co-developed dengan Google), baru didonasikan ke **FIDO Alliance** bersama **AP2 (Agent Payments Protocol, Google)** pada Mei 2026. Nama field/struct di kode Galleon adalah pemetaan LANGSUNG dari terminologi resmi AP2/VI:

| Istilah AP2/VI resmi | Field/konsep di kode Galleon | Sumber |
|---|---|---|
| Checkout/Payment **Mandate**, stage **Open** (batasan sebelum eksekusi) | `MandateMode::Autonomous` | [AP2 — Key Concept: Verifiable Digital Credentials][ap2-vdc] |
| Mandate stage **Closed** (otorisasi final per-transaksi) | `MandateMode::Immediate` | sda |
| **Identity binding** ke kunci kriptografis perangkat | `Cnf` (confirmation key binding) | [FIDO Alliance — Building the Trust Layer for Agentic Payments with AP2 and Verifiable Intent][fido-vi] |
| **Selective disclosure** (tampilan berbeda per-pihak, demi privasi) | `create_disclosure`/`serialize_sd_jwt` | sda |
| 3 pertanyaan inti: *Authorization, Authenticity, Accountability* | Pertanyaan yang coba dijawab modul `verification.rs` | [AP2 — Why an Agent Payments Protocol is Needed][ap2-why] |

**Usulan DAC di §3 SEJALAN dengan 2 upaya standardisasi IETF yang sedang berjalan**, independen dari AP2/VI (bukti bahwa arah ini bukan taruhan pada satu vendor):

- **[IETF WIMSE — "Capability-Bound Delegation Chains with Scoped Context Disclosure for AI Agents"][ietf-wimse]** (draft 2026): rantai delegasi lintas sistem untuk AI agent, wewenang MENYEMPIT di tiap hop — konsep yang PERSIS sama dengan "attenuasi" yang diusulkan DAC untuk hierarki Quartermaster→Ship→Squad→Crew.
- **[IETF OAuth — DAAP, Delegated Agent Authorization Protocol][ietf-daap]** (draft 2026): mengisi gap yang OAuth 2.0 tidak punya — membuktikan manusia tertentu mengotorisasi agent tertentu untuk tindakan tertentu, bisa dicabut real-time, menghasilkan rekaman tamper-evident. Pembanding langsung untuk "Charter/Fleet Code yang ditandatangani Owner" yang diusulkan DAC.
- **Riset akademik pendukung rekomendasi "token sederhana dulu"**: [arXiv — "Capability-Based Delegation of Privileges in Multi-Agent Systems"][arxiv-capmas] menggabungkan macaroon-based token untuk delegasi offline dengan privilege reduction monoton — mengonfirmasi rekomendasi §3 ("gunakan macaroon/biscuit-style dulu, SD-JWT penuh hanya bila ada verifier pihak ketiga") BUKAN jalan pintas, tapi arah yang didukung riset terbaru.
- **Konteks urgensi industri**: [arXiv — "Agent Identity Protocol for Verifiable Delegation Across MCP and A2A"][arxiv-identity] men-scan ~2.000 MCP server dan menemukan SEMUA tidak punya autentikasi identitas agent — gap yang sangat luas di seluruh industri, bukan kekurangan spesifik Galleon.

**Kesimpulan riset lanjutan**: keputusan "hapus implementasi, simpan ide DAC" (§4) diperkuat, bukan diubah. Implementasi AP2/VI yang ada tepat dihapus karena (a) domain commerce tidak relevan untuk Galleon **hari ini**, (b) standar ini baru didonasikan ke FIDO Mei 2026 — masih sangat baru dan berisiko berubah bentuk sebelum stabil (konsisten dengan temuan kode asli: "verifier penuh ditunda ke tahap upstream"). Tapi ide DAC-nya sejalan dengan 2 upaya standardisasi IETF independen yang berjalan paralel — bukan taruhan pada satu standar vendor, tapi arah umum industri agent-authorization.

[ap2-vdc]: https://ap2-protocol.org/
[ap2-why]: https://ap2-protocol.org/#why-an-agent-payments-protocol-is-needed
[fido-vi]: https://fidoalliance.org/building-the-trust-layer-for-agentic-payments-with-ap2-and-verifiable-intent/
[ietf-wimse]: https://www.ietf.org/archive/id/draft-atakora-wimse-sadp-delegation-01.html
[ietf-daap]: https://www.ietf.org/archive/id/draft-mishra-oauth-agent-grants-01.html
[arxiv-capmas]: https://arxiv.org/html/2609.06500v1
[arxiv-identity]: https://arxiv.org/html/2603.24775

## 3c. Keputusan Owner (2026-10-05) — Arah Commerce TERBUKA, AP2 Jadi Dependency Masa Depan yang Dilacak

**Konfirmasi Owner**: cakupan "bayar" di `galleon-product-definition.md` §5 **berpotensi meluas ke agentic commerce** — agent membeli dari merchant pihak ketiga atas nama user, bukan sekadar pembayaran internal (top-up API/infra yang user sudah setup sendiri).

**Implikasi langsung terhadap keputusan hapus-modul (§4)**:
- Keputusan **hapus implementasi AP2/VI yang ADA SEKARANG tetap benar** — alasannya TIDAK berubah: kode yang ada setengah jadi (verifier belum ada), bergantung spesifikasi yang baru donasi ke FIDO Mei 2026 (berisiko berubah bentuk), dan nol pemakai saat ini. Menyimpan kode mati yang akan jadi usang adalah beban maintenance tanpa manfaat.
- **TAPI** ini **BUKAN berarti "tutup topik AP2 selamanya"** — dengan arah commerce terbuka, AP2/VI (atau penerusnya) kemungkinan akan **relevan kembali** saat Galleon benar-benar membangun fitur agentic commerce. Bedanya dengan sebelumnya: ini bukan lagi *"ide bagus yang kebetulan ada di kode lama"*, tapi *"dependency arsitektur yang diketahui akan dibutuhkan, timing-nya saja belum pasti"*.

**Rekomendasi konkret (revisi atas §4)**:
1. **Hapus kode tetap jalan** (R-38/BR-D3) — TIDAK berubah, tag `archive/pre-galleon` sudah menyimpan referensi untuk dipulihkan.
2. **Upgrade status ADR DAC (G-37) dari "P3, opsional" menjadi item yang WAJIB ditinjau ulang SEBELUM fitur commerce pertama mulai didesain** — bukan P3 yang bisa diabaikan tanpa batas waktu, tapi gate eksplisit: "jangan desain fitur commerce tanpa membaca ADR ini dulu."
3. **Pantau evolusi standar AP2/VI/FIDO secara berkala** (bukan one-time read) — karena baru didonasikan Mei 2026, spesifikasinya kemungkinan masih berubah; keputusan arsitektur commerce Galleon sebaiknya menunggu standar ini lebih stabil, BUKAN berburu-buru implementasi versi awal yang mungkin berubah drastis.
4. **Saat fitur commerce mulai didesain nanti**: evaluasi ulang apakah re-implementasi AP2/VI dari nol lebih baik daripada memulihkan kode dari tag arsip — kemungkinan besar re-implementasi dari nol lebih baik (standar akan sudah lebih matang + lebih stabil dibanding versi yang diaudit saat ini), tapi kode arsip tetap berguna sebagai referensi pola (`ring` ECDSA, SD-JWT disclosure) yang sudah disebut di §3.
5. **Catat keterkaitan dengan invariant F1** ("Treasury read-only", "zero platform markup", larangan "reseller kredit" di §7 product definition) — fitur commerce APAPUN bentuknya nanti HARUS didesain supaya tetap konsisten dengan invariant ini (Galleon memfasilitasi bukan jadi perantara/reseller) — ini pagar yang sudah ada, bukan sesuatu yang perlu dibuat baru, tapi perlu eksplisit dirujuk saat desain commerce dimulai.

## 4. Keputusan & tindak lanjut

- **Hapus** modul, tool `vi_*`, `VerifiableIntentConfig`, peringatan (R-38), setelah membuat tag `archive/pre-galleon`.
- **Catat** konsep DAC sebagai kandidat ADR masa depan (G-37, P3) — bukan komitmen implementasi.
- Konfigurasi pengguna yang masih memuat `[verifiable_intent]` diabaikan dengan peringatan satu kali.
- **(Update 2026-10-05)** G-37 di-upgrade jadi gate wajib: ADR DAC harus ditinjau ulang sebelum desain fitur commerce pertama dimulai — lihat §3c untuk alasan dan detail.
