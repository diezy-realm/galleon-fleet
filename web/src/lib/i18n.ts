/**
 * Web dashboard internationalization (i18n) contract.
 * Follows AGENTS.md: "Web dashboard text follows the TypeScript web/src/lib/i18n.ts contract".
 * English is the documentation and fallback source of truth.
 */

export type Locale = 'en' | 'id';

export interface TranslationDictionary {
  [key: string]: string;
}

const translations: Record<Locale, TranslationDictionary> = {
  en: {
    // Navigation & Common
    'app.title': 'Galleon Fleet Command',
    'app.tagline': 'Autonomous Agent Fleet Orchestration',
    'common.save': 'Save',
    'common.cancel': 'Cancel',
    'common.approve': 'Approve',
    'common.reject': 'Reject',
    'common.deny': 'Deny',
    'common.rollback': 'Rollback',
    'common.status': 'Status',
    'common.actions': 'Actions',
    'common.details': 'Details',
    'common.loading': 'Loading...',
    'common.error': 'An error occurred',
    'common.success': 'Success',

    // Organization View (FW-C1)
    'org.title': 'Fleet Organization',
    'org.subtitle': 'Hierarchical command structure: Quartermaster → Ship → Squad → Crew',
    'org.quartermaster': 'Quartermaster (Fleet HQ)',
    'org.ships': 'Ships (Departments)',
    'org.squads': 'Squads (Teams)',
    'org.crew': 'Crew Members (Agents)',
    'org.audit_policy': 'Policy & Audit Log (Read-Only)',
    'org.no_members': 'No crew members assigned to this squad.',
    'org.authority_level': 'Authority Level',
    'org.model_provider': 'Model Provider',

    // Payment-Lockout Gate (BL-43)
    'payment.gate.title': 'Preventive Payment-Lockout Gate',
    'payment.gate.status_locked': 'LOCKED (Fail-Closed)',
    'payment.gate.status_unlocked': 'UNLOCKED',
    'payment.gate.raw_state': 'Raw Enforcement State (OWASP ASI09)',
    'payment.gate.toggle_allow': 'Allow Agents to Make Payments',
    'payment.gate.auto_relock': 'Auto Re-locks in',
    'payment.gate.unlock_prompt': 'Unlock Duration (minutes)',

    // Terminal & Approvals (FW-C3)
    'terminal.approval.title': 'Authenticated Execution Request',
    'terminal.approval.principal': 'Verified Principal',
    'terminal.approval.command': 'Terminal Command',
    'terminal.approval.risk_tier': 'Risk Tier',
    'terminal.approval.required': 'Explicit authenticated approval is mandatory.',

    // Connected Devices (FW-C4)
    'devices.title': 'Connected Devices & Pairing',
    'devices.subtitle': 'Authorized companion devices and local gateways',
    'devices.pair_new': 'Pair New Device',
    'devices.qr_code': 'Scan QR Code with Companion App',
    'devices.revoke': 'Revoke Authorization',
    'devices.last_seen': 'Last Seen',
    'devices.device_type': 'Device Type',

    // Memory Proposals (FW-C5)
    'memory.proposals.title': 'Learning by Consent — Memory Proposals',
    'memory.proposals.subtitle': 'Learned behavioral corrections awaiting Owner approval',
    'memory.proposals.pending': 'Pending Approvals',
    'memory.proposals.rule': 'Proposed Rule',
    'memory.proposals.scope': 'Active Scope',
    'memory.proposals.version': 'Version',
    'memory.proposals.feedback': 'Owner Feedback',
    'memory.proposals.add_feedback': 'Add Feedback',
    'memory.proposals.rollback_to': 'Rollback to Version',
  },
  id: {
    // Navigasi & Umum
    'app.title': 'Komando Armada Galleon',
    'app.tagline': 'Orkestrasi Armada Agen Otonom',
    'common.save': 'Simpan',
    'common.cancel': 'Batal',
    'common.approve': 'Setujui',
    'common.reject': 'Tolak',
    'common.deny': 'Tolak',
    'common.rollback': 'Kembalikan Versi',
    'common.status': 'Status',
    'common.actions': 'Tindakan',
    'common.details': 'Rincian',
    'common.loading': 'Memuat...',
    'common.error': 'Terjadi kesalahan',
    'common.success': 'Berhasil',

    // Layar Organisasi (FW-C1)
    'org.title': 'Struktur Organisasi Armada',
    'org.subtitle': 'Hierarki komando: Quartermaster → Kapal → Skuad → Kru',
    'org.quartermaster': 'Quartermaster (Markas Armada)',
    'org.ships': 'Kapal (Departemen)',
    'org.squads': 'Skuad (Tim)',
    'org.crew': 'Anggota Kru (Agen)',
    'org.audit_policy': 'Log Audit & Kebijakan (Hanya-Baca)',
    'org.no_members': 'Tidak ada anggota kru di skuad ini.',
    'org.authority_level': 'Tingkat Otoritas',
    'org.model_provider': 'Penyedia Model',

    // Gerbang Payment-Lockout (BL-43)
    'payment.gate.title': 'Gerbang Penguncian Pembayaran Preventif',
    'payment.gate.status_locked': 'TERKUNCI (Fail-Closed)',
    'payment.gate.status_unlocked': 'TERBUKA',
    'payment.gate.raw_state': 'Status Penegakan Mentah (OWASP ASI09)',
    'payment.gate.toggle_allow': 'Izinkan Agen Melakukan Pembayaran',
    'payment.gate.auto_relock': 'Terkunci otomatis dalam',
    'payment.gate.unlock_prompt': 'Durasi Buka Kunci (menit)',

    // Terminal & Persetujuan (FW-C3)
    'terminal.approval.title': 'Permintaan Eksekusi Terotentikasi',
    'terminal.approval.principal': 'Identitas Terverifikasi',
    'terminal.approval.command': 'Perintah Terminal',
    'terminal.approval.risk_tier': 'Tingkat Risiko',
    'terminal.approval.required': 'Persetujuan terotentikasi eksplisit wajib diberikan.',

    // Perangkat Terhubung (FW-C4)
    'devices.title': 'Perangkat Terhubung & Pairing',
    'devices.subtitle': 'Perangkat pendamping resmi dan gateway lokal',
    'devices.pair_new': 'Hubungkan Perangkat Baru',
    'devices.qr_code': 'Pindai Kode QR dengan Aplikasi Pendamping',
    'devices.revoke': 'Cabut Otorisasi',
    'devices.last_seen': 'Terakhir Terlihat',
    'devices.device_type': 'Tipe Perangkat',

    // Proposal Memori (FW-C5)
    'memory.proposals.title': 'Pembelajaran Berbasis Persetujuan — Proposal Memori',
    'memory.proposals.subtitle': 'Koreksi perilaku terpelajari yang menunggu persetujuan Owner',
    'memory.proposals.pending': 'Menunggu Persetujuan',
    'memory.proposals.rule': 'Aturan yang Diajukan',
    'memory.proposals.scope': 'Cakupan Aktif',
    'memory.proposals.version': 'Versi',
    'memory.proposals.feedback': 'Umpan Balik Pemilik',
    'memory.proposals.add_feedback': 'Beri Masukan',
    'memory.proposals.rollback_to': 'Kembalikan ke Versi',
  },
};

let currentLocale: Locale = 'en';

export function setLocale(locale: Locale): void {
  currentLocale = locale;
}

export function getLocale(): Locale {
  return currentLocale;
}

export function t(key: string, params?: Record<string, string | number>): string {
  const dict = translations[currentLocale] || translations.en;
  let text = dict[key] || translations.en[key] || key;

  if (params) {
    Object.entries(params).forEach(([paramKey, paramVal]) => {
      text = text.replace(new RegExp(`{${paramKey}}`, 'g'), String(paramVal));
    });
  }

  return text;
}
