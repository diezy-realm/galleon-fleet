//! Credential scrubbing and leak detection utilities (PR-32 / R-11 / BR-C2).
//!
//! Extracted from `clawcrew-runtime::security` so that channel crates and
//! gateway services can perform credential scrubbing without linking to the
//! runtime monolith.

pub mod content_safety;
pub mod leak_detector;
pub mod prompt_guard;

pub use content_safety::{
    cap_untrusted, frame_untrusted, new_marker_id, sanitize_untrusted,
    sanitize_untrusted_topic, scan_untrusted, scrub_outbound, ContentSafety, FramingPolicy,
    OutboundPolicy, ScanOutcome, ScanPolicy, ScreenVerdict,
};
pub use leak_detector::{LeakDetector, LeakResult};
pub use prompt_guard::{GuardAction, GuardResult, PromptGuard};
pub use clawcrew_config::schema::LeakDetectionConfig;

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

pub static SENSITIVE_KEY_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r#"(?i)(authorization|token|api[_-]?key|private[_-]?key|password|secret|user[_-]?key|bearer|credential|set[_-]?cookie|cookie)"#)
        .expect("static sensitive-key regex must compile")
});
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
