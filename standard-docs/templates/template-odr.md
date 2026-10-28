# ODR-YYYY-MM-DD — [Judul Keputusan Owner]

- **Status:** Ratified | Proposed | Superseded
- **Tanggal:** YYYY-MM-DD
- **Otoritas:** Owner (Achmad) / Principle
- **Konteks:** Galleon Fleet Platform
- **Terkait:** ODR-xx, ADR-xxxx, PR-xx

---

## 1. Latar Belakang & Masalah
*Deskripsikan konteks strategis, bisnis, atau teknis tingkat tinggi yang memerlukan keputusan Owner.*

## 2. Pernyataan Keputusan (Mandat Mutlak)
*Tuliskan poin-poin keputusan yang bersifat mengikat dan tidak dapat ditawar.*
1. **Keputusan 1:** ...
2. **Keputusan 2:** ...

## 3. Batasan & Larangan (Guardrails)
*Apa yang secara eksplisit DILARANG dilakukan oleh tim atau agen AI.*
- **Dilarang:** ...
- **Diharuskan:** ...

## 4. Dampak Arsitektur & Operasional
- **Tier 1 (Desktop shell — Tauri):** ...
- **Tier 2 (Rust Core — workspace crates + gateway):** ...
- **Tier 3 (UI Web):** ...
- **Add-on Go/bahasa lain (repo terpisah, via HTTP/WS publik):** ... (isi "tidak ada" bila tidak relevan)

## 5. Rencana Pelaksanaan & Verifikasi
- [ ] Implementasi pada PR-xx
- [ ] Verifikasi invariant melalui boundary gate
