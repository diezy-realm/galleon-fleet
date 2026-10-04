# Lane G — Go Engine (otak tunggal, security layer)

Ref: `finalize/task-breakdown.md` GRUP 1–5, `tech-standart/tech-stack/go-conventions.md`. Test: `go test -race ./...` di `engine/`.

## G-0x Fail-closed & boundary

- [ ] **G-01 (P0, M) Hapus eksekusi shell/fs lokal** di `engine/src/tool/{services,sandbox,plugin,mcp_client,builtin_workspace}.go`; semua lewat `ExecuteNativeTool` (Rust). Acceptance: grep `os/exec`/`os.Open*` di jalur tool = nol (kecuali store); test fail-closed saat Rust mati. *Status 2026-10-04 (diverifikasi): kode laten — `ExecuteWithContext` sudah fail-closed untuk tool `native`; `os/exec`/fs di `services.go`, `sandbox.go`, cabang fs `builtin_workspace.go` tak terjangkau; `plugin.go`/`mcp_client.go` tak digerbang tetapi tanpa pemanggil. Aksi = hapus body eksekusi lokal; `plugin.go` & `mcp_client.go` dihapus (Q-2, MCP dibuat ulang di Rust bila perlu).*
- [ ] **G-02 (P1, S) Hapus route Ollama palsu** `/api/providers/ollama/status` di `engine/src/fleet/delivery.go` (JSON statis, bukan probe; web tidak memanggil) dan diagnostik `d-3` yang tak berbasis sinyal (Q-3). Discovery nyata = RPC baru di Rust (tiket terpisah bila diminta).
- [ ] **G-03 (P1, S) Koreksi komentar/fallback di `wire_gen.go`** agar sesuai fail-closed.
- [ ] **G-04 (P0, L) Endpoint terminal ber-policy** `POST /api/terminal/exec`: policy risk-tier → approval → `ExecuteNativeTool` (sandbox). Audit log tiap eksekusi. Dep: G-01. Acceptance: perintah berisiko tinggi minta approval; injeksi tidak lolos allowlist.
- [ ] **G-05 (P1, M) Gerbang boundary otomatis** (test arsitektur Go: larang import `os/exec`, SDK vendor LLM). Dep: G-01.

## G-1x Persistensi (ODR-05)

- [ ] **G-10 (P1, M) ADR desain store:** interface `Store` per agregat, dialek SQLite & Postgres, migrasi (golang-migrate/atlas), strategi vektor (sqlite-vec/pgvector), enkripsi at-rest, backup. Output ADR baru.
- [ ] **G-11 (P1, L) Implementasi SQLite** (pure-Go driver, WAL) + migrasi dari DiskStore JSON/JSONL (satu arah, idempoten). Dep: G-10.
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
- [ ] **G-38 (P1, S) Ganti path stale `filepath.Join("..","web-2","data")`** di `engine/src/fleet/services.go` (±55, ±313) dengan direktori data eksplisit (config/env, default `engine/data`); test traversal nama koleksi (`safeFilenameRegex` sudah ada).

## G-3x Menyerap fungsi orkestrasi dari Rust (hasil R-00, ODR-11)

- [ ] **G-30 (P1, L) Agent loop/turn orchestration** lengkap di Go (tool-call loop, kompaksi, batas iterasi) di atas ExecuteTurn Rust. Dep: R-00.
- [ ] **G-31 (P1, L) SOP engine**: definisi, eksekusi, approval gate di Go. Dep: R-00.
- [ ] **G-32 (P2, M) Skills & scheduler (cron)** di Go terikat Squad/Crew. Dep: R-00.
- [ ] **G-34 (P2, M) Padanan `control_plane` & `approval`** di Go (fleet governance, approval gate) — cek duplikasi dengan `fleet/policies.go`. Dep: R-00.
- [ ] **G-35 (P1, L) Identity & akses di Go:** principal resolver (provider → identitas → grants, deny-by-default, generasi policy), WebAuthn, pairing perangkat; dasar G-20/G-21. Dep: R-00a.
- [ ] **G-36 (P1, L) Server Galleon Client Protocol v1** (WS JSON-RPC 2.0) + spesifikasi `api-contracts/client-protocol.md` dari `rpc/types.rs`+`acp.rs`; tes kontrak. Dep: G-35, G-30.
- [ ] **G-33 (P2, M) Sub-agent / handoff / A2A** sebagai Artifact reference antar Crew di Go. Dep: R-00.


- [ ] **G-37 (P3, S) ADR opsional: Delegated Authority Chain** (wewenang bertingkat Quartermaster→Crew yang ditandatangani, attenuasi, fail-closed) — rujukan 
esearch/verifiable-intent-assessment.md §3. Dep: G-35, G-23. Bukan komitmen implementasi.
