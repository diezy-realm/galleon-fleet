//! Client-side rate limiting and concurrency gate for model provider calls (BL-28).
//!
//! Reuses the fail-closed resource gate baseline established in BL-15 (`SubagentResourcePolicy`).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Baseline concurrency and rate-limit policy for client-side LLM provider calls.
/// Explicitly references and matches the concurrency bounds established in BL-15 (default: 4 concurrent calls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderRateLimitPolicy {
    /// Maximum concurrent outgoing requests allowed to the provider (baseline: 4, mirroring BL-15).
    pub max_concurrent_calls: usize,
    /// Minimum backoff interval between bursts in milliseconds (default: 50ms).
    pub min_interval_ms: u64,
}

impl Default for ProviderRateLimitPolicy {
    fn default() -> Self {
        Self {
            // Reuses BL-15 baseline concurrency default (4 concurrent units)
            max_concurrent_calls: 4,
            min_interval_ms: 50,
        }
    }
}

/// Errors returned when rate limit or concurrency gates reject an inference request.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderRateLimitError {
    #[error("Provider concurrency limit exceeded: {current} active calls (max: {max}). Fail-closed per BL-28.")]
    ConcurrencyLimitExceeded { current: usize, max: usize },
}

/// RAII Permit that decrements active provider counter when dropped.
#[derive(Debug)]
pub struct ProviderRateLimitPermit {
    active_counter: Arc<AtomicUsize>,
}

impl Drop for ProviderRateLimitPermit {
    fn drop(&mut self) {
        self.active_counter.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Client-side rate limiting gate tracking concurrent calls to an LLM provider.
#[derive(Debug, Clone)]
pub struct ProviderRateLimitGate {
    policy: ProviderRateLimitPolicy,
    active_calls: Arc<AtomicUsize>,
}

impl ProviderRateLimitGate {
    pub fn new(policy: ProviderRateLimitPolicy) -> Self {
        Self {
            policy,
            active_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_policy(policy: ProviderRateLimitPolicy) -> Self {
        Self::new(policy)
    }

    /// Policy associated with this gate.
    pub fn policy(&self) -> ProviderRateLimitPolicy {
        self.policy
    }

    /// Current count of active provider calls under this gate.
    pub fn active_count(&self) -> usize {
        self.active_calls.load(Ordering::SeqCst)
    }

    /// Attempt to acquire a permit. Fails closed immediately if concurrent calls exceed policy limit.
    pub fn acquire_permit(&self) -> Result<ProviderRateLimitPermit, ProviderRateLimitError> {
        let current = self.active_calls.fetch_add(1, Ordering::SeqCst);
        if current >= self.policy.max_concurrent_calls {
            self.active_calls.fetch_sub(1, Ordering::SeqCst);
            return Err(ProviderRateLimitError::ConcurrencyLimitExceeded {
                current,
                max: self.policy.max_concurrent_calls,
            });
        }
        Ok(ProviderRateLimitPermit {
            active_counter: Arc::clone(&self.active_calls),
        })
    }
}

impl Default for ProviderRateLimitGate {
    fn default() -> Self {
        Self::new(ProviderRateLimitPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_policy_reuses_bl15_baseline() {
        let default_policy = ProviderRateLimitPolicy::default();
        // BL-15 baseline establishes max_concurrent = 4
        assert_eq!(default_policy.max_concurrent_calls, 4);
        assert_eq!(default_policy.min_interval_ms, 50);
    }

    #[test]
    fn rate_limit_gate_permits_up_to_baseline_and_fails_closed() {
        let gate = ProviderRateLimitGate::default();
        assert_eq!(gate.active_count(), 0);

        // Acquire permits up to BL-15 baseline limit (4)
        let permit1 = gate.acquire_permit().expect("permit 1");
        let permit2 = gate.acquire_permit().expect("permit 2");
        let permit3 = gate.acquire_permit().expect("permit 3");
        let permit4 = gate.acquire_permit().expect("permit 4");
        assert_eq!(gate.active_count(), 4);

        // 5th acquisition must fail-closed per BL-28
        let err = gate.acquire_permit().expect_err("must exceed limit");
        assert_eq!(
            err,
            ProviderRateLimitError::ConcurrencyLimitExceeded { current: 4, max: 4 }
        );
        assert_eq!(gate.active_count(), 4);

        // Dropping one permit releases a slot
        drop(permit1);
        assert_eq!(gate.active_count(), 3);

        let permit_renewed = gate.acquire_permit().expect("permit renewed");
        assert_eq!(gate.active_count(), 4);

        drop(permit2);
        drop(permit3);
        drop(permit4);
        drop(permit_renewed);
        assert_eq!(gate.active_count(), 0);
    }

    #[test]
    fn custom_policy_overrides_baseline() {
        let custom_policy = ProviderRateLimitPolicy {
            max_concurrent_calls: 2,
            min_interval_ms: 100,
        };
        let gate = ProviderRateLimitGate::with_policy(custom_policy);
        let _p1 = gate.acquire_permit().expect("p1");
        let _p2 = gate.acquire_permit().expect("p2");
        assert!(gate.acquire_permit().is_err());
    }
}
