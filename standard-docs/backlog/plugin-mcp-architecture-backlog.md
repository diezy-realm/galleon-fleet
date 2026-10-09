---
title: Backlog — Plugin & MCP Architecture Gaps
status: backlog
date: 2026-10-05
author: squad-lead
depends_on: plugin-mcp-architecture-audit.md, fleet-wide-mcp-governance-followup.md
---

# Backlog — Plugin & MCP Architecture Gaps

> Diturunkan dari `research/plugin-mcp-architecture-audit.md` dan `archive/research-closed-loop/fleet-wide-mcp-governance-followup.md`.
> Nomor lanjutan dari BL-18 (`backlog/subagent-resource-management-backlog.md`).

---

## BL-19 — Sandbox MCP Stdio Child Process (PRIORITAS TERTINGGI, HIGH)

**Rujukan riset:** `plugin-mcp-architecture-audit.md` §2.3, §5 poin 1-2; cross-check `fleet-wide-mcp-governance-followup.md` §8.3 (dikonfirmasi: topik ini **belum pernah diangkat tim dev-tasks mana pun** — bukan sesuatu yang sudah "ditangani di tempat lain").

**Masalah (terverifikasi langsung ke kode, bukan asumsi):** `StdioTransport::spawn()` (`crates/clawcrew-tools/src/mcp_transport.rs`) memanggil `Command::new(&config.command)...spawn()` **langsung**, tanpa pembungkus `Sandbox` trait yang sudah ada di `crates/clawcrew-runtime/src/security/traits.rs` dan dipakai konsisten oleh `ShellTool` (`crates/clawcrew-runtime/src/tools/shell.rs:61,65` — eksplisit menerima `Arc<dyn Sandbox>`). Tidak ada Landlock/Bubblewrap/Firejail/Seatbelt di sekitar spawn MCP sama sekali.

**Dampak:** MCP spec resmi mengasumsikan server punya full OS access level user yang menjalankan proses. Begitu operator memberi agent satu `mcp_bundle`, proses MCP server itu berjalan dengan privilege PENUH user daemon Galleon — bisa baca/tulis file arbitrary di luar workspace, buka koneksi jaringan bebas, fork proses lain — tanpa containment OS apa pun. Permission-scoping yang matang di level LLM (bundle-level + tool-level + deferred-loading, lihat BL-20) HANYA mengatur tool mana yang boleh LLM panggil — bukan apa yang proses MCP server itu sendiri bisa lakukan ke sistem begitu dia hidup.

**Kontras arsitektur yang mencolok:** Plugin WASM/WIT (`clawcrew-plugins`) — sistem ekstensibilitas SEJENIS yang membuka pintu ke kode pihak ketiga — justru dikontain sangat ketat (Wasmtime fuel limit, memory/table limit, egress deny-by-default). MCP (yang menurut spec protokolnya SENDIRI mengasumsikan full-trust, jadi secara desain LEBIH berisiko) mendapat containment PALING LONGGAR dari keduanya. Ini kebalikan dari logika risk-based security.

**Scope implementasi:**
1. Reuse `Sandbox` trait yang SUDAH ADA (`security/traits.rs`) — jangan bangun mekanisme sandbox baru.
2. Bungkus `Command::new(&config.command)` di `StdioTransport::spawn()` dengan sandbox, scoped ke `workspace_dir` + whitelist jalur eksplisit per-server (karena beberapa MCP server resmi legitimately butuh akses luas, misal filesystem MCP server — tidak semua MCP server bisa langsung dibatasi ke workspace tanpa breaking).
3. **Opt-out eksplisit per-`McpServerConfig`** (field `sandbox_profile` atau serupa) — BUKAN blanket enforce tanpa jalan keluar, karena ini one-way-door yang bisa breaking existing config operator yang MCP server-nya legitimately butuh akses luas.
4. `#[serde(default, skip_serializing_if = ...)]` untuk field baru supaya tidak merusak config existing — pola yang sudah konsisten dipakai di `schema.rs`.

**File ownership:** Backend-Rust — `crates/clawcrew-tools/src/mcp_transport.rs` (titik spawn), `crates/clawcrew-config/src/schema.rs` (field `McpServerConfig` baru).

**Definition of Done:** Test baru yang membuktikan: (a) MCP stdio server default-nya di-sandbox scoped ke workspace; (b) operator bisa opt-out eksplisit per-server dengan config yang jelas; (c) MCP server yang mencoba akses path di luar workspace tanpa opt-out ditolak OS-level (bukan hanya ditolak di level tool-call Rust).

**Dependency:** Tidak ada blocker teknis — infrastruktur `Sandbox` trait sudah matang dan battle-tested di `ShellTool`.

**Risiko/Severity:** **HIGH** — gap keamanan nyata di jalur yang sudah dipakai produksi (bukan fitur belum dirilis), kontras tajam dengan standar sandboxing yang sudah ditegakkan di seluruh `clawcrew-runtime/src/security/` untuk tool lain.

---

## BL-20 — Dokumentasikan Gap Sandbox MCP sebagai Security Disclosure (interim, sebelum BL-19 selesai)

**Rujukan riset:** `plugin-mcp-architecture-audit.md` §6 poin 2.

**Masalah:** Sampai BL-19 diimplementasi (butuh waktu — one-way-door, perlu testing matang supaya tidak breaking existing MCP server), operator yang menambah `mcp_bundle` pihak ketiga TIDAK PUNYA cara untuk tahu bahwa ini memberi akses OS penuh ke proses tersebut.

**Scope:** Tambah 1 paragraf eksplisit di dokumentasi operator/config schema (`McpServerConfig` doc comment + README/deployment guide) yang menyatakan: *"MCP stdio servers currently run WITHOUT OS-level sandboxing — the spawned process has full privileges of the user running the Galleon daemon. Only configure MCP servers from sources you trust completely."* Ini MURNI dokumentasi, bisa dikerjakan SEKARANG tanpa menunggu BL-19.

**File ownership:** Siapa saja (Backend-Rust atau Docs) — `crates/clawcrew-config/src/schema.rs` (doc comment `McpServerConfig`), dokumentasi operator terkait.

**Definition of Done:** Disclosure eksplisit ada di minimal 2 tempat: doc comment struct, dan dokumentasi operator-facing (README/deployment guide MCP).

**Dependency:** Tidak ada — independent, bisa dikerjakan hari ini.

**Risiko/Severity:** LOW implementasi, tapi MEDIUM impact — mitigasi sementara yang murah sebelum fix teknis (BL-19) selesai.

---

## BL-21 — Secret Injection MCP: Plaintext Env/Headers Tanpa Vault-Mediation

**Rujukan riset:** `plugin-mcp-architecture-audit.md` §2.4.

**Masalah:** `McpServerConfig.env`/`.headers` ditandai `#[secret]` (hanya menyembunyikan dari UI/schema export saat serialize/display), tapi disuntik **plaintext langsung** dari config ke child process (`.envs(&config.env)` di `mcp_transport.rs`). Tidak ada indirection vault seperti pola `cron_secret_request` yang sudah ada di tool lain pada ekosistem ini (lihat tool `cron_secret_request` — mapping env-var ke vault secret name, butuh approval operator, pinned ke kode cron saat request).

**Scope:** Evaluasi apakah MCP server config layak mendapat pola vault-mediated yang sama dengan cron secrets — env var MCP di-reference by name ke vault, bukan plaintext di config file. Ini butuh keputusan desain dulu (apakah MCP server config harus se-strict cron, mengingat MCP server biasanya dikonfigurasi manual oleh operator yang sudah punya akses config file secara langsung — beda threat model dari cron job yang mungkin didorong dari luar).

**File ownership:** TBD — tergantung hasil evaluasi desain. Backend-Rust kalau diputuskan perlu (`clawcrew-config/src/secrets.rs` kemungkinan tempat natural untuk reuse `SecretStore` yang sudah ada).

**Definition of Done (tahap evaluasi):** Dokumen keputusan: perlu/tidak perlu vault-mediation untuk MCP env/headers, dan kalau perlu, desain konkret reuse `SecretStore`.

**Dependency:** Tidak ada untuk evaluasi.

**Risiko/Severity:** MEDIUM — bukan vulnerability aktif (pola umum di banyak tool config MCP industri), tapi inkonsisten dengan standar vault yang sudah lebih matang di tool lain pada sistem yang sama.

---

## BL-22 — ADR Pembanding WASM Plugin vs MCP (Dokumentasi Arsitektur)

**Rujukan riset:** `plugin-mcp-architecture-audit.md` §3.6/§6 poin 3; temuan sub-agent: *"0 match untuk 'MCP' di seluruh `crates/clawcrew-plugins/` dan ADR-006/ADR-009/ADR-014."*

**Masalah:** Galleon punya DUA sistem ekstensibilitas (Plugin WASM/WIT dan MCP client) dengan filosofi keamanan yang sangat berbeda, tapi TIDAK ADA satu dokumen pun yang menyatakan eksplisit "kapan pakai yang mana." Pembagian use-case hanya bisa disimpulkan implisit dari sejarah migrasi channel (ADR-006: alasan migrasi ke WASM adalah hindari rebuild binary untuk tiap integrasi messaging baru).

**Scope:** Tulis ADR baru (atau tambahan ke ADR existing) yang menyatakan eksplisit:
- WASM Plugin: untuk channel/tool native performant yang mau dikontain ketat (sandboxing OS-level penuh), tidak mau rebuild binary, cocok untuk kapabilitas yang sering dipakai/reusable (channel messaging, tool berulang).
- MCP: untuk integrasi tool eksternal DINAMIS, standar industri, interop dengan ekosistem MCP yang luas (filesystem, GitHub, database, dll) — trade-off: belum di-sandbox (lihat BL-19), jadi untuk sumber yang benar-benar dipercaya operator.
- Kapan developer internal harus pilih WASM plugin baru vs MCP server existing untuk kebutuhan baru.

**File ownership:** squad-lead atau Backend-Rust (dokumentasi arsitektur) — `standard-docs/decisions/` atau `docs/book/src/contributing/architecture-map.md` setara.

**Definition of Done:** Satu dokumen yang bisa dirujuk developer baru tanpa perlu menebak dari membaca kode/ADR lama.

**Dependency:** Tidak ada — independent, murni dokumentasi.

**Risiko/Severity:** MEDIUM (dokumentasi, bukan bug) — tapi mencegah keputusan arsitektur yang salah di masa depan karena developer menebak.

---

## BL-23 — Verifikasi Wiring `PluginPermission::FileRead`/`FileWrite`

**Rujukan riset:** `plugin-mcp-architecture-audit.md` §3.5.

**Masalah:** Enum `PluginPermission` (`crates/clawcrew-plugins/src/lib.rs`) mendeklarasikan varian `FileRead`/`FileWrite`, tapi audit TIDAK menemukan konsumsi nyata di `host.rs`/`component.rs` yang menegakkan permission ini terhadap kapabilitas sungguhan. WIT world yang diaudit (`tool.wit`, `channel.wit`, `memory.wit`) juga tidak punya interface filesystem — kemungkinan permission ini reserved for future, belum pernah di-wire.

**Scope:** Verifikasi definitif: grep menyeluruh pemakaian `PluginPermission::FileRead`/`FileWrite` di seluruh `crates/clawcrew-plugins/` dan pemanggil eksternal. Kalau benar belum di-wire, salah satu dari: (a) tandai jelas di komentar enum sebagai "declared, not yet enforced" supaya operator tidak salah kira sudah aktif, atau (b) hapus varian kalau genuinely tidak akan dipakai (YAGNI), atau (c) implementasikan kalau ada use-case nyata yang menunggu.

**File ownership:** Backend-Rust — `crates/clawcrew-plugins/src/lib.rs`.

**Definition of Done:** Salah satu dari 3 opsi di atas dieksekusi, dengan keputusan terdokumentasi kenapa.

**Dependency:** Tidak ada — quick verification task.

**Risiko/Severity:** MEDIUM — "false sense of capability" kalau operator mengira field ini sudah aktif menegakkan sesuatu padahal belum.

---

## BL-24 — Governance MCP Fleet-Wide (Exploratory — Butuh Keputusan Arsitektur Owner Dulu)

**Rujukan riset:** `fleet-wide-mcp-governance-followup.md` (seluruh dokumen).

**Masalah (terverifikasi, termasuk koreksi atas asumsi awal yang salah):** Tidak ada mekanisme bagi Fleet-level (Go, kebijakan/organisasi) untuk melihat atau membatasi MCP server yang di-resolve di level Rust. `fleet/policies.go` yang awalnya diduga "tempat natural" **terbukti BUKAN** itu — `GetFleetPolicies` murni data generik read-only untuk display dashboard, tidak punya hubungan ke tool/MCP execution apa pun. Grep `mcp_server|McpServer` di seluruh `engine/` = 0 hasil — Go sama sekali tidak punya konsep MCP. Ini **dikonfirmasi sebagai keputusan arsitektur sadar** (bukan oversight): Owner sudah menyetujui `Q-2` (2026-10-04) menghapus `tool/mcp_client.go` Go dan membangunnya ulang murni di Rust.

**Dampak:** Kalau operator Crew lokal menambah `mcp_bundle` berbahaya/belum di-review ke config Rust-nya, tidak ada lapis Fleet/Go yang bisa menolak sebelum server itu tersambung — berbeda dari delegasi sub-agent yang sudah punya privilege-ceiling teruji (`TestDelegationPolicyCeiling`). Dibandingkan Kiro Crew (fail-closed MCP Registry) dan GitHub Copilot (`managed-settings.json` allowlist/denylist terpusat), Galleon belum punya mekanisme setara.

**Scope (exploratory, BUKAN langsung implementasi):** Dua jalur arsitektur mungkin, keduanya one-way-door yang perlu persetujuan Owner dulu:
1. RPC baru di `SystemGateway` proto (`ListMcpServers`/`GetMcpGovernancePolicy`) — Go baca state MCP dari Rust, terapkan kebijakan Fleet-level di atasnya tanpa perlu tahu detail teknis MCP.
2. Policy check murni di Rust tapi dengan config yang di-push dari luar (file policy terpisah yang di-sync dari Fleet, bukan config lokal per-instance) — lebih dekat pola Kiro Crew/GitHub Copilot, tidak butuh proto baru, tapi butuh mekanisme distribusi config (yang juga belum ada di Galleon).

**File ownership:** TBD sepenuhnya tergantung keputusan Owner — opsi 1 butuh Backend-Rust (proto) + Backend-Go (consumer); opsi 2 butuh desain mekanisme distribusi config baru (scope lebih besar, kemungkinan lintas-tim).

**Definition of Done (tahap ini):** Keputusan Owner: opsi 1, opsi 2, atau "belum perlu sekarang" — baru setelah itu breakdown task konkret dibuat.

**Dependency:** **WAJIB keputusan Owner dulu** — ini one-way-door yang menyentuh batas tanggung jawab Go/Rust yang sudah dikonfirmasi eksplisit (Go=kebijakan, Rust=eksekusi). Jangan assign ke squad sebelum ini diputuskan.

**Risiko/Severity:** MEDIUM-HIGH secara dampak keamanan jangka panjang (terutama kalau Galleon menyasar use-case multi-user/enterprise), tapi LOW urgency immediate dibanding BL-19 (yang sudah berdampak ke instalasi single-user hari ini).

---

## Urutan Eksekusi yang Disarankan

1. **BL-20 dulu** — murni dokumentasi, bisa selesai dalam hitungan jam, memberi mitigasi sementara sambil BL-19 digarap.
2. **BL-19** — fix teknis utama, prioritas tertinggi secara severity, tapi butuh desain opt-out yang matang supaya tidak breaking (estimasi lebih dari 1 hari kerja).
3. **BL-23** — quick verification, bisa paralel dengan BL-19/BL-20 (file berbeda, tidak overlap).
4. **BL-22** — dokumentasi arsitektur, bisa paralel kapan saja, tidak ada dependency teknis.
5. **BL-21** — butuh evaluasi desain dulu sebelum implementasi, tidak urgent (bukan vulnerability aktif).
6. **BL-24 terakhir** — exploratory, WAJIB keputusan Owner dulu sebelum breakdown lebih lanjut; jangan dikerjakan sampai ada jawaban.

## Catatan Proses

Sama seperti backlog lain: setiap item perlu diangkat ke `roadmap/*.md` (lane R untuk BL-19/20/21/22/23 yang murni Rust) sebelum di-assign ke squad — folder `backlog/` ini bukan pengganti `roadmap/`, hanya staging area riset→roadmap. BL-24 TIDAK diangkat ke roadmap sebagai tiket aktif sampai Owner memutuskan — tetap didokumentasikan di sini sebagai rujukan siap-pakai.
