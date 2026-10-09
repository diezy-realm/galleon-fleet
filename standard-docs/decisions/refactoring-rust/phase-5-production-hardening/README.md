# Phase 5 — Rust Production Hardening: Asesmen & Roadmap

> **Branch:** `feat/refactoring-rust` · **Tanggal observasi:** 2026-10-04 · **Status:** Proposed (menunggu sign-off Owner + Principle)
> **Peran penulis:** Technical Lead (tidak mengeksekusi kode; dokumen ini = asesmen + rencana).
> **Normatif terkait:** [`code-quality/rust-module-size-and-decomposition-standard.md`](../../../tech-standart/code-quality/rust-module-size-and-decomposition-standard.md), [`plugin-standard/plugin-codebase-standard.md`](../../../tech-standart/plugin-standard/plugin-codebase-standard.md), `tech-stack/rust-conventions.md`.

Semua angka diukur langsung dari working tree (`cargo metadata`, hitung baris non-kosong, grep). Angka LOC = baris non-kosong; bisa sedikit di bawah `wc -l`.

---

## 1. Fakta terukur

### 1.1 Ukuran crate (27 crate di `crates/`)

| Crate | LOC | File | Dep internal (non-dev) | Status |
|---|---:|---:|---:|---|
| `clawcrew-runtime` | 286k | 313 | 16 | 🔴 Monolit terbesar |
| `clawcrew-channels` | 198k | 93 | 13 | 🔴 Monolit; **bergantung ke runtime** |
| `clawcrew-tools` | 85k | 96 | 7 | 🟠 |
| `clawcrew-config` | 85k | 42 | 4 | 🟠 `schema.rs` 25k baris; hub semua crate |
| `clawcrew-providers` | 84k | 49 | 4 | 🟠 (SSOT LLM, ADR 0001) |
| `clawcrew-gateway` | 42k | 53 | 14 | 🟠 46 `mod` di `lib.rs` |
| `clawcrew-memory` | 26k | 38 | 5 | 🟡 |
| `clawcrew-plugins` | 18k | 39 | 3 | 🟢 Sehat, boundary bersih |
| `galleon-channel-{core,imessage,signal,bluesky}` | 0.2–1.2k | 1 | 2–3 | 🟢 Pola benar |

Total ± 1,0 juta baris Rust. File >10k baris sudah turun di sisi test (RF-A0/A2–A7 tahap-1), tetapi **body** file besar belum dipecah (tahap-2): `config/schema.rs` 25k, `channels/orchestrator/mod.rs` 17.5k, `channels/orchestrator/acp_server.rs` 9.7k, `config/policy.rs` 9.3k, `channels/slack/mod.rs` 9.5k, `discord/mod.rs` 8k, `telegram.rs` 8k, `agent/turn/mod.rs` 7.2k, `rpc/dispatch/mod.rs` 6.9k.

### 1.2 Temuan dependency (yang sebenarnya menghambat pemecahan)

1. **`clawcrew-channels → clawcrew-runtime` (677 referensi simbol).** Arah ini terbalik secara konsep: channel adalah *adapter tepi*, runtime adalah *inti*. Yang dipakai bukan hal acak, tapi sekumpulan kecil kontrak:
   `i18n` (87, sudah diekstrak ke `galleon-i18n` — tinggal ganti path), `agent::loop_` (77), `tools::scoped` (71), `agent::memory_strategy` (60), `security::scrub` (36), `hooks::{HookResult,HookRunner}` (57), `sop::types` (29).
   → **Selama edge ini ada, tidak ada channel besar yang bisa dipecah jadi crate** (siklus: channel butuh runtime, runtime tidak boleh butuh channel). Inilah blocker nyata RF-B1+ (temuan RF-B0 yang tertulis di tracker).
2. **`clawcrew-gateway → clawcrew-runtime` (426 referensi).** Dominan `cron::*` (±100), `observability::*` (±50), `security::pairing` (14). Ini kandidat crate `galleon-cron`, `galleon-observability`, `galleon-pairing` — lebih kecil dan lebih mudah daripada memecah runtime sekaligus.
3. **`clawcrew-config` adalah hub** (dipakai hampir semua crate, termasuk `galleon-i18n`). Edit `schema.rs` memicu rebuild hampir seluruh workspace.
4. **`clawcrew-memory → providers`, `tools → memory + providers`** — rantai berat untuk crate yang seharusnya hanya butuh trait embedding/LLM (`clawcrew-api`).
5. **`clawcrew-routing` sekarang sudah masuk `[dependencies]` gateway** (perubahan uncommitted di working tree) → klaim "orphan crate" di `finalize/analysis-conclusion.md` sudah tidak benar begitu commit.
6. `clawcrew-plugins` hanya bergantung ke `api`, `infra`, `log` → **boundary plugin sudah bersih; jangan dirusak.**

### 1.3 Kualitas production (`unwrap`/`expect`)

- Grep mentah `unwrap()/expect(` = ±24.000 hit, tetapi **hampir semuanya di blok `#[cfg(test)]` inline**. Dengan heuristik "hanya baris sebelum `#[cfg(test)]` pertama, tanpa `tests/`": **±400 hit produksi** — `runtime` 237, `channels` 34, `memory` 22, `tls` 19, `tool-call-parser` 18, `providers` 17, `tools` 16, `config` 15. (Heuristik; yang pasti benar: masalahnya kecil dan *terkonsentrasi di runtime*.)
- **Tidak ada `[workspace.lints]`** dan tidak ada `clippy::unwrap_used` yang dipaksakan. Aturan "NO unwrap/expect" di `rust-conventions.md` saat ini hanya konvensi tertulis, bukan gerbang CI.
- `panic = "abort"` di profil release → satu `unwrap` produksi = proses mati. Alasan tambahan untuk mengunci gerbang ini.

### 1.4 Drift dokumen (harus dikoreksi, ADR tidak diedit — lihat §5)

| Dokumen | Klaim | Fakta |
|---|---|---|
| `ADR-001` (Konteks #6) | WIT v0 `@unstable`, belum beku | `wit/v0/.frozen` ada, `stabilized_at = 2026-10-04` |
| `ADR-001` (Tier C) | `bluesky` = WASI plugin | Diimplementasi sebagai crate native `galleon-channel-bluesky` (butuh outbound HTTP aktif → seharusnya Tier B sesuai garis B↔C ADR itu sendiri) |
| `analysis-conclusion.md` §ADR-001 | belum ada pemisahan `galleon-channel-*` | Sudah ada 4 crate (`core`, `imessage`, `signal`, `bluesky`) |
| `finalize/README.md` | Phase 4 di `decisions/phase-4/` | Aktualnya `decisions/refactoring-rust/phase-4/` |
| `rust-decomposition-plan.md` tracker | status per 2026-10-01 | Stale untuk RF-B1 dan RF-M2 |
| `rust-conventions.md` | MSRV 1.82+ | `Cargo.toml` `rust-version = "1.96.0"` |

---

> **Koreksi 2026-10-04:** `get_decrypted_secret` sudah nyata dan `ExecuteTurn` sudah terimplementasi; klaim sebelumnya disalin dari `finalize/analysis-conclusion.md` yang basi. Lihat `standard-docs/foundation/04-drift-register-and-hygiene.md`. UI/desktop aktif = `web-2` + `tauri-2` (`foundation/02`).

## 2. Pendapat saya (jawaban atas pertanyaan)

**Q1 — Apa langkah berikutnya untuk memecah kode besar?**
Jangan lanjut "pecah file" dulu. Tahap-1 (test-split) sudah memberi manfaat yang bisa didapat dengan murah. Dua hal yang tersisa punya karakter berbeda:
- **Body-split file** hanya menolong keterbacaan, tidak menolong waktu build (satu crate = satu unit kompilasi).
- **Pemecahan crate** yang menolong build — tapi **diblokir oleh edge `channels → runtime`** dan hub `config`.

Maka urutan yang benar: **(1) pasang pagar CI → (2) putuskan edge terbalik lewat ekstraksi kontrak ke crate kecil → (3) baru pecah crate → (4) body-split sebagai pekerjaan oportunistik.** Memecah crate sebelum memutus edge hanya memindahkan kompleksitas.

**Q2 — Standar codebase plugin?** Sudah ada fondasi bagus (`clawcrew-plugins`, WIT v0 beku, fixture, signature, catalog). Yang belum ada adalah **standar tertulis tunggal** bagi penulis plugin/channel — kini dibuat di [`plugin-standard/plugin-codebase-standard.md`](../../../tech-standart/plugin-standard/plugin-codebase-standard.md): 3 tier, kriteria keputusan, struktur crate, aturan boundary, manifest, keamanan, testing, versioning.

**Q3 — Apa lagi yang perlu di-refactor agar production-ready?** Ya, urutan prioritas di §3. Yang paling bernilai bukan "lebih banyak crate" tetapi: gerbang CI (unwrap, ukuran file, arah dependency), memutus `channels → runtime`, dan menutup bypass keamanan (`execute_terminal_command` di tauri-2, eksekusi shell/fs lokal di Go = Lane D1 fail-closed).

**Hal yang sengaja TIDAK saya rekomendasikan (YAGNI):**
- Memecah `clawcrew-runtime` menjadi 10+ crate sekaligus. Mulai dari 3 potongan dengan fan-in nyata (§3 W2).
- Rename massal `clawcrew-*` → `galleon-*`. Biaya besar (churn `Cargo.toml`, publish name, import) tanpa manfaat arsitektur. Aturan "crate baru = `galleon-`" cukup.
- Memigrasi 15 channel Tier C ke WASI sekaligus. Pilot dulu (sudah ada `channel-nostr-pilot`).
- Membuat crate per-handler di gateway (`plugin-telegram` dst.) — sudah diputuskan *defer* (Gap 2).

---

## 3. Roadmap (P0 → P2), tanpa circular dependency

Notasi ukuran: S ≤ 1 hari, M 2–3 hari, L > 3 hari. Setiap item = 1 PR kecil, satu concern.

### W0 — Guardrails (P0, paralel, tidak mengubah perilaku)

| ID | Task | Ukuran | DoD |
|---|---|---|---|
| H0.1 | `[workspace.lints]` di root `Cargo.toml`: `clippy::unwrap_used`, `expect_used`, `panic` = `warn` dulu; `clippy.toml` `allow-unwrap-in-tests = true`; tiap crate `[lints] workspace = true` | S | `cargo clippy --workspace` jalan; baseline jumlah warning tercatat |
| H0.2 | Ratchet: CI gagal bila jumlah `unwrap_used` **naik** vs baseline; naikkan ke `deny` per-crate saat crate sudah 0 | S | Crate kecil (`tls`, `tool-call-parser`, `api`, `plugins`, `routing`) langsung `deny` |
| H0.3 | RF-M3: gerbang ukuran file (>2k baris warning, >5k baris fail untuk file *baru/berubah*) | S | Script `scripts/ci/` + dokumentasi pengecualian |
| H0.4 | Gerbang arah dependency (test di `tests/architecture/` memakai `cargo metadata`; daftar edge terlarang) | S | Edge terlarang awal: `plugins→{runtime,channels,gateway}`, `galleon-channel-*→runtime`, `api→*`, `runtime→{channels,gateway}` |
| H0.5 | RF-M1: baseline `cargo build --timings` + `cargo nextest` waktu per crate, simpan angka di dokumen ini | S | Tabel before/after untuk tiap W2 |

### W1 — Putuskan edge terbalik (P0, prasyarat semua pemecahan crate)

| ID | Task | Ukuran | Catatan |
|---|---|---|---|
| H1.1 | Ganti 87 ref `clawcrew_runtime::i18n` di channels → `galleon_i18n` | S | Mekanis; verifikasi `cargo check -p clawcrew-channels` |
| H1.2 | Peta pemakaian `runtime` oleh `channels` per kategori (hook, scoped-tools, memory_strategy, scrub, sop types, agent loop) → keluarkan **trait + tipe data** ke `clawcrew-api` atau crate baru `galleon-agent-contract` | M | Aturan: hanya trait/DTO; implementasi tetap di runtime. Ini RF-B0 yang tertunda |
| H1.3 | `security::scrub` (36 ref) + `security::pairing` (14 ref) → crate `galleon-security-core` (pure fn, tanpa I/O) | S–M | Dipakai channels, gateway, runtime — fan-in tinggi, risiko rendah |
| H1.4 | Pecahkan agent loop dependency: channels memanggil `agent::loop_` (77 ref) lewat trait `AgentTurnRunner` di contract crate, di-inject saat wiring (bin) | M | DI manual/trait object; tidak ada global |

**Exit criteria W1:** `cargo metadata` menunjukkan **tidak ada** edge `clawcrew-channels → clawcrew-runtime` (gerbang H0.4 mengunci).

### W2 — Pemecahan crate berdampak (P1)

Urutan berdasarkan rasio manfaat/risiko:

| ID | Task | Ukuran | Dep |
|---|---|---|---|
| H2.1 | `galleon-cron` (dari `runtime::cron`), `galleon-observability` (`runtime::observability`) → gateway tidak lagi menarik runtime penuh | M | W1 |
| H2.2 | Channel Tier A besar tetap di `clawcrew-channels` (sesuai ADR-001) **tetapi** pecah *internal*: `orchestrator/` (17.5k+9.7k) → crate `galleon-channel-orchestrator` | M–L | H1.2, H1.4 |
| H2.3 | Channel Tier B berikutnya sebagai `galleon-channel-<name>`: urutan `mattermost` → `irc` → `nextcloud` → `whatsapp-web` (triad util/identity) → `wechat`/`qq` | M per channel | H2.2; ikuti [plugin standard](../../../tech-standart/plugin-standard/plugin-codebase-standard.md) §4 |
| H2.4 | `clawcrew-providers` per-vendor (feature-crate: anthropic, compatible, reliable-wrapper) — hanya jika timing H0.5 menunjukkan providers ≥ 15% build kritis | M | H0.5 |
| H2.5 | `clawcrew-config`: `schema.rs` 25k → submodul per-domain **setelah** extract tipe pure-data ke `galleon-config-types`, supaya edit logic tidak me-rebuild seluruh workspace | L | H0.5; RF-A1 |
| H2.6 | `clawcrew-memory` dan `tools`: ganti dep konkret `providers` dengan trait dari `api` | M | — |

### W3 — Hardening produksi (P1, paralel dengan W2)

| ID | Task | Ukuran |
|---|---|---|
| H3.1 | Turunkan `unwrap/expect` produksi di `runtime` (237) → 0, per submodul; naikkan lint ke `deny` | M |
| H3.2 | Round-trip test Go→Rust Vault untuk `get_decrypted_secret` (sudah terimplementasi) + implementasi/pencabutan `execute_task` (satu-satunya stub); **fail-closed** sesuai keputusan #3 | S–M |
| H3.3 | Lane D2: test Landlock boundary + round-trip `ExecuteTool` | M |
| H3.4 | Secret scrubbing: uji otomatis bahwa log/tracing tidak memuat kredensial (golden test pada `security::scrub`) | S |
| H3.5 | `cargo-deny` (advisories, license, bans duplikat versi) + `cargo-audit` di CI | S |
| H3.6 | `cargo nextest run --workspace` sebagai gerbang wajib; jalur terpisah untuk test lambat (feature `slow-tests` / `#[ignore]`) | S |

### W4 — Plugin & channel distribution (P2, sesudah sign-off ADR-001)

| ID | Task | Ukuran |
|---|---|---|
| H4.1 | Validasi pilot `channel-nostr-pilot` e2e; pilot kedua inbound-first (`line` atau `dingtalk`) | M |
| H4.2 | `wit-breaking-change-check` masuk CI untuk `wit/v0/**` (kontrak sudah beku) | S |
| H4.3 | Koreksi tier `bluesky` (ADR errata → B atau tunggu `wasi:http` outbound channel) | S |
| H4.4 | Catalog + signature enforcement default-on untuk plugin non-bundled (`signature.rs` sudah ada) | M |

### W5 — Body-split oportunistik (P2)

Hanya untuk file >5k yang *sedang disentuh* oleh task lain; aturan di standar ukuran §3. Jangan membuat PR "pecah file" murni kecuali ada pemilik modul.

---

## 4. Definition of Done (per PR)

- [ ] `cargo check -p <crate>` & `cargo clippy -p <crate> -- -D warnings` hijau (`$env:CARGO_TARGET_DIR="target/agent"` di Windows).
- [ ] `cargo nextest run -p <crate>` hijau; tidak ada test yang dihapus untuk "meloloskan" refactor.
- [ ] Tidak ada `unwrap()/expect()` baru di jalur produksi.
- [ ] Gerbang arah dependency (H0.4) hijau; tidak ada edge baru yang melanggar.
- [ ] Pure-move: diff hanya perpindahan + `mod`/`use`; perubahan perilaku = PR terpisah.
- [ ] Tabel timing before/after (H0.5) untuk PR pemecahan crate.
- [ ] Dokumen/ADR terkait diperbarui (errata, bukan edit ADR).

## 5. Keputusan yang perlu sign-off

| # | Keputusan | Rekomendasi |
|---|---|---|
| D1 | Urutan: W0 → W1 sebelum pemecahan crate apa pun | Setuju |
| D2 | Crate kontrak baru `galleon-agent-contract` vs menambah ke `clawcrew-api` | Crate baru bila >~3k baris trait/DTO; `api` harus tetap leaf |
| D3 | Lint `unwrap_used` mulai `warn` + ratchet (bukan langsung `deny`) | Setuju |
| D4 | Errata (bukan edit) untuk ADR-001: WIT beku & tier `bluesky` | Tulis `ADR-001-errata` di folder phase-2 |
| D5 | Konvensi MSRV di `rust-conventions.md` diselaraskan ke `rust-version` workspace | Selaraskan |

## 6. Risiko & rollback

| Risiko | Mitigasi / Rollback |
|---|---|
| Ekstraksi kontrak memicu visibility churn (`pub(crate)` → `pub`) | Hanya ekspor trait/DTO; revert per-PR (pure move) |
| Feature-flag matrix meledak (channels punya 44 feature) | CI matrix: `default`, `no-default`, `all`; jangan tambah flag tanpa use case |
| Cache sccache salah → hasil build menyesatkan | Timing diukur dengan `cargo clean -p` terarah + `--timings` |
| Pemecahan paralel oleh banyak agent → konflik writer | Satu crate = satu pemilik; kunci via tracker di `rust-decomposition-plan.md` |
