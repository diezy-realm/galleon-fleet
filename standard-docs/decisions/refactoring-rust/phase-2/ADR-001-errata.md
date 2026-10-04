# Errata & Addendum ADR-001: Channel Distribution Updates

- **Status:** Ratified
- **Tanggal:** 2026-10-04
- **Dokumen Terkait:** [`ADR-001-channel-plugin-distribution.md`](./ADR-001-channel-plugin-distribution.md)
- **Otoritas:** Owner & Architecture Quality Gate
- **Kepatuhan:** ODR-01, ODR-02, ODR-11 (ADR asli tidak diedit secara mutasi langsung; pembaruan dicatat via errata).

---

## 1. Latar Belakang

Dokumen normatif `ADR-001-channel-plugin-distribution.md` (2026-10-01) mendefinisikan pembagian channel ke dalam 3 tier: Tier A (Embed), Tier B (Feature-crate opsional), dan Tier C (WASI plugin). Sejak dirumuskan, terdapat perkembangan implementasi konkret dan prioritas produk yang memerlukan klarifikasi resmi.

---

## 2. Poin-Poin Koreksi & Pembaruan

### 2.1 WhatsApp Masuk ke Tier A (Embed / First-Party Core)
- **ADR-001 Asli:** Menempatkan `whatsapp` / `whatsapp-web` di Tier B (Feature-crate opsional).
- **Errata / Ketetapan:** WhatsApp adalah saluran komunikasi primer bagi pasar inti Galleon Fleet (Indonesia & Asia Tenggara). Untuk memastikan ketersediaan out-of-the-box tanpa konfigurasi build opsional yang membebani pengguna akhir, WhatsApp ditetapkan sebagai **Tier A (First-Party Default)** bersama Telegram, Discord, Slack, dan Matrix.

### 2.2 Status Kontrak WIT v0: Resmi Dibekukan (`.frozen`)
- **ADR-001 Asli:** Menyatakan kontrak masih `@unstable(feature = plugins-wit-v0)` dan belum beku.
- **Errata / Ketetapan:** Kontrak antarmuka WebAssembly WIT pada direktori `wit/v0/` telah **dibekukan secara resmi** dengan kehadiran marker `.frozen`. Kontrak ini menjadi basis stabil untuk plugin channel eksternal tanpa risiko backward compatibility breaking pada level ABI v0.

### 2.3 Bluesky Dipindahkan dari Tier C ke Native Crate Mandiri
- **ADR-001 Asli:** Menempatkan `bluesky` di Tier C (WASI plugin / marketplace).
- **Errata / Ketetapan:** Implementasi integrasi Bluesky membutuhkan fitur autentikasi AT Protocol dan koneksi streaming XRPC firehose yang lebih andal secara native. Oleh karena itu, Bluesky diimplementasikan sebagai crate Rust mandiri di [`crates/galleon-channel-bluesky`](../../../../crates/galleon-channel-bluesky), bukan sebagai WASI plugin Tier C.

---

## 3. Ringkasan Klasifikasi Channel Terkini

| Channel | Status Asli ADR-001 | Status Terkini (Errata) | Bentuk Implementasi |
|---|---|---|---|
| **WhatsApp** | Tier B | **Tier A** (Core) | Built-in first-party |
| **Matrix** | Tier B → A (rev 10-03) | **Tier A** (Core) | Built-in first-party |
| **Telegram / Discord / Slack** | Tier A | **Tier A** (Core) | Built-in first-party |
| **Bluesky** | Tier C | **Tier B** (Native Crate) | `crates/galleon-channel-bluesky` |
| **Signal / WeChat / QQ** | Tier B | **Tier B** (Native Crate) | Feature-gated native crate |
| **WIT Plugin Runtime** | Unstable | **Frozen (`v0`)** | `crates/clawcrew-plugins` + `wit/v0/` |
