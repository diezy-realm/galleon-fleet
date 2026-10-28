# Arsitektur Ekosistem Open Skills

**Status:** Normative Specification  
**Domain:** Agent Plugins & Skill Loading Architecture  
**Reference:** BL-03, `crates/galleon-runtime/src/skills/mod.rs`  

---

## 1. Ikhtisar Arsitektur 3-Lapis (Three-Layer Resolution)

Sistem ClawCrew / Galleon Fleet mengadopsi resolusi skill 3-lapis hierarkis:

```
[Layer 1: Workspace Skills] (Prioritas Tertinggi — Project Specific)
            ↓ (fallback jika tidak ada)
[Layer 2: Global User Skills] (Shared across projects di mesin lokal)
            ↓ (fallback jika tidak ada)
[Layer 3: Plugin / WASM Skills] (Didistribusikan via Wasm/Registries)
```

1. **Layer 1 — Workspace Scope:**
   - Direktori: `<project_root>/.gemini/skills/` atau `.clawcrew/skills/`
   - Memiliki override authority penuh terhadap global skills dengan nama yang sama.
2. **Layer 2 — Global User Scope:**
   - Direktori: `~/.kiro/crew/skills/` atau `~/.clawcrew/skills/`
   - Berisi skill lintas proyek yang dikelola oleh operator mesin.
3. **Layer 3 — WASM & External Registry Plugins:**
   - Direktori: `skills-registry/` atau `<workspace>/extra-registry-<name>/`
   - Di-download dan di-sinkronisasi secara periodik dari git remote repository.

---

## 2. Struktur Kanonikal Skill

Setiap skill dikemas dalam sebuah direktori independen:

```
<skill-name>/
├── SKILL.md            # Wajib: Instruksi agent dengan YAML frontmatter
├── META.toml           # Opsional: Metadata rilis, author, versi, triggers
├── scripts/            # Opsional: Skrip automasi pendukung
├── references/         # Opsional: Dokumentasi & spesifikasi mendalam
└── assets/             # Opsional: File statis atau template
```

### Format `SKILL.md` (Spesifikasi AgentSkills.io):
```markdown
---
name: sample-skill
description: Ringkasan singkat kapabilitas skill untuk LLM selector
author: Galleon Team
version: 1.0.0
---

# Instruksi Operasional
Petunjuk langsung yang diinjeksikan ke agent runtime saat skill diaktifkan...
```

---

## 3. Pencegahan Duplikasi SSOT

- **Resolusi Deterministik:** Parser selalu menyelesaikan skill berdasarkan prioritas lapis (Workspace > Global > Registry).
- **In-Memory Caching:** Hasil audit resolusi disimpan di `galleon-runtime::skills::cache` untuk mencegah stat file redundant.
- **Audit Logging:** Setiap kali sebuah skill di-drop karena invalid manifest atau shadow override, pesan tercatat di structured log.
