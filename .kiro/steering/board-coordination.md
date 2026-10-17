---
title: Board koordinasi KiroCrew ⇄ agy
inclusion: always
---

# Board SSOT — koordinasi KiroCrew ⇄ agy (Antigravity)

> **Scope:** steering **workspace `galleon-fleet` saja** (bukan global). Aturan board
> ini hanya aktif saat sesi bekerja di repo ini.

Ada papan kerja bersama **`~/Apps/board/`** (git lokal privat, tidak di-push).
KiroCrew & agy sama-sama baca/tulis. **Sebelum menulis ke board, baca
`~/Apps/board/RULES.md` + `~/Apps/board/TEMPLATES.md`** — penulisan yang tidak
sesuai front-matter **ditolak `validate.py`**.

## Alur (Scrum + Jira-style)
```
Story → Backlog → PRD/Riset │ Spec → Task → Report
product-backlog/              sprints/sprint-NN/
```
ID per jenis: `US- BL- PRD- SPEC- TASK- RPT-` + 3 digit. Tiap artefak simpan
jejak balik (`source_story/_prd/_spec/_task`) ke ID yang **benar-benar ada**.

## Aturan inti (ringkas; detail di RULES.md)
- **Task** = JSON di `tasks/`; `assignee: kirocrew|agy`, `status: todo→in-progress→blocked→review→done`.
- Task `assignee: agy, status: todo` dieksekusi otomatis oleh cron `board-agy-bridge`
  (script `~/.kiro/crew/crons/board_agy_bridge.py`, tiap 5m): agy headless (`-p`,
  **bukan `-c`**) → tulis `reports/RPT-xxx.md` → set task `done`. Jangan jalankan agy manual untuk task board.
- **Report `done` wajib bukti** (path/commit) di bagian Cek Penerimaan.
- Sprint sekuensial dari 01; tutup sprint = `sprint-manager.py close NN` (carry-over
  task belum-done ke sprint berikut, lalu arsipkan). Satu sprint aktif.
- Status task bisa diubah via drag-to-move di UI board (`/board?view=status`).

## Landasan vs pekerjaan (penting)
- **`standard-docs/`** = landasan baku (arsitektur, standar teknis, arah produk/ODR).
  Baca `standard-docs/ONBOARDING.md` saat onboarding.
- **`~/Apps/board/`** = pekerjaan berjalan (riset, backlog, task, sprint, report).
- Keputusan FINAL hanya bila jadi **ODR** di `standard-docs/decisions/owner-decisions/`;
  rekomendasi di dokumen riset/finding **bukan** keputusan.
