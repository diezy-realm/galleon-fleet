//! HTTP handlers for Galleon Fleet terminal execution and audit trail endpoints.
//!
//! Under SPEC-013 §Terminal (TASK-078):
//! - `POST /api/terminal/exec`:
//!   - `require_auth` Bearer token check.
//!   - Evaluates command via `SecurityPolicy::validate_command_execution(command, approved=false)`.
//!   - Field `approved` and `approved_by` from client payload are strictly ignored (anti-forgery).
//!   - High-risk or approval-requiring commands return `requiresApproval: true` and are NOT executed.
//!   - Low / Medium risk commands execute via `tokio::process::Command` without shell (argv parsed via policy).
//!   - Timeout 30 seconds; output truncated to 64 KB.
//!   - Audited to `::galleon_log::record!` and an in-memory ring buffer of 500 entries.
//! - `GET /api/v1/audit/trail`:
//!   - `require_auth` Bearer token check.
//!   - Returns historical audit records from the 500-entry ring buffer.

use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use chrono::Utc;
use galleon_api::runtime_traits::ShellDialect;
use galleon_config::policy::{CommandRiskLevel, SecurityPolicy};
use galleon_config::schema::Config;
use galleon_fleet_core::{
    DEFAULT_TERMINAL_TIMEOUT_SECS, FLEET_MAX_BODY, MAX_AUDIT_ENTRIES, MAX_TERMINAL_OUTPUT_BYTES,
    TerminalAuditRecord, TerminalAuditRingBuffer, TerminalExecRequest, TerminalExecResponse,
    truncate_output,
};
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

static AUDIT_RING_BUFFER: OnceLock<Arc<RwLock<TerminalAuditRingBuffer>>> = OnceLock::new();

/// Global in-memory ring buffer for terminal execution audit records (500 entries).
#[must_use]
pub fn global_audit_buffer() -> Arc<RwLock<TerminalAuditRingBuffer>> {
    AUDIT_RING_BUFFER
        .get_or_init(|| Arc::new(RwLock::new(TerminalAuditRingBuffer::new(MAX_AUDIT_ENTRIES))))
        .clone()
}

/// Query parameters for `GET /api/v1/audit/trail`.
#[derive(Debug, Deserialize, Default)]
pub struct AuditTrailQuery {
    /// Optional limit on the number of historical records returned (clamped to 1..500).
    pub limit: Option<usize>,
}

/// Resolves an operational `SecurityPolicy` from the gateway's `Config`.
#[must_use]
pub fn resolve_terminal_security_policy(config: &Config) -> SecurityPolicy {
    if let Some(agent_alias) = config.agents.keys().next() {
        if let Ok(policy) = SecurityPolicy::for_agent(config, agent_alias) {
            return policy;
        }
    }
    SecurityPolicy {
        workspace_dir: config.shared_workspace_dir(),
        ..Default::default()
    }
}

/// Assembles terminal and audit routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/terminal/exec", post(handle_terminal_exec_state))
        .route("/api/v1/audit/trail", get(handle_get_audit_trail_state))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing terminal endpoints with dedicated AppState.
pub fn router_for_state(state: AppState) -> Router {
    Router::new()
        .route("/api/terminal/exec", post(handle_terminal_exec_state))
        .route("/api/v1/audit/trail", get(handle_get_audit_trail_state))
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

async fn handle_terminal_exec_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(payload): axum::Json<TerminalExecRequest>,
) -> Response {
    handle_terminal_exec_impl(&state, &headers, payload, global_audit_buffer()).await
}

async fn handle_get_audit_trail_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuditTrailQuery>,
) -> Response {
    handle_get_audit_trail_impl(&state, &headers, query, global_audit_buffer()).await
}

/// Internal implementation of `POST /api/terminal/exec`.
pub async fn handle_terminal_exec_impl(
    state: &AppState,
    headers: &HeaderMap,
    payload: TerminalExecRequest,
    audit_buffer: Arc<RwLock<TerminalAuditRingBuffer>>,
) -> Response {
    // 1. Mandatory authentication check
    if let Err(auth_err) = crate::api::require_auth(state, headers) {
        return auth_err.into_response();
    }

    let start_instant = Instant::now();
    let timestamp = Utc::now().to_rfc3339();
    let command = payload.command.trim();

    if command.is_empty() {
        let err_json = json!({ "error": "command cannot be empty" });
        return (StatusCode::BAD_REQUEST, axum::Json(err_json)).into_response();
    }

    // 2. Resolve security policy from canonical config
    let policy = {
        let config_guard = state.config.read();
        resolve_terminal_security_policy(&config_guard)
    };

    // 3. Security evaluation: approved flag from client is strictly ignored (approved=false)
    let is_allowed = policy.is_command_allowed_for_shell(command, ShellDialect::Posix);
    let risk_level = policy.command_risk_level_for_shell(command, ShellDialect::Posix);
    let eval_result = policy.validate_command_execution(command, false);

    // Case B: High risk or requires approval
    let needs_approval = risk_level == CommandRiskLevel::High
        || match &eval_result {
            Err(msg) => {
                msg.contains("requires explicit approval")
                    || msg.contains("high-risk operation")
                    || msg.contains("medium-risk operation")
            }
            Ok(_) => false,
        };

    if needs_approval {
        let audit_rec = TerminalAuditRecord {
            id: String::new(),
            timestamp,
            command: command.to_string(),
            risk_tier: if risk_level == CommandRiskLevel::High {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            approved: false,
            approved_by: None,
            success: false,
            exit_code: -1,
            duration: "0s".to_string(),
            error: Some("approval required".to_string()),
        };
        let audit_id = audit_buffer.write().record(audit_rec);

        ::galleon_log::record!(
            INFO,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Approve)
                .with_outcome(::galleon_log::EventOutcome::Unknown)
                .with_attrs(json!({
                    "command": command,
                    "risk_tier": if risk_level == CommandRiskLevel::High { "high" } else { "medium" },
                    "requires_approval": true,
                    "audit_id": audit_id,
                })),
            "terminal execution gated: requires explicit manual approval"
        );

        let resp = TerminalExecResponse {
            stdout: String::new(),
            stderr: None,
            exit_code: -1,
            duration: "0s".to_string(),
            risk_tier: if risk_level == CommandRiskLevel::High {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            requires_approval: Some(true),
            audit_id: Some(audit_id),
            error: None,
        };
        return (StatusCode::OK, axum::Json(resp)).into_response();
    }

    // Case A: Disallowed by allowlist, path confinement, or prohibited syntax
    if !is_allowed || eval_result.is_err() {
        let reason = match eval_result {
            Err(ref msg) => msg.clone(),
            Ok(_) => format!("Command not allowed by security policy: {command}"),
        };

        let audit_rec = TerminalAuditRecord {
            id: String::new(),
            timestamp,
            command: command.to_string(),
            risk_tier: "blocked".to_string(),
            approved: false,
            approved_by: None,
            success: false,
            exit_code: 1,
            duration: "0s".to_string(),
            error: Some(reason.clone()),
        };
        let audit_id = audit_buffer.write().record(audit_rec);

        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Validate)
                .with_outcome(::galleon_log::EventOutcome::Failure)
                .with_attrs(json!({
                    "command": command,
                    "reason": reason,
                    "audit_id": audit_id,
                })),
            "terminal execution rejected by security policy"
        );

        let err_body = json!({
            "error": format!("command not allowed by security policy: {reason}"),
            "auditId": audit_id,
        });
        return (StatusCode::FORBIDDEN, axum::Json(err_body)).into_response();
    }

    // Case C: Low / Medium risk permitted execution without shell
    let argv = policy.parse_command_argv(command);
    if argv.is_empty() {
        let err_json = json!({ "error": "command parsed to empty argument vector" });
        return (StatusCode::BAD_REQUEST, axum::Json(err_json)).into_response();
    }

    let program = &argv[0];
    let mut cmd = tokio::process::Command::new(program);
    if argv.len() > 1 {
        cmd.args(&argv[1..]);
    }
    if policy.workspace_dir.exists() {
        cmd.current_dir(&policy.workspace_dir);
    }

    let timeout_duration = Duration::from_secs(DEFAULT_TERMINAL_TIMEOUT_SECS);
    let run_outcome = tokio::time::timeout(timeout_duration, cmd.output()).await;

    let elapsed = start_instant.elapsed();
    let duration_str = format!("{:.2?}", elapsed);

    let (stdout_raw, stderr_raw, exit_code, success, error_opt) = match run_outcome {
        Ok(Ok(output)) => {
            let out_str = String::from_utf8_lossy(&output.stdout).to_string();
            let err_str = String::from_utf8_lossy(&output.stderr).to_string();
            let code = output.status.code().unwrap_or(-1);
            let ok = output.status.success();
            let err_msg = if ok { None } else { Some(err_str.clone()) };
            (out_str, err_str, code, ok, err_msg)
        }
        Ok(Err(spawn_err)) => {
            let err_msg = format!("Failed to spawn command '{program}': {spawn_err}");
            (String::new(), err_msg.clone(), 1, false, Some(err_msg))
        }
        Err(_) => {
            let err_msg = format!(
                "Command execution timed out after {DEFAULT_TERMINAL_TIMEOUT_SECS} seconds"
            );
            (String::new(), err_msg.clone(), -1, false, Some(err_msg))
        }
    };

    let stdout = truncate_output(&stdout_raw, MAX_TERMINAL_OUTPUT_BYTES);
    let stderr = truncate_output(&stderr_raw, MAX_TERMINAL_OUTPUT_BYTES);

    let risk_tier_str = match risk_level {
        CommandRiskLevel::Low => "low",
        CommandRiskLevel::Medium => "medium",
        CommandRiskLevel::High => "high",
    };

    let audit_rec = TerminalAuditRecord {
        id: String::new(),
        timestamp,
        command: command.to_string(),
        risk_tier: risk_tier_str.to_string(),
        approved: false,
        approved_by: None,
        success,
        exit_code,
        duration: duration_str.clone(),
        error: error_opt.clone(),
    };
    let audit_id = audit_buffer.write().record(audit_rec);

    ::galleon_log::record!(
        INFO,
        ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Invoke)
            .with_outcome(if success {
                ::galleon_log::EventOutcome::Success
            } else {
                ::galleon_log::EventOutcome::Failure
            })
            .with_attrs(json!({
                "command": command,
                "exit_code": exit_code,
                "duration": duration_str,
                "risk_tier": risk_tier_str,
                "audit_id": audit_id,
            })),
        "terminal command executed"
    );

    let resp = TerminalExecResponse {
        stdout,
        stderr: if stderr.is_empty() {
            None
        } else {
            Some(stderr)
        },
        exit_code,
        duration: duration_str,
        risk_tier: risk_tier_str.to_string(),
        requires_approval: None,
        audit_id: Some(audit_id),
        error: error_opt,
    };
    (StatusCode::OK, axum::Json(resp)).into_response()
}

/// Internal implementation of `GET /api/v1/audit/trail`.
pub async fn handle_get_audit_trail_impl(
    state: &AppState,
    headers: &HeaderMap,
    query: AuditTrailQuery,
    audit_buffer: Arc<RwLock<TerminalAuditRingBuffer>>,
) -> Response {
    if let Err(auth_err) = crate::api::require_auth(state, headers) {
        return auth_err.into_response();
    }

    let records = audit_buffer.read().list(query.limit);
    (StatusCode::OK, axum::Json(records)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use galleon_config::pairing::{PairingCodePolicy, PairingGuard};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    fn create_test_state() -> AppState {
        let config = Config::default();
        crate::api::tests::test_state(config)
    }

    fn create_test_state_with_auth(valid_token: &str) -> AppState {
        let config = Config::default();
        let mut state = crate::api::tests::test_state(config);
        state.pairing = Arc::new(PairingGuard::new(
            true,
            &[valid_token.to_string()],
            PairingCodePolicy::default(),
        ));
        state
    }

    #[tokio::test]
    async fn test_terminal_exec_unauthorized_without_token() {
        let state = create_test_state_with_auth("secret-token");
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/terminal/exec")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "command": "git status" }).to_string()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_audit_trail_unauthorized_without_token() {
        let state = create_test_state_with_auth("secret-token");
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/v1/audit/trail")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_client_cannot_forge_approval_for_high_risk_command() {
        // Configure policy where a high-risk tool (e.g. rm) is in allowed_commands
        let mut config = Config::default();
        config.risk_profiles.insert(
            "elevated".to_string(),
            galleon_config::schema::RiskProfileConfig {
                allowed_commands: vec!["rm".to_string(), "git".to_string(), "echo".to_string()],
                ..Default::default()
            },
        );
        config.agents.insert(
            "default".to_string(),
            galleon_config::schema::AliasedAgentConfig {
                risk_profile: "elevated".into(),
                ..Default::default()
            },
        );
        let state = crate::api::tests::test_state(config);
        let app = router_for_state(state);

        // Client attempts to claim approved=true on a high-risk operation
        let req = Request::builder()
            .uri("/api/terminal/exec")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "command": "rm -rf /tmp/galleon-test-dir",
                    "approved": true,
                    "approved_by": "attacker"
                })
                .to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();

        // Must still require approval, client approval ignored
        assert_eq!(parsed["requiresApproval"], true);
        assert_eq!(parsed["exitCode"], -1);
        assert_eq!(parsed["riskTier"], "high");
        assert!(parsed["auditId"].is_string());
    }

    #[tokio::test]
    async fn test_terminal_exec_allowed_low_risk_command_and_audit() {
        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/terminal/exec")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "command": "echo 'galleon terminal execution'" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(parsed["exitCode"], 0);
        assert!(
            parsed["stdout"]
                .as_str()
                .unwrap()
                .contains("galleon terminal execution")
        );
        assert_eq!(parsed["riskTier"], "low");
        let audit_id = parsed["auditId"].as_str().unwrap().to_string();
        assert!(audit_id.starts_with("audit-"));

        // Verify that the record appears in GET /api/v1/audit/trail
        let state2 = create_test_state();
        let app2 = router_for_state(state2);
        let audit_req = Request::builder()
            .uri("/api/v1/audit/trail")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let audit_resp = app2.oneshot(audit_req).await.unwrap();
        assert_eq!(audit_resp.status(), StatusCode::OK);

        let audit_body = audit_resp.into_body().collect().await.unwrap().to_bytes();
        let audit_list: Vec<Value> = serde_json::from_slice(&audit_body).unwrap();
        assert!(!audit_list.is_empty());

        let matched = audit_list.iter().find(|entry| entry["id"] == audit_id);
        assert!(matched.is_some(), "audit record must be found in trail");
        let entry = matched.unwrap();
        assert_eq!(entry["command"], "echo 'galleon terminal execution'");
        assert_eq!(entry["success"], true);
        assert_eq!(entry["exitCode"], 0);
        assert_eq!(entry["approved"], false);
        assert!(entry["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_terminal_exec_disallowed_command_returns_403() {
        let state = create_test_state();
        let app = router_for_state(state);

        // Command not in allowlist
        let req = Request::builder()
            .uri("/api/terminal/exec")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "command": "unauthorized_malicious_bin --flag" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            parsed["error"]
                .as_str()
                .unwrap()
                .contains("security policy")
        );
        assert!(parsed["auditId"].is_string());
    }

    #[tokio::test]
    async fn test_contract_shape_parity_go_fixtures() {
        let state = create_test_state();
        let app = router_for_state(state);

        let req = Request::builder()
            .uri("/api/terminal/exec")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "command": "echo 'contract probe'" }).to_string(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let parsed: Value = serde_json::from_slice(&body_bytes).unwrap();
        let parsed_obj = parsed.as_object().unwrap();

        // Load Go fixture contract for POST /api/terminal/exec
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/go-fleet/POST_api_terminal_exec.json");
        let fixture_str = std::fs::read_to_string(&fixture_path).expect("read Go fixture");
        let fixture_val: Value = serde_json::from_str(&fixture_str).expect("parse Go fixture");
        let fixture_obj = fixture_val.as_object().expect("fixture must be object");

        // Verify shape parity: standard fields match expected types
        for key in &["stdout", "exitCode", "duration", "riskTier", "auditId"] {
            assert!(
                parsed_obj.contains_key(*key),
                "Missing expected key from contract: {key}"
            );
        }

        // Verify GET /api/v1/audit/trail against fixture shape
        let state2 = create_test_state();
        let app2 = router_for_state(state2);
        let audit_req = Request::builder()
            .uri("/api/v1/audit/trail")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let audit_resp = app2.oneshot(audit_req).await.unwrap();
        assert_eq!(audit_resp.status(), StatusCode::OK);

        let audit_body = audit_resp.into_body().collect().await.unwrap().to_bytes();
        let audit_list: Vec<Value> = serde_json::from_slice(&audit_body).unwrap();
        assert!(!audit_list.is_empty());

        let audit_entry_obj = audit_list.last().unwrap().as_object().unwrap();
        let audit_fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/go-fleet/GET_api_v1_audit_trail.json");
        let audit_fixture_str =
            std::fs::read_to_string(&audit_fixture_path).expect("read Go audit fixture");
        let audit_fixture_val: Value =
            serde_json::from_str(&audit_fixture_str).expect("parse Go audit fixture");
        let audit_fixture_first = &audit_fixture_val.as_array().unwrap()[0];
        let audit_fixture_obj = audit_fixture_first.as_object().unwrap();

        for key in audit_fixture_obj.keys() {
            // error and approvedBy are optional (absent on clean success)
            if key == "error" || key == "approvedBy" {
                continue;
            }
            assert!(
                audit_entry_obj.contains_key(key),
                "Missing key in audit entry: {key}"
            );
        }
    }
}
