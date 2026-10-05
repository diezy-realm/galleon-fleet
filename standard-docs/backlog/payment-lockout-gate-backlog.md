---
title: Backlog — Payment Lockout Gate (Pencegahan Agent Melakukan Pembayaran)
status: backlog
date: 2026-10-05
author: squad-lead
depends_on: decisions/payment-lockout-gate-scheme.md
---

# Backlog — Payment Lockout Gate

> Diturunkan dari `decisions/payment-lockout-gate-scheme.md` atas permintaan Achmad. Nomor lanjutan dari BL-39 (`backlog/channel-cicd-observability-backlog.md`). **Preventif** — tidak ada tool pembayaran di codebase saat ini; tujuan backlog ini memasang gate SEBELUM kapabilitas semacam itu pernah dibangun.

---

## BL-40 — `PaymentLockConfig` di `clawcrew-config` Schema (P1, MEDIUM-importance/LOW-urgency)

**Rujukan desain:** §3.1 (`payment-lockout-gate-scheme.md`)

**Scope:** Tambah `PaymentLockConfig`/`PaymentLockMode`/`PaymentActionCategory` ke `crates/clawcrew-config/src/schema.rs`, mengikuti pola `EstopConfig`/`OtpConfig` yang sudah ada di file yang sama. Default pabrik: `mode: Locked`, `unlocked_scopes: []`, `require_otp_to_unlock: true`, `unlock_expires_at: None` (wajib diisi eksplisit saat unlock, lihat §3.1), `covered_categories: [semua 5 kategori]`.

**File ownership:** Backend-Rust — `crates/clawcrew-config/src/schema.rs`.

**Definition of Done:** Struct baru ter-serialize/deserialize dengan default fail-closed; test baru membuktikan `PaymentLockConfig::default().mode == PaymentLockMode::Locked`; test tambahan membuktikan unlock tanpa `unlock_expires_at` eksplisit ditolak di layer config/validasi (pola time-boxed authorization selaras Stripe SPT/Plaid VRP/AP2 `exp` claim — lihat riset industri).

**Dependency:** Tidak ada — isolated, scope kecil.

**Risiko/Severity:** LOW — field baru opsional, backward-compatible (`#[serde(default)]`).

---

## BL-41 — `PaymentLockManager` + Titik Gate di `tool_execution.rs` (P1, MEDIUM)

**Rujukan desain:** §3.2

**Scope:** Buat `crates/clawcrew-runtime/src/security/payment_lock.rs` (BARU) — `PaymentLockManager` dengan `check_allowed(category, scope)`, pola persisten sama seperti `EstopManager` (atomic write, 0o600 Unix). Tambah titik panggil di `agent/tool_execution.rs` SEBELUM eksekusi tool apa pun yang `payment_category.is_some()` — dicek LEBIH DULU sebelum jalur approval normal (`Playbook::requires_approval`).

**File ownership:** Backend-Rust — `crates/clawcrew-runtime/src/security/payment_lock.rs` (baru), `crates/clawcrew-runtime/src/agent/tool_execution.rs` (titik panggil).

**Definition of Done:** Test: tool dengan `payment_category: Some(_)` DITOLAK saat `mode: Locked` (default) dengan error terstruktur `PaymentLockDenied::GloballyLocked`; DITERIMA hanya setelah `mode: Unlocked` DAN scope cocok di `unlocked_scopes` DAN belum lewat `unlock_expires_at`; tool DITOLAK dengan `PaymentLockDenied::UnlockExpired` setelah waktu kedaluwarsa lewat meski `mode` masih tercatat `Unlocked` (fail-closed otomatis tanpa perlu Owner ingat lock manual); kategori di `covered_categories` yang di-opt-out lolos ke jalur approval normal.

**Dependency:** Setelah BL-40 (butuh `PaymentLockConfig`).

**Risiko/Severity:** MEDIUM — titik integrasi ke jalur eksekusi tool existing, perlu pastikan tidak mengganggu tool non-payment (gate hanya aktif untuk tool dengan `payment_category.is_some()`).

---

## BL-42 — Field `payment_category` di `ToolManifest` + Fail-Closed di Waktu Registrasi (P1, MEDIUM-HIGH)

**Rujukan desain:** §3.3

**Scope:** Tambah `payment_category: Option<PaymentActionCategory>` ke struct manifest tool (lokasi tepat perlu dikonfirmasi — kemungkinan `clawcrew-tools` atau titik registrasi di `clawcrew-runtime/src/tools/mod.rs`). Tambah pemeriksaan di waktu REGISTRASI tool: heuristik kata kunci finansial (`pay`, `transfer`, `charge`, `wallet`, `crypto`, dst, daftar dikonfigurasi di `clawcrew-config`) pada nama/deskripsi tool TANPA `payment_category` terisi → registrasi DITOLAK.

**File ownership:** Backend-Rust — titik definisi `ToolManifest` (perlu `grep` lokasi persis saat eksekusi, belum dipastikan di riset ini) + `clawcrew-config` untuk daftar kata kunci.

**Definition of Done:** Test: tool dummy bernama `transfer_crypto` TANPA `payment_category` gagal registrasi dengan error jelas; tool yang sama DENGAN `payment_category: Some(CryptoTransaction)` berhasil registrasi (lanjut ke gate BL-41 saat dipanggil).

**Dependency:** Independen dari BL-41 (bisa paralel, file berbeda), tapi secara LOGIKA melengkapi BL-41 (mencegah tool payment baru lolos TANPA metadata, bukan hanya memblokir yang sudah bermetadata).

**Risiko/Severity:** MEDIUM-HIGH — ini adalah lapisan fail-closed PALING penting (mencegah lupa-tag jadi lubang), tapi heuristik kata kunci berisiko false-positive untuk tool yang namanya kebetulan mengandung kata serupa (perlu daftar kata kunci yang hati-hati, bukan terlalu agresif).

---

## BL-43 — UI Settings "Allow Agents to Make Payments" (P2, MEDIUM)

**Rujukan desain:** §3.5 — ini "1 setting tambahan" yang diminta Achmad secara eksplisit.

**Scope:** Toggle tunggal di `web/src/components/features/SettingsView.tsx` — default OFF, visual warning saat ON. Scope picker (Fleet/Ship mana yang di-unlock) muncul wajib diisi saat toggle ON — tidak ada default "semua". Input kedaluwarsa (`unlock_expires_at`) WAJIB diisi saat unlock, tidak ada opsi "tanpa batas waktu" tanpa pilihan sadar (pola time-boxed konsisten riset industri Stripe SPT/Plaid VRP/AP2). Sub-opsi 5 kategori (`PaymentActionCategory`) muncul HANYA setelah master toggle ON. Badge status payment-lock di dashboard utama (pola serupa indikator E-Stop yang sudah ada) WAJIB menampilkan nilai config MENTAH (mode/scope/kategori/expiry persis) — tidak ada ringkasan yang digenerate agent (OWASP ASI09, lihat riset industri).

**File ownership:** Frontend — `web/src/components/features/SettingsView.tsx`, kemungkinan komponen badge baru di layout dashboard.

**Definition of Done:** Toggle+scope picker+sub-kategori berfungsi end-to-end lewat API yang expose state BL-40/41 (read+write); badge status terlihat di luar halaman Settings; snapshot test untuk kondisi default (OFF, semua terkunci).

**Dependency:** Setelah BL-40/BL-41 selesai (butuh API/state untuk di-bind).

**Risiko/Severity:** MEDIUM — murni UI, tapi kejelasan visual KRITIKAL (ini satu-satunya permukaan yang Achmad akan lihat untuk memastikan payment benar-benar terkunci).

---

## BL-44 — Audit Log untuk Perubahan Mode Locked→Unlocked (P2, LOW-MEDIUM)

**Rujukan desain:** §3.4

**Scope:** Setiap perubahan `Locked→Unlocked` wajib tercatat dengan alasan (field `why` wajib diisi), lewat jalur audit yang sudah ada (`memory_audit` atau setara — perlu konfirmasi titik integrasi saat eksekusi). `Unlocked→Locked` TIDAK butuh friksi/alasan wajib (mengunci lebih ketat tidak boleh ada barrier).

**File ownership:** Backend-Rust — titik mana pun `PaymentLockManager::set_mode()` dipanggil, plus integrasi ke sistem audit existing.

**Definition of Done:** Test: `Locked→Unlocked` tanpa alasan diisi DITOLAK; `Locked→Unlocked` dengan alasan tercatat di audit trail dengan timestamp+actor; `Unlocked→Locked` berhasil tanpa field alasan.

**Dependency:** Setelah BL-41 (butuh `PaymentLockManager` ada).

**Risiko/Severity:** LOW-MEDIUM — melengkapi kontrol, bukan jalur kritis fungsional.

---

## Urutan Eksekusi yang Disarankan

1. **BL-40** dulu — fondasi config, tidak ada dependency.
2. **BL-41** dan **BL-42** paralel setelah BL-40 — keduanya melengkapi (gate eksekusi + fail-closed registrasi), file berbeda.
3. **BL-43** setelah BL-40/41 — butuh API/state untuk di-bind ke UI.
4. **BL-44** paralel dengan BL-43 — keduanya independen satu sama lain, keduanya butuh BL-41.

## Catatan Proses

**Severity keseluruhan backlog ini: MEDIUM-importance, LOW-urgency** — tidak ada eksploitasi aktif mungkin terjadi hari ini (nol tool pembayaran eksis), tapi dampak jika TIDAK dipasang sebelum tool pembayaran pertama dibangun adalah migrasi besar + jendela risiko nyata. Rekomendasi: kerjakan setelah BL-32 (bug nyata, HIGH urgency) tapi SEBELUM/BERSAMAAN BL-06..09 (SOP/RACI, bagian dari gate kelengkapan "Bet C" yang diputuskan Achmad) — payment-lock adalah guardrail yang murah dipasang sekarang, mahal diretrofit nanti.

**Validasi industri (2026-10-05)**: riset pembanding (`research/payment-lockout-gate-industry-research.md`) mengonfirmasi desain ini SELARAS dengan praktik terbaik yang sudah terbukti — bukan rekayasa internal yang terisolasi. Insiden nyata Comet (checkout tanpa konfirmasi manusia) dan Replit (mengabaikan code-freeze eksplisit) membuktikan ketakutan Achmad bukan hipotetis. Field `unlock_expires_at` (BL-40/41) dan prinsip "state mentah bukan ringkasan agent" (BL-43) adalah penguatan langsung dari temuan riset ini, bukan scope creep.

Setiap item perlu diangkat ke `roadmap/*.md` sebelum di-assign ke squad, mengikuti pola backlog lain di folder ini.
