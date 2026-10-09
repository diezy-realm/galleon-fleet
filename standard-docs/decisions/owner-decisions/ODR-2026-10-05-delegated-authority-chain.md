# Owner Decision Record — 2026-10-05: Delegated Authority Chain (DAC)

> **Status:** Accepted. Mandatory Gate.
> **Referensi:** `archive/research-closed-loop/verifiable-intent-assessment.md` §3c, ODR-21 (Addendum 4), Invariant F1.
> **ID Tiket:** BG-F6 · G-37 · PR-99

---

## 1. Konteks & Latar Belakang

Pada ODR-21, implementasi warisan `verifiable_intent` (AP2 v0.1 draft) dihapus dari core codebase karena statusnya setengah jadi, spesifikasi hulu masih bergerak aktif (donasi ke FIDO Alliance per Mei 2026), dan ketiadaan pemakai aktif.

Namun, sebagaimana dikonfirmasi oleh Owner pada tinjauan 2026-10-05 (`verifiable-intent-assessment.md` §3c):
- Cakupan transaksi di `galleon-product-definition.md` §5 berpotensi meluas ke **agentic commerce** — situasi di mana autonomous agent bertindak atas nama pengguna untuk membeli layanan atau resource dari pihak ketiga.
- Status ADR Delegated Authority Chain (DAC / G-37) secara resmi **di-upgrade dari "P3, opsional" menjadi GATE WAJIB (Mandatory Gate)** yang harus dipenuhi dan ditinjau ulang sebelum desain fitur commerce apa pun dimulai.

---

## 2. Definisi Delegated Authority Chain (DAC)

Delegated Authority Chain adalah rantai kriptografis otorisasi berjenjang yang menghubungkan niat asli pemilik (Pirate King / User) hingga eksekusi aksi di level agent:

```
[User / Pirate King] 
       │ (Passkey / WebAuthn / Hardware Signature)
       ▼
[Go Engine: Principal & Policy Gate]
       │ (Scoped Delegated Token + Ephemeral Expiry)
       ▼
[Autonomous Agent / Crew Member]
       │ (Constrained Intent: Action, Amount, Merchant, Nonce)
       ▼
[Payment-Lockout Gate (Rust Core / Hardware Tool Loader)]
       │ (Fail-Closed Enforcement)
       ▼
[External Merchant / Payment Gateway]
```

---

## 3. Prinsip & Aturan Non-Negosiasi (Invariants)

1. **Gate Wajib Sebelum Desain Commerce:**
   Tidak ada komponen atau tool pembayaran baru yang boleh didesain atau diimplementasikan tanpa validasi rantai DAC ini. Fitur commerce tidak boleh mengandalkan "trust" semata pada model LLM.

2. **Konsistensi Invariant F1:**
   - **Treasury Read-Only**: Agent tidak boleh memiliki akses write langsung ke dana atau credential mentah.
   - **Zero Platform Markup**: Galleon tidak bertindak sebagai reseller kredit atau perantara penampung dana. Galleon murni fasilitator keamanan lokal.

3. **Integrasi dengan Preventive Payment-Lockout Gate (BL-40..44):**
   - Mode default sistem selalu `Locked` (fail-closed).
   - Setiap delegasi wewenang finansial harus memiliki TTL/`unlock_expires_at` eksplisit.
   - Tool bermuatan finansial tanpa identifikasi kategori pembayaran ditolak saat registrasi (fail-closed registration).

4. **Kriptografi & Standarisasi Terbuka:**
   - Evaluasi arsitektur harus mengikuti standar FIDO Alliance / AP2 v0.2+ yang matang, menggunakan signature berbasis ECDSA (`ring`) dan Selective Disclosure (SD-JWT) untuk meminimalkan paparan data identitas ke merchant.

5. **Pemisahan Boundary:**
   - **Go Engine**: Menangani resolusi identitas principal, persetujuan pengguna (consent), evaluasi risiko intent, dan audit trail.
   - **Rust Core & Hardware**: Menegakkan sandboxing ketat, validasi manifesto tool, dan lockout preventif di level I/O execution.

---

## 4. Dampak Arsitektur

- **Sprint 7**: Penegakan `PaymentLockConfig`, `PaymentLockManager`, dan audit lockout (BL-40..BL-44) berfungsi sebagai fondasi preventif sebelum kapabilitas commerce dibangun.
- **Masa Depan**: Saat inisiatif Agentic Commerce diaktifkan, spesifikasi DAC ini menjadi acuan tunggal kontrak delegasi wewenang dan persetujuan.
