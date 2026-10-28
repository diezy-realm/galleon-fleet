# Owner Decision Record — 2026-10-04

> **Status:** Accepted (diputuskan langsung oleh Owner pada sesi `feat/refactoring-rust`).
> **Sifat:** immutable. Perubahan = Owner Decision Record baru yang menyatakan `Supersedes ODR-2026-10-04`.
> **Dampak:** dasar `standard-docs/product/galleon-product-definition.md` dan `standard-docs/roadmap/`.

## Keputusan

| ID | Topik | Keputusan Owner | Implikasi teknis |
|---|---|---|---|
| ODR-01 | Lokasi final UI & desktop | UI di **`web/`**, desktop di **`apps/tauri/`**. `web-2/` dan `apps/tauri-2/` **dihapus** (kode sudah dipindah) | Dokumen/konfigurasi yang menyebut `web-2`/`tauri-2` adalah alias lama. `tauri.conf.json` (`frontendDist`) dan Dockerfile harus diselaraskan |
| ODR-02 | Definisi "channel" produk | Galleon **local-first** dengan **4 permukaan akses**: **CLI, Desktop, Web, QR** (QR = login perangkat lain) | "Channel" produk ≠ channel messaging. QR pairing = fitur keamanan inti, Go sebagai gerbang |
| ODR-03 | Fokus produk | **Organisasi AI**: orkestrator agent dengan **memori persisten** yang terus berkembang. Hierarki: **Quartermaster (CEO)** → **Ship (departemen)** → **Squad** → **Crew/agent**. Pengalaman mirip Kiro Crew / GitHub Copilot, berorientasi organisasi | Go engine = otak tunggal; memori persisten dan *learning by consent* menjadi prioritas |
| ODR-04 | Distribusi channel messaging | **Tier A (embed saat instalasi):** channel umum — Slack, WhatsApp, Discord, dst. **Sisanya plugin tambahan** (WeChat, QQ, dst.) | ADR-001 perlu errata: `whatsapp`/`whatsapp-web` naik dari Tier B ke **Tier A** |
| ODR-05 | Persistensi | Menuju **SQLite (lokal) / PostgreSQL (server)** sebagai memori persisten | Disk JSON/JSONL = kondisi sementara; butuh ADR desain store + migrasi |
| ODR-06 | Deployment | **Local-first.** Nginx sebagai jembatan agar CLI/desktop yang berjalan dapat dihubungkan dari perangkat lain; juga dapat dipasang di **VPS/cloud** | Satu artefak server harus berjalan lokal maupun remote; TLS + autentikasi perangkat wajib bila terekspos |
| ODR-07 | Eksekusi terminal dari Tauri | Harus **lewat Go** sebagai lapisan keamanan (anti-injeksi pihak lain) | `execute_terminal_command` langsung di Tauri **dihapus**; diganti Tauri → Go (policy/approval) → Rust (sandbox) |
| ODR-08 | `@google/genai` di UI | Berasal dari **export Google AI Studio** (alat edit/preview), bukan keputusan arsitektur | Dependensi & kode AI Studio dibersihkan dari `web/`; UI tidak memanggil LLM langsung |
| ODR-09 | Lingkup perencanaan | **Semua jalur sekaligus**, tiket berupa checkbox kosong `[ ]` + deskripsi | `standard-docs/roadmap/` |
| ODR-10 | Git | Perubahan **dipisah per concern** saat commit | 4 commit terstruktur |

## Asumsi Technical Lead — perlu koreksi bila salah

| # | Asumsi | Alasan |
|---|---|---|
| A1 | CLI Galleon (binary root `clawcrew` dan/atau `apps/zerocode`) menjadi **klien tipis ke Go :9090**, bukan menjalankan otak agent sendiri | Memenuhi ODR-03 (satu otak). Kini `galleon-runtime` (286k LOC) memuat agent loop, SOP, subagents, cron — tumpang tindih dengan Go |
| A2 | "Plugin tambahan" WeChat/QQ = channel opsional di luar instalasi default. Mekanisme (feature-crate Tier B vs WASI Tier C) ditentukan batas teknis, bukan label | WeChat/QQ butuh protokol native kompleks → kemungkinan Tier B; WASI v0 belum punya outbound HTTP |
| A3 | REST `api_*` gateway Rust yang melayani dashboard lama tidak lagi menjadi permukaan produk; yang wajib hanya gRPC `SystemGatewayService` | Dashboard lama sudah diganti; UI bicara ke Go :9090 |
| A4 | Nginx = reverse proxy + TLS di depan Go :9090 (+WS), tanpa logika bisnis | Sejalan ODR-06 dan zero backend logic di UI |
