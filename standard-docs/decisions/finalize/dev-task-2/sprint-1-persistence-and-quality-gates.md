---
sprint: 1
title: Persistence & Quality Gates — Postgres, RAG/Vector, Rust Burn-down, Token/Knowledge Efficiency
status: planned
depends_on: BG-B1/BG-B2 (sudah selesai, lihat dev-tasks/01)
---

# Sprint 1 — Persistence & Quality Gates

## A. Persistensi Go Lanjutan

- [ ] **BG-B3 · G-12 · PR-42 (P2, L)** Store PostgreSQL + test container. Dep: BG-B1 (selesai).
  #### Detail Eksekusi -- BG-B3
  1. **File & simbol spesifik:**
     - `/engine/pkg/store/postgres/store.go` (baru) - implementasi PostgreSQL `Store` interface
     - `/engine/pkg/store/postgres/test_container.go` (baru) - PostgreSQL test container setup
     - `/engine/pkg/store/vector/types.go` (baru) - common vector types
     - `engine/pkg/store/vector/` - directory baru untuk RAG/vector layer
  2. **Spesifikasi API:**
     ```go
     // VectorStore interface (common for all vector stores)
     type VectorStore interface {
         Store(ctx context.Context, doc *Document) error
         Search(ctx context.Context, queryEmbedding []float32, topK int) ([]*SearchResult, error)
         SearchWithScope(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*SearchResult, error)
         Delete(ctx context.Context, id string) error
     }
     ```
     - PostgreSQL connection string dari env: `POSTGRES_URL`
     - Test container: `testcontainers.PostgresContainer` dengan `testcontainers.WithWaitStrategy`
  3. **Contoh pengkodean (pola dari memory/store.go):**
     ```go
     func NewPostgresVectorStore(ctx context.Context, dsn string) (*PostgresVectorStore, error) {
         // Connect dengan retry
         // Create table jika belum ada
         // Migrate schema jika perlu
     }
     
     func (s *PostgresVectorStore) Store(ctx context.Context, doc *Document) error {
         // INSERT INTO vector_store (id, content, embedding, metadata, scope)
         // Gunakan pgvector extension untuk vector similarity
         // return error jika duplicate ID
     }
     ```
  4. **Konvensi penamaan:**
     - Package: `postgres` di `/engine/pkg/store/postgres/`
     - Struct: `PostgresVectorStore`
     - Constructor: `NewPostgresVectorStore(ctx, dsn)`
  5. **Langkah pengerjaan berurutan:**
     1. Buat `/engine/pkg/store/postgres/` directory
     2. Buat `types.go` - define `PostgresVectorStore` struct dan interface
     3. Buat `store.go` - implementasi `Store`, `Search`, `Delete` dengan pgvector
     4. Buat `test_container.go` - setup test container dengan testcontainers-go
     5. Update `engine/pkg/store/vector/factory.go` - route ke PostgreSQL when config
     6. Test: `go test -v ./engine/pkg/store/postgres/...`
  6. **Acceptance criteria testable:**
     - [ ] PostgreSQL container berjalan via testcontainers
     - [ ] `Store()` menyimpan document dengan embedding
     - [ ] `Search()` return vector similarity results
     - [ ] `Delete()` hapus document by ID
     - [ ] Test suite 100% pass
  7. **Dependency antar-tiket:** BG-B1 (selesai) - baseline vector store interface
  8. **Edge case:** Handle migration dari in-memory ke PostgreSQL, backfill existing vectors
  9. **JANGAN DIUBAH:** Behavior yang sudah ada di `VectorStore` interface - hanya tambah backend baru
  10. **ASUMSI:** PostgreSQL with pgvector extension tersedia; testcontainers-go tersedia

- [ ] **BG-B4 · G-13 · PR-43 (P1, M)** Memori vektor/RAG per scope Fleet/Ship/Squad/Crew/Run. Dep: BG-B2 (selesai). **Acceptance criteria tambahan (eks-BL-51, DITUNDA → DIANGKAT sprint ini karena dependency terpenuhi):** desain koordinasi lintas Go-Rust untuk RAG/vector mengikuti pola yang sama dengan `knowledge_graph.rs` Rust (lihat §C) — JANGAN bangun 2 sistem vector paralel tanpa jembatan.
  #### Detail Eksekusi -- BG-B4
  1. **File & simbol spesifik:**
     - `/engine/src/memory/store.go` - sudah ada `inMemoryVectorStore`, `Document`, `SearchResult`, `VectorStore` interface
     - `/engine/pkg/store/vector/types.go` (baru) - scope-scoped storage types
     - `/engine/pkg/store/vector/scoped.go` (baru) - scoped vector store implementation
  2. **Spesifikasi API:**
     - Tambah `Scope` field ke `Document` struct (sudah ada di store.go:113)
     - Tambah `ScopedVectorStore` interface yang extends `VectorStore` dengan scope filtering
     ```go
     type ScopedVectorStore interface {
         VectorStore
         StoreScoped(ctx context.Context, doc *Document, scope map[string]string) error
         SearchScoped(ctx context.Context, queryEmbedding []float32, topK int, scope map[string]string) ([]*SearchResult, error)
     }
     ```
  3. **Contoh pengkodean (pola dari knowledge_graph.rs Rust):**
     - Gunakan pattern yang sama dengan `clawcrew-memory/src/knowledge_graph.rs`
     - Scope: Fleet/Ship/Squad/Crew/Run sebagai columns di database
     - Query decomposition: filter by scope fields
  4. **Konvensi penamaan:**
     - Scope keys: `fleet`, `ship`, `squad`, `crew`, `run_id`
     - Interface method: `SearchScoped`, `StoreScoped`, `DeleteScoped`
  5. **Langkah pengerjaan berurutan:**
     1. Review `Document` struct - pastikan `Scope` field ada (sudah ada di store.go)
     2. Buat `scoped.go` dengan `ScopedVectorStore` interface
     3. Implementasi PostgreSQL scoped store dengan WHERE clause pada scope fields
     4. Buat unit test untuk each scope level
     5. Integration test: verify cross-scope search isolation
  6. **Acceptance criteria testable:**
     - [ ] Search Fleet scope tidak return Ship-level results
     - [ ] Search Crew scope tidak return Squad-level results
     - [ ] Search dengan scope kosong = full scan (backward compatibility)
  7. **Dependency antar-tiket:** BG-B2 (selesai) - baseline vector store; R-01 (Rust knowledge_graph.rs pattern)
  8. **Edge case:** Scope override, nested scope inheritance, empty scope handling
  9. **JANGAN DIUBAH:** Existing `inMemoryVectorStore` behavior - tambah scoped variant baru
  10. **ASUMSI:** Go engine maintains single source of truth for vector storage policy; Rust `knowledge_graph.rs` pattern di-reuse untuk koherensi arsitektur

- [ ] **BG-B5 · G-14 · PR-43 (P2, M)** Retensi, ekspor, hapus (privasi) per scope. Dep: BG-B2 (selesai).
  #### Detail Eksekusi -- BG-B5
  1. **File & simbol spesifik:**
     - `/engine/pkg/store/retention.go` (baru) - retention policy implementation
     - `/engine/pkg/store/export.go` (baru) - export functionality
     - `/engine/pkg/store/privacy.go` (baru) - GDPR-style delete
     - `/engine/pkg/store/vector/types.go` - tambah `RetentionConfig` struct
  2. **Spesifikasi API:**
     ```go
     type RetentionConfig struct {
         FleetRetentionDays  int `json:"fleet_retention_days"`
         ShipRetentionDays   int `json:"ship_retention_days"`
         SquadRetentionDays  int `json:"squad_retention_days"`
         CrewRetentionDays   int `json:"crew_retention_days"`
         RunRetentionDays    int `json:"run_retention_days"`
     }
     
     type ExportRequest struct {
         Scope    map[string]string `json:"scope"`
         Format   string            `json:"format"` // "json", "csv", "parquet"
         FromTime time.Time         `json:"from_time"`
         ToTime   time.Time         `json:"to_time"`
     }
     ```
  3. **Contoh pengkodean (pola dari memory/proposal.go):**
     ```go
     func (s *PostgresVectorStore) ExportByScope(ctx context.Context, req ExportRequest) ([]byte, error) {
         // Query dengan WHERE scope = req.Scope
         // Filter by time range
         // Marshal to requested format
     }
     
     func (s *PostgresVectorStore) DeleteExpired(ctx context.Context, now time.Time) error {
         // DELETE FROM vector_store WHERE created_at < now - retention_days
         // Per scope dengan retention config
     }
     ```
  4. **Konvensi penamaan:**
     - Methods: `ExportByScope`, `DeleteExpired`, `PruneByScope`
     - Config: `RetentionConfig`, `ExportRequest`
  5. **Langkah pengerjaan berurutan:**
     1. Buat `retention.go` - define `RetentionConfig` dan `PruneByScope`
     2. Buat `export.go` - define `ExportRequest` dan `ExportByScope`
     3. Buat `privacy.go` - define `DeleteByScope` untuk GDPR
     4. Integration test: verify export format conversion
     5. Test retention policy: verify expired docs are deleted
  6. **Acceptance criteria testable:**
     - [ ] Export JSON/CSS/Parquet berfungsi untuk satu scope
     - [ ] Delete expired by scope work correctly
     - [ ] GDPR-style delete (pseudonymization) tersedia
  7. **Dependency antar-tiket:** BG-B2 (selesai), BG-B3 (PostgreSQL ready)
  8. **Edge case:** Export besar (>1GB), concurrent exports, partial failures
  9. **JANGAN DIUBAH:** Existing data structures - tambah method baru
  10. **ASUMSI:** PostgreSQL dengan proper indexing untuk time-based queries; export format menggunakan external library (e.g., `github.com/olekukonko/tablewriter`)

## B. Burn-down Kualitas Rust

- [ ] **BR-A5 · R-02 · PR-30 (P1, M, berseri per crate)** Burn-down `unwrap/expect` produksi (±400 hit; `gateway`, `channels`, `runtime`). Dep: BR-A1 (selesai).
  #### Detail Eksekusi -- BR-A5
  1. **File & simbol spesifik:**
     - `/crates/clawcrew-gateway/src/api.rs` - 374 hit (produksi: 200+)
     - `/crates/clawcrew-gateway/src/api_config.rs` - 164 hit
     - `/crates/clawcrew-gateway/src/ws.rs` - 122 hit
     - `/crates/clawcrew-gateway/src/api_sections.rs` - 104 hit
     - `/crates/clawcrew-gateway/src/a2a.rs` - 36 hit
     - `/crates/clawcrew-channels/src/` - verify count per-crate
     - `/crates/clawcrew-runtime/src/` - verify count per-crate
  2. **Spesifikasi API:**
     - Ganti `unwrap()` → `map_err(|e| Status::internal(e.to_string()))?` untuk gRPC
     - Ganti `expect()` → `return Err(anyhow!("..."))` atau logging + return
     - Gunakan `thiserror` untuk custom error types
     - Tambah `clippy::expect-usage` lint in CI
  3. **Contoh pengkodean (pola dari grpc_system_gateway.rs):**
     ```rust
     // BAD:
     let value = some_option.unwrap();
     
     // GOOD:
     let value = some_option.ok_or_else(|| Status::invalid_argument("value missing"))?;
     ```
     ```rust
     // BAD:
     let config = load_config().expect("config must exist");
     
     // GOOD:
     let config = load_config().map_err(|e| {
         log::error!("config load failed: {}", e);
         Status::internal("config load failed")
     })?;
     ```
  4. **Konvensi penamaan:**
     - Error type: `clawcrew_{crate}::Error`
     - Method: `map_err(|e| Status::internal(e.to_string()))?`
     - Lint: `clippy::unwrap-usage`, `clippy::expect-usage`
  5. **Langkah pengerjaan berurutan:**
     1. `cargo clippy -p clawcrew-gateway -- -D clippy::unwrap-usage -D clippy::expect-usage`
     2. Fix all warnings in `gateway/src/api.rs`, `api_config.rs`, `ws.rs`, `api_sections.rs`, `a2a.rs`
     3. Repeat untuk `clawcrew-channels`, `clawcrew-runtime`
     4. Tambah CI check: `clippy::unwrap-usage`, `clippy::expect-usage`
     5. Verify: `cargo build --release` tanpa warnings
  6. **Acceptance criteria testable:**
     - [ ] CI lint check: `clippy::unwrap-usage` = error
     - [ ] CI lint check: `clippy::expect-usage` = error
     - [ ] `cargo clippy` di semua crate = 0 warnings
     - [ ] Burn-down count: gateway <50, channels <50, runtime <200 (sisa dari baseline 400)
  7. **Dependency antar-tiket:** BR-A1 (selesai) - baseline error handling pattern
  8. **Edge case:** Error propagation chain, nested unwrap, expect in async fn
  9. **JANGAN DIUBAH:** Logic yang ada - hanya ganti error handling style
  10. **ASUMSI:** `thiserror` sudah ter-install di semua crate; no `#[allow(clippy::unwrap-usage)]` di production code

- [ ] **BR-A6 · R-05 · PR-20 (P2, S)** Selaraskan `rust-conventions.md` (MSRV, jumlah crate).
  #### Detail Eksekusi -- BR-A6
  1. **File & simbol spesifik:**
     - `/standard-docs/tech-standart/tech-stack/rust-conventions.md` - source of truth
     - `/crates/*/Cargo.toml` - verify MSRV field
     - `/rust-toolchain.toml` - verify MSRV definition
     - `/crates/*/Cargo.toml` - verify dependency count
  2. **Spesifikasi API:**
     - MSRV (Minimum Supported Rust Version): current = 1.78.0 (verified di rust-toolchain.toml)
     - Max crate count per workspace: 50 (verified di rust-decomposition-plan.md)
     - Dependency tree depth: max 3 levels
  3. **Contoh pengkodean (pola Cargo.toml):**
     ```toml
     [package]
     name = "clawcrew-gateway"
     version = "0.1.0"
     edition = "2021"
     rust-version = "1.78.0"  # MSRV explicit
     
     [dependencies]
     # Max 3 levels: direct → transitive → transitive-transitive
     tokio = { version = "1", features = ["full"] }
     ```
  4. **Konvensi penamaan:**
     - MSRV field: `rust-version = "x.y.z"`
     - Edition: `2021`
     - Workspace: single `Cargo.toml` di root dengan `members = [...]`
  5. **Langkah pengerjaan berurutan:**
     1. Baca `rust-conventions.md` current version
     2. Verify `rust-toolchain.toml` MSRV match
     3. Check setiap crate Cargo.toml punya `rust-version` field
     4. Count dependencies: `cargo tree -i | wc -l`
     5. Update convention doc jika ada discrepancy
     6. Add CI check: `cargo metadata --format-version=1 | jq '.packages[].rust_version'`
  6. **Acceptance criteria testable:**
     - [ ] `rust-toolchain.toml` MSRV = `rust-version` di semua Cargo.toml
     - [ ] Dependency count <50 per crate
     - [ ] Dependency tree depth ≤3
     - [ ] CI fail jika MSRV mismatch
  7. **Dependency antar-tiket:** BR-A1 (selesai) - baseline convention
  8. **Edge case:** Dev dependency, build dependency, optional feature deps
  9. **JANGAN DIUBAH:** Kebijakan MSRV yang sudah ada - hanya selaraskan dokumentasi
  10. **ASUMSI:** No breaking changes di dependencies yang naikkan MSRV; no workspace members >50

## C. Token/Cache Efficiency (diangkat ke roadmap sebagai R-88..91)

- [ ] **R-88 (eks-BL-45a, P1, MEDIUM, PRIORITAS — murah)** Ukur korelasi trim-history vs cache-miss sebelum mitigasi apa pun. Logging-only, menjawab apakah R-89 layak dengan data nyata.
  #### Detail Eksekusi -- R-88
  1. **File & simbol spesifik:**
     - `/crates/clawcrew-runtime/src/agent/history_trim.rs` - history trimming logic
     - `/crates/clawcrew-runtime/src/agent/cost.rs` - cost tracking
     - `/crates/clawcrew-runtime/src/agent/loop_/mod.rs` - turn loop entry point
  2. **Spesifikasi API:**
     - Tambah logging di history trim: `clawcrew_log::record!(INFO, ..., "trim_history")`
     - Tambah logging di cache hit/miss: `clawcrew_log::record!(INFO, ..., "cache_miss")`
     - Correlate: trim history timestamp vs cache miss timestamp
     - Output: CSV/JSON log dengan fields: `timestamp`, `trim_length`, `cache_hit`
  3. **Contoh pengkodean (pola dari history_trim.rs):**
     ```rust
     fn trim_history(&mut self, max_tokens: usize) {
         let trim_before = self.history.len();
         // ... trim logic ...
         clawcrew_log::record!(
             INFO,
             Event::new(module_path!(), Action::Note)
                 .with_attrs(json!({"trimmed": trim_before - self.history.len()})),
             "trim_history completed"
         );
     }
     ```
     ```rust
     fn recall_with_cache(&mut self, key: &str) -> Option<MemoryEntry> {
         if let Some cached) = self.cache.get(key) {
             clawcrew_log::record!(INFO, ..., "cache_hit");
             return Some(cached);
         }
         let result = self.backend.recall(key, ...);
         clawcrew_log::record!(INFO, ..., "cache_miss");
         result
     }
     ```
  4. **Konvensi penamaan:**
     - Event: `trim_history`, `cache_hit`, `cache_miss`
     - Fields: `trim_length`, `cache_hit`, `timestamp`
  5. **Langkah pengerjaan berurutan:**
     1. Update `history_trim.rs` - tambah logging trim length
     2. Update `cost.rs` - tambah cache hit/miss tracking
     3. Run turn loop - collect logs for 100+ turns
     4. Analyze logs - correlation analysis (trim_history vs cache_miss)
     5. Generate CSV/JSON summary untuk decision R-89
  6. **Acceptance criteria testable:**
     - [ ] Log `trim_history` dengan field `trimmed` tercatat
     - [ ] Log `cache_hit` dan `cache_miss` tercatat
     - [ ] CSV/JSON output bisa dibaca untuk correlation analysis
  7. **Dependency antar-tiket:** None (independen)
  8. **Edge case:** Empty history, cache disabled, no cacheable entries
  9. **JANGAN DIUBAH:** trim/history/cache logic - hanya tambah logging
  10. **ASUMSI:** `clawcrew_log` macro tersedia; log output bisa di-collect dan analyze

- [ ] **R-89 (eks-BL-45b, KONDISIONAL)** Align titik potong trim dengan rolling cache-breakpoint. **Hanya dikerjakan jika R-88 konfirmasi dampak signifikan** — jangan mulai sebelum data R-88 ada.
  #### Detail Eksekusi -- R-89
  1. **File & simbol spesifik:**
     - `/crates/clawcrew-runtime/src/agent/turn/mod.rs` - turn entry point
     - `/crates/clawcrew-runtime/src/agent/loop_/mod.rs` - loop control
  2. **Spesifikasi API:**
     - Dynamic trim threshold berdasarkan cache hit rate
     - Formula: `trim_threshold = base_threshold * (1 - cache_hit_rate)`
     - Rolling window: 100 most recent turns
  3. **Contoh pengkodean (pola dari cost.rs):**
     ```rust
     fn dynamic_trim_threshold(&self) -> usize {
         let cache_hit_rate = self.stats.cache_hits as f64 / (self.stats.cache_hits + self.stats.cache_misses) as f64;
         let multiplier = 1.0 - cache_hit_rate.min(0.5); // max 50% reduction
         (self.base_threshold as f64 * multiplier) as usize
     }
     ```
  4. **Konvensi penamaan:**
     - Method: `dynamic_trim_threshold`
     - Stats: `cache_hits`, `cache_misses`, `trim_count`
  5. **Langkah pengerjaan berurutan:**
     1. Wait for R-88 data
     2. If cache_miss > 30%, proceed with R-89
     3. Implement dynamic trim threshold logic
     4. Test dengan varied cache hit rates
  6. **Acceptance criteria testable:**
     - [ ] Dynamic threshold berubah sesuai cache hit rate
     - [ ] Trim threshold <= base threshold
     - [ ] No performance regression
  7. **Dependency antar-tiket:** R-88 (data must show significant impact)
  8. **Edge case:** Zero cache hits, zero cache misses (division by zero)
  9. **JANGAN DIUBAH:** Existing trim/cache logic - hanya tambah dynamic threshold
  10. **ASUMSI:** R-88 data menunjukkan korelasi signifikan; cache statistics cukup untuk rolling window

- [ ] **R-90 (eks-BL-46, LOW-MEDIUM)** Surface cache hit-rate ke dashboard/CLI operator. Setelah R-88 minimal selesai.
  #### Detail Eksekusi -- R-90
  1. **File & simbol spesifik:**
     - `/crates/clawcrew-runtime/src/observability/prometheus.rs` - metrics endpoint
     - `/crates/clawcrew-gateway/src/api.rs` - API handler untuk metrics
  2. **Spesifikasi API:**
     - Prometheus metric: `clawcrew_cache_hit_rate`
     - Label: `agent_id`, `scope`
     - Value: ratio cache_hits / (cache_hits + cache_misses)
  3. **Contoh pengkodean (pola dari prometheus.rs):**
     ```rust
     registry.register(Box::new(Gauge::with_opts(
         GaugeOpts {
             name: "clawcrew_cache_hit_rate".to_string(),
             help: "Ratio of cache hits to total requests".to_string(),
             const_label_values: &[],
         }
     )?))?;
     ```
  4. **Konvensi penamaan:**
     - Metric: `clawcrew_cache_hit_rate`
     - Labels: `agent_id`, `scope`
  5. **Langkah pengerjaan berurutan:**
     1. Add cache hit/miss counter metrics
     2. Calculate hit rate per agent
     3. Expose via Prometheus endpoint
     4. Add CLI command: `clawcrew cache stats`
  6. **Acceptance criteria testable:**
     - [ ] `curl http://localhost:9090/metrics` return `clawcrew_cache_hit_rate`
     - [ ] `clawcrew cache stats` CLI command works
  7. **Dependency antar-tiket:** R-88 minimal selesai (stats collection)
  8. **Edge case:** Zero requests, empty cache
  9. **JANGAN DIUBAH:** Existing metrics structure - hanya tambah cache metrics
  10. **ASUMSI:** Prometheus client library tersedia; no breaking changes di metrics API

- [ ] **R-91 (eks-BL-47, LOW, independen, biaya nol)** Dokumentasikan prinsip "steering stabil = cache-friendly".
  #### Detail Eksekusi -- R-91
  1. **File & simbol spesifik:**
     - `/standard-docs/tech-standart/cache-strategy.md` (baru)
  2. **Spesifikasi API:**
     - Doc prinsip: steering configuration stabil → cache invalidation minimal
     - Pattern: `agent_id` atau `workspace_path` yang konstan
     - Anti-pattern: frequent config change → cache flush
  3. **Contoh pengkodean:**
     - Tidak ada kode baru, dokumentasi murni
     - Contoh konfigurasi stabil vs tidak stabil
  4. **Konvensi penamaan:**
     - Title: "Cache Strategy"
     - Sections: Principles, Anti-patterns, Examples
  5. **Langkah pengerjaan berurutan:**
     1. Buat `/standard-docs/tech-standart/cache-strategy.md`
     2. Document principle: steering stabil = cache-friendly
     3. Add anti-pattern section
     4. Add example configurations
  6. **Acceptance criteria testable:**
     - [ ] Dokumentasi tersedia di `standard-docs/tech-standart/cache-strategy.md`
     - [ ] Document menjelaskan prinsip steering stabil
  7. **Dependency antar-tiket:** None (independen)
  8. **Edge case:** N/A (dokumentasi)
  9. **JANGAN DIUBAH:** N/A
  10. **ASUMSI:** Template dokumentasi sudah ada di `standard-docs/tech-standart/`

## D. Knowledge-Base / Citation (diangkat ke roadmap sebagai R-92..95)

- [ ] **R-92 (eks-BL-48, MEDIUM)** Field citation + verifikasi pada node Lesson/Decision di `knowledge_graph.rs` — mitigasi bug "configuration-lies" (`query_by_similarity` nama vector tapi impl FTS5 keyword).
  #### Detail Eksekusi -- R-92
  1. **File & simbol spesifik:**
     - `/crates/clawcrew-memory/src/knowledge_graph.rs` - current implementation
     - `/crates/clawcrew-api/src/knowledge.rs` - API layer
     - `/crates/clawcrew-memory/src/traits.rs` - Memory trait definition
  2. **Spesifikasi API:**
     - Tambah field `citations: Vec<String>` ke `KnowledgeEntry`
     - Tambah `verify_citations()` method untuk validasi source
     - Fix `query_by_similarity` → actual vector search (bukan FTS5)
  3. **Contoh pengkodean (pola dari knowledge_graph.rs):**
     ```rust
     pub struct KnowledgeEntry {
         pub id: String,
         pub content: String,
         pub category: MemoryCategory,
         pub citations: Vec<String>,  // NEW
         pub created_at: SystemTime,
     }
     
     impl KnowledgeEntry {
         pub fn verify_citations(&self) -> Result<(), anyhow::Error> {
             for citation in &self.citations {
                 // Validate URL or document reference
                 if !self.source_exists(citation) {
                     return Err(anyhow!("citation not found: {}", citation));
                 }
             }
             Ok(())
         }
     }
     ```
  4. **Konvensi penamaan:**
     - Field: `citations: Vec<String>`
     - Method: `verify_citations()`
     - Struct: `KnowledgeEntry`
  5. **Langkah pengerjaan berurutan:**
     1. Update `knowledge_graph.rs` - add citations field
     2. Update `traits.rs` - extend `Memory` trait
     3. Implement `verify_citations()`
     4. Fix `query_by_similarity` → actual vector search dengan embedding
     5. Add test: verify citation validation
  6. **Acceptance criteria testable:**
     - [ ] `KnowledgeEntry` punya field `citations: Vec<String>`
     - [ ] `verify_citations()` return error jika source tidak ada
     - [ ] `query_by_similarity` menggunakan vector search (bukan FTS5)
  7. **Dependency antar-tiket:** None (independen, tapi koherensi dengan R-93)
  8. **Edge case:** Empty citations, malformed URL citation
  9. **JANGAN DIUBAH:** Existing `KnowledgeEntry` fields - hanya tambah citations
  10. **ASUMSI:** `clawcrew_memory` crate punya embedding support; no breaking changes di Memory trait

- [ ] **R-93 (eks-BL-49, MEDIUM-HIGH)** Aktifkan Knowledge Library untuk `standard-docs/` — setelah R-92 untuk koherensi (tidak hard-block).
  #### Detail Eksekusi -- R-93
  1. **File & simbol spesifik:**
     - `/crates/clawcrew-runtime/src/skills/mod.rs` - skills loading logic
     - `/crates/clawcrew-runtime/src/skills/cache.rs` - skills cache
     - `/standard-docs/tech-standart/skills-index.md` (baru) - skills manifest
  2. **Spesifikasi API:**
     - Load skills dari `standard-docs/tech-standart/skills/`
     - Index skills dengan metadata: `name`, `description`, `category`, `path`
     - Cache skills di `clawcrew_runtime/skills/cache/`
  3. **Contoh pengkodean (pola dari skills/mod.rs):**
     ```rust
     pub fn load_skills_from_directory(path: &Path) -> Result<Vec<SkillMetadata>, anyhow::Error> {
         let mut skills = Vec::new();
         for entry in fs::read_dir(path)? {
             let entry = entry?;
             if entry.file_type()?.is_dir() {
                 let skill_path = entry.path();
                 let manifest_path = skill_path.join("MANIFEST.toml");
                 if manifest_path.exists() {
                     let manifest = fs::read_to_string(&manifest_path)?;
                     let meta: SkillMetadata = toml::from_str(&manifest)?;
                     skills.push(meta);
                 }
             }
         }
         Ok(skills)
     }
     ```
  4. **Konvensi penamaan:**
     - Method: `load_skills_from_directory`, `index_skills`
     - Struct: `SkillMetadata`
     - Manifest: `MANIFEST.toml`
  5. **Langkah pengerjaan berurutan:**
     1. Create `/standard-docs/tech-standart/skills-index.md` manifest
     2. Update `skills/mod.rs` - add load/index functions
     3. Add skills caching di `skills/cache.rs`
     4. Test: verify skills load correctly
  6. **Acceptance criteria testable:**
     - [ ] Skills load dari `standard-docs/tech-standart/skills/`
     - [ ] Index tersimpan di `skills-index.md`
     - [ ] Cache berfungsi (no reload on repeat)
  7. **Dependency antar-tiket:** R-92 untuk koherensi (tidak hard-block)
  8. **Edge case:** Missing MANIFEST.toml, invalid TOML syntax
  9. **JANGAN DIUBAH:** Existing skills loading pattern - hanya tambah knowledge library support
  10. **ASUMSI:** Skills format sudah matang (lihat `open-skills-ecosystem.md`); no breaking changes di skill metadata

- [ ] **R-94 (eks-BL-50, LOW, murni dokumentasi)** Query decomposition + domain-filtering sebagai SOP riset (pola dari metodologi Perplexity, bukan kode baru).
  #### Detail Eksekusi -- R-94
  1. **File & simbol spesifik:**
     - `/standard-docs/tech-standart/research-sop.md` (baru)
  2. **Spesifikasi API:**
     - Dokumentasi SOP riset
     - Query decomposition: breaking down complex query
     - Domain filtering: scope to relevant domain
  3. **Contoh pengkodean:**
     - Tidak ada kode baru, dokumentasi murni
     - Template query decomposition:
       ```
       Original query: "How to debug memory leaks in Rust?"
       Decomposed:
       - What causes memory leaks in Rust?
       - How to detect memory leaks?
       - Tools for memory leak detection in Rust?
       ```
  4. **Konvensi penamaan:**
     - Title: "Research SOP"
     - Sections: Query Decomposition, Domain Filtering, Examples
  5. **Langkah pengerjaan berurutan:**
     1. Buat `/standard-docs/tech-standart/research-sop.md`
     2. Document query decomposition method
     3. Document domain filtering technique
     4. Add examples from galleon-fleet context
  6. **Acceptance criteria testable:**
     - [ ] Dokumentasi tersedia di `standard-docs/tech-standart/research-sop.md`
     - [ ] Document mencakup query decomposition
     - [ ] Document mencakup domain filtering
  7. **Dependency antar-tiket:** None (independen, dokumentasi murni)
  8. **Edge case:** N/A
  9. **JANGAN DIUBAH:** N/A
  10. **ASUMSI:** Template dokumentasi sudah ada di `standard-docs/tech-standart/`

## E. Skills/Steering/Lessons Persistence (dari `skills-steering-lessons-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)

> Tema sama dengan §A/§D di atas (jembatan Go-Rust untuk storage vektor/persisten) — BL-01 digabung ke sini karena pola arsitekturnya identik: Go pegang kebijakan, Rust pegang storage, gRPC `SystemGateway` sebagai jembatan.

- [x] **BL-01 (HIGH, bug fungsional nyata)** Jembatan gRPC: persist Lesson/Learning dari Go (`engine/src/memory/proposal.go`, "Learning by Consent" lifecycle lengkap) ke Rust vector store (`clawcrew-memory`) — saat ini `Approve()`/`Revert()` menulis ke `inMemoryVectorStore` Go yang hilang total saat restart proses. Scope: RPC baru `StoreLearnedRule`/`RetrieveLearnedRules` di proto `SystemGateway`, implementasi di `clawcrew-gateway/src/grpc_system_gateway.rs` (reuse `clawcrew-memory`, JANGAN tulis ulang vector-store). **Serialisasi: proto dulu, baru implementasi Go+Rust paralel.**
  *Status: SELESAI (100% test passing)*
  *Bukti konkret:*
  - `proto/agent_service.proto`: RPC `StoreLearnedRule` & `RetrieveLearnedRules` beserta message types `StoreLearnedRuleRequest/Response`, `RetrieveLearnedRulesRequest/Response`, `LearnedRule`.
  - `engine/pkg/pb`: Go protobuf code regenerated (`agent_service.pb.go`, `agent_service_grpc.pb.go`).
  - `crates/clawcrew-gateway/src/grpc_system_gateway.rs`: Implementasi `store_learned_rule` & `retrieve_learned_rules` memanfaatkan `clawcrew-memory` (SqliteMemory/Memory trait). Test `store_and_retrieve_learned_rules_persists_across_queries` lulus via `cargo nextest run -p clawcrew-gateway --lib grpc_system_gateway` (15/15 passed).
  - `engine/pkg/client/system_gateway.go`: Implementasi client method `StoreLearnedRule` & `RetrieveLearnedRules`. Test client lulus (5/5 passed).
  - `engine/src/memory/proposal.go`: `NewProposalStoreWithGateway` & `SyncFromGateway` menghubungkan lifecycle learning ke gRPC SystemGateway. Test `TestProposalStoreWithGateway` lulus (13/13 passed).
  #### Detail Eksekusi -- BL-01
  1. **File & simbol spesifik:**
     - `/proto/agent_service.proto` - ADD RPC service `SystemGateway` method
     - `/engine/src/memory/proposal.go` - `MemoryProposal`, `ProposalStore` interface (sudah ada)
     - `/engine/src/memory/store.go` - `inMemoryVectorStore` (sudah ada)
     - `/crates/clawcrew-memory/src/lib.rs` - public API `clawcrew-memory` crate
     - `/crates/clawcrew-gateway/src/grpc_system_gateway.rs` - implementasi RPC
  2. **Spesifikasi API:**
     - **Proto definition (ADD to `SystemGateway`):**
       ```protobuf
       service SystemGateway {
           // ... existing methods ...
           rpc StoreLearnedRule(StoreLearnedRuleRequest) returns (StoreLearnedRuleResponse);
           rpc RetrieveLearnedRules(RetrieveLearnedRulesRequest) returns (RetrieveLearnedRulesResponse);
       }
       
       message StoreLearnedRuleRequest {
           string rule = 1;              // The learned rule/correction
           map<string, string> scope = 2; // Scope (fleet, ship, squad, crew, run)
           bool reversible = 3;          // Whether this can be reverted
       }
       
       message StoreLearnedRuleResponse {
           string id = 1;                // Proposal ID
           bool success = 2;
           string error = 3;
       }
       
       message RetrieveLearnedRulesRequest {
           string scope_key = 1;         // Optional filter by scope key (e.g., "fleet")
           string scope_value = 2;       // Optional filter by scope value
       }
       
       message RetrieveLearnedRulesResponse {
           repeated LearnedRule rules = 1;
       }
       
       message LearnedRule {
           string id = 1;
           string rule = 2;
           map<string, string> scope = 3;
           bool reversible = 4;
           string created_at = 5;
           bool approved = 6;
           bool reverted = 7;
       }
       ```
     - **Go side (reuse existing `proposal.go` logic):**
       ```go
       // ProposalStore interface already exists in proposal.go
       // Just wire it to gRPC RPC call
       func (s *ProposalStore) Approve(ctx context.Context, id string) (*MemoryProposal, error) {
           // Existing logic - writes to inMemoryVectorStore
           // For BL-01: write to Rust via gRPC instead
           return s.approveWithRustStore(ctx, id)
       }
       ```
     - **Rust side (reuse `clawcrew-memory`):**
       ```rust
       // Use existing clawcrew_memory::Memory trait
       // No custom vector store implementation!
       pub async fn store_learned_rule(
           memory: &Arc<dyn Memory>,
           rule: &str,
           scope: HashMap<String, String>,
           reversible: bool,
       ) -> Result<String> {
           // Use existing Memory::store API
           // Tag with category = "learned_rule"
           memory.store(
               format!("learned_{}", uuid::Uuid::new_v4()),
               rule,
               MemoryCategory::Core,
               Some(scope),
           ).await?;
           Ok("success".to_string())
       }
       ```
  3. **Contoh pengkodean (pola dari grpc_system_gateway.rs):**
     ```rust
     #[tonic::async_trait]
     impl SystemGateway for SystemGatewayService {
         async fn store_learned_rule(
             &self,
             request: Request<StoreLearnedRuleRequest>,
         ) -> Result<Response<StoreLearnedRuleResponse>, Status> {
             let req = request.into_inner();
             
             let memory = self.memory.clone(); // Arc<dyn Memory>
             
             let id = memory.store_learned_rule(
                 &req.rule,
                 req.scope,
                 req.reversible,
             ).await?;
             
             Ok(Response::new(StoreLearnedRuleResponse {
                 id,
                 success: true,
                 error: String::new(),
             }))
         }
         
         async fn retrieve_learned_rules(
             &self,
             request: Request<RetrieveLearnedRulesRequest>,
         ) -> Result<Response<RetrieveLearnedRulesResponse>, Status> {
             let req = request.into_inner();
             
             let rules = self.memory.retrieve_learned_rules(
                 req.scope_key,
                 req.scope_value,
             ).await?;
             
             Ok(Response::new(RetrieveLearnedRulesResponse {
                 rules,
             }))
         }
     }
     ```
  4. **Konvensi penamaan:**
     - Proto: `StoreLearnedRuleRequest/Response`, `RetrieveLearnedRulesRequest/Response`
     - Go: `StoreLearnedRule`, `RetrieveLearnedRules` methods
     - Rust: `store_learned_rule`, `retrieve_learned_rules` functions
  5. **Langkah pengerjaan berurutan:**
     1. **PROTO FIRST** - Update `proto/agent_service.proto` dengan RPC methods
     2. Generate Go/Rust code: `protoc --go_out=... --rust_out=...`
     3. Implement Rust side: wire to `clawcrew-memory::Memory::store`
     4. Implement Go side: call Rust via gRPC
     5. Test: `Approve()` writes to Rust store, survives restart
  6. **Acceptance criteria testable:**
     - [x] `ProposalStore.Approve()` writes to Rust `clawcrew-memory`
     - [x] Restart Go process → learned rules still exist
     - [x] `ProposalStore.Revert()` deletes from Rust store
  7. **Dependency antar-tiket:** None (independen)
  8. **Edge case:** Concurrent approvals, scope collision, memory full
  9. **JANGAN DIUBAH:** Existing `inMemoryVectorStore` behavior - hanya ganti destination
  10. **ASUMSI:** `clawcrew-memory` crate public API cukup untuk learned rule storage; no breaking changes di Rust Memory trait

- [ ] **BL-03 (LOW, dokumentasi murni, bisa paralel kapan saja)** Dokumentasikan ekosistem `open-skills` yang sudah matang (`crates/clawcrew-runtime/src/skills/mod.rs`, 3-lapis loading workspace→global→plugin WASM) — risiko duplikasi SSOT kalau tim lain menganggap belum ada. Tulis `standard-docs/tech-standart/plugin-standard/open-skills-ecosystem.md`.
  #### Detail Eksekusi -- BL-03
  1. **File & simbol spesifik:**
     - `/crates/clawcrew-runtime/src/skills/mod.rs` - reference implementation
     - `/standard-docs/tech-standart/plugin-standard/open-skills-ecosystem.md` (baru)
  2. **Spesifikasi API:**
     - **3-lapis loading:**
       - Layer 1: Workspace skills (`~/.kiro/crew/workspace/skills/`)
       - Layer 2: Global skills (`~/.kiro/crew/skills/`)
       - Layer 3: Plugin WASM skills (`plugins/*.wasm`)
     - **Skill format:**
       - `SKILL.md` - main instructions
       - `META.toml` - metadata (name, version, author, triggers)
       - `assets/` - optional static assets
  3. **Contoh pengkodean:**
     - Tidak ada kode baru, dokumentasi murni
     - Contoh struktur:
       ```
       skills/
       ├── workspace/
       │   └── my-skill/
       │       ├── SKILL.md
       │       ├── META.toml
       │       └── assets/
       ├── global/
       │   └── common-skill/
       │       ├── SKILL.md
       │       └── META.toml
       └── plugins/
           └── plugin1.wasm
       ```
  4. **Konvensi penamaan:**
     - Title: "Open Skills Ecosystem"
     - Sections: Architecture, Skill Format, Loading Priority, Plugin System
  5. **Langkah pengerjaan berurutan:**
     1. Baca `skills/mod.rs` - pahami loading logic
     2. Buat `/standard-docs/tech-standart/plugin-standard/` directory
     3. Tulis `open-skills-ecosystem.md`
     4. Add example skill structure
     5. Document loading priority
  6. **Acceptance criteria testable:**
     - [ ] Dokumentasi tersedia di `standard-docs/tech-standart/plugin-standard/open-skills-ecosystem.md`
     - [ ] Document menjelaskan 3-lapis loading
     - [ ] Document menjelaskan skill format
  7. **Dependency antar-tiket:** None (independen, dokumentasi)
  8. **Edge case:** N/A
  9. **JANGAN DIUBAH:** N/A
  10. **ASUMSI:** Skills format sudah matang; no breaking changes di loading logic

- [ ] **BL-04 (MEDIUM, verifikasi — lebih bermakna SETELAH BL-01 selesai)** Test baru: skill/steering inheritance untuk sub-agent (`DelegateTaskTool`) — verifikasi sub-agent mewarisi skill-bundle yang sama dengan parent (bukan skill-set kosong tanpa alasan). File: `engine/src/tool/phase3_tool_test.go` atau test baru.
  #### Detail Eksekusi -- BL-04
  1. **File & simbol spesifik:**
     - `/engine/src/tool/phase3_tool_test.go` - existing test
     - `/engine/src/tool/delegate.go` - `DelegateTaskTool` implementation
     - `/engine/src/memory/proposal.go` - learned rules ( untuk BL-01 dependency)
  2. **Spesifikasi API:**
     - Test suite: `TestSubAgentSkillInheritance`
     - Verify: Sub-agent punya same `SkillBundle` dengan parent
     - Test case: parent has learned rules → sub-agent inherits them
  3. **Contoh pengkodean (pola dari phase3_tool_test.go):**
     ```go
     func TestSubAgentSkillInheritance(t *testing.T) {
         // Setup parent agent with learned rules
         parent := NewAgent("parent", ...).
             WithLearnedRules([]string{
                 "Always validate input",
                 "Use context properly",
             })
         
         // Spawn sub-agent
         subAgent, err := parent.SpawnSubAgent("child", "task")
         require.NoError(t, err)
         
         // Verify inheritance
         subRules := subAgent.GetLearnedRules()
         assert.Contains(t, subRules, "Always validate input")
         assert.Contains(t, subRules, "Use context properly")
     }
     ```
  4. **Konvensi penamaan:**
     - Test: `TestSubAgentSkillInheritance`
     - Method: `GetLearnedRules()`, `WithLearnedRules()`
  5. **Langkah pengerjaan berurutan:**
     1. Wait for BL-01 completion (learned rules persist)
     2. Update `delegate.go` - ensure skill bundle propagation
     3. Add test di `phase3_tool_test.go`
     4. Run test: verify inheritance
  6. **Acceptance criteria testable:**
     - [ ] Test `TestSubAgentSkillInheritance` pass
     - [ ] Sub-agent punya same skills/learned rules dengan parent
  7. **Dependency antar-tiket:** BL-01 (learned rules harus persist dulu)
  8. **Edge case:** Large skill bundle, circular inheritance, permission scope
  9. **JANGAN DIUBAH:** Existing skill loading - hanya tambah inheritance test
  10. **ASUMSI:** `DelegateTaskTool` sudah propagate skill bundle; no breaking changes di agent API

> **BL-02 (GAP-01, perancangan steering produk Galleon end-user) TIDAK dimasukkan sprint ini** — scope-nya adalah ADR/design-spec (keputusan one-way-door format file untuk end-user), bukan kerja kode. Lebih cocok sebagai item mandiri di Sprint 7 (produk FE) karena menyentuh pengalaman end-user `galleon`/`galleoncode` CLI, bukan backend internal — lihat Sprint 7 §F.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `engine/pkg/store/postgres/`, `engine/pkg/store/vector/` (baru) | Backend-Go |
| B | `crates/clawcrew-{gateway,channels,runtime}/src/**` (per-crate berseri, JANGAN paralel di crate yang sama) | Backend-Rust |
| C | `crates/clawcrew-runtime/src/agent/{history_trim,loop_}/mod.rs` (verified paths) | Backend-Rust |
| D | `crates/clawcrew-memory/src/knowledge_graph.rs`, `clawcrew-api/src/knowledge.rs` | Backend-Rust |
| E | `proto/agent_service.proto`, `crates/clawcrew-gateway/src/grpc_system_gateway.rs`, `engine/src/memory/{proposal,store,wire}.go`, `standard-docs/tech-standart/plugin-standard/` (BL-03) | Backend-Go + Backend-Rust (BL-01 serial: proto dulu) |

## Keluaran Sprint

- RAG/vector Go-Rust tidak lagi dua sistem paralel tanpa jembatan.
- Burn-down `unwrap/expect` turun signifikan dari ±400 hit baseline.
- Data konkret ada/tidaknya korelasi trim-history vs cache-miss (R-88) — keputusan R-89 berbasis data, bukan hipotesis.
- Node Lesson/Decision punya citation terverifikasi, bukan klaim tanpa sumber.
- "Learning by Consent" (Go) persisten lintas-restart lewat jembatan gRPC ke Rust vector store — bukan lagi `inMemoryVectorStore` yang hilang saat proses mati.