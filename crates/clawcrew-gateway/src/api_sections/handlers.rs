//! Route handlers and helpers for curated config sections.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use clawcrew_config::api_error::{ConfigApiCode, ConfigApiError};
use clawcrew_config::dto::{
    CatalogModelProvider, CatalogModelsResult, CatalogResponse, ConfigSectionEntry,
    ConfigSectionsResult, ConfigStatusResult, PickerItem, PickerResponse, SelectItemResponse,
};

use crate::AppState;
use crate::api::require_auth;
use crate::api_config::{persist_and_swap, try_compute_drift};

use super::{AgentOptionsResponse, ModelsQuery, SectionItemPath, SectionPath, SectionSelectBody};
/// `GET /api/config/catalog` — list every model provider the CLI wizard knows
/// about. The dashboard shows these in the "+ Add model provider" picker so
/// CLI / web stay in sync.
pub async fn handle_catalog(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let _ = state;

    let model_providers: Vec<CatalogModelProvider> = clawcrew_providers::list_model_providers()
        .into_iter()
        .map(|p| CatalogModelProvider {
            name: p.name.to_string(),
            display_name: p.display_name.to_string(),
            local: p.local,
        })
        .collect();

    axum::Json(CatalogResponse { model_providers }).into_response()
}

pub async fn handle_catalog_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ModelsQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let local = clawcrew_providers::model_provider_is_local(&q.model_provider);
    // Snapshot config so the catalog resolves the alias credential and can reach
    // the native /models endpoint (surfacing new native-only models that the
    // models.dev snapshot may not carry yet) instead of silently falling back.
    let cfg = state.config.read().clone();
    let (models, pricing, live) =
        clawcrew_providers::catalog::model_catalog_with_config(Some(&cfg), &q.model_provider).await;
    axum::Json(CatalogModelsResult {
        model_provider: q.model_provider,
        models,
        pricing,
        local,
        live,
    })
    .into_response()
}

pub(crate) fn error_response(err: ConfigApiError) -> Response {
    let status = axum::http::StatusCode::from_u16(err.code.http_status())
        .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::Json(err)).into_response()
}

// ── Section + picker (mirrors the TUI flow) ──────────────────────────

#[must_use]
pub fn derive_section_status(cfg: &clawcrew_config::schema::Config) -> ConfigStatusResult {
    let missing = quickstart_missing_requirements(cfg);
    let ready = missing.is_empty();
    let has_partial_state = !cfg.onboard_state.completed_sections.is_empty()
        || cfg.providers.models.iter_entries().next().is_some()
        || !cfg.risk_profiles.is_empty()
        || !cfg.runtime_profiles.is_empty()
        || !cfg.agents.is_empty();
    let reason = if ready {
        "has_dispatchable_agent"
    } else if has_partial_state {
        "incomplete_agent"
    } else {
        "fresh_install"
    };
    ConfigStatusResult {
        needs_quickstart: !ready,
        reason: reason.to_string(),
        has_partial_state,
        missing,
    }
}

pub(crate) fn quickstart_missing_requirements(
    cfg: &clawcrew_config::schema::Config,
) -> Vec<String> {
    let mut missing = Vec::new();
    if cfg.providers.models.iter_entries().next().is_none() {
        missing.push("Add a model provider.".to_string());
    }
    if cfg.agents.is_empty() {
        missing.push("Create an agent.".to_string());
        return missing;
    }

    let mut agent_aliases: Vec<&String> = cfg.agents.keys().collect();
    agent_aliases.sort();
    let mut has_dispatchable_agent = false;
    for alias in agent_aliases {
        let agent_missing = quickstart_agent_missing_requirements(cfg, alias, &cfg.agents[alias]);
        if agent_missing.is_empty() {
            has_dispatchable_agent = true;
            break;
        }
        missing.extend(agent_missing);
    }
    if has_dispatchable_agent {
        missing.clear();
    }
    missing
}

pub(crate) fn quickstart_agent_missing_requirements(
    cfg: &clawcrew_config::schema::Config,
    alias: &str,
    agent: &clawcrew_config::schema::AliasedAgentConfig,
) -> Vec<String> {
    let mut missing = Vec::new();
    if !agent.enabled {
        missing.push(format!("Enable agent `{alias}`."));
    }

    let model_ref = agent.model_provider.trim();
    if model_ref.is_empty() {
        missing.push(format!("Set a model provider for agent `{alias}`."));
    } else if let Some((family, _, provider)) = cfg.resolved_model_provider_for_agent(alias) {
        let has_model = provider
            .model
            .as_deref()
            .map(str::trim)
            .is_some_and(|m| !m.is_empty());
        if !has_model {
            missing.push(format!("Choose a model for model provider `{model_ref}`."));
        } else if !model_provider_alias_usable(
            provider,
            clawcrew_providers::model_provider_is_local(family),
        ) {
            missing.push(format!(
                "Set credential/auth for model provider `{model_ref}`."
            ));
        }
    } else {
        missing.push(format!(
            "Fix agent `{alias}` model provider `{model_ref}`; it does not resolve to a configured provider."
        ));
    }

    let risk_ref = agent.risk_profile.trim();
    if risk_ref.is_empty() {
        missing.push(format!("Set a risk profile for agent `{alias}`."));
    } else if !cfg.risk_profiles.contains_key(risk_ref) {
        missing.push(format!(
            "Fix agent `{alias}` risk profile `{risk_ref}`; it does not resolve to a configured profile."
        ));
    }

    let runtime_ref = agent.runtime_profile.trim();
    if runtime_ref.is_empty() {
        missing.push(format!("Set a runtime profile for agent `{alias}`."));
    } else if !cfg.runtime_profiles.contains_key(runtime_ref) {
        missing.push(format!(
            "Fix agent `{alias}` runtime profile `{runtime_ref}`; it does not resolve to a configured profile."
        ));
    }

    missing
}

pub async fn handle_section_status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let cfg = state.config.read().clone();
    axum::Json(derive_section_status(&cfg)).into_response()
}
pub fn build_agent_options(cfg: &clawcrew_config::schema::Config) -> AgentOptionsResponse {
    use clawcrew_config::traits::AliasSource;

    let channels = cfg.resolve_alias_source(AliasSource::Channels);
    let mut channel_types: Vec<String> = channels
        .iter()
        .filter_map(|d| d.split_once('.').map(|(t, _)| t.to_string()))
        .collect();
    channel_types.sort();
    channel_types.dedup();

    AgentOptionsResponse {
        channels,
        channel_types,
        model_providers: cfg.resolve_alias_source(AliasSource::ModelProviders),
        risk_profiles: cfg.resolve_alias_source(AliasSource::RiskProfiles),
        runtime_profiles: cfg.resolve_alias_source(AliasSource::RuntimeProfiles),
        skill_bundles: cfg.resolve_alias_source(AliasSource::SkillBundles),
        knowledge_bundles: cfg.resolve_alias_source(AliasSource::KnowledgeBundles),
        mcp_bundles: cfg.resolve_alias_source(AliasSource::McpBundles),
        agents: cfg.resolve_alias_source(AliasSource::Agents),
    }
}

/// `GET /api/config/agent-options` — every alias-reference list the
/// agent form needs, derived from the live config. Mirrors the lists the
/// TUI computes locally for its alias pickers.
pub async fn handle_agent_options(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let cfg = state.config.read().clone();
    axum::Json(build_agent_options(&cfg)).into_response()
}

pub async fn handle_sections(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let cfg = state.config.read().clone();
    let completed: std::collections::HashSet<String> = cfg
        .onboard_state
        .completed_sections
        .iter()
        .cloned()
        .collect();

    // First segment of every reachable prop path. BTreeSet for stable
    // alphabetical order and dedup.
    let mut roots: std::collections::BTreeSet<String> = cfg
        .prop_fields()
        .iter()
        .filter_map(|f| f.name.split('.').next().map(str::to_string))
        .collect();

    // System / housekeeping fields the user never edits via the dashboard.
    for hidden in HIDDEN_TOP_LEVEL {
        roots.remove(*hidden);
    }

    let all_map_paths: Vec<&'static str> = clawcrew_config::schema::Config::map_key_sections()
        .iter()
        .map(|s| s.path)
        .collect();
    let section_has_picker_for_key = |key: &str| -> bool {
        let key_dot = format!("{key}.");
        all_map_paths.iter().any(|p| {
            *p == key
                || p.strip_prefix(&key_dot)
                    .is_some_and(|rest| !rest.contains('.'))
        })
    };

    // Ensure map-keyed sections surface as sidebar entries even when their
    // HashMap is empty (prop_fields() only yields paths for populated
    // entries). First segments only — the prefix-dedup pass below drops
    // bare parent segments when a multi-segment child is present.
    let map_keyed_roots: std::collections::HashSet<&'static str> = all_map_paths
        .iter()
        .filter_map(|p| p.split('.').next())
        .collect();
    for &prefix in &map_keyed_roots {
        roots.insert(prefix.to_string());
    }

    // Synthetic curated sections — keys that aren't fields on Config
    // but are part of the wizard flow (personality lives as markdown
    // files, not TOML). Inject so the canonical-order sort places them
    // correctly and frontends don't need to know which ones to splice.
    for s in clawcrew_config::sections::QUICKSTART_SECTIONS {
        roots.insert(s.as_str().to_string());
    }

    // Drop bare parent-segment entries when a dotted child is present
    // — `providers` is phantom once `providers.models` etc. are listed.
    let prefixes_with_children: std::collections::HashSet<String> = roots
        .iter()
        .filter_map(|k| k.split_once('.').map(|(parent, _)| parent.to_string()))
        .collect();
    roots.retain(|k| k.contains('.') || !prefixes_with_children.contains(k));

    roots.retain(|k| !k.starts_with("cost.rates"));

    let mut ordered: Vec<String> = roots.into_iter().collect();
    ordered.sort_by(|a, b| {
        match (
            clawcrew_config::sections::section_index_for_key(a),
            clawcrew_config::sections::section_index_for_key(b),
        ) {
            (Some(ai), Some(bi)) => ai.cmp(&bi),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.cmp(b),
        }
    });

    let sections: Vec<ConfigSectionEntry> = ordered
        .into_iter()
        .map(|key| {
            let wizard = clawcrew_config::sections::Section::from_key(&key);
            let has_picker = match wizard {
                Some(w) => !matches!(
                    w,
                    clawcrew_config::sections::Section::Hardware
                        | clawcrew_config::sections::Section::Mcp
                        | clawcrew_config::sections::Section::Skills
                ),
                None => section_has_picker_for_key(&key),
            };
            let group = clawcrew_config::sections::section_group_for_key(&key);
            ConfigSectionEntry {
                completed: completed.contains(&key),
                ready: section_ready(&cfg, &key, completed.contains(&key)),
                label: clawcrew_config::sections::humanize_section_key(&key),
                help: section_help(&key).to_string(),
                has_picker,
                group: group.label().to_string(),
                group_key: group.key().to_string(),
                is_quickstart: wizard.is_some(),
                shape: wizard.map(clawcrew_config::sections::Section::shape),
                cost_category: clawcrew_config::schema::cost_category_for_provider_section(&key)
                    .unwrap_or_default()
                    .to_string(),
                key,
            }
        })
        .collect();

    axum::Json(ConfigSectionsResult { sections }).into_response()
}

pub(crate) fn section_ready(
    cfg: &clawcrew_config::schema::Config,
    key: &str,
    completed_marker: bool,
) -> bool {
    use clawcrew_config::sections::Section;
    match Section::from_key(key) {
        Some(Section::ModelProviders) => any_usable_model_provider(cfg),
        Some(Section::RiskProfiles) => !cfg.risk_profiles.is_empty(),
        Some(Section::RuntimeProfiles) => !cfg.runtime_profiles.is_empty(),
        Some(Section::Storage) => cfg
            .prop_fields()
            .iter()
            .any(|field| field.name.starts_with("storage.")),
        Some(Section::Memory) => completed_marker,
        Some(Section::Agents) => cfg.agents.iter().any(|(alias, agent)| {
            quickstart_agent_missing_requirements(cfg, alias, agent).is_empty()
        }),
        _ => completed_marker,
    }
}

/// Top-level fields that exist on `Config` but are never user-editable
/// from the dashboard (schema bookkeeping, resolved at runtime).
pub(crate) const HIDDEN_TOP_LEVEL: &[&str] = &[
    "schema_version",
    "onboard_state",
    "onboard-state",
    "config_path",
    "workspace_dir",
    "env_overridden_paths",
    "pre_override_snapshots",
];

/// Help text for a section. Delegates to `clawcrew_config::sections::section_help`
/// so gateway, CLI, and TUI all read from one source — wizard variants
/// pull from `Section::help`, everything else from the matching
/// `#[nested]` field's `///` docstring on the `Config` struct.
pub(crate) fn section_help(key: &str) -> &'static str {
    clawcrew_config::sections::section_help(key)
}

pub async fn handle_section_picker(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(SectionPath { section }): axum::extract::Path<SectionPath>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let cfg = state.config.read().clone();

    use clawcrew_config::sections::Section;
    let Some(section_enum) = Section::from_key(&section) else {
        return error_response(
            ConfigApiError::new(
                ConfigApiCode::PathNotFound,
                format!(
                    "section `{section}` has no picker; render its fields \
                     via GET /api/config/list?prefix={section}"
                ),
            )
            .with_path(section.as_str()),
        );
    };
    let help = section_help(section_enum.as_str()).to_string();
    let items = match picker_items_for(section_enum, &cfg) {
        PickerDispatch::Items(items) => items,
        PickerDispatch::DirectForm => {
            return error_response(
                ConfigApiError::new(
                    ConfigApiCode::PathNotFound,
                    format!(
                        "section `{section_enum}` is a direct-form section with no picker; \
                         render fields via GET /api/config/list?prefix={section_enum}"
                    ),
                )
                .with_path(section_enum.as_str()),
            );
        }
    };

    axum::Json(PickerResponse {
        section,
        items,
        help,
    })
    .into_response()
}

pub(crate) enum PickerDispatch {
    Items(Vec<PickerItem>),
    DirectForm,
}

/// Per-section picker dispatch. Exhaustive over [`Section`] so adding a
/// variant fails to compile until it gets a routing arm. The DRY
/// version of what the dashboard's per-section view boils down to.
pub(crate) fn picker_items_for(
    section: clawcrew_config::sections::Section,
    cfg: &clawcrew_config::schema::Config,
) -> PickerDispatch {
    use clawcrew_config::sections::Section;
    match section {
        Section::ModelProviders => PickerDispatch::Items(providers_picker(cfg)),
        // TTS / transcription share the typed-family two-tier shape. Each
        // family enumerates its picker via `schema_walk_picker(<family>)`
        // — the same machinery channels uses, so no per-section catalog
        // table to drift.
        Section::TtsProviders | Section::TranscriptionProviders => {
            PickerDispatch::Items(schema_walk_picker(cfg, section.as_str()))
        }
        Section::Memory => PickerDispatch::Items(memory_picker(cfg)),
        Section::Channels => PickerDispatch::Items(schema_walk_picker(cfg, "channels")),
        Section::Tunnel => PickerDispatch::Items(tunnel_provider_picker(cfg)),
        Section::Agents => PickerDispatch::Items(agents_picker(cfg)),
        // Storage is two-tier (`storage.<kind>.<alias>`) — same shape
        // and walker as channels and the typed-provider families.
        Section::Storage => PickerDispatch::Items(storage_picker(cfg)),
        // OneTierAliasMap explorer sections: pick a key from the live
        // HashMap. Generic walker covers every section whose schema is
        // `<section>.<alias>` (operator-named keys, no closed kind set).
        Section::PeerGroups
        | Section::Cron
        | Section::McpServers
        | Section::McpBundles
        | Section::KnowledgeBundles
        | Section::SkillBundles
        | Section::RiskProfiles
        | Section::RuntimeProfiles
        | Section::ModelRoutes
        | Section::EmbeddingRoutes => {
            PickerDispatch::Items(one_tier_alias_map_picker(cfg, section.as_str()))
        }
        Section::Hardware | Section::Mcp | Section::Skills | Section::QuickstartState => {
            PickerDispatch::DirectForm
        }
    }
}

pub(crate) fn providers_picker(cfg: &clawcrew_config::schema::Config) -> Vec<PickerItem> {
    clawcrew_providers::list_model_providers()
        .into_iter()
        .map(|p| PickerItem {
            key: p.name.to_string(),
            label: p.display_name.to_string(),
            description: if p.local {
                Some("Local — no API key required".to_string())
            } else {
                None
            },
            badge: provider_type_badge(cfg, p.name, p.local),
        })
        .collect()
}

pub(crate) fn any_usable_model_provider(cfg: &clawcrew_config::schema::Config) -> bool {
    cfg.providers
        .models
        .iter_entries()
        .any(|(family, _, base)| {
            model_provider_alias_usable(base, clawcrew_providers::model_provider_is_local(family))
        })
}

pub(crate) fn provider_type_badge(
    cfg: &clawcrew_config::schema::Config,
    family: &str,
    local: bool,
) -> Option<String> {
    let mut has_alias = false;
    let mut has_usable_alias = false;
    for (ty, _, base) in cfg.providers.models.iter_entries() {
        if ty != family {
            continue;
        }
        has_alias = true;
        if model_provider_alias_usable(base, local) {
            has_usable_alias = true;
        }
    }
    if has_usable_alias {
        Some("configured".to_string())
    } else if has_alias {
        Some("needs setup".to_string())
    } else {
        None
    }
}

pub(crate) fn model_provider_alias_usable(
    base: &clawcrew_config::schema::ModelProviderConfig,
    local: bool,
) -> bool {
    let has_model = base
        .model
        .as_deref()
        .map(str::trim)
        .is_some_and(|model| !model.is_empty());
    if !has_model {
        return false;
    }
    base.api_key
        .as_deref()
        .map(str::trim)
        .is_some_and(|key| !key.is_empty())
        || base.requires_openai_auth
        || local
}

pub(crate) fn storage_picker(cfg: &clawcrew_config::schema::Config) -> Vec<PickerItem> {
    let mut items = schema_walk_picker(cfg, "storage");
    for item in &mut items {
        item.description = storage_description(&item.key).map(str::to_string);
        if item.badge.as_deref() == Some("configured") {
            item.badge = Some("created".to_string());
        }
    }
    items.sort_by_key(|item| storage_rank(&item.key));
    items
}

pub(crate) fn storage_rank(key: &str) -> usize {
    match key {
        "sqlite" => 0,
        "postgres" => 1,
        "qdrant" => 2,
        "markdown" => 3,
        "lucid" => 4,
        _ => 99,
    }
}

pub(crate) fn storage_description(key: &str) -> Option<&'static str> {
    match key {
        "sqlite" => Some(
            "Safe default for single-node installs: file-based, zero-config, no external service.",
        ),
        "postgres" => {
            Some("Shared or multi-instance deployments that need durable server-backed storage.")
        }
        "qdrant" => {
            Some("Vector database backend for semantic search when you already run Qdrant.")
        }
        "markdown" => {
            Some("Human-readable files with simple local storage and no database service.")
        }
        "lucid" => {
            Some("Bridge to local lucid-memory CLI while keeping SQLite-style local operation.")
        }
        _ => None,
    }
}

pub(crate) fn memory_picker(cfg: &clawcrew_config::schema::Config) -> Vec<PickerItem> {
    let current = cfg.memory.backend.clone();
    let memory_completed = cfg
        .onboard_state
        .completed_sections
        .iter()
        .any(|section| section == "memory");
    galleon_memory::selectable_memory_backends()
        .iter()
        .map(|b| PickerItem {
            key: b.key.to_string(),
            label: b.label.to_string(),
            description: None,
            badge: if b.key == current && memory_completed {
                Some("active".to_string())
            } else {
                None
            },
        })
        .collect()
}

pub(crate) fn schema_walk_picker(
    cfg: &clawcrew_config::schema::Config,
    section: &str,
) -> Vec<PickerItem> {
    let prefix_with_dot = format!("{section}.");

    // Configured: any alias present on this type (has at least one entry in its HashMap).
    let configured: std::collections::BTreeSet<String> = cfg
        .prop_fields()
        .iter()
        .filter_map(|f| f.name.strip_prefix(&prefix_with_dot))
        .filter_map(|suffix| suffix.split_once('.').map(|(head, _)| head.to_string()))
        .collect();

    // All known channel/section types from schema metadata — statically known,
    // no HashMap entries needed.
    let all: std::collections::BTreeSet<String> =
        clawcrew_config::schema::Config::map_key_sections()
            .into_iter()
            .filter_map(|s| {
                s.path
                    .strip_prefix(&prefix_with_dot)
                    .filter(|rest| !rest.contains('.'))
                    .map(String::from)
            })
            .collect();

    all.into_iter()
        .map(|name| {
            // Channel configs no longer carry an `enabled` field; a channel is
            // active when an enabled agent references it. Badge = "configured" when
            // at least one alias exists, absent otherwise.
            let badge = if configured.contains(&name) {
                Some("configured".to_string())
            } else {
                None
            };
            PickerItem {
                key: name.clone(),
                label: name.clone(),
                description: None,
                badge,
            }
        })
        .collect()
}

pub(crate) fn one_tier_alias_map_picker(
    cfg: &clawcrew_config::schema::Config,
    section: &str,
) -> Vec<PickerItem> {
    let prefix_with_dot = format!("{section}.");
    let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for field in cfg.prop_fields() {
        let Some(suffix) = field.name.strip_prefix(&prefix_with_dot) else {
            continue;
        };
        let head = suffix.split_once('.').map_or(suffix, |(h, _)| h);
        if head.is_empty() {
            continue;
        }
        keys.insert(head.to_string());
    }
    keys.into_iter()
        .map(|key| PickerItem {
            key: key.clone(),
            label: key,
            description: None,
            badge: Some("configured".to_string()),
        })
        .collect()
}

/// Agents picker: walks `cfg.agents` and returns each alias with an activity badge.
/// `active` = agent exists and `enabled = true`; `configured` = exists but disabled.
pub(crate) fn agents_picker(cfg: &clawcrew_config::schema::Config) -> Vec<PickerItem> {
    let mut items: Vec<PickerItem> = cfg
        .agents
        .iter()
        .map(|(alias, agent)| PickerItem {
            key: alias.clone(),
            label: alias.clone(),
            description: None,
            badge: if agent.enabled {
                Some("active".to_string())
            } else {
                Some("configured".to_string())
            },
        })
        .collect();
    items.sort_by(|a, b| a.key.cmp(&b.key));
    items
}

pub(crate) fn apply_first_run_agent_defaults(
    cfg: &mut clawcrew_config::schema::Config,
    alias: &str,
) {
    let model_provider = cfg
        .providers
        .models
        .iter_entries()
        .next()
        .map(|(ty, alias, _)| format!("{ty}.{alias}"));
    let risk_profile = first_alias(cfg.risk_profiles.keys());
    let runtime_profile = first_alias(cfg.runtime_profiles.keys());

    let Some(agent) = cfg.agents.get_mut(alias) else {
        return;
    };
    if agent.model_provider.trim().is_empty()
        && let Some(model_provider) = model_provider
    {
        agent.model_provider = model_provider.into();
    }
    if agent.risk_profile.trim().is_empty()
        && let Some(risk_profile) = risk_profile
    {
        agent.risk_profile = risk_profile.into();
    }
    if agent.runtime_profile.trim().is_empty()
        && let Some(runtime_profile) = runtime_profile
    {
        agent.runtime_profile = runtime_profile.into();
    }
}

pub(crate) fn mark_section_completed(
    cfg: &mut clawcrew_config::schema::Config,
    section: &str,
) -> bool {
    if cfg
        .onboard_state
        .completed_sections
        .iter()
        .any(|completed| completed == section)
    {
        return false;
    }
    cfg.onboard_state
        .completed_sections
        .push(section.to_string());
    cfg.mark_dirty("onboard_state.completed_sections");
    true
}

pub(crate) fn first_alias<'a>(aliases: impl Iterator<Item = &'a String>) -> Option<String> {
    let mut aliases: Vec<&String> = aliases.collect();
    aliases.sort();
    aliases.first().map(|alias| (*alias).clone())
}

pub(crate) fn tunnel_provider_picker(cfg: &clawcrew_config::schema::Config) -> Vec<PickerItem> {
    // The canonical prop name uses an underscore (`tunnel_provider`); the
    // hyphenated form is unknown to get_prop and silently yields "" (no active
    // provider ever badged).
    let active = cfg.get_prop("tunnel.tunnel_provider").unwrap_or_default();
    let mut items = vec![PickerItem {
        key: "none".to_string(),
        label: "none".to_string(),
        description: Some("Localhost only — no public tunnel.".to_string()),
        badge: if active == "none" || active.is_empty() {
            Some("active".to_string())
        } else {
            None
        },
    }];
    for entry in cfg.tunnel.nested_option_entries() {
        let badge = if entry.field == active {
            Some("active".to_string())
        } else if entry.present {
            Some("configured".to_string())
        } else {
            None
        };
        items.push(PickerItem {
            key: entry.field.to_string(),
            label: entry.display_name.to_string(),
            description: if entry.description.is_empty() {
                None
            } else {
                Some(entry.description.to_string())
            },
            badge,
        });
    }
    items
}

pub async fn handle_section_select(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(SectionItemPath { section, key }): axum::extract::Path<SectionItemPath>,
    body: Option<axum::extract::Json<SectionSelectBody>>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let alias = body
        .and_then(|b| b.0.alias)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_string());

    // Held through the swap at the end of this handler so a concurrent
    // config writer can't land between this read and the save below.
    let _cfg_guard = std::sync::Arc::clone(&state.config_write_lock)
        .lock_owned()
        .await;
    let mut working = state.config.read().clone();

    use clawcrew_config::sections::Section;
    let Some(section_enum) = Section::from_key(&section) else {
        return error_response(
            ConfigApiError::new(
                ConfigApiCode::PathNotFound,
                format!("no picker semantics defined for section `{section}`"),
            )
            .with_path(section.as_str()),
        );
    };

    let (fields_prefix, created) = match section_enum {
        Section::ModelProviders | Section::TtsProviders | Section::TranscriptionProviders => {
            let family = section_enum.as_str();
            let created = working
                .create_map_key(&format!("{family}.{key}"), &alias)
                .map_err(|msg| {
                    error_response(
                        ConfigApiError::new(
                            ConfigApiCode::PathNotFound,
                            format!("could not select {family} `{key}` alias `{alias}`: {msg}"),
                        )
                        .with_path(format!("{family}.{key}")),
                    )
                });
            let created = match created {
                Ok(c) => c,
                Err(resp) => return resp,
            };
            // Per-family typed configs derive their own default endpoint
            // URI via family traits at runtime construction time.
            (format!("{family}.{key}.{alias}"), created)
        }
        Section::Channels => {
            let created = working
                .create_map_key(&format!("channels.{key}"), &alias)
                .map_err(|msg| {
                    error_response(
                        ConfigApiError::new(
                            ConfigApiCode::PathNotFound,
                            format!("could not select channel `{key}` alias `{alias}`: {msg}"),
                        )
                        .with_path(format!("channels.{key}")),
                    )
                });
            let created = match created {
                Ok(c) => c,
                Err(resp) => return resp,
            };
            if created {
                let enabled_path = format!("channels.{key}.{alias}.enabled");
                if let Err(e) = working.set_prop_persistent(&enabled_path, "true") {
                    ::clawcrew_log::record!(
                        WARN,
                        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                            .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                            .with_attrs(
                                ::serde_json::json!({"path": enabled_path, "error": format!("{}", e)})
                            ),
                        "failed to default-enable newly created channel; operator must toggle manually"
                    );
                }
            }
            (format!("channels.{key}.{alias}"), created)
        }
        Section::Agents
        | Section::PeerGroups
        | Section::Cron
        | Section::McpServers
        | Section::McpBundles
        | Section::KnowledgeBundles
        | Section::SkillBundles
        | Section::RiskProfiles
        | Section::RuntimeProfiles
        | Section::ModelRoutes
        | Section::EmbeddingRoutes => {
            let section_key = section_enum.as_str();
            let created = match clawcrew_config::alias_refs::create_map_key_checked(
                &mut working,
                section_key,
                &key,
            ) {
                Ok(c) => c,
                Err(clawcrew_config::alias_refs::CreateError::Reserved(a)) => {
                    return error_response(
                        ConfigApiError::new(
                            ConfigApiCode::ValidationFailed,
                            format!("alias `{a}` is reserved and cannot be created"),
                        )
                        .with_path(format!("{section_key}.{key}")),
                    );
                }
                Err(clawcrew_config::alias_refs::CreateError::Invalid(msg)) => {
                    return error_response(
                        ConfigApiError::new(
                            ConfigApiCode::PathNotFound,
                            format!("could not select {section_key} alias `{key}`: {msg}"),
                        )
                        .with_path(section_key),
                    );
                }
            };
            // Agents need a per-alias workspace dir on disk so the
            // PersonalityEditor and the runtime have somewhere to read
            // and write IDENTITY.md / SOUL.md / USER.md / etc.
            if created && matches!(section_enum, Section::Agents) {
                apply_first_run_agent_defaults(&mut working, &key);
                let workspace_dir = working.agent_workspace_dir(&key);
                if let Err(err) = tokio::fs::create_dir_all(&workspace_dir).await {
                    return error_response(
                        ConfigApiError::new(
                            ConfigApiCode::ValidationFailed,
                            format!(
                                "created agent `{key}` but failed to scaffold workspace at {}: {err}",
                                workspace_dir.display()
                            ),
                        )
                        .with_path(section_key),
                    );
                }
                if let Some(runner) = &state.personality_runner {
                    use clawcrew_config::multi_agent::MemoryBackendKind;
                    let include_memory = working
                        .agents
                        .get(&key)
                        .map(|agent| agent.memory.backend != MemoryBackendKind::None)
                        .unwrap_or(true);
                    if let Err(err) = runner
                        .seed_default_personality(&key, &workspace_dir, include_memory)
                        .await
                    {
                        ::clawcrew_log::record!(WARN, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_outcome(::clawcrew_log::EventOutcome::Unknown).with_attrs(::serde_json::json!({"agent": key, "workspace": workspace_dir.display().to_string(), "err": err.to_string()})), "agent workspace scaffolded but personality seed failed (continuing)");
                    }
                }
            }
            (format!("{section_key}.{key}"), created)
        }
        Section::Storage => {
            let created = working
                .create_map_key(&format!("storage.{key}"), &alias)
                .map_err(|msg| {
                    error_response(
                        ConfigApiError::new(
                            ConfigApiCode::PathNotFound,
                            format!("could not select storage `{key}` alias `{alias}`: {msg}"),
                        )
                        .with_path(format!("storage.{key}")),
                    )
                });
            let created = match created {
                Ok(c) => c,
                Err(resp) => return resp,
            };
            mark_section_completed(&mut working, "storage");
            (format!("storage.{key}.{alias}"), created)
        }
        Section::Memory => {
            // Set memory.backend to the picked key. Fields_prefix points at
            // `memory` so the form renders the whole memory section
            // (the active backend's specific fields show up there).
            let selection_changed = working.memory.backend != key;
            if selection_changed && let Err(e) = working.set_prop_persistent("memory.backend", &key)
            {
                return error_response(
                    ConfigApiError::new(
                        ConfigApiCode::ValidationFailed,
                        format!("could not set memory.backend = `{key}`: {e}"),
                    )
                    .with_path("memory.backend"),
                );
            }
            let completion_changed = mark_section_completed(&mut working, "memory");
            (
                "memory".to_string(),
                selection_changed || completion_changed,
            )
        }
        Section::Tunnel => {
            let selection_changed = working.tunnel.tunnel_provider != key;
            if selection_changed
                && let Err(e) = working.set_prop_persistent("tunnel.tunnel_provider", &key)
            {
                return error_response(
                    ConfigApiError::new(
                        ConfigApiCode::ValidationFailed,
                        format!("could not set tunnel.tunnel_provider = `{key}`: {e}"),
                    )
                    .with_path("tunnel.tunnel_provider"),
                );
            }
            let (prefix, defaults_changed) = if key == "none" {
                ("tunnel".to_string(), false)
            } else {
                let p = format!("tunnel.{key}");
                let initialized = working.init_defaults(Some(&p));
                (p, !initialized.is_empty())
            };
            (prefix, selection_changed || defaults_changed)
        }
        Section::Hardware | Section::Mcp | Section::Skills | Section::QuickstartState => {
            return error_response(
                ConfigApiError::new(
                    ConfigApiCode::PathNotFound,
                    format!(
                        "section `{}` is a direct-form section with no picker; \
                         render fields via GET /api/config/list?prefix={}",
                        section_enum, section_enum
                    ),
                )
                .with_path(section_enum.as_str()),
            );
        }
    };

    if created {
        working.mark_dirty(&fields_prefix);
    }

    if working.dirty_paths.is_empty() {
        let drifted = match section_enum {
            Section::Memory | Section::Tunnel => match try_compute_drift(&working).await {
                Ok(drifted) => drifted,
                Err(error) => return error_response(error),
            },
            _ => Vec::new(),
        };
        let tunnel_prefix =
            (section_enum == Section::Tunnel && key != "none").then(|| format!("tunnel.{key}."));
        let conflict_paths: Vec<String> = drifted
            .into_iter()
            .filter(|drift| match section_enum {
                Section::Memory => drift.path == "memory.backend",
                Section::Tunnel => {
                    drift.path == "tunnel.tunnel_provider"
                        || tunnel_prefix
                            .as_deref()
                            .is_some_and(|prefix| drift.path.starts_with(prefix))
                }
                _ => false,
            })
            .map(|drift| drift.path)
            .collect();
        if !conflict_paths.is_empty() {
            return error_response(ConfigApiError::new(
                ConfigApiCode::ConfigChangedExternally,
                format!(
                    "on-disk config has drifted from in-memory state on {} path(s) required by this selection: {}. Reload the config or GET /api/config/drift to inspect first.",
                    conflict_paths.len(),
                    conflict_paths.join(", "),
                ),
            ));
        }
        return axum::Json(SelectItemResponse {
            fields_prefix,
            created,
        })
        .into_response();
    }

    if let Err(e) = persist_and_swap(&state, working, &_cfg_guard).await {
        return error_response(e);
    }

    axum::Json(SelectItemResponse {
        fields_prefix,
        created,
    })
    .into_response()
}
