# Galleon — Definisi Produk (Ringkas, Normatif)

> **Sumber:** `decisions/owner-decisions/ODR-2026-10-04-product-direction.md` + `.kiro/steering/galleon-product-fundamentals.md` (invariant F1 dll tetap berlaku).
> **Catatan:** `prd-hybrid-engine.md` di folder ini adalah PRD fase lama (era ClawCrew Phase 2); bila bertentangan, dokumen ini menang.
> **Tier & Pricing:** diselaraskan ke `standard-docs/product-research/tier-pricing-and-entitlement-final.md`, 2026-10-06. Tangga tier final: Community (gratis) / Team ($15/bln flat) / Dockyard ($28/bln flat + per seat) / Private Dockyard (custom). Dokumen ini tidak merinci angka tier — lihat sumber tersebut untuk tabel plafon lengkap (Ships/Squads/CrewPerShip/ConcurrentVoyages). Dua dokumen lama sudah di-supersede: `community-tier-licensing-enforcement.md` dan `tier-enforcement-method-d-c-deep-dive.md`.

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

> [!NOTE]
> **Pembaruan Arsitektur (ODR-2026-10-29 / ADR-0018):** Seluruh permukaan akses kini berbicara langsung ke Rust gateway (`galleon-gateway`, :42617). Tier Go engine telah diarsipkan dan dihapus dari repositori inti (lihat `standard-docs/tech-standart/architecture/01-boundary-policy.md` v2).

| Permukaan | Wujud | Berbicara ke |
|---|---|---|
| CLI | binary terminal | Rust gateway (`galleon-gateway`, :42617) |
| Desktop | Tauri v2 (`apps/tauri/`) membungkus UI | Rust gateway (`galleon-gateway`, :42617) |
| Web | React (`web/`) | Rust gateway (`galleon-gateway`, :42617) |
| QR | login perangkat lain ke instance yang sedang berjalan | Rust gateway (pairing + otorisasi perangkat) |

Semua permukaan masuk lewat **Rust Gateway**. Tidak ada perantara runtime Go.

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
