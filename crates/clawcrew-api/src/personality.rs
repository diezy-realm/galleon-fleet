use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PersonalityFileEntry {
    pub filename: String,
    pub exists: bool,
    #[serde(default)]
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PersonalityListResult {
    pub files: Vec<PersonalityFileEntry>,
    pub max_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PersonalityGetResult {
    pub filename: String,
    #[serde(default)]
    pub content: Option<String>,
    pub exists: bool,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PersonalityPutResult {
    pub bytes_written: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TemplateFileEntry {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PersonalityTemplatesResult {
    pub preset: String,
    pub files: Vec<TemplateFileEntry>,
}

/// Maximum characters per personality file before truncation.
pub const MAX_FILE_CHARS: usize = 20_000;

/// Well-known editable personality files.
pub const EDITABLE_PERSONALITY_FILES: &[&str] = &[
    "SOUL.md",
    "IDENTITY.md",
    "USER.md",
    "AGENTS.md",
    "TOOLS.md",
    "HEARTBEAT.md",
    "MEMORY.md",
];

/// Per-render context for personality file starter templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    pub agent: String,
    pub user: String,
    pub timezone: String,
    pub communication_style: String,
    /// When `false`, omits MEMORY.md from the rendered set and renders
    /// the no-memory variant of AGENTS.md.
    pub include_memory: bool,
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self {
            agent: "ClawCrew".to_string(),
            user: "User".to_string(),
            timezone: "UTC".to_string(),
            communication_style:
                "Be warm, natural, and clear. Use occasional relevant emojis (1-2 max) and avoid robotic phrasing."
                    .to_string(),
            include_memory: true,
        }
    }
}

/// Runner trait implemented by the runtime to render and seed default personality templates.
#[async_trait::async_trait]
pub trait PersonalityRunner: Send + Sync {
    fn render_templates(&self, ctx: &TemplateContext) -> Vec<TemplateFileEntry>;
    async fn seed_default_personality(
        &self,
        alias: &str,
        workspace_dir: &std::path::Path,
        include_memory: bool,
    ) -> std::io::Result<Vec<String>>;
}
