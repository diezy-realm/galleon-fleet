//! Dynamic route handlers for plugin self-registration and core handlers migration.

use std::collections::HashMap;
use std::sync::Arc;
use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use clawcrew_routing::{DynamicRouter, RouteHandler};
use crate::AppState;

/// Adapter for `/api/config`
#[derive(Clone)]
pub struct ConfigApiHandler {
    pub state: AppState,
}

impl std::fmt::Debug for ConfigApiHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigApiHandler").finish()
    }
}

impl ConfigApiHandler {
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl RouteHandler for ConfigApiHandler {
    async fn handle(&self, req: Request) -> Result<Response, String> {
        let headers = req.headers().clone();
        let resp = crate::api_config::handle_config_get(State(self.state.clone()), headers).await;
        Ok(resp)
    }

    fn path_pattern(&self) -> &str {
        "/api/config"
    }

    fn http_method(&self) -> &str {
        "GET"
    }

    fn name(&self) -> &str {
        "ConfigApiHandler"
    }
}

/// Adapter for `/api/logs`
#[derive(Clone)]
pub struct LogsApiHandler {
    pub state: AppState,
}

impl std::fmt::Debug for LogsApiHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogsApiHandler").finish()
    }
}

impl LogsApiHandler {
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl RouteHandler for LogsApiHandler {
    async fn handle(&self, req: Request) -> Result<Response, String> {
        let headers = req.headers().clone();
        let query_map: HashMap<String, String> = Query::try_from_uri(req.uri())
            .map(|Query(q)| q)
            .unwrap_or_default();

        let resp = crate::api_logs::handle_api_logs(
            State(self.state.clone()),
            headers,
            Query(query_map),
        )
        .await;
        Ok(resp)
    }

    fn path_pattern(&self) -> &str {
        "/api/logs"
    }

    fn http_method(&self) -> &str {
        "GET"
    }

    fn name(&self) -> &str {
        "LogsApiHandler"
    }
}

/// Adapter for `/api/auth/ratelimit`
#[derive(Clone)]
pub struct AuthRateLimitHandler {
    pub state: AppState,
}

impl std::fmt::Debug for AuthRateLimitHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthRateLimitHandler").finish()
    }
}

impl AuthRateLimitHandler {
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl RouteHandler for AuthRateLimitHandler {
    async fn handle(&self, _req: Request) -> Result<Response, String> {
        let resp = Json(serde_json::json!({
            "status": "active",
            "max_attempts": crate::auth_rate_limit::MAX_ATTEMPTS,
            "window_secs": crate::auth_rate_limit::WINDOW_SECS,
            "lockout_secs": crate::auth_rate_limit::LOCKOUT_SECS,
        }))
        .into_response();
        Ok(resp)
    }

    fn path_pattern(&self) -> &str {
        "/api/auth/ratelimit"
    }

    fn http_method(&self) -> &str {
        "GET"
    }

    fn name(&self) -> &str {
        "AuthRateLimitHandler"
    }
}

/// Adapter for `/api/webauthn`
#[derive(Clone)]
pub struct WebAuthnApiHandler {
    pub state: AppState,
}

impl std::fmt::Debug for WebAuthnApiHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebAuthnApiHandler").finish()
    }
}

impl WebAuthnApiHandler {
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl RouteHandler for WebAuthnApiHandler {
    async fn handle(&self, req: Request) -> Result<Response, String> {
        let headers = req.headers().clone();
        if let Err(e) = crate::api::require_auth(&self.state, &headers) {
            return Ok(e.into_response());
        }

        let resp = Json(serde_json::json!({
            "enabled": cfg!(feature = "webauthn"),
            "status": "ready"
        }))
        .into_response();
        Ok(resp)
    }

    fn path_pattern(&self) -> &str {
        "/api/webauthn"
    }

    fn http_method(&self) -> &str {
        "GET"
    }

    fn name(&self) -> &str {
        "WebAuthnApiHandler"
    }
}

/// Adapter for `/api/audit`
#[derive(Clone)]
pub struct AuditApiHandler {
    pub state: AppState,
}

impl std::fmt::Debug for AuditApiHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditApiHandler").finish()
    }
}

impl AuditApiHandler {
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl RouteHandler for AuditApiHandler {
    async fn handle(&self, req: Request) -> Result<Response, String> {
        let headers = req.headers().clone();
        let query: crate::api_audit::AuditQuery = Query::try_from_uri(req.uri())
            .map(|Query(q)| q)
            .unwrap_or_default();

        let resp = crate::api_audit::handle_api_audit(
            State(self.state.clone()),
            headers,
            Query(query),
        )
        .await;
        Ok(resp)
    }

    fn path_pattern(&self) -> &str {
        "/api/audit"
    }

    fn http_method(&self) -> &str {
        "GET"
    }

    fn name(&self) -> &str {
        "AuditApiHandler"
    }
}

/// Registers the 5 core immutable route handlers with the dynamic router.
pub async fn register_core_handlers(
    router: &DynamicRouter,
    state: AppState,
) -> Result<(), String> {
    router.register(Box::new(ConfigApiHandler::new(state.clone()))).await?;
    router.register(Box::new(LogsApiHandler::new(state.clone()))).await?;
    router.register(Box::new(AuthRateLimitHandler::new(state.clone()))).await?;
    router.register(Box::new(WebAuthnApiHandler::new(state.clone()))).await?;
    router.register(Box::new(AuditApiHandler::new(state))).await?;
    Ok(())
}

/// Fallback handler that dispatches unmatched requests to the dynamic router.
pub async fn dynamic_router_fallback(
    State(router): State<Arc<DynamicRouter>>,
    req: Request,
) -> Response {
    match router.dispatch(req).await {
        Ok(resp) => resp,
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": err
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Method;

    fn mock_app_state() -> AppState {
        let config = crate::Config::default();
        crate::api_config::tests::test_state(config)
    }

    #[tokio::test]
    async fn test_register_core_handlers_and_collision_rejection() {
        let state = mock_app_state();
        let router = DynamicRouter::new();

        // 1. Register 5 core handlers
        register_core_handlers(&router, state.clone())
            .await
            .expect("register core handlers should succeed");
        assert_eq!(router.len().await, 5);

        // 2. Collision rejection: Attempting to register another handler on /api/config must fail
        #[derive(Debug)]
        struct RoguePluginHandler;
        #[async_trait::async_trait]
        impl RouteHandler for RoguePluginHandler {
            async fn handle(&self, _req: Request) -> Result<Response, String> {
                Ok((StatusCode::OK, "rogue").into_response())
            }
            fn path_pattern(&self) -> &str {
                "/api/config"
            }
            fn http_method(&self) -> &str {
                "GET"
            }
            fn name(&self) -> &str {
                "RoguePluginHandler"
            }
        }

        let collision_res = router.register(Box::new(RoguePluginHandler)).await;
        assert!(collision_res.is_err(), "Collision with core /api/config must be rejected");
        assert!(
            collision_res.unwrap_err().contains("Route collision"),
            "Error message must specify route collision"
        );
    }

    #[tokio::test]
    async fn test_core_handlers_dispatch_responses() {
        let state = mock_app_state();
        let router = DynamicRouter::new();
        register_core_handlers(&router, state).await.unwrap();

        // Check /api/auth/ratelimit
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/auth/ratelimit")
            .body(Body::empty())
            .unwrap();

        let resp = router.dispatch(req).await.expect("dispatch to /api/auth/ratelimit");
        assert_eq!(resp.status(), StatusCode::OK);

        // Check /api/webauthn
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/webauthn")
            .body(Body::empty())
            .unwrap();

        let resp = router.dispatch(req).await.expect("dispatch to /api/webauthn");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dynamic_router_as_axum_fallback() {
        use axum::routing::get;
        use tower::ServiceExt;

        let state = mock_app_state();
        let router = Arc::new(DynamicRouter::new());
        register_core_handlers(&router, state).await.unwrap();

        let app = axum::Router::new()
            .route("/api/ping", get(|| async { "pong" }))
            .fallback(dynamic_router_fallback)
            .with_state(router);

        // Standard route
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ping")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Dynamic fallback route (/api/auth/ratelimit)
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/ratelimit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
