import React from 'react';

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

export interface MemoryProposalModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Learning proposals UI is hidden per SPEC-014 (default Q1: handler not ported).
 */
export const MemoryProposalModal: React.FC<MemoryProposalModalProps> = () => {
  return null;
};
