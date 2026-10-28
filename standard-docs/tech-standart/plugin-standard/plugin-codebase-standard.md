# Plugin & Channel Codebase Standard

> **Status:** Normatif (Proposed → berlaku setelah sign-off Lead + PO)
> **Berdasarkan:** `ADR-001-channel-plugin-distribution.md`, `wit/v0/*.wit` (beku, `wit/v0/.frozen`), `crates/galleon-plugins`, fixture di `crates/galleon-plugins/tests/fixtures/`.
> **Catatan:** ADR tidak diedit; koreksi fakta ada di `phase-5-production-hardening/README.md` §1.4.

## 1. Pilih tier dengan pohon keputusan

```
Channel/ekstensi baru
 ├─ Identitas inti produk / berbagi governance primitive (approval queue) dgn Tier A? ──► Tier A: embed di galleon-channels
 ├─ Butuh socket 2-arah native / native crypto / SDK berat / HTTP outbound aktif / OS-spesifik? ──► Tier B: crate galleon-channel-<name> (compiled-in, Cargo feature)
 └─ Cukup inbound webhook/polling, niche/regional, boleh di-sandbox? ──► Tier C: WASI plugin (signed, hot-load)
```

Tool, memory backend, observer, skill: default **Tier C (WASI)** bila tidak butuh akses native; selain itu feature-crate. Mengacu ke `PluginCapability` (`Tool | Channel | Memory | Observer | Skill`) di `galleon-plugins`.

## 2. Aturan umum (semua tier)

1. **Least privilege & fail-closed.** Izin default kosong; izin eksplisit di manifest (network host allowlist, secret key, filesystem scope). Pelanggaran → error, bukan fallback.
2. **Secrets hanya via Vault** (Rust, AES-256-GCM). Plugin menerima secret lewat interface `secrets` WIT/host; tidak pernah di config plaintext, env, atau log. Field secret ditandai `x-secret` di `config_schema`.
3. **Tidak ada logika fleet-governance/policy di plugin.** Risk-tier & auto-approval hanya di Go (`fleet/policies.go`); channel melaporkan event, tidak memutuskan kebijakan.
4. **Tidak ada `unwrap()/expect()/panic!` di jalur produksi**; error pakai `thiserror`, `Result<T>`.
5. **Log wajib scrub** kredensial; gunakan `logging` WIT / `tracing` dengan `#[instrument(skip(...))]` untuk argumen sensitif.
6. **User-facing text** lewat Fluent (`galleon-i18n`), bukan literal.
7. **Idempotent & bounded:** setiap panggilan punya timeout, batas ukuran payload, dan backpressure; retry eksponensial dengan jitter hanya untuk error transient.

## 3. Tier C — WASI plugin

### 3.1 Struktur

```
plugins/<name>/                 # atau crates/... untuk fixture/pilot
├── Cargo.toml                  # crate-type = ["cdylib"]; target wasm32-wasip2
├── plugin-manifest.toml
├── wit/ → merujuk ../../wit/v0 (jangan menyalin)
├── src/
│   ├── lib.rs                  # wit_bindgen::generate! + export! saja (≤ 150 baris)
│   ├── inbound.rs              # parse webhook / poll → inbound-message
│   ├── outbound.rs             # (bila diizinkan host) format pesan
│   ├── config.rs               # validasi config sesuai schema
│   └── error.rs
└── tests/                      # unit (native) + e2e via host fixture
```

### 3.2 Manifest wajib

`id` (reverse-DNS), `version` (semver), `capability`, `wit_version` (`0.1.0`), `config_schema` (JSON Schema; secret = `x-secret`), `permissions` (network hosts, secrets, dll), `min_host_version`, `signature` (dihasilkan pipeline rilis; verifikasi oleh `signature.rs`).

### 3.3 Batas teknis (dari kode saat ini)

- Channel-plugin **inbound-first**: `poll-message` wajib; webhook opsional (`webhook-ingress`, `webhook-path`, `parse-webhook`).
- Channel store **tidak** diberi `wasi:http` outbound. Channel yang wajib mengirim HTTP aktif → **Tier B** sampai kebijakan ini diubah lewat ADR.
- Plugin tidak boleh membaca filesystem/jaringan di luar yang dideklarasikan; host menegakkan via Wasmtime + egress allowlist (`egress.rs`).
- Ukuran artefak `.wasm` target ≤ 2 MB (release, `opt-level = "z"`, `lto`, `strip`).

### 3.4 Distribusi

Hanya plugin **bertanda tangan** yang bisa di-load di mode default. Plugin tak bertanda tangan butuh flag dev eksplisit dan tampil sebagai peringatan di UI. Catalog (`catalog.rs`) adalah satu-satunya sumber daftar plugin tersedia.

## 4. Tier B — feature-crate `galleon-channel-<name>`

Pola referensi: `galleon-channel-signal`, `-imessage`, `-bluesky` (1 file, < 1.3k baris saat dibuat).

```
crates/galleon-channel-<name>/
├── Cargo.toml     # name = "galleon-channel-<name>"; deps hanya: clawcrew-api, galleon-channel-core, galleon-log, + SDK
├── src/lib.rs     # impl trait Channel dari galleon-channel-core; fasad kecil
└── src/{transport,codec,auth,config,error}.rs   # pecah bila > 800 baris
```

Aturan tambahan:
- **Dilarang** mengimpor `clawcrew-runtime` atau `galleon-channels` (arah dependency; dikunci CI). Kebutuhan runtime lewat trait di crate kontrak/`galleon-channel-core`.
- Registrasi mandiri (self-register) via `galleon-channel-core`; `galleon-channels` hanya meng-*aggregate* di balik Cargo feature `channel-<name>`.
- Feature flag di bin/`galleon-channels`: `channel-<name>`; masuk `default-channels` **hanya** jika Tier A.
- Platform-spesifik dengan `#[cfg(target_os = "...")]` dan target `compile_error!`/stub yang jelas; tidak ada kode mati.
- Test: unit tanpa jaringan (mock transport) + 1 contract test terhadap trait `Channel`.
- Ambang pecah: bila crate > 5k baris, pisahkan `-protocol` (pure codec) dari `-transport`.

## 5. Tier A — embed

Hanya channel inti (telegram, discord, slack, matrix + primitif transport). Tidak menambah anggota baru tanpa ADR. Struktur internal mengikuti standar ukuran (`code-quality/rust-module-size-and-decomposition-standard.md`): satu folder per channel, `mod.rs` fasad, `tests.rs` terpisah, file ≤ 2k baris. File raksasa saat ini (`slack/mod.rs` 9.5k, `discord/mod.rs` 8k, `telegram.rs` 8k, `orchestrator/*` 27k) masuk daftar ratchet.

## 6. Versioning & kompatibilitas

- WIT `v0` **beku** (`wit/v0/.frozen`). Perubahan breaking → `wit/v1/`; perubahan aditif non-breaking diperbolehkan dan wajib lolos `wit-breaking-change-check` di CI.
- Host mendukung N dan N-1 versi WIT; plugin menyatakan `wit_version`; mismatch = tolak load (fail-closed) dengan pesan jelas.
- Plugin semver sendiri; perubahan `permissions` yang menambah izin = major atau butuh persetujuan ulang user.

## 7. Testing wajib

| Lapisan | Wajib |
|---|---|
| Unit | codec/parse/validasi config, tanpa jaringan |
| Contract | trait `Channel` (Tier B) / host fixture (Tier C) |
| Security | secret tidak muncul di log; permission ditolak bila tak dideklarasikan; signature invalid ditolak |
| E2E | webhook/poll → inbound-message → ack, memakai fixture host (`channel-fixture`, `channel-nostr-pilot`) |
| Perf | batas payload & timeout ditegakkan |

Runner: `cargo nextest run -p <crate>`.

## 8. Checklist PR plugin/channel baru

- [ ] Tier dipilih via pohon keputusan §1 dan dicatat di PR.
- [ ] Manifest lengkap (Tier C) / crate mengikuti struktur §4 (Tier B).
- [ ] Tidak ada edge dependency terlarang (CI hijau).
- [ ] Tidak ada `unwrap/expect/panic` produksi; clippy `-D warnings` hijau.
- [ ] Secret via Vault, log ter-scrub, i18n via Fluent.
- [ ] Test §7 ada; fixture e2e hijau.
- [ ] Dokumentasi: entri di catalog + `docs/` channel terkait; ADR-001 mapping tabel diperbarui lewat errata bila tier berbeda.
