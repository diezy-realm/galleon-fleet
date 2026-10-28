# Galleon Fleet — Analisis & Kesimpulan Observasi

> **Pembaruan 2026-10-04:** isi di bawah adalah snapshot saat observasi awal. Status terkini diverifikasi dari kode di [dev-tasks/README.md](./dev-tasks/README.md) (Kondisi Terakhir): Lane D & ADR 0001/0002/0003 sisi Go sudah selesai sebagian besar; sisa pekerjaan di dev-tasks/.
> **Scope:** Phase 2 (SSOT Migration) → Phase 3 (DynamicRouter) → Phase 4 (ADR 0001/0002/0003)  
> **Branch:** `feat/refactoring-rust`  
> **Tanggal observasi:** 2026-10-04  
> **Status dokumen:** Living — diperbarui setelah tiap sesi eksekusi selesai

---

## 1. Ringkasan Eksekutif

Dari tiga sesi observasi terhadap dokumen `standard-docs/decisions/` dan perbandingan langsung dengan kode aktual di branch `feat/refactoring-rust`, ditemukan:

- **~80% pekerjaan Phase 2 sudah selesai** di sesi-sesi sebelumnya (Lane A/B/C/E/F mayoritas ✅).
- **Phase 2 Lane D (Go→Rust sandboxed tool execution)** adalah satu-satunya lane yang belum dimulai, namun teknisnya sudah *unblocked* (Rust `SystemGatewayService.execute_native_tool` aktif di `:50052`, Go `WithSystemGateway` sudah terpasang di `wire_gen.go`).
- **Phase 3 (DynamicRouter)** memiliki fondasi prototipe yang sudah ada (`galleon-routing` dengan 3 unit test lolos), namun belum di-wire sama sekali ke `clawcrew-gateway`.
- **ADR 0001 (LLM SSOT Rust)** — sudah diterima prinsipnya, tetapi `ExecuteTurn` di proto belum ada, Go `engine/src/llm/` masih aktif, tidak ada wiring Rust ke gRPC LLM sama sekali.
- **ADR 0002 & ADR 0003** — diterima, namun pelanggaran kode aktif masih ada: QR + telemetri di `web/server.ts`, dan `get_system_metrics()` di `apps/tauri/src/main.rs` masih mengembalikan konstanta palsu.

---

## 2. Kesimpulan Per Area

### 2.1 Phase 2 — SSOT Migration (Go Engine + Web-2)

#### Lane A — Build Green & Hygiene
| Kode | Temuan | Status Aktual |
|------|--------|---------------|
| A1 | Hapus modul orphan `apps/zerocode/src/api/engine_client.rs` | ✅ Selesai |
| A2 | Wire `orchestrator/` sebagai executive planner (otak #2) | ✅ Tersambung: `wire_gen.go:74-76` sudah memanggil `qmorch.NewService` dan `SetObjectiveProposer` |
| A3 | Pindahkan skrip `fix_*.py` / `patch*.py` → `scripts/migrations/` | ✅ Selesai |
| A4 | Sinkronkan `AUDIT.md` | ✅ Selesai |
| A5 | Clippy workspace | ✅ Selesai (lib targets) |

#### Lane B — SSOT Data ke Go
| Kode | Temuan | Status Aktual |
|------|--------|---------------|
| B1 | Wire `DiskStore` menggantikan `MemoryStore` | ✅ Terverifikasi di `wire_gen.go:43,50,65` |
| B2 | Seed JSON kanonikal di `engine/data/*.json` + endpoint `/api/fleet/seed` | ✅ 15 file JSON dibuat |
| B3 | `/api/fleet/policies` sebagai satu-satunya sumber risk-tier | ✅ Merged di commit `56cd4888` |

#### Lane C — Stub Go menjadi Logic Nyata
| Kode | Temuan | Status Aktual |
|------|--------|---------------|
| C1 | `GetMetrics` baca store nyata | ✅ Selesai |
| C2 | `ExecuteTask` gRPC route | ✅ Proto extended + client implemented |
| C2a | Quartermaster router dua-mode (ClassifyIntent enum) | ✅ Merged di commit `56cd4888` |
| C3 | `GetDiagnostics`/`GetExecutiveBriefing` dari sinyal nyata | ⚠️ Code complete, namun frontend type regressions masih ada (E2/E3 blocker) |
| C4 | Selaraskan kontrak route frontend↔engine | ❌ Belum dimulai |
| C5 | "Learning by consent" — `MemoryProposal` sebagai Artifact pending approval | ❌ Belum dimulai |

#### Lane D — Go→Rust SystemGateway (⚠️ Critical — belum dimulai)
| Kode | Temuan | Status Aktual |
|------|--------|---------------|
| D1 | `builtin_*.go` eksekusi melalui gRPC ke Rust `:50052`, bukan langsung di Go | ❌ Belum. `WithSystemGateway` terpasang, tapi masih ada fallback in-process di setiap `builtin_*.go` |
| D2 | Verifikasi sisi Rust `SystemGateway.ExecuteTool` menerima panggilan Go, Landlock boundary aktif | ❌ Belum. `execute_native_tool` di Rust sudah ada, tapi belum ada round-trip test + Landlock test |

**Temuan kritis Lane D:** Di `crates/clawcrew-gateway/src/grpc_system_gateway.rs`, method `get_decrypted_secret` masih berupa `// ponytail: stub` (mengembalikan `Status::unimplemented`). Artinya sekalinya D1 dieksekusi, alur `Go → bash/git/read_file via Rust` akan berjalan, tapi alur pengambilan secret (`Go → GetDecryptedSecret → Rust Vault`) masih stub.

#### Lane E — Frontend Tipe & De-mock
| Kode | Temuan | Status Aktual |
|------|--------|---------------|
| E1 | Ketik ulang `apiClient.ts` (hilangkan `Promise<any>`) | ✅ grep `any` = 0 match |
| E2 | Hapus `seedData.ts`, store hidrasi dari `apiClient.getCollection()` | ⚠️ QA blocker: `HarborTool` type undefined (`HarborView.tsx:110`), `ProcessItem` shape mismatch (`EngineRoomView.tsx:382,395`) |
| E3 | Hapus `initialRiskTiers`/`initialFleetPolicies` dari TS | ✅ Merged di commit `56cd4888` |

#### Lane F — Invariant Branding & Core Business
| Kode | Temuan | Status Aktual |
|------|--------|---------------|
| F1 | Treasury invariant (Timber bukan unit inference, BYOK transparan read-only) | ✅ Tertulis di steering + interfaces.go |
| F2 | CI gate `no_duplicate_state` | ✅ `tests/architecture/no_duplicate_risk_tier.rs` berjalan di CI |

#### ADR-001 — Channel Distribution (3-Tier)
- **Status:** `Proposed` — **menunggu sign-off Owner.**
- Fondasi WASI host (`galleon-plugins`) dan WIT contract (`wit/v0/channel.wit`) sudah tersedia.
- `crates/galleon-channels` masih monolit ~197k LOC. Belum ada pemisahan `galleon-channel-*`.

---

### 2.2 Phase 3 — DynamicRouter (`galleon-routing` → `clawcrew-gateway`)

**Temuan kunci:**
1. `crates/galleon-routing/src/lib.rs` **sudah ada** dengan `RouteHandler` trait + `DynamicRouter` struct + 3 passing unit tests. Namun **tidak ada satu pun crate/binary yang mengimpornya** — orphan crate.
2. `PluginCapability` enum (`Tool | Channel | Memory | Observer | Skill`) **sudah ada** di `crates/galleon-plugins/src/lib.rs:107`. Gap 1 adalah reuse, bukan membuat baru.
3. `crates/clawcrew-gateway/src/lib.rs` memiliki **49 `pub mod`** (sebagian feature-gated), membenarkan urgensi dekomposisi.
4. `SlidingWindowRateLimiter` sudah ada di `auth_rate_limit.rs` — bisa di-reuse untuk rate-limit per-IP saat rute miss (Gap 3), tidak perlu membuat limiter baru.
5. `security_headers::apply` sudah berupa `axum::middleware::Next` layer yang berfungsi (Gap 4).

**Keputusan yang perlu sign-off Principle sebelum Backend mulai:**
1. Gap 1: Approve mengubah `capability() -> Option<&str>` → `Option<PluginCapability>`.
2. Gap 2: Approve *defer* compile-time plugin-crate split (tidak ada `crates/plugin-telegram` dulu, cukup feature-gate di dalam gateway).
3. Gap 5: Konfirmasi jumlah handler 41 modul (bukan 39 seperti di brief) dan urutan migrasi 3 fase.

---

### 2.3 Phase 4 — ADR 0001 / 0002 / 0003

#### ADR 0001 — LLM SSOT: Rust `galleon-providers`
| Kondisi Dokumen | Kondisi Kode Aktual |
|---|---|
| Diterima — Go mendelegasikan LLM ke Rust gRPC | **BELUM diimplementasikan sama sekali** |
| `ExecuteTurn` RPC harus ditambahkan ke proto | `proto/agent_service.proto` hanya punya `ExecuteNativeTool`, `GetDecryptedSecret`, `ExecuteTask` — **`ExecuteTurn` belum ada** |
| Go `engine/src/llm/` harus dihapus | File masih lengkap: `provider.go`, `multi_provider.go`, `dispatcher.go`, `mock_provider.go`, dll. |
| Rust `galleon-providers/` SSOT | Crate sangat lengkap (~40+ file vendor), tapi belum ada handler gRPC untuk melayani Go |

**Ini adalah item implementasi terbesar dan paling fundamental** karena menyentuh path eksekusi utama (`crew.StartTurn`).

#### ADR 0002 — web Zero Backend Logic
| Kondisi Dokumen | Kondisi Kode Aktual |
|---|---|
| `web` dilarang memuat logika backend | `web/server.ts:7` masih `import qrcode from 'qrcode'` |
| QR code → Go HTTP endpoint | `server.ts:59-75` masih generator QR aktif di Express |
| System telemetry → Go HTTP endpoint | `server.ts:77-95` masih membaca `os.totalmem()`, `os.cpus()` langsung |

**Pelanggaran ini adalah technical debt yang sudah dicatat secara eksplisit** di ADR 0002 sebagai "temporarily accepted as-is."

#### ADR 0003 — Tauri: Real API, Bukan Fake Data
| Kondisi Dokumen | Kondisi Kode Aktual |
|---|---|
| `get_system_metrics()` harus baca dari Go `:9090/api/system/metrics` | `apps/tauri/src/main.rs:88-96` masih mengembalikan `gateway_latency_ms: 8` dan `memory_db_mb: 18.4` sebagai konstanta palsu |
| Perintah Tauri lain sudah ter-migrasi | ✅ Betul: `get_executive_briefing`, `get_harbor_providers`, `get_diagnostics`, `get_fleet_policies`, `chat_quartermaster` semuanya sudah `engine_get(...)` |

**Ini adalah item terisolasi paling kecil** — hanya satu fungsi di satu file, bisa diselesaikan < 1 jam.

---

## 3. Peta Ketergantungan (Dependency Map)

```
ADR 0002 (web zero logic)
    └── Phase 1: Go engine QR endpoint
            └── ADR 0003 (Tauri real metrics)
                    └── Phase 1: Go /api/system/metrics (depends ADR 0002 Phase 1)

ADR 0001 (LLM SSOT Rust) ← TERBESAR
    ├── Phase 1: proto ExecuteTurn + Rust handler
    ├── Phase 2: Go crew/services.go delegation
    └── Phase 3: Delete Go engine/src/llm/**

Phase 2 Lane D (Sandbox Tool via Rust)
    ├── D1: bulitin_*.go → Rust gRPC (unblocked, perlu fail-closed decision)
    └── D2: Rust Landlock boundary test round-trip

Phase 2 (C4/C5)
    ├── C4: Selaraskan kontrak route frontend↔engine
    └── C5: MemoryProposal learning-by-consent

Phase 2 Lane E2 (QA Blocker)
    └── Fix HarborTool type + ProcessItem shape → buka jalan C3 merge

Phase 3 DynamicRouter
    ├── Fase 1: Wire RouteHandler (5 core handlers) — gate selanjutnya
    ├── Fase 2: Migrasi 15 handler API paralel
    └── Fase 3: Handler streaming (SSE/WS) — setelah streaming test lulus

ADR-001 Channel Distribution
    └── Pilot Tier C (2 WASI plugin inbound-first) → bekukan WIT v0 → lapis B crate split
```

---

## 4. Prioritas Eksekusi

Berdasarkan dampak arsitektur dan dependency, urutan yang direkomendasikan:

| # | Item | Ukuran | Dampak | Blocker? |
|---|------|--------|--------|----------|
| 1 | **ADR 0003** — `get_system_metrics` Tauri real API | XS (<1 jam) | Menghilangkan fake data di production desktop | — |
| 2 | **E2 QA Fix** — `HarborTool` type + `ProcessItem` shape | S (<2 jam) | Membuka jalan merge C3 (diagnostics nyata) | C3 merger |
| 3 | **ADR 0002** — Go QR + metrics endpoint | S (1 hari) | Menutup pelanggaran batas tier paling visible | ADR 0003 Phase 1 |
| 4 | **C4** — Route contract frontend↔engine | M (1 hari) | Mencegah routing mismatch API silent | — |
| 5 | **D1/D2** — Sandbox tool execution via Rust | M (2 hari) | Menegakkan "Autonomy under Command" — sandbox nyata | Perlu approval fail-closed |
| 6 | **ADR 0001** — Proto `ExecuteTurn` + Rust gRPC handler | L (2-3 hari) | Fondasi LLM SSOT, menghapus duplikasi vendor | Terbesar |
| 7 | **Phase 3 Fase 1** — Wire 5 core RouteHandler ke DynamicRouter | M (1-2 hari) | Membuka migrasi 41 handler secara bertahap | Sign-off Gap 1/2/5 |
| 8 | **C5** — MemoryProposal learning-by-consent | L (multi-hari) | Core product promise "learning" | B1 + approval flow |
| 9 | **ADR-001 Channel Pilot** — 2 WASI plugin Tier C | L | Membuktikan channel distribution model | Sign-off ADR-001 |
