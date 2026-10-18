# Workflow Directory Layout

GitHub Actions only loads workflow entry files from:

- `.github/workflows/*.yml`
- `.github/workflows/*.yaml`

Subdirectories are not valid locations for workflow entry files.

Repository convention:

1. Keep runnable workflow entry files at `.github/workflows/` root.
2. Keep cross-tooling/local CI scripts under `dev/` or `scripts/ci/` when used outside Actions.

Workflow behavior documentation in this directory:

- `.github/workflows/master-branch-flow.md`

## Bespoke CI gate provenance

Every repository-specific CI gate must record the invariant it protects, the
issue, pull request, or incident that motivated it, and the condition under
which it can be retired. Keep a short breadcrumb beside the job or the step in the
workflow YAML.

For script-backed gates, keep the durable explanation in this
registry rather than in source comments covered by the comment-hygiene gate.

| Gate | Protected invariant | Origin | Retirement condition |
|---|---|---|---|
| Repository Structure | Index gitlinks match `.gitmodules`, and only the approved translation submodule is present. | [PR #8516](https://github.com/clawcrew-labs/clawcrew/pull/8516) | Retire only when submodules are removed or an equivalent repository-policy check enforces the same restriction. |
| Zerocode RPC Boundary | Zerocode depends only on shared `clawcrew-api` contracts; runtime and other implementation behavior remains behind RPC. | [PR #7850](https://github.com/clawcrew-labs/clawcrew/pull/7850) | Retire only if the architecture intentionally permits implementation dependencies or an equivalent dependency-boundary check replaces it. |
| Nix Hash Drift | `nix/hashes.json` contains exactly the fixed-output hash keys required by Git dependencies in `Cargo.lock`. | [PR #8336](https://github.com/clawcrew-labs/clawcrew/pull/8336) | Retire only when Nix no longer uses the tracked hash registry or another mechanism enforces the same synchronization. |
| Installer Drift | Tracked installer, packaging, container, Nix, and installation-documentation surfaces match the canonical generator specification. | [PR #7558](https://github.com/clawcrew-labs/clawcrew/pull/7558) | Retire only when those surfaces are no longer generated and tracked, or an equivalent consistency check replaces this gate. |
| Brand Compliance Gate | No unauthorized legacy brand names (`clawcrew`, `zeroclaw`) leak into production source code. | [PR #88 (Sprint 8)](https://github.com/diezy-realm/galleon-fleet) | Permanent release gate. |

## Galleon Fleet — Required CI Checks & Pruning Matrix (PL-A5)

### Required Gates for PR Merge:
1. **`ci.yml`**: Rust compilation, `cargo clippy`, format, dan workspace tests.
2. **`engine-ci.yml`**: Go engine unit tests (`go test -race ./src/...`).
3. **`web-check.yml`**: Frontend TypeScript typecheck (`tsc --noEmit`) dan bundle build (`vite build`).
4. **`desktop-check.yml`**: Tauri v2 desktop build dan validation.
5. **`pr-testing-gate.yml`**: Quality gate and DoD checklist verification.
6. **`brand-gate`** (`scripts/ci/brand_gate.sh`): Verifikasi kepatuhan penamaan merek.

### Pruned / Retired Workflows:
- `pub-aur.yml`, `aur-freshness-check.yml`: Retired (distribusi Arch AUR eksternal dinonaktifkan untuk microkernel).
- `pub-crates.yml`: Retired (workspace internal tidak lagi dipublikasikan terpisah ke crates.io).
- `pub-scoop.yml`, `scoop-bucket-canary.yml`: Retired (distribusi Scoop eksternal dipangkas menjadi MSI/NSIS mandiri).
- `testing-to-prerelease-pr.yml`, `release-gate-approval.yml`: Dihapus (YAML tidak valid sehingga tidak pernah berjalan, dan memakai runner GitHub-hosted yang tidak tersedia untuk org ini).
- `ci-code-analysis.yml` (Semgrep), `npm-deps-review.yml`: Dihapus (hanya terpicu ke `master` yang tidak ada, dan keduanya butuh GitHub Advanced Security untuk SARIF / dependency review).
- `sop-eval-advisory.yml`: Dihapus (advisory `continue-on-error` yang selalu gagal tersembunyi; `clawcrew-eval` tetap diuji oleh job `test` di `ci.yml`).
- `validate-translations-pin.yml`: Dihapus (submodule `docs/book/po` tidak punya gitlink di repo ini).
- `codeql.yml`: hanya `workflow_dispatch` sampai GitHub Advanced Security aktif.

### Runner & biaya
Semua job berjalan di Blacksmith; runner GitHub-hosted tidak tersedia untuk org ini. Job macOS (`blacksmith-6vcpu-macos-15`, tarif 20x menit Linux 2-vCPU) hanya berjalan saat push ke `main`/`testing` (`build-macos`), mingguan (`platform-tests.yml`), atau manual. Audit keamanan terjadwal (`daily-audit.yml`, `daily-npm-audit.yml`, `ci-sbom.yml`) berjalan mingguan; `security` di `ci.yml` tetap menjalankan `cargo deny check` di setiap PR.

`Test (Landlock)` tidak menjadi syarat `CI Required Gate`: kernel Blacksmith tidak mengaktifkan Landlock LSM, jadi job ini sengaja gagal terlihat (bukan skip diam-diam). Cakupan sandbox Landlock tidak teruji di CI sampai job ini berjalan lagi di kernel yang mendukung Landlock; saat itu kembalikan `test-landlock` ke `gate.needs`.
