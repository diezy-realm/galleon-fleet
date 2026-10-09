# Galleon — Definisi Produk (Ringkas, Normatif)

> **Sumber:** `decisions/owner-decisions/ODR-2026-10-04-product-direction.md` + `.kiro/steering/galleon-product-fundamentals.md` (invariant F1 dll tetap berlaku).
> **Catatan:** `prd-hybrid-engine.md` di folder ini adalah PRD fase lama (era ClawCrew Phase 2); bila bertentangan, dokumen ini menang.

## 1. Apa produknya

**Galleon = local-first AI organization workspace.** Satu orkestrator (Quartermaster) memimpin organisasi agent yang **memiliki memori persisten dan terus berkembang**. Pengalaman mirip Kiro Crew / GitHub Copilot, tetapi unit kerjanya organisasi, bukan satu asisten.

```
Quartermaster (CEO, router dua-lapis: jawab ringan / rutekan quest)
 └─ Ship        = departemen        (banyak)
     └─ Squad   = tim dalam departemen (banyak)
         └─ Crew = agent individu (skills, tool scope, memory scope, budget)
```

Istilah lore (Ship, Quartermaster, …) hanya untuk UX; **kode/API/DB memakai istilah netral** (`fleet`, `ship`, `squad`, `crew`, `run`, `artifact`, `policy`, `approval`).

## 2. Empat permukaan akses (bukan "channel messaging")

| Permukaan | Wujud | Berbicara ke |
|---|---|---|
| CLI | binary terminal | Go engine (HTTP :9090) |
| Desktop | Tauri v2 (`apps/tauri/`) membungkus UI | Go engine (HTTP :9090), hanya proxy + fitur OS |
| Web | React (`web/`) | Go engine (HTTP :9090) |
| QR | login perangkat lain ke instance yang sedang berjalan | Go engine (pairing + otorisasi perangkat) |

Semua permukaan masuk lewat **Go**. Tidak ada yang bicara langsung ke Rust.

## 3. Channel messaging (integrasi eksternal)

| Tier | Isi | Distribusi |
|---|---|---|
| A | channel umum: Slack, WhatsApp, Discord, Telegram, Matrix + primitif transport | ikut terinstal |
| B | channel butuh native/protokol berat (mis. WeChat, QQ, Signal, iMessage, IRC) | add-on terkompilasi, dipilih saat instal |
| C | channel inbound-first niche | plugin WASI bertanda tangan |

Detail teknis: `tech-standart/plugin-standard/plugin-codebase-standard.md`.

## 4. Memori & pembelajaran

- Memori **persisten** per scope (Fleet/Ship/Squad/Crew/Run), disimpan di SQLite (lokal) atau PostgreSQL (server).
- **Learning by consent**: umpan balik menjadi memori/aturan hanya setelah disetujui Owner; scoped, versioned, reversible.
- Kolaborasi antar-agent lewat **Artifact reference** dan handoff terkontrol; berbagi memori lintas-Ship default diblokir.

## 5. Keamanan & kontrol

- **Autonomy under command**: aksi berdampak tinggi (write/publish/merge/deploy/delete/bayar, ubah Fleet Code, lampaui budget) wajib persetujuan; default read-only.
- Go = satu-satunya gerbang masuk (anti-injeksi): eksekusi terminal/tool berjalan **UI/Tauri → Go (policy+approval) → Rust (sandbox)**. Tidak ada eksekusi lokal di Go atau Tauri.
- Secrets hanya di Vault Rust (AES-256-GCM).
- Perangkat tambahan didaftarkan lewat QR + token terikat perangkat; akses remote wajib TLS.

## 6. Deployment

- **Utama: lokal** (desktop/CLI). 
- **Opsional: remote** — Nginx (reverse proxy + TLS) di depan Go; dapat dipasang di VPS/cloud, perangkat lain terhubung via QR pairing.
- BYOK/BYOM/BYOI: user membawa key/model/infra; **zero platform markup** (invariant F1, Treasury read-only).

## 7. Bukan tujuan produk

Reseller kredit, chatbot generik, workflow builder no-code, sistem otonom tanpa batas Owner, UI yang berisi logika backend.
