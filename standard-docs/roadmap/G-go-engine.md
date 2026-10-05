# Lane G — Go Engine (otak tunggal, security layer)

Ref: `finalize/task-breakdown.md` GRUP 1–5, `tech-standart/tech-stack/go-conventions.md`. Test: `go test -race ./...` di `engine/`.

## G-0x Fail-closed & boundary

- [x] **G-01 (P0, M) Hapus eksekusi shell/fs lokal** [Selesai PR-05] Body eksekusi lokal pada `WriteFileTool`, `EditFileTool`, `ExecuteCommandTool`, `GitDiffTool`, dan `SandboxedCodeRunnerTool` telah diganti dengan fail-closed `executeViaGateway` ke Rust. `plugin.go` dan `mcp_client.go` dihapus. `grep os/exec engine/src` = 0.
- [x] **G-02 (P1, S) Hapus route Ollama palsu** [Selesai PR-16] Route `/api/providers/ollama/status` dan handler dihapus dari `delivery.go`; unit test dihapus; probe HTTP `checkOllama` (port 11434) dihapus dari `services.go`; diagnostik `d-3` didelegasikan ke System Gateway (:50052); method `getOllamaStatus` dan UI test dihapus dari `web/`.
- [x] **G-03 (P1, S) Koreksi komentar/fallback di `wire_gen.go`** [Selesai PR-05] Komentar di `wire_gen.go` dan `services.go` diperbarui menegaskan batasan fail-closed (nil gateway langsung menolak eksekusi).
- [x] **G-04 (P0, L) Endpoint terminal ber-policy** [Selesai PR-15] `POST /api/terminal/exec` dan alias `POST /api/engine/execute` diimplementasikan dengan risk-tier → approval → `ExecuteNativeTool` (Rust SystemGateway) → audit log (`/api/terminal/audit`). Injeksi ditolak, perintah destruktif minta approval; test lulus 100%. Dep: G-01.
- [x] **G-05 (P1, M) Gerbang boundary otomatis** [Selesai PR-11] Test arsitektur Go di `tests/architecture/tier_boundaries.rs` (`test_go_engine_tier_boundaries`) melarang import `os/exec` dan SDK vendor LLM di `engine/src/`; test lolos 100%. Dep: G-01.

## G-1x Persistensi (ODR-05)

- [x] **G-10 (P1, M) ADR desain store:** [Selesai PR-40] Diratifikasi di `standard-docs/decisions/finalize/ADR-005-persistence-store-design.md`: interface `Store` per agregat, dialek SQLite pure-Go WAL & Postgres pgvector, migrasi idempoten DiskStore, enkripsi at-rest, dan backup online.
- [x] **G-11 (P1, L) Implementasi SQLite:** [Selesai PR-41] Diimplementasikan di `engine/pkg/store/sqlite` (pure-Go WAL mode) dengan implementasi `RunStore` & `TaskStore` dan migrasi satu arah idempoten dari DiskStore legacy.
- [ ] **G-12 (P2, L) Implementasi PostgreSQL** + test dengan container. Dep: G-10.
- [ ] **G-13 (P1, M) Memori vektor/RAG** pada store baru; scope Fleet/Ship/Squad/Crew/Run. Dep: G-11.
- [ ] **G-14 (P2, M) Retensi, ekspor, hapus (privasi)** per scope. Dep: G-11.

## G-2x Organisasi, memori, pembelajaran

- [ ] **G-20 (P1, L) Pairing perangkat via QR:** buat token sekali pakai terikat perangkat, ekspirasi, daftar perangkat, revoke; endpoint `/api/pairing/*`. Acceptance: tanpa token valid, perangkat lain ditolak; rate-limit. (Menggantikan QR di `web/server.ts`.)
- [ ] **G-21 (P1, M) Autentikasi perangkat/sesi** untuk akses remote (middleware) — dasar Nginx/VPS. Dep: G-20.
- [ ] **G-22 (P1, M) Endpoint metrik sistem nyata** (proses, CPU/mem) — via Rust bila butuh OS; menggantikan `get_system_metrics` palsu. *Status 2026-10-04: endpoint `/api/system/metrics` & `/api/network/qrcode` di Go sudah ada; sisa metrik OS via Rust.*
- [ ] **G-23 (P1, L) Model organisasi:** Quartermaster→Ship→Squad→Crew sebagai agregat persisten + API CRUD + validasi kuota/budget. Ref GRUP 4.
- [ ] **G-24 (P2, L) Learning by consent (C5):** umpan balik → kandidat memori → persetujuan Owner → versi/rollback. Dep: G-13.
- [ ] **G-25 (P2, M) Kontrak rute QM two-layer (C4)** + test. Ref GRUP 3.
- [ ] **G-26 (P2, M) Audit trail & policy SSOT** (`fleet/policies.go`) diekspos read-only ke UI/Tauri.
- [ ] **G-27 (P2, M) CLI API:** endpoint streaming stabil untuk CLI tipis (A1). Dep: R-34.
- [ ] **G-28 (P3, M) Observabilitas:** zap terstruktur + request-id lintas Go→Rust; scrubbing.
- [ ] **G-29 (P0, L) Kontrak native tool Go↔Rust (SSOT tunggal).** Rust `ALLOWED_TOOLS=[bash,read_file,write_file,git]` (arg `command/args/path/contents`) vs 19 tool Go (`workspace.*`, `execute_command`, `git.*`, `code.*`, `sandbox.run_code`, `web.fetch`, `browser.*`, `edit_file`...); hanya `write_file` sama nama dan kunci argumennya beda (`content`/`contents`). Isi `workspace_path` di proto (kini tak diisi `ExecuteNativeTool`). Daftar tool + skema argumen di `proto/`, dipakai kedua sisi; test kontrak. Dep: G-01. Pasangan Rust: R-21 (perluas cakupan).
- [x] **G-38 (P1, S) Ganti path stale `filepath.Join("..","web-2","data")`** [Selesai PR-05] Diganti dengan `resolveDataDir` (`GALLEON_DATA_DIR` / default `engine/data`), test path traversal `TestCollectionPathTraversalSanitization` ditambahkan (pass 100%).

## G-3x Menyerap fungsi orkestrasi dari Rust (hasil R-00, ODR-11)

- [ ] **G-30 (P1, L) Agent loop/turn orchestration** lengkap di Go (tool-call loop, kompaksi, batas iterasi) di atas ExecuteTurn Rust. Dep: R-00.
- [ ] **G-31 (P1, L) SOP engine**: definisi, eksekusi, approval gate di Go. Dep: R-00.
- [ ] **G-32 (P2, M) Skills & scheduler (cron)** di Go terikat Squad/Crew. Dep: R-00.
- [ ] **G-34 (P2, M) Padanan `control_plane` & `approval`** di Go (fleet governance, approval gate) — cek duplikasi dengan `fleet/policies.go`. Dep: R-00.
- [ ] **G-35 (P1, L) Identity & akses di Go:** principal resolver (provider → identitas → grants, deny-by-default, generasi policy), WebAuthn, pairing perangkat; dasar G-20/G-21. Dep: R-00a.
- [ ] **G-36 (P1, L) Server Galleon Client Protocol v1** (WS JSON-RPC 2.0) + spesifikasi `api-contracts/client-protocol.md` dari `rpc/types.rs`+`acp.rs`; tes kontrak. Dep: G-35, G-30.
- [ ] **G-33 (P2, M) Sub-agent / handoff / A2A** sebagai Artifact reference antar Crew di Go. Dep: R-00.


- [ ] **G-37 (P2 — naik dari P3, DIUPGRADE jadi gate wajib, keputusan Owner 2026-10-05, S) ADR: Delegated Authority Chain** (wewenang bertingkat Quartermaster→Crew yang ditandatangani, attenuasi, fail-closed) — rujukan `research/verifiable-intent-assessment.md` §3,§3b,§3c. **Konteks baru**: Owner konfirmasi arah "bayar" di product definition §5 berpotensi meluas ke agentic commerce (agent beli dari merchant pihak ketiga). ADR ini **WAJIB ditinjau ulang SEBELUM desain fitur commerce pertama dimulai** — bukan lagi opsional P3. Harus merujuk AP2/VI (Google/Mastercard, donasi FIDO Mei 2026), IETF WIMSE delegation chains, IETF DAAP, dan invariant F1 (Treasury read-only, zero platform markup, larangan reseller kredit) sebagai pagar desain. Dep: G-35, G-23. Bukan komitmen implementasi DAC itu sendiri — tapi gate review wajib sebelum commerce.
