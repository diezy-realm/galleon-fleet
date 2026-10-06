---
sprint: 8
title: Rebrand Massal, WIT v1, CI Cleanup, Release/Installer
status: planned
depends_on: Sprint 0-7 SELURUHNYA — aturan non-negosiasi dev-tasks asli (hapus/pecah SEBELUM rename massal)
---

# Sprint 8 — Rebrand & Release (SELALU TERAKHIR)

> **Aturan keras dari dev-tasks asli, dipertahankan:** "Hapus/pecah **sebelum** rename massal." Sprint ini TIDAK BOLEH mulai sebelum Sprint 0-7 selesai — rename menyentuh identifier di seluruh kode, kalau dijalankan lebih awal setiap sprint lain akan bentrok merge terus-menerus.

## A. Persiapan & Hapus Legacy Sisa

- [ ] **RB-02 · N-02 · PR-80 (P1, M)** Skrip rename + `brand_gate.sh`.
- [ ] **RB-12 · N-12 · PR-62 (P1, M)** ⚑ Hapus kode warisan tak terpakai hasil R-00 (REST/WS/A2A, hardware/peripheral). Dep: `BR-D4` (Sprint 4, hapus `rpc`).
- [ ] **RB-16 · N-16 · PR-19 (P2, S)** Fixture & contoh tak relevan.
- [ ] **RB-17 · N-17 · PR-19 (P2, S)** Terjemahan tak terpakai; regenerasi `.ftl/.po`.

## B. Rename Massal (Gelombang 3 — mekanis, tidak campur perilaku)

- [ ] **RB-20 · N-20 · PR-83 (P1, M)** Binary: `clawcrew`→`galleon`, `zerocode`→`galleoncode`, `zerorelay`→`galleonrelay`, `acp-bridge`→`galleon-acp-bridge`.
- [ ] **RB-21 · N-21 · PR-82 (P1, L, berseri)** Crate `clawcrew-*`→`galleon-*` (bawah→atas; `runtime` setelah R-16/Sprint 4 selesai).
- [ ] **RB-22 · N-22 · PR-82 (P1, M)** Identifier & string dalam kode.
- [ ] **RB-23 · N-23 · PR-81 (P1, M)** Env `CLAWCREW_*`→`GALLEON_*`, `~/.clawcrew`→`~/.config/galleon` + migrasi data (tanpa alias).
- [ ] **RB-24 · N-24 · PR-86 (P1, M)** WIT v1 `wit/v1` `galleon:plugin@1.0.0`; v0 beku lolos `.frozen`. Pasangan `BR-F2` (Sprint 5).
- [ ] **RB-25 · N-25 · PR-84 (P1, M)** Go module → `github.com/diezy-realm/galleon-fleet/engine`; regen wire/proto.
- [ ] **RB-26 · N-26 · PR-85 (P1, S)** UI `web/` (lihat `FW-D1`).
- [ ] **RB-27 · N-27 · PR-85 (P1, S)** Desktop (lihat `FD-C1`).
- [ ] **RB-28 · N-28 · PR-87 (P1, M)** CI/scripts/xtask/Docker/Make; image `ghcr.io/diezy-realm/galleon-fleet`.
- [ ] **RB-29 · N-29 · PR-87 (P2, S)** Metadata repo; pertahankan atribusi lisensi upstream.
- [ ] **RB-30 · N-30 · PR-87 (P2, M)** Dokumen normatif non-ADR (`AGENTS.md`, `CLAUDE.md`, copilot-instructions, tech-standart, foundation, **dan `finalize/`**).
- [ ] **FW-D1 · N-26 · PR-85 (P1, S)** `react-example`→`galleon-web`, "Fleet AI"→"Galleon Fleet", migrasi key localStorage.
- [ ] **FD-C1 · N-27 · PR-85 (P1, S)** `fleet-ai-desktop`→`galleon-desktop`, `productName` "Galleon Fleet". Pertahankan `identifier = com.galleon.sovereign` (verifikasi).

## C. CI Cleanup & Platform

- [ ] **PL-A5 · P-08 · PR-93 (P2, M)** Pangkas workflow warisan (39) & tandai required checks.
- [ ] **PL-B2 · P-02 + P-03 · PR-90 (P1, M)** Dockerfile & `include` `/web/dist`: sesuaikan stub `apps/tauri`; putuskan UI di-embed vs disajikan Nginx (ADR kecil).

## D. Deploy & Rilis

- [ ] **PL-C1 · P-10 · PR-91 (P1, L)** Nginx + TLS: reverse proxy ke Go :9090 (WS/SSE), HSTS, rate-limit, header keamanan. Dep: `BG-C3` (Sprint 0).
- [ ] **PL-C2 · P-11 · PR-91 (P2, M)** Panduan deploy lokal↔VPS/cloud, model ancaman, hardening; tetapkan peran `galleonrelay` vs Nginx.
- [ ] **FD-B1 · U-09 · PR-92 (P1, M)** Sidecar desktop: jalankan Go engine + Rust gateway (lifecycle, health, shutdown).
- [ ] **FD-B2 · (ODR-01)** System tray, dialog file native, window — fitur OS shell, tiap command dicatat di allowlist.
- [ ] **PL-D1 · P-12 · PR-93 (P2, L)** Pipeline rilis: desktop (MSI/NSIS, signing), server (Docker image, checksum, SBOM). Dep: FD-B1. **Sub-item eks-BL-33 (Windows Authenticode signing):** `signtool sign /fd SHA256 /tr <timestamp-url> /td SHA256` di job `build-desktop-windows`, graceful-degrade sama seperti macOS (skip kalau secret absen). Dep: pengadaan sertifikat (non-teknis, bisa jadi blocker waktu).
- [ ] **FD-C2 · P-12 · PR-93 (P2, L)** Bundling Windows (MSI/NSIS, signing) bersama Platform. **Sub-item eks-BL-34 (`tauri.bundled.conf.json` hilang):** verifikasi SEGERA (murah) apakah file ini genuinely hilang atau di-generate runtime oleh `scripts/desktop/prepare-kernel.sh` — kalau hilang, 3 job desktop build berisiko gagal CI diam-diam.
- [ ] **PL-D2 · P-13 · PR-94 (P3, M)** Installer pilih Tier B.
- [ ] **BL-10 (MEDIUM, dari `automation-cron-webhook-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)** Service installer setara `kirocrew service install` — command `galleon service install/status/uninstall`, generate unit systemd (Linux)/plist launchd (macOS), sudo-scope dibatasi (hanya langkah instalasi, daemon jalan sebagai user biasa). Independen dari PL-D1/D2 (beda OS-integration layer), tapi cocok dikerjakan bersamaan karena temanya sama (deployment convenience).
- [ ] **BL-11 (MEDIUM, risiko data-loss operasional)** State migration tooling — dokumentasi/script untuk pindahkan state Galleon (cron `jobs.db`, memory SQLite/vector, config, skills) ke host baru. **Perhatian khusus:** verifikasi dulu apakah `clawcrew-memory` pakai SQLite WAL mode — kalau ya, sync WAJIB include `.db-wal`/`.db-shm` atau data terbaru hilang diam-diam.

## E. QA/Integrasi Akhir & Gerbang Akhir

- [ ] **PL-E1 · PR-plan §4 (GW-6)** Branch `integration/wave-N`: `cargo nextest run --workspace` + `go test -race ./...` + `npm run build`.
- [ ] **PL-E2** Smoke e2e: Engine + gateway + UI/desktop; uji negatif fail-closed.
- [ ] **PL-E3 · N-41 · PR-88** Smoke test upgrade data lama (`~/.clawcrew`) → baru.
- [ ] **PL-E4 · T-9.1 / CO-07** End-to-end demo manual: UI/Tauri → Go → Rust, tanpa fallback/konstanta palsu, bukti di `finalize/e2e-proof.md`.
- [ ] **CO-05 (dipersempit) · T-6.9** Handler protokol stateful yang BERTAHAN (`sse`, `ws`, `ws_approval`, `ws_sop_runs`, `acp`) → `RouteHandler`. Hanya untuk handler yang tidak dihapus (lihat `CO-09` keputusan Owner).
- [ ] **CO-06 (dipersempit) · T-6.10** Handler transport lanjutan yang BERTAHAN (`webhook_ingress`, `tls`, `version`, `node_tool`, `hardware_context`, dst). Hanya untuk handler yang tidak dihapus.
- [ ] **CO-08** Hapus artefak Phase 2 tertinggal: komentar/doc lama yang menyebut `web-2`.
- [ ] **RB-40 · N-40 · PR-88 (P1, M)** Gerbang akhir: `brand_gate` hijau, nextest workspace, `go test -race`, `npm run build`, build desktop.
- [ ] **RB-41 · N-41 · PR-88 (P2, S)** Catat sisa pengecualian di `naming-map.md`.
- [ ] **GV-01 · D-01 · PR-00 (P0, S)** Commit terpisah per concern lalu PR dari `feat/refactoring-rust`.
- [ ] **GV-11** ⚑ Sign-off Owner untuk PR bertanda ⚑ sebelum merge.
- [ ] **GV-12** Principle re-verify semua `[x]` di `finalize/task-breakdown.md` memakai `testing-gate-checklist.md` (gate G0–G8) sebelum ditutup resmi — **termasuk re-verify `BR-F1` yang perlu dicentang `[x]` manual (lihat README §Reconciliation)**.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A/B | Seluruh repo (mekanis, skrip rename) — `RB-21` berseri bawah→atas | Lintas-tim, dikoordinasi Platform/DevOps |
| C | `.github/workflows/`, `Dockerfile` | Platform/DevOps |
| D | `nginx/`, `apps/tauri/`, `.github/workflows/release-stable-manual.yml` | Platform/DevOps + Backend-Rust (sidecar) |
| E | `tests/architecture/`, `finalize/e2e-proof.md` | QA + Lead (sign-off) |

## Keluaran Sprint

- Rename `clawcrew`→`galleon` tuntas di SELURUH kode tanpa alias (ODR-19).
- Release Windows+macOS+Linux ter-sign, installer siap distribusi.
- `brand_gate` + nextest workspace + `go test -race` + `npm run build` semua hijau sebagai gerbang akhir sebelum rilis resmi.
