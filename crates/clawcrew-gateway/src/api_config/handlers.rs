//! Route handlers for `/api/config/*` endpoints.

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use clawcrew_config::api_error::{ConfigApiCode, ConfigApiError};
use clawcrew_config::field_visibility;
use clawcrew_config::sections::section_for_path;
use clawcrew_config::traits::MaskSecrets;
use std::sync::Arc;

use super::{
    AliasSourceQuery, AppState, ChannelBindBody, ConfigWriteGuard, DeletePlanResponse,
    DriftResponse, InitQuery, InitResponse, ListEntry, ListQuery, ListResponse, MapKeyQuery,
    MapKeyResponse, MapPathQuery, MigrateResponse, PropPutBody, PropQuery, PropResponse,
    RefSiteDto, ReloadStatusResponse, RenameMapKeyBody, RenameMapKeyResponse, SecretResponse,
    TemplateEntry, TemplatesResponse, compute_drift, persist_and_swap, reject_masked_secret_value,
    require_auth,
};

/// GET /api/config — compatibility whole-config read for older bundled
/// dashboard pages. New clients should prefer the per-property API, but
/// returning a masked snapshot here avoids a hard 405 when an older page is
/// served by a newer gateway.
pub async fn handle_config_get(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let mut cfg = state.config.read().clone();
    cfg.mask_secrets();
    Json(cfg).into_response()
}

/// Stable wire-form name for a `PropKind` variant. Matches the lower-kebab
/// convention the rest of the API uses for stable string IDs.
fn prop_kind_wire(kind: clawcrew_config::traits::PropKind) -> &'static str {
    use clawcrew_config::traits::PropKind;
    match kind {
        PropKind::String => "string",
        PropKind::Bool => "bool",
        PropKind::Integer => "integer",
        PropKind::Float => "float",
        PropKind::Enum => "enum",
        PropKind::AliasRef => "alias-ref",
        PropKind::StringArray => "string-array",
        PropKind::ObjectArray => "object-array",
        PropKind::Object => "object",
    }
}

pub(crate) fn error_response(err: ConfigApiError) -> Response {
    let status =
        StatusCode::from_u16(err.code.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::Json(err)).into_response()
}

/// Wrap an `anyhow::Error` from `Config::set_prop` / `get_prop` into a
/// `ConfigApiError`. Path-not-found errors get the specific code; everything
/// else falls through to ValidationFailed.
pub(crate) fn map_prop_error(err: anyhow::Error, path: &str) -> ConfigApiError {
    let msg = err.to_string();
    if msg.starts_with("Unknown property") {
        ConfigApiError::path_not_found(path)
    } else {
        ConfigApiError::from_validation(err).with_path(path)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

// Typed-value coercion lives in `clawcrew_config::typed_value` — both the
// gateway PATCH/PUT handlers and the CLI `config patch` flow consume it.
// Single source of truth for the "JSON in, set_prop string out, validated
// against the declared PropKind" contract.
use clawcrew_config::typed_value::coerce_for_set_prop as json_to_setprop_string;

/// Look up the prop_field metadata for a path. Used by the per-prop GET / PUT
/// handlers to decide whether the field is a secret.
pub(crate) fn lookup_prop_field(
    config: &clawcrew_config::schema::Config,
    path: &str,
) -> Option<clawcrew_config::traits::PropFieldInfo> {
    config
        .prop_fields()
        .into_iter()
        .find(|info| info.name == path)
        .or_else(|| {
            clawcrew_config::schema::Config::prop_is_secret(path).then(|| {
                clawcrew_config::traits::PropFieldInfo {
                    name: path.to_string(),
                    category: "Secrets",
                    display_value: clawcrew_config::traits::UNSET_DISPLAY.to_string(),
                    type_hint: "String",
                    kind: clawcrew_config::traits::PropKind::String,
                    is_secret: true,
                    enum_variants: None,
                    description: "",
                    derived_from_secret: false,
                    credential_class: Some(
                        clawcrew_config::traits::CredentialSurfaceClass::EncryptedSecret,
                    ),
                    tab: clawcrew_config::traits::ConfigTab::None,
                    alias_source: None,
                    multiline: false,
                }
            })
        })
}

pub(crate) fn scoped_validate(
    working: &clawcrew_config::schema::Config,
) -> Result<Vec<clawcrew_config::validation_warnings::ValidationWarning>, ConfigApiError> {
    if let Err(e) = working.validate() {
        let api_err = ConfigApiError::from_validation(e);
        let err_path = api_err.path.as_deref().unwrap_or("");
        let touches_dirty = !err_path.is_empty()
            && working.dirty_paths.iter().any(|d| {
                err_path == d.as_str()
                    || err_path.starts_with(&format!("{d}."))
                    || d.starts_with(&format!("{err_path}."))
            });
        if touches_dirty || err_path.is_empty() {
            return Err(api_err);
        }
        ::clawcrew_log::record!(
            WARN,
            ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                .with_attrs(::serde_json::json!({"path": err_path})),
            &format!(
                "validate() failed on a path outside this PATCH's dirty set; saving anyway and \
             surfacing as a warning: {}",
                api_err.message
            )
        );
        return Ok(vec![
            clawcrew_config::validation_warnings::ValidationWarning::new(
                "pre_existing_validation_error",
                api_err.message,
                err_path.to_string(),
            ),
        ]);
    }
    Ok(Vec::new())
}

/// Save `new_config` to disk, then install it as the live config.
///

/// bearer auth as every other config write. Because the gateway and the
/// running channels share one `Arc<RwLock<Config>>`, the swap makes the new
/// peer live immediately — no daemon restart, and no `/bind` message.
pub async fn handle_api_channel_bind(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChannelBindBody>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // Serialize the whole read-mutate-swap section: acquired before the
    // read-for-modify below and held through the swap at the end of this
    // handler, so a concurrent config writer can't land between this
    // handler's read and its `save()`/swap.
    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let channel_type = body.channel_type.trim();
    let alias = body.alias.trim();

    // Closed-set gate: only telegram/wechat/line have an operator-bind surface.
    if clawcrew_channels::orchestrator::channel_identity_normalizer(channel_type).is_none() {
        return error_response(ConfigApiError::new(
            ConfigApiCode::ValidationFailed,
            format!(
                "channel type `{channel_type}` does not support identity binding \
                 (supported: telegram, wechat, line)"
            ),
        ));
    }

    let mut working = state.config.read().clone();

    // The daemon gives the gateway and the channels separate `Config` copies
    // of the same file, so `_cfg_guard` alone is not enough: this handle's
    // `peer_groups` can be older than what another writer has already saved.
    // Without the refresh, an `ignore` persisted concurrently is both invisible
    // to the bind check below and overwritten by the save.
    match clawcrew_config::schema::persisted_peer_groups(&working.config_path).await {
        Ok(Some(persisted)) => working.peer_groups = persisted,
        Ok(None) => {}
        Err(e) => {
            return error_response(ConfigApiError::new(
                ConfigApiCode::ReloadFailed,
                format!("could not read the persisted peer policy: {e}"),
            ));
        }
    }

    // Reject a phantom alias loudly (404) rather than minting a peer group the
    // runtime never reads.
    if !clawcrew_channels::orchestrator::channel_alias_configured(&working, channel_type, alias) {
        return error_response(ConfigApiError::new(
            ConfigApiCode::PathNotFound,
            format!("channel `{channel_type}.{alias}` is not configured"),
        ));
    }

    let target = match clawcrew_channels::orchestrator::bind_channel_identity_into(
        &mut working,
        channel_type,
        alias,
        &body.identity,
    ) {
        Ok(target) => target,
        Err(e) => {
            return error_response(ConfigApiError::new(
                ConfigApiCode::ValidationFailed,
                e.to_string(),
            ));
        }
    };

    let channel = format!("{channel_type}.{alias}");

    // The writer picks its target by the group's `channel` field, so the
    // destination may be any key, not the conventional `<type>_<alias>`.
    // Reporting the conventional name would name a group that need not exist.
    let Some(group) = target else {
        // Name the group that actually carries the grant, including a bare
        // type-wide one. Falling back to the conventional `<type>_<alias>` key
        // named a block that need not exist and sent operators to edit it.
        let source = clawcrew_channels::orchestrator::channel_authorizing_group_key(
            &working,
            channel_type,
            alias,
            &body.identity,
        )
        .or_else(|| {
            clawcrew_channels::orchestrator::channel_peer_group_key(&working, channel_type, alias)
        });
        return Json(serde_json::json!({
            "saved": false,
            "already_bound": true,
            "group": source,
            "channel": channel,
        }))
        .into_response();
    };

    // Incremental: only `peer_groups` is applied onto the current on-disk
    // document, so the rest of this snapshot, which is still whatever this
    // handle last saw, cannot drop another writer's keys. A full `save` here
    // wrote the whole stale snapshot back. A direct peer-group mutation is not
    // dirty-tracked, so the explicit `mark_dirty` is what makes `save_dirty`
    // write it at all, and it materializes a brand-new table the same way.
    // Then swap the shared in-memory config so the channel authorizes live.
    working.mark_dirty("peer_groups");
    if let Err(e) = working.save_dirty().await {
        return error_response(ConfigApiError::new(
            ConfigApiCode::ReloadFailed,
            format!("save failed: {e}"),
        ));
    }
    *state.config.write() = working;
    state
        .pending_reload
        .store(true, std::sync::atomic::Ordering::Relaxed);

    Json(serde_json::json!({
        "saved": true,
        "already_bound": false,
        "group": group,
        "channel": channel,
    }))
    .into_response()
}

pub async fn handle_prop_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PropQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let info = match lookup_prop_field(&config, &q.path) {
        Some(info) => info,
        None => return error_response(ConfigApiError::path_not_found(&q.path)),
    };

    if info.is_secret || info.derived_from_secret {
        let populated = info.display_value != clawcrew_config::traits::UNSET_DISPLAY;
        return axum::Json(SecretResponse {
            path: q.path,
            populated,
        })
        .into_response();
    }

    match config.get_prop(&q.path) {
        Ok(value_str) => {
            // get_prop returns the display string; surface it as JSON.
            // For typed-value fidelity, callers should hit OPTIONS to learn
            // the type and parse client-side. Future iterations can route
            // typed values through serde directly.
            let warnings = config.collect_warnings();
            axum::Json(PropResponse {
                path: q.path,
                value: serde_json::Value::String(value_str),
                warnings,
            })
            .into_response()
        }
        Err(e) => error_response(map_prop_error(e, &q.path)),
    }
}

pub async fn handle_prop_put(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<PropPutBody>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let mut new_config = state.config.read().clone();
    if new_config.ensure_map_key_for_path(&body.path) {
        // Refused to vivify the reserved `default` agent: surface the same
        // reserved error the explicit create surfaces do, not a generic 404.
        return error_response(
            ConfigApiError::new(
                ConfigApiCode::ValidationFailed,
                "alias `default` is reserved and cannot be created",
            )
            .with_path(&body.path),
        );
    }
    let info = match lookup_prop_field(&new_config, &body.path) {
        Some(info) => info,
        None => return error_response(ConfigApiError::path_not_found(&body.path)),
    };

    let value_str = match json_to_setprop_string(&body.value, Some(info.kind)) {
        Ok(s) => s,
        Err(e) => return error_response(e.with_path(&body.path)),
    };

    let is_sensitive = info.is_secret || info.derived_from_secret;
    if let Err(e) = reject_masked_secret_value(&body.path, is_sensitive, &value_str) {
        return error_response(e);
    }

    if let Err(e) = new_config.set_prop_persistent(&body.path, &value_str) {
        return error_response(map_prop_error(e, &body.path));
    }

    let scoped_validation_warnings = match scoped_validate(&new_config) {
        Ok(ws) => ws,
        Err(err) => return error_response(err),
    };

    let config_path = new_config.config_path.clone();
    let mut warnings = new_config.collect_warnings();
    warnings.extend(scoped_validation_warnings);
    if let Err(e) = persist_and_swap(&state, new_config, &_cfg_guard).await {
        return error_response(e);
    }
    if let Some(comment) = body.comment.as_ref() {
        let annotations = [(body.path.clone(), comment.clone())];
        if let Err(e) =
            clawcrew_config::comment_writer::apply_comments(&config_path, &annotations).await
        {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                "failed to apply PUT comment to config.toml"
            );
        }
    }

    if info.is_secret || info.derived_from_secret {
        axum::Json(SecretResponse {
            path: body.path,
            populated: !value_str.is_empty(),
        })
        .into_response()
    } else {
        axum::Json(PropResponse {
            path: body.path,
            value: serde_json::Value::String(value_str),
            warnings,
        })
        .into_response()
    }
}

pub async fn handle_prop_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PropQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let mut new_config = state.config.read().clone();
    let info = match lookup_prop_field(&new_config, &q.path) {
        Some(info) => info,
        None => return error_response(ConfigApiError::path_not_found(&q.path)),
    };

    if let Err(e) = new_config.set_prop_persistent(&q.path, "") {
        return error_response(map_prop_error(e, &q.path));
    }

    let scoped_validation_warnings = match scoped_validate(&new_config) {
        Ok(ws) => ws,
        Err(err) => return error_response(err),
    };

    let mut warnings = new_config.collect_warnings();
    warnings.extend(scoped_validation_warnings);
    if let Err(e) = persist_and_swap(&state, new_config, &_cfg_guard).await {
        return error_response(e);
    }

    if info.is_secret || info.derived_from_secret {
        axum::Json(SecretResponse {
            path: q.path,
            populated: false,
        })
        .into_response()
    } else {
        axum::Json(PropResponse {
            path: q.path,
            value: serde_json::Value::Null,
            warnings,
        })
        .into_response()
    }
}

pub async fn handle_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let config = state.config.read().clone();
    let prefix = q.prefix.as_deref();

    // Drop fields that don't apply to the current shape of the config —
    // azure_* on a non-azure model_provider, qdrant.* when memory.backend is
    // sqlite, etc. Keeps the form scoped to relevant inputs only.
    let excluded = field_visibility::excluded_paths(&config, prefix.unwrap_or(""));

    let entries: Vec<ListEntry> = config
        .prop_fields()
        .into_iter()
        .filter(|info| match prefix {
            Some(p) => field_visibility::path_matches_prefix(&info.name, p),
            None => true,
        })
        .filter(|info| !field_visibility::is_excluded(&info.name, &excluded))
        .map(|info| {
            let populated = info.display_value != clawcrew_config::traits::UNSET_DISPLAY;
            let is_sensitive = info.is_secret || info.derived_from_secret;
            let value = if is_sensitive {
                None
            } else {
                Some(serde_json::Value::String(info.display_value.clone()))
            };
            let section = section_for_path(&info.name).map(|s| s.as_str());
            let enum_variants = info.enum_variants.map(|f| f()).unwrap_or_default();
            let is_env_overridden = config.prop_is_env_overridden(&info.name);
            ListEntry {
                path: info.name,
                category: info.category.to_string(),
                kind: prop_kind_wire(info.kind),
                type_hint: info.type_hint,
                value,
                populated,
                is_secret: is_sensitive,
                is_env_overridden,
                enum_variants,
                section,
                tab: info.tab.label(),
                multiline: info.multiline,
            }
        })
        .collect();

    let drifted = compute_drift(&config).await;
    axum::Json(ListResponse { entries, drifted }).into_response()
}

/// `GET /api/config/drift` — explicit drift summary for clients that want just
/// the diff. Same `DriftEntry` shape used in `ListResponse.drifted`.
pub async fn handle_drift(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let config = state.config.read().clone();
    let drifted = compute_drift(&config).await;
    axum::Json(DriftResponse { drifted }).into_response()
}

/// `GET /api/config/reload-status` — pending-reload flag for the dashboard's
/// reload banner. Goes true on any config write, false on `/admin/reload`.
pub async fn handle_reload_status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let pending_reload = state
        .pending_reload
        .load(std::sync::atomic::Ordering::Relaxed);
    axum::Json(ReloadStatusResponse { pending_reload }).into_response()
}

pub async fn handle_templates(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let _ = state; // templates are static per build, but auth-gated for consistency

    let templates: Vec<TemplateEntry> = clawcrew_config::schema::Config::map_key_sections()
        .into_iter()
        .map(|s| TemplateEntry {
            path: s.path,
            kind: match s.kind {
                clawcrew_config::traits::MapKeyKind::Map => "map",
                clawcrew_config::traits::MapKeyKind::List => "list",
            },
            value_type: s.value_type,
            description: s.description,
        })
        .collect();

    axum::Json(TemplatesResponse { templates }).into_response()
}

/// `GET /api/config/resolve-alias-source?source=<source>` — list the configured
/// alias values valid for an alias-reference field, resolved from the live
/// config via the shared `Config::resolve_alias_source`.
pub async fn handle_resolve_alias_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AliasSourceQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let cfg = state.config.read().clone();
    let values = cfg.resolve_alias_source(q.source);
    axum::Json(serde_json::json!({ "source": q.source, "values": values })).into_response()
}

/// `GET /api/config/map-keys?path=<section>` — list the current alias keys at
/// a map-keyed section path, e.g. `channels.discord` → `["default","work"]`.
pub async fn handle_get_map_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<MapPathQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let cfg = state.config.read().clone();
    match cfg.get_map_keys(&q.path) {
        Some(keys) => {
            axum::Json(serde_json::json!({ "path": q.path, "keys": keys })).into_response()
        }
        None => error_response(
            ConfigApiError::new(
                ConfigApiCode::PathNotFound,
                format!("no map-keyed section at `{}`", q.path),
            )
            .with_path(&q.path),
        ),
    }
}

/// `DELETE /api/config/map-key?path=<section>&key=<alias>` — remove an alias
/// from a map-keyed section. Aliased config sections with executable delete
/// support route through the same cascade engine as the delete preview;
/// non-aliased sections keep the generic raw key removal. Persists on success.
pub async fn handle_delete_map_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<MapKeyQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    // Acquired before this read-for-modify, threaded into the cascade
    // helpers below, and held through whichever branch's swap runs.
    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let working = state.config.read().clone();
    match clawcrew_config::alias_refs::alias_kind_for_map_path(&q.path) {
        Some(clawcrew_config::alias_refs::AliasKind::Agent) => {
            // Agent deletion is special: it must scrub config references
            // (heartbeat, peer-groups, delegates, workspace.access, …) via
            // `delete_with_cascade` and cascade owned non-config state (memory /
            // cron / acp / session).
            return delete_agent_cascade(&state, working, &q.key, _cfg_guard).await;
        }
        Some(kind) => {
            return delete_config_cascade(&state, working, &kind, &q.path, &q.key, &_cfg_guard)
                .await;
        }
        None => {}
    }
    let mut working = working;
    let removed = match working.delete_map_key(&q.path, &q.key) {
        Ok(b) => b,
        Err(msg) => {
            return error_response(
                ConfigApiError::new(ConfigApiCode::PathNotFound, msg).with_path(&q.path),
            );
        }
    };
    if removed {
        working.mark_dirty(&format!("{}.{}", q.path, q.key));
        if let Err(e) = persist_and_swap(&state, working, &_cfg_guard).await {
            return error_response(e);
        }
    }
    axum::Json(MapKeyResponse {
        path: q.path,
        key: q.key,
        created: false,
        warnings: None,
    })
    .into_response()
}

/// Agent-deletion cascade: refuse on HARD references (enabled `heartbeat.agent`
/// or live ACP sessions), else scrub config refs + remove the entry via
/// `delete_with_cascade`, archive the workspace, run the owned-state cascade
/// (export-then-delete memory/cron/acp + clear session attribution), and persist.
pub(crate) async fn delete_agent_cascade(
    state: &AppState,
    mut working: clawcrew_config::schema::Config,
    alias: &str,
    guard: ConfigWriteGuard,
) -> Response {
    use clawcrew_config::alias_refs::{self, AliasKind, CascadePolicy};

    if !working.agents.contains_key(alias) {
        return error_response(
            ConfigApiError::new(
                ConfigApiCode::PathNotFound,
                format!("agents.{alias} is not configured"),
            )
            .with_path("agents"),
        );
    }

    // Refuse on HARD: config blockers (e.g. enabled heartbeat.agent) OR live ACP
    // sessions (the operator must end those first). The ACP gate FAILS CLOSED:
    // if the session store can't be read we refuse rather than risk orphaning
    // live sessions.
    let plan = alias_refs::plan_delete(&working, &AliasKind::Agent, alias);
    let live_acp = match crate::agent_owned_state::live_acp_session_count(&working, alias) {
        Ok(n) => n,
        Err(e) => {
            return error_response(
                ConfigApiError::new(
                    ConfigApiCode::ValidationFailed,
                    format!(
                        "cannot delete agent `{alias}`: could not verify live ACP sessions ({e}); refusing to avoid orphaning active sessions"
                    ),
                )
                .with_path(format!("agents.{alias}")),
            );
        }
    };
    if !plan.allowed || live_acp > 0 {
        let mut reasons: Vec<String> = plan
            .blockers
            .iter()
            .map(|b| format!("{} (hard config reference)", b.path))
            .collect();
        if live_acp > 0 {
            reasons.push(format!("{live_acp} live ACP session(s) — end them first"));
        }
        return error_response(
            ConfigApiError::new(
                ConfigApiCode::ValidationFailed,
                format!("cannot delete agent `{alias}`: {}", reasons.join("; ")),
            )
            .with_path(format!("agents.{alias}")),
        );
    }

    let workspace = working.agent_workspace_dir(alias);

    // Config cascade: scrub soft refs + remove the agents entry.
    let cascade = match alias_refs::delete_with_cascade(
        &mut working,
        &AliasKind::Agent,
        alias,
        CascadePolicy::RefuseOnHard,
    ) {
        Ok(report) => report,
        Err(e) => {
            return error_response(
                ConfigApiError::new(
                    ConfigApiCode::ValidationFailed,
                    format!("agent config cascade failed: {e}"),
                )
                .with_path(format!("agents.{alias}")),
            );
        }
    };

    for path in cascade.dirty_paths() {
        working.mark_dirty(&path);
    }
    if let Err(e) = persist_and_swap(state, working, &guard).await {
        return error_response(e);
    }
    // Config is committed (saved + swapped). Release before the post-commit
    // side effects below: workspace archive and the memory/cron/ACP/session
    // cascade can be slow or wedge, and holding the lock across them would
    // stall every other gateway config write process-wide.
    drop(guard);
    // Config is durably committed: the agent is GONE from the persisted config.
    // Read it back from the (now-swapped) AppState for the side-effects below.
    let committed = state.config.read().clone();

    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let archive_dir = committed
        .data_dir
        .join("agents")
        .join("_deleted")
        .join(format!("{alias}-{ts}"));
    let mut warnings: Vec<String> = Vec::new();
    if let Err(err) = tokio::fs::create_dir_all(&archive_dir).await {
        ::clawcrew_log::record!(WARN, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_outcome(::clawcrew_log::EventOutcome::Unknown).with_attrs(::serde_json::json!({"agent": alias, "archive": archive_dir.display().to_string(), "err": err.to_string()})), "agent delete: archive dir creation failed");
        warnings.push(format!(
            "archive dir creation failed ({}): {err}",
            archive_dir.display()
        ));
    }
    if workspace.exists() {
        let dest = archive_dir.join("workspace");
        if let Err(err) = tokio::fs::rename(&workspace, &dest).await {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({"agent": alias, "from": workspace.display().to_string(), "to": dest.display().to_string(), "err": err.to_string()})),
                "agent delete: workspace archive failed"
            );
            warnings.push(format!(
                "workspace archive failed ({} -> {}): {err}",
                workspace.display(),
                dest.display()
            ));
        }
    }

    // Owned-state cascade (export-then-delete memory/cron/acp + clear sessions).
    let owned = crate::agent_owned_state::cascade_owned_state(
        &committed,
        &state.mem,
        state.session_backend.as_ref(),
        alias,
        &archive_dir,
    )
    .await;
    // Combine per-side-effect failures (archive dir / workspace rename) with
    // the per-store failures surfaced by `cascade_owned_state`, so the operator
    // sees the FULL partial-failure picture in the response, not just the
    // server log.
    warnings.extend(owned.warnings.iter().cloned());
    ::clawcrew_log::record!(INFO, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"agent": alias, "memory": owned.memory_purged, "cron": owned.cron_removed, "acp": owned.acp_removed, "sessions_cleared": owned.sessions_cleared, "archive": archive_dir.display().to_string(), "warnings": warnings.len()})), "agent deleted with owned-state cascade");

    axum::Json(MapKeyResponse {
        path: "agents".to_string(),
        key: alias.to_string(),
        created: false,
        warnings: if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        },
    })
    .into_response()
}

/// Config-only delete cascade for providers/channels (no owned state): refuse
/// on hard refs, scrub soft refs, mark every touched path dirty, persist.
pub(crate) async fn delete_config_cascade(
    state: &AppState,
    mut working: clawcrew_config::schema::Config,
    kind: &clawcrew_config::alias_refs::AliasKind,
    path: &str,
    key: &str,
    guard: &ConfigWriteGuard,
) -> Response {
    let report = match clawcrew_config::alias_refs::delete_with_cascade(
        &mut working,
        kind,
        key,
        clawcrew_config::alias_refs::CascadePolicy::RefuseOnHard,
    ) {
        Ok(r) => r,
        Err(e) => return delete_error_response(path, key, e),
    };
    let dirty_paths = report.dirty_paths();
    for dirty_path in &dirty_paths {
        working.mark_dirty(dirty_path);
    }
    if let Err(e) = persist_and_swap(state, working, guard).await {
        return error_response(e);
    }
    ::clawcrew_log::record!(
        INFO,
        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(
            ::serde_json::json!({"path": path, "key": key, "dirty_paths": dirty_paths.len()})
        ),
        "alias deleted with config-ref cascade"
    );
    axum::Json(MapKeyResponse {
        path: path.to_string(),
        key: key.to_string(),
        created: false,
        warnings: None,
    })
    .into_response()
}

pub async fn handle_map_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<MapKeyQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let mut working = state.config.read().clone();
    let path = q.path.clone();
    let key = q.key.clone();

    // Create through the shared guarded boundary so the reserved-agent rule (the
    // `default` runtime fallback) is enforced once for every surface. Reserved ->
    // 400 (validation_failed), symmetric with the rename guard; an unknown
    // section or invalid key stays 404 (path_not_found) as before.
    let created =
        match clawcrew_config::alias_refs::create_map_key_checked(&mut working, &path, &key) {
            Ok(b) => b,
            Err(clawcrew_config::alias_refs::CreateError::Reserved(a)) => {
                return error_response(
                    ConfigApiError::new(
                        ConfigApiCode::ValidationFailed,
                        format!("alias `{a}` is reserved and cannot be created"),
                    )
                    .with_path(format!("{path}.{key}")),
                );
            }
            Err(clawcrew_config::alias_refs::CreateError::Invalid(msg)) => {
                return error_response(
                    ConfigApiError::new(ConfigApiCode::PathNotFound, msg).with_path(&path),
                );
            }
        };

    if created {
        // skill-bundles: materialize the bundle's resolved directory so
        // skills have a home immediately. Run before persist so a failed
        // mkdir surfaces in logs alongside the config write.
        if path == "skill_bundles" {
            let install_root = working.install_root_dir();
            if let Ok(dir) =
                clawcrew_config::skill_bundles::resolve_directory(&working, &install_root, &key)
                && let Err(e) = tokio::fs::create_dir_all(&dir).await
            {
                ::clawcrew_log::record!(
                    WARN,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                        .with_outcome(::clawcrew_log::EventOutcome::Unknown),
                    &format!(
                        "skill-bundle '{key}' directory creation failed at {}: {e}",
                        dir.display().to_string()
                    )
                );
            }
        }

        working.mark_dirty(&format!("{path}.{key}"));
        if let Err(e) = persist_and_swap(&state, working, &_cfg_guard).await {
            return error_response(e);
        }
    }

    axum::Json(MapKeyResponse {
        path,
        key,
        created,
        warnings: None,
    })
    .into_response()
}

/// `GET /api/config/delete-plan?path=<section>&key=<alias>` — dry-run the delete
/// cascade for an aliased entry. Read-only; never mutates.
pub async fn handle_delete_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<MapKeyQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let config = state.config.read().clone();
    let to_dto = |s: &clawcrew_config::alias_refs::RefSite| RefSiteDto {
        path: s.path.clone(),
        raw_value: s.raw_value.clone(),
    };
    let Some(kind) = clawcrew_config::alias_refs::alias_kind_for_map_path(&q.path) else {
        // Non-aliased section (e.g. `mcp.servers`): generic key removal with no
        // reference cascade — nothing to preview.
        return axum::Json(DeletePlanResponse {
            path: q.path,
            key: q.key,
            allowed: true,
            blockers: Vec::new(),
            scrubs: Vec::new(),
            live_acp_sessions: None,
            cascades_owned_state: false,
        })
        .into_response();
    };
    if let Some(message) = unsupported_delete_cascade_message(&kind) {
        return error_response(
            ConfigApiError::new(ConfigApiCode::OpNotSupported, message)
                .with_path(format!("{}.{}", q.path, q.key)),
        );
    }
    let plan = clawcrew_config::alias_refs::plan_delete(&config, &kind, &q.key);
    let is_agent = matches!(kind, clawcrew_config::alias_refs::AliasKind::Agent);
    // For agents the live-ACP gate also blocks; it fails closed (an error
    // counting sessions ⇒ "not allowed"), matching the real delete.
    let live_acp = if is_agent {
        crate::agent_owned_state::live_acp_session_count(&config, &q.key).ok()
    } else {
        None
    };
    let allowed = plan.allowed && (!is_agent || live_acp == Some(0));
    axum::Json(DeletePlanResponse {
        path: q.path,
        key: q.key,
        allowed,
        blockers: plan.blockers.iter().map(to_dto).collect(),
        scrubs: plan.scrubs.iter().map(to_dto).collect(),
        live_acp_sessions: live_acp,
        cascades_owned_state: is_agent,
    })
    .into_response()
}

fn unsupported_delete_cascade_message(
    kind: &clawcrew_config::alias_refs::AliasKind,
) -> Option<&'static str> {
    use clawcrew_config::alias_refs::{AliasKind, ProviderCategory};
    match kind {
        AliasKind::Provider {
            category: ProviderCategory::Tts | ProviderCategory::Transcription,
            ..
        } => Some("TTS/transcription provider delete-with-cascade is not yet implemented"),
        _ => None,
    }
}

/// Map a [`RenameError`](clawcrew_config::alias_refs::RenameError) to the HTTP
/// error response (NotFound→404, InvalidName/Reserved→400, PostCondition→500).
fn rename_error_response(
    path: &str,
    from: &str,
    err: clawcrew_config::alias_refs::RenameError,
) -> Response {
    use clawcrew_config::alias_refs::RenameError;
    let (code, msg) = match err {
        RenameError::NotFound(p) => (
            ConfigApiCode::PathNotFound,
            format!("{p} is not configured"),
        ),
        RenameError::InvalidName(m) => (ConfigApiCode::ValidationFailed, m),
        RenameError::Reserved(a) => (
            ConfigApiCode::ValidationFailed,
            format!("alias `{a}` is reserved and cannot be renamed"),
        ),
        RenameError::PostCondition(m) => (
            ConfigApiCode::InternalError,
            format!("rename cascade post-condition failed: {m}"),
        ),
    };
    error_response(ConfigApiError::new(code, msg).with_path(format!("{path}.{from}")))
}

/// Map a [`CascadeError`](clawcrew_config::alias_refs::CascadeError) to the
/// HTTP error response for config-only alias deletes. `Refused` and
/// `NotImplemented` are expected operator-facing outcomes, while
/// `PostCondition` is an internal guard failure and must not be persisted.
fn delete_error_response(
    path: &str,
    key: &str,
    err: clawcrew_config::alias_refs::CascadeError,
) -> Response {
    use clawcrew_config::alias_refs::CascadeError;
    let (code, msg) = match err {
        CascadeError::Refused(report) => {
            let blockers: Vec<_> = report.blockers.iter().map(|b| b.path.as_str()).collect();
            let detail = if blockers.is_empty() {
                "hard references remain".to_string()
            } else {
                format!("hard reference(s) remain: {}", blockers.join(", "))
            };
            (
                ConfigApiCode::ValidationFailed,
                format!("cannot delete alias `{key}`: {detail}"),
            )
        }
        CascadeError::NotFound(p) => (
            ConfigApiCode::PathNotFound,
            format!("{p} is not configured"),
        ),
        CascadeError::NotImplemented(m) => (ConfigApiCode::OpNotSupported, m),
        CascadeError::PostCondition(m) => (
            ConfigApiCode::InternalError,
            format!("delete cascade post-condition failed: {m}"),
        ),
    };
    error_response(ConfigApiError::new(code, msg).with_path(format!("{path}.{key}")))
}

pub async fn handle_rename_map_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<RenameMapKeyBody>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // Acquired before this read-for-modify, threaded into the cascade
    // helpers below, and held through whichever branch's swap runs.
    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let working = state.config.read().clone();

    match clawcrew_config::alias_refs::alias_kind_for_map_path(&body.path) {
        Some(clawcrew_config::alias_refs::AliasKind::Agent) => {
            rename_agent_cascade(&state, working, &body, _cfg_guard).await
        }
        Some(kind) => rename_config_cascade(&state, working, &kind, &body, &_cfg_guard).await,
        None => {
            // Non-aliased section: the generic key-swap rename (unchanged).
            let mut working = working;
            let renamed = match working.rename_map_key(&body.path, &body.from, &body.to) {
                Ok(b) => b,
                Err(msg) => {
                    return error_response(
                        ConfigApiError::new(ConfigApiCode::ValidationFailed, msg)
                            .with_path(&body.path),
                    );
                }
            };
            if renamed {
                working.mark_dirty(&format!("{}.{}", body.path, body.from));
                working.mark_dirty(&format!("{}.{}", body.path, body.to));
                if let Err(e) = persist_and_swap(&state, working, &_cfg_guard).await {
                    return error_response(e);
                }
            }
            axum::Json(RenameMapKeyResponse {
                path: body.path,
                from: body.from,
                to: body.to,
                renamed,
                warnings: Vec::new(),
            })
            .into_response()
        }
    }
}

/// Config-only rename cascade for providers/channels (no owned state): rewrite
/// references, mark every touched path dirty, persist.
pub(crate) async fn rename_config_cascade(
    state: &AppState,
    mut working: clawcrew_config::schema::Config,
    kind: &clawcrew_config::alias_refs::AliasKind,
    body: &RenameMapKeyBody,
    guard: &ConfigWriteGuard,
) -> Response {
    let report = match clawcrew_config::alias_refs::rename_with_cascade(
        &mut working,
        kind,
        &body.from,
        &body.to,
    ) {
        Ok(r) => r,
        Err(e) => return rename_error_response(&body.path, &body.from, e),
    };
    for path in &report.dirty_paths {
        working.mark_dirty(path);
    }
    if let Err(e) = persist_and_swap(state, working, guard).await {
        return error_response(e);
    }
    ::clawcrew_log::record!(INFO, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"path": body.path, "from": body.from, "to": body.to, "dirty_paths": report.dirty_paths.len()})), "alias renamed with config-ref cascade");
    axum::Json(RenameMapKeyResponse {
        path: body.path.clone(),
        from: body.from.clone(),
        to: body.to.clone(),
        renamed: true,
        warnings: Vec::new(),
    })
    .into_response()
}

pub(crate) async fn move_renamed_workspace(
    old_ws: &std::path::Path,
    new_ws: &std::path::Path,
) -> Option<String> {
    if old_ws == new_ws || !old_ws.exists() {
        return None;
    }
    if let Some(parent) = new_ws.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    match tokio::fs::rename(old_ws, new_ws).await {
        Ok(()) => None,
        Err(err) => {
            ::clawcrew_log::record!(
                WARN,
                ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note)
                    .with_outcome(::clawcrew_log::EventOutcome::Unknown)
                    .with_attrs(::serde_json::json!({
                        "old": old_ws.display().to_string(),
                        "new": new_ws.display().to_string(),
                        "err": err.to_string()
                    })),
                "agent rename: workspace move failed"
            );
            Some(format!(
                "workspace move {} -> {} failed: {err}",
                old_ws.display(),
                new_ws.display()
            ))
        }
    }
}

async fn rename_residue_exists(
    state: &AppState,
    working: &clawcrew_config::schema::Config,
    from: &str,
) -> bool {
    // Workspace: the default per-alias dir for `from`. A custom/alias-independent
    // path is not moved by the cascade, so it is not residue.
    if working.agent_workspace_dir(from).exists() {
        return true;
    }

    // Short-lived clone for the DB-backed stores - never hold the lock across an
    // `.await`.
    let cfg = state.config.read().clone();

    // Cron jobs still owned by `from`.
    if let Ok(store) = clawcrew_infra::cron_store::CronStore::new(&cfg.data_dir)
        && store
            .list_jobs_by_agent(from)
            .map(|jobs| !jobs.is_empty())
            .unwrap_or(false)
    {
        return true;
    }

    // ACP sessions (live OR killed) still owned by `from`.
    if let Ok(store) = clawcrew_infra::acp_session_store::AcpSessionStore::new(&cfg.data_dir)
        && store
            .list_sessions_by_agent(from)
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    {
        return true;
    }

    // Memory rows still attributed to `from`.
    if state.mem.count_agent(from).await.unwrap_or(0) > 0 {
        return true;
    }

    // Session-metadata attribution still pointing at `from`.
    if let Some(backend) = state.session_backend.as_ref()
        && backend.count_agent_attribution(from).unwrap_or(0) > 0
    {
        return true;
    }

    false
}

pub(crate) async fn rename_agent_cascade(
    state: &AppState,
    mut working: clawcrew_config::schema::Config,
    body: &RenameMapKeyBody,
    guard: ConfigWriteGuard,
) -> Response {
    use clawcrew_config::alias_refs::{self, AliasKind};
    let (from, to) = (&body.from, &body.to);

    // Capture the OLD workspace path while the entry still lives under `from`
    // (custom paths are read off the entry, which is about to move).
    let old_ws = working.agent_workspace_dir(from);

    let committed_to = working.agent(from).is_none() && working.agent(to).is_some();
    let dirty_count = if committed_to && rename_residue_exists(state, &working, from).await {
        0
    } else {
        match alias_refs::rename_with_cascade(&mut working, &AliasKind::Agent, from, to) {
            Ok(report) => {
                for path in &report.dirty_paths {
                    working.mark_dirty(path);
                }
                let dirty_count = report.dirty_paths.len();
                if let Err(e) = persist_and_swap(state, working, &guard).await {
                    return error_response(e);
                }
                dirty_count
            }
            Err(e) => return rename_error_response(&body.path, from, e),
        }
    };
    // Config is committed (saved + swapped, or already committed by a prior
    // crashed run). Release before the post-commit side effects below:
    // workspace move and the memory/cron/ACP/session-backend cascade can be
    // slow or wedge, and holding the lock across them would stall every
    // other gateway config write process-wide.
    drop(guard);

    let cfg = state.config.read().clone();
    // The NEW workspace path off the committed config (the rewritten `to`).
    let new_ws = cfg.agent_workspace_dir(to);

    // Move the workspace dir. For the default per-alias location this is
    // `<install>/agents/<from>/workspace` → `…/<to>/workspace`. A custom
    // workspace path is alias-independent, so `old_ws == new_ws` and we skip.
    let ws_existed = old_ws != new_ws && old_ws.exists();
    let move_warning = move_renamed_workspace(&old_ws, &new_ws).await;
    let workspace_moved = ws_existed && move_warning.is_none();
    let mut warnings: Vec<String> = Vec::new();
    warnings.extend(move_warning);

    // Re-point owned DB state (memory/cron/acp/session). Best-effort + reported.
    let owned = crate::agent_owned_state::cascade_rename_agent(
        &cfg,
        &state.mem,
        state.session_backend.as_ref(),
        from,
        to,
    )
    .await;
    // Combine the workspace-move warning (if any) with the owned-store warnings
    // so every partial failure reaches the caller, not just the server log.
    warnings.extend(owned.warnings);

    // The config rename committed. A non-empty `warnings` means a post-persist
    // side-effect did not follow (config is `to`, some follower lags at `from`,
    // re-runnable) - escalate to WARN so that degraded outcome is visible
    // operationally instead of buried at INFO.
    if warnings.is_empty() {
        ::clawcrew_log::record!(INFO, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"from": from, "to": to, "memory": owned.memory_rows, "cron": owned.cron_jobs, "acp": owned.acp_sessions, "sessions": owned.sessions_repointed, "workspace_moved": workspace_moved, "dirty_paths": dirty_count})), "agent renamed with owned-state cascade");
    } else {
        ::clawcrew_log::record!(WARN, ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Note).with_attrs(::serde_json::json!({"from": from, "to": to, "memory": owned.memory_rows, "cron": owned.cron_jobs, "acp": owned.acp_sessions, "sessions": owned.sessions_repointed, "workspace_moved": workspace_moved, "dirty_paths": dirty_count, "warnings": warnings})), "agent rename persisted but a post-persist side-effect did not follow; re-issue the rename to converge");
    }

    // Persisted rename. `warnings` carries any post-persist side-effect that did
    // not follow, so the split can be remediated rather than reported as a clean
    // success (207-style partial success).
    axum::Json(RenameMapKeyResponse {
        path: body.path.clone(),
        from: from.clone(),
        to: to.clone(),
        renamed: true,
        warnings,
    })
    .into_response()
}

pub async fn handle_refresh_context_window(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path((provider_type, alias)): axum::extract::Path<(String, String)>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let path = format!("providers.models.{provider_type}.{alias}");

    // Build the minimal provider config the fetch below needs from a brief,
    // un-witnessed read that clones what it needs out immediately (the
    // `parking_lot` guard never survives past this block, well before the
    // network `.await`). Deliberately NOT under `config_write_lock`: the
    // outbound provider fetch can be slow or hang, and holding the witness
    // across it would block every other gateway config write process-wide
    // for the duration -- a self-inflicted availability bottleneck.
    let provider_config = {
        let snapshot = state.config.read();
        if snapshot.get_prop(&format!("{path}.model")).is_err() {
            return error_response(
                ConfigApiError::new(
                    ConfigApiCode::PathNotFound,
                    format!("model provider '{provider_type}.{alias}' not found"),
                )
                .with_path(&path),
            );
        }
        let model = snapshot
            .get_prop(&format!("{path}.model"))
            .ok()
            .unwrap_or_default();
        let uri = snapshot.get_prop(&format!("{path}.uri")).ok();
        // Read api_key via JSON serialization to bypass #[secret] masking in get_prop.
        let api_key = serde_json::to_value(&snapshot.providers)
            .ok()
            .and_then(|v| {
                v.pointer(&format!("/models/{provider_type}/{alias}/api_key"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty() && *s != "<unset>")
                    .map(String::from)
            });
        clawcrew_config::schema::ModelProviderConfig {
            model: Some(model),
            uri,
            api_key,
            ..Default::default()
        }
    };

    // Fetch context window from provider. No lock -- neither `config` nor
    // `config_write_lock` -- is held across this await.
    let context_window = match galleon_providers::fetch_context_window(
        &provider_type,
        &provider_config,
    )
    .await
    {
        Some(ctx) => ctx,
        None => {
            return error_response(
                ConfigApiError::new(
                    ConfigApiCode::InvalidFormat,
                    format!("provider '{provider_type}' does not support context window auto-detection or fetch failed"),
                )
                .with_path(&path),
            );
        }
    };

    // The witness is acquired only now, spanning just the config mutation:
    // read-for-modify, apply the fetched value, save, swap.
    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let mut working = state.config.read().clone();

    // Re-verify: a concurrent writer could have removed this entry while
    // the un-witnessed fetch above was in flight.
    if working.get_prop(&format!("{path}.model")).is_err() {
        return error_response(
            ConfigApiError::new(
                ConfigApiCode::PathNotFound,
                format!("model provider '{provider_type}.{alias}' not found"),
            )
            .with_path(&path),
        );
    }

    if let Err(e) = working.set_prop_persistent(
        &format!("{path}.context_window"),
        &context_window.to_string(),
    ) {
        return error_response(
            ConfigApiError::new(
                ConfigApiCode::InternalError,
                format!("failed to persist context_window: {e}"),
            )
            .with_path(&path),
        );
    }

    working.mark_dirty(&format!("{path}.context_window"));
    if let Err(e) = persist_and_swap(&state, working, &_cfg_guard).await {
        return error_response(e);
    }

    axum::Json(serde_json::json!({
        "path": path,
        "context_window": context_window,
    }))
    .into_response()
}

/// through `POST /api/config/map-key`. When every requested section is already
/// configured, returns `{initialized: []}`.
pub async fn handle_init(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<InitQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let mut working = state.config.read().clone();
    let initialized: Vec<String> = working
        .init_defaults(q.section.as_deref())
        .into_iter()
        .map(str::to_string)
        .collect();

    if initialized.is_empty() {
        return axum::Json(InitResponse { initialized }).into_response();
    }

    for section in &initialized {
        working.mark_dirty(section);
    }

    if let Err(err) = scoped_validate(&working) {
        return error_response(err);
    }
    if let Err(e) = persist_and_swap(&state, working, &_cfg_guard).await {
        return error_response(e);
    }

    axum::Json(InitResponse { initialized }).into_response()
}

pub async fn handle_migrate(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // Held through the final swap below so two concurrent migrate calls
    // can't interleave their read-migrate-swap sections.
    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let (config_path, data_dir) = {
        let live = state.config.read();
        (live.config_path.clone(), live.data_dir.clone())
    };

    let raw = match tokio::fs::read_to_string(&config_path).await {
        Ok(s) => s,
        Err(e) => {
            return error_response(ConfigApiError::new(
                ConfigApiCode::InternalError,
                format!("failed to read config file: {e}"),
            ));
        }
    };

    let migrated = match clawcrew_config::migration::migrate_file(&raw) {
        Ok(out) => out,
        Err(e) => {
            return error_response(ConfigApiError::new(
                ConfigApiCode::ValidationFailed,
                format!("migration failed: {e}"),
            ));
        }
    };

    match migrated {
        Some(new_content) => {
            // Validate the migrated snapshot before touching the canonical
            // file. `config_path` and `data_dir` are runtime-selected and
            // skipped by serde, so restore them explicitly on the parsed live
            // snapshot.
            let mut new_cfg: clawcrew_config::schema::Config = match toml::from_str(&new_content) {
                Ok(c) => c,
                Err(e) => {
                    return error_response(ConfigApiError::new(
                        ConfigApiCode::ReloadFailed,
                        format!("re-parse after migration failed: {e}"),
                    ));
                }
            };
            new_cfg.config_path = config_path.clone();
            new_cfg.data_dir = data_dir;

            let backup_path = config_path.with_extension("toml.bak");
            let parent = match config_path.parent() {
                Some(p) => p.to_path_buf(),
                None => {
                    return error_response(ConfigApiError::new(
                        ConfigApiCode::InternalError,
                        format!(
                            "config path has no parent: {}",
                            config_path.display().to_string()
                        ),
                    ));
                }
            };
            let file_name = match config_path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => {
                    return error_response(ConfigApiError::new(
                        ConfigApiCode::InternalError,
                        format!(
                            "config path has no file name: {}",
                            config_path.display().to_string()
                        ),
                    ));
                }
            };
            let temp_path = parent.join(format!(".{file_name}.tmp-{}", uuid::Uuid::new_v4()));

            // 1. Write migrated content to temp + fsync.
            match tokio::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)
                .await
            {
                Ok(mut temp) => {
                    use tokio::io::AsyncWriteExt;
                    if let Err(e) = temp.write_all(new_content.as_bytes()).await {
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        return error_response(ConfigApiError::new(
                            ConfigApiCode::InternalError,
                            format!("failed to write migrated config to temp: {e}"),
                        ));
                    }
                    if let Err(e) = temp.sync_all().await {
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        return error_response(ConfigApiError::new(
                            ConfigApiCode::InternalError,
                            format!("failed to fsync migrated config temp: {e}"),
                        ));
                    }
                }
                Err(e) => {
                    return error_response(ConfigApiError::new(
                        ConfigApiCode::InternalError,
                        format!("failed to create temp config file: {e}"),
                    ));
                }
            }

            // 2. Backup BEFORE replacing the original.
            if let Err(e) = tokio::fs::copy(&config_path, &backup_path).await {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return error_response(ConfigApiError::new(
                    ConfigApiCode::InternalError,
                    format!("failed to write backup: {e}"),
                ));
            }

            // 3. Atomic rename. On failure, restore from backup.
            if let Err(e) = tokio::fs::rename(&temp_path, &config_path).await {
                let _ = tokio::fs::remove_file(&temp_path).await;
                if backup_path.exists() {
                    let _ = tokio::fs::copy(&backup_path, &config_path).await;
                }
                return error_response(ConfigApiError::new(
                    ConfigApiCode::InternalError,
                    format!("failed to atomically replace config: {e}"),
                ));
            }

            // 4. Fsync the parent directory so the rename is durable.
            #[cfg(unix)]
            if let Ok(dir) = tokio::fs::File::open(&parent).await {
                let _ = dir.sync_all().await;
            }

            *state.config.write() = new_cfg;
            state
                .pending_reload
                .store(true, std::sync::atomic::Ordering::Relaxed);

            axum::Json(MigrateResponse {
                migrated: true,
                backup_path: Some(backup_path.display().to_string()),
                schema_version: clawcrew_config::migration::CURRENT_SCHEMA_VERSION,
            })
            .into_response()
        }
        None => axum::Json(MigrateResponse {
            migrated: false,
            backup_path: None,
            schema_version: clawcrew_config::migration::CURRENT_SCHEMA_VERSION,
        })
        .into_response(),
    }
}

pub async fn handle_options_config(headers: HeaderMap) -> Response {
    // CORS preflight short-circuit
    if headers.contains_key("access-control-request-method") {
        let mut response = StatusCode::NO_CONTENT.into_response();
        let h = response.headers_mut();
        h.insert(
            "Access-Control-Allow-Methods",
            HeaderValue::from_static("GET, PUT, PATCH, OPTIONS"),
        );
        h.insert(
            "Access-Control-Allow-Headers",
            HeaderValue::from_static("Authorization, Content-Type, If-None-Match"),
        );
        return response;
    }

    schema_response("clawcrew_config_schema_full")
}

pub async fn handle_options_prop(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PropQuery>,
) -> Response {
    if headers.contains_key("access-control-request-method") {
        let mut response = StatusCode::NO_CONTENT.into_response();
        let h = response.headers_mut();
        h.insert(
            "Access-Control-Allow-Methods",
            HeaderValue::from_static("GET, PUT, DELETE, OPTIONS"),
        );
        h.insert(
            "Access-Control-Allow-Headers",
            HeaderValue::from_static("Authorization, Content-Type, If-None-Match"),
        );
        return response;
    }

    // Resolve the path against the in-memory config; 404 if it doesn't
    // exist. (No auth required for shape discovery — same as OPTIONS /api/config.)
    let config = state.config.read().clone();
    let info = match lookup_prop_field(&config, &q.path) {
        Some(info) => info,
        None => return error_response(ConfigApiError::path_not_found(&q.path)),
    };

    let (whole_body, etag) = cached_schema();
    let mut body = whole_body.clone();
    if let serde_json::Value::Object(ref mut map) = body {
        map.insert(
            "x-clawcrew-requested-path".into(),
            serde_json::Value::String(q.path.clone()),
        );
        map.insert(
            "x-clawcrew-prop".into(),
            serde_json::json!({
                "path": q.path,
                "kind": prop_kind_wire(info.kind),
                "type_hint": info.type_hint,
                "is_secret": info.is_secret || info.derived_from_secret,
                "enum_variants": info.enum_variants.map(|f| f()).unwrap_or_default(),
                "category": info.category,
            }),
        );
    }
    let mut response = (StatusCode::OK, axum::Json(body)).into_response();
    response.headers_mut().insert(
        header::ALLOW,
        HeaderValue::from_static("GET, PUT, DELETE, OPTIONS"),
    );
    insert_etag(&mut response, etag);
    response
}

fn schema_response(_label: &'static str) -> Response {
    let (body, etag) = cached_schema();
    let mut response = (StatusCode::OK, axum::Json(body.clone())).into_response();
    response.headers_mut().insert(
        header::ALLOW,
        HeaderValue::from_static("GET, PUT, PATCH, OPTIONS"),
    );
    insert_etag(&mut response, etag);
    response
}

pub(crate) fn insert_etag(response: &mut Response, etag: &str) {
    if let Ok(value) = HeaderValue::from_str(etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
}

pub(crate) fn cached_schema() -> (&'static serde_json::Value, &'static str) {
    use std::sync::OnceLock;
    static CACHE: OnceLock<(serde_json::Value, String)> = OnceLock::new();
    let entry = CACHE.get_or_init(|| {
        let body = schema_body_value();
        let etag = build_etag_for(&body);
        (body, etag)
    });
    (&entry.0, entry.1.as_str())
}

#[cfg(feature = "schema-export")]
pub(crate) fn schema_body_value() -> serde_json::Value {
    let schema = schemars::schema_for!(clawcrew_config::schema::Config);
    serde_json::to_value(schema).unwrap_or(serde_json::Value::Null)
}

#[cfg(not(feature = "schema-export"))]
pub(crate) fn schema_body_value() -> serde_json::Value {
    serde_json::json!({
        "error": "schema-export feature not enabled in this build",
    })
}

/// Stable ETag derived from the rendered schema bytes. Computed once via
/// `cached_schema()`; this helper is kept separate so tests can verify
/// determinism.
pub(crate) fn build_etag_for(body: &serde_json::Value) -> String {
    use std::hash::{Hash, Hasher};
    let bytes = body.to_string();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("\"{:016x}\"", hasher.finish())
}
