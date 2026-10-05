---
sprint: 7
title: Product Features (FE Organization/Approval UI) & Payment-Lockout Preventif
status: planned
depends_on: Sprint 4 §E (BG-F1/F3 org model), Sprint 1 §A (BG-B4 untuk FW-C5)
---

# Sprint 7 — Product Features & Payment-Lockout Preventif

## A. Fitur Produk FE (dep org model Sprint 4)

- [ ] **FW-C1 · U-13 · PR-72 (P2, L)** Layar organisasi: Quartermaster/Ship/Squad/Crew, memori, approval, audit/policy read-only. Dep: BG-F1, BG-F3 (Sprint 4).
- [ ] **FW-C2 · U-14 · PR-98 (P3, M)** i18n & aksesibilitas dasar.
- [ ] **FW-C3 · (ODR-07)** Terminal UI memakai `POST /api/terminal/exec` setelah `BG-A4` (sudah selesai) + `S2-SEC-01` (Sprint 0, approval-identity) — tampilkan prompt approval yang BENAR-BENAR terverifikasi identitas, bukan checkbox kosong.
- [ ] **FW-C4 · (G-20)** Halaman perangkat terhubung: daftar perangkat, revoke, QR pairing. Dep: `BG-C2` (Sprint 0).
- [ ] **FW-C5 · (G-24)** UI persetujuan `MemoryProposal` (learning by consent). Dep: `BG-F4` (Go, lihat §B di bawah).

## B. Learning by Consent & ADR Lanjutan

- [ ] **BG-F4 · G-24 · PR-96 (P2, L)** Learning by consent lanjutan: tipe `MemoryProposal` sudah ada (T-3.2 ✔); tinggal alur umpan balik → persetujuan Owner → versi/rollback di atas store baru. Dep: `BG-B4` (Sprint 1).
- [ ] **BG-F6 · G-37 · PR-99 (P3, S)** ADR opsional Delegated Authority Chain (rujukan `research/verifiable-intent-assessment.md` §3c — di-upgrade jadi gate WAJIB sebelum desain fitur commerce, bukan lagi opsional, per keputusan Owner 2026-10-05).

## C. Payment-Lockout Gate (Preventif — nol tool pembayaran eksis saat ini, pasang gate SEBELUM kapabilitas dibangun)

> Desain+riset sudah lengkap (`decisions/payment-lockout-gate-scheme.md`, diperkuat 4 temuan riset: AP2 v0.2, insiden Comet/Replit, OWASP Agentic ASI03/09/10). Ditunda Owner 2026-10-05 (fokus BL-32 dulu) — sekarang BL-32 sudah di Sprint 0, jadi sprint ini adalah titik eksekusi wajar.

- [ ] **BL-40 (MEDIUM-importance/LOW-urgency, tidak ada dependency)** `PaymentLockConfig` schema (termasuk `unlock_expires_at`) di `clawcrew-config`.
- [ ] **BL-41 (MEDIUM, SETELAH BL-40)** `PaymentLockManager` + titik gate di `tool_execution.rs` (termasuk expiry check).
- [ ] **BL-42 (MEDIUM-HIGH, paralel dengan BL-41)** Field `payment_category` di `ToolManifest` + fail-closed waktu registrasi.
- [ ] **BL-43 (MEDIUM, SETELAH BL-40/41)** UI Settings "Allow Agents to Make Payments" — **state mentah, bukan ringkasan agent** (OWASP ASI09: state harus dari sistem, bukan laporan agent yang bisa salah/menipu).
- [ ] **BL-44 (LOW-MEDIUM, paralel dengan BL-43)** Audit log perubahan mode Locked→Unlocked.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `web/src/pages/organization/`, `web/src/pages/terminal/`, `web/src/pages/devices/` (baru) | Frontend |
| B | `engine/src/mission/proposal.go` (existing `MemoryProposal`), ADR baru di `standard-docs/decisions/` | Backend-Go + Lead (ADR) |
| C | `crates/clawcrew-config/src/schema.rs`, `crates/clawcrew-runtime/src/agent/tool_execution.rs` (**overlap dengan BL-37/38 Sprint 2 — cek merge status dulu**), `web/src/pages/settings/` | Backend-Rust + Frontend |

## Catatan File-Overlap Lintas-Sprint

`BL-41`/`BL-42` menyentuh `tool_execution.rs` yang SAMA dengan `BL-37`/`BL-38` (Sprint 2). **Pastikan Sprint 2 sudah merge sebelum Sprint 7 §C mulai** — kalau belum, koordinasikan dengan Lead untuk serialisasi tambahan di luar urutan sprint normal.

## Keluaran Sprint

- Layar organisasi FE hidup, menampilkan hierarki Fleet=Perusahaan/Ship=Departemen/Squad=Tim/Crew=individu secara nyata (bukan mock).
- Payment-lockout gate terpasang SEBELUM ada tool pembayaran nyata dibangun — mencegah kelas insiden Comet/Replit sebelum terjadi di Galleon.
