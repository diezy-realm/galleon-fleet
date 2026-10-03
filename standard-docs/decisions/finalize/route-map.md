# Frontend↔Engine Route Map Documentation (C4)

Dokumen ini memetakan seluruh kontrak antarmuka API antara frontend (`web-2/src/utils/apiClient.ts`) dan Go Engine (`engine/src/fleet/delivery.go`, `engine/src/crew/delivery.go`, `engine/src/tool/delivery.go`).

## 1. Status Pemetaan Rute

| Method `apiClient.ts` | HTTP Endpoint | HTTP Method | Handler Go Engine | Status Kontrak |
| :--- | :--- | :--- | :--- | :--- |
| `getCollection(name)` | `/api/collections/{name}` | `GET` | `fleet.handleCollections` | ✅ Terdaftar & Terintegrasi |
| `saveCollection(name, data)` | `/api/collections/{name}` | `POST` | `fleet.handleCollections` | ✅ Terdaftar & Terintegrasi |
| `getFleetMetrics()` | `/api/fleet/metrics` | `GET` | `fleet.handleMetrics` | ✅ Terdaftar & Terintegrasi |
| `ringDeckBell()` | `/api/fleet/deck-bell` | `POST` | `fleet.handleDeckBell` | ✅ Terdaftar & Terintegrasi |
| `getSystemMetrics()` | `/api/system/metrics` | `GET` | `fleet.handleSystemMetrics` | ✅ Terdaftar & Terintegrasi (SSOT ADR 0002/0003) |
| `getNetworkQrCode(url)` | `/api/network/qrcode?url=...` | `GET` | `fleet.handleNetworkQRCode` | ✅ Terdaftar & Terintegrasi (ADR 0002) |
| `getExecutiveBriefing()` | `/api/system/executive-briefing` | `GET` | `fleet.handleBriefing` | ✅ Terdaftar & Terintegrasi |
| `getHarborProviders()` | `/api/providers/harbor` | `GET` | `fleet.handleHarborProviders` | ✅ Terdaftar & Terintegrasi |
| `getDiagnostics()` | `/api/diagnostics` | `GET` | `fleet.handleDiagnostics` | ✅ Terdaftar & Terintegrasi |
| `applyRemedy()` | `/api/diagnostics/remedy` | `POST` | `fleet.handleRemedy` | ✅ Terdaftar & Terintegrasi |
| `getSnapshots()` | `/api/snapshots` | `GET` | `fleet.handleSnapshots` | ✅ Terdaftar & Terintegrasi |
| `createSnapshot(title)` | `/api/snapshots` | `POST` | `fleet.handleSnapshots` | ✅ Terdaftar & Terintegrasi |
| `getFleetPolicies()` | `/api/fleet/policies` | `GET` | `fleet.handlePolicies` | ✅ Terdaftar & Terintegrasi |
| `chatQuartermaster(msg, ctx)` | `/api/chat/quartermaster` | `POST` | `fleet.handleQuartermasterChat` | ✅ Terdaftar & Terintegrasi |
| `getHealth()` | `/api/health` | `GET` | `fleet.handleHealth` | ✅ Terdaftar & Terintegrasi |
| `getNetwork()` | `/api/system/network` | `GET` | `fleet.handleNetwork` | ✅ Terdaftar & Terintegrasi |
| `getEngineProcesses()` | `/api/engine/processes` | `GET` | `fleet.handleEngineProcesses` | ✅ Terdaftar & Terintegrasi (Shape ProcessItem) |
| `executeTerminalCommand(cmd)` | `/api/engine/execute` | `POST` | `fleet.handleEngineExecute` | ✅ Terdaftar & Terintegrasi |
| `getOllamaStatus()` | `/api/providers/ollama/status` | `GET` | `fleet.handleOllamaStatus` | ✅ Terdaftar & Terintegrasi |

## 2. Invarian Reverse Proxy
Semua panggilan `/api/*` dari klien web browser dilayani oleh Node/Vite development server yang secara otomatis me-reverse-proxy ke Go Engine port `http://127.0.0.1:9090` (atau Tauri IPC bridge jika berada dalam shell desktop).

## 3. Hasil Pengujian & Verifikasi
- Pengujian Go Engine: `go test -v ./src/fleet/...` (100% PASS, Exit code 0).
- Pengujian Frontend TypeScript: `npm run lint` (`tsc --noEmit`) pada `web-2` (100% PASS, Exit code 0).
