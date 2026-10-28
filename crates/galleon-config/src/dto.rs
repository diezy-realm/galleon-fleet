use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogModelProvider {
    pub name: String,
    pub display_name: String,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogResponse {
    pub model_providers: Vec<CatalogModelProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CatalogModelsResult {
    pub model_provider: String,
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pricing:
        Option<std::collections::HashMap<String, galleon_api::model_provider::ModelPricing>>,
    pub local: bool,
    pub live: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigSectionEntry {
    pub key: String,
    pub label: String,
    pub help: String,
    pub has_picker: bool,
    pub completed: bool,
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub group_key: String,
    #[serde(default)]
    pub is_quickstart: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<crate::sections::SectionShape>,
    #[serde(default)]
    pub cost_category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigSectionsResult {
    pub sections: Vec<ConfigSectionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigStatusResult {
    pub needs_quickstart: bool,
    pub reason: String,
    pub has_partial_state: bool,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PickerItem {
    pub key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PickerResponse {
    pub section: String,
    pub items: Vec<PickerItem>,
    pub help: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SelectItemResponse {
    pub fields_prefix: String,
    pub created: bool,
}
