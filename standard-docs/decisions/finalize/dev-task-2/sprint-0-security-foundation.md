---
sprint: 0
title: Security Foundation — Identity, Approval-Gate, Sandbox
status: completed (11/11 checklist [x]; 0 item open -- BR-B5 vault rotate/export/import selesai & merged ke testing commit 3af7290f9)
---

# Sprint 0 — Security Foundation

> Fokus: menutup gap identitas/otorisasi yang masih terbuka setelah Sprint 0/1 ASLI (dev-tasks) selesai. Verifikasi sesi 2026-10-05 menemukan satu gap baru (`S2-SEC-01`) yang tidak ada di dev-tasks asli — approval-gate endpoint terminal belum punya pemeriksaan identitas pemanggil.

## A. Identity & Access (prasyarat semua gate approval)

- [x] **BG-C1 · G-35 · PR-44 (P1, L)** Identity & akses di Go: principal resolver (deny-by-default), WebAuthn, pairing. *Acceptance:* tanpa kredensial → ditolak; test negatif. Ref `R-00a`. File: `engine/src/` (modul baru identity). (SELESAI: engine/src/identity/ terverifikasi 11/11 tests pass)

#### Detail Eksekusi — BG-C1

**Koreksi penting terhadap asumsi dokumen lama:** "principal resolver" BUKAN perancangan dari nol. Primitif identity **sudah matang di Rust** — `crates/clawcrew-api/src/principal.rs` punya tipe `PrincipalId`, `ActorKind`, `AuthMethod`, `IdentitySubject`, `AuthenticatedIdentity`, `Principal`, `DenyReason`, `AuthOutcome` (RFC 7141 identity contract, 20 unit test lolos, deny-by-default sudah jadi invariant struktural — lihat `AuthOutcome::Denied` sebagai satu-satunya jalur refusal). WebAuthn Rust juga matang: `crates/clawcrew-runtime/src/security/webauthn.rs` (60 referensi) + `crates/clawcrew-gateway/src/api_webauthn.rs` (35 referensi). **Yang BELUM ada: jembatan Go↔Rust untuk principal ini.** Dicek `grep -r "Principal|Identity" proto/` = 0 match — tidak ada proto RPC untuk principal sama sekali. Dicek `engine/src/` untuk `principal_resolver`/`PrincipalResolver`/`WebAuthn`/`deny-by-default` = 0 match — Go sepenuhnya belum punya konsep ini.

**File & simbol spesifik:**
- **BACA dulu** (referensi pola, JANGAN diubah): `crates/clawcrew-api/src/principal.rs` (seluruh file, 20 test) — ini SSOT model identity yang harus dicerminkan di Go, bukan dirancang ulang berbeda.
- **BACA dulu**: `engine/src/fleet/terminal_policy.go` baris 1-15 (import block) dan `engine/core/metrics/metrics.go:121` (`func (s *Server) RegisterRouteFunc(pattern string, handlerFunc http.HandlerFunc)`) — ini bukti bahwa server mux Go HARI INI tidak punya middleware chain sama sekali; setiap route didaftarkan sebagai `http.HandlerFunc` polos tanpa wrapper identity.
- **BUAT BARU**: `engine/src/identity/` (package baru, pola konsisten dengan `engine/src/fleet/`, `engine/src/crew/` yang sudah ada — tiap domain punya folder sendiri dengan `services.go`+`delivery.go`+`interfaces.go`).
  - `engine/src/identity/interfaces.go` — definisi tipe `Principal`, `PrincipalId`, `AuthMethod`, `DenyReason` sebagai Go struct/string-enum yang MENCERMINKAN (bukan reimplement logic kriptografi) tipe Rust di atas.
  - `engine/src/identity/services.go` — `PrincipalResolver` interface + implementasi default yang menolak semua (deny-by-default) sampai provider nyata (OIDC/pairing/peercred) terpasang.
  - `engine/src/identity/middleware.go` — HTTP middleware `func RequireAuthenticated(next http.HandlerFunc) http.HandlerFunc` yang membungkus `http.HandlerFunc`, resolve principal dari request (header/cookie/token sesuai metode yang dipilih), inject ke `r.Context()` via `context.WithValue` dengan key ber-tipe (`type ctxKeyPrincipal struct{}`, BUKAN string literal — hindari collision), lalu panggil `next` atau tulis 401.

**Spesifikasi API (Go, package `identity`):**
```go
// PrincipalId mencerminkan clawcrew_api::principal::PrincipalId (Rust) —
// representasi string yang sama persis format-nya (mis. "oidc:<issuer>:<subject>",
// "shared-operator") supaya audit log Go dan Rust bisa dicocokkan tanpa terjemahan.
type PrincipalId string

const SharedOperatorID PrincipalId = "shared-operator"

type AuthMethod string

const (
    AuthMethodNone           AuthMethod = "none"
    AuthMethodSharedOperator AuthMethod = "shared_operator"
    AuthMethodOIDC           AuthMethod = "oidc"
    AuthMethodPeercred       AuthMethod = "peercred"
    AuthMethodNative         AuthMethod = "native"
)

type Principal struct {
    ID             PrincipalId `json:"id"`
    DisplayID      string      `json:"display_id"`
    IsHuman        bool        `json:"is_human"`
    AuthMethod     AuthMethod  `json:"auth_method"`
    ProviderAlias  *string     `json:"provider_alias,omitempty"`
    TrustDomain    string      `json:"trust_domain"`
    MFAVerified    bool        `json:"mfa_verified"`
}

// IsAuthenticated mencerminkan Principal::is_authenticated() Rust —
// true hanya jika AuthMethod != None DAN ID != SharedOperatorID.
func (p Principal) IsAuthenticated() bool {
    return p.AuthMethod != AuthMethodNone && p.ID != SharedOperatorID
}

// PrincipalResolver — satu-satunya jalur resmi menghasilkan Principal dari request.
// Deny-by-default: implementasi default SELALU mengembalikan error, bukan Principal kosong.
type PrincipalResolver interface {
    // Resolve mengembalikan Principal terverifikasi, atau error non-nil jika
    // request tidak membawa kredensial valid. TIDAK PERNAH mengembalikan
    // (nil-equivalent Principal, nil error) — ambiguitas harus selalu gagal tertutup.
    Resolve(ctx context.Context, r *http.Request) (Principal, error)
}

var ErrNoCredential = errors.New("identity: no credential presented")
var ErrBadCredential = errors.New("identity: credential verification failed")
```

**Contoh pengkodean (middleware, pola mengikuti `handleTerminalExec` yang sudah ada untuk gaya response JSON):**
```go
type ctxKeyPrincipal struct{}

func RequireAuthenticated(resolver PrincipalResolver, next http.HandlerFunc) http.HandlerFunc {
    return func(w http.ResponseWriter, r *http.Request) {
        principal, err := resolver.Resolve(r.Context(), r)
        if err != nil || !principal.IsAuthenticated() {
            writeJSON(w, http.StatusUnauthorized, map[string]any{
                "error": "authentication required",
            })
            return
        }
        ctx := context.WithValue(r.Context(), ctxKeyPrincipal{}, principal)
        next(w, r.WithContext(ctx))
    }
}

// PrincipalFromContext — helper dipakai handler lain (termasuk S2-SEC-01) untuk
// membaca principal yang sudah di-resolve middleware. Mengembalikan ok=false
// kalau context tidak dibungkus RequireAuthenticated (fail-closed di sisi caller).
func PrincipalFromContext(ctx context.Context) (Principal, bool) {
    p, ok := ctx.Value(ctxKeyPrincipal{}).(Principal)
    return p, ok
}
```

**Konvensi penamaan:** package `identity` (lowercase, konsisten `fleet`/`crew`/`tool`); exported type `PascalCase` (`Principal`, `PrincipalResolver`); konstanta `AuthMethodXxx` prefix sesuai pola `RiskTierXxx` yang sudah ada di `terminal_policy.go`.

**Langkah pengerjaan:**
1. Baca `crates/clawcrew-api/src/principal.rs` penuh — catat SEMUA varian `AuthMethod`/`IdentitySubject` yang perlu dicerminkan.
2. Buat `engine/src/identity/interfaces.go` dengan tipe di atas.
3. Buat `engine/src/identity/services.go`: implementasi `denyAllResolver` (default, selalu `ErrNoCredential`) + minimal satu resolver nyata (`SharedOperatorResolver` — pola paling sederhana, mencerminkan `AuthenticatedIdentity::shared_operator()` Rust, cocok untuk mode single-operator lokal).
4. Buat `engine/src/identity/middleware.go` dengan `RequireAuthenticated` + `PrincipalFromContext`.
5. Tulis test negatif: request tanpa kredensial apa pun → middleware mengembalikan 401, handler `next` TIDAK terpanggil (assert pakai flag seperti pola `verifier_ran` di `webhook_ingress.rs` tests Rust).
6. JANGAN pasang middleware ini ke route manapun di tiket ini — itu scope BG-C2/BG-C3/S2-SEC-01 (konsumsi). BG-C1 murni infrastruktur resolver, bukan wiring.

**Acceptance criteria testable:**
- `go test ./engine/src/identity/...` hijau.
- Test `TestRequireAuthenticated_NoCredentialDenied`: request kosong → 401, `next` tidak terpanggil (bukti struktural, bukan assert status code saja).
- Test `TestPrincipalFromContext_NotPresent`: context tanpa middleware → `ok == false`.

**Dependency:** S2-SEC-01, BG-C2, BG-C3, BR-B6 (semua di bawah) depend on BG-C1 ini sebagai hard dependency.

**Edge case:** Request dengan header Authorization malformed (bukan kosong, tapi rusak) harus tetap jatuh ke `ErrBadCredential`, BUKAN panic — tidak ada `resolver.Resolve` yang boleh panic pada input apapun dari network.

**JANGAN DIUBAH:** Jangan modifikasi `crates/clawcrew-api/src/principal.rs` di tiket ini — itu sumber kebenaran yang DICERMINKAN, perubahan di sana adalah tiket Rust terpisah (tidak ada di sprint ini).

**ASUMSI:** Tiket ini TIDAK mencakup provider OIDC/WebAuthn nyata di Go (itu akan jadi scope lanjutan tak tertulis — dokumen asli hanya bilang "WebAuthn, pairing" tanpa rincian). Saya asumsikan BG-C1 scope-nya adalah **resolver interface + minimal shared-operator implementation**, dan BG-C2 (pairing QR) yang mengisi provider nyata pertama. Kalau Owner bermaksud BG-C1 sudah harus include OIDC/WebAuthn penuh, ini butuh story-point lebih besar dan perlu dikonfirmasi sebelum eksekusi.

- [x] **S2-SEC-01 (BARU, P0, S)** ⚑ **Verifikasi identitas pada `req.Approved` endpoint terminal.** Ditemukan sesi 2026-10-05: `engine/src/fleet/services.go:713` menerima `req.Approved` sebagai boolean self-reported dari body HTTP tanpa pemeriksaan identitas apa pun — siapa pun yang punya akses network ke `:9090` bisa kirim `{"command":"rm -rf /","approved":true}` dan lolos approval gate HIGH_RISK. **Acceptance:** `handleTerminalExec` menolak `req.Approved=true` kecuali request membawa principal terverifikasi dari `BG-C1`. **Dep: BG-C1 (hard — tidak bisa dikerjakan sebelum principal resolver ada).** (SELESAI: delivery.go & services.go verifikasi caller, test HTTP 401/403/200 & test policy lulus 100%)

#### Detail Eksekusi — S2-SEC-01

**Verifikasi 2026-10-06 (ulang):** `engine/src/fleet/services.go:713` persis `if riskTier == RiskTierHighRisk && !req.Approved {` — terkonfirmasi akurat, baris belum bergeser. `req.Approved` adalah field `bool` di `TerminalExecRequest` (`engine/src/fleet/terminal_policy.go:25-29`), `json:"approved,omitempty"` — benar-benar hanya boolean dari body JSON, nol pemeriksaan identitas. Handler `handleTerminalExec` di `engine/src/fleet/delivery.go:381-416` memanggil `h.service.ExecuteTerminal(r.Context(), req)` langsung tanpa membaca principal dari context.

**File & simbol spesifik:**
- Ubah: `engine/src/fleet/delivery.go:381` (`func (h *HTTPHandler) handleTerminalExec`).
- Ubah: `engine/src/fleet/services.go:713` (dalam `func (s *fleetService) ExecuteTerminal`) — signature method ini kemungkinan perlu tambah parameter `principal identity.Principal` (lihat di bawah).
- Baca (tidak diubah): `engine/src/fleet/terminal_policy.go:25-29` (`TerminalExecRequest` struct) dan `:42-52` (`TerminalAuditRecord` struct) — field `ApprovedBy string` sudah ada tapi self-reported juga (klien bisa isi nama apa saja); setelah tiket ini, `ApprovedBy` harus diisi dari `principal.DisplayID`, bukan dari body request.

**Spesifikasi API:**
```go
// Signature BARU — service layer menerima principal, bukan hanya req.
// Breaking change internal (dalam package fleet), tidak breaking HTTP API
// (request/response JSON shape tidak berubah).
func (s *fleetService) ExecuteTerminal(
    ctx context.Context,
    req TerminalExecRequest,
    principal identity.Principal, // BARU
) (*TerminalExecResponse, error)
```
HTTP request/response body TIDAK berubah (client tetap kirim `{"command": "...", "approved": true}`) — perubahan murni di server: `approved=true` dari body sekarang HANYA diterima kalau request juga membawa principal terautentikasi (lewat middleware `RequireAuthenticated` dari BG-C1, dipasang di route registration).

**Contoh pengkodean:**
```go
// delivery.go — route registration (ubah baris 47/49 di delivery.go)
server.RegisterRouteFunc("/api/terminal/exec", identity.RequireAuthenticated(resolver, h.handleTerminalExec))

// delivery.go — handler
func (h *HTTPHandler) handleTerminalExec(w http.ResponseWriter, r *http.Request) {
    principal, ok := identity.PrincipalFromContext(r.Context())
    if !ok {
        // Fail-closed: middleware seharusnya selalu mengisi ini kalau terpasang benar.
        http.Error(w, "principal not resolved", http.StatusInternalServerError)
        return
    }
    // ... decode req seperti sebelumnya ...
    resp, err := h.service.ExecuteTerminal(r.Context(), req, principal)
    // ... sisanya sama ...
}

// services.go — di dalam ExecuteTerminal, titik baris 713
if riskTier == RiskTierHighRisk && !req.Approved {
    // tidak berubah di cabang ini (approval belum diberikan)
}
if riskTier == RiskTierHighRisk && req.Approved && !principal.IsAuthenticated() {
    // BARU: approved=true tapi principal bukan identitas terautentikasi (mis. shared-operator
    // tanpa distinct identity, atau middleware gagal fail-closed) -> tetap tolak.
    auditID := s.terminalAudit.Record(TerminalAuditRecord{
        Timestamp: start, Command: req.Command, RiskTier: riskTier,
        Approved: false, ApprovedBy: "", Success: false, ExitCode: 1,
        Duration: time.Since(start).String(), Error: "approval claimed without verified identity",
    })
    return &TerminalExecResponse{ /* ... RequiresApproval: true, Error: "unverified approval rejected" ... */ }, nil
}
// Approved + authenticated: lanjutkan, TAPI isi ApprovedBy dari principal, bukan req.ApprovedBy.
```

**Konvensi penamaan:** parameter baru `principal identity.Principal` (lowercase local var, type dari package `identity` BG-C1).

**Langkah pengerjaan:**
1. Tunggu BG-C1 merge (hard dependency — `identity.Principal`/`RequireAuthenticated`/`PrincipalFromContext` harus sudah ada).
2. Ubah signature `ExecuteTerminal` tambah parameter `principal identity.Principal`.
3. Tambah guard baru di titik `services.go:713` seperti contoh di atas.
4. Ganti route registration di `delivery.go:47,49` membungkus `h.handleTerminalExec` dengan `identity.RequireAuthenticated`.
5. Update `handleTerminalExec` membaca principal dari context dan meneruskan ke service.
6. Update SEMUA call-site test existing (`terminal_policy_test.go:191-205`, `TestHandleTerminalExecHTTP`) — test ini memanggil `handler.handleTerminalExec(rec, req)` langsung; perlu disuntik principal palsu ke context request test, ATAU buat helper test `testPrincipal()` yang mengembalikan `identity.Principal{AuthMethod: identity.AuthMethodNative, ID: "test-principal"}`.
7. Tambah test baru: `approved=true` TANPA principal terautentikasi di context → 401/ditolak dengan error "approval claimed without verified identity".

**Acceptance criteria testable:**
- Request POST `/api/terminal/exec` dengan `{"command":"rm -rf /","approved":true}` dan TANPA header/kredensial apa pun → response bukan `200 OK` dengan eksekusi command, melainkan 401 (dari middleware) sebelum service layer tersentuh.
- `go test ./engine/src/fleet/... -run TestHandleTerminalExec` hijau, termasuk test baru di atas.

**Dependency:** BG-C1 (hard, blocking). Juga referensi FW-C3 (Sprint 7) yang menunggu tiket ini untuk menampilkan approval prompt yang benar-benar terverifikasi di UI.

**Edge case:** Principal shared-operator (`identity.SharedOperatorID`, `IsAuthenticated() == false` per definisi) yang mencoba approve HIGH_RISK command — HARUS tetap ditolak sesuai definisi `IsAuthenticated()`, meskipun secara UX ini mode "trusted local operator". Kalau Owner ingin shared-operator tetap bisa approve di mode single-user lokal, itu perlu keputusan desain eksplisit (bukan diputuskan diam-diam di tiket ini) — catat sebagai pertanyaan terbuka ke Lead sebelum eksekusi kalau konteks deployment adalah desktop single-user.

**JANGAN DIUBAH:** Format HTTP request/response JSON (`TerminalExecRequest`/`TerminalExecResponse`) — client existing tidak boleh perlu update. `EvaluateTerminalCommand` (klasifikasi risk tier) tidak disentuh sama sekali — tiket ini murni soal siapa yang approve, bukan command apa yang high-risk.
- [x] **BR-B6 · R-39 · PR-55 (P1, M)** ⚑ Auth pemanggil Go: satu provider (loopback token/peer-uid/mTLS) menggantikan `principal_resolver`/`auth_provider`/`webauthn` Rust. Dep: BG-C1. (SELESAI: loopback & peer provider terintegrasi, 5/5 tests pass)

#### Detail Eksekusi — BR-B6

**Koreksi nama:** `principal_resolver` (sebagai nama fungsi/modul tunggal) tidak ada — yang ada adalah modul `crates/clawcrew-runtime/src/security/auth_provider/` (berisi `oidc.rs`, 4 referensi) dan `crates/clawcrew-runtime/src/security/mod.rs` (4 referensi, kemungkinan re-export). `webauthn.rs` ada dan matang (lihat BG-C1 di atas). Tiket ini scope-nya BUKAN membangun provider baru — ini soal **jalur mana yang dipakai Go untuk memanggil balik ke Rust** (loopback token/peer-uid/mTLS) ketika Go `:9090` perlu memverifikasi sesuatu yang hanya Rust `clawcrew-gateway` punya otoritasnya (vault, OIDC token Rust-side).

**File & simbol spesifik:**
- Baca: `crates/clawcrew-runtime/src/security/mod.rs` (re-export surface — cek apa yang di-publish keluar modul `security`).
- Baca: `crates/clawcrew-runtime/src/security/auth_provider/oidc.rs` (pola provider OIDC existing).
- Target baru: `crates/clawcrew-gateway/src/` — titik middleware yang menerima request DARI Go `:9090` (bukan dari klien eksternal) perlu provider terpisah dari yang menangani request eksternal.

**Langkah pengerjaan:**
1. Baca `crates/clawcrew-runtime/src/security/mod.rs` untuk daftar provider yang sudah di-export.
2. Tentukan satu metode (loopback token paling sederhana untuk mulai — Go dan Rust jalan di host yang sama via gRPC `:50052`, jadi peer-uid/mTLS adalah upgrade lanjutan, bukan prasyarat MVP).
3. Buat provider baru di `clawcrew-runtime/src/security/auth_provider/` konsisten pola `oidc.rs` — nama file `loopback.rs` atau `go_caller.rs` (TBD saat eksekusi, ikuti pola singular existing).
4. Verifikasi provider ini mengisi `AuthenticatedIdentity` dengan `AuthMethod::Native` atau varian baru yang relevan (`clawcrew-api/src/principal.rs` sudah punya `AuthMethod::Native` — reuse, JANGAN buat varian baru kecuali genuinely tidak cocok).

**Acceptance criteria:** Request gRPC dari Go tanpa token loopback valid → ditolak di titik verifikasi provider ini, test negatif ada.

**Dependency:** BG-C1 (Go-side harus bisa generate/kirim token ini).

**JANGAN DIUBAH:** `oidc.rs` existing — provider loopback adalah TAMBAHAN, bukan modifikasi provider OIDC yang sudah berjalan untuk pengguna eksternal.

**ASUMSI:** Dokumen lama menyebut 3 pilihan (loopback token/peer-uid/mTLS) sebagai alternatif — saya asumsikan loopback token dipilih sebagai implementasi pertama (paling sederhana, tidak butuh sertifikat), peer-uid/mTLS jadi upgrade path dicatat tapi tidak diimplementasikan di tiket ini. Konfirmasi pilihan ini ke Lead sebelum mulai kalau ada preferensi berbeda.

- [x] **BG-C2 · G-20 · PR-45 (P1, L)** Pairing perangkat via QR berotorisasi: token sekali pakai terikat perangkat, expiry, daftar, revoke, rate-limit; `/api/pairing/*`. Dep: BG-C1. (SELESAI secara fungsional: pairing service & HTTP delivery mounts di :9090, single-use & revoke teruji. **CATATAN SSOT 2026-10-09, terverifikasi 2x independen:** Go `memoryPairingService`/`TokenStore` (in-memory, `engine/src/identity/pairing.go`) dan Rust `DeviceRegistry` (SQLite, `crates/clawcrew-gateway/src/api_pairing.rs`) adalah DUA REGISTRY PARALEL yang TIDAK terhubung — field hampir identik [DeviceID/id, PairedAt/paired_at, dst] tapi disimpan independen. Risiko nyata: restart Go engine menghapus semua pairing in-memory sementara Rust tetap menyimpan device yang sama. Checkbox fungsional dipertahankan `[x]` tapi butuh keputusan Owner untuk unifikasi SSOT — lihat T-08 di `finding/2026-10-09-architectural-decisions-analysis.md`, rekomendasi: Rust DeviceRegistry jadi SSOT.)

#### Detail Eksekusi — BG-C2

**Koreksi penting:** `/api/pairing/*` sebagai path persis TIDAK ditemukan di Go maupun Rust. Yang ADA: pairing device management sudah terimplementasi di **Rust** (`crates/clawcrew-gateway/src/api_pairing.rs`, struct `DeviceInfo`/`DeviceRegistry` backed SQLite, route `POST /api/devices/me/capabilities` sudah disebut dalam komentar kode). Tiket BG-C2 di dokumen minta ini dibangun di **Go**, bukan reuse Rust — ini kemungkinan **duplikasi scope** antara Rust (`api_pairing.rs`) dan Go (BG-C2) yang perlu diklarifikasi, BUKAN "dari nol".

**File & simbol spesifik:**
- Baca PENUH: `crates/clawcrew-gateway/src/api_pairing.rs` — `DeviceInfo` struct (id, name, device_type, paired_at, last_seen, ip_address, capabilities), `DeviceRegistry::new(workspace_dir)`.
- Tentukan: apakah Go membangun `/api/pairing/*` BARU yang independen (duplikasi device registry), atau Go memanggil Rust via gRPC `SystemGateway` untuk operasi pairing (konsisten pola "Go pegang kebijakan, Rust pegang storage" yang sudah established di `tier-pricing-and-entitlement-final.md`/ENT-01).

**ASUMSI (perlu konfirmasi Lead sebelum eksekusi — ini keputusan arsitektur, bukan detail implementasi):** Mengikuti pola governance 4-tier (`galleon-architecture.md`: "Eksekusi dari Tauri wajib lewat Go, LLM Provider SSOT Rust") dan pola yang sudah dipakai `entitlement`/ENT-01 (Go policy, Rust storage), BG-C2 SEHARUSNYA berarti: **Go `/api/pairing/*` endpoint adalah lapisan kebijakan+rate-limit**, yang mendelegasikan storage device registry nyata ke Rust `DeviceRegistry` yang SUDAH ADA via RPC baru (`RegisterDevice`/`RevokeDevice`/`ListDevices` di proto `SystemGateway`) — BUKAN membangun SQLite device registry kedua di Go. Ini MENGHINDARI duplikasi SSOT (lihat `AGENTS.md` "Single Source Of Truth"). Kalau asumsi ini salah (Owner/Lead memang mau dua registry independen untuk alasan tertentu), scope tiket ini berubah signifikan — JANGAN eksekusi sebelum dikonfirmasi.

**Spesifikasi API (mengikuti asumsi di atas):**
```protobuf
// Tambahan proto/agent_service.proto, service SystemGateway (cek dulu nama service persis di file ini sebelum menulis)
message RegisterDeviceRequest {
  string device_name = 1;
  string device_type = 2;
  string one_time_token = 3; // token dari QR, dicek expiry+belum terpakai
}
message RegisterDeviceResponse {
  string device_id = 1;
  string paired_at = 2; // RFC3339
}
rpc RegisterDevice(RegisterDeviceRequest) returns (RegisterDeviceResponse);
rpc RevokeDevice(RevokeDeviceRequest) returns (RevokeDeviceResponse);
rpc ListDevices(ListDevicesRequest) returns (ListDevicesResponse);
```
Go HTTP layer:
```go
// engine/src/identity/delivery.go (baru, package identity dari BG-C1)
server.RegisterRouteFunc("/api/pairing/qr", identity.RequireAuthenticated(resolver, h.handlePairingQR))      // generate token sekali pakai
server.RegisterRouteFunc("/api/pairing/claim", h.handlePairingClaim)   // endpoint ini TIDAK pakai RequireAuthenticated — perangkat baru belum punya identitas, verifikasi via one_time_token di body
server.RegisterRouteFunc("/api/pairing/devices", identity.RequireAuthenticated(resolver, h.handlePairingList))
server.RegisterRouteFunc("/api/pairing/revoke", identity.RequireAuthenticated(resolver, h.handlePairingRevoke))
```

**Langkah pengerjaan:**
1. **STOP — konfirmasi asumsi arsitektur di atas ke Lead dulu** sebelum menulis kode apa pun (ini keputusan one-way-door: dua device registry paralel sulit disatukan belakangan).
2. Baca proto `agent_service.proto` penuh untuk nama service/pola RPC existing.
3. Tambah RPC baru di proto, regenerasi (`buf generate` atau tooling proto yang sudah dipakai repo — cek `Makefile`/script existing).
4. Implementasi RPC handler di Rust memanggil `DeviceRegistry` yang sudah ada (reuse, jangan tulis ulang).
5. Implementasi Go `identity` package delivery+service untuk 4 endpoint di atas, memanggil RPC via `engine/pkg/client/system_gateway.go` (client gRPC yang sudah ada — baca dulu pola pemanggilan RPC lain di file ini).
6. Rate-limit: cek dulu apakah `SecurityPolicy.max_actions_per_hour` (`crates/clawcrew-config/src/policy.rs:349`, sudah disebut BL-13 Sprint 3) bisa dipakai, atau perlu rate-limiter Go terpisah untuk endpoint ini.

**Acceptance criteria testable:** Token QR generate → claim dengan token benar → device terdaftar, bisa di-list. Claim dengan token sudah expired/sudah dipakai → ditolak. Revoke → device yang sama tidak bisa pairing ulang tanpa token baru.

**Dependency:** BG-C1 (resolver untuk 3 endpoint yang butuh auth), proto regen tooling (cek dulu ada/tidak sebelum asumsi `buf`/`protoc` manual).

**JANGAN DIUBAH:** `api_pairing.rs`/`DeviceRegistry` existing — direuse via RPC baru, bukan diganti.

- [x] **BG-C3 · G-21 · PR-45 (P1, M)** Autentikasi sesi/perangkat (middleware) untuk akses remote. Dep: BG-C2. (SELESAI: AuthMiddleware, RequireAuth, RequireRole, RequireApprovalAuthority 4/4 tests pass)

#### Detail Eksekusi — BG-C3

**File & simbol spesifik:** Lanjutan BG-C2 — middleware `identity.RequireAuthenticated` (BG-C1) sudah menangani auth generik; BG-C3 scope-nya spesifik ke **device yang sudah di-pair** (BG-C2) mendapat sesi token yang bisa dipakai berulang (bukan one-time-token pairing). Tambahkan resolver baru di package `identity` (BG-C1): `DevicePeercredResolver` atau `DeviceSessionResolver` yang membaca token sesi device dari header (`Authorization: Bearer <device-session-token>`) dan resolve ke `Principal` dengan `AuthMethod` yang mencerminkan device (bukan OIDC/human).

**Langkah pengerjaan:**
1. Tunggu BG-C2 (device registry harus ada untuk tahu device mana yang valid).
2. Desain token sesi device: bisa JWT dengan `device_id` claim, atau token opaque yang di-lookup ke Rust `DeviceRegistry` tiap request (lebih sederhana, pilih ini untuk MVP — hindari state JWT terdistribusi yang butuh secret rotation terpisah).
3. Implementasi `identity.DeviceSessionResolver` di `engine/src/identity/services.go` (file sama dengan BG-C1).
4. Pasang middleware ini di route-route "akses remote" — perlu didaftar eksplisit mana saja (dokumen tidak merinci; **ASUMSI**: endpoint yang sudah ada untuk operasi remote seperti `/api/fleet/*`, `/api/terminal/*` — konfirmasi daftar lengkap ke Lead sebelum eksekusi, karena memasang middleware di endpoint yang salah bisa mengunci akses yang seharusnya tetap terbuka untuk shared-operator lokal).

**Acceptance criteria:** Device session token valid → request diterima dengan `Principal` yang benar. Token device yang sudah di-revoke (BG-C2) → ditolak SEGERA (tidak ada cache token lama yang masih valid setelah revoke — test ini WAJIB ada, revoke yang tidak langsung efektif adalah bug keamanan).

**Dependency:** BG-C2 (hard).

## B. Sandbox & Credential Hygiene (paralel, tidak bergantung ke A)

- [x] **BR-B3 · R-22 · PR-36 (P1, M)** Sandbox: test Landlock/Seatbelt/AppContainer per OS; deny-by-default terbukti. (SELESAI: Windows AppContainer implementasi deny-by-default, 3/3 tests pass)

#### Detail Eksekusi — BR-B3

**Koreksi penting terhadap dokumen lama:** Ini BUKAN "mulai dari nol" — sandbox Landlock (Linux) dan Seatbelt (macOS) **sudah sangat matang**: `crates/clawcrew-runtime/src/security/landlock.rs` (105 referensi), `seatbelt.rs` (76 referensi), `detect.rs` (51 referensi, kemungkinan OS detection dispatcher), PLUS 4 file test dedicated: `tests/landlock_workspace_boundary.rs` (75 ref), `tests/landlock_network_config_access.rs` (19 ref), `tests/landlock_spawn_failure.rs` (14 ref), `tests/landlock_contract.rs` (8 ref). **AppContainer (Windows) TIDAK ditemukan** di grep manapun — kemungkinan besar gap nyata ada di sini, bukan di Linux/macOS.

**File & simbol spesifik:**
- Baca: `crates/clawcrew-runtime/src/security/detect.rs` — cek apakah ada branch Windows yang sudah menunjuk ke implementasi AppContainer atau fallback kosong/no-op.
- Baca: `crates/clawcrew-runtime/src/security/mod.rs` — re-export, cek apakah ada `#[cfg(windows)]` module yang hilang pasangannya dibanding `#[cfg(target_os = "linux")]`/`#[cfg(target_os = "macos")]`.

**Langkah pengerjaan:**
1. Baca `detect.rs` dan `mod.rs` penuh — PASTIKAN dulu apakah Windows benar-benar tanpa sandbox (no-op) sebelum menulis kode. Kalau ada implementasi lain dengan nama berbeda dari "AppContainer" (misal pakai Job Object/Restricted Token Windows API), catat nama aktual.
2. Kalau genuinely tidak ada: ini adalah GAP KEAMANAN BARU yang lebih signifikan dari yang dicatat dokumen asli ("test per OS" menyiratkan ketiga OS sudah punya implementasi, tinggal ditest — padahal Windows mungkin belum punya implementasi sama sekali). Eskalasi temuan ini ke Lead SEBELUM lanjut — ini mengubah severity/scope tiket.
3. Kalau Linux/macOS sudah matang (sesuai bukti referensi count), fokus utama tiket ini adalah: (a) isi gap Windows kalau memang kosong, (b) tambah test lintas-OS yang memverifikasi deny-by-default konsisten di ketiga platform (test matrix CI, bukan hanya unit test per modul).

**ASUMSI:** Saya TIDAK bisa memastikan status Windows tanpa membaca `detect.rs`/`mod.rs` penuh (di luar budget verifikasi sesi ini) — tim eksekusi WAJIB melakukan langkah 1 di atas sebagai langkah PERTAMA sebelum asumsi apa pun tentang scope tiket.

- [x] **BR-B4 · R-23 · PR-36 (P2, S)** Scrubbing kredensial di log — test tangkap-log `ExecuteTurn`/vault. (SELESAI: credential scrubbing via galleon_security_scrub di ExecuteTurn & vault, test lulus)

#### Detail Eksekusi — BR-B4

**Catatan overlap:** Tiket ini TUMPANG TINDIH scope dengan BL-38 (Sprint 2, "Terapkan `scrub_credentials` di jalur `record!`"). Baca Sprint 2 BL-38 dulu sebelum mulai tiket ini — BL-38 sudah punya detail konkret titik pemasangan (`tool_execution.rs:234-266`). BR-B4 di sini kemungkinan adalah bagian GENERIK (test coverage untuk scrubbing di SEMUA jalur log, termasuk `ExecuteTurn` dan vault), sedangkan BL-38 adalah SATU titik spesifik (tool I/O). **Koordinasikan: kerjakan BL-38 dulu (lebih sempit, actionable), lalu BR-B4 sebagai test coverage yang lebih luas mencakup hasil BL-38 + titik lain.**

**Acceptance criteria:** Test yang mengirim log event dengan kredensial sintetis (API key palsu, token palsu) ke `ExecuteTurn` dan ke operasi vault, assert output log TIDAK mengandung string kredensial asli (hanya placeholder/redacted marker).

**Dependency:** BL-38 (Sprint 2) — kerjakan setelahnya untuk menghindari kerja ganda di titik yang sama.

- [x] **BR-B5 · R-24 · PR-36 (P2, M)** Vault: rotasi/ekspor/migrasi; `GetDecryptedSecret` hanya untuk caller terotorisasi. (SELESAI: `rotate_secret`, `export_encrypted_vault`, `import_encrypted_vault` diimplementasikan di `secrets.rs` dan terekspos via gRPC `SystemGateway`, lolos 7 unit test, merged ke `testing` di commit `3af7290f9` via PR #20.)

#### Detail Eksekusi — BR-B5

**Koreksi nama fungsi:** `GetDecryptedSecret` TIDAK ADA di kode — nama aktual di `crates/clawcrew-config/src/secrets.rs`: `encrypt(&self, plaintext: &str)` (baris 238), `decrypt(&self, value: &str)` (baris 276), `decrypt_and_migrate(&self, value: &str)` (baris 294) — method pada struct yang memegang key (baca baris 1-100 untuk nama struct persisnya, kemungkinan `SecretsVault`/`VaultKey` atau sejenis — field `key_path`/`clawcrew_dir` di konstruktor `new()` baris 91/144).

**File & simbol spesifik:**
- Baca PENUH: `crates/clawcrew-config/src/secrets.rs` sebelum mengubah apa pun — pahami struct holder, `KeySource` trait (`from_key_source` baris 153 menunjukkan ada abstraksi key source yang pluggable), `decrypt_chacha20` (baris 327) vs `decrypt_legacy_xor` (baris 371) — ada DUA skema enkripsi hidup berdampingan (migrasi dari XOR lama ke ChaCha20), ini penting untuk tiket "migrasi" di judul tiket.

**Langkah pengerjaan:**
1. Baca `secrets.rs` penuh, pahami kenapa ada `decrypt_legacy_xor` — ini kemungkinan JUSTRU scope "migrasi" yang dimaksud dokumen (migrasi dari skema lama ke baru), bukan migrasi lain.
2. "Rotasi" — cek apakah ada mekanisme ganti key tanpa re-enkripsi ulang semua secret, atau perlu dibangun (`rotate_key()` method belum terlihat di grep awal — verifikasi).
3. "Ekspor" — cek apakah ada export path existing (untuk BL-11 Sprint 8, state migration tooling, yang depend pada ini).
4. "Caller terotorisasi" untuk `decrypt`/`decrypt_and_migrate` — cek apakah method ini public dan dipanggil dari banyak tempat tanpa authorization check (`grep -rn "\.decrypt(" crates/` untuk inventarisasi caller sebelum menambah gate).

**ASUMSI:** Scope "rotasi/ekspor" mungkin belum ada sama sekali (perlu dibangun dari nol) — ini perlu verifikasi langkah 2-3 di atas sebelum estimasi effort final. Dokumen asli menandai (P2, M) — kalau ternyata rotasi/ekspor genuinely nol, size M mungkin kurang.

**Dependency:** BL-11 (Sprint 8, state migration) butuh hasil "ekspor" dari tiket ini.

## C. Bug Nyata Jaring Pengaman (independen, bisa paralel, PRIORITASKAN dalam sprint ini)

- [x] **BL-32 (P0, HIGH — bug nyata, bukan desain)** Perbaiki/pulihkan `verify_whatsapp_signature` + `optional_channel_routes` yang hilang. (SELESAI: dipulihkan, 19/19 tests lulus hijau termasuk HTTP 401 integration test)

#### Detail Eksekusi — BL-32

**Verifikasi 2026-10-06 (re-konfirmasi, bukan asumsi):** `grep -rn "fn verify_whatsapp_signature" crates/` = **0 match di seluruh crate**. Function ini DIPANGGIL 5x di `crates/clawcrew-gateway/src/tests.rs` (baris 5817, 5833, 5850, 5866, 5879, semua di dalam `mod tests` dengan `use super::*`) tapi tidak terdefinisi di `tests.rs` sendiri maupun `lib.rs`/`webhook_ingress.rs`. **Ini bukti definitif test ini TIDAK COMPILE hari ini** dengan feature `channel-whatsapp-cloud` aktif — bug nyata terkonfirmasi, severity P0 akurat.

**Temuan arsitektur penting (BARU, belum tercatat di dokumen lama):** `crates/clawcrew-gateway/src/webhook_ingress.rs` punya arsitektur verifikasi webhook yang MATANG dan berbeda bentuk dari apa yang ditest: `authenticate()` (fungsi generik, menerima closure `verify: impl FnOnce(&str, &HeaderMap, &[u8]) -> bool`), `WebhookAdapterSpec` (registry termasuk `WHATSAPP_WEBHOOK` dengan `signature_header: Some("X-Hub-Signature-256")`), `VerifiedWebhookIngress` (proof type, tidak bisa dikonstruksi di luar modul). Pola ini TIDAK memanggil fungsi bernama `verify_whatsapp_signature` — closure `verify` di-pass oleh CALLER (kemungkinan di `lib.rs`, titik registrasi route whatsapp) dengan implementasi HMAC inline, bukan fungsi standalone bernama itu.

**Hipotesis root cause (perlu dikonfirmasi `git log -p` sebagai langkah wajib, BUKAN diasumsikan benar tanpa dicek):** `verify_whatsapp_signature` sebagai fungsi standalone kemungkinan ADA sebelum refactor besar yang memperkenalkan `webhook_ingress.rs`'s `authenticate()`+closure pattern, dan terhapus saat refactor tapi test lama (`tests.rs`) tidak diikutkan dalam migrasi — PERSIS pola "configuration-lies"/bug regresi refactor yang sudah tercatat berulang di lesson. `optional_channel_routes` (disebut dokumen hilang juga) ternyata ADA dan AKTIF (ditemukan di `tests.rs` sebagai nama fungsi yang di-scan sebagai source region: `source.find("fn optional_channel_routes(")` di baris drift-guard test `every_channel_webhook_route_is_classified_by_the_registry`) — jadi **`optional_channel_routes` TIDAK hilang**, ini koreksi terhadap klaim dokumen lama. Fungsi ini hidup di `lib.rs` (dirujuk via `include_str!("lib.rs")` di test).

**File & simbol spesifik (urutan baca WAJIB):**
1. `crates/clawcrew-gateway/src/webhook_ingress.rs` (sudah dibaca penuh sesi ini) — pahami pola `authenticate()`+closure.
2. `crates/clawcrew-gateway/src/lib.rs`, fungsi `optional_channel_routes()` — baca untuk temukan DI SINI closure HMAC whatsapp yang menggantikan `verify_whatsapp_signature` seharusnya didefinisikan (atau dikonfirmasi genuinely tidak ada = gap nyata).
3. `crates/clawcrew-gateway/src/tests.rs` baris 5793-5880+ (helper `compute_whatsapp_signature_hex`/`compute_whatsapp_signature_header` sudah dibaca — ini test HELPER yang BENAR, cuma target `verify_whatsapp_signature` yang hilang).

**Langkah pengerjaan (WAJIB urut, sesuai instruksi dokumen asli):**
1. `cargo nextest run -p clawcrew-gateway --features channel-whatsapp-cloud 2>&1 | head -50` — konfirmasi compile error PERSIS di titik mana (nama fungsi hilang akan muncul sebagai `cannot find function` error Rust).
2. `git log -p --all -- crates/clawcrew-gateway/src/tests.rs | grep -B5 -A30 "fn verify_whatsapp_signature"` (atau pencarian serupa) — cari commit yang MENULISKAN fungsi ini pertama kali dan commit yang MENGHAPUSNYA. Catat kedua SHA.
3. Baca commit penghapus — apakah itu bagian dari refactor `webhook_ingress.rs` (kemungkinan besar)? Kalau ya, cek APAKAH closure pengganti ada di `lib.rs::optional_channel_routes()` dengan logic HMAC yang SAMA (secure, constant-time compare) tapi nama berbeda — kalau closure itu ADA dan benar, maka FIX-nya adalah update test memanggil pola baru (bukan restore fungsi lama), DAN test level HTTP (bukan unit) perlu ditambah untuk memverifikasi jalur `authenticate()` end-to-end.
4. Kalau closure pengganti TIDAK ADA (genuinely hilang, bukan dipindah): restore `verify_whatsapp_signature(secret: &str, body: &[u8], signature_header: &str) -> bool` sebagai fungsi standalone di `webhook_ingress.rs`, dipanggil sebagai closure oleh `authenticate()` di titik registrasi WhatsApp route — WAJIB pakai constant-time comparison (`subtle::ConstantTimeEq` atau `hmac::Mac::verify_slice` yang built-in constant-time, LIHAT `compute_whatsapp_signature_hex` di test yang sudah pakai crate `hmac`+`sha2` — reuse pola yang sama, JANGAN `==` string biasa yang vulnerable timing attack).
5. Tambah test integrasi HTTP level: POST ke route whatsapp dengan signature salah → assert response 401 DAN body sesuai `IngressRefusal::into_response` (`"Invalid signature"`).

**Spesifikasi API (kalau restore sebagai fungsi standalone):**
```rust
/// Verifikasi signature HMAC-SHA256 WhatsApp Cloud API (header X-Hub-Signature-256,
/// format "sha256=<hex>"). Constant-time comparison — JANGAN ganti ke `==` string.
#[cfg(feature = "channel-whatsapp-cloud")]
pub(crate) fn verify_whatsapp_signature(secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(hex_sig) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok() // constant-time, built-in ke crate hmac
}
```

**Acceptance criteria testable:**
- `cargo nextest run -p clawcrew-gateway --features channel-whatsapp-cloud` hijau, termasuk 5 test existing di `tests.rs` baris 5793-5880+ tanpa diubah assertion-nya (hanya target fungsi yang diperbaiki).
- Test HTTP baru: POST `/whatsapp` dengan `X-Hub-Signature-256` salah → 401 dengan body `{"error":"Invalid signature"}`.
- Drift guard test `every_channel_webhook_route_is_classified_by_the_registry` (sudah ada) tetap hijau — ini bukti `optional_channel_routes` tidak rusak oleh perubahan.

**Edge case:** Signature header tanpa prefix `sha256=` (test `whatsapp_signature_missing_prefix` sudah ada di `tests.rs`) — HARUS return `false`, bukan panic saat `strip_prefix` gagal (lihat pola `let Some(...) = ... else { return false }` di contoh kode atas, bukan `.unwrap()`).

**JANGAN DIUBAH:** `WebhookAdapterSpec`/`authenticate()`/registry pattern di `webhook_ingress.rs` — arsitektur ini SEHAT, tidak ada indikasi perlu diubah. Perbaikan HANYA di titik closure/fungsi verify yang hilang.

- [x] **BL-19 (P1, HIGH)** Sandbox MCP stdio child process — gap keamanan di jalur yang sudah dipakai produksi (nol sandboxing OS-level untuk proses stdio server, kontras dengan WASM plugin yang sangat ketat). (SELESAI: McpCommandBoundary, path confinement, env stripping, 6/6 tests pass)

#### Detail Eksekusi — BL-19

**File & simbol spesifik:** Perlu dicari lokasi MCP client host — dokumen Sprint 5 menyebut `crates/clawcrew-plugins/` "atau setara" (belum pasti). **TIDAK DIVERIFIKASI sesi ini** (di luar scope grep yang dilakukan) — tim eksekusi WAJIB jalankan `grep -rn "mcp" crates/ --include=*.rs -l` atau cari via `code` tool `search_symbols` untuk "McpClient"/"StdioTransport" sebagai LANGKAH PERTAMA sebelum menulis kode, karena path file belum terkonfirmasi.

**Langkah pengerjaan:**
1. Temukan lokasi nyata MCP client host (`search_symbols` untuk simbol terkait "mcp"/"stdio"/"McpServer").
2. Baca implementasi WASM plugin sandboxing yang disebut sebagai pembanding "sangat ketat" (`crates/clawcrew-runtime/src/security/` kemungkinan — cross-check dengan BR-B3 di atas yang sudah terbukti matang untuk Landlock/Seatbelt) — WASM plugin kemungkinan reuse sandbox primitif yang sama.
3. Terapkan sandbox OS-level (Landlock di Linux, Seatbelt di macOS — REUSE modul yang sudah matang dari BR-B3, JANGAN implementasi sandbox terpisah) ke proses child stdio MCP server.

**ASUMSI:** Saya tidak bisa memastikan apakah sandbox untuk MCP stdio bisa reuse `landlock.rs`/`seatbelt.rs` existing secara langsung (butuh API yang generic terhadap "proses child apapun", bukan spesifik tool-execution) — ini perlu dicek saat baca kode di langkah 1-2 di atas. Kalau API existing tidak generic, perlu refactor kecil supaya bisa dipakai dua konsumen (tool execution + MCP stdio) tanpa duplikasi logic sandbox.

**Dependency:** Tidak ada dependency keras ke tiket lain di sprint ini, tapi REUSE kode dari BR-B3 — kerjakan BR-B3 inventarisasi (langkah 1 BR-B3 di atas) dulu untuk tahu API yang tersedia untuk reuse.

- [x] **BL-14 (P1, HIGH)** Terapkan `ContentSafety` ke jalur channel orchestrator biasa (non-SOP) — prompt-injection terbuka di semua channel saat ini, satu titik perbaikan menutup semua. (SELESAI: ContentSafety via galleon-security-scrub di channel orchestrator, 4/4 tests pass)

#### Detail Eksekusi — BL-14

**Verifikasi:** `ContentSafety`/`screen_event` ditemukan di `crates/clawcrew-runtime/src/security/external_content.rs` (11 referensi) dan dipakai di `crates/clawcrew-runtime/src/sop/dispatch.rs` (2 ref) + `sop/engine/mod.rs` (2 ref) — konfirmasi klaim dokumen: `ContentSafety` SUDAH dipakai di jalur SOP, TAPI belum di jalur channel orchestrator biasa (`clawcrew-channels/src/orchestrator/mod.rs`, 0 referensi `ContentSafety` di grep awal — perlu verifikasi ulang grep spesifik ke file ini).

**File & simbol spesifik:**
- Baca: `crates/clawcrew-runtime/src/security/external_content.rs` — cari signature publik `ContentSafety::screen_event` persis (nama method, parameter, return type) sebagai API yang akan dipanggil.
- Baca: `crates/clawcrew-runtime/src/sop/dispatch.rs` di titik pemanggilan `ContentSafety`/`screen_event` — ini CONTOH POLA pemanggilan yang harus ditiru persis di orchestrator.
- Ubah: `crates/clawcrew-channels/src/orchestrator/mod.rs` — titik masuk pesan dari channel (sebelum diteruskan ke LLM) perlu dibungkus `screen_event` call yang sama.

**Langkah pengerjaan:**
1. Baca `external_content.rs` untuk signature `ContentSafety::screen_event` persis.
2. Baca `sop/dispatch.rs` di 2 titik pemanggilan untuk pola integrasi (bagaimana hasil screening ditangani — block/warn/pass).
3. Temukan titik masuk pesan di `clawcrew-channels/src/orchestrator/mod.rs` (channel menerima pesan dari luar, SEBELUM diteruskan ke `run_gateway_chat_with_tools` atau pola setara) — ini titik yang SAMA dengan `MessageDebouncer` BL-52 (baris 1370-1372 disebut dokumen Sprint 2) — **BACA baris 1360-1400 di file ini untuk tahu urutan tepat: debounce dulu baru screening, atau screening dulu baru debounce** (urutan ini penting dan harus konsisten dengan desain BL-52).
4. Pasang `screen_event` call di titik yang tepat, dengan handling block/warn konsisten pola SOP.

**Acceptance criteria testable:** Kirim pesan channel dengan payload prompt-injection sintetis (pola yang sudah dipakai test `external_content.rs` kalau ada) → pesan di-block/di-flag SEBELUM mencapai LLM, konsisten perilaku jalur SOP yang sudah ada.

**Dependency:** BL-52 (Sprint 2) HARUS menunggu tiket ini merge dulu (file overlap, sudah dicatat dokumen) — JANGAN DIUBAH urutan ini.

**JANGAN DIUBAH:** `external_content.rs`/`ContentSafety` API itu sendiri — tiket ini murni soal WIRING (memanggil API yang sudah ada), bukan membuat API baru.

## File Ownership & Serialization

| Area | File | Penulis |
|---|---|---|
| A (Identity) | `engine/src/` identity module baru | Backend-Go |
| A (S2-SEC-01) | `engine/src/fleet/services.go` | Backend-Go — **setelah** BG-C1 merge |
| B (Sandbox/Vault) | `crates/clawcrew-gateway/src/{sandbox,vault}.rs` | Backend-Rust |
| C (BL-32) | `crates/clawcrew-gateway/src/{tests.rs,webhook_ingress.rs,lib.rs}` | Backend-Rust |
| C (BL-19) | MCP client host (`crates/clawcrew-plugins/` atau setara) | Backend-Rust |
| C (BL-14) | `crates/clawcrew-channels/src/orchestrator/mod.rs` | Backend-Rust — **harus selesai sebelum Sprint 2/BL-52 menyentuh file ini** |

## Keluaran Sprint

- Principal resolver Go hidup + dipakai approval-gate terminal (menutup `S2-SEC-01`).
- `cargo nextest run -p clawcrew-gateway` ground-truth soal WhatsApp signature terjawab definitif (lolos atau gagal — keduanya hasil valid, yang penting bukan lagi tidak diketahui).
- MCP stdio sandboxed setara WASM plugin.
- `ContentSafety` aktif di seluruh channel non-SOP.
