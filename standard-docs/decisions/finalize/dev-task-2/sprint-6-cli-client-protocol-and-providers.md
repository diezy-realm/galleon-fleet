---
sprint: 6
title: CLI/Client Protocol & Provider Reliability/Memory Governance
status: planned
depends_on: Sprint 4 §B (BG-E1/E2 Client Protocol harus ada dulu)
---

# Sprint 6 — CLI/Client Protocol & Providers

## A. CLI & Klien (dep Client Protocol Sprint 4)

- [ ] **BR-E1 · R-36 · PR-57 (P1, M)** Audit `galleoncode` & `galleonrelay`: petakan pemanggilan ke daemon Rust → endpoint Go.
- [ ] **BR-E2 · R-34 · PR-57 (P1, M)** CLI `galleon` `chat/run/approve/status` → Go :9090; `service/doctor/onboard/vault` tetap lokal. Dep: BG-E2 (Sprint 4).
- [ ] **BR-E3 · R-40 · PR-58 (P1, L)** Retarget `galleoncode` (TUI) ke Client Protocol; matikan `spawn_owned_ephemeral_daemon`. Dep: BG-E2 (Sprint 4).
- [ ] **BR-E4 · R-42 · PR-58 (P1, M)** Retarget `galleon-acp-bridge` (ODR-24).

## B. Provider LLM — Reliability & OAuth

- [ ] **BL-25 (MEDIUM)** BYOC — override `client_id` OAuth xAI/OpenAI. Naik prioritas kalau keputusan open-source "ya".
- [ ] **BL-26 (LOW effort/MEDIUM impact)** Dokumentasikan model keamanan single-tenant auth.
- [ ] **BL-27 (LOW effort)** Perbaiki UX pesan error OAuth refresh-revoked.
- [ ] **BL-28 (MEDIUM, tunggu BL-15 untuk baseline)** Client-side rate limiting LLM provider calls. Dep: BL-15 (Sprint 5, selesai duluan).
- [ ] **BL-29 (LOW-MEDIUM)** Lengkapi test reliability: cascading-all-down & cooldown-expiry.

## C. Memory Governance

- [ ] **BL-30 (MEDIUM-HIGH)** `erase_identity()` lintas-backend untuk memory — saat ini primitif delete nyata ada TAPI tidak ada single entry-point "hapus semua data saya".
- [ ] **BL-31 (MEDIUM, scope besar, evaluasi trait dulu)** Hygiene/retensi otomatis backend-agnostic — saat ini hanya jalan di SQLite.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A | `apps/zerocode` (→ `galleoncode`), `apps/zerorelay` (→ `galleonrelay`), `crates/galleon-acp-bridge/` | Backend-Rust |
| B | `crates/clawcrew-providers/src/` | Backend-Rust |
| C | `crates/clawcrew-memory/src/` | Backend-Rust |

## Keluaran Sprint

- CLI/TUI sepenuhnya lewat Client Protocol Go, tidak ada lagi daemon Rust ephemeral yang di-spawn langsung.
- Satu entry-point hapus semua data user, bekerja lintas backend memory (bukan hanya SQLite).
