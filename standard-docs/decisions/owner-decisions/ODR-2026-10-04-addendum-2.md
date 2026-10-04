# Owner Decision Record — 2026-10-04 (Addendum 2)

> **Status:** Accepted. Melengkapi addendum 1. Immutable.

| ID | Keputusan | Efek |
|---|---|---|
| ODR-17 | Penamaan: `zerocode` → **`galleoncode`**, `zerorelay` → **`galleonrelay`**. Keduanya **dipertahankan** (hanya rename + penyesuaian); `galleoncode` = TUI CLI, `galleonrelay` = relay mTLS buta | N-00, N-20 |
| ODR-18 | Plugin **WIT dijadikan v1** (`galleon:plugin@1.0.0` di `wit/v1/`); v0 tetap beku sebagai kompatibilitas | N-24, R-35 |

Bukti peran (2026-10-04): `apps/zerocode` = "Interactive TUI config manager" dengan modul `chat`, `acp`, `agent_sidebar`, `config_manager`; `apps/zerorelay` = "standalone blind forwarder yang menerowong mTLS klien↔daemon tanpa dekripsi".
