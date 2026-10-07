//! Tool input/output capture: leak-scan + truncation + denylist.

use crate::config::{LlmRequestPayloadPolicy, ResolvedPolicy, ToolIoPolicy};

/// Result of a tool-io capture pass. The string in `text` is what should
/// land in the `attributes.tool_input` (or `tool_output`) field. Metadata
/// goes into `original_bytes` / `truncated` so the dashboard can render
/// a "truncated" badge.
#[derive(Debug, Clone)]
pub struct ToolIoCapture {
    pub text: String,
    pub original_bytes: usize,
    pub truncated: bool,
}

/// Capture redacted tool input.
///
/// `redacted` is the input string AFTER the runtime has scanned it for
/// credential leaks (using `clawcrew_runtime::security::LeakDetector`).
/// This function only handles truncation + denylist enforcement.
///
/// Returns `None` when policy/denylist says to skip capture entirely.
#[must_use]
pub fn capture_tool_input(
    policy: &ResolvedPolicy,
    tool: &str,
    redacted: &str,
) -> Option<ToolIoCapture> {
    capture_with_policy(policy, tool, redacted)
}

/// Capture redacted tool output. Same shape as [`capture_tool_input`].
#[must_use]
pub fn capture_tool_output(
    policy: &ResolvedPolicy,
    tool: &str,
    redacted: &str,
) -> Option<ToolIoCapture> {
    capture_with_policy(policy, tool, redacted)
}

/// Returns true if a tool is conversational (e.g. messaging, chat, channel reply).
/// Conversational tools default to `ToolIoPolicy::Off` to safeguard user chat privacy.
#[must_use]
pub fn is_conversational_tool(tool: &str) -> bool {
    let t = tool.trim().to_ascii_lowercase();
    t == "send_message_to_peer"
        || t == "send_message"
        || t == "chat_message"
        || t == "message_user"
        || t == "channel_reply"
        || t.contains("send_message")
        || t.contains("chat")
        || t.contains("conversation")
}

fn capture_with_policy(
    policy: &ResolvedPolicy,
    tool: &str,
    redacted: &str,
) -> Option<ToolIoCapture> {
    let effective_policy = policy.policy_for_tool(tool);
    if !effective_policy.captures_io() {
        return None;
    }
    if policy.is_tool_denylisted(tool) {
        return None;
    }
    let original_bytes = redacted.len();
    match effective_policy {
        ToolIoPolicy::Off => None,
        ToolIoPolicy::Full => Some(ToolIoCapture {
            text: redacted.to_string(),
            original_bytes,
            truncated: false,
        }),
        ToolIoPolicy::Redacted => Some(truncate_to_cap(redacted, policy.tool_io_truncate_bytes)),
    }
}

#[must_use]
pub fn capture_llm_request(
    policy: LlmRequestPayloadPolicy,
    truncate_bytes: usize,
    redacted: &str,
) -> Option<ToolIoCapture> {
    match policy {
        LlmRequestPayloadPolicy::Off => None,
        LlmRequestPayloadPolicy::Full => Some(ToolIoCapture {
            text: redacted.to_string(),
            original_bytes: redacted.len(),
            truncated: false,
        }),
        LlmRequestPayloadPolicy::Redacted => Some(truncate_to_cap(redacted, truncate_bytes)),
    }
}

/// Truncate `redacted` to at most `cap` bytes on a char boundary, flagging
/// whether truncation occurred and the original byte length.
fn truncate_to_cap(redacted: &str, cap: usize) -> ToolIoCapture {
    let original_bytes = redacted.len();
    if original_bytes <= cap {
        return ToolIoCapture {
            text: redacted.to_string(),
            original_bytes,
            truncated: false,
        };
    }
    let mut acc = String::with_capacity(cap);
    for ch in redacted.chars() {
        if acc.len() + ch.len_utf8() > cap {
            break;
        }
        acc.push(ch);
    }
    ToolIoCapture {
        text: acc,
        original_bytes,
        truncated: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LogConfig;

    fn make_policy(io: &str, cap: usize, denylist: Vec<String>) -> ResolvedPolicy {
        let cfg = LogConfig {
            log_tool_io: io.into(),
            log_tool_io_truncate_bytes: cap,
            log_tool_io_denylist: denylist,
            ..LogConfig::default()
        };
        ResolvedPolicy::from_config(&cfg, std::path::Path::new("/"))
    }

    #[test]
    fn off_policy_returns_none() {
        let p = make_policy("off", 8192, vec![]);
        assert!(capture_tool_input(&p, "shell", "hello").is_none());
    }

    #[test]
    fn denylist_skips_capture() {
        let p = make_policy("redacted", 8192, vec!["memory_recall".into()]);
        assert!(capture_tool_input(&p, "memory_recall", "hello").is_none());
        assert!(capture_tool_input(&p, "shell", "hello").is_some());
    }

    #[test]
    fn redacted_truncates_when_over_cap() {
        let p = make_policy("redacted", 4, vec![]);
        let cap = capture_tool_input(&p, "shell", "hello world").unwrap();
        assert_eq!(cap.text, "hell");
        assert_eq!(cap.original_bytes, 11);
        assert!(cap.truncated);
    }

    #[test]
    fn full_policy_keeps_everything() {
        let p = make_policy("full", 4, vec![]);
        let cap = capture_tool_output(&p, "shell", "hello world").unwrap();
        assert_eq!(cap.text, "hello world");
        assert!(!cap.truncated);
    }

    #[test]
    fn llm_request_off_is_default_and_returns_none() {
        // Default config resolves to Off => no capture.
        let default = ResolvedPolicy::from_config(&LogConfig::default(), std::path::Path::new("/"));
        assert_eq!(default.llm_request_payload, LlmRequestPayloadPolicy::Off);
        assert!(capture_llm_request(LlmRequestPayloadPolicy::Off, 8192, "system prompt").is_none());
    }

    #[test]
    fn llm_request_redacted_truncates_at_cap() {
        let cap = capture_llm_request(LlmRequestPayloadPolicy::Redacted, 4, "hello world").unwrap();
        assert_eq!(cap.text, "hell");
        assert_eq!(cap.original_bytes, 11);
        assert!(cap.truncated);
    }

    #[test]
    fn llm_request_redacted_truncation_respects_utf8_char_boundaries() {
        let cap = capture_llm_request(LlmRequestPayloadPolicy::Redacted, 3, "éé").unwrap();
        assert_eq!(cap.text, "é");
        assert_eq!(cap.original_bytes, 4);
        assert!(cap.truncated);
        assert!(cap.text.len() <= 3);
    }

    #[test]
    fn llm_request_full_keeps_everything() {
        let cap = capture_llm_request(LlmRequestPayloadPolicy::Full, 4, "hello world").unwrap();
        assert_eq!(cap.text, "hello world");
        assert!(!cap.truncated);
    }

    #[test]
    fn redacted_keeps_input_at_or_under_cap() {
        let p = make_policy("redacted", 8, vec![]);
        // Under cap.
        let c = capture_tool_input(&p, "shell", "hello").unwrap();
        assert_eq!(c.text, "hello");
        assert_eq!(c.original_bytes, 5);
        assert!(!c.truncated);
        // Exactly at cap is kept (the check is ).
        let c = capture_tool_input(&p, "shell", "12345678").unwrap();
        assert_eq!(c.text, "12345678");
        assert!(!c.truncated);
    }

    #[test]
    fn redacted_truncation_respects_utf8_char_boundaries() {
        // 'é' is 2 bytes (U+00E9). With cap=3, "éé" (4 bytes) must truncate to
        // the first whole char rather than splitting one mid-byte.
        let p = make_policy("redacted", 3, vec![]);
        let c = capture_tool_input(&p, "shell", "éé").unwrap();
        assert_eq!(c.text, "é");
        assert!(c.truncated);
        assert_eq!(c.original_bytes, 4);
        // Kept text stays within the byte cap and remains valid UTF-8.
        assert!(c.text.len() <= 3);
    }

    #[test]
    fn capture_tool_output_uses_the_same_policy_path() {
        let p = make_policy("off", 8192, vec![]);
        assert!(capture_tool_output(&p, "shell", "x").is_none());
    }

    #[test]
    fn conversational_tool_io_defaults_to_off_while_technical_is_captured() {
        let default_policy = ResolvedPolicy::from_config(&LogConfig::default(), std::path::Path::new("/"));
        // Technical tool like 'shell' or 'file_read' is Redacted (captured).
        assert!(capture_tool_input(&default_policy, "shell", "echo hello").is_some());
        // Conversational tool like 'send_message_to_peer' defaults to Off (omitted).
        assert!(capture_tool_input(&default_policy, "send_message_to_peer", "private msg").is_none());
        assert!(capture_tool_output(&default_policy, "send_message_to_peer", "peer reply").is_none());
    }

    #[test]
    fn conversational_tool_io_can_be_explicitly_configured() {
        let cfg = LogConfig {
            log_conversational_tool_io: "redacted".into(),
            ..LogConfig::default()
        };
        let policy = ResolvedPolicy::from_config(&cfg, std::path::Path::new("/"));
        assert!(capture_tool_input(&policy, "send_message_to_peer", "hello").is_some());
    }
}
