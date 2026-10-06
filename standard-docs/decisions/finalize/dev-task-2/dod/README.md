---
title: DoD Index — dev-task-2
status: active
date: 2026-10-05
---

# Definition of Done & PR Checklist — Index

> Satu file per sprint, format konsisten. Checklist DoD di sini TIDAK mengubah gerbang resmi `roadmap/PR-plan.md` §1 (GW-0..9) — hanya mengelompokkannya per-sprint supaya Principle punya satu daftar per PR-batch, sesuai pola kerja existing "PR batch-by-phase, bukan per-task" dan "Principle review langsung ke FE/BE, Lead sentuh yang sudah lolos".
>
> **GW-0b Commit/PR naming** (ditambahkan 2026-10-05, berlaku di SETIAP sprint di bawah): judul commit DAN judul PR WAJIB `feat({tiket}): {deskripsi}` — lihat `../README.md` §Konvensi Commit & PR untuk aturan lengkap + §Kebijakan Branch (satu branch `feat/refactoring-finalize-01`, tanpa sub-branch kecuali darurat). PR dengan ID tiket typo/tidak ketemu di sprint file manapun, atau prefix selain `feat(...)`, DITOLAK Principle sampai diperbaiki.

## Alur Review (dipertahankan dari lesson existing, tidak berubah)

```
Sprint N semua task [x] (tim dev + Lead verifikasi)
     ↓
Satu PR batch ke `testing` (bukan per-task)
     ↓
Principle review: gerbang relevan (lihat tabel per-sprint) + checklist DoD sprint ini
     ↓
Temuan? → feedback LANGSUNG ke FE/BE (bukan lewat Lead) → revisi → re-review
     ↓
Clean (quality + CI hijau) → Merge `testing`
     ↓
QA test → lolos → PR `testing→prerelease` (Principle) → Lead verifikasi DoD final
     ↓
`prerelease→main` = approval Achmad (release)
```

## Peta File

| File | Sprint | Gerbang Dominan |
|---|---|---|
| [`dod-sprint-0-security-foundation.md`](dod-sprint-0-security-foundation.md) | 0 | GW-2 (Go), GW-1 (Rust), GW-8 (Keamanan) |
| [`dod-sprint-1-persistence-and-quality-gates.md`](dod-sprint-1-persistence-and-quality-gates.md) | 1 | GW-1 (Rust), GW-2 (Go) |
| [`dod-sprint-2-attachment-and-messaging-integrity.md`](dod-sprint-2-attachment-and-messaging-integrity.md) | 2 | GW-1 (Rust), GW-8 (Keamanan — privasi) |
| [`dod-sprint-3-sop-constraint-and-raci.md`](dod-sprint-3-sop-constraint-and-raci.md) | 3 | GW-1 (Rust), GW-5 (Kontrak) |
| [`dod-sprint-4-crate-decoupling-and-runtime-strangler.md`](dod-sprint-4-crate-decoupling-and-runtime-strangler.md) | 4 | GW-1 (Rust), GW-2 (Go), GW-6 (Integrasi) |
| [`dod-sprint-5-mcp-plugin-and-subagent-resource.md`](dod-sprint-5-mcp-plugin-and-subagent-resource.md) | 5 | GW-1 (Rust), GW-8 (Keamanan) |
| [`dod-sprint-6-cli-client-protocol-and-providers.md`](dod-sprint-6-cli-client-protocol-and-providers.md) | 6 | GW-1 (Rust), GW-5 (Kontrak) |
| [`dod-sprint-7-product-features-and-org-model.md`](dod-sprint-7-product-features-and-org-model.md) | 7 | GW-3 (UI/Desktop), GW-2 (Go) |
| [`dod-sprint-8-rebrand-and-release.md`](dod-sprint-8-rebrand-and-release.md) | 8 | GW-0 (Hygiene), GW-6 (Integrasi), GW-9 (Rollback) |

## Aturan Merge (tidak berubah dari `PR-plan.md`)

Semua gerbang relevan hijau + 1 approval Principle + tidak ada thread terbuka. PR rename massal (N-2x, Sprint 8) hanya mekanis (skrip + laporan), tidak bercampur perubahan perilaku.
