import React, { useState, useEffect } from 'react';
import { useFleetStore } from '../../store/fleetStore';
import { apiClient } from '../../utils/apiClient';
import { t } from '../../lib/i18n';
import { Shield, Users, Anchor, ChevronRight, Terminal, Eye, Layers } from 'lucide-react';

import { CrewMember } from '../../types';

interface AuditEntry {
  id: string;
  timestamp: string;
  principal: string;
  action: string;
  command?: string;
  risk_tier?: string;
}

export const OrganizationView: React.FC = () => {
  const { ships, squads, crew, fleetPolicies } = useFleetStore();
  const policies = fleetPolicies || [];
  const [selectedShipId, setSelectedShipId] = useState<string | null>(null);
  const [selectedSquadId, setSelectedSquadId] = useState<string | null>(null);
  const [auditEntries, setAuditEntries] = useState<AuditEntry[]>([]);
  const [activeSubTab, setActiveSubTab] = useState<'hierarchy' | 'audit'>('hierarchy');

  useEffect(() => {
    // Fetch live audit trail from Go engine backend (BG-F3)
    const fetchAudit = async () => {
      try {
        const data = await apiClient.get<AuditEntry[]>('/api/v1/audit/trail');
        if (Array.isArray(data)) {
          setAuditEntries(data);
        }
      } catch (err) {
        console.warn('Failed to load live audit log:', err);
      }
    };
    fetchAudit();
  }, []);

  const filteredSquads = selectedShipId
    ? squads.filter(sq => !sq.shipId || sq.shipId === selectedShipId)
    : squads;

  const filteredCrew = selectedSquadId
    ? crew.filter((cm: CrewMember) => cm.squadId === selectedSquadId)
    : selectedShipId
    ? crew.filter((cm: CrewMember) => cm.shipId === selectedShipId)
    : crew;

  return (
    <div className="flex-1 flex flex-col h-full bg-neutral-950 text-neutral-100 overflow-hidden">
      {/* Top Header */}
      <div className="p-6 border-b border-neutral-800 bg-neutral-900/60 backdrop-blur-sm flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-bold flex items-center gap-2">
            <Users className="w-6 h-6 text-teal-400" />
            {t('org.title')}
          </h1>
          <p className="text-xs text-neutral-400 mt-1">
            {t('org.subtitle')}
          </p>
        </div>

        {/* Sub-Tab Navigation */}
        <div className="flex bg-neutral-800/80 p-1 rounded-lg border border-neutral-700">
          <button
            onClick={() => setActiveSubTab('hierarchy')}
            className={`px-3 py-1.5 text-xs font-semibold rounded-md transition-colors flex items-center gap-1.5 ${
              activeSubTab === 'hierarchy'
                ? 'bg-teal-600 text-white'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            <Layers className="w-3.5 h-3.5" />
            Hierarchy
          </button>
          <button
            onClick={() => setActiveSubTab('audit')}
            className={`px-3 py-1.5 text-xs font-semibold rounded-md transition-colors flex items-center gap-1.5 ${
              activeSubTab === 'audit'
                ? 'bg-teal-600 text-white'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            <Shield className="w-3.5 h-3.5" />
            {t('org.audit_policy')}
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto p-6">
        {activeSubTab === 'hierarchy' ? (
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            {/* Level 1: Ships (Departments) */}
            <div className="bg-neutral-900/50 rounded-xl border border-neutral-800 p-4 flex flex-col">
              <div className="flex items-center justify-between pb-3 mb-3 border-b border-neutral-800">
                <span className="text-xs font-bold uppercase tracking-wider text-neutral-400 flex items-center gap-2">
                  <Anchor className="w-4 h-4 text-cyan-400" />
                  {t('org.ships')}
                </span>
                <span className="text-xs text-neutral-500">{ships.length} Active</span>
              </div>
              <div className="space-y-2 flex-1 overflow-y-auto max-h-[550px]">
                <button
                  onClick={() => {
                    setSelectedShipId(null);
                    setSelectedSquadId(null);
                  }}
                  className={`w-full text-left p-3 rounded-lg text-xs font-medium transition-all ${
                    selectedShipId === null
                      ? 'bg-cyan-500/10 border border-cyan-500/40 text-cyan-300'
                      : 'bg-neutral-800/40 hover:bg-neutral-800 text-neutral-300 border border-transparent'
                  }`}
                >
                  All Ships (Fleet HQ)
                </button>
                {ships.map(ship => (
                  <button
                    key={ship.id}
                    onClick={() => {
                      setSelectedShipId(ship.id);
                      setSelectedSquadId(null);
                    }}
                    className={`w-full text-left p-3 rounded-lg text-xs transition-all flex items-center justify-between ${
                      selectedShipId === ship.id
                        ? 'bg-cyan-500/15 border border-cyan-500/50 text-cyan-200 font-semibold shadow-sm'
                        : 'bg-neutral-800/40 hover:bg-neutral-800 text-neutral-300 border border-transparent'
                    }`}
                  >
                    <div>
                      <div className="font-bold">{ship.name}</div>
                      <div className="text-[10px] text-neutral-400 mt-0.5 line-clamp-1">{ship.charter?.purpose || 'Active vessel'}</div>
                    </div>
                    <ChevronRight className="w-3.5 h-3.5 opacity-50" />
                  </button>
                ))}
              </div>
            </div>

            {/* Level 2: Squads (Teams) */}
            <div className="bg-neutral-900/50 rounded-xl border border-neutral-800 p-4 flex flex-col">
              <div className="flex items-center justify-between pb-3 mb-3 border-b border-neutral-800">
                <span className="text-xs font-bold uppercase tracking-wider text-neutral-400 flex items-center gap-2">
                  <Users className="w-4 h-4 text-emerald-400" />
                  {t('org.squads')}
                </span>
                <span className="text-xs text-neutral-500">{filteredSquads.length} Teams</span>
              </div>
              <div className="space-y-2 flex-1 overflow-y-auto max-h-[550px]">
                <button
                  onClick={() => setSelectedSquadId(null)}
                  className={`w-full text-left p-3 rounded-lg text-xs font-medium transition-all ${
                    selectedSquadId === null
                      ? 'bg-emerald-500/10 border border-emerald-500/40 text-emerald-300'
                      : 'bg-neutral-800/40 hover:bg-neutral-800 text-neutral-300 border border-transparent'
                  }`}
                >
                  All Squads
                </button>
                {filteredSquads.map(sq => (
                  <button
                    key={sq.id}
                    onClick={() => setSelectedSquadId(sq.id)}
                    className={`w-full text-left p-3 rounded-lg text-xs transition-all flex items-center justify-between ${
                      selectedSquadId === sq.id
                        ? 'bg-emerald-500/15 border border-emerald-500/50 text-emerald-200 font-semibold shadow-sm'
                        : 'bg-neutral-800/40 hover:bg-neutral-800 text-neutral-300 border border-transparent'
                    }`}
                  >
                    <div>
                      <div className="font-bold">{sq.name}</div>
                      <div className="text-[10px] text-neutral-400 mt-0.5 line-clamp-1">{sq.purpose}</div>
                    </div>
                    <ChevronRight className="w-3.5 h-3.5 opacity-50" />
                  </button>
                ))}
              </div>
            </div>

            {/* Level 3: Crew Members (Individual Agents) */}
            <div className="bg-neutral-900/50 rounded-xl border border-neutral-800 p-4 flex flex-col">
              <div className="flex items-center justify-between pb-3 mb-3 border-b border-neutral-800">
                <span className="text-xs font-bold uppercase tracking-wider text-neutral-400 flex items-center gap-2">
                  <Shield className="w-4 h-4 text-teal-400" />
                  {t('org.crew')}
                </span>
                <span className="text-xs text-neutral-500">{filteredCrew.length} Agents</span>
              </div>
              <div className="space-y-3 flex-1 overflow-y-auto max-h-[550px]">
                {filteredCrew.length === 0 ? (
                  <div className="text-xs text-neutral-500 text-center py-8">
                    {t('org.no_members')}
                  </div>
                ) : (
                  filteredCrew.map((agent: CrewMember) => (
                    <div
                      key={agent.id}
                      className="p-3.5 rounded-lg bg-neutral-800/60 border border-neutral-700/60 space-y-2"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-bold text-xs text-neutral-200">{agent.name}</span>
                        <span className="px-2 py-0.5 rounded text-[10px] bg-teal-500/10 border border-teal-500/30 text-teal-300 font-mono">
                          {agent.role}
                        </span>
                      </div>
                      <p className="text-[11px] text-neutral-400 line-clamp-2">
                        {agent.purpose}
                      </p>
                      <div className="flex flex-wrap items-center gap-1.5 pt-1">
                        <span className="text-[10px] text-neutral-500">Skills:</span>
                        {agent.skills?.slice(0, 3).map((sk: string, idx: number) => (
                          <span
                            key={idx}
                            className="px-1.5 py-0.5 rounded text-[9px] bg-neutral-700/60 text-neutral-300"
                          >
                            {sk}
                          </span>
                        ))}
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        ) : (
          /* Read-Only Policy & Audit Log Surface (BG-F3) */
          <div className="space-y-6">
            <div className="bg-neutral-900/50 rounded-xl border border-neutral-800 p-5 space-y-3">
              <h2 className="text-sm font-bold text-neutral-200 flex items-center gap-2">
                <Shield className="w-4 h-4 text-teal-400" />
                Active Fleet Policies (Read-Only)
              </h2>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 pt-2">
                <div className="p-3 rounded-lg bg-neutral-800/40 border border-neutral-700/60">
                  <div className="text-[10px] text-neutral-400 uppercase font-semibold">Strict Consent Boundary</div>
                  <div className="text-xs font-mono font-bold text-emerald-400 mt-1">ENFORCED (ODR-07)</div>
                </div>
                <div className="p-3 rounded-lg bg-neutral-800/40 border border-neutral-700/60">
                  <div className="text-[10px] text-neutral-400 uppercase font-semibold">Treasury Access Mode</div>
                  <div className="text-xs font-mono font-bold text-emerald-400 mt-1">READ-ONLY (Invariant F1)</div>
                </div>
                <div className="p-3 rounded-lg bg-neutral-800/40 border border-neutral-700/60">
                  <div className="text-[10px] text-neutral-400 uppercase font-semibold">Payment Lockout Gate</div>
                  <div className="text-xs font-mono font-bold text-amber-400 mt-1">FAIL-CLOSED (BL-40..44)</div>
                </div>
                <div className="p-3 rounded-lg bg-neutral-800/40 border border-neutral-700/60">
                  <div className="text-[10px] text-neutral-400 uppercase font-semibold">Terminal Execution Gate</div>
                  <div className="text-xs font-mono font-bold text-cyan-400 mt-1">AUTHENTICATED (S2-SEC-01)</div>
                </div>
              </div>
            </div>

            {/* Live Audit Trail */}
            <div className="bg-neutral-900/50 rounded-xl border border-neutral-800 p-5 space-y-3">
              <div className="flex items-center justify-between">
                <h2 className="text-sm font-bold text-neutral-200 flex items-center gap-2">
                  <Terminal className="w-4 h-4 text-amber-400" />
                  Terminal & Security Audit Stream (Read-Only)
                </h2>
                <span className="text-xs text-neutral-500 font-mono">
                  {auditEntries.length} Records Logged
                </span>
              </div>

              <div className="overflow-x-auto">
                <table className="w-full text-left text-xs">
                  <thead>
                    <tr className="border-b border-neutral-800 text-neutral-400">
                      <th className="py-2.5 px-3">Timestamp</th>
                      <th className="py-2.5 px-3">Principal</th>
                      <th className="py-2.5 px-3">Action / Command</th>
                      <th className="py-2.5 px-3">Risk Tier</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-neutral-800/50 font-mono">
                    {auditEntries.length === 0 ? (
                      <tr>
                        <td colSpan={4} className="py-6 text-center text-neutral-500 font-sans">
                          No audit records found or audit log empty.
                        </td>
                      </tr>
                    ) : (
                      auditEntries.map(entry => (
                        <tr key={entry.id} className="hover:bg-neutral-800/30">
                          <td className="py-2.5 px-3 text-neutral-400 text-[11px]">{entry.timestamp}</td>
                          <td className="py-2.5 px-3 text-cyan-300 font-semibold">{entry.principal || 'system'}</td>
                          <td className="py-2.5 px-3 text-neutral-200">{entry.command || entry.action}</td>
                          <td className="py-2.5 px-3">
                            <span className="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/10 text-amber-300 border border-amber-500/30">
                              {entry.risk_tier || 'standard'}
                            </span>
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
