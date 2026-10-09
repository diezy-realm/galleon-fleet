# R-00a — Analisis 3 Keputusan Terbuka Inventaris Runtime

> Pelengkap `R-00-runtime-inventory.md` §6. Status: **DISETUJUI Owner 2026-10-04 (ODR-21..25)**. Tidak ada kode diubah. Detail `verifiable_intent`: `../../../archive/research-closed-loop/verifiable-intent-assessment.md`.
> Penyelarasan produk: ODR-03 (organisasi AI, Go = otak), ODR-07 (Go = security layer), ODR-06 (local-first + remote via Nginx/QR), ODR-12 (CLI Rust, klien Go), invarian F1 (Treasury read-only, tanpa perantara tagihan/komersial).

---

## 1. `verifiable_intent` (4.616 LOC)

**Fakta**
- Modul `crypto`, `issuance`, `types`, `verification`: implementasi spesifikasi *Verifiable Intent* (kredensial/mandat untuk aksi komersial agent).
- Komentar konfigurasi (`clawcrew-config/src/schema.rs` ±5771): "implements issuance, crypto, types and constraint checking, **but not a credential chain verifier**… tool `vi_verify` **withheld** dari registry model-visible". Aktifkan `[verifiable_intent]` hanya **menghasilkan peringatan**.
- Pemakai kode: `src/main.rs` (`warn_verifiable_intent_withheld` ×2) + deklarasi `mod`. Tidak ada konsumen di crate lain, Go, `web/`, maupun dokumen produk (grep `standard-docs/product`, `.kiro/steering`: nol).
- Produk Galleon: BYOK/BYOM, **Treasury hanya melapor**, tanpa perantara tagihan (F1). Tidak ada fitur perdagangan/mandat pembayaran di PRD.

**Analisis:** fitur setengah jadi (verifier tidak ada, tool disembunyikan), tanpa pemakai, dan tidak tercakup visi produk. Menyimpannya = 4,6k LOC kripto yang harus dirawat dan diaudit tanpa nilai. Risiko hapus rendah; satu-satunya efek: kunci konfigurasi `[verifiable_intent]` dan `vi_*` tool hilang.

**Keputusan (ODR-21): HAPUS** setelah penilaian konsep untuk masa depan (`archive/research-closed-loop/verifiable-intent-assessment.md`). Hapus modul, tool `vi_*`, `VerifiableIntentConfig`, peringatan di `main.rs`/doctor; konfigurasi lama yang masih memuat bagian ini harus **diabaikan dengan peringatan** satu kali (bukan error) saat migrasi config. Pulihkan dari git tag `archive/pre-galleon` bila kelak dibutuhkan (agentic commerce = fitur baru dengan ADR sendiri).
Tiket: **R-38**.

---

## 2. Identitas, principal, WebAuthn di `runtime::security`

**Fakta (isi file)**
| File | LOC | Isi | Status pemakaian |
|---|---|---|---|
| `security/principal_resolver.rs` | 859 | RFC 7141: memetakan identitas terverifikasi → `Principal` + grants (profile_map OIDC, roster `[users.*]`, `[permission_profiles]`), deny-by-default, re-resolve saat generasi policy berubah | dipakai gateway (`state.rs`, `api*.rs`) |
| `security/auth_provider/{mod,oidc}.rs` | 3.059 | seam autentikasi masuk (OIDC, peer-uid, native pairing bearer) | komentar kode: **"no production call sites yet"** |
| `security/webauthn.rs` | 1.260 | verifikasi FIDO2/ES256, memakai `SecretStore` | dipakai `gateway/api_webauthn.rs` |
| `security/cert_ledger.rs` | 3.335 | ledger sertifikat mTLS & pairing `devices.db` (`token_hash`), enrollment | terkait relay/enroll |
| `security/{seatbelt,landlock,leak_detector,audit,detect,…}` | ±10k | sandbox OS, deteksi kebocoran, audit | inti System Core |
- **Go tidak punya autentikasi/otorisasi sama sekali** (grep `webauthn|principal|authz|device` di `engine/src`: nol).

**Analisis terhadap produk**
- Siapa yang boleh memerintah Quartermaster (user, perangkat QR, remote via Nginx) adalah **keputusan kebijakan akses produk** → ODR-07: Go = security layer; policy SSOT di Go (`fleet/policies.go`); QR pairing/token perangkat sudah dijadwalkan di Go (G-20/G-21).
- Rust adalah **leaf service**: yang perlu diautentikasi di Rust hanya satu hal — *pemanggil adalah Go engine lokal* (loopback + token/peer-uid/mTLS). Model multi-user OIDC/roster/profil di Rust menduplikasi policy (dua sumber kebenaran, melanggar SSOT) dan tidak punya jalur masuk yang sah (UI/CLI → Go).
- Mekanisme kripto/sandbox/vault/audit/leak tetap Rust (kriptografi + OS = kekuatan Rust).

**Rekomendasi (pemisahan mekanisme vs kebijakan):**
| Komponen | Keputusan |
|---|---|
| `principal_resolver` | **MOVE→Go** (pola RFC 7141 dipertahankan sebagai desain: provider → identitas, resolver → grants, deny-by-default, generasi policy) |
| `auth_provider` (OIDC/pairing bearer) | **MOVE→Go**; di Rust sisakan **satu provider**: autentikasi pemanggil Go (loopback token/peer-uid) |
| `webauthn` | **MOVE→Go** (ceremony & penyimpanan kredensial di store Go; pustaka `go-webauthn`); Rust tidak lagi memverifikasi |
| `cert_ledger` | **KEEP** untuk jalur `galleonrelay`/mTLS; keputusan relay vs Nginx di P-11/R-36. Bila relay dipensiunkan: ikut dihapus |
| sandbox, `leak_detector`, `audit`, `SecretStore`/Vault | **KEEP** |
Tiket: **G-35** (identity/principal/WebAuthn/pairing di Go; dasar G-20/G-21), **R-39** (Rust: autentikasi pemanggil Go + hapus provider/resolver setelah G-35).
Urutan: G-35 dulu; Rust memakai warisan sampai Go siap (strangler).

---

## 3. `rpc` (32.105 LOC) & penghapusan

**Fakta**
- Isi: `dispatch` 18,2k, `wss` 2,5k, `session` 2,1k, `local` 2,1k, `turn` 1,8k, `types` 1,9k, `acp` 0,6k, `attachments`, `approval_channel`, `fs`, `git`, `locales`, `tui_identity`.
- Konsumen eksternal: `apps/zerocode` (TUI **77.241 LOC**, `client.rs` 6.423 baris, mem-parse JSON-RPC, dan **bahkan menjalankan daemon Rust ephemeral sendiri** `spawn_owned_ephemeral_daemon`), 5 file `clawcrew-gateway`, `src/main.rs`.
- Ada biner `clawcrew-acp-bridge` (605 baris) → protokol **ACP (Agent Client Protocol)** untuk integrasi editor/IDE.
- Artinya: hari ini CLI/TUI **menjalankan agent di Rust** dan bicara ke daemon Rust, bukan ke Go.

**Analisis terhadap produk**
- ODR-12: CLI = binary Rust, klien Go. Maka `galleoncode` harus berbicara ke Go. Menghapus `rpc` tanpa pengganti = mematikan TUI (77k LOC aset) dan ACP/IDE.
- Mengganti seluruh protokol TUI ke HTTP baru = menulis ulang `client.rs` (6,4k) + semua pemanggil. Alternatif lebih murah: **pertahankan kosakata JSON-RPC 2.0** (method `session/*`, `turn/*`, approval, dll.) sebagai **"Galleon Client Protocol"**, tetapi **servernya dipindah ke Go** (endpoint WS di :9090, di belakang Nginx/QR). TUI hanya mengganti transport/endpoint dan memetakan method yang berubah; ACP bridge menunjuk ke Go yang sama.
- Isi `rpc` yang dihapus: `dispatch`, `turn`, `session`, `local`, `wss`, `approval_channel` (otak/transport Rust). Dipindah sebagai fungsi: `fs`/`git` → tool native lewat `ExecuteNativeTool` (dipanggil Go); `attachments`, `locales`, `types` → kontrak di `api-contracts/` + implementasi Go.

**Rekomendasi: urutan terakhir TETAP benar, dengan syarat tertulis:**
1. Tulis spesifikasi **Galleon Client Protocol v1** (`tech-standart/api-contracts/client-protocol.md`) dari `rpc/types.rs` + `acp.rs` — SSOT, versi, daftar method, auth (G-35).
2. Go mengimplementasi server WS/JSON-RPC (G-36) di atas `StartTurn`/store/approval.
3. `galleoncode` + `acp-bridge` dialihkan (R-36, R-40); matikan `spawn_owned_ephemeral_daemon` agent-mode (Rust daemon hanya sebagai gateway sistem).
4. Parity: `galleon-eval` + test kontrak protokol hijau.
5. Hapus `rpc` + REST/WS/A2A gateway lama (R-14, R-41).
Tiket: **G-36** server protokol, **R-40** retarget TUI/ACP, **R-41** hapus `rpc`.

---

## 4. Temuan tambahan dari verifikasi (memperbarui `R-00`)

| Modul | Fakta | Klasifikasi baru |
|---|---|---|
| `identity` (1,3k) | persona agent format OpenClaw/AIEOS | **MOVE→Go** (persona Crew; pertahankan format impor) |
| `control_plane` (6,9k) | "durable run/task control-plane — lifecycle delegated & subagent runs" | **MOVE→Go** (= `run` Go) |
| `subagent` (0,3k) | sub-agent efemer mewarisi identitas/policy induk, tanpa konsumen eksternal | **MOVE→Go / hapus** (diserap G-33) |
| `peers` (0,1k) | resolusi peer-group per agent per channel | tetap sementara (dipakai channels); evaluasi di R-13 |
| `runtime/tools` vs crate `clawcrew-tools` | **tidak ada duplikasi nama file** (hanya `mod.rs`, `attribution.rs`); `runtime/tools` = tool yang butuh runtime (`delegate`, `skill_*`, `cron_add`, `schedule`, `shell`, `file_read`…), crate `tools` = 96 file implementasi | bagi: `delegate`/`skill_*`/`cron_*`/`schedule` → MOVE; `shell`/`file_*`/native → pindah ke `clawcrew-tools` (KEEP) |
| `integrations`, `hooks`, `trust`, `calendar` | `mod.rs` tanpa dokumentasi modul; perlu baca isi | tetap DECIDE, selesaikan saat R-00 verifikasi per-modul (§4 R-00) |

---

## 5. Ringkasan rekomendasi & dampak roadmap

| # | Keputusan | Rekomendasi | Tiket baru |
|---|---|---|---|
| 1 | `verifiable_intent` | **Hapus** (tak selesai, tak dipakai, di luar visi) | R-38 |
| 2 | Identitas/principal/WebAuthn | **Policy & identitas → Go**; Rust hanya autentikasi pemanggil Go + sandbox/vault/audit; `cert_ledger` ikut relay | G-35, R-39 |
| 3 | `rpc` | **Hapus terakhir**, setelah **Galleon Client Protocol v1** dilayani Go dan TUI/ACP dialihkan | G-36, R-40, R-41 |
