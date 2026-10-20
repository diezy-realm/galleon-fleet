//! Credential redaction for the rendering layer (logs, observer events, and
//! UI-facing turn events). The success data path is untouched: a successful
//! tool result fed back to the model and signed by an HMAC receipt carries
//! raw bytes. The one data-path exception is the failure arms of
//! `execute_one_tool`, which fold a tool's detailed error body into the
//! model-visible text and scrub that combined string first, so a reflected
//! token in a 4xx/5xx body is not forwarded to the model provider.

pub use galleon_security_scrub::{
    is_credential_key, scrub_credentials, scrub_credentials_value, SENSITIVE_KEY_REGEX,
    SENSITIVE_KV_REGEX,
};


#[cfg(test)]
mod tests {
    use super::{is_credential_key, scrub_credentials, scrub_credentials_value};

    #[test]
    fn credential_key_classification_combines_shared_secret_policies() {
        for key in ["private_key", "api_key", "cookie", "set-cookie", "user-key"] {
            assert!(is_credential_key(key), "expected credential key: {key}");
        }
        assert!(!is_credential_key("status"));
    }

    /// The shape the RPC run-detail response leans on: tool arguments are
    /// arbitrary JSON, so a secret under a credential-named key is routinely
    /// carried by a descendant that names nothing sensitive. A key-based walk
    /// that recursed would leave every one of these intact. Sibling coverage in
    /// `scrub_credentials_value_replaces_every_credential_value_shape` takes the
    /// nested-object, numeric, and boolean shapes; this one adds the array and
    /// `null` cases and asserts no seeded secret reaches the wire.
    #[test]
    fn a_credential_key_redacts_its_value_whatever_the_shape() {
        let value = serde_json::json!({
            "api_key": {"value": "NESTEDSECRET"},
            "token": ["ARRAYSECRET"],
            "password": 12345678,
            "secret": true,
            "credential": null,
        });

        let scrubbed = scrub_credentials_value(value);
        let wire = serde_json::to_string(&scrubbed).expect("serializable");

        for secret in ["NESTEDSECRET", "ARRAYSECRET", "12345678"] {
            assert!(
                !wire.contains(secret),
                "{secret} must not survive scrubbing"
            );
        }
        for key in ["api_key", "token", "password", "secret"] {
            assert_eq!(scrubbed[key], "[REDACTED]", "{key} must be replaced whole");
        }
        // Even `null` is replaced rather than preserved: the marker is what
        // tells a reader the key was withheld, and a surviving `null` would
        // read as a value the run genuinely had.
        assert_eq!(scrubbed["credential"], "[REDACTED]");
    }

    #[test]
    fn scrub_credentials_value_redacts_nested_secret_and_keeps_key() {
        let input = serde_json::json!({
            "body": {"access_token": "sk-live-abcdef0123456789", "status": "ok"},
            "count": 3
        });
        let out = scrub_credentials_value(input);
        let token = out["body"]["access_token"].as_str().unwrap();
        assert!(token.contains("[REDACTED]"));
        assert!(!token.contains("abcdef0123456789"));
        assert_eq!(out["body"]["status"], "ok");
        assert_eq!(out["count"], 3);
    }

    #[test]
    fn scrub_credentials_value_redacts_authorization_and_cookie_keys() {
        let input = serde_json::json!({
            "body": {
                "authorization": "Bearer sk-live-abcdef0123456789",
                "cookie": "session=deadbeefcafebabe0123",
                "set-cookie": "sid=9f8e7d6c5b4a3210feed",
                "status": "ok"
            }
        });
        let out = scrub_credentials_value(input);
        let authorization = out["body"]["authorization"].as_str().unwrap();
        assert!(authorization.contains("[REDACTED]"));
        assert!(!authorization.contains("sk-live-abcdef0123456789"));
        let cookie = out["body"]["cookie"].as_str().unwrap();
        assert!(cookie.contains("[REDACTED]"));
        assert!(!cookie.contains("deadbeefcafebabe0123"));
        let set_cookie = out["body"]["set-cookie"].as_str().unwrap();
        assert!(set_cookie.contains("[REDACTED]"));
        assert!(!set_cookie.contains("9f8e7d6c5b4a3210feed"));
        assert_eq!(out["body"]["status"], "ok");
    }

    #[test]
    fn scrub_credentials_value_matches_summary_secret_key_policy() {
        let input = serde_json::json!({"body": {"private_key": "tiny"}});
        let out = scrub_credentials_value(input);

        assert_eq!(out["body"]["private_key"], "[REDACTED]");
        assert_ne!(out["body"]["private_key"], "tiny");
    }

    #[test]
    fn scrub_credentials_value_replaces_every_credential_value_shape() {
        let input = serde_json::json!({
            "body": {
                "numeric_token": 1234,
                "credentials": {"value": "tiny"},
                "auth_enabled": true,
                "status": "ok"
            }
        });
        let out = scrub_credentials_value(input);

        for key in ["numeric_token", "credentials", "auth_enabled"] {
            assert_eq!(out["body"][key], "[REDACTED]");
        }
        assert_eq!(out["body"]["status"], "ok");
    }

    #[test]
    fn scrub_credentials_redacts_unquoted_base64_credential_values() {
        let input = "token=QWxh+GRpbjpvcGVu/IHNlc2FtZQ== next=public";
        let scrubbed = scrub_credentials(input);

        assert_eq!(scrubbed, "token=QWxh*[REDACTED] next=public");
        assert!(!scrubbed.contains("IHNlc2FtZQ"));
        assert!(!scrubbed.contains("=="));
    }

    #[test]
    fn scrub_credentials_redacts_quoted_base64_credential_values() {
        let input = r#"secret="QWxhZGRpbjpvcGVu/IHNlc2FtZQ==""#;
        let scrubbed = scrub_credentials(input);

        assert_eq!(scrubbed, r#"secret="QWxh*[REDACTED]""#);
        assert!(!scrubbed.contains("IHNlc2FtZQ"));
        assert!(!scrubbed.contains("=="));
    }
}
