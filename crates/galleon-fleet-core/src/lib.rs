#![forbid(unsafe_code)]

pub mod briefing;
pub mod collections;
pub mod error;
pub mod intent;
pub mod metrics;
pub mod policies;
pub mod project;
pub mod seed;
pub mod snapshots;
pub mod store;

pub use briefing::{
    calculate_executive_briefing, calculate_treasury_burn_rate, ExecutiveBriefing,
    DEFAULT_BRIEFING_TITLE,
};
pub use collections::{
    is_allowed_collection, is_canonical_collection, resolve_allowed_collection,
    validate_collection_payload, CANONICAL_COLLECTIONS, POLICY_DATA_COLLECTIONS,
};
pub use error::FleetError;
pub use metrics::{calculate_fleet_metrics, DeckBellResponse, FleetMetrics, DECK_BELL_MESSAGE};
pub use policies::{
    load_seed_policies, load_seed_policies_response, load_seed_risk_tiers, FleetPoliciesResponse,
    FleetPolicy, RiskTier,
};
pub use project::{parse_git_status, ProjectGitStatus};
pub use seed::{
    get_canonical_seed_map, get_policy_seed_map, get_seed_raw, get_seed_response_json,
    load_fleet_seed_data, load_seed_response, FleetSeedData, SeedResponse,
};
pub use snapshots::{CreateSnapshotRequest, SnapshotItem, MAX_SNAPSHOTS};
pub use store::{FleetStore, ImportReport, FLEET_MAX_BODY};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_initialization() {
        let err = FleetError::Custom("scaffold initialized".to_string());
        assert_eq!(err.to_string(), "scaffold initialized");
    }
}
