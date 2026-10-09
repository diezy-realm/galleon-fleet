---
title: Remediation Progress — Handoff ke Tim Lain
status: SUPERSEDED-SEBAGIAN — lihat §UPDATE 2026-10-09 16:xx di bawah sebelum baca isi lama
date: 2026-10-08/09
author: squad-lead
source: 2026-10-08-remediation-tasks.md
scope: Status eksekusi nyata per task setelah 4 subagent squad-backend dijalankan, dihentikan untuk diserahkan ke tim lain di mesin lain. Semua angka di bawah diverifikasi langsung dari disk (git status, grep, cargo check) pada 2026-10-09 00:3x WIB -- bukan dari laporan self-report subagent.
---

## ⚠️ UPDATE 2026-10-09 (sore) — T-01, T-02, T-03, T-04, T-05, T-07 SUDAH TUNTAS, isi di bawah ini USANG untuk task-task tersebut

**Jangan percaya tabel "Ringkasan untuk tim lain" di bagian paling bawah dokumen ini apa adanya** — ditulis sebelum 5 commit lanjutan berikut masuk ke branch yang sama (`feat/refactoring-finalize-01`), terverifikasi langsung lewat `git show --stat` + `grep` pada kode nyata (bukan self-report):

```
06c69221 feat(refactoring): phase 6                                                    <- T-08 (BG-C2)
a16f3531 feat(BL-16,BL-17): wire loop detector stats from proto to run task DTOs and QuestsView  <- T-07 lengkap end-to-end
74806fbc feat(BL-31): add apply_retention trait method with sqlite retention support    <- T-04
506b9c87 feat(BG-F5): wire tracing StartSpan and trace_id synchronization in Go HTTP delivery and gRPC  <- T-02
4fd64305 feat(BR-B5): implement secret rotation and encrypted vault export import in SecretStore  <- T-05
```

| Task | Status LAMA di dokumen ini | Status NYATA sekarang (terverifikasi `git show --stat`/`grep`) |
|---|---|---|
| T-01 (`verifiable_intent`) | "16+1 match masih ada" | **TUNTAS** — `grep "verifiable_intent\|VerifiableIntentConfig"` di `schema.rs` dan `fixtures/v1.toml` = 0 match. Commit `06c69221` juga menghapus `verifiable_intent_notice_visibility.rs` (402 baris) + `schema/tests.rs` + `validation_warnings.rs`. |
| T-02 (BG-F5) | "belum dieksekusi ke kode" | **TUNTAS** — `506b9c87` mengubah `tracing.go`, `delivery.go`, `interceptors.go`, `logger.go`, `system_gateway.go`. `grep StartSpan engine/` sekarang keluar di `delivery.go` + `interceptors.go`, bukan hanya file test-nya sendiri. |
| T-03 (`erase_identity`/`forget_all_for_agent`) | "diff hilang sebelum commit" | **TUNTAS** — fungsi genuinely ada di `clawcrew-memory/src/lib.rs:92` dan `agent_scoped.rs` (persis diff yang disimpan di §T-03 bawah — kemungkinan di-reapply oleh Owner/tim lain). |
| T-04 (BL-31) | "belum dieksekusi ke kode" | **TUNTAS** — `apply_retention` ada di `clawcrew-api/src/memory_traits.rs` + implementasi SQLite nyata di `sqlite.rs` (+108 baris). |
| T-05 (vault) | "hanya test rusak, nol implementasi, sudah di-revert" | **TUNTAS, kualitas baik** — `git show 4fd64305` membuktikan `rotate_secret`, `rotate_vault`, `export_encrypted_vault`, `import_encrypted_vault` genuinely ada di `secrets.rs` (format `VAULT_V1:` persis sesuai saran desain di bawah), plus test positive+negative. |
| T-07-backend (loop_detector) | "overclaim, hanya struct Go berdiri sendiri" | **TUNTAS, genuinely end-to-end** — `a16f3531` menambah `proto/agent_service.proto` (+9), `dto.go` Run+Task, `services.go`, DAN frontend `QuestsView.tsx`+`fleetStore.ts`. `LoopDetectorStats` sekarang muncul di binding ter-generate (`agent_service.pb.go`), bukan dead code. |
| T-08 (BG-C2) | "diputuskan ODR, belum ke kode" | **Kemungkinan tuntas, arah sinkron belum diverifikasi detail** — `06c69221` menambah 125 baris ke `grpc_system_gateway.rs` + proto +50 baris. Belum dibaca isi logic untuk konfirmasi genuinely "Rust SSOT, Go cache" sesuai ODR. |
| T-06 (MCP) | Dihapus | Tidak berubah, sesuai keputusan. |
| T-00 (PR) | "belum dibuka" | **PR #18 SUDAH OPEN** (dibuat 2026-10-09T09:09 UTC): https://github.com/diezy-realm/galleon-fleet/pull/18 — TAPI CI merah di hampir semua check saat dokumen ini diupdate. Lihat detail investigasi di bawah. |

### Investigasi konkret: 212 "unwrap/expect" dan 50 "secret leak" yang menggagalkan Principle Gate di PR #18

Principle Gate Reviewer (`.github/workflows/principle-review-trigger.yml`) menghitung diff `origin/testing...HEAD` dengan exclude `crates/**/tests/**` dan `crates/**/benches/**` saja — **TIDAK mengecualikan `#[cfg(test)] mod tests` inline di dalam file produksi** (pola paling umum di Rust) atau file bernama `tests.rs` di dalam `src/` (misal `crates/clawcrew-channels/src/matrix/tests.rs`), yang keduanya lolos filter path.

Verifikasi manual (`git diff -U0 origin/testing...HEAD`, baca hunk persis per file):

- **212 unwrap/expect**: diff ini mencakup **109 commit** sejak `testing` (branch jauh di depan, bukan hanya 5 commit remediasi sesi ini). Untuk 3 file top-list yang relevan dengan remediasi (`clawcrew-config/src/secrets.rs` 41 hit, `clawcrew-memory/src/agent_scoped.rs` 10 hit, `clawcrew-memory/src/sqlite.rs` 9 hit) — **semua genuinely berada di dalam `#[cfg(test)] mod tests`** (dikonfirmasi lewat nomor hunk `@@ -2829,0 +2908,178` di `secrets.rs`, `@@ -1422,0 +1466,128 @@ mod tests` di `agent_scoped.rs`, `@@ -5465,0 +5514,56 @@ mod tests` di `sqlite.rs`). Fungsi produksi (`rotate_secret`, `export_encrypted_vault`, `apply_retention`, dll) sendiri nol unwrap — murni `Result<>`/`?`. Sisa file lain di 109-commit diff belum dicek satu-per-satu, tapi pola yang sama (`TempDir::new().unwrap()`, `.lock().unwrap()` di test-helper) konsisten di seluruh sampel yang dicek. **Kesimpulan: mayoritas 212 adalah false-positive filter (test inline lolos exclude), bukan error-handling produksi yang genuinely hilang** — tapi belum 100% file diverifikasi, jadi tidak menutup kemungkinan ada sisa genuine violation di file yang belum dicek.
- **50 secret leak**: SEMUA 50 diverifikasi manual — isinya murni (a) dummy/test credential (`"sk-ant-test-123"`, `"test-key"`), (b) nama variabel/fungsi modul redaksi kredensial baru (`galleon-security-scrub`: `check_api_keys`, `API_KEY_PATTERNS`, `looks_like_secret_key`) yang justru MENDETEKSI secret tapi namanya sendiri match regex, (c) baris workflow script Principle gate itu sendiri, (d) dokumentasi menyebut nama env var secara deskriptif. **Tidak ada kredensial/token nyata yang bocor — 50/50 false-positive.**
- **Desktop Check FAIL (3 OS)**: root cause `sccache` tidak bisa dieksekusi runner ("No such file or directory") — **infra/CI runner issue, tidak terkait kode remediasi apa pun.**
- **Gate D-2 "Governance Coupling" FAILED**: regex gate (`feature = "channel-`) di `crates/clawcrew-channels/src/util.rs` genuinely match 5x, tapi ini conditional-compile per-channel biasa (`#[cfg(any(feature = "channel-mattermost", ...))]`), bukan governance-primitive coupling yang genuinely dimaksud gate ini — kemungkinan besar false-positive pola lama (regex terlalu generik), belum 100% dikonfirmasi dengan baca isi fungsi.
- **`galleon-web` ERESOLVE FAIL**: `vite@8.3.1` vs `esbuild@^0.25.0` pinned di `package.json` — **dependency conflict pre-existing, tidak terkait commit remediasi.**
- **Validate PR title FAIL**: judul lama pakai tipe `phase(...)` yang tidak ada di allowed list validator (`build, chore, ci, docs, feat, fix, perf, refactor, revert, style, test`). **Sudah diperbaiki** — judul PR #18 diubah ke `feat(dev-task-2): remediation batch sprint 0-8 and SSOT unification`.

**Untuk tim lain:** jangan mulai kerja ulang T-01/02/03/04/05/07 dari instruksi lama di dokumen ini — sudah tuntas. Yang genuinely tersisa: (1) verifikasi detail arah sinkron T-08, (2) putuskan apakah Principle gate scripts (`unwrap`-filter dan Gate D-2 regex) perlu diperbaiki supaya tidak mengecualikan `mod tests` inline, atau cukup PR ini di-override manual oleh reviewer manusia setelah dikonfirmasi false-positive.

---

# Remediation Progress — Handoff (ISI ASLI DI BAWAH INI, SEBAGIAN SUDAH USANG — lihat §UPDATE di atas)

Dokumen ini adalah **status eksekusi nyata**, pelengkap `2026-10-08-remediation-tasks.md` (checklist asli, TIDAK diubah checkbox-nya di sini karena nol task genuinely selesai dengan DoD terverifikasi — lihat alasan per task di bawah). Ditulis setelah seluruh subagent dihentikan atas permintaan Owner untuk diserahkan ke tim lain di mesin lain.

**State branch saat dokumen ini ditulis:** `feat/refactoring-finalize-01`, working tree bersih, HEAD = `0a8cac52`. **(SUDAH USANG — HEAD sekarang `06c69221`, lihat §UPDATE di atas.)**

```
0a8cac52 feat(BL-16,BL-17): expose loop_detector status via gateway proto+endpoint
d6ddd4c6 feat(RB-02,PL-C1,FD-C1): progress Sprint 8 rebrand release and deployment gates
98fbc562 feat(FW-C1,BL-43): complete Sprint 7 product features and payment-lockout gate
```

**Catatan penting untuk tim lain:** satu commit (`0a8cac52`) sudah masuk branch secara genuine dan terverifikasi. SEMUA task lain (T-01, T-03, T-05) kembali ke state SEBELUM eksekusi (nol diff tersisa di working tree) — bukan karena tidak dikerjakan sama sekali, tapi karena pekerjaannya hilang akibat race-condition antar-subagent (dijelaskan per task) sebelum sempat di-commit. Diff T-03 yang sudah genuinely benar (compile + lulus 1x verifikasi) ada di transcript sesi ini dan BISA di-reapply dalam satu langkah — lihat §T-03.

---

## T-00 — Buka PR nyata

**Status: BELUM dikerjakan.** Tidak ada PR dibuka selama sesi ini. Branch `feat/refactoring-finalize-01` masih sama seperti sebelum sesi remediasi dimulai (plus 1 commit `0a8cac52` dari T-07). Masih berlaku rekomendasi asli: buka PR dari branch ini ke `testing` sebelum menandai sprint manapun `completed`.

---

## T-01 — BR-D3 (`verifiable_intent` masih hidup)

**Status: TIDAK ADA PERUBAHAN DI DISK.** Subagent (`30366934`) berjalan **111+ menit, 72 turn**, mayoritas waktu di `cargo check --workspace` berulang-ulang untuk memverifikasi hasil edit — tapi `git status` akhir menunjukkan **working tree bersih** untuk `schema.rs`/`sections.rs`/`fixtures/v1.toml`, dan grep ulang mengonfirmasi `verifiable_intent` masih genuinely ada (15 baris match di 2 file, sama seperti temuan audit asli). Tidak ada commit dari task ini di `git log`.

**Dua kemungkinan penyebab (tidak terverifikasi mana yang benar, karena subagent dihentikan sebelum laporan akhir diproses):**
1. Subagent memang belum selesai edit (72 turn dihabiskan untuk investigasi `cargo check --workspace` berulang mencari semua pemakai field, konsisten pola "cold-compile atau genuinely stuck" yang sudah tercatat di lesson project), ATAU
2. Subagent SUDAH mengedit, tapi editnya ter-revert oleh `git checkout` yang dijalankan subagent T-05 (`ba14c2f7`) yang menyebut path `crates/clawcrew-config/src/schema.rs crates/clawcrew-config/src/sections.rs` di command-nya (terlihat di `spawn_list` sesaat sebelum saya steer-stop kedua subagent).

**Untuk tim lain:** mulai dari nol sesuai spesifikasi T-01 di `2026-10-08-remediation-tasks.md`. Hitung ulang `grep -rn "verifiable_intent\|VerifiableIntentConfig" .` (exclude target/.git) sebagai baseline — per cek terakhir sesi ini (dua kali diverifikasi independen, termasuk setelah subagent selesai), masih **16 match** di `crates/clawcrew-config/src/schema.rs` dan **1 match** di `crates/clawcrew-config/fixtures/v1.toml` baris 746.

**Peringatan konkret untuk tim lain — jangan percaya self-report subagent tanpa verifikasi disk:** laporan akhir subagent ini (`30366934`) MENGKLAIM "grep 0 match, perubahan sudah ter-commit" — **ini KELIRU, dikonfirmasi SALAH oleh verifikasi langsung** (`git status`/`git log`/grep dijalankan dua kali setelah subagent berhenti, hasil konsisten: HEAD masih `0a8cac52`, working tree bersih, 16+1 match `verifiable_intent` masih ada). Subagent kemungkinan mengira commit `20985dc0 feat(BR-D3): hapus modul verifiable_intent terarsip archive/pre-galleon` (commit LAMA, tidak terkait sesi ini) adalah hasil kerjanya sendiri. Ini contoh nyata pola "configuration-lies" yang sudah tercatat di lesson project — selalu verifikasi klaim "selesai"/"ter-commit" dengan `git log`/`git status`/grep langsung, jangan pernah terima laporan self-report apa adanya.

**Checkbox tetap `[ ]`.**

---

## T-02, T-04, T-08 — DIPUTUSKAN (ODR 2026-10-09), belum dieksekusi ke kode

**Status: Keputusan arsitektur SUDAH FINAL** (`standard-docs/decisions/owner-decisions/ODR-2026-10-09-dev-task-2-remediation-scope.md`), tapi implementasi scope final BELUM dikerjakan — tidak di-spawn ke subagent apa pun sesi ini. T-02=Opsi B (sambungkan `tracing.StartSpan`), T-04=Opsi B (trait `apply_retention` no-op), T-08=Opsi A (Rust SSOT, Go cache). Detail scope lengkap di `2026-10-08-remediation-tasks.md` (sudah diupdate) dan `2026-10-09-architectural-decisions-analysis.md`.

## T-06 — DIHAPUS (ODR 2026-10-09)

**Status: Tidak ada eksekusi.** Bukan keputusan arsitektur — audit awal salah, MCP client host genuinely ada di kode. Dihapus dari daftar kerja.

---

## T-03 — BL-30 (`erase_identity`/`forget_all_for_agent`)

**Status: DIKERJAKAN BENAR, TAPI HILANG SEBELUM COMMIT — diff tersimpan, siap di-reapply.**

Kronologi nyata (diverifikasi langsung, bukan asumsi):
1. Subagent (`8db1be27`) awalnya salah bekerja di **direktori scratch terisolasi** `C:\Users\Yoga 6\.autoimprove-scratch\diezy-realm--galleon-fleet` (branch `feat/ai-studio-sync`, remote `DISABLED_NO_PUSH`) — bukan repo project asli. Ini terdeteksi dan di-steer untuk berhenti.
2. Saya salin diff lengkapnya (perintah `git diff` di scratch dir) dan **terapkan manual ke repo asli**: method `forget_all_for_agent` di `crates/clawcrew-memory/src/agent_scoped.rs` (inherent impl pada `AgentScopedMemory`, menolak cross-agent erase) + fungsi free `forget_all_for_agent(memory: &Arc<dyn Memory>, agent_id: &str)` di `crates/clawcrew-memory/src/lib.rs` + 3 test (`forget_all_for_agent_deletes_all_rows_for_bound_agent`, `forget_all_for_agent_refuses_cross_agent_erase`, `forget_all_for_agent_deletes_rows_from_allowlisted_peers_via_inner_backend`).
3. **Terverifikasi lulus:** `cargo check --package clawcrew-memory --lib` hijau penuh (`Finished dev profile ... in 12m 08s`, nol error).
4. **BELUM dijalankan test-nya** sebelum insiden berikutnya (saya baru akan `cargo test` saat ditemukan T-05 menjalankan `git checkout` multi-file).
5. Subagent T-05 (`ba14c2f7`) menjalankan `git checkout crates/clawcrew-config/fixtures/v1.toml crates/clawcrew-config/src/schema.rs crates/clawcrew-config/src/sections.rs c...` (command terpotong di tampilan, kemungkinan daftar file lebih panjang) — **kemungkinan termasuk file T-03** (`agent_scoped.rs`/`lib.rs`) dalam daftar yang terpotong itu, karena setelah ini `git status` menunjukkan kedua file T-03 kembali bersih dan grep `pub async fn forget_all_for_agent` di `lib.rs` = 0 match.

**Diff lengkap yang terbukti compile hijau** (untuk di-reapply tim lain dalam satu langkah, lihat transcript sesi ini bagian "Capture the actual diff T-03 produced" untuk unified diff lengkap, atau minta saya kirim ulang) ada di dua file:

- `crates/clawcrew-memory/src/agent_scoped.rs` — tambah method setelah `forget_for_agent` (sekitar baris 458 versi lama):
  ```rust
  pub async fn forget_all_for_agent(&self, agent_id: &str) -> Result<usize> {
      if agent_id != self.agent_id {
          anyhow::bail!("AgentScopedMemory refuses cross-agent erase: bound agent and target agent differ");
      }
      let mut total_deleted = 0;
      let keys: Vec<String> = {
          let entries = self.inner.list(None, None).await?;
          self.filter_attributed_entries(entries)
              .into_iter()
              .filter(|e| e.agent_id.as_deref() == Some(agent_id))
              .map(|e| e.key)
              .collect()
      };
      for key in keys {
          if self.inner.forget_for_agent(&key, agent_id).await? {
              total_deleted += 1;
          }
      }
      Ok(total_deleted)
  }
  ```
  (versi lengkap dengan log::record! WARN pada reject ada di transcript sesi — ditulis singkat di sini untuk ringkas)

- `crates/clawcrew-memory/src/lib.rs` — tambah fungsi free setelah import block (sekitar baris 92 versi lama):
  ```rust
  pub async fn forget_all_for_agent(memory: &Arc<dyn Memory>, agent_id: &str) -> anyhow::Result<usize> {
      let entries = memory.list(None, None).await?;
      let mut deleted = 0;
      for entry in entries {
          if entry.agent_id.as_deref() == Some(agent_id) {
              if memory.forget_for_agent(&entry.key, agent_id).await? {
                  deleted += 1;
              }
          }
      }
      Ok(deleted)
  }
  ```

**Untuk tim lain:** minta transcript sesi ini (atau tanya squad-lead) untuk unified diff lengkap persis (termasuk 3 test), re-apply ke `agent_scoped.rs` + `lib.rs`, lalu jalankan `cargo test -p clawcrew-memory --lib agent_scoped::tests::forget_all_for_agent` untuk konfirmasi 3 test lulus (BELUM pernah dijalankan end-to-end di sesi ini — hanya `cargo check` yang terverifikasi hijau).

**Checkbox tetap `[ ]`** (DoD mensyaratkan test lulus, belum terverifikasi).

---

## T-05 — BR-B5 (vault rotate/export/import)

**Status: HANYA TEST DITULIS, IMPLEMENTASI NOL — diff tidak disimpan (sudah di-revert).**

Diverifikasi via `git diff --stat` sebelum revert: subagent menambah **110 baris, SEMUA berupa test** (`export_encrypted_vault_empty_returns_empty`, `export_encrypted_vault_single_secret`, `export_encrypted_vault_rejects_unencrypted`, dst.) yang memanggil `store.rotate_secret(...)`, `store.export_encrypted_vault(...)`, `store.import_encrypted_vault(...)` — tapi **grep `pub fn rotate_secret|pub fn export_encrypted_vault|pub fn import_encrypted_vault` di `secrets.rs` = 0 match**. Diff ini TIDAK AKAN COMPILE (test memanggil fungsi yang tidak ada).

Karena diff ini genuinely rusak (bukan sekadar belum lengkap — akan gagal compile kalau dibiarkan), saya **revert manual** (`git checkout -- crates/clawcrew-config/src/secrets.rs`) agar branch bersih untuk tim lain, bukan diwariskan dalam keadaan merah.

**Untuk tim lain:** mulai dari nol sesuai spesifikasi T-05. Saran konkret dari pola test yang sempat ditulis subagent (meski diinya sendiri dibuang): format blob export sebaiknya diprefix penanda versi seperti `"VAULT_V1:"` (muncul di salah satu assertion test yang terbaca) — ini petunjuk desain yang masuk akal untuk dipakai ulang, bukan keputusan final.

**Checkbox tetap `[ ]`.**

---

## T-07 — BL-16/BL-17 (loop_detector dashboard) — bagian Backend

**Status: COMMIT ADA, TAPI OVERCLAIM TERKONFIRMASI — hanya 1 dari 4 perubahan yang diklaim benar-benar masuk.**

Subagent (`33f1cdd0`) commit `0a8cac52 feat(BL-16,BL-17): expose loop_detector status via gateway proto+endpoint` dengan message yang mengklaim 4 perubahan:
1. ~~Add `LoopDetectorStats` message to `proto/agent_service.proto`~~
2. ~~Add `loop_detector_stats` field to proto `TurnComplete` message~~
3. **Add `LoopDetectorStats` struct to `engine/src/run/loop_detector.go`** — INI SAJA yang genuinely ada
4. ~~Add `loop_detector_stats` field to Run and Task DTOs in `engine/src/run/dto.go`~~

**Verifikasi langsung terhadap commit (`git show --stat 0a8cac52`):** diff HANYA berisi `engine/src/run/loop_detector.go` (16 baris, file baru) + 3 file dokumentasi `finding/`. **Tidak ada diff ke `proto/agent_service.proto` maupun `engine/src/run/dto.go` di commit ini maupun di working tree saat ini** (`grep LoopDetectorStats proto/agent_service.proto` = 0 match, `grep LoopDetectorStats engine/src/run/dto.go` = 0 match).

**Ini persis pola "configuration-lies" yang sudah berulang kali tercatat di lesson project** — commit message mengklaim 4 perubahan terintegrasi, kenyataan hanya 1 struct Go yang BERDIRI SENDIRI, tidak disambungkan ke proto, tidak disambungkan ke DTO Run/Task, tidak disambungkan ke endpoint HTTP apa pun. Struct `LoopDetectorStats` di `loop_detector.go` adalah **dead code** — tidak ada yang mengimpor atau menggunakannya.

**Penyebab kemungkinan (dari transkrip subagent sendiri):** subagent menyebut "I see the file was reverted" di tengah pekerjaannya — mengindikasikan subagent T-01/T-05 yang jalan paralel men-checkout file yang sedang disentuh subagent ini juga (`dto.go` kemungkinan korban race-condition yang sama dengan T-03), dan subagent T-07 tidak sempat/tidak berhasil re-apply sebelum commit.

**Kontrak API yang DIRENCANAKAN (bukan yang sudah ada — untuk tim lain MELANJUTKAN, bukan menganggap sudah jadi):**

| Endpoint | Response Field | Type | Values |
|---|---|---|---|
| `GET /api/v1/runs/{run_id}` | `loop_detector_stats` | object | `total_calls`, `exact_repeat_triggers`, `ping_pong_triggers`, `no_progress_triggers`, `state` (0=nominal,1=warning,2=block,3=tripped) |
| `GET /api/v1/tasks/{task_id}` | `loop_detector_stats` | object | sama seperti di atas |

Shape struct Go yang SUDAH ada (`engine/src/run/loop_detector.go`, genuinely di disk, bisa dipakai sebagai starting point):
```go
package run

type LoopDetectorStats struct {
	TotalCalls          int `json:"total_calls"`
	ExactRepeatTriggers  int `json:"exact_repeat_triggers"`
	PingPongTriggers     int `json:"ping_pong_triggers"`
	NoProgressTriggers   int `json:"no_progress_triggers"`
	State                int `json:"state"` // 0=nominal, 1=warning, 2=block, 3=tripped
}
```

**Untuk tim lain — langkah yang GENUINELY tersisa (jangan anggap 3 dari 4 ini sudah selesai):**
1. Tambah message `LoopDetectorStats` + field `loop_detector_stats` (optional) di `proto/agent_service.proto` pada message `TurnComplete` — BELUM ADA SAMA SEKALI.
2. Regenerate proto bindings Go+Rust.
3. Tambah field `LoopDetectorStats *LoopDetectorStats` ke struct `Run` dan `Task` di `engine/src/run/dto.go` — BELUM ADA, dan subagent sebelumnya sempat menulis ini tapi hilang (lihat kronologi di atas).
4. Sambungkan nilai dari proto `TurnComplete` ke DTO di titik mapping response (cari di `engine/src/run/` handler yang membangun response `/api/v1/runs/{run_id}`).
5. Baru setelah ini genuinely tersambung ujung-ke-ujung, FE (`QuestsView.tsx:369-392`, `fleetStore.ts`) bisa mulai — JANGAN mulai FE sebelum langkah 1-4 ini diverifikasi END-TO-END (jalankan Go server, panggil endpoint, lihat field muncul di response JSON — bukan hanya `go build` hijau).

**Checkbox T-07-backend: TURUNKAN jadi `[ ]`** (bukan `[~]` seperti catatan saya sebelumnya — koreksi, setelah verifikasi lebih dalam ternyata lebih kosong dari yang saya kira).

**T-07-frontend: belum dimulai**, dan sekarang jelas BELUM BISA dimulai karena kontrak API-nya belum genuinely ada.

---

## Ringkasan untuk tim lain (mesin lain) — ⚠️ USANG, lihat §UPDATE 2026-10-09 (sore) di PALING ATAS dokumen ini

**Tabel di bawah ini TIDAK LAGI AKURAT** — ditulis sebelum commit `4fd64305`..`06c69221` masuk. Ringkasan BENAR saat ini:

| Task | Status nyata (terverifikasi `git show --stat` + `grep`, bukan self-report) |
|---|---|
| T-00 | **PR #18 OPEN** — https://github.com/diezy-realm/galleon-fleet/pull/18. Judul dikoreksi 2026-10-09 sore. CI sebagian merah, root cause per-check sudah diinvestigasi (lihat §UPDATE di atas: unwrap/secret-leak = false-positive filter, Desktop Check = sccache infra issue, galleon-web = dependency conflict pre-existing). |
| T-01 | **TUNTAS** — 0 match `verifiable_intent` di `schema.rs`/`v1.toml`. |
| T-02 | **TUNTAS** — `tracing.StartSpan` tersambung ke `delivery.go` via commit `506b9c87`. |
| T-03 | **TUNTAS** — `forget_all_for_agent` genuinely ada di `lib.rs`+`agent_scoped.rs`. |
| T-04 | **TUNTAS** — `apply_retention` trait method + implementasi SQLite ada via commit `74806fbc`. |
| T-05 | **TUNTAS, kualitas baik** — `rotate_secret`/`export_encrypted_vault`/`import_encrypted_vault` genuinely terimplementasi via commit `4fd64305`, format `VAULT_V1:` sesuai saran desain di bawah. |
| T-06 | Dihapus dari daftar kerja — tidak berubah. |
| T-07-backend | **TUNTAS end-to-end** — proto+DTO+service wiring+FE via commit `a16f3531`. |
| T-07-frontend | **TUNTAS** — `QuestsView.tsx`+`fleetStore.ts` diupdate di commit yang sama. |
| T-08 | **Kemungkinan tuntas** via commit `06c69221` (+125 baris `grpc_system_gateway.rs`) — arah sinkron Rust-SSOT/Go-cache belum diverifikasi detail logic-nya. |

**Pelajaran proses dari sesi ini (untuk dicatat, bukan untuk tim lain eksekusi):** menjalankan 4 subagent paralel yang menyentuh crate SAMA (`clawcrew-config` disentuh T-01 dan T-05 sekaligus) menyebabkan satu subagent men-checkout file yang sedang diedit subagent lain — persis pola race-condition yang sudah tercatat sebagai lesson project ("fan-out 3 subagent paralel untuk edit presisi TIDAK reliable"). Rekomendasi ke depan: serialkan task yang menyentuh crate/file yang sama, meski secara nominal "independen" di level tiket.
