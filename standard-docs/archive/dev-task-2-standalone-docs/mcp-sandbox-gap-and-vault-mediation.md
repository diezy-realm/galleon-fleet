# Analisis Gap Sandbox MCP dan Mediasi Vault Rahasia (BL-20, BL-21)

- **Tanggal:** 2026-10-07
- **Referensi Tugas:** Sprint 5 (`BL-20`, `BL-21`)
- **Implementasi Terkait:**
  - MCP Transport Host: `crates/galleon-tools/src/mcp_transport.rs`
  - Secrets Management: `crates/galleon-config/src/secrets.rs`

---

## 1. Status Implementasi MCP Host Aktual (Hasil Verifikasi Kode)

Berlawanan dengan asumsi awal bahwa MCP client host belum ada atau berada di `galleon-plugins`, verifikasi kode menunjukkan:
- **Lokasi Nyata:** MCP Client Host berada di `crates/galleon-tools/src/mcp_transport.rs`, `mcp_client.rs`, `mcp_deferred.rs`, dan `mcp_protocol.rs`.
- **Protokol:** Mendukung JSON-RPC melalui stdio subprocess, SSE, dan HTTP streaming.
- **WASM vs MCP:** `galleon-plugins` adalah murni host WASM Component Model (`wasmtime`), sedangkan `galleon-tools` mengelola koneksi MCP.

---

## 2. Gap Analysis Sandbox MCP (BL-20)

### Apa yang Sudah Dilindungi di Runtime MCP Saat Ini
Dalam `crates/galleon-tools/src/mcp_transport.rs::apply_default_mcp_sandbox`:
1. **Deny-by-default execution**: Jika `config.sandbox_profile` diset ke `"deny-all"` atau `"blocked"`, eksekusi child process langsung digagalkan sebelum spawn.
2. **Sanitasi Direktori Kerja**: Jika `config.allowed_paths` ditentukan, working directory anak dipaksa ke jalur yang diizinkan pertama.
3. **Pembersihan Environment Variables Kritis (CWE-200)**: Host secara otomatis menghapus environment variables berbahaya dari proses induk:
   - `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`
   - `AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`, `GH_TOKEN`, `DATABASE_URL`

### Gap Keamanan Dibandingkan Host WASM (`galleon-plugins`)
1. **Ambient OS Permissions**: Jika `McpCommandBoundary` (Landlock/Bubblewrap di Linux, Seatbelt di macOS, AppContainer di Windows) tidak dipasang oleh host runtime, proses MCP stdio berjalan dengan user privilege host dan dapat membaca filesystem di luar direktori kerja.
2. **Network Egress Hook**: Proses MCP memiliki akses raw socket sistem operasi, tidak melewati filter interceptor terpusat seperti `PluginEgressHooks` pada WASM.
3. **Resource Ceiling Enforcement**: Tidak ada penghitungan instruksi/fuel secara deterministik seperti `call_fuel` pada WASM; pembatasan hanya bergantung pada timeout dan kill signal.

---

## 3. Desain Vault-Mediation untuk Secret MCP (BL-21)

### Prinsip Utama
- **Single Source of Truth**: Memanfaatkan `crates/galleon-config/src/secrets.rs` (`encrypt`, `decrypt`, `decrypt_and_migrate`) sebagai backend penyimpanan dan enkripsi rahasia. Jangan membuat modul vault terpisah.
- **Resolusi JIT (Just-In-Time)**: Secret dalam header HTTP atau environment variable MCP (`config.env` dan `config.headers`) tidak boleh disimpan dalam plaintext di konfigurasi on-disk.
- **Protokol Enkripsi**:
  1. Config menyimpan ciphertext berformat `$clawvault$v1$...`.
  2. Saat `McpTransport::connect` atau `SpawnCommand` diinisiasi, host memanggil `secrets::decrypt` untuk memetakan key rahasia ke environment variable child process atau header request.
  3. Environment variable proses induk yang tidak terdaftar tetap dibersihkan per `apply_default_mcp_sandbox`.
