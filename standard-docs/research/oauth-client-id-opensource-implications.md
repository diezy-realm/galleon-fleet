---
title: Implikasi Open-Source dari `client_id` OAuth Hardcoded — Analisa Lanjutan
status: research
date: 2026-10-05
author: squad-lead
depends_on: providers-reliability-auth-audit.md
---

# Implikasi Open-Source dari `client_id` OAuth Hardcoded

> Lanjutan dari `providers-reliability-auth-audit.md` §B5. Owner meminta analisa lebih dalam sebelum keputusan, plus cross-check `finalize/dev-tasks/` agar tidak duplikasi.

## 1. Verifikasi Teknis Lanjutan

Dua temuan baru yang mengubah penilaian risiko secara presisi:

**1.1 PKCE terkonfirmasi dipakai dengan benar.** Baik `xai_oauth.rs` maupun `openai_oauth.rs` memakai `code_challenge`/`code_challenge_method: S256` (standar PKCE/RFC 7636). Ini memang pola OAuth2 yang BENAR untuk "public client" (native app/CLI) — PKCE secara desain menggantikan kebutuhan `client_secret` rahasia. **Jadi hardcoded `client_id` BUKAN kelalaian keamanan** — ini sesuai standar, bukan "lupa pakai secret."

**1.2 Tidak ada jalur override — ini yang mengubah penilaian risiko.** Grep menyeluruh membuktikan `XAI_OAUTH_CLIENT_ID`/`OPENAI_OAUTH_CLIENT_ID` dipakai LANGSUNG sebagai konstanta di 6+ titik pemanggilan (`params.insert("client_id", XAI_OAUTH_CLIENT_ID)`), **TIDAK ADA parameter fungsi atau pembacaan config/env** untuk operator menyuplai `client_id` sendiri. Ini KONTRAS dengan Gemini dan Email OAuth2 di crate yang SAMA, yang justru menerima `client_id`/`client_secret` sebagai parameter fungsi dari config.

**Kesimpulan presisi**: risiko bukan "kebocoran kredensial" (PKCE mencegah itu), tapi **ketergantungan 100% pada SATU `client_id` yang didaftarkan Owner/tim Galleon** untuk SEMUA instalasi xAI/OpenAI di dunia — baik 1 instalasi atau 100.000 instalasi, semuanya memukul kuota/rate-limit OAuth app yang SAMA di sisi provider.

## 2. Analisa Dampak — Dua Skenario Konkret

### Skenario A: Produk tetap closed-source / distribusi terbatas
Risiko RENDAH. Jumlah instalasi terkendali, kemungkinan kecil memukul rate-limit OAuth app xAI/OpenAI yang didaftarkan.

### Skenario B: Produk open-source, distribusi bebas (fork, self-build, komunitas)
Risiko **MEDIUM-HIGH secara operasional** (bukan keamanan data):
- Siapa pun yang build dari source akan otomatis memakai `client_id` YANG SAMA — tidak ada cara mereka "mendaftar sendiri" tanpa mengubah kode.
- Kalau basis pengguna besar, provider (xAI/OpenAI) BISA mendeteksi pola "satu `client_id`, puluhan ribu alamat IP/user berbeda" sebagai sinyal abuse dan membatasi/mencabut app tersebut — ini akan mematikan fitur OAuth untuk SEMUA pengguna sekaligus (termasuk yang legitimate), bukan cuma yang menyalahgunakan.
- Precedent industri: ini pola yang sudah terjadi di banyak tool open-source CLI yang terhubung ke provider API besar (rate-limit per-app bukan per-user adalah desain umum provider untuk native-app OAuth).

## 3. Rekomendasi Konkret (bukan keputusan final — bahan diskusi Owner)

Tiga opsi, bukan saling eksklusif:

1. **"Bring your own client_id" (BYOC) sebagai override opsional** — tambah field config (env var atau `[auth.xai]`/`[auth.openai]` di config.toml) yang kalau diisi, OVERRIDE konstanta hardcoded. Default tetap pakai `client_id` Galleon (supaya pengguna awam tidak perlu daftar OAuth app sendiri), tapi power-user/kontributor yang khawatir rate-limit bisa pakai `client_id` mereka sendiri. Ini **tidak breaking** — murni penambahan opsional, pola `#[serde(default)]` yang sudah konsisten dipakai di `schema.rs`.
2. **Pantau kuota OAuth app secara proaktif** — kalau provider menyediakan dashboard kuota untuk OAuth app, siapkan alerting sebelum limit tercapai. Ini mitigasi operasional, bukan kode.
3. **Dokumentasikan model ini secara eksplisit** sebelum open-source — supaya kontributor paham risiko ini dari awal, bukan kaget saat OAuth tiba-tiba berhenti bekerja untuk semua orang.

**Rekomendasi prioritas**: #1 (BYOC override) adalah perbaikan teknis paling murah dengan dampak mitigasi risiko terbesar — tidak menghapus kenyamanan default, hanya menambah jalan keluar untuk kasus skala besar.

## 4. Cross-Check `finalize/dev-tasks/` — AMAN, Tidak Ada Duplikasi

Grep menyeluruh `open.source|open-source|client_id|OAuth|community|komunitas` di SELURUH `standard-docs/decisions/finalize/dev-tasks/` = **0 hasil**. Topik kesiapan open-source, `client_id`, maupun strategi komunitas **belum pernah disinggung sama sekali** di rencana tim dev-tasks manapun (BE-Go, BE-Rust, FE-Web, FE-Tauri, Platform/DevOps, Rebrand, Docs-Governance, Carryover).

**Konfirmasi**: riset dan rekomendasi di dokumen ini **genuinely baru**, tidak tumpang tindih dengan rencana apa pun yang sudah ada. Aman untuk diangkat ke backlog tanpa risiko duplikasi kerja.

---

## 5. Sumber

- `crates/clawcrew-providers/src/auth/xai_oauth.rs:16,20,149-157,284,579` (`XAI_OAUTH_CLIENT_ID`, `XAI_OAUTH_REDIRECT_URI`, PKCE `build_authorize_url`, test assertion)
- `crates/clawcrew-providers/src/auth/openai_oauth.rs:18,71-79` (`OPENAI_OAUTH_CLIENT_ID`, PKCE `build_authorize_url`)
- `crates/clawcrew-providers/src/auth/mod.rs` (pembanding: Gemini/Email OAuth2 menerima `client_id` sebagai parameter, bukan konstanta)
- Cross-check: `standard-docs/decisions/finalize/dev-tasks/*.md` (seluruh folder, grep nihil untuk topik ini)
- Dokumen asal: `research/providers-reliability-auth-audit.md` §B5
