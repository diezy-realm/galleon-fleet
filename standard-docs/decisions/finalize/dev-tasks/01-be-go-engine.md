# 01 — BE-Go (Engine, `engine/`, :9090)

Konvensi: `tech-stack/go-conventions.md`. Gerbang: GW-2 (+GW-4/8 untuk keamanan). Test: `go test -race ./...`.

## A. Fail-closed & Boundary (P0 — kerjakan pertama)

- [x] **BG-A1 · G-01 · PR-05 (P0, M) Hapus sisa eksekusi shell/fs lokal.** [Selesai PR-05] Body eksekusi lokal pada `WriteFileTool`, `EditFileTool`, `ExecuteCommandTool`, `GitDiffTool`, dan `SandboxedCodeRunnerTool` telah diganti dengan routing fail-closed `executeViaGateway` ke `SystemGateway`. File `plugin.go` dan `mcp_client.go` telah dihapus. `grep os/exec engine/src` kini 0 (nol). Test fail-closed dan direct dispatch denial pass 100%.
- [ ] **BG-A1b · G-29 · PR-100 (P0, L) ⚑ Kontrak native tool Go↔Rust (SSOT tunggal).** Rust `ALLOWED_TOOLS=[bash,read_file,write_file,git]` (`grpc_system_gateway.rs:14`) dengan arg `command/args/path/contents`, sedangkan Go meregistrasi 19 tool (`workspace.*`, `execute_command`, `git_diff`, `git.*`, `code.*`, `sandbox.run_code`, `web.fetch`, `browser.*`, `edit_file`, ...). Hanya `write_file` cocok nama, dan beda kunci (`content` vs `contents`). Hasilnya aman (fail-closed) tetapi hampir semua tool Go gagal end-to-end. Workspace root dikirim Go lewat envelope args, sedangkan Rust membaca `req.workspace_path` (tidak diisi `ExecuteNativeTool(ctx,name,argsJSON)`). *Aksi:* satu daftar tool + skema argumen di `proto/` (atau `api-contracts/`), dipakai Go dan Rust; isi `workspace_path` di proto; test kontrak dua sisi. Dep: BG-A1. Pasangan Rust: BR-A*.
- [x] **BG-A2 · G-03 · PR-05 (P1, S) Koreksi komentar/fallback `wire_gen.go`** [Selesai PR-05] Komentar di `engine/app/wire_gen.go` dan `engine/src/tool/services.go` telah diperbarui menegaskan batas fail-closed: gateway nil/unavailable langsung menolak eksekusi (bukan fallback in-process).
- [x] **BG-A3 · G-05 · PR-11 (P1, M) Test arsitektur Go:** [Selesai PR-11] Diimplementasikan di `tests/architecture/tier_boundaries.rs` (`test_go_engine_tier_boundaries`), memvalidasi bahwa `engine/src/` tidak mengimpor `os/exec` dan SDK vendor LLM langsung per doktrin fail-closed dan GW-4. Dep: BG-A1.
- [x] **BG-A4 · G-04 · PR-15 (P0, L) Endpoint terminal ber-policy** [Selesai PR-15] Endpoint `POST /api/terminal/exec` dan alias legacy `POST /api/engine/execute` diimplementasikan di `engine/src/fleet/` (ODR-07): klasifikasi RiskTier (READ, EXECUTE, HIGH_RISK, BLOCKED), penolakan injeksi/bypass langsung (fork bomb, format disk, null byte), approval gate untuk perintah berisiko tinggi (rm -rf, del, git reset --hard, curl|sh), fail-closed bila gateway nil/unavailable, routing ke Rust SystemGateway `bash`, serta pencatatan audit ledger `TerminalAuditLedger` (`GET /api/terminal/audit`). Unit test komprehensif pass 100%. Dep: BG-A1.
- [x] **BG-A5 · G-02 · PR-16 (P1, S) Hapus route Ollama palsu** [Selesai PR-16] Route `/api/providers/ollama/status` dan handler dihapus dari `fleet/delivery.go`; unit test dihapus dari `delivery_test.go`; probe HTTP keluar `checkOllama` (port 11434) dihapus dari `fleet/services.go`; diagnostik `d-3` didelegasikan ke System Gateway (:50052) status; `apiClient.getOllamaStatus` dan UI test dihapus dari `web/`.
- [x] **BG-A6 · G-38 · PR-05 (P1, S) Ganti path `filepath.Join("..","web-2","data")`** [Selesai PR-05] Path diganti dengan fungsi helper `resolveDataDir` (`GALLEON_DATA_DIR` / `FLEET_DATA_DIR` / default `engine/data`). Sanitasi `safeFilenameRegex` dijaga dengan fallback `invalid`, dan test path traversal `TestCollectionPathTraversalSanitization` ditambahkan ke `delivery_test.go` (pass 100%).

## B. Persistensi (SQLite/Postgres, ODR-05)

- [x] **BG-B1 · G-10 · PR-40 (P1, M) ADR desain store:** [Selesai PR-40] Dibuat `standard-docs/decisions/finalize/ADR-005-persistence-store-design.md` menetapkan interface `Store` per agregat, dialek SQLite (pure-Go WAL) & Postgres (pgvector), alur migrasi idempoten DiskStore, enkripsi at-rest, isolasi multi-scope RAG, dan backup online.
- [x] **BG-B2 · G-11 · PR-41 (P1, L) Store SQLite:** [Selesai PR-41] Diimplementasikan di `engine/pkg/store/sqlite` menggunakan `modernc.org/sqlite` (pure-Go, CGO-free, WAL mode) dengan implementasi `RunStore` (`run.Store`, `run.ResumableStore`) dan `TaskStore` (`task.Store`), serta migrator idempoten dari DiskStore legacy. Unit test 100% pass.
- [ ] **BG-B3 · G-12 · PR-42 (P2, L) Store PostgreSQL** + test container. Dep: BG-B1.
- [ ] **BG-B4 · G-13 · PR-43 (P1, M) Memori vektor/RAG** per scope Fleet/Ship/Squad/Crew/Run. Dep: BG-B2.
- [ ] **BG-B5 · G-14 · PR-43 (P2, M) Retensi, ekspor, hapus (privasi)** per scope. Dep: BG-B2.

## C. Identitas, Pairing, Akses (QR sudah ada, otorisasi belum)

- [ ] **BG-C1 · G-35 · PR-44 (P1, L) Identity & akses di Go:** principal resolver (deny-by-default), WebAuthn, pairing. *Acceptance:* tanpa kredensial → ditolak; test negatif. Ref `R-00a`.
- [ ] **BG-C2 · G-20 · PR-45 (P1, L) Pairing perangkat via QR berotorisasi:** token sekali pakai terikat perangkat, expiry, daftar, revoke, rate-limit; `/api/pairing/*`. (`/api/network/qrcode` saat ini hanya render QR tanpa token.) Dep: BG-C1.
- [ ] **BG-C3 · G-21 · PR-45 (P1, M) Autentikasi sesi/perangkat** (middleware) untuk akses remote. Dep: BG-C2.
- [x] **BG-C4a · G-22 (sebagian) · T-2.1/T-2.2 Endpoint `/api/network/qrcode` & `/api/system/metrics` di Go** (`fleet/delivery.go`, `ReadMemStats`).
- [ ] **BG-C4b · G-22 · PR-46 (P1, M) Metrik OS nyata** (CPU/mem/proses host) via Rust `process_stats`, bukan hanya metrik proses Go.

## D. Orkestrasi — Otak Agent di Go (strangler dari `clawcrew-runtime`, `R-00`)

- [ ] **BG-D1 · G-30 · PR-47 (P1, L) Agent loop/turn:** tool-call loop, kompaksi, batas iterasi di atas `ExecuteTurn` Rust. *Acceptance:* paritas dengan `eval` replay.
- [ ] **BG-D2 · G-31 · PR-48 (P1, L) SOP engine** + approval gate. Dep: BG-D1.
- [ ] **BG-D3 · G-32 · PR-49 (P2, M) Skills & scheduler (cron)** terikat Squad/Crew. Dep: BG-D1.
- [ ] **BG-D4 · G-33 · PR-49 (P2, M) Sub-agent / handoff / A2A** sebagai Artifact reference. Dep: BG-D1.
- [ ] **BG-D5 · G-34 · PR-50 (P2, M) Padanan `control_plane` & `approval`;** cek duplikasi dengan `fleet/policies.go` (policy tetap SSOT di sana).

## E. Protokol Klien & CLI

- [ ] **BG-E1 · G-36 · PR-51 (P1, M) Spesifikasi Galleon Client Protocol v1** `api-contracts/client-protocol.md` (dari `rpc/types.rs`+`acp.rs`). Perbarui `finalize/route-map.md` bila rute berubah.
- [ ] **BG-E2 · G-36/G-27 · PR-52 (P1, L) Server Client Protocol** (WS JSON-RPC 2.0) + streaming stabil untuk CLI tipis; test kontrak. Dep: BG-E1, BG-C1, BG-D1.

## F. Organisasi AI, Pembelajaran, Observabilitas

- [ ] **BG-F1 · G-23 · PR-53 (P1, L) Model organisasi persisten** Quartermaster→Ship→Squad→Crew. Dep: BG-B2.
- [ ] **BG-F2 · G-25 · PR-53 (P2, M) Kontrak rute QM two-layer** + test.
- [ ] **BG-F3 · G-26 · PR-54 (P2, M) Audit trail & policy SSOT** read-only ke UI/Tauri.
- [ ] **BG-F4 · G-24 · PR-96 (P2, L) Learning by consent lanjutan:** tipe `MemoryProposal` sudah ada (T-3.2 ✔); tinggal alur umpan balik → persetujuan Owner → versi/rollback di atas store baru. Dep: BG-B4.
- [ ] **BG-F5 · G-28 · PR-96 (P3, M) Observabilitas:** zap + request-id Go→Rust; scrubbing.
- [ ] **BG-F6 · G-37 · PR-99 (P3, S) ADR opsional Delegated Authority Chain** (rujukan `research/verifiable-intent-assessment.md`).

## Selesai (arsip, terbukti di kode)

- [x] T-2.1/2.2 QR & metrics di Go · [x] T-3.2 `MemoryProposal` · [x] T-4.1..4.3 builtin_* fail-closed + round-trip + path traversal · [x] T-5.4..5.6 LLM Go dihapus, delegasi `ExecuteTurn` · [x] T-8.2 vault secret via gRPC

## Definition of Done (BE-Go)

- [ ] `go vet`, `go test -race ./...`, `wire` bersih · [ ] test fail-closed & negatif keamanan · [ ] kontrak (`proto/`, `api-contracts/`) sinkron · [ ] rollback tertulis di PR
