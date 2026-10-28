//! Quickstart interface, traits, and DTO types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::presets::BuilderSubmission;
use crate::schema::Config;
use crate::traits::PropKind;

/// Which surface invoked the Quickstart.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Web,
    Tui,
    Cli,
    Test,
}

impl Surface {
    pub fn as_str(self) -> &'static str {
        match self {
            Surface::Web => "web",
            Surface::Tui => "tui",
            Surface::Cli => "cli",
            Surface::Test => "test",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedAgent {
    pub alias: String,
    pub model_provider: String,
    pub risk_profile: String,
    pub runtime_profile: String,
    pub channels: Vec<String>,
    pub memory_backend: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuickstartStep {
    ModelProvider,
    RiskProfile,
    RuntimeProfile,
    Memory,
    Channels,
    PeerGroups,
    Agent,
}

impl QuickstartStep {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::ModelProvider => "Model provider",
            Self::RiskProfile => "Risk profile",
            Self::RuntimeProfile => "Runtime profile",
            Self::Memory => "Memory",
            Self::Channels => "Channels",
            Self::PeerGroups => "Peer groups",
            Self::Agent => "Agent",
        }
    }

    #[must_use]
    pub fn label_key(self) -> &'static str {
        match self {
            Self::ModelProvider => "cli-quickstart-step-model-provider",
            Self::RiskProfile => "cli-quickstart-step-risk-profile",
            Self::RuntimeProfile => "cli-quickstart-step-runtime-profile",
            Self::Memory => "cli-quickstart-step-memory",
            Self::Channels => "cli-quickstart-step-channels",
            Self::PeerGroups => "cli-quickstart-step-peer-groups",
            Self::Agent => "cli-quickstart-step-agent",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickstartError {
    pub step: QuickstartStep,
    pub field: String,
    pub message: String,
}

impl QuickstartError {
    pub fn new(step: QuickstartStep, field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            step,
            field: field.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for QuickstartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for QuickstartError {}

/// Selector kinds that the Quickstart "field shape" descriptor covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldSection {
    ModelProvider,
    Channel,
    PeerGroup,
}

/// Dynamic field descriptor for Quickstart inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FieldDescriptor {
    pub key: String,
    pub label: String,
    pub help: String,
    pub kind: PropKind,
    pub is_secret: bool,
    pub enum_variants: Option<Vec<String>>,
    pub required: bool,
    pub default: Option<String>,
}

/// Abstract runner trait for quickstart operations, decoupling callers from runtime.
#[async_trait]
pub trait QuickstartRunner: Send + Sync {
    fn snapshot_state(&self, config: &Config) -> serde_json::Value;
    fn field_shape(&self, section: FieldSection, type_key: &str) -> Vec<FieldDescriptor>;
    fn validate_only(
        &self,
        submission: &BuilderSubmission,
        config: &Config,
        surface: Surface,
    ) -> Result<(), Vec<QuickstartError>>;
    async fn apply(
        &self,
        submission: BuilderSubmission,
        config: &mut Config,
        surface: Surface,
    ) -> Result<AppliedAgent, Vec<QuickstartError>>;
    fn record_dismissed(
        &self,
        run_id: &str,
        surface: Surface,
        last_step: Option<QuickstartStep>,
    );
}
