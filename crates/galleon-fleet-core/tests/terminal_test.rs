//! Tests for terminal execution domain models and audit ring buffer.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleon_fleet_core::{
    MAX_AUDIT_ENTRIES, MAX_TERMINAL_OUTPUT_BYTES, TerminalAuditRecord, TerminalAuditRingBuffer,
    TerminalExecRequest, TerminalExecResponse, compute_audit_id, truncate_output,
};
use serde_json::json;

#[test]
fn test_compute_audit_id_format_and_determinism() {
    let id1 = compute_audit_id("2026-10-30T10:00:00Z", "git status", "low");
    let id2 = compute_audit_id("2026-10-30T10:00:00Z", "git status", "low");
    let id3 = compute_audit_id("2026-10-30T10:00:01Z", "git status", "low");

    assert_eq!(id1, id2, "audit ID must be deterministic");
    assert_ne!(id1, id3, "different timestamp produces different audit ID");
    assert!(id1.starts_with("audit-"), "must start with audit- prefix");
    assert_eq!(id1.len(), 22, "audit- plus 16 hex chars = 22 characters");
}

#[test]
fn test_ring_buffer_eviction_at_capacity() {
    let mut ring = TerminalAuditRingBuffer::new(5);
    assert_eq!(ring.len(), 0);
    assert!(ring.is_empty());

    for i in 0..7 {
        let entry = TerminalAuditRecord {
            id: format!("audit-{i}"),
            timestamp: format!("2026-10-30T10:00:0{i}Z"),
            command: format!("cmd-{i}"),
            risk_tier: "low".to_string(),
            approved: false,
            approved_by: None,
            success: true,
            exit_code: 0,
            duration: "10ms".to_string(),
            error: None,
        };
        ring.record(entry);
    }

    assert_eq!(ring.len(), 5, "ring buffer must not exceed capacity");
    let list = ring.list(None);
    assert_eq!(list.len(), 5);
    // Oldest two (0 and 1) should have been evicted
    assert_eq!(list[0].id, "audit-2");
    assert_eq!(list[4].id, "audit-6");

    // Test list with smaller limit
    let limited = ring.list(Some(3));
    assert_eq!(limited.len(), 3);
    assert_eq!(limited[0].id, "audit-4");
    assert_eq!(limited[2].id, "audit-6");
}

#[test]
fn test_ring_buffer_default_capacity() {
    let ring = TerminalAuditRingBuffer::default();
    assert_eq!(ring.len(), 0);
    // Buffer created with MAX_AUDIT_ENTRIES = 500
    assert_eq!(MAX_AUDIT_ENTRIES, 500);
}

#[test]
fn test_truncate_output_boundaries() {
    assert_eq!(truncate_output("hello", 10), "hello");
    assert_eq!(truncate_output("hello world", 5), "hello");

    // Multibyte UTF-8 test (🦀 is 4 bytes: [240, 159, 166, 128])
    let crab = "🦀";
    assert_eq!(crab.len(), 4);
    // Slicing at 3 bytes must not panic and must backtrack to 0
    assert_eq!(truncate_output(crab, 3), "");
    assert_eq!(truncate_output(crab, 4), "🦀");

    let text_with_crab = "rust🦀fast";
    // "rust" is 4 bytes, crab is 4 bytes (ends at index 8)
    assert_eq!(truncate_output(text_with_crab, 6), "rust");
    assert_eq!(truncate_output(text_with_crab, 8), "rust🦀");
}

#[test]
fn test_terminal_exec_request_response_serde() {
    let req_json = json!({
        "command": "git status",
        "approved": true,
        "approved_by": "web_terminal_ui"
    });
    let req: TerminalExecRequest = serde_json::from_value(req_json).expect("deserialize request");
    assert_eq!(req.command, "git status");
    assert_eq!(req.approved, Some(true));

    let resp = TerminalExecResponse {
        stdout: "On branch testing".to_string(),
        stderr: None,
        exit_code: 0,
        duration: "41ms".to_string(),
        risk_tier: "low".to_string(),
        requires_approval: None,
        audit_id: Some("audit-123".to_string()),
        error: None,
    };
    let serialized = serde_json::to_value(&resp).expect("serialize response");
    assert_eq!(serialized["stdout"], "On branch testing");
    assert_eq!(serialized["exitCode"], 0);
    assert_eq!(serialized["duration"], "41ms");
    assert_eq!(serialized["riskTier"], "low");
    assert_eq!(serialized["auditId"], "audit-123");
    assert!(serialized.get("stderr").is_none());
    assert!(serialized.get("requiresApproval").is_none());
}

#[test]
fn test_max_terminal_output_bytes_constant() {
    assert_eq!(MAX_TERMINAL_OUTPUT_BYTES, 65536);
}
