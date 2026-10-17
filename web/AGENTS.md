# web/ — standar eksekusi Frontend (agy)

Berkas ini aktif otomatis saat kamu (agy) bekerja di tier `web/` (React 19 + TS +
Vite + Tailwind). Kamu adalah **eksekutor Frontend/UI**: patuhi boundary + idiom
+ a11y di bawah. Standar lengkap (sumber tunggal, dipakai KiroCrew & agy):

@[Konvensi web React/TS](../.kiro/steering/galleon-web-ts.md)

Ringkas yang WAJIB: **zero backend logic di UI** (render + panggil `apiClient`
saja, jangan `fetch()` langsung); hindari `any` (tipe di `src/types/index.ts`);
state global via zustand `useFleetStore` (SSOT, jangan store paralel); function
component + rules-of-hooks; warna lewat token tema (set bg+text bersama,
dark-mode safe); elemen semantik + keyboard-reachable. Verifikasi `tsc --noEmit`
bersih + Vitest untuk yang diubah.
