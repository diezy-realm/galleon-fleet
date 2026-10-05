---
title: Audit Mendalam clawcrew-providers — Reliability & Auth/OAuth Lifecycle
status: research
date: 2026-10-05
author: squad-lead
depends_on: research-gap-map-2026-10-05.md
---

# Audit Mendalam `clawcrew-providers` — Reliability & Auth/OAuth

> Konteks dari Owner: crate ini belum pernah diriset mendalam, dan arah open-source/komunitas untuk produk belum diputuskan. Riset ini mencatat implikasi kesiapan open-source secara eksplisit, bukan hanya audit teknis biasa.

## A. Reliability

### A1. Retry/Backoff/Fallback & "Circuit-Breaker" (ADA, matang, bukan state-machine klasik)

Mekanisme cooldown in-memory per-entry berfungsi setara circuit-breaker meski bukan pola closed/open/half-open klasik:
- `ReliableModelProviderEntry` (`reliable/mod.rs:1712`) — field `cooldown_key` jadi identitas provider+alias.
- `rate_limit_cooldowns: Mutex<HashMap<String, Instant>>` — tabel cooldown in-memory.
- `provider_cooldown_active()`/`provider_should_skip_for_cooldown()` (~1912-1925) — entry yang masih cooldown di-skip otomatis, lanjut ke entry berikutnya.
- `set_rate_limit_cooldown()` (~1965-1995) — durasi dari header `Retry-After` (capped 60s), fallback 10s.
- Backoff: fixed/linear berbasis `Retry-After` (capped 30s), **bukan exponential backoff klasik**.

Test pembuktian nyata: `falls_back_after_retries_exhausted` (tests.rs:644, 2-provider), `pinned_rate_limit_and_cooldown_logs_use_each_served_model` (tests.rs:1108, 3-provider dengan shared cooldown_key skip).

**Catatan arsitektur penting**: komentar di baris ~1824 eksplisit: *"Per-model failover chains are also test-only — the schema no longer surfaces them"* — artinya ada mekanisme failover yang **hanya dipakai di test, tidak di produksi**. Perlu klarifikasi: apakah `fallback_models` config benar-benar jalan di jalur produksi, berbeda dari `model_fallbacks: HashMap` yang test-only.

### A2. Client-Side Rate Limiting — TIDAK ADA (gap)

Grep `Semaphore|governor|leaky|token_bucket|concurrency_limit` di `reliable/mod.rs` = 0 hasil. Satu-satunya "rate limiting" adalah REAKTIF (cooldown setelah 429 provider), tidak ada mekanisme PROAKTIF membatasi laju request keluar dari Galleon sendiri. **Risiko**: banyak agent/sub-agent jalan paralel (relevan dengan temuan sesi sebelumnya soal spawn sub-agent tanpa cap) bisa memicu 429 dari provider karena tidak ada local throttle sebelum request dikirim.

### A3. Diferensiasi Strategi Error — Granular dan well-engineered

| Kategori | Strategi |
|---|---|
| 401/403 (auth) | Non-retryable, langsung lompat provider lain |
| 429 (rate-limit) | Retryable + cooldown, KECUALI business hint ("insufficient quota") → non-retryable permanen |
| 5xx (server) | Retryable, hint switch provider |
| timeout/network | Retryable |
| context window exceeded | Retryable VIA TRUNCATION (bukan sekadar ulang) |
| tool schema error | Retryable via switch ke prompt-guided instructions |
| model refusal (Anthropic) | Non-retryable di entry sama, advance ke entry lain |

### A4. Transparansi Fallback ke User — Infrastruktur ada, konsumen belum diverifikasi

`SafeguardFallbackNotice` (3 kind: `ServerSide`/`ClientSide`/`ClientAndServer`) menyimpan `requested_model` vs `served_model` eksplisit — secara TEKNIS user BISA tahu kalau model dialihkan. Tapi konsumen aktual (apakah benar-benar dirender di CLI/chat/Slack) ada di crate LAIN yang tidak dibaca di audit ini — **perlu verifikasi lanjutan** di layer delivery/channel.

### A5. Kematangan Testing — Kuat di billing, lemah di cascading-failure

`reliable/tests.rs` ~305KB, 55+ test function. Porsi TERBESAR: accounting/billing correctness (`accounted_chat_outcome_retains_rejected_attempts_on_error`, dll) — masuk akal karena ini mempengaruhi uang. Fallback dasar 2-3 provider teruji baik.

**GAP TEST yang ditemukan** (nol hasil grep untuk nama fungsi terkait):
- Tidak ada test "semua provider down bersamaan" (3+ provider gagal total, cascading failure murni).
- Tidak ada test "cooldown expire lalu provider dicoba lagi dan recover" — hanya ada test cooldown aktif men-skip, bukan cooldown yang berakhir.

---

## B. Auth/OAuth Lifecycle

### B1. Storage Token — Satu abstraksi terpusat, encryption solid

`AuthProfilesStore` (`auth/profiles.rs:144`) membungkus `SecretStore`. SEMUA provider (OpenAI/Gemini/xAI) lewat jalur yang sama — tidak ada provider yang "roll kode sendiri". Lokasi: `state_dir.join("auth-profiles.json")`, co-located dengan `config.toml`.

Enkripsi: **ChaCha20-Poly1305 AEAD**, nonce random 12-byte, master key 256-bit di `~/.clawcrew/.secret_key` (permission 0600). Field yang dienkripsi: `access_token`, `refresh_token`, `id_token`, `token`. Bisa di-disable (`secrets.encrypt = false`, "for sovereign users who prefer plaintext" — komentar asli di kode).

### B2. Refresh — PROAKTIF (bukan reaktif)

Setiap `get_valid_*_access_token()` cek `is_expiring_within(90s)` SEBELUM dipakai — refresh dipicu di muka, bukan menunggu 401. Ada lock per-profile (cegah refresh ganda saat concurrent) dan backoff kalau refresh gagal (10s). Retry refresh call sendiri: linear delay (350ms × attempt), max 3 attempt.

### B3. Pesan Error Saat Refresh Token Revoked — GAP UX

`is_non_retryable_oauth_refresh_error()` mengklasifikasi `invalid_grant`/`invalid_client`/dll sebagai non-retryable (benar, tidak retry sia-sia). **TAPI** pesan yang di-propagate ke user adalah **string error mentah dari provider** (misal `"invalid_grant"`), BUKAN instruksi jelas "jalankan `clawcrew auth login` lagi". Satu-satunya pesan terkait re-auth yang ditemukan hanya log WARN internal, bukan user-facing. **Ini akan jadi sumber keluhan/issue pertama dari kontributor open-source baru.**

### B4. Revoke/Logout — Solid, benar-benar hapus disk

CLI `AuthCommands::Logout` → `AuthService::remove_profile()` → `AuthProfilesStore::remove_profile()` → `data.profiles.remove()` + tulis ulang `auth-profiles.json`. Bukan sekadar clear memori.

### B5. KRITIS — `client_id` OAuth HARDCODED (xAI, OpenAI)

```rust
// auth/xai_oauth.rs:16
pub const XAI_OAUTH_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
// auth/openai_oauth.rs:18
pub const OPENAI_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
```

Gemini dan Email OAuth2: `client_id`/`client_secret` **TIDAK** hardcoded (parameter fungsi, disuplai dari config/env). **Tidak ditemukan `client_secret` hardcoded** di manapun (grep khusus nihil).

**Analisis risiko open-source**: `client_id` publik untuk flow PKCE/native-app **secara desain OAuth2 memang boleh publik** (pola umum CLI/desktop tools — PKCE menggantikan kebutuhan secret rahasia). Risiko kebocoran data RENDAH. Tapi dua hal HARUS dipastikan sebelum open-source: (a) konfirmasi app OAuth xAI/OpenAI ini memang terdaftar sebagai "public client" bukan "confidential client" yang lupa di-set secretnya; (b) risiko OPERASIONAL — kalau ribuan instalasi komunitas pakai `client_id` yang SAMA, bisa memicu rate-limit/abuse detection di sisi provider (xAI/OpenAI), bukan risiko kebocoran tapi risiko availability.

### B6. Isolasi Multi-Tenant — TIDAK ADA

Tiga bukti: (1) `state_dir_from_config()` = parent `config.toml`, satu path per instalasi, bukan per-agent/crew; (2) master key juga satu per instalasi (`~/.clawcrew/.secret_key` global); (3) field `AuthProfile.workspace_id` ADA tapi tidak ditemukan dipakai sebagai filter isolasi aktif di jalur `select_profile_id`/`get_profile` (menyiratkan niat desain yang belum diimplementasi, perlu verifikasi lanjutan).

**Implikasi**: Semua Crew/Agent dalam SATU instalasi Galleon berbagi satu `auth-profiles.json` + satu master key. Model storage adalah **single-tenant/single-machine**, bukan multi-tenant-safe. Ini bukan bug untuk use-case single-user, tapi WAJIB didokumentasikan sebelum ada yang salah pakai di konteks SaaS/multi-user-di-satu-server.

---

## C. Pricing/Cost

`pricing.rs` **BUKAN tabel statis** seperti diasumsikan sebelum riset — ini **live-fetch dinamis**: sumber utama `/models` listing dari gateway provider (refresh tiap 1 jam), fallback `models.dev` catalog, dengan sanity-check harga (`is_sane_usd_rate`, cegah harga rusak dari API ter-cache). Docstring eksplisit: ini FALLBACK di bawah config operator `[cost.rates]` yang jadi prioritas utama (file config itu sendiri tidak dibaca di audit ini).

---

## Kesimpulan — Kesiapan Open-Source

Arsitektur auth/credential **cukup baik**: abstraksi `SecretStore` terpusat, ChaCha20-Poly1305 AEAD, proactive refresh, logout yang benar-benar hapus disk, klasifikasi error granular. Tapi **tiga hal harus diberesi/diverifikasi dulu** sebelum dibuka ke komunitas:

1. **Konfirmasi status registrasi OAuth app** (xAI, OpenAI) — pastikan `client_id` yang hardcoded memang "public client" tanpa secret pendamping yang hilang. Siapkan mitigasi untuk risiko rate-limit/abuse kalau ribuan instalasi komunitas pakai `client_id` yang sama (opsi: model "bawa client_id sendiri" untuk kontributor serius).
2. **Dokumentasikan eksplisit model keamanan single-machine/single-tenant** — komunitas perlu tahu semua profile OAuth di satu instalasi berbagi satu kunci; kalau mereka deploy multi-user di satu server, token satu user bisa dibaca proses apa pun yang punya akses ke state directory.
3. **Perbaiki UX pesan error refresh-token-revoked** — ganti string error mentah provider dengan instruksi jelas "jalankan `clawcrew auth login`". Ini akan jadi sumber issue GitHub pertama dari kontributor baru kalau tidak diperbaiki dulu.

Non-blocking tapi disarankan: lengkapi test suite dengan skenario cascading-all-providers-down dan cooldown-expiry-then-recover sebelum komunitas menemukannya lewat bug report produksi.

---

## Sumber

- `crates/clawcrew-providers/src/reliable/mod.rs` (`ReliableModelProviderEntry`, cooldown mechanism, error classification, `ProviderFallbackInfo`)
- `crates/clawcrew-providers/src/reliable/tests.rs` (55+ test, kategorisasi)
- `crates/clawcrew-providers/src/auth/mod.rs`, `profiles.rs`, `oauth_common.rs`, `xai_oauth.rs`, `openai_oauth.rs`, `gemini_oauth.rs`, `email_oauth2.rs`
- `crates/clawcrew-config/src/secrets.rs` (`SecretStore`, ChaCha20-Poly1305, master key location)
- `crates/clawcrew-providers/src/pricing.rs`, `safeguard_notice.rs`
- `src/main.rs:3467,9224-9230` (CLI `AuthCommands::Logout`)
