//! Credential scrubbing and leak detection utilities (PR-32 / R-11 / BR-C2).
//!
//! Extracted from `clawcrew-runtime::security` so that channel crates and
//! gateway services can perform credential scrubbing without linking to the
//! runtime monolith.

pub mod leak_detector;

pub use leak_detector::{LeakDetector, LeakResult};
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
