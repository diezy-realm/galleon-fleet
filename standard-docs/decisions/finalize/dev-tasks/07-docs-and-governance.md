# 07 — Dokumen & Governance (Tech Lead / Docs)

Gerbang: GW-7. ADR/ODR lama tidak diedit; koreksi lewat file baru (errata/addendum).

- [ ] **GV-01 · D-01 · PR-00 (P0, S) Commit terpisah per concern** lalu PR dari `feat/refactoring-rust` (commit lokal sudah dipisah; belum di-push).
- [x] **GV-02 · D-03 · PR-01 (P1, S) Catatan alias `web-2`/`tauri-2`** di 5 dokumen normatif (`tauri-conventions`, `web-conventions`, `00-overview`, `01-boundary-policy`, `http-engine-api.md`) merujuk ODR-01. `finalize/` sudah diseragamkan ke `web/`/`apps/tauri/`.
- [ ] **GV-03 · D-02 · PR-20 (P1, M) Sinkron drift D1–D14** (`foundation/04`).
- [ ] **GV-04 · D-05 · PR-20 (P1, S) `.kiro/steering`:** rujukan `docs/finalize`/`docs/fundamental` → `standard-docs/`.
- [ ] **GV-05 · D-06 · PR-20 (P2, S) `docs/README.md`:** nyatakan `standard-docs/` = SSOT Galleon.
- [ ] **GV-06 · D-09 · PR-20 (P2, M) `prd-hybrid-engine.md`:** perbarui atau tandai superseded.
- [ ] **GV-07 · D-04 · PR-21 (P1, S) Errata ADR-001** (file baru).
- [ ] **GV-08 · D-07 · PR-21 (P2, M) Rapikan steering agent:** satu sumber, lainnya pointer.
- [ ] **GV-09 · D-08 · PR-21 (P2, S) Kebersihan root** & putuskan `decisions/bug-fix/`.
- [ ] **GV-10 · D-10 · PR-21 (P3, S) Template ODR/ADR/tiket** di `standard-docs/templates/`.
- [ ] **GV-11 · Sign-off Owner ⚑** untuk PR bertanda ⚑ sebelum merge (PR-04, 17–19, 55, 59–62).
- [ ] **GV-12 · Principle re-verify** semua `[x]` di `finalize/task-breakdown.md` memakai `testing-gate-checklist.md` (gate G0–G8 masih kosong) sebelum ditutup resmi.
