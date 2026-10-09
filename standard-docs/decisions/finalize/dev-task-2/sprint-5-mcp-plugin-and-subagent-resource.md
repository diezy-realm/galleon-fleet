---
sprint: 5
title: MCP/Plugin Governance & Sub-Agent Resource Management
status: in-progress (9/11 checklist [x]; 2 item masih [ ])
---

# Sprint 5 — MCP/Plugin Governance & Sub-Agent Resource Management

## A. Resource Management Sub-Agent (akar masalah crash yang dilaporkan Owner)

- [x] **BL-15 (P1, HIGH, PRIORITAS TERTINGGI file ini)** Resource gate SEBELUM spawn sub-agent. Nol gate saat ini adalah penyebab langsung crash yang sudah dilaporkan.

#### Detail Eksekusi — BL-15

**TIDAK DIVERIFIKASI PENUH sesi ini** (lokasi titik spawn sub-agent di `clawcrew-runtime` belum dibaca langsung — budget sesi ini terbatas). Yang TERVERIFIKASI: `crates/clawcrew-runtime/src/agent/loop_detector.rs` (modul `LoopDetector`, dipakai `results_collect.rs`) ADA dan matang (dipakai aktif di collect_tool_results). File target spawn sub-agent menurut dokumen: `crates/clawcrew-runtime/src/agent/spawn.rs` "atau setara" — **belum dikonfirmasi nama file persis**.

**Langkah pertama WAJIB:** `search_symbols` untuk "spawn_subagent"/"SubAgent"/"delegate" di `crates/clawcrew-runtime/src/` untuk menemukan titik Rust-side yang memicu subagent (BEDA dari jalur Go `dispatcher.go`/`delegation.go` yang sudah dianalisis mendalam di Sprint 4 BL-05 — ini sisi Rust runtime, kemungkinan `clawcrew-runtime/src/subagent/mod.rs` yang disebut di lesson "BL-13 ... subagent/mod.rs").

**Spesifikasi API (pola, verifikasi struktur persis saat eksekusi):**
```rust
/// Gate checked immediately before a sub-agent spawn is dispatched.
/// Returns Err when spawning would exceed the configured ceiling —
/// caller must NOT proceed to spawn.
pub fn check_resource_gate(
    current_active_subagents: usize,
    current_memory_estimate_mb: u64,
    policy: &SubagentResourcePolicy, // nama tipe TBD, cari existing config struct
) -> Result<(), ResourceGateDenied> {
    if current_active_subagents >= policy.max_concurrent_subagents {
        return Err(ResourceGateDenied::TooManyConcurrent {
            current: current_active_subagents,
            max: policy.max_concurrent_subagents,
        });
    }
    if current_memory_estimate_mb >= policy.max_memory_mb {
        return Err(ResourceGateDenied::MemoryBudgetExceeded {
            current_mb: current_memory_estimate_mb,
            max_mb: policy.max_memory_mb,
        });
    }
    Ok(())
}
```

**Langkah pengerjaan:**
1. Temukan titik spawn sub-agent Rust-side persis (`search_symbols`, prioritas: cari di `clawcrew-runtime/src/subagent/`).
2. Cek apakah ada konfigurasi resource policy existing yang bisa direuse (mis. `SecurityPolicy.max_actions_per_hour` dari `policy.rs:349` yang sudah disebut BL-13 Sprint 3 — TAPI itu rate per JAM, bukan concurrent cap; kemungkinan butuh dimensi BARU, bukan reuse langsung).
3. Tambah gate SEBELUM titik spawn — pola fail-closed (gate menolak secara default kalau policy tidak terkonfigurasi, BUKAN mengizinkan unlimited).
4. Test: simulasikan N+1 spawn concurrent saat cap = N → spawn ke-N+1 ditolak dengan error jelas, N spawn pertama tetap jalan.

**Acceptance criteria testable:** Test yang membuktikan root cause crash tertutup — spawn beruntun yang dulu menyebabkan OOM/crash sekarang ditolak gate SEBELUM memori terpakai, bukan crash saat eksekusi.

**Dependency:** Berbeda dari BL-18 (Sprint 4, jalur Go `spawn_subagent`→`ExecuteTurn`) — BL-15 ini sisi RUST (`clawcrew-runtime`), BL-18 sisi GO. Dua gate terpisah untuk dua titik masuk berbeda, KEDUANYA perlu ada (bukan salah satu saja).

**JANGAN DIUBAH:** Logic sub-agent spawn itu sendiri — tiket ini HANYA menambah gate di depan, tidak mengubah bagaimana sub-agent berjalan setelah lolos gate.

**ASUMSI:** Nilai cap (`max_concurrent_subagents`/`max_memory_mb`) belum ditentukan dokumen — perlu keputusan Lead berdasarkan data crash yang sudah dilaporkan (kalau ada angka crash threshold yang diketahui, pakai itu sebagai starting point, bukan angka sembarangan).

- [ ] **BL-16 (MEDIUM)** Surface `loop_detector_circuit_breaker` ke Tasks-view — 3-pola deteksi sudah matang tapi hasil tidak pernah sampai ke dashboard. (REVISI 2026-10-09, terverifikasi 2x independen: commit `0a8cac52` mengklaim "proto+endpoint" tapi `git show --stat` membuktikan HANYA 1 file baru `engine/src/run/loop_detector.go` — struct berdiri sendiri, NOL perubahan ke `proto/agent_service.proto`, NOL ke `engine/src/run/dto.go`, NOL wiring ke handler/service manapun. `QuestsView.tsx:369-392` tetap render field yang tidak pernah diisi API. Overclaim terkonfirmasi — lihat `finding/2026-10-09-remediation-progress-handoff.md` §T-07.)

#### Detail Eksekusi — BL-16

**Koreksi nama simbol:** `loop_detector_circuit_breaker` sebagai satu nama fungsi/tipe TIDAK ADA — nama aktual adalah modul `crate::agent::loop_detector::LoopDetector` (`crates/clawcrew-runtime/src/agent/loop_detector.rs`), dipakai di `crates/clawcrew-runtime/src/agent/turn/results_collect.rs:9` (`use crate::agent::loop_detector::LoopDetector;`) dan dipanggil di `collect_tool_results` (baris ~38, parameter `loop_detector: &mut LoopDetector`). Komentar modul sendiri (baris 1-3 `results_collect.rs`): *"feed the pattern-based loop detector, and run the time-gated identical-output abort"* — konfirmasi klaim dokumen "3-pola deteksi sudah matang" kemungkinan akurat (Warning/Block/Break disebut di komentar fungsi `collect_tool_results`), TAPI nama fungsi "circuit_breaker" tidak match — mungkin itu nama konseptual bukan nama kode.

**File & simbol spesifik:**
- Baca PENUH: `crates/clawcrew-runtime/src/agent/loop_detector.rs` — cari struct `LoopDetector`, method yang mengembalikan hasil deteksi (Warning/Block/Break per komentar), dan apakah ada struct hasil yang bisa di-serialize untuk dashboard.
- Baca: `web/` dashboard "Tasks-view" — cari komponen existing yang menampilkan status task/sub-agent untuk tahu pola integrasi API yang sudah dipakai.

**Langkah pengerjaan:**
1. Baca `loop_detector.rs` penuh untuk API publik `LoopDetector` (method untuk membaca state deteksi saat ini).
2. Tambah endpoint gateway baru (GET, read-only, pola sama dengan Eval-02 Sprint 3 / R-90 Sprint 1) yang expose state loop-detector per sub-agent/turn aktif.
3. Tambah UI di Tasks-view (`web/src/`) untuk render status ini — cari komponen Tasks-view existing dulu sebelum menambah baru.

**Dependency:** BL-17 (di bawah) depend pada ini.

**ASUMSI:** "Tasks-view" lokasi di `web/src/` belum diverifikasi — cari komponen dengan nama serupa sebelum menebak path.

- [ ] **BL-17 (LOW, exploratory, dep BL-16)** Agregasi lintas-sub-agent untuk fan-out besar. (REVISI 2026-10-09: dependency BL-16 masih `[ ]` — tiket ini genuinely belum bisa mulai sampai BL-16 selesai.)

#### Detail Eksekusi — BL-17

**Langkah pengerjaan:** Setelah BL-16 memberi visibility PER sub-agent, tiket ini agregasi lintas BANYAK sub-agent (fan-out besar, pola `spawn_sub_agents` paralel) — tampilkan ringkasan (berapa yang Warning/Block/Break) bukan daftar individual yang membengkak untuk fan-out besar.

**Dependency:** BL-16 (hard, butuh data per-sub-agent dulu).

**ASUMSI:** Scope UI/UX agregasi (chart? tabel ringkas? badge count?) belum ditentukan dokumen — ini keputusan desain, bukan detail teknis bebas pilih. Tandai sebagai exploratory sesuai label dokumen asli, konfirmasi bentuk UI ke Lead/Design sebelum eksekusi.

> **BL-18 TIDAK di sprint ini** — sudah diangkat ke Sprint 4 sebagai acceptance-criteria tambahan BG-D1/BG-D4 (jalur Go `spawn_subagent`→`ExecuteTurn`), karena dependency aslinya ada di sana.

## B. Plugin & MCP Architecture (lanjutan BL-19 Sprint 0)

- [x] **BL-20 (MEDIUM)** Dokumentasikan gap sandbox MCP (interim) — sambil BL-19 (Sprint 0) berjalan.

#### Detail Eksekusi — BL-20

**RESOLVED (2026-10-09, squad-lead, diverifikasi langsung ke disk):** MCP client host **GENUINELY ADA** di `crates/clawcrew-tools/src/` — 10 file: `mcp_transport.rs`, `mcp_client.rs`, `mcp_deferred.rs`, `mcp_protocol.rs`, `mcp_context.rs`, `mcp_prompt.rs`, `mcp_prompts_tool.rs`, `mcp_resource.rs`, `mcp_resources_tool.rs`, `mcp_tool.rs` (dikonfirmasi `glob` langsung ke disk, bukan asumsi). Ini TERPISAH TOTAL dari `crates/clawcrew-plugins/` (WASM Plugin host, Component Model/Wasmtime) — dua arsitektur ekstensi berbeda, bukan nama lain untuk hal yang sama. Analisis gap sandbox lengkap sudah ditulis di `standard-docs/decisions/finalize/dev-task-2/mcp-sandbox-gap-and-vault-mediation.md` (2026-10-07): MCP stdio subprocess belum punya `McpCommandBoundary` (Landlock/Bubblewrap/Seatbelt/AppContainer) setara isolasi WASM — berjalan privilege host penuh kecuali `sandbox_profile` diset `deny-all`/`blocked`. Checkbox `[x]` genuinely benar — dokumentasi gap sudah solid dan akurat.

~~Koreksi penting lama (SUDAH TIDAK BERLAKU, dipertahankan untuk jejak audit):~~ ~~MCP client host BUKAN `crates/clawcrew-plugins/`... kemungkinan di lokasi BERBEDA — belum ditemukan di grep sesi ini.~~ — ini KELIRU karena scope grep sesi audit lama terlalu sempit/stale sebelum dokumen gap ditulis. Lihat `2026-10-09-architectural-decisions-analysis.md` di folder `finding/` untuk kronologi koreksi lengkap.

- [x] **BL-21 (MEDIUM, evaluasi desain dulu)** Vault-mediation untuk secret MCP env/headers.

#### Detail Eksekusi — BL-21

**RESOLVED:** Desain vault-mediation sudah lengkap di `mcp-sandbox-gap-and-vault-mediation.md` §3 — JIT (Just-In-Time) decrypt via `secrets::decrypt` (reuse `clawcrew-config/src/secrets.rs`, tidak bangun vault terpisah), format ciphertext `$clawvault$v1$...`, environment variable proses induk yang tidak terdaftar dibersihkan via `apply_default_mcp_sandbox`. Checkbox `[x]` genuinely benar.


- [x] **BL-22 (MEDIUM)** ADR pembanding WASM Plugin vs MCP.

#### Detail Eksekusi — BL-22

**Langkah pengerjaan:** Tulis ADR baru (`standard-docs/decisions/`) membandingkan `clawcrew-plugins` (WASM/WIT host, matang, sandbox ketat via `wasi_http.rs`/capability model) vs MCP (stdio child process, sandbox gap per BL-19/20) — kapan pakai yang mana untuk plugin baru, trade-off keamanan/portabilitas/kemudahan-development.

**Dependency:** BL-20 (butuh temuan gap MCP sebagai bahan perbandingan).

- [x] **BL-23 (MEDIUM)** Verifikasi wiring `PluginPermission::FileRead/Write`.

#### Detail Eksekusi — BL-23

**Verifikasi:** `crates/clawcrew-plugins/src/lib.rs:132,134` — `PluginPermission::FileRead`/`FileWrite` ADA sebagai enum variant. **TAPI** grep pemakaian (`PluginPermission::FileRead` ATAU `::FileWrite` sebagai referensi, bukan deklarasi) di seluruh `clawcrew-plugins` hanya **1 match** (`component.rs`) — dibandingkan `ConfigRead` yang punya 5 match tersebar (`config.rs` x4, `lib.rs` x1). Ini **indikasi kuat** `FileRead`/`FileWrite` terdaftar sebagai permission tapi TIDAK diperiksa (enforced) di banyak titik seperti `ConfigRead` — konsisten klaim dokumen, perlu verifikasi lebih dalam.

**File & simbol spesifik:**
- Baca: `crates/clawcrew-plugins/src/component.rs` — baris persis yang menyebut `PluginPermission::FileRead`/`FileWrite` (match tunggal yang ditemukan) — apakah ini enforcement nyata atau hanya pattern match tanpa efek (match arm kosong/pass-through)?
- Baca: `crates/clawcrew-plugins/src/config.rs:207,387,714,729` — pola `ConfigRead` SEBAGAI REFERENSI "cara yang benar" untuk membandingkan apakah `FileRead`/`FileWrite` punya pola enforcement setara.

**Langkah pengerjaan:**
1. Baca titik pemakaian `FileRead`/`FileWrite` di `component.rs` — tentukan apakah genuinely enforced atau dead-permission (pola bug "configuration-lies" yang sudah tercatat berulang di lesson project ini — JANGAN simpulkan "aman" hanya dari nama enum terdaftar).
2. Kalau dead/tidak enforced: tambah enforcement check di titik file I/O plugin (sebelum WASM component diberi akses baca/tulis file), pola sama dengan `ConfigRead` check di `config.rs:387` (`scope.grants().allows(PluginPermission::ConfigRead)`).
3. Test: plugin TANPA `FileWrite` permission di manifest mencoba tulis file → ditolak.

**Acceptance criteria testable:** Test negatif plugin tanpa `FileRead`/`FileWrite` permission gagal saat mencoba akses file, konsisten pola `ConfigRead` yang sudah teruji.

**JANGAN DIUBAH:** `ConfigRead` enforcement yang sudah benar — itu referensi pola, bukan target perubahan.

> **BL-24 TETAP TIDAK DIJADWALKAN** — exploratory, butuh keputusan arsitektur Owner eksplisit (RPC proto baru vs config distribution, menyentuh boundary Go/Rust yang sudah one-way-door). Tercatat di `backlog/README.md`, bukan di sprint manapun sampai Owner memutuskan.

## C. Plugin & Channel Tier (dev-tasks asli, scope berdekatan)

- [x] **BR-F2 · R-35 · PR-86 (P1, M)** Host WIT v1 (dual v0/v1; v0 tetap lolos `.frozen`). Pasangan `RB-24` (Sprint 8).

#### Detail Eksekusi — BR-F2

**TIDAK DIVERIFIKASI sesi ini** — struktur `wit/v0`/`wit/v1` belum dibaca. **Langkah pertama WAJIB:** `read` direktori `wit/` untuk struktur v0 existing sebelum merancang v1.

**Dependency:** RB-24 (Sprint 8) — koordinasikan timing, RB-24 adalah bagian rename massal yang menunggu Sprint 0-7 selesai, sementara BR-F2 adalah pekerjaan FUNGSIONAL (host dual version) yang BISA dikerjakan sebelum rename.

- [x] **BR-F3 · R-32 · PR-64 (P2, L)** Channel regional Tier B (WeChat/QQ…) + feature flag.
- [x] **BR-F4 · R-33 · PR-94 (P2, M)** Pilih Tier B saat instalasi.
- [x] **BR-F5 · R-31 · PR-95 (P2, M)** Tier C WASI: pilot `channel-nostr-pilot` + `.frozen` v0 sudah ada; tinggal e2e penuh (host, signature, capability) dan ukur batas `wasi:http` outbound.

#### Detail Eksekusi — BR-F3/BR-F4/BR-F5

**TIDAK DIVERIFIKASI sesi ini** (channel regional dan nostr pilot di luar budget grep sesi ini). **Verifikasi awal:** `channel_nostr_pilot_e2e.rs` ADA di `crates/clawcrew-plugins/tests/` (2 ref ditemukan saat verifikasi BL-20/23) — konfirmasi SEBAGIAN klaim BR-F5 ("pilot sudah ada"), tapi "e2e penuh" dan "ukur batas wasi:http outbound" perlu verifikasi test coverage aktual di file tersebut sebelum eksekusi.

**Langkah pertama WAJIB tim eksekusi:** baca `crates/clawcrew-plugins/tests/channel_nostr_pilot_e2e.rs` penuh untuk tahu PERSIS apa yang sudah ditest vs yang masih gap (host/signature/capability masing-masing).

> **BR-F1 TIDAK dijadwalkan** — duplikat `GV-07` yang sudah `[x]` selesai PR-21 (lihat `README.md` §Reconciliation). Tidak ada kerja baru di sini, hanya perlu dicentang `[x]` di dev-tasks asli.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `crates/clawcrew-runtime/src/agent/spawn.rs` (atau setara), dashboard Tasks-view (`web/`) | Backend-Rust + Frontend |
| B | MCP client host (`crates/clawcrew-plugins/` atau setara — SAMA file dengan BL-19 Sprint 0, koordinasikan) | Backend-Rust |
| C | `wit/v1/`, `crates/clawcrew-channels/` Tier B/C | Backend-Rust |

## Keluaran Sprint

- Sub-agent spawn punya cap jumlah/memori — root cause crash tertutup.
- MCP governance (vault-mediation, permission wiring) terverifikasi, bukan asumsi.
- WIT v1 hidup berdampingan v0 tanpa breaking existing plugin.
