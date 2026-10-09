# CI/Infra Standards & Quality Gate Steering

> **Status:** Canonical Workspace Steering & CI Governance (Normative)  
> **Source of Truth:** [`standard-docs/decisions/finalize/dev-task-2/ci-infra-fix/`](../../standard-docs/decisions/finalize/dev-task-2/ci-infra-fix/)  
> **Normative Standards:** [`standard-docs/tech-standart/architecture/01-boundary-policy.md`](../../standard-docs/tech-standart/architecture/01-boundary-policy.md), [`standard-docs/decisions/refactoring-rust/RF-D-principle-gateway-checklist.md`](../../standard-docs/decisions/refactoring-rust/RF-D-principle-gateway-checklist.md), `AGENTS.md`  
> **Applies to:** Semua Developer, AI Agents, Pull Request Authors, dan CI Workflows (`.github/workflows/`).

---

## 1. Latar Belakang & Prinsip Utama

Berdasarkan temuan konkret pada validasi PR #18 (dev-task-2 remediation batch), ditemukan 4 kegagalan CI yang 100% merupakan isu *tooling/infra/governance*, bukan cacat logika pada kode fitur:
1. **False-Positive Zero-Unwrap Gate:** Regex diff CI menandai `unwrap()` di dalam modul test inline sebagai pelanggaran produksi.
2. **Compiler Wrapper Runner Crash:** Job desktop clippy gagal di seluruh OS (Ubuntu, Windows, macOS) karena `sccache` diaktifkan di `.cargo/config.toml` namun binary belum terpasang di runner.
3. **Overbroad Gate D-2 Detection:** Regex Gate D-2 mendeteksi baris conditional compilation biasa (`#[cfg(feature = "channel-*")]`) di `util.rs` sebagai pelanggaran coupling governance.
4. **Web Peer Dependency Conflict (`ERESOLVE`):** Ketidaksesuaian versi pin `esbuild` vs `vite@8.3.1` di `web/package.json` yang memblokir `npm install`.

Dokumen ini menetapkan aturan keras (*hard rules*) dan pedoman operasional (*operational guidelines*) agar masalah infrastruktur dan gate ini tidak berulang pada pengembangan selanjutnya.

---

## 2. Pilar 1: Zero-Unwrap Invariant vs Test Code Scoping

### Aturan untuk Penulis Kode (Code Authors & Agents)
- **Zero `unwrap()` / `expect()` di Jalur Produksi:** Seluruh kode produksi Rust WAJIB menggunakan error propagation `?` dengan `Result<T, E>`. Pengecualian hanya untuk invarian mutlak yang didokumentasikan eksplisit dengan komentar:
  ```rust
  // INVARIANT: [penjelasan mengapa panic tidak mungkin terjadi secara matematis/logis]
  ```
- **Scoping Eksklusif untuk Test:** Pemanggilan `.unwrap()` dan `.expect()` HANYA dilegalkan di dalam:
  1. Modul inline test: `#[cfg(test)] mod tests { ... }`.
  2. File test mandiri: `tests.rs` (termasuk `src/**/tests.rs`).
  3. Direktori test integrasi: `tests/` atau benchmark: `benches/`.
- **Dilarang Menempatkan Test Helper di Scope Produksi:** Seluruh helper mock, test fixture, atau builder yang menggunakan `unwrap()` HARUS berada di dalam scope `#[cfg(test)]`. Jangan sekali-kali mengekspos helper yang mengandung `unwrap()` di luar flag `#[cfg(test)]`.

### Aturan untuk CI & Principle Gate Scripts
- **Presisi Diff Filter:** Script audit diff di CI (`.github/workflows/principle-review-trigger.yml`) WAJIB membedakan secara tegas antara baris kode produksi dan test.
- **Path Exclusion Lengkap:** Filter path `git diff` tidak boleh hanya mengecualikan direktori `tests/` (`':!crates/**/tests/**'`), melainkan WAJIB mengecualikan file `tests.rs` di subfolder mana pun:
  ```bash
  ':!crates/**/tests/**' ':!crates/**/benches/**' ':!crates/**/tests.rs' ':!crates/**/*/tests.rs'
  ```
- **Hunk Context Awareness:** Pemeriksaan `unwrap()`/`expect()` terhadap file produksi tidak boleh berbasis naive single-line grep diff tanpa konteks. Script gate harus mengabaikan hunk yang berada di bawah deklarasi `#[cfg(test)] mod tests`. Jika gate gagal membedakan konteks AST, adopsi lint clippy berbasis AST (`#![warn(clippy::unwrap_used)]` dengan pengecualian `#[cfg(test)]`).

---

## 3. Pilar 2: CI Runner Environment & Compiler Toolchain Wrapper Isolation

### Aturan Workspace & Compiler Wrapper (`sccache`)
- **Single Source of Configuration:** `.cargo/config.toml` mengonfigurasi `rustc-wrapper = "sccache"` untuk akselerasi cache kompilasi lokal dan CI.
- **Larangan Penghapusan Arbitrer:** Dilarang menghapus konfigurasi `rustc-wrapper = "sccache"` dari `.cargo/config.toml` hanya untuk membuat CI yang rusak "berjalan cepat". Menghapus sccache memperlambat seluruh inner loop developer dan runtime CI jangka panjang.

### Aturan untuk GitHub Actions Workflows
- **Penyelarasan Toolchain & Wrapper:** Setiap job CI (di Ubuntu, Windows, maupun macOS) yang mengeksekusi perintah Cargo (`cargo clippy`, `cargo test`, `cargo check`) WAJIB memastikan binary `sccache` tersedia di `PATH` SEBELUM perintah Cargo dijalankan.
- **Urutan Langkah Wajib di Workflow CI:**
  ```yaml
  # 1. Pasang rust toolchain
  - uses: dtolnay/rust-toolchain@...
  
  # 2. Pasang sccache binary ke PATH runner
  - name: Install sccache
    uses: mozilla-actions/sccache-action@v0.0.8

  # 3. Konfigurasi cache & env
  - name: Configure sccache environment
    run: |
      echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV
      echo "SCCACHE_GHA_ENABLED=true" >> $GITHUB_ENV
  ```
- **Fallback Graceful:** Jika sebuah job CI sengaja tidak menggunakan `sccache` (misalnya job audit ringan atau platform eksperimental), env var WAJIB di-override eksplisit pada level job/step:
  ```yaml
  env:
    RUSTC_WRAPPER: ""
  ```
  Hal ini mencegah Cargo gagal dengan error fatal `could not execute process 'sccache ...' : No such file or directory (os error 2)`.

---

## 4. Pilar 3: Feature-Flag Transport vs Governance Coupling (Gate D-2)

> **Koreksi 2026-10-09:** Versi sebelumnya dari bagian ini menyatakan "`util.rs` tidak mengandung primitif approval sama sekali" — klaim itu SALAH, hasil sampling 2 fungsi dari 59 occurrence `feature = "channel-"` di file tersebut tanpa membaca sisanya (pola audit-sampel-sempit yang sama dengan insiden T-06/MCP). Faktanya `util.rs` **genuinely berisi** `PendingApproval`, `resolve_pending_approval`, `new_approval_token`, `build_yesno_approval_prompt` — semua di-gate dengan `#[cfg(any(feature = "channel-discord", feature = "channel-slack", ...))]`. Investigasi lanjutan (lihat di bawah) menyimpulkan ini BUKAN governance-coupling yang dilarang, tapi dengan alasan yang berbeda dari klaim lama.

### Aturan Batas Arsitektur (Boundary Policy)
- **Definisi Utilitas Transport Channel:** File `crates/clawcrew-channels/src/util.rs` bertindak sebagai utilitas teknis transport (HTTP stream limiting, pemotongan string UTF-8, formatting pesan, i18n breadcrumbs, **dan prompt/parser approval berbasis teks per-channel**).
- **Legitimasi Conditional Compilation:** Penggunaan `#[cfg(any(feature = "channel-slack", ...))]` pada fungsi utilitas murni (seperti `read_response_body_limited` atau `localized_lifecycle_progress`) adalah **SAH dan DIHARUSKAN** untuk mencegah peringatan `unused code` saat channel tertentu dinonaktifkan.
- **Klarifikasi batas governance vs transport (bukan larangan mutlak atas nama simbol):** `PendingApproval`/`resolve_pending_approval`/`new_approval_token`/`build_yesno_approval_prompt` di `util.rs` **memang ada dan SAH** — fungsi-fungsi itu murni I/O layer: generate token 6-karakter, menyimpan `oneshot::Sender` menunggu balasan teks user di platform channel (contoh reply `"abc123 yes"`), lalu meneruskan hasilnya. Dibuktikan: `clawcrew-channels/Cargo.toml` hanya depend ke `clawcrew-api` (interface/trait leaf), **tidak** depend ke `clawcrew-runtime` tempat logic kebijakan sungguhan hidup (`clawcrew-runtime/src/approval/mod.rs`, `clawcrew-runtime/src/security/playbook.rs`). Channel tidak pernah memutuskan APA yang perlu approval atau risk-tier-nya — itu diputuskan di tempat lain; channel hanya tahu CARA menampilkan prompt & mem-parse balasan untuk platform tertentu.
  - **Yang TETAP dilarang keras:** memindahkan logika KEPUTUSAN kebijakan ke `util.rs` — threshold risk-tier, RACI matrix, kriteria auto-approve, atau pemanggilan langsung ke `clawcrew-runtime`/Go policy engine dari dalam `clawcrew-channels`. Itu baru genuinely governance-coupling.

### Aturan untuk Principle Gate Reviewer (Gate D-2)
- **Kepatuhan terhadap Dokumen Kanonikal:** Verifikasi Gate D-2 wajib mengacu pada kontrak asli di [`standard-docs/decisions/refactoring-rust/RF-D-principle-gateway-checklist.md`](../../standard-docs/decisions/refactoring-rust/RF-D-principle-gateway-checklist.md) baris 25-28 — kontrak itu sendiri sudah benar; yang salah adalah penerapannya di dokumen asesmen sebelumnya.
  - Helper teknis generik (`read_response_body_limited`, HTTP parser, formatting) yang menggunakan `feature = "channel-*"` adalah **AMAN / LOLOS**.
  - Fungsi approval **I/O/transport** (`PendingApproval`, `resolve_pending_approval`, `new_approval_token`, prompt builder) yang hanya menyampaikan/mem-parse teks dan TIDAK memanggil `clawcrew-runtime`/policy engine juga **AMAN / LOLOS** — verifikasi dengan cek dependency graph crate (`cargo tree -p clawcrew-channels -e normal` tidak boleh menunjukkan `clawcrew-runtime`), bukan sekadar nama simbol.
  - Hanya jika ditemukan fungsi yang genuinely mengandung keputusan kebijakan (threshold risk-tier, RACI, auto-approve criteria) atau dependency langsung ke `clawcrew-runtime`/Go policy barulah gate memberikan vonis **REJECT**.
- **Larangan Grep Teks Buta — ke DUA arah:** Script otomatisasi CI dilarang menandai kegagalan HANYA karena string `feature = "channel-` ditemukan (false-positive lama), TAPI juga dilarang menyimpulkan "aman" hanya dari sampel sebagian kecil occurrence tanpa membaca semua baris yang match (false-negative yang baru dikoreksi di sini). Deteksi otomatis harus memfilter keberadaan simbol tata kelola sungguhan (`grep -E '(RiskTier|should_auto_approve|RACI|GovernancePolicy)'` dan cross-check dependency graph), bukan nama simbol approval I/O semata.

---

## 5. Pilar 4: Web Dependency Peer Governance & Strict Lockfile Management

### Aturan Versi & Ketergantungan Frontend (`web/`)
- **Penyelarasan Peer Dependency Matrix:** Ketergantungan bundler dan transpiler di `web/package.json` wajib selaras tanpa konflik peer dependencies:
  - Versi `vite` (misal `^8.3.0` / `8.3.1`) yang meminta `peerOptional esbuild@"^0.27.0 || ^0.28.0"` HARUS dipasangkan dengan versi `esbuild` yang kompatibel (misal `^0.28.0` atau `^0.27.0`), BUKAN versi lama yang tertinggal (`^0.25.0`).
- **Larangan Keras Bypass Flag:** DILARANG menggunakan `--force` atau `--legacy-peer-deps` baik di skrip lokal, package scripts, maupun workflow CI (`.github/workflows/web-check.yml`). Bypass flags menyembunyikan inkonsistensi AST toolchain yang berpotensi meloloskan bundle build yang crash saat runtime.
- **Verifikasi Instalasi Bersih:** Setiap perubahan pada `web/package.json` WAJIB diverifikasi dengan instalasi bersih di lokal:
  ```bash
  cd web
  npm install
  npm run typecheck
  npm run lint
  npm run build
  ```
- **Atomic Lockfile Commit:** Perubahan `web/package.json` HARUS selalu di-commit bersamaan dengan `web/package-lock.json` yang terbarui secara sinkron dalam commit yang sama.

---

## 6. Checklist Verifikasi Mandiri untuk Developer & Agent Sebelum PR

Sebelum mengajukan Pull Request yang menyentuh Rust, CI workflows, atau Frontend Web, pastikan checklist berikut terpenuhi:

- [ ] **Rust Production Zero-Unwrap:** Tidak ada pemanggilan `.unwrap()` atau `.expect()` di luar blok `#[cfg(test)]`, file `tests.rs`, atau folder `tests/`.
- [ ] **Runner Wrapper Compatibility:** Jika mengedit workflow yang menjalankan `cargo`, pastikan `sccache` diinstal sebelum perintah dijalankan, atau nonaktifkan dengan `RUSTC_WRAPPER=""`.
- [ ] **Channel Boundary Check:** Modifikasi pada `crates/clawcrew-channels/` tidak mencampur logika approval/governance ke dalam utilitas transport.
- [ ] **Web Clean Install:** Direktori `web/` dapat menjalankan `npm ci` atau `npm install` tanpa pesan error `ERESOLVE` dan tanpa flag `--force`/`--legacy-peer-deps`.
- [ ] **Lockfile Sync:** `Cargo.lock` dan `web/package-lock.json` selalu sinkron dengan manifest masing-masing.
