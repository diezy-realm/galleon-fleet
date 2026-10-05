---
title: Plugin & MCP Architecture — Audit Galleon vs Kompetitor (Antigravity, Kiro Crew, GitHub Copilot, Hermes Agent, OpenClaw)
status: research
date: 2026-10-05
author: squad-lead
depends_on: null
---

# Plugin & MCP Architecture — Audit Mendalam dan Perbandingan Kompetitor

> Audit kode langsung (bukan asumsi) terhadap dua sistem ekstensibilitas Galleon — MCP client (`clawcrew-tools`) dan Plugin WASM/WIT (`clawcrew-plugins`) — dibandingkan dengan Google Antigravity, Kiro Crew, GitHub Copilot, dan dua proyek open-source komunitas: Hermes Agent (NousResearch) dan OpenClaw.

---

## 1. Galleon Punya DUA Sistem Ekstensibilitas Terpisah, dengan Filosofi Keamanan yang SANGAT Berbeda

Audit membuktikan Galleon tidak punya satu "sistem plugin" — ada dua jalur independen:

| | **MCP Client** (`clawcrew-tools`) | **Plugin WASM/WIT** (`clawcrew-plugins`) |
|---|---|---|
| Tujuan | Integrasi tool eksternal dinamis (standar industri MCP) | Channel/tool native performant, tanpa rebuild binary |
| Isolasi proses | Child process OS biasa | WASM Component (Wasmtime, sandbox penuh) |
| Sandboxing | **TIDAK ADA** | Fuel limit, memory/table limit, host-call cap, egress deny-by-default |
| Signing/verifikasi | Tidak ada konsep ini | Ed25519, 3 mode (`Strict`/`Permissive`/**`Disabled` default**) |
| Dokumentasi pembagian use-case | **0 referensi MCP di seluruh `clawcrew-plugins`** — gap dokumentasi murni, bukan kesimpulan audit yang keliru |

**Temuan paling signifikan:** sistem yang membuka pintu ke KODE PIHAK KETIGA TERPERCAYA (plugin WASM, dipasang operator, sering dari katalog kurasi) mendapat sandboxing OS-level yang sangat ketat. Sistem yang membuka pintu ke PROSES EKSTERNAL SEMBARANG (MCP server, bisa dari mana saja, spek resminya mengasumsikan full-trust) **TIDAK dapat sandboxing apa pun**. Ini kebalikan dari intuisi risk-based security — MCP server (lebih berisiko secara desain protokol) seharusnya mendapat containment LEBIH ketat, bukan lebih longgar.

---

## 2. MCP Client — Temuan Konkret

### 2.1 Transport & Lifecycle (matang)
`crates/clawcrew-tools/src/mcp_transport.rs` — 3 transport: `Stdio`, `Http` (streamable HTTP, header `Mcp-Session-Id`), `Sse` (legacy HTTP+SSE). Lifecycle stdio sangat matang: `stdio_child_exit_watcher` poll `try_wait()` tiap 200ms (independen dari EOF stdout — menutup celah descendant-process-masking), `reset()` benar-benar respawn child baru (generation increment, bukan sekadar reconnect socket), graceful shutdown dengan grace period 2 detik lalu escalate ke `kill()`, dan `kill_on_drop(true)` sebagai jaring pengaman kalau registry di-drop tanpa `close()` eksplisit.

`RecoveryBarrier` (`mcp_client.rs`) — exactly-once write guarantee: request yang outcome-nya "unknown" (byte sudah terkirim tapi response tak jelas) TIDAK pernah di-replay, hanya re-handshake. Ini desain yang hati-hati terhadap efek samping ganda (misal tool call yang menulis data).

### 2.2 Permission/Scoping (matang, berlapis)
Dua layer independen + lazy-loading, ketiganya SECARA BERSAMAAN:
- **Per-server (bundle-based)**: `Config::mcp_servers_for_agent` — agent tanpa `mcp_bundles` = nol server (secure-by-default, "omission ≠ grant"). Deny menang di union bundle.
- **Per-tool (`ToolAccessPolicy`)**: filter individual tiap tool dari server yang sudah granted — bukan blanket semua tool exposed.
- **Deferred-loading (`tool_search`)**: tool MCP yang belum "diaktifkan" cuma muncul sebagai stub 1-baris di prompt (bukan full schema) sampai LLM memanggil `tool_search` — mengatasi context bloat dari banyak MCP server (persis mekanisme yang disebut Kiro Crew "Tool Search", lihat §4).

Test suite (`orchestrator_mcp_scope.rs`) menguji skenario negatif (unscoped agent, unknown agent) tapi TIDAK menguji exclude-list atau channel-level dispatch di file itu sendiri (tercakup parsial di file lain).

### 2.3 GAP KRITIS — Nol Sandboxing (HIGH)
`StdioTransport::spawn()` memanggil `Command::new(&config.command)...spawn()` **langsung**, tanpa pembungkus `Sandbox` trait yang sudah ada di `security/traits.rs` dan dipakai konsisten oleh `ShellTool` (native tool). Tidak ada Landlock/Bubblewrap/Firejail/Seatbelt di sekitar spawn MCP. Artinya: begitu operator memberi agent akses ke satu `mcp_bundle`, proses MCP server itu berjalan dengan privilege PENUH user yang menjalankan daemon Galleon — bisa baca/tulis file di luar workspace, buka koneksi jaringan bebas, fork proses lain — tanpa containment OS apa pun. Scoping yang matang di §2.2 (bundle/tool/deferred) HANYA mengatur **mana tool yang LLM boleh panggil**, bukan **apa yang proses MCP server itu sendiri bisa lakukan ke sistem** begitu dia hidup.

### 2.4 Secret Injection (MEDIUM)
`McpServerConfig.env`/`.headers` ditandai `#[secret]` (hanya menyembunyikan dari UI/schema export), tapi disuntik **plaintext langsung** dari config ke child process env (`.envs(&config.env)`). Tidak ada indirection vault seperti pola `cron_secret_request` yang sudah ada di tool lain pada ekosistem ini.

---

## 3. Plugin WASM/WIT — Temuan Konkret

### 3.1 Sandboxing (sangat matang)
`crates/clawcrew-plugins/src/component.rs`: Wasmtime + WASI Preview 2, `consume_fuel(true)` wajib untuk SEMUA plugin call, `StoreLimitsBuilder` membatasi memory/table/instance per `PluginLimits` dari config (komentar eksplisit: *"no `Default` — limits always come from the config registry so no code path can construct an unsandboxed store by accident"*), `fuel_async_yield_interval` cegah infinite-loop guest memblokir host, `MAX_HOST_CALLS_PER_FRAME = 1000` sebagai ceiling terpisah dari fuel. Host function di-scope per `PluginCallPhase` (`Standard`/`ToolExecute`/`ChannelService`) — tool plugin tidak bisa baca secret channel dan sebaliknya.

### 3.2 Egress Model (matang, defense-in-depth)
HTTP egress deny-by-default MESKIPUN granted — docstring eksplisit: *"This grants the surface, never the reach... every request the guest issues is denied"* tanpa policy eksplisit tambahan. Destination grammar strict (exact host atau `*.suffix`, no bare wildcard), redirect TIDAK diikuti otomatis (setiap hop re-evaluasi policy independen — dibuktikan test `egress-fixture`), metadata/loopback/private address diblok kecuali whitelist eksplisit (ADR-014).

### 3.3 Signing (ada mekanisme, DISABLED by default)
Ed25519 via `ring`, 3 mode: `Strict`/`Permissive`/`Disabled` (**default**). Manifest canonical bytes strip hanya field `signature`+`publisher_key`, sisanya (termasuk `[egress]` table) ikut ditandatangani — mencegah plugin signed memodifikasi egress declaration tanpa invalidate signature. **Tapi default `Disabled` berarti tanpa konfigurasi eksplisit operator, plugin apa pun (signed atau tidak) dimuat tanpa verifikasi** — ini "secure by configuration," bukan "secure by default," untuk aspek supply-chain (meskipun network-nya sendiri secure-by-default lewat egress deny).

### 3.4 Lifecycle — Warm/Reused Instance
Tool, channel, DAN memory plugin semuanya pakai `WarmPluginState` — store+binding dibuat sekali, di-reuse lintas call (bukan fresh-instance tiap panggilan seperti komentar `runtime.rs` yang menyebut "fresh store per call" — audit menemukan ini mengacu ke pola caller eksternal, bukan primitive di file itu sendiri, perlu verifikasi lanjut). Fuel di-reset per call (`refuel()`) justru karena store-nya warm.

### 3.5 Gap: `PluginPermission::FileRead`/`FileWrite` Dideklarasikan Tapi Tidak Diverifikasi Terpakai
Enum permission ini ada di `lib.rs` tapi audit tidak menemukan konsumsi nyata di `host.rs`/`component.rs` — kemungkinan reserved for future, belum di-wire ke kapabilitas nyata. Risiko: false sense of capability kalau operator mengira field ini sudah aktif menegakkan sesuatu.

### 3.6 Fixture Test Coverage (sangat baik)
`tool-secret-fixture` menguji cross-binding secret isolation (dua instance plugin package yang sama tidak bisa baca secret satu sama lain). `tool-timeout-fixture` menguji fuel/deadline enforcement dengan mode `"spin"` (infinite loop). `egress-fixture` menguji redirect-chase 2-hop dengan re-evaluasi policy per-hop. Ini level testing yang jauh lebih dalam dari yang biasa ditemukan di plugin system open-source.

---

## 4. Perbandingan Kompetitor

### 4.1 Kiro Crew — Governance Fail-Closed + Tool Search

- **Enterprise MCP governance**: admin set MCP Registry URL; client beroperasi dalam "registry access mode" — entry agent-spec yang tidak match catalog entry dengan nama SAMA di-drop SILENT. **Fail-closed eksplisit**: kalau governance API tak terjangkau, MCP **dimatikan total**, bukan fallback terbuka. Ini kontras dengan Galleon yang belum punya governance tersentralisasi untuk MCP sama sekali (scoping Galleon murni per-agent config lokal, bukan policy terdistribusi fleet-wide).
- **"Tool Search"**: mekanisme deferred-loading MCP persis sama konsepnya dengan `mcp_deferred.rs` Galleon — tool MCP tidak dikirim penuh ke tiap request, dimuat on-demand. Galleon dan Kiro Crew sudah setara di sini.
- **"Powers"**: bundle MCP server + dokumentasi + best-practice, loading on-demand berbasis keyword percakapan — ini lebih dari sekadar `mcp_bundles` Galleon (yang murni daftar server, tanpa dokumentasi/best-practice terlampir).
- Tidak ada bukti publik soal sandboxing OS-level untuk MCP stdio server di Kiro Crew juga — kemungkinan gap industri umum, bukan unik Galleon (MCP spec resmi memang mengasumsikan trust).

### 4.2 GitHub Copilot — Governance Allowlist/Denylist Enterprise

`managed-settings.json` dengan `allowedMcpServers`/`deniedMcpServers`, match by name/URL/command, GA sejak 6 Agustus 2026. Policy terpusat didistribusikan ke semua Copilot client (CLI, VS Code) dari satu sumber. Ini pola yang **Galleon belum punya** — scoping Galleon ada di level `Config` per-instance, bukan policy fleet-wide yang didorong dari admin pusat (meski secara arsitektur Galleon punya `fleet/policies.go` yang BISA jadi tempat natural untuk ini, perlu riset lanjut terpisah).

### 4.3 Google Antigravity — Mengikuti Standar MCP Resmi

Dari hasil pencarian, Antigravity mengintegrasikan MCP sesuai standar resmi (Google Workspace MCP servers, dll) tanpa dokumentasi publik yang merinci sandboxing tambahan di luar apa yang MCP spec sendiri sediakan. Konsisten dengan riset sesi sebelumnya (`agent-subagent-permission-scoping-survey.md`) yang menemukan insiden RCE nyata di Antigravity dari tool "native" yang bypass gate yang sama — pola kerentanan container/sandbox yang relevan dibandingkan dengan gap sandboxing MCP Galleon di atas.

### 4.4 Hermes Agent (NousResearch, open-source) — UX Instalasi Terkurasi

MCP native dengan **"Catalog"**: server MCP yang sudah di-review staf Nous, disabled by default, instalasi one-click dengan **"setup card"** interaktif — field config, secret masking, link OAuth otomatis (Hermes tidak membuka browser sendiri, demi kontrol user), dan **transaction-safety eksplisit**: kalau server menolak token atau user cancel sebelum selesai, TIDAK ADA yang disimpan dari attempt itu — config/token lama tetap utuh. Ini pola UX governance yang jauh lebih matang dari Galleon (yang murni config file manual, tanpa kurasi/katalog/wizard).

### 4.5 OpenClaw (open-source, komunitas sangat besar — 391k star) — Benchmark Governance Plugin Paling Matang

OpenClaw TIDAK punya MCP client native (komunitas membangun >8 MCP bridge pihak ketiga terpisah — tanda jelas butuh user nyata belum terpenuhi platform inti) — tapi **sistem plugin native-nya** adalah benchmark governance paling matang dari semua yang diriset:

- **ClawHub**: registry komunitas terpusat untuk discovery plugin, dengan trust-tier eksplisit: `bundled`/`trusted-official` = accepted sources; npm/git/local arbitrary = butuh `--force` eksplisit setelah review.
- **`security.installPolicy`**: policy command LOKAL yang dijalankan SEBELUM install/update plugin, bisa allow/warn/block, menerima metadata + staged source path. Mencakup jalur CLI maupun Gateway-backed.
- **`plugins.allow`/`deny`**: deny menang, allow adalah **exclusive allowlist** yang menang DI ATAS wildcard tool permission (`tools.allow: ["*"]` tidak membuka plugin tool yang di luar `plugins.allow`) — pemisahan tegas antara "tool apa yang LLM boleh pakai" vs "plugin apa yang boleh ada."
- **Payload verification + auto-quarantine saat Gateway startup**: plugin yang gagal verifikasi di-quarantine untuk BOOT itu, service lain tetap jalan normal (`configured-unavailable` status, bukan crash seluruh gateway).
- **Filesystem ownership check**: plugin yang dimiliki Unix user berbeda dari proses yang memuatnya di-block ("suspicious ownership") — prinsip yang bergema dengan filosofi Landlock Galleon, tapi diterapkan di LEVEL PLUGIN FILE, bukan cuma proses sandbox runtime.
- **Trust/provenance tracking**: `installSource`/`installSpec`/`reason`/`origin` disimpan per-install, dicek ulang saat load (`record-missing`, `provenance-missing`, `origin-path`, `owner-ambiguous`, `provenance-invalid` sebagai kode diagnosa spesifik) — audit trail instalasi yang jauh lebih granular dari apapun yang ada di Galleon untuk WASM plugin maupun MCP.

**Kesimpulan OpenClaw:** governance plugin mereka (bukan sandboxing runtime — itu beda axis) adalah yang paling matang dari semua kompetitor yang diriset, meski basis teknologinya Node.js biasa (bukan WASM sandbox seperti Galleon). Galleon menang di SANDBOXING RUNTIME (Wasmtime capability model jauh lebih kuat dari apapun yang bisa dicapai proses Node biasa), tapi OpenClaw menang di GOVERNANCE INSTALASI (trust-tier, policy command, provenance tracking, quarantine granular).

---

## 5. Sintesis — Peta Kekuatan/Gap

| Aspek | Galleon MCP | Galleon WASM Plugin | Kiro Crew | GitHub Copilot | OpenClaw | Hermes Agent |
|---|---|---|---|---|---|---|
| Sandboxing proses/runtime | ❌ Nol | ✅ Sangat matang (Wasmtime+fuel+limits) | Tidak diverifikasi publik | N/A (bukan plugin system) | ⚠️ Proses OS biasa, tanpa WASM sandbox | Tidak diverifikasi publik |
| Egress/network control | N/A (tidak relevan, MCP server kontrol sendiri) | ✅ Deny-by-default, redirect re-check | — | — | Tidak diverifikasi | — |
| Signing/provenance plugin | N/A | ⚠️ Ada mekanisme, default DISABLED | — | — | ✅ Trust-tier + provenance tracking granular | — |
| Governance terpusat (fleet/enterprise) | ❌ Belum ada | ❌ Belum ada | ✅ Fail-closed registry mode | ✅ Allowlist/denylist managed-settings | ⚠️ Policy command lokal, bukan fleet-wide | ❌ Per-instance |
| Deferred/lazy tool loading | ✅ Matang (`tool_search`) | N/A | ✅ "Tool Search" setara | Tidak diverifikasi | Tidak diverifikasi | Tidak diverifikasi |
| UX instalasi (katalog, wizard) | ❌ Config file manual | ❌ Config file manual | ⚠️ "Powers" bundling | N/A | ✅ ClawHub + CLI search/install | ✅ Catalog + setup card interaktif |
| Secret handling | ⚠️ Plaintext + `#[secret]` UI-hide | ✅ Point-of-use, cross-binding isolation teruji | Tidak diverifikasi | — | ✅ `secret://` URI multi-backend (community bridge) | ✅ Masked di setup card |

---

## 6. Rekomendasi (bahan diskusi, belum backlog resmi)

1. **PRIORITAS TERTINGGI — Sandbox MCP stdio child process.** Reuse `Sandbox` trait yang sudah ada (`security/traits.rs`), scoped ke workspace_dir + whitelist eksplisit per-server. Ini one-way-door yang berpotensi breaking MCP server yang legitimately butuh akses luas (misal filesystem MCP server resmi) — perlu opt-out eksplisit (`sandbox_profile` per-`McpServerConfig`), bukan blanket enforce tanpa jalan keluar.
2. **Dokumentasikan risiko nol-sandbox MCP sebagai security disclosure** di dokumentasi operator — sampai #1 diimplementasi, operator harus sadar bahwa menambah `mcp_bundle` = memberi proses eksternal privilege penuh user daemon.
3. **Tulis ADR pembanding WASM Plugin vs MCP** — kapan pakai yang mana. Saat ini keputusan ini hanya bisa disimpulkan implisit, tidak pernah dinyatakan eksplisit di satu dokumen pun.
4. **Pertimbangkan default `SignatureMode` untuk WASM plugin** di jalur produksi/deployment guide — saat ini `Disabled` adalah default pabrik, perlu rekomendasi eksplisit "set ke `Strict` untuk production" di dokumentasi onboarding.
5. **Verifikasi wiring `PluginPermission::FileRead`/`FileWrite`** — pastikan tidak ada "false sense of capability" kalau field ini belum benar-benar ditegakkan.
6. **Governance MCP terpusat (fleet-wide)** — belajar dari pola Kiro Crew (fail-closed registry) atau GitHub Copilot (allowlist/denylist managed-settings) untuk kasus multi-user/enterprise Galleon ke depan — saat ini scoping Galleon murni per-config-instance, belum ada policy yang didorong terpusat dari Fleet-level governance (`fleet/policies.go` kemungkinan tempat natural, perlu riset terpisah untuk konfirmasi).
7. **UX instalasi MCP/plugin** — Galleon saat ini config-file-only; OpenClaw (ClawHub) dan Hermes (Catalog+setup card) menunjukkan standar UX yang jauh lebih tinggi untuk discovery+instalasi aman. Ini longer-term, tidak urgent dibanding #1-2.

---

## 7. Sumber

- Audit kode (sub-agent, dibaca langsung): `crates/clawcrew-tools/src/mcp_transport.rs`, `mcp_client.rs`, `mcp_tool.rs`, `mcp_deferred.rs`, `mcp_context.rs`; `crates/clawcrew-config/src/schema.rs` (`McpServerConfig`, `McpTransport`, `mcp_servers_for_agent`/`mcp_servers_for_bundles`); `crates/clawcrew-runtime/src/tools/scoped.rs` (`ScopedAssembled`, `ActivatedToolSet`, `ToolAccessPolicy`); `crates/clawcrew-runtime/src/tools/shell.rs` (pembanding Sandbox trait); `crates/clawcrew-channels/tests/orchestrator_mcp_scope.rs`
- Audit kode (sub-agent, dibaca langsung): `crates/clawcrew-plugins/src/component.rs`, `runtime.rs`, `host.rs`, `wasm_channel.rs`, `wasm_memory.rs`, `signature.rs`, `egress.rs`, `lib.rs`; `wit/v0/*.wit` (channel.wit, tool.wit, memory.wit, inbound.wit); `tests/fixtures/{tool,channel,egress}-*-fixture/`; ADR-006, ADR-009, ADR-014; `roadmap/R-rust.md` (R-31)
- Kiro Crew enterprise MCP governance: https://github.com/kirodotdev/KiroCrew/blob/main/docs/guides/enterprise-mcp-governance.md
- Kiro Crew Tool Search: https://kiro.dev/docs/mcp/tool-search/
- GitHub Copilot MCP allowlist enterprise: https://docs.github.com/en/copilot/how-tos/administer-copilot/manage-mcp-usage/configure-enterprise-allowlist
- Hermes Agent MCP: https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/mcp.md
- OpenClaw plugin system: https://github.com/openclaw/openclaw/blob/main/docs/tools/plugin.md
- OpenClaw community MCP bridges (bukti "no built-in MCP client"): https://github.com/ChrisLAS/openclaw-mcp-bridge, https://github.com/amor71/openclaw-mcp
- Google Antigravity MCP (standar resmi, tanpa detail sandboxing tambahan): https://developers.google.com/workspace/guides/configure-mcp-servers
