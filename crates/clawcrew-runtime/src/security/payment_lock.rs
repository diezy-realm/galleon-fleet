//! Payment Lockout Gate Manager (BL-41, BL-44).
//!
//! Enforces preventive fail-closed isolation preventing agents from invoking
//! payment or financial transactions unless deliberately unlocked by the operator.
//! Compliant with OWASP Agentic ASI09/ASI10 and AP2/FIDO preventive security principles.

use clawcrew_config::schema::{PaymentActionCategory, PaymentLockConfig, PaymentLockMode};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Reasons for denying a payment or financial tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PaymentLockDenied {
    #[error("Payment operations are globally locked (fail-closed per BL-41). Agents cannot execute financial transactions.")]
    GloballyLocked,

    #[error("Payment unlock expired at unix timestamp {expired_at} (current: {now}). Fail-closed per BL-41.")]
    UnlockExpired { expired_at: u64, now: u64 },

    #[error("Scope '{scope}' is not included in the unlocked scopes for payment operations.")]
    ScopeNotUnlocked { scope: String },

    #[error("Payment category '{category:?}' is not included in the permitted categories.")]
    CategoryNotUnlocked { category: PaymentActionCategory },
}

/// Raw, un-summarized state of the payment lock gate (OWASP ASI09 compliant).
/// Guaranteed to represent direct system configuration rather than agent-interpreted summaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaymentLockRawState {
    pub mode: PaymentLockMode,
    pub locked: bool,
    pub unlocked_scopes: Vec<String>,
    pub require_otp_to_unlock: bool,
    pub unlock_expires_at: Option<u64>,
    pub covered_categories: Vec<PaymentActionCategory>,
    pub is_effective_locked: bool,
}

/// Thread-safe manager responsible for gating payment-category tool execution (BL-41).
#[derive(Debug, Clone)]
pub struct PaymentLockManager {
    config: Arc<RwLock<PaymentLockConfig>>,
}

impl PaymentLockManager {
    /// Creates a new manager with the provided configuration.
    pub fn new(config: PaymentLockConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Creates a manager with default configuration (Locked, fail-closed).
    pub fn default_locked() -> Self {
        Self::new(PaymentLockConfig::default())
    }

    /// Returns the raw configuration snapshot.
    pub fn config(&self) -> PaymentLockConfig {
        self.config.read().clone()
    }

    /// Checks whether an action in the specified payment category is allowed to proceed right now.
    pub fn check_allowed(
        &self,
        category: PaymentActionCategory,
        scope: Option<&str>,
    ) -> Result<(), PaymentLockDenied> {
        let cfg = self.config.read();

        // 1. Check global lockout mode
        if cfg.mode == PaymentLockMode::Locked {
            return Err(PaymentLockDenied::GloballyLocked);
        }

        // 2. Check automatic expiry (fail-closed)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(expires_at) = cfg.unlock_expires_at {
            if now >= expires_at {
                return Err(PaymentLockDenied::UnlockExpired {
                    expired_at: expires_at,
                    now,
                });
            }
        }

        // 3. Category coverage
        if !cfg.covered_categories.contains(&category) {
            // Category explicitly opted out by operator; allowed through normal approval flow
            return Ok(());
        }

        // 4. Scope authorization
        if let Some(target_scope) = scope {
            if !cfg.unlocked_scopes.is_empty()
                && !cfg
                    .unlocked_scopes
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(target_scope))
            {
                return Err(PaymentLockDenied::ScopeNotUnlocked {
                    scope: target_scope.to_string(),
                });
            }
        } else if !cfg.unlocked_scopes.is_empty() {
            return Err(PaymentLockDenied::ScopeNotUnlocked {
                scope: "unspecified".to_string(),
            });
        }

        Ok(())
    }

    /// Unlocks the payment gate with explicit operator attribution and audit logging (BL-44).
    pub fn unlock(
        &self,
        actor: &str,
        reason: &str,
        scopes: Vec<String>,
        expires_at: Option<u64>,
    ) -> anyhow::Result<()> {
        let mut cfg = self.config.write();
        let prev_mode = cfg.mode;
        cfg.mode = PaymentLockMode::Unlocked;
        cfg.unlocked_scopes = scopes.clone();
        cfg.unlock_expires_at = expires_at;

        // Emit audit record (BL-44)
        let now_str = chrono::Utc::now().to_rfc3339();
        ::clawcrew_log::record!(
            WARN,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                .with_outcome(::clawcrew_log::EventOutcome::Success)
                .with_attrs(::serde_json::json!({
                    "audit_event": "PaymentLockTransition",
                    "actor": actor,
                    "previous_mode": format!("{prev_mode:?}"),
                    "new_mode": "Unlocked",
                    "reason": reason,
                    "unlocked_scopes": scopes,
                    "expires_at": expires_at,
                    "timestamp": now_str,
                })),
            &format!(
                "Payment lockout gate transitioned to Unlocked by {actor}: {reason}"
            )
        );

        Ok(())
    }

    /// Re-locks the payment gate immediately with explicit attribution and audit logging (BL-44).
    pub fn lock(&self, actor: &str) -> anyhow::Result<()> {
        let mut cfg = self.config.write();
        let prev_mode = cfg.mode;
        cfg.mode = PaymentLockMode::Locked;
        cfg.unlock_expires_at = None;

        let now_str = chrono::Utc::now().to_rfc3339();
        ::clawcrew_log::record!(
            INFO,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                .with_outcome(::clawcrew_log::EventOutcome::Success)
                .with_attrs(::serde_json::json!({
                    "audit_event": "PaymentLockTransition",
                    "actor": actor,
                    "previous_mode": format!("{prev_mode:?}"),
                    "new_mode": "Locked",
                    "timestamp": now_str,
                })),
            &format!("Payment lockout gate locked by {actor}")
        );

        Ok(())
    }

    /// Extracts the raw, unmanipulated state for frontend consumption (OWASP ASI09, BL-43).
    pub fn raw_state(&self) -> PaymentLockRawState {
        let cfg = self.config.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let is_expired = cfg
            .unlock_expires_at
            .map(|exp| now >= exp)
            .unwrap_or(false);

        let is_effective_locked = cfg.mode == PaymentLockMode::Locked || is_expired;

        PaymentLockRawState {
            mode: cfg.mode,
            locked: cfg.mode == PaymentLockMode::Locked,
            unlocked_scopes: cfg.unlocked_scopes.clone(),
            require_otp_to_unlock: cfg.require_otp_to_unlock,
            unlock_expires_at: cfg.unlock_expires_at,
            covered_categories: cfg.covered_categories.clone(),
            is_effective_locked,
        }
    }
}

impl Default for PaymentLockManager {
    fn default() -> Self {
        Self::default_locked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state_is_locked_fail_closed() {
        let manager = PaymentLockManager::default_locked();
        let check = manager.check_allowed(PaymentActionCategory::CardPayment, Some("fleet-finance"));
        assert_eq!(check, Err(PaymentLockDenied::GloballyLocked));

        let state = manager.raw_state();
        assert!(state.locked);
        assert!(state.is_effective_locked);
        assert_eq!(state.mode, PaymentLockMode::Locked);
    }

    #[test]
    fn test_unlocked_with_valid_scope_succeeds() {
        let manager = PaymentLockManager::default_locked();
        manager
            .unlock(
                "operator-alice",
                "test transaction",
                vec!["fleet-finance".to_string()],
                Some(u64::MAX),
            )
            .expect("unlock should succeed");

        let allowed = manager.check_allowed(PaymentActionCategory::CardPayment, Some("fleet-finance"));
        assert!(allowed.is_ok());

        let denied_scope = manager.check_allowed(PaymentActionCategory::CardPayment, Some("fleet-ops"));
        assert!(matches!(
            denied_scope,
            Err(PaymentLockDenied::ScopeNotUnlocked { .. })
        ));
    }

    #[test]
    fn test_unlock_expiration_fails_closed() {
        let manager = PaymentLockManager::default_locked();
        // Set expiry 1 second in the past
        let past_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(10);

        manager
            .unlock(
                "operator-bob",
                "expired unlock test",
                vec!["fleet-finance".to_string()],
                Some(past_ts),
            )
            .expect("unlock should succeed");

        let check = manager.check_allowed(PaymentActionCategory::CryptoTransaction, Some("fleet-finance"));
        assert!(matches!(
            check,
            Err(PaymentLockDenied::UnlockExpired { .. })
        ));

        let state = manager.raw_state();
        assert!(!state.locked); // Mode was unlocked
        assert!(state.is_effective_locked); // But effectively locked due to expiry
    }

    #[test]
    fn test_lock_transition_restores_locked_state() {
        let manager = PaymentLockManager::default_locked();
        manager
            .unlock("operator", "temporary", vec![], Some(u64::MAX))
            .unwrap();
        assert_eq!(manager.raw_state().mode, PaymentLockMode::Unlocked);

        manager.lock("operator").unwrap();
        assert_eq!(manager.raw_state().mode, PaymentLockMode::Locked);
        assert!(manager.raw_state().is_effective_locked);
    }
}
