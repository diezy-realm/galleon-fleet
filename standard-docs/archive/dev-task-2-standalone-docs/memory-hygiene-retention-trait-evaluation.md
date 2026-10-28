# Evaluasi Trait: Memory Hygiene & Retensi Otomatis Backend-Agnostic (BL-31)

**Ticket:** BL-31 (Hygiene/Retensi Backend-Agnostic Trait Evaluation)  
**Status:** Evaluated / Recommendation Formulated  
**Date:** 2026-10-08  
**Prasyarat & Relasi:** BL-30 (`erase_identity`), Ponytail Principle (No speculative complexity)  

---

## 1. Analisis Kondisi Saat Ini (As-Is)

Berdasarkan audit langsung pada `crates/galleon-memory/src/hygiene.rs`:
1. **Coupling Ketat ke SQLite:**
   - File `hygiene.rs` mengimpor `use rusqlite::{Connection, params};` secara langsung.
   - Fungsi `run_if_due(config: &MemoryConfig, workspace_dir: &Path)` mengasumsikan database memori selalu berupa file SQLite lokal di disk workspace (`memories.db`).
   - `HygieneReport` dan `HygieneState` menghitung metrik berbasis file dan row SQL: `archived_memory_files`, `archived_session_files`, `pruned_conversation_rows`, `pruned_daily_rows`, `pruned_core_rows`.
2. **Ketiadaan Dukungan untuk Backend Lain:**
   - 11 backend memory lainnya (`postgres.rs`, `qdrant.rs`, `markdown.rs`, `agent_scoped.rs`, `lucid.rs`, `consolidation.rs`, dll.) tidak tersentuh oleh siklus hygiene ini.
   - Vector store (Qdrant) dapat terus mengumpulkan embedding lama tanpa mekanisme TTL pruning.
   - File markdown log terus bertambah tanpa rotasi/kompresi berkala.

---

## 2. Usulan Desain Trait Backend-Agnostic

Untuk membuat hygiene dan retensi bersifat backend-agnostic tanpa memecah arsitektur:

### 2.1 Definisi Kontrak Policy & Report
```rust
/// Kebijakan retensi berbasis durasi dan kategori memori.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub conversation_retention_days: Option<u32>,
    pub daily_retention_days: Option<u32>,
    pub core_retention_days: Option<u32>,
    pub dry_run: bool,
}

/// Laporan hasil pembersihan retensi per backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackendHygieneReport {
    pub backend_name: String,
    pub pruned_entries: u64,
    pub bytes_reclaimed: u64,
}
```

### 2.2 Penambahan Trait Method pada `Memory`
```rust
#[async_trait]
pub trait Memory: Send + Sync + Attributable {
    // ... method existing ...

    /// Terapkan kebijakan retensi/hygiene pada backend ini (BL-31).
    /// Default: No-op yang mengembalikan 0 entries terpangkas.
    async fn apply_retention(&self, _policy: &RetentionPolicy) -> anyhow::Result<BackendHygieneReport> {
        Ok(BackendHygieneReport {
            backend_name: self.name().to_string(),
            pruned_entries: 0,
            bytes_reclaimed: 0,
        })
    }
}
```

---

## 3. Evaluasi Cost-Benefit & Prinsip Ponytail (YAGNI)

| Aspek | Implementasi Sekarang ke 12 Backend | Evaluasi & Rekomendasi Bertahap |
| :--- | :--- | :--- |
| **Effort** | Sangat Tinggi (perlu menulis query delete di PostgreSQL, payload filter di Qdrant API, fs walk di Markdown) | **Rendah-Sedang**: Cukup sediakan default implementation di trait, SQLite tetap menjadi backend referensi |
| **Kebutuhan Nyata** | Sebagian besar instalasi memakai SQLite sebagai primary chat & context storage | SQLite sudah bekerja dengan stabil; Qdrant/PostgreSQL digunakan untuk skenario enterprise spesifik |
| **Risiko Regresi** | Tinggi jika query filter payload Qdrant memangkas vector aktif tanpa validasi menyeluruh | Nol risiko regresi pada storage engine yang sudah berjalan |

---

## 4. Rekomendasi Keputusan Arsitektural

1. **Tahap 1 (Sprint 6 - Selesai):**
   - Tetapkan evaluasi trait tertulis ini sebagai dokumen rujukan desain.
   - Sinkronkan paradigma operasi lintas-backend dengan `BL-30` (`erase_identity()`) yang telah menambahkan `forget_all_for_agent()` dengan default implementation aman.
2. **Tahap 2 (Sprint Berikutnya / Saat Qdrant Production Scale):**
   - Implementasikan `apply_retention` pada `SqliteMemory` dengan mereuse logika query di `hygiene.rs`.
   - Tambahkan implementasi `apply_retention` untuk `QdrantMemory` menggunakan payload filter timestamp sebelum release multi-tenant enterprise.
