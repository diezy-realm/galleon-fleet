//! Contract and unit tests for diagnostics domain model.
//!
//! Under SPEC-013 §Model Data (TASK-077):
//! - Parity with Go fixture shape `GET_api_diagnostics.json`.
//! - Strict compliance with TS `DiagnosticItem` contract in `web/src/utils/apiClient.ts`.
//! - Honest latency em-dash `"—"`, valid status string representations.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleon_fleet_core::{
    DiagnosticItem, LATENCY_UNMEASURED, STATUS_ERROR, STATUS_HEALTHY, STATUS_WARNING,
    severity_str_to_status,
};
use serde_json::Value;

#[test]
fn test_diagnostics_matches_go_fixture_shape() {
    let fixture_str = include_str!("../../../tests/fixtures/go-fleet/GET_api_diagnostics.json");
    let fixture_val: Value = serde_json::from_str(fixture_str).expect("parse go fixture");
    let fixture_len = fixture_val.as_array().expect("fixture array").len();

    // Deserialize go fixture into DiagnosticItem structs
    let deserialized: Result<Vec<DiagnosticItem>, _> = serde_json::from_value(fixture_val);
    assert!(
        deserialized.is_ok(),
        "fixture JSON must deserialize into DiagnosticItem: {:?}",
        deserialized.err()
    );

    let diags = deserialized.unwrap();
    assert_eq!(diags.len(), fixture_len);

    let first = &diags[0];
    assert_eq!(first.id, "d-1");
    assert_eq!(first.component, "Sovereign Gateway (Warp Engine)");
    assert_eq!(first.status, "unavailable");
    assert_eq!(first.latency, "0ms");
    assert!(first.detail.contains("Gateway port 50052"));
}

#[test]
fn test_diagnostic_item_builder_and_serialization() {
    let item = DiagnosticItem::new(
        "d-1",
        "workspace",
        STATUS_HEALTHY,
        LATENCY_UNMEASURED,
        "Agent soul found and verified",
    );

    let val = serde_json::to_value(&item).expect("serialize diagnostic item");
    assert_eq!(val["id"], "d-1");
    assert_eq!(val["component"], "workspace");
    assert_eq!(val["status"], "healthy");
    assert_eq!(val["latency"], "—");
    assert_eq!(val["detail"], "Agent soul found and verified");
}

#[test]
fn test_severity_str_to_status_mapping() {
    assert_eq!(severity_str_to_status("ok"), STATUS_HEALTHY);
    assert_eq!(severity_str_to_status("healthy"), STATUS_HEALTHY);
    assert_eq!(severity_str_to_status("warn"), STATUS_WARNING);
    assert_eq!(severity_str_to_status("warning"), STATUS_WARNING);
    assert_eq!(severity_str_to_status("error"), STATUS_ERROR);
    assert_eq!(severity_str_to_status("unknown"), STATUS_WARNING);
}
