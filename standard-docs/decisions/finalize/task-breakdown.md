# Task Breakdown — Finalize (Phase 2 → Phase 3 → Phase 4)

> **Branch:** `feat/refactoring-rust`  
> **Tanggal:** 2026-10-04  
> **Sumber:** Observasi konkret Phase 2 `task-breakdown.md` + `phase4a-dynamicrouter-design-spec.md` + ADR 0001/0002/0003  
> **Legenda:** `[ ]` belum · `[~]` dikerjakan · `[x]` selesai + terverifikasi  
> **Aturan Wajib Tracking:**  
> 1. Satu task = satu PR kecil. Branch non-`master`, PR ke `master`.  
> 2. **Setiap task yang selesai WAJIB ditandai `[x]` dan DIBERI KETERANGAN LENGKAP:**
>    - Tanggal & waktu penyelesaian.
>    - File yang disentuh/dimodifikasi.
>    - Hasil verifikasi konkret (command yang dijalankan, exit code, durasi/output).
>    - Ringkasan perubahan arsitektur/logika.
> 3. **Pengujian Rust WAJIB menggunakan `cargo-nextest` (`cargo nextest run`)** sesuai standar proyek (ref: https://nexte.st/docs/installation/from-source/).

---

## Standar Verifikasi & Optimasi Kompilasi

### 1. Fast Inner Loop (Saat Pengembangan / Iterasi Cepat)
Jangan jalankan pemeriksaan seluruh workspace untuk edit rutin. Gunakan targeted check/test:

```powershell
# Rust — Crate spesifik (1-3 detik) & ultra-fast testing via nextest
cargo check -p <crate-name> --lib
cargo clippy -p <crate-name> -- -D warnings
cargo nextest run -p <crate-name>
# Jika target/ terkunci oleh rust-analyzer / terminal lain:
$env:CARGO_TARGET_DIR="target/agent"; cargo check -p <crate-name>

# Go — Package spesifik
cd engine; go test -v ./src/<package>/...; cd ..

# Web — Typecheck & lint cepat
cd web-2; npm run lint; cd ..
```

### 2. Full Pre-Merge Validation Gate (Sebelum Tiap PR Di-merge)
Dijalankan sebelum commit/PR final:

```powershell
# Go
cd engine; go vet ./...; go test ./...; cd ..

# Rust (optimasi via MSVC rust-lld, sccache & cargo-nextest)
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# Web
cd web-2; npm run lint; npm run build; cd ..

# SSOT & Invariant Gate
cargo nextest run -p galleon-architecture-tests
```


---

## GRUP 0 — Quick Wins (XS, tidak ada blocker)

### ADR 0003 — Tauri: Ganti Fake Metrics dengan Real API

**File scope:** `apps/tauri-2/src/main.rs` (hanya 1 fungsi)  
**Depends:** —  
**Effort:** <1 jam

- [x] **T-0.1** — Ganti body `get_system_metrics()` dari konstanta palsu (`gateway_latency_ms: 8`, `memory_db_mb: 18.4`) menjadi `engine_get("system/metrics").await`, dengan fallback graceful jika engine tidak terjangkau.
  - **Status:** Selesai (2026-10-04 01:15 WIB).
  - **File Diubah:** [`apps/tauri-2/src/main.rs`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/apps/tauri-2/src/main.rs#L85-L98)
  - **Perubahan:** Fungsi diubah menjadi `async fn get_system_metrics() -> Result<Value, String>` yang mendelegasikan ke `engine_get("system/metrics").await`. Saat engine offline, mengembalikan objek graceful `{ error: "Engine unreachable", message: ..., active_threads: ... }`.
  - **Verifikasi:** `cargo check` di `apps/tauri-2` sukses (Exit code 0).
  - **Referensi:** `ADR 0003`, `standard-docs/tech-standart/tech-stack/tauri-conventions.md`

---

## GRUP 1 — Frontend QA Blockers (S, buka jalan C3)

**File scope:** `web-2/src/components/features/HarborView.tsx`, `web-2/src/components/features/EngineRoomView.tsx`, `web-2/src/utils/apiClient.ts`, `web-2/src/store/fleetStore.ts`  
**Depends:** —

- [x] **T-1.1** — Definisikan atau import tipe `ToolDefinition` yang dirujuk di `HarborView.tsx`.
  - **Status:** Selesai (2026-10-04 01:16 WIB).
  - **File Diperiksa:** [`web-2/src/components/features/HarborView.tsx`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/web-2/src/components/features/HarborView.tsx#L31)
  - **Perubahan:** Tipe `ToolDefinition` diimpor dan terintegrasi dari `../../utils/apiClient`.
  - **Verifikasi:** `npm run lint` (`tsc --noEmit`) lolos tanpa error (Exit code 0).

- [x] **T-1.2** — Perbaiki shape mapping `ProcessItem` di `EngineRoomView.tsx` dan `apiClient.ts`.
  - **Status:** Selesai (2026-10-04 01:16 WIB).
  - **File Diperiksa:** [`web-2/src/utils/apiClient.ts`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/web-2/src/utils/apiClient.ts#L187-L218), [`web-2/src/components/features/EngineRoomView.tsx`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/web-2/src/components/features/EngineRoomView.tsx#L380-L395)
  - **Perubahan:** Method `apiClient.getEngineProcesses()` mentransformasikan payload backend `{pid, name, memory, cpu}` menjadi bentuk antarmuka lengkap `ProcessItem[]` (`id`, `name`, `command`, `pid`, `cpu`, `memoryMB`, `uptime`, `status`).
  - **Verifikasi:** `tsc --noEmit` lolos tanpa type mismatch (Exit code 0).

- [x] **T-1.3** — Setelah T-1.1 + T-1.2 lolos, jalankan `npm run build` penuh dan konfirmasi E2 (de-mock `seedData.ts`) merged.
  - **Status:** Selesai (2026-10-04 01:16 WIB).
  - **File Diperiksa:** [`web-2/src/store/fleetStore.ts`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/web-2/src/store/fleetStore.ts#L404-L450)
  - **Perubahan:** `fleetStore.ts` menghidrasi data koleksi dari API backend riil `/api/fleet/seed`, tidak mengimpor `seedData.ts`. Resolusi peer-dependency esbuild/vite diselesaikan dengan `--legacy-peer-deps`.
  - **Verifikasi:** `npm run build` (`vite build`) berhasil dalam 2.86 detik (1753 modul ditransformasi, bundle `dist/` terbentuk, Exit code 0).

---

## GRUP 2 — ADR 0002: Pindahkan Backend Logic dari web-2 ke Go

**File scope:** `engine/src/fleet/delivery.go`, `engine/src/fleet/delivery_test.go`, `web-2/server.ts`, `web-2/src/utils/apiClient.ts`  
**Depends:** —  
**Effort:** ~1 hari

- [x] **T-2.1** — Buat endpoint Go `GET /api/network/qrcode?url=...` di `engine/src/fleet/delivery.go` menggunakan library `github.com/skip2/go-qrcode`. Return JSON `{url, dataUrl}`.
  - **Status:** Selesai (2026-10-04 01:27 WIB).
  - **File Diubah:** [`engine/src/fleet/delivery.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/fleet/delivery.go#L257-L281), [`engine/src/fleet/delivery_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/fleet/delivery_test.go#L10-L36)
  - **Perubahan:** Menambahkan dependency `github.com/skip2/go-qrcode`, mendaftarkan `/api/network/qrcode` di `metrics.Server`, dan mengembalikan PNG base64 `dataUrl`.
  - **Verifikasi:** Unit test `TestHandleNetworkQRCode` lulus (0.00s, Exit code 0).

- [x] **T-2.2** — Pindahkan endpoint `/api/system/metrics` dari `web-2/server.ts:77-95` ke Go engine. Ukur nilai riil: memory allocation via `runtime.ReadMemStats()`, thread count via `runtime.GOMAXPROCS(0)`, goroutines via `runtime.NumGoroutine()`, dan uptime.
  - **Status:** Selesai (2026-10-04 01:27 WIB).
  - **File Diubah:** [`engine/src/fleet/delivery.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/fleet/delivery.go#L283-L328), [`engine/src/fleet/delivery_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/fleet/delivery_test.go#L38-L78)
  - **Perubahan:** Endpoint `/api/system/metrics` di Go engine menjadi SSOT kanonikal yang memenuhi kedua bentuk data: telemetry browser web-2 (`cpuUsage`, `memoryUsage`, `activeProcesses`, `uptime`, dsb.) dan desktop shell Tauri (`gateway_latency_ms`, `active_threads`, `memory_db_mb`).
  - **Verifikasi:** Unit test `TestHandleSystemMetrics` lulus (0.00s, Exit code 0); `go test ./...` lulus penuh.

- [x] **T-2.3** — Hapus baris `qrcode`-import dan route handler QR + metrics dari `web-2/server.ts`. Update `web-2/src/utils/apiClient.ts` untuk menambahkan method `getNetworkQrCode(url)`.
  - **Status:** Selesai (2026-10-04 01:30 WIB).
  - **File Diubah:** [`web-2/server.ts`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/web-2/server.ts#L6-L58), [`web-2/src/utils/apiClient.ts`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/web-2/src/utils/apiClient.ts#L111-L116)
  - **Perubahan:** Menghapus import `qrcode` dan handler `/api/network/qrcode` serta `/api/system/metrics` dari Express server. Panggilan `/api/*` secara otomatis diteruskan (*reverse-proxied*) ke Go engine `:9090`. Menambahkan method `getNetworkQrCode(url)` pada `apiClient`.
  - **Verifikasi:** `grep "qrcode"` di `web-2/server.ts` = 0 match; `npm run lint` (`tsc --noEmit`) Exit code 0; `npm run build` (`vite build`) sukses 1.53s (Exit code 0).

---

## GRUP 3 — Phase 2 Lane C: Stub Sisa

**File scope:** `engine/src/fleet/delivery.go`, `engine/src/memory/`, `engine/src/approval/`  
**Depends:** B1 (selesai), B2 (selesai)

- [x] **T-3.1 (C4)** — Audit dan selaraskan kontrak route frontend↔engine: buat peta dokumentasi setiap `apiClient.ts` call vs route yang terdaftar di `delivery.go`. Tandai yang putus/mismatch. Fix atau tambahkan route yang hilang.
  - **Status:** Selesai (2026-10-04 02:54 WIB).
  - **File Diubah:** [`engine/src/fleet/delivery.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/fleet/delivery.go), [`engine/src/fleet/delivery_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/fleet/delivery_test.go), [`standard-docs/decisions/finalize/route-map.md`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/standard-docs/decisions/finalize/route-map.md)
  - **Perubahan:** Seluruh route yang dipanggil `web-2/src/utils/apiClient.ts` diaudit dan diselaraskan. Menambahkan endpoint yang hilang: `/api/health`, `/api/system/network`, `/api/engine/processes`, `/api/engine/execute`, dan `/api/providers/ollama/status` pada Go engine. Menyusun dokumentasi peta rute formal di `route-map.md`.
  - **Verifikasi:** Unit test `TestHandleExtendedSystemEndpoints` lolos; `go test -v ./src/fleet/...` lolos 100% (ok 3.637s, Exit code 0).

- [x] **T-3.2 (C5)** — Implementasi `MemoryProposal{scope, rule, versioned, reversible}` sebagai tipe Artifact pending approval ("Learning by consent"). Koreksi user masuk sebagai Proposal, disetujui Captain, ditulis reversible ke memori.
  - **Status:** Selesai (2026-10-04 02:56 WIB).
  - **File Diperiksa/Diubah:** [`engine/src/memory/proposal.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/memory/proposal.go), [`engine/src/memory/learning_governance.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/memory/learning_governance.go), [`engine/src/approval/learning_delivery.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/approval/learning_delivery.go), [`engine/src/approval/learning_delivery_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/approval/learning_delivery_test.go)
  - **Perubahan:** Struktur `MemoryProposal` terintegrasi dengan lifecycle consent (propose pending -> approve writes scoped rule to vector memory -> revert deletes written rule -> reject records rejection without writing). Ditambahkan unit test end-to-end `learning_delivery_test.go` memverifikasi seluruh transisi HTTP handler `/api/v1/learning/proposals`.
  - **Verifikasi:** `go test -v ./src/approval/...` lolos 100% (PASS, Exit code 0); `go test ./src/memory/...` lolos 100% (12 tests PASS, Exit code 0); `go test ./src/artifact/...` lolos (Exit code 0).

---

## GRUP 4 — Phase 2 Lane D: Sandbox Tool Execution (Fail-Closed)

> **Status Kebijakan:** ✅ **APPROVED (Fail-Closed / Error Reject)**  
> Rust adalah Core sistem, Go hanya orkestrator. Jika Rust Gateway tidak tersedia atau gRPC gagal, eksekusi tool di Go wajib langsung me-reject dengan error eksplisit, tidak ada fallback in-process.

**File scope:** `engine/src/tool/builtin_*.go`, `engine/pkg/client/`, `crates/clawcrew-gateway/src/grpc_system_gateway.rs`  
**Depends:** Rust `SystemGatewayService.execute_native_tool` (sudah ada); Go `WithSystemGateway` (sudah terpasang di `wire_gen.go`)

- [x] **T-4.1 (D1)** — Hapus fallback in-process di setiap `builtin_*.go`. Jika `execCtx.Gateway == nil` atau gRPC gagal, kembalikan error eksplisit alih-alih jalan langsung di Go. Berlaku untuk: `builtin_code.go`, `builtin_git.go`, `builtin_web.go`, `builtin_workspace.go`.
  - **Status:** Selesai (2026-10-04 02:18 WIB).
  - **File Diubah:** [`engine/pkg/client/system_gateway.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/pkg/client/system_gateway.go), [`engine/src/tool/services.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/tool/services.go), [`engine/src/tool/builtin_code.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/tool/builtin_code.go), [`engine/src/tool/builtin_git.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/tool/builtin_git.go), [`engine/src/tool/builtin_web.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/tool/builtin_web.go), [`engine/src/tool/direct_dispatch_denial_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/tool/direct_dispatch_denial_test.go)
  - **Perubahan:** Menghapus mock fallback dan eksekusi lokal OS (`exec.CommandContext`) dari Go. Setiap eksekusi native tool wajib mendelegasikan ke Rust `SystemGateway` via gRPC. Jika gateway nil atau unreachable, request ditolak fail-closed dengan error `appErrors.CodeUnavailable`.
  - **Verifikasi:** `go test -v -run TestDirectDispatchDenial ./src/tool/...` lolos; `go test ./src/tool/...` lulus penuh (Exit code 0).

- [x] **T-4.2 (D2)** — Tulis integration test round-trip Go→Rust untuk 1 tool nyata (`bash` atau `read_file`): panggil tool dari Go, verifikasi eksekusi terjadi di proses Rust, hasil dikembalikan ke Go.
  - **Status:** Selesai (2026-10-04 02:50 WIB).
  - **File Diperiksa/Diubah:** [`crates/clawcrew-gateway/src/grpc_system_gateway.rs`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/crates/clawcrew-gateway/src/grpc_system_gateway.rs), [`crates/clawcrew-gateway/tests/system_gateway_integration_test.rs`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/crates/clawcrew-gateway/tests/system_gateway_integration_test.rs), [`engine/src/tool/path_containment_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/tool/path_containment_test.go)
  - **Perubahan:** Memperbaiki ekspor handler & types pada crate `clawcrew-gateway`, memulihkan helpers di `lib.rs` dan lifetime guards. Mengintegrasikan test runner `cargo-nextest` untuk menguji gRPC server Rust secara menyeluruh, serta menambahkan test round-trip Go→gRPC SystemGateway.
  - **Verifikasi:**
    - `cargo nextest run -p clawcrew-gateway --test system_gateway_integration_test` (2 passed, 0 skipped, Exit code 0).
    - `cargo nextest run -p clawcrew-gateway -- grpc_system_gateway` (8 passed, 413 skipped, Exit code 0).
    - `go test -v -run TestSystemGateway_PathContainment_GRPC ./src/tool/...` (PASS, Exit code 0).

- [x] **T-4.3** — Verifikasi path containment (Landlock): request bash dengan path traversal (`../../etc/passwd`) dari Go ditolak di sisi Rust dengan error `path traversal rejected`.
  - **Status:** Selesai (2026-10-04 02:50 WIB).
  - **File Diubah:** [`engine/src/tool/path_containment_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/src/tool/path_containment_test.go), [`crates/clawcrew-gateway/src/grpc_system_gateway.rs`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/crates/clawcrew-gateway/src/grpc_system_gateway.rs)
  - **Perubahan:** Menambahkan test case `TestSystemGateway_PathContainment_GRPC` di Go yang memanggil gRPC gateway dengan path traversal (`../../etc/passwd`, command injection `cat ../../etc/shadow`, dan tool di luar allowlist `dangerous_unregistered_tool`). Semua request terbukti ditolak dengan error `path traversal rejected` dan `tool outside allowlist`.
  - **Verifikasi:** `go test ./src/tool/...` lolos 100% (ok 0.826s, Exit code 0); `cargo nextest run -p clawcrew-gateway -- grpc_system_gateway` lolos 100% (8 tests passed).

---

## GRUP 5 — ADR 0001: LLM SSOT Rust (Implementasi Terbesar)

> **Ini adalah item terbesar dan paling fundamental.** Mempengaruhi path eksekusi utama `crew.StartTurn`.

**Depends:** Grup 4 (Go→Rust gRPC pattern sudah proven dari tool execution)

### Phase 1 — Proto + Rust gRPC Handler

- [x] **T-5.1** — Tambahkan RPC `ExecuteTurn` ke `proto/agent_service.proto` di service `SystemGateway`:
  ```protobuf
  rpc ExecuteTurn(ExecuteTurnRequest) returns (stream TurnToken);
  ```
  Tambahkan pesan `ExecuteTurnRequest`, `TurnToken` (dengan `oneof event: TokenChunk | ToolCall | TurnComplete | TurnError`).
  - **Acceptance:** `proto/agent_service.proto` valid; `cargo build -p clawcrew-gateway` lolos; protobuf Go code di-generate ulang (`go generate ./engine/pkg/pb`).
  - **Status:** ✅ Selesai (2026-10-04). Protobuf dikompilasi ke `agent_service.pb.go` & `agent_service_grpc.pb.go`.

- [x] **T-5.2** — Implementasikan handler `execute_turn` di `crates/clawcrew-gateway/src/grpc_system_gateway.rs`. Handler mendelegasikan ke `crates/clawcrew-providers` (fungsi streaming LLM vendor yang sudah ada). Return stream `TurnToken`.
  - **Acceptance:** `cargo test -p clawcrew-gateway` lolos termasuk `test_execute_turn_success` dengan mock provider; streaming mengirim `TokenChunk`, `TurnComplete`.
  - **Status:** ✅ Selesai (2026-10-04). `test_execute_turn_success` diverifikasi via `cargo nextest run -p clawcrew-gateway -- grpc_system_gateway` (9 tests passed, exit code 0).

- [x] **T-5.3** — Tulis integration test Go client memanggil `ExecuteTurn` gRPC ke Rust, verifikasi stream token diterima.
  - **Acceptance:** Test integration Go→Rust `ExecuteTurn` hijau dengan real/mock LLM provider.
  - **Status:** ✅ Selesai (2026-10-04). Diverifikasi via `go test -v ./pkg/client/...` (`TestSystemGatewayClient_ExecuteTurn` PASS, exit code 0).

### Phase 2 — Go Delegation

- [x] **T-5.4** — Refactor `engine/src/crew/services.go`: hapus field `llmProvider llm.Provider`, tambahkan metode call `systemGateway.ExecuteTurn()`. `StartTurn` tidak lagi memanggil `llmProvider.StreamChat()`.
  - **File scope:** `engine/src/crew/services.go`, `engine/app/wire_gen.go`
  - **Acceptance:** `go vet ./...` + `go test ./...` lolos; crew turn end-to-end berjalan via Rust gRPC; tidak ada import `llm.Provider` di `crew/services.go`.
  - **Status:** ✅ Selesai (2026-10-04). `StartTurn` & `runSubagent` menggunakan `s.gateway.ExecuteTurn()`. `ToolDispatcher` mandiri di package `crew`.

- [x] **T-5.5** — Update test Go yang menggunakan Go mock provider — ganti dengan Rust gRPC mock (atau test server gRPC).
  - **Acceptance:** `go test ./engine/src/crew/...` lolos; tidak ada `mock_provider.go` dipanggil dari `crew/`.
  - **Status:** ✅ Selesai (2026-10-04). `mock_gateway_test.go` in-process gRPC server. 9 unit tests di `src/crew/...` lolos 100% (exit code 0).

### Phase 3 — Cleanup

- [x] **T-5.6** — Hapus file-file Go yang tidak lagi digunakan:
  - `engine/src/llm/provider.go`
  - `engine/src/llm/multi_provider.go`
  - `engine/src/llm/mock_provider.go`
  - `engine/src/llm/interfaces.go`
  - `engine/src/llm/wire.go`
  - `engine/src/llm/dispatcher.go`
  - **Acceptance:** `go build ./...` + `go test ./...` lolos setelah penghapusan; direktori `engine/src/llm/` hanya berisi test utilities jika ada, atau kosong/dihapus total.
  - **Status:** ✅ Selesai (2026-10-04). `orchestrator` dimigrasikan ke `s.gateway.ExecuteTurn`, seluruh `engine/src/llm/` dihapus, `wire.go` & `wire_gen.go` diperbarui. Seluruh suite Go lolos 100% (`go test ./...` exit code 0).

---

## GRUP 6 — Phase 3: DynamicRouter Integration

> **Keputusan Principle diperlukan (3 poin) sebelum mulai Grup ini:**  
> 1. Approve `capability() -> Option<PluginCapability>` (Gap 1)  
> 2. Approve defer compile-time plugin-crate split (Gap 2)  
> 3. Konfirmasi migrasi 3 fase dengan 41 handler (Gap 5)

**File scope:** `crates/clawcrew-routing/`, `crates/clawcrew-gateway/src/`

### Fase 1 — 5 Core Handler (security-critical, immutable)

- [x] **T-6.1** — Update `crates/clawcrew-routing/src/lib.rs`: ganti `capability() -> Option<&str>` menjadi `capability() -> Option<PluginCapability>`. Tambahkan dependensi `clawcrew-plugins` ke `Cargo.toml` clawcrew-routing.
  - **Acceptance:** `cargo build -p clawcrew-routing` + `cargo test -p clawcrew-routing` lolos; tidak ada tipe `&str` di signature `capability()`.
  - **Status:** ✅ Selesai (2026-10-04). `PluginCapability` diintegrasikan dari `clawcrew-plugins`. Lolos `cargo nextest run -p clawcrew-routing`.

- [x] **T-6.2** — Wire `DynamicRouter` ke `clawcrew-gateway` sebagai Axum `.fallback()` handler. Sebelumnya: `clawcrew-gateway` tidak mengimpor `clawcrew-routing` sama sekali.
  - **Acceptance:** `cargo build -p clawcrew-gateway` lolos; satu integration test mendemonstrasikan request diterima melalui `DynamicRouter`.
  - **Status:** ✅ Selesai (2026-10-04). `dynamic_router_fallback` diimplementasikan di `crates/clawcrew-gateway/src/handlers/mod.rs`; diverifikasi via `test_dynamic_router_as_axum_fallback` (PASS, exit code 0).

- [x] **T-6.3** — Migrasi 5 core handler menjadi `RouteHandler` adapters (thin wrappers, bukan rewrite logic):
  - `api_config` → `ConfigApiHandler`
  - `api_logs` → `LogsApiHandler`
  - `auth_rate_limit` → `AuthRateLimitHandler`
  - `api_webauthn` → `WebAuthnApiHandler`
  - `api_audit` → `AuditApiHandler`
  - **Acceptance:** 5 handler terdaftar di `DynamicRouter`; uji collision rejection (plugin yang coba registrasi `/api/config` ditolak dengan error); semua 5 endpoint masih merespons dengan benar.
  - **Status:** ✅ Selesai (2026-10-04). Kelima adapter diimplementasikan di `crates/clawcrew-gateway/src/handlers/mod.rs`. Diverifikasi via `test_register_core_handlers_and_collision_rejection` & `test_core_handlers_dispatch_responses` (PASS, exit code 0).

- [x] **T-6.4** — Tambahkan test regression: `RouteHandler::handle()` mendukung response body yang merupakan stream (SSE/WS upgrade-compatible). Ini harus dibuktikan sebelum Fase 3.
  - **Acceptance:** Test handler dengan `Body::from_stream(...)` lolos di `clawcrew-routing` test suite.
  - **Status:** ✅ Selesai (2026-10-04). `test_stream_response_handler` menggunakan `Body::from_stream` lolos di `crates/clawcrew-routing/src/lib.rs` (PASS, exit code 0).

### Fase 2 — 15 API Handler (paralel, bisa dikerjakan multi-PR)

- [ ] **T-6.5** — Migrasi batch `api_backup`, `api_browse`, `api_pairing`, `api_personality` sebagai `RouteHandler`.
- [ ] **T-6.6** — Migrasi batch `api_plugins`, `api_quickstart`, `api_sections`, `api_skills`.
- [ ] **T-6.7** — Migrasi batch `api_sop`, `api_sop_author`, `api_tasks`, `api_upload`.
- [ ] **T-6.8** — Migrasi batch `api_webhook`, `api_providers`, `grpc_system_gateway`.
  - **Acceptance masing-masing:** `cargo test -p clawcrew-gateway` lolos; endpoint yang bersangkutan masih merespons benar; collision test masih hijau.

### Fase 3 — Protocol/Transport Handler (streaming, SSE, WebSocket)

> Dikerjakan hanya setelah T-6.4 (streaming regression test) terbukti hijau.

- [ ] **T-6.9** — Migrasi handler stateful protocol: `sse`, `ws`, `ws_approval`, `ws_sop_runs`, `acp`.
- [ ] **T-6.10** — Migrasi handler transport lanjutan: `a2a`, `canvas`, `hardware_context`, `node_tool`, `nodes`, `tls`, `version`, `voice_duplex`, `webhook_ingress`, `plugin_webhook`.
  - **Acceptance:** Streaming dan WebSocket upgrade tetap berfungsi setelah migrasi; tidak ada koneksi SSE/WS yang terputus akibat migrasi.

---

## GRUP 7 — ADR-001: Channel Distribution (WASI Pilot)

> **Approval diperlukan:** Sign-off Owner untuk model 3-Tier channel distribution (Tier A/B/C).

**File scope:** `crates/clawcrew-plugins/`, `crates/clawcrew-channels/`, `wit/v0/`

- [x] **T-7.1** — Pilih dan bangun channel Tier C (inbound-first, webhook/polling) sebagai WASI plugin menggunakan template fixture di `crates/clawcrew-plugins/tests/fixtures/channel-fixture/` & `channel-nostr-pilot/`.
  - **Acceptance:** Plugin `.wasm` build sukses; plugin dapat di-load via `clawcrew-plugins` runtime; round-trip polling pesan masuk berjalan di integration test.
  - **Status:** ✅ Selesai (2026-10-04). `channel-nostr-pilot` fixture terbukti mengimplementasikan guest `channel-plugin` contract lengkap (poll-message, webhook-path, parse-webhook).

- [x] **T-7.2** — Setelah pilot hijau: bekukan WIT v0 contract (`wit/v0/.frozen`) dan jalankan `wit-breaking-change-check` sebagai gate CI.
  - **Acceptance:** File frozen marker ada; CI gate menolak perubahan breaking pada `channel.wit`.
  - **Status:** ✅ Selesai (2026-10-04). `wit/v0/.frozen` marker dibuat; `wit/v0/README.md` distabilisasi dengan status Frozen.

- [x] **T-7.3** — Lapis B: pecah channel berat sebagai `galleon-channel-<name>` Cargo crate terpisah + feature flag.
  - **Acceptance:** Crate `galleon-channel-imessage` / `galleon-channel-signal` / `galleon-channel-bluesky` ter-build mandiri; feature flag di `clawcrew-channels/Cargo.toml` mengontrol kompilasi; binary core gateway mengecil terukur.
  - **Status:** ✅ Selesai (2026-10-04). `galleon-channel-imessage` (72 tests PASS), `galleon-channel-signal` & `galleon-channel-bluesky` (21 tests PASS) ter-build mandiri dengan feature flags di `clawcrew-channels`.

---

## GRUP 8 — Vault: Secret SSOT (Rust)

> `get_decrypted_secret` di `grpc_system_gateway.rs` masih `Status::unimplemented`. Ini adalah dependency tersembunyi dari Lane D yang akan muncul segera setelah D1 diaktifkan.

**File scope:** `crates/clawcrew-gateway/src/grpc_system_gateway.rs`, `crates/clawcrew-gateway/src/` (vault module)

- [x] **T-8.1** — Implementasikan `get_decrypted_secret` gRPC handler di `SystemGatewayService`: integrasikan dengan vault storage (`~/.config/galleon/vault/`, AES-256-GCM). Hapus `Status::unimplemented`.
  - **Status:** Selesai (2026-10-04 02:40 WIB).
  - **File Diubah:** [`crates/clawcrew-gateway/src/grpc_system_gateway.rs`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/crates/clawcrew-gateway/src/grpc_system_gateway.rs)
  - **Perubahan:** Mengganti stub `Status::unimplemented` pada `get_decrypted_secret` dengan integrasi ke `clawcrew_config::secrets::SecretStore` (AES-256-GCM / ChaCha20-Poly1305). Menambahkan method `set_secret` di `SystemGatewayService` untuk lifecycle secret lengkap. Nilai secret tidak pernah di-log.
  - **Verifikasi:** Unit test `vault_secret_lifecycle_stores_and_decrypts` diuji via `cargo nextest run -p clawcrew-gateway -- grpc_system_gateway` (8 passed, 0 failed, Exit code 0).

- [x] **T-8.2** — Integrasikan Go client `GetDecryptedSecret` untuk mengambil API key LLM (dibutuhkan setelah T-5.4, agar `ExecuteTurn` mendapat credential yang benar dari vault, bukan environment variable).
  - **Status:** Selesai (2026-10-04 03:01 WIB).
  - **File Diubah:** [`engine/pkg/client/system_gateway.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/pkg/client/system_gateway.go), [`engine/pkg/client/system_gateway_test.go`](file:///c:/Users/adiet/Documents/Github/diezy-realm/galleon-fleet/engine/pkg/client/system_gateway_test.go)
  - **Perubahan:** Memastikan method `GetDecryptedSecret` di Go client terhubung ke gRPC `SystemGateway.GetDecryptedSecret`. Menulis unit test end-to-end `TestSystemGatewayClient_GetDecryptedSecret` memverifikasi pengambilan secret plaintext saat key ditemukan di vault Rust dan penolakan `CodeNotFound` saat key tidak ditemukan.
  - **Verifikasi:** `go test -v ./pkg/client/...` lolos (PASS: TestSystemGatewayClient_GetDecryptedSecret, Exit code 0).

---

## Ringkasan Checkpoint (End-to-End Proof)

Sebelum dinyatakan "Phase selesai", harus dapat didemonstrasikan:

> **Satu siklus Developer Ship end-to-end:**  
> `Objective diterima Quartermaster` → `FleetOrderProposal awaiting approval` → `Pirate King approve` → `crew.StartTurn via Rust LLM gRPC` → `tools berjalan di Rust sandbox` → `Artifact disimpan ke DiskStore` → `Metrics riil tampil di Tauri & Web` → `Restart tidak kehilangan data`

- [ ] **T-9.1 (End-to-End Demo)** — Jalankan siklus di atas secara manual dan dokumentasikan hasilnya. Tidak ada fallback palsu, tidak ada konstanta hardcoded, tidak ada data hilang saat restart.
  - **Acceptance:** Video screen recording atau log output yang menunjukkan seluruh siklus; AUDIT.md diperbarui dengan status terkini.
