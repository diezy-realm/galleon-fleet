# Lane D — Dokumen & Kebersihan Repo

- [ ] **D-01 (P0, S) Commit terpisah per concern** (ODR-10): UI/desktop, Rust, standard-docs, agent config.
- [ ] **D-02 (P1, M) Sinkron drift D1–D14** (`foundation/04`): `analysis-conclusion.md` (ExecuteTurn/get_decrypted_secret), persistensi, "25 crates", MSRV.
- [x] **D-03 (P1, S) Catatan alias `web-2`/`tauri-2`** [Selesai PR-01] Ditambahkan callout normatif ODR-01 di 5 dokumen normatif (`tech-stack/{web,tauri}-conventions.md`, `architecture/00-overview.md`, `01-boundary-policy.md`, `api-contracts/http-engine-api.md`) merujuk ODR-01.
- [ ] **D-04 (P1, S) Errata ADR-001** sebagai file baru (WhatsApp Tier A, WIT v0 beku, bluesky). Jangan edit ADR asli.
- [ ] **D-05 (P1, S) `.kiro/steering`** rujukan `docs/finalize`, `docs/fundamental` → `standard-docs/`.
- [ ] **D-06 (P2, S) `docs/README.md` otoritas:** nyatakan `standard-docs/` = SSOT Galleon, `docs/book` = warisan ClawCrew.
- [ ] **D-07 (P2, M) Rapikan steering agent** (`.gemini/.kiro/.claude/.agents/.copilot`): satu sumber, sisanya pointer.
- [ ] **D-08 (P2, S) Kebersihan root:** artefak, file sementara, `.gitignore`; putuskan `standard-docs/decisions/bug-fix/`.
- [ ] **D-09 (P2, M) Perbarui `prd-hybrid-engine.md`** atau tandai superseded oleh `product/galleon-product-definition.md`.
- [ ] **D-10 (P3, S) Template ODR/ADR/tiket** di `standard-docs/templates/`.
