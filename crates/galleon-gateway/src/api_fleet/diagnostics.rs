//! HTTP handlers for Galleon Fleet diagnostics endpoint (`GET /api/diagnostics`).
//!
//! Under SPEC-013 §Model Data (TASK-077):
//! - `GET /api/diagnostics`:
//!   - Maps `galleon_runtime::doctor::diagnose(&Config)` / `DiagResult` to `DiagnosticItem`.
//!   - Real diagnostic results (not static/mocked list).
//!   - Severity mapped to status: `Severity::Ok` -> `"healthy"`, `Severity::Warn` -> `"warning"`, `Severity::Error` -> `"error"`.
//!   - Latency set to honest em-dash `"—"`.
//!   - `POST /api/diagnostics/remedy` is intentionally NOT ported (dropped per SPEC-013).

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use galleon_api::doctor::{DiagResult, DiagnosticsRunner, Severity};
use galleon_config::schema::Config;
use galleon_fleet_core::{
    DiagnosticItem, FLEET_MAX_BODY, LATENCY_UNMEASURED, STATUS_ERROR, STATUS_HEALTHY,
    STATUS_WARNING,
};
use parking_lot::RwLock;
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Maps a `DiagResult` from runtime doctor to a `DiagnosticItem` conforming to SPEC-013.
#[must_use]
pub fn map_diag_result_to_item(index: usize, result: &DiagResult) -> DiagnosticItem {
    let status = match result.severity {
        Severity::Ok => STATUS_HEALTHY,
        Severity::Warn => STATUS_WARNING,
        Severity::Error => STATUS_ERROR,
    };

    DiagnosticItem {
        id: format!("d-{}", index + 1),
        component: result.category.clone(),
        status: status.to_string(),
        latency: LATENCY_UNMEASURED.to_string(),
        detail: result.message.clone(),
    }
}

/// Assembles diagnostics routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/diagnostics", get(handle_get_diagnostics_state))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing with dedicated configuration.
pub fn router_for_config(config: Arc<RwLock<Config>>) -> Router {
    Router::new()
        .route(
            "/api/diagnostics",
            get(move || {
                let cfg = Arc::clone(&config);
                async move {
                    let config_guard = cfg.read();
                    handle_get_diagnostics_impl(&config_guard)
                }
            }),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
}

/// Standalone router with default configuration.
pub fn router() -> Router {
    router_for_config(Arc::new(RwLock::new(Config::default())))
}

/// Standalone router with a custom diagnostics runner.
pub fn router_for_runner(runner: Arc<dyn DiagnosticsRunner>) -> Router {
    Router::new()
        .route(
            "/api/diagnostics",
            get(move || {
                let r = Arc::clone(&runner);
                async move {
                    let results = r.run_diagnostics();
                    handle_diag_results_response(&results)
                }
            }),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
}

async fn handle_get_diagnostics_state(State(state): State<AppState>) -> Response {
    let results = if let Some(diag) = &state.diagnostics {
        diag.run_diagnostics()
    } else {
        let config = state.config.read();
        galleon_runtime::doctor::diagnose(&config)
    };
    handle_diag_results_response(&results)
}

/// Pure implementation that runs diagnostics on the given config.
pub fn handle_get_diagnostics_impl(config: &Config) -> Response {
    let results = galleon_runtime::doctor::diagnose(config);
    handle_diag_results_response(&results)
}

/// Formats diagnostic results into an HTTP JSON response.
pub fn handle_diag_results_response(results: &[DiagResult]) -> Response {
    let items: Vec<DiagnosticItem> = results
        .iter()
        .enumerate()
        .map(|(i, r)| map_diag_result_to_item(i, r))
        .collect();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json!(items).to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    struct MockDiagnosticsRunner {
        results: Vec<DiagResult>,
    }

    impl DiagnosticsRunner for MockDiagnosticsRunner {
        fn run_diagnostics(&self) -> Vec<DiagResult> {
            self.results.clone()
        }
    }

    #[tokio::test]
    async fn test_get_diagnostics_real_results() {
        let app = router();
        let req = Request::builder()
            .uri("/api/diagnostics")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let array = json.as_array().expect("must be a JSON array");

        // Real diagnostic results from doctor::diagnose(&Config::default())
        assert!(!array.is_empty(), "diagnostics must produce real results");

        for (idx, item) in array.iter().enumerate() {
            assert_eq!(item["id"], format!("d-{}", idx + 1));
            assert!(item["component"].is_string());
            assert!(!item["component"].as_str().unwrap().is_empty());

            let status = item["status"].as_str().unwrap();
            assert!(
                status == STATUS_HEALTHY || status == STATUS_WARNING || status == STATUS_ERROR,
                "status must be healthy, warning, or error, got: {status}"
            );

            assert_eq!(item["latency"], "—");
            assert!(item["detail"].is_string());
            assert!(!item["detail"].as_str().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn test_get_diagnostics_mock_runner_mapping() {
        let mock_runner = Arc::new(MockDiagnosticsRunner {
            results: vec![
                DiagResult {
                    severity: Severity::Ok,
                    category: "workspace".to_string(),
                    message: "workspace root verified".to_string(),
                },
                DiagResult {
                    severity: Severity::Warn,
                    category: "providers.models".to_string(),
                    message: "no models configured".to_string(),
                },
                DiagResult {
                    severity: Severity::Error,
                    category: "daemon".to_string(),
                    message: "daemon lockfile corrupted".to_string(),
                },
            ],
        });

        let app = router_for_runner(mock_runner);
        let req = Request::builder()
            .uri("/api/diagnostics")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let array = json.as_array().expect("must be array");
        assert_eq!(array.len(), 3);

        assert_eq!(array[0]["id"], "d-1");
        assert_eq!(array[0]["component"], "workspace");
        assert_eq!(array[0]["status"], "healthy");
        assert_eq!(array[0]["latency"], "—");
        assert_eq!(array[0]["detail"], "workspace root verified");

        assert_eq!(array[1]["id"], "d-2");
        assert_eq!(array[1]["component"], "providers.models");
        assert_eq!(array[1]["status"], "warning");
        assert_eq!(array[1]["latency"], "—");
        assert_eq!(array[1]["detail"], "no models configured");

        assert_eq!(array[2]["id"], "d-3");
        assert_eq!(array[2]["component"], "daemon");
        assert_eq!(array[2]["status"], "error");
        assert_eq!(array[2]["latency"], "—");
        assert_eq!(array[2]["detail"], "daemon lockfile corrupted");
    }

    #[tokio::test]
    async fn test_diagnostics_remedy_not_ported_returns_404() {
        let app = router();
        let req = Request::builder()
            .uri("/api/diagnostics/remedy")
            .method("POST")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
