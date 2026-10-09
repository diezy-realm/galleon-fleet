---
title: T-CI-03 — Gate D-2 regex terlalu generik, menangkap cfg-gate biasa
status: open
severity: MEDIUM (menengah — belum 100% dipastikan false-positive, perlu verifikasi dulu)
---

# T-CI-03 — Verifikasi & perbaiki regex Gate D-2 (Governance Coupling)

- [ ] **T-CI-03.** Verifikasi apakah 5 match `feature = "channel-` di `crates/clawcrew-channels/src/util.rs` genuinely governance-coupling, atau cfg-gate biasa yang salah tertangkap. Perbaiki regex jika genuinely false-positive.

**File:** `.github/workflows/principle-review-trigger.yml`, step `Analyze PR Diff & Invariants`, bagian `# 3. Gate D-2 Check (Governance coupling in util.rs)`.

**Masalah konkret (diverifikasi sebagian 2026-10-09 di PR #18):**
```bash
COUPLING=$(grep -n 'feature = "channel-' crates/clawcrew-channels/src/util.rs || true)
```
Regex ini match SETIAP baris yang punya teks `feature = "channel-`, termasuk conditional-compile biasa seperti:
```rust
#[cfg(any(feature = "channel-slack", feature = "channel-telegram"))]
#[cfg(any(feature = "channel-mattermost", feature = "channel-qq"))]
```
Ditemukan 59 match total di file ini (5 di antaranya genuinely ter-flag Gate D-2 di PR #18) — **belum diverifikasi** apakah match yang di-flag itu genuinely menunjukkan governance-primitive (RACI/SOP/approval logic) yang dicampur dengan feature-flag per-channel (pelanggaran arsitektur yang genuinely ingin dicegah gate ini), atau murni cfg-gate fungsi utilitas biasa yang kebetulan nama fungsinya mengandung governance-adjacent term.

**Langkah:**
1. Baca isi fungsi tepat di baris yang ter-flag Gate D-2 di run PR #18 (`crates/clawcrew-channels/src/util.rs`, cek 5 baris match yang genuinely menyebabkan `D2_VIOLATION=true` — ambil dari log run `37909598662` atau re-run grep lokal untuk daftar lengkap).
2. Untuk setiap match: apakah fungsi tersebut mengandung logic governance (approval gate, RACI check, SOP constraint) ATAU murni dispatch/routing per-channel (connect, parse, format)?
3. Jika SEMUA match genuinely cfg-gate biasa (dispatch/parsing) — regex gate ini false-positive dan perlu diperbaiki: ganti dari deteksi "ada teks `feature = \"channel-`" ke deteksi yang lebih spesifik (misal: cari `feature = "channel-` yang satu fungsi dengan panggilan ke modul governance tertentu, atau exclude fungsi yang namanya cocok pola dispatch/parse/route/format).
4. Jika SEBAGIAN match genuinely governance-coupling — regex-nya tetap perlu diperbaiki supaya presisi (hanya flag yang genuine), tapi temuan governance-coupling yang genuine itu jadi task TERPISAH untuk diperbaiki di kode (bukan di gate).

**DoD:** daftar 5 match asli diklasifikasi genuine-violation vs false-positive dengan bukti baris kode; jika ada false-positive, regex diperbaiki dan diverifikasi tidak lagi menangkap cfg-gate biasa pada PR test; jika ada genuine-violation, dicatat sebagai task kode terpisah (bukan ditutup di sini).

**Jangan diubah:** jangan hapus Gate D-2 sepenuhnya — tujuannya (mencegah governance primitive tercampur feature-flag channel) tetap valid, hanya presisi deteksinya yang perlu diperbaiki.
