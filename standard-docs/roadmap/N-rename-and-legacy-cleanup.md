# Lane N — Rebrand ke Galleon & Pembersihan Legacy

Ref: ODR-16. Pola yang dicari: `zeroclaw`, `zerocode`, `zerorelay`, `claw-crew`, `clawcrew`, `ClawCrew`, `FleetAI`/`Fleet AI`/`fleet-ai`.
**Skala terukur (file tracked, di luar docs/ADR/lock/i18n):** ±1.656 file, ±40.000 kemunculan. Terbesar: `clawcrew-channels` 9,1k, `clawcrew-runtime` 8,9k, `docs/_archive` 3,6k (216 file), `providers` 2,2k, `tools` 2,1k, `gateway` 1,8k, `config` 1,3k, **`bin/engine.exe` (biner ter-commit)**, `apps/zerocode` 627, `src/main.rs` 487, `scripts/ci` 439, `.claude/skills` 368, `xtask` 354, `.github/workflows` 294.

**Aturan umum**
- Satu PR = satu lapisan; tiap PR hijau (`cargo check/clippy/nextest`, `go test -race`, `npm run build`).
- Rename mekanis lewat skrip (`scripts/rename/…`), bukan tangan; ada mode `--dry-run` dan laporan.
- **Jangan ubah:** ADR/riwayat di `standard-docs/decisions/` (immutable), `CHANGELOG`, sertifikat/lisensi atribusi, data fixture bertanda tangan, path/ID yang dipakai format on-disk pengguna **tanpa jalur migrasi**.
- Kompatibilitas data: nama direktori config (`~/.clawcrew`→`~/.config/galleon`), env var (`CLAWCREW_*`→`GALLEON_*`), nama service, WIT package. Wajib ada **migrasi data sekali jalan** untuk path pengguna; env var diganti langsung, tanpa alias (ODR-19).
- Aturan penamaan baru crate: `galleon-*` (rust-conventions); legacy `clawcrew-*` diganti bertahap.

## N-0x Keputusan & alat (wajib dulu)

- [x] **N-00 (P0, S) Matriks penamaan resmi** — *DISETUJUI Owner 2026-10-04. ODR-19: tanpa alias; draf selesai: `tech-standart/naming/naming-map.md`; menunggu sign-off Owner (3 pertanyaan terbuka §8).* `standard-docs/tech-standart/naming/naming-map.md`: lama→baru untuk crate, binary (`clawcrew`→`galleon`; `zerocode`→`galleoncode`; `zerorelay`→`galleonrelay`), paket npm, env var, direktori config, service name, bundle id, WIT package (v0 beku; v1 = `galleon:plugin`), header HTTP, metric prefix, Docker image. Owner sign-off. Out: eksekusi.
- [x] **N-01 (P0, S) Inventaris klasifikasi** [Selesai PR-06] 1.709 file dan 40.422 hit diklasifikasikan ke `rename` (1.400), `delete` (246), `keep` (43), `needs-migration` (20) di `files/brand-hit-inventory.csv` + ringkasan `brand-hit-inventory-summary.md`.
- [ ] **N-02 (P1, M) Skrip rename mekanis + gerbang CI** yang gagal bila pola terlarang muncul di jalur non-whitelist (`scripts/ci/brand_gate.sh`). Dep: N-00, N-01.

## N-1x Hapus yang tidak relevan (lakukan SEBELUM rename agar volume turun)

- [x] **N-10 (P0, S) Hapus `bin/engine.exe` dari git** [Selesai PR-02] Binary 40MB `bin/engine.exe` dihapus dari git tracking (`git rm bin/engine.exe`) dan `.gitignore` diperbarui mengabaikan `/bin/` dan `apps/tauri/target/`.
- [x] **N-11 (P1, M) `docs/_archive/` (307 file tracked)** [Selesai PR-17] Tag `archive/pre-galleon` dibuat, direktori `docs/_archive` (307 file) dihapus dari git tracking (`git rm -r docs/_archive`), rujukan `docs/book` di `AGENTS.md` diperbaiki ke `standard-docs/`.
- [ ] **N-12 (P1, M) Hapus kode warisan tak terpakai** hasil R-00: REST/WS/A2A gateway (`api*.rs`, `ws.rs`, `a2a.rs`) bila tak ada pemakai; hardware/peripheral tak relevan; sub-sistem yang tidak masuk produk Galleon. Dep: R-00, sign-off Owner.
- [x] **N-13 (P1, S) `.claude/skills`, `.agents`, `.kiro`, `.gemini`, `.copilot`:** [Selesai PR-18] Skill khusus ClawCrew (`clawcrew`, `changelog-generation`, `feature-matrix-parity`, `github-issue-triage`, `pr-architecture-check`, `wit-breaking-change-check`) telah dihapus; skill workflow git netral dipertahankan.
- [x] **N-14 (P1, S) `.github/workflows` (39):** [Selesai PR-18] Workflow usang khusus ClawCrew (`docs-deploy.yml`, `discord-release.yml`, `tweet-release.yml`) telah dihapus.
- [x] **N-15 (P2, S) Packaging warisan:** [Selesai PR-19] Packaging warisan tidak terpakai (`nix/`, `deploy-k8s/`, `dist/freebsd/`) telah dihapus via `git rm`.
- [ ] **N-16 (P2, S) Fixture & contoh tak relevan** di `tests/`, `crates/*/tests/fixtures`, `examples/`.
- [ ] **N-17 (P2, S) Terjemahan (`.ftl`/`.po`):** hapus locale/kunci yang tak dipakai; ganti merek di string sumber lalu regenerasi (`tools/fill-translations`).

## N-2x Rename per lapisan

- [ ] **N-20 (P1, M) Binary & CLI:** `clawcrew`→`galleon` (root `Cargo.toml` package/bin, `src/main.rs` string UX, help, completions); `apps/zerocode`→`apps/galleoncode` (biner `galleoncode`, TUI CLI: chat, ACP, agent sidebar, config manager) dan `apps/zerorelay`→`apps/galleonrelay` (biner `galleonrelay`, blind mTLS relay untuk perangkat di balik NAT) — **keduanya dipertahankan, hanya rename + penyesuaian** (ODR-17). Tanpa alias (ODR-19).
- [ ] **N-21 (P1, L) Crate `clawcrew-*` → `galleon-*`** (22 crate): urutan bawah→atas (api, infra, log, config, …, runtime, gateway); `[workspace.dependencies]`, `use clawcrew_*`, `Cargo.lock`. Satu crate per PR; gunakan `cargo-edit`/skrip. Dep: N-00, N-02, R-04 (arah dependensi) agar tidak mengacaukan pemecahan crate. Koordinasi dengan R-10..R-16: **rename setelah pecah/hapus** untuk crate yang akan diubah.
- [ ] **N-22 (P1, M) Identifier & string di dalam kode:** tipe/ fungsi/ komentar/ doc comment (`ClawCrew*`, `zeroclaw`), log target, `tracing` span, metric prefix. Dep: N-21 per crate.
- [ ] **N-23 (P1, M) Config & env:** `CLAWCREW_*`→`GALLEON_*`, direktori `~/.clawcrew`→`~/.config/galleon` (vault path sudah `galleon`); migrasi data (tanpa alias env). Dep: N-00. **Risiko data pengguna.**
- [ ] **N-24 (P1, M) WIT v1 (ODR-17):** v0 (`clawcrew:plugin@0.1.0`) tetap beku. Buat `wit/v1/` dengan paket `galleon:plugin@1.0.0` sebagai versi aktif; host mendukung v0 dan v1 selama masa transisi; fixture dan `plugin-codebase-standard.md` pindah ke v1; breaking-change check memakai marker `.frozen` per versi. Dep: N-00, R-30.
- [ ] **N-25 (P1, M) Go engine:** module path, package, string, header, log field (`engine/go.mod`, `app/`), tanda “FleetAI”. Dep: N-00.
- [ ] **N-26 (P1, S) UI `web/`:** nama paket `react-example`→`galleon-web`, `Fleet AI` di title/teks/i18n, localStorage key dengan migrasi. (Tumpang tindih U-05.)
- [ ] **N-27 (P1, S) Desktop `apps/tauri`:** `fleet-ai-desktop`→`galleon-desktop`, `productName`, `identifier` (`com.galleon.sovereign` — verifikasi), window title, tray, updater endpoint; perbarui CI (P-05).
- [ ] **N-28 (P1, M) CI/scripts/Docker/Make:** `scripts/ci` (439), `xtask` (354), workflows, `Dockerfile*`, image name `ghcr.io/…/galleon`.
- [ ] **N-29 (P2, S) Metadata repo:** `README.md`, `LICENSE` header attribution (**pertahankan atribusi lisensi upstream**), `CONTRIBUTING`, `SECURITY`, `CODE_OF_CONDUCT`, issue/PR template, repo description, URL `diezy-realm/galleon-fleet`.
- [ ] **N-30 (P2, M) Dokumen normatif non-ADR:** `AGENTS.md`, `CLAUDE.md`, `.github/copilot-instructions.md`, `standard-docs/tech-standart`, `foundation` — pakai nama baru; ADR lama cukup diberi catatan alias di `naming-map.md`.

## N-3x Verifikasi

- [ ] **N-40 (P1, M) Gerbang akhir:** `brand_gate` hijau di CI, `cargo nextest run --workspace`, `go test -race ./...`, `npm run build`, build desktop; smoke test upgrade (data lama → baru).
- [ ] **N-41 (P2, S) Catat sisa pengecualian** (atribusi/historis) di `naming-map.md` dengan alasan.
