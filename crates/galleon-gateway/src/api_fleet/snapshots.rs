//! HTTP handlers for Galleon Fleet snapshots (`GET/POST /api/snapshots`).
//!
//! Under SPEC-013 §Snapshot:
//! - `POST /api/snapshots {label}` copies collection files to `root/snapshots/<id>/`
//!   and calculates real sizeMB, entityCount, and SHA-256 checksum.
//! - `GET /api/snapshots` returns list of snapshots sorted newest first.
//! - Snapshot capacity limit is MAX_SNAPSHOTS = 20 (oldest pruned automatically).

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use galleon_fleet_core::{CreateSnapshotRequest, FLEET_MAX_BODY, FleetStore};
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Assembles snapshot routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/snapshots",
            get(handle_get_snapshots_state).post(handle_create_snapshot_state),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Assembles standalone snapshot router for a dedicated FleetStore.
pub fn router_for_store(store: Arc<FleetStore>) -> Router {
    Router::new()
        .route(
            "/api/snapshots",
            get(handle_get_snapshots_store).post(handle_create_snapshot_store),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(store)
}

async fn handle_get_snapshots_state(State(state): State<AppState>) -> Response {
    let store = {
        let config = state.config.read();
        FleetStore::from_data_dir(&config.data_dir)
    };
    get_snapshots_impl(&store)
}

async fn handle_create_snapshot_state(State(state): State<AppState>, body: Bytes) -> Response {
    let store = {
        let config = state.config.read();
        FleetStore::from_data_dir(&config.data_dir)
    };
    create_snapshot_impl(&store, body).await
}

async fn handle_get_snapshots_store(State(store): State<Arc<FleetStore>>) -> Response {
    get_snapshots_impl(&store)
}

async fn handle_create_snapshot_store(
    State(store): State<Arc<FleetStore>>,
    body: Bytes,
) -> Response {
    create_snapshot_impl(&store, body).await
}

fn get_snapshots_impl(store: &FleetStore) -> Response {
    match store.list_snapshots() {
        Ok(snaps) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json!(snaps).to_string(),
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

async fn create_snapshot_impl(store: &FleetStore, body: Bytes) -> Response {
    if body.len() > FLEET_MAX_BODY {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            [(header::CONTENT_TYPE, "application/json")],
            json!({ "error": "payload exceeds limit" }).to_string(),
        )
            .into_response();
    }

    let label = if body.is_empty() {
        "Manual Snapshot".to_string()
    } else {
        match serde_json::from_slice::<CreateSnapshotRequest>(&body) {
            Ok(req) => req
                .label
                .or(req.title)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Manual Snapshot".to_string()),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    json!({ "error": "invalid snapshot request body" }).to_string(),
                )
                    .into_response();
            }
        }
    };

    match store.create_snapshot(&label).await {
        Ok(snap) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "application/json")],
            json!(snap).to_string(),
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

    async fn response_to_json(
        resp: Response,
    ) -> Result<(StatusCode, serde_json::Value), Box<dyn std::error::Error>> {
        let status = resp.status();
        let body_bytes = resp.into_body().collect().await?.to_bytes();
        let json_val = serde_json::from_slice(&body_bytes)?;
        Ok((status, json_val))
    }

    #[tokio::test]
    async fn test_get_snapshots_empty() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        let app = router_for_store(store);

        let req = Request::builder()
            .method("GET")
            .uri("/api/snapshots")
            .body(Body::empty())?;

        let resp = app.oneshot(req).await?;
        let (status, val) = response_to_json(resp).await?;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(val, json!([]));
        Ok(())
    }

    #[tokio::test]
    async fn test_create_and_get_snapshot_http() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let store = Arc::new(FleetStore::new(tmp.path()));
        store
            .save_collection("ships", b"[{\"id\":\"v-1\",\"name\":\"Galleon Prime\"}]")
            .await?;

        let app = router_for_store(store.clone());

        let req = Request::builder()
            .method("POST")
            .uri("/api/snapshots")
            .header("Content-Type", "application/json")
            .body(Body::from(
                json!({ "label": "Genesis Commissioning State" }).to_string(),
            ))?;

        let resp = app.oneshot(req).await?;
        let (status, snap_val) = response_to_json(resp).await?;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(snap_val["label"], "Genesis Commissioning State");
        assert_eq!(snap_val["entityCount"], 1);
        assert!(
            snap_val["checksum"]
                .as_str()
                .unwrap_or("")
                .starts_with("sha256:")
        );
        assert_eq!(snap_val["includes"], json!(["ships"]));

        let app = router_for_store(store);
        let req = Request::builder()
            .method("GET")
            .uri("/api/snapshots")
            .body(Body::empty())?;

        let resp = app.oneshot(req).await?;
        let (status, list_val) = response_to_json(resp).await?;

        assert_eq!(status, StatusCode::OK);
        let list = list_val.as_array().ok_or("expected array")?;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["label"], "Genesis Commissioning State");
        Ok(())
    }
}
