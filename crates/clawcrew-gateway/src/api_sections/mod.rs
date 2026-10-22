//! Curated config-section endpoints. Used by the `/config` page in the
//! web dashboard to navigate the schema by curated section rather than
//! raw prop paths. OpenAPI is authoritative for the exact route set.

use serde::{Deserialize, Serialize};

pub mod handlers;

#[cfg(test)]
pub(crate) mod tests;

pub use handlers::*;

// ── Request / response DTOs ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ModelsQuery {
    /// ModelProvider name (canonical, from CatalogModelProvider.name).
    /// `provider` alias matches the query-string name the web dashboard uses.
    #[serde(alias = "provider")]
    pub model_provider: String,
}

/// All alias-reference choices an agent form needs, in one round-trip.
/// Channels and model model_providers are returned in dotted form
/// (`telegram.default`, `anthropic.work`); the bundle/profile/namespace
/// lists are bare HashMap keys.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct AgentOptionsResponse {
    pub channels: Vec<String>,
    /// Distinct channel types with at least one configured alias —
    /// `["discord", "telegram"]`. Source for peer-group channel picker.
    pub channel_types: Vec<String>,
    pub model_providers: Vec<String>,
    pub risk_profiles: Vec<String>,
    pub runtime_profiles: Vec<String>,
    pub skill_bundles: Vec<String>,
    pub knowledge_bundles: Vec<String>,
    pub mcp_bundles: Vec<String>,
    pub agents: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SectionPath {
    pub section: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SectionItemPath {
    pub section: String,
    pub key: String,
}

#[derive(Debug, Default, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SectionSelectBody {
    pub alias: Option<String>,
}
