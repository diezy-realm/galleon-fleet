# Owner Decision Record — 2026-10-04 (Addendum 4)

> **Status:** Accepted. Immutable. Menutup keputusan terbuka `R-00a`.

| ID | Keputusan | Efek |
|---|---|---|
| ODR-21 | `verifiable_intent`: dianalisis dulu untuk pertimbangan masa depan (`research/verifiable-intent-assessment.md`); karena legacy komersial tanpa pemakai → **dihapus** | R-38; G-37 (ADR opsional DAC) |
| ODR-22 | Identitas, principal, WebAuthn, pairing → **Go**; Rust hanya mengautentikasi pemanggil Go | G-35, R-39 |
| ODR-23 | **Galleon Client Protocol v1**: kosakata JSON-RPC dipertahankan, server di Go; TUI/ACP hanya ganti transport | G-36, R-40; `rpc` dihapus terakhir (R-41) |
| ODR-24 | Integrasi editor/IDE via **ACP adalah fitur produk yang dipertahankan** | `galleon-acp-bridge` dipertahankan (rename N-20) |
| ODR-25 | `cert_ledger` dan `galleonrelay` dipertahankan untuk perangkat di balik NAT sampai P-11 memutuskan peran terhadap Nginx | R-36, P-11 |
