# Lane D — Dokumen & Kebersihan Repo

- [ ] **D-01 (P0, S) Commit terpisah per concern** (ODR-10): UI/desktop, Rust, standard-docs, agent config.
- [x] **D-02 (P1, M) Sinkron drift D1–D14** (`foundation/04`): [Selesai PR-20] `analysis-conclusion.md` (ExecuteTurn/get_decrypted_secret), persistensi, 27 crates, MSRV 1.96.0 disinkronkan di tabel register dan dokumen normatif.
- [x] **D-03 (P1, S) Catatan alias `web-2`/`tauri-2`** [Selesai PR-01] Ditambahkan callout normatif ODR-01 di 5 dokumen normatif (`tech-stack/{web,tauri}-conventions.md`, `architecture/00-overview.md`, `01-boundary-policy.md`, `api-contracts/http-engine-api.md`) merujuk ODR-01.
- [ ] **D-04 (P1, S) Errata ADR-001** sebagai file baru (WhatsApp Tier A, WIT v0 beku, bluesky). Jangan edit ADR asli.
- [x] **D-05 (P1, S) `.kiro/steering`** [Selesai PR-20] Rujukan `docs/finalize`, `docs/fundamental` diarahkan ke kanonikal `standard-docs/`.
- [x] **D-06 (P2, S) `docs/README.md` otoritas:** [Selesai PR-20] Ditambahkan `docs/README.md` menegaskan `standard-docs/` = SSOT Galleon Fleet, dan `docs/` berisi lineage historis.
- [ ] **D-07 (P2, M) Rapikan steering agent** (`.gemini/.kiro/.claude/.agents/.copilot`): satu sumber, sisanya pointer.
- [ ] **D-08 (P2, S) Kebersihan root:** artefak, file sementara, `.gitignore`; putuskan `standard-docs/decisions/bug-fix/`.
- [x] **D-09 (P2, M) Perbarui `prd-hybrid-engine.md`** [Selesai PR-20] Ditandai superseded oleh `product/galleon-product-definition.md`.
- [ ] **D-10 (P3, S) Template ODR/ADR/tiket** di `standard-docs/templates/`.
