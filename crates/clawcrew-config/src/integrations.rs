//! Integration status and catalog metadata types.

use crate::schema::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IntegrationStatus {
    /// Fully implemented and ready to use
    Available,
    /// Configured and active
    Active,
}

/// Integration category
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IntegrationCategory {
    Chat,
    AiModel,
    AiRouter,
    ToolsAutomation,
    Platform,
}

impl IntegrationCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "Chat Providers",
            Self::AiModel => "AI Models",
            Self::AiRouter => "AI Router",
            Self::ToolsAutomation => "Tools & Automation",
            Self::Platform => "Platforms",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Chat,
            Self::AiModel,
            Self::AiRouter,
            Self::ToolsAutomation,
            Self::Platform,
        ]
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntegrationEntry {
    pub name: String,
    pub description: String,
    pub category: IntegrationCategory,
    pub status: IntegrationStatus,
    /// Canonical config map key for entries backed by a schema config slot —
    /// the model-provider family key for AI-model entries and the
    /// `ChannelsConfig` map key for chat entries.
    pub key: Option<String>,
}

/// Provider trait for resolving the integration catalog.
pub trait IntegrationCatalogProvider: Send + Sync {
    fn all_integrations(&self, config: &Config) -> Vec<IntegrationEntry>;
}
