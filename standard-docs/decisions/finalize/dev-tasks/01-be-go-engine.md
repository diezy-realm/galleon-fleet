# 01 — BE-Go (Engine, `engine/`, :9090)

Konvensi: `tech-stack/go-conventions.md`. Gerbang: GW-2 (+GW-4/8 untuk keamanan). Test: `go test -race ./...`.

## A. Fail-closed & Boundary (P0 — kerjakan pertama)

- [ ] **BG-A1 · G-01 · PR-05 (P0, M) Hapus sisa eksekusi shell/fs lokal.** *Status (diverifikasi 2026-10-04): kode laten, bukan jalur produksi.* `toolService.ExecuteWithContext` sudah fail-closed: tool `Source=="native"` ditolak bila `gateway==nil`, dan bila gateway ada dipanggil `ExecuteNativeTool` (Rust), bukan `t.ExecuteWithContext`. Jadi `os/exec`/fs lokal di `services.go` (`ExecuteCommandTool`, `GitDiffTool`, `WriteFileTool`, `EditFileTool`), `sandbox.go` dan cabang "else fs" di `builtin_workspace.go` **tidak terjangkau**, tetapi masih bisa bocor lewat `.Execute` langsung. `plugin.go` (`Source:"plugin"`) dan `mcp_client.go` (stdio `exec`) **tidak digerbang** namun **tanpa pemanggil produksi**. *Aksi:* hapus semua body eksekusi lokal (tool native tinggal definisi + skema; eksekusi hanya via gateway); `plugin.go`/`mcp_client.go` dihapus atau dipindah ke Rust plugin host/WASI (keputusan: lihat 08 §Pertanyaan). *Acceptance:* `grep os/exec engine/src` kosong kecuali store; test: gateway nil → ditolak; fs/exec tidak ada di binary.
- [ ] **BG-A1b · G-29 · PR-100 (P0, L) ⚑ Kontrak native tool Go↔Rust (SSOT tunggal).** Rust `ALLOWED_TOOLS=[bash,read_file,write_file,git]` (`grpc_system_gateway.rs:14`) dengan arg `command/args/path/contents`, sedangkan Go meregistrasi 19 tool (`workspace.*`, `execute_command`, `git_diff`, `git.*`, `code.*`, `sandbox.run_code`, `web.fetch`, `browser.*`, `edit_file`, ...). Hanya `write_file` cocok nama, dan beda kunci (`content` vs `contents`). Hasilnya aman (fail-closed) tetapi hampir semua tool Go gagal end-to-end. Workspace root dikirim Go lewat envelope args, sedangkan Rust membaca `req.workspace_path` (tidak diisi `ExecuteNativeTool(ctx,name,argsJSON)`). *Aksi:* satu daftar tool + skema argumen di `proto/` (atau `api-contracts/`), dipakai Go dan Rust; isi `workspace_path` di proto; test kontrak dua sisi. Dep: BG-A1. Pasangan Rust: BR-A*.
- [ ] **BG-A2 · G-03 · PR-05 (P1, S) Koreksi komentar/fallback `wire_gen.go`** agar sesuai fail-closed; regen Wire bersih.
- [ ] **BG-A3 · G-05 · PR-11 (P1, M) Test arsitektur Go:** larang import `os/exec` dan SDK vendor LLM di `engine/`. Dep: BG-A1.
- [ ] **BG-A4 · G-04 · PR-15 (P0, L) Endpoint terminal ber-policy** `POST /api/terminal/exec` (ODR-07): risk-tier → approval → `ExecuteNativeTool` → audit log. *Acceptance:* perintah berisiko tinggi minta approval; injeksi/allowlist-bypass ditolak (test negatif). Dep: BG-A1.
- [ ] **BG-A5 · G-02 · PR-16 (P1, S) Hapus route Ollama palsu** `/api/providers/ollama/status` (`fleet/delivery.go:48`, handler ±399): bukan probe, hanya JSON statis (`standby`, `available:true`). Web tidak memanggilnya. Hapus route + test; discovery nyata (jika diinginkan) lewat RPC baru di Rust provider. Juga `diagnostics` `d-3` "Ollama ping pending" di `fleet/services.go` harus dari sinyal nyata atau dibuang.
- [ ] **BG-A6 · G-30 · PR-05 (P1, S) Ganti path `filepath.Join("..","web-2","data")`** (`fleet/services.go:55,313`) dengan direktori data eksplisit (config/env, default `engine/data`). Pembersihan nama koleksi di Go sudah ada (`safeFilenameRegex`); tambah test traversal.

## B. Persistensi (SQLite/Postgres, ODR-05)

- [ ] **BG-B1 · G-10 · PR-40 (P1, M) ADR desain store:** interface `Store` per agregat, dialek SQLite/Postgres, migrasi, vektor (sqlite-vec/pgvector), enkripsi at-rest, backup.
- [ ] **BG-B2 · G-11 · PR-41 (P1, L) Store SQLite** (pure-Go, WAL) + migrasi satu arah idempoten dari DiskStore JSON/JSONL; rollback tertulis. Dep: BG-B1.
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
