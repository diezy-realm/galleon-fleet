# R-00 — Inventaris `clawcrew-runtime` terhadap Boundary Policy

> **Status:** Klasifikasi DISETUJUI Owner 2026-10-04 (keputusan terbuka ditutup ODR-21..25). Verifikasi per-modul §4 tetap wajib sebelum eksekusi. Tidak ada kode yang diubah atau dihapus.
> **Metode:** ukuran (non-blank LOC per modul top-level di `crates/clawcrew-runtime/src`), jumlah referensi eksternal `clawcrew_runtime::<modul>` per konsumen (grep `*.rs` tracked, 2026-10-04), dan keterkaitan internal `crate::<modul>`. Klasifikasi = **usulan berbasis nama, ukuran, dan pola pemakai**; sebelum eksekusi tiap modul wajib dibaca isinya (kolom *Verifikasi*).
> **Dasar:** `01-boundary-policy.md`, ODR-03/07/11/12/14, `foundation/05-orchestrator-placement-analysis.md`.

## 1. Ringkasan

`clawcrew-runtime` ≈ 286k LOC, 39 modul top-level. Usulan:

| Kategori | Arti | LOC (kasar) | Modul |
|---|---|---|---|
| **KEEP** | milik System Core / host lokal Rust | ±55k | `security`, `observability`, `platform`, `plugin_runtime`, `health`, `util`, `hooks`, `doctor`, `daemon`, `service`, `restart`, `quickstart`, `migration`, `enroll`, `identity`, `trust`, `tunnel`, `relay`, `process_stats`, `cli_input`, `browse`, `integrations`, `peers` |
| **MOVE→Go** | otak/orkestrasi, state, policy | ±180k | `agent`, `sop`, `skills`, `cron`, `heartbeat`, `routines`, `control_plane`, `session`, `rag`, `subagent`, `calendar`, `cost`, `approval`, `tools/delegate` |
| **DECIDE** | transport/legacy; nasib mengikuti MOVE | ±40k | `rpc`, `verifiable_intent`, `tools` (non-native), `hooks` (sebagian) |

Artinya **±63% runtime adalah kandidat pindah**. Memecah crate sebelum ini = kerja ganda.

## 2. Tabel per modul

Kolom: LOC · referensi eksternal (jumlah) · konsumen (jumlah *file*) · internal (`crate::`).

| Modul | LOC | Ext | Konsumen eksternal | Int | Usulan | Alasan & catatan |
|---|---|---|---|---|---|---|
| `agent` | 77.219 | 251 | channels 5, eval 1, gateway 9, infra 1, src 2, tests 2 | 566 | **MOVE→Go** | `turn` 19,8k, `loop_` 19,2k, `agent` 15,2k, `safety_net` 3,7k, `tool_execution` 2,2k, `cost` 2k, `memory_inject` 1,8k, `prompt` 1,6k. Inti otak (ODR-11). Yang **tinggal di Rust**: eksekusi tool native & panggilan provider (sudah via `ExecuteTurn`/`ExecuteNativeTool`). `safety_net`/`leak` sebagian mungkin KEEP — verifikasi |
| `tools` (di runtime) | 39.490 | 111 | channels 3, eval 1, gateway 8, tools 1, src 2, tests 2, xtask 1 | 590 | **DECIDE** | `delegate` 13,5k = sub-agent → MOVE. Tool native (`shell`, `file_read`, …) → KEEP, **tetapi duplikasi dengan crate `clawcrew-tools` (85k)** — wajib cek tumpang tindih. `skill_*`, `cron_add`, `schedule` ikut modulnya |
| `sop` | 38.516 | 207 | channels 6, gateway 9, src 3, xtask 2 | 467 | **MOVE→Go** | `engine` 15k, `approval` 4,6k, `dispatch`, `store`, `capability`. Tipe wire `sop::types` (29 ref dari channels) → kontrak proto/`galleon-sop-graph` |
| `rpc` | 32.105 | 17 | zerocode 1, gateway 5, src 1 | 240 | **DECIDE→hapus** | `dispatch` 18k, `wss` 2,5k, `local` 2k, `session`, `turn`: transport klien↔daemon Rust. Digantikan Go HTTP/WS (R-34). Hapus **setelah** `galleoncode`/CLI pindah |
| `security` | 19.036 | 85 | relay 1, channels 7, gateway 14, log 1, src 4, tests 1 | 174 | **KEEP** | sandbox (`seatbelt`, `landlock`), `leak_detector`, `audit`, `webauthn`, `cert_ledger`, `auth_provider`, `principal_resolver`. `scrub` (36 ref dari channels) → ekstrak (R-11). **Periksa:** `principal_resolver`/`auth_provider` apakah policy (→Go) atau mekanisme (KEEP) |
| `skills` | 12.426 | 59 | channels 6, gateway 2, src 1 | 98 | **MOVE→Go** | `creator`, `improver`, `suggestions`, `audit` = pembelajaran & memori (ODR-03) |
| `cron` | 10.039 | 106 | gateway 3, src 2, tests 1 | 85 | **MOVE→Go** | scheduler terikat Squad/Crew; gateway↔runtime cron (426 ref) hilang bersamanya |
| `control_plane` | 6.882 | 24 | gateway 1 | 136 | **MOVE→Go** | tata kelola fleet = Go. Verifikasi isi |
| `daemon` | 6.160 | 10 | gateway 4, src 1 | 8 | **KEEP** (ramping) | proses latar Rust (gateway sidecar). Hapus bagian agent/cron setelah pindah |
| `verifiable_intent` | 4.616 | 1 | src 1 | 17 | **DECIDE** | hampir tak dipakai; kandidat hapus. Perlu keputusan produk |
| `observability` | 4.355 | 67 | channels 3, gateway 6, src 1, tests 1 | 174 | **KEEP** | tracing/metrik/scrub. Dipecah ke crate kecil (R-12) |
| `service` | 3.964 | 1 | src 1 | 9 | **KEEP** | instal service OS (CLI lokal) |
| `quickstart` | 3.484 | 32 | zerocode 1, gateway 2, src 2, tests 1 | 14 | **KEEP** | onboarding CLI; sesuaikan ke Go |
| `doctor` | 3.242 | 8 | channels 1, gateway 1, src 1, tests 1 | 14 | **KEEP** | diagnosis host; tambah cek Go engine |
| `hooks` | 2.270 | 73 | channels 2, src 1 | 75 | **DECIDE** | hook lifecycle agent → MOVE; hook transport channel → KEEP |
| `relay` | 2.004 | 6 | src 1 | 1 | **KEEP** | terkait `galleonrelay` |
| `platform` | 1.942 | 4 | channels 1, eval 1, gateway 1, src 1 | 78 | **KEEP** | abstraksi OS |
| `enroll` | 1.779 | 11 | zerorelay 1, src 1 | 13 | **KEEP** | pairing relay; selaraskan dengan G-20 (QR) |
| `tunnel` | 1.592 | 1 | src 1 | 0 | **KEEP** | tunnel host |
| `identity` | 1.305 | 3 | gateway 1, src 1 | 2 | **DECIDE** | identitas perangkat vs identitas Quartermaster — cek |
| `heartbeat` | 1.079 | 1 | src 1 | 7 | **MOVE→Go** | |
| `approval` | 1.076 | 5 | channels 1, src 1 | 30 | **MOVE→Go** | policy/approval = SSOT Go (`fleet/policies.go`) |
| `plugin_runtime` | 924 | 11 | channels 3, tests 2 | 2 | **KEEP** | host WASI |
| `integrations` | 829 | 11 | gateway 1, src 1 | 0 | **DECIDE** | cek isi |
| `trust` | 772 | 1 | src 1 | 0 | **KEEP** | |
| `browse` | 706 | 4 | gateway 1, src 1 | 0 | **KEEP** | tool native browser |
| `migration` | 634 | 1 | src 1 | 0 | **KEEP** | migrasi data lokal (bisa dipakai N-23) |
| `routines` | 557 | 1 | src 1 | 1 | **MOVE→Go** | |
| `cli_input` | 458 | 2 | channels 1, src 1 | 0 | **KEEP** | |
| `session` | 333 | 2 | gateway 1 | 3 | **MOVE→Go** | |
| `calendar` | 327 | 0 | – | 2 | **MOVE/HAPUS** | tak ada konsumen eksternal |
| `rag` | 309 | 1 | src 1 | 8 | **MOVE→Go** | RAG = Go (G-13) |
| `subagent` | 302 | 0 | – | 1 | **MOVE/HAPUS** | tak ada konsumen eksternal |
| `process_stats` | 271 | 2 | gateway 1 | 1 | **KEEP** | sumber metrik nyata (G-22) |
| `health` | 254 | 56 | channels 5, gateway 4, src 1 | 42 | **KEEP** | |
| `util` | 251 | 3 | channels 2, src 1 | 11 | **KEEP** | |
| `restart` | 235 | 12 | gateway 1, src 1 | 4 | **KEEP** | |
| `peers` | 130 | 4 | channels 1, tests 1 | 2 | **DECIDE** | |
| `cost` | 4 | 5 | channels 2, gateway 1, src 1 | 27 | **MOVE→Go** | stub; biaya = Treasury Go |

**Modul tanpa konsumen eksternal (kandidat hapus cepat, tetap setelah verifikasi):** `calendar`, `subagent`; hampir tanpa konsumen: `verifiable_intent`, `trust`, `tunnel`, `migration`, `routines`, `heartbeat`, `rag`.

## 3. Pola migrasi (strangler, bukan big-bang)

1. **Freeze** — tidak ada fitur baru di modul MOVE (aturan PR: Principle menolak). Bug-fix keamanan diperbolehkan.
2. **Bangun di Go** (G-30..G-33) di atas `ExecuteTurn`/`ExecuteNativeTool`; parity diuji dengan **`galleon-eval` yang diretarget** (R-37) — fixture trace yang sama dijalankan melawan Rust lama dan Go baru.
3. **Alihkan konsumen:** CLI (`galleon`, `galleoncode`) → Go HTTP/WS (R-34, R-36); gateway REST/WS/A2A (`api*.rs`, `ws.rs`, `a2a.rs`) dihapus bila tak ada pemakai (ODR-14).
4. **Hapus** modul MOVE di Rust setelah: (a) nol konsumen (`cargo metadata` + grep), (b) parity eval hijau, (c) sign-off Owner.
5. **Pecah/ramping** sisa runtime (R-16) → crate kecil (`security`, `observability`, `host` (daemon/service/doctor/…)).

Urutan modul yang disarankan (risiko rendah→tinggi): `calendar`/`subagent`/`cost` → `rag`/`session`/`routines`/`heartbeat` → `cron` → `skills` → `approval`/`control_plane` → `sop` → `agent` (terakhir) → `rpc`.

## 4. Verifikasi wajib sebelum eksekusi tiap modul (belum dilakukan)

- [ ] Baca isi modul: apakah ada logika yang memang harus tinggal di Rust (sandbox, parsing protokol, kripto).
- [ ] Pemanggil runtime-time (bukan hanya compile-time): jalur CLI/gateway/channel yang memicu modul.
- [ ] Data on-disk milik modul (format, migrasi) — hindari kehilangan data pengguna.
- [ ] Padanan Go sudah ada? (cek `engine/src/*` agar tidak duplikat).
- [ ] Perbandingan `runtime/tools` vs crate `clawcrew-tools` (85k) — peta duplikasi.
- [ ] Kontribusi ke `Cargo.toml` features (root punya feature `agent-runtime` dst.) dan test yang menggantung.

## 5. Dampak ke rencana lain

| Dokumen | Perubahan |
|---|---|
| `R-rust.md` | R-10..R-13 (ekstrak i18n/scrub/traits dari runtime) **tetap berguna** (channels dan gateway tetap butuh modul KEEP). R-15 pecah channels tidak terhalang. R-16 = eksekusi hasil dokumen ini |
| `G-go-engine.md` | G-30..G-33 menjadi jalur kritis; tambahkan G-34 (padanan `control_plane`/`approval`) |
| `N-*` | **Jangan rename `clawcrew-runtime` sebelum R-16** (isi berubah); `eval` & `sop-graph` rename biasa |
| `PR-plan.md` | PR-05 = dokumen ini; PR-25 dipecah per modul (1 PR per modul, urutan §3) |

## 6. Keputusan terbuka — dianalisis di `R-00a-open-decisions-analysis.md`

1. **`verifiable_intent` (4,6k)** — fitur produk yang diinginkan? Bila tidak: hapus.
2. **Batas "keamanan sebagai mekanisme vs kebijakan"** untuk `security::{principal_resolver,auth_provider,webauthn}` — usulan: mekanisme di Rust, keputusan policy di Go.
3. **`rpc` penghapusan** setelah CLI pindah — setuju sebagai urutan terakhir?
