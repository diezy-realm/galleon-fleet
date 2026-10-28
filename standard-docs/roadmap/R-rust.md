# Lane R — Rust (System Core)

Ref: `decisions/refactoring-rust/phase-5-production-hardening/README.md` (W0–W5), `tech-standart/code-quality/`, `tech-standart/plugin-standard/`.
Build Windows: `$env:CARGO_TARGET_DIR="target/agent"`; test: `cargo nextest run -p <crate>`.

## R-0x Klasifikasi & gerbang kualitas

- [x] **R-00 (P0 — PRIORITAS UTAMA, ODR-11, M) — *draf selesai: `decisions/refactoring-rust/phase-5-production-hardening/R-00-runtime-inventory.md`; menunggu sign-off Owner/Principle (3 keputusan §6).*  Inventaris `clawcrew-runtime` vs boundary.** Konteks terukur: `agent` 77k, `sop` 38k, `rpc` 32k, `skills` 12k, `cron` 10k, `control_plane` 7k LOC masih dipakai `src/main.rs` dan `gateway/{api,api_config,ws,a2a}.rs` (`foundation/05`). Keluaran per modul: keputusan + daftar pemanggil + tiket tujuan (G-30..G-33 bila pindah ke Go). Per modul (agent, sop, subagents, cron, memory_strategy, skills, hooks, tools, security, i18n, observability, rpc…) tetapkan: *keep-in-Rust* / *move-to-Go* / *delete* / *feature-gate*, dengan bukti pemakai (siapa memanggil). Output: tabel di `decisions/refactoring-rust/phase-5-production-hardening/R-00-runtime-inventory.md`. **Tidak ada penghapusan tanpa sign-off Owner.** Acceptance: tiap top-level modul punya keputusan + alasan; daftar dependen terukur. Out: eksekusi pemindahan.
- [x] **R-01 (P1, S) `[workspace.lints]`** [Selesai PR-10] `[workspace.lints.clippy]` (unwrap_used/expect_used = warn) dan `[workspace.lints.rust]` (unsafe_code = warn) di root `Cargo.toml`; `lints.workspace = true`.
- [ ] **R-02 (P1, M) Burn-down `unwrap/expect` produksi** (±400 hit; runtime 237). Per crate, mulai `gateway`, `channels`. Dep: R-01. Acceptance: nol hit di jalur produksi atau invariant terdokumentasi.
- [x] **R-03 (P1, S) Gerbang ukuran file/crate** [Selesai PR-10] Ditambahkan `tests/architecture/module_size_limits.rs` dengan daftar pengecualian bertanggal 2026-10-04 (ratchet lock).
- [x] **R-04 (P1, S) Gerbang arah dependency antar-crate** [Selesai PR-10] Ditambahkan `tests/architecture/dependency_direction.rs` mengunci leaf boundary `clawcrew-api` dan isolasi channel/plugins dari `runtime`.
- [x] **R-05 (P2, S) Selaraskan `rust-conventions.md`** [Selesai PR-20] MSRV 1.96.0 dan jumlah 27 crates diselaraskan dengan root `Cargo.toml`. Dep: –.
- [x] **R-06 (P2, S) Ukur waktu build/test baseline** [Selesai PR-10] Waktu cold check (7m 02s), warm check (3.78s), dan gate checks diukur dan disimpan di `foundation/03`.

## R-1x Putus kopling & pecah crate (setelah R-00)

- [x] **R-10 (P1, L) Ekstrak `i18n`** [Selesai PR-31] Dependensi `galleon-i18n` dikonfigurasi di `clawcrew-channels` dan seluruh 87 pemanggilan dialihkan dari runtime ke `galleon_i18n`.
- [x] **R-11 (P1, L) Ekstrak `security::scrub`** [Selesai PR-32] 36 ref di channels dan gateway diekstrak ke crate `galleon-security-scrub`; runtime men-reexport untuk backward-compatibility.
- [x] **R-12 (P1, L) Pisahkan kontrak tools-scoped / memory_strategy / hooks / sop::types** yang dipakai channels menjadi trait di `clawcrew-api`. [Selesai PR-33] Kontrak diekstrak ke `clawcrew-api` (`ToolRegistry`, `ScopedToolRegistry`, `ChannelHookRunner`, tipe-tipe enum/struct kanonikal `sop`), `DefaultMemoryStrategy` dipindahkan ke `galleon-memory`, dan implementasi adapter runtime diselaraskan. Dep: R-00.
- [ ] **R-13 (P1, L) `channels` tidak bergantung ke `runtime`** (677 ref → 0). Dep: R-10..R-12. Acceptance: edge dihapus di `cargo metadata`; gerbang R-04 mengunci.
- [ ] **R-14 (P1, L) `gateway` tidak bergantung ke `runtime`; hapus REST/WS/A2A lama bila tak ada pemakai (ODR-14)** (426 ref; cron, observability). Dep: R-00. Bila cron pindah ke Go: koordinasi G-lane.
- [ ] **R-15 (P2, XL→dipecah) Pecah `channels` (198k LOC)** per tier/protokol: crate `galleon-channel-<nama>` untuk channel baru, bertahap untuk lama. Dep: R-13.
- [ ] **R-16 (P2, XL→dipecah) Pecah/ramping `runtime` (286k LOC)** sesuai hasil R-00 (move/delete/gate). Dep: R-00, sign-off Owner.
- [ ] **R-17 (P2, M) Pecah file terbesar** yang tetap tinggal di Rust (daftar dari R-03).
- [ ] **R-18 (P3, M) `tools` (85k) & `config` (85k) & `providers` (84k):** audit pemisahan sub-domain; jangan pecah spekulatif (YAGNI).

## R-2x Hardening & kontrak

- [x] **R-20 (P1, M) Implementasi nyata `execute_task`** [Selesai PR-35] Menghapus stub sukses palsu; menggantikannya dengan fail-closed sejati (`Status::unimplemented`) dengan logging failure dan validasi input.
- [x] **R-21 (P1, M) Test integrasi gRPC Go↔Rust** [Selesai PR-35] Test integrasi lengkap di Go client (`ExecuteNativeTool`, `GetDecryptedSecret`, `ExecuteTurn` stream, `ExecuteTask` fail-closed, dan `UnreachableServer`) dan unit test Rust gateway lolos.
- [ ] **R-22 (P1, M) Sandbox:** test Landlock/Seatbelt/AppContainer per OS; kebijakan deny-by-default terbukti.
- [ ] **R-23 (P2, S) Scrubbing kredensial di log** — test tangkap-log untuk `ExecuteTurn`/vault.
- [ ] **R-24 (P2, M) Vault:** uji rotasi/ekspor/migrasi; `GetDecryptedSecret` hanya untuk caller terotorisasi.

## R-3x Plugin & channel tier

- [ ] **R-30 (P1, S) Errata ADR-001** (WhatsApp → Tier A, WIT v0 beku, bluesky) — lihat D-04; perbarui `plugin-codebase-standard.md`.
- [ ] **R-35 (P1, M) Dukungan WIT v1** di host plugin (dual v0/v1) — pasangan N-24.
- [ ] **R-31 (P2, L) Pilot channel Tier C WASI** end-to-end (host, signature, capability) pada fixture; ukur batas `wasi:http` outbound. *Status 2026-10-04: fixture `channel-nostr-pilot` + `wit/v0/.frozen` ada; sisa e2e & ukur `wasi:http`.*
- [ ] **R-32 (P2, S) WeChat/QQ/dst. = Tier B** (ODR-13, regional): buat crate `galleon-channel-<nama>` + feature flag; catat di errata ADR-001.
- [ ] **R-33 (P2, M) Pasang mekanisme pilih-Tier-B saat instalasi** (feature flags / installer). Dep: P-lane installer.
- [ ] **R-34 (P1, M) CLI (`galleon`, `galleoncode` TUI) = binary Rust, klien Go untuk orkestrasi (ODR-12)** — `chat/run/approve/status` memanggil Go :9090 (SSE/WS); `service/doctor/onboard/vault` tetap lokal Rust; tanpa agent loop lokal. Desain di `foundation/05`. Dep: R-00, G-lane API.

- [ ] **R-37 (P2, M) Retarget `galleon-eval`** (replay deterministik) dari `runtime::agent::Agent` ke jalur Go (`StartTurn` + mock `ExecuteTurn`) setelah G-30; pertahankan fixture trace. `galleon-sop-graph` mengikuti keputusan modul `sop` di R-00 (tipe wire → kontrak proto bila SOP pindah). Dep: R-00, G-30.
- [ ] **R-38 (P2, S) Hapus `verifiable_intent`** (ODR-21; buat tag `archive/pre-galleon` dulu, penilaian di `archive/research-closed-loop/verifiable-intent-assessment.md`) (modul, tool `vi_*`, `VerifiableIntentConfig`, peringatan di `main.rs`/doctor); config lama diabaikan dengan peringatan sekali. Dep: sign-off Owner. Ref R-00a §1.
- [ ] **R-39 (P1, M) Auth pemanggil Go di Rust:** satu provider (loopback token/peer-uid/mTLS) menggantikan `principal_resolver`/`auth_provider`/`webauthn` setelah G-35. Dep: G-35. Ref R-00a §2.

- [ ] **R-40 (P1, L) Retarget `galleoncode` (TUI)** ke Galleon Client Protocol (Go); matikan mode agent daemon-ephemeral (`spawn_owned_ephemeral_daemon`). Dep: G-36.
- [ ] **R-42 (P1, M) Pertahankan & retarget `galleon-acp-bridge`** (ODR-24): ACP ke Galleon Client Protocol; dokumentasikan integrasi editor. Dep: G-36. (P1, L) Retarget `galleoncode` (TUI) & `galleon-acp-bridge`** ke Galleon Client Protocol (Go); matikan mode agent daemon-ephemeral. Dep: G-36.
- [ ] **R-41 (P2, L) Hapus `rpc` + REST/WS/A2A lama** setelah parity (eval + test kontrak). Dep: R-40, R-14.
- [ ] **R-36 (P1, M) Audit galleoncode (TUI) & galleonrelay:** petakan tiap pemanggilan ke daemon/runtime Rust (chat, ACP, agent sidebar, enroll) → endpoint Go; TUI tetap sebagai antarmuka CLI. Relay: jelaskan peran vs Nginx (relay = tembus NAT tanpa dekripsi; Nginx = TLS terminator di VPS) dan masuk P-11 panduan deploy.

## R-5x Sub-agent resource management & observability

Ref: `research/subagent-resource-management-audit.md`, `backlog/subagent-resource-management-backlog.md` (BL-15..BL-17). Konteks: Owner melaporkan crash nyata dari spawn ~20 sub-agent sekaligus tanpa cek memori; audit kode membuktikan tidak ada resource-gate sama sekali (bukan asumsi), sementara mesin deteksi stuck (`LoopDetector`) sudah matang tapi hasilnya tidak pernah sampai ke dashboard.

- [ ] **R-50 (P0 — gap crash nyata, M) Resource gate sebelum spawn sub-agent (BL-15).** Tambah `resource_limits.max_total_memory_mb`/`max_total_processes` di `clawcrew-config`; gate di `DelegateTool` sebelum `clawcrew_spawn::spawn!` memakai `process_stats::sample()` + hitung task `Running` kind `Delegate` di `control_plane::TaskRegistry`. Aggregate ceiling (bukan per-sub-agent), adaptif, menyusul pola Kiro Crew `resource_status`. *Catatan 2026-10-05: scope ini HANYA menjaga jalur `DelegateTool`; jalur kedua (`ExecuteTurn` dari Go `spawn_subagent`, R-53/BL-18) ditunda sampai BG-D1/BG-D4 — R-50 tetap bisa jalan independen sekarang, cukup didokumentasikan bahwa ini TIDAK menutup R-53's gap.* Dep: –. Acceptance: test membuktikan spawn ditolak/di-queue saat cap terlampaui, bukan silent-allow; task yang di-queue tereksekusi begitu kapasitas tersedia.
- [ ] **R-51 (P1, M) Surface `loop_detector_circuit_breaker` ke Tasks-view (BL-16).** Tambah `last_loop_detector_event` pada `TaskRecord` (`#[serde(default)]`), tulis dari `agent/turn/results_collect.rs` saat `Break` sebelum `anyhow::bail!`, expose ke endpoint Tasks-view existing (Go) — reuse, jangan bikin endpoint baru. Dep: –. Acceptance: sub-agent yang kena circuit-breaker muncul di Tasks-view dengan alasan spesifik, bukan status "Failed" generik.
- [ ] **R-52 (P2, exploratory, L) Agregasi lintas-sub-agent untuk fan-out besar (BL-17).** Korelasi "N dari M sub-agent paralel kena pola stuck serupa" sebagai satu insight. Dep: R-51 (butuh data individual tersurface dulu). Perlu design-spec terpisah sebelum breakdown — jangan masuk sprint sebelum itu.
- [ ] **R-53 (DITUNDA — keputusan Achmad 2026-10-05, lihat `backlog/subagent-resource-management-backlog.md` BL-18) Gate resource di `ExecuteTurn` (BL-18).** Terverifikasi CRITICAL secara teknis: `grpc_system_gateway.rs::execute_turn` nol resource gate (tidak ada semaphore/cek `process_stats`/cek sesi aktif) — jalur yang dipakai Go `spawn_subagent` via gRPC. **TAPI eksekusi DITUNDA sengaja** sampai tim dev-tasks menyelesaikan BG-D1/BG-D4 (`finalize/dev-tasks/01-be-go-engine.md` — strangler `clawcrew-runtime` → Go, status `[ ]` belum dikerjakan) — supaya gate didesain pas dengan bentuk final RPC boundary pasca-migrasi, bukan dibangun 2x. Status item ini: riset+backlog matang untuk dirujuk nanti, BUKAN tiket aktif — jangan assign ke squad sebelum BG-D1/BG-D4 bergerak.

## R-6x Plugin & MCP security hardening

Ref: `research/plugin-mcp-architecture-audit.md`, `archive/research-closed-loop/fleet-wide-mcp-governance-followup.md`, `backlog/plugin-mcp-architecture-backlog.md` (BL-19..BL-24). Konteks: audit komparatif vs Antigravity/Kiro Crew/GitHub Copilot/Hermes Agent/OpenClaw menemukan MCP client Galleon (`galleon-tools`) punya scoping LLM-level matang tapi NOL sandboxing OS-level untuk proses MCP server itu sendiri — kontras tajam dengan Plugin WASM/WIT (`galleon-plugins`) yang sandboxing-nya sangat matang (Wasmtime fuel/memory limit, egress deny-by-default).

- [ ] **R-60 (P0 — gap keamanan HIGH, belum pernah diangkat tim mana pun, M-L) Sandbox MCP stdio child process (BL-19).** `StdioTransport::spawn()` (`galleon-tools/src/mcp_transport.rs`) memanggil `Command::new().spawn()` langsung tanpa `Sandbox` trait yang sudah ada dan dipakai `ShellTool`. Reuse trait existing, scoped ke workspace_dir + whitelist eksplisit per-server, dengan opt-out eksplisit (`sandbox_profile` per-`McpServerConfig`) karena beberapa MCP server resmi legitimately butuh akses luas. Dep: –. Acceptance: MCP server default di-sandbox ke workspace; operator bisa opt-out eksplisit; akses di luar workspace tanpa opt-out ditolak OS-level.
- [ ] **R-61 (P1, S, interim sebelum R-60 selesai) Dokumentasikan gap sandbox MCP sebagai security disclosure (BL-20).** Doc comment `McpServerConfig` + dokumentasi operator menyatakan eksplisit MCP stdio server saat ini TIDAK di-sandbox. Dep: –, bisa dikerjakan sebelum R-60 selesai sebagai mitigasi sementara.
- [ ] **R-62 (P2, M) Evaluasi vault-mediation untuk secret MCP env/headers (BL-21).** `McpServerConfig.env`/`.headers` saat ini plaintext + `#[secret]` UI-hide saja, tidak lewat `SecretStore`/vault seperti pola `cron_secret_request`. Dep: keputusan desain dulu (apakah MCP config layak se-strict cron, beda threat model).
- [ ] **R-63 (P2, S, dokumentasi) ADR pembanding WASM Plugin vs MCP (BL-22).** 0 referensi MCP ditemukan di seluruh `crates/galleon-plugins/` dan ADR-006/009/014 — pembagian use-case hanya tersimpul implisit. Dep: –.
- [ ] **R-64 (P2, S, quick verification) Verifikasi wiring `PluginPermission::FileRead`/`FileWrite` (BL-23).** Dideklarasikan di `galleon-plugins/src/lib.rs`, tidak ditemukan konsumsi nyata di `host.rs`/`component.rs`. Dep: –. Acceptance: ditandai "not yet enforced", dihapus (YAGNI), atau diimplementasi — salah satu, dengan keputusan terdokumentasi.

## R-7x Provider reliability, OAuth, & memory data governance

Ref: `research/providers-reliability-auth-audit.md`, `research/memory-data-governance-audit.md`, `research/oauth-client-id-opensource-implications.md`, `backlog/providers-memory-governance-backlog.md` (BL-25..BL-31). Konteks: audit pertama kali untuk `clawcrew-providers` (LLM SSOT, belum pernah diriset) dan data governance `galleon-memory` (retention/privacy, berbeda dari "Learning by Consent" yang sudah diriset). Relevan langsung dengan keputusan open-source yang Owner sedang pertimbangkan.

- [ ] **R-70 (P2, M) BYOC — override `client_id` OAuth xAI/OpenAI (BL-25).** Hardcoded `XAI_OAUTH_CLIENT_ID`/`OPENAI_OAUTH_CLIENT_ID` dipakai langsung tanpa jalur override, kontras dengan Gemini/Email OAuth2 yang sudah terima parameter. PKCE sudah benar (bukan kelalaian keamanan) — risiko murni operasional (satu client_id dipukul semua instalasi). Dep: –. Prioritas naik kalau open-source diputuskan.
- [ ] **R-71 (P1, S) Dokumentasikan model keamanan single-tenant auth (BL-26).** Satu master key + satu token store per instalasi untuk semua Crew/Agent. Dep: –, bisa dikerjakan sekarang.
- [ ] **R-72 (P1, S) Perbaiki UX pesan error OAuth refresh-revoked (BL-27).** String error mentah provider → instruksi jelas re-auth. Dep: –.
- [ ] **R-73 (P2, M, evaluasi dulu) Client-side rate limiting LLM provider calls (BL-28).** Hanya reaktif (cooldown post-429) saat ini. Dep: disarankan setelah R-50 (resource gate sub-agent) untuk baseline evaluasi.
- [ ] **R-74 (P2, M) Lengkapi test reliability: cascading-all-down & cooldown-expiry (BL-29).** `reliable/tests.rs` kuat di billing, lemah di 2 skenario ini. Dep: –.
- [ ] **R-75 (P1, L) Fungsi `erase_identity()` lintas-backend untuk memory (BL-30).** Tidak ada single entry point yang menyapu semua kategori+backend+audit+snapshot untuk satu identitas. Dep: –.
- [ ] **R-76 (P2, L, scope besar — evaluasi trait dulu) Hygiene/retensi otomatis backend-agnostic (BL-31).** `hygiene.rs` hardcoded SQLite-only; Postgres/Qdrant tidak dapat retensi otomatis setara. Dep: disarankan setelah R-75 untuk konsistensi pola.

## R-8x Channel signature bug, Windows signing, & observability content PII

Ref: `research/channel-signature-verification-audit.md`, `research/cicd-supplychain-audit-partial.md`, `research/observability-content-pii-audit.md`, `backlog/channel-cicd-observability-backlog.md` (BL-32..BL-39). Konteks: audit berlapis (termasuk koreksi 2x atas kesimpulan awal yang keliru) menemukan bug nyata di jaring pengaman webhook WhatsApp, gap signing Windows, dan gap privasi konten percakapan di logging.

- [ ] **R-80 (P0 — bug nyata, terverifikasi independen, M) Perbaiki/pulihkan `verify_whatsapp_signature` & `optional_channel_routes` yang hilang (BL-32).** Grep seluruh repo = nol definisi untuk kedua fungsi yang dirujuk test/drift-guard. Jalankan `cargo test -p clawcrew-gateway --features channel-whatsapp-cloud` untuk ground-truth SEBELUM mengerjakan apa pun lain di section ini. Dep: –, PRIORITASKAN di atas semua item lain.
- [ ] **R-81 (P1, HIGH, M) Pisahkan policy technical vs conversational tool I/O logging (BL-37).** `log_tool_io=Redacted` hanya truncate, bukan content-redact; konten percakapan (`send_message`, dll) ter-log penuh ke disk. Reuse pola `LlmRequestPayloadPolicy` yang sudah benar. Dep: –.
- [ ] **R-82 (P1, M) Konfirmasi/perbaiki referensi `tauri.bundled.conf.json` yang hilang (BL-34).** Dirujuk 3× tapi tidak ditemukan di 2 audit independen — 3 job desktop build berisiko gagal. Dep: –, cek SEGERA (murah).
- [ ] **R-83 (P1, M) Terapkan scrub_credentials di jalur record! tool I/O (BL-38).** Scrub saat ini hanya di jalur observer, bukan di jalur yang menulis ke disk. Dep: independen dari R-81, bisa paralel.
- [ ] **R-84 (P2, M) Proteksi permission file log Windows (BL-39).** `0o600` hanya `#[cfg(unix)]`, nol proteksi eksplisit Windows. Dep: –.
- [ ] **R-85 (P1, HIGH-UX, L, butuh pengadaan sertifikat) Windows Authenticode signing (BL-33).** Binary Windows unsigned → SmartScreen warning tiap instalasi. Dep: kemungkinan terkait R-82 (lokasi config signing).
- [ ] **R-86 (P2, S) Pin esp-idf-* ke rev spesifik (BL-35).** Git dependency firmware tanpa rev — supply-chain risk. Dep: –.
- [ ] **R-87 (P2, S) Cakupan cargo deny untuk firmware (BL-36).** `deny.toml` root tidak audit `firmware/esp32`/`nucleo` (Cargo.lock terpisah). Dep: –.

## R-8x (lanjutan) Token/Credit Efficiency (Prompt Caching & Trim Coordination)

Ref: `research/token-efficiency-codebase-audit.md`, `backlog/token-efficiency-backlog.md` (BL-45..47). Konteks: Owner melaporkan Kiro Crew terasa 30-50% lebih hemat kredit vs Copilot/Hermes (observasi pribadi, tanpa benchmark publik); riset pembanding industri (Anthropic/OpenAI/Gemini prompt caching, Cursor Merkle-tree indexing, Copilot embedding retrieval) dicocokkan ke kode nyata. Dua hipotesis awal (aktifkan `cache_passthrough`; niru `defer_loading` MCP Claude Code) DICABUT setelah audit membuktikan keduanya bukan gap nyata di arsitektur ini — native Anthropic provider sudah cache otomatis tanpa toggle, dan MCP server dimuat statis per-proses tanpa hot-reload. Gap nyata: rolling cache-breakpoint (`compatible/mod.rs::apply_cache_breakpoints`, native setara di `anthropic/mod.rs`) tidak terkoordinasi dengan `trim_to_recent_turns_with_crumb` (`history_trim.rs`) — trim membuang turn dari tengah riwayat, mengubah prefix yang sebelumnya di-cache, memaksa cache-miss (bayar premium cache-write 1.25-2x) pada request pasca-trim pertama.

- [ ] **R-88 (P2, S) Ukur korelasi trim-history vs cache-miss sebelum membangun mitigasi (BL-45a).** Tambah logging/metric yang mencatat pasangan (trim_event_timestamp, next_request `cache_read_input_tokens`) — titik integrasi `crates/clawcrew-runtime/src/agent/turn/context_recovery.rs` (event trim) + `crates/clawcrew-providers/src/dispatch/accounting.rs` (korelasi cache_read request berikutnya). Dep: –. Acceptance: data nyata dari minimal 1 sesi uji; `DEFAULT_MAX_HISTORY_MESSAGES=50` (`agent/history.rs:13`) membuat trim jarang terjadi — JANGAN bangun R-89 sebelum data ini mengonfirmasi dampaknya signifikan secara biaya, bukan asumsi.
- [ ] **R-89 (P3, M, dependen R-88) Align titik potong trim dengan rolling cache-breakpoint (BL-45b).** Hanya dikerjakan JIKA R-88 menunjukkan dampak biaya signifikan. Opsi: trim pada boundary yang align breakpoint sebelumnya, atau terima cache-miss satu kali per-trim sebagai trade-off sadar yang didokumentasikan (bukan silent cost). Dep: R-88.
- [ ] **R-90 (P2, S) Surface cache hit-rate ke dashboard/CLI operator (BL-46).** `cache_read_input_tokens`/`cache_creation_input_tokens` sudah ditrack penuh sampai `clawcrew-config/src/cost/types.rs:11-23` — tambah titik tampilan (dashboard ATAU `kirocrew doctor`/`cost` command, cek `clawcrew-api` field yang sudah terekspos dulu) yang menyurfaceкan rasio cache-hit per sesi. Dep: setelah R-88 (minimal) supaya angka yang ditampilkan sudah termasuk konteks trim-event, bukan angka telanjang. Acceptance: operator melihat cache-hit-rate tanpa membaca log mentah.
- [ ] **R-91 (P3, S) Dokumentasikan prinsip "steering stabil = cache-friendly" (BL-47).** Steering file (`inclusion: always`) sudah cache-friendly secara struktural (efek samping arsitektur) — tambah satu paragraf eksplisit di `AGENTS.md` atau dokumen operator-facing setara: jangan ubah steering file inti di tengah sesi aktif (membatalkan prompt cache byte-exact-prefix-match untuk seluruh sesi). Dep: –. Acceptance: paragraf tersimpan, menyebut mekanisme prefix-match sebagai alasan.

## R-9x (lanjutan) Knowledge-Base Citation & Library Activation

Ref: `research/rag-vector-knowledge-base-audit.md`, `archive/research-closed-loop/perplexity-web-browsing-methodology.md`, `backlog/knowledge-base-citation-backlog.md` (BL-48..51). Konteks: Owner menyetujui kombinasi pola GitHub Copilot Memory (citation-based verification) + Kiro Crew Knowledge Library (chunk/index/embed hybrid) setelah audit menemukan `query_by_similarity` di `knowledge_graph.rs` sebenarnya FTS5 keyword search (nama menyesatkan) dan `clawcrew-api/src/knowledge.rs` adalah skema lengkap tapi nol dipanggil (dead code). Riset Perplexity AI (web browsing/akademik) murni jadi referensi — hanya 2 pola kecil (query decomposition, domain-filtering) diadopsi ke SOP, bukan ke kode.

- [ ] **R-92 (P1, M) Field citation + verifikasi pada node Lesson/Decision di knowledge graph (BL-48).** Tambah `citations: Vec<Citation>` ke `KnowledgeNode` (`crates/galleon-memory/src/knowledge_graph.rs`) dengan `file_path`/`line_range`/`content_hash`; fungsi `verify_citations()` membaca ulang lokasi dan bandingkan hash, status `Verified`/`Stale`/`Missing` disurfaceкan di output `search`/`graph_neighbors` (`galleon-tools/src/knowledge_tool.rs`), tidak disembunyikan. Dep: –. Acceptance: node bersitasi ke kode yang berubah terdeteksi `Stale`, bukan dipercaya diam-diam — mitigasi langsung kelas bug "configuration-lies" (WhatsApp signature, capture_tool_input, query_by_similarity — 3x ditemukan sesi ini).
- [ ] **R-93 (P1, L) Aktifkan Knowledge Library untuk `standard-docs/` (BL-49).** Wiring nyata untuk tipe `KnowledgeSource`/`KnowledgeDocument`/`KnowledgeChunk`/`IndexJob` yang sudah ada di `crates/clawcrew-api/src/knowledge.rs` (saat ini dead code, nol pemanggil) — pipeline chunk→index→embed REUSE `EmbeddingProvider` trait existing (`galleon-memory/src/embeddings.rs`, jangan bangun provider baru), fallback FTS5 keyword saat model tak tersedia (pola hybrid graceful-degradation ala Kiro Crew Knowledge Library). Target awal: index `standard-docs/` supaya riset/backlog/roadmap yang menumpuk searchable secara semantic. Dep: setelah R-92 untuk koherensi desain (opsional, tidak hard-block). Acceptance: search atas `standard-docs/` fixture mengembalikan chunk relevan; matikan embedding provider → fallback keyword tetap berfungsi.
- [ ] **R-94 (P2, S) Query decomposition + domain-filtering sebagai SOP riset squad (BL-50).** Murni dokumentasi (bukan kode) — tambah ke steering/SOP riset: pecah pertanyaan kompleks jadi sub-query sebelum `web_search`/`web_fetch` batch; gunakan filter domain (`site:arxiv.org`, `site:*.edu`) langsung di query string untuk riset akademik (provider AnySearch/Serply sudah support native, tidak perlu field baru). Dep: –. Acceptance: paragraf SOP tersimpan dengan contoh query konkret.
- [ ] **R-95 (P3, DITUNDA) Koordinasi dengan BG-B4 dev-tasks (BL-51).** `BG-B4`/`G-13` (dev-tasks `01-be-go-engine.md`) adalah vector MEMORY Go per-scope Fleet/Ship/Squad/Crew/Run — scope berbeda dari R-92/R-93 (Rust, knowledge graph + library) tapi sama domain RAG/vector. **DITUNDA sengaja** (keputusan Owner 2026-10-05: "mungkin sudah dikerjakan, bereskan kembali setelahnya") — jangan assign ke squad. Saat diangkat nanti: cek status `BG-B4` di dev-tasks, pastikan desain Go dan Rust tidak membangun filosofi RAG/vector yang inkonsisten tanpa alasan terdokumentasi. Dep: `BG-B4`/`BG-B2` selesai (tim lain).








