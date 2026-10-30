//! Fleet metrics domain types and calculations.
//!
//! Under SPEC-013 §Model Data:
//! - `GET /api/fleet/metrics` serves aggregate counts across fleet collections,
//!   with honest telemetry where unmeasured signals return `null`.
//! - `POST /api/fleet/deck-bell` serves the ceremonial alert confirmation.

use serde::{Deserialize, Serialize};

use crate::store::FleetStore;

/// Ceremonial announcement message broadcast by `POST /api/fleet/deck-bell`.
pub const DECK_BELL_MESSAGE: &str =
    "Sovereign Ship Bell sounded across the fleet deck. All hands on alert.";

/// Response returned by `POST /api/fleet/deck-bell`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckBellResponse {
    pub message: String,
    pub success: bool,
}

impl Default for DeckBellResponse {
    fn default() -> Self {
        Self {
            message: DECK_BELL_MESSAGE.to_string(),
            success: true,
        }
    }
}

/// Metrics snapshot for Galleon Fleet dashboard (`GET /api/fleet/metrics`).
///
/// Follows the contract with `web/src/utils/apiClient.ts:75-87`:
/// - Collection counters reflect real array counts stored in `FleetStore`.
/// - `underway_quests` counts quests with status "underway", "in_progress", or "active".
/// - `pending_approvals` counts approvals with status "pending".
/// - Unmeasured or uninstrumented telemetry fields are honestly serialized as `null`
///   (SPEC-013 intentional difference F-13: `systemUptime`, `gatewayLatencyMs`, `activeWorkers`).
/// - `memory_used_mb` is provided by the gateway adapter from host RSS via `sysinfo`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetMetrics {
    pub active_vessels: usize,
    pub underway_quests: usize,
    pub pending_approvals: usize,
    pub total_treasury_ledgers: usize,
    pub total_artifacts: usize,
    pub total_specialists: usize,
    pub total_squads: usize,
    pub system_uptime: Option<String>,
    pub gateway_latency_ms: Option<f64>,
    pub active_workers: Option<usize>,
    #[serde(rename = "memoryUsedMB")]
    pub memory_used_mb: Option<f64>,
}

/// Calculates `FleetMetrics` from a `FleetStore` and optional process RSS memory (in MB).
pub fn calculate_fleet_metrics(store: &FleetStore, memory_used_mb: Option<f64>) -> FleetMetrics {
    FleetMetrics {
        active_vessels: store.count_collection("ships"),
        underway_quests: store.count_by_status("quests", &["underway", "in_progress", "active"]),
        pending_approvals: store.count_by_status("approvals", &["pending"]),
        total_treasury_ledgers: store.count_collection("treasuryLedger"),
        total_artifacts: store.count_collection("artifacts"),
        total_specialists: store.count_collection("crew"),
        total_squads: store.count_collection("squads"),
        system_uptime: None,
        gateway_latency_ms: None,
        active_workers: None,
        memory_used_mb,
    }
}
