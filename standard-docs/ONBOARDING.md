# ONBOARDING — Pintu Masuk `standard-docs/`

> **Baca ini DULU** jika kamu sesi baru / agent baru / orang baru di `galleon-fleet`.
> Tujuannya: tahu **apa yang landasan baku** (wajib dipahami sebelum kerja) dan
> **apa yang bukan** (pekerjaan berjalan — ada di tempat lain).

---

## Prinsip pemisahan (ditetapkan 2026-10-12)

`standard-docs/` sekarang **murni landasan baku** — hal yang jarang berubah dan
mengatur bagaimana produk dibangun:

- **Standar teknis** — arsitektur, boundary, konvensi kode, kontrak API.
- **Arah produk** — definisi produk + keputusan one-way-door (ODR/ADR).
- **Fondasi repo** — peta repo, permukaan aktif, build/CI, drift register.

**Pekerjaan berjalan (task, sprint, riset, backlog, governance operasional) TIDAK
lagi di sini** — pindah ke papan kerja bersama **`~/Apps/board/`** (board SSOT,
privat/git-lokal, dibaca KiroCrew & agy). Lihat `~/Apps/board/RULES.md`.

---

## Urutan baca onboarding (landasan)

1. **Fondasi repo** → [`foundation/README.md`](./foundation/README.md)
   (peta repo, permukaan aktif, build/test/CI, drift register)
2. **Arsitektur & boundary** → [`tech-standart/architecture/00-overview.md`](./tech-standart/architecture/00-overview.md)
   lalu [`01-boundary-policy.md`](./tech-standart/architecture/01-boundary-policy.md)
3. **Konvensi kode** per tier → [`tech-standart/tech-stack/`](./tech-standart/tech-stack/)
   (Rust / Go / web / Tauri) + [`tech-standart/code-quality/`](./tech-standart/code-quality/)
4. **Arah produk** → [`product/galleon-product-definition.md`](./product/galleon-product-definition.md)
5. **Keputusan final** (immutable) → [`decisions/owner-decisions/`](./decisions/owner-decisions/) (ODR)
   + ADR di [`decisions/`](./decisions/) (`adr-*.md`, `0001-*`..)
6. **Template** menulis ADR/ODR/task → [`templates/`](./templates/)

---

## Peta isi `standard-docs/` — landasan vs bukan

| Folder | Sifat | Onboarding? |
|--------|-------|-------------|
| `foundation/` | Fondasi repo (deskriptif) | ✅ Ya |
| `tech-standart/` | Standar teknis (preskriptif) | ✅ Ya |
| `product/` | Definisi produk | ✅ Ya |
| `decisions/owner-decisions/` (ODR) + ADR | Keputusan baku, immutable | ✅ Ya |
| `templates/` | Template artefak | ✅ Rujukan |
| `archive/`, `execute-done/` | Historis / fase selesai | 🗄️ Rujukan, bukan onboarding |
| `files/` | Lampiran data (inventaris) | 🗄️ Rujukan |
| `roadmap/` | **Tiket/lane aktif (SSOT eksekusi)** | ⚠️ Task berjalan — bukan landasan |
| `decisions/finalize/dev-task-2`, `dev-task-3` | **Task sprint tim lain (mesin terpisah)** | ⚠️ JANGAN diubah tanpa konfirmasi |

**Sudah DIPINDAH ke `~/Apps/board/`** (ada stub penunjuk di lokasi lama):
`backlog/` → `board/product-backlog/backlog-legacy/` · `research/` → `board/research/`
· `product-research/` → `board/product-research/` · `decisions/governance/` → `board/governance/`

---

## Aturan untuk agent (ringkas)

- **Mau tahu "bagaimana sistem dibangun / aturan baku"?** → baca `standard-docs/`.
- **Mau ambil / mengerjakan task?** → `~/Apps/board/` (ikuti `RULES.md` + `TEMPLATES.md` di sana).
- **Keputusan baru dianggap FINAL hanya bila jadi ODR** di `decisions/owner-decisions/`.
  Rekomendasi di dokumen riset/finding **bukan** keputusan.
- **`roadmap/` dan `dev-task-2/3` tidak dipindah** — `dev-task-2/3` dikerjakan tim
  lain di mesin terpisah; jangan sentuh tanpa konfirmasi Owner.

_Versi 1 — 2026-10-12._
