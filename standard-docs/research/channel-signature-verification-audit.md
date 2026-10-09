---
title: Audit Channel Signature Verification — KOREKSI KEDUA (Bug Nyata Ditemukan, Verifikasi Independen Dilakukan)
status: research
date: 2026-10-05
author: squad-lead
depends_on: automation-cron-webhook-audit.md
---

# Audit Channel Signature Verification — Temuan Final (2 Sub-Agent + Verifikasi Independen)

> **Riwayat dokumen ini**: draft 1 (audit manual darurat) salah menyimpulkan CRITICAL vulnerability dari `#[cfg(test)]` tanpa cek logic produksi terpisah. Draft 2 (sub-agent `28089da6`) mengoreksi ke "tidak ada gap" — TAPI sub-agent ini TIDAK menggali cukup dalam untuk menemukan bahwa fungsi yang dirujuk sebenarnya tidak eksis. Sub-agent KEDUA (`19883579`), yang menggali lebih persisten saat mencari definisi fungsi, menemukan kontradiksi dan bug nyata. **Saya verifikasi independen temuan ini sendiri** (grep menyeluruh seluruh repo, bukan cuma satu crate) sebelum menuliskan dokumen final ini — hasilnya mengonfirmasi sub-agent kedua benar.

## Verdict Final: CHANGES-REQUESTED — Bug Nyata, Severity HIGH

## Temuan Terverifikasi Independen

**Grep saya sendiri di SELURUH repo** (bukan dibatasi satu crate) untuk `fn verify_whatsapp_signature` dan `fn optional_channel_routes`:

```
Hasil: 1 match total — dan itu BUKAN definisi fungsi.
crates/clawcrew-gateway/src/webhook_ingress.rs:959:
    let start = source.find("fn optional_channel_routes(").expect(...)
```

Ini adalah **string literal di dalam drift-guard test** yang mencari teks tersebut di `lib.rs` via `include_str!`. **Kedua fungsi ini genuinely tidak ada di manapun di codebase** — bukan di `clawcrew-gateway`, bukan di `clawcrew-channels`, bukan di crate lain mana pun.

### Bug #1 (HIGH): `verify_whatsapp_signature` Dirujuk 9+ Kali di Test, Tapi Tidak Pernah Didefinisikan

`crates/clawcrew-gateway/src/tests.rs:5811-5956,7492` memanggil `verify_whatsapp_signature(...)` di 9+ test case (valid signature, wrong secret, tampered body, missing prefix, empty header, invalid hex, empty body, unicode body) — skenario test yang KOMPREHENSIF dan BENAR secara desain. Tapi fungsi yang dites ini **tidak ada definisinya**. Implikasi: file test ini TIDAK BISA compile sebagaimana adanya, ATAU modul ini di-exclude diam-diam dari build (orphaned test file yang tidak ter-wire ke `lib.rs`). **Tidak ada bukti otomatis yang BERJALAN hari ini yang membuktikan verifikasi HMAC WhatsApp benar-benar bekerja end-to-end** — meskipun desainnya (dilihat dari isi test yang ditulis) sudah benar.

### Bug #2 (HIGH): Drift-Guard Test Sendiri Rusak — Jaring Pengaman Tidak Berfungsi

`crates/clawcrew-gateway/src/webhook_ingress.rs:959` — test `every_channel_webhook_route_is_classified_by_the_registry` dirancang untuk MENCEGAH regresi "ada route webhook baru yang lolos tanpa autentikasi" dengan cara membaca source `lib.rs` dan mencari fungsi `optional_channel_routes`. Karena fungsi itu tidak ada, `.expect(...)` di baris itu akan **panic saat test dijalankan**. Artinya **test regresi yang seharusnya menjaga keamanan registry `MESSAGE_DISPATCHING_WEBHOOKS` saat ini TIDAK BERFUNGSI** — baik karena gagal (kalau test suite ini benar-benar dijalankan di CI dan harusnya kelihatan merah), atau (lebih mengkhawatirkan) kalau entah bagaimana tidak pernah actually run di CI, berarti ada BLIND SPOT di pipeline testing yang tidak diketahui.

### Yang TETAP Benar dari Audit Sebelumnya (Tidak Berubah)

- **Desain abstraksi `webhook_ingress::authenticate()` + `WebhookAdapterSpec` + `VerifiedWebhookIngress` tetap SOLID** — ini bukan masalah desain, masalahnya adalah dua bagian KODE PEMBUKTIAN (verify function, drift-guard) yang hilang/rusak.
- **Linq dan Nextcloud Talk tetap terverifikasi bekerja** — test `failed_verification_refuses` di `webhook_ingress.rs:699-706` ada dan terlihat valid untuk keduanya (sub-agent kedua mengonfirmasi ini masih baik).
- **Telegram/Discord/Slack/QQ/WeChat tetap benar tidak butuh signature** — kelimanya pola outbound (polling/WebSocket), dikonfirmasi ULANG oleh KEDUA sub-agent secara independen (Telegram `getUpdates`, Discord Gateway WS `discord/mod.rs:68`, Slack Socket Mode `slack/mod.rs:4043`, QQ OAuth2+WS, WeChat polling) — ini bukan gap, ini desain yang benar.
- **`clawcrew-plugins/src/signature.rs` adalah topik BERBEDA** (Ed25519 plugin manifest signing) — jangan dicampur dengan webhook channel signature verification.

## Dampak Nyata

**Tidak diketahui pasti** apakah WhatsApp HMAC verification di produksi benar-benar berjalan atau tidak — karena bukti otomatis (test) yang seharusnya membuktikannya rusak/tidak ada. Ini BERBEDA dari "pasti ada gap keamanan" (kesimpulan draft 1 yang salah) MAUPUN "pasti tidak ada gap" (kesimpulan draft 2 yang kurang teliti) — kesimpulan yang BENAR adalah: **status aktual tidak terverifikasi, dan mekanisme yang seharusnya memverifikasi secara otomatis itu sendiri rusak.**

## Rekomendasi — Tindakan Konkret, Urutan Prioritas

1. **[P0, SEGERA]** Jalankan `cargo test -p clawcrew-gateway --features channel-whatsapp-cloud` secara nyata (bukan dibaca statis) untuk mendapat ground-truth pass/fail. Ini pertanyaan yang HARUS dijawab dengan menjalankan compiler, bukan dengan audit baca kode lagi.
2. **[P0]** Cari di riwayat git (`git log -p` / `git blame`) kapan dan mengapa `verify_whatsapp_signature`/`optional_channel_routes` dihapus atau dipindah — apakah ini regresi dari refactor yang tidak lengkap (test lupa dibersihkan) atau file yang hilang karena merge/rebase yang salah.
3. **[P0]** Perbaiki atau pulihkan implementasi fungsi yang hilang, pastikan `every_channel_webhook_route_is_classified_by_the_registry` benar-benar lolos (bukan cuma compile, tapi PASS).
4. **[P1]** Setelah #1-3 selesai, tambah test INTEGRASI level HTTP (bukan cuma unit-level `authenticate()`) yang benar-benar POST ke route Axum WhatsApp dengan signature salah dan assert 401 — menutup kemungkinan "unit test lolos tapi wiring HTTP-nya sendiri bocor".
5. **[P2]** Dokumentasikan di `webhook_ingress.rs` bahwa `MESSAGE_DISPATCHING_WEBHOOKS` adalah **daftar otoritatif dan lengkap** untuk channel yang butuh verifikasi inbound — channel baru yang menerima webhook (bukan outbound-poll) WAJIB didaftarkan di sini sebelum rilis.

## Pelajaran Proses (Untuk Audit Masa Depan)

Dokumen ini melalui 3 iterasi sebelum sampai ke kesimpulan yang benar — ini bukan kegagalan, ini bukti nilai verifikasi berlapis:
1. Audit cepat/dangkal → false positive (CRITICAL yang tidak nyata).
2. Audit sub-agent pertama → false negative (menyimpulkan aman karena berhenti setelah menemukan REGISTRY entry, tanpa memverifikasi fungsi yang didaftarkan itu benar-benar punya implementasi).
3. Audit sub-agent kedua (lebih persisten, mengikuti rantai pemanggilan sampai tuntas) + verifikasi independen saya sendiri → temuan yang benar.

**Prinsip untuk audit berikutnya**: menemukan ENTRY di registry/config TIDAK CUKUP untuk menyimpulkan "fitur ini berfungsi" — harus ditelusuri sampai ke IMPLEMENTASI NYATA yang dipanggil, dan idealnya dikonfirmasi dengan menjalankan test sungguhan, bukan hanya membaca kode secara statis.

---

## Sumber

- Verifikasi independen (saya, grep seluruh repo): `fn verify_whatsapp_signature`, `fn optional_channel_routes` → 1 match (string literal di drift-guard, bukan definisi)
- Audit sub-agent kedua (`19883579`): `crates/clawcrew-gateway/src/{tests.rs:5811-5956,7492, webhook_ingress.rs:959,197-205,699-706}`, `crates/clawcrew-channels/src/{telegram,discord/mod:68,slack/mod:4043,qq,wechat}.rs`
- Audit sub-agent pertama (`28089da6`) — tetap valid untuk bagian Linq/Nextcloud/outbound-channel, TIDAK valid untuk klaim WhatsApp "sudah terverifikasi dengan 7 test negatif" (klaim itu salah — testnya tidak bisa jalan karena fungsi yang didukung tidak ada)
- Dokumen terkait: `automation-cron-webhook-audit.md` (BL-14 — topik berbeda, sudah ditutup)
