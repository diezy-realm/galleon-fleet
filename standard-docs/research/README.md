---
title: Index Riset — Peta Lengkap `standard-docs/research/`
status: index
date: 2026-10-05
author: squad-lead
---

# Index Riset `standard-docs/research/`

> Peta navigasi seluruh dokumen riset yang dihasilkan dari audit mendalam `galleon-fleet` (branch `feat/refactoring-rust`), dengan rujukan ke backlog dan roadmap yang menampungnya. Diurutkan per tema. Setiap riset HARUS diverifikasi langsung ke kode (file:baris) — bukan asumsi dari nama file atau dokumentasi, sesuai prinsip yang dipegang di seluruh sesi.

## Tema 1 — Permission & Delegasi Sub-Agent

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `agent-subagent-permission-scoping-survey.md` | Survei kompetitor (Kiro Crew, Claude Code, Copilot CLI, Antigravity, Cursor) soal permission scoping agent/sub-agent | `skills-steering-lessons-backlog.md` (BL-05) |
| `agent-subagent-ship-squad-permission-scheme.md` | Audit kode: `RiskTier`/`ExecutionContext`/`ApprovalGate` matang; temuan V10 — 2 jalur delegasi paralel, 1 tanpa ceiling-check | `skills-steering-lessons-backlog.md` (BL-05) |

## Tema 2 — Capabilities, Skills, Lessons, Steering

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `capabilities-skills-lessons-steering-audit.md` | 3-layer skill loading (Rust); "Learning by Consent" ADA di Go (`proposal.go`) tapi storage murni in-memory nol persistence; GAP-01 steering produk end-user belum ada | `skills-steering-lessons-backlog.md` (BL-01..04) |

## Tema 3 — SOP, Workflow, RACI, Role

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `orders-quest-sop-workflow-role-audit.md` | SOP engine Rust matang; Do's/Don'ts cuma 1 persona generik; `Quest` Go skeleton 4 field | `sop-raci-role-quest-backlog.md` (BL-06..09) |

## Tema 4 — Automation, Cron, Webhook

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `automation-cron-webhook-audit.md` | Cron scheduler Rust sangat matang; **gap kritis**: channel biasa (non-SOP) tidak lewat `ContentSafety` — prompt-injection terbuka di SEMUA channel, 1 titik perbaikan menutup semua | `automation-cron-webhook-backlog.md` (BL-10..14) |

## Tema 5 — Resource Management Sub-Agent

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `subagent-resource-management-audit.md` | Nol resource gate sebelum spawn sub-agent (penyebab crash yang dilaporkan); `LoopDetector` 3-pola matang tapi hasil deteksi tidak pernah sampai ke dashboard; jalur masuk `ExecuteTurn` dari Go `spawn_subagent` nol gate juga | `subagent-resource-management-backlog.md` (BL-15..18, **BL-18 DITUNDA** — tunggu BG-D1/BG-D4 dev-tasks) |

## Tema 6 — Plugin WASM/WIT & MCP

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `plugin-mcp-architecture-audit.md` | Plugin WASM sangat matang (sandboxing, fuel limit, egress deny-by-default); MCP client **nol sandboxing OS-level** untuk proses stdio server | `plugin-mcp-architecture-backlog.md` (BL-19..23) |
| `fleet-wide-mcp-governance-followup.md` | Koreksi: `fleet/policies.go` BUKAN tempat governance MCP (klaim awal salah); Go nol tahu konsep MCP; keputusan Owner `Q-2` sudah menghapus MCP dari Go secara sadar | `plugin-mcp-architecture-backlog.md` (**BL-24 EXPLORATORY — butuh keputusan Owner, tidak diangkat ke roadmap aktif**) |

## Tema 7 — Provider LLM (Reliability, Auth/OAuth) & Memory Governance

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `providers-reliability-auth-audit.md` | Cooldown/fallback matang; auth OAuth solid (ChaCha20-Poly1305) tapi `client_id` hardcoded + nol isolasi multi-tenant | `providers-memory-governance-backlog.md` (BL-25..29) |
| `oauth-client-id-opensource-implications.md` | Analisa lanjutan `client_id` hardcoded — PKCE benar secara standar, risiko murni operasional (bukan keamanan), rekomendasi BYOC | `providers-memory-governance-backlog.md` (BL-25) |
| `memory-data-governance-audit.md` | Primitif delete nyata (DELETE SQL sungguhan), TAPI tidak ada single entry-point "hapus semua data saya"; hygiene retensi hanya jalan di SQLite | `providers-memory-governance-backlog.md` (BL-30..31) |

## Tema 8 — Channel Signature, CI/CD, Observability

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `channel-signature-verification-audit.md` | **3 iterasi koreksi** — final: `verify_whatsapp_signature`/`optional_channel_routes` dirujuk banyak test TAPI tidak ada definisinya di manapun (bug nyata, terverifikasi independen grep seluruh repo) | `channel-cicd-observability-backlog.md` (BL-32) |
| `cicd-supplychain-audit-partial.md` | CI/CD jauh lebih matang dari placeholder dev-tasks (SLSA, cosign, SBOM dua layer) TAPI Windows Authenticode signing nol | `channel-cicd-observability-backlog.md` (BL-33..36) |
| `observability-content-pii-audit.md` | LLM request payload default off (benar); tool I/O: API redaksi `capture_tool_input`/`capture_tool_output` dirancang benar TAPI nol dipanggil produksi — "konfigurasi berbohong", dikonfirmasi 2 sub-agent independen | `channel-cicd-observability-backlog.md` (BL-37..39) |

## Tema 9 — Lain-lain (Closed-Loop, Tidak Perlu Backlog Baru)

| Dokumen | Isi Ringkas | Status |
|---|---|---|
| `verifiable-intent-assessment.md` | Penilaian modul `verifiable_intent` (dibuat sebelum sesi riset ini) — rekomendasi hapus modul (DAC concept dipertahankan sebagai ide masa depan). §3c: keputusan Owner 2026-10-05 — arah "bayar" produk berpotensi meluas ke agentic commerce, G-37/DAC di-upgrade jadi gate wajib sebelum desain fitur commerce (bukan lagi opsional) | **Sudah closed-loop** ke roadmap existing `R-38`/`G-37`/`PR-59`/`PR-99` — tidak perlu BL baru |
| `research-gap-map-2026-10-05.md` | Peta gap riset — menandai topik yang belum tersentuh (`clawcrew-providers`, memory governance, channel/CI-CD/observability) dan prioritasnya | Dokumen **meta/navigasi**, bukan audit temuan — tidak perlu BL sendiri; isinya sudah dieksekusi jadi riset Tema 7-8 di atas |
| `product-industry-competitor-insight.md` | Riset industri/kompetitor manual (Sovereign AI, Kiro Crew, Antigravity 93-subagent) + arah teknis Lead (pemetaan backlog ke dampak posisi produk) | Dasar diskusi Positioning Bet A/B/C — ditindaklanjuti di `product-research/` |

## Tema 10 — Payment Lockout Gate (Preventif, Permintaan Owner)

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `payment-lockout-gate-industry-research.md` | Riset pembanding industri untuk skema pencegahan agent melakukan pembayaran: AP2 v0.2 (Checkout/Payment Mandate, Direct/Autonomous mode), MCP tool annotations (hint tidak bisa dipercaya, enforcement harus di runtime), OpenAI/Stripe/Comet (konsisten scoped-token, default minta izin per-aksi), insiden nyata Guardio/Comet (checkout tanpa konfirmasi manusia) & Replit (mengabaikan code-freeze eksplisit), Plaid/PSD2 (separation read vs Payment Initiation butuh approval terpisah), OWASP Agentic Top 10 ASI03/ASI09/ASI10 (Least Agency, state mentah bukan ringkasan agent, kill-switch wajib berfungsi) | `payment-lockout-gate-backlog.md` (BL-40..44) |

> Catatan: `decisions/payment-lockout-gate-scheme.md` (dokumen desain, bukan riset) sudah diperkuat dengan 4 temuan riset ini — lihat §1 (bukti insiden Comet), §3.1 (field `unlock_expires_at`), §3.4 (rujukan insiden Replit), §3.5 (state mentah bukan ringkasan agent).

## Tema 11 — Token/Credit Efficiency (Permintaan Owner, Pembanding Kompetitor)

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `riset-teknik-penghematan-token-ai-coding-assistant` (artifact, bukan file repo) | Riset eksternal: prompt caching Anthropic/OpenAI/Gemini (90%/50-90%/90% diskon), Copilot (embedding model kustom), Cursor (Merkle-tree incremental sync + vector DB line-range), MCP SEP-2549 (TTL list-only), Claude Code cache-safe forking untuk compaction, code-graph/LSIF/SCIP | Hipotesis awal — lihat koreksi di `token-efficiency-codebase-audit.md` |
| `token-efficiency-codebase-audit.md` | **Audit koreksi**: 2 dari 3 hipotesis riset eksternal TERBUKTI TIDAK APPLICABLE setelah baca kode nyata — `cache_passthrough` hanya untuk compatible-gateway (native Anthropic sudah cache otomatis tanpa toggle), `defer_loading` MCP tidak relevan (MCP server statis per-proses, nol hot-reload). Gap nyata yang ditemukan: rolling cache-breakpoint tidak dikoordinasikan dengan titik potong trim history — setiap trim membatalkan cache yang terbentuk | `token-efficiency-backlog.md` (BL-45..47) → **diangkat ke roadmap 2026-10-05 sebagai R-88 s.d. R-91** |

> Pelajaran proses yang sama berulang: riset eksternal memberi kosakata/hipotesis yang baik, tapi WAJIB diverifikasi ke implementasi nyata sebelum ditulis sebagai backlog.

## Tema 12 — RAG/Vector/Knowledge-Base & Metodologi Web Browsing

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `rag-vector-knowledge-base-audit.md` | Audit internal: `knowledge_graph.rs` aktif (node Pattern/Decision/Lesson/Expert) TAPI `query_by_similarity` ternyata FTS5 keyword, bukan vector (nama menyesatkan — pola bug "configuration-lies" ke-3 di sesi ini). `clawcrew-api/src/knowledge.rs` skema Knowledge-Library lengkap tapi dead code. Riset kompetitor: Kiro Crew (hybrid vector+keyword), GitHub Copilot Memory (citation-based, sengaja hindari vector, +7% merge rate), OpenClaw & Hermes (markdown-first, bukan vector-primary), Antigravity (klaim shared-knowledge-93-subagent TIDAK terverifikasi), ZeroClaw (tidak terkait galleon-fleet) | `knowledge-base-citation-backlog.md` (BL-48..51) → **diangkat ke roadmap 2026-10-05 sebagai R-92 s.d. R-95** |
| `perplexity-web-browsing-methodology.md` | Riset metode Perplexity AI (crawler PerplexityBot + API hybrid, span-level citation, query decomposition, Deep Research plan→search→read→re-plan) + peringatan etika (Cloudflare/Wired: stealth user-agent/ASN untuk lewati robots.txt, berujung gugatan). **Murni referensi** — keputusan Owner: tidak diimplementasikan sebagai fitur, hanya 2 pola (query decomposition, domain-filtering) diadopsi ke SOP riset (R-94) | `knowledge-base-citation-backlog.md` (BL-50/R-94) |

## Tema 13 — Attachment Handling (Bug Fix + Multi-Format + 2-Layer Validation)

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `document-volume-concurrency-readiness-audit.md` | Audit kesiapan file-besar/concurrency untuk tim profesional: limit per-channel sudah matang tapi nol pola "upload-once-reference-by-id", SOP belum punya konsep attachment first-class, `execute_tools_parallel` nol resource gate (gap industri luas, bukan unik galleon-fleet) | `attachment-handling-backlog.md` (BL-52..56) |
| `attachment-debounce-bug-and-layer-design.md` | **Bug nyata terverifikasi sampai baris kode**: `MessageDebouncer` membuang attachment pesan ke-2+ saat menggabungkan pesan rapid (hanya teks digabung). Riset skema Gemini `Part`/`Blob`/`FileData` vs Claude `source.type` seragam (base64/url/file) sebagai referensi desain. Desain 2-layer: Front Apps validasi count=10 di titik masuk, Core agregasi+resource-gate bytes | `attachment-handling-backlog.md` (BL-52..56) |

## Tema 14 — SOP Constraint Layer (Role-Based Separation of Duties)

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `agentic-ai-fundamentals-and-lifecycle-phase.md` | Riset fundamental Agentic AI/Orchestrator/Companion + jawaban eksplisit soal fase lifecycle produk pasca-MVP (Salesforce/Microsoft/AWS maturity model) — memicu klarifikasi fundamental produk Galleon sebagai AI Companion yang mengadopsi struktur Perusahaan nyata (Fleet=Perusahaan, Ship=Departemen, Squad=Tim, Crew=individu permanen 1-role) | Memicu desain `decisions/sop-eval-telemetry-loop-design.md` dan `sop-constraint-layer-design.md` |
| `decisions/sop-eval-telemetry-loop-design.md` | Desain evaluation/telemetry loop ADVISORY (bukan blocking) untuk SOP engine, dieksekusi Lead→Backend→Principle (bukan Owner), divalidasi lewat skenario kerja nyata squad sebagai `SopRun` test-case — bukan dokumen `.md` | Belum ada backlog terpisah — tersambung ke BL-60 |
| `decisions/sop-constraint-layer-design.md` | **Temuan kunci**: diagram arrow (BPMN/n8n) TIDAK BISA merepresentasikan larangan role — bahkan Camunda/jBPM tidak native punya Separation-of-Duty. Desain two-layer: Flow (existing) + Constraint (baru, `SopConstraint`/`RoleRestriction`/`DistinctPerformer`), enforcement via hook `preToolUse` yang sudah ada | `sop-constraint-backlog.md` (BL-57..61) |

---

## Pola Berulang Lintas-Tema (Insight Meta)

Tiga pola arsitektur ditemukan BERULANG di banyak tema independen — dicatat di sini supaya tidak "ditemukan ulang" sebagai hal baru di riset berikutnya:

1. **Go dan Rust punya sistem paralel untuk konsep yang sama, tanpa jembatan RPC.** Memory (Go in-memory vs Rust persistent), MCP (dulu di Go, sekarang Rust-only by decision), delegasi sub-agent (`DelegateTaskTool` Go vs `DelegateTool` Rust vs `ExecuteTurn` langsung). Proto `SystemGateway` cuma 3 RPC (`ExecuteTurn`/`ExecuteNativeTool`/`GetDecryptedSecret`) — bottleneck berulang.
2. **API/mekanisme yang dirancang benar tapi tidak pernah di-wire ke call-site produksi.** `capture_tool_input`/`capture_tool_output` (observability), kemungkinan `verify_whatsapp_signature` (channel — masih perlu `cargo test` untuk ground-truth). Pelajaran: menemukan ENTRY di registry/config TIDAK CUKUP untuk simpulkan fitur berfungsi — harus ditelusuri sampai implementasi nyata yang dipanggil.
3. **Rigor keamanan tidak konsisten antar-subsistem yang risikonya sejenis.** WASM plugin sandboxing sangat ketat; MCP stdio nol sandboxing; native tool shell ketat. Sistem yang membuka pintu ke kode/proses eksternal seharusnya dikontain SETARA, tapi faktanya bervariasi tanpa alasan risk-based yang jelas.

## Tema 15 — UI/UX Parity & Nice-to-Have (Referensi KiroCrew Toolbox)

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `ui-right-rail-journal-newchat-audit.md` | Audit 3 fitur UI referensi KiroCrew: Right Rail Git/Sub-Agent tab (ADA tapi isi beda total), Captain's Journal workspace grouping (flat, bug `selectedWorkspace` global salah-konteks), "+ New Chat" entry-point (nol tombol). Koreksi: `Workspace` scoped Journal saja, `Quest.workspaceId` dihapus (vestigial) | `ui-nice-to-have-backlog.md` (BL-62..70) |
| `toolbox-parity-audit.md` | Audit 8 fitur toolbox KiroCrew vs galleon-fleet (approval-mode, attachment, quest-loop, context-bar, chat-to-entity, @/slash, project-bar, optimize-prompt) — pola backend-matang-UI-salah-bentuk (approval, project-bar) vs backend-ada-bug (attachment, re-konfirmasi BL-52) vs nol-total (quest-loop, chat-to-entity, @/slash, optimize-prompt) | `toolbox-parity-backlog.md` (BL-71..78) |

## Tema 16 — Tier, Pricing, dan Entitlement (Pematangan Lanjutan, Menggantikan Riset Lama)

| Dokumen | Isi Ringkas | → Backlog |
|---|---|---|
| `community-tier-licensing-enforcement.md` | **status: superseded** (2026-10-06) — angka Community lama (≤2 Ship/≤3 Squad), provenance ZeroClaw (Bagian 2 TETAP berlaku, lihat `license-provenance-attribution-research.md`), Metode D awal | → `tier-pricing-and-entitlement-final.md` |
| `tier-enforcement-method-d-c-deep-dive.md` | **status: superseded** (2026-10-06) — 5 bug arsitektur (TETAP berlaku, dikutip ulang di dokumen baru), Q1-Q5 terbuka (sudah dijawab) | → `tier-pricing-and-entitlement-final.md` |
| `tier-pricing-and-entitlement-final.md` | **status: final** — tangga tier (Community/Team/Dockyard/Private Dockyard dengan dimensi baru `CrewPerShip`/`ConcurrentVoyages`), harga flat ($15/$28), model entitlement D (pemisah paket) + P2 (token Ed25519 di modul tertutup REPO TERPISAH), 5 bug arsitektur diverifikasi ulang ke kode nyata (`ship`/`fleet`/`squad`/`entitlement` packages) | `tier-entitlement-pricing-backlog.md` (BL-80..82) |

## Lihat Juga

- `standard-docs/backlog/README.md` — index backlog BL-01 s.d. BL-78 dengan status roadmap.
- `standard-docs/roadmap/R-rust.md` — item roadmap resmi (R-00 s.d. R-87) yang diangkat dari backlog ini.
