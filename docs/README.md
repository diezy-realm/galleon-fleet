# Dokumentasi Galleon Fleet

> [!IMPORTANT]
> **Single Source of Truth (SSOT)** arsitektur, keputusan, konvensi teknis, dan roadmap aktif repository Galleon Fleet berada di direktori **[`standard-docs/`](../standard-docs/)**.

Direktori `docs/` ini berisi dokumen historis, artefak lineage, dan referensi transisional dari fase awal pengembangan (ClawCrew Phase 1 & 2). 

## Peta Dokumentasi

| Kategori | Lokasi Kanonikal (SSOT) | Keterangan |
|---|---|---|
| **Arsitektur & Prinsip** | [`standard-docs/tech-standart/architecture/`](../standard-docs/tech-standart/architecture/) | Isolasi 4-tier, boundary policy, ODR-01 s/d ODR-25 |
| **Kontrak API & Protokol** | [`standard-docs/tech-standart/api-contracts/`](../standard-docs/tech-standart/api-contracts/) | gRPC (:50051, :50052) & HTTP REST (:9090) |
| **Konvensi Teknologi** | [`standard-docs/tech-standart/tech-stack/`](../standard-docs/tech-standart/tech-stack/) | Rust 2024, Go 1.27+, React 19 / TS, Tauri v2 |
| **Keputusan Arsitektur (ADR)** | [`standard-docs/decisions/`](../standard-docs/decisions/) | ADR-0001 s/d ADR-0003 dan addenda |
| **Roadmap & Tiket Pelaksanaan** | [`standard-docs/roadmap/`](../standard-docs/roadmap/) | PR Plan, Lane G, R, U, P, N, D |
| **Fondasi & Analisis Silsilah** | [`standard-docs/foundation/`](../standard-docs/foundation/) | Drift register, lineage, placement analysis |
| **Dokumen Historis / Lineage** | `docs/` (direktori ini) | Referensi historis ClawCrew & KiroCrew |
