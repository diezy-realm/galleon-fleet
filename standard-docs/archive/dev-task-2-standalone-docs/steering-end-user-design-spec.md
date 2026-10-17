# Design Specification — Galleon End-User Steering Mechanism (GAP-01 / BL-02)

> **Status:** Proposed / Ready for Owner Review.
> **ID Tiket:** BL-02 (GAP-01) — Sprint 7 Area F.
> **Referensi:** `skills-steering-lessons-backlog.md`, `~/.kiro/steering/` reference pattern, AGENTS.md, ODR-2026-10-04.

---

## 1. Latar Belakang & Tujuan

Hingga saat ini, repositori Galleon memiliki pedoman developer internal di `standard-docs/` dan `.gemini/steering.md`. Namun, produk jadi Galleon untuk end-user (yang dijalankan via `galleoncode` CLI — saat ini `zerocode` di `apps/zerocode/` sebelum final rebrand Sprint 8 RB-20) **belum memiliki mekanisme steering end-user**.

Di ekosistem Kiro Crew, pengguna dapat menempatkan file steering di `~/.kiro/steering/` untuk mengarahkan perilaku AI secara persisten tanpa memodifikasi kode sumber. Galleon memerlukan mekanisme serupa yang dirancang khusus untuk end-user, dengan tata kelola keamanan dan boundary Go-Rust yang terstandarisasi.

---

## 2. Empat Keputusan Arsitektural Inti

### Keputusan 1: Lokasi Fisik File (Physical Locations)

Galleon membagi hierarki steering menjadi dua cakupan:

1. **Global Steering (User-Level):**
   - **Linux / macOS:** `~/.config/galleon/steering/`
   - **Windows:** `%APPDATA%\galleon\steering\` (atau `$HOME/.config/galleon/steering/` sesuai standar XDG fallback).
   - *Tujuan:* Aturan preferensi global pengguna yang berlaku di semua proyek (misal: gaya bahasa, preferensi komentar, aturan keamanan personal).

2. **Workspace Steering (Project-Level):**
   - `<workspace_root>/.galleon/steering/`
   - File utama konvensi: `.galleon/steering/main.md` atau file topik modular seperti `.galleon/steering/rules-*.md`.
   - *Tujuan:* Konvensi teknis, pedoman arsitektur, dan SOP khusus repositori/proyek yang dapat di-commit ke version control bersama tim.

---

### Keputusan 2: Format File (Structured Frontmatter + Markdown)

Setiap file steering menggunakan format **Markdown dengan Frontmatter YAML** terstruktur:

```markdown
---
name: backend-guidelines
description: Standard backend coding patterns for Galleon services
globs:
  - "engine/**"
  - "crates/**"
priority: 100
enabled: true
authorities:
  - "read_only"
  - "draft_only"
  - "gated_write"
---

# Backend Guidelines
- Use explicit error propagation (no silent unwraps).
- Enforce fail-closed boundaries on all security checks.
```

**Metadata Fields:**
- `name` *(string)*: Pengenal unik file steering.
- `description` *(string)*: Penjelasan singkat fungsi steering.
- `globs` *(list of strings, opsional)*: Pola path file di mana steering ini aktif (jika kosong, aktif di seluruh workspace).
- `priority` *(integer)*: Urutan injeksi (nilai lebih tinggi diinjeksi lebih dulu atau memiliki bobot lebih tinggi).
- `enabled` *(boolean)*: Toggle aktif/nonaktif.
- `authorities` *(list of strings, opsional)*: Batasan authority level yang terikat pada rule ini.

---

### Keputusan 3: Resolusi Konflik (Conflict Resolution Strategy)

Mekanisme resolusi mengikuti pola **Workspace-Wins-Over-Global**:

1. **Rule Precedence:**
   ```
   [Workspace Steering: .galleon/steering/*.md]
           ▼ (Meng-override jika nama/kategori identik)
   [Global Steering: ~/.config/galleon/steering/*.md]
           ▼ (Menambah jika tidak bertentangan)
   [Default Galleon System Steering]
   ```
2. **Behavioral Rules:**
   - Jika file global dan workspace mendefinisikan `name` yang sama (misal `code-style`), file **Workspace** menggantikan konfigurasi file global sepenuhnya untuk sesi kerja tersebut.
   - Jika `globs` didefinisikan, steering hanya diinjeksi ke context turn jika set file yang aktif atau disentuh cocok dengan pola glob.
   - Penggabungan (Merge): Instruksi teks non-konflik digabungkan secara sekuensial berdasarkan `priority` desc.

---

### Keputusan 4: Pemisahan Peran Go vs Rust (Boundary Allocation)

Mengikuti arsitektur Galleon dan preseden `identity.RequireAuthenticated` / `PrincipalResolver` (Sprint 0 BG-C1):

1. **Go Engine (Policy, Consent & Active Context Authority):**
   - Bertanggung jawab memutuskan *file steering mana yang sah dan aktif*.
   - Memvalidasi izin path (mencegah path traversal di luar root workspace).
   - Menyediakan API query steering `/api/v1/steering/active` yang mengembalikan daftar file terverifikasi beserta metadata frontmatter.
   - Memastikan aturan keamanan (misal: steering tidak boleh mengecualikan payment-lockout gate).

2. **Rust `zerocode` / `galleoncode` CLI (Teknis I/O & Injeksi Prompt):**
   - Menangani pembacaan fisik file dari filesystem lokal.
   - Parsing frontmatter YAML dan token budgeting.
   - Memasukkan konten steering ke dalam context window / system prompt LLM sebelum request generate dikirim.
   - Menghitung token overhead agar steering tidak melampaui context limit yang ditentukan.

---

## 3. Rencana Implementasi & Transisi Nama

- **Fase Desain (Sprint 7 BL-02):** Dokumen spesifikasi ini diajukan untuk persetujuan Owner.
- **Fase Implementasi (Pasca-Review):**
  - Implementasi Go policy resolver di `engine/src/steering/` (atau `engine/src/fleet/`).
  - Implementasi CLI reader di `apps/zerocode/src/steering.rs` (yang nantinya beralih ke `galleoncode` pada Sprint 8 RB-20).
  - Integrasi UI di Settings (`web/src/components/features/SettingsView.tsx`) untuk menginspeksi steering aktif.

---

## 4. Keamanan & Sanitasi

1. **Path Traversal Prevention:** Root workspace dikunci menggunakan evaluasi canonical path.
2. **Prompt Injection Boundary:** Steering end-user dibungkus dalam tag isolasi sistem (`<user_steering name="...">...</user_steering>`) agar LLM membedakan instruksi sistem dasar dan steering custom.
3. **No Credential Ingestion:** File steering yang mengandung pattern credential/token otomatis ditolak saat parsing.
