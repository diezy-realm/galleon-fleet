---
title: Index Backlog — Peta Lengkap BL-01 s.d. BL-39
status: index
date: 2026-10-05
author: squad-lead
---

# Index Backlog `standard-docs/backlog/`

> `backlog/` BUKAN pengganti `roadmap/` — ini staging area antara riset (`research/`) dan roadmap resmi (`roadmap/*.md`). Setiap item di sini idealnya sudah/akan diangkat ke roadmap sebagai tiket `R-nn`/`G-nn` bernomor. Peta ini memberi satu pandangan lintas-file supaya nomor BL tidak bentrok dan status (aktif/ditunda/exploratory) terlihat jelas tanpa buka 7 file.

## Peta Lengkap per File

### `skills-steering-lessons-backlog.md` (BL-01 s.d. BL-05)
| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-01 | Jembatan gRPC: persist Lesson/Learning Go→Rust vector store | — | Aktif |
| BL-02 | GAP-01: rancang steering produk Galleon end-user | — | Aktif, perlu diskusi Owner (scope produk besar) |
| BL-03 | Dokumentasikan ekosistem `open-skills` existing | — | Aktif |
| BL-04 | Test skill/steering inheritance untuk sub-agent | — | Aktif |
| BL-05 | V10: satukan 2 jalur delegasi sub-agent paralel (`DelegateTaskTool` vs `spawn_subagent`) | **CRITICAL** | Aktif — prioritaskan di atas BL-01..04 |

### `sop-raci-role-quest-backlog.md` (BL-06 s.d. BL-09)
| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-06 | Field Do's/Don'ts terstruktur di `SopStep` | — | Aktif |
| BL-07 | Field RACI lengkap di `Sop` struct | — | Aktif |
| BL-08 | Template role per-Crew (Backend/Frontend/QA) | — | Aktif |
| BL-09 | Matangkan `Quest` (Go) + `mission.Service` baru | — | Aktif |

### `automation-cron-webhook-backlog.md` (BL-10 s.d. BL-14)
| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-10 | Service installer setara `kirocrew service install` | LOW-MEDIUM | Aktif |
| BL-11 | State migration tooling (Cron/Memory/Config/Skills DB) | — | Aktif |
| BL-12 | Pemisahan eksekusi vs delivery SOP/cron trigger eksternal | LOW | Aktif, evaluasi dulu |
| BL-13 | Cost cap eksplisit per cron-job/SOP-run | LOW-MEDIUM | Aktif, verifikasi `SecurityPolicy` existing dulu |
| BL-14 | Terapkan `ContentSafety` ke jalur channel orchestrator biasa | **HIGH** | Aktif — PRIORITAS TERTINGGI di file ini |

### `subagent-resource-management-backlog.md` (BL-15 s.d. BL-18)
| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-15 | Resource gate sebelum spawn sub-agent | **HIGH** | Aktif — PRIORITAS TERTINGGI |
| BL-16 | Surface `loop_detector_circuit_breaker` ke Tasks-view | MEDIUM | Aktif |
| BL-17 | Agregasi lintas-sub-agent untuk fan-out besar | LOW | Aktif, exploratory, depends BL-16 |
| BL-18 | Gate resource di `ExecuteTurn` (jalur Go `spawn_subagent`) | CRITICAL (teknis) | **DITUNDA** — keputusan Achmad 2026-10-05, tunggu BG-D1/BG-D4 dev-tasks |

### `plugin-mcp-architecture-backlog.md` (BL-19 s.d. BL-24)
| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-19 | Sandbox MCP stdio child process | **HIGH** | Aktif — PRIORITAS TERTINGGI |
| BL-20 | Dokumentasikan gap sandbox MCP (interim) | MEDIUM | Aktif |
| BL-21 | Vault-mediation untuk secret MCP env/headers | MEDIUM | Aktif, evaluasi desain dulu |
| BL-22 | ADR pembanding WASM Plugin vs MCP | MEDIUM | Aktif |
| BL-23 | Verifikasi wiring `PluginPermission::FileRead/Write` | MEDIUM | Aktif |
| BL-24 | Governance MCP fleet-wide | MEDIUM-HIGH (jangka panjang) | **EXPLORATORY — butuh keputusan Owner, TIDAK diangkat ke roadmap aktif** |

### `providers-memory-governance-backlog.md` (BL-25 s.d. BL-31)
| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-25 | BYOC — override `client_id` OAuth xAI/OpenAI | MEDIUM | Aktif, naik prioritas kalau open-source "ya" |
| BL-26 | Dokumentasikan model keamanan single-tenant auth | LOW effort/MEDIUM impact | Aktif |
| BL-27 | Perbaiki UX pesan error OAuth refresh-revoked | LOW effort | Aktif |
| BL-28 | Client-side rate limiting LLM provider calls | MEDIUM | Aktif, tunggu BL-15 untuk baseline |
| BL-29 | Lengkapi test reliability: cascading-all-down & cooldown-expiry | LOW-MEDIUM | Aktif |
| BL-30 | `erase_identity()` lintas-backend untuk memory | MEDIUM-HIGH | Aktif |
| BL-31 | Hygiene/retensi otomatis backend-agnostic | MEDIUM | Aktif, scope besar — evaluasi trait dulu |

### `channel-cicd-observability-backlog.md` (BL-32 s.d. BL-39)
| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-32 | Perbaiki/pulihkan `verify_whatsapp_signature`+`optional_channel_routes` hilang | **HIGH** (bug nyata) | Aktif — **PRIORITASKAN, jalankan `cargo test` dulu untuk ground-truth** |
| BL-33 | Windows Authenticode code signing | HIGH (UX) | Aktif, butuh pengadaan sertifikat (non-teknis) |
| BL-34 | Verifikasi referensi `tauri.bundled.conf.json` hilang | MEDIUM | Aktif, cek SEGERA (murah) |
| BL-35 | Pin `esp-idf-*` ke rev spesifik | MEDIUM | Aktif, firmware-scope |
| BL-36 | Cakupan `cargo deny` untuk firmware | MEDIUM | Aktif, firmware-scope |
| BL-37 | Sambungkan API redaksi tool I/O (bug wiring) | **HIGH** | Aktif — tahap 1 (wiring fix) PRIORITAS, tahap 2 (redesign policy) menyusul |
| BL-38 | Terapkan `scrub_credentials` konsisten di jalur `record!` | MEDIUM | Aktif |
| BL-39 | Proteksi permission file log Windows | MEDIUM | Aktif |

### `payment-lockout-gate-backlog.md` (BL-40 s.d. BL-44)
> Preventif — nol tool pembayaran eksis di codebase saat ini; tujuan: pasang gate SEBELUM kapabilitas semacam itu pernah dibangun. Ditunda eksekusinya oleh Owner 2026-10-05 (fokus BL-32 dulu), tapi desain+riset sudah lengkap dan siap eksekusi kapan pun diangkat.

| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-40 | `PaymentLockConfig` schema (termasuk `unlock_expires_at`) di `clawcrew-config` | MEDIUM-importance/LOW-urgency | Aktif, siap eksekusi — tidak ada dependency |
| BL-41 | `PaymentLockManager` + titik gate di `tool_execution.rs` (termasuk expiry check) | MEDIUM | Aktif, setelah BL-40 |
| BL-42 | Field `payment_category` di `ToolManifest` + fail-closed waktu registrasi | MEDIUM-HIGH | Aktif, paralel dengan BL-41 |
| BL-43 | UI Settings "Allow Agents to Make Payments" (state mentah, bukan ringkasan agent) | MEDIUM | Aktif, setelah BL-40/41 |
| BL-44 | Audit log perubahan mode Locked→Unlocked | LOW-MEDIUM | Aktif, paralel dengan BL-43 |

### `token-efficiency-backlog.md` (BL-45 s.d. BL-47 → diangkat sebagai R-88 s.d. R-91)
> 2 dari 3 rekomendasi awal riset eksternal DICABUT setelah audit kode (lihat tabel "Rekomendasi yang Dicabut" di file ini) — `cache_passthrough` dan `defer_loading` MCP terbukti bukan gap nyata untuk arsitektur galleon-fleet. BL-45 dipecah jadi 45a (observasi) + 45b (mitigasi kondisional).

| BL | → Roadmap | Judul | Severity | Status |
|---|---|---|---|---|
| BL-45a | R-88 | Ukur korelasi trim-history vs cache-miss sebelum mitigasi | MEDIUM | Aktif — PRIORITAS, murah (logging), menjawab apakah BL-45b layak dengan data |
| BL-45b | R-89 | Align titik potong trim dengan rolling cache-breakpoint | MEDIUM (kondisional) | Aktif HANYA jika R-88 konfirmasi dampak signifikan |
| BL-46 | R-90 | Surface cache hit-rate ke dashboard/CLI operator | LOW-MEDIUM | Aktif, setelah R-88 minimal |
| BL-47 | R-91 | Dokumentasikan prinsip "steering stabil = cache-friendly" | LOW | Aktif, independen, biaya nol |

### `knowledge-base-citation-backlog.md` (BL-48 s.d. BL-51 → diangkat sebagai R-92 s.d. R-95)
> Kombinasi pola GitHub Copilot Memory (citation-verify) + Kiro Crew Knowledge Library (hybrid vector+keyword). Riset Perplexity murni referensi — hanya query-decomposition+domain-filtering diadopsi ke SOP (BL-50), bukan kode.

| BL | → Roadmap | Judul | Severity | Status |
|---|---|---|---|---|
| BL-48 | R-92 | Field citation + verifikasi pada node Lesson/Decision | MEDIUM | Aktif — mitigasi bug "configuration-lies" ke-3 kalinya ditemukan sesi ini |
| BL-49 | R-93 | Aktifkan Knowledge Library untuk `standard-docs/` | MEDIUM-HIGH | Aktif, setelah BL-48 untuk koherensi (tidak hard-block) |
| BL-50 | R-94 | Query decomposition + domain-filtering sebagai SOP riset | LOW | Aktif, murni dokumentasi |
| BL-51 | R-95 | Koordinasi dengan BG-B4 dev-tasks (vector memory Go) | LOW (saat ini) | **DITUNDA** — keputusan Owner 2026-10-05, tunggu BG-B4/BG-B2 selesai dulu |

### `attachment-handling-backlog.md` (BL-52 s.d. BL-56)
> Bug nyata ditemukan: `MessageDebouncer` membuang attachment pesan ke-2+ saat menggabungkan pesan rapid (hanya teks yang digabung, attachment hilang diam-diam). Keputusan Achmad: limit 10 attachment, desain 2-layer (Front Apps validasi awal + Core agregasi/resource-gate).

| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-52 | Perbaiki bug attachment hilang di `MessageDebouncer` | **P0 — bug data-loss nyata** | Aktif — PRIORITASKAN di atas semua item lain di backlog ini |
| BL-53 | Perluas `MediaKind` dengan `Document`/`Archive`/`Link` | LOW-MEDIUM | Aktif, paralel dengan BL-52 |
| BL-54 | Layer 1: validasi hard-cap 10 attachment di Front App entry point | MEDIUM | Aktif, paralel, tersebar per-channel |
| BL-55 | Layer 2: Core count+bytes enforcement setelah agregasi | MEDIUM | Aktif, SETELAH BL-52 (hard dependency) |
| BL-56 | Ekstraksi ZIP dengan zip-bomb protection | LOW (saat ini) | **EXPLORATORY/DITUNDA** — hanya kalau ada kebutuhan nyata dikonfirmasi |

### `sop-constraint-backlog.md` (BL-57 s.d. BL-61)
> Layer baru "Constraint" (SoD/larangan berbasis role) terpisah dari Layer "Flow" (SopStep yang sudah ada) — menjawab kasus nyata "Lead tidak boleh coding, hanya BE/FE boleh". Riset membuktikan BPMN/Camunda juga tidak punya ini native. Enforcement via hook `preToolUse` yang sudah ada infrastrukturnya.

| BL | Judul | Severity | Status |
|---|---|---|---|
| BL-57 | Tipe `SopConstraint`/`ConstraintKind` di `sop::types` | LOW | Aktif, tidak ada risiko regresi |
| BL-58 | Enforcement `RoleRestriction` via hook `preToolUse` | MEDIUM | Aktif, setelah BL-57 |
| BL-59 | Enforcement `DistinctPerformer` (Dynamic SoD) | MEDIUM | Aktif, setelah BL-58 |
| BL-60 | Contoh SOP nyata "Engineering Feature Delivery" + sambung ke eval loop | LOW | Aktif, setelah BL-58 |
| BL-61 | Contoh SOP non-engineering (Marketing) — bukti department-agnostic | LOW | Aktif, setelah BL-60 |

---

## Item yang DITUNDA / EXPLORATORY (Jangan Di-assign ke Squad Tanpa Keputusan Owner)

| BL | Alasan Tunda | Pemicu untuk Diangkat |
|---|---|---|
| BL-18 | Tunggu tim dev-tasks selesaikan BG-D1/BG-D4 (strangler `clawcrew-runtime`→Go) | Cek status BG-D1/BG-D4 di `finalize/dev-tasks/01-be-go-engine.md` secara berkala |
| BL-24 | Butuh keputusan arsitektur Owner (RPC proto baru vs config distribution) — menyentuh batas tanggung jawab Go/Rust yang sudah dikonfirmasi one-way-door | Ajukan pertanyaan eksplisit ke Owner |
| BL-51 | Tunggu tim dev-tasks selesaikan BG-B4/BG-B2 (vector memory Go per-scope) sebelum koordinasi desain RAG/vector lintas Go-Rust | Cek status BG-B4 di `finalize/dev-tasks/01-be-go-engine.md` secara berkala |

## Prioritas Tertinggi Lintas-Semua-File (Top 6, severity HIGH/P0 + siap eksekusi sekarang)

1. **BL-32** — bug nyata di jaring pengaman webhook WhatsApp, perlu `cargo test` SEGERA untuk ground-truth.
2. **BL-52** — bug data-loss nyata: attachment hilang diam-diam saat `MessageDebouncer` menggabungkan pesan rapid, mempengaruhi semua channel messaging hari ini.
3. **BL-15** — resource gate sub-agent, akar masalah crash yang dilaporkan Owner.
4. **BL-19** — sandbox MCP stdio, gap keamanan di jalur yang sudah dipakai produksi.
5. **BL-14** — `ContentSafety` ke channel orchestrator biasa, prompt-injection terbuka.
6. **BL-37** — sambungkan API redaksi tool I/O, "konfigurasi berbohong" soal privasi.

## Lihat Juga

- `standard-docs/research/README.md` — index riset dengan rujukan ke tiap BL di atas.
- `standard-docs/roadmap/R-rust.md` — item roadmap resmi (R-00 s.d. R-87).
- `standard-docs/roadmap/README.md` — papan status total per-lane.
