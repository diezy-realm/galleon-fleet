---
title: Backlog — Provider Reliability, OAuth, & Memory Data Governance
status: backlog
date: 2026-10-05
author: squad-lead
depends_on: providers-reliability-auth-audit.md, memory-data-governance-audit.md, oauth-client-id-opensource-implications.md
---

# Backlog — Provider Reliability, OAuth, & Memory Data Governance

> Diturunkan dari 3 dokumen riset: `providers-reliability-auth-audit.md`, `memory-data-governance-audit.md`, `oauth-client-id-opensource-implications.md`.
> **Cross-check `finalize/dev-tasks/` SELESAI** (lihat `oauth-client-id-opensource-implications.md` §4 dan verifikasi tambahan) — grep menyeluruh untuk topik reliability/circuit-breaker/cost-cap/purge/erase/hygiene/open-source/client_id/OAuth = **0 hasil genuine** (hasil yang muncul adalah false-positive di konteks lain: fail-closed gateway, pairing, Nginx). **Semua item di bawah aman diangkat, tidak ada duplikasi kerja tim dev-tasks.**
> Nomor lanjutan dari BL-24 (`backlog/plugin-mcp-architecture-backlog.md`).

---

## BL-25 — "Bring Your Own client_id" (BYOC) untuk OAuth xAI/OpenAI (Open-Source Readiness)

**Rujukan riset:** `oauth-client-id-opensource-implications.md` §1-3.

**Masalah:** `XAI_OAUTH_CLIENT_ID`/`OPENAI_OAUTH_CLIENT_ID` hardcoded sebagai konstanta, dipakai langsung di 6+ titik tanpa jalur override. PKCE sudah benar dipakai (bukan kelalaian keamanan), tapi SEMUA instalasi (1 atau 100.000) memukul kuota OAuth app yang SAMA di sisi provider — risiko operasional kalau produk open-source dan basis pengguna membesar: provider bisa mendeteksi pola abuse dan membatasi/mencabut app, mematikan OAuth untuk SEMUA pengguna sekaligus.

**Scope:** Tambah field config opsional (`[auth.xai].client_id`, `[auth.openai].client_id` di `clawcrew-config/src/schema.rs`, `#[serde(default)]`) yang kalau diisi, OVERRIDE konstanta hardcoded. Default tetap pakai `client_id` Galleon (pengguna awam tidak perlu daftar OAuth app sendiri). Reuse pola yang SUDAH ADA untuk Gemini/Email OAuth2 di crate yang sama (mereka sudah terima `client_id` sebagai parameter fungsi, bukan konstanta) — jangan desain baru dari nol.

**File ownership:** Backend-Rust — `crates/clawcrew-providers/src/auth/xai_oauth.rs`, `openai_oauth.rs`, `crates/clawcrew-config/src/schema.rs`.

**Definition of Done:** Operator bisa override `client_id` via config; default (tanpa override) tetap berfungsi seperti sekarang; test baru membuktikan override dipakai saat diisi, konstanta dipakai saat kosong.

**Dependency:** Tidak ada blocker teknis — independent, bisa dikerjakan kapan saja.

**Risiko/Severity:** MEDIUM — bukan vulnerability aktif hari ini, tapi risiko operasional yang akan MEMBESAR seiring basis pengguna bertambah (relevan langsung dengan keputusan open-source yang Owner sedang pertimbangkan). Prioritas naik signifikan JIKA open-source diputuskan "ya".

---

## BL-26 — Dokumentasikan Model Keamanan Single-Tenant/Single-Machine (Auth)

**Rujukan riset:** `providers-reliability-auth-audit.md` §B6.

**Masalah:** Satu master key (`~/.clawcrew/.secret_key`) dan satu `auth-profiles.json` per instalasi — SEMUA Crew/Agent dalam satu instalasi berbagi kredensial OAuth yang sama. Field `AuthProfile.workspace_id` ada tapi tidak ditemukan dipakai sebagai filter isolasi aktif. Ini bukan bug untuk use-case single-user, tapi berisiko disalahgunakan di skenario multi-user-satu-server tanpa dokumentasi eksplisit.

**Scope:** Tambah 1 bagian eksplisit di dokumentasi operator/deployment guide: *"Galleon's OAuth credential storage is single-tenant per installation — all Crews/Agents share one encryption key and one token store. Do not deploy multiple untrusted users on one Galleon instance without additional isolation."* Murni dokumentasi, bisa dikerjakan sekarang.

**File ownership:** Siapa saja — dokumentasi deployment/operator guide, doc comment `AuthProfilesStore`/`SecretStore`.

**Definition of Done:** Disclosure eksplisit ada di dokumentasi operator-facing.

**Dependency:** Tidak ada.

**Risiko/Severity:** LOW implementasi, MEDIUM impact — mitigasi murah untuk ekspektasi yang harus jelas sejak awal.

---

## BL-27 — Perbaiki UX Pesan Error OAuth Refresh-Token-Revoked

**Rujukan riset:** `providers-reliability-auth-audit.md` §B3.

**Masalah:** `is_non_retryable_oauth_refresh_error()` mengklasifikasi `invalid_grant`/dll dengan benar (tidak retry sia-sia), tapi pesan yang di-propagate ke user adalah string error MENTAH dari provider, bukan instruksi jelas "jalankan `clawcrew auth login` lagi".

**Scope:** Di titik non-retryable refresh error terdeteksi (`auth/mod.rs:807-865` area), bungkus error asli dengan pesan tambahan yang actionable, mempertahankan error asli untuk debugging (`anyhow::Context` atau serupa) tapi menambah instruksi re-auth yang jelas di message utama.

**File ownership:** Backend-Rust — `crates/clawcrew-providers/src/auth/mod.rs`.

**Definition of Done:** Error yang di-tampilkan ke user mengandung instruksi konkret, bukan hanya string provider mentah. Test baru membuktikan format pesan.

**Dependency:** Tidak ada.

**Risiko/Severity:** LOW effort, akan jadi sumber keluhan/issue pertama dari kontributor open-source baru kalau tidak diperbaiki dulu — prioritaskan sebelum open-source, bukan sesudah.

---

## BL-28 — Client-Side Rate Limiting untuk LLM Provider Calls

**Rujukan riset:** `providers-reliability-auth-audit.md` §A2.

**Masalah:** Tidak ada mekanisme PROAKTIF yang membatasi laju request keluar dari Galleon ke provider LLM — hanya REAKTIF (cooldown setelah 429). Relevan dengan temuan sesi sebelumnya soal spawn sub-agent tanpa cap (BL-15/R-50) — banyak agent/sub-agent paralel bisa membanjiri provider API dan memicu 429 yang seharusnya bisa dicegah di muka.

**Scope:** Evaluasi dulu apakah ini perlu field terpisah atau bisa reuse `resource_limits` yang mungkin dibangun dari BL-15 (gate jumlah sub-agent paralel) — kalau BL-15 sudah membatasi jumlah sub-agent aktif, tekanan ke provider API mungkin otomatis berkurang tanpa perlu rate-limiter terpisah. Perlu keputusan desain: client-side token-bucket/semaphore per-provider, ATAU cukup reuse gate dari BL-15.

**File ownership:** TBD tergantung hasil evaluasi — Backend-Rust (`clawcrew-providers/src/reliable/mod.rs`) kalau diputuskan perlu mekanisme terpisah.

**Definition of Done (tahap evaluasi):** Keputusan: perlu rate-limiter client-side terpisah, atau cukup reuse BL-15.

**Dependency:** Sebaiknya setelah BL-15 (resource gate sub-agent) selesai, supaya evaluasi punya baseline yang jelas.

**Risiko/Severity:** MEDIUM — bukan gap kritis saat ini (cooldown reaktif sudah menangani), tapi akan jadi masalah skala kalau concurrent agent bertambah.

---

## BL-29 — Lengkapi Test Suite Reliability: Cascading-All-Down & Cooldown-Expiry

**Rujukan riset:** `providers-reliability-auth-audit.md` §A5.

**Masalah:** Test suite `reliable/tests.rs` kuat di billing/accounting dan fallback 2-3 provider, tapi tidak ada test untuk (a) semua provider gagal total (cascading failure murni, 3+ provider), (b) cooldown yang expire lalu provider dicoba lagi dan recover.

**Scope:** Tambah 2 kategori test baru menyusul pola existing di `reliable/tests.rs` (gunakan test harness yang sudah ada, jangan bangun baru).

**File ownership:** Backend-Rust — `crates/clawcrew-providers/src/reliable/tests.rs`.

**Definition of Done:** Minimal 1 test untuk skenario all-providers-exhausted (pesan error akhir yang jelas), minimal 1 test cooldown-expire-then-retry-succeeds.

**Dependency:** Tidak ada.

**Risiko/Severity:** LOW-MEDIUM — gap test, bukan bug aktif, tapi disarankan ditutup SEBELUM open-source karena komunitas akan menemukan edge case ini lewat bug report produksi kalau tidak diantisipasi.

---

## BL-30 — Satu Fungsi `erase_identity()` Lintas-Backend untuk Memory (Right-to-Delete)

**Rujukan riset:** `memory-data-governance-audit.md` §4, §5, §8, Kesimpulan.

**Masalah:** Tidak ada SATU entry point yang menyapu semua kategori (`conversation`+`daily`+`core`) + semua backend (SQLite+Postgres+Qdrant) + audit log (`memory_audit`) untuk SATU identitas sekaligus. Primitif (`purge_namespace`/`purge_session`/`purge_agent`) ada tapi terpisah-pisah, scope-nya "agent/alias" bukan "user", dan TIDAK membersihkan audit log maupun snapshot.

**Scope:**
1. Desain fungsi `erase_identity(agent_or_user_id)` di layer yang punya akses ke SEMUA backend terdaftar (bukan di `agent_scoped.rs` yang sengaja menolak cross-agent purge — butuh "admin Memory handle" yang disebut di komentar kode itu sendiri).
2. Fungsi ini memanggil `purge_agent` di SETIAP backend aktif (SQLite+Postgres+Qdrant kalau multi-backend dipakai), lalu `audit::prune_entries_for(agent_id)` (fungsi baru, scoped bukan time-based), lalu re-export snapshot kosong untuk agent tersebut.
3. Dokumentasikan eksplisit bahwa commit Git lama (kalau `MEMORY_SNAPSHOT.md` di-commit) tetap menyimpan jejak kecuali history di-rewrite — ini di luar kendali kode, perlu disclosure bukan fix teknis.

**File ownership:** Backend-Rust — kemungkinan `clawcrew-memory/src/` level baru (bukan di `agent_scoped.rs`), `clawcrew-tools/src/memory_purge.rs` (expose sebagai opsi tool baru atau extend existing).

**Definition of Done:** Satu panggilan menghapus data dari SEMUA backend aktif + audit + snapshot untuk satu identitas; test membuktikan tidak ada residu di backend manapun setelah panggilan.

**Dependency:** Tidak ada blocker teknis — infrastruktur dasar (`purge_agent` per-backend) sudah ada di ketiganya.

**Risiko/Severity:** MEDIUM-HIGH — ini bukan vulnerability aktif, tapi gap desain yang signifikan untuk produk yang menyimpan data personal lintas-sesi; makin penting kalau target pengguna makin luas (termasuk pertimbangan open-source/komunitas).

---

## BL-31 — Hygiene/Retensi Otomatis Backend-Agnostic (Bukan SQLite-Only)

**Rujukan riset:** `memory-data-governance-audit.md` §5.

**Masalah:** `hygiene.rs` (cron archive/purge berbasis umur, cadence 12 jam) HARDCODED membuka `brain.db` SQLite langsung via `rusqlite::Connection::open` — tidak ada cabang kode yang memanggil Postgres atau Qdrant client. Instalasi yang pakai Postgres/Qdrant TIDAK MENDAPAT retensi otomatis berbasis umur yang sama dengan instalasi SQLite.

**Scope:** Generalisasi `prune_category_rows`/`compact_category_to_budget` di `hygiene.rs`/`budget.rs` untuk bekerja lewat trait `Memory` (abstraksi yang sudah ada di `clawcrew-api::memory_traits`) alih-alih SQL langsung — supaya cadence hygiene yang sama berlaku untuk backend apa pun yang aktif, bukan hardcoded SQLite.

**File ownership:** Backend-Rust — `crates/clawcrew-memory/src/hygiene.rs`, `budget.rs`, kemungkinan perlu method baru di trait `Memory` (`clawcrew-api::memory_traits`) untuk ekspos "prune by age" generik yang bisa diimplementasi tiap backend.

**Definition of Done:** Instalasi Postgres/Qdrant mendapat retensi otomatis berbasis umur yang setara SQLite; test membuktikan prune-by-age bekerja di ketiga backend.

**Dependency:** Kemungkinan perlu desain trait baru dulu — evaluasi scope sebelum eksekusi, bisa lebih besar dari estimasi awal karena menyentuh kontrak trait yang dipakai banyak tempat.

**Risiko/Severity:** MEDIUM — menciptakan ketidakkonsistenan compliance/retensi antar-pilihan-backend yang seharusnya transparan bagi operator.

---

## Urutan Eksekusi yang Disarankan

1. **BL-26 dan BL-27 dulu** — murni dokumentasi/UX, biaya rendah, bisa selesai cepat, langsung mengurangi risiko sebelum keputusan open-source.
2. **BL-25** — fix teknis BYOC, biaya rendah-medium, prioritas naik signifikan kalau open-source diputuskan "ya".
3. **BL-30** — gap desain paling signifikan untuk data governance, worth dikerjakan independen dari keputusan open-source (relevan untuk SEMUA pengguna, bukan cuma komunitas).
4. **BL-29** — lengkapi test, bisa paralel dengan di atas.
5. **BL-31** — scope lebih besar (kemungkinan desain trait baru), setelah BL-30 selesai supaya pola `erase_identity` dan hygiene generik konsisten satu sama lain.
6. **BL-28** — tunggu BL-15 (resource gate sub-agent) selesai dulu supaya evaluasi punya baseline.

## Catatan Proses

Setiap item perlu diangkat ke `roadmap/*.md` (lane R, murni Rust) sebelum di-assign ke squad — folder `backlog/` bukan pengganti `roadmap/`, hanya staging area.
