# 08 — Carryover dari `finalize/task-breakdown.md`

Hasil analisis 2026-10-04: status tiap task `T-*` dibandingkan dengan kode aktual (bukan hanya centang dokumen).

## A. Belum dikerjakan (masuk daftar kerja)

- [ ] **CO-01 · T-6.5 (P2, M) Migrasi batch `api_backup`, `api_browse`, `api_pairing`, `api_personality` → `RouteHandler`.** *Tertahan keputusan §C.* Catatan: `api_pairing` akan digantikan pairing Go (BG-C2).
- [ ] **CO-02 · T-6.6 (P2, M) Batch `api_plugins`, `api_quickstart`, `api_sections`, `api_skills`.** *Tertahan §C.* `api_skills` terkait modul `skills` → Go (R-00).
- [ ] **CO-03 · T-6.7 (P2, M) Batch `api_sop`, `api_sop_author`, `api_tasks`, `api_upload`.** *Tertahan §C.* `sop` → Go.
- [ ] **CO-04 · T-6.8 (P2, M) Batch `api_webhook`, `api_providers`, `grpc_system_gateway`.** *Tertahan §C.*
- [ ] **CO-05 · T-6.9 (P2, L) Handler protokol stateful:** `sse`, `ws`, `ws_approval`, `ws_sop_runs`, `acp`. Syarat: regresi streaming (T-6.4 ✔). `ws`/`sse` terkait penghapusan `rpc` (R-41).
- [ ] **CO-06 · T-6.10 (P2, L) Handler transport lanjutan:** `a2a`, `canvas`, `hardware_context`, `node_tool`, `nodes`, `tls`, `version`, `voice_duplex`, `webhook_ingress`, `plugin_webhook`, dst. *Tertahan §C.*
- [ ] **CO-07 · T-9.1 (P1, M) End-to-end demo manual:** UI/Tauri → Go → Rust: `StartTurn` stream, tool via sandbox (fail-closed), secret via vault, tanpa fallback/konstanta palsu. Simpan bukti di `finalize/e2e-proof.md`. Dikerjakan setelah BG-A1, FD-A1..A3.
- [ ] **CO-08 · Hapus artefak Phase 2 yang tertinggal:** `web/src/utils/seedData.ts` yatim (FW-A5); komentar/doc lama yang menyebut `web-2`.

## B. Sudah selesai (terbukti di kode, 2026-10-04) — perlu re-verify Principle (GV-12)

| Task | Bukti |
|---|---|
| T-0.1 Tauri metrics nyata | `apps/tauri/src/main.rs` `get_system_metrics` → `engine_get("system/metrics")` |
| T-1.1..1.3 tipe & de-mock FE | `fleetStore.ts` hydrate dari API; `seedData.ts` tidak diimpor |
| T-2.1..2.3 QR/metrics → Go | `fleet/delivery.go` mendaftarkan `/api/network/qrcode`, `/api/system/metrics`; `web/server.ts` tanpa qrcode |
| T-3.1 peta rute | `finalize/route-map.md` |
| T-3.2 `MemoryProposal` | (klaim task-breakdown; Principle verifikasi) |
| T-4.1..4.3 fail-closed tool | `tool/builtin_*.go` memanggil gateway `ExecuteNativeTool`. **Catatan:** `tool/services.go` dll. masih `exec` lokal → BG-A1 |
| T-5.1..5.6 LLM SSOT Rust | `llmProvider` hilang dari `crew/services.go`; `engine/src/llm/` tidak ada |
| T-6.1..6.4 routing | `clawcrew-routing` ada di `Cargo.toml` gateway |
| T-7.1..7.3 WASI pilot & crate | `channel-nostr-pilot`, `wit/v0/.frozen`, `galleon-channel-{core,imessage,signal,bluesky}` |
| T-8.1..8.2 vault secret | `get_decrypted_secret` terimplementasi (`grpc_system_gateway.rs`) |

## C. Keputusan terbuka (Owner) — dampak ke T-6.5..T-6.10

`finalize` merencanakan **memigrasi** semua handler REST/WS ke `RouteHandler`, sedangkan keputusan Owner kemudian (ODR-14, ODR-21..25) menetapkan REST/WS/A2A lama **dihapus** setelah parity, `rpc` dihapus terakhir, auth/skills/sop/cron pindah ke Go.
Rekomendasi (menunggu persetujuan): **jangan migrasi handler yang akan dihapus/dipindah**. Hanya handler yang bertahan sebagai host lokal Rust (mis. `webhook_ingress`, `plugin_webhook`, `tls`, `version`, `node_tool`/`nodes`, `hardware_context`) yang dimigrasi ke `RouteHandler`; sisanya masuk R-14/R-41/N-12.
- [ ] **CO-09 · Owner:** setujui/ubah rekomendasi di atas. Jika disetujui: tutup CO-01..CO-04 sebagai *superseded* dan pecah CO-05/CO-06 hanya untuk handler yang bertahan (daftar final dari verifikasi R-00 §4).
