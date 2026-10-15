//! Provider reliability tracking and health metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Cooldown is applied after this many consecutive failures.
const COOLDOWN_FAILURE_THRESHOLD: u32 = 3;

/// Default cooldown minutes recorded by the live provider-health registry.
const PROVIDER_COOLDOWN_MINUTES: u32 = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthRecord {
    pub provider_name: String,
    pub total_invocations: u64,
    pub successful_invocations: u64,
    pub consecutive_failures: u32,
    pub in_cooldown_until: Option<String>,
    /// Sum of observed latencies in milliseconds (for the average).
    #[serde(default)]
    pub total_latency_ms: u64,
    /// Most recent observed latency in milliseconds.
    #[serde(default)]
    pub last_latency_ms: Option<u64>,
}

impl ProviderHealthRecord {
    pub fn new(provider_name: String) -> Self {
        Self {
            provider_name,
            total_invocations: 0,
            successful_invocations: 0,
            consecutive_failures: 0,
            in_cooldown_until: None,
            total_latency_ms: 0,
            last_latency_ms: None,
        }
    }

    pub fn record_success(&mut self) {
        self.total_invocations += 1;
        self.successful_invocations += 1;
        self.consecutive_failures = 0;
        self.in_cooldown_until = None;
    }

    /// Record a success and the observed latency in milliseconds.
    pub fn record_success_with_latency(&mut self, latency_ms: u64) {
        self.record_success();
        self.observe_latency(latency_ms);
    }

    pub fn record_failure(&mut self, cooldown_minutes: u32) {
        self.total_invocations += 1;
        self.consecutive_failures += 1;
        if self.consecutive_failures > COOLDOWN_FAILURE_THRESHOLD {
            let now = chrono::Utc::now();
            let cooldown = now + chrono::Duration::minutes(cooldown_minutes as i64);
            self.in_cooldown_until = Some(cooldown.to_rfc3339());
        }
    }

    /// Record a failure and the observed latency in milliseconds.
    pub fn record_failure_with_latency(&mut self, cooldown_minutes: u32, latency_ms: u64) {
        self.record_failure(cooldown_minutes);
        self.observe_latency(latency_ms);
    }

    fn observe_latency(&mut self, latency_ms: u64) {
        self.total_latency_ms = self.total_latency_ms.saturating_add(latency_ms);
        self.last_latency_ms = Some(latency_ms);
    }

    pub fn is_in_cooldown(&self) -> bool {
        if let Some(ref until_str) = self.in_cooldown_until {
            if let Ok(until_time) = chrono::DateTime::parse_from_rfc3339(until_str) {
                return chrono::Utc::now() < until_time.with_timezone(&chrono::Utc);
            }
        }
        false
    }

    /// Observed success ratio in `[0.0, 1.0]`; `1.0` when never invoked (an
    /// unproven provider is not treated as unreliable).
    pub fn success_rate(&self) -> f64 {
        if self.total_invocations == 0 {
            return 1.0;
        }
        self.successful_invocations as f64 / self.total_invocations as f64
    }

    /// Mean observed latency in milliseconds, or `None` when never observed.
    pub fn avg_latency_ms(&self) -> Option<f64> {
        if self.total_invocations == 0 {
            return None;
        }
        Some(self.total_latency_ms as f64 / self.total_invocations as f64)
    }

    /// A provider is selectable when it is not in cooldown.
    pub fn is_available(&self) -> bool {
        !self.is_in_cooldown()
    }
}

impl Default for ProviderHealthRecord {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Process-wide live provider health registry, fed at the provider-call
/// boundary so `/api/providers/health` reports true status/latency.
static HEALTH: OnceLock<Mutex<HashMap<String, ProviderHealthRecord>>> = OnceLock::new();

fn health_registry() -> &'static Mutex<HashMap<String, ProviderHealthRecord>> {
    HEALTH.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record one provider call outcome (retained for the process lifetime).
pub fn record_provider_outcome(provider: &str, success: bool, latency_ms: u64) {
    if provider.trim().is_empty() {
        return;
    }
    let Ok(mut map) = health_registry().lock() else {
        return;
    };
    let entry = map
        .entry(provider.to_string())
        .or_insert_with(|| ProviderHealthRecord::new(provider.to_string()));
    if success {
        entry.record_success_with_latency(latency_ms);
    } else {
        entry.record_failure_with_latency(PROVIDER_COOLDOWN_MINUTES, latency_ms);
    }
}

/// Snapshot the live provider health records.
pub fn provider_snapshot() -> Vec<ProviderHealthRecord> {
    health_registry()
        .lock()
        .map(|map| map.values().cloned().collect())
        .unwrap_or_default()
}
