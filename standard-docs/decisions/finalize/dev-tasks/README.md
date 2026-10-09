# Dev Tasks — Task Breakdown Tim BE / FE (Galleon Fleet)

> Turunan operasional dari `roadmap/` (121 tiket) + sisa `finalize/task-breakdown.md` (T-6.5..T-6.10, T-9.1), dikelompokkan per **tim & kategori** agar bisa langsung dibagi ke developer.
> Kotak `[ ]` = belum; `[x]` = sudah terbukti di kode (verifikasi 2026-10-04, lihat `08-carryover-from-finalize.md` §B). Centang hanya setelah PR merge, gerbang `roadmap/PR-plan.md` §1 lulus, dan Principle approve.
> Sumber kebenaran tiket tetap `roadmap/*.md`. File di sini adalah **tampilan kerja**; bila ada selisih, `roadmap/` menang lalu perbaiki file ini.

## Peta Tim

| File | Tim | Kategori |
|---|---|---|
| [`01-be-go-engine.md`](01-be-go-engine.md) | **BE-Go** | Fail-closed, persistensi, identitas/pairing, orkestrasi, protokol klien, organisasi |
| [`02-be-rust-core.md`](02-be-rust-core.md) | **BE-Rust** | Kualitas, dekoplasi crate, kontrak gRPC, keamanan, plugin/channel, CLI |
| [`03-fe-web.md`](03-fe-web.md) | **FE-Web** | `web/` (React 19 + TS strict + Tailwind 4) |
| [`04-fe-desktop-tauri.md`](04-fe-desktop-tauri.md) | **FE-Desktop** | `apps/tauri/` (Tauri v2, shell OS saja) |
| [`05-platform-devops-qa.md`](05-platform-devops-qa.md) | **Platform/DevOps/QA** | CI, boundary gate, Docker, Nginx/VPS, rilis |
| [`06-rebrand-and-legacy-cleanup.md`](06-rebrand-and-legacy-cleanup.md) | **Lintas tim** | Rename ke Galleon, hapus legacy, WIT v1 |
| [`07-docs-and-governance.md`](07-docs-and-governance.md) | **Tech Lead/Docs** | Drift dokumen, ADR, hygiene |
| [`08-carryover-from-finalize.md`](08-carryover-from-finalize.md) | **Lintas tim** | Sisa task `finalize/` yang belum selesai + arsip yang sudah selesai + keputusan terbuka |

## Invarian yang tidak boleh dilanggar (semua tim)

1. Alur: `web/ | apps/tauri/ → Go :9090 → Rust gRPC :50052`. UI/Tauri tidak pernah memanggil Rust langsung.
2. Go = otak (orkestrasi, policy SSOT di `fleet/policies.go`, identitas/auth). Rust = System Core (LLM executor ADR 0001, sandbox, vault, tool native, channel, WASI, binary CLI).
3. Tidak ada shell/fs lokal di Go maupun Tauri; tidak ada vendor LLM di Go; rahasia hanya di Vault Rust.
4. Local-first; SQLite default, Postgres opsional (ODR-05). Nginx = reverse proxy + TLS saja.
5. Rust produksi: nol `unwrap()/expect()`; Go: wrap `%w`, zap; TS: strict, tanpa `any`.
6. Rebrand tanpa alias (ODR-19); WIT v0 beku (`wit/v0/.frozen`), v1 `galleon:plugin@1.0.0`.

## Legenda

`(P0..P3, S/M/L/XL)` = prioritas, ukuran. Format ID: `<tim>-<kategori><no> · tiket roadmap · PR`. `Dep` = prasyarat. ⚑ = butuh sign-off Owner tertulis.
Verifikasi cepat: Rust `cargo nextest run -p <crate>` + `cargo clippy -p <crate> -- -D warnings` (Windows: `$env:CARGO_TARGET_DIR="target/agent"`); Go `go vet ./... && go test -race ./...`; Web `npm run lint && npm run build`.

## Kondisi Terakhir (diverifikasi dari kode, 2026-10-04)

| Fakta | Dampak ke tugas |
|---|---|
| Go sudah punya `/api/network/qrcode` & `/api/system/metrics` (`fleet/delivery.go`) | Selesai (T-2.1/2.2); tersisa pairing berotorisasi (BG-C2) dan metrik OS via Rust |
| Go `tool/builtin_*.go` sudah mendelegasikan ke gateway, tapi `tool/{services,sandbox,plugin,mcp_client}.go` masih `os/exec` lokal | BG-A1 masih terbuka (parsial) |
| `delivery.go` masih memuat probe Ollama (baris 48, 399) | BG-A5 terbuka |
| `engine/src/llm/` sudah dihapus; `llmProvider` hilang dari `crew/services.go` | ADR 0001 sisi Go selesai |
| Tauri `get_system_metrics` sudah memanggil `engine_get`; `execute_terminal_command` masih **simulasi palsu**; `get_collection/save_collection` masih lokal | FD-A2/FD-A3b terbuka |
| `apps/tauri/tauri.conf.json` masih `frontendDist=../../web-2/dist`, CSP masih `unsafe-eval` & `connect-src https:` | FD-A1, FD-A4 terbuka |
| `web/server.ts` (Express) masih ada; QR sudah dihapus dari server, tapi `qrcode`/`better-sqlite3`/`@google/genai` masih di `package.json`; skrip `dev=tsx watch server.ts`; `lint=tsc --noEmit` | FW-A1/A2 terbuka |
| `galleon-i18n` sudah diekstrak, namun `clawcrew-channels` masih 87 ref ke `runtime::i18n` | BR-C1 sisa: repoint ref |
| `galleon-channel-{core,imessage,signal,bluesky}` ada; fixture pilot `channel-nostr-pilot` ada; `wit/v0/.frozen` ada | T-7.1..7.3 selesai; BR-F5 tinggal ukur `wasi:http`+e2e |
| `clawcrew-routing` sudah dipasang di gateway; 5 core handler migrasi selesai | T-6.5..6.10 bentrok dengan keputusan hapus REST/WS lama — lihat 08 §C |

## Urutan Kerja yang Disarankan

1. **Sprint 0 (P0 keamanan):** BG-A1, BG-A4; FD-A1..A3; PL-B1; GV-01.
2. **Sprint 1:** gerbang kualitas (PL-A*, BR-A*), FW-A1/A2/B1, pembersihan legacy (RB-10..).
3. **Sprint 2+:** persistensi & identitas (BE-Go), dekoplasi crate (BE-Rust), migrasi otak runtime → Go (strangler, `R-00`).
4. **Rename massal** (RB-2x) hanya setelah hapus/pecah selesai; **deploy/rilis** terakhir.
