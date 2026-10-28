//! Integrations REST API handlers.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Json},
};

use super::require_auth;
use crate::AppState;

pub(crate) fn integration_entry_json(
    entry: &galleon_config::integrations::IntegrationEntry,
) -> serde_json::Value {
    serde_json::json!({
        "name": &entry.name,
        "description": &entry.description,
        "category": entry.category,
        "category_label": entry.category.label(),
        "status": entry.status,
        // Canonical config map key (provider family key / ChannelsConfig map
        // key) for deep links; null when the entry has no config section.
        "key": &entry.key,
    })
}

/// GET /api/integrations — list all integrations with status
pub async fn handle_api_integrations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let entries = state
        .integration_catalog
        .as_ref()
        .map(|c| c.all_integrations(&config))
        .unwrap_or_default();

    let integrations: Vec<serde_json::Value> = entries.iter().map(integration_entry_json).collect();

    Json(serde_json::json!({"integrations": integrations})).into_response()
}

/// GET /api/integrations/settings — return per-integration settings (enabled + category)
pub async fn handle_api_integrations_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let entries = state
        .integration_catalog
        .as_ref()
        .map(|c| c.all_integrations(&config))
        .unwrap_or_default();

    let mut settings = serde_json::Map::new();
    for entry in &entries {
        let enabled = matches!(
            entry.status,
            galleon_config::integrations::IntegrationStatus::Active
        );
        settings.insert(
            entry.name.clone(),
            serde_json::json!({
                "enabled": enabled,
                "category": entry.category,
                "status": entry.status,
            }),
        );
    }

    Json(serde_json::json!({"settings": settings})).into_response()
}
