//! SOP authoring surface for the web node editor.
//!
//! HTTP twin of the daemon's `sops/*` RPC methods, backed by the decoupled
//! `clawcrew_api::sop::SopRunner` interface. All routes require gateway
//! auth. Draft endpoints (`wire-draft`, `graph-draft`) are pure: they
//! transform the submitted SOP and never touch disk.

use std::net::SocketAddr;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use super::AppState;
use super::api::require_auth;
use clawcrew_api::sop::{ApprovalDecision, ApprovalPrincipal, Sop, SopRunStatus, SopRunSummary};

#[derive(Debug, Default, serde::Deserialize)]
pub struct RunsQuery {
    #[serde(default)]
    pub sop: Option<String>,
}

pub async fn handle_sops_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.list_sops() {
        Ok(sops) => Json(serde_json::json!({ "sops": sops })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn handle_sop_trigger_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.get_trigger_sources() {
        Ok(registry) => Json(registry).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Body for `POST /api/tools/param-options`: resolve selectable values
/// for a domain-typed tool parameter. `args` carries sibling arguments
/// already chosen so cascading domains (e.g. peer targets narrowing on
/// a channel) can filter.
#[derive(serde::Deserialize)]
pub struct ParamOptionsBody {
    pub domain: clawcrew_api::tool::OptionDomain,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub args: serde_json::Value,
}

pub async fn handle_tools_param_options(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ParamOptionsBody>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let config = state.config.read();
    let agent_alias = body
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(str::to_string)
        .or_else(|| config.agents.keys().min().cloned())
        .unwrap_or_default();

    let entries = clawcrew_tools::param_options::resolve_options(
        body.domain,
        &config,
        &agent_alias,
        &body.args,
        &[],
    );
    Json(serde_json::json!({ "options": entries })).into_response()
}

pub async fn handle_sop_graph(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.get_sop_graph(&name) {
        Ok(Some(graph)) => Json(graph).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("SOP '{name}' not found") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Body for `POST /api/sops/{name}/run`: fire a Manual trigger. `payload` is
/// an optional JSON string handed to the run as the step-1 input. Deserializes
/// from the shared `SopRunRequest` shape (minus `name`, which comes from the
/// path).
#[derive(serde::Deserialize)]
pub struct SopRunBody {
    #[serde(default)]
    pub payload: Option<String>,
}

/// Fire a Manual run for the named SOP and return its `run_id`.
pub async fn handle_sop_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(body): Json<SopRunBody>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    if let Some(payload) = body.payload.as_deref()
        && !payload.trim().is_empty()
        && serde_json::from_str::<serde_json::Value>(payload).is_err()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "payload is not valid JSON" })),
        )
            .into_response();
    }

    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };

    match runner.start_sop(&name).await {
        Ok((code, resp)) => (
            StatusCode::from_u16(code).unwrap_or(StatusCode::OK),
            Json(resp),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn handle_sop_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<RunsQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.list_runs(query.sop.as_deref()) {
        Ok(runs) => Json(serde_json::json!({ "runs": runs })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn handle_sop_run_overlay(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((name, run_id)): Path<(String, String)>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.get_run_overlay(&name, &run_id) {
        Ok(Some(overlay)) => Json(overlay).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Run '{run_id}' not found") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Resolve a gated live run. Body carries the raw `ApprovalDecision` wire
/// value. Waiting approvals and deterministic checkpoints both resolve through
/// the broker-backed chokepoint with an HTTP principal, so named approval
/// policies, membership, and quorum are enforced before a gate or checkpoint can
/// clear. Returns the refreshed overlay.
pub async fn handle_sop_decide(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((name, run_id)): Path<(String, String)>,
    Json(decision_value): Json<serde_json::Value>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    // Derive the transport-authenticated approval subject (the paired-token hash)
    // from the validated bearer, mirroring the /admin/sop approval route's
    // `authorize`, so the broker can enforce a required-group / quorum policy on this
    // authoring surface instead of resolving an anonymous `http(None)` past it. Gated
    // on `require_pairing`: when pairing is OFF every token is a no-op pass-through, so
    // deriving an identity from an unauthenticated header would let any caller fabricate
    // an approval subject - fall back to `http(None)` (which fails a required-group
    // policy closed) in that mode, matching `authorize`.
    let subject = state
        .pairing
        .require_pairing()
        .then(|| crate::api::extract_bearer_token(&headers))
        .flatten()
        .and_then(|t| state.pairing.authenticate_and_hash(t));
    let principal = ApprovalPrincipal::http(subject);
    let decision: ApprovalDecision = match serde_json::from_value(decision_value) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("decision is not a valid approval decision: {e}")
                })),
            )
                .into_response();
        }
    };
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };

    match runner.resolve_author_approval(&name, &run_id, decision, principal).await {
        Ok((code, body)) => (
            StatusCode::from_u16(code).unwrap_or(StatusCode::OK),
            Json(body),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct SopCancelBody {
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct SopCancelResponse {
    run_id: String,
    sop_name: String,
    status: SopRunStatus,
    already_terminal: bool,
    run: SopRunSummary,
}

fn authorize_sop_cancel(
    state: &AppState,
    peer: &SocketAddr,
    headers: &HeaderMap,
) -> Result<ApprovalPrincipal, Box<Response>> {
    if state.pairing.require_pairing() {
        require_auth(state, headers).map_err(|error| Box::new(error.into_response()))?;
        let subject = super::api::extract_bearer_token(headers)
            .and_then(|token| state.pairing.authenticate_and_hash(token));
        return Ok(ApprovalPrincipal::http(subject));
    }
    let effective_client_ip = if state.trust_forwarded_headers {
        super::forwarded_client_ip(headers)
    } else {
        Some(peer.ip())
    };
    if effective_client_ip.is_some_and(|ip| ip.is_loopback()) {
        return Ok(ApprovalPrincipal::cli(None));
    }
    Err(Box::new(
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Remote SOP cancellation requires gateway pairing. Enable \
                          gateway.require_pairing and pair first, or call from localhost."
            })),
        )
            .into_response(),
    ))
}

/// POST /api/sops/{name}/runs/{run_id}/cancel - operator cancellation for a
/// running SOP. This is a SAFE cancel: the in-flight step keeps running to
/// its own completion and the run stops at the next step boundary, not
/// mid-step. Idempotent and race-safe against normal completion - a run that
/// is already terminal (previously cancelled, or it finished or failed
/// first) is reported as-is with `already_terminal: true`, never a second
/// cancellation or an error.
pub async fn handle_sop_cancel(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path((name, run_id)): Path<(String, String)>,
    body: Bytes,
) -> Response {
    let principal = match authorize_sop_cancel(&state, &peer, &headers) {
        Ok(principal) => principal,
        Err(response) => return *response,
    };
    // The body is optional (`{ "reason": ... }` or nothing at all), unlike
    // `handle_sop_decide`'s required decision payload, so an empty body is a
    // valid no-reason request rather than a parse error.
    let reason = if body.is_empty() {
        None
    } else {
        match serde_json::from_slice::<SopCancelBody>(&body) {
            Ok(b) => b.reason,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("invalid cancel request body: {e}")
                    })),
                )
                    .into_response();
            }
        }
    };

    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };

    match runner.cancel_run(&name, &run_id, reason.as_deref(), principal) {
        Ok((code, resp)) => (
            StatusCode::from_u16(code).unwrap_or(StatusCode::OK),
            Json(resp),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn handle_sop_full(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.get_sop(&name) {
        Ok(Some(sop)) => Json(sop).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("SOP '{name}' not found") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn handle_sop_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(sop): Json<Sop>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.create_sop(&sop) {
        Ok(()) => Json(serde_json::json!({ "created": sop.name })).into_response(),
        Err(e) => {
            let code = if e.to_string().contains("already exists") {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            (code, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}

pub async fn handle_sop_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(mut sop): Json<Sop>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    if !sop.name.is_empty() && sop.name != name {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!(
                    "body name '{}' does not match URL name '{name}'",
                    sop.name
                )
            })),
        )
            .into_response();
    }
    sop.name = name.clone();
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.save_sop(&name, &sop) {
        Ok(()) => Json(serde_json::json!({ "saved": sop.name })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn handle_sop_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.delete_sop(&name) {
        Ok(()) => Json(serde_json::json!({ "deleted": name })).into_response(),
        Err(e) => {
            let code = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (code, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}

/// Body for `wire-draft`: a full draft SOP plus one edit to apply.
#[derive(serde::Deserialize)]
pub struct WireDraftRequest {
    pub sop: Sop,
    pub edit: clawcrew_sop_graph::WireEdit,
}

pub async fn handle_sop_wire_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<WireDraftRequest>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    let edit_val = match serde_json::to_value(&req.edit) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    match runner.wire_draft(&req.sop, &edit_val) {
        Ok(res) => Json(res).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Body for `graph-draft`: a full draft SOP to project without saving.
#[derive(serde::Deserialize)]
pub struct GraphDraftRequest {
    pub sop: Sop,
}

pub async fn handle_sop_graph_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<GraphDraftRequest>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(runner) = state.sop_runner.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
        )
            .into_response();
    };
    match runner.graph_draft(&req.sop) {
        Ok(res) => Json(res).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn handle_sop_graph_legend(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    Json(clawcrew_sop_graph::GraphLegend::canonical()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::extract::{Path, State};
    use axum::http::{HeaderValue, header};
    use clawcrew_api::sop::{BrokerOutcome, ResolveOutcome, SopRunner};
    use clawcrew_config::pairing::PairingGuard;
    use http_body_util::BodyExt;

    #[derive(Default)]
    struct MockAuthorSopRunner {
        sop_name: String,
        runs: Mutex<HashMap<String, SopRunStatus>>,
        member_hash: String,
        required_quorum: usize,
        approved_voters: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl SopRunner for MockAuthorSopRunner {
        fn list_sops(&self) -> anyhow::Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }
        fn get_trigger_sources(&self) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        fn get_sop_graph(&self, _name: &str) -> anyhow::Result<Option<serde_json::Value>> {
            Ok(None)
        }
        fn list_runs(&self, _sop_name: Option<&str>) -> anyhow::Result<Vec<SopRunSummary>> {
            let guard = self.runs.lock().unwrap();
            let summaries = guard
                .iter()
                .map(|(id, status)| SopRunSummary {
                    run_id: id.clone(),
                    sop_name: self.sop_name.clone(),
                    status: *status,
                    current_step: 1,
                    total_steps: 1,
                    started_at: "2026-10-17T00:00:00Z".into(),
                    completed_at: None,
                    trigger_source: "manual".into(),
                    active: !matches!(
                        status,
                        SopRunStatus::Cancelled | SopRunStatus::Completed | SopRunStatus::Failed
                    ),
                })
                .collect();
            Ok(summaries)
        }
        fn get_run_overlay(
            &self,
            name: &str,
            run_id: &str,
        ) -> anyhow::Result<Option<serde_json::Value>> {
            let guard = self.runs.lock().unwrap();
            if let Some(status) = guard.get(run_id) {
                Ok(Some(serde_json::json!({
                    "run_id": run_id,
                    "sop_name": name,
                    "status": status,
                    "waiting": matches!(status, SopRunStatus::WaitingApproval | SopRunStatus::PausedCheckpoint),
                })))
            } else {
                Ok(None)
            }
        }
        fn get_pending_runs(&self) -> anyhow::Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }
        fn get_metrics(&self) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        fn subscribe_run_changes(
            &self,
        ) -> anyhow::Result<(
            Vec<SopRunSummary>,
            Option<tokio::sync::broadcast::Receiver<SopRunSummary>>,
        )> {
            Ok((vec![], None))
        }
        async fn start_sop(&self, _name: &str) -> anyhow::Result<(u16, serde_json::Value)> {
            Ok((200, serde_json::json!({ "run_id": "test-run" })))
        }
        async fn resolve_approval(
            &self,
            _run_id: &str,
            _decision: ApprovalDecision,
            _principal: ApprovalPrincipal,
        ) -> anyhow::Result<BrokerOutcome> {
            Ok(BrokerOutcome::Resolved(ResolveOutcome::Resumed))
        }
        async fn resolve_author_approval(
            &self,
            name: &str,
            run_id: &str,
            _decision: ApprovalDecision,
            principal: ApprovalPrincipal,
        ) -> anyhow::Result<(u16, serde_json::Value)> {
            if name != self.sop_name {
                return Ok((
                    400,
                    serde_json::json!({
                        "error": format!("run '{run_id}' belongs to SOP '{}', not '{name}'", self.sop_name)
                    }),
                ));
            }
            let mut guard = self.runs.lock().unwrap();
            let Some(status) = guard.get_mut(run_id) else {
                return Ok((
                    404,
                    serde_json::json!({ "error": format!("Run {run_id} not found") }),
                ));
            };
            if !self.member_hash.is_empty()
                && principal.identity.as_deref() != Some(&self.member_hash)
            {
                return Ok((
                    403,
                    serde_json::json!({ "error": "not authorized: requires group 'release'" }),
                ));
            }
            if self.required_quorum > 1 {
                let mut voters = self.approved_voters.lock().unwrap();
                if let Some(id) = principal.identity {
                    if !voters.contains(&id) {
                        voters.push(id);
                    }
                }
                if voters.len() < self.required_quorum {
                    return Ok((
                        202,
                        serde_json::json!({
                            "run_id": run_id,
                            "sop_name": name,
                            "waiting": true,
                        }),
                    ));
                }
            }
            *status = SopRunStatus::Running;
            Ok((
                200,
                serde_json::json!({
                    "run_id": run_id,
                    "sop_name": name,
                    "waiting": false,
                }),
            ))
        }
        fn cancel_run(
            &self,
            name: &str,
            run_id: &str,
            _reason: Option<&str>,
            _principal: ApprovalPrincipal,
        ) -> anyhow::Result<(u16, serde_json::Value)> {
            if run_id == "nonexistent" {
                return Ok((
                    404,
                    serde_json::json!({ "error": format!("Run {run_id} not found") }),
                ));
            }
            if name != self.sop_name {
                return Ok((
                    400,
                    serde_json::json!({
                        "error": format!("run '{run_id}' belongs to SOP '{}', not '{name}'", self.sop_name)
                    }),
                ));
            }
            let mut guard = self.runs.lock().unwrap();
            let Some(status) = guard.get_mut(run_id) else {
                return Ok((
                    404,
                    serde_json::json!({ "error": format!("Run {run_id} not found") }),
                ));
            };
            let (code, already_terminal) = match *status {
                SopRunStatus::Cancelled => (200, true),
                SopRunStatus::CancelRequested => (202, false),
                SopRunStatus::WaitingApproval | SopRunStatus::PausedCheckpoint => {
                    *status = SopRunStatus::Cancelled;
                    (200, false)
                }
                _ => {
                    *status = SopRunStatus::CancelRequested;
                    (202, false)
                }
            };
            let summary = SopRunSummary {
                run_id: run_id.to_string(),
                sop_name: name.to_string(),
                status: *status,
                current_step: 1,
                total_steps: 1,
                started_at: "2026-10-17T00:00:00Z".into(),
                completed_at: None,
                trigger_source: "manual".into(),
                active: !matches!(status, SopRunStatus::Cancelled),
            };
            Ok((
                code,
                serde_json::json!({
                    "run_id": run_id,
                    "sop_name": name,
                    "status": *status,
                    "already_terminal": already_terminal,
                    "run": summary,
                }),
            ))
        }
        fn get_sop(&self, name: &str) -> anyhow::Result<Option<Sop>> {
            if name == self.sop_name {
                Ok(Some(Sop {
                    name: name.to_string(),
                    ..Default::default()
                }))
            } else {
                Ok(None)
            }
        }
        fn create_sop(&self, _sop: &Sop) -> anyhow::Result<()> {
            Ok(())
        }
        fn save_sop(&self, _name: &str, _sop: &Sop) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_sop(&self, _name: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn wire_draft(
            &self,
            _sop: &Sop,
            _edit: &serde_json::Value,
        ) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        fn graph_draft(&self, _sop: &Sop) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        fn has_matching_webhook_sop(&self, _path: &str) -> anyhow::Result<bool> {
            Ok(true)
        }
        async fn dispatch_webhook_sop(
            &self,
            _path: &str,
            _payload: Option<&str>,
        ) -> anyhow::Result<Option<clawcrew_api::sop::WebhookDispatchResult>> {
            Ok(Some(clawcrew_api::sop::WebhookDispatchResult {
                blocked_only: false,
                results: vec![],
            }))
        }
    }

    fn bearer(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers
    }

    fn loopback_peer() -> ConnectInfo<SocketAddr> {
        ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 42_000)))
    }

    fn remote_peer() -> ConnectInfo<SocketAddr> {
        ConnectInfo(SocketAddr::from(([203, 0, 113, 7], 42_000)))
    }

    fn authoring_state_with_policied_gate(
        member_token: &str,
        other_token: &str,
    ) -> (tempfile::TempDir, AppState, String) {
        let tmp = tempfile::tempdir().unwrap();
        let member_hash = PairingGuard::token_hash(member_token);
        let run_id = "deploy-run-1".to_string();

        let mut runs = HashMap::new();
        runs.insert(run_id.clone(), SopRunStatus::WaitingApproval);

        let runner = Arc::new(MockAuthorSopRunner {
            sop_name: "deploy".into(),
            runs: Mutex::new(runs),
            member_hash,
            required_quorum: 1,
            approved_voters: Mutex::new(Vec::new()),
        });

        let mut config = clawcrew_config::schema::Config {
            config_path: tmp.path().join("config.toml"),
            ..clawcrew_config::schema::Config::default()
        };
        config.gateway.allow_remote_admin = true;
        let mut state = crate::api::test_state(config);
        state.sop_runner = Some(runner);
        state.pairing = Arc::new(PairingGuard::new(
            true,
            &[member_token.to_string(), other_token.to_string()],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));
        (tmp, state, run_id)
    }

    fn authoring_state_with_quorum_gate(
        first_member: &str,
        second_member: &str,
        quorum: usize,
    ) -> (tempfile::TempDir, AppState, String) {
        let tmp = tempfile::tempdir().unwrap();
        let first_hash = PairingGuard::token_hash(first_member);
        let run_id = "deploy-run-1".to_string();

        let mut runs = HashMap::new();
        runs.insert(run_id.clone(), SopRunStatus::WaitingApproval);

        let runner = Arc::new(MockAuthorSopRunner {
            sop_name: "deploy".into(),
            runs: Mutex::new(runs),
            member_hash: first_hash,
            required_quorum: quorum,
            approved_voters: Mutex::new(Vec::new()),
        });

        let mut config = clawcrew_config::schema::Config {
            config_path: tmp.path().join("config.toml"),
            ..clawcrew_config::schema::Config::default()
        };
        config.gateway.allow_remote_admin = true;
        let mut state = crate::api::test_state(config);
        state.sop_runner = Some(runner);
        state.pairing = Arc::new(PairingGuard::new(
            true,
            &[first_member.to_string(), second_member.to_string()],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));
        (tmp, state, run_id)
    }

    fn authoring_state_with_running_run(token: &str) -> (AppState, String) {
        let (_tmp, state, run_id) = authoring_state_with_running_run_sop("deploy", token);
        (state, run_id)
    }

    fn authoring_state_with_running_run_sop(
        sop_name: &str,
        token: &str,
    ) -> (tempfile::TempDir, AppState, String) {
        let tmp = tempfile::tempdir().unwrap();
        let run_id = format!("{sop_name}-run-1");

        let mut runs = HashMap::new();
        runs.insert(run_id.clone(), SopRunStatus::Running);

        let runner = Arc::new(MockAuthorSopRunner {
            sop_name: sop_name.into(),
            runs: Mutex::new(runs),
            member_hash: String::new(),
            required_quorum: 1,
            approved_voters: Mutex::new(Vec::new()),
        });

        let mut config = clawcrew_config::schema::Config {
            config_path: tmp.path().join("config.toml"),
            ..clawcrew_config::schema::Config::default()
        };
        config.gateway.allow_remote_admin = true;
        let mut state = crate::api::test_state(config);
        state.sop_runner = Some(runner);
        state.pairing = Arc::new(PairingGuard::new(
            true,
            &[token.to_string()],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));
        (tmp, state, run_id)
    }

    fn run_status(state: &AppState, run_id: &str) -> Option<SopRunStatus> {
        state
            .sop_runner
            .as_ref()
            .unwrap()
            .list_runs(None)
            .unwrap()
            .into_iter()
            .find(|r| r.run_id == run_id)
            .map(|r| r.status)
    }

    #[tokio::test]
    async fn authoring_decide_enforces_broker_policy_membership() {
        let member = "member-token";
        let outsider = "outsider-token";
        let (_tmp, state, run_id) = authoring_state_with_policied_gate(member, outsider);

        let resp = handle_sop_decide(
            State(state.clone()),
            bearer(outsider),
            Path(("deploy".to_string(), run_id.clone())),
            Json(serde_json::json!("approve")),
        )
        .await;

        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "a paired non-member must not clear a policied authoring gate"
        );
        let status = run_status(&state, &run_id);
        assert_eq!(status, Some(SopRunStatus::WaitingApproval));
    }

    #[tokio::test]
    async fn authoring_decide_pending_quorum_returns_overlay_shape() {
        let first_member = "member-token-1";
        let second_member = "member-token-2";
        let (_tmp, state, run_id) = authoring_state_with_quorum_gate(first_member, second_member, 2);

        let resp = handle_sop_decide(
            State(state.clone()),
            bearer(first_member),
            Path(("deploy".to_string(), run_id.clone())),
            Json(serde_json::json!("approve")),
        )
        .await;

        assert_eq!(
            resp.status(),
            StatusCode::ACCEPTED,
            "the first quorum member should leave the run pending"
        );
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json.get("run_id").and_then(|value| value.as_str()),
            Some(run_id.as_str())
        );
        assert_eq!(
            json.get("sop_name").and_then(|value| value.as_str()),
            Some("deploy")
        );
        assert_eq!(
            json.get("waiting").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(
            json.get("outcome").is_none(),
            "pending quorum responses must keep the RunOverlay shape, got {json:?}"
        );
    }

    #[tokio::test]
    async fn authoring_decide_rejects_run_id_from_different_sop_before_broker_resolution() {
        let token = "member-token";
        let (_tmp, state, run_id) = authoring_state_with_running_run_sop("deploy-b", token);

        let resp = handle_sop_decide(
            State(state.clone()),
            bearer(token),
            Path(("deploy-a".to_string(), run_id.clone())),
            Json(serde_json::json!("approve")),
        )
        .await;

        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "a path SOP must not resolve a run owned by another SOP"
        );
        assert_eq!(run_status(&state, &run_id), Some(SopRunStatus::Running));
    }

    #[tokio::test]
    async fn authoring_cancel_active_run_returns_requested_and_keeps_it_active() {
        let token = "cancel-token";
        let (state, run_id) = authoring_state_with_running_run(token);

        let resp = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            bearer(token),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "cancel_requested");
        assert_eq!(json["run"]["status"], "cancel_requested");
        assert_eq!(json["run"]["active"], true);
        assert_eq!(json["already_terminal"], false);
        assert_eq!(json["run_id"], run_id.as_str());

        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::CancelRequested)
        );
    }

    #[tokio::test]
    async fn authoring_cancel_is_idempotent_on_a_second_request() {
        let token = "cancel-token";
        let (state, run_id) = authoring_state_with_running_run(token);

        let first = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            bearer(token),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;
        assert_eq!(first.status(), StatusCode::ACCEPTED);

        let second = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            bearer(token),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::from(serde_json::to_vec(&serde_json::json!({ "reason": "again" })).unwrap()),
        )
        .await;

        assert_eq!(
            second.status(),
            StatusCode::ACCEPTED,
            "a repeat cancellation request must be idempotent"
        );
        let body = second.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "cancel_requested");
        assert_eq!(json["already_terminal"], false);
    }

    #[tokio::test]
    async fn authoring_cancel_rejects_unauthenticated_request() {
        let token = "cancel-token";
        let (state, run_id) = authoring_state_with_running_run(token);

        let resp = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            HeaderMap::new(),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::Running),
            "an unauthenticated cancel attempt must not touch the run"
        );
    }

    #[tokio::test]
    async fn authoring_cancel_fails_closed_remotely_when_pairing_is_disabled() {
        let (mut state, run_id) = authoring_state_with_running_run("unused");
        state.pairing = Arc::new(PairingGuard::new(
            false,
            &[],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));

        let remote = handle_sop_cancel(
            State(state.clone()),
            remote_peer(),
            HeaderMap::new(),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;
        assert_eq!(remote.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::Running),
            "a remote unauthenticated request must not mutate the run"
        );

        let loopback = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            HeaderMap::new(),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;
        assert_eq!(loopback.status(), StatusCode::ACCEPTED);
        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::CancelRequested),
            "loopback-only access may remain unpaired"
        );
    }

    #[tokio::test]
    async fn authoring_cancel_rejects_remote_client_behind_trusted_loopback_proxy() {
        let (mut state, run_id) = authoring_state_with_running_run("unused");
        state.pairing = Arc::new(PairingGuard::new(
            false,
            &[],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));
        state.trust_forwarded_headers = true;
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.17"));

        let response = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            headers,
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(run_status(&state, &run_id), Some(SopRunStatus::Running));
    }

    #[tokio::test]
    async fn authoring_cancel_unknown_run_returns_404() {
        let token = "cancel-token";
        let (state, _run_id) = authoring_state_with_running_run(token);

        let resp = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            bearer(token),
            Path(("deploy".to_string(), "nonexistent".to_string())),
            Bytes::new(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn authoring_cancel_rejects_run_id_from_different_sop() {
        let token = "cancel-token";
        let (_tmp, state, run_id) = authoring_state_with_running_run_sop("deploy-b", token);

        let resp = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            bearer(token),
            Path(("deploy-a".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;

        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "a path SOP must not cancel a run owned by another SOP"
        );
        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::Running),
            "a mismatched-SOP cancel attempt must not touch the run"
        );
    }

    #[tokio::test]
    async fn authoring_cancel_rejects_malformed_json_body() {
        let token = "cancel-token";
        let (state, run_id) = authoring_state_with_running_run(token);

        let resp = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            bearer(token),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::from_static(b"{not json"),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::Running),
            "a malformed cancel body must not touch the run"
        );
    }

    #[tokio::test]
    async fn authoring_cancel_a_gated_run_frees_it_regardless_of_approval_policy() {
        let member = "member-token";
        let outsider = "outsider-token";
        let (_tmp, state, run_id) = authoring_state_with_policied_gate(member, outsider);

        let resp = handle_sop_cancel(
            State(state.clone()),
            loopback_peer(),
            bearer(outsider),
            Path(("deploy".to_string(), run_id.clone())),
            Bytes::new(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "cancelled");
        assert_eq!(json["already_terminal"], false);
        assert_eq!(run_status(&state, &run_id), Some(SopRunStatus::Cancelled));
    }
}
