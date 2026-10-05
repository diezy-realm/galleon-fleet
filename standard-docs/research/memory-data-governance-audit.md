---
title: Audit Mendalam clawcrew-memory — Data Governance (Retention/Purge/Privacy)
status: research
date: 2026-10-05
author: squad-lead
depends_on: research-gap-map-2026-10-05.md
---

# Audit Mendalam `clawcrew-memory` — Data Governance (Retention/Purge/Privacy)

> Fokus murni retention/purge/privasi data yang SUDAH tersimpan — berbeda dari "Learning by Consent" (riset sesi sebelumnya, soal data BARU masuk).

## 1. `hygiene.rs` — Aturan Archive vs Purge

Cadence cek: 12 jam (`HYGIENE_INTERVAL_HOURS`), gate via state file `memory_hygiene_state.json`.

- **Archive** (`archive_daily_memory_files`/`archive_session_files`) — pindah file `.md`/sesi ke folder `archive/` jika umur `> archive_after_days`. Ini **move**, bukan hapus.
- **Purge** (`purge_memory_archives`/`purge_session_archives`) — `fs::remove_file` kalau umur archive `> purge_after_days`. **Unlink fisik, bukan soft-delete.**
- **Prune kategori SQLite** (`prune_category_rows`): `DELETE FROM memories WHERE category = ?1 AND {timestamp_col} < ?2 AND pinned = 0`. **DELETE SQL asli.** Baris `pinned = 1` DIKECUALIKAN dari purge by design — implikasi: user bisa pin data yang seharusnya ikut terhapus saat "lupakan saya" diminta (lihat §4).
- **Prune audit** (`prune_audit_entries`): `DELETE FROM memory_audit WHERE timestamp < ?1`.

Threshold (`archive_after_days`, `purge_after_days`, retensi per kategori) berasal dari `MemoryConfig`, **bukan hardcoded** — tapi granularitasnya per kategori-memori/workspace, **bukan per-individual-user**.

## 2. `policy.rs` — Kuota & Override Retensi, Bukan Access-Control

`PolicyEnforcer`: `is_read_only`, `check_namespace_limit`/`check_category_limit` (kuota), `retention_days_for_category` (override retensi per kategori dari config). Tidak ada logic authz/akses user di sini — namanya "policy" tapi scope-nya murni kuota+retensi.

## 3. `audit.rs` — Audit Trail Operasi Memory (Punya Retensi Sendiri)

`AuditedMemory<M>` decorator mencatat tiap operasi (`Store, Recall, Get, List, Forget, Purge, StoreProcedural`) ke tabel `memory_audit` di `audit.db` TERPISAH dari `brain.db`. Ini audit trail "siapa hapus apa kapan", bukan audit kualitas memori.

**Retensi audit log**: `prune_older_than(retention_days)` — `DELETE FROM memory_audit WHERE timestamp < ?1`, dipanggil dari hygiene, INDEPENDEN dari `config.audit_enabled` (test eksplisit menegaskan "disabling collection must not disable retention for existing audit rows"). Default 30 hari. **Audit log tidak tumbuh selamanya** — ini poin plus.

## 4. "Right to Delete" / User-Initiated — GAP TEMUAN UTAMA

**Yang ADA**: `forget(key)` (hapus 1 baris by key, semua backend), `purge_namespace`/`purge_session`/`purge_agent` (trait-level bulk delete, default `bail!` di trait, override nyata di SQLite/Postgres/Qdrant), dan tool `memory_purge` (`clawcrew-tools/src/memory_purge.rs`) — dipanggil via agent-tool-call, bukan tombol self-service UI end-user.

**Yang TIDAK DITEMUKAN** (grep nol hasil untuk `delete_all`, `purge_user`, `wipe`, `erase`, `gdpr`, `right_to_delete`):
- Tidak ada SATU entry point yang menyapu SEMUA kategori (`conversation`+`daily`+`core`+namespace lain) + SEMUA backend (SQLite+Postgres+Qdrant) + audit log untuk SATU identitas sekaligus.
- `purge_agent` scope-nya **agent/alias**, bukan **user** — pemetaan ke permintaan privasi end-user tidak langsung.
- `purge_agent` **tidak menyentuh `audit.db`** — hapus agent, baris memory hilang, tapi jejak akses di `memory_audit` tetap ada sampai retensi alami lewat.
- Satu-satunya pemanggil `purge_agent` yang ditemukan: `clawcrew-gateway/src/agent_owned_state.rs:71`, dipicu saat alias agent **dicabut dari config** — bukan dari permintaan privasi user.

## 5. Lintas-Store — Celah Signifikan: Hygiene Hanya SQLite

- SQLite: `purge_namespace`/`purge_session`/`purge_agent` implement nyata.
- PostgreSQL: `purge_agent` implement. `purge_namespace`/`purge_session` kemungkinan mewarisi default `bail!` trait (perlu verifikasi tambahan, tidak dibaca full).
- Qdrant: `purge_agent`/`export_agent` implement nyata via `delete_points_matching` filter payload.

**TEMUAN SIGNIFIKAN**: `hygiene.rs` (cron archive/purge berbasis umur, cadence 12 jam) **HARDCODED membuka `brain.db` SQLite langsung** via `rusqlite::Connection::open` — **tidak ada cabang kode yang memanggil Postgres atau Qdrant client**. Instalasi yang pakai Postgres/Qdrant sebagai backend **TIDAK MENDAPAT retensi otomatis berbasis umur** yang instalasi SQLite dapatkan secara default — kecuali purge dipicu manual (`memory_purge` tool, scope namespace/session, bukan time-based).

## 6. `budget.rs` — Resource Budget, BUKAN Privacy Budget

`compact_category_to_budget` adalah eviction berbasis KAPASITAS (row-count cap, byte-size cap), strategi `Value`(importance) atau `Oldest`. `pinned=1` selalu dilindungi (pola sama seperti hygiene). Ini murni soal ukuran DB tidak membengkak — **tidak ada hubungan dengan retensi legal/privasi**. Nama "budget" berpotensi membingungkan jika disalahartikan.

## 7. `agent_scoped.rs` — Isolasi Antar-Agent Sudah Benar

`AgentScopedMemory` filter berdasarkan `agent_id`+`allowed_agent_ids`+`allowed_categories`. Baris tanpa `agent_id` (legacy/unattributed) tetap lolos filter — potensi kebocoran data LAMA yang belum di-attribute (bukan kebocoran antar-agent yang sudah benar).

**Purge spesifik**: `purge_namespace` → **DITOLAK eksplisit** (`bail!`: *"cross-agent bulk delete must run through an admin Memory handle"*) — by design yang baik, cegah satu agent hapus data cross-agent. `purge_session` → di-redirect ke `purge_session_for_agent` (terskop ke agent bersangkutan saja).

**Implikasi**: hanya "admin Memory handle" (akses langsung tanpa wrapper) yang bisa `purge_agent`/`purge_namespace` lintas-agent — konsisten dengan temuan §4 bahwa tidak ada self-service delete-all, karena arsitekturnya memang sengaja mendorong itu ke "admin path" yang ternyata belum ada implementasi yang menyapu semua store.

## 8. `snapshot.rs` — Celah Potensial: Git History Menyimpan Data "Terhapus"

`export_snapshot()` hanya ekspor kategori `core` ke SATU file `MEMORY_SNAPSHOT.md` (overwrite in-place, bukan timestamped/append) — tidak ada akumulasi versi di dalam mekanisme ini sendiri.

**Celah potensial** (ditandai eksplisit sebagai TEMUAN POTENSIAL, bukan bukti pasti): file ini didesain *"lives at workspace root for Git visibility"* (komentar asli kode). Kalau workspace di-commit ke Git (sesuai niat desain — "soul" file untuk disaster recovery), **riwayat Git akan menyimpan versi lama file ini**, termasuk entry yang sudah dihapus dari `brain.db` via `purge_agent`/hygiene — karena `export_snapshot` menulis state BARU setelah purge, tapi commit Git SEBELUMNYA (kalau pernah di-commit) tetap menyimpan isi lama. Tidak ditemukan kode yang melakukan `git commit` otomatis di file ini — perlu verifikasi terpisah apakah ada automation lain yang mengcommit file ini secara rutin.

---

## Kesimpulan

Galleon **belum punya jawaban memadai** untuk "user minta semua datanya dihapus, apa yang terjadi?" dibandingkan prinsip dasar GDPR right-to-erasure (bukan untuk compliance legal formal, tapi sebagai tolok ukur kematangan desain).

**Yang sudah solid**:
- Primitif delete NYATA — DELETE SQL sungguhan (SQLite/Postgres), delete-points nyata (Qdrant), **bukan soft-delete/flag**.
- Isolasi antar-agent di level wrapper (`agent_scoped.rs`) sudah benar secara desain.
- Audit log punya retensinya sendiri, tidak tumbuh selamanya.

**Tiga gap signifikan**:
1. **Tidak ada single entry point** yang menyapu seluruh kategori + semua backend + audit log untuk satu identitas sekaligus — primitif ada tapi terpisah-pisah, perlu dirangkai manual oleh admin. Konsep "agent/alias" ≠ "user" membuat pemetaan ke permintaan privasi end-user tidak langsung.
2. **Hygiene/retensi otomatis hardcoded ke SQLite saja** — instalasi Postgres/Qdrant tidak mendapat proteksi retensi berbasis umur yang sama, menciptakan ketidakkonsistenan antar-backend.
3. **Snapshot/audit log tidak otomatis diselaraskan dengan purge** — `purge_agent` tidak membersihkan `memory_audit`; `MEMORY_SNAPSHOT.md` yang didesain untuk Git visibility berarti riwayat Git (bila dicommit) bisa menyimpan data yang sudah "dihapus" dari live store — celah klasik yang prinsip right-to-erasure (GDPR Art. 17) secara eksplisit mewajibkan ditangani (penghapusan harus mencakup salinan dan backup, bukan hanya live store).

**Rekomendasi tech-lead**: desain satu fungsi `erase_identity(user_or_agent_id)` yang secara eksplisit memanggil `purge_agent` di SETIAP backend terdaftar sekaligus (bukan satu-satu manual), lalu prune baris `memory_audit` terkait, lalu re-export snapshot kosong — dan dokumentasikan eksplisit bahwa commit Git lama tetap menyimpan jejak kecuali history di-rewrite.

---

## Sumber

- `crates/clawcrew-memory/src/hygiene.rs` (full read, 424 baris) — `run_if_due`, `HygieneReport`, `prune_category_rows`, `prune_audit_entries`
- `crates/clawcrew-memory/src/policy.rs` — `PolicyEnforcer`
- `crates/clawcrew-memory/src/audit.rs` — `AuditedMemory<M>`, `prune_older_than`
- `crates/clawcrew-memory/src/budget.rs` — `compact_category_to_budget`
- `crates/clawcrew-memory/src/agent_scoped.rs` — `AgentScopedMemory`, isolasi purge
- `crates/clawcrew-memory/src/snapshot.rs` — `export_snapshot`, `hydrate_from_snapshot`
- `crates/clawcrew-memory/src/sqlite.rs`, `postgres.rs`, `qdrant.rs` — implementasi `purge_namespace`/`purge_session`/`purge_agent` per-backend
- `crates/clawcrew-api/src/memory_traits.rs` — definisi trait default (`bail!`)
- `crates/clawcrew-tools/src/memory_purge.rs` — tool user-facing (via agent-call)
- `crates/clawcrew-gateway/src/agent_owned_state.rs:71` — satu-satunya pemanggil `purge_agent` ditemukan (trigger: alias dicabut dari config)
