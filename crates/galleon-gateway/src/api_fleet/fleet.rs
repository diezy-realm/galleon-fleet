//! HTTP handlers for Galleon Fleet general endpoints (`/api/fleet/*`).
//!
//! Under SPEC-013 §Seed (TASK-070):
//! - `GET /api/fleet/seed` returns `{"status":"ok","seed":{<camelCase>: [...]}}`
//!   with all 15 canonical collections populated as non-empty arrays,
//!   fixing F-16 and enabling complete frontend hydration via `hydrateFleet`.

use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use galleon_fleet_core::{FLEET_MAX_BODY, get_seed_response_json, load_seed_response};
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Assembles fleet general routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/fleet/seed", get(handle_get_fleet_seed))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing without AppState.
pub fn router() -> Router {
    Router::new()
        .route("/api/fleet/seed", get(handle_get_fleet_seed))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
}

async fn handle_get_fleet_seed() -> Response {
    match get_seed_response_json() {
        Ok(json_str) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json_str.to_string(),
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_get_fleet_seed_endpoint_returns_ok_and_15_collections() {
        let app = router();

        let req = Request::builder()
            .method("GET")
            .uri("/api/fleet/seed")
            .body(Body::empty())
            .expect("request builder");

        let resp = app.oneshot(req).await.expect("oneshot call");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );

        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let parsed: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("parse JSON response");

        assert_eq!(parsed["status"], "ok");

        let seed = parsed["seed"]
            .as_object()
            .expect("seed field must be object");
        assert_eq!(
            seed.len(),
            15,
            "seed must contain exactly 15 canonical collections"
        );

        // Verify key acceptance test: hydrateFleet web can fill 15/15 collections
        let seed_map = [
            "squads",
            "ships",
            "crew",
            "quests",
            "artifacts",
            "approvals",
            "logbook",
            "treasuryLedger",
            "notifications",
            "journalSessions",
            "chatMessages",
            "trainingSkills",
            "globalSteering",
            "steeringDirectives",
            "trainingHooks",
        ];

        let mut hydrated_count = 0;
        for col in seed_map {
            let col_val = seed.get(col);
            assert!(
                col_val.is_some(),
                "missing required collection for web hydrateFleet: {col}"
            );
            let arr = col_val.unwrap().as_array();
            assert!(
                arr.is_some(),
                "collection {col} must be JSON array in seed response"
            );
            assert!(
                !arr.unwrap().is_empty(),
                "collection {col} must be non-empty for web hydration"
            );
            hydrated_count += 1;
        }

        assert_eq!(
            hydrated_count, 15,
            "web hydrateFleet must receive 15/15 non-empty collections"
        );
    }
}
