---
title: Riset — Metode Web Browsing/Crawling Perplexity AI (Bahan Riset, Tidak Diimplementasikan)
status: research
date: 2026-10-05
author: squad-lead
note: Keputusan Achmad 2026-10-05 — murni bahan riset/referensi, TIDAK untuk diimplementasikan sebagai fitur. Dua pola kecil (query decomposition, domain-filtering akademik) diadopsi ke SOP riset squad, bukan ke kode.
---

# Riset Teknis: Metode Web Browsing/Crawling Perplexity AI

## 1. Arsitektur Retrieval Perplexity

**Hybrid — bukan murni crawler sendiri, tapi juga bukan murni reseller API pihak ketiga.**

- [Architecting and Evaluating an AI-First Search API](https://www.perplexity.ai/api-platform/resources/architecting-and-evaluating-an-ai-first-search-api): fase awal mereka bergantung pada search API eksternal (dilaporkan: Bing API + provider lain).
- [Introducing the Perplexity Search API](https://www.perplexity.ai/hub/blog/introducing-the-perplexity-search-api): klaim sekarang punya "hybrid retrieval mechanisms, multi-stage ranking pipelines, distributed indexing, dynamic parsing" sendiri.
- **PerplexityBot** (crawler resmi, user-agent `PerplexityBot/1.0`, IP range di [perplexitybot.json](https://www.perplexity.com/perplexitybot.json)), didokumentasikan di [docs.perplexity.ai/guides/bots](https://docs.perplexity.ai/guides/bots): "designed to surface and link websites in search results... not used to crawl content for AI foundation models."
- Dokumen dipecah jadi "fine-grained sub-document units" yang di-score individual — mirip passage-retrieval, bukan whole-document BM25.

**Kesimpulan**: crawler sendiri (PerplexityBot) + kemungkinan tetap pakai search-index pihak ketiga, lalu ranking/indexing proprietary di atasnya.

## 2. Fitur Resmi untuk Riset Akademik

- **Academic/Scholarly focus filtering** ([cookbook resmi](https://docs.perplexity.ai/docs/cookbook/articles/academic-search/README)): domain filter (`arxiv.org`, `nature.com`, `*.edu`) via parameter Search/Agent API — bukan model terpisah.
- **Academic Research Finder CLI** ([docs.perplexity.ai](https://docs.perplexity.ai/docs/cookbook/examples/research-finder/README)): dibangun di atas Sonar API dengan prompt khusus mengarahkan model memprioritaskan jurnal/conference proceedings.
- Tidak ada integrasi API langsung ke Semantic Scholar/PubMed/arXiv API — pendekatannya domain-filtering pada search umum, bukan koneksi database akademik terstruktur. Mendukung upload PDF, link DOI/arXiv, OCR paper (fitur produk).
- Catatan riset independen ([arXiv 2609.00248](https://arxiv.org/html/2609.00248)): "authority bias" pada conversational search engine — preferensi ke paper berdasar prestige penulis/venue, bukan konten.

## 3. Metode Sitasi (Citation)

- [Better Extraction, Dynamic Benchmarks](https://www.perplexity.ai/hub/blog/search-api-better-extraction-dynamic-benchmarks): span-level relevance classification ("vital"/"irrelevant"/"duplicate") sebagai fondasi sitasi granular.
- [What is Pro Search](https://www.perplexity.ai/help-center/en/articles/10352903-what-is-pro-search): user bisa melihat bagaimana AI memecah pertanyaan + pendekatan riset, bukan hanya ringkasan buta sumber.
- Pola umum: tiap klaim/kalimat jawaban diberi marker sitasi `[1][2]` ke sumber tertentu — RAG standar dengan inline citation per-span, bukan bibliografi di akhir.

## 4. Real-Time Web Access: Speed vs Depth

- **Query decomposition resmi** ([Search API Best Practices](https://docs.perplexity.ai/docs/search/best-practices)): "Break your main topic into related sub-queries... multi-query search feature to run multiple related queries in a single request."
- Trade-off speed/depth via tiering produk: pencarian biasa (1 pass) vs Pro Search/Deep Research (multi-pass, 2-4 menit).
- [Rethinking Search as Code Generation](https://www.perplexity.ai/el/hub/blog/rethinking-search-as-code-generation): model menulis kode yang menyusun strategi search sendiri secara dinamis per-query ("Search as Code").

## 5. Kontroversi Teknis — Pelajaran yang HARUS DIHINDARI

Kronologi terverifikasi:
- **Juni 2024** — [Wired investigation](https://www.wired.com/story/perplexity-is-a-bullshit-machine/): Perplexity diam-diam scraping situs yang memblokirnya via robots.txt, bertentangan dengan kebijakan tertulis mereka sendiri.
- **Agustus 2025** — [Cloudflare blog resmi](https://blog.cloudflare.com/perplexity-is-using-stealth-undeclared-crawlers-to-evade-website-no-crawl-directives/): Perplexity berulang kali mengubah user-agent dan ASN untuk menyembunyikan aktivitas crawling, mengabaikan robots.txt. Dikonfirmasi [TechCrunch](https://techcrunch.com/2025/08/04/perplexity-accused-of-scraping-websites-that-explicitly-blocked-ai-scraping/) dan [Ars Technica](https://arstechnica.com/information-technology/2025/08/ai-site-perplexity-uses-stealth-tactics-to-flout-no-crawl-edicts-cloudflare-says/).
- Berujung gugatan hukum ([ringkasan](https://aiwiki.ai/wiki/perplexity_copyright_lawsuits)): scraping melewati paywall, mengabaikan robots.txt, mereproduksi konten berhak cipta dengan atribusi salah.

**Yang HARUS DIHINDARI**: jangan menyamarkan user-agent/ASN untuk melewati blokir (risiko CFAA/trespass-to-chattel); jangan mengabaikan robots.txt setelah mengklaim menghormatinya secara publik (representasi palsu); jangan mereproduksi konten berhak cipta tanpa lisensi dengan atribusi keliru.

## 6. Perplexity Deep Research — Arsitektur

- [Introducing Perplexity Deep Research](https://perplexity.ai/hub/blog/introducing-perplexity-deep-research): menjalankan belasan/puluhan pencarian, membaca ratusan sumber, dalam 2-4 menit.
- Pola kerja: agentic iterative search — plan→search→read→re-plan (mirip ReAct), bukan satu pencarian sekali tembak.
- Versi lanjutan dibangun di atas Agent Search SDK + "Search as Code": model menulis kode strategi search sendiri, ribuan langkah retrieval paralel.
- Backend per Feb 2026: model reasoning top (Opus 4.5 untuk tier Max/Pro) + search engine proprietary + sandbox infrastructure.

## Keputusan Achmad (2026-10-05)

Riset ini **murni bahan referensi**, tidak diimplementasikan sebagai fitur baru. Dua pola kecil yang DIADOPSI (ke SOP riset squad, bukan ke kode):
1. **Query decomposition** — pecah pertanyaan riset kompleks jadi sub-query spesifik sebelum memanggil `web_search_tool` berulang, bukan satu query generik.
2. **Domain-filtering akademik** — tambahkan filter domain (`arxiv.org`, `*.edu`) ke call `web_search_tool`/provider yang sudah ada saat riset bersifat akademik.

**Yang TIDAK diadopsi**: crawler proprietary (galleon-fleet sudah benar dengan `domain_guard`+DNS pinning di `web_fetch.rs`, arah berlawanan dari stealth-crawling Perplexity — tidak perlu diubah).

## Sumber

Lihat URL inline di tiap bagian di atas. Repo galleon-fleet sudah dicek: `crates/galleon-tools/src/web_fetch.rs` (domain_guard, DNS pinning, Firecrawl fallback) dan `web_search_tool.rs` (multi-provider: AnySearch, Serply) — murni API-based, nol crawler sendiri.
