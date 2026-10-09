---
title: Riset Tambahan — Go-to-Market, Segmentasi, Risiko & Rekomendasi (Prodcut-Owner)
status: research
date: 2026-10-05
author: Prodcut-Owner (crew)
depends_on: product-industry-competitor-insight.md
---

> Dokumen ini **melanjutkan dan tidak mengulang** riset di `archive/research-closed-loop/product-industry-competitor-insight.md`. Dikerjakan oleh crew `Prodcut-Owner` atas delegasi, dengan instruksi eksplisit membaca baseline dulu dan hanya menambah perspektif baru.

# Riset Tambahan: Go-to-Market, Segmentasi, Risiko & Rekomendasi Baru untuk Galleon Fleet

## Bagian 1 — Go-to-Market & Model Monetisasi: Pemain yang Belum Disebut

### 1.1 OpenClaw — Model MIT + Ekosistem Marketplace

OpenClaw (dirilis November 2025 oleh Peter Steinberger, Austria) adalah pemain paling relevan yang belum ada di riset awal. Ini bukan sekadar framework — ini pesaing **arsitektur** terdekat dengan Galleon.

- **Lisensi**: MIT (sepenuhnya gratis, bukan Apache 2.0 seperti Kiro Crew)
- **Model bisnis**: Framework gratis, monetisasi via **ClawHub marketplace** — ekosistem 13.700+ skill komunitas. Pendapatan dari sponsorship korporat (OpenAI, GitHub, NVIDIA, Vercel), bukan dari pengguna individu.
- **Skala**: 374.000 GitHub stars per April 2026. Steinberger kemudian bergabung ke OpenAI; OpenClaw bertransisi ke yayasan independen.
- **Deployment**: Docker Compose < 30 menit, tagihan VPS $5–10/bulan + API cost $15–80/bulan tergantung volume — TCO jauh di bawah SaaS.

**Insight untuk Galleon**: OpenClaw membuktikan MIT license + marketplace dapat menarik sponsor korporat besar sebagai sumber pendapatan — model berbeda total dari model kredit Kiro Crew. Galleon belum mendefinisikan apakah akan open-source, dan jika ya, apa sumber pendapatannya.

Sumber: [shiftasia.com, Juni 2026](https://shiftasia.com/column/nous-hermes-agent-vs-openclaw-architectural-deep-dive/); [medium.com, 2026](https://medium.com/@Daniel.O.Ayo/claude-vs-hermes-vs-openclaw-which-ai-agent-is-actually-worth-paying-for-in-2026-81ad77de8225)

### 1.2 Hermes Agent (Nous Research) — MIT + Self-Improving Architecture

- **Lisensi**: MIT, gratis.
- **Model bisnis**: Nous Research memonetisasi lewat model weights komersial (Hermes 3/4), bukan dari agen itu sendiri — pola "razor and blade" terbalik: agen gratis, model berbayar.
- **Biaya operasional nyata**: VPS $11–35/bulan all-in untuk self-hosting dengan Ollama. Stack Overflow Pulse Survey (Mei 2026) menyebut Hermes sebagai salah satu dari tiga "newcomers with high usage AND high interest" — 17% sudah pakai, 32% ingin pakai.
- **Diferensiasi arsitektur kunci**: Kanban board di `~/.hermes/kanban.db` untuk koordinasi multi-agent dengan heartbeat monitoring, zombie detection, dan task reclaim — **observability primitive** yang tidak dimiliki OpenClaw maupun Kiro Crew.

Sumber: [shiftasia.com, Juni 2026](https://shiftasia.com/column/nous-hermes-agent-vs-openclaw-architectural-deep-dive/); [stackoverflow.com, Mei 2026](https://blog.stackoverflow.com/2026/05/27/agents-on-a-leash-agentic-ai-remains-mostly-monitored-at-work/)

### 1.3 Model Enterprise On-Premise: TCO vs Per-Seat SaaS

Data konkret (dev.to, September 2026, Haradhan Sharma, Senior Enterprise Architect):

| Skenario | 100 knowledge workers, 3 tahun |
|---|---|
| ChatGPT Enterprise ($60/user/bulan) | $216.000 total |
| Sovereign AI on-premise (A100 GPU) | $65.000–75.000 total |
| Penghematan | ~65% |

Ini **argumen TCO** yang berbeda dari narasi riset awal (fokus ke sovereignty sebagai nilai, bukan penghematan biaya). Untuk enterprise 30+ seat, self-hosted sovereign AI secara matematis lebih murah setelah tahun pertama.

Sumber: [dev.to, September 2026](https://dev.to/haradhansharma/why-forward-thinking-enterprises-are-replacing-chatgpt-enterprise-with-sovereign-ai-in-2026-1fa1)

### 1.4 Theta42 — Model Enterprise On-Premise $50K+

Theta42 ([theta42.com](https://theta42.com)): *"Deploy a Sovereign system custom-built for your enterprise. Starts at $50k. Zero token limits."* Model implementasi sekali bayar, bukan SaaS — seperti software enterprise tradisional.

**Relevansi**: ada segmen enterprise yang bersedia membayar $50K+ sekali untuk zero token limit + zero vendor dependency. Galleon dengan BYOK/BYOM berpotensi masuk celah ini, di harga lebih terjangkau (open-source core, bukan custom build dari nol).

---

## Bagian 2 — Segmentasi Target User: Siapa Pembeli Sebenarnya?

### 2.1 Data Adopsi Aktual (McKinsey, Stack Overflow, JetBrains — 2026)

- **McKinsey State of AI 2026**: 40% organisasi *besar* (revenue >$1B) sudah men-scaling AI agent (naik dari 27%). Organisasi kecil: hanya 22%, flat. **Enterprise besar = pembeli paling aktif saat ini.**
- **Stack Overflow Pulse Survey, April 2026** (1.100 responden): 59% developer pakai AI agent di kerja (naik dari 31%). 68% masih prefer single-agent; hanya 17% koordinasi multi-agent — tapi dari yang pakai multi-agent, 85% justru pakai *daily*. **Multi-agent workspace = segmen early adopter power user, bukan mass market.**
- **JetBrains, Mei–Juli 2026**: 90% developer profesional pakai AI coding agent setidaknya mingguan, 68% harian.
- **Gartner**: Sovereign cloud IaaS spending $80 miliar di 2026, tumbuh 35,6% dari 2025.

### 2.2 Segmentasi Riil: Tiga Bucket Berbeda

**Bucket A — Regulated Enterprise (Healthcare, Finance, Government, Defense)**
- Butuh sovereign AI secara *legal* (HIPAA, GDPR, ITAR, SOC 2), bukan preferensi. Budget besar ($50K+), procurement lambat.
- Fintech adopter *harian* tertinggi (55%) — lebih tinggi dari software development sendiri.
- Barrier masuk: sertifikasi, audit trail, SLA, dukungan korporat.

**Bucket B — Software Engineering Team / Architect (10–50 orang)**
- Konsumen utama multi-agent koordinasi saat ini: architects (52% daily) dan senior exec (50% daily) jauh di depan developer biasa.
- Paling mungkin jadi early adopter Galleon — cukup teknis self-host, cukup besar butuh hierarki Squad/Crew.
- Willingness to pay: $20–100/bulan/seat (selaras Kiro Crew Pro). Pain point: observability multi-agent + cost predictability.

**Bucket C — Individual Developer / Indie Hacker**
- Pasar terbesar secara jumlah, willingness-to-pay terendah.
- OpenClaw & Hermes Agent sudah lebih optimal untuk segmen ini (MIT, $11/bulan).
- Galleon dengan Fleet–Ship–Squad–Crew overkill untuk 1-2 orang — bisa serve tapi bukan differentiator, cuma "table stakes".

**Rekomendasi**: Bucket B (engineering team 10–50 orang) adalah ICP paling realistis untuk 12–18 bulan pertama — cukup teknis menghargai sovereignty+observability, cukup kecil untuk keputusan beli cepat, belum dilayani baik oleh OpenClaw (terlalu individual) maupun Theta42 (terlalu mahal).

---

## Bagian 3 — Risiko & Ancaman: Perspektif Skeptis

### 3.1 Complexity Tax yang Underestimated

**aicompetence.org (2026)**: *"The mistake many organizations make is assuming local AI is primarily a deployment choice. In practice, local AI becomes a distributed operational system with its own maintenance burden, governance complexity, observability gaps, and lifecycle management problems."*

Konkret: 79% enterprise klaim sudah adopsi AI agent. Kurang dari 17% benar-benar menjalankan di production. Gap ini hampir tidak soal kemampuan model — soal keputusan deployment yang terus ditunda karena beban operasional.

### 3.2 Local Hosting ≠ Sovereignty (Forrester)

Forrester senior analyst Dario Maisto (ITPro, Juli 2026): *"Local hosting does not protect workloads and data with regard to sovereignty concerns... If the infrastructure and the tools are owned by a third party operating under a different jurisdiction, the sovereignty risk is not mitigated."* Dan: *"Integration is another underestimated aspect... agents are pervasive and have the potential to increase vendor lock-in with non-sovereign vendors."*

**Implikasi untuk Galleon**: integrasi channel yang tergantung cloud pihak ketiga (WhatsApp via Meta API, Slack, Discord) berpotensi membuat klaim "sovereign" berlubang — gap komunikasi produk yang perlu diantisipasi sejak awal.

### 3.3 Supply Chain Attack — Bukti Nyata dari OpenClaw

ClawHub audit Q1 2026: **341 malicious entries dari 2.857 skill submissions (~12% malware rate)**. Satu aktor upload 354 paket berbahaya via typosquatting. 135.000+ instance OpenClaw terekspos di Shodan. Cisco mengategorikan ini "a security nightmare."

**Relevansi**: kalau Galleon buka marketplace plugin/skill ke komunitas, risiko ini harus sudah diantisipasi di arsitektur — sandboxing WASM yang sudah ada (BL-19) adalah langkah benar, perlu dikomunikasikan eksplisit sebagai pembeda.

### 3.4 Self-Hosting Hanya Masuk Akal di Atas Threshold Tertentu

Futurum Group analyst Brad Shimmin (Agustus 2026): metered API billing runtuh ketika multi-step agentic workflow di-scale — TAPI ada threshold. Data analis independen: *"Self-hosting inference pays off... you push past roughly two million tokens a day, or your data legally cannot leave your network. Below that line a hosted API is cheaper."*

**Implikasi**: untuk user di bawah threshold volume, argumen cost sovereignty Galleon lemah — lebih murah pakai Kiro Crew Pro $20/bulan. Galleon perlu argumen sovereignty yang TIDAK bergantung pada argumen biaya untuk user di bawah threshold (kontrol data, determinism, audit trail — ada, tapi perlu diposisikan berbeda).

### 3.5 Risiko Talent

Forrester forecast: *"The tech workforce is the only sovereignty index that is going to decrease in the next five years."* Makin spesifik skill untuk deploy/maintain sovereign AI, makin kecil talent pool. Risiko bukan Galleon-as-product, tapi Galleon-as-customer: enterprise yang mau pakai Galleon mungkin tidak punya orang yang bisa menjalankannya.

**Implikasi**: "ease of deployment" dan "zero-ops mode" bisa jadi differentiator nyata, bukan nice-to-have.

---

## Bagian 4 — Rekomendasi Baru

### Rekomendasi A — "Minimum Viable Sovereignty" per Tier User

Forrester punya framework "Minimum Viable Sovereignty" untuk bantu CIO memilih. Galleon bisa adopsi jadi 3 profile deployment eksplisit:
1. **Local-only** (BYOK+BYOM+zero cloud) — regulated enterprise (Bucket A), sovereignty tertinggi.
2. **Hybrid** (local model, cloud channel) — engineering team (Bucket B), tradeoff realistis.
3. **Cloud-assisted** (kredit cloud opsional, data tetap tidak keluar) — individual developer (Bucket C), onboarding mudah.

Ini menjawab kritik Forrester "local hosting ≠ sovereignty" — Galleon menawarkan spektrum transparan, bukan klaim binary yang mudah dipatahkan.

### Rekomendasi B — Observability/Kanban Layer sebagai Diferensiasi Utama

Multi-agent observability (task board, heartbeat, zombie detection, task reclaim) adalah gap nyata — bahkan OpenClaw (374K star) belum punya. 63% developer masih tidak mau biarkan agent autopilot, concern #1 adalah security+accuracy, bukan cost.

Galleon sudah punya fondasi hierarki Quartermaster→Ship→Squad→Crew. Yang belum tampak jelas: observability layer user-facing (lihat apa yang dikerjakan tiap Crew, status, kapan macet). Ini pembeda konkret dari OpenClaw dan Kiro Crew.

### Rekomendasi C — "Certified Partner" Model untuk Talent Gap

Masalah terbesar adopsi enterprise bukan produk — talent. Galleon bisa bangun certified implementation partner program (pola Odoo/GitLab/HashiCorp) — IT consulting/system integrator tersertifikasi deploy Galleon di enterprise. Mempercepat penetrasi Bucket A tanpa harus bangun tim sales enterprise sendiri; recurring revenue dari sertifikasi+support contract. Belum ada pemain "local-first AI agent workspace" yang melakukan ini secara sistematis.
