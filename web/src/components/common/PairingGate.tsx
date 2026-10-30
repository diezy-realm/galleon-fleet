import React, { useState, useEffect } from 'react';
import { Key, AlertCircle, Info, Sparkles } from 'lucide-react';
import { GalleonLogo } from './GalleonLogo';
import { Button } from './Button';
import { CodeSnippet } from './CodeSnippet';
import { apiClient, getToken, setToken as saveToken } from '../../utils/apiClient';
import { t } from '../../lib/i18n';

export interface PairingGateProps {
  children: React.ReactNode;
}

export const PairingGate: React.FC<PairingGateProps> = ({ children }) => {
  const [token, setTokenState] = useState<string | null>(() => getToken());
  const [code, setCode] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [prefilled, setPrefilled] = useState(false);

  // Listen for 401 unauthorized events dispatched by request wrapper
  useEffect(() => {
    const handleUnauthorized = () => {
      setTokenState(null);
      setError(t('pairing.unauthorized_notice'));
    };

    window.addEventListener('galleon:unauthorized', handleUnauthorized);
    return () => {
      window.removeEventListener('galleon:unauthorized', handleUnauthorized);
    };
  }, []);

  // When unauthenticated, attempt to fetch initial pairing code from gateway
  useEffect(() => {
    if (token) {
      return;
    }

    let active = true;

    apiClient
      .getPairCode()
      .then((res) => {
        if (!active) return;
        if (res && res.pairing_code) {
          setCode(res.pairing_code);
          setPrefilled(true);
        }
      })
      .catch((_err) => {
        // Ignored: expected when already paired or gateway not ready
      });

    return () => {
      active = false;
    };
  }, [token]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = code.trim();
    if (!trimmed) {
      setError(t('pairing.code_placeholder'));
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const res = await apiClient.pair(trimmed);
      if (res && res.token) {
        saveToken(res.token);
        setTokenState(res.token);
        setPrefilled(false);
      } else {
        throw new Error(res?.message || 'Gateway did not return a pairing token');
      }
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message || 'Failed to pair with gateway');
    } finally {
      setIsLoading(false);
    }
  };

  // If already authenticated with a token, render wrapped application
  if (token) {
    return <>{children}</>;
  }

  return (
    <div className="min-h-screen w-full bg-[#0b0c0e] flex items-center justify-center p-4 sm:p-6 text-neutral-200">
      <div className="w-full max-w-md bg-[#111215] border border-neutral-800/90 rounded-2xl p-6 sm:p-8 shadow-2xl space-y-6">
        {/* Branding header */}
        <div className="text-center space-y-3">
          <div className="flex justify-center">
            <GalleonLogo size={56} className="mx-auto" />
          </div>
          <div className="space-y-1">
            <h1 className="text-xl sm:text-2xl font-bold tracking-tight text-neutral-100">
              {t('pairing.title')}
            </h1>
            <p className="text-xs sm:text-sm text-neutral-400">
              {t('pairing.subtitle')}
            </p>
          </div>
        </div>

        {/* Error notification */}
        {error && (
          <div
            role="alert"
            className="flex items-start gap-2.5 p-3 rounded-xl bg-rose-950/40 border border-rose-800/50 text-rose-300 text-xs"
          >
            <AlertCircle className="w-4 h-4 shrink-0 text-rose-400 mt-0.5" />
            <div className="flex-1 break-words leading-relaxed">{error}</div>
          </div>
        )}

        {/* Prefilled notification note */}
        {prefilled && (
          <div className="flex items-center gap-2 p-2.5 rounded-lg bg-teal-950/40 border border-teal-800/50 text-teal-300 text-xs">
            <Sparkles className="w-3.5 h-3.5 shrink-0 text-teal-400" />
            <span>{t('pairing.prefilled_notice')}</span>
          </div>
        )}

        {/* Pairing code form */}
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5 text-left">
            <label
              htmlFor="pairing-code-input"
              className="block text-xs font-semibold text-neutral-300"
            >
              {t('pairing.code_label')}
            </label>
            <div className="relative">
              <input
                id="pairing-code-input"
                type="text"
                autoFocus
                autoComplete="off"
                spellCheck={false}
                value={code}
                onChange={(e) => {
                  setCode(e.target.value);
                  if (error) setError(null);
                }}
                disabled={isLoading}
                placeholder={t('pairing.code_placeholder')}
                className="w-full px-3.5 py-2.5 rounded-xl bg-[#090a0d] border border-neutral-700/80 focus:border-teal-500 focus:ring-1 focus:ring-teal-500 text-sm font-mono tracking-wider text-neutral-100 placeholder-neutral-500 outline-hidden transition-all disabled:opacity-50"
              />
            </div>
          </div>

          <Button
            type="submit"
            variant="primary"
            size="lg"
            loading={isLoading}
            disabled={isLoading || !code.trim()}
            className="w-full justify-center"
            icon={<Key className="w-4 h-4" />}
          >
            {isLoading ? t('pairing.submitting') : t('pairing.submit')}
          </Button>
        </form>

        {/* CLI Helper section */}
        <div className="pt-2 border-t border-neutral-800/80 space-y-2">
          <div className="flex items-center gap-1.5 text-xs text-neutral-400">
            <Info className="w-3.5 h-3.5 text-neutral-400 shrink-0" />
            <span>{t('pairing.help_text')}</span>
          </div>
          <CodeSnippet
            code="galleon gateway get-paircode"
            language="bash"
            title="Terminal"
          />
        </div>
      </div>
    </div>
  );
};
