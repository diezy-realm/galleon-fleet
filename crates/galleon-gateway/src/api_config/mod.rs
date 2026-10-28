//! Per-property CRUD endpoints for `/api/config/*`.

use serde::{Deserialize, Serialize};

use super::AppState;
use super::ConfigWriteGuard;
use super::api::require_auth;

pub mod drift;
pub mod handlers;
pub mod patch;

#[cfg(test)]
pub(crate) mod tests;

pub use drift::*;
pub use handlers::*;
pub use patch::*;

#[cfg(test)]
pub(crate) use galleon_config::typed_value::coerce_for_set_prop as json_to_setprop_string;

pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

// ── Request / response shapes ───────────────────────────────────────

/// `?path=...` query parameter shared by GET / DELETE / OPTIONS-with-path.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct PropQuery {
    pub path: String,
}

/// `?prefix=...` query parameter for list.
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ListQuery {
    #[serde(default)]
    pub prefix: Option<String>,
}

/// PUT body. Value is `serde_json::Value` so typed values (booleans, arrays,
/// numbers) round-trip correctly without going through the CLI's
/// comma-delimited string parser.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct PropPutBody {
    pub path: String,
    pub value: serde_json::Value,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Response for a non-secret GET / PUT / DELETE.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct PropResponse {
    pub path: String,
    pub value: serde_json::Value,
    /// Non-fatal validation warnings against the current config state.
    /// On GET this surfaces warnings present in the loaded config; on PUT
    /// this surfaces warnings against the post-save state. Empty when
    /// nothing is flagged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<galleon_config::validation_warnings::ValidationWarning>,
}

/// Response for a secret GET / PUT / DELETE — never carries the value or its
/// length. `populated: true` means the secret has a non-empty value on disk;
/// `populated: false` means the field is unset or empty.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SecretResponse {
    pub path: String,
    pub populated: bool,
}

/// Single entry in the list response for configuration properties.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ListEntry {
    pub path: String,
    pub category: String,
    /// Stable kind tag — `string`, `bool`, `integer`, `float`, `enum`,
    /// `string-array`. Lowercase-kebab so it can be used directly as a CSS
    /// class or React key.
    pub kind: &'static str,
    /// Rust type signature, e.g. `Option<String>`, `Vec<String>`, `u64`.
    /// Render in tooltips / hover state for the technically-curious.
    pub type_hint: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    pub populated: bool,
    pub is_secret: bool,
    /// Whether this field was populated by a `CLAWCREW_*` env-var override
    /// at load time. The dashboard renders the 💉 badge and a persistent
    /// warning *"Edits here won't take effect — overridden by CLAWCREW_..."*
    /// when this is `true`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_env_overridden: bool,
    /// Variants for `enum`-kind fields — non-empty means the frontend should
    /// render a `<select>` with these options. Empty for non-enum fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_variants: Vec<String>,
    /// Onboard section name derived from the path's first segment via
    /// `Section::from_path`. `None` for paths that aren't part of any wizard
    /// section. The dashboard groups list entries by this for per-section
    /// rendering — same source the CLI wizard uses, no schema attribute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<&'static str>,
    /// Tab grouping label from the field's `#[tab(...)]` annotation
    /// (`ConfigTab::label`). Absent for `ConfigTab::None`. Surfaces group
    /// list entries into a tab bar by this; the agent edit form depends on
    /// it to split General / Providers / Channels / etc.
    #[serde(skip_serializing_if = "str::is_empty")]
    pub tab: &'static str,
    /// Surface hint from `#[multiline]`: render a multi-line text area
    /// (e.g. a PEM key body) instead of a single-line input.
    #[serde(default, skip_serializing_if = "is_false")]
    pub multiline: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ListResponse {
    pub entries: Vec<ListEntry>,
    /// Properties where in-memory and on-disk values disagree. Empty when the
    /// daemon's view matches the file. Each entry follows the `DriftEntry`
    /// shape (secrets carry only `{path, secret: true, drifted: true}`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drifted: Vec<DriftEntry>,
}

/// `POST /api/channels/bind` request body. The GUI/HTTP equivalent of
/// `clawcrew channel bind-<type> <identity> --alias <alias>`: authorize an
/// operator-named identity on one channel alias without the in-chat
/// `/bind <code>` round trip.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ChannelBindBody {
    pub channel_type: String,
    pub alias: String,
    pub identity: String,
}

/// POST /api/channels/bind — add an inbound identity to a pairing channel's
/// allowlist. Shares the exact bind core the CLI uses
/// (`bind_channel_identity_into`), writes ONLY to
/// `peer_groups.<type>_<alias>.external_peers`, and is gated by the same

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct DriftResponse {
    pub drifted: Vec<DriftEntry>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct ReloadStatusResponse {
    /// Whether any config write has landed since the last admin reload and may
    /// still require subsystem re-instantiation to take effect.
    pub pending_reload: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct MapKeyQuery {
    /// Map-keyed section path, e.g. `providers.models`, `agents`, `risk_profiles`.
    pub path: String,
    /// New key to insert under that section, e.g. `anthropic`.
    pub key: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct MapKeyResponse {
    pub path: String,
    pub key: String,
    pub created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateEntry>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct TemplateEntry {
    pub path: &'static str,
    /// `map` for `HashMap<String, T>`, `list` for `Vec<T>`.
    pub kind: &'static str,
    /// Rust type name of the value, e.g. `ModelProviderConfig`.
    pub value_type: &'static str,
    /// Doc comment from the schema (description of what gets added).
    pub description: &'static str,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct MapPathQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct AliasSourceQuery {
    pub source: galleon_config::traits::AliasSource,
}

/// A single config reference site to an aliased entry, for the delete preview.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct RefSiteDto {
    /// Dotted config path that references the alias, e.g.
    /// `agents.forge.model_provider` or `heartbeat.agent`.
    pub path: String,
    /// The stored reference text, e.g. `anthropic.default`.
    pub raw_value: String,
}

/// Dry-run impact of deleting an aliased entry — the cascade preview a surface
/// renders before confirming. Pure/read-only: computed from `plan_delete` (the
/// same reference walk the real delete uses) plus the live-ACP gate for agents.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct DeletePlanResponse {
    pub path: String,
    pub key: String,
    /// True iff nothing HARD blocks the delete (no hard config reference and,
    /// for agents, no live ACP session). Mirrors the real delete's refusal gate.
    pub allowed: bool,
    /// HARD references that block the delete — the operator must change these
    /// first (e.g. an enabled `heartbeat.agent`).
    pub blockers: Vec<RefSiteDto>,
    /// SOFT references the delete would scrub automatically.
    pub scrubs: Vec<RefSiteDto>,
    /// Agent delete only: number of live ACP sessions (a non-zero count blocks
    /// the delete; `null` for non-agent sections or if the count couldn't be
    /// read — in which case the delete fails closed too).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_acp_sessions: Option<usize>,
    /// Agent delete only: the agent's owned non-config state (memory / cron /
    /// session history) is exported and removed on delete. Counts are not
    /// enumerated in the preview.
    pub cascades_owned_state: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct RenameMapKeyBody {
    /// Section path, e.g. `channels.discord` or `model_providers.anthropic`.
    pub path: String,
    /// Current alias name.
    pub from: String,
    /// New alias name.
    pub to: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct RenameMapKeyResponse {
    pub path: String,
    pub from: String,
    pub to: String,
    pub renamed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct InitQuery {
    /// Optional section prefix to scope the init pass (e.g. `model_providers`).
    /// Without it, every uninitialized nested section gets its defaults.
    #[serde(default)]
    pub section: Option<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct InitResponse {
    pub initialized: Vec<String>,
}

/// POST /api/config/init?section=model_providers — instantiate `None` nested
/// sections with defaults, and only those: dynamic-map aliases are created

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct MigrateResponse {
    pub migrated: bool,
    /// Backup path written when migration ran; absent when the config was
    /// already at the current schema version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    pub schema_version: u32,
}
