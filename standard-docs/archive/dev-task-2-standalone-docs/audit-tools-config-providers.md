# Audit: `galleon-tools`, `clawcrew-config`, `galleon-providers` (BR-C8)

**Ticket:** BR-C8 · R-18 · PR-97  
**Principle:** Single Source of Truth & Ponytail (No speculative splitting / YAGNI)  
**Date:** 2026-10-07  

---

## 1. Executive Summary

Audit ini mengevaluasi apakah `crates/galleon-tools`, `crates/clawcrew-config`, dan `crates/galleon-providers` memerlukan pemecahan crate (crate splitting) pada fase ini.

**Kesimpulan:** **TIDAK PERLU DIPECAH (Keep As-Is, No Speculative Splitting).**  
Ketiga crate memiliki batas tanggung jawab (cohesion) yang jelas, dependency graph yang satu arah tanpa siklus, dan pemecahan spekulatif hanya akan menambah overhead kompilasi dan boilerplate re-export tanpa manfaat arsitektural yang nyata.

---

## 2. Metrik & Ukuran Crate

| Crate | Jumlah File `.rs` | Total Baris Kode (LOC) | Peran Arsitektural |
| :--- | :--- | :--- | :--- |
| `crates/clawcrew-config` | 46 | 85,838 | Canonical SSOT Schema & Defaults untuk seluruh workspace |
| `crates/galleon-providers` | 49 | 84,251 | Implementasi Model & Inference Providers |
| `crates/galleon-tools` | 97 | 85,720 | Implementasi Native Tools platform |

---

## 3. Analisis per Crate

### 3.1 `clawcrew-config`
- **Fungsi:** Menyediakan schema validasi, serialisasi TOML/JSON, dan defaults.
- **Ketergantungan:** Leaf/near-leaf crate. Hampir seluruh crate di workspace bergantung padanya untuk membaca config secara read-only.
- **Risiko jika dipecah:** Memecah schema per-fitur akan menciptakan siklus atau duplikasi tipe (`ApprovalConfig`, `ChannelConfig`, dll.) yang saat ini berada rapi di bawah satu schema validator.
- **Rekomendasi:** Pertahankan sebagai satu crate monolitik config.

### 3.2 `galleon-providers`
- **Fungsi:** Mengintegrasikan model provider (OpenAI, Anthropic, Gemini, Harbor, Ollama, DeepSeek).
- **Ketergantungan:** Bergantung pada `clawcrew-config` dan HTTP transport.
- **Risiko jika dipecah:** Provider sudah diisolasi di balik feature flags (`feature = "provider-openai"`, dsb.). Cargo feature gates sudah cukup untuk mengontrol kompilasi tanpa perlu memecah repo menjadi lusinan sub-crate mini.
- **Rekomendasi:** Pertahankan arsitektur feature-gated tunggal.

### 3.3 `galleon-tools`
- **Fungsi:** Menampung implementasi native tools (filesystem, git, terminal, browser, dsb.).
- **Ketergantungan:** Bergantung pada `clawcrew-api` dan interface runtime.
- **Risiko jika dipecah:** Tool dispatch dipanggil via satu registry terpadu. Memecah per tool hanya memperumit pendaftaran di `ToolDispatcher`.
- **Rekomendasi:** Pertahankan sebagai satu crate tools.

---

## 4. Keputusan Arsitektural

Sesuai arahan tiket BR-C8 *"tanpa pemecahan spekulatif"*:
- **Status:** Audit Selesai — Tidak ada pemecahan crate baru untuk `tools`, `config`, maupun `providers`.
