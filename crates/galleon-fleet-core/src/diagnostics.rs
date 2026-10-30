//! Diagnostics domain model and utilities.
//!
//! Under SPEC-013 §Model Data (TASK-077):
//! - `GET /api/diagnostics` returns a list of `DiagnosticItem`.
//! - Honest latency: `"—"` (`LATENCY_UNMEASURED`).
//! - Status mapped from runtime doctor severity: `healthy`, `warning`, `error`.
//! - No unwrap / expect in production code.

use serde::{Deserialize, Serialize};

/// Em-dash string used for honest latency display when no ping is measured.
pub const LATENCY_UNMEASURED: &str = "—";

/// Canonical diagnostic status strings.
pub const STATUS_HEALTHY: &str = "healthy";
pub const STATUS_WARNING: &str = "warning";
pub const STATUS_ERROR: &str = "error";

/// Diagnostic item matching `web/src/utils/apiClient.ts` `DiagnosticItem` contract
/// and `tests/fixtures/go-fleet/GET_api_diagnostics.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticItem {
    /// Unique identifier for the diagnostic check (e.g. "d-1", "d-2").
    pub id: String,
    /// System component or category being inspected (e.g. "workspace", "daemon").
    pub component: String,
    /// Diagnostic status: "healthy", "warning", or "error".
    pub status: String,
    /// Response latency string: "—" for unmeasured honest metrics, or measured latency from Go fixtures.
    pub latency: String,
    /// Human-readable detail message describing the check outcome.
    pub detail: String,
}

impl DiagnosticItem {
    /// Constructs a new `DiagnosticItem`.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        component: impl Into<String>,
        status: impl Into<String>,
        latency: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            component: component.into(),
            status: status.into(),
            latency: latency.into(),
            detail: detail.into(),
        }
    }
}

/// Normalizes severity string to canonical status.
#[must_use]
pub fn severity_str_to_status(severity: &str) -> &'static str {
    match severity.to_ascii_lowercase().as_str() {
        "ok" | "healthy" => STATUS_HEALTHY,
        "warn" | "warning" => STATUS_WARNING,
        "error" => STATUS_ERROR,
        _ => STATUS_WARNING,
    }
}
