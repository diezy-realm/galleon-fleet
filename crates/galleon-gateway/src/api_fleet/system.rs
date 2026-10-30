//! HTTP handlers for Galleon Fleet system endpoints (`/api/system/*`, `/api/engine/processes`, `/api/network/qrcode`).
//!
//! Under SPEC-013 §Model Data (TASK-073 & TASK-074):
//! - `GET /api/system/metrics`:
//!   - Uses `sysinfo` 0.36 for real OS CPU, memory, and active process count.
//!   - `goVersion` is removed from the response.
//!   - `version` is added using `env!("CARGO_PKG_VERSION")`.
//!   - `uptime` is derived from `galleon_infra::health::snapshot().uptime_seconds`
//!     formatted as `"Xh Ym"`.
//!   - Unmeasured metrics (`gateway_latency_ms`, `isolation_mode`, `memory_db_mb`, `active_threads`)
//!     honestly serialize to `null` (no fabricated constants).
//! - `GET /api/system/network`:
//!   - `engine_port` is removed from the response.
//!   - `gateway_port` reports the real configured bind port.
//!   - `local_address` and `status` ("online") reflect canonical gateway state.
//! - `GET /api/network/qrcode`:
//!   - Generates QR code using `qrcode 0.14` with SVG feature without pulling `image`.
//!   - Encodes SVG payload as base64 in `dataUrl` (`data:image/svg+xml;base64,...`).
//!   - Strict validation: URL must start with `http://` or `https://`, length <= 2048 chars.
//!   - Default target URL falls back to local gateway LAN URL.
//! - `GET /api/system/executive-briefing`:
//!   - Aggregates sovereign fleet watch report across active artifacts, crew, underway voyages,
//!     and pending approvals.
//!   - `treasuryBurnRate` is honestly calculated from `treasuryLedger` collection or returns `null`.
//! - `GET /api/engine/processes`:
//!   - Uses `sysinfo` 0.36 to enumerate processes.
//!   - Filters strictly for processes whose name starts with `galleon*`.
//!   - Returns array of `{ pid, name, memory, cpu }`.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use base64::Engine;
use galleon_fleet_core::{FleetStore, FLEET_MAX_BODY};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System,
};
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Memory usage telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryUsage {
    pub percent: u64,
    #[serde(rename = "totalMB")]
    pub total_mb: u64,
    #[serde(rename = "usedMB")]
    pub used_mb: u64,
}

/// System metrics telemetry response satisfying web apiClient contract with honest telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMetricsResponse {
    #[serde(rename = "activeProcesses")]
    pub active_processes: usize,
    pub active_threads: Option<u32>,
    pub arch: String,
    #[serde(rename = "cpuUsage")]
    pub cpu_usage: f64,
    pub gateway_latency_ms: Option<u64>,
    pub isolation_mode: Option<String>,
    #[serde(rename = "memoryUsage")]
    pub memory_usage: MemoryUsage,
    pub memory_db_mb: Option<f64>,
    pub platform: String,
    pub uptime: String,
    pub version: String,
}

/// System network telemetry response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemNetworkResponse {
    pub gateway_port: u16,
    pub local_address: String,
    pub status: String,
}

/// Query parameters for `GET /api/network/qrcode`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct QrCodeQuery {
    pub url: Option<String>,
}

/// Network QR code response with base64 dataUrl and canonical url.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkQrCodeResponse {
    #[serde(rename = "dataUrl")]
    pub data_url: String,
    pub url: String,
}

/// Process item representation for `/api/engine/processes`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineProcessItem {
    pub cpu: f64,
    pub memory: u64,
    pub name: String,
    pub pid: u32,
}

/// Standalone configuration for testing system network and briefing endpoints without AppState.
#[derive(Clone, Debug)]
pub struct SystemApiConfig {
    pub gateway_port: u16,
    pub local_address: String,
    pub store: Arc<FleetStore>,
}

impl Default for SystemApiConfig {
    fn default() -> Self {
        Self {
            gateway_port: 50052,
            local_address: "127.0.0.1".to_string(),
            store: Arc::new(FleetStore::from_env()),
        }
    }
}

/// Assembles system routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/system/metrics", get(handle_get_system_metrics))
        .route("/api/system/network", get(handle_get_system_network_state))
        .route(
            "/api/system/executive-briefing",
            get(handle_get_executive_briefing_state),
        )
        .route("/api/network/qrcode", get(handle_get_network_qrcode_state))
        .route("/api/engine/processes", get(handle_get_engine_processes))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing with dedicated configuration.
pub fn router_for_config(config: SystemApiConfig) -> Router {
    let cfg_net = config.clone();
    let cfg_qr = config.clone();
    let cfg_brief = config.clone();
    Router::new()
        .route("/api/system/metrics", get(handle_get_system_metrics))
        .route(
            "/api/system/network",
            get({
                let config = cfg_net;
                move || async move {
                    handle_get_system_network_impl(config.gateway_port, config.local_address)
                }
            }),
        )
        .route(
            "/api/system/executive-briefing",
            get({
                let config = cfg_brief;
                move || async move { handle_get_executive_briefing_impl(&config.store) }
            }),
        )
        .route(
            "/api/network/qrcode",
            get({
                let config = cfg_qr;
                move |Query(query): Query<QrCodeQuery>| async move {
                    handle_get_network_qrcode_impl(
                        query,
                        config.gateway_port,
                        &config.local_address,
                    )
                }
            }),
        )
        .route("/api/engine/processes", get(handle_get_engine_processes))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
}

/// Standalone router with default test configuration.
pub fn router() -> Router {
    router_for_config(SystemApiConfig::default())
}

async fn handle_get_system_metrics() -> Response {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    );
    sys.refresh_memory();
    sys.refresh_cpu_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let total_ram_bytes = sys.total_memory();
    let used_ram_bytes = sys.used_memory();
    let total_mb = total_ram_bytes / (1024 * 1024);
    let used_mb = used_ram_bytes / (1024 * 1024);
    let percent = (used_mb * 100).checked_div(total_mb).unwrap_or(0);

    let cpu_usage = sys.global_cpu_usage() as f64;
    let active_processes = sys.processes().len();

    let uptime_seconds = galleon_infra::health::snapshot().uptime_seconds;
    let hours = uptime_seconds / 3600;
    let minutes = (uptime_seconds % 3600) / 60;
    let uptime = format!("{hours}h {minutes}m");

    let metrics = SystemMetricsResponse {
        active_processes,
        active_threads: None,
        arch: std::env::consts::ARCH.to_string(),
        cpu_usage,
        gateway_latency_ms: None,
        isolation_mode: None,
        memory_usage: MemoryUsage {
            percent,
            total_mb,
            used_mb,
        },
        memory_db_mb: None,
        platform: std::env::consts::OS.to_string(),
        uptime,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

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

async fn handle_get_system_network_state(State(state): State<AppState>) -> Response {
    let (gateway_port, local_address) = {
        let config = state.config.read();
        (config.gateway.port, config.gateway.host.clone())
    };
    handle_get_system_network_impl(gateway_port, local_address)
}

fn handle_get_system_network_impl(gateway_port: u16, local_address: String) -> Response {
    let resp = SystemNetworkResponse {
        gateway_port,
        local_address,
        status: "online".to_string(),
    };
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

async fn handle_get_executive_briefing_state(State(state): State<AppState>) -> Response {
    let store = {
        let config = state.config.read();
        FleetStore::from_data_dir(&config.data_dir)
    };
    handle_get_executive_briefing_impl(&store)
}

pub fn handle_get_executive_briefing_impl(store: &FleetStore) -> Response {
    let date = chrono::Local::now().format("%A, %d %B %Y").to_string();
    let generated_at = chrono::Utc::now().to_rfc3339();
    let briefing = store.get_executive_briefing(date, generated_at);

    match serde_json::to_string(&briefing) {
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

async fn handle_get_network_qrcode_state(
    State(state): State<AppState>,
    Query(query): Query<QrCodeQuery>,
) -> Response {
    let (gateway_port, local_address) = {
        let config = state.config.read();
        (config.gateway.port, config.gateway.host.clone())
    };
    handle_get_network_qrcode_impl(query, gateway_port, &local_address)
}

pub fn handle_get_network_qrcode_impl(
    query: QrCodeQuery,
    gateway_port: u16,
    local_address: &str,
) -> Response {
    let target_url = match query.url {
        Some(u) if !u.trim().is_empty() => {
            let trimmed = u.trim();
            if trimmed.len() > 2048 {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    json!({ "error": "url exceeds 2048 characters" }).to_string(),
                )
                    .into_response();
            }
            if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    json!({ "error": "url must start with http:// or https://" }).to_string(),
                )
                    .into_response();
            }
            trimmed.to_string()
        }
        _ => format!("http://{local_address}:{gateway_port}"),
    };

    let code = match qrcode::QrCode::new(target_url.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "application/json")],
                json!({ "error": format!("failed to generate QR code: {e}") }).to_string(),
            )
                .into_response();
        }
    };

    let svg_xml = code.render::<qrcode::render::svg::Color>().build();
    let encoded = base64::engine::general_purpose::STANDARD.encode(svg_xml.as_bytes());
    let data_url = format!("data:image/svg+xml;base64,{encoded}");

    let resp = NetworkQrCodeResponse {
        data_url,
        url: target_url,
    };

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

async fn handle_get_engine_processes() -> Response {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_cpu().with_memory()),
    );
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );

    let procs = collect_galleon_processes(&sys);

    match serde_json::to_string(&procs) {
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

/// Helper that filters processes to only include those whose name starts with "galleon".
pub fn collect_galleon_processes(sys: &System) -> Vec<EngineProcessItem> {
    let mut procs = Vec::new();
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy();
        if name.starts_with("galleon") {
            let memory_mb = proc.memory() / (1024 * 1024);
            let cpu = proc.cpu_usage() as f64;
            procs.push(EngineProcessItem {
                cpu,
                memory: memory_mb,
                name: name.to_string(),
                pid: pid.as_u32(),
            });
        }
    }
    procs.sort_by_key(|p| p.pid);
    procs
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
    async fn test_get_system_metrics_contract_and_honest_values() {
        let app = router();

        let req = Request::builder()
            .method("GET")
            .uri("/api/system/metrics")
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
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/go-fleet/GET_api_system_metrics.json");
        let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go fixture");
        let fixture_val: Value = serde_json::from_str(&fixture_str).expect("parse Go fixture");
        let fixture_obj = fixture_val.as_object().expect("fixture must be object");

        // SPEC-013 & TASK-073 mandate: goVersion is removed, version added
        assert!(
            !parsed_obj.contains_key("goVersion"),
            "goVersion must be removed from response"
        );
        assert_eq!(
            parsed["version"],
            env!("CARGO_PKG_VERSION"),
            "version must match CARGO_PKG_VERSION"
        );

        // Every key in Go fixture except goVersion must exist in Rust response
        for key in fixture_obj.keys() {
            if key == "goVersion" {
                continue;
            }
            assert!(
                parsed_obj.contains_key(key),
                "Missing expected key from Go fixture: {key}"
            );
        }

        // Every key in Rust response must either be in Go fixture or be the new 'version' key
        for key in parsed_obj.keys() {
            if key == "version" {
                continue;
            }
            assert!(
                fixture_obj.contains_key(key),
                "Unexpected extra key not in Go fixture: {key}"
            );
        }

        // Verify honest telemetry: unmeasured values serialize to null (no fabricated constants)
        assert!(
            parsed["gateway_latency_ms"].is_null(),
            "gateway_latency_ms must be null (unmeasured)"
        );
        assert!(
            parsed["active_threads"].is_null(),
            "active_threads must be null (unmeasured)"
        );
        assert!(
            parsed["isolation_mode"].is_null(),
            "isolation_mode must be null (unmeasured)"
        );
        assert!(
            parsed["memory_db_mb"].is_null(),
            "memory_db_mb must be null (unmeasured)"
        );

        // Verify measured values
        assert!(
            parsed["activeProcesses"].as_u64().is_some(),
            "activeProcesses must be a non-negative integer"
        );
        let uptime_str = parsed["uptime"]
            .as_str()
            .expect("uptime must be string formatted as 'Xh Ym'");
        assert!(
            uptime_str.ends_with('m'),
            "uptime format should end with 'm'"
        );
        assert!(
            parsed["platform"].as_str().is_some(),
            "platform must be non-empty string"
        );
        assert!(
            parsed["arch"].as_str().is_some(),
            "arch must be non-empty string"
        );
        assert!(
            parsed["cpuUsage"].as_f64().is_some(),
            "cpuUsage must be a float number"
        );

        let mem_usage = &parsed["memoryUsage"];
        assert!(
            mem_usage["totalMB"].as_u64().unwrap_or(0) > 0,
            "totalMB must be > 0"
        );
        assert!(
            mem_usage["usedMB"].as_u64().is_some(),
            "usedMB must be integer"
        );
        assert!(
            mem_usage["percent"].as_u64().is_some(),
            "percent must be integer"
        );
    }

    #[tokio::test]
    async fn test_get_system_network_contract_and_engine_port_removed() {
        let app = router_for_config(SystemApiConfig {
            gateway_port: 50052,
            local_address: "127.0.0.1".to_string(),
            ..Default::default()
        });

        let req = Request::builder()
            .method("GET")
            .uri("/api/system/network")
            .body(Body::empty())
            .expect("request builder");

        let resp = app.oneshot(req).await.expect("oneshot call");
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).expect("parse JSON response");
        let parsed_obj = parsed.as_object().expect("response must be JSON object");

        // Load Go fixture contract
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/go-fleet/GET_api_system_network.json");
        let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go fixture");
        let fixture_val: Value = serde_json::from_str(&fixture_str).expect("parse Go fixture");
        let fixture_obj = fixture_val.as_object().expect("fixture must be object");

        // SPEC-013 & TASK-073 mandate: engine_port is removed
        assert!(
            !parsed_obj.contains_key("engine_port"),
            "engine_port must be removed from response"
        );

        // All other fixture keys must exist
        for key in fixture_obj.keys() {
            if key == "engine_port" {
                continue;
            }
            assert!(
                parsed_obj.contains_key(key),
                "Missing expected key from Go fixture: {key}"
            );
        }

        assert_eq!(parsed["status"], "online");
        assert_eq!(parsed["local_address"], "127.0.0.1");
        assert_eq!(parsed["gateway_port"], 50052);
    }

    #[tokio::test]
    async fn test_get_engine_processes_contract_and_filtering() {
        let app = router();

        let req = Request::builder()
            .method("GET")
            .uri("/api/engine/processes")
            .body(Body::empty())
            .expect("request builder");

        let resp = app.oneshot(req).await.expect("oneshot call");
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).expect("parse JSON response");
        let procs = parsed.as_array().expect("response must be a JSON array");

        // Verify each process in the array strictly begins with "galleon" and has correct shape
        for p in procs {
            let name = p["name"].as_str().expect("name must be string");
            assert!(
                name.starts_with("galleon"),
                "Process name must start with 'galleon', found: {name}"
            );
            assert!(p["pid"].as_u64().is_some(), "pid must be number");
            assert!(p["memory"].as_u64().is_some(), "memory must be number");
            assert!(p["cpu"].as_f64().is_some(), "cpu must be float");
        }
    }

    #[test]
    fn test_collect_galleon_processes_filters_names() {
        let sys = System::new();
        let procs = collect_galleon_processes(&sys);
        for p in &procs {
            assert!(
                p.name.starts_with("galleon"),
                "expected galleon prefix for {}",
                p.name
            );
        }
    }

    #[tokio::test]
    async fn test_get_network_qrcode_default_and_contract_shape() {
        let app = router_for_config(SystemApiConfig {
            gateway_port: 9090,
            local_address: "127.0.0.1".to_string(),
            ..Default::default()
        });

        // 1. Default URL fallback
        let req = Request::builder()
            .method("GET")
            .uri("/api/network/qrcode")
            .body(Body::empty())
            .expect("request builder");

        let resp = app.clone().oneshot(req).await.expect("oneshot call");
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).expect("parse JSON response");
        assert_eq!(parsed["url"], "http://127.0.0.1:9090");

        let data_url = parsed["dataUrl"].as_str().expect("dataUrl must be string");
        assert!(
            data_url.starts_with("data:image/svg+xml;base64,"),
            "dataUrl must be SVG base64"
        );
        let b64_payload = data_url
            .strip_prefix("data:image/svg+xml;base64,")
            .expect("strip prefix");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64_payload)
            .expect("decode base64");
        let svg_str = String::from_utf8(decoded).expect("valid utf-8 SVG");
        assert!(svg_str.contains("<svg") && svg_str.contains("</svg>"));

        // 2. Go fixture contract check
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/go-fleet/GET_api_network_qrcode.json");
        let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go fixture");
        let fixture_val: Value = serde_json::from_str(&fixture_str).expect("parse Go fixture");
        let fixture_obj = fixture_val.as_object().expect("fixture object");

        for key in fixture_obj.keys() {
            assert!(
                parsed.as_object().unwrap().contains_key(key),
                "Missing fixture key in QR code response: {key}"
            );
        }

        // 3. Custom URL query parameter
        let custom_url = "https://galleon.local:8443/dashboard";
        let req_custom = Request::builder()
            .method("GET")
            .uri("/api/network/qrcode?url=https%3A%2F%2Fgalleon.local%3A8443%2Fdashboard")
            .body(Body::empty())
            .expect("request builder");

        let resp_custom = app.oneshot(req_custom).await.expect("oneshot call");
        assert_eq!(resp_custom.status(), StatusCode::OK);
        let body_bytes_custom = resp_custom
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let parsed_custom: Value =
            serde_json::from_slice(&body_bytes_custom).expect("parse JSON response");
        assert_eq!(parsed_custom["url"], custom_url);
    }

    #[tokio::test]
    async fn test_get_network_qrcode_validation_errors() {
        let app = router();

        // 1. Non-http(s) scheme
        let req_ftp = Request::builder()
            .method("GET")
            .uri("/api/network/qrcode?url=ftp://invalid-scheme.org")
            .body(Body::empty())
            .expect("request builder");
        let resp_ftp = app.clone().oneshot(req_ftp).await.expect("oneshot call");
        assert_eq!(resp_ftp.status(), StatusCode::BAD_REQUEST);

        // 2. Javascript scheme
        let req_js = Request::builder()
            .method("GET")
            .uri("/api/network/qrcode?url=javascript:alert(1)")
            .body(Body::empty())
            .expect("request builder");
        let resp_js = app.clone().oneshot(req_js).await.expect("oneshot call");
        assert_eq!(resp_js.status(), StatusCode::BAD_REQUEST);

        // 3. Length > 2048 chars
        let long_path = "a".repeat(2045);
        let long_url = format!("http://example.com/{long_path}");
        let req_long = Request::builder()
            .method("GET")
            .uri(format!("/api/network/qrcode?url={long_url}"))
            .body(Body::empty())
            .expect("request builder");
        let resp_long = app.oneshot(req_long).await.expect("oneshot call");
        assert_eq!(resp_long.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_executive_briefing_contract_and_burn_rate(
    ) -> Result<(), galleon_fleet_core::FleetError> {
        let tmp = tempfile::tempdir().map_err(galleon_fleet_core::FleetError::Io)?;
        let store = Arc::new(FleetStore::new(tmp.path()));

        // Populate mock collections
        store
            .save_collection("artifacts", b"[{\"id\":\"art-1\"},{\"id\":\"art-2\"}]")
            .await?;
        store
            .save_collection("crew", b"[{\"id\":\"c1\"},{\"id\":\"c2\"}]")
            .await?;
        store
            .save_collection(
                "approvals",
                b"[{\"id\":\"a1\",\"status\":\"pending\"},{\"id\":\"a2\",\"status\":\"pending\"}]",
            )
            .await?;
        store
            .save_collection(
                "treasuryLedger",
                b"[{\"id\":\"tr-1\",\"costUSD\":0.12},{\"id\":\"tr-2\",\"costUSD\":0.04}]",
            )
            .await?;

        let app = router_for_config(SystemApiConfig {
            gateway_port: 50052,
            local_address: "127.0.0.1".to_string(),
            store: store.clone(),
        });

        let req = Request::builder()
            .method("GET")
            .uri("/api/system/executive-briefing")
            .body(Body::empty())
            .expect("request builder");

        let resp = app.oneshot(req).await.expect("oneshot call");
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).expect("parse JSON response");
        let parsed_obj = parsed.as_object().expect("response must be object");

        // Verify contract against Go fixture
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/go-fleet/GET_api_system_executive_briefing.json");
        let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go fixture");
        let fixture_val: Value = serde_json::from_str(&fixture_str).expect("parse Go fixture");
        let fixture_obj = fixture_val.as_object().expect("fixture object");

        for key in fixture_obj.keys() {
            assert!(
                parsed_obj.contains_key(key),
                "Missing expected key from Go executive briefing fixture: {key}"
            );
        }

        // Verify fields
        assert_eq!(parsed["title"], "Sovereign Fleet Watch Report");
        assert_eq!(
            parsed["admiralStatus"],
            "Awaiting Captain — 2 approval(s) pending"
        );
        assert_eq!(
            parsed["activeAlerts"],
            json!(["2 pending Captain's Approval await signature"])
        );
        assert_eq!(parsed["treasuryBurnRate"], "$0.16");

        // Now test empty treasury ledger -> null
        let tmp_empty = tempfile::tempdir().map_err(galleon_fleet_core::FleetError::Io)?;
        let store_empty = Arc::new(FleetStore::new(tmp_empty.path()));
        let app_empty = router_for_config(SystemApiConfig {
            gateway_port: 50052,
            local_address: "127.0.0.1".to_string(),
            store: store_empty,
        });

        let req_empty = Request::builder()
            .method("GET")
            .uri("/api/system/executive-briefing")
            .body(Body::empty())
            .expect("request builder");

        let resp_empty = app_empty.oneshot(req_empty).await.expect("oneshot call");
        assert_eq!(resp_empty.status(), StatusCode::OK);
        let body_empty = resp_empty
            .into_body()
            .collect()
            .await
            .expect("read body")
            .to_bytes();
        let parsed_empty: Value = serde_json::from_slice(&body_empty).expect("parse JSON response");
        assert!(parsed_empty["treasuryBurnRate"].is_null());

        Ok(())
    }
}
