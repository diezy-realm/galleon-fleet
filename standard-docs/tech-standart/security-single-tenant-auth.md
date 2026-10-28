# Standard: Single-Tenant Authentication & Trust Security Model (BL-26)

**Ticket:** BL-26 (Single-Tenant Auth Security Model)  
**Status:** Approved  
**Date:** 2026-10-08  
**Domain:** Keamanan Autentikasi & Otorisasi Galleon  
**Source of Truth:** `crates/galleon-api/src/principal.rs` (`PrincipalId::shared_operator()`)  

---

## 1. Konteks & Latar Belakang

Galleon dirancang untuk mendukung deployment fleksibel, mulai dari workstation lokal pengembang independen (single-tenant local operator) hingga cluster kolaborasi tim / multi-operator (multi-principal). 

Dokumen ini mendefinisikan model keamanan **Single-Tenant Authentication**: trust model, batas hak akses (boundaries), mitigasi privilege escalation, dan keterkaitannya dengan arsitektur multi-principal (RFC 7141 identity contract).

---

## 2. Definisi Trust Model Single-Tenant

Dalam mode single-tenant (default pada deployment standalone CLI, TUI, dan workstation development):
- **Asumsi Identitas:** Terdapat satu operator manusia tepercaya (*trusted local operator*) yang memiliki akses fisik atau administratif ke proses host mesin tempat Galleon daemon dijalankan.
- **Sentinel Canonical ID:** Representasi kode kanonikal untuk identitas ini didefinisikan secara eksplisit di `galleon-api/src/principal.rs`:
  ```rust
  impl PrincipalId {
      pub const SHARED_OPERATOR: &'static str = "shared-operator";

      #[must_use]
      pub fn shared_operator() -> Self {
          Self(Self::SHARED_OPERATOR.to_owned())
      }
  }
  ```
- **Karakteristik Sentinel:**
  1. `PrincipalId::shared_operator()` bukan bernilai `None` atau pointer kosong (`Option::None`), melainkan sebuah struct `PrincipalId` valid dan terisolasi.
  2. Mencegah code path melakukan branching berbahaya `if principal.is_none() { allow_all() }`. Seluruh grant, memory lookup, session owner, dan audit log tetap terikat pada ID konkret `"shared-operator"`.

---

## 3. Batas Keamanan & Trust Boundaries

Meskipun diasumsikan hanya ada satu operator, batas-batas keamanan internal tetap ditegakkan:

```
┌────────────────────────────────────────────────────────┐
│                    Host Mesin Lokal                    │
│                                                        │
│   Operator Konsol (CLI/TUI) ◄──► Token/Pairing Gate    │
│              │                                         │
│              ▼                                         │
│   ┌────────────────────────────────────────────────┐   │
│   │   Galleon Gateway / Daemon (:9090)             │   │
│   │   Principal: `shared-operator`                 │   │
│   │   Resolved Grants: [Full Operator Capabilities]│   │
│   └───────────────────────┬────────────────────────┘   │
│                           │                            │
│           Trust Boundary  ▼ (Sandboxed Subprocesses)   │
│   ┌────────────────────────────────────────────────┐   │
│   │   Sub-agents / WASM Plugins / External Tools   │   │
│   │   Principal: `agent:<id>` (Restricted Grants)   │   │
│   │   NO direct host access without approval       │   │
│   └────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────┘
```

1. **Loopback & Named Pipe Isolation:**
   - IPC lokal dibatasi hanya pada user akun OS yang sama (permissions `0600` untuk Unix domain socket dan DACL restricted pada Windows Named Pipes).
2. **Sub-agent Privilege Demotion:**
   - Sub-agent atau tool yang di-spawn oleh `shared-operator` **tidak otomatis** mewarisi hak akses tak terbatas operator.
   - Resource gate sub-agent (`BL-15`), capability filter tool (`BL-17`), dan approval gate (`BG-A1/A2`) tetap aktif untuk melindungi host dari eksploitasi prompt injection atau model hallucinations.
3. **No Credential Leakage:**
   - Token API LLM (OpenAI, Anthropic, Gemini) disimpan dalam credential vault lokal dan tidak pernah diekspos ke model context atau sub-agent unprivileged.

---

## 4. Hubungan dengan Multi-Principal (Sprint 0 BG-C1/C2/C3)

Pada konfigurasi multi-tenant / enterprise (OIDC, OAuth, Webhook ingress):
- Identitas ditransformasikan dari verified JWT claims menjadi `PrincipalId::for_oidc(issuer, sub)` atau API key scoped principal.
- `PrincipalId::shared_operator()` dinonaktifkan atau hanya diizinkan untuk root console session (break-glass local recovery) yang membutuhkan flag `--allow-local-operator`.
- Semua data memori, session state, dan approval queue di-namespace berdasarkan `PrincipalId` unik masing-masing.

---

## 5. Ringkasan Prinsip Keamanan

| Prinsip | Implementasi di Galleon |
| :--- | :--- |
| **Fail-Closed** | Tanpa token pairing atau IPC DACL yang sah, koneksi klien ditolak |
| **No Implicit Privilege** | `shared-operator` tetap divalidasi lewat `ResolvedGrants` |
| **Audit Traceability** | Semua tindakan runtime dicatat dengan `principal_id: "shared-operator"` |
| **Portabilitas Trait** | API `Memory`, `Runtime`, dan `Gateway` memakai interface `PrincipalId` yang sama persis |
