---
title: Insight Produk — Riset Industri/Kompetitor & Arah Teknis Lead
status: research
date: 2026-10-05
author: squad-lead
depends_on: null
---

# Insight Produk — Riset Industri, Kompetitor, dan Arah Teknis

> **Catatan proses:** permintaan awal untuk berkoordinasi dengan crew `Prodcut-Owner` gagal — spawn tidak ter-register di sistem tracking setelah ditunggu ~9 menit (konsisten dengan beberapa kegagalan spawn serupa di sesi ini). Dokumen ini dikerjakan langsung oleh saya, menggabungkan riset industri/kompetitor baru dengan analisis arah teknis dari backlog BL-01 s.d. BL-39 yang sudah dihasilkan sesi ini.

## 1. Riset Industri & Produk Serupa — Yang SEDANG Relevan (bukan historis)

### 1.1 "Sovereign AI" naik jadi kategori produk, bukan sekadar fitur

Istilah **"Sovereign AI"** sedang menjadi wacana arus utama di industri sepanjang 2026 — dikutip CNBC Juli 2026, dibahas lewat esai CEO (Satya Nadella dkk.), dan jadi dasar produk nyata (Oracle "Building Sovereign AI: Eight Layers", Haystack 3.0 "Architecture of Sovereign Agents"). Insight paling tajam datang dari **deepset.ai (Haystack 3.0, Juli 2026)**: *"Every agent runs inside a harness... If you cannot understand or leave that harness, moving it into a more secure environment does not make it yours. Sovereignty needs to be a property of the architecture itself."*

**Relevansi langsung untuk Galleon**: ini PERSIS bahasa yang bisa dipakai untuk positioning — Galleon BYOK/BYOM/BYOI + local-first + 4-tier open architecture (Tauri/Rust/Go/React, semua auditable) adalah implementasi KONKRET dari "sovereignty sebagai properti arsitektur," bukan klaim marketing kosong. Kompetitor besar (Haystack, Oracle) baru MULAI mengartikulasikan ini sebagai kategori pada 2026 — Galleon sudah dibangun dengan prinsip ini dari awal (BYOK/BYOM/BYOI, zero platform markup di invariant F1).

### 1.2 Multi-agent hierarchy jadi arah utama, bukan single-assistant

`futureagi.com` (2026): perbandingan CrewAI, LangGraph, AutoGen, OpenAI Agents SDK, MS Agent Framework — semua bergerak ke arah orkestrasi multi-agent dengan observability/eval sebagai lapisan wajib, bukan opsional. **designthinkingblog.com**: *"one person, empowered by agentic AI, can now create and run multiple complex software products... shifts the traditional organizational model."* Ini mengonfirmasi hipotesis dasar Galleon (hierarki Quartermaster→Ship→Squad→Crew, unit kerja = organisasi bukan asisten tunggal) BUKAN taruhan spekulatif, tapi arah yang didukung tren industri luas.

### 1.3 Demo headline kompetitor menunjukkan arah skala — dan risiko yang sama yang Galleon temukan sendiri

**Google Antigravity 2.0** (I/O Mei 2026): demo headline membangun kernel OS dalam 12 jam dengan **93 sub-agent paralel**, 2.6 miliar token, di bawah $1.000 biaya API. Ini angka yang mengesankan secara marketing — TAPI secara langsung relevan dengan temuan `subagent-resource-management-audit.md` sesi ini: skala sub-agent paralel sebesar ini SECARA DEFINISI butuh resource-gating yang matang (persis BL-15, yang justru ditemukan NOL ADA di Galleon hari ini). Kompetitor terbesar menunjukkan arah "banyak sub-agent paralel" sebagai pencapaian marketing — artinya Galleon HARUS menutup BL-15 sebelum bisa klaim kapabilitas serupa dengan aman.

### 1.4 Kiro Crew — pembanding langsung yang SUDAH monetisasi dan open-source sekaligus

Kiro Crew (disebut eksplisit sebagai pembanding di `galleon-product-definition.md`) sudah: **open-source penuh (Apache 2.0)**, **self-hostable**, DAN punya model harga jelas (`kiro.dev/pricing`): Free (50 kredit), Pro $20/bulan, Pro+ $40, Pro Max $100, Power $200/bulan — model kredit (1 API call ≈ 1 kredit ≈ $0.05). Fitur "Memory, lessons, and skills carry across sessions... Inspect, edit, or remove what your crew carries forward" — ini SAMA PERSIS dengan klaim memory persisten + Learning by Consent Galleon, tapi Kiro Crew SUDAH shipped dan dijual, Galleon masih di fase riset arsitektur.

**Insight paling penting**: Kiro Crew membuktikan model "open-source core + paid tier untuk convenience/cloud" BISA berjalan secara komersial untuk kategori produk yang sama persis dengan Galleon. Ini bukan risiko kanibalisasi — ini VALIDASI pasar bahwa kategori ini punya pembeli nyata.

## 2. Riset Kompetitor — Positioning Konkret

| Dimensi | Galleon | Kiro Crew | Google Antigravity | GitHub Copilot |
|---|---|---|---|---|
| Model harga | Belum ada (riset ini) | Kredit $20-200/bln, free tier 50 kredit | $0-200/bln (riding di Google AI plan), Enterprise via Gemini license | Per-seat enterprise, bundling GitHub |
| Open-source | Belum diputuskan (dibahas sesi ini) | Ya, Apache 2.0 | Tidak | Tidak |
| Sovereignty (BYOK/BYOM) | **Ya, invariant F1 — zero platform markup** | Sebagian (self-host, tapi model API masih lewat kredit mereka) | Tidak (terkunci ke Google AI plan) | Tidak (terkunci ke GitHub Copilot billing) |
| Unit kerja | Organisasi (Fleet→Ship→Squad→Crew) | Personal agent + Crew (tim kecil) | IDE agent-first, belum eksplisit "organisasi" | Repo-task-level agent |
| Deployment | Local-first + remote opsional | Local/remote (gateway) | Cloud-first (riding Google infra) | Cloud-first (GitHub infra) |
| Governance/Enterprise | Belum ada riset khusus (lihat BL-24, ditunda) | Enterprise MCP governance ADA (fail-closed registry) | Enterprise plan via Gemini license, admin spend control | Enterprise managed-settings (MCP allowlist) |

### Di mana Galleon punya keunggulan nyata (dari bukti kode sesi ini, bukan klaim marketing)
- **Zero platform markup + BYOK/BYOM/BYOI sebagai invariant produk**, bukan fitur opsional — kompetitor besar (Antigravity, Copilot) SEMUA terkunci ke billing provider mereka sendiri.
- **4-tier architecture yang auditable** (Tauri/Rust/Go/React terpisah jelas) — sejalan dengan bahasa "harness yang bisa diaudit" dari deepset.ai, kompetitor closed-source tidak bisa klaim ini.
- **Sandboxing plugin WASM yang sangat matang** (ditemukan sesi ini: fuel limit, egress deny-by-default) — ini DI ATAS standar yang terlihat di dokumentasi publik kompetitor manapun yang diriset.

### Di mana kompetitor sudah lebih maju dan Galleon berisiko tertinggal
- **Governance MCP enterprise** — Kiro Crew dan GitHub Copilot SUDAH punya mekanisme fail-closed registry/allowlist terpusat. Galleon (BL-24, ditunda) belum punya jalur ini SAMA SEKALI karena Go tidak tahu konsep MCP (keputusan arsitektur sadar, bukan oversight — tapi tetap gap kompetitif).
- **Resource-aware spawning** — Kiro Crew sudah punya `resource_status` tool + aggregate ceiling adaptif yang TERBUKTI dipakai (saya alami langsung sepanjang sesi riset ini). Galleon (BL-15) masih nol gate — ini gap yang paling mendesak ditutup SEBELUM klaim kapabilitas multi-sub-agent seperti demo Antigravity.
- **Model harga & distribusi yang sudah shipped** — Kiro Crew sudah di app store/GitHub release dengan harga jelas; Galleon masih di fase pre-release (ditemukan sesi ini: Windows code signing nol, BL-33).

## 3. Arah Teknis dari Sisi Lead — Backlog Mana yang Paling Berdampak ke Posisi Produk

Dari BL-01 s.d. BL-39 yang sudah dihasilkan sesi riset ini, berikut pemetaan dampak **ke posisi produk di pasar** (bukan sekadar kebersihan kode):

### Kalau Galleon mau diposisikan sebagai "yang paling aman/trustworthy" (selaras narasi sovereignty)
Backlog yang PALING KUAT untuk klaim ini — karena SUDAH terverifikasi lewat audit kode nyata, bukan asumsi:
1. **BL-19** (sandbox MCP stdio) — begitu ditutup, Galleon bisa klaim "setiap jalur ekstensibilitas (plugin WASM DAN MCP) dikontain setara" — klaim yang BAHKAN Kiro Crew/Antigravity belum terbukti punya (tidak ditemukan dokumentasi publik soal sandboxing MCP di kompetitor manapun).
2. **BL-32** (bug WhatsApp signature) — HARUS ditutup dulu sebelum klaim keamanan apa pun ke publik, karena ini bug nyata yang kalau ketahuan sebelum diperbaiki justru jadi liability PR.
3. **BL-37** (API redaksi tool I/O tidak ter-wire) — klaim "privacy-by-design" hanya kredibel kalau ini benar-benar ditutup; "konfigurasi berbohong" adalah risiko reputasi besar kalau ditemukan auditor eksternal setelah klaim publik dibuat.

### Kalau Galleon mau buka open-source (sesuai diskusi `oauth-client-id-opensource-implications.md`)
**Blocker yang WAJIB ditutup dulu:**
- **BL-25** (BYOC client_id) — tanpa ini, basis pengguna open-source besar berisiko memukul rate-limit OAuth app yang sama, mematikan fitur untuk SEMUA orang sekaligus.
- **BL-26** (dokumentasi single-tenant auth) — murah, tapi WAJIB sebelum rilis publik supaya tidak ada kontributor yang salah pakai di konteks multi-user tanpa peringatan.

**Selling point yang BISA langsung dipasarkan begitu open-source:**
- Sandboxing WASM plugin yang matang (fuel limit, egress deny-by-default) — fitur teknis yang BISA jadi bahan blog post teknis/HN post pembuka ("kami open-source sistem plugin paling ter-sandbox di kategori ini").
- CI/CD supply-chain yang sudah SLSA Build Level 2 + cosign + SBOM dua format — ini LANGSUNG di atas standar kebanyakan proyek open-source baru (temuan `cicd-supplychain-audit-partial.md`), bisa jadi trust signal untuk adopsi enterprise sejak hari pertama.

### Kalau Galleon mau antisipasi arah agentic commerce (sudah dikonfirmasi Owner sebagai arah terbuka)
- **G-37/DAC** (sudah di-upgrade jadi gate wajib sesi ini) — harus ditinjau SEBELUM desain fitur commerce pertama, selaras standar industri AP2/VI yang baru donasi FIDO Mei 2026.

## 4. Rekomendasi Strategis (3-5, Actionable untuk Owner)

1. **Adopsi bahasa "sovereignty sebagai properti arsitektur" (bukan sekadar BYOK) untuk positioning**, mengikuti pola framing deepset.ai/Haystack 3.0 — Galleon punya BUKTI arsitektur nyata untuk klaim ini (4-tier auditable, zero markup), yang kompetitor besar closed-source tidak bisa tandingi.
2. **Prioritaskan BL-15 (resource gate sub-agent) sebelum klaim kapabilitas multi-sub-agent apa pun secara publik** — demo headline kompetitor (Antigravity 93 sub-agent) menunjukkan ini jadi ekspektasi pasar, tapi Galleon belum punya pelindungnya.
3. **Kalau open-source jadi keputusan**: urutan rilis idealnya BL-26 (dokumentasi) → BL-25 (BYOC) → baru pengumuman publik — supaya kontributor pertama tidak langsung menemukan gap operasional yang merusak kesan pertama.
4. **Pertimbangkan model harga mengikuti pola Kiro Crew** (core gratis/open-source, convenience/cloud berbayar) sebagai validasi pasar yang SUDAH terbukti untuk kategori produk yang sama — bukan menebak dari nol.
5. **Siapkan satu dokumen comparison matrix publik** (versi ringkas dari §2 di atas) untuk materi pitch/landing page begitu produk siap rilis — kerangka tabelnya sudah ada di dokumen ini, tinggal diperbarui saat fitur benar-benar shipped.

---

## Sumber

- deepset.ai (Haystack 3.0, Juli 2026): https://www.deepset.ai/blog/haystack-3-sovereign-agents
- Google Antigravity 2.0 (I/O 2026): https://o-mega.ai/articles/google-antigravity-2-0-the-complete-2026-guide, https://cloudzero.com/blog/google-antigravity-pricing/
- Kiro Crew pricing & positioning: https://kiro.dev/pricing/, https://kiro.dev/docs/crew/, https://dev.to/aws-builders/introducing-kiro-crew-awss-open-source-ai-agent-orchestrator-1e63
- Multi-agent industry trend: http://futureagi.com/blog/multi-agent-systems-2025, https://designthinkingblog.com/ai-operations/the-local-first-agentic-operator/
- Sovereign AI sebagai kategori: https://blogs.oracle.com/cloud-infrastructure/building-sovereign-ai
- Dokumen riset internal sesi ini (dirujuk untuk analisis Lead): `subagent-resource-management-audit.md` (BL-15), `plugin-mcp-architecture-audit.md` (BL-19), `channel-signature-verification-audit.md` (BL-32), `observability-content-pii-audit.md` (BL-37), `oauth-client-id-opensource-implications.md` (BL-25/26), `cicd-supplychain-audit-partial.md`, `verifiable-intent-assessment.md` (G-37)
