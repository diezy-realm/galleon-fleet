# 07 — Dokumen & Governance (Tech Lead / Docs)

Gerbang: GW-7. ADR/ODR lama tidak diedit; koreksi lewat file baru (errata/addendum).

- [ ] **GV-01 · D-01 · PR-00 (P0, S) Commit terpisah per concern** lalu PR dari `feat/refactoring-rust` (commit lokal sudah dipisah; belum di-push).
- [x] **GV-02 · D-03 · PR-01 (P1, S) Catatan alias `web-2`/`tauri-2`** di 5 dokumen normatif (`tauri-conventions`, `web-conventions`, `00-overview`, `01-boundary-policy`, `http-engine-api.md`) merujuk ODR-01. `finalize/` sudah diseragamkan ke `web/`/`apps/tauri/`.
- [x] **GV-03 · D-02 · PR-20 (P1, M) Sinkron drift D1–D14** (`foundation/04`): [Selesai PR-20] Klaim MSRV (1.96.0), jumlah crate (27), Go 1.27+, dan rujukan SSOT disinkronkan.
- [x] **GV-04 · D-05 · PR-20 (P1, S) `.kiro/steering`:** [Selesai PR-20] Rujukan `docs/finalize`/`docs/fundamental` diarahkan ke kanonikal `standard-docs/`.
- [x] **GV-05 · D-06 · PR-20 (P2, S) `docs/README.md`:** [Selesai PR-20] Dibuat `docs/README.md` menyatakan `standard-docs/` = SSOT Galleon Fleet, dan `docs/` berisi lineage historis.
- [x] **GV-06 · D-09 · PR-20 (P2, M) `prd-hybrid-engine.md`:** [Selesai PR-20] Diberi notice tegas superseded oleh `product/galleon-product-definition.md`.
- [x] **GV-07 · D-04 · PR-21 (P1, S) Errata ADR-001** (file baru): [Selesai PR-21] Dibuat `standard-docs/decisions/refactoring-rust/phase-2/ADR-001-errata.md` meratifikasi WhatsApp Tier A, WIT v0 beku, dan Bluesky native crate.
- [x] **GV-08 · D-07 · PR-21 (P2, M) Rapikan steering agent:** [Selesai PR-21] `.gemini/steering.md` ditegaskan sebagai satu SSOT operasional; `.kiro`, `.claude`, `.copilot`, `.agents` merujuk ke sana dan `standard-docs/`.
- [x] **GV-09 · D-08 · PR-21 (P2, S) Kebersihan root** & putuskan `decisions/bug-fix/`: [Selesai PR-21] Root repo bersih (untracked = 0); `standard-docs/decisions/bug-fix/` ditetapkan sebagai audit quality gate review resmi.
- [x] **GV-10 · D-10 · PR-21 (P3, S) Template ODR/ADR/tiket** di `standard-docs/templates/`: [Selesai PR-21] Dibuat `template-odr.md`, `template-adr.md`, dan `template-task.md`.
- [ ] **GV-11 · Sign-off Owner ⚑** untuk PR bertanda ⚑ sebelum merge (PR-04, 17–19, 55, 59–62).
- [ ] **GV-12 · Principle re-verify** semua `[x]` di `finalize/task-breakdown.md` memakai `testing-gate-checklist.md` (gate G0–G8 masih kosong) sebelum ditutup resmi.
