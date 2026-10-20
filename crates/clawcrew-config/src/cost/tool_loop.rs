//! Cost tracking context and accumulators for the agent tool execution loop.

use parking_lot::Mutex;
use std::sync::Arc;

use super::pricing::{ModelProviderPricing, build_model_provider_pricing};
use super::tracker::CostTracker;
use crate::schema::Config;

/// Per-scope token/cost accumulator derived from the usage events emitted
/// during a single task-local runtime invocation.
#[derive(Default, Clone, Copy, Debug)]
pub struct TurnUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub last_input_tokens: u64,
}

pub fn tool_loop_cost_tracking_context_for_agent(
    config: &Config,
    agent_alias: &str,
) -> Option<ToolLoopCostTrackingContext> {
    CostTracker::get_or_init_global(config.cost.clone(), &config.data_dir)
        .map(|tracker| tool_loop_cost_tracking_context_from_tracker(config, agent_alias, tracker))
}

pub fn tool_loop_cost_tracking_context_from_tracker(
    config: &Config,
    agent_alias: &str,
    tracker: Arc<CostTracker>,
) -> ToolLoopCostTrackingContext {
    ToolLoopCostTrackingContext::new(tracker, Arc::new(build_model_provider_pricing(config)))
        .with_agent_alias(agent_alias)
}

/// Context for cost tracking within the tool call loop.
/// Scoped via `tokio::task_local!` at call sites (channels, gateway).
#[derive(Clone)]
pub struct ToolLoopCostTrackingContext {
    pub tracker: Option<Arc<CostTracker>>,
    pub model_provider_pricing: Arc<ModelProviderPricing>,
    pub turn_usage: Arc<Mutex<TurnUsage>>,
    pub agent_alias: Option<String>,
}

impl ToolLoopCostTrackingContext {
    pub fn new(
        tracker: Arc<CostTracker>,
        model_provider_pricing: Arc<ModelProviderPricing>,
    ) -> Self {
        Self {
            tracker: Some(tracker),
            model_provider_pricing,
            turn_usage: Arc::new(Mutex::new(TurnUsage::default())),
            agent_alias: None,
        }
    }

    pub fn usage_only() -> Self {
        Self {
            tracker: None,
            model_provider_pricing: Arc::new(ModelProviderPricing::new()),
            turn_usage: Arc::new(Mutex::new(TurnUsage::default())),
            agent_alias: None,
        }
    }

    #[must_use]
    pub fn with_agent_alias(mut self, agent_alias: impl Into<String>) -> Self {
        self.agent_alias = Some(agent_alias.into());
        self
    }

    pub fn snapshot_turn_usage(&self) -> TurnUsage {
        TOOL_LOOP_TURN_USAGE
            .try_with(|turn_usage| turn_usage.as_ref().map(|u| *u.lock()).unwrap_or_default())
            .unwrap_or_else(|_| *self.turn_usage.lock())
    }
}

tokio::task_local! {
    pub static TOOL_LOOP_COST_TRACKING_CONTEXT: Option<ToolLoopCostTrackingContext>;
}

tokio::task_local! {
    pub static TOOL_LOOP_TURN_USAGE: Option<Arc<Mutex<TurnUsage>>>;
}
