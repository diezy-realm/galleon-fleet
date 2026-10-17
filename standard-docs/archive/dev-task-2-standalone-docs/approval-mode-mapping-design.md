# Desain & Analisis Pemetaan Approval-Mode Per-Chat (BL-71)

## 1. Konteks & Pertanyaan Desain
Tiket **BL-71** mencatat:
> "Toggle approval-mode per-chat (Normal/Reads/Trust/YOLO ala KiroCrew) terhubung ke `RiskTier`/`RiskClass` API yang sudah ada di `approval_gate.go`/`policy.go`. Keputusan terbuka: apakah mode ini dipetakan ke `authority: read_only|draft_only|gated_write` existing di `CrewMember`, atau butuh bentuk baru — jangan tiru UI KiroCrew mentah tanpa cek kesesuaian model."

## 2. Analisis Model Otoritas & Risiko Eksisting

### Model A: Permission Entitas Statis (`CrewMember.authority`)
- Definisi: `read_only | draft_only | gated_write` (terletak di level profil anggota kru).
- Sifat: Statis, terikat pada *peran & kredensial* anggota kru individual.
- Invariant: Anggota kru berkategori `read_only` tidak boleh diizinkan melakukan write/exec meskipun mode obrolan diatur santai.

### Model B: Policy Engine & Risk Tiering (`RiskTier` di `engine/src/fleet/terminal_policy.go`)
- Klasifikasi:
  - `RiskTierRead`: Perintah inspeksi aman (`git status`, `ls`, `grep`, `cat`). Bebas approval.
  - `RiskTierExecute`: Perintah developer standar (`npm test`, `cargo build`). Wajib dicatat.
  - `RiskTierHighRisk`: Perintah destruktif / modifikasi (`rm -rf`, `sudo`, `git reset --hard`). Wajib approval manusia.
  - `RiskTierBlocked`: Perintah terlarang (`:(){ :|:& };:`, format disk, injection). Ditolak tanpa kecuali.

### Model C: Sesi Obrolan Interaktif (`ChatApprovalMode`)
Mode interaksi per sesi obrolan (Journal/Chat):
1. **Normal (Default)**: Mengikuti standar `RiskTier`. Perintah `Execute` & `HighRisk` wajib minta approval.
2. **Reads-Only**: Mengetatkan sesi sehingga seluruh eksekusi dibatasi ke `RiskTierRead`. Aksi `Execute`/`HighRisk` langsung ditolak.
3. **Trust**: Menurunkan friksi untuk `RiskTierExecute` (auto-approve), tetapi `RiskTierHighRisk` TETAP WAJIB approval manusia.
4. **YOLO (Supervised Dev)**: Auto-approve untuk `RiskTierExecute`, namun `RiskTierBlocked` dan `PaymentLockoutGate` TETAP FAIL-CLOSED dan tidak dapat ditembus.

## 3. Keputusan Arsitektur

### Evaluasi Rekomendasi (a vs b):
1. **Opsi (a) - UI-only mapping ke `CrewMember.authority`:**
   - **DITOLAK**. `authority` kru adalah izin batas atas (least-privilege capability cap) yang bersifat persisten. Jika mode chat mengubah `authority`, ada risiko state leak antar sesi atau eskalasi hak istimewa tak sengaja.
2. **Opsi (b) - Dimensi Baru Terpisah (`ChatApprovalMode`):**
   - **DISETUJUI**. Mode approval per-chat adalah *konteks runtime per-turn/per-sesi* yang dibatasi oleh:
     $$\text{Effective Risk Policy} = \text{Strict}(\text{CrewAuthority}, \text{ChatApprovalMode}, \text{PaymentLock})$$
   - Artinya, `ChatApprovalMode` hanya boleh **mempersempit atau memilih alur persetujuan**, BUKAN memperluas privilege melebihi `CrewMember.authority`.

## 4. Spesifikasi Kontrak API & Tipe Data

```typescript
// web/src/types/index.ts
export type ChatApprovalMode = 'normal' | 'reads_only' | 'trust' | 'yolo';

export interface ChatSessionPolicy {
  approvalMode: ChatApprovalMode;
  enforcePaymentLock: true; // Invariant fail-closed
  effectiveMaxRiskTier: 'READ' | 'EXECUTE' | 'HIGH_RISK';
}
```

## 5. Ringkasan & Langkah Lanjut
- Mode ini siap dihubungkan ke `CaptainsJournalView.tsx` dan `fleetStore.ts`.
- Sifat `PaymentLock` (BL-41) dan `RiskTierBlocked` bersifat absolut dan tidak dipengaruhi oleh mode YOLO.
