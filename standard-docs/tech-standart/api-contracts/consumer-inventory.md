# API Consumer Inventory & Route Migration Matrix

> **Status:** Ratified & Kanonikal  
> **Source Spec:** SPEC-012 (PRD-002 §5, BL-087)  
> **Target Release:** Galleon Fleet Single-Language Rust Architecture  
> **CWD Audit Date:** 2026-10-29  

Dokumen ini adalah Single Source of Truth (SSOT) inventaris konsumen API HTTP/WebSocket untuk seluruh rute di `galleon-fleet` (Go Engine `:9090`, Rust Gateway, dan versi OLD `db18a3745^:crates/clawcrew-gateway/src/lib.rs`). Inventaris ini memetakan setiap rute ke lokasi konsumen nyata (`file:line`), status implementasi saat ini, keputusan migrasi (`port`, `restore`, atau `drop`), serta skema otentikasi.

---

## 1. Inventaris Lengkap Konsumen & Keputusan Rute

Setiap baris mencantumkan referensi konsumen konkret (`file:line`) atau `'none'` bila tidak ditemukan konsumen di kode produksi.

| Method | Path | Konsumen (`file:line`) | Implementasi Sekarang | Keputusan | Auth |
|---|---|---|---|---|---|
| **Rute Admin (CLI Management)** | | | | | |
| POST | `/admin/shutdown` | `src/main.rs:8271` | none (terhapus di `db18a3745`) | restore | Localhost (ConnectInfo) |
| POST | `/admin/reload` | `src/main.rs:5192, 9614` | none (terhapus di `db18a3745`) | restore | Localhost (ConnectInfo) |
| GET | `/admin/sop/pending` | `src/main.rs:8328` | none (handler di `api_sop.rs:131`, unmounted) | restore | Localhost (ConnectInfo) |
| POST | `/admin/sop/approve` | `src/main.rs:8390` | none (handler di `api_sop.rs:160`, unmounted) | restore | Localhost (ConnectInfo) |
| POST | `/admin/sop/deny` | `src/main.rs:8394` | none (handler di `api_sop.rs:171`, unmounted) | restore | Localhost (ConnectInfo) |
| GET | `/admin/paircode` | `src/main.rs:8513` | none (terhapus di `db18a3745`) | restore | Localhost (ConnectInfo) |
| POST | `/admin/paircode/new` | `src/main.rs:8502` | none (terhapus di `db18a3745`) | restore | Localhost (ConnectInfo) |
| **Rute Pairing & Device Management** | | | | | |
| GET | `/pair/code` | `src/bin/clawcrew-acp-bridge.rs:287` | none (terhapus di `db18a3745`) | restore | Public (sebelum pairing pertama) |
| POST | `/pair` | `src/bin/clawcrew-acp-bridge.rs:291` | none (terhapus di `db18a3745`) | restore | Public + Rate Limit Pair |
| POST | `/api/pairing/initiate` | `web/src/components/features/ConnectedDevicesView.tsx:53` | Go (`engine/src/identity/pairing_delivery.go:27`), Rust handler unmounted (`api_pairing.rs:362`) | restore | Bearer |
| POST | `/api/pair` | `crates/galleon-gateway/src/api_pairing.rs:1141` (second device / enhanced) | Rust handler unmounted (`api_pairing.rs:388`) | restore | Public (one-time code) |
| GET | `/api/devices` | `web/src/components/features/ConnectedDevicesView.tsx:33` (target migrasi dari `/api/pairing/devices`) | Rust handler unmounted (`api_pairing.rs:533`) | restore | Bearer |
| PATCH | `/api/devices/me/capabilities` | `crates/galleon-gateway/src/api_pairing.rs:623` (node / peer) | Rust handler unmounted (`api_pairing.rs:623`) | restore | Bearer |
| DELETE | `/api/devices/{id}` | `web/src/components/features/ConnectedDevicesView.tsx:71` (target migrasi dari `/api/pairing/revoke`) | Rust handler unmounted (`api_pairing.rs:568`) | restore | Bearer |
| POST | `/api/devices/{id}/token/rotate` | `crates/galleon-gateway/src/api_pairing.rs:674` (device token lifecycle) | Rust handler unmounted (`api_pairing.rs:674`) | restore | Bearer |
| **Rute Fleet / Dashboard (Port dari Go ke Rust `galleon-fleet-core`)** | | | | | |
| GET | `/api/fleet/metrics` | `web/src/utils/apiClient.ts:188`, `apps/tauri/src/main.rs:45` | Go (`engine/src/fleet/delivery.go:56`) | port | Bearer |
| POST | `/api/fleet/deck-bell` | `web/src/utils/apiClient.ts:200`, `apps/tauri/src/main.rs:50` | Go (`engine/src/fleet/delivery.go:57`) | port | Bearer |
| GET | `/api/collections/{name}` | `web/src/utils/apiClient.ts:154`, `apps/tauri/src/main.rs:58` | Go (`engine/src/fleet/delivery.go:65`) | port | Bearer |
| PUT | `/api/collections/{name}` | `web/src/utils/apiClient.ts:170`, `apps/tauri/src/main.rs:66` (via POST) | Go (`engine/src/fleet/delivery.go:65`) | port | Bearer |
| GET | `/api/system/metrics` | `web/src/utils/apiClient.ts:213`, `apps/tauri/src/main.rs:73` | Go (`engine/src/fleet/delivery.go:68`) | port | Bearer |
| GET | `/api/network/qrcode` | `web/src/utils/apiClient.ts:219` | Go (`engine/src/fleet/delivery.go:67`) | port | Bearer |
| GET | `/api/system/executive-briefing` | `web/src/utils/apiClient.ts:231`, `apps/tauri/src/main.rs:87` | Go (`engine/src/fleet/delivery.go:59`) | port | Bearer |
| GET | `/api/providers/harbor` | `web/src/utils/apiClient.ts:243`, `apps/tauri/src/main.rs:92` | Go (`engine/src/fleet/delivery.go:60`) | port | Bearer |
| GET | `/api/diagnostics` | `web/src/utils/apiClient.ts:255`, `apps/tauri/src/main.rs:97` | Go (`engine/src/fleet/delivery.go:61`) | port | Bearer |
| POST | `/api/diagnostics/remedy` | `web/src/utils/apiClient.ts:267`, `apps/tauri/src/main.rs:102` | Go (`engine/src/fleet/delivery.go:62`) | drop | Bearer (Go palsu / stub) |
| GET | `/api/snapshots` | `web/src/utils/apiClient.ts:279`, `apps/tauri/src/main.rs:107` | Go (`engine/src/fleet/delivery.go:63`) | port | Bearer |
| POST | `/api/snapshots` | `web/src/utils/apiClient.ts:291`, `apps/tauri/src/main.rs:112` | Go (`engine/src/fleet/delivery.go:63`) | port | Bearer |
| GET | `/api/engine/processes` | `web/src/utils/apiClient.ts:318`, `apps/tauri/src/main.rs:117` | Go (`engine/src/fleet/delivery.go:71`) | port | Bearer |
| POST | `/api/terminal/exec` | `web/src/utils/apiClient.ts:349` | Go (`engine/src/fleet/delivery.go:74`) | port | Bearer |
| GET | `/api/fleet/policies` | `web/src/utils/apiClient.ts:387`, `apps/tauri/src/main.rs:122` | Go (`engine/src/fleet/delivery.go:64`) | port | Bearer |
| POST | `/api/chat/quartermaster` | `web/src/utils/apiClient.ts:403`, `apps/tauri/src/main.rs:127` | Go (`engine/src/fleet/delivery.go:66`) | port | Bearer |
| GET | `/api/system/network` | `web/src/utils/apiClient.ts:417` | Go (`engine/src/fleet/delivery.go:70`) | port | Bearer |
| GET | `/api/project/git-status` | `web/src/utils/apiClient.ts:423` | Go (`engine/src/fleet/delivery.go:72`) | port | Bearer |
| GET | `/api/v1/audit/trail` | `web/src/components/features/OrganizationView.tsx:30`, `nginx/galleon.conf:68` | Go (`engine/src/fleet/delivery.go:76`) | port | Bearer |
| GET | `/api/fleet/seed` | `web/src/store/fleetStore.ts:429` | Go (`engine/src/fleet/delivery.go:58`) | port | Bearer |
| **Rute Channel, Webhook, dan ACP/SSE** | | | | | |
| POST | `/sop/{*rest}` | penyedia webhook SOP eksternal | Rust unmounted (`crates/galleon-gateway/src/lib.rs:539`) | restore | Signature Webhook |
| GET, POST | `/whatsapp` | penyedia webhook WhatsApp | Rust unmounted (`crates/galleon-gateway/src/lib.rs:518-519`) | restore | Signature Webhook |
| GET, POST | `/whatsapp/{alias}` | penyedia webhook WhatsApp per-alias | Rust unmounted (`crates/galleon-gateway/src/lib.rs:520-521`) | restore | Signature Webhook |
| POST | `/linq` | penyedia webhook Linq | Rust unmounted (`crates/galleon-gateway/src/lib.rs:524`) | restore | Signature Webhook |
| POST | `/linq/{alias}` | penyedia webhook Linq per-alias | Rust unmounted (`crates/galleon-gateway/src/lib.rs:525`) | restore | Signature Webhook |
| POST | `/nextcloud-talk` | penyedia webhook Nextcloud Talk | Rust unmounted (`crates/galleon-gateway/src/lib.rs:528`) | restore | Signature Webhook |
| POST | `/nextcloud-talk/{alias}` | penyedia webhook Nextcloud per-alias | Rust unmounted (`crates/galleon-gateway/src/lib.rs:529`) | restore | Signature Webhook |
| POST | `/webhook/gmail` | Google Pub/Sub push notification | Rust unmounted (`crates/galleon-gateway/src/lib.rs:534`) | restore | Signature Webhook |
| GET | `/acp` | `src/bin/clawcrew-acp-bridge.rs` (WebSocket bridge) | Rust unmounted (`crates/galleon-gateway/src/acp.rs:300`) | restore | Public / Localhost |
| GET | `/api/events` | `nginx/galleon.conf:68`, SSE client | Rust unmounted (`crates/galleon-gateway/src/sse.rs:925`) | restore | Bearer |
| **Rute Rust Gateway yang Saat Ini Aktif di `build_gateway_router`** | | | | | |
| GET | `/health` | `src/main.rs:5757, 7677`, `apps/tauri/src/main.rs:150`, `web/src/utils/apiClient.ts:412`, `scripts/ci/smoke_docker_compose.sh:84` | Rust (`crates/galleon-gateway/src/gateway.rs:137`) | port (pertahankan) | Public |
| GET | `/ready` | K8s readiness probe, scripts | Rust (`crates/galleon-gateway/src/gateway.rs:138`) | port (pertahankan) | Public |
| GET | `/api/sections` | web config section picker | Rust (`crates/galleon-gateway/src/gateway.rs:140`) | port (pertahankan) | Bearer |
| GET | `/api/sections/status` | web config section status | Rust (`crates/galleon-gateway/src/gateway.rs:141`) | port (pertahankan) | Bearer |
| GET | `/api/sections/agent-options` | web config agent options | Rust (`crates/galleon-gateway/src/gateway.rs:145`) | port (pertahankan) | Bearer |
| GET | `/api/sections/catalog` | web catalog | Rust (`crates/galleon-gateway/src/gateway.rs:149`) | port (pertahankan) | Bearer |
| GET | `/api/sections/catalog/models` | web catalog models | Rust (`crates/galleon-gateway/src/gateway.rs:153`) | port (pertahankan) | Bearer |
| GET | `/api/sections/{section}` | web section view | Rust (`crates/galleon-gateway/src/gateway.rs:157`) | port (pertahankan) | Bearer |
| POST | `/api/sections/{section}/items/{key}` | web section item select | Rust (`crates/galleon-gateway/src/gateway.rs:161`) | port (pertahankan) | Bearer |
| GET | `/api/config/sections` | web config compatibility alias | Rust (`crates/galleon-gateway/src/gateway.rs:166`) | port (pertahankan) | Bearer |
| GET | `/api/config/sections/{section}` | web config compatibility alias | Rust (`crates/galleon-gateway/src/gateway.rs:170`) | port (pertahankan) | Bearer |
| POST | `/api/config/sections/{section}/items/{key}` | web config compatibility alias | Rust (`crates/galleon-gateway/src/gateway.rs:174`) | port (pertahankan) | Bearer |
| GET | `/api/config/catalog` | web config compatibility alias | Rust (`crates/galleon-gateway/src/gateway.rs:178`) | port (pertahankan) | Bearer |
| GET | `/api/config/catalog/models` | web config compatibility alias | Rust (`crates/galleon-gateway/src/gateway.rs:182`) | port (pertahankan) | Bearer |
| GET | `/api/config/status` | web config compatibility alias | Rust (`crates/galleon-gateway/src/gateway.rs:186`) | port (pertahankan) | Bearer |
| GET | `/api/config/agent-options` | web config compatibility alias | Rust (`crates/galleon-gateway/src/gateway.rs:190`) | port (pertahankan) | Bearer |
| GET | `/ws` | web agent chat WebSocket | Rust (`crates/galleon-gateway/src/gateway.rs:195`) | port (pertahankan) | Bearer |
| GET | `/ws/chat` | web agent chat WebSocket | Rust (`crates/galleon-gateway/src/gateway.rs:196`) | port (pertahankan) | Bearer |
| GET | `/api/status` | web / CLI status | Rust (`crates/galleon-gateway/src/gateway.rs:198`) | port (pertahankan) | Bearer |
| GET | `/api/health` | Rust gateway health check | Rust (`crates/galleon-gateway/src/gateway.rs:199`) | port (pertahankan) | Public |
| GET, PATCH, OPTIONS | `/api/config` | CLI `src/main.rs:2743`, web config | Rust (`crates/galleon-gateway/src/gateway.rs:200`) | port (pertahankan) | Bearer |
| GET | `/api/config/drift` | web / CLI drift detector | Rust (`crates/galleon-gateway/src/gateway.rs:206`) | port (pertahankan) | Bearer |
| GET | `/api/logs` | web log viewer | Rust (`crates/galleon-gateway/src/gateway.rs:210`) | port (pertahankan) | Bearer |
| GET | `/api/auth/ratelimit` | web / client rate limit query | Rust (`crates/galleon-gateway/src/gateway.rs:211`) | port (pertahankan) | Public |
| **mDNS Subsystem (Q9)** | | | | | |
| UDP | `_clawcrew._tcp` / `_galleon._tcp` | none (0 konsumen di web, Tauri, CLI, scripts) | Rust dormant (`crates/galleon-gateway/src/nodes/mdns.rs`) | drop (tidak dipulihkan) | LAN Multicast |
| **Rute Go yang Di-Drop (Tanpa Konsumen / Sesuai PRD-002 §5)** | | | | | |
| POST | `/api/turn` | none | Go (`engine/src/crew/delivery.go:41`) | drop | None |
| POST | `/api/query` | none | Go (`engine/src/crew/delivery.go:42`) | drop | None |
| GET | `/api/v1/crews` | none | Go (`engine/src/crew/delivery.go:43`) | drop | None |
| GET | `/api/v1/crews/{id}` | none | Go (`engine/src/crew/delivery.go:44`) | drop | None |
| GET, POST | `/api/v1/runs` | none (hanya test snippet di `apps/galleoncode/tests/contract_test.rs:33`) | Go (`engine/src/run/delivery.go:33`) | drop | None |
| GET | `/api/v1/runs/{id}` | none | Go (`engine/src/run/delivery.go:34`) | drop | None |
| GET | `/api/v1/runs/{id}/events` | none (hanya test snippet di `apps/galleoncode/tests/contract_test.rs:33`) | Go (`engine/src/run/delivery.go:121`) | drop | None |
| GET, POST, DELETE | `/api/v1/tasks/{id}` | none | Go (`engine/src/task/delivery.go:27`) | drop | None |
| GET | `/api/v1/tools` | none (`web/src/components/features/HarborView.tsx:109` komentar) | Go (`engine/src/tool/delivery.go:27`) | drop | None |
| GET, POST | `/api/v1/approvals/{id}` | none | Go (`engine/src/tool/delivery.go:28`) | drop | None |
| POST | `/api/v1/tool-requests` | none | Go (`engine/src/tool/delivery.go:30`) | drop | None |
| POST | `/api/v1/tool-executions/{id}/{action}` | `web/src/utils/apiClient.ts:465` (tidak dipanggil komponen mana pun) | Go (`engine/src/tool/delivery.go:29`) | drop | None |
| GET, POST | `/api/v1/workflows` | none | Go (`engine/src/workflow/delivery.go:24`) | drop | None |
| GET, POST | `/api/v1/workflows/{id}` | none | Go (`engine/src/workflow/delivery.go:25`) | drop | None |
| GET, POST | `/api/v1/artifacts/{id}` | none | Go (`engine/src/artifact/delivery.go:26`) | drop | None |
| GET, POST | `/api/v1/learning/proposals*` | `web/src/components/features/MemoryProposalModal.tsx:39,62,71,80,92` | Go (`engine/src/approval/learning_delivery.go:28-29`) | drop (PRD-002 Q1: UI disembunyikan) | None |
| POST | `/api/pairing/claim` | none | Go (`engine/src/identity/pairing_delivery.go:28`) | drop | None |
| GET | `/api/pairing/devices` | `web/src/components/features/ConnectedDevicesView.tsx:33` | Go (`engine/src/identity/pairing_delivery.go:29`) | drop (diganti `/api/devices` Rust) | None |
| POST | `/api/pairing/revoke` | `web/src/components/features/ConnectedDevicesView.tsx:71` | Go (`engine/src/identity/pairing_delivery.go:30`) | drop (diganti `DELETE /api/devices/{id}` Rust) | None |
| POST | `/api/terminal/audit` | none (web memanggil `/api/v1/audit/trail`) | Go (`engine/src/fleet/delivery.go:75`) | drop | None |
| POST | `/api/engine/execute` | none (web memanggil `/api/terminal/exec`) | Go (`engine/src/fleet/delivery.go:77`) | drop | None |
| GET | `/api/v1/entitlement/usage` | none | Go (`engine/src/fleet/delivery.go:78`) | drop | None |
| GET | `/test/runs` | none (hanya ada di test Go `run_test.go:102`) | Go (`engine/src/run/run_test.go:102`) | drop | None |
| GET | `/metrics` (Go :9090) | none | Go (`engine/core/metrics/metrics.go:98`) | drop | None |
| GET | `/healthz` (Go :9090) | none | Go (`engine/core/metrics/metrics.go:99`) | drop | None |
| **Rute OLD Rust (db18a3745^) Tanpa Konsumen yang Tetap Di-Drop** | | | | | |
| GET | `/api/version/check` | none (komentar `src/main.rs:6804`) | OLD:1937 | drop | Bearer |
| POST | `/api/version/upgrade` | none | OLD:1938 | drop | Bearer |
| GET | `/api/version/upgrade/status` | none | OLD:1940 | drop | Bearer |
| GET, POST | `/api/dashboard/tasks*` | none | OLD:1944-1953 | drop | Bearer |
| GET, PUT, DELETE | `/api/config/prop` | none (komentar `src/main.rs:2688`) | OLD:1960 | drop | Bearer |
| GET | `/api/config/list` | none | OLD:1967 | drop | Bearer |
| GET, POST, PUT, DELETE | `/api/sops*` | none | OLD:1968-2005 | drop | Bearer |
| GET, POST, DELETE, PATCH | `/api/cron*` | none | OLD:2010-2025 | drop | Bearer |
| GET | `/api/integrations*` | none | OLD:2030 | drop | Bearer |
| GET, POST | `/api/doctor` | none | OLD:2035 | drop | Bearer |
| GET, POST, DELETE | `/api/memory*` | none | OLD:2040 | drop | Bearer |
| GET | `/api/cost` | none | OLD:2045 | drop | Bearer |
| GET | `/api/cli-tools` | none | OLD:2050 | drop | Bearer |
| GET, POST | `/api/channels*` | none | OLD:2055 | drop | Bearer |
| GET, POST | `/api/backup*` | none (`CrowsNestView.tsx:310` label teks span UI) | OLD:2070 | drop | Bearer |
| GET, POST, DELETE, PUT | `/api/sessions*` | none | OLD:2080-2100 | drop | Bearer |
| GET, POST, DELETE | `/api/canvas*` | none | OLD:2120 | drop | Bearer |
| GET, POST, DELETE | `/api/webauthn*` | none | OLD:2130 | drop | Bearer |
| GET | `/api/plugins*` | none | OLD:2140 | drop | Bearer |
| POST | `/api/upload` | none | OLD:2150 | drop | Bearer |
| GET, POST | `/api/browse*` | none | OLD | drop | Bearer |
| GET, POST | `/api/personality*` | none | OLD | drop | Bearer |
| GET | `/api/skills*` | none | OLD | drop | Bearer |
| GET | `/api/tuis` | none | OLD | drop | Bearer |
| GET, POST | `/api/quickstart*` | none | OLD | drop | Bearer |
| GET | `/ws/canvas/{id}` | none | OLD | drop | Bearer |
| GET | `/ws/nodes` | none | OLD | drop | Bearer |
| GET | `/ws/sops/runs` | none | OLD | drop | Bearer |
| POST | `/hooks/claude-code` | none | OLD | drop | Bearer |
| POST | `/webhook` (generic) | none | OLD:1930 | drop | Bearer |

---

## 2. Analisis & Konfirmasi Rute 'Tanpa Konsumen' PRD-002 §5

PRD-002 §5 menetapkan daftar rute yang tidak dikonsumsi oleh web dan menuntut konfirmasi independen terhadap klien Tauri (`apps/tauri/src`) dan CLI (`src/main.rs`, `src/bin/*`).

Hasil audit dengan verifikasi 2 metode independen (`rg` string path dan `rg` token parser):

1. `/api/turn`:
   - Metode 1 (`rg "/api/turn"`): 0 temuan di `apps/tauri/`, `apps/galleoncode/`, `src/main.rs`, `src/bin/`.
   - Metode 2 (`rg "Turn" / "turn"` pada modul klien): Hanya ditemukan di engine test Go (`engine/src/crew/delivery_test.go:137`).
   - **Status:** Konfirmasi 0 konsumen di Tauri, CLI, dan Web. Keputusan: **drop**.

2. `/api/query`:
   - Metode 1 (`rg "/api/query"`): 0 temuan di seluruh klien konsumen.
   - Metode 2 (`rg "handleHTTPQuery"`): Hanya ada di `engine/src/crew/delivery.go:42`.
   - **Status:** Konfirmasi 0 konsumen. Keputusan: **drop**.

3. `/api/v1/{crews,runs,tasks,tools,tool-requests,workflows,approvals,artifacts,entitlement/usage}`:
   - `/api/v1/crews`: 0 konsumen di Tauri/CLI/Web. Keputusan: **drop**.
   - `/api/v1/runs`: Ditemukan string literal pada test deserialisasi kontrak `apps/galleoncode/tests/contract_test.rs:33` (unit test parser saja, bukan konsumen runtime yang memanggil server). 0 pemanggil HTTP di Tauri/CLI/Web. Keputusan: **drop**.
   - `/api/v1/tasks`: 0 konsumen. Keputusan: **drop**.
   - `/api/v1/tools`: Ditemukan komentar di `web/src/components/features/HarborView.tsx:109` ("Backend /api/v1/tools = tool type definitions... Until /api/harbor/tools exists... array remains empty"). 0 pemanggil HTTP aktif. Keputusan: **drop**.
   - `/api/v1/tool-requests`: 0 konsumen. Keputusan: **drop**.
   - `/api/v1/workflows`: 0 konsumen. Keputusan: **drop**.
   - `/api/v1/approvals`: 0 konsumen. Keputusan: **drop**.
   - `/api/v1/artifacts`: 0 konsumen. Keputusan: **drop**.
   - `/api/v1/entitlement/usage`: 0 konsumen. Keputusan: **drop**.

4. `/api/engine/execute` & `/api/terminal/audit`:
   - Web menggunakan `/api/terminal/exec` (`web/src/utils/apiClient.ts:349`) dan `/api/v1/audit/trail` (`web/src/components/features/OrganizationView.tsx:30`).
   - Tauri dan CLI tidak memanggil `/api/engine/execute` maupun `/api/terminal/audit`.
   - **Status:** Konfirmasi 0 konsumen untuk path alias lama Go tersebut. Keputusan: **drop**.

5. `/test/runs`:
   - Ditemukan hanya di `engine/src/run/run_test.go:102`. 0 konsumen eksternal. Keputusan: **drop**.

6. `/api/v1/learning/proposals*` (PRD-002 Q1):
   - Dikonsumsi UI web di `web/src/components/features/MemoryProposalModal.tsx:39,62,71,80,92`. Namun handler Go tidak pernah terpasang (`app.go` tidak memanggil `learningHandler.RegisterHTTP`), sehingga pemanggilan UI selalu 404.
   - Sesuai keputusan Owner (2026-10-29 pada PRD-002 Q1): UI disembunyikan, endpoint tidak di-port. Keputusan: **drop**.

---

## 3. Keputusan Subsystem Spesifik

### 3.1. mDNS Discovery Subsystem (PRD-002 Q9)
- **Status di Kode:** Crate `galleon-gateway` memiliki modul `crates/galleon-gateway/src/nodes/mdns.rs` dan struktur konfigurasi di `galleon-config/src/schema.rs:5793`.
- **Fakta Konsumen:**
  - `web/src/components/features/EngineRoomView.tsx:184,259` memuat string dummy nama service (`mesh-discovery-mdns`) pada antarmuka mockup terminal, bukan pemanggilan HTTP/mDNS nyata.
  - Tidak ada konsumen di Tauri (`apps/tauri/src/main.rs`).
  - Tidak ada konsumen di CLI (`src/main.rs`).
  - Tidak ada konsumen di `apps/galleoncode`.
- **Keputusan:** Sesuai default PRD-002 Q9 yang diterima Owner, mDNS **tidak dipulihkan** ke runtime gateway (keputusan: **drop / dormant**).

### 3.2. Rute `/admin/*` CLI
- CLI `src/main.rs` memiliki sub-perintah manajemen gateway lokal yang saat ini mengalami 404 sejak commit `db18a3745`:
  - `galleon gateway shutdown` -> `POST /admin/shutdown` (`src/main.rs:8271`)
  - `galleon gateway reload` -> `POST /admin/reload` (`src/main.rs:5192, 9614`)
  - `galleon gateway get-paircode` -> `GET /admin/paircode` (`src/main.rs:8513`)
  - `galleon gateway new-paircode` -> `POST /admin/paircode/new` (`src/main.rs:8502`)
  - `galleon gateway sop list` -> `GET /admin/sop/pending` (`src/main.rs:8328`)
  - `galleon gateway sop approve` -> `POST /admin/sop/approve` (`src/main.rs:8390`)
  - `galleon gateway sop deny` -> `POST /admin/sop/deny` (`src/main.rs:8394`)
- **Keputusan:** Semua rute ini wajib dipulihkan (**restore**) pada TASK-061 dengan pengamanan `require_localhost` + `ConnectInfo<SocketAddr>`.

---

## 4. Perintah Verifikasi yang Dijalankan (Bukti Primer)

Seluruh verifikasi dilakukan secara deterministik pada branch `feat/docs-api-consumer-inventory`:

```bash
# 1. Verifikasi rute aktif di Web
rg -n "/api/" web/src/
# Hasil: 22 rute teridentifikasi di apiClient.ts, ConnectedDevicesView.tsx, OrganizationView.tsx, MemoryProposalModal.tsx, fleetStore.ts.

# 2. Verifikasi rute aktif di Tauri Desktop
rg -n 'engine_(get|post)' apps/tauri/src/main.rs
# Hasil: 14 pemanggilan ke /api/* (semua merupakan proxy delegasi ke backend).

# 3. Verifikasi rute admin di CLI
rg -n 'gateway_admin_url' src/main.rs
# Hasil: /admin/shutdown, /admin/sop/pending, /admin/sop/approve, /admin/sop/deny, /admin/paircode/new, /admin/paircode.

# 4. Verifikasi rute di ACP Bridge
rg -n 'gateway_http_url' src/bin/clawcrew-acp-bridge.rs
# Hasil: /pair/code, /pair.

# 5. Verifikasi ketiadaan konsumen rute PRD-002 §5
rg "/api/turn|/api/query|/api/v1/crews|/api/v1/tasks|/api/v1/tools|/api/v1/workflows" web/ apps/ src/ nginx/ scripts/
# Hasil: 0 referensi pemanggil aktif di runtime.
```
