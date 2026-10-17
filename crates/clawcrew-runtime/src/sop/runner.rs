//! Implementation of `clawcrew_api::sop::SopRunner` for gateway decoupling.

use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clawcrew_api::sop::{
    ApprovalDecision, ApprovalPrincipal, BrokerOutcome, ResolveOutcome, Sop, SopRunSummary,
    SopRunner,
};
use clawcrew_config::schema::Config;

use super::audit::SopAuditLogger;
use super::engine::SopEngine;
use super::graph::SopGraphExt;

pub struct RuntimeSopRunner {
    engine: Option<Arc<Mutex<SopEngine>>>,
    audit: Option<Arc<SopAuditLogger>>,
    config: Arc<RwLock<Config>>,
    install_root: PathBuf,
}

impl RuntimeSopRunner {
    pub fn new(
        engine: Option<Arc<Mutex<SopEngine>>>,
        audit: Option<Arc<SopAuditLogger>>,
        config: Arc<RwLock<Config>>,
        install_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            engine,
            audit,
            config,
            install_root: install_root.into(),
        }
    }
}

fn map_broker_outcome(outcome: &super::approval::BrokerOutcome) -> BrokerOutcome {
    match outcome {
        super::approval::BrokerOutcome::Resolved(r) => {
            let res = match r {
                super::approval::ResolveOutcome::Resumed(_) => ResolveOutcome::Resumed,
                super::approval::ResolveOutcome::Denied => ResolveOutcome::Denied,
                super::approval::ResolveOutcome::Revised => ResolveOutcome::Revised,
                super::approval::ResolveOutcome::AlreadyResolved => ResolveOutcome::AlreadyResolved,
                super::approval::ResolveOutcome::NotWaiting => ResolveOutcome::NotWaiting,
                super::approval::ResolveOutcome::RejectedSelfApproval => {
                    ResolveOutcome::RejectedSelfApproval
                }
                super::approval::ResolveOutcome::RejectedConstraint(msg) => {
                    ResolveOutcome::RejectedConstraint(msg.clone())
                }
                super::approval::ResolveOutcome::DeferredAtCapacity => {
                    ResolveOutcome::DeferredAtCapacity
                }
            };
            BrokerOutcome::Resolved(res)
        }
        super::approval::BrokerOutcome::PendingQuorum { have, need } => {
            BrokerOutcome::PendingQuorum {
                have: *have,
                need: *need,
            }
        }
        super::approval::BrokerOutcome::NotAuthorized { required_group } => {
            BrokerOutcome::NotAuthorized {
                required_group: required_group.clone(),
            }
        }
        super::approval::BrokerOutcome::PolicyMissing { name } => {
            BrokerOutcome::PolicyMissing { name: name.clone() }
        }
        super::approval::BrokerOutcome::PolicyUnavailable { reason } => {
            BrokerOutcome::PolicyUnavailable {
                reason: reason.clone(),
            }
        }
        super::approval::BrokerOutcome::NotWaiting => BrokerOutcome::NotWaiting,
    }
}

#[async_trait::async_trait]
impl SopRunner for RuntimeSopRunner {
    fn list_sops(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        let mode = super::parse_execution_mode(&config.sop.default_execution_mode);
        let sops = super::load_sops_from_directory(&dir, mode);
        let val = serde_json::to_value(sops)?;
        match val {
            serde_json::Value::Array(arr) => Ok(arr),
            other => Ok(vec![other]),
        }
    }

    fn get_trigger_sources(&self) -> anyhow::Result<serde_json::Value> {
        let config = self.config.read();
        let registry = super::registry_from_config(&config);
        Ok(serde_json::to_value(registry)?)
    }

    fn get_sop_graph(&self, name: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        let mode = super::parse_execution_mode(&config.sop.default_execution_mode);
        match super::load_sop_by_name(&dir, name, mode) {
            Ok(sop) => {
                let agent = config.agents.keys().min().cloned().unwrap_or_default();
                let specs = super::tool_specs_from_config(&config, &agent);
                let graph = super::SopGraph::from_sop_with_specs(&sop, &specs);
                Ok(Some(serde_json::to_value(graph)?))
            }
            Err(_) => Ok(None),
        }
    }

    fn list_runs(&self, sop_name: Option<&str>) -> anyhow::Result<Vec<SopRunSummary>> {
        let Some(engine) = &self.engine else {
            return Ok(Vec::new());
        };
        let runs = super::run_summaries_for(engine, sop_name)
            .map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
        let val = serde_json::to_value(runs)?;
        let mapped: Vec<SopRunSummary> = serde_json::from_value(val)?;
        Ok(mapped)
    }

    fn get_run_overlay(
        &self,
        name: &str,
        run_id: &str,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        let Some(engine) = &self.engine else {
            return Ok(None);
        };
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        let mode = super::parse_execution_mode(&config.sop.default_execution_mode);
        let Ok(sop) = super::load_sop_by_name(&dir, name, mode) else {
            return Ok(None);
        };
        match super::run_overlay_for(&sop, engine, run_id) {
            Ok(overlay) => Ok(Some(serde_json::to_value(overlay)?)),
            Err(e) if e.to_string().contains("not found") => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn get_pending_runs(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let Some(engine) = &self.engine else {
            return Err(anyhow::anyhow!("sop_disabled"));
        };
        let guard = engine
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let pending: Vec<serde_json::Value> = guard
            .active_runs()
            .values()
            .filter(|r| {
                matches!(
                    r.status,
                    super::types::SopRunStatus::WaitingApproval
                        | super::types::SopRunStatus::PausedCheckpoint
                )
            })
            .map(|r| {
                serde_json::json!({
                    "run_id": r.run_id,
                    "sop_name": r.sop_name,
                    "step": r.current_step,
                    "total_steps": r.total_steps,
                    "waiting_since": r.waiting_since,
                    "kind": if r.status == super::types::SopRunStatus::PausedCheckpoint {
                        "checkpoint"
                    } else {
                        "approval"
                    },
                })
            })
            .collect();
        Ok(pending)
    }

    fn get_metrics(&self) -> anyhow::Result<serde_json::Value> {
        let Some(engine) = &self.engine else {
            return Err(anyhow::anyhow!("sop_disabled"));
        };
        let guard = engine
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        let snapshot = guard.metrics().snapshot();
        Ok(serde_json::to_value(snapshot)?)
    }

    fn subscribe_run_changes(
        &self,
    ) -> anyhow::Result<(
        Vec<SopRunSummary>,
        Option<tokio::sync::broadcast::Receiver<SopRunSummary>>,
    )> {
        let Some(engine) = &self.engine else {
            return Err(anyhow::anyhow!("sop_disabled"));
        };
        let (summaries, rx) = {
            let guard = engine
                .lock()
                .map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
            (guard.run_summaries(None), guard.subscribe_run_changes())
        };
        let val = serde_json::to_value(summaries)?;
        let mapped_summaries: Vec<SopRunSummary> = serde_json::from_value(val)?;
        let mapped_rx = rx.map(|mut r| {
            let (tx, mapped_receiver) = tokio::sync::broadcast::channel(32);
            tokio::spawn(async move {
                while let Ok(item) = r.recv().await {
                    if let Ok(val) = serde_json::to_value(item) {
                        if let Ok(mapped) = serde_json::from_value::<SopRunSummary>(val) {
                            let _ = tx.send(mapped);
                        }
                    }
                }
            });
            mapped_receiver
        });
        Ok((mapped_summaries, mapped_rx))
    }

    async fn start_sop(&self, name: &str) -> anyhow::Result<(u16, serde_json::Value)> {
        let Some(engine) = &self.engine else {
            return Err(anyhow::anyhow!("sop_disabled"));
        };
        let Some(audit) = self.audit.as_ref() else {
            return Err(anyhow::anyhow!("sop_audit_unavailable"));
        };
        let event = super::types::SopEvent {
            source: super::types::SopTriggerSource::Manual,
            topic: None,
            payload: None,
            timestamp: super::engine::now_iso8601(),
        };
        let results = super::dispatch::dispatch_sop_event_to(engine, audit, event, name).await;
        super::dispatch::process_headless_results(&results);
        for r in &results {
            match r {
                super::dispatch::DispatchResult::Started { run_id, action, .. } => {
                    let needs_driver = matches!(
                        action.as_ref(),
                        super::types::SopRunAction::ExecuteStep { .. }
                            | super::types::SopRunAction::DeterministicStep { .. }
                    );
                    if needs_driver {
                        let config = self.config.read().clone();
                        super::spawn_headless_run_driver(
                            config,
                            Arc::clone(engine),
                            Some(Arc::clone(audit)),
                            action.as_ref().clone(),
                        );
                    }
                    return Ok((
                        200,
                        serde_json::json!({
                            "run_id": run_id
                        }),
                    ));
                }
                super::dispatch::DispatchResult::Skipped { reason, .. } => {
                    return Ok((
                        409,
                        serde_json::json!({
                            "error": reason
                        }),
                    ));
                }
                super::dispatch::DispatchResult::BlockedUnsafe { reason, .. } => {
                    return Ok((
                        422,
                        serde_json::json!({
                            "error": reason
                        }),
                    ));
                }
                super::dispatch::DispatchResult::Deferred { reason, .. } => {
                    return Ok((
                        503,
                        serde_json::json!({
                            "error": reason
                        }),
                    ));
                }
                super::dispatch::DispatchResult::Coalesced {
                    existing_run_id, ..
                } => {
                    return Ok((
                        200,
                        serde_json::json!({
                            "run_id": existing_run_id
                        }),
                    ));
                }
                super::dispatch::DispatchResult::NoMatch => {}
            }
        }
        Ok((
            404,
            serde_json::json!({
                "error": format!("SOP '{name}' has no matching manual trigger")
            }),
        ))
    }

    async fn resolve_approval(
        &self,
        run_id: &str,
        decision: ApprovalDecision,
        principal: ApprovalPrincipal,
    ) -> anyhow::Result<BrokerOutcome> {
        let Some(engine) = &self.engine else {
            return Err(anyhow::anyhow!("sop_disabled"));
        };
        let outcome = {
            let mut guard = engine
                .lock()
                .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
            guard
                .resolve_via_broker_deferred(run_id, decision, principal)
                .map_err(|e| anyhow::anyhow!("resolve failed: {e}"))?
        };
        let config = self.config.read();
        super::drive_resumed_broker_action(
            &config,
            Arc::clone(engine),
            self.audit.clone(),
            &outcome,
        );
        Ok(map_broker_outcome(&outcome))
    }

    async fn resolve_author_approval(
        &self,
        name: &str,
        run_id: &str,
        decision: ApprovalDecision,
        principal: ApprovalPrincipal,
    ) -> anyhow::Result<(u16, serde_json::Value)> {
        let Some(engine) = &self.engine else {
            return Ok((
                503,
                serde_json::json!({ "error": "SOP subsystem not enabled" }),
            ));
        };
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        let mode = super::parse_execution_mode(&config.sop.default_execution_mode);
        let sop = match super::load_sop_by_name(&dir, name, mode) {
            Ok(s) => s,
            Err(e) => {
                return Ok((
                    404,
                    serde_json::json!({ "error": format!("SOP '{name}': {e}") }),
                ));
            }
        };

        let mut resolved_outcome = None;
        let mut pending_quorum = false;
        {
            let mut guard = engine
                .lock()
                .map_err(|_| anyhow::anyhow!("SOP engine lock poisoned"))?;
            let run_sop_name = match guard.get_run(run_id).map(|run| run.sop_name.clone()) {
                Some(n) => n,
                None => {
                    return Ok((
                        404,
                        serde_json::json!({ "error": format!("Run {run_id} not found") }),
                    ));
                }
            };
            if run_sop_name != name {
                return Ok((
                    400,
                    serde_json::json!({
                        "error": format!("run '{run_id}' belongs to SOP '{run_sop_name}', not '{name}'")
                    }),
                ));
            }
            let status = guard.get_run(run_id).map(|r| r.status);
            match status {
                Some(
                    super::types::SopRunStatus::WaitingApproval
                    | super::types::SopRunStatus::PausedCheckpoint,
                ) => {
                    use super::approval::{BrokerOutcome, ResolveOutcome};
                    match guard.resolve_via_broker_deferred(run_id, decision, principal) {
                        Ok(outcome @ BrokerOutcome::Resolved(ResolveOutcome::Resumed(_))) => {
                            resolved_outcome = Some(outcome);
                        }
                        Ok(BrokerOutcome::Resolved(
                            ResolveOutcome::Denied
                            | ResolveOutcome::AlreadyResolved
                            | ResolveOutcome::Revised,
                        )) => {}
                        Ok(
                            BrokerOutcome::Resolved(ResolveOutcome::NotWaiting)
                            | BrokerOutcome::NotWaiting,
                        ) => {
                            return Ok((
                                409,
                                serde_json::json!({
                                    "error": format!("Run {run_id} is not waiting for approval")
                                }),
                            ));
                        }
                        Ok(BrokerOutcome::Resolved(ResolveOutcome::RejectedSelfApproval)) => {
                            return Ok((
                                403,
                                serde_json::json!({
                                    "error": "approval_mode forbids this principal from clearing the gate"
                                }),
                            ));
                        }
                        Ok(BrokerOutcome::Resolved(ResolveOutcome::RejectedConstraint(reason))) => {
                            return Ok((
                                403,
                                serde_json::json!({
                                    "error": format!("constraint violation: {reason}")
                                }),
                            ));
                        }
                        Ok(BrokerOutcome::Resolved(ResolveOutcome::DeferredAtCapacity)) => {
                            return Ok((
                                503,
                                serde_json::json!({
                                    "outcome": "deferred_at_capacity",
                                    "run_id": run_id,
                                }),
                            ));
                        }
                        Ok(BrokerOutcome::NotAuthorized { required_group }) => {
                            return Ok((
                                403,
                                serde_json::json!({
                                    "error": format!("not authorized: requires group '{required_group}'")
                                }),
                            ));
                        }
                        Ok(BrokerOutcome::PolicyMissing { name }) => {
                            return Ok((
                                500,
                                serde_json::json!({
                                    "error": format!("approval policy '{name}' is not configured (gate left waiting)")
                                }),
                            ));
                        }
                        Ok(BrokerOutcome::PolicyUnavailable { reason }) => {
                            return Ok((
                                500,
                                serde_json::json!({
                                    "error": "policy_unavailable",
                                    "reason": reason,
                                }),
                            ));
                        }
                        Ok(BrokerOutcome::PendingQuorum { .. }) => {
                            pending_quorum = true;
                        }
                        Err(e) => {
                            return Ok((500, serde_json::json!({ "error": e.to_string() })));
                        }
                    }
                }
                _ => {
                    return Ok((
                        400,
                        serde_json::json!({
                            "error": format!("Run {run_id} is not waiting for approval or paused at a checkpoint")
                        }),
                    ));
                }
            }
        }

        if pending_quorum {
            return match super::run_overlay_for(&sop, engine, run_id) {
                Ok(overlay) => Ok((202, serde_json::to_value(overlay)?)),
                Err(e) => {
                    let msg = e.to_string();
                    let code = if msg.contains("not found") { 404 } else { 500 };
                    Ok((code, serde_json::json!({ "error": msg })))
                }
            };
        }

        if let Some(outcome) = resolved_outcome {
            let config = self.config.read();
            super::drive_resumed_broker_action(
                &config,
                Arc::clone(engine),
                self.audit.clone(),
                &outcome,
            );
        }

        match super::run_overlay_for(&sop, engine, run_id) {
            Ok(overlay) => Ok((200, serde_json::to_value(overlay)?)),
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("not found") { 404 } else { 500 };
                Ok((code, serde_json::json!({ "error": msg })))
            }
        }
    }

    fn cancel_run(
        &self,
        name: &str,
        run_id: &str,
        reason: Option<&str>,
        principal: ApprovalPrincipal,
    ) -> anyhow::Result<(u16, serde_json::Value)> {
        let Some(engine) = &self.engine else {
            return Ok((
                503,
                serde_json::json!({ "error": "SOP subsystem not enabled" }),
            ));
        };
        let mut guard = engine
            .lock()
            .map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
        let Some(run) = guard.get_run(run_id) else {
            return Ok((
                404,
                serde_json::json!({ "error": format!("Run {run_id} not found") }),
            ));
        };
        if run.sop_name != name {
            return Ok((
                400,
                serde_json::json!({
                    "error": format!("run '{run_id}' belongs to SOP '{}', not '{name}'", run.sop_name)
                }),
            ));
        }
        let outcome = guard.cancel_run_idempotent(
            run_id,
            reason.map(str::to_string),
            Some(principal.voter_key()),
        );
        match outcome {
            Ok(Some(outcome)) => {
                let (status_code, already_terminal) = match outcome {
                    super::CancelOutcome::Requested | super::CancelOutcome::AlreadyRequested => {
                        (202, false)
                    }
                    super::CancelOutcome::Cancelled => (200, false),
                    super::CancelOutcome::AlreadyTerminal(_) => (200, true),
                };
                let Some(run) = guard.get_run(run_id) else {
                    return Ok((
                        500,
                        serde_json::json!({ "error": "run disappeared after cancellation transition" }),
                    ));
                };
                let is_active = guard.active_runs().contains_key(run_id);
                let summary = SopRunSummary {
                    run_id: run.run_id.clone(),
                    sop_name: run.sop_name.clone(),
                    status: run.status,
                    current_step: run.current_step,
                    total_steps: run.total_steps,
                    started_at: run.started_at.clone(),
                    completed_at: run.completed_at.clone(),
                    trigger_source: run.trigger_event.source.to_string(),
                    active: is_active,
                };
                Ok((
                    status_code,
                    serde_json::json!({
                        "run_id": run_id,
                        "sop_name": name,
                        "status": summary.status,
                        "already_terminal": already_terminal,
                        "run": summary,
                    }),
                ))
            }
            Ok(None) => Ok((
                404,
                serde_json::json!({ "error": format!("Run {run_id} not found") }),
            )),
            Err(e) if super::err_is_cancellation_persistence_retained(&e) => Ok((
                503,
                serde_json::json!({
                    "error": "cancellation could not be durably persisted; the run remains active - retry"
                }),
            )),
            Err(e) => Ok((500, serde_json::json!({ "error": e.to_string() }))),
        }
    }

    fn get_sop(&self, name: &str) -> anyhow::Result<Option<Sop>> {
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        let mode = super::parse_execution_mode(&config.sop.default_execution_mode);
        match super::load_sop_by_name(&dir, name, mode) {
            Ok(rt_sop) => {
                let val = serde_json::to_value(rt_sop)?;
                let sop: Sop = serde_json::from_value(val)?;
                Ok(Some(sop))
            }
            Err(e) if e.to_string().contains("not found") => Ok(None),
            Err(e) => Err(anyhow::anyhow!("{e}")),
        }
    }

    fn create_sop(&self, sop: &Sop) -> anyhow::Result<()> {
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        let val = serde_json::to_value(sop)?;
        let rt_sop: super::types::Sop = serde_json::from_value(val)?;
        super::create_sop_typed(&dir, &rt_sop).map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn save_sop(&self, name: &str, sop: &Sop) -> anyhow::Result<()> {
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        let val = serde_json::to_value(sop)?;
        let mut rt_sop: super::types::Sop = serde_json::from_value(val)?;
        rt_sop.name = name.to_string();
        super::save_sop(&dir, &rt_sop).map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn delete_sop(&self, name: &str) -> anyhow::Result<()> {
        let config = self.config.read();
        let install_root = self.install_root.clone();
        let dir = super::resolve_sops_dir(&install_root, config.sop.sops_dir.as_deref());
        super::delete_sop_typed(&dir, name).map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn wire_draft(&self, sop: &Sop, edit: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let val = serde_json::to_value(sop)?;
        let mut rt_sop: super::types::Sop = serde_json::from_value(val)?;
        let wire_edit: clawcrew_sop_graph::WireEdit = serde_json::from_value(edit.clone())?;
        super::apply_wire(&mut rt_sop, &wire_edit).map_err(|e| anyhow::anyhow!("{e}"))?;
        let config = self.config.read();
        let agent = config.agents.keys().min().cloned().unwrap_or_default();
        let specs = super::tool_specs_from_config(&config, &agent);
        let graph = super::SopGraph::from_sop_with_specs(&rt_sop, &specs);
        Ok(serde_json::json!({ "sop": rt_sop, "graph": graph }))
    }

    fn graph_draft(&self, sop: &Sop) -> anyhow::Result<serde_json::Value> {
        let val = serde_json::to_value(sop)?;
        let rt_sop: super::types::Sop = serde_json::from_value(val)?;
        let config = self.config.read();
        let agent = config.agents.keys().min().cloned().unwrap_or_default();
        let specs = super::tool_specs_from_config(&config, &agent);
        let graph = super::SopGraph::from_sop_with_specs(&rt_sop, &specs);
        Ok(serde_json::to_value(graph)?)
    }

    fn has_matching_webhook_sop(&self, path: &str) -> anyhow::Result<bool> {
        let (Some(engine), Some(_audit)) = (&self.engine, &self.audit) else {
            if self.engine.is_none() && self.audit.is_none() {
                return Ok(false);
            }
            return Err(anyhow::anyhow!("sop_dispatch_unavailable"));
        };
        let event = super::types::SopEvent {
            source: super::types::SopTriggerSource::Webhook,
            topic: Some(path.to_string()),
            payload: None,
            timestamp: super::engine::now_iso8601(),
        };
        engine
            .lock()
            .map(|engine| !engine.match_trigger(&event).is_empty())
            .map_err(|_| anyhow::anyhow!("sop_dispatch_unavailable"))
    }

    async fn dispatch_webhook_sop(
        &self,
        path: &str,
        payload: Option<&str>,
    ) -> anyhow::Result<Option<clawcrew_api::sop::WebhookDispatchResult>> {
        let (Some(engine), Some(audit)) = (&self.engine, &self.audit) else {
            return Err(anyhow::anyhow!("sop_dispatch_unavailable"));
        };
        let results = super::dispatch::dispatch_untrusted_fan_in(
            engine,
            audit,
            super::types::SopTriggerSource::Webhook,
            Some(path),
            payload,
            None,
        )
        .await;
        if results.is_empty() {
            return Err(anyhow::anyhow!("sop_dispatch_unavailable"));
        }
        if results
            .iter()
            .all(|result| matches!(result, super::dispatch::DispatchResult::NoMatch))
        {
            return Ok(None);
        }
        let blocked_only = results.iter().all(|result| {
            matches!(
                result,
                super::dispatch::DispatchResult::BlockedUnsafe { .. }
                    | super::dispatch::DispatchResult::NoMatch
            )
        });
        let mapped = results
            .into_iter()
            .filter_map(|result| match result {
                super::dispatch::DispatchResult::Started {
                    run_id, sop_name, ..
                } => Some(serde_json::json!({
                    "status": "started",
                    "sop": sop_name,
                    "run_id": run_id,
                })),
                super::dispatch::DispatchResult::Skipped { sop_name, reason } => {
                    Some(serde_json::json!({
                        "status": "skipped",
                        "sop": sop_name,
                        "reason": reason,
                    }))
                }
                super::dispatch::DispatchResult::Deferred { sop_name, reason } => {
                    Some(serde_json::json!({
                        "status": "deferred",
                        "sop": sop_name,
                        "reason": reason,
                    }))
                }
                super::dispatch::DispatchResult::Coalesced {
                    sop_name,
                    existing_run_id,
                } => Some(serde_json::json!({
                    "status": "coalesced",
                    "sop": sop_name,
                    "run_id": existing_run_id,
                })),
                super::dispatch::DispatchResult::BlockedUnsafe { sop_name, reason } => {
                    Some(serde_json::json!({
                        "status": "blocked_unsafe",
                        "sop": sop_name,
                        "reason": reason,
                    }))
                }
                super::dispatch::DispatchResult::NoMatch => None,
            })
            .collect::<Vec<_>>();
        Ok(Some(clawcrew_api::sop::WebhookDispatchResult {
            blocked_only,
            results: mapped,
        }))
    }

    fn get_run_detail(&self, run_id: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let Some(engine) = &self.engine else {
            return Ok(None);
        };
        match super::run_detail_for(engine, run_id) {
            Ok((run, _active)) => Ok(Some(serde_json::to_value(run)?)),
            Err(e) if e.to_string().contains("not found") => Ok(None),
            Err(e) => Err(e),
        }
    }
}
