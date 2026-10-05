# Lane D — Dokumen & Kebersihan Repo

- [ ] **D-01 (P0, S) Commit terpisah per concern** (ODR-10): UI/desktop, Rust, standard-docs, agent config.
- [x] **D-02 (P1, M) Sinkron drift D1–D14** (`foundation/04`): [Selesai PR-20] `analysis-conclusion.md` (ExecuteTurn/get_decrypted_secret), persistensi, 27 crates, MSRV 1.96.0 disinkronkan di tabel register dan dokumen normatif.
- [x] **D-03 (P1, S) Catatan alias `web-2`/`tauri-2`** [Selesai PR-01] Ditambahkan callout normatif ODR-01 di 5 dokumen normatif (`tech-stack/{web,tauri}-conventions.md`, `architecture/00-overview.md`, `01-boundary-policy.md`, `api-contracts/http-engine-api.md`) merujuk ODR-01.
- [x] **D-04 (P1, S) Errata ADR-001** [Selesai PR-21] Dibuat `standard-docs/decisions/refactoring-rust/phase-2/ADR-001-errata.md` (WhatsApp Tier A, WIT v0 beku, Bluesky native crate); ADR-001 asli tetap immutable.
- [x] **D-05 (P1, S) `.kiro/steering`** [Selesai PR-20] Rujukan `docs/finalize`, `docs/fundamental` diarahkan ke kanonikal `standard-docs/`.
- [x] **D-06 (P2, S) `docs/README.md` otoritas:** [Selesai PR-20] Ditambahkan `docs/README.md` menegaskan `standard-docs/` = SSOT Galleon Fleet, dan `docs/` berisi lineage historis.
- [x] **D-07 (P2, M) Rapikan steering agent** (`.gemini/.kiro/.claude/.agents/.copilot`): [Selesai PR-21] `.gemini/steering.md` ditegaskan sebagai single authority; AGENTS.md dan sub-agent lainnya merujuk ke sana.
- [x] **D-08 (P2, S) Kebersihan root:** [Selesai PR-21] File sementara/artefak dibersihkan, `.gitignore` dipelihara; `standard-docs/decisions/bug-fix/` diratifikasi sebagai catatan audit quality gate review.
- [x] **D-09 (P2, M) Perbarui `prd-hybrid-engine.md`** [Selesai PR-20] Ditandai superseded oleh `product/galleon-product-definition.md`.
- [x] **D-10 (P3, S) Template ODR/ADR/tiket** [Selesai PR-21] Template dibuat di `standard-docs/templates/` (`template-odr.md`, `template-adr.md`, `template-task.md`).
