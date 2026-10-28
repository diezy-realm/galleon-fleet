# Foundation — Landasan Dasar Repo `galleon-fleet`

> **Tujuan:** pintu masuk untuk memahami *isi repo ini sebenarnya* sebelum membaca ADR/standar lain.
> **Dasar bukti:** observasi langsung working tree branch `feat/refactoring-rust`, 2026-10-04 (struktur direktori, `Cargo.toml`, `engine/go.mod`, `proto/`, git log, CI). Hal yang belum diverifikasi ditandai **[belum diverifikasi]**.
> **Sifat:** *deskriptif* (fakta saat ini). Standar *preskriptif* ada di `tech-standart/`. Bila fakta di sini bertentangan dengan dokumen lain, fakta kode menang dan kontradiksi dicatat di [`04-drift-register-and-hygiene.md`](./04-drift-register-and-hygiene.md).
> **Konfirmasi Owner (2026-10-04):** antarmuka yang dipakai = **`web/` (UI)** dan **`apps/tauri/` (desktop shell)** (dulu `web-2`/`tauri-2`; sudah dipindah, ODR-01).

## Isi

| File | Isi |
|---|---|
| [`01-repository-map.md`](./01-repository-map.md) | Peta folder/file top-level: tujuan, stack, status, tier |
| [`02-lineage-and-active-surfaces.md`](./02-lineage-and-active-surfaces.md) | Permukaan aktif (Galleon) vs warisan ClawCrew; aturan memilih target perubahan |
| [`03-build-test-ci-release.md`](./03-build-test-ci-release.md) | Build/test/lint, lapisan test, CI, rilis, deployment |
| [`04-drift-register-and-hygiene.md`](./04-drift-register-and-hygiene.md) | Ketidaksesuaian dokumen↔kode, pelanggaran boundary aktif, kebersihan repo, backlog |

## Ringkasan satu halaman

1. Repo = fork produk **ClawCrew** (Rust, ±1 juta LOC) yang dibentuk ulang menjadi **Galleon**: platform organisasi AI local-first, 4 tier (Tauri shell → web UI → Go orchestrator → Rust system core).
2. **Antarmuka aktif:** `web/` + `apps/tauri/` (kode Galleon; kode ClawCrew lama sudah diganti). Konfigurasi build/Docker/embed masih mengasumsikan produk lama (02 §2).
3. **Rust workspace** (edition 2024, `rust-version 1.96.0`, v`0.8.5`): root bin `galleon`, `apps/{galleoncode,galleonrelay,tauri}`, ±27 crate, fixture plugin WASI, `xtask`, `tools/fill-translations`. `apps/tauri` kini berisi paket Galleon `fleet-ai-desktop`.
4. **Go engine** (`engine/`, modul terpisah, `go 1.27.1`, Wire): persistensi saat ini **JSON/JSONL di disk**, bukan SQLite/Postgres seperti tertulis di beberapa dokumen.
5. **Kontrak lintas bahasa:** `proto/agent_service.proto` (Go `AgentEngine` :50051; Rust `SystemGateway` :50052: `ExecuteNativeTool`, `GetDecryptedSecret`, `ExecuteTask` [stub], `ExecuteTurn` streaming [nyata]) dan `wit/v0/` (plugin WASI, beku).
6. **Governance dokumen:** `standard-docs/` = otoritas; `docs/` = warisan upstream tanpa indeks otoritas; `docs/_archive/` = historis.
7. **Pagar kualitas ada:** `deny.toml`, `clippy.toml`, `tests/architecture/*`, 39 workflow CI. **Belum ada:** lint `unwrap_used`, gerbang ukuran file, gerbang arah dependency, gerbang boundary tier, dan CI/rilis khusus `web/` dan `apps/tauri`.

## Cara pakai oleh peran

| Peran | Mulai dari |
|---|---|
| Engineer/agent baru | 01 → 02 → `tech-standart/architecture/00-overview.md` |
| Pemecah modul Rust | 01 §2 → `tech-standart/code-quality/` → `decisions/refactoring-rust/phase-5-production-hardening/` |
| Reviewer (Principle) | 04 §2 → `tech-standart/architecture/01-boundary-policy.md` |
| DevOps/Release | 03 |
