//! HTTP handlers for Galleon Fleet system endpoints (`/api/system/*`, `/api/engine/processes`).
//!
//! Under SPEC-013 §Model Data (TASK-073):
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
//! - `GET /api/engine/processes`:
//!   - Uses `sysinfo` 0.36 to enumerate processes.
//!   - Filters strictly for processes whose name starts with `galleon*`.
//!   - Returns array of `{ pid, name, memory, cpu }`.

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use galleon_fleet_core::FLEET_MAX_BODY;
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

/// Process item representation for `/api/engine/processes`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineProcessItem {
    pub cpu: f64,
    pub memory: u64,
    pub name: String,
    pub pid: u32,
}

/// Standalone configuration for testing system network endpoints without AppState.
#[derive(Clone, Debug)]
pub struct SystemApiConfig {
    pub gateway_port: u16,
    pub local_address: String,
}

impl Default for SystemApiConfig {
    fn default() -> Self {
        Self {
            gateway_port: 50052,
            local_address: "127.0.0.1".to_string(),
        }
    }
}

/// Assembles system routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/system/metrics", get(handle_get_system_metrics))
        .route("/api/system/network", get(handle_get_system_network_state))
        .route("/api/engine/processes", get(handle_get_engine_processes))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing with dedicated configuration.
pub fn router_for_config(config: SystemApiConfig) -> Router {
    Router::new()
        .route("/api/system/metrics", get(handle_get_system_metrics))
        .route(
            "/api/system/network",
            get({
                let config = config.clone();
                move || async move {
                    handle_get_system_network_impl(config.gateway_port, config.local_address)
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
}
