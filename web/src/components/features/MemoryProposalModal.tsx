import React, { useState, useEffect } from 'react';
import { apiClient } from '../../utils/apiClient';
import { t } from '../../lib/i18n';
import { Brain, Check, X, Undo2, MessageSquare, AlertCircle, RefreshCw, Send } from 'lucide-react';

export interface MemoryProposalItem {
  id: string;
  rule: string;
  scope: Record<string, string>;
  status: 'pending' | 'approved' | 'rejected' | 'reverted';
  version: number;
  reversible: boolean;
  document_id?: string;
  feedbacks?: Array<{
    id: string;
    author: string;
    comment: string;
    created_at: string;
  }>;
  created_at: string;
}

interface MemoryProposalModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const MemoryProposalModal: React.FC<MemoryProposalModalProps> = ({ isOpen, onClose }) => {
  const [proposals, setProposals] = useState<MemoryProposalItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [feedbackInput, setFeedbackInput] = useState<{ [id: string]: string }>({});
  const [activeTab, setActiveTab] = useState<'pending' | 'approved'>('pending');

  const fetchProposals = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await apiClient.get<{ proposals: MemoryProposalItem[] }>('/api/v1/learning/proposals');
      if (data && data.proposals) {
        setProposals(data.proposals);
      } else {
        setProposals([]);
      }
    } catch (err: any) {
      setError(err?.message || 'Failed to fetch memory proposals');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      fetchProposals();
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleApprove = async (id: string) => {
    try {
      await apiClient.post(`/api/v1/learning/proposals/${id}/approve`, {});
      await fetchProposals();
    } catch (err: any) {
      setError(err?.message || 'Failed to approve proposal');
    }
  };

  const handleReject = async (id: string) => {
    try {
      await apiClient.post(`/api/v1/learning/proposals/${id}/reject`, {});
      await fetchProposals();
    } catch (err: any) {
      setError(err?.message || 'Failed to reject proposal');
    }
  };

  const handleRevert = async (id: string) => {
    try {
      await apiClient.post(`/api/v1/learning/proposals/${id}/revert`, {});
      await fetchProposals();
    } catch (err: any) {
      setError(err?.message || 'Failed to revert proposal');
    }
  };

  const handleAddFeedback = async (id: string) => {
    const comment = feedbackInput[id]?.trim();
    if (!comment) return;

    try {
      await apiClient.post(`/api/v1/learning/proposals/${id}/feedback`, {
        author: 'Pirate King (Owner)',
        comment
      });
      setFeedbackInput(prev => ({ ...prev, [id]: '' }));
      await fetchProposals();
    } catch (err: any) {
      // If endpoint returns error, store feedback in local list gracefully
      setProposals(prev =>
        prev.map(p =>
          p.id === id
            ? {
                ...p,
                feedbacks: [
                  ...(p.feedbacks || []),
                  {
                    id: `fb-${Date.now()}`,
                    author: 'Pirate King (Owner)',
                    comment,
                    created_at: new Date().toISOString()
                  }
                ]
              }
            : p
        )
      );
      setFeedbackInput(prev => ({ ...prev, [id]: '' }));
    }
  };

  const filtered = proposals.filter(p =>
    activeTab === 'pending' ? p.status === 'pending' : p.status !== 'pending'
  );

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-xs flex items-center justify-center p-4">
      <div className="bg-neutral-900 border border-neutral-800 rounded-2xl max-w-2xl w-full flex flex-col max-h-[85vh] shadow-2xl overflow-hidden">
        {/* Modal Header */}
        <div className="p-5 border-b border-neutral-800 flex items-center justify-between bg-neutral-900/80">
          <div className="flex items-center gap-2.5">
            <div className="w-9 h-9 rounded-xl bg-teal-500/10 border border-teal-500/30 text-teal-400 flex items-center justify-center">
              <Brain className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-sm font-bold text-neutral-100">{t('memory.proposals.title')}</h2>
              <p className="text-[11px] text-neutral-400">{t('memory.proposals.subtitle')}</p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={fetchProposals}
              disabled={isLoading}
              className="p-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 cursor-pointer"
              title="Refresh proposals"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            </button>
            <button
              onClick={onClose}
              className="px-2.5 py-1 text-xs rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-400 hover:text-neutral-200 cursor-pointer"
            >
              Close
            </button>
          </div>
        </div>

        {/* Tab Filters */}
        <div className="px-5 pt-3 pb-2 border-b border-neutral-800/60 bg-neutral-950/40 flex items-center gap-2 text-xs">
          <button
            onClick={() => setActiveTab('pending')}
            className={`px-3 py-1 rounded-md font-semibold transition-colors ${
              activeTab === 'pending'
                ? 'bg-teal-600 text-white'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            Pending Approvals ({proposals.filter(p => p.status === 'pending').length})
          </button>
          <button
            onClick={() => setActiveTab('approved')}
            className={`px-3 py-1 rounded-md font-semibold transition-colors ${
              activeTab === 'approved'
                ? 'bg-teal-600 text-white'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            History &amp; Approved ({proposals.filter(p => p.status !== 'pending').length})
          </button>
        </div>

        {/* Modal Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="p-3 rounded-xl border border-rose-500/30 bg-rose-500/10 text-rose-300 text-xs flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {filtered.length === 0 ? (
            <div className="py-12 text-center text-neutral-500 text-xs space-y-2">
              <Brain className="w-8 h-8 mx-auto text-neutral-600" />
              <div>No memory proposals in this state.</div>
              <p className="text-[11px] text-neutral-600">
                When an agent learns a correction or guideline during a run, it will submit a proposal here for Owner consent.
              </p>
            </div>
          ) : (
            filtered.map(prop => (
              <div
                key={prop.id}
                className="p-4 rounded-xl border border-neutral-800 bg-neutral-950/60 space-y-3 font-mono text-xs"
              >
                <div className="flex items-center justify-between">
                  <span className="font-bold text-teal-300">{prop.id} (v{prop.version})</span>
                  <span
                    className={`px-2 py-0.5 rounded text-[10px] uppercase font-bold ${
                      prop.status === 'pending'
                        ? 'bg-amber-500/10 text-amber-300 border border-amber-500/30'
                        : prop.status === 'approved'
                        ? 'bg-emerald-500/10 text-emerald-300 border border-emerald-500/30'
                        : 'bg-rose-500/10 text-rose-300 border border-rose-500/30'
                    }`}
                  >
                    {prop.status}
                  </span>
                </div>

                <div className="p-3 rounded-lg bg-neutral-900 border border-neutral-800 text-neutral-200 font-sans text-xs">
                  <strong className="text-[10px] uppercase tracking-wider text-neutral-500 block font-mono mb-1">
                    {t('memory.proposals.rule')}:
                  </strong>
                  {prop.rule}
                </div>

                {/* Scope Metadata */}
                <div className="flex flex-wrap items-center gap-2 text-[10px] text-neutral-400">
                  <span className="text-neutral-500 uppercase">{t('memory.proposals.scope')}:</span>
                  {Object.entries(prop.scope || {}).map(([k, v]) => (
                    <span key={k} className="px-1.5 py-0.5 rounded bg-neutral-800 text-cyan-300">
                      {k}: {v}
                    </span>
                  ))}
                </div>

                {/* Feedbacks list if any */}
                {prop.feedbacks && prop.feedbacks.length > 0 && (
                  <div className="space-y-1 pt-2 border-t border-neutral-800/80">
                    <span className="text-[10px] text-neutral-500 uppercase block">Owner Feedback:</span>
                    {prop.feedbacks.map(fb => (
                      <div key={fb.id} className="text-[11px] text-neutral-300 bg-neutral-900/40 p-2 rounded border border-neutral-800">
                        <span className="font-bold text-amber-300">{fb.author}: </span>
                        <span>{fb.comment}</span>
                      </div>
                    ))}
                  </div>
                )}

                {/* Actions Bar */}
                <div className="pt-2 flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-t border-neutral-800/80">
                  {/* Feedback Input */}
                  <div className="flex items-center gap-1.5 flex-1">
                    <input
                      type="text"
                      placeholder="Add guidance / feedback..."
                      value={feedbackInput[prop.id] || ''}
                      onChange={e => setFeedbackInput({ ...feedbackInput, [prop.id]: e.target.value })}
                      className="bg-neutral-900 border border-neutral-700/80 rounded px-2.5 py-1 text-xs text-neutral-200 flex-1 focus:outline-none focus:border-teal-500"
                    />
                    <button
                      onClick={() => handleAddFeedback(prop.id)}
                      disabled={!feedbackInput[prop.id]?.trim()}
                      className="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 disabled:opacity-40 cursor-pointer"
                      title="Send feedback"
                    >
                      <Send className="w-3.5 h-3.5" />
                    </button>
                  </div>

                  {/* Decision Buttons */}
                  <div className="flex items-center gap-2 self-end sm:self-auto">
                    {prop.status === 'pending' && (
                      <>
                        <button
                          onClick={() => handleReject(prop.id)}
                          className="px-2.5 py-1 rounded bg-rose-500/10 border border-rose-500/30 text-rose-300 hover:bg-rose-500/20 text-xs flex items-center gap-1 cursor-pointer"
                        >
                          <X className="w-3 h-3" />
                          Reject
                        </button>
                        <button
                          onClick={() => handleApprove(prop.id)}
                          className="px-3 py-1 rounded bg-teal-600 hover:bg-teal-500 text-white font-semibold text-xs flex items-center gap-1 cursor-pointer"
                        >
                          <Check className="w-3 h-3" />
                          Approve
                        </button>
                      </>
                    )}

                    {prop.status === 'approved' && prop.reversible && (
                      <button
                        onClick={() => handleRevert(prop.id)}
                        className="px-2.5 py-1 rounded bg-amber-500/10 border border-amber-500/30 text-amber-300 hover:bg-amber-500/20 text-xs flex items-center gap-1 cursor-pointer"
                      >
                        <Undo2 className="w-3 h-3" />
                        Revert / Rollback
                      </button>
                    )}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
