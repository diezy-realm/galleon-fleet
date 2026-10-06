# Cache Strategy — Steering Stabil = Cache-Friendly

**Status:** Normative Architecture Guideline  
**Domain:** LLM Token Efficiency & Context Management  
**Reference:** R-88, R-89, R-90, R-91  

---

## 1. Prinsip Inti: Prefix Caching pada LLM

Modern LLM providers (Anthropic Claude Prompt Caching, OpenAI Structured Cache, Google Gemini Context Caching) mengoptimalkan biaya dan latensi menggunakan **prefix caching**:
- Token yang identik dari awal urutan konteks (prompt prefix) hingga breakpoint tertentu disimpan dalam cache RAM/GPU provider.
- Cache hit memberikan reduksi biaya input token hingga 80-90% dan memangkas Time-to-First-Token (TTFT) secara drastis.
- Perubahan sekecil 1 karakter atau pergeseran indeks pada prefix membatalkan seluruh blok cache di belakangnya (**cache invalidation cascade**).

```
[System Steering (Stabil)] -> [Tool Definitions (Statis)] -> [Historical Turns] -> [Latest User Prompt]
|<----------------- CACHED PREFIX (HIT) ------------------>|  (CACHE MISS/UNCACHED TAIL)
```

**Aturan Emas:** *Steering stabil = Cache-friendly. Komponen statis di depan, data dinamis di belakang.*

---

## 2. Praktik Terbaik Struktur Konteks

### A. Ordering Hirarki Konteks
1. **Paling Stabil (Awal Prompt):**
   - System Identity & Behavioral Rules (AGENTS.md / Steering)
   - Safety Boundaries & Boundary Policy
   - Static Tool Definitions & JSON Schema
2. **Semi-Stabil (Tengah Konteks):**
   - Active SOP graph & skill definitions
   - Ringkasan memori sesi jangka panjang
3. **Paling Dinamis (Akhir Konteks):**
   - Riwayat percakapan terkini (turns)
   - Breadcrumbs pemotongan riwayat
   - User turn input terkini

### B. Anti-Patterns yang Merusak Cache
| Anti-Pattern | Dampak | Solusi Kanonikal |
| :--- | :--- | :--- |
| **Dynamic Timestamps in System Prompt** | Setiap request mengubah string system prompt, menghasilkan 100% cache miss. | Masukkan timestamp saat ini pada turn input user terkini atau per-message metadata, bukan di system steering prefix. |
| **Shuffling/Reordering Tools** | Urutan definisi tools yang berubah-ubah membatalkan token hash. | Pertahankan urutan kamus tool secara deterministik (alphabetical sorting). |
| **Aggressive/Premature Turn Trimming** | Pemotongan riwayat yang terlalu sering mengubah panjang konteks sebelum batas breakpoint cache. | Gunakan whole-turn history trimming dan sesuaikan budget dengan rolling cache performance (`dynamic_trim_budget`). |
| **Per-request Random Salt/Session IDs in Prefix** | Membatalkan kesamaan prefix antar-turn dalam sesi yang sama. | Letakkan session metadata di header wire transport atau di pesan user pertama. |

---

## 3. Observabilitas dan Metrik Cache (R-90)

Sistem memantau efisiensi cache melalui metrik Prometheus:
- `clawcrew_cache_hits_total{cache_type}`: Total hit cache konteks.
- `clawcrew_cache_misses_total{cache_type}`: Total miss cache konteks.
- `clawcrew_cache_tokens_saved_total{cache_type}`: Estimasi token input yang dihemat.
- `clawcrew_cache_hit_rate{cache_type}`: Rasio hit rate real-time $\frac{\text{hits}}{\text{hits} + \text{misses}}$.

Operator dapat memantau endpoint `/metrics` untuk memastikan rasio hit rate berada di atas target optimal (> 75% untuk multi-turn conversation berulang).
