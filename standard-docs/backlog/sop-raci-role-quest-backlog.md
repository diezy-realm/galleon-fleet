---
title: Backlog — SOP Do's/Don'ts, RACI, Role Templates, Quest Maturity
status: backlog
date: 2026-10-04
author: squad-lead
derived_from:
  - research/orders-quest-sop-workflow-role-audit.md
  - decisions/sop-do-dont-raci-role-templates-design-spec.md
---

# Backlog — SOP Do's/Don'ts, RACI, Role Templates, Quest Maturity

> **Tujuan folder ini:** sama seperti `skills-steering-lessons-backlog.md` — menjembatani
> `research/`+`decisions/` ke task-breakdown siap-bagi. Design-spec lengkap (kode siap-tempel) ada di
> `standard-docs/decisions/sop-do-dont-raci-role-templates-design-spec.md`; dokumen ini HANYA
> task-breakdown + urutan + dependency, bukan duplikasi kode.
>
> **Belum ada di `roadmap/` resmi.** **TIDAK menyentuh `decisions/finalize/dev-tasks/`.**

---

## BL-06 — Field Do's/Don'ts Terstruktur di `SopStep`

**Rujukan design-spec:** §1 (`sop-do-dont-raci-role-templates-design-spec.md`)

**Scope:** Tambah `StepConduct{dos, donts}` + field `conduct: Option<StepConduct>` di `SopStep` (`crates/clawcrew-runtime/src/sop/types.rs`); update titik render prompt (lokasi di `sop/executor.rs` atau `agent/turn/`) agar `conduct` ikut disuntik ke step prompt.

**File ownership:** Backend-Rust, satu file utama (`sop/types.rs`) + satu titik render prompt.

**Definition of Done:** Test baru menyusul pola `default_step_contract_fields_do_not_serialize` yang sudah ada (`conduct: None` tidak serialize); 211 call-site existing (`SopStep{..}` via `..Default::default()`) tetap compile tanpa perubahan.

**Dependency:** Tidak ada. Scope kecil, isolated.

**Risiko/Severity:** LOW — field opsional baru, backward-compatible by design (`#[serde(default)]`).

---

## BL-07 — Field RACI Lengkap di `Sop` Struct

**Rujukan design-spec:** §2

**Scope:** Tambah `accountable_role: Option<String>` dan `informed_roles: Vec<String>` di `Sop` + `SopMeta` (TOML manifest) + `SopManifest::from_sop()` mapping. Tambah event emission baru (`sop.run_completed_notify` atau nama serupa) di titik transisi state terminal (`sop/engine/mod.rs` atau `sop/dispatch.rs`) untuk `informed_roles`.

**File ownership:** Backend-Rust — `sop/types.rs` (sama file dengan BL-06, serialkan) + titik transisi state.

**Definition of Done:** Test baru: SOP dengan `informed_roles` yang mencapai status terminal menghasilkan event dengan payload `informed_roles`. Reuse channel notifikasi existing — JANGAN bangun mekanisme pengiriman baru di Rust (itu kebijakan Go, lihat prinsip split architecture).

**Dependency:** Setelah BL-06 (file sama, `types.rs`) — serialkan untuk hindari merge conflict.

**Risiko/Severity:** LOW-MEDIUM — field baru low-risk, tapi event-emission perlu dicek tidak mengganggu konsumen event existing.

---

## BL-08 — Template Role per-Crew (Backend/Frontend/QA)

**Rujukan design-spec:** §3 (isi lengkap 3 file sudah ditulis di design-spec, siap-tempel)

**Scope:** Buat 3 file baru di `crates/clawcrew-runtime/src/agent/personality_templates/`: `AGENTS.backend.md`, `AGENTS.frontend.md`, `AGENTS.qa.md`.

**BLOCKER yang harus diverifikasi SEBELUM task ini bermakna:** apakah loader `load_skills_for_agent`/konfigurasi `AliasedAgentConfig` punya mekanisme pilih file `AGENTS.<role>.md` berdasarkan agent alias/role, atau saat ini hardcode satu nama file (`AGENTS.md`) tanpa variasi? **Verifikasi ini dulu** (grep loader untuk nama file personality template) — kalau hardcode, BL-08 butuh sub-task tambahan "tambah logic pemilihan file per-role" sebelum 3 file baru ini punya efek.

**File ownership:** squad-lead (isi konten, sudah final di design-spec) + Backend-Rust (verifikasi + logic loader kalau perlu).

**Definition of Done:** 3 file baru ada; DAN dikonfirmasi (via test atau manual trace) bahwa agent dengan role tertentu benar-benar memuat file yang sesuai, bukan selalu `AGENTS.md` generik.

**Dependency:** Tidak ada untuk pembuatan file; verifikasi loader independen, bisa paralel dengan BL-06/BL-07.

**Risiko/Severity:** LOW untuk pembuatan file; MEDIUM untuk verifikasi loader (bisa jadi menemukan gap tambahan).

---

## BL-09 — Matangkan `Quest` (Go) + `mission.Service` Baru

**Rujukan design-spec:** §4

**Scope:**
1. Perluas `Quest` struct (`engine/src/mission/interfaces.go`): `QuestStatus` enum (ganti dari `string` bebas), `StartedAt`/`CompletedAt`/`FailureReason`, `SopRunIDs []string`, `AccountableRole`/`InformedRoles` (selaras BL-07 tapi level Quest, independen).
2. **Verifikasi sebelum eksekusi:** `git grep "\.Status = "` di kode yang pakai `Quest` — pastikan tidak ada assignment string sembarang yang akan pecah saat tipe diketatkan ke `QuestStatus` enum.
3. Buat `mission.Service` baru (interface + implementasi) menyusul pola `fleet.Service`/`memory.Service`: `CreateQuest`, `GetQuest`, `UpdateQuestStatus`, `ListQuestsByShip`. **Ini scope TERPISAH dan LEBIH BESAR** dari sekadar tambah field — `mission` package saat ini tidak punya service layer sama sekali.

**File ownership:** Backend-Go — `engine/src/mission/interfaces.go` (field) + `engine/src/mission/service.go` (BARU, service layer).

**Definition of Done:** Test lifecycle `Quest` (Pending→Active→Completed/Failed) menyusul pola `sop_run_serde_roundtrip` Rust; `SopRunIDs` terisi saat satu Quest mengorkestrasi >1 SOP run (test integrasi, bisa pakai mock).

**Dependency:** Independen dari BL-06/07/08 — bisa paralel, tapi scope JAUH lebih besar (service layer baru). **Jangan under-estimate sebagai "tambah field" — alokasikan sebagai task tersendiri, bukan dikerjakan sambil lalu.**

**Risiko/Severity:** MEDIUM — perubahan tipe `Status` dari string ke enum punya risiko breaking kalau ada caller yang belum teridentifikasi.

---

## Urutan Eksekusi yang Disarankan

1. **BL-06** dulu (scope kecil, tidak ada dependency, langsung actionable)
2. **BL-07** setelah BL-06 (file sama, serialkan)
3. **BL-08** bisa paralel dengan BL-06/07, tapi verifikasi loader dulu sebelum isi file dianggap "selesai"
4. **BL-09** terakhir atau sebagai inisiatif terpisah — scope-nya beda kelas (service layer baru, bukan sekadar field)

## Item yang BELUM masuk backlog (masih level riset)

- **Error Workflow terpusat** (setara n8n Error Trigger node) — dari `orders-quest-sop-workflow-role-audit.md` §4 poin 5. Belum dirancang karena dinilai belum urgent untuk skala SOP saat ini. Angkat ke backlog kalau jumlah SOP produksi sudah banyak dan pola gagal berulang.

## Catatan Proses

Sama seperti `skills-steering-lessons-backlog.md`: setiap item perlu diangkat ke `roadmap/*.md` sebelum di-assign ke squad. Setelah dieksekusi, update status di sini (atau hapus) agar tidak dua tempat mencatat hal sama.
