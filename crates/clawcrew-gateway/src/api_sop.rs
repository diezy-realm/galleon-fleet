//! Out-of-band SOP approval surface (EPIC C, C6; EPIC G broker).
//!
//! `GET /admin/sop/pending`, `POST /admin/sop/approve`, `POST /admin/sop/deny`, `GET /admin/sop/metrics`.
//! Auth reuses the vetted `/admin/reload` gate: loopback is always allowed and
//! attributed as `cli` UNLESS pairing is required and it also presents a valid
//! bearer token, in which case it is attributed as `http` (the authenticated
//! subject); a non-loopback caller needs `gateway.allow_remote_admin` + pairing
//! and passes `require_auth`, attributed as an `http` principal. The principal
//! is ALWAYS derived from the transport here, never from the request body.
//! Resolution funnels through `resolve_via_broker` (EPIC G's authorization/
//! quorum layer over the shared engine's `resolve_gate` chokepoint - a no-op
//! pass-through when no `[sop.approval]` policy applies); `sop_engine = None`
//! yields 503.

use std::net::SocketAddr;

use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::{AdminReloadGate, AppState, admin_reload_gate};
#[cfg(test)]
use clawcrew_api::sop::SopRunStatus;
use clawcrew_api::sop::{ApprovalDecision, ApprovalPrincipal, BrokerOutcome, ResolveOutcome};

type JsonErr = (StatusCode, Json<serde_json::Value>);

/// Body for approve/deny.
#[derive(Deserialize)]
pub struct SopResolveBody {
    run_id: String,
    #[serde(default)]
    reason: Option<String>,
}

fn sop_disabled() -> JsonErr {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({ "error": "SOP subsystem not enabled" })),
    )
}

fn lock_poisoned() -> JsonErr {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "SOP engine lock poisoned" })),
    )
}

/// Authorize an admin SOP call and derive the transport-bound principal. Mirrors
/// `handle_admin_reload`'s gate so the two never diverge.
fn authorize(
    state: &AppState,
    peer: &SocketAddr,
    headers: &HeaderMap,
) -> Result<ApprovalPrincipal, JsonErr> {
    let allow_remote = state.config.read().gateway.allow_remote_admin;
    let require_pairing = state.pairing.require_pairing();
    match admin_reload_gate(peer.ip().is_loopback(), allow_remote, require_pairing) {
        AdminReloadGate::Allow => {
            // A loopback caller is always allowed, but if pairing is actually
            // REQUIRED and it ALSO presents a valid paired bearer token, capture
            // that authenticated identity instead of discarding it as anonymous
            // CLI - otherwise a local dashboard/HTTP client with a valid token and
            // `http:<hash>` group membership would be rejected as an anonymous
            // `cli(None)` (no identity, so it can never satisfy a required-group
            // policy), while the SAME token from a non-loopback peer (the
            // `RequireAuth` arm below) resolves correctly.
            //
            // Gated on `require_pairing`: when pairing is OFF,
            // `authenticate_and_hash` treats EVERY token as valid (a no-op
            // pass-through for that mode - see `PairingGuard::is_authenticated`),
            // so deriving an identity from an unauthenticated header in that mode
            // would let any caller-supplied bearer value fabricate an approval
            // identity. WS already only derives a subject when auth is actually
            // meaningful; this matches that.
            let subject = require_pairing
                .then(|| crate::api::extract_bearer_token(headers))
                .flatten()
                .and_then(|t| state.pairing.authenticate_and_hash(t));
            Ok(match subject {
                Some(hash) => ApprovalPrincipal::http(Some(hash)),
                None => ApprovalPrincipal::cli(None),
            })
        }
        AdminReloadGate::RequireAuth => {
            crate::api::require_auth(state, headers)?;
            // Auth passed: derive a STABLE transport-authenticated subject (the
            // paired-token hash) so a required-group approval policy can be satisfied
            // by an authenticated gateway caller. An operator grants approval rights
            // to this paired device via an `http:<token-hash>` group member. Without
            // this the gateway would carry no identity and every policied gate would
            // be unsatisfiable from HTTP.
            let subject = crate::api::extract_bearer_token(headers)
                .and_then(|t| state.pairing.authenticate_and_hash(t));
            Ok(ApprovalPrincipal::http(subject))
        }
        AdminReloadGate::Forbidden | AdminReloadGate::ForbiddenNoPairing => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Remote SOP approval is disabled. Call from localhost, or set \
                          gateway.allow_remote_admin = true with pairing enabled, then pair."
            })),
        )),
    }
}

/// Map a `ResolveOutcome` to its HTTP status + wire label. Pure (unit-tested).
fn outcome_response(outcome: &ResolveOutcome) -> (StatusCode, &'static str) {
    match outcome {
        ResolveOutcome::Resumed => (StatusCode::OK, "resumed"),
        ResolveOutcome::Denied => (StatusCode::OK, "denied"),
        ResolveOutcome::Revised => (StatusCode::OK, "revised"),
        ResolveOutcome::AlreadyResolved => (StatusCode::OK, "already_resolved"),
        ResolveOutcome::NotWaiting => (StatusCode::NOT_FOUND, "not_waiting"),
        ResolveOutcome::RejectedSelfApproval => (StatusCode::FORBIDDEN, "rejected_self_approval"),
        ResolveOutcome::RejectedConstraint(_) => (StatusCode::FORBIDDEN, "rejected_constraint"),
        // Approved, but re-admitting would exceed the concurrency caps: temporary
        // backpressure, retry once a slot frees (the gate stays waiting).
        ResolveOutcome::DeferredAtCapacity => {
            (StatusCode::SERVICE_UNAVAILABLE, "deferred_at_capacity")
        }
    }
}

/// GET /admin/sop/pending - list the runs parked on a human: `WaitingApproval`
/// gates AND deterministic `PausedCheckpoint` runs (both are resolved by the same
/// approve/deny chokepoint), distinguished by the `kind` field.
pub async fn handle_sop_pending(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, JsonErr> {
    // Pending is a read, gated the same as the resolve endpoints.
    authorize(&state, &peer, &headers)?;
    let runner = state.sop_runner.as_ref().ok_or_else(sop_disabled)?;
    let pending = runner.get_pending_runs().map_err(|_| lock_poisoned())?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "pending": pending })),
    ))
}

/// GET /admin/sop/metrics - return a snapshot of global and per-SOP counters
/// from the engine's metrics collector.
pub async fn handle_sop_metrics(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, JsonErr> {
    authorize(&state, &peer, &headers)?;
    let runner = state.sop_runner.as_ref().ok_or_else(sop_disabled)?;
    let snapshot = runner.get_metrics().map_err(|_| lock_poisoned())?;
    Ok((StatusCode::OK, Json(snapshot)))
}

/// POST /admin/sop/approve - clear a waiting gate out-of-band.
pub async fn handle_sop_approve(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<SopResolveBody>,
) -> Result<impl IntoResponse, JsonErr> {
    let principal = authorize(&state, &peer, &headers)?;
    resolve(&state, &body.run_id, ApprovalDecision::Approve, principal).await
}

/// POST /admin/sop/deny - deny (cancel) a waiting run out-of-band.
pub async fn handle_sop_deny(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<SopResolveBody>,
) -> Result<impl IntoResponse, JsonErr> {
    let principal = authorize(&state, &peer, &headers)?;
    resolve(
        &state,
        &body.run_id,
        ApprovalDecision::Deny {
            reason: body.reason,
        },
        principal,
    )
    .await
}

async fn resolve(
    state: &AppState,
    run_id: &str,
    decision: ApprovalDecision,
    principal: ApprovalPrincipal,
) -> Result<(StatusCode, Json<serde_json::Value>), JsonErr> {
    let runner = state.sop_runner.as_ref().ok_or_else(sop_disabled)?;
    let outcome = runner
        .resolve_approval(run_id, decision, principal)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("resolve failed: {e}") })),
            )
        })?;
    let (code, label) = broker_outcome_response(&outcome);
    Ok((
        code,
        Json(serde_json::json!({ "outcome": label, "run_id": run_id })),
    ))
}

/// Map a broker outcome to an HTTP status + label. `Resolved` delegates to the
/// existing `resolve_gate` mapping; the broker-specific outcomes get their own.
fn broker_outcome_response(outcome: &BrokerOutcome) -> (StatusCode, String) {
    match outcome {
        BrokerOutcome::Resolved(o) => {
            let (code, label) = outcome_response(o);
            (code, label.to_string())
        }
        BrokerOutcome::NotWaiting => (StatusCode::NOT_FOUND, "not_waiting".to_string()),
        BrokerOutcome::PendingQuorum { have, need } => (
            StatusCode::ACCEPTED,
            format!("pending_quorum ({have}/{need})"),
        ),
        BrokerOutcome::NotAuthorized { required_group } => (
            StatusCode::FORBIDDEN,
            format!("not_authorized (requires group '{required_group}')"),
        ),
        // A step naming an absent policy is a server-side config defect: fail closed
        // (the gate is left waiting) with a non-success status, never a silent clear.
        BrokerOutcome::PolicyMissing { name } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("policy_missing ('{name}')"),
        ),
        BrokerOutcome::PolicyUnavailable { reason } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("policy_unavailable ({reason})"),
        ),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn outcome_response_maps_status_codes() {
        assert_eq!(
            outcome_response(&ResolveOutcome::Denied),
            (StatusCode::OK, "denied")
        );
        assert_eq!(
            outcome_response(&ResolveOutcome::AlreadyResolved),
            (StatusCode::OK, "already_resolved")
        );
        assert_eq!(
            outcome_response(&ResolveOutcome::NotWaiting),
            (StatusCode::NOT_FOUND, "not_waiting")
        );
        assert_eq!(
            outcome_response(&ResolveOutcome::RejectedSelfApproval),
            (StatusCode::FORBIDDEN, "rejected_self_approval")
        );
        assert_eq!(
            broker_outcome_response(&BrokerOutcome::PolicyMissing {
                name: "prod".into()
            }),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "policy_missing ('prod')".to_string()
            )
        );
    }

    use clawcrew_config::pairing::PairingGuard;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    pub struct MockPoliciedSopRunner {
        pub runs: Mutex<HashMap<String, SopRunStatus>>,
        pub member_hash: String,
    }

    #[async_trait::async_trait]
    impl clawcrew_api::sop::SopRunner for MockPoliciedSopRunner {
        fn list_sops(&self) -> anyhow::Result<Vec<serde_json::Value>> {
            Ok(Vec::new())
        }
        fn get_trigger_sources(&self) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        fn get_sop_graph(&self, _name: &str) -> anyhow::Result<Option<serde_json::Value>> {
            Ok(None)
        }
        fn list_runs(
            &self,
            _sop_name: Option<&str>,
        ) -> anyhow::Result<Vec<clawcrew_api::sop::SopRunSummary>> {
            let guard = self.runs.lock().unwrap();
            let runs = guard
                .iter()
                .map(|(id, status)| clawcrew_api::sop::SopRunSummary {
                    run_id: id.clone(),
                    sop_name: "deploy".into(),
                    status: *status,
                    current_step: 1,
                    total_steps: 1,
                    started_at: String::new(),
                    completed_at: None,
                    trigger_source: "manual".into(),
                    active: *status == SopRunStatus::WaitingApproval,
                })
                .collect();
            Ok(runs)
        }
        fn get_run_overlay(
            &self,
            _name: &str,
            _run_id: &str,
        ) -> anyhow::Result<Option<serde_json::Value>> {
            Ok(None)
        }
        fn get_pending_runs(&self) -> anyhow::Result<Vec<serde_json::Value>> {
            let guard = self.runs.lock().unwrap();
            let pending = guard
                .iter()
                .filter(|(_, s)| **s == SopRunStatus::WaitingApproval)
                .map(|(id, _)| {
                    serde_json::json!({
                        "run_id": id,
                        "sop_name": "deploy",
                        "step": 1,
                        "total_steps": 1,
                        "waiting_since": "now",
                        "kind": "approval",
                    })
                })
                .collect();
            Ok(pending)
        }
        fn get_metrics(&self) -> anyhow::Result<serde_json::Value> {
            let guard = self.runs.lock().unwrap();
            let approved = guard
                .values()
                .filter(|s| **s == SopRunStatus::Running)
                .count() as u64;
            Ok(serde_json::json!({
                "global": { "human_approvals": approved },
                "per_sop": {},
                "pending_approvals": 0,
                "pending_timeout_approvals": 0,
            }))
        }
        fn subscribe_run_changes(
            &self,
        ) -> anyhow::Result<(
            Vec<clawcrew_api::sop::SopRunSummary>,
            Option<tokio::sync::broadcast::Receiver<clawcrew_api::sop::SopRunSummary>>,
        )> {
            Ok((Vec::new(), None))
        }
        async fn start_sop(&self, _name: &str) -> anyhow::Result<(u16, serde_json::Value)> {
            Ok((200, serde_json::json!({ "run_id": "test-run" })))
        }
        async fn resolve_approval(
            &self,
            run_id: &str,
            _decision: ApprovalDecision,
            principal: ApprovalPrincipal,
        ) -> anyhow::Result<BrokerOutcome> {
            let authorized = principal.identity.as_deref() == Some(&self.member_hash);
            if authorized {
                let mut guard = self.runs.lock().unwrap();
                if let Some(status) = guard.get_mut(run_id) {
                    *status = SopRunStatus::Running;
                }
                Ok(BrokerOutcome::Resolved(ResolveOutcome::Resumed))
            } else {
                Ok(BrokerOutcome::NotAuthorized {
                    required_group: "release".into(),
                })
            }
        }
        async fn resolve_author_approval(
            &self,
            name: &str,
            run_id: &str,
            decision: ApprovalDecision,
            principal: ApprovalPrincipal,
        ) -> anyhow::Result<(u16, serde_json::Value)> {
            match self.resolve_approval(run_id, decision, principal).await? {
                BrokerOutcome::Resolved(_) => Ok((
                    200,
                    serde_json::json!({ "run_id": run_id, "sop_name": name }),
                )),
                BrokerOutcome::NotAuthorized { required_group } => Ok((
                    403,
                    serde_json::json!({ "error": format!("not authorized: requires group '{required_group}'") }),
                )),
                _ => Ok((500, serde_json::json!({ "error": "unhandled" }))),
            }
        }
        fn cancel_run(
            &self,
            name: &str,
            run_id: &str,
            _reason: Option<&str>,
            _principal: ApprovalPrincipal,
        ) -> anyhow::Result<(u16, serde_json::Value)> {
            let mut guard = self.runs.lock().unwrap();
            if let Some(status) = guard.get_mut(run_id) {
                *status = SopRunStatus::Cancelled;
            }
            Ok((
                200,
                serde_json::json!({ "run_id": run_id, "sop_name": name, "status": "cancelled", "outcome": "cancelled" }),
            ))
        }
        fn get_sop(&self, _name: &str) -> anyhow::Result<Option<clawcrew_api::sop::Sop>> {
            Ok(None)
        }
        fn create_sop(&self, _sop: &clawcrew_api::sop::Sop) -> anyhow::Result<()> {
            Ok(())
        }
        fn save_sop(&self, _name: &str, _sop: &clawcrew_api::sop::Sop) -> anyhow::Result<()> {
            Ok(())
        }
        fn delete_sop(&self, _name: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn wire_draft(
            &self,
            _sop: &clawcrew_api::sop::Sop,
            _edit: &serde_json::Value,
        ) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        fn graph_draft(&self, _sop: &clawcrew_api::sop::Sop) -> anyhow::Result<serde_json::Value> {
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

    /// Build a gateway `AppState` whose SOP engine holds a run parked at a
    /// `policy = "prod"` approval gate, with group `release` = [`http:<token-hash>`]
    /// so the paired `token` (and only it) satisfies the policy over HTTP.
    pub(crate) fn state_with_policied_gate(token: &str) -> (AppState, String) {
        let hash = PairingGuard::token_hash(token);
        let run_id = "deploy-run-1".to_string();
        let mut runs = HashMap::new();
        runs.insert(run_id.clone(), SopRunStatus::WaitingApproval);
        let runner = Arc::new(MockPoliciedSopRunner {
            runs: Mutex::new(runs),
            member_hash: hash,
        });

        let mut config = clawcrew_config::schema::Config::default();
        config.gateway.allow_remote_admin = true;
        let mut state = crate::api::test_state(config);
        state.sop_runner = Some(runner);
        state.pairing = Arc::new(PairingGuard::new(
            true,
            &[token.to_string()],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));
        (state, run_id)
    }

    fn bearer(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        headers
    }

    #[tokio::test]
    async fn http_surface_enforces_policy_membership_via_authenticated_subject() {
        let run_status = |state: &AppState, id: &str| {
            state
                .sop_runner
                .as_ref()
                .unwrap()
                .list_runs(None)
                .unwrap()
                .into_iter()
                .find(|r| r.run_id == id)
                .map(|r| r.status)
        };
        let token = "pair-token-abc";
        let (state, run_id) = state_with_policied_gate(token);
        // A remote (non-loopback) caller forces the authenticated `http` path.
        let peer: SocketAddr = "203.0.113.7:5000".parse().unwrap();

        // An authenticated caller whose paired-token hash is NOT in `release` is
        // rejected (membership is enforced on the derived gateway identity), and the
        // gate is left waiting.
        let other = "some-other-paired-token";
        let mut state_other = state.clone();
        state_other.pairing = std::sync::Arc::new(clawcrew_config::pairing::PairingGuard::new(
            true,
            &[token.to_string(), other.to_string()],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));
        let resp = handle_sop_approve(
            State(state_other),
            ConnectInfo(peer),
            bearer(other),
            Json(SopResolveBody {
                run_id: run_id.clone(),
                reason: None,
            }),
        )
        .await
        .expect("handler returns a response")
        .into_response();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "an authenticated non-member must NOT clear a policied gate"
        );
        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::WaitingApproval),
            "the gate stays waiting after a non-member attempt"
        );

        // The authenticated member (its `http:<hash>` is in `release`) clears it.
        let resp = handle_sop_approve(
            State(state.clone()),
            ConnectInfo(peer),
            bearer(token),
            Json(SopResolveBody {
                run_id: run_id.clone(),
                reason: None,
            }),
        )
        .await
        .expect("handler returns a response")
        .into_response();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "an authenticated member clears the policied gate over HTTP"
        );
        assert_ne!(
            run_status(&state, &run_id),
            Some(SopRunStatus::WaitingApproval),
            "the gate is cleared (run resumes) once an authorized member approves"
        );
    }

    #[tokio::test]
    async fn loopback_caller_with_a_valid_token_keeps_its_authenticated_identity() {
        // Regression: a loopback caller is always allowed
        // (`AdminReloadGate::Allow`), but `authorize()` used to map that straight
        // to `ApprovalPrincipal::cli(None)` regardless of whether a valid paired
        // bearer token was also presented, discarding the authenticated identity.
        // A local dashboard/HTTP client with a valid token whose hash is a policy's
        // `required_group` member would then be rejected as an anonymous CLI (no
        // identity, cannot satisfy any required-group membership), even though the
        // same token from a non-loopback peer resolves correctly (see
        // `http_surface_enforces_policy_membership_via_authenticated_subject`).
        let run_status = |state: &AppState, id: &str| {
            state
                .sop_runner
                .as_ref()
                .unwrap()
                .list_runs(None)
                .unwrap()
                .into_iter()
                .find(|r| r.run_id == id)
                .map(|r| r.status)
        };
        let token = "pair-token-abc";
        let (state, run_id) = state_with_policied_gate(token);
        let loopback: SocketAddr = "127.0.0.1:9".parse().unwrap();

        let resp = handle_sop_approve(
            State(state.clone()),
            ConnectInfo(loopback),
            bearer(token),
            Json(SopResolveBody {
                run_id: run_id.clone(),
                reason: None,
            }),
        )
        .await
        .expect("handler returns a response")
        .into_response();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "a loopback caller presenting its member token must clear the policied \
             gate, not be treated as anonymous CLI"
        );
        assert_ne!(
            run_status(&state, &run_id),
            Some(SopRunStatus::WaitingApproval),
            "the gate is cleared once the loopback caller's authenticated identity \
             is recognized as a member"
        );
    }

    #[tokio::test]
    async fn loopback_bearer_identity_is_not_derived_when_pairing_is_disabled() {
        // Regression: a loopback caller's identity must only be derived from a
        // bearer token when pairing is required and the token is a real
        // authentication credential. `PairingGuard::is_authenticated` treats every
        // token as valid when `require_pairing` is false (a pass-through for that
        // mode, not real authentication). Without gating on `require_pairing`, a
        // loopback caller with pairing disabled could present an arbitrary bearer
        // value and be attributed the derived hash as its approval identity,
        // fabricating membership in a required group from an unauthenticated
        // header. The gate must not clear from an unauthenticated loopback request
        // even if its pairing-off token hash happens to match a real group member.
        let run_status = |state: &AppState, id: &str| {
            state
                .sop_runner
                .as_ref()
                .unwrap()
                .list_runs(None)
                .unwrap()
                .into_iter()
                .find(|r| r.run_id == id)
                .map(|r| r.status)
        };
        let token = "pair-token-abc";
        let (mut state, run_id) = state_with_policied_gate(token);
        // Pairing is now OFF - `is_authenticated` accepts any token, so the SAME
        // token string that legitimately satisfied membership when paired must NOT
        // grant an authenticated identity anymore.
        state.pairing = std::sync::Arc::new(clawcrew_config::pairing::PairingGuard::new(
            false,
            &[],
            clawcrew_config::pairing::PairingCodePolicy::default(),
        ));
        let loopback: SocketAddr = "127.0.0.1:9".parse().unwrap();

        let resp = handle_sop_approve(
            State(state.clone()),
            ConnectInfo(loopback),
            bearer(token),
            Json(SopResolveBody {
                run_id: run_id.clone(),
                reason: None,
            }),
        )
        .await
        .expect("handler returns a response")
        .into_response();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "a loopback caller must NOT get an authenticated identity from an \
             unauthenticated bearer header when pairing is disabled"
        );
        assert_eq!(
            run_status(&state, &run_id),
            Some(SopRunStatus::WaitingApproval),
            "the gate stays waiting - no fabricated identity may clear it"
        );
    }

    #[tokio::test]
    async fn sop_metrics_endpoint_returns_snapshot_and_rejects_unauthorized() {
        let token = "pair-token-metrics";
        let (state, run_id) = state_with_policied_gate(token);
        let loopback: SocketAddr = "127.0.0.1:9".parse().unwrap();
        let remote: SocketAddr = "203.0.113.8:5000".parse().unwrap();

        // 1. Unauthorized remote caller is rejected
        let unauthorized_resp =
            match handle_sop_metrics(State(state.clone()), ConnectInfo(remote), HeaderMap::new())
                .await
            {
                Err(e) => e,
                Ok(_) => panic!("unauthenticated remote call must fail authorization"),
            };
        assert_eq!(unauthorized_resp.0, StatusCode::UNAUTHORIZED);

        // 2. Loopback caller gets 200 OK with valid metrics snapshot
        let resp = handle_sop_metrics(
            State(state.clone()),
            ConnectInfo(loopback),
            HeaderMap::new(),
        )
        .await
        .expect("loopback call succeeds")
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse json");

        assert!(json.get("global").is_some());
        assert!(json.get("per_sop").is_some());
        assert!(json.get("pending_approvals").is_some());
        assert!(json.get("pending_timeout_approvals").is_some());

        // 3. Clear the gate to record an approval metric, then verify counter updates
        let approve_resp = handle_sop_approve(
            State(state.clone()),
            ConnectInfo(loopback),
            bearer(token),
            Json(SopResolveBody {
                run_id: run_id.clone(),
                reason: None,
            }),
        )
        .await
        .expect("approve succeeds")
        .into_response();
        assert_eq!(approve_resp.status(), StatusCode::OK);

        let resp2 = handle_sop_metrics(
            State(state.clone()),
            ConnectInfo(loopback),
            HeaderMap::new(),
        )
        .await
        .expect("loopback metrics call succeeds")
        .into_response();
        let body2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
            .await
            .expect("read body");
        let json2: serde_json::Value = serde_json::from_slice(&body2).expect("parse json");
        let global_approvals = json2["global"]["human_approvals"].as_u64().unwrap_or(0);
        assert_eq!(global_approvals, 1);
    }
}
