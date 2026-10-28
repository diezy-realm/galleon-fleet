# 04 — Drift Register, Pelanggaran Boundary Aktif, Kebersihan Repo

> Diverifikasi terhadap kode 2026-10-04. ADR tidak diedit; koreksi dicatat di sini dan lewat errata.

## 1. Drift dokumen ↔ kode

| # | Dokumen | Klaim | Fakta kode | Aksi |
|---|---|---|---|---|
| D1 | `finalize/analysis-conclusion.md` §2.3, `finalize/README.md` | ADR 0001: `ExecuteTurn` belum ada; Go `engine/src/llm/` aktif; "belum satu baris pun" | Proto punya `ExecuteTurn`; `execute_turn` Rust nyata; `engine/src/llm` **tidak ada**; `crew`/`orchestrator` memakainya | ADR 0001 **sebagian besar terimplementasi**; sisa: e2e + hapus probe Ollama di `fleet/services.go` |
| D2 | `analysis-conclusion.md` Lane D | `get_decrypted_secret` = stub | Membaca vault, valid. **Stub = `execute_task`** | Koreksi (Phase 5 diperbarui) |
| D3 | `00-overview.md`, `go-conventions.md` | Persistensi Go SQLite/Postgres; vektor ChromaDB/Qdrant | `engine/go.mod` tanpa driver SQL; `persistence` = DiskStore JSON/JSONL | Ratifikasi JSON-disk atau rencanakan migrasi |
| D4 | `00-overview.md` | `engine/src/tools/`, `ws.rs`, `vault.rs` | Package Go bernama `tool`; file Rust **[belum diverifikasi]** | Verifikasi nama |
| D5 | `00-overview.md` | "25 crates", Go 1.25+ | 27 crate; `go 1.27.1` | Diselaraskan di PR-20 (27 crate, Go 1.27+) |
| D6 | `rust-conventions.md` | MSRV 1.82+ | `rust-version = 1.96.0` | Diselaraskan di PR-20 (MSRV 1.96.0, 27 crate) |
| D7 | `ADR-001` | WIT unstable; `bluesky` Tier C | `.frozen` ada; `galleon-channel-bluesky` native | Errata |
| D8 | `analysis-conclusion.md` | `galleon-routing` orphan | Sudah dependency gateway (belum commit) | Perbarui setelah commit |
| D9 | `finalize/README.md` | Phase 4 di `decisions/phase-4/` | `decisions/refactoring-rust/phase-4/` | Dikoreksi |
| D10 | `tech-standart/README.md` | `design-system/`, `grpc-agent-engine.md`, `03-c4-diagram.puml`, `decisions/000x` | Tidak ada / ADR di lokasi lain | Struktur diperbarui |
| D11 | `00-overview.md` | Tauri "OS-shell only" | `apps/tauri` punya `execute_terminal_command`, koleksi lokal, metrik proses | Lihat V2 |
| D12 | `.kiro/steering/*` | SSOT di `docs/finalize/*`, `docs/fundamental/*` | Tidak ada; penerus `standard-docs/decisions/finalize/` | Diselaraskan di PR-20 (repoint ke `standard-docs/`) |
| D13 | `rust-decomposition-plan.md` | Tracker 2026-10-01 | Stale | Sinkronkan tiap sesi |
| D14 | Dokumen lama yang menyebut `web/` / `apps/tauri` sebagai UI/desktop | — | UI/desktop aktif = `web/` / `apps/tauri` (ODR-01); alias `web-2`/`tauri-2` usang | Catatan alias (02) |
| D15 | `tech-standart/architecture/00-overview.md` | "25 crates" (duplikat klaim D5, lokasi berbeda) | 27 crate (`crates/` dihitung langsung) | Selaraskan — lihat juga D5 |
| D16 | ODR-01 (`decisions/owner-decisions/ODR-2026-10-04-product-direction.md`) | "`web-2/` dan `apps/tauri-2/` dihapus (kode sudah dipindah)" | Verifikasi 2026-10-04: `web-2/` masih 10.052 file; `apps/tauri/` baru 40 file vs `web/` 16.279 file — migrasi **belum selesai**, bukan sudah dihapus | Perbarui status ODR atau tandai sebagai target, bukan fakta selesai |

## 2. Pelanggaran boundary aktif

| # | Pelanggaran | Bukti | Aturan | Status |
|---|---|---|---|---|
| V1 | Express + QR + telemetri OS di UI | `web/server.ts`; deps `express`, `qrcode`, `better-sqlite3`, `@google/genai` | T4 zero backend logic | ADR 0002 diterima sementara; **`better-sqlite3` & `@google/genai` di UI tidak tercakup ADR** — periksa pemakaian (potensi panggilan LLM langsung dari UI, melanggar ADR 0001/0002) |
| V2 | Shell Tauri berlogika | `apps/tauri/src/main.rs`: `execute_terminal_command`, `get_engine_processes`, `get_collection/save_collection`, `get_system_metrics` (konstanta palsu) | T1 no business logic, no fake data | ADR 0003 sebagian. `execute_terminal_command` **melompati sandbox Rust & policy Go** → keamanan P0. **Catatan arah (diskusi 2026-10-04, lihat `research/agent-subagent-permission-scoping-survey.md`):** produk Galleon Local-first — terminal lokal via Tauri `shell` plugin **tetap relevan** (bukan untuk dihapus), tapi harus lewat Go sebagai gate (ODR-07) dan `fs` plugin harus di-scope default ke workspace-dir yang user buka, bukan `fs:default` (HOME penuh). Flow desain saat ini: tahap awal masih mock/`capability` luas (`fs:default`, `shell:allow-open`) sebagai hasil alur desain→scoping-API yang belum selesai ke tahap akhir — bukan keputusan final. Semua kompetitor yang disurvei (Kiro Crew, Claude Code, Copilot CLI, Antigravity) default-kan scope filesystem ke working directory, konsisten dengan arah ini. |
| V3 | Go mengeksekusi shell/fs langsung | `engine/src/tool/{services,sandbox,plugin,mcp_client,builtin_workspace}.go` | T3 delegasi ke Rust; Owner: fail-closed, tanpa fallback lokal | Lane D1 belum selesai |
| V4 | Go memanggil vendor | `engine/src/fleet/services.go` (probe HTTP Ollama, label vendor) | T3 tanpa panggilan vendor | Sisa ADR 0001 |
| V5 | `execute_task` Rust stub | `grpc_system_gateway.rs` | Kontrak proto | Implementasi atau cabut |
| V6 | Channel → runtime | `clawcrew-channels` → `clawcrew-runtime` (677 ref) | Arah dependency | Phase 5 W1 |
| V7 | Komentar DI menyebut fallback, klien menolak | `wire_gen.go` vs `system_gateway.go` | Keputusan Owner #3 | Hapus komentar/jalur fallback |
| V8 | Tool contract Go↔Rust tidak sinkron | `engine/src/crew/dispatcher.go` mendaftarkan 8 tool (`execute_command`, `read_file`, `write_file`, `spawn_subagent`, `workspace.list_files`, `workspace.search_code`, `web.fetch`, `code.run_linter`) ke LLM; `crates/clawcrew-gateway/src/grpc_system_gateway.rs:14` `ALLOWED_TOOLS` hanya `["bash","read_file","write_file","git"]`. Nama dikirim mentah tanpa mapping | Kontrak proto/tool harus konsisten dua sisi | Fail-closed bekerja (aman), tapi 5 dari 8 tool (`execute_command`, `workspace.*`, `web.fetch`, `code.run_linter`) **selalu ditolak** Rust — end-to-end rusak untuk mayoritas kapabilitas yang dijanjikan ke agent. Aksi: satukan daftar tool + skema arg di satu kontrak (`api-contracts/native-tools.md`), tambah test penolakan dua sisi. Lihat `research/agent-subagent-permission-scoping-survey.md` §3 — pola insiden Antigravity (tool "native" bypass gate) jadi alasan tambahan untuk mengaudit SEMUA jalur tool |
| V9 | CSP Tauri terlalu luas untuk tujuan yang (mungkin) sah | `apps/tauri/tauri.conf.json`: `script-src 'self' 'unsafe-eval' 'unsafe-inline'`; `connect-src 'self' https: ipc: ws:` (wildcard HTTPS) | OWASP: no `unsafe-eval`; `connect-src` harus scoped | **Bukan vulnerability murni** — `web/src/components/features/RemoteAccessModal.tsx` mendukung custom domain/Cloudflare Tunnel/Tailscale untuk fitur Remote Access (QR pairing), jadi `https:` wildcard kemungkinan disengaja untuk use-case itu. Aksi: ganti CSP statis wildcard dengan origin-scoping dinamis sesuai domain yang user set di Remote Access settings (runtime config), bukan hardcoded build-time. `unsafe-eval` tetap harus dihapus — tidak ada use-case sah yang butuh eval() |

## 3. Kebersihan repo

| Item | Temuan | Rekomendasi |
|---|---|---|
| `rebrand.py`, `rebrand_icons.py`, `create_logo.py`, `clean_docs.py`, `update_docs.py` | Utilitas sekali-pakai di root | Pindah `scripts/migrations/` atau hapus |
| `PR_PHASE2B_TEMPLATE.md`, `CHANGELOG_FIXES.md`, `base_logo.png` | Artefak proses/desain di root | Pindah ke `docs/` / `assets/` |
| `.pr-evidence/` (40) | Screenshot bukti PR dalam repo | Simpan di PR/storage eksternal |
| `docs/refactoring/` | Kosong | Hapus / arahkan ke `standard-docs/` |
| `docs/` | Tanpa indeks otoritas | Tambah `docs/README.md`: otoritas = `standard-docs/` |
| `.gemini .kiro .claude .agents .copilot` | 5 set steering tumpang tindih | `standard-docs/` sebagai sumber tunggal; steering hanya menunjuk |
| `web/`, `apps/tauri` | Warisan di jalur utama repo | Putuskan freeze/extract (02 §1) |

## 4. Backlog fondasi

| Prioritas | Item |
|---|---|
| P0 | V2: hapus/ganti `execute_terminal_command` (jalur Go→Rust ber-policy); V2 metrik palsu (ADR 0003) |
| P0 | V3/V7: selesaikan Lane D1 fail-closed |
| P0 | V1: audit pemakaian `@google/genai` & `better-sqlite3` di `web/` |
| P0 | Phase 5 W0: lint unwrap, gerbang ukuran, gerbang arah dependency |
| P0 | V8: satukan kontrak tool Go↔Rust (8 tool Go vs 4 `ALLOWED_TOOLS` Rust, hanya 2 cocok) — blocker fungsional nyata, bukan risiko dokumentasi |
| P0 | V9: hapus `unsafe-eval` dari CSP Tauri; ganti `connect-src https:` wildcard dengan origin-scoping dinamis sesuai Remote Access settings |
| P1 | Pagar CI untuk `web/` & `apps/tauri` (lint/typecheck/build/test) + gerbang boundary tier di `tests/architecture/` |
| P1 | Jalur rilis Galleon (engine + gateway + tauri sidecar); ADR nasib warisan |
| P1 | D3 persistensi; D1/D2/D5/D6/D12/D14 sinkronisasi dokumen |
| P2 | Kebersihan §3; V5 `execute_task` |

## 5. Aturan menjaga register

- Tambah baris saat menemukan kontradiksi; tutup (✅ + commit) saat disinkronkan.
- Tinjau tiap akhir fase; angka kuantitatif selalu bertanggal.
- Jangan menyalin klaim dari dokumen lain tanpa verifikasi ke kode (D1/D2 lahir dari klaim basi).
