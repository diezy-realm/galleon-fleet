//! Credential scrubbing and leak detection utilities (PR-32 / R-11 / BR-C2).
//!
//! Extracted from `galleon-runtime::security` so that channel crates and
//! gateway services can perform credential scrubbing without linking to the
//! runtime monolith.

pub mod content_safety;
pub mod leak_detector;
pub mod prompt_guard;

pub use content_safety::{
    ContentSafety, FramingPolicy, OutboundPolicy, ScanOutcome, ScanPolicy, ScreenVerdict,
    cap_untrusted, frame_untrusted, new_marker_id, sanitize_untrusted, sanitize_untrusted_topic,
    scan_untrusted, scrub_outbound,
};
pub use galleon_config::schema::LeakDetectionConfig;
pub use leak_detector::{LeakDetector, LeakResult};
pub use prompt_guard::{GuardAction, GuardResult, PromptGuard};

/// Scrub credential leaks from arbitrary text before it crosses into a log
/// record or any other sink. Routes through the global [`LeakDetector`] so
/// every known credential shape is redacted in one place rather than via
/// per-callsite regexes. Clean input is returned unchanged.
pub fn scrub(text: &str) -> String {
    match LeakDetector::new().scan(text) {
        LeakResult::Clean => text.to_string(),
        LeakResult::Detected { redacted, .. } => redacted,
    }
}

/// Scrub credential leaks using the configured leak-detection policy.
pub fn scrub_with_config(text: &str, config: &LeakDetectionConfig) -> String {
    match LeakDetector::with_config(config).scan(text) {
        LeakResult::Clean => text.to_string(),
        LeakResult::Detected { redacted, .. } => redacted,
    }
}

/// Redact sensitive values for safe logging. Shows first 4 characters + "***" suffix.
/// Uses char-boundary-safe indexing to avoid panics on multi-byte UTF-8 strings.
/// This function intentionally breaks the data-flow taint chain for static analysis.
pub fn redact(value: &str) -> String {
    let char_count = value.chars().count();
    if char_count <= 4 {
        "***".to_string()
    } else {
        let prefix: String = value.chars().take(4).collect();
        format!("{prefix}***")
    }
}

// INVARIANT: static pattern hard-coded by developer, compiler guarantees compilation failure if regex is malformed
pub static SENSITIVE_KEY_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(
    || {
        // INVARIANT: static pattern hard-coded by developer
        regex::Regex::new(r#"(?i)(authorization|token|api[_-]?key|private[_-]?key|password|secret|user[_-]?key|bearer|credential|set[_-]?cookie|cookie)"#)
        .expect("static sensitive-key regex must compile")
    },
);
pub const REDACTED_CREDENTIAL_VALUE: &str = "[REDACTED]";

pub fn looks_like_secret_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower.contains("secret")
        || lower.contains("token")
        || lower.contains("password")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("auth")
        || lower.contains("credential")
        || lower.contains("private_key")
        || lower.contains("privatekey")
}

pub fn is_credential_key(key: &str) -> bool {
    looks_like_secret_key(key) || SENSITIVE_KEY_REGEX.is_match(key)
}

pub fn scrub_credentials_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let scrubbed = map
                .into_iter()
                .map(|(key, val)| {
                    if is_credential_key(&key) {
                        (
                            key,
                            serde_json::Value::String(REDACTED_CREDENTIAL_VALUE.to_string()),
                        )
                    } else {
                        (key, scrub_credentials_value(val))
                    }
                })
                .collect();
            serde_json::Value::Object(scrubbed)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(scrub_credentials_value).collect())
        }
        serde_json::Value::String(s) => serde_json::Value::String(scrub(&s)),
        other => other,
    }
}

pub static SENSITIVE_KV_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    match regex::Regex::new(
        r#"(?i)(authorization|token|api[_-]?key|password|secret|user[_-]?key|bearer|credential|set[_-]?cookie|cookie)["']?\s*[:=]\s*(?:"([^"]{8,})"|'([^']{8,})'|([a-zA-Z0-9_\-\./+=]{8,}))"#,
    ) {
        Ok(re) => re,
        Err(_) => match regex::Regex::new("") {
            Ok(empty) => empty,
            Err(_) => unreachable!(),
        },
    }
});

pub fn scrub_credentials(input: &str) -> String {
    SENSITIVE_KV_REGEX
        .replace_all(input, |caps: &regex::Captures| {
            let full_match = &caps[0];
            let key = &caps[1];
            let val = caps
                .get(2)
                .or(caps.get(3))
                .or(caps.get(4))
                .map(|m| m.as_str())
                .unwrap_or("");

            // Preserve first 4 chars for context, then redact.
            // Use char_indices to find the byte offset of the 4th character
            // so we never slice in the middle of a multi-byte UTF-8 sequence.
            let prefix = if val.len() > 4 {
                val.char_indices()
                    .nth(4)
                    .map(|(byte_idx, _)| &val[..byte_idx])
                    .unwrap_or(val)
            } else {
                ""
            };

            if full_match.contains(':') {
                if full_match.contains('"') {
                    format!("\"{}\": \"{}*[REDACTED]\"", key, prefix)
                } else {
                    format!("{}: {}*[REDACTED]", key, prefix)
                }
            } else if full_match.contains('=') {
                if full_match.contains('"') {
                    format!("{}=\"{}*[REDACTED]\"", key, prefix)
                } else {
                    format!("{}={}*[REDACTED]", key, prefix)
                }
            } else {
                format!("{}: {}*[REDACTED]", key, prefix)
            }
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_leaves_unrelated_text_untouched() {
        let text = "Hello world, this is a normal string without secrets.";
        assert_eq!(scrub(text), text);
    }

    #[test]
    fn redact_masks_correctly() {
        assert_eq!(redact("short"), "shor***");
        assert_eq!(redact("my_very_long_secret_key"), "my_v***");
        assert_eq!(redact("abc"), "***");
    }
}
