# Archive — Dokumen Usang / Selesai

Folder ini berisi dokumen `standard-docs/` yang sudah **usang** (digantikan dokumen lain secara eksplisit) atau **selesai sepenuhnya** (checklist 100% `[x]` tanpa catatan revisi/overclaim, atau kesimpulan definitif tanpa pekerjaan lanjutan). Dipisahkan 2026-10-09 agar dokumen kerja aktif tidak campur dengan arsip historis.

**Prinsip pemindahan:** hanya dipindah jika ada bukti tertulis eksplisit (status `superseded`/`done`/`Final`, atau audit independen yang mengonfirmasi tuntas tanpa sisa) — bukan karena tanggal file lama. Isi dokumen TIDAK diubah substansinya, hanya lokasi.

## Sub-folder

| Folder | Isi | Kenapa diarsip |
|---|---|---|
| `decisions-finalize-legacy/` | 5 file top-level `decisions/finalize/` (README, analysis-conclusion, task-breakdown, testing-gate-checklist, route-map) | Dokumen-dokumen ini eksplisit menyatakan kerja aktif sudah pindah ke `decisions/finalize/dev-tasks/`; isi historis Grup 0-8 sudah `[x]` dengan bukti konkret. |
| `dev-task-2-standalone-docs/` | 8 dokumen evaluasi/analisis berdiri sendiri dari `dev-task-2/` (steering-spec, safe-outputs-spec, hygiene-evaluation, mcp-sandbox-gap, cost-cap, 2 audit, approval-mode-design) + `fixing-task.md` (eks `decisions/bug-fix/`) | Semua berstatus "Final/Evaluated/Approved/Verified" dengan kesimpulan definitif — bukan checklist kerja, murni output analisis yang sudah tuntas. |
| `research-closed-loop/` | 5 file dari `research/` (research-gap-map, product-industry-competitor-insight, verifiable-intent-assessment, perplexity-web-browsing-methodology, fleet-wide-mcp-governance-followup) | Masing-masing eksplisit "closed-loop" — ditindaklanjuti jadi dokumen lain, atau keputusan Owner sudah final menutup pertanyaannya. |
| `product-research-superseded/` | `tier-enforcement-method-d-c-deep-dive.md` | Status `superseded`, digantikan `product-research/tier-pricing-and-entitlement-final.md`. |
| `superseded-specs/` | `prd-hybrid-engine.md` (eks `product/`), `go-engine-http-grpc-api-spec.md` (eks `tech-standart/api-contracts/`) | Keduanya self-tagged/isi substansi SUPERSEDED oleh `galleon-product-definition.md` dan `http-engine-api.md`. |
| `dev-task-2-completed-sprints/` | 4 sprint `dev-task-2/` genuinely 100% `[x]` tanpa overclaim (sprint-1 63/63, sprint-2 8/8, sprint-3 13/13, sprint-7 23/23 — diverifikasi langsung hitung checkbox 2026-10-09) | Dipisah dari 5 sprint yang masih aktif (sprint-0/4/5/6 ada item `[ ]`, sprint-8 `planned`) sesuai keputusan Owner 2026-10-09 mencabut keputusan sebelumnya ("biarkan 9 sprint utuh") setelah PR #18/#19 menuntaskan sebagian besar kerja. |

## Catatan pengecualian penting

- **`product-research/community-tier-licensing-enforcement.md` TIDAK dipindah** meski Bagian 1 & 3-nya superseded — Bagian 2 (provenance lisensi ZeroClaw) eksplisit "TIDAK digantikan, tetap berlaku". File campuran tetap di tempat asal.
- **`dev-task-2/` 5 sprint aktif TIDAK dipisah** (sprint-0 1 item open, sprint-4 12 item open XL/berseri, sprint-5 2 item open, sprint-6 2 item open, sprint-8 `planned` 15 item open) — 4 sprint yang genuinely 100% selesai (1,2,3,7) SUDAH dipindah ke `dev-task-2-completed-sprints/` (2026-10-09), mencabut keputusan sebelumnya untuk membiarkan 9 sprint utuh.
- **`decisions/owner-decisions/` (ODR) TIDAK pernah diarsip** — ODR adalah record permanen, bukan dokumen kerja yang bisa "usang".
- **`decisions/finalize/dev-tasks/` (tanpa -2) TIDAK disentuh** — dokumen kerja aktif tim lain yang sedang coding di mesin lain.
- **`execute-done/`** sudah di tempat yang benar by definition, tidak dipindah lagi — tapi 3 file di `execute-done/phase3-go-engine-track-ab-and-tool-calling/` (`PRINCIPLE-PHASE3-REVIEW-FOCUS.md`, `PRINCIPLE-PHASE3-BRIEFING.md`, `PHASE3-SCOPE-TECH-SPEC.md`) punya teks literal "awaiting merge decision"/"before coding starts" yang berpotensi tidak 100% konsisten dengan folder induknya — perlu verifikasi manual sebelum dianggap final, tidak diotomatis pindahkan di sesi ini.

Lihat `standard-docs/decisions/finalize/dev-task-2/finding/` untuk audit lengkap yang menjadi dasar klasifikasi ini.
