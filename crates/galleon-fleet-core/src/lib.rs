#![forbid(unsafe_code)]

pub mod briefing;
pub mod collections;
pub mod diagnostics;
pub mod error;
pub mod harbor;
pub mod intent;
pub mod metrics;
pub mod policies;
pub mod project;
pub mod seed;
pub mod snapshots;
pub mod store;
pub mod terminal;

pub use briefing::{
    DEFAULT_BRIEFING_TITLE, ExecutiveBriefing, calculate_executive_briefing,
    calculate_treasury_burn_rate,
};
pub use collections::{
    CANONICAL_COLLECTIONS, POLICY_DATA_COLLECTIONS, is_allowed_collection, is_canonical_collection,
    resolve_allowed_collection, validate_collection_payload,
};
pub use diagnostics::{
    DiagnosticItem, LATENCY_UNMEASURED, STATUS_ERROR, STATUS_HEALTHY, STATUS_WARNING,
    severity_str_to_status,
};
pub use error::FleetError;
pub use harbor::{
    HarborProvider, STATUS_CONFIGURED, STATUS_MISSING_CREDENTIALS, build_harbor_provider,
    resolve_capabilities, resolve_vendor_and_name,
};
pub use metrics::{DECK_BELL_MESSAGE, DeckBellResponse, FleetMetrics, calculate_fleet_metrics};
pub use policies::{
    FleetPoliciesResponse, FleetPolicy, RiskTier, load_seed_policies, load_seed_policies_response,
    load_seed_risk_tiers,
};
pub use project::{ProjectGitStatus, parse_git_status};
pub use seed::{
    FleetSeedData, SeedResponse, get_canonical_seed_map, get_policy_seed_map, get_seed_raw,
    get_seed_response_json, load_fleet_seed_data, load_seed_response,
};
pub use snapshots::{CreateSnapshotRequest, MAX_SNAPSHOTS, SnapshotItem};
pub use store::{FLEET_MAX_BODY, FleetStore, ImportReport};
pub use terminal::{
    DEFAULT_TERMINAL_TIMEOUT_SECS, MAX_AUDIT_ENTRIES, MAX_TERMINAL_OUTPUT_BYTES,
    TerminalAuditRecord, TerminalAuditRingBuffer, TerminalExecRequest, TerminalExecResponse,
    compute_audit_id, truncate_output,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_initialization() {
        let err = FleetError::Custom("scaffold initialized".to_string());
        assert_eq!(err.to_string(), "scaffold initialized");
    }
}
