//! HTTP handlers for Galleon Fleet general endpoints (`/api/fleet/*`).
//!
//! Under SPEC-013:
//! - §Seed (TASK-070): `GET /api/fleet/seed` returns `{"status":"ok","seed":{<camelCase>: [...]}}`
//!   with all 15 canonical collections populated as non-empty arrays,
//!   fixing F-16 and enabling complete frontend hydration via `hydrateFleet`.
//! - §Model Data (TASK-071):
//!   - `GET /api/fleet/metrics` serves collection entity counters and honest telemetry
//!     where unmeasured metrics (`systemUptime`, `gatewayLatencyMs`, `activeWorkers`) return `null`.
//!   - `POST /api/fleet/deck-bell` broadcasts the sovereign ceremonial ship bell alert.

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use galleon_fleet_core::{DeckBellResponse, FLEET_MAX_BODY, FleetStore, get_seed_response_json};
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Assembles fleet general routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/fleet/seed", get(handle_get_fleet_seed))
        .route("/api/fleet/metrics", get(handle_get_fleet_metrics_state))
        .route("/api/fleet/deck-bell", post(handle_post_deck_bell))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing with a dedicated FleetStore.
pub fn router_for_store(store: Arc<FleetStore>) -> Router {
    Router::new()
        .route("/api/fleet/seed", get(handle_get_fleet_seed))
        .route("/api/fleet/metrics", get(handle_get_fleet_metrics_store))
        .route("/api/fleet/deck-bell", post(handle_post_deck_bell))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(store)
}

/// Standalone router for testing without AppState using environment data dir.
pub fn router() -> Router {
    let store = Arc::new(FleetStore::from_env());
    router_for_store(store)
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

async fn handle_get_fleet_metrics_state(State(state): State<AppState>) -> Response {
    let store = {
        let config = state.config.read();
        FleetStore::from_data_dir(&config.data_dir)
    };
    handle_get_fleet_metrics_impl(&store)
}

async fn handle_get_fleet_metrics_store(State(store): State<Arc<FleetStore>>) -> Response {
    handle_get_fleet_metrics_impl(&store)
}

fn handle_get_fleet_metrics_impl(store: &FleetStore) -> Response {
    let memory_used_mb = current_memory_used_mb();
    let metrics = store.get_metrics(memory_used_mb);
    match serde_json::to_string(&metrics) {
        Ok(json_str) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json_str,
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

async fn handle_post_deck_bell() -> Response {
    let resp = DeckBellResponse::default();
    match serde_json::to_string(&resp) {
        Ok(json_str) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json_str,
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

fn current_memory_used_mb() -> Option<f64> {
    let stats = galleon_infra::process_stats::sample();
    if stats.rss_bytes > 0 {
        Some(stats.rss_bytes as f64 / (1024.0 * 1024.0))
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::Value;
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
        let parsed: Value = serde_json::from_slice(&body_bytes).expect("parse JSON response");

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

    #[tokio::test]
    async fn test_get_fleet_metrics_endpoint_and_shape_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(FleetStore::new(tmp.path()));

        // Populate realistic test data matching Go fixture expectations
        store
            .save_collection(
                "ships",
                b"[{\"id\":\"s1\"},{\"id\":\"s2\"},{\"id\":\"s3\"}]",
            )
            .await
            .expect("save ships");
        store
            .save_collection(
                "quests",
                b"[{\"id\":\"q1\",\"status\":\"underway\"},{\"id\":\"q2\",\"status\":\"completed\"}]",
            )
            .await
            .expect("save quests");
        store
            .save_collection(
                "approvals",
                b"[{\"id\":\"a1\",\"status\":\"pending\"},{\"id\":\"a2\",\"status\":\"pending\"}]",
            )
            .await
            .expect("save approvals");
        store
            .save_collection("artifacts", b"[{\"id\":\"art1\"},{\"id\":\"art2\"}]")
            .await
            .expect("save artifacts");
        store
            .save_collection(
                "crew",
                b"[{\"id\":\"c1\"},{\"id\":\"c2\"},{\"id\":\"c3\"},{\"id\":\"c4\"},{\"id\":\"c5\"},{\"id\":\"c6\"},{\"id\":\"c7\"}]",
            )
            .await
            .expect("save crew");
        store
            .save_collection("squads", b"[{\"id\":\"sq1\"},{\"id\":\"sq2\"}]")
            .await
            .expect("save squads");

        let app = router_for_store(store);

        let req = Request::builder()
            .method("GET")
            .uri("/api/fleet/metrics")
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
        let parsed: Value = serde_json::from_slice(&body_bytes).expect("parse JSON response");
        let parsed_obj = parsed.as_object().expect("response must be JSON object");

        // Load Go fixture contract
        let fixture_str =
            include_str!("../../../../tests/fixtures/go-fleet/GET_api_fleet_metrics.json");
        let fixture_val: Value = serde_json::from_str(fixture_str).expect("parse Go fixture");
        let fixture_obj = fixture_val.as_object().expect("fixture must be object");

        // Verify shape parity: all Go fixture keys exist in Rust response
        for key in fixture_obj.keys() {
            assert!(
                parsed_obj.contains_key(key),
                "Missing expected key from Go fixture: {key}"
            );
        }

        // Verify no foreign unexpected keys
        for key in parsed_obj.keys() {
            assert!(
                fixture_obj.contains_key(key),
                "Unexpected extra key not in Go fixture: {key}"
            );
        }

        // Verify counts against populated data
        assert_eq!(parsed["activeVessels"], 3);
        assert_eq!(parsed["underwayQuests"], 1);
        assert_eq!(parsed["pendingApprovals"], 2);
        assert_eq!(parsed["totalArtifacts"], 2);
        assert_eq!(parsed["totalSpecialists"], 7);
        assert_eq!(parsed["totalSquads"], 2);
        assert_eq!(parsed["totalTreasuryLedgers"], 0);

        // Verify intentional honest differences (SPEC-013 F-13)
        assert!(
            parsed["systemUptime"].is_null(),
            "systemUptime must be null (honest unmeasured)"
        );
        assert!(
            parsed["gatewayLatencyMs"].is_null(),
            "gatewayLatencyMs must be null (honest unmeasured)"
        );
        assert!(
            parsed["activeWorkers"].is_null(),
            "activeWorkers must be null (honest unmeasured)"
        );

        // memoryUsedMB is either number (RSS sampled) or null
        assert!(
            parsed["memoryUsedMB"].is_number() || parsed["memoryUsedMB"].is_null(),
            "memoryUsedMB must be number or null"
        );
    }

    #[tokio::test]
    async fn test_post_deck_bell_endpoint_and_shape_contract() {
        let app = router();

        let req = Request::builder()
            .method("POST")
            .uri("/api/fleet/deck-bell")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
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
        let parsed: Value = serde_json::from_slice(&body_bytes).expect("parse JSON response");

        // Load Go fixture contract
        let fixture_str =
            include_str!("../../../../tests/fixtures/go-fleet/POST_api_fleet_deck_bell.json");
        let fixture_val: Value = serde_json::from_str(fixture_str).expect("parse Go fixture");

        assert_eq!(parsed["success"], fixture_val["success"]);
        assert_eq!(parsed["message"], fixture_val["message"]);
        assert_eq!(parsed["success"], true);
        assert_eq!(
            parsed["message"],
            "Sovereign Ship Bell sounded across the fleet deck. All hands on alert."
        );
    }

    #[tokio::test]
    async fn test_methods_not_allowed() {
        let app = router();

        // POST /api/fleet/metrics is not allowed (only GET)
        let req1 = Request::builder()
            .method("POST")
            .uri("/api/fleet/metrics")
            .body(Body::empty())
            .expect("request builder");
        let resp1 = app.clone().oneshot(req1).await.expect("call");
        assert_eq!(resp1.status(), StatusCode::METHOD_NOT_ALLOWED);

        // GET /api/fleet/deck-bell is not allowed (only POST)
        let req2 = Request::builder()
            .method("GET")
            .uri("/api/fleet/deck-bell")
            .body(Body::empty())
            .expect("request builder");
        let resp2 = app.oneshot(req2).await.expect("call");
        assert_eq!(resp2.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
