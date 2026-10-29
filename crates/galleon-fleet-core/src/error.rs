//! Domain error types for galleon-fleet-core.

use thiserror::Error;

/// Domain errors encountered in fleet operations.
#[derive(Debug, Error)]
pub enum FleetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Custom(String),
}
