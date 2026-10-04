# PR Plan & Branch-Gateway DoD (untuk koordinasi dengan Principle)

> Setiap PR di bawah memetakan tiket roadmap (`R-`, `G-`, `U-`, `P-`, `D-`, `N-`) ke satu branch dan satu review Principle.
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

## 2. Daftar PR (urutan gelombang)

Format: `[ ] PR-xx  branch · tiket · gerbang · Dep · fokus review Principle`

### Gelombang 0 — keputusan & P0

- [ ] **PR-01** `docs/naming-map-signoff` · N-00, D-03 · GW-0,7 · – · Principle cek konsistensi pengecualian (ADR immutable, WIT v0 beku)
- [ ] **PR-02** `chore/remove-engine-binary` · N-10 · GW-0 · – · `.gitignore`, tidak force-push
- [ ] **PR-03** `fix/tauri-frontend-dist` · U-01, P-01 · GW-0,3 · – · `frontendDist=../../web/dist`; `cargo check -p fleet-ai-desktop`; workspace hijau
- [ ] **PR-04** `security/remove-tauri-terminal-and-fake-metrics` · U-06, U-07 · GW-3,4,8 · (G-04 untuk pengganti; sementara UI nonaktif) · tidak ada eksekusi shell di Tauri; tidak ada data palsu
- [ ] **PR-05** `docs/runtime-inventory` · R-00 (draf: `R-00-runtime-inventory.md`) · GW-0,7 · – · **tabel keputusan per modul + pemanggil**; belum ada penghapusan kode
- [ ] **PR-06** `security/go-fail-closed-tools` · G-01, G-03, G-05 · GW-2,4,8 · – · tidak ada `os/exec`/fs lokal; test fail-closed
- [ ] **PR-07** `docs/inventory-brand-hits` · N-01 · GW-0 · PR-01 · klasifikasi rename/delete/keep/migrate

### Gelombang 1 — gerbang kualitas & pembersihan

- [ ] **PR-10** `ci/rust-workspace-lints` · R-01, R-03, R-04, R-06 · GW-1 · – · `[workspace.lints]`, gerbang ukuran, gerbang arah dependensi
- [ ] **PR-11** `ci/boundary-gates` · P-07 · GW-4,6 · PR-06 · `tests/architecture/`
- [ ] **PR-12** `ci/web-and-desktop` · P-04, P-05, U-12 · GW-3 · PR-03 · CI web, desktop (`fleet-ai-desktop`), skrip test/lint
- [ ] **PR-13** `ci/engine` · P-06 · GW-2 · – 
- [ ] **PR-14** `refactor/web-remove-express-aistudio` · U-02, U-04, U-05 · GW-3,4 · G-20/G-22 tersedia (atau fitur dinonaktifkan sementara) · hapus `server.ts`, `@google/genai`, `better-sqlite3`, artefak AI Studio
- [ ] **PR-15** `feat/go-terminal-policy-endpoint` · G-04 · GW-2,4,8 · PR-06 · risk-tier + approval + audit
- [ ] **PR-16** `chore/legacy-removal-docs-archive` · N-11 · GW-0 · PR-01
- [ ] **PR-17** `chore/legacy-removal-skills-workflows` · N-13, N-14, N-15, N-16, N-17 · GW-0 · PR-07
- [ ] **PR-18** `docs/drift-sync` · D-02, D-04, D-05, D-06, D-07, D-08, D-09, D-10, R-05, R-30 · GW-7 · –
- [ ] **PR-19** `feat/tauri-csp-capabilities` · U-08 · GW-3,8 · PR-03

### Gelombang 2 — pemecahan Rust, persistensi, pairing

- [ ] **PR-20** `refactor/runtime-extract-i18n` · R-10 · GW-1 · PR-05, PR-10
- [ ] **PR-21** `refactor/runtime-extract-scrub` · R-11 · GW-1,8 · PR-05
- [ ] **PR-22** `refactor/api-traits-for-channels` · R-12 · GW-1,5 · PR-05
- [ ] **PR-23** `refactor/channels-decouple-runtime` · R-13 · GW-1,6 · PR-20..22
- [ ] **PR-24** `refactor/gateway-decouple-runtime-remove-rest` · R-14, N-12 (bagian gateway) · GW-1,5,6 · PR-05, sign-off Owner
- [ ] **PR-25** `refactor/runtime-shrink-<modul>` (1 PR per modul, urutan di R-00 §3) · R-16, R-17 · GW-1,6 · PR-05, **sign-off Owner per modul**
- [ ] **PR-26** `refactor/split-channels` · R-15, R-32 · GW-1 · PR-23 (satu PR per kelompok channel)
- [ ] **PR-26b** `refactor/remove-verifiable-intent` · R-38 · GW-1,7 · PR-05 + sign-off
- [ ] **PR-26c** `feat/go-identity-access` · G-35 · GW-2,8 · – · principal/WebAuthn/pairing, test negatif
- [ ] **PR-26d** `docs/client-protocol-v1` + `feat/go-client-protocol` · G-36 · GW-2,5,6 · PR-26c
- [ ] **PR-26e** `refactor/rust-caller-auth` · R-39 · GW-1,8 · PR-26c
- [ ] **PR-26f** `refactor/tui-acp-retarget` · R-40 · GW-1,6 · PR-26d
- [ ] **PR-26g** `refactor/remove-rpc-legacy-gateway` · R-41 · GW-1,5,6 · PR-26f
- [ ] **PR-27** `fix/grpc-execute-task` · R-20, R-21, R-22, R-23, R-24 · GW-1,5,6,8 · –
- [ ] **PR-28** `docs/adr-persistence` · G-10 · GW-7 · –
- [ ] **PR-29** `feat/go-sqlite-store` · G-11, G-13, G-14 · GW-2,6,9 · PR-28 (migrasi data + rollback)
- [ ] **PR-30** `feat/go-postgres-store` · G-12 · GW-2,6 · PR-28
- [ ] **PR-31** `feat/go-device-pairing-qr` · G-20, G-21 · GW-2,8 · – · token sekali pakai, expiry, revoke, rate-limit
- [ ] **PR-32** `feat/go-system-metrics` · G-22 · GW-2,4 · –
- [ ] **PR-33** `feat/go-orchestrator-absorb-*` · G-30..G-33 · GW-2,6 · PR-05 (satu PR per fungsi: agent loop, SOP, skills/cron, handoff)
- [ ] **PR-34** `feat/cli-go-client` · R-34, R-36, G-27 · GW-1,2,6 · PR-05, PR-33 · `galleon`/`galleoncode` → Go :9090
- [ ] **PR-35** `feat/web-qr-pairing-ui` · U-03, U-10, U-11 · GW-3 · PR-31
- [ ] **PR-36** `feat/org-model` · G-23, G-25, G-26, U-13 · GW-2,3,6 · PR-29

### Gelombang 3 — rename (setelah hapus/pecah)

- [ ] **PR-40** `chore/brand-gate-and-rename-script` · N-02 · GW-0 · PR-07
- [ ] **PR-41** `refactor/rename-env-and-config-paths` · N-23 · GW-1,9 · PR-40 · migrasi data `~/.clawcrew`→`galleon`; env diganti langsung (ODR-19)
- [ ] **PR-42** `refactor/rename-crates-<batch>` · N-21, N-22 · GW-1,6 · PR-40; **bawah→atas, tiap batch 3–5 crate**; `runtime` setelah PR-25
- [ ] **PR-43** `refactor/rename-binaries` · N-20 (`galleon`, `galleoncode`, `galleonrelay`, `galleon-acp-bridge`) · GW-1,9 · PR-42
- [ ] **PR-44** `refactor/rename-go-module` · N-25 · GW-2 · PR-40
- [ ] **PR-45** `refactor/rename-ui-desktop` · N-26, N-27 · GW-3 · PR-12
- [ ] **PR-46** `feat/wit-v1` · N-24, R-35, R-30 · GW-1,5 · PR-42 · v0 beku tetap lolos
- [ ] **PR-47** `chore/rename-ci-docker-scripts` · N-28, N-29, N-30 · GW-0,6 · PR-43
- [ ] **PR-48** `test/brand-gate-final-and-upgrade-smoke` · N-40, N-41 · GW-6 · semua PR-4x

### Gelombang 4 — deploy & rilis

- [ ] **PR-50** `chore/docker-workspace-embed` · P-02, P-03 · GW-0,6 · PR-03, PR-43
- [ ] **PR-51** `feat/nginx-tls-vps` · P-10, P-11 · GW-8,9 · PR-31 · Nginx = reverse proxy + TLS (ODR-15)
- [ ] **PR-52** `feat/desktop-sidecar` · U-09 · GW-3,6 · PR-43
- [ ] **PR-53** `ci/release-pipeline` · P-12, P-08 · GW-0,6,8 · PR-52
- [ ] **PR-54** `feat/tier-b-installer` · R-33, P-13 · GW-1,6 · PR-26
- [ ] **PR-55** `feat/wasi-tier-c-pilot` · R-31 · GW-1,5,8 · PR-46
- [ ] **PR-56** `feat/learning-by-consent` · G-24, G-28, R-18 · GW-2,8 · PR-29

## 3. Branch integrasi (GW-6) per gelombang

Setelah tiap gelombang selesai, branch `integration/wave-N` dibuat dari `main` + semua PR gelombang itu, lalu dijalankan:

- [ ] `cargo nextest run --workspace` · `go test -race ./...` · `npm run build`
- [ ] smoke e2e (engine + gateway + UI/desktop), termasuk uji negatif fail-closed
- [ ] laporan hasil dilampirkan ke PR terakhir gelombang → keputusan lanjut/tahan oleh Principle

## 4. Checklist reviewer Principle (salin ke tiap PR)

- [ ] Scope sesuai tiket; tidak ada refactor/format tak terkait
- [ ] Boundary 4-tier & invarian (LLM SSOT Rust, policy SSOT Go, secrets di Vault) utuh
- [ ] Nol `unwrap/expect` produksi baru; error dipropagasi; log bersih rahasia
- [ ] Test di batas perilaku nyata (bukan hanya happy path); negatif keamanan ada
- [ ] Kontrak (`proto/`, `wit/`, API docs) sinkron; WIT v0 tidak berubah
- [ ] Dokumen/ADR diperbarui; rollback tertulis
- [ ] Tidak ada kode mati / `allow(dead_code)` baru; dependensi baru dibenarkan
