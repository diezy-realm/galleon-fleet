---
title: Audit Mendalam — RAG/Vector/Knowledge-Base galleon-fleet vs Kompetitor
status: research
date: 2026-10-05
author: squad-lead
depends_on: research/payment-lockout-gate-industry-research.md (metodologi serupa)
---

# Audit: Sejauh Mana galleon-fleet Sudah Membangun RAG/Vector/Knowledge-Base

## 1. Temuan Internal — Arsitektur Lebih Matang dari Asumsi Awal, TAPI Satu Nama Menyesatkan

galleon-fleet **sudah punya infrastruktur RAG/knowledge yang signifikan**, tersebar di dua tempat:

### a. `clawcrew-memory` (Rust) — Knowledge Graph + Vector Memory
- **Knowledge Graph** (`crates/clawcrew-memory/src/knowledge_graph.rs`): node types (`Pattern`, `Decision`, `Lesson`, `Expert`, `Technology`, `Client`, `Contact`, `Interaction`) + relasi berarah (`Uses`, `Replaces`, `Extends`, `AuthoredBy`, dst), backend SQLite lokal (`db_path`), fungsi `add_node`, `find_related`, `find_outbound`/`find_inbound`, `query_by_tags`.
- **Vector memory backend terpisah**: `embeddings.rs` (trait `EmbeddingProvider`, pluggable), `qdrant.rs` (vector DB eksternal opsional), `vector.rs`, `rerank.rs` — ini untuk Memory (percakapan), bukan Knowledge Graph.
- **Tool aktif** (`clawcrew-tools/src/knowledge_tool.rs`, 52 referensi, benar-benar dipakai): action `capture`/`search`/`relate`/`suggest`/`expert_find`/`lessons_extract`/`graph_stats`/`graph_neighbors` — expose knowledge graph ke agent.

### b. KOREKSI PENTING: `query_by_similarity` adalah KEYWORD search, bukan vector
Nama fungsi `query_by_similarity` (`knowledge_graph.rs:317`) **menyesatkan** — setelah dibaca implementasinya, ini adalah **SQLite FTS5 full-text search** (`nodes_fts MATCH`, BM25-style ranking), BUKAN embedding/vector cosine-similarity. Tidak ada pemanggilan `EmbeddingProvider` di `knowledge_graph.rs` sama sekali (dikonfirmasi grep: nol match untuk `embed`/`vector` di file itu).

**Ini pola bug yang SAMA PERSIS** dengan temuan sesi-sesi sebelumnya (`verify_whatsapp_signature` tidak ada fn-nya; `capture_tool_input` dibangun tapi nol dipanggil): **nama fungsi mengklaim kapabilitas yang implementasinya tidak benar-benar punya**. Risiko konkret: siapa pun yang membaca kode cepat (termasuk saya di awal audit ini) akan menyimpulkan "knowledge graph sudah vector-search", padahal yang terjadi adalah keyword matching biasa dibungkus nama yang terdengar seperti semantic search.

### c. Dead code yang ditemukan: `clawcrew-api/src/knowledge.rs`
Tipe `KnowledgeSource`/`KnowledgeDocument`/`KnowledgeChunk`/`KnowledgeVersion`/`IndexJob`/`IndexStatus`/`Lesson` — skema yang terlihat SEPERTI desain "Knowledge Library" ala Kiro Crew (chunk→index→embed, dengan versioning dan provenance) — **tapi nol referensi di luar file definisinya sendiri** (dikonfirmasi grep seluruh `crates/`). Ini tipe data yang dirancang tapi tidak pernah di-wire ke implementasi nyata — kemungkinan rencana desain yang belum dieksekusi, bukan fitur aktif.

### d. Sisi Go (`engine/src/memory/`)
Ada `store.go`, `proposal.go` (mekanisme "Learning by Consent" yang sudah diaudit sesi sebelumnya — in-memory, nol persistence), `vector_validation_test.go`, `deterministic_ranking_test.go`. Scope audit sesi ini tidak mendalami Go memory lebih jauh — sudah tercakup sebagian oleh audit `capabilities-skills-lessons-steering-audit.md` sesi sebelumnya.

## 2. Perbandingan Kompetitor (Riset Eksternal Lengkap)

Riset detail tersimpan terpisah: lihat ringkasan di bawah, detail lengkap ada di percakapan sesi ini (subagent `fe3d8b21`).

**Pola paling signifikan yang muncul BERULANG dari sumber independen (Kiro CLI, OpenClaw, Hermes LLM Wiki, tesis Karpathy, bahkan GitHub Copilot Memory)**: mayoritas platform matang 2026 **TIDAK menjadikan vector-DB sebagai knowledge primary**. Markdown/file-based ground truth + vector sebagai lapisan OPSIONAL pelengkap adalah pola dominan, bukan vector-first.

| Platform | Mekanisme Utama | Vector? | Insight Kunci |
|---|---|---|---|
| **Kiro Crew (Knowledge Library)** | Chunk→index→embed, hybrid vector+keyword fallback | Ya, dengan graceful degradation ke keyword | `local_knowledge_search` tool yang dipakai sesi ini ADALAH implementasi fitur ini |
| **Kiro CLI (`/knowledge`)** | User pilih sadar BM25 ("Fast") vs `all-MiniLM-L6-v2` ("Best") | Opsional, pilihan eksplisit | Trade-off diserahkan ke user, bukan auto-decided |
| **KiroGraph (komunitas)** | AST graph (tree-sitter) deterministik + embedding opsional | Opsional overlay | Klaim -90% token usage vs grep manual; auto-sync via file-save hook |
| **GitHub Copilot Memory** | **Citation-based**: fakta + `file:line` + re-verifikasi real-time setiap dipakai | TIDAK (sengaja dihindari) | +7% PR merge rate (p<0.00001); self-heal saat sitasi usang terdeteksi |
| **OpenClaw** (374K★) | Markdown files = source of truth; RAG hanya plugin opsional | Plugin only (hybrid 70% vector/30% BM25 jika dipasang) | "No built-in enterprise vector store" — kutipan langsung arsitek |
| **Hermes Agent — LLM Wiki** | Markdown + wikilink + sha256 drift detection + confidence/contested frontmatter | TIDAK — eksplisit alternatif RAG | "Compile once, keep current" vs "re-retrieve every query" |
| **Google Antigravity** | Knowledge Items — summary file-based dari Gemini 2.5 Flash | **Tidak terverifikasi** sebagai vector store | Klaim "shared knowledge antar 93 sub-agent" TIDAK ada bukti resmi |
| **ZeroClaw** | 4 project berbeda, nama generik tren OSS | Tidak diketahui | **Tidak ada hubungan dengan galleon-fleet** — bukan nama historis proyek ini |

## 3. Rekomendasi Konkret (Prioritas)

1. **Perbaiki nama `query_by_similarity` → `query_by_fts`/`query_by_keyword`** (atau implementasikan vector sungguhan jika memang dimaksudkan). Minimal: perbaiki dokumentasi/komentar fungsi supaya tidak menyesatkan pembaca kode selanjutnya — ini adalah bug kelas "configuration-lies" yang sudah berulang kali ditemukan sesi ini.
2. **Hapus atau wire `clawcrew-api/src/knowledge.rs`** — tipe dead-code yang terlihat seperti rencana desain belum dieksekusi. Keputusan: lanjutkan jadi fitur nyata (chunk/index/embed penuh ala Kiro Crew Knowledge Library) ATAU hapus karena YAGNI.
3. **Adopsi pola citation-based verification (GitHub Copilot Memory) untuk Lesson/Decision node di knowledge graph** — tiap `Lesson`/`Decision` yang mengklaim "fitur X ada di file Y" sebaiknya menyimpan sitasi `file:line` dan di-reverify sebelum dipercaya ulang. Ini mitigasi arsitektural langsung untuk kelas bug "configuration-lies" yang berulang kali ditemukan di sesi-sesi riset ini (WhatsApp signature, capture_tool_input, dan sekarang query_by_similarity).
4. **Adopsi pola "Agentic Wiki" (Karpathy/Hermes) untuk `standard-docs/`** — sudah sangat paralel dengan lesson existing (drift register, execute-done/). Tambahan konkret: frontmatter `confidence`/`contested`, sha256 drift-hash per dokumen vs file kode yang dirujuknya, lint rutin (orphan docs, stale >90 hari) sebagai Tier-1 auto-exec.
5. **Untuk code-navigation, pola KiroGraph (AST graph) lebih cocok daripada vector** untuk Go+Rust — deterministik, auto-sync, tidak perlu model embedding. Tool `code` (LSP-based) yang sudah dipakai sesi ini sejalan dengan pola ini.
6. **Jangan tiru "shared vector store lintas-93-sub-agent" dari Antigravity** — klaim itu sendiri tidak terverifikasi, dan arsitektur matang (OpenClaw, Hermes) justru pakai state lokal ringan + hand-off terstruktur.

## Sumber

- `crates/clawcrew-memory/src/knowledge_graph.rs` — definisi node/relation, `query_by_similarity` (FTS5, bukan vector), `add_node`, `find_related`.
- `crates/clawcrew-memory/src/{embeddings,qdrant,vector,rerank,retrieval}.rs` — vector memory backend (untuk Memory, terpisah dari Knowledge Graph).
- `crates/clawcrew-tools/src/knowledge_tool.rs` — tool aktif yang expose knowledge graph ke agent.
- `crates/clawcrew-api/src/knowledge.rs` — tipe dead-code, nol referensi di luar file.
- Riset eksternal (subagent, 2026-10-05): Kiro Crew/CLI docs resmi, KiroGraph (dev.to/aws-builders), GitHub Copilot Memory (github.blog Jan 2026), OpenClaw (markaicode.com), Hermes Agent LLM Wiki (SKILL.md resmi), Karpathy LLM Wiki gist, AWS sample-kiro-llm-wiki, Google Antigravity (cloud.google.com/blog, discuss.ai.google.dev).
