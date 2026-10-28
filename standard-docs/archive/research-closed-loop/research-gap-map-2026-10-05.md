---
title: Peta Topik Riset — Apa yang Sudah Dicakup, Apa yang Belum (Gap Analysis)
status: research
date: 2026-10-05
author: squad-lead
depends_on: null
---

# Peta Topik Riset — Gap Analysis untuk Prioritas Riset Berikutnya

> Permintaan Owner: tinjau ulang semua riset sesi ini, identifikasi topik yang BELUM tersentuh, rekomendasikan mana yang layak dimatangkan jadi riset+backlog. Dokumen ini murni peta/rekomendasi — belum ada audit kode mendalam di sini (itu scope riset LANJUTAN, bukan dokumen ini).

---

## 1. Inventaris — Yang SUDAH Dicakup Sesi Ini

| # | Topik | Dokumen | Kedalaman |
|---|---|---|---|
| 1 | Agent/Sub-agent permission scoping (survei kompetitor) | `agent-subagent-permission-scoping-survey.md` | Dalam |
| 2 | Agent→Squad→Ship permission scheme (kode nyata) | `agent-subagent-ship-squad-permission-scheme.md` | Dalam |
| 3 | Skills/Lessons/Steering (3-layer loading, memory Go vs Rust) | `capabilities-skills-lessons-steering-audit.md` | Dalam |
| 4 | SOP/Workflow/RACI/Role | `orders-quest-sop-workflow-role-audit.md` | Dalam |
| 5 | Automation/Cron/Webhook + prompt-injection channel gap | `automation-cron-webhook-audit.md` | Sangat dalam (3 tahap verifikasi) |
| 6 | Sub-agent resource management (memory/RAM/zombie) | `subagent-resource-management-audit.md` | Sangat dalam |
| 7 | Plugin WASM/WIT vs MCP architecture | `plugin-mcp-architecture-audit.md` | Sangat dalam |
| 8 | Fleet-wide MCP governance (follow-up) | `fleet-wide-mcp-governance-followup.md` | Dalam |
| 9 | Verifiable Intent (assessment lama, sudah ada sebelum sesi ini) | `verifiable-intent-assessment.md` | Ringan |

**Pola yang sudah terbentuk** dari ke-8 riset baru: audit kode langsung (file:baris) → cross-check kompetitor → cross-check `finalize/dev-tasks/` → backlog BL bernomor → sebagian diangkat ke `roadmap/R-rust.md`. Total BL-01 s.d. BL-24 sudah tercatat.

**Benang merah lintas-dokumen yang MUNCUL BERULANG** (pola, bukan topik baru — penting untuk riset lanjutan supaya tidak mengulang):
- Go dan Rust berulang kali punya DUA sistem paralel untuk konsep yang sama tanpa jembatan RPC (memory: Go in-memory vs Rust persistent; MCP: dulu ada di Go sekarang Rust-only; delegasi sub-agent: `DelegateTaskTool` Go vs `DelegateTool` Rust vs `ExecuteTurn` langsung).
- Proto `SystemGateway` cuma 3 RPC (`ExecuteTurn`/`ExecuteNativeTool`/`GetDecryptedSecret`) — ini bottleneck berulang setiap kali sebuah topik butuh Go dan Rust saling tahu sesuatu.
- Mekanisme keamanan/sandboxing TIDAK konsisten rigor-nya antar subsistem yang risikonya sejenis (WASM plugin sangat ketat, MCP longgar, native tool shell ketat).

---

## 2. Area Besar yang BELUM Tersentuh Sama Sekali

Hasil penyisiran `crates/` (27 crate) dan `standard-docs/product/` — area dengan infrastruktur SUBSTANSIAL (bukan skeleton) yang belum pernah diaudit sepanjang sesi ini:

### 2.1 `galleon-providers` — LLM Provider SSOT (CRATE TERBESAR, BELUM TERSENTUH SAMA SEKALI)
Crate ini adalah **jantung arsitektur** menurut ADR 0001 (LLM SSOT di Rust) — tapi belum pernah diriset sedikit pun di sesi ini. Temuan permukaan dari penyisiran cepat:
- `lib.rs` 238KB, `openai_codex.rs` 104KB, `openai.rs` 109KB, `bedrock.rs` 108KB, `multimodal.rs` 149KB — ukuran file yang jauh melebihi standar (`rust-module-size-and-decomposition-standard.md` sudah dicatat di roadmap R-17 sebagai technical debt umum, tapi belum ada audit KHUSUS provider).
- `reliable/mod.rs` (302 match retry/fallback/rate_limit) — mekanisme reliability yang KOMPLEKS, belum diverifikasi apakah testing-nya memadai untuk production (ironisnya file test-nya `reliable/tests.rs` punya 522 match — rasio test:kode yang tinggi, kemungkinan sudah matang, tapi belum pernah dibaca).
- `auth/` submodule dengan OAuth per-provider (`oauth_common.rs`, `xai_oauth.rs`, `gemini_oauth.rs`, `openai_oauth.rs`, `email_oauth2.rs`) — token lifecycle, refresh, revocation BELUM pernah diaudit. Ini area berisiko tinggi (credential handling) yang belum tersentuh.
- `pricing.rs` (34KB) — kalkulasi biaya LLM, relevan untuk cost-cap yang disinggung BL-13 (automation backlog) tapi belum pernah dihubungkan.
- **Kenapa layak diriset**: ini SSOT yang menurut ADR 0001 adalah keputusan arsitektur eksplisit (Go TIDAK boleh punya LLM call sendiri) — kegagalan/kerentanan di sini berdampak ke SELURUH sistem, bukan satu fitur. Rasio effort:dampak sangat tinggi.

### 2.2 Identity, Pairing & Vault — Lintas-Tier Credential Lifecycle (BELUM TERSENTUH)
- `crates/galleon-runtime/src/enroll/` (`mod.rs`, `paircode_admin.rs`) — mekanisme pairing/QR code yang menghubungkan Tauri Desktop Shell ↔ Go Engine ↔ Rust Core. Ini jalur ONBOARDING pengguna baru — belum pernah diverifikasi keamanannya (apakah pairing code bisa di-brute-force, expiry, replay protection).
- `crates/galleon-config/src/secrets.rs` (`SecretStore`) — SUDAH disinggung berulang di riset lain (BL-21 MCP, cron secrets) sebagai "pola yang lebih matang," tapi **belum pernah diaudit langsung** sebagai subjek utama. Apakah benar-benar matang, atau asumsi berulang yang belum diverifikasi?
- **Kenapa layak diriset**: pairing/vault adalah titik SATU-SATUNYA yang menghubungkan ke-4 tier (Tauri/Rust/Go/Web) secara otentikasi — kerentanan di sini = kompromi seluruh sistem. Sering diasumsikan "sudah matang" di riset lain tanpa pernah diverifikasi langsung.

### 2.3 Data Governance Memory — Retention, Privacy, Right-to-Delete (SEBAGIAN TERSENTUH, PERLU DIPERDALAM)
`capabilities-skills-lessons-steering-audit.md` sudah menyentuh memory TAPI fokusnya ke "Go in-memory vs Rust persistent" (temuan arsitektur), BUKAN ke governance data personal. Penyisiran baru menemukan `galleon-memory` punya modul `hygiene.rs`, `policy.rs`, `audit.rs` yang **belum pernah dibaca isinya** — ini kemungkinan jawaban untuk pertanyaan privasi (user minta hapus semua data, retention otomatis, dll) yang relevan untuk produk yang menyimpan percakapan + "deepening model of who you are" (istilah dari riset Hermes Agent sebelumnya — pola umum di seluruh kompetitor, termasuk Galleon).
- **Kenapa layak diriset**: produk yang menyimpan memori personal LINTAS SESI punya kewajiban privasi (mirip prinsip GDPR "right to erasure") yang belum pernah diverifikasi punya mekanisme nyata di Galleon.

### 2.4 Channel Tier A/B/C — Keamanan Spesifik per-Protokol (TERSENTUH SEPINTAS, BELUM MENDALAM)
Riset automation/cron menemukan gap sanitasi di `process_channel_message_body` (BL-14, sudah tertutup satu titik perbaikan untuk semua channel). TAPI belum ada riset KHUSUS yang membandingkan risiko SPESIFIK per-protokol channel: `galleon-channels` adalah crate 198k LOC (menurut roadmap R-15) dengan puluhan channel (Telegram, Discord, WhatsApp, QQ, WeChat, Signal, iMessage, Bluesky, dll) — masing-masing punya model keamanan native BERBEDA (contoh: WhatsApp Business API webhook signature vs Telegram bot token vs Matrix E2E encryption). Riset BL-14 menutup gap SATU TITIK (prompt-injection), tapi tidak membahas apakah tiap channel plugin memverifikasi signature/origin request dari provider masing-masing dengan benar.
- **Kenapa layak diriset**: permukaan serangan TERBESAR secara LOC (198k) tapi granularitas riset sejauh ini masih di level "satu titik orchestrator," belum per-channel.

### 2.5 CI/CD, Build Supply-Chain & Dependency Security (BELUM TERSENTUH SAMA SEKALI)
Tidak ada satu pun riset sesi ini yang menyentuh: bagaimana release dibuat (`clawcrew-dist`), apakah dependency di-pin dengan benar di seluruh 27 crate, apakah ada SBOM/dependency scanning, bagaimana binary ditandatangani untuk distribusi (code-signing Tauri app untuk macOS/Windows). Ini relevan karena riset plugin/MCP sudah menyentuh "supply-chain trust" untuk PLUGIN pihak ketiga (BL-19/22), tapi belum untuk BINARY GALLEON SENDIRI.
- **Kenapa layak diriset**: supply-chain attack terhadap tooling AI-agent adalah vektor yang makin disorot industri (mirip insiden xz-utils) — belum ada verifikasi bagaimana Galleon sendiri melindungi rantai build→distribusi.

### 2.6 Observability/Telemetry — Siapa Melihat Apa (BELUM TERSENTUH)
Riset resource-management menyentuh `process_stats`/dashboard Tasks-view SECARA SEMPIT (untuk kasus loop-detector). Belum ada riset menyeluruh soal: apa yang di-log/di-trace secara default, apakah ada PII yang bocor ke log (`galleon-log` crate ada scrubbing kredensial menurut `rust-conventions`/roadmap R-23, tapi belum diverifikasi langsung), siapa yang punya akses ke log tersebut lintas-tier.

---

## 3. Rekomendasi Prioritas (Ranking, Bukan Sekadar Daftar)

| Prioritas | Topik | Alasan Ranking |
|---|---|---|
| **1** | **`galleon-providers` LLM SSOT** | Crate terbesar, SSOT inti arsitektur (ADR 0001), belum tersentuh SAMA SEKALI meski jadi dependency SETIAP fitur lain yang sudah diriset (semua turn/sub-agent akhirnya lewat sini). Auth/OAuth lifecycle di dalamnya adalah credential-handling berisiko tinggi. |
| **2** | **Identity/Pairing/Vault lintas-tier** | Satu-satunya titik yang menghubungkan otentikasi ke-4 tier; sering diasumsikan "sudah matang" di riset lain (secrets, cron, MCP) TANPA PERNAH diverifikasi langsung — risiko "asumsi berantai" tanpa dasar. |
| **3** | **Data governance memory (retention/privacy)** | Modul `hygiene.rs`/`policy.rs`/`audit.rs` sudah ADA tapi isinya belum dibaca — kemungkinan quick-win (infrastruktur sudah dibangun, tinggal diverifikasi dan didokumentasikan) dibanding topik lain yang mungkin dari nol. |
| **4** | **Channel Tier A/B/C keamanan per-protokol** | Permukaan serangan terbesar (198k LOC), tapi granularitas riset BL-14 sudah menutup gap PALING kritis (prompt-injection) — sisanya adalah hardening lanjutan, bukan gap darurat. |
| **5** | **CI/CD & supply-chain** | Penting untuk kematangan produk jangka panjang, tapi dampak langsung ke runtime risk lebih rendah dibanding 1-3 (ini soal "bagaimana Galleon dibangun," bukan "apa yang Galleon lakukan saat berjalan"). |
| **6** | **Observability/log PII scrubbing** | Scope lebih kecil, kemungkinan besar sudah tertutup pola `galleon-log` yang konsisten (sudah terlihat sepintas di riset lain) — perlu verifikasi tapi effort lebih kecil dari 1-4. |

---

## 4. Rekomendasi Konkret untuk Langkah Berikutnya

Bukan semua topik di atas perlu riset mendalam SEKARANG — mengikuti pola sesi ini (satu topik per sesi, audit kode mendalam, bukan permukaan), saya sarankan:

1. **Mulai dari #1 (`galleon-providers`)** kalau Owner ingin topik dengan dampak arsitektur terluas — tapi scope-nya BESAR (238KB `lib.rs` saja), kemungkinan perlu dipecah jadi beberapa sesi (reliability/retry dulu, lalu auth/OAuth, lalu pricing/cost).
2. **Atau mulai dari #2 (Identity/Pairing/Vault)** kalau Owner ingin menutup "asumsi berantai" yang sudah berulang kali disebut tapi belum diverifikasi — scope lebih terkontain (2-3 file inti), kemungkinan selesai 1 sesi.
3. **#3 (data governance memory)** adalah **quick-win candidate** — infrastrukturnya sudah ada (`hygiene.rs`/`policy.rs`/`audit.rs`), riset ini mungkin lebih cepat dari yang lain karena tinggal MEMBACA dan MENDOKUMENTASIKAN, bukan menemukan gap dari nol.

Saya tidak akan memilih salah satu secara sepihak — ini keputusan prioritas yang baiknya datang dari Owner, karena menyangkut alokasi sesi riset berikutnya, bukan sesuatu yang bisa disimpulkan murni dari kode.

---

## 6. Cross-Check ke `finalize/dev-tasks/` (read-only, atas permintaan Owner) — REVISI PRIORITAS

Dibaca `01-be-go-engine.md`, `02-be-rust-core.md`, `03-fe-web.md`, `08-carryover-from-finalize.md`, `README.md` secara menyeluruh untuk ketiga topik kandidat §4. Hasilnya **mengubah ranking** — satu topik ternyata sudah punya rencana rinci, satu topik cuma tersentuh permukaan, satu topik belum tersentuh sama sekali.

### 6.1 Topik #2 (Identity/Pairing/Vault) — SUDAH ADA RENCANA RINCI, JANGAN DIRISET DUPLIKAT

`01-be-go-engine.md` bagian C ("Identitas, Pairing, Akses — QR sudah ada, otorisasi belum") sudah berisi breakdown konkret:
- **BG-C1** (P1, L): "Identity & akses di Go: principal resolver (deny-by-default), WebAuthn, pairing. *Acceptance:* tanpa kredensial → ditolak; test negatif."
- **BG-C2** (P1, L): "Pairing perangkat via QR berotorisasi: token sekali pakai terikat perangkat, expiry, daftar, revoke, rate-limit." Dengan catatan tajam yang SUDAH diketahui tim: *"`/api/network/qrcode` saat ini **hanya render QR tanpa token**."*
- **BG-C3** (P1, M): "Autentikasi sesi/perangkat (middleware) untuk akses remote."
- Sisi Rust: **BR-B4/BR-B5** (scrubbing kredensial di log, rotasi/ekspor/migrasi vault) — juga sudah terjadwal `[ ]`.

**Kesimpulan: topik ini DICABUT dari rekomendasi riset lanjutan.** Bukan karena tidak penting (justru temuan `/api/network/qrcode` tanpa token itu sendiri signifikan), tapi karena breakdown-nya sudah ada dan cukup rinci — riset ulang di sini berisiko 100% duplikasi kerja tim dev-tasks. Kalau Owner ingin "memperdalam" topik ini, bentuknya BUKAN riset baru, tapi VERIFIKASI apakah implementasi BG-C1/C2/C3 (saat nanti dikerjakan) benar-benar menutup gap yang disebutkan — itu kerja REVIEW, bukan riset discovery.

### 6.2 Topik #1 (`galleon-providers`) — HANYA TERSENTUH PERMUKAAN, MASIH LAYAK DIRISET

Satu-satunya referensi: **BR-C8** (P3, M) — *"Audit `tools`/`config`/`providers` — tanpa pemecahan spekulatif."* Ini task GENERIK level-tinggi (bahkan judulnya cakup 3 crate sekaligus) yang isinya hanya "jangan pecah spekulatif" (prinsip YAGNI) — BUKAN audit reliability/retry/auth-OAuth/pricing yang saya usulkan di §2.1. Tidak ada satu pun item dev-tasks yang menyentuh `reliable/mod.rs`, OAuth lifecycle per-provider, atau `pricing.rs` secara spesifik.

**Kesimpulan: topik ini TETAP LAYAK, prioritas naik.** Gap riset genuine — dev-tasks hanya punya placeholder generik P3 ("jangan pecah dulu"), bukan audit kedalaman apa pun untuk reliability/credential-handling yang justru paling berisiko.

### 6.3 Topik #3 (Data governance memory) — TERSENTUH DARI SUDUT BERBEDA (Consent, bukan Retention/Privacy)

`BG-F4` (P2, L): *"Learning by consent lanjutan: tipe `MemoryProposal` sudah ada (T-3.2 ✔); tinggal alur umpan balik → persetujuan Owner → versi/rollback di atas store baru."* Plus `FW-C5` (UI persetujuan `MemoryProposal`).

**Ini topik yang BERBEDA dari yang saya usulkan.** `BG-F4` adalah tentang mekanisme CONSENT untuk Agent mengusulkan pembelajaran baru (sudah diriset mendalam di `capabilities-skills-lessons-steering-audit.md` sesi ini — "Learning by Consent" di `engine/src/memory/proposal.go`). Yang saya usulkan di §2.3 adalah **retention/right-to-delete/privacy** untuk data yang SUDAH tersimpan (`hygiene.rs`/`policy.rs`/`audit.rs` di `galleon-memory` Rust) — ini sudut yang BERBEDA, tidak tumpang tindih.

**Kesimpulan: topik ini TETAP LAYAK, tidak ada overlap.** `BG-F4` adalah "bagaimana data BARU masuk dengan consent," bukan "bagaimana data LAMA dihapus/dipertahankan sesuai privasi." Dua pertanyaan berbeda, dev-tasks hanya menjawab yang pertama.

### 6.4 Ranking Final (REVISI dari §3)

| Prioritas | Topik | Status setelah cross-check |
|---|---|---|
| **1** | **`galleon-providers` reliability + auth/OAuth lifecycle** | ✅ Gap genuine, dev-tasks hanya placeholder P3 generik |
| **2** | **Data governance memory (`hygiene.rs`/`policy.rs`/`audit.rs`) — retention/privacy, BUKAN consent** | ✅ Gap genuine, berbeda dari `BG-F4` yang sudah ada |
| ~~~~ | ~~Identity/Pairing/Vault lintas-tier~~ | ❌ **DICABUT** — sudah ada breakdown rinci BG-C1/C2/C3, riset ulang = duplikasi |
| 3 | Channel Tier A/B/C keamanan per-protokol | Belum di-cross-check — perlu verifikasi lanjutan sebelum masuk rencana final |
| 4 | CI/CD & supply-chain | Belum di-cross-check |
| 5 | Observability/log PII scrubbing | Sebagian tersentuh (`BR-B4` scrubbing kredensial log) — kemungkinan sebagian sudah terencana, perlu cross-check lanjutan kalau mau diangkat |

---

## 7. Sumber (tambahan §6)

- `standard-docs/decisions/finalize/dev-tasks/01-be-go-engine.md` bagian C (BG-C1/C2/C3/C4a/C4b) dan F (BG-F4) — dibaca read-only
- `standard-docs/decisions/finalize/dev-tasks/02-be-rust-core.md` (BR-B2/B4/B5/C8)
- `standard-docs/decisions/finalize/dev-tasks/03-fe-web.md` (FW-C4/C5)
- `standard-docs/decisions/finalize/dev-tasks/08-carryover-from-finalize.md` (status T-2.1-2.3, T-3.2, T-8.1-8.2)
