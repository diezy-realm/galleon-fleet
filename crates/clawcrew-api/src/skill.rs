use std::collections::BTreeMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::attribution::{Attributable, Role};

/// A skill is a user-defined or community-built capability.
/// Skills live in `~/.clawcrew/workspace/skills/<name>/SKILL.md`
/// and can include tool definitions, prompts, and automation scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    /// Per-locale translations of `description`, keyed by Discord locale code
    /// (e.g. `fr`, `es-ES`, `ja`). Consumed by slash-capable channels to
    /// localize the command description; empty for unlocalized skills. Declared
    /// in SKILL.toml under `[skill]` as `description_localizations`.
    #[serde(default)]
    pub description_localizations: BTreeMap<String, String>,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tools: Vec<SkillTool>,
    #[serde(default)]
    pub prompts: Vec<String>,
    /// Typed slash-command options a `slash`-tagged skill exposes (e.g. on
    /// Discord). Empty for skills that take no structured input — slash channels
    /// then fall back to a single free-text option. See [`SkillSlashOption`].
    #[serde(default)]
    pub slash_options: Vec<SkillSlashOption>,
    /// When `true`, this skill's full instructions stay inlined in the system
    /// prompt even in [`clawcrew_config::schema::SkillsPromptInjectionMode::Compact`]
    /// mode, instead of being loaded on demand via `read_skill(name)`. Intended
    /// for policy/critical skills that must always be visible to the model.
    #[serde(default)]
    pub always: bool,
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

impl Attributable for Skill {
    fn role(&self) -> Role {
        Role::Skill
    }
    fn alias(&self) -> &str {
        &self.name
    }
}

/// A tool defined by a skill (shell command, HTTP call, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    /// "shell", "http", "script", "builtin", "mcp"
    pub kind: String,
    /// The command/URL/script to execute (unused for builtin/mcp kinds)
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: std::collections::HashMap<String, String>,
    /// For `kind = "builtin"`: the name of the built-in tool to delegate to.
    /// For `kind = "mcp"`: the prefixed MCP tool name `{server}__{tool}`
    /// (e.g. `images__generate`).
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default, alias = "default_args")]
    pub locked_args: std::collections::HashMap<String, String>,
    /// For `kind = "shell"` / `kind = "script"`: maximum execution time in
    /// seconds before the command is killed. Unset falls back to the built-in
    /// `SKILL_SHELL_TIMEOUT_SECS` (60s) default; long-running skills (e.g. a
    /// build pipeline) raise it via `timeout_secs` in SKILL.toml.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlashOptionKind {
    String,
    Integer,
    Number,
    Boolean,
    User,
    Channel,
    Role,
    Mentionable,
}

impl SlashOptionKind {
    /// Every kind, in the order surfaces should offer them. Walked (not
    /// restated) by every registry consumer.
    pub const ALL: [Self; 8] = [
        Self::String,
        Self::Integer,
        Self::Number,
        Self::Boolean,
        Self::User,
        Self::Channel,
        Self::Role,
        Self::Mentionable,
    ];

    /// The canonical `type` token written in frontmatter.
    pub fn manifest_name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::User => "user",
            Self::Channel => "channel",
            Self::Role => "role",
            Self::Mentionable => "mentionable",
        }
    }

    /// Predefined `choices` apply only to string/integer/number options.
    pub fn supports_choices(self) -> bool {
        match self {
            Self::String | Self::Integer | Self::Number => true,
            Self::Boolean | Self::User | Self::Channel | Self::Role | Self::Mentionable => false,
        }
    }

    /// `min`/`max` numeric bounds apply only to integer/number options.
    pub fn supports_numeric_bounds(self) -> bool {
        match self {
            Self::Integer | Self::Number => true,
            Self::String
            | Self::Boolean
            | Self::User
            | Self::Channel
            | Self::Role
            | Self::Mentionable => false,
        }
    }

    /// `min_length`/`max_length` bounds apply only to string options.
    pub fn supports_length_bounds(self) -> bool {
        match self {
            Self::String => true,
            Self::Integer
            | Self::Number
            | Self::Boolean
            | Self::User
            | Self::Channel
            | Self::Role
            | Self::Mentionable => false,
        }
    }

    /// The wire-facing capability row for this kind, consumed by API surfaces.
    pub fn descriptor(self) -> SlashOptionKindDescriptor {
        SlashOptionKindDescriptor {
            manifest_name: self.manifest_name().to_string(),
            supports_choices: self.supports_choices(),
            supports_numeric_bounds: self.supports_numeric_bounds(),
            supports_length_bounds: self.supports_length_bounds(),
        }
    }
}

/// Serialized capability row for one [`SlashOptionKind`], as published to
/// surfaces (the web dashboard mirrors this shape). Built by walking
/// [`SlashOptionKind::ALL`]; never hand-authored.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SlashOptionKindDescriptor {
    pub manifest_name: String,
    pub supports_choices: bool,
    pub supports_numeric_bounds: bool,
    pub supports_length_bounds: bool,
}

/// The full registry, produced by exhaustively walking [`SlashOptionKind::ALL`].
pub fn slash_option_kinds() -> Vec<SlashOptionKindDescriptor> {
    SlashOptionKind::ALL
        .into_iter()
        .map(SlashOptionKind::descriptor)
        .collect()
}

/// A typed option a `slash`-tagged skill exposes on its slash command. Shaped
/// after the Discord Application Command Option model but channel-agnostic; a
/// slash-capable channel maps `kind` to its wire option type. Declared in
/// SKILL.toml under `[[skill.slash_options]]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillSlashOption {
    pub name: String,
    pub description: String,
    /// Per-locale translations of `description`, keyed by Discord locale code.
    /// Empty for unlocalized options. Declared under
    /// `[[skill.slash_options]]` as `description_localizations`.
    #[serde(default)]
    pub description_localizations: BTreeMap<String, String>,
    /// `string` | `integer` | `number` | `boolean` | `user` | `channel` |
    /// `role` | `mentionable`. Unknown values are dropped by the channel.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    /// Predefined choices (string/integer/number options only). The `value` is
    /// kept as text and coerced to the option's type by the channel.
    #[serde(default)]
    pub choices: Vec<SkillSlashChoice>,
    /// Inclusive bounds for integer/number options.
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    /// Length bounds for string options.
    #[serde(default)]
    pub min_length: Option<u32>,
    #[serde(default)]
    pub max_length: Option<u32>,
}

/// A predefined choice for a typed slash option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillSlashChoice {
    pub name: String,
    pub value: String,
}
