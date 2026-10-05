---
title: Laporan Strategis Pasar — Industri, Positioning, dan Prioritas Backlog (Prodcut-Owner)
status: research
date: 2026-10-05
author: squad-product-owner (crew Prodcut-Owner)
depends_on: product-industry-competitor-insight.md
---

> **Catatan verifikasi**: dokumen ini dari delegasi PERTAMA ke crew `Prodcut-Owner` (sempat terlihat gagal register ~20+ menit sebelum muncul di tracking — ternyata berhasil). Konten diperkaya dari artifact penuh `galleon-fleet-market-strategic-report-oct2026` yang disimpan sub-agent sendiri (ringkasan chat awal tidak lengkap). Dua klaim paling spesifik (Trinity by Ability.ai, GitHub Copilot Dynamic Workflows 1 Okt 2026) sudah **diverifikasi ulang via web search independen** dan terkonfirmasi AKURAT. Klaim RCE `/api/engine/execute` dicek silang ke `finalize/dev-tasks/03-fe-web.md` — **sudah selesai diperbaiki** (`FW-A2a`, PR-04), bukan temuan baru.
>
> Berbeda dari artifact sibling `galleon-phase2b-competitive-analysis` (2026-10-02, scope UI components Phase 2b E4 — tool execution panel, approval card, memory dashboard — untuk Lead Squad/Design Crew). Dokumen ini scope level strategi produk & pasar.

# Galleon Fleet — Market Research & Strategic Positioning Report

**Scope:** (1) Industri & pemain kini, (2) Competitor positioning vs Galleon, (3) Backlog impact terhadap posisi produk.

## Bagian 1: Riset Industri — "Local-First AI Organization Workspace" Oktober 2026

### 1.1 Konteks: Pergeseran Paradigma yang Sedang Terjadi

Kategori "agentic AI workspace" secara aktif bergerak dari **single-assistant chatbot** ke **organisasi agent hierarkis** pada 2026 — ini sudah happening, bukan prediksi:

- **GitHub Copilot (Microsoft)**: 1 Oktober 2026, GitHub meluncurkan **Dynamic Workflows** di Copilot CLI, Copilot App, dan Copilot SDK — orkestrasi multi-agent sebagai kode (bukan chat). Akselerasi dari Copilot App (Build 2026, 2 Juni) sebagai standalone desktop app. Pricing: token-based AI Credits sejak 1 Juni 2026 ($0.01/credit). **(Terverifikasi independen: `github.blog/changelog/2026-10-01-dynamic-workflows...`)**
- **Trinity by Ability.ai**: v0.9.0 (17 Agustus 2026, 499 commit, 217 issues closed, 1.367 file changed) — *structural release*: departments + reporting lines + shared skills library + shared knowledge canon + permissions + staged autonomy + audit ledger. Self-hosted, Apache 2.0. **Platform paling dekat secara arsitektur dengan visi Galleon di sisi open-source enterprise.** **(Terverifikasi independen: `ability.ai/trinity`, `github.com/Abilityai/trinity`)**
- **OpenClaw 2.0**: v2026.8.1 (30 Agustus 2026, 16.977 PR dari 987 kontributor). Fitur baru: shared sessions, multiplayer workflows, cloud workers, rebuilt Control UI, automatic skill learning. MIT, free; cloud plan $59/bulan (promo $29.50 bulan pertama). 350K+ GitHub stars.
- **Hermes Agent (Nous Research)**: diluncurkan Februari 2026, 61.000 stars dalam ~2 bulan, kini 140K+. Native desktop app, Bot Mode, group chats antar-agent, 27+ messaging integrations, A2A support, self-evolving skills via GEPA prompt evolution. MIT. Fokus: satu agent yang terus belajar sendiri.
- **AgentTeams (AgentScope)**: open-source collaborative multi-agent runtime — multiple agent berkolaborasi di "room" terkontrol dengan full human visibility+intervention (`github.com/agentscope-ai/AgentTeams`). Arah baru Q3 2026: Matrix room-based coordination.

### 1.2 Tren yang Sedang Mendefinisikan Kategori

**Tren 1 — Bottleneck bergeser dari model ke struktur organisasi.** Survey Trinity blog (Agustus 2026): enterprise rata-rata menjalankan 12 agent, 50% bekerja sendirian tanpa koordinasi. Masalahnya bukan lagi "bisakah model ini coding?" tapi "bagaimana 12 agent ini tidak menginjak kerja satu sama lain?" — ini PERSIS positioning Galleon (Quartermaster→Ship→Squad→Crew).

**Tren 2 — Governance sebagai diferensiator nyata.** Dataiku (Agustus 2026): *"Governance, not build capability, is the real differentiator among enterprise AI agent platforms in 2026."* McKinsey AI Trust Maturity Survey 2026: 64% responden menyebut security & risk concerns sebagai barrier utama scale agentic AI — mengalahkan regulatory uncertainty dan technical limits. Ini membuat backlog security Galleon bukan sekadar hygiene, tapi **marketing claim yang bisa diuangkan**.

**Tren 3 — Local-first mulai mainstream, bukan niche.** MarketWatch/EDB (2026): hanya 13% organisasi capai true AI/data sovereignty pada 2025, proyeksi naik ~30% akhir 2026 (5x ROI vs non-sovereign). Microsoft counter dengan "Foundry Local" + "Agentic Retrieval on-prem" (Build 2026). GitHub Copilot resmi dukung BYOK via Ollama (GA April 2026).

**Tren 4 — Agentic Commerce mulai terstandarisasi.** Mastercard+Google: Verifiable Intent (Maret 2026) → donasi ke FIDO Alliance sebagai AP2 (April 2026). FIDO bentuk 2 working group baru: Agentic Authentication TWG + Payments TWG. Proyeksi: agentic commerce drive *trillions* transaction value by 2030. Relevan untuk Galleon — fondasi `verifiable_intent` yang sudah diaudit sesi ini menjadi strategic asset.

## Bagian 2: Competitor Positioning vs Galleon

### 2.1 Peta Kompetitor

| Pemain | Model | Harga | Organizational AI? | Desktop-First? | BYOK/BYOM? | Data Sovereignty? |
|---|---|---|---|---|---|---|
| **GitHub Copilot** | Cloud, usage-based | $19/seat (Business) + overage AI Credits | Belum — Copilot App baru orkestrasi basic | Copilot App (baru Juni 2026) | BYOK GA April 2026 | Partial (data di Azure) |
| **Trinity (Ability.ai)** | Self-hosted / Trinity Cloud | Apache 2.0 free self-host / Cloud pro-rata DigitalOcean | ✅ Ya — departments, reporting lines, playbook calls | Tidak (server/VPS-first) | ✅ Ya | ✅ Ya (VPN-only, perimeter user) |
| **OpenClaw 2.0** | Self-hosted / cloud | MIT free / Cloud $59/mo | Partial — multi-agent tapi flat (tidak hierarkis) | Tidak — browser UI | ✅ Ya (Ollama) | ✅ Ya (self-host) |
| **Hermes Agent** | Self-hosted | MIT free, model cost user-borne | Tidak — single-agent evolving | Desktop native (baru 2026) | ✅ Ya (200+ backend) | ✅ Ya |
| **Kiro Crew (AWS)** | Self-hosted, OSS | Apache 2.0 free | Multi-agent, scope developer-tool (bukan org) | ✅ Ya (Tauri+CLI) | ✅ Ya | ✅ Ya |
| **Copilot Studio (Microsoft)** | Cloud SaaS | $200/mo (25 msg), $0.01/msg, $2/conversation | ✅ Ya (enterprise workflows) | Tidak | Tidak (Azure-locked) | ❌ Tidak |
| **Salesforce Agentforce** | Cloud SaaS | $2/conversation atau Flex Credits $0.10/action | ✅ Ya (CRM-native) | Tidak | Tidak | ❌ Tidak |

### 2.2 Keunggulan Nyata Galleon ✅

**A. Hierarki organisasi sesungguhnya (Quartermaster→Ship→Squad→Crew)** — tidak ada kompetitor dengan *genuine multi-tier organizational hierarchy*: reporting lines, delegation chain, policy inheritance. Trinity paling dekat (departments+reporting lines) tapi tanpa konsep "Quartermaster" sebagai supreme context holder.

**B. Desktop-native + zero platform dependency** — Tauri v2. OpenClaw/Trinity berbasis web/server. Copilot App baru desktop Juni 2026 tapi cloud-native. Kiro Crew pembanding paling langsung tapi developer-tool, bukan organization workspace.

**C. BYOK+BYOM+BYOI zero platform markup** — GitHub Copilot BYOK ada tapi tetap ada platform overhead. Galleon tidak charging apa pun per token. **Klaim marketing yang bisa dihitung calon user secara langsung.**

**D. Treasury read-only access** — tidak ada kompetitor yang punya ini. Audit pengeluaran agent transparan tanpa agent bisa transaksi sendiri — foundation agentic commerce yang responsible ketika AP2/VI matang.

**E. Learning Governance sebagai primitive** — approval = fleet policy education, bukan sekadar gate. Tidak ada kompetitor dengan framing ini.

### 2.3 Kompetitor Lebih Maju ⚠️

- **Trinity** — skills ecosystem/community jauh lebih mature (multi-source skills library). Risiko: user baru akan langsung lihat gap ini vs Galleon.
- **OpenClaw** — 350K+ stars, 987 kontributor aktif. Default pilihan "self-hosted agent" di komunitas. Risiko: mind share/SEO dominan di keyword "self-hosted AI agent".
- **Hermes Agent** — self-evolving skills (GEPA) adalah selling point kuat untuk early adopter "agent yang grows with them". Galleon belum punya equivalent.
- **GitHub Copilot/Microsoft** — developer ecosystem lock-in; Dynamic Workflows pakai SDK Python programmable. Risiko: Galleon tidak bisa fight di developer-tooling positioning — harus differentiate sebagai "organization workspace".
- **Kiro Crew** — closer comparison, lebih public, battle-tested. User pertama Galleon akan SELALU membandingkan dengan Kiro Crew — perlu messaging tajam kenapa Galleon bukan "Kiro Crew untuk non-developer".

## Bagian 3: Backlog Teknis → Dampak Posisi Produk

*Tidak mengulang isi BL-01..39 — memposisikan backlog dalam konteks market claim, bukan urutan teknis.*

### 3.1 Tiga Positioning Bet

**Bet A — "The Trustworthy Organization Agent"**: satu-satunya platform teraudit/terverifikasi untuk agentic operations sensitif. Target: regulated industries, fintech, healthcare.

**Bet B — "The Sovereign Workspace"**: zero data loss, zero markup, kontrol total. Target: privacy-conscious individuals, SME EU (GDPR), tech-savvy frustrasi cloud lock-in.

**Bet C — "The Organization OS"**: satu-satunya yang paham AI agent butuh org chart, bukan hanya chat. Target: small teams/startup yang mau delegate ke agent fleet.

**Rekomendasi PO**: Galleon unggul di ketiganya, tapi MVP positioning paling tajam = **Bet B sebagai trojan horse menuju Bet C**. Bet A adalah long-term moat setelah security backlog matang.

### 3.2 Backlog Impact Map — Security Track (untuk Bet A/B)

| Backlog Category | Marketing Claim yang Bisa Dibuka | Prioritas Market |
|---|---|---|
| Channel signature verification (HMAC) | "Semua channel event terverifikasi kriptografis — tidak ada spoofed webhook" | **TINGGI** |
| Vault encryption + BYOK secret mgmt | "API key kamu tidak pernah bisa diakses platform" | **SANGAT TINGGI** — klaim kompetitif langsung vs Copilot/cloud |
| Agent permission scoping (sub-agent containment) | "Agent tidak bisa lebih dari yang diizinkan — secara kriptografis" | **TINGGI** — differentiator vs OpenClaw (security disabled by default) |
| WASM plugin sandboxing | "Plugin pihak ketiga di sandbox — tidak akses data fleet lain" | **TINGGI** — Trinity belum klaim ini eksplisit |
| PII/log redaction | "Galleon tidak log data sensitif kamu" | **MEDIUM** — relevan EU/GDPR |
| Resource mgmt sub-agent (BL-15/18) | "Agent tidak bisa monopoli resource/DoS fleet sendiri" | **MEDIUM** — enterprise concern, secondary |

**Kondisi saat ini**: channel signature verification belum selesai (temuan audit teknis sesi ini: `verify_whatsapp_signature` tidak ada implementasi nyata — persis BL-32). **Backlog paling mendesak untuk security claim** — gap konkret, dapat diverifikasi, ada di beberapa channel. **Jangan klaim "semua channel diverifikasi" sampai benar-benar teruji.**

### 3.3 Backlog Impact Map — Open-Source Track

| Kondisi | Blocker atau Selling Point? |
|---|---|
| Vault/secret bersih (no hardcoded di code) | **BLOCKER** — rilis OSS dengan secret pattern = security report instan di GitHub |
| WASM plugin interface terdokumentasi+stable | **SELLING POINT** — komunitas bisa build plugin sendiri |
| Channel adapter interface terdokumentasi | **SELLING POINT** — komunitas bisa tambah channel |
| Resource gate (BL-15/18, tergantung BG-D1/D4) | **BLOCKER enterprise adoption** — tanpa ini, deployment OSS bisa diabuse |
| RCE `/api/engine/execute` di-gate/dihapus | **Sudah closed** — `FW-A2a`/PR-04, dihapus dari `web/server.ts`, bukan temuan baru |
| CI/CD supply-chain (build reproducibility) | **SELLING POINT** — enterprise akan tanya ini sebelum adopt |
| LLM provider reliability+failover | **SELLING POINT** — differentiator vs OpenClaw (provider-agnostic tapi tanpa failover otomatis) |

### 3.4 Backlog Impact Map — Organization OS Track (differentiator paling unik)

| Backlog/Fitur | Impact ke Positioning |
|---|---|
| SOP/RACI engine — policy inheritance ke Ship/Squad/Crew | **PALING TINGGI** — zero-equivalent di pasar; Trinity punya departments tapi bukan SOP/RACI |
| Skills/lessons/steering per-Ship (bukan hanya global) | **TINGGI** — Ship "Marketing" punya skills beda dari Ship "Engineering" — demonstrable |
| Automation/cron/webhook per-Squad | **MEDIUM** — parity dengan OpenClaw |
| Memory data governance (per-agent data scope) | **TINGGI** — framing "Fleet tidak boleh tahu lebih dari yang diizinkan" |
| Verifiable Intent (`verifiable_intent`) | **FUTURE-PROOF** — AP2/FIDO belum mature, tidak perlu rush, tapi positioning kuat kalau siap saat standar tiba |

## Rekomendasi Strategis untuk Owner (Achmad)

> **KEPUTUSAN ACHMAD (2026-10-05)**: produk ini **belum rilis publik sama sekali**. Arah yang diputuskan: **tunda rilis publik dan narasi positioning apa pun sampai produk genuinely robust, dengan Bet C (Organization OS) sebagai target kelengkapan** sebelum pintu publik dibuka. Ini mengubah gate R1/R3 di bawah — tidak ada "soft launch" di Bet B sambil Bet C masih dibangun. Rilis publik (termasuk aktivitas "build in public" di R3) ditahan sampai Bet C tercapai secara substansi, bukan hanya diklaim.
>
> **Status konkret BL-06..09 saat ini** (dicek ulang 2026-10-05): keempatnya masih di fase backlog murni — **belum ada satu pun yang diangkat ke `roadmap/` resmi atau di-assign ke squad**. BL-06 (Do's/Don'ts di `SopStep`) dan BL-07 (RACI lengkap, serialkan setelah BL-06) adalah scope kecil-menengah, LOW-MEDIUM risiko, bisa langsung actionable. BL-08 (role template per-Crew) punya blocker verifikasi loader yang belum dicek. BL-09 (Quest lifecycle + `mission.Service` baru di Go) scope JAUH lebih besar — service layer baru, bukan sekadar tambah field, MEDIUM risiko breaking change. Ini adalah jarak nyata antara kondisi sekarang dan "Bet C tercapai".

> **Implikasi konkret ke backlog**: R3 di bawah **TIDAK dieksekusi untuk saat ini**. R2 (3 security blocker) tetap prioritas tertinggi sebagai prasyarat "robust". Backlog yang naik urgensi: **BL-06..09 (SOP/RACI engine)** dan item skills-per-Ship — ini isi literal Bet C yang jadi gate kelengkapan, bukan sekadar differentiator opsional lagi. — Bet C (Organization OS) adalah gate kelengkapan sebelum rilis publik, bukan pilihan narasi untuk sekarang.** Bet B (Sovereign Workspace) tetap valid secara teknis (BYOK/BYOM/zero-markup sudah nyata di kode) dan dipakai sebagai *framework prioritization internal*, bukan untuk marketing dini. Bet A dibuka hanya setelah security backlog critical selesai DAN Bet C tercapai. Tidak ada rilis bertahap — publik dibuka saat produk genuinely robust.

**R2 — Selesaikan 3 security blocker sebelum pengumuman publik apa pun**: (1) channel signature verification selesai+tested, (2) ~~RCE endpoint~~ (sudah closed), (3) vault/secret tanpa hardcoded pattern. Tanpa ini, Galleon berisiko jadi headline negatif di komunitas hacker saat diluncurkan.

**R3 — DITUNDA oleh Achmad.** ~~Build community signals paralel dengan development, tidak perlu tunggu launch.~~ Keputusan Achmad: tidak ada aktivitas publik (blog, GitHub repo publik, engagement komunitas) sampai Bet C tercapai. Dipertahankan di dokumen sebagai opsi yang bisa dipertimbangkan ulang nanti, bukan rekomendasi aktif saat ini.

**R4 — Skills ecosystem adalah priority 1 untuk community adoption.** OpenClaw dan Hermes menang bukan di arsitektur — menang di jumlah skills siap pakai. Perlu: Skill interface stable, 10-15 first-party skills demonstrable (bukan API wrapper — misal "Daily Standup Skill untuk Squad", "Invoice Processing Skill untuk Ship Finance"), plugin marketplace WASM sebagai long-term moat.

**R5 — Verifiable Intent/AP2: jaga arsitektur ready, jangan rush build.** Standard belum final (baru donasi April 2026). Maintain `verifiable_intent` sebagai documented interface, pastikan audit trail kompatibel, tapi jangan full implement sekarang — bangun saat standar&pasar clear (~2027). Selaras keputusan G-37 (gate wajib, bukan komitmen implementasi) yang sudah diputuskan Owner sesi riset teknis.

## Lampiran: Summary Competitive Intelligence

| Pemain | Rilis Kunci Terbaru | Kekuatan | Kelemahan yang Bisa Dieksploitasi Galleon |
|---|---|---|---|
| **GitHub Copilot** | Dynamic Workflows 1 Okt 2026; BYOK GA April 2026 | Developer ecosystem dominan; Microsoft backing | Cloud lock-in; data di Azure; billing shock per-token; tidak ada organizational hierarchy |
| **Trinity (Ability.ai)** | v0.9.0, 17 Agt 2026 | OSS enterprise; departments+audit trail; Apache 2.0 | Server/VPS-first; tidak ada Treasury; tidak ada RACI/SOP primitive |
| **OpenClaw 2.0** | v2026.8.1, 30 Agt 2026 | Community momentum; free; multiplayer | Security disabled by default; tidak hierarkis; tidak ada persistent organizational identity |
| **Hermes Agent** | Feb 2026 + desktop 2026 | Self-evolving skills (GEPA); MIT; 200+ model backend | Single-agent; no governance primitive |
| **Kiro Crew** | Ongoing | Desktop+CLI; persistent memory; skills; Apache 2.0 | Developer-tool framing, bukan organization workspace; tidak ada Quartermaster/Fleet hierarchy |

---

## Catatan Silang-Rujuk ke Temuan Teknis Sesi Ini

Laporan ini independen mengonfirmasi beberapa prioritas dari audit kode teknis sesi ini, dari sudut pandang BERBEDA (produk, bukan kode):
- **BL-32** (bug signature WhatsApp) — dikonfirmasi blocker *produk*, bukan cuma bug teknis.
- **BL-19/BL-22** (sandbox MCP, ADR WASM vs MCP) — dikonfirmasi *selling point*, bukan cuma hardening teknis.
- **G-37/DAC** — dikonfirmasi arah "jangan rush", selaras keputusan Owner yang sudah diambil.
- **RCE `/api/engine/execute`** — klaim awal laporan ternyata SUDAH closed (`FW-A2a`, PR-04 di `finalize/dev-tasks/03-fe-web.md`), dikoreksi di dokumen ini.

## Sumber

- GitHub Copilot Dynamic Workflows: https://github.blog/changelog/2026-10-01-dynamic-workflows-in-copilot-cli-and-the-copilot-app/
- Trinity by Ability.ai: https://www.ability.ai/trinity, https://github.com/Abilityai/trinity, https://docs.ability.ai/
- Artifact sumber penuh (disimpan sub-agent): `galleon-fleet-market-strategic-report-oct2026`
- Cross-check RCE: `standard-docs/decisions/finalize/dev-tasks/03-fe-web.md:10` (FW-A2a), `08-carryover-from-finalize.md:42` (Q-4)
