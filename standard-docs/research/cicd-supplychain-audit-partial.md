---
title: Audit CI/CD & Supply-Chain — Jauh Lebih Matang dari Placeholder, 1 Gap HIGH Ditemukan
status: research
date: 2026-10-05
author: squad-lead
depends_on: null
---

# Audit CI/CD & Supply-Chain Security (Lengkap)

> Menggantikan versi "partial" sebelumnya — hasil sub-agent (`369f9af4`) berhasil tiba dengan bukti lengkap. Cross-check awal: `finalize/dev-tasks/05-platform-devops-qa.md` hanya punya placeholder generik `PL-D1` tanpa detail — audit ini membuktikan kondisi riil **under-represented** oleh placeholder tersebut.

## Verdict

**CHANGES-REQUESTED untuk klaim "siap rilis desktop".** Infrastruktur CI/CD jauh lebih matang dari asumsi (SBOM, cosign, SLSA attestation, cargo-deny, Dependabot semua ADA dan battle-tested) — tapi ada **1 gap HIGH** (Windows code signing tidak ada) dan beberapa gap MEDIUM yang harus ditutup sebelum rilis publik/open-source.

## Temuan Positif (Sudah Matang, Tidak Perlu Dikerjakan Ulang)

**1. CI workflow & dependency scanning**: `ci.yml` (Quality Gate: fmt/build-matrix/clippy/test), `daily-audit.yml` (`cargo deny check advisories` harian, auto-buka issue kalau gagal), `trivy-scheduled.yml` (scan image GHCR mingguan + SARIF ke tab Security), `codeql.yml`, `dependabot.yml` (cargo daily, npm weekly, github-actions daily, docker daily, semua grouped). `deny.toml` root (243 baris): advisories deny-by-default dengan 4 ignore BER-JUSTIFIKASI tertulis (RUSTSEC spesifik, transitive via `matrix-sdk`, tracking issue dicatat), license allow-list, `multiple-versions = "deny"`, `unknown-git = "deny"`.

**2. Dependency pinning**: `Cargo.lock` root ADA, **~1322 crate pihak ketiga** ter-resolve. Caret range di `Cargo.toml` (`reqwest = "0.12"` dll) adalah **normal untuk ekosistem Rust** — bukan temuan, karena `Cargo.lock` dikomit dan CI build pakai `--locked` (dikonfirmasi di `release-stable-manual.yml`).

**3. SBOM — dua layer**: CI reguler (`ci-sbom.yml`, push+harian, CycloneDX Rust+npm) DAN release (`release-stable-manual.yml` job `sbom`, DUA format SPDX-json + CycloneDX-json via `anchore/sbom-action`, **wajib ada** sebagai release asset — gagal publish kalau hilang).

**4. Release integrity — sangat solid, di atas standar industri**: `SHA256SUMS` untuk semua artifact, **SLSA v1.0 Build Level 2 provenance attestation** (`actions/attest`), "offline verification archive" (trusted-root + attestation bundle, bisa diverifikasi tanpa network), container image di-sign keyless via **cosign/Rekor**. Dokumentasi verifikasi (`gh attestation verify`) ditulis otomatis ke release notes.

**5. macOS code signing — ADA LENGKAP**: `APPLE_CERTIFICATE`/`APPLE_SIGNING_IDENTITY` secrets (graceful degrade ke unsigned kalau secret absen — "secrets never gate the build"), notarization (`xcrun notarytool submit --wait`), `xcrun stapler staple` pada `.dmg`.

## Gap Ditemukan (Konkret, Perlu Ditutup)

### [HIGH] Windows Authenticode Signing — TIDAK ADA SAMA SEKALI
`build-desktop-windows` job build `.msi`/`-setup.exe` lalu LANGSUNG upload — tidak ada `signtool`, tidak ada secret `WINDOWS_CERT*`/`AUTHENTICODE*` di manapun (grep seluruh workflows = nol match), `tauri.conf.json` tidak punya field `bundle.windows.certificateThumbprint`/`digestAlgorithm`/`timestampUrl`. **Dampak**: binary Windows unsigned → SmartScreen/Defender warning ke SETIAP pengguna yang install, dan tanpa Authenticode user tidak punya cara verifikasi publisher identity di level OS (SHA256SUMS/SLSA ada tapi itu verifikasi command-line, bukan trust OS-native).

### [MEDIUM] `tauri.bundled.conf.json` Dirujuk 3× Tapi Tidak Ada di Working Tree
Direferensikan di `release-stable-manual.yml:537,635,686`, `TESTING.md:56`, `prepare-kernel.sh:29` — tapi file genuinely tidak ditemukan (kemungkinan di-generate runtime atau gitignored). **Perlu konfirmasi**: kalau ini satu-satunya tempat `bundle.windows` seharusnya didefinisikan dan generator-nya tidak pernah mengisi signing field, Windows akan SELALU unsigned bahkan kalau secret di-provide nanti.

### [MEDIUM] Git-Dependency Tanpa `rev` di Firmware ESP32
`firmware/esp32/Cargo.toml:21-26` — `esp-idf-sys`/`esp-idf-hal`/`esp-idf-svc` di-pin ke `git = "..."` TANPA `rev`/`branch` → mengambil HEAD default branch tiap resolve ulang. Supply-chain risk nyata: upstream bisa push commit kompromais dan ter-pull otomatis. Scope: firmware ESP32 saja, bukan desktop/core.

### [MEDIUM] `deny.toml` Root Tidak Mengaudit `firmware/esp32`/`firmware/nucleo`
Keduanya punya `Cargo.lock` TERPISAH (bukan workspace member root) — blind-spot audit, bukan vulnerability langsung.

## Rekomendasi

1. **[HIGH]** Tambah Windows Authenticode signing ke `build-desktop-windows` job — `WINDOWS_CERTIFICATE`(base64 PFX)+`WINDOWS_CERTIFICATE_PASSWORD` secrets, `signtool sign /fd SHA256 /tr <timestamp-url> /td SHA256` terhadap `.msi`/`-setup.exe`, dengan graceful-degrade pattern yang SAMA seperti macOS (skip kalau secret absen, jangan gate build).
2. **[MEDIUM]** Konfirmasi isi generator `tauri.bundled.conf.json` (baca `prepare-kernel.sh` lengkap) — pastikan template punya slot `bundle.windows`.
3. **[MEDIUM]** Pin `esp-idf-*` ke `rev` spesifik, bukan default branch HEAD.
4. **[MEDIUM]** Tambah `cargo deny` check untuk `firmware/esp32`/`firmware/nucleo` (manifest-path terpisah di CI).
5. **[LOW]** Dokumentasikan status "Windows unsigned" eksplisit di release notes sampai #1 selesai.

**Catatan arsitektur**: placeholder `PL-D1` di dev-tasks **under-represents** kondisi riil secara signifikan — sebagian besar scope (SBOM, checksum, container signing, SLSA) sudah IMPLEMENTED dan battle-tested, bukan "to-do". Rekomendasikan `PL-D1` di-rewrite jadi tracking item SEMPIT: "Windows Authenticode signing + firmware git-dep pinning + firmware cargo-deny coverage" — 3 item konkret, bukan placeholder generik yang menyesatkan soal seberapa banyak yang masih harus dikerjakan.

---

## Sumber

- Audit sub-agent lengkap (`369f9af4`): `.github/workflows/{ci,daily-audit,trivy-scheduled,codeql,ci-sbom,docker-publish,release-stable-manual}.yml`, `.github/dependabot.yml`, `deny.toml` (root), `Cargo.lock` (root, ~1322 entries), `firmware/esp32/Cargo.toml:21-26`, `apps/tauri/tauri.conf.json`
- Cross-check: `standard-docs/decisions/finalize/dev-tasks/05-platform-devops-qa.md` (PL-D1, placeholder generik yang terbukti under-represent kondisi riil)
