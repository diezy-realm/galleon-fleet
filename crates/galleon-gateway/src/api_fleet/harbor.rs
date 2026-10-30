//! HTTP handlers for Galleon Fleet Harbor provider endpoints (`/api/providers/harbor`).
//!
//! Under SPEC-013 §Model Data (TASK-076):
//! - `GET /api/providers/harbor`:
//!   - Dynamic listing derived from configured `config.providers.models` (no hardcoded list).
//!   - Status strictly ∈ `{"configured", "missing_credentials"}`.
//!   - `latencyMs` is `null` (honest metrics, no fake ping).
//!   - Zero credential leakage: API keys and secrets are never serialized.

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use galleon_config::schema::Config;
use galleon_fleet_core::{FLEET_MAX_BODY, HarborProvider, build_harbor_provider};
use parking_lot::RwLock;
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Assembles harbor routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/providers/harbor",
            get(handle_get_harbor_providers_state),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing with dedicated configuration.
pub fn router_for_config(config: Arc<RwLock<Config>>) -> Router {
    Router::new()
        .route(
            "/api/providers/harbor",
            get(move || {
                let cfg = Arc::clone(&config);
                async move {
                    let config_guard = cfg.read();
                    handle_get_harbor_providers_impl(&config_guard)
                }
            }),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
}

/// Standalone router with default configuration.
pub fn router() -> Router {
    router_for_config(Arc::new(RwLock::new(Config::default())))
}

async fn handle_get_harbor_providers_state(State(state): State<AppState>) -> Response {
    let config = state.config.read();
    handle_get_harbor_providers_impl(&config)
}

/// Pure implementation that inspects the configured providers and builds harbor items.
pub fn handle_get_harbor_providers_impl(config: &Config) -> Response {
    let mut providers: Vec<HarborProvider> = Vec::new();

    for (family, alias, base_cfg) in config.providers.models.iter_entries() {
        let has_credentials = base_cfg
            .api_key
            .as_deref()
            .map(str::trim)
            .is_some_and(|k| !k.is_empty())
            || base_cfg.requires_openai_auth;

        let is_local_endpoint = base_cfg.uri.as_deref().map(str::trim).is_none_or(|u| {
            u.contains("localhost") || u.contains("127.0.0.1") || !u.starts_with("https://")
        });

        let provider = build_harbor_provider(
            family,
            alias,
            base_cfg.model.as_deref(),
            &base_cfg.fallback_models,
            has_credentials,
            is_local_endpoint,
        );

        providers.push(provider);
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json!(providers).to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use galleon_config::schema::{
        AnthropicModelProviderConfig, ModelProviderConfig, OllamaModelProviderConfig,
        OpenAIModelProviderConfig,
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_get_harbor_providers_empty_config() {
        let app = router();
        let req = Request::builder()
            .uri("/api/providers/harbor")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, json!([]));
    }

    #[tokio::test]
    async fn test_get_harbor_providers_status_and_null_latency() {
        let mut config = Config::default();

        // 1. OpenAI with API key -> configured
        config.providers.models.openai.insert(
            "default".to_string(),
            OpenAIModelProviderConfig {
                base: ModelProviderConfig {
                    api_key: Some("sk-secret-key-12345".to_string()),
                    model: Some("gpt-4o".to_string()),
                    ..Default::default()
                },
            },
        );

        // 2. Anthropic without API key -> missing_credentials
        config.providers.models.anthropic.insert(
            "default".to_string(),
            AnthropicModelProviderConfig {
                base: ModelProviderConfig {
                    api_key: None,
                    model: Some("claude-3-7-sonnet".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        // 3. Ollama local without API key -> configured
        config.providers.models.ollama.insert(
            "default".to_string(),
            OllamaModelProviderConfig {
                base: ModelProviderConfig {
                    api_key: None,
                    uri: Some("http://127.0.0.1:11434".to_string()),
                    model: Some("llama3.2:3b".to_string()),
                    fallback_models: vec!["qwen2.5-coder:7b".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        let app = router_for_config(Arc::new(RwLock::new(config)));
        let req = Request::builder()
            .uri("/api/providers/harbor")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Security check: Zero secrets leaked
        assert!(!body_str.contains("sk-secret-key-12345"));
        assert!(!body_str.contains("apiKey"));
        assert!(!body_str.contains("api_key"));

        let json: Value = serde_json::from_str(&body_str).unwrap();
        let array = json.as_array().unwrap();
        assert_eq!(array.len(), 3);

        // Check honest latencyMs is null for all
        for item in array {
            assert!(item["latencyMs"].is_null());
            let status = item["status"].as_str().unwrap();
            assert!(status == "configured" || status == "missing_credentials");
        }

        let openai_item = array.iter().find(|i| i["id"] == "prov-openai").unwrap();
        assert_eq!(openai_item["status"], "configured");
        assert_eq!(openai_item["name"], "OpenAI API");

        let anthropic_item = array.iter().find(|i| i["id"] == "prov-anthropic").unwrap();
        assert_eq!(anthropic_item["status"], "missing_credentials");
        assert_eq!(anthropic_item["name"], "Anthropic Claude API");

        let ollama_item = array.iter().find(|i| i["id"] == "prov-ollama").unwrap();
        assert_eq!(ollama_item["status"], "configured");
        assert_eq!(ollama_item["name"], "Ollama Local Gateway");
    }
}
