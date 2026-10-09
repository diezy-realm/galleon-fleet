---
title: Backlog — Knowledge-Base Citation & Library Activation
status: backlog
date: 2026-10-05
author: squad-lead
depends_on:
  - research/rag-vector-knowledge-base-audit.md
  - archive/research-closed-loop/perplexity-web-browsing-methodology.md
---

# Backlog — Knowledge-Base Citation & Library Activation

> Diturunkan dari `research/rag-vector-knowledge-base-audit.md` + diskusi kombinasi Kiro Crew/GitHub Copilot. Nomor lanjutan dari BL-47 (`token-efficiency-backlog.md`). Lima keputusan Achmad (2026-10-05): (1) setuju semua rekomendasi, (2) aktifkan `knowledge.rs` sebagai Knowledge Library, (3) koordinasi `BG-B4` DITUNDA sampai tim Go selesai duluan, (4) terapkan query-decomposition+domain-filtering ke SOP riset, (5) riset Perplexity murni referensi.

---

## BL-48 — Field Citation pada Node `Lesson`/`Decision` di Knowledge Graph (P1, MEDIUM)

**Rujukan:** Pola GitHub Copilot Memory (`research/rag-vector-knowledge-base-audit.md` §3).

**Scope:** Tambah field citation ke `KnowledgeNode` (`crates/clawcrew-memory/src/knowledge_graph.rs`): `citations: Vec<Citation>` dengan `Citation { file_path: String, line_range: (usize, usize), content_hash: String }`. Saat node `Lesson`/`Decision` dibuat via `KnowledgeTool::capture`, kalau klaimnya menyebut lokasi kode spesifik, sitasi WAJIB diisi. Tambah fungsi `verify_citations(&self, node_id: &str) -> Vec<CitationStatus>` yang membaca ulang `file_path` pada `line_range`, hash isinya, dan bandingkan ke `content_hash` tersimpan — `Stale` kalau beda, `Missing` kalau file/baris tidak ada lagi, `Verified` kalau cocok.

**Titik integrasi:** `verify_citations` dipanggil otomatis saat `graph_neighbors`/`search`/`expert_find` mengembalikan node bersitasi — hasil `Stale`/`Missing` ditandai eksplisit di output tool (bukan disembunyikan), supaya agent yang membaca tidak mempercayai klaim usang secara diam-diam.

**File ownership:** Backend-Rust — `crates/clawcrew-memory/src/knowledge_graph.rs` (struct + fungsi verifikasi), `crates/clawcrew-tools/src/knowledge_tool.rs` (wiring ke output `search`/`graph_neighbors`).

**Definition of Done:** Test: node dengan sitasi ke baris yang masih identik → `Verified`; sitasi ke baris yang kontennya sudah berubah → `Stale`; sitasi ke file yang sudah terhapus → `Missing`. Output tool search menyertakan status sitasi per node, tidak disembunyikan.

**Dependency:** Tidak ada — isolated ke modul yang sudah aktif.

**Risiko/Severity:** MEDIUM — mitigasi arsitektural langsung untuk kelas bug "configuration-lies" yang berulang ditemukan sesi riset ini (WhatsApp signature, capture_tool_input, query_by_similarity).

---

## BL-49 — Aktifkan `clawcrew-api/src/knowledge.rs` sebagai Knowledge Library (P1, L)

**Rujukan:** Pola Kiro Crew Knowledge Library (`research/rag-vector-knowledge-base-audit.md` §2). Keputusan Achmad: aktifkan, bukan hapus.

**Scope:** Tipe `KnowledgeSource`/`KnowledgeDocument`/`KnowledgeVersion`/`KnowledgeChunk`/`IndexJob`/`IndexStatus` sudah terdefinisi lengkap tapi nol dipanggil. Bangun jalur eksekusi nyata: (1) tool baru atau extend `knowledge_tool.rs` dengan action `add_source`/`index_folder`/`search_library` yang menjalankan pipeline chunk→index→embed (reuse `EmbeddingProvider` trait yang sudah ada di `clawcrew-memory/src/embeddings.rs` — JANGAN bangun embedding provider baru), (2) fallback keyword (FTS5, pola yang sudah ada di `knowledge_graph.rs`) saat model embedding tidak tersedia — pola "hybrid vector+keyword graceful degradation" ala Kiro Crew, (3) target awal: index `standard-docs/` sebagai sumber pertama, supaya riset/backlog/roadmap yang sudah menumpuk sesi ini jadi searchable secara semantic, bukan hanya lewat `grep`/`graphify`.

**File ownership:** Backend-Rust — `crates/clawcrew-api/src/knowledge.rs` (tipe sudah ada, tambah impl), `crates/clawcrew-tools/src/knowledge_tool.rs` ATAU tool baru (`knowledge_library_tool.rs` kalau scope terlalu besar untuk extend yang lama — keputusan di waktu eksekusi), `crates/clawcrew-memory/src/embeddings.rs` (reuse, bukan bangun baru).

**Definition of Done:** Test: index satu folder `standard-docs/` kecil (fixture), search query mengembalikan chunk relevan dengan score; matikan embedding provider (simulasikan model tak tersedia) → fallback keyword tetap mengembalikan hasil, tidak error total.

**Dependency:** Tidak ada hard dependency ke BL-48, tapi SETELAH BL-48 lebih koheren (supaya pola citation-verification dan knowledge-library sama-sama hidup berdampingan dengan desain konsisten).

**Risiko/Severity:** MEDIUM-HIGH — scope paling besar di backlog ini (L), tapi reuse infrastruktur existing (`EmbeddingProvider`, FTS5 pattern) sehingga bukan membangun dari nol.

---

## BL-50 — Query Decomposition + Domain-Filtering sebagai SOP Riset Squad (P2, S)

**Rujukan:** Pola Perplexity Search API Best Practices (`archive/research-closed-loop/perplexity-web-browsing-methodology.md` §4, §2). Keputusan Achmad: terapkan ke SOP, BUKAN ke kode `web_search_tool.rs`.

**Scope:** Ini TIDAK butuh perubahan kode Rust — murni dokumentasi/prompt-level. Tambahkan ke steering/dokumen SOP riset (lokasi: steering workspace atau `standard-docs/` yang relevan untuk panduan kerja squad) dua prinsip: (1) **Query decomposition**: sebelum memanggil `web_search`/`web_fetch` untuk topik riset kompleks, pecah dulu jadi beberapa sub-pertanyaan spesifik dan jalankan sebagai batch terpisah — bukan satu query generik yang berharap mendapat semua jawaban sekali panggil. (2) **Domain-filtering untuk mode akademik**: saat riset bersifat akademik/butuh sumber otoritatif, tambahkan filter domain (`site:arxiv.org`, `site:*.edu`, nama jurnal spesifik) langsung di query string yang dikirim ke `web_search` — provider yang sudah ada (AnySearch/Serply) mendukung sintaks ini secara native, tidak perlu field baru di kode.

**File ownership:** Dokumentasi — steering/SOP riset (lokasi final ditentukan saat eksekusi, kandidat: `.kiro/steering/` workspace atau `standard-docs/` panduan kerja).

**Definition of Done:** Satu dokumen/paragraf tersimpan yang menyatakan kedua prinsip ini sebagai praktik standar riset squad, dengan contoh konkret format query.

**Dependency:** Tidak ada.

**Risiko/Severity:** LOW — murni dokumentasi, nol risiko kode, langsung bisa dipraktikkan di sesi riset berikutnya.

---

## BL-51 — Koordinasi dengan `BG-B4` (dev-tasks, Memori Vektor/RAG Go) — DITUNDA (P3)

**Rujukan:** `standard-docs/decisions/finalize/dev-tasks/01-be-go-engine.md` — `BG-B4 · G-13 · PR-43`: "Memori vektor/RAG per scope Fleet/Ship/Squad/Crew/Run", depends `BG-B2` (store SQLite Go).

**Konteks:** `BG-B4` adalah vector MEMORY (percakapan per hierarki organisasi) di Go — scope BERBEDA dari knowledge-graph-citation (BL-48) dan Knowledge Library (BL-49) yang keduanya di Rust (`clawcrew-memory`/`clawcrew-api`). Tidak ada tumpang tindih implementasi langsung, tapi KEDUANYA menyentuh domain "RAG/vector" dan berpotensi menghasilkan dua desain paralel yang tidak sinkron kalau dikerjakan terpisah tanpa koordinasi.

**Keputusan Achmad (2026-10-05): "mungkin sudah dikerjakan, tapi kita bereskan kembali setelahnya"** — scope ini SENGAJA DITUNDA. Jangan assign ke squad sekarang. Tunggu tim yang mengerjakan `dev-tasks/` (kemungkinan tim lain, sesuai lesson folder itu dikecualikan dari auto-maintenance) menyelesaikan `BG-B4`/`BG-B2` dulu.

**Next action saat diangkat nanti:** cek status `BG-B4` di `dev-tasks/01-be-go-engine.md` (apakah sudah `[x]`), lalu rujuk balik ke BL-48/BL-49 untuk memastikan desain Go (vector memory per-scope) dan desain Rust (knowledge graph citation + knowledge library) tidak membangun dua pola vector/RAG yang inkonsisten secara filosofi (misal: satu sisi vector-first, sisi lain citation-first tanpa alasan yang didokumentasikan).

**File ownership:** Tidak ada — ini item koordinasi/observasi, bukan implementasi.

**Dependency:** `BG-B4`/`BG-B2` selesai dulu (dikerjakan tim lain, bukan squad ini).

**Risiko/Severity:** LOW saat ini (murni catatan tunda) — berpotensi MEDIUM kalau diabaikan sampai kedua desain sudah jauh berjalan sendiri-sendiri.

---

## Urutan Eksekusi yang Disarankan

1. **BL-50** dulu — paling murah (dokumentasi), bisa langsung dipraktikkan sesi riset berikutnya.
2. **BL-48** — fondasi citation-verification di knowledge graph yang sudah aktif.
3. **BL-49** setelah BL-48 — aktivasi Knowledge Library, scope lebih besar, lebih koheren setelah pola citation mapan.
4. **BL-51** tetap DITUNDA — cek berkala status `BG-B4` di dev-tasks, jangan di-assign ke squad.

## Catatan Proses

Severity keseluruhan: MEDIUM — tidak ada bug aktif yang dieksploitasi, tapi BL-48 menutup kelas bug "configuration-lies" yang sudah 3x ditemukan di sesi riset ini (WhatsApp signature, capture_tool_input, query_by_similarity) sehingga prioritasnya naik di atas severity nominal.
