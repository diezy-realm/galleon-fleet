# ADR: WASM Plugin Host vs Model Context Protocol (MCP) Integration

- **Status:** Accepted (Sprint 5, BL-22)
- **Deciders:** ClawCrew Core Architecture, Antigravity Team
- **Date:** 2026-10-07

## Context

ClawCrew menyediakan dua mekanisme ekstensi eksternal untuk mengintegrasikan alat (tools), channel, dan sumber daya komputasi pihak ketiga ke dalam siklus hidup agen:
1. **WASM Plugin Host (`crates/clawcrew-plugins/`)**: Berbasis WebAssembly Component Model (WIT v0/v1) yang dijalankan in-process melalui runtime Wasmtime dengan sandboxing deterministik berbasis capability.
2. **MCP Client Host (`crates/clawcrew-tools/`)**: Berbasis Model Context Protocol (MCP) stdio/HTTP/SSE subprocess yang menjalankan executable biner atau skrip eksternal melalui protokol JSON-RPC.

Terdapat kebutuhan untuk mendefinisikan batas arsitektural, perbandingan keamanan, trade-offs, dan panduan keputusan ("kapan menggunakan yang mana") bagi kontributor dan pengguna ekosistem.

## Perbandingan Arsitektural & Trade-Offs

| Karakteristik | WASM Plugin Host (`clawcrew-plugins`) | MCP Client Host (`clawcrew-tools`) |
|---|---|---|
| **Model Eksekusi** | In-process WebAssembly component model via Wasmtime engine. | Out-of-process OS subprocess (`tokio::process::Command`) via JSON-RPC stdio/HTTP. |
| **Batas Isolasi Memori** | Hardware-enforced sandboxing virtual memory (linear memory bounded per store, `PluginLimits`). | Isolasi proses OS level (bergantung pada boundary eksternal seperti Landlock/Seatbelt). |
| **Resource Quota** | Deterministic fuel counting (`call_fuel`), execution timeout, memory limit (`max_memory_bytes`), table element limit. | Hanya dibatasi oleh alokasi sistem operasi atau cgroups (kecuali dibungkus `McpCommandBoundary`). |
| **Filesystem Access** | Default empty WASI context (tanpa preopened directories). Akses filesystem diverifikasi via `PluginPermission::FileRead`/`FileWrite`. | Ambient filesystem access penuh milik user penjalan proses, kecuali direktori kerja disanitasi via `allowed_paths`. |
| **Network Egress** | Gated strict via `WasiHttpView` dan `PluginEgressHooks`. Tidak ada raw socket access tanpa izin. | Subprocess memiliki akses jaringan ambient OS tanpa hook egress terpadu secara default. |
| **Secret Management** | Hook `secrets.wit` tersinkronisasi dengan config resolver in-process; rahasia diambil point-of-use dan tidak disimpan di memory tamu. | Environment variables diteruskan ke subprocess (`config.env`) dengan filtering daftar hitam CVE/CWE. |
| **Kompatibilitas Bahasa** | Bahasa yang mendukung kompilasi WASM/WIT (Rust, Go via TinyGo, C/C++, AssemblyScript). | Bahasa pemrograman apa pun yang dapat dijalankan sebagai command line (Node.js, Python, Bash, Go, Rust, Java, dll). |
| **Kematangan Ekosistem** | Ekosistem Component Model sedang berkembang pesat (WASI 0.2 / preview 2). | Ekosistem luas tools LLM/agent berbasis MCP siap pakai (GitHub, filesystem, SQLite, Postgres MCP servers). |

## Keputusan Arsitektur

1. **WASM Plugin Host sebagai Default Keamanan Tinggi**:
   - Komponen internal, channel pihak ketiga (seperti `channel-nostr-pilot`), dan integrasi enterprise yang memproses data sensitif atau memerlukan multi-tenant security **harus** dibangun sebagai WASM Component.
   - Menggunakan model capability: tidak ada hak istimewa implisit.

2. **MCP sebagai Bridge Integrasi Eksternal Fleksibel**:
   - Digunakan untuk memanfaatkan ekosistem open-source MCP yang sudah ada (alat CLI, bridge database, integrasi sistem operasi).
   - Seluruh eksekusi MCP stdio **wajib** melalui `apply_default_mcp_sandbox` dan dianjurkan menggunakan `McpCommandBoundary` pada lingkungan produksi terisolasi.
   - Rahasia wajib dimediasi melalui config vault (`clawcrew-config::secrets`).

## Konsekuensi

- **Positif:** Menggabungkan keamanan deterministik zero-trust dari WASM dengan fleksibilitas integrasi cepat dari MCP.
- **Tantangan:** Perlu menjaga dua jalur pengujian integrasi dan menjamin dokumentasi gap sandbox MCP tetap transparan bagi operator sistem.
