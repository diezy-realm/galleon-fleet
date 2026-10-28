# Owner Decision Record — 2026-10-04 (Addendum 1)

> **Status:** Accepted. Melengkapi `ODR-2026-10-04-product-direction.md` (tidak menggantikan). Immutable.

## Jawaban Owner atas asumsi A1–A4 & arahan baru

| ID | Keputusan | Efek |
|---|---|---|
| ODR-11 | **Agent orchestrator = Go** (disetujui sebagai arah; R-00 menjadi **prioritas utama**) | Agent loop/SOP/subagents/cron di `galleon-runtime` diklasifikasi: pindah ke Go, hapus, atau feature-gate. Lihat `roadmap/R-rust.md` R-00 |
| ODR-12 | **CLI dijalankan sebagai binary Rust.** Bahasa ≠ otak: CLI Rust bertindak sebagai klien ke Go engine untuk orkestrasi; Rust CLI memiliki urusan lokal (service/daemon, doctor, vault, tool native) | Menggantikan A1 versi "klien tipis murni". Detail: `foundation/05-orchestrator-placement-analysis.md` |
| ODR-13 | **WeChat, QQ, dst. = Tier B** (alasan regional) | Add-on terkompilasi, dipilih saat instalasi; bukan WASI |
| ODR-14 | A3 diterima bersyarat: REST gateway Rust lama dihapus **bila terbukti tidak dipakai** (butuh inventaris pemakai) | Tiket R-14, R-00 |
| ODR-15 | A4 diterima: Nginx = reverse proxy + TLS saja | P-10 |
| ODR-16 | **Rebrand penuh ke Galleon**: nama `zeroclaw`, `zerocode`, `zerorelay`, `claw-crew`, `ClawCrew`, `FleetAI`/`Fleet AI` diganti `galleon` / `Galleon` / `galleon-fleet`; kode/dokumen/ artefak yang tidak relevan dihapus | Lane baru `roadmap/N-rename-and-legacy-cleanup.md` |
