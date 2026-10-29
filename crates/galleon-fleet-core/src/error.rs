//! Domain error types for galleon-fleet-core.

use thiserror::Error;

/// Domain errors encountered in fleet operations.
#[derive(Debug, Error)]
pub enum FleetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("collection exceeds 1 MiB limit ({size} bytes, limit: {limit} bytes)")]
    PayloadTooLarge { size: usize, limit: usize },

    #[error("unknown or invalid collection: {0}")]
    InvalidCollection(String),

    #[error("collection not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Custom(String),
}
