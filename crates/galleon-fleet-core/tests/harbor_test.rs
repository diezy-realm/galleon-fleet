//! Contract and unit tests for Harbor provider domain model.
//!
//! Under SPEC-013 §Model Data (TASK-076):
//! - Parity with Go fixture shape `GET_api_providers_harbor.json`.
//! - Strict compliance with TS `HarborProvider` contract in `web/src/utils/apiClient.ts`.
//! - No secrets leaked, honest status and null latencyMs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleon_fleet_core::{
    HarborProvider, STATUS_CONFIGURED, STATUS_MISSING_CREDENTIALS, build_harbor_provider,
};
use serde_json::Value;

#[test]
fn test_harbor_provider_matches_go_fixture_shape() {
    let fixture_str =
        include_str!("../../../tests/fixtures/go-fleet/GET_api_providers_harbor.json");
    let fixture_val: Value = serde_json::from_str(fixture_str).expect("parse go fixture");
    let fixture_len = fixture_val.as_array().expect("fixture array").len();

    // Deserialize go fixture into HarborProvider structs
    let deserialized: Result<Vec<HarborProvider>, _> = serde_json::from_value(fixture_val);
    assert!(
        deserialized.is_ok(),
        "fixture JSON must deserialize into HarborProvider: {:?}",
        deserialized.err()
    );

    let providers = deserialized.unwrap();
    assert_eq!(providers.len(), fixture_len);

    // Verify fields on first provider from fixture
    let first = &providers[0];
    assert_eq!(first.id, "prov-ollama");
    assert_eq!(first.name, "Ollama Local Gateway");
    assert_eq!(first.vendor, "Ollama Local Daemon");
    assert_eq!(first.latency_ms, Some(18)); // In Go fixture it was 18
}

#[test]
fn test_built_harbor_provider_honest_latency_null() {
    let prov = build_harbor_provider(
        "openrouter",
        "default",
        Some("claude-3-5-sonnet"),
        &["o3-mini".into(), "mistral-large".into()],
        true,
        false,
    );

    let json_val = serde_json::to_value(&prov).expect("serialize provider");
    assert_eq!(json_val["id"], "prov-openrouter");
    assert_eq!(json_val["name"], "OpenRouter Mesh Broker");
    assert_eq!(json_val["vendor"], "OpenRouter");
    assert_eq!(json_val["status"], STATUS_CONFIGURED);
    assert_eq!(json_val["latencyMs"], Value::Null);

    let models = json_val["models"].as_array().expect("models array");
    assert_eq!(models.len(), 3);
    assert_eq!(models[0], "claude-3-5-sonnet");
    assert_eq!(models[1], "o3-mini");
    assert_eq!(models[2], "mistral-large");

    let caps = json_val["capabilities"].as_array().expect("caps array");
    assert!(caps.iter().any(|c| c == "BYOK Routing"));
    assert!(caps.iter().any(|c| c == "Redundancy Fallback"));
}

#[test]
fn test_built_harbor_provider_missing_credentials_status() {
    let prov = build_harbor_provider(
        "gemini",
        "default",
        Some("gemini-2.5-pro"),
        &[],
        false, // No API key
        false,
    );

    let json_val = serde_json::to_value(&prov).expect("serialize provider");
    assert_eq!(json_val["status"], STATUS_MISSING_CREDENTIALS);
    assert_eq!(json_val["latencyMs"], Value::Null);
}

#[test]
fn test_harbor_provider_never_leaks_secrets() {
    let secret = "sk-ant-api03-super-secret-production-token-1234567890";
    let prov = build_harbor_provider(
        "anthropic",
        "default",
        Some("claude-3-5-sonnet"),
        &[],
        true, // Has credentials
        false,
    );

    let serialized = serde_json::to_string(&prov).expect("serialize");
    assert!(!serialized.contains(secret));
    assert!(!serialized.contains("apiKey"));
    assert!(!serialized.contains("api_key"));
    assert!(!serialized.contains("secret"));
    assert!(!serialized.contains("token"));
}

#[test]
fn test_harbor_provider_custom_alias() {
    let prov = build_harbor_provider(
        "deepseek",
        "coding",
        Some("deepseek-coder"),
        &[],
        true,
        false,
    );

    assert_eq!(prov.id, "prov-deepseek-coding");
    assert_eq!(prov.name, "DeepSeek API (coding)");
    assert_eq!(prov.vendor, "DeepSeek");
    assert_eq!(prov.status, STATUS_CONFIGURED);
    assert_eq!(prov.latency_ms, None);
}
