# PR Plan & Branch-Gateway DoD (untuk koordinasi dengan Principle)

> **Satu dokumen untuk seluruh PR** (126 tiket → PR, matriks cakupan di §5). Setiap PR memetakan tiket roadmap (`R-`, `G-`, `U-`, `P-`, `D-`, `N-`) ke satu branch dan satu review Principle.
> Aturan repo: branch bukan `main`; PR ke `main`; conventional commits; satu concern per PR; deklarasikan `Depends on #…`.
> Kotak `[ ]` diisi nomor PR dan dicentang hanya setelah **seluruh gerbang** di bagian 1 lulus dan Principle approve.

## 1. Gerbang (gateway) wajib per PR

| Gerbang | Berlaku untuk | Perintah / bukti |
|---|---|---|
| **GW-0 Hygiene** | semua | `git diff --check`; tidak ada secret (`gitleaks`/review); commit conventional + trailer; tidak ada perubahan di luar scope tiket |
| **GW-1 Rust** | menyentuh `crates/`, `src/`, `apps/*` Rust | `cargo fmt --all -- --check`; `cargo clippy -p <crate> -- -D warnings`; `cargo nextest run -p <crate>`; Windows: `$env:CARGO_TARGET_DIR="target/agent"`; **nol `unwrap()/expect()` baru di jalur produksi**; tidak ada `#[allow(dead_code)]` baru |
| **GW-2 Go** | menyentuh `engine/` | `go vet ./...`; `go test -race ./...`; `wire` regen bersih (`git diff --exit-code engine/app/wire_gen.go`); wrap error `%w`; zap |
| **GW-3 UI/Desktop** | menyentuh `web/`, `apps/tauri/` | `npm ci && npm run lint && npm run build` (+ `test` bila ada); TS strict tanpa `any`; `cargo check -p galleon-desktop`; tanpa `fetch` langsung ke Rust |
| **GW-4 Boundary** | semua kode | UI/Tauri → Go :9090 → Rust :50052; tidak ada shell/fs di Go atau Tauri; tidak ada vendor LLM di Go; policy hanya di `fleet/policies.go`; rahasia hanya di Vault |
| **GW-5 Kontrak** | menyentuh `proto/`, `wit/`, API HTTP | proto/dokumen `api-contracts/` berubah di PR yang sama; regen kedua sisi; WIT v0 tidak diedit (beku); `.frozen` check |
| **GW-6 Integrasi (branch test)** | PR lintas-tier & akhir tiap gelombang | smoke e2e: start engine Go + gateway Rust → `StartTurn` stream → `ExecuteNativeTool` fail-closed saat Rust mati → UI memuat; hasil tertulis di deskripsi PR |
| **GW-7 Docs/ADR** | perubahan perilaku/arsitektur | dokumen terkait di `standard-docs/` diperbarui; ADR/ODR baru bila paradigma berubah (tidak edit ADR lama) |
| **GW-8 Keamanan** | sandbox/vault/auth/pairing/Nginx/terminal | threat-note di PR (permukaan baru default tertutup); test negatif (tanpa token → ditolak; injeksi → ditolak); log bebas rahasia |
| **GW-9 Rollback** | semua | deskripsi PR memuat risiko, efek samping, cara rollback; PR migrasi data memuat jalur mundur |

**Aturan merge:** semua gerbang relevan hijau + 1 approval Principle + tidak ada thread terbuka. PR rename massal (N-2x) hanya mekanis (skrip + laporan), tidak bercampur perubahan perilaku.

## 2. Konvensi daftar

Format: `- [ ] **PR-xx** `branch` · tiket · gerbang · Dep · fokus review Principle · ukuran`. Centang hanya setelah merge + gerbang lulus. `Dep` = PR yang harus merge dulu. PR bertanda **⚑** butuh sign-off Owner tertulis sebelum merge (penghapusan/perubahan perilaku).
Penamaan PR: `<type>(<scope>): …` (conventional commits). Tidak ada footer atribusi AI di PR (AGENTS.md); trailer commit sesuai kebijakan sesi.

## 3. Daftar PR

### Gelombang 0 — dasar, keamanan P0

- [ ] **PR-00** `feat/refactoring-rust` → main · D-01 (+ N-00 ✔, R-00 ✔ sebagai dokumen) · GW-0,7 · – · 13 commit lokal: penggantian `web/`+`apps/tauri/`, `standard-docs/` (foundation, product, roadmap, ODR, R-00/R-00a, naming, research), agent config. Principle: pastikan commit terpisah per concern, tidak ada rahasia · M
- [x] **PR-01** `docs/alias-notes-normative` · D-03 · GW-0,7 · PR-00 · [Selesai] Catatan alias kanonikal ODR-01 (`web-2` = `web/`, `tauri-2` = `apps/tauri/`) di 5 dokumen normatif · S
- [x] **PR-02** `chore/remove-engine-binary` · N-10 · GW-0 · – · [Selesai commit fabeb12c1] `bin/engine.exe` dihapus dari git tracking + `.gitignore` ignore /bin/ dan apps/tauri/target/ · S
- [x] **PR-03** `fix/tauri-frontend-dist` · U-01, P-01 · GW-0,3 · – · [Selesai commit ec31d7562] `frontendDist=../../web/dist`; root Cargo.toml `exclude = ["apps/tauri"]`; `cargo metadata --no-deps` dan `cargo check` hijau · S
- [x] **PR-04** `security/tauri-remove-terminal-and-fake-metrics` · U-06, U-07, U-16 · GW-3,4,8 · – · [Selesai commit 36e6774a8] Hapus `execute_terminal_command`, proxy `get_collection`/`save_collection` dan `get_engine_processes` ke Go :9090; `web/server.ts` matikan RCE `POST /api/engine/execute` & duplikat Ollama/processes; terminal dikunci · S
- [x] **PR-05** `security/go-fail-closed-tools` · G-01, G-03, G-38 · GW-2,4,8 · – · [Selesai commit 131930265] Hapus sisa eksekusi shell/fs lokal di `services.go`, `sandbox.go`, hapus `plugin.go` & `mcp_client.go`; grep os/exec = 0; fail-closed direct dispatch denial test pass; `resolveDataDir` (`GALLEON_DATA_DIR`) + test path traversal · M
- [ ] **PR-06** `docs/brand-hit-inventory` · N-01 · GW-0 · PR-00 · klasifikasi rename/delete/keep/migrate (CSV + ringkasan) · S

### Gelombang 1 — gerbang kualitas & pembersihan

- [ ] **PR-10** `ci/rust-workspace-lints-and-gates` · R-01, R-03, R-04, R-06 · GW-1 · PR-00 · `[workspace.lints]`, gerbang ukuran, gerbang arah dependensi, baseline waktu build · M
- [ ] **PR-11** `ci/boundary-gates` · P-07 · GW-4,6 · PR-05 · `tests/architecture/` (UI tanpa backend, Tauri tanpa shell, Go tanpa exec/vendor LLM) · M
- [ ] **PR-12** `ci/web-and-desktop` · P-04, P-05, U-12 · GW-3 · PR-03 · CI `web/` & `fleet-ai-desktop`, skrip `typecheck/lint/test`, TS strict · M
- [ ] **PR-13** `ci/engine` · P-06 · GW-2 · – · `go test -race`, vet, staticcheck, cek `wire_gen` · S
- [ ] **PR-14** `chore/web-remove-aistudio-artifacts` · U-05, U-15 · GW-3 · PR-12 · `@google/genai`, `better-sqlite3`, `metadata.json`, `bun.lock`, dll.; audit pemakaian dulu · S
- [ ] **PR-15** `feat/go-terminal-policy-endpoint` · G-04 · GW-2,4,8 · PR-05 · risk-tier + approval + audit; test injeksi ditolak · M
- [ ] **PR-16** `fix/go-remove-ollama-probe` · G-02 · GW-2,4 · PR-05 · LLM/discovery milik Rust · S
- [ ] **PR-17** `chore/legacy-removal-docs-archive` · N-11 · GW-0 · PR-06 ⚑ · buat tag `archive/pre-galleon`; hapus `docs/_archive`; perbaiki rujukan `docs/book` di AGENTS.md · M
- [ ] **PR-18** `chore/legacy-removal-agent-skills-and-workflows` · N-13, N-14 · GW-0 · PR-06 ⚑ · skill & workflow khusus ClawCrew · S
- [ ] **PR-19** `chore/legacy-removal-packaging-fixtures-i18n` · N-15, N-16, N-17 · GW-0 · PR-06 ⚑ · `dist/`, `nix/`, `deploy-k8s/`, fixture, locale tak terpakai · M
- [ ] **PR-20** `docs/drift-sync` · D-02, D-05, D-06, D-09, R-05 · GW-7 · PR-00 · selaraskan klaim usang (D1–D14), MSRV, jumlah crate, `.kiro`, otoritas `docs/README.md`, PRD · M
- [ ] **PR-21** `docs/adr-errata-and-hygiene` · D-04, R-30, D-07, D-08, D-10 · GW-7 · PR-00 · errata ADR-001 (file baru), rapikan steering, kebersihan root, template ODR/ADR · M
- [ ] **PR-22** `feat/tauri-csp-capabilities` · U-08, U-17 · GW-3,8 · PR-03 · hapus `unsafe-eval`, `connect-src` ke :9090, scope fs · S

### Gelombang 2 — pemecahan Rust, identitas, persistensi, protokol

**Rust — kualitas & kontrak**
- [ ] **PR-30** `refactor/rust-unwrap-burndown-<crate>` (berseri) · R-02 · GW-1 · PR-10 · satu PR per crate (gateway, channels, runtime…), nol hit produksi · L
- [ ] **PR-31** `refactor/extract-i18n` · R-10 · GW-1 · PR-10 · 87 ref channels → crate kecil · M
- [ ] **PR-32** `refactor/extract-scrub` · R-11 · GW-1,8 · PR-10 · 36 ref; scrubbing log tetap · M
- [ ] **PR-33** `refactor/api-traits-for-channels` · R-12 · GW-1,5 · PR-10 · tools-scoped/memory_strategy/hooks/sop::types → trait di `api` · L
- [ ] **PR-34** `refactor/channels-decouple-runtime` · R-13 · GW-1,6 · PR-31, PR-32, PR-33 · 677 ref → 0; gerbang R-04 mengunci · L
- [ ] **PR-35** `fix/grpc-execute-task-and-contract-tests` · R-20, R-21 · GW-1,5,6 · – · tanpa stub sukses palsu; test integrasi Go↔Rust · M
- [ ] **PR-36** `test/sandbox-log-scrub-vault` · R-22, R-23, R-24 · GW-1,8 · – · Landlock/Seatbelt/AppContainer, scrub log, vault rotasi · L

**Go — persistensi, identitas, orkestrasi, protokol**
- [ ] **PR-40** `docs/adr-persistence` · G-10 · GW-7 · – · Store per agregat, SQLite/Postgres, migrasi, vektor, enkripsi, backup · S
- [ ] **PR-41** `feat/go-sqlite-store` · G-11 · GW-2,6,9 · PR-40 · migrasi DiskStore idempoten + rollback · L
- [ ] **PR-42** `feat/go-postgres-store` · G-12 · GW-2,6 · PR-40 · test container · L
- [ ] **PR-43** `feat/go-vector-memory-and-retention` · G-13, G-14 · GW-2,6 · PR-41 · RAG per scope; retensi/ekspor/hapus · L
- [ ] **PR-44** `feat/go-identity-access` · G-35 · GW-2,8 · PR-05 · principal resolver, WebAuthn, pairing; deny-by-default; test negatif · L
- [ ] **PR-45** `feat/go-device-pairing-qr-and-auth` · G-20, G-21 · GW-2,8 · PR-44 · token sekali pakai, expiry, revoke, rate-limit; middleware sesi · L
- [ ] **PR-46** `feat/go-system-metrics` · G-22 · GW-2,4 · PR-05 · metrik nyata via Rust (`process_stats`) · M
- [ ] **PR-47** `feat/go-agent-loop` · G-30 · GW-2,6 · PR-35 · tool-call loop, kompaksi, batas iterasi di atas `ExecuteTurn` · L
- [ ] **PR-48** `feat/go-sop-engine` · G-31 · GW-2,6 · PR-47 · SOP + approval gate · L
- [ ] **PR-49** `feat/go-skills-scheduler-handoff` · G-32, G-33 · GW-2,6 · PR-47 · skills, cron, sub-agent/handoff via Artifact · L
- [ ] **PR-50** `feat/go-run-control-approval` · G-34 · GW-2,6 · PR-47 · padanan `control_plane`/`approval`; cek duplikasi `fleet/policies.go` · M
- [ ] **PR-51** `docs/client-protocol-v1` · G-36 (spesifikasi) · GW-5,7 · PR-44 · `api-contracts/client-protocol.md` dari `rpc/types.rs`+`acp.rs` · M
- [ ] **PR-52** `feat/go-client-protocol-server` · G-36 (implementasi), G-27 · GW-2,5,6 · PR-51, PR-47 · WS JSON-RPC 2.0, test kontrak · L
- [ ] **PR-53** `feat/go-org-model` · G-23, G-25 · GW-2,6 · PR-41 · Quartermaster→Ship→Squad→Crew persisten; rute QM two-layer · L
- [ ] **PR-54** `feat/go-policy-audit-readonly` · G-26 · GW-2,4 · PR-53 · policy SSOT & audit diekspos read-only · M

**Rust — migrasi otak & klien**
- [ ] **PR-55** `refactor/rust-caller-auth` · R-39 · GW-1,8 · PR-44 · hanya autentikasi pemanggil Go; hapus resolver/provider/webauthn Rust ⚑ · M
- [ ] **PR-56** `refactor/eval-retarget-go` · R-37 · GW-1,2,6 · PR-47 · replay trace via Go; paritas dengan Rust · M
- [ ] **PR-57** `feat/cli-go-client` · R-34, R-36 · GW-1,2,6 · PR-52 · `galleon` chat/run/approve/status → Go; urusan lokal tetap Rust; audit `galleoncode`/`galleonrelay` · L
- [ ] **PR-58** `refactor/tui-acp-retarget` · R-40, R-42 · GW-1,6 · PR-52, PR-57 · TUI & `galleon-acp-bridge` ke Client Protocol; matikan daemon ephemeral · L
- [ ] **PR-59** `refactor/remove-verifiable-intent` · R-38 · GW-1,7 · PR-17 ⚑ · hapus modul, tool `vi_*`, config; tag arsip dulu (ODR-21) · S
- [ ] **PR-60** `refactor/runtime-shrink-<modul>` (berseri, urutan R-00 §3: calendar/subagent/cost → rag/session/routines/heartbeat → cron → skills → approval/control_plane → sop → agent) · R-16, R-17 · GW-1,6 · modul terkait di Go + paritas eval ⚑ · satu PR per modul · XL
- [ ] **PR-61** `refactor/gateway-decouple-runtime` · R-14 · GW-1,5,6 · PR-60 (cron/observability) ⚑ · 426 ref → 0 · L
- [ ] **PR-62** `refactor/remove-rpc-and-legacy-gateway` · R-41, N-12 · GW-1,5,6 · PR-58, PR-61 ⚑ · hapus `rpc`, REST/WS/A2A lama, kode tak relevan hasil R-00 · L
- [ ] **PR-63** `refactor/split-channels-<kelompok>` (berseri) · R-15 · GW-1 · PR-34 · `galleon-channel-<nama>` per kelompok · XL
- [ ] **PR-64** `feat/channels-tier-b-regional` · R-32 · GW-1,6 · PR-63 · WeChat/QQ/dst. sebagai Tier B (ODR-13), feature flag · L

**UI/Desktop**
- [ ] **PR-70** `refactor/web-remove-express-use-go` · U-02, U-03, U-04 · GW-3,4 · PR-45, PR-46, PR-14 · hapus `server.ts`; QR/telemetri dari Go · M
- [ ] **PR-71** `refactor/web-typed-client-and-contracts` · U-10, U-11 · GW-3,5 · PR-12 · `apiClient.ts` tunggal, tipe E2 · M
- [ ] **PR-72** `feat/web-org-screens` · U-13 · GW-3,6 · PR-53 · Quartermaster/Ship/Squad/Crew, memori, approval · L

### Gelombang 3 — rename & branding (setelah hapus/pecah)

- [ ] **PR-80** `chore/brand-gate-and-rename-script` · N-02 · GW-0 · PR-06 · `scripts/rename/` (dry-run + laporan), `scripts/ci/brand_gate.sh` · M
- [ ] **PR-81** `refactor/rename-env-and-config-paths` · N-23 · GW-1,9 · PR-80 · env ganti langsung (tanpa alias); migrasi data `~/.clawcrew`→`galleon` · L
- [ ] **PR-82** `refactor/rename-crates-<batch>` (berseri, bawah→atas, 3–5 crate/PR) · N-21, N-22 · GW-1,6 · PR-80; `runtime` setelah PR-60 · XL
- [ ] **PR-83** `refactor/rename-binaries` · N-20 · GW-1,9 · PR-82 · `galleon`, `galleoncode`, `galleonrelay`, `galleon-acp-bridge`; tanpa alias · M
- [ ] **PR-84** `refactor/rename-go-module` · N-25 · GW-2 · PR-80 · `github.com/diezy-realm/galleon-fleet/engine`; regen wire/proto · M
- [ ] **PR-85** `refactor/rename-ui-desktop` · N-26, N-27 · GW-3 · PR-12 · `galleon-web`, `galleon-desktop`, productName "Galleon Fleet" · S
- [ ] **PR-86** `feat/wit-v1` · N-24, R-35 · GW-1,5 · PR-82 · `wit/v1` `galleon:plugin@1.0.0`; dual v0/v1; v0 tetap lolos `.frozen` · L
- [ ] **PR-87** `chore/rename-ci-docker-docs` · N-28, N-29, N-30 · GW-0,6 · PR-83 · scripts/xtask/workflows/Dockerfile/README/AGENTS · M
- [ ] **PR-88** `test/brand-gate-final-and-upgrade-smoke` · N-40, N-41 · GW-6 · semua PR-8x · gerbang akhir + smoke upgrade data lama→baru · M

### Gelombang 4 — deploy, rilis, lanjutan

- [ ] **PR-90** `chore/docker-workspace-embed` · P-02, P-03 · GW-0,6 · PR-03, PR-83 · stub Dockerfile & `include` `/web/dist` diputuskan (ADR kecil) · M
- [ ] **PR-91** `feat/nginx-tls-vps` · P-10, P-11 · GW-8,9 · PR-45 · Nginx = reverse proxy + TLS saja (ODR-15); peran `galleonrelay` vs Nginx (ODR-25) · L
- [ ] **PR-92** `feat/desktop-sidecar` · U-09 · GW-3,6 · PR-83 · Tauri menjalankan Go + Rust gateway, lifecycle & health · L
- [ ] **PR-93** `ci/release-pipeline` · P-12, P-08 · GW-0,6,8 · PR-92 · MSI/NSIS, image server, checksum, SBOM; pangkas 39 workflow · L
- [ ] **PR-94** `feat/tier-b-installer` · R-33, P-13 · GW-1,6 · PR-64 · pilih Tier B saat instalasi · M
- [ ] **PR-95** `feat/wasi-tier-c-pilot` · R-31 · GW-1,5,8 · PR-86 · pilot end-to-end; ukur batas `wasi:http` · L
- [ ] **PR-96** `feat/learning-by-consent-and-observability` · G-24, G-28 · GW-2,8 · PR-43, PR-53 · umpan balik → memori terpersetujui, versi/rollback; zap + request-id Go→Rust · L
- [ ] **PR-97** `refactor/rust-heavy-crates-audit` · R-18 · GW-1,7 · PR-60 · audit `tools`/`config`/`providers`; tanpa pemecahan spekulatif · M
- [ ] **PR-98** `feat/web-i18n-a11y` · U-14 · GW-3 · PR-72 · M
- [ ] **PR-99** `docs/adr-delegated-authority-chain` · G-37 · GW-7 · PR-44, PR-53 · ADR opsional; rujukan `research/verifiable-intent-assessment.md` · S
- [ ] **PR-100** `feat/native-tool-contract` · G-29 · GW-1,2,4,8 · PR-05 · SSOT daftar tool + skema argumen di `proto/`, `workspace_path` terisi, test kontrak Go↔Rust; ⚑ · L

## 4. Branch integrasi (GW-6) & checklist Principle

Setelah tiap gelombang: branch `integration/wave-N` dari `main` + PR gelombang itu.
- [ ] `cargo nextest run --workspace` · `go test -race ./...` · `npm run build`
- [ ] smoke e2e (engine + gateway + UI/desktop), termasuk uji negatif fail-closed (Rust mati, token salah, injeksi perintah)
- [ ] laporan dilampirkan ke PR terakhir gelombang → keputusan lanjut/tahan oleh Principle

Checklist reviewer (salin ke tiap PR):
- [ ] Scope sesuai tiket; tanpa refactor/format tak terkait
- [ ] Boundary 4-tier & invarian (LLM SSOT Rust, policy SSOT Go, secrets di Vault) utuh
- [ ] Nol `unwrap/expect` produksi baru; error dipropagasi; log bersih rahasia
- [ ] Test di batas perilaku nyata; negatif keamanan ada
- [ ] Kontrak (`proto/`, `wit/`, API docs) sinkron; WIT v0 tidak berubah
- [ ] Dokumen/ADR diperbarui; rollback tertulis
- [ ] Tanpa kode mati / `allow(dead_code)` baru; dependensi baru dibenarkan
- [ ] PR ⚑: sign-off Owner terlampir
## 5. Matriks cakupan tiket → PR

126 tiket, seluruhnya terpetakan (diverifikasi skrip). Tiket yang dipecah ke beberapa PR terdaftar semuanya.

| Tiket | PR |
|---|---|
| D-01 | PR-00 |
| D-02 | PR-20 |
| D-03 | PR-01 |
| D-04 | PR-21 |
| D-05 | PR-20 |
| D-06 | PR-20 |
| D-07 | PR-21 |
| D-08 | PR-21 |
| D-09 | PR-20 |
| D-10 | PR-21 |
| G-01 | PR-05 |
| G-02 | PR-16 |
| G-03 | PR-05 |
| G-04 | PR-15 |
| G-05 | PR-05 |
| G-10 | PR-40 |
| G-11 | PR-41 |
| G-12 | PR-42 |
| G-13 | PR-43 |
| G-14 | PR-43 |
| G-20 | PR-45 |
| G-21 | PR-45 |
| G-22 | PR-46 |
| G-23 | PR-53 |
| G-24 | PR-96 |
| G-25 | PR-53 |
| G-26 | PR-54 |
| G-27 | PR-52 |
| G-28 | PR-96 |
| G-29 | PR-100 |
| G-38 | PR-05 |
| G-30 | PR-47 |
| G-31 | PR-48 |
| G-32 | PR-49 |
| G-33 | PR-49 |
| G-34 | PR-50 |
| G-35 | PR-44 |
| G-36 | PR-51, PR-52 |
| G-37 | PR-99 |
| N-00 | PR-00 |
| N-01 | PR-06 |
| N-02 | PR-80 |
| N-10 | PR-02 |
| N-11 | PR-17 |
| N-12 | PR-62 |
| N-13 | PR-18 |
| N-14 | PR-18 |
| N-15 | PR-19 |
| N-16 | PR-19 |
| N-17 | PR-19 |
| N-20 | PR-83 |
| N-21 | PR-82 |
| N-22 | PR-82 |
| N-23 | PR-81 |
| N-24 | PR-86 |
| N-25 | PR-84 |
| N-26 | PR-85 |
| N-27 | PR-85 |
| N-28 | PR-87 |
| N-29 | PR-87 |
| N-30 | PR-87 |
| N-40 | PR-88 |
| N-41 | PR-88 |
| P-01 | PR-03 |
| P-02 | PR-90 |
| P-03 | PR-90 |
| P-04 | PR-12 |
| P-05 | PR-12 |
| P-06 | PR-13 |
| P-07 | PR-11 |
| P-08 | PR-93 |
| P-10 | PR-91 |
| P-11 | PR-91 |
| P-12 | PR-93 |
| P-13 | PR-94 |
| R-00 | PR-00 |
| R-01 | PR-10 |
| R-02 | PR-30 |
| R-03 | PR-10 |
| R-04 | PR-10 |
| R-05 | PR-20 |
| R-06 | PR-10 |
| R-10 | PR-31 |
| R-11 | PR-32 |
| R-12 | PR-33 |
| R-13 | PR-34 |
| R-14 | PR-61 |
| R-15 | PR-63 |
| R-16 | PR-60 |
| R-17 | PR-60 |
| R-18 | PR-97 |
| R-20 | PR-35 |
| R-21 | PR-35 |
| R-22 | PR-36 |
| R-23 | PR-36 |
| R-24 | PR-36 |
| R-30 | PR-21 |
| R-31 | PR-95 |
| R-32 | PR-64 |
| R-33 | PR-94 |
| R-34 | PR-57 |
| R-35 | PR-86 |
| R-36 | PR-57 |
| R-37 | PR-56 |
| R-38 | PR-59 |
| R-39 | PR-55 |
| R-40 | PR-58 |
| R-41 | PR-62 |
| R-42 | PR-58 |
| U-01 | PR-03 |
| U-02 | PR-70 |
| U-03 | PR-70 |
| U-04 | PR-70 |
| U-05 | PR-14 |
| U-06 | PR-04 |
| U-07 | PR-04 |
| U-08 | PR-22 |
| U-09 | PR-92 |
| U-10 | PR-71 |
| U-11 | PR-71 |
| U-12 | PR-12 |
| U-13 | PR-72 |
| U-14 | PR-98 |
| U-15 | PR-14 |
| U-16 | PR-04 |
| U-17 | PR-22 |
