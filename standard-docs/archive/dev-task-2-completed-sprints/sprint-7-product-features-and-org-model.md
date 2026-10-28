---
sprint: 7
title: Product Features (FE Organization/Approval UI) & Payment-Lockout Preventif
status: completed
depends_on: Sprint 4 §E (BG-F1/F3 org model), Sprint 1 §A (BG-B4 untuk FW-C5)
---

# Sprint 7 — Product Features & Payment-Lockout Preventif

## A. Fitur Produk FE (dep org model Sprint 4)

- [x] **FW-C1 · U-13 · PR-72 (P2, L)** Layar organisasi: Quartermaster/Ship/Squad/Crew, memori, approval, audit/policy read-only. Dep: BG-F1, BG-F3 (Sprint 4).

#### Detail Eksekusi — FW-C1

**File & simbol spesifik:** `web/src/components/features/` — direktori ini sudah berisi view FE existing (`EngineRoomView.tsx`, `CaptainsJournalView.tsx`, `QuestsView.tsx`, `ApprovalPage.tsx`, `FleetCodeView.tsx` — terverifikasi ada). Komponen "layar organisasi" baru kemungkinan ditambahkan di direktori yang sama, pola nama `*View.tsx`.

**Dependency:** BG-F1 (Sprint 4, model organisasi Go harus ada — endpoint data), BG-F3 (Sprint 4, audit trail read-only endpoint).

**Langkah pengerjaan:**
1. Tunggu BG-F1/BG-F3 (Sprint 4) selesai — endpoint data harus ada sebelum FE bisa render data nyata (bukan mock).
2. Buat komponen baru (pola `*View.tsx` di `web/src/components/features/`) menampilkan hierarki Fleet→Ship→Squad→Crew dari endpoint BG-F1.
3. Integrasikan memori (BG-B4 Sprint 1 scope-aware), approval (lihat BL-72 §D di bawah — endpoint approval), audit/policy (BG-F3) sebagai READ-ONLY — UI ini TIDAK boleh menulis policy langsung (prinsip `galleon-architecture.md`: Tauri wajib lewat Go).

**JANGAN DIUBAH:** View existing (`ApprovalPage.tsx`, dll.) — ini komponen BARU, bukan refactor yang sudah ada, kecuali BL-72 (integrasi approval nyata) secara eksplisit menyentuh `ApprovalForm.tsx`.

- [x] **FW-C2 · U-14 · PR-98 (P3, M)** i18n & aksesibilitas dasar.

#### Detail Eksekusi — FW-C2

**TIDAK DIVERIFIKASI sesi ini** — i18n setup `web/` (disebut AGENTS.md "Web dashboard text follows the TypeScript `web/src/lib/i18n.ts` contract") belum dibaca detail. **Langkah pertama:** baca `web/src/lib/i18n.ts` untuk pola existing, terapkan ke komponen BARU yang dibangun di sprint ini (FW-C1 dkk) — JANGAN bangun sistem i18n terpisah.

**Dependency:** Setelah FW-C1 dan komponen lain di sprint ini punya teks UI final — i18n lebih bermakna setelah UI stabil.

- [x] **FW-C3 · (ODR-07)** Terminal UI memakai `POST /api/terminal/exec` setelah `BG-A4` (sudah selesai) + `S2-SEC-01` (Sprint 0, approval-identity) — tampilkan prompt approval yang BENAR-BENAR terverifikasi identitas, bukan checkbox kosong.

#### Detail Eksekusi — FW-C3

**File & simbol spesifik:** Endpoint `POST /api/terminal/exec` sudah terverifikasi ada di `engine/src/fleet/delivery.go:47` (`server.RegisterRouteFunc("/api/terminal/exec", h.handleTerminalExec)`) — lihat detail lengkap di Sprint 0 S2-SEC-01. Setelah S2-SEC-01 selesai (middleware `identity.RequireAuthenticated` dipasang di route ini), FE harus menampilkan state principal yang ACTUALLY resolved (nama/identitas penyetuju asli), bukan checkbox `approved: true` kosong seperti pola `ApprovalForm.tsx` saat ini (lihat BL-72 — pola mock serupa).

**Langkah pengerjaan:**
1. Tunggu S2-SEC-01 (Sprint 0) selesai — endpoint sekarang mengharuskan principal terautentikasi.
2. Update komponen terminal UI (cari `terminal`/`Terminal` component di `web/src/components/`) untuk menampilkan identitas principal yang resolved dari response/session, BUKAN input bebas dari user yang mengklik approve.
3. Kalau request terminal ditolak karena principal tidak terautentikasi (401 dari middleware baru), tampilkan pesan error yang jelas — bukan silent fail.

**Dependency:** BG-A4 (sudah selesai, prasyarat terpenuhi), S2-SEC-01 (Sprint 0, HARD — tanpa ini, "approval yang terverifikasi" tidak punya makna di backend).

- [x] **FW-C4 · (G-20)** Halaman perangkat terhubung: daftar perangkat, revoke, QR pairing. Dep: `BG-C2` (Sprint 0).

#### Detail Eksekusi — FW-C4

**Dependency:** BG-C2 (Sprint 0, HARD — endpoint `/api/pairing/*` harus ada; CATATAN dari Sprint 0: BG-C2 scope-nya kemungkinan mendelegasikan storage ke `clawcrew-gateway/src/api_pairing.rs` Rust `DeviceRegistry` existing via Go policy layer — FE di tiket ini konsumsi endpoint Go `/api/pairing/*`, bukan langsung ke Rust).

**Langkah pengerjaan:** Setelah BG-C2 selesai, buat halaman FE (`web/src/components/features/`, pola `*View.tsx`) menampilkan `DeviceInfo` list (id, name, device_type, paired_at, last_seen — field sudah terverifikasi ada di `clawcrew-gateway/src/api_pairing.rs`), tombol revoke, dan flow QR pairing (generate QR dari token sekali-pakai yang di-return BG-C2).

- [x] **FW-C5 · (G-24)** UI persetujuan `MemoryProposal` (learning by consent). Dep: `BG-F4` (Go, lihat §B di bawah).

#### Detail Eksekusi — FW-C5

**Verifikasi:** `MemoryProposal` type dikonfirmasi ADA di `engine/src/memory/proposal.go` (1 ref ditemukan, konsisten klaim dokumen "`MemoryProposal` sudah ada T-3.2 ✔").

**Dependency:** BG-F4 (§B di bawah, hard — alur persetujuan lanjutan harus ada dulu).

**Langkah pengerjaan:** Setelah BG-F4 selesai, buat UI (pola `ApprovalForm.tsx`/`ApprovalPage.tsx` existing sebagai referensi struktur) untuk menampilkan `MemoryProposal` pending, tombol approve/reject/version-rollback.

## B. Learning by Consent & ADR Lanjutan

- [x] **BG-F4 · G-24 · PR-96 (P2, L)** Learning by consent lanjutan: tipe `MemoryProposal` sudah ada (T-3.2 ✔); tinggal alur umpan balik → persetujuan Owner → versi/rollback di atas store baru. Dep: `BG-B4` (Sprint 1).

#### Detail Eksekusi — BG-F4

**File & simbol spesifik:** `engine/src/memory/proposal.go` — baca struct `MemoryProposal` penuh dan method existing (`Approve()`/`Revert()` disebut Sprint 1 BL-01 sebagai lifecycle yang sudah lengkap tapi tidak persisten) sebelum menambah alur baru.

**Dependency:** BG-B4 (Sprint 1, store vector/RAG persisten) — DAN secara implisit BL-01 (Sprint 1, jembatan gRPC persist learned rule ke Rust) karena "versi/rollback di atas STORE BARU" menyiratkan store yang persisten, bukan `inMemoryVectorStore` lama yang hilang saat restart (temuan BL-01).

**Langkah pengerjaan:**
1. Tunggu BG-B4 DAN BL-01 (Sprint 1) — store baru harus persisten dulu sebelum "versi/rollback" bermakna (rollback ke store yang hilang saat restart tidak ada gunanya).
2. Tambah alur umpan-balik: user/Owner memberi feedback ke `MemoryProposal` pending.
3. Tambah versioning: setiap approve menyimpan versi baru, bukan overwrite — memungkinkan rollback ke versi sebelumnya.

**JANGAN DIUBAH:** `MemoryProposal` type existing — tiket ini menambah alur DI ATASNYA, bukan redesign struct.

- [x] **BG-F6 · G-37 · PR-99 (P3, S)** ADR opsional Delegated Authority Chain (rujukan `archive/research-closed-loop/verifiable-intent-assessment.md` §3c — di-upgrade jadi gate WAJIB sebelum desain fitur commerce, bukan lagi opsional, per keputusan Owner 2026-10-05).

#### Detail Eksekusi — BG-F6

**Langkah pengerjaan:** Tulis ADR (dokumen, bukan kode) di `standard-docs/decisions/owner-decisions/` merujuk `archive/research-closed-loop/verifiable-intent-assessment.md` §3c — definisikan Delegated Authority Chain sebagai GATE WAJIB (bukan opsional, sesuai update judul tiket) sebelum desain fitur commerce apa pun dimulai.

**Dependency:** Prasyarat untuk §C (Payment-Lockout Gate) secara konseptual — ADR ini mendefinisikan model otoritas yang BL-40-44 implementasikan.

## C. Payment-Lockout Gate (Preventif — nol tool pembayaran eksis saat ini, pasang gate SEBELUM kapabilitas dibangun)

> Desain+riset sudah lengkap (`decisions/payment-lockout-gate-scheme.md`, diperkuat 4 temuan riset: AP2 v0.2, insiden Comet/Replit, OWASP Agentic ASI03/09/10). Ditunda Owner 2026-10-05 (fokus BL-32 dulu) — sekarang BL-32 sudah di Sprint 0, jadi sprint ini adalah titik eksekusi wajar.

- [x] **BL-40 (MEDIUM-importance/LOW-urgency, tidak ada dependency)** `PaymentLockConfig` schema (termasuk `unlock_expires_at`) di `clawcrew-config`.

#### Detail Eksekusi — BL-40

**Verifikasi:** `PaymentLockConfig` **TIDAK ADA** di kode (0 match) — konfirmasi genuinely baru (desain sudah lengkap di `decisions/payment-lockout-gate-scheme.md`, implementasi belum mulai). **Langkah pertama WAJIB:** baca `decisions/payment-lockout-gate-scheme.md` PENUH untuk spesifikasi schema persis sebelum menulis struct — dokumen desain itu adalah SSOT untuk shape field, bukan diasumsikan dari judul tiket saja.

**File & simbol spesifik:** Tambah ke `crates/clawcrew-config/src/schema.rs` — ikuti pola struct config lain di file ini (lihat `ToolIoPolicy`/field lain yang sudah di-grep Sprint 2).

**Spesifikasi API (pola, verifikasi detail persis dari dokumen desain):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLockConfig {
    /// Default state: locked. Agents cannot invoke payment-category tools
    /// unless explicitly unlocked by the user via Settings UI (BL-43).
    #[serde(default = "default_true")]
    pub locked: bool,
    /// When set, the unlock automatically re-locks after this UNIX timestamp —
    /// prevents an unlock from silently staying open indefinitely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlock_expires_at: Option<u64>,
}
```

**Langkah pengerjaan:**
1. Baca `decisions/payment-lockout-gate-scheme.md` penuh untuk field persis (field di atas adalah MINIMAL dari judul tiket, dokumen desain mungkin punya field tambahan).
2. Tambah struct ke `schema.rs`, default `locked: true` (fail-closed — SESUAI prinsip "nol tool pembayaran eksis saat ini, pasang gate SEBELUM kapabilitas dibangun").

**Dependency:** Tidak ada (tiket pertama di chain BL-41/42/43/44).

- [x] **BL-41 (MEDIUM, SETELAH BL-40)** `PaymentLockManager` + titik gate di `tool_execution.rs` (termasuk expiry check).

#### Detail Eksekusi — BL-41

**Catatan file-overlap (eksplisit di dokumen):** `tool_execution.rs` adalah file YANG SAMA dengan BL-37/BL-38 (Sprint 2) — **WAJIB Sprint 2 sudah merge dulu** (dicatat di §Catatan File-Overlap Lintas-Sprint dokumen ini).

**Spesifikasi API:**
```rust
pub struct PaymentLockManager {
    config: Arc<RwLock<PaymentLockConfig>>, // BL-40
}

impl PaymentLockManager {
    /// Check whether payment-category tools may run RIGHT NOW — accounts
    /// for both the `locked` flag and `unlock_expires_at` (an unlock that
    /// has silently expired behaves as locked, fail-closed).
    pub fn is_unlocked(&self) -> bool {
        let config = self.config.read();
        if config.locked {
            return false;
        }
        match config.unlock_expires_at {
            Some(expires_at) => now_unix() < expires_at,
            None => true, // unlocked tanpa expiry (perlu konfirmasi apakah ini valid state atau harus selalu ada expiry)
        }
    }
}
```

**Titik gate di `tool_execution.rs`:** SEBELUM tool dieksekusi (titik yang sama area dengan BL-37/38 — `execute_one_tool`), cek `tool.payment_category()` (BL-42, di bawah) — kalau tool berkategori payment DAN `!payment_lock_manager.is_unlocked()` → tolak eksekusi, jangan lanjut.

**Dependency:** BL-40 (hard), BL-42 (soft — gate butuh `payment_category` field untuk tahu tool mana yang perlu dicek, tapi BL-42 ditandai "paralel" bukan hard-sequential).

**Edge case:** `unlock_expires_at` sudah lewat tapi `locked` masih `false` — HARUS dianggap locked (expiry adalah fail-safe, bukan opsional).

- [x] **BL-42 (MEDIUM-HIGH, paralel dengan BL-41)** Field `payment_category` di `ToolManifest` + fail-closed waktu registrasi.

#### Detail Eksekusi — BL-42

**Koreksi lokasi:** `ToolManifest` struct ditemukan di `crates/galleon-hardware/src/manifest.rs` — lokasi TAK TERDUGA (hardware crate, bukan `clawcrew-runtime`/`clawcrew-tools` generik). **PERLU VERIFIKASI:** apakah `ToolManifest` di `galleon-hardware` ini genuinely struct yang dimaksud dokumen untuk SEMUA tool (termasuk tool non-hardware), atau ada `ToolManifest` LAIN yang lebih generik di crate berbeda yang belum ditemukan grep sesi ini. **Langkah pertama WAJIB:** baca `galleon-hardware/src/manifest.rs` penuh untuk konteks (kenapa `ToolManifest` di crate hardware), dan `search_symbols` ulang untuk `ToolManifest` dengan cakupan lebih luas sebelum menambah field.

**Langkah pengerjaan:**
1. Verifikasi lokasi struct persis (langkah di atas).
2. Tambah field `payment_category: Option<PaymentCategory>` (enum baru atau string — verifikasi dari dokumen desain `payment-lockout-gate-scheme.md`).
3. **"Fail-closed waktu registrasi"**: saat tool manifest didaftarkan ke registry, kalau tool punya kapabilitas yang BERPOTENSI payment (heuristik apa? perlu dari dokumen desain) TAPI `payment_category` tidak diset → TOLAK registrasi tool itu sepenuhnya, jangan biarkan lolos dengan asumsi "bukan payment tool".

**Dependency:** Paralel dengan BL-41 (sesuai dokumen) — TAPI BL-41 butuh field INI ada untuk gate check-nya bermakna, jadi urutan commit idealnya BL-42 duluan atau bersamaan, bukan BL-41 duluan.

**ASUMSI:** Heuristik "tool berpotensi payment" belum didefinisikan — WAJIB baca dokumen desain `payment-lockout-gate-scheme.md` untuk kriteria persis sebelum implementasi fail-closed check ini.

- [x] **BL-43 (MEDIUM, SETELAH BL-40/41)** UI Settings "Allow Agents to Make Payments" — **state mentah, bukan ringkasan agent** (OWASP ASI09: state harus dari sistem, bukan laporan agent yang bisa salah/menipu).

#### Detail Eksekusi — BL-43

**File & simbol spesifik:** UI baru di `web/src/` Settings page (cari komponen Settings existing dulu — belum diverifikasi lokasi persis). Backend: endpoint GET yang membaca `PaymentLockConfig` LANGSUNG dari config state (BL-40/41), BUKAN endpoint yang bertanya ke agent "apakah kamu locked" (itu yang dimaksud "bukan ringkasan agent" — prinsip OWASP ASI09 di judul tiket).

**Spesifikasi API:** `GET /api/v1/payment-lock/status` → `{ "locked": bool, "unlock_expires_at": number | null }` — dibaca LANGSUNG dari `PaymentLockManager.config` state Rust (via gRPC dari Go kalau endpoint ini di Go, atau langsung HTTP kalau gateway Rust expose langsung — verifikasi arsitektur mana yang tepat sesuai boundary Go/Rust).

**Langkah pengerjaan:**
1. Tunggu BL-40/41 (hard).
2. Tambah endpoint read state mentah (bukan agent-reported).
3. Tambah toggle UI "Allow Agents to Make Payments" yang memanggil endpoint unlock (dengan expiry wajib — field `unlock_expires_at` HARUS diisi UI, tidak boleh unlock permanen tanpa expiry eksplisit dari user).

**JANGAN DIUBAH:** Prinsip "state dari sistem, bukan dari agent" — JANGAN membuat endpoint yang menerima status locked/unlocked dari laporan agent sendiri, itu melanggar OWASP ASI09 eksplisit yang dicatat dokumen.

- [x] **BL-44 (LOW-MEDIUM, paralel dengan BL-43)** Audit log perubahan mode Locked→Unlocked.

#### Detail Eksekusi — BL-44

**Langkah pengerjaan:** Setiap kali `PaymentLockManager` state berubah (BL-41/43), tulis entry audit log (siapa/principal yang mengubah — REUSE `identity.Principal` dari BG-C1 Sprint 0 untuk atribusi, konsisten pola audit lain di project ini) — kapan, dari state apa ke state apa.

**Dependency:** BL-41/43 (titik mutasi state), BG-C1 (Sprint 0, untuk atribusi principal yang mengubah).

## D. Toolbox Parity — KiroCrew vs Galleon (dari `toolbox-parity-backlog.md`, audit 2026-10-05)

> `BL-73` (Quest-loop), `BL-78` (Optimize prompt), dan `BL-79` (Smart rekomendasi top-bar, baru) **TIDAK dimasukkan sebagai tiket `[ ]` aktif di sini** — ketiganya tetap EXPLORATORY di `backlog/toolbox-parity-backlog.md`, menunggu keputusan Owner, sesuai §Catatan Non-Negosiasi poin 4. Dicatat sebagai REFERENSI saja supaya tim tahu metodenya sudah dipertajam (2026-10-05) tanpa disalahartikan sudah siap eksekusi:
> - **BL-78**: metode sekarang "spawn sub-agent Prompt Engineer sebelum turn utama" (bukan rewrite sederhana), hasil editable tidak auto-send, kemungkinan reuse infrastruktur delegasi `clawcrew-runtime/src/subagent/mod.rs` (BL-05, §C di atas). Lihat `research/toolbox-parity-audit.md` §Penajaman Metode untuk detail flow lengkap.
> - **BL-79 (baru)**: top-bar "smart rekomendasi" — post-response tarik kesimpulan + array maks 5 pertanyaan lanjutan (hybrid instruksi-prompt + hard-cap), klik item → teks masuk composer (editable), klik item ke-2+ → gabung separator `", "`. Satu pertanyaan terbuka tersisa: siapa menjalankan tarik-kesimpulan (LLM utama vs sub-agent terpisah) — keputusan ini mempengaruhi apakah BL-78 dan BL-79 berbagi SATU mekanisme delegasi atau dua mekanisme terpisah.

- [x] **BL-74 (LOW-MEDIUM, PALING MURAH, kerjakan duluan)** Hapus context-window placeholder hardcoded (`EngineRoomView.tsx:335` — teks statis "8,192 tokens... 68%" tidak terhubung sesi nyata) — ganti data real atau hapus.

#### Detail Eksekusi — BL-74

**Koreksi baris:** Teks persis "Context window allocated: 8,192 tokens. Local GPU memory at 68% threshold." ditemukan di `web/src/components/features/EngineRoomView.tsx` sebagai VALUE string dalam object log entry (`message: '...'`), BUKAN di baris 335 persis (bergeser sedikit — baca ulang file untuk nomor baris akurat saat eksekusi, konfirmasi dengan grep `"8,192 tokens"` langsung). Entry ini adalah bagian dari array log statis (`id: 'log-5'`, `level: 'warn'`, `source: 'ollama-bridge'`) — ini SIMULASI log event untuk tampilan demo "Engine Room", bukan widget metric terpisah seperti disangka.

**Langkah pengerjaan:**
1. `grep -n "8,192 tokens" web/src/components/features/EngineRoomView.tsx` untuk baris persis.
2. Baca konteks array log statis penuh — ada BERAPA entry log hardcoded serupa (bukan hanya satu "log-5")? Lihat pola `log-4`/`log-6` di sekitarnya yang juga tampak statis.
3. **Keputusan desain:** apakah SELURUH array log ini perlu diganti data real-time (butuh backend log stream), atau HANYA entry context-window yang spesifik ini (karena ini satu-satunya yang mengklaim angka token/memory SPESIFIK yang bisa menyesatkan user mengira itu data nyata)? Dokumen asli hanya menyebut SATU placeholder spesifik — scope minimal adalah menghapus/mengganti entry ini saja, bukan merombak seluruh "Engine Room" log display.
4. Kalau diganti data real: perlu endpoint yang expose context window usage sesi aktif (lihat R-90 Sprint 1, pola serupa `cache_hit_rate` metric).
5. Kalau dihapus: hapus entry log ini dari array statis, JANGAN ganti dengan placeholder lain yang sama-sama palsu.

**JANGAN DIUBAH:** Entry log LAIN yang bersifat genuinely demo/contoh (kalau ada context yang menjelaskan "Engine Room" adalah demo visual, bukan dashboard data nyata) — hanya hapus/ganti yang MENGKLAIM angka spesifik yang terlihat seperti data sesi nyata.

- [x] **BL-72 (MEDIUM, independen)** Sambungkan `ApprovalForm.tsx`/`fleetStore.ts::handleApproval` ke endpoint nyata (`POST /api/v1/tool-executions/{id}/approve|deny`, sudah ada di `approval_gate.go` — saat ini UI mock state lokal).

#### Detail Eksekusi — BL-72

**Koreksi lokasi file:** `approval_gate.go` ADA di `engine/src/tool/approval_gate.go` (BUKAN di `fleet/` — koreksi path dari asumsi awal). `ApprovalForm.tsx` (`web/src/components/ApprovalForm.tsx`) terverifikasi PERSIS mock: `handleSubmit` hanya memanggil `onSubmit?.({ decision, comments })` — prop callback, TIDAK ADA fetch/API call di file ini sama sekali. `fleetStore.ts:122,695` — `handleApproval: (id: string, decision: 'approved' | 'rejected') => void` dideklarasikan di interface store, implementasi baris 695 (BELUM dibaca isinya sesi ini — verifikasi apakah sudah memanggil API atau genuinely state-only).

**File & simbol spesifik:**
- Baca: `engine/src/tool/approval_gate.go` penuh — endpoint persis, request/response shape (dokumen menyebut `POST /api/v1/tool-executions/{id}/approve|deny` — VERIFIKASI path ini match dengan route registration aktual di file ini, bisa jadi path sedikit berbeda).
- Baca: `web/src/store/fleetStore.ts:690-710` (sekitar baris 695) untuk implementasi `handleApproval` saat ini.
- Ubah: `web/src/store/fleetStore.ts` (`handleApproval`), `web/src/components/ApprovalForm.tsx` (`handleSubmit` — hubungkan ke store action yang sudah memanggil API).

**Spesifikasi API (verifikasi shape persis dari `approval_gate.go`):**
```typescript
// fleetStore.ts
handleApproval: async (id: string, decision: 'approved' | 'rejected') => {
  const endpoint = decision === 'approved'
    ? `/api/v1/tool-executions/${id}/approve`
    : `/api/v1/tool-executions/${id}/deny`;
  const response = await apiClient.post(endpoint); // reuse apiClient.ts existing pattern
  if (!response.ok) {
    // handle error — jangan optimistically update UI state sebelum konfirmasi backend
  }
  // update local state SETELAH backend confirms, bukan sebelum (hindari state drift
  // kalau network request gagal setelah UI sudah "optimis" menampilkan approved)
}
```

**Langkah pengerjaan:**
1. Baca `approval_gate.go` untuk endpoint/shape persis.
2. Baca `web/src/utils/apiClient.ts` (sudah terverifikasi ada, 2 ref ditemukan) untuk pola HTTP call existing — REUSE, jangan buat client terpisah.
3. Update `fleetStore.ts::handleApproval` memanggil endpoint nyata.
4. Update `ApprovalForm.tsx` `handleSubmit` untuk await hasil dan handle error (saat ini `onSubmit` dipanggil tanpa menunggu apa pun).

**Acceptance criteria testable:** Klik Approve di UI → network request nyata ke `/api/v1/tool-executions/{id}/approve` terkirim (verifikasi via browser devtools/test mock), state lokal update HANYA setelah response sukses.

**Edge case:** Network request gagal (timeout/500) — UI harus menampilkan error, BUKAN silently tetap menunjukkan "approved" di state lokal.

- [x] **BL-71 (MEDIUM-HIGH, butuh keputusan desain dulu)** Toggle approval-mode per-chat (Normal/Reads/Trust/YOLO ala KiroCrew) terhubung ke `RiskTier`/`RiskClass` API yang sudah ada di `approval_gate.go`/`policy.go`. **Keputusan terbuka**: apakah mode ini dipetakan ke `authority: read_only|draft_only|gated_write` existing di `CrewMember`, atau butuh bentuk baru — jangan tiru UI KiroCrew mentah tanpa cek kesesuaian model.

#### Detail Eksekusi — BL-71

**Verifikasi:** `RiskTier` dikonfirmasi ADA (`engine/src/fleet/terminal_policy.go` — `RiskTierRead`/`RiskTierExecute`/`RiskTierHighRisk`/`RiskTierBlocked`, sudah dibaca penuh di Sprint 0). `policy.go` ADA di `engine/src/tool/policy.go` (5 ref). `RiskClass` **TIDAK ditemukan** sebagai nama persis — mungkin `RiskTier` yang dimaksud dokumen (nama berbeda, konsep sama) atau ada enum lain di `tool/policy.go` yang belum dibaca.

**Langkah pengerjaan (DESAIN dulu, bukan langsung kode):**
1. Baca `engine/src/tool/policy.go` dan `approval_gate.go` penuh — cari `RiskClass` atau konsep setara, dan struct `CrewMember` (field `authority: read_only|draft_only|gated_write` disebut dokumen — verifikasi field ini ada persis).
2. **KEPUTUSAN** (perlu Lead, BUKAN diputuskan sendiri oleh tim eksekusi): apakah toggle Normal/Reads/Trust/YOLO ini:
   - (a) UI-only mapping ke `authority` existing di `CrewMember` (4 toggle → 3 nilai authority, perlu 1 toggle digabung atau authority diperluas), ATAU
   - (b) Dimensi BARU terpisah dari `authority` (toggle ini kontrol approval PER-SESI-CHAT, authority mungkin kontrol permission PER-CREW-MEMBER — beda scope/lifetime).
3. Tulis temuan + rekomendasi SEBELUM implementasi — ini "butuh keputusan desain dulu" PERSIS sesuai label dokumen, jangan tiru UI KiroCrew mentah (lesson: pola ini institusional, bukan styling).

**Dependency:** Keputusan desain BL-71 ini mempengaruhi BL-75/76/77 kalau toggle ini muncul di komponen yang sama.

**ASUMSI:** Saya tidak bisa memutuskan (a) vs (b) tanpa baca `CrewMember` struct penuh dan konteks penggunaan `authority` di lebih banyak tempat — scope verifikasi ini di luar budget sesi ini, WAJIB dilakukan tim eksekusi sebagai LANGKAH PERTAMA.

- [x] **BL-75 (MEDIUM, SETELAH BL-68 §B di bawah)** Tombol "Chat" di `CrewView.tsx`/`SquadsView.tsx`/`ShipsView.tsx` yang membuka sesi Journal terikat entitas.

#### Detail Eksekusi — BL-75

**TIDAK DIVERIFIKASI PENUH** — `CrewView.tsx`/`SquadsView.tsx`/`ShipsView.tsx` belum dikonfirmasi ada di grep sesi ini (FW-C1 di atas menyebut komponen "layar organisasi" BARU — kemungkinan file-file ini ADALAH bagian dari FW-C1, belum dibuat). **Langkah pertama:** cek apakah 3 file ini sudah ada atau akan dibuat sebagai bagian FW-C1 — kalau belum ada, BL-75 depend juga pada FW-C1 (tidak hanya BL-68).

**Dependency:** BL-68 (§E, hard — `JournalSession.projectId?` harus ada untuk "sesi Journal terikat entitas" bermakna), KEMUNGKINAN juga FW-C1 (§A, kalau `CrewView.tsx`/dst belum ada).

- [x] **BL-76 (MEDIUM-HIGH, SETELAH BL-62 §B di bawah)** Autocomplete `@` (mention `Project`) + `/` (slash skill) di composer `CaptainsJournalView.tsx`.

#### Detail Eksekusi — BL-76

**Verifikasi:** `CaptainsJournalView.tsx` dikonfirmasi ADA (`web/src/components/features/CaptainsJournalView.tsx`, 1 ref + test file `__tests__/CaptainsJournalView.test.tsx` 5 ref).

**Dependency:** BL-62 (§E, hard — `Project` entity harus ada untuk `@`-mention Project bermakna).

**Langkah pengerjaan:**
1. Tunggu BL-62 (Project entity).
2. Baca `CaptainsJournalView.tsx` untuk titik composer/input text existing.
3. Tambah autocomplete trigger `@` (daftar `Project` entity) dan `/` (daftar skill — cari sumber skill list, kemungkinan endpoint Go baru atau existing).

- [x] **BL-77 (MEDIUM, MENYATU dengan BL-62/68/69 §B — jangan kerjakan terpisah)** Pindahkan switcher Workspace dari sidebar `QuestsView.tsx` ke bentuk benar: Workspace scoped Journal, Project jadi bar global `ContextBar.tsx` yang BENAR-BENAR dirender (saat ini hanya diimpor, tidak pernah muncul di JSX).

#### Detail Eksekusi — BL-77

**Verifikasi parsial:** `ContextBar.tsx` ADA (`web/src/components/layout/ContextBar.tsx`) dengan import `useFleetStore`, `SettingsCategory`/`NavigationTab` types, `PWAInstallButton` — **TIDAK diverifikasi** klaim "hanya diimpor, tidak pernah muncul di JSX" (perlu cek file yang MENGIMPOR `ContextBar` dan konfirmasi apakah benar-benar dirender `<ContextBar />` di JSX manapun atau genuinely dead import). `workspaceId`/`selectedWorkspace` tersebar di **10 file** (`fleetStore.ts` 9 ref, `QuestsView.tsx` 6 ref, `CaptainsJournalView.tsx` 3 ref, `LiveCanvasPane.tsx`/`QuarterdeckView.tsx`/`QuestBrainstormModal.tsx`/`VoiceQuartermasterModal.tsx` masing-masing 2 ref, `ContextBar.tsx` 2 ref, `types/index.ts` 2 ref, `MissionBoard.tsx` 1 ref) — scope ini LEBIH LUAS dari asumsi "hanya `QuestsView.tsx`" di judul tiket.

**Langkah pertama WAJIB:** `grep -rn "<ContextBar" web/src/` untuk konfirmasi definitif apakah `ContextBar` genuinely tidak pernah di-render (0 match JSX usage) sebelum eksekusi — klaim dokumen ini signifikan (komponen built tapi dead) dan perlu dikonfirmasi, bukan diterima begitu saja.

**Langkah pengerjaan (MENYATU dengan BL-62/68/69, urutan gabungan):**
1. Kerjakan BL-68 dulu (hapus `Quest.workspaceId` vestigial, scope `selectedWorkspace` ke Journal) — ini akan menyentuh BANYAK dari 10 file di atas, bukan hanya `QuestsView.tsx`.
2. Kerjakan BL-62 (Project entity + endpoint git-status).
3. BARU kerjakan BL-77: render `<ContextBar />` di layout yang tepat (App shell level, bukan di dalam `QuestsView.tsx`), isi dengan `Project` (bar global, scope SEMUA halaman) terpisah dari Workspace switcher (scope Journal saja, dari BL-68).
4. Kerjakan BL-69 (dropdown "+ New" di `AppSidebar.tsx`) sebagai bagian akhir rangkaian ini.

**JANGAN DIUBAH:** Jangan kerjakan BL-77 TERPISAH dari BL-68/62/69 — eksplisit di judul tiket "jangan kerjakan terpisah", karena switcher Workspace lama dan `ContextBar` baru akan BERTABRAKAN secara UX kalau tidak dikoordinasikan dalam satu perubahan.

## E. UI Nice-to-Have (dari `ui-nice-to-have-backlog.md`, prasyarat §D di atas)

> BL-62/68/69 sudah dirancang lengkap di `backlog/ui-nice-to-have-backlog.md` dengan urutan eksekusi dan serialisasi file sendiri — rujuk file itu untuk detail. Diringkas di sini karena jadi dependency langsung §D:

- [x] **BL-68 (MEDIUM-HIGH, PRIORITAS)** Hapus `Quest.workspaceId` (vestigial), scope `selectedWorkspace`/`Workspace[]` ke Captain's Journal saja, tambah `JournalSession.projectId?`.

#### Detail Eksekusi — BL-68

**Rujuk dokumen lengkap:** `standard-docs/backlog/ui-nice-to-have-backlog.md` adalah SSOT detail tiket ini — baca PENUH sebelum eksekusi, jangan hanya rujuk ringkasan di sprint file ini.

**Verifikasi scope nyata:** `workspaceId`/`selectedWorkspace` ada di **10 file** (lihat BL-77 di atas untuk daftar lengkap) — "vestigial" (`Quest.workspaceId`) perlu dibedakan dari penggunaan `selectedWorkspace`/`Workspace[]` yang LEGITIMATE (akan di-scope ulang ke Journal, bukan dihapus).

**Langkah pengerjaan:**
1. Baca `ui-nice-to-have-backlog.md` penuh untuk urutan eksekusi/serialisasi file yang sudah dirancang di sana.
2. Hapus field `workspaceId` dari type `Quest` (`web/src/types/index.ts` — 2 ref ditemukan, verifikasi apakah salah satunya adalah definisi `Quest` type).
3. Pastikan `selectedWorkspace`/`Workspace[]` TETAP ADA tapi di-scope ke Captain's Journal (`CaptainsJournalView.tsx`) saja — bukan dihapus total, dipindah scope-nya.
4. Tambah field opsional `JournalSession.projectId?: string` (type baru, siap untuk BL-62 Project entity).

**JANGAN DIUBAH:** Jangan hapus `selectedWorkspace`/`Workspace[]` KESELURUHAN — hanya `Quest.workspaceId` yang vestigial, workspace concept lain tetap valid (hanya pindah scope).

- [x] **BL-62 (MEDIUM-HIGH)** `Project` entity + Go endpoint `GET /api/project/git-status` — wajib lewat policy gate (`terminal_policy.go`), bukan `exec git` langsung dari Tauri.

#### Detail Eksekusi — BL-62

**File & simbol spesifik:** `engine/src/fleet/terminal_policy.go` (sudah dibaca penuh Sprint 0) — `EvaluateTerminalCommand` adalah fungsi policy gate yang HARUS dilalui, bukan di-bypass. Endpoint baru `GET /api/project/git-status` HARUS memanggil `EvaluateTerminalCommand("git status", ...)` atau fungsi serupa SEBELUM menjalankan `exec.Command("git", "status")` — konsisten prinsip `galleon-architecture.md` ("Eksekusi terminal dari Tauri wajib lewat Go untuk penegakan policy").

**Spesifikasi API:**
```go
// engine/src/project/ (package baru) atau engine/src/fleet/ (existing)
func (h *HTTPHandler) handleProjectGitStatus(w http.ResponseWriter, r *http.Request) {
    projectPath := r.URL.Query().Get("path")
    // WAJIB: evaluasi command "git status" via policy gate yang SAMA dengan terminal exec
    riskTier, reason, err := EvaluateTerminalCommand("git status")
    if err != nil || riskTier == RiskTierBlocked {
        http.Error(w, reason, http.StatusForbidden)
        return
    }
    // git status adalah RiskTierRead (sudah terverifikasi di readCommands list Sprint 0:
    // "git status" ADA di daftar readCommands terverifikasi `terminal_policy.go`)
    // ... exec git status, parse output ...
}
```

**Langkah pengerjaan:**
1. Konfirmasi `"git status"` ADA di `readCommands` list (`terminal_policy.go` — SUDAH terverifikasi Sprint 0, baris `readCommands = []string{"git status", "git diff", ...}`) — ini BAGUS, berarti git status sudah RiskTierRead, tidak butuh approval, HANYA butuh LEWAT fungsi evaluasi yang sama (bukan bypass sepenuhnya).
2. Buat `Project` entity (struct Go baru) + endpoint `GET /api/project/git-status`.
3. PASTIKAN endpoint memanggil `EvaluateTerminalCommand` sebelum `exec.Command` — JANGAN langsung exec tanpa lewat gate, meskipun "git status" low-risk, PRINSIP konsistensi (semua command lewat gate yang sama) lebih penting daripada shortcut untuk command yang "kelihatannya aman".
4. FE: ganti `exec git` langsung dari Tauri (kalau ada — cari implementasi existing yang di-bypass) dengan panggilan endpoint Go baru ini.

**JANGAN DIUBAH:** `EvaluateTerminalCommand`/`readCommands` list — reuse langsung, jangan buat gate kedua yang terpisah untuk project git-status.

- [x] **BL-65/66/67 (lihat `ui-nice-to-have-backlog.md`)** Bugfix + grouping visual + tombol kontekstual Captain's Journal, SETELAH BL-68.

#### Detail Eksekusi — BL-65/66/67

**Rujuk dokumen lengkap:** Detail penuh di `standard-docs/backlog/ui-nice-to-have-backlog.md` — TIDAK diringkas ulang di sini sesuai pola dokumen asli ("lihat ui-nice-to-have-backlog.md"), baca file itu untuk AC lengkap per tiket.

**Dependency:** BL-68 (hard, semua tiga tiket).

- [x] **BL-69 (MEDIUM)** Dropdown "+ New" di `AppSidebar.tsx`, SETELAH BL-68.

#### Detail Eksekusi — BL-69

**TIDAK DIVERIFIKASI sesi ini** — `AppSidebar.tsx` belum dikonfirmasi lokasi (grep terakhir Sprint 7 menemukan `AppShell.tsx` 6 ref, BUKAN `AppSidebar.tsx` — nama file mungkin berbeda, verifikasi sebelum eksekusi).

**Dependency:** BL-68 (hard).

## F. GAP-01 — Perancangan Steering Produk Galleon End-User (dari `skills-steering-lessons-backlog.md`, belum pernah masuk sprint manapun sebelum audit 2026-10-05)

> Scope PERANCANGAN (ADR/design-spec), bukan implementasi langsung — keputusan one-way-door format file untuk end-user `galleon`/`galleoncode` CLI. Ditempatkan di sprint ini (bukan Sprint 1) karena menyentuh pengalaman end-user produk jadi, konsisten dengan tema Sprint 7 (fitur produk).

- [x] **BL-02 (MEDIUM, perancangan dulu — implementasi menunggu desain disetujui Owner)** Rancang mekanisme steering produk Galleon end-user setara `~/.kiro/steering/` Kiro Crew (saat ini TIDAK ADA — `standard-docs/` hanya untuk tim developer, bukan end-user produk jadi). 4 keputusan yang perlu dijawab design-spec: (1) lokasi fisik file (`~/.config/galleon/steering/`?), (2) format (markdown bebas vs frontmatter terstruktur), (3) resolusi konflik (workspace-menang-atas-global, pola Kiro Crew), (4) siapa membaca — Go pegang keputusan "steering mana aktif" (kebijakan), Rust `galleoncode` CLI eksekusi baca+suntik ke prompt (I/O teknis). Output: design-spec baru di `standard-docs/decisions/`. File: `web/src/pages/settings/` (opsional, kalau butuh UI toggle) — murni Lead (dokumentasi) di tahap ini, belum ada file kode.

#### Detail Eksekusi — BL-02

**Catatan nama CLI aktual:** `galleoncode` BELUM di-rename (nama aktual `zerocode`, lihat Sprint 6 BR-E1-E4) — design-spec HARUS ditulis dengan kesadaran bahwa implementasi akan menyentuh `apps/zerocode/` (nama saat ini), path final akan `galleoncode` setelah Sprint 8 RB-20.

**Langkah pengerjaan:**
1. Tulis design-spec (`standard-docs/decisions/`) menjawab 4 keputusan di atas — INI MURNI DOKUMEN, bukan kode (sesuai label "perancangan dulu").
2. Untuk keputusan #4 (siapa membaca): rujuk pola `identity.RequireAuthenticated`/`PrincipalResolver` (BG-C1, Sprint 0) sebagai PRESEDEN "Go pegang keputusan, Rust/CLI eksekusi I/O" yang SUDAH established di project ini — konsisten arsitektur, bukan pola baru.
3. **JANGAN implementasi apa pun** sampai design-spec disetujui Owner secara eksplisit (one-way-door keputusan format file).

**JANGAN DIUBAH:** `~/.kiro/steering/` (Kiro Crew sendiri) — ini REFERENSI pola, bukan sistem yang disentuh. Target adalah sistem BARU untuk produk Galleon end-user, terpisah total dari tooling internal Kiro Crew yang dipakai tim dev.

## Catatan File-Overlap Lintas-Sprint

`BL-41`/`BL-42` menyentuh `tool_execution.rs` yang SAMA dengan `BL-37`/`BL-38` (Sprint 2). **Pastikan Sprint 2 sudah merge sebelum Sprint 7 §C mulai** — kalau belum, koordinasikan dengan Lead untuk serialisasi tambahan di luar urutan sprint normal.

## Keluaran Sprint

- Layar organisasi FE hidup, menampilkan hierarki Fleet=Perusahaan/Ship=Departemen/Squad=Tim/Crew=individu secara nyata (bukan mock).
- Payment-lockout gate terpasang SEBELUM ada tool pembayaran nyata dibangun — mencegah kelas insiden Comet/Replit sebelum terjadi di Galleon.
- Context-window placeholder tidak lagi menampilkan data palsu; approval-queue UI tersambung endpoint nyata; Workspace/Project switcher muncul di bentuk yang benar (Journal-scoped + bar global), bukan tersembunyi/salah lokasi.
