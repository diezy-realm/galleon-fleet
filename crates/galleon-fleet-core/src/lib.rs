#![forbid(unsafe_code)]

pub mod collections;
pub mod error;
pub mod intent;
pub mod metrics;
pub mod policies;
pub mod seed;
pub mod snapshots;
pub mod store;

pub use error::FleetError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_initialization() {
        let err = FleetError::Custom("scaffold initialized".to_string());
        assert_eq!(err.to_string(), "scaffold initialized");
    }
}
