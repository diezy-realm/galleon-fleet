# Galleon Fleet Skills Index & Knowledge Library

**Status:** Normative Knowledge Manifest  
**Version:** 1.0.0  
**Reference:** R-92, R-93, open-skills specification  

---

## 1. Ringkasan Knowledge Library

Dokumen ini adalah katalog kanonikal skills yang terpasang dan tersedia di repositori Galleon Fleet. Setiap skill mengikuti spesifikasi `agentskills.io` yang memuat manifest `SKILL.md`, metadata frontmatter, referensi, dan skrip executable pendukung.

---

## 2. Katalog Skills Terdaftar

| Nama Skill | Kategori | Path Relatif | Deskripsi Singkat |
| :--- | :--- | :--- | :--- |
| **rust-cargo-optimize** | Architecture / CI | `.gemini/config/skills/rust-cargo-optimize/SKILL.md` | Auto-activates pada berkas Rust. Menjamin kompilasi Cargo kecepatan tinggi, fast linking, dan eksekusi nextest. |
| **graphify** | Knowledge Graph | `.gemini/config/skills/graphify/SKILL.md` | Visualisasi struktur kode, arsitektur, dan relasi antarmuka melalui knowledge graph interaktif. |
| **git-pr** | Git / Workflow | `.gemini/config/skills/git-pr/SKILL.md` | Konvensi canonical commit `feat({tiket}): {deskripsi}` dan manajemen branching/PR. |
| **ponytail** | Quality / Simplicity | `.gemini/config/skills/ponytail/SKILL.md` | Pola minimalis senior developer: pencegahan over-engineering, YAGNI, standard library first. |
| **apm-check** | Observability | `.gemini/config/skills/apm-check/SKILL.md` | Standar instrumentasi Elastic APM per-layer (usecase, repository, HTTP client, Redis). |
| **research-methodology** | Research & SOP | `.gemini/config/skills/research-methodology/SKILL.md` | Framework riset teknis terstruktur dengan dekomposisi query dan evaluasi primer. |
| **crew-last-report** | Logging / State | `.gemini/config/skills/crew-last-report/SKILL.md` | Logging dan retrieval status laporan sesi terbaru per-crew ke persistent log. |
| **frontend-modern-stack** | Frontend Stack | `.gemini/config/skills/frontend-modern-stack/SKILL.md` | Aturan arsitektur Next.js 15, React 19, dan Tailwind CSS v4. |
| **sql-review** | Database / GORM | `.gemini/config/skills/sql-review/SKILL.md` | Review query database, indexing, pencegahan N+1, dan isolasi transaksi. |

---

## 3. Struktur Kanonikal Skill Directory

```
<skill-dir>/
├── SKILL.md            # Manifest utama (YAML frontmatter + petunjuk instruksional)
├── scripts/            # (Opsional) Skrip helper / automasi yang dapat dieksekusi
├── references/         # (Opsional) Dokumentasi pendukung, manual, atau cheat-sheet
└── assets/             # (Opsional) Berkas statis pendukung
```

---

## 4. Mekanisme Loading & Cache

Runtime `galleon-runtime::skills` memuat skill melalui:
1. **Workspace Scope:** `.gemini/skills/` dan direct repository workspace skills.
2. **Global User Scope:** `~/.kiro/crew/skills/` atau `~/.config/galleon/skills/`.
3. **In-Memory Cache:** `galleon-runtime::skills::cache` meng-cache manifest yang telah diparsing untuk menghindari I/O disk berulang selama sesi agent aktif.
