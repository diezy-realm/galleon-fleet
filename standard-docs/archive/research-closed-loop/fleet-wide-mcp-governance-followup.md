---
title: Riset Lanjutan — Fleet-Wide MCP Governance (Koreksi Asumsi + Temuan Konkret)
status: research
date: 2026-10-05
author: squad-lead
depends_on: plugin-mcp-architecture-audit.md
---

# Riset Lanjutan — Fleet-Wide MCP Governance

> Follow-up dari `plugin-mcp-architecture-audit.md` §6 rekomendasi 6, yang menyebut "`fleet/policies.go` kemungkinan tempat natural, perlu riset terpisah untuk konfirmasi". Audit kode membuktikan klaim itu SALAH dalam detail — berikut koreksi dan temuan sesungguhnya.

## 1. Koreksi: Tidak Ada `fleet/policies.go`, dan `GetFleetPolicies` Murni Read-Only Display

File yang sebenarnya ada: `engine/src/fleet/policies_test.go` (test saja) — implementasi `GetFleetPolicies` ada di `engine/src/fleet/services.go:425`.

**Yang fungsi ini benar-benar lakukan** (dibaca langsung, bukan asumsi):
```go
policiesRaw, _ := os.ReadFile(filepath.Join(s.dataDir, "policies.json"))
riskTiersRaw, _ := os.ReadFile(filepath.Join(s.dataDir, "riskTiers.json"))
// unmarshal ke []map[string]any (generik, bukan struct typed)
return map[string]any{"policies": policies, "riskTiers": riskTiers}, nil
```

Skema `policies.json` (dari `policies_test.go`): `{"id", "name", "scope", "enforcement", "description"}` — ini kebijakan generik level governance/compliance (nama, scope, level enforcement sebagai STRING deskriptif), **BUKAN allowlist/denylist teknis untuk server/tool apapun**.

**Konsumen fungsi ini hanya 1**: `delivery.go::handlePolicies` — HTTP handler `GET /api/fleet/policies` yang **cuma** `writeJSON(w, http.StatusOK, policies)`. Read-only, GET-only, tidak ada method POST/PUT, tidak ada logic enforcement apa pun di sekitarnya. Tujuan nyata fungsi ini: **menampilkan daftar kebijakan ke dashboard**, bukan menegakkan apa pun di jalur eksekusi.

**Kesimpulan tegas:** `fleet/policies.go` BUKAN "tempat natural" untuk governance MCP — itu klaim yang keliru di dokumen sebelumnya. Infrastrukturnya tidak dirancang untuk ini (data generik display-only, bukan policy engine).

## 2. Temuan Utama: Go Sama Sekali Tidak Punya Konsep MCP

Grep menyeluruh `mcp_server|McpServer|MCPServer|mcp_bundle|McpBundle` di SELURUH `engine/` = **0 hasil**. Ini mengonfirmasi dan memperkuat temuan `plugin-mcp-architecture-audit.md`: MCP adalah **konsep yang murni hidup di Rust** (`galleon-config::McpServerConfig`, `galleon-tools`). Go — yang menurut arsitektur yang dikonfirmasi Achmad adalah layer kebijakan/konsensus/hierarki organisasi (Fleet→Ship→Squad→Crew) — **tidak punya jalur apa pun** untuk melihat, mengatur, atau membatasi MCP server yang terpasang di level Rust.

**Implikasi:** governance MCP fleet-wide (dalam arti "satu kebijakan dari Fleet-level yang mengikat semua Ship/Squad/Crew di bawahnya") **tidak mungkin diterapkan hari ini** — bukan karena implementasinya kurang matang, tapi karena jembatannya BELUM ADA SAMA SEKALI. Ini konsisten dengan temuan sesi riset sebelumnya soal memory (`capabilities-skills-lessons-steering-audit.md`): proto `SystemGateway` hanya punya 3 RPC (`ExecuteTurn`/`ExecuteNativeTool`/`GetDecryptedSecret`) — tidak ada RPC untuk MCP governance juga.

## 3. Apa yang SEBENARNYA Ada untuk "Governance" MCP Hari Ini

Scoping MCP yang ada (dari audit sebelumnya) seluruhnya di level **config lokal satu instance Rust**:
- `mcp_bundles` per-agent (siapa boleh akses server mana) — Rust-side, resolve dari `Config` yang di-load dari file lokal.
- Tidak ada registry terpusat, tidak ada policy yang didorong dari luar (beda dengan Kiro Crew yang punya MCP Registry URL dari admin console, atau GitHub Copilot `managed-settings.json`).
- Tidak ada hubungan dengan `RiskTier`/`ApprovalGate`/`ExecutionContext` yang sudah matang di Go untuk delegasi sub-agent (`agent-subagent-ship-squad-permission-scheme.md`) — infrastruktur itu TIDAK menyentuh MCP sama sekali, karena Go tidak tahu MCP itu ada.

## 4. Mengapa Ini Penting (Dampak Konkret)

Konsekuensi nyata dari gap ini:
1. **Tidak ada cara bagi Fleet admin (manusia atau Go-level policy) untuk melarang MCP server tertentu secara terpusat** — kalau operator Crew lokal menambah `mcp_bundle` berisi server berbahaya/belum di-review ke config Rust-nya, tidak ada lapis Go yang bisa menolak/memblokir itu sebelum MCP server tersebut tersambung dan tool-nya ter-expose ke LLM.
2. **Tidak konsisten dengan model organisasi Fleet→Ship→Squad→Crew** — delegasi sub-agent sudah punya privilege-ceiling enforcement yang matang dan ditest (`TestDelegationPolicyCeiling`), tapi MCP (jalur ekstensibilitas yang sama berbahayanya, bahkan menurut temuan sandboxing di audit sebelumnya LEBIH berbahaya) berjalan sepenuhnya di luar hierarki itu.
3. **Dibandingkan kompetitor**: Kiro Crew (fail-closed registry) dan GitHub Copilot (`managed-settings.json`) keduanya punya mekanisme TEPAT untuk kasus ini — kebijakan terpusat yang didorong dari admin/enterprise level, independen dari config lokal tiap instance.

## 5. Rekomendasi (bahan diskusi — bukan backlog resmi, karena menyentuh desain gRPC baru)

Ini BUKAN perbaikan kecil — menambah governance MCP fleet-wide butuh salah satu dari:
1. **RPC baru di `SystemGateway` proto** (`ListMcpServers`/`GetMcpGovernancePolicy` atau serupa) supaya Go bisa membaca state MCP dari Rust, lalu Go menerapkan kebijakan Fleet-level di atasnya (menolak start turn kalau MCP bundle yang di-resolve melanggar policy Fleet). Ini SELARAS dengan arsitektur Go=kebijakan/Rust=eksekusi yang sudah dikonfirmasi Achmad — Go tidak perlu tahu DETAIL teknis MCP (transport, env, dll), cukup tahu NAMA server mana yang granted ke agent mana, lalu Go putuskan boleh/tidak berdasarkan `RiskTier`/Fleet policy.
2. **ATAU**: policy check dilakukan murni di Rust tapi dengan config yang bisa di-push dari luar (file policy terpisah yang di-sync dari Fleet, bukan config lokal per-instance) — lebih dekat ke pola Kiro Crew/GitHub Copilot, tidak butuh proto baru, tapi butuh mekanisme distribusi config (yang mana juga belum ada di Galleon sejauh diriset).

**Pilihan arsitektur mana yang tepat BUKAN keputusan yang bisa diambil dari riset kode saja** — ini one-way-door yang menyentuh batas tanggung jawab Go/Rust yang sudah dikonfirmasi Achmad. Perlu diajukan sebagai pertanyaan eksplisit, bukan diasumsikan.

## 6. Koreksi ke Dokumen Sebelumnya

`plugin-mcp-architecture-audit.md` §6 rekomendasi 6 perlu dibaca dengan catatan: kalimat "`fleet/policies.go` kemungkinan tempat natural" adalah **asumsi yang terbukti keliru** setelah verifikasi — file itu tidak ada, dan `GetFleetPolicies` yang dimaksud murni data generik read-only tanpa hubungan ke MCP maupun tool-execution apa pun. Governance MCP fleet-wide butuh infrastruktur BARU (RPC proto atau mekanisme distribusi config), bukan reuse sesuatu yang sudah ada.

## 8. Cross-Check ke `finalize/dev-tasks/` (read-only, dibaca atas permintaan Owner untuk mengkonkretkan temuan)

Dibaca `01-be-go-engine.md`, `08-carryover-from-finalize.md`, `README.md` di `standard-docs/decisions/finalize/dev-tasks/` (TIDAK diedit, sesuai batasan sesi).

### 8.1 Temuan: Keputusan MCP di Go SUDAH diambil Owner secara eksplisit — bukan oversight

`08-carryover-from-finalize.md` bagian D, **Q-2 (disetujui Owner 2026-10-04)**:
> *"`tool/plugin.go` & `tool/mcp_client.go`: hapus (rekomendasi; tanpa pemanggil, tidak digerbang) atau pindah ke Rust plugin host. **MCP client dibuat ulang di Rust bila dibutuhkan.**"*

Ini **mengonfirmasi dengan kuat** temuan `plugin-mcp-architecture-audit.md`: MCP murni hidup di Rust bukan karena Go "belum sempat" mengimplementasikannya, tapi karena **keputusan sadar Owner** untuk menghapus implementasi Go yang lama (`tool/plugin.go`, `tool/mcp_client.go` — nama file persis cocok dengan yang disebut di `BG-A1`) dan membangunnya ulang di Rust. Verifikasi disk (`glob engine/src/tool/*.go`) membuktikan eksekusi keputusan ini **sudah selesai**: `plugin.go` dan `mcp_client.go` TIDAK ADA LAGI di `engine/src/tool/` saat ini — konsisten dengan `BG-A1` yang menandai *"File `plugin.go` dan `mcp_client.go` telah dihapus"* status `[x]` selesai.

### 8.2 Dampak ke temuan governance fleet-wide (§1-5 di atas)

Ini memperkuat, bukan melemahkan, kesimpulan §2: gap governance MCP fleet-wide BUKAN sekadar "belum sempat dikerjakan" — ini **konsekuensi langsung** dari keputusan arsitektur yang sudah diambil (MCP = Rust-only domain). Siapa pun yang nanti membangun governance fleet-wide untuk MCP HARUS bekerja dengan kenyataan ini sebagai titik awal: Go tidak akan "mendapatkan kembali" visibility MCP secara alami dari sisa kode lama — jalurnya harus baru (RPC proto, sesuai §5 opsi 1).

### 8.3 Temuan sampingan: sandboxing MCP TIDAK disinggung sama sekali di dev-tasks

Grep `MCP|sandbox` di seluruh `dev-tasks/` menghasilkan 12 match — SEMUA soal migrasi/penghapusan file (`Q-2`, `BG-A1`) atau soal WIT/plugin WASM (`BR-F1`, `BR-F2`, `RB-24` — WIT v1 versioning), **nol match** yang menyinggung sandboxing proses MCP stdio. Ini mengonfirmasi temuan `plugin-mcp-architecture-audit.md` §2.3 (gap sandbox MCP, severity HIGH): **tim dev-tasks belum pernah membahas ini**, bukan karena sudah dianggap "selesai" atau "ditangani di tempat lain" — topik ini genuinely belum pernah diangkat di jalur resmi mana pun yang terverifikasi.

### 8.4 Catatan ketepatan temuan sebelumnya

Tabel "Kondisi Terakhir" di `README.md` dev-tasks (per 2026-10-04) sempat menyebut `tool/{services,sandbox,plugin,mcp_client}.go` "masih `os/exec` lokal" sebagai status BG-A1 "parsial" — ini snapshot SEBELUM Q-2 dieksekusi penuh. Verifikasi disk saya (hari ini) membuktikan `plugin.go`/`mcp_client.go` sudah tidak ada; `sandbox.go` yang masih ada ternyata adalah file tool definitions (`SandboxedCodeRunnerTool`, `BrowserViewPageTool`, dll — nama mengandung kata "sandbox" tapi beda konteks dari Sandbox trait Rust) yang sudah fail-closed routing ke gateway, bukan sisa `os/exec` lokal yang aktif. BG-A1 kemungkinan sudah lebih dekat ke selesai dari status "parsial" yang tercatat di snapshot tersebut — tapi ini di luar scope riset MCP/plugin, hanya dicatat sebagai observasi sampingan.

---

## 9. Sumber (tambahan §8)

- `standard-docs/decisions/finalize/dev-tasks/08-carryover-from-finalize.md` bagian D (Q-2, disetujui Owner 2026-10-04) — dibaca read-only
- `standard-docs/decisions/finalize/dev-tasks/01-be-go-engine.md` (BG-A1)
- `standard-docs/decisions/finalize/dev-tasks/README.md` (tabel "Kondisi Terakhir")
- Verifikasi disk: `glob engine/src/tool/*.go` — `plugin.go`/`mcp_client.go` tidak ada; `sandbox.go` ada tapi berisi tool definitions, bukan sisa `os/exec` MCP
