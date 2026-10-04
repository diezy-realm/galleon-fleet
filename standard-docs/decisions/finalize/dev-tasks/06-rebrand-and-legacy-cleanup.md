# 06 — Rebrand ke Galleon Fleet & Pembersihan Legacy (lintas tim)

Acuan: `tech-standart/naming/naming-map.md` (disetujui), ODR-16/17/19. Aturan: **tanpa alias**; rename hanya mekanis (skrip + laporan), tidak bercampur perubahan perilaku; atribusi lisensi upstream dipertahankan. Hapus/pecah **sebelum** rename massal.

## A. Persiapan

- [x] **RB-00 · N-00:** matriks penamaan resmi `naming-map.md` (disetujui Owner).
- [x] **RB-01 · N-01 · PR-06 (P0, S) Inventaris klasifikasi hit** [Selesai PR-06] 1.709 file dan 40.422 hit diklasifikasikan ke `rename` (1.400), `delete` (246), `keep` (43), `needs-migration` (20) di `files/brand-hit-inventory.csv` + ringkasan.
- [ ] **RB-02 · N-02 · PR-80 (P1, M) Skrip rename + `brand_gate.sh`.**

## B. Hapus Legacy (Gelombang 1)

- [x] **RB-10 · N-10 · PR-02 (P0, S) Hapus `bin/engine.exe`** [Selesai PR-02] Binary 40MB `bin/engine.exe` dihapus dari git tracking (`git rm bin/engine.exe`) dan `.gitignore` diperbarui mengabaikan `/bin/` dan `apps/tauri/target/`.
- [ ] **RB-11 · N-11 · PR-17 (P1, M) ⚑ `docs/_archive` (307 file):** tag `archive/pre-galleon`, hapus, perbaiki rujukan `docs/book` di `AGENTS.md`.
- [ ] **RB-12 · N-12 · PR-62 (P1, M) ⚑ Hapus kode warisan tak terpakai** hasil R-00 (REST/WS/A2A, hardware/peripheral). Terkait 08 §C.
- [ ] **RB-13 · N-13 · PR-18 (P1, S) ⚑ Skill agent khusus ClawCrew.**
- [ ] **RB-14 · N-14 · PR-18 (P1, S) ⚑ Workflow CI khusus ClawCrew.**
- [ ] **RB-15 · N-15 · PR-19 (P2, S) ⚑ Packaging warisan** (`dist/`, `nix/`, `deploy-k8s/`, `Containerfile`, `setup.bat`, `install.*`).
- [ ] **RB-16 · N-16 · PR-19 (P2, S) Fixture & contoh tak relevan.**
- [ ] **RB-17 · N-17 · PR-19 (P2, S) Terjemahan tak terpakai;** regenerasi `.ftl/.po`.

## C. Rename (Gelombang 3)

- [ ] **RB-20 · N-20 · PR-83 (P1, M) Binary:** `clawcrew`→`galleon`, `zerocode`→`galleoncode`, `zerorelay`→`galleonrelay`, `acp-bridge`→`galleon-acp-bridge`.
- [ ] **RB-21 · N-21 · PR-82 (P1, L, berseri) Crate `clawcrew-*`→`galleon-*`** (bawah→atas; `runtime` setelah R-16). Catatan: `galleon-i18n`/`galleon-channel-*` sudah berprefiks baru.
- [ ] **RB-22 · N-22 · PR-82 (P1, M) Identifier & string dalam kode.**
- [ ] **RB-23 · N-23 · PR-81 (P1, M) Env `CLAWCREW_*`→`GALLEON_*`**, `~/.clawcrew`→`~/.config/galleon` + migrasi data (tanpa alias).
- [ ] **RB-24 · N-24 · PR-86 (P1, M) WIT v1** `wit/v1` `galleon:plugin@1.0.0`; v0 beku lolos `.frozen`. Pasangan BR-F2.
- [ ] **RB-25 · N-25 · PR-84 (P1, M) Go module** → `github.com/diezy-realm/galleon-fleet/engine`; regen wire/proto.
- [ ] **RB-26 · N-26 · PR-85 (P1, S) UI `web/`** (lihat FW-D1).
- [ ] **RB-27 · N-27 · PR-85 (P1, S) Desktop** (lihat FD-C1).
- [ ] **RB-28 · N-28 · PR-87 (P1, M) CI/scripts/xtask/Docker/Make;** image `ghcr.io/diezy-realm/galleon-fleet`.
- [ ] **RB-29 · N-29 · PR-87 (P2, S) Metadata repo;** pertahankan atribusi lisensi upstream.
- [ ] **RB-30 · N-30 · PR-87 (P2, M) Dokumen normatif non-ADR** (`AGENTS.md`, `CLAUDE.md`, copilot-instructions, tech-standart, foundation, **dan `finalize/`**).

## D. Gerbang Akhir

- [ ] **RB-40 · N-40 · PR-88 (P1, M) Gerbang akhir:** `brand_gate` hijau, nextest workspace, `go test -race`, `npm run build`, build desktop.
- [ ] **RB-41 · N-41 · PR-88 (P2, S) Catat sisa pengecualian** di `naming-map.md`.
