#![forbid(unsafe_code)]

pub mod collections;
pub mod error;
pub mod intent;
pub mod metrics;
pub mod policies;
pub mod seed;
pub mod snapshots;
pub mod store;

pub use collections::{
    CANONICAL_COLLECTIONS, POLICY_DATA_COLLECTIONS, is_allowed_collection, is_canonical_collection,
    resolve_allowed_collection, validate_collection_payload,
};
pub use error::FleetError;
pub use seed::{
    FleetSeedData, SeedResponse, get_canonical_seed_map, get_policy_seed_map, get_seed_raw,
    get_seed_response_json, load_fleet_seed_data, load_seed_response,
};
pub use snapshots::{CreateSnapshotRequest, MAX_SNAPSHOTS, SnapshotItem};
pub use store::{FLEET_MAX_BODY, FleetStore, ImportReport};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_initialization() {
        let err = FleetError::Custom("scaffold initialized".to_string());
        assert_eq!(err.to_string(), "scaffold initialized");
    }
}
