# Tier Boundary Policy — v2 (Rust Single Language)

**Status:** Normative (code MUST follow this)
**Last updated:** 2026-10-29
**Supersedes:** v1 (2026-10-03, model 4-tier dengan Go Orchestrator)
**Dasar:** ODR-2026-10-29 (ODR-26…ODR-33), ADR-0018
**Enforcement:** PR review gate + `scripts/ci/check_no_go.sh` + test tabel rute di `galleon-gateway`

> Catatan alias (ODR-01, tetap berlaku): UI = `web/`, desktop = `apps/tauri/`. Rujukan `web-2/`/`apps/tauri-2/` adalah alias historis.

---

## 1. Purpose

Galleon Fleet memakai **3 tier**. Setiap fakta punya satu sumber otoritatif. Go **tidak** termasuk tier produk (lihat §6 untuk add-on).

## 2. Tier 1 — Desktop Shell (`apps/tauri/`, Rust)

**Owns:** window lifecycle, tray, dialog native, notifikasi, file picker, packaging, start/stop sidecar `galleon daemon`.

**Forbidden:** logika bisnis, handler API, persistensi di luar direktori Tauri, state crew, secret (gunakan gateway/vault), eksekusi terminal langsung.

**Communication:**
- Outbound: HTTP ke Rust gateway lokal `http://127.0.0.1:<gateway.port>` (default 42617) dengan bearer token pairing. Tidak ada `:9090`.
- Inbound: event Tauri untuk window lifecycle saja.

## 3. Tier 2 — System Core (`crates/*`, `src/`, Rust)

**Owns (satu-satunya backend):**
- Gateway HTTP/WS (`galleon-gateway`): semua API untuk Web, Desktop, CLI, QR/pairing.
- Agent loop, orkestrasi, SOP, subagent, cron, approval (`galleon-runtime`).
- LLM execution (ADR-0001), channel, plugin WASM, memory, security microkernel, vault.
- Fleet dashboard domain (`galleon-fleet-core`).
- Identitas, pairing, WebAuthn, DAC gate.

**Forbidden:**
- gRPC/protobuf internal antar crate Rust.
- Rute HTTP tanpa `require_auth` kecuali daftar publik §4.
- Logika bisnis di handler HTTP (handler = parse → auth → panggil crate domain → map error).
- Sukses palsu saat gagal.
- Crate leaf bergantung ke `galleon-runtime` atau `galleon-gateway`.

**Communication:** inbound HTTP/WS di gateway; antar-crate = panggilan fungsi.

## 4. Permukaan Publik Gateway (tanpa bearer)

| Rute | Alasan | Kontrol pengganti |
|---|---|---|
| `/health`, `/ready`, `/api/health` | probe | tidak membocorkan detail internal |
| `/pair`, `/api/pair` | penukaran kode pairing | rate limit bersama (`auth_rate_limit`), kode sekali pakai |
| Webhook channel (`/whatsapp*`, `/linq*`, `/nextcloud-talk*`, `/webhook/gmail`, `/sop/{*rest}`) | dipanggil pihak ketiga | verifikasi tanda tangan per channel |

Semua rute lain **wajib** 401 tanpa token yang sah — dibuktikan oleh test tabel rute.

## 5. Tier 3 — UI Layer (`web/`, TypeScript)

**Owns:** komponen React, routing klien, `apiClient.ts`, view model.

**Forbidden:** logika backend, generate QR, telemetri OS, secret, memanggil LLM langsung (ODR-08).

**Communication:** HTTP/WS ke Rust gateway (dev: proxy Vite `/api` → `GALLEON_GATEWAY_URL`, default `http://127.0.0.1:42617`). Produksi: disajikan oleh gateway (embedded `web/dist`) atau di belakang Nginx (ODR-15: reverse proxy + TLS saja).

## 6. Add-on Non-Core (termasuk Go)

- Hidup di **repository terpisah** di org `diezy-realm` (ODR-30).
- Integrasi hanya melalui: HTTP API gateway (token pairing/scoped), MCP, atau plugin WASM `galleon:plugin@1.0.0` (`wit/v1/`).
- Repository core **tidak** memuat kode, proto, SDK, workflow, atau dependensi build add-on.
- Kontrak yang dipakai add-on adalah kontrak publik yang sama dengan UI — tidak ada endpoint khusus add-on tanpa ADR.

## 7. Aturan Data Lintas Tier

1. **UI → Rust gateway** (tidak ada lapisan perantara bahasa lain).
2. **Persistensi milik Rust.** Tauri hanya menyimpan state window. Data dashboard fleet = `galleon-fleet-core` (file JSON atomik, 0600). Pairing = `DeviceRegistry` SQLite. Secret = vault Rust.
3. **Secret hanya di vault Rust**; tidak pernah dikirim ke UI kecuali nilai yang ditampilkan dengan izin eksplisit dan sudah di-redact.

## 8. Enforcement

- `scripts/ci/check_no_go.sh`: gagal bila ada `*.go`, `go.mod`, `go.sum`, `go.work` di repo (aktif penuh setelah Sprint 13; sebelum itu hanya melarang file Go **baru**).
- Test tabel rute (`galleon-gateway`): daftar rute terpasang = snapshot; rute non-publik → 401.
- Review PR: checklist §3 Forbidden.
