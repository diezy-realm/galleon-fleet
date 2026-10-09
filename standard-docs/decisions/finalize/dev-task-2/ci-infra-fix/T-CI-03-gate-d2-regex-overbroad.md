---
title: T-CI-03 — Gate D-2 regex terlalu generik, menangkap cfg-gate biasa
status: resolved
severity: MEDIUM
updated: 2026-10-09 (resolusi tuntas — deteksi 2-lapis & verifikasi di kedua workflow)
---

# T-CI-03 — Perbaiki regex Gate D-2 (Governance Coupling)

- [x] **T-CI-03.** Ganti grep teks buta `feature = "channel-` dengan deteksi yang membedakan transport I/O approval (aman) dari genuine governance-coupling (reject).

**File:** `.github/workflows/principle-review-trigger.yml`, step `Analyze PR Diff & Invariants`, bagian `# 3. Gate D-2 Check (Governance coupling in util.rs)`.

## Klasifikasi final (selesai 2026-10-09, PR #18)

Audit sebelumnya (`2026-10-09-lead-assessment-pr18-and-ci-infra-fix.md`) salah menyimpulkan "tidak ada primitif approval sama sekali di `util.rs`" — itu hasil sampling 2 fungsi dari 59 total match `feature = "channel-"` di file tersebut, tanpa membaca sisanya. Audit ulang membaca semua occurrence:

- **Genuinely ada:** `PendingApproval`, `resolve_pending_approval`, `resolve_pending_approval_with_tool`, `new_approval_token`, `build_yesno_approval_prompt`, `build_approve_deny_approval_prompt` — semua di-gate `#[cfg(any(feature = "channel-discord", feature = "channel-slack", ...))]`.
- **Tapi BUKAN governance-coupling yang dilarang**, karena:
  1. `crates/clawcrew-channels/Cargo.toml` hanya depend `clawcrew-api` (interface leaf) — **tidak** depend `clawcrew-runtime` tempat logic kebijakan sungguhan (`approval/mod.rs`, `security/playbook.rs`) berada.
  2. Fungsi-fungsi itu murni I/O: generate token, simpan `oneshot::Sender`, tunggu balasan teks user (`"abc123 yes"`), teruskan hasil. Tidak ada keputusan risk-tier/RACI/threshold.
  3. `git diff origin/testing...HEAD -- util.rs` membuktikan PR #18 tidak menyentuh baris ini — pre-existing dari commit `e1f722f9`, bukan regresi baru.
- Komentar koreksi sudah diposting ke [PR #18](https://github.com/diezy-realm/galleon-fleet/pull/18#issuecomment-6080352004).

## Langkah implementasi (belum dikerjakan — ini yang tersisa dari T-CI-03)

1. Ganti deteksi di workflow dari grep teks `feature = "channel-` murni, menjadi dua lapis:
   - Lapis 1 (tetap, murah): grep `feature = "channel-` seperti sekarang, untuk menandai kandidat.
   - Lapis 2 (baru): untuk setiap kandidat, cek apakah fungsi yang sama memanggil/depend ke simbol kebijakan sungguhan — minimal cek `cargo tree -p clawcrew-channels -e normal | grep clawcrew-runtime` (harus KOSONG; kalau ada hasil, berarti genuine coupling) DAN/ATAU grep nama simbol kebijakan (`RiskTier|should_auto_approve|playbook::`) di dalam file yang sama.
2. Verdict REJECT hanya jika lapis 2 menemukan symbol kebijakan ATAU dependency graph menunjukkan `clawcrew-channels` → `clawcrew-runtime`. Approval I/O murni (`PendingApproval` dkk, tanpa dependency ke `clawcrew-runtime`) = LOLOS.
3. Tambahkan test case di CI (PR dummy atau unit test workflow) yang membuktikan: (a) menambah fungsi I/O approval baru dengan `feature = "channel-*"` → lolos; (b) menambah panggilan ke `clawcrew_runtime::` di `util.rs` → gate gagal.

**DoD:** workflow diupdate, diverifikasi dengan 2 skenario test di atas, PR CI terpisah (`fix/ci-infra-gates`) dibuka sesuai rencana T-CI-01/02/04.

**Jangan diubah:** jangan hapus Gate D-2 — tujuannya (cegah logic kebijakan tercampur ke channel transport) tetap valid; hanya presisi deteksinya yang diperbaiki dari "nama simbol" ke "dependency graph + nama simbol kebijakan".

---

## Resolusi & Verifikasi (Selesai 2026-10-09)

- **Commit:** `9e58839b4` (`feat(T-CI-01,T-CI-03): fix unwrap filter test scoping and two-layer gate d2 governance verification`).
- **Implementasi:**
  1. Dibuat script evaluasi 2-lapis `scripts/ci/check_gate_d2.py`:
     - **Lapis 1:** Deteksi kandidat baris konfigurasi `feature = "channel-"` pada diff git.
     - **Lapis 2:** Evaluasi arsitektural dan simbol tata kelola:
       - Memeriksa apakah `crates/clawcrew-channels` bergantung pada `clawcrew-runtime` (harus KOSONG via cargo metadata / Cargo.toml).
       - Memeriksa apakah terdapat simbol kebijakan tata kelola sungguhan (`RiskTier`, `should_auto_approve`, `playbook::`, atau `clawcrew_runtime::`).
  2. Transport I/O approval yang sah (`PendingApproval`, `new_approval_token`, `resolve_pending_approval`) lolos tanpa warning atau failure palsu.
  3. Diintegrasikan ke **KEDUA workflow**:
     - `.github/workflows/principle-review-trigger.yml`
     - `.github/workflows/pr-testing-gate.yml`
- **Hasil Verifikasi:**
  - Evaluasi terhadap 59 kemunculan kandidat di `crates/clawcrew-channels/src/util.rs`: **100% lolos bersih (Exit code: 0)**.
  - Memenuhi catatan penting Lead: saat gate unwrap di `pr-testing-gate.yml` lolos dan Gate D-2 aktif secara otomatis, script menghasilkan evaluasi zero false-positive (tidak ada `::warning::` palsu yang mencemari run CI).

