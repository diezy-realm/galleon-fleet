import React, { useState, useEffect } from 'react';
import { apiClient } from '../../utils/apiClient';
import { t } from '../../lib/i18n';
import { Smartphone, Laptop, QrCode, Trash2, RefreshCw, Plus, CheckCircle, Shield, AlertCircle } from 'lucide-react';

export interface DeviceInfo {
  id: string;
  name?: string | null;
  device_type?: string | null;
  paired_at: string;
  last_seen: string;
  status?: string | null;
  ip_address?: string | null;
  capabilities?: string[] | null;
}

interface PairingSession {
  pairing_code: string;
  token?: string | null;
  expires_in_seconds?: number | null;
  message?: string | null;
}

export const ConnectedDevicesView: React.FC = () => {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeSession, setActiveSession] = useState<PairingSession | null>(null);
  const [isPairingModalOpen, setIsPairingModalOpen] = useState(false);
  const [revokingId, setRevokingId] = useState<string | null>(null);

  const fetchDevices = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await apiClient.get<{ devices: DeviceInfo[]; count: number } | DeviceInfo[]>('/api/devices');
      if (Array.isArray(data)) {
        setDevices(data);
      } else if (data && Array.isArray(data.devices)) {
        setDevices(data.devices);
      } else {
        setDevices([]);
      }
    } catch (err: any) {
      setError(err?.message || 'Failed to fetch paired devices');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchDevices();
  }, []);

  const handleInitiatePairing = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.post<PairingSession>('/api/pairing/initiate');
      if (data) {
        setActiveSession(data);
        setIsPairingModalOpen(true);
      }
    } catch (err: any) {
      setError(err?.message || 'Failed to initiate pairing');
    } finally {
      setIsLoading(false);
    }
  };

  const handleRevoke = async (deviceId: string) => {
    setRevokingId(deviceId);
    try {
      await apiClient.delete(`/api/devices/${encodeURIComponent(deviceId)}`);
      setDevices(prev => prev.filter(d => d.id !== deviceId));
    } catch (err: any) {
      setError(err?.message || 'Failed to revoke device');
    } finally {
      setRevokingId(null);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-neutral-950 text-neutral-100 overflow-hidden">
      {/* Header */}
      <div className="p-6 border-b border-neutral-800 bg-neutral-900/60 backdrop-blur-sm flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-bold flex items-center gap-2">
            <Smartphone className="w-6 h-6 text-teal-400" />
            {t('devices.title')}
          </h1>
          <p className="text-xs text-neutral-400 mt-1">
            {t('devices.subtitle')}
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={fetchDevices}
            disabled={isLoading}
            className="px-3 py-1.5 text-xs font-semibold rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 flex items-center gap-1.5 cursor-pointer"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
          <button
            onClick={handleInitiatePairing}
            className="px-3.5 py-1.5 text-xs font-semibold rounded-lg bg-teal-600 hover:bg-teal-500 text-white flex items-center gap-1.5 cursor-pointer shadow-sm transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            {t('devices.pair_new')}
          </button>
        </div>
      </div>

      {/* Body Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {error && (
          <div className="p-4 rounded-xl border border-rose-500/30 bg-rose-500/10 text-rose-300 text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{error}</span>
          </div>
        )}

        {/* Device List */}
        <div className="bg-neutral-900/50 rounded-xl border border-neutral-800 p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-bold text-neutral-200 flex items-center gap-2">
              <Shield className="w-4 h-4 text-cyan-400" />
              Authorized Companions &amp; Nodes ({devices.length})
            </h2>
            <span className="text-[11px] text-neutral-500">Mutual TLS &amp; Token Auth</span>
          </div>

          {devices.length === 0 ? (
            <div className="py-12 text-center text-neutral-500 text-xs space-y-2">
              <Laptop className="w-8 h-8 mx-auto text-neutral-600" />
              <div>No companion devices paired yet.</div>
              <p className="text-[11px] text-neutral-600">
                Pair your mobile phone, secondary workstation, or local dashboard node to control your Fleet remotely.
              </p>
            </div>
          ) : (
            <div className="divide-y divide-neutral-800/60 font-mono text-xs">
              {devices.map(device => (
                <div key={device.id} className="py-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                  <div className="flex items-center gap-3">
                    <div className="w-9 h-9 rounded-lg bg-teal-500/10 border border-teal-500/30 text-teal-400 flex items-center justify-center shrink-0">
                      {device.device_type === 'mobile' ? (
                        <Smartphone className="w-5 h-5" />
                      ) : (
                        <Laptop className="w-5 h-5" />
                      )}
                    </div>
                    <div>
                      <div className="font-bold text-neutral-200 font-sans text-sm">{device.name || '—'}</div>
                      <div className="text-[10px] text-neutral-500 mt-0.5">
                        ID: {device.id} · Type: <span className="text-cyan-300">{device.device_type || '—'}</span>
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-4 self-end sm:self-auto">
                    <div className="text-right text-[11px] text-neutral-400">
                      <div>{t('devices.last_seen')}: <span className="text-neutral-300">{device.last_seen || '—'}</span></div>
                      <div className="text-[10px] text-neutral-500">Paired: {device.paired_at || '—'}</div>
                    </div>
                    <button
                      onClick={() => handleRevoke(device.id)}
                      disabled={revokingId === device.id}
                      className="px-2.5 py-1 text-xs rounded bg-rose-500/10 border border-rose-500/30 text-rose-300 hover:bg-rose-500/20 disabled:opacity-40 flex items-center gap-1 cursor-pointer transition-colors"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                      {revokingId === device.id ? 'Revoking...' : t('devices.revoke')}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* QR Pairing Modal */}
      {isPairingModalOpen && activeSession && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-xs flex items-center justify-center p-4">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-sm w-full p-6 text-center space-y-4 shadow-2xl">
            <div className="flex items-center justify-center w-12 h-12 mx-auto rounded-full bg-teal-500/10 border border-teal-500/30 text-teal-400">
              <QrCode className="w-6 h-6" />
            </div>

            <div className="space-y-1">
              <h3 className="text-base font-bold text-neutral-100">{t('devices.pair_new')}</h3>
              <p className="text-xs text-neutral-400">{t('devices.qr_code')}</p>
            </div>

            {/* QR Mock / Pairing Code Box */}
            <div className="p-4 rounded-xl bg-neutral-950 border border-neutral-800 flex flex-col items-center justify-center space-y-3">
              <div className="w-36 h-36 bg-white p-2 rounded-lg flex items-center justify-center shadow-inner">
                {/* SVG Visual QR Placeholder */}
                <div className="w-full h-full border-4 border-black border-dashed flex items-center justify-center font-mono text-[9px] text-black font-bold p-1 leading-tight text-center">
                  [QR ENCODED]
                  <br />
                  TOKEN:
                  <br />
                  {activeSession.pairing_code}
                </div>
              </div>

              <div className="space-y-1">
                <span className="text-[10px] uppercase font-mono tracking-wider text-neutral-500">Pairing Code</span>
                <div className="text-lg font-mono font-bold tracking-widest text-teal-300">
                  {activeSession.pairing_code}
                </div>
              </div>
            </div>

            <p className="text-[11px] text-neutral-500">
              Valid for {activeSession.expires_in_seconds ? Math.floor(activeSession.expires_in_seconds / 60) : 10} minutes. One-time exchange.
            </p>

            <button
              onClick={() => {
                setIsPairingModalOpen(false);
                fetchDevices();
              }}
              className="w-full py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-semibold cursor-pointer transition-colors"
            >
              Done / Close
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
