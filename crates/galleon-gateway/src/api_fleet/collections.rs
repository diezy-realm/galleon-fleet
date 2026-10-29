//! HTTP handlers for Galleon Fleet collections (`GET/PUT/POST /api/collections/{name}`).
//!
//! Under SPEC-013 §Koleksi:
//! - Collection names are strictly validated against the 15 canonical camelCase
//!   collections allowlist plus policies and riskTiers. Any unknown name returns 404.
//! - Path traversal attacks are structurally impossible (paths are resolved to static literals).
//! - Body must be a valid JSON array. Non-arrays return 400.
//! - Max payload limit is FLEET_MAX_BODY = 1 MiB (1,048,576 bytes). Exceeding returns 413.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use galleon_fleet_core::{FleetError, FleetStore, FLEET_MAX_BODY};
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Assembles collection routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/collections/{name}",
            get(handle_get_collection_state)
                .put(handle_save_collection_state)
                .post(handle_save_collection_state),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Assembles standalone collection router for a dedicated FleetStore.
pub fn router_for_store(store: Arc<FleetStore>) -> Router {
    Router::new()
        .route(
            "/api/collections/{name}",
            get(handle_get_collection_store)
                .put(handle_save_collection_store)
                .post(handle_save_collection_store),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(store)
}

async fn handle_get_collection_state(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let store = {
        let config = state.config.read();
        FleetStore::from_data_dir(&config.data_dir)
    };
    get_collection_impl(&store, &name)
}

async fn handle_save_collection_state(
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: Bytes,
) -> Response {
    let store = {
        let config = state.config.read();
        FleetStore::from_data_dir(&config.data_dir)
    };
    save_collection_impl(&store, &name, body).await
}

async fn handle_get_collection_store(
    State(store): State<Arc<FleetStore>>,
    Path(name): Path<String>,
) -> Response {
    get_collection_impl(&store, &name)
}

async fn handle_save_collection_store(
    State(store): State<Arc<FleetStore>>,
    Path(name): Path<String>,
    body: Bytes,
) -> Response {
    save_collection_impl(&store, &name, body).await
}

fn get_collection_impl(store: &FleetStore, name: &str) -> Response {
    match store.get_collection(name) {
        Ok(Some(bytes)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            bytes,
        )
            .into_response(),
        Ok(None) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            b"[]".to_vec(),
        )
            .into_response(),
        Err(FleetError::UnknownCollection(_))
        | Err(FleetError::InvalidCollection(_))
        | Err(FleetError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": "unknown collection" }).to_string(),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": e.to_string() }).to_string(),
        )
            .into_response(),
    }
}

async fn save_collection_impl(store: &FleetStore, name: &str, body: Bytes) -> Response {
    if body.len() > FLEET_MAX_BODY {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": "collection exceeds 1 MiB" }).to_string(),
        )
            .into_response();
    }

    match store.save_collection(name, &body).await {
        Ok(()) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "success": true }).to_string(),
        )
            .into_response(),
        Err(FleetError::UnknownCollection(_)) | Err(FleetError::InvalidCollection(_)) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": "unknown collection" }).to_string(),
        )
            .into_response(),
        Err(FleetError::PayloadTooLarge { .. }) => (
            StatusCode::PAYLOAD_TOO_LARGE,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": "collection exceeds 1 MiB" }).to_string(),
        )
            .into_response(),
        Err(FleetError::NotAJsonArray) => (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": "collection body must be a JSON array" }).to_string(),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": e.to_string() }).to_string(),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn response_to_json(resp: Response) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
        let status = resp.status();
        let body_bytes = resp.into_body().collect().await?.to_bytes();
        let json_val = serde_json::from_slice(&body_bytes)?;
        Ok((status, json_val))
    }

    #[tokio::test]
    async fn test_get_collection_empty_returns_array() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        let app = router_for_store(store);

        let req = Request::builder()
            .method("GET")
            .uri("/api/collections/squads")
            .body(Body::empty())?;

        let resp = app.oneshot(req).await?;
        let (status, val) = response_to_json(resp).await?;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(val, json!([]));
        Ok(())
    }

    #[tokio::test]
    async fn test_put_and_get_collection_success() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        let app = router_for_store(store);

        let payload = json!([{"id": "squad-1", "name": "Core Dev"}]);
        let req = Request::builder()
            .method("PUT")
            .uri("/api/collections/squads")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(payload.to_string()))?;

        let resp = app.clone().oneshot(req).await?;
        let (status, val) = response_to_json(resp).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(val, json!({"success": true}));

        // Now GET the collection
        let get_req = Request::builder()
            .method("GET")
            .uri("/api/collections/squads")
            .body(Body::empty())?;

        let get_resp = app.oneshot(get_req).await?;
        let (get_status, get_val) = response_to_json(get_resp).await?;
        assert_eq!(get_status, StatusCode::OK);
        assert_eq!(get_val, payload);
        Ok(())
    }

    #[tokio::test]
    async fn test_post_collection_compatibility() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        let app = router_for_store(store);

        let payload = json!([{"id": "ship-1"}]);
        let req = Request::builder()
            .method("POST")
            .uri("/api/collections/ships")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(payload.to_string()))?;

        let resp = app.oneshot(req).await?;
        let (status, val) = response_to_json(resp).await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(val, json!({"success": true}));
        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_collection_returns_404() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        let app = router_for_store(store);

        // GET unknown
        let get_req = Request::builder()
            .method("GET")
            .uri("/api/collections/notes")
            .body(Body::empty())?;

        let get_resp = app.clone().oneshot(get_req).await?;
        let (get_status, get_val) = response_to_json(get_resp).await?;
        assert_eq!(get_status, StatusCode::NOT_FOUND);
        assert_eq!(get_val, json!({"error": "unknown collection"}));

        // PUT unknown
        let put_req = Request::builder()
            .method("PUT")
            .uri("/api/collections/notes")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("[]"))?;

        let put_resp = app.oneshot(put_req).await?;
        let (put_status, put_val) = response_to_json(put_resp).await?;
        assert_eq!(put_status, StatusCode::NOT_FOUND);
        assert_eq!(put_val, json!({"error": "unknown collection"}));
        Ok(())
    }

    #[tokio::test]
    async fn test_path_traversal_returns_404() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        let app = router_for_store(store);

        let attacks = ["..%2F..%2Fevil", "evil%2F..%2Fsquads"];
        for attack in attacks {
            let req = Request::builder()
                .method("GET")
                .uri(format!("/api/collections/{attack}"))
                .body(Body::empty())?;

            let resp = app.clone().oneshot(req).await?;
            let (status, val) = response_to_json(resp).await?;
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert_eq!(val, json!({"error": "unknown collection"}));
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_non_array_body_returns_400() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        let app = router_for_store(store);

        let non_arrays = [
            "{\"foo\": \"bar\"}",
            "\"a string\"",
            "123",
            "true",
            "null",
            "not json at all",
        ];

        for body in non_arrays {
            let req = Request::builder()
                .method("PUT")
                .uri("/api/collections/squads")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))?;

            let resp = app.clone().oneshot(req).await?;
            let (status, val) = response_to_json(resp).await?;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(val, json!({"error": "collection body must be a JSON array"}));
        }
        Ok(())
    }
}
