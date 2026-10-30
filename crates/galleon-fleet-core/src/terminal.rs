//! Terminal execution and audit domain models and ring buffer.
//!
//! Under SPEC-013 §Terminal (TASK-078):
//! - `POST /api/terminal/exec` request and response models.
//! - `TerminalAuditRecord` and in-memory ring buffer of 500 entries.
//! - Max execution output is 64 KB (65,536 bytes).
//! - Timeout is 30 seconds.
//! - No unwrap / expect in production code.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Maximum entries retained in the in-memory terminal audit ring buffer.
pub const MAX_AUDIT_ENTRIES: usize = 500;

/// Default timeout in seconds for terminal command execution.
pub const DEFAULT_TERMINAL_TIMEOUT_SECS: u64 = 30;

/// Maximum bytes retained for stdout and stderr (64 KB).
pub const MAX_TERMINAL_OUTPUT_BYTES: usize = 64 * 1024;

/// Request payload for `POST /api/terminal/exec`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalExecRequest {
    /// Command string to execute.
    pub command: String,
    /// Optional approved flag sent by client (ignored by backend under SPEC-013).
    #[serde(default)]
    pub approved: Option<bool>,
    /// Optional approved_by identifier sent by client (ignored by backend under SPEC-013).
    #[serde(default)]
    pub approved_by: Option<String>,
}

/// Response payload for `POST /api/terminal/exec`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalExecResponse {
    /// Standard output of the executed process (or empty on failure/approval gate).
    pub stdout: String,
    /// Standard error of the process, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    /// Process exit code, or -1 when gated for approval or timed out.
    pub exit_code: i32,
    /// Duration of the execution or check (e.g. "41ms", "0s").
    pub duration: String,
    /// Risk tier classification: "low", "medium", "high", or "blocked".
    pub risk_tier: String,
    /// True if command execution was halted pending verified manual approval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_approval: Option<bool>,
    /// Unique audit record identifier generated for this execution attempt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_id: Option<String>,
    /// Error message if policy rejection or execution failure occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Single record in the terminal execution audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalAuditRecord {
    /// Unique audit record ID (e.g. "audit-c0c0b22f392386a0").
    pub id: String,
    /// RFC3339 timestamp of the execution attempt.
    pub timestamp: String,
    /// Evaluated command string.
    pub command: String,
    /// Assessed risk tier ("low", "medium", "high", "blocked").
    pub risk_tier: String,
    /// Whether explicit authenticated approval was granted (always false from client input).
    pub approved: bool,
    /// Identity that granted approval, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    /// True if execution completed successfully (exitCode == 0).
    pub success: bool,
    /// Process exit code, or non-zero / -1 on failure or approval gate.
    pub exit_code: i32,
    /// Elapsed duration string.
    pub duration: String,
    /// Error description if execution failed or was rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// In-memory ring buffer retaining up to `max_size` (default 500) terminal audit records.
#[derive(Debug, Clone)]
pub struct TerminalAuditRingBuffer {
    records: VecDeque<TerminalAuditRecord>,
    max_size: usize,
}

impl Default for TerminalAuditRingBuffer {
    fn default() -> Self {
        Self::new(MAX_AUDIT_ENTRIES)
    }
}

impl TerminalAuditRingBuffer {
    /// Creates a new `TerminalAuditRingBuffer` with the specified capacity.
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        let capacity = if max_size == 0 {
            MAX_AUDIT_ENTRIES
        } else {
            max_size
        };
        Self {
            records: VecDeque::with_capacity(capacity),
            max_size: capacity,
        }
    }

    /// Appends a new audit record into the ring buffer, evicting the oldest record if full.
    /// Returns the audit record ID (generating one if not already provided).
    pub fn record(&mut self, mut entry: TerminalAuditRecord) -> String {
        if entry.id.is_empty() {
            entry.id = compute_audit_id(&entry.timestamp, &entry.command, &entry.risk_tier);
        }
        let id = entry.id.clone();

        if self.records.len() >= self.max_size {
            self.records.pop_front();
        }
        self.records.push_back(entry);
        id
    }

    /// Lists up to `limit` audit records in chronological order.
    /// If `limit` is `None` or greater than the record count, all records are returned.
    #[must_use]
    pub fn list(&self, limit: Option<usize>) -> Vec<TerminalAuditRecord> {
        let total = self.records.len();
        let take_count = match limit {
            Some(n) if n > 0 && n < total => n,
            _ => total,
        };
        let start = total - take_count;
        self.records.iter().skip(start).cloned().collect()
    }

    /// Number of records currently retained in the buffer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the buffer is currently empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Clears all retained records from the buffer.
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

/// Generates a deterministic audit identifier in the format `audit-<16 hex chars>`
/// using SHA-256 over timestamp, command, and risk tier.
#[must_use]
pub fn compute_audit_id(timestamp: &str, command: &str, risk_tier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(timestamp.as_bytes());
    hasher.update(b":");
    hasher.update(command.as_bytes());
    hasher.update(b":");
    hasher.update(risk_tier.as_bytes());
    let hash = hasher.finalize();
    // First 8 bytes formatted as 16 hex characters
    let mut hex_buf = [0u8; 16];
    let hex_chars = b"0123456789abcdef";
    for (i, &byte) in hash[..8].iter().enumerate() {
        hex_buf[i * 2] = hex_chars[(byte >> 4) as usize];
        hex_buf[i * 2 + 1] = hex_chars[(byte & 0x0f) as usize];
    }
    let hex_str = std::str::from_utf8(&hex_buf).unwrap_or("0000000000000000");
    format!("audit-{hex_str}")
}

/// Safely truncates a UTF-8 string to at most `max_bytes` without splitting code points.
#[must_use]
pub fn truncate_output(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}
