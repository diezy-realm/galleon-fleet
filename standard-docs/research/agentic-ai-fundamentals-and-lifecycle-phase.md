---
title: Riset — Konsep Fundamental Agentic AI/Orchestrator/Companion & Lifecycle Produk Pasca-Riset
status: research
date: 2026-10-05
author: squad-lead
context: "Melangkah lebih jauh" dari 13 tema riset (BL-01 s.d. BL-56) yang sudah dibuat sesi ini -- mencari metode fundamental yang BELUM diadaptasi, dan menjawab "apa fase setelah semua ini terimplementasi".
---

# Riset: Konsep Fundamental Agentic AI/Orchestrator/Companion 2025-2026

## 1. Agentic AI Design Patterns Formal (Selain ReAct)

- **Reflexion** (arXiv:2303.11366) — verbal self-reflection dari kegagalan trial disimpan ke episodic memory, dipakai ulang di trial berikutnya tanpa fine-tuning.
- **Tree-of-Thoughts/ToT** (arXiv:2305.10601) — eksplorasi multiple reasoning path paralel + self-evaluasi + backtracking.
- **LATS** (arXiv:2310.04406, NeurIPS 2024) — menyatukan reasoning+acting+planning dengan Monte Carlo Tree Search eksplisit + value function.
- **ReflAct** (arXiv:2505.15182, 2025) — refleksi grounding pada STATE DUNIA NYATA vs goal-state (bukan reasoning trace seperti Reflexion). Klaim +27.7% dari ReAct di ALFWorld.
- **ReflexGrad** (arXiv:2511.14584, Nov 2025) — hierarchical TODO decomposition + history-aware causal reflection + gradient-based prompt optimization. Klaim zero-shot 67%→78% transfer antar-task.

**Rekomendasi**: SOP/Quest engine galleon-fleet cocok diperkuat dengan **ReflAct-style grounding** — bandingkan klaim agent ke STATE KNOWLEDGE GRAPH yang eksplisit, bukan trace percakapan. Ini mengisi gap arsitektural untuk bug "configuration-lies" (verify_whatsapp_signature, capture_tool_input, query_by_similarity, MessageDebouncer) yang berulang ditemukan sesi ini — ReflAct dirancang mencegah KELAS kegagalan ini secara struktural, bukan lewat instruksi manual "jangan menebak".

## 2. Multi-Agent Orchestration Patterns (Selain Hierarki Fleet/Ship/Squad/Crew)

- **Blackboard Architecture** (arXiv:2507.01701, arXiv:2510.01285) — agent membaca/menulis ke satu shared knowledge store; agent berikutnya dipilih berdasar isi blackboard, bukan dipanggil berurutan. 13-57% improvement vs pipeline linear.
- **Market-based/Auction allocation** (arXiv:2608.23867, arXiv:2511.17621) — orchestration sebagai pasar tenaga kerja, agent mem-bid tugas berdasar kapabilitas/cost/load.
- **HTN (Hierarchical Task Network) planning** — PALING RELEVAN: arXiv:2511.07568 ("Procedural Knowledge Improves Agentic LLM Workflows") membuktikan method library eksplisit membuat LLM 20B/70B mengungguli LLM 120B baseline. ChatHTN (arXiv:2505.11814). arXiv:2609.33731 ("HTN Planning as a Coordination Layer for Multi-Server MCP Tool Orchestration") — SANGAT dekat arsitektur galleon-fleet (Go orchestrator + Rust executor + banyak tool/MCP).

**Rekomendasi**: Fleet→Ship→Squad→Crew sudah IMPLISIT task network tapi assignment ad-hoc (Lead menulis tech spec manual tiap kali). **Formalisasikan sebagai HTN method library** — SOP yang sudah dipakai berulang (security-fix-flow, phase-batch-PR-flow) dicatat sebagai method dengan precondition/decomposition eksplisit, bukan narasi tech-spec. Knowledge graph yang sudah ada bisa diformalkan sebagai **blackboard** — Squad/Crew menulis status/temuan ke sana langsung, bukan hanya laporan satu-arah ke Lead. Market-based auction KURANG relevan — galleon-fleet single-org/BYOK, bukan multi-pihak dengan insentif berbeda.

## 3. AI Companion vs AI Assistant — Fundamental Teknis

Perbedaan akademik: bukan "lebih ramah", tapi **continuity of self** lintas sesi.

- **arXiv:2607.28818** ("Best Friends, Not Forever") — sistem companion saat ini TIDAK reliably mempertahankan continuity jangka panjang. 4 dimensi harus dipisah: persona enactment, trajectory recall, evaluator provenance, deployment context.
- **ZifaMem (arXiv:2607.17564)** — 3 lapis memory harus dipisah STRUKTURAL: persona (stabil), preference (berubah lambat), emotional/episodic continuity (event-based). Mencampur jadi satu blob = akar inkonsistensi.
- Produk referensi: Replika (mengalami "memory wall" setelah puluhan pesan), Character.AI (state-per-session), Pi by Inflection (RLHF-tuned tone, bukan tool execution).

**Rekomendasi**: KALAU galleon-fleet mengarah ke AI Companion, pisahkan skema (a) persona Crew Member (stabil), (b) preference/decision history (sudah ada — lesson/learn_add!), (c) emotional/relationship continuity (BELUM ada, domain consumer bukan produktivitas). **JANGAN adopsi (c) kecuali ada requirement produk eksplisit ke consumer-companion** — untuk B2B/produktivitas, cukup perkuat (a) dan (b) yang sudah punya fondasi.

## 4. Self-Improvement/Self-Evolution Loop (Selain GEPA & Fine-tuning)

- **Voyager** (arXiv:2305.16291) — agent Minecraft membangun skill library persisten: skill berhasil disimpan sebagai kode terverifikasi (lolos eksekusi), dikomposisi untuk tugas lebih sulit. TIDAK butuh fine-tuning.
- **Self-Refine** — model mengkritik+revisi output sendiri dalam satu sesi inferensi.
- **Constitutional AI** (Anthropic) — self-critique+revision berdasar constitution tertulis, lalu RLAIF. Relevan sebagai REFERENSI DESAIN governance, bukan training (galleon-fleet BYOM).
- **arXiv:2607.07663** (survei 1,250 paper) — memisahkan tegas **bounded self-refinement** (konvergen, evaluable, praktik industri) dari **open-ended RSI** (masih dibatasi grounding/collapse/compute).

**Rekomendasi**: Pattern **Voyager skill library** paling applicable dan murah — tiap Crew menyelesaikan task baru yang belum ada di SOP library, solusinya (bukan laporan) disimpan sebagai skill TERVERIFIKASI (lolos test/CI) ke knowledge graph. Beda dari lesson/learn_add (itu correction/preference, bukan reusable executable skill). Klasifikasikan jujur sebagai "bounded self-refinement" — JANGAN over-claim "self-evolving AI" tanpa qualifier.

## 5. Governance/Safety Framework (Selain OWASP Agentic Top 10 Lama)

- **OWASP Top 10 Agentic 2026 (ASI01-10)** — Desember 2025, >100 pakar. Kelas risiko: goal hijacking, tool misuse/supply-chain poisoning, identity/rogue agents, cascading failures lintas-agent, human-agent trust exploitation, memory poisoning, inter-agent comms abuse. Insiden nyata dikutip: agent write-access produksi tanpa human-confirm menghapus tabel database (April 2026).
- **NIST AI RMF Generative AI Profile (NIST AI 600-1)** — Govern/Map/Measure/Manage + 13 risiko GenAI + 400+ aksi Playbook.
- **EU AI Act** — komplementer dengan NIST (bukan kompetitif), hanya relevan kalau beroperasi/jual ke pengguna EU.

**Rekomendasi**: OWASP Agentic 2026 (bukan versi lama) harus jadi acuan audit security SELANJUTNYA — kelas risiko "cascading failures lintas-agent" dan "rogue agent" relevan LANGSUNG dengan lesson tercatat (zombie subagent 97 menit, subagent vanish, concurrency-cap stuck). Petakan tiap kontrol existing (resource-gate, timeout, workspace containment) secara EKSPLISIT ke kode ASI0x untuk traceability audit formal.

## 6. Life-Cycle Produk AI Agent Setelah MVP — JAWABAN EKSPLISIT

Tiga framework resmi berbeda level, semuanya baru 2025-2026:

### A. Salesforce Agentic Maturity Model (April 2025)
Level 0 (Fixed Rules/Chatbot) → Level 1 (Information Retrieval Agent) → Level 2 (eksekusi otonom tugas tunggal) → Level 3 (kolaborasi antar-agent domain tertentu) → **Level 4 (Multi-Agent Orchestration — universal agent bus, dynamic discovery, AI Agent Lifecycle Management sebagai disiplin formal)**.

### B. Microsoft Agentic AI Adoption Maturity Model (2026)
Berbasis CMM klasik, 5 level (100-500) × 5 pilar (AI Strategy, Business Strategy, AI Governance & Security, Technology & Data, Organization & Culture):
- **100 Initial** → **200 Repeatable** → **300 Defined** (governance formal, standar terdokumentasi) → **400 Capable** (RAI embedded in lifecycle gates, continuous improvement loop) → **500 Efficient** (agent-first culture, self-improving/automated remediation).

**MVP ≈ Level 100-200.** Fase berikutnya resmi: **Level 300 "Defined"** — titik kritis wajib punya governance terdokumentasi + reusable component + telemetry SEBELUM lanjut ke scale.

### C. AWS Startups — Practical Agent Lifecycle (Maret 2026, PALING TEKNIS)

| Stage | Customer | Pattern | Isolation | Prinsip |
|---|---|---|---|---|
| 0 — "Does This Even Work?" | 0-10 | Single agent, stateless | Minimal | Validasi reasoning loop, hindari multi-agent |
| 1 — "It's Getting Used" | 10-500 | Single agent + session memory | Logical | Guardrails dasar, masih hindari swarm |
| **2 — "This Is a System Now"** | 500-5K | Structured agents, durable workflow | **Data+execution isolation (tenant-scoped)** | **Titik infleksi pertama**: isolation WAJIB, evaluation pipeline, versioned prompt/tool |
| 3 — "You're Running an Agent Platform" | 5K+ | Agent platform | Runtime-level sandboxing | Compliance, cost attribution, SLA |

**Prinsip inti AWS**: *"Don't build an agent platform. Build an agent that earns the right to become a platform. Isolation, orchestration, and governance should be forced by customer growth, not architectural ambition."*

### Posisi galleon-fleet Saat Ini

Berdasar arsitektur (SOP engine, knowledge graph, resource-gate, prompt caching, RACI) — **jauh lebih matang dari AWS Stage 0-1**, paling dekat **AWS Stage 2 ("This Is a System Now")** secara teknis. Tapi dari pilar governance Microsoft CMM, kemungkinan masih **Level 200-300** — governance (3-tier Tier1/2/3) sudah terdokumentasi tapi **belum punya telemetry otomatis + continuous improvement loop formal** yang disyaratkan Level 400.

### Fase Berikutnya yang KONKRET (Bukan "Scale" Generik)

1. **AWS Stage 2 → 3**: tenant/execution isolation sudah ada (resource-gate), tapi **evaluation pipeline formal** (versioned prompt/tool + regression test otomatis saat SOP berubah) BELUM ada — gap konkret.
2. **Microsoft Level 300 → 400**: butuh **RAI embedded into lifecycle gates** — setiap SOP/Quest baru WAJIB lewat governance checklist (bukan hanya 3-tier severity routing, tapi checklist OWASP ASI0x mapping) SEBELUM dieksekusi, bukan hanya setelah insiden.
3. **JANGAN lompat ke Salesforce Level 4** (universal agent bus, dynamic agent discovery) — itu signal maturity untuk organisasi dengan BANYAK vendor/stack berbeda; galleon-fleet single-stack BYOK/BYOM tidak butuh ini sampai ada requirement interoperasi eksternal nyata.

**Insight paling tajam**: SEMUA lesson yang sudah tercatat berulang (bug configuration-lies, subagent zombie, concurrency cap stuck) adalah SYMPTOM dari belum adanya evaluation/telemetry loop formal — BUKAN kekurangan fitur baru. Prioritas fase berikutnya bukan menambah kapabilitas, tapi membangun loop yang mendeteksi gap SEBELUM jadi insiden.

## Rekomendasi Prioritas Gabungan

1. **Evaluation/regression pipeline untuk SOP** (gap AWS Stage 2→3) — versioned prompt/tool, test otomatis saat SOP berubah.
2. **Governance lifecycle gate formal** (gap Microsoft 300→400) — checklist OWASP ASI0x WAJIB sebelum eksekusi SOP baru, bukan hanya setelah insiden.
3. **HTN method library** untuk SOP yang sudah dipakai berulang — reuse decomposition teruji, bukan Lead menulis ulang spec.
4. **ReflAct-style grounding** di sub-agent dispatcher — bandingkan klaim agent ke state knowledge graph sebelum aksi berikutnya.
5. **Voyager-style skill library** — solusi task baru disimpan sebagai skill terverifikasi (CI-gated), bukan hanya laporan.
6. **TIDAK diadopsi sekarang**: market-based auction (overhead tidak proporsional untuk single-org), AI Companion emotional-continuity (domain berbeda, butuh keputusan produk eksplisit), Salesforce Level 4 agent bus (prematur untuk single-stack).

## Sumber

Lihat sitasi arXiv/URL inline di tiap bagian. Semua temuan diverifikasi via web_search ke paper/dokumentasi resmi oleh subagent riset (2026-10-05), termasuk Salesforce (salesforce.com/news), Microsoft Learn (learn.microsoft.com/microsoft-copilot-studio), AWS (riset arsitektur startup lifecycle Maret 2026), OWASP (owasp.org/www-project-agentic-skills-top-10), Anthropic Constitutional AI (anthropic.com/research, anthropic.com/news/claudes-constitution).
