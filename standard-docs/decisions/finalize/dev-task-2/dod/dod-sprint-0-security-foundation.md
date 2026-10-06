---
sprint: 0
title: DoD & PR Checklist — Security Foundation
---

# DoD & PR Checklist — Sprint 0 (Security Foundation)

## Gerbang Wajib (dari `roadmap/PR-plan.md` §1)

- [ ] **GW-0 Hygiene** — `git diff --check`; tidak ada secret (`gitleaks`); commit conventional + trailer; tidak ada perubahan di luar scope tiket.
- [ ] **GW-0b Commit/PR naming** — judul commit DAN judul PR memakai template `feat({tiket}): {deskripsi}` persis (lihat README §Konvensi Commit & PR); `{tiket}` cocok PERSIS ID di sprint file (case-sensitive); multi-tiket dipisah koma tanpa spasi (`BL-32,BL-14`); PR description baris pertama menyebut `Sprint 0` eksplisit; commit dilakukan di branch `feat/refactoring-finalize-01` (bukan sub-branch baru kecuali darurat + wajib sync balik, lihat README §Kebijakan Branch).
- [ ] **GW-1 Rust** — `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>`; **nol `unwrap()/expect()` baru di jalur produksi**; tidak ada `#[allow(dead_code)]` baru.
- [ ] **GW-2 Go** — `go vet ./...`; `go test -race ./...`; `wire` regen bersih (`git diff --exit-code engine/app/wire_gen.go`); wrap error `%w`; zap.
- [ ] **GW-4 Boundary** — UI/Tauri → Go :9090 → Rust :50052; tidak ada shell/fs di Go atau Tauri; policy hanya di `fleet/policies.go`; rahasia hanya di Vault.
- [ ] **GW-8 Keamanan** — threat-note di PR (permukaan baru default tertutup); **test negatif WAJIB**: tanpa token → ditolak; injeksi → ditolak; log bebas rahasia.
- [ ] **GW-9 Rollback** — deskripsi PR memuat risiko, efek samping, cara rollback.

## Acceptance Criteria Spesifik Sprint Ini

- [x] `S2-SEC-01`: test negatif membuktikan `req.Approved=true` TANPA principal terverifikasi dari `BG-C1` DITOLAK (bukan hanya diterima dengan warning).
- [x] `BG-C1`: test negatif "tanpa kredensial → ditolak" lolos untuk SETIAP endpoint yang bergantung principal resolver (bukan hanya satu contoh).
- [x] `BL-32`: `cargo nextest run -p clawcrew-gateway --features channel-whatsapp-cloud` lolos PENUH termasuk drift-guard; test integrasi HTTP baru (signature salah → 401) ada dan lolos.
- [x] `BR-B3`: sandbox deny-by-default terbukti test per-OS (Landlock/Seatbelt/AppContainer) — minimal 1 OS yang relevan ke target rilis, bukan placeholder skip-all.
- [x] `BL-19`: MCP stdio child process sandboxed — buktikan dengan test yang mencoba akses file/network di luar scope dan assert DITOLAK.
- [x] `BL-14`: `ContentSafety` aktif terverifikasi di SEMUA channel non-SOP yang ada saat ini (daftar eksplisit channel yang dicek, bukan klaim umum "semua channel").

## Checklist Review Principle (7 Security Gates existing + tambahan sprint ini)

1. Path validation
2. Workspace containment
3. Timeout enforcement
4. Spawn array args (bukan string interpolation)
5. Proto alignment
6. No secrets
7. RCE closure
8. **BARU khusus sprint ini:** approval-gate identity verification — PASTIKAN tidak ada endpoint yang menerima `approved`/`authorized` boolean dari body request tanpa cross-check ke principal resolver.

## Verdict Template

```
## Verdict
APPROVE / CHANGES-REQUESTED

## Temuan
<file:baris, severity>

## Perubahan diminta
<daftar konkret>

## Catatan arsitektur/keamanan
<khusus sprint ini: status S2-SEC-01 dan BL-32 WAJIB disebutkan eksplisit>
```
