//! HTTP adapter over `galleon_api::skill::SkillsRunner`.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use galleon_api::skill::{
    AgentSkillEntry, DroppedSkill, DroppedSkillEntry, EffectiveSkill, RemoveMode, ScaffoldOptions,
    ServiceError, ShadowedSkillEntry, SkillDocument, SkillDropReason, SkillFrontmatter,
    SkillOrigin, SkillsBundlesResult, SkillsListResult, SlashOptionKindDescriptor,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use super::api::require_auth;

// ── HTTP-specific request shapes (not shared) ───────────────────────

/// Response for `GET /api/skills/slash-option-kinds`: the canonical registry,
/// built by walking `SlashOptionKind::ALL`.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct SlashOptionKindsResult {
    pub kinds: Vec<SlashOptionKindDescriptor>,
}

#[derive(Debug, Deserialize)]
pub struct SkillCreateBody {
    pub name: String,
    pub frontmatter: SkillFrontmatter,
    /// Initial markdown body. When empty, the service writes a default
    /// `# <Title>` heading derived from the skill name.
    #[serde(default)]
    pub body: String,
    /// Skip scaffolding the optional `scripts/`, `references/`, `assets/`
    /// subdirs. Defaults to `false` (create them).
    #[serde(default)]
    pub no_scaffold: bool,
}

#[derive(Debug, Deserialize)]
pub struct SkillWriteBody {
    pub frontmatter: SkillFrontmatter,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct DeleteQuery {
    /// When `true`, hard-delete the skill instead of archiving. Defaults to
    /// `false` — same as `RemoveMode::Archive`.
    #[serde(default)]
    pub purge: bool,
}

// ── Handlers ────────────────────────────────────────────────────────

/// `GET /api/skills/slash-option-kinds` — the canonical typed-slash-option kind
/// registry (kind list + per-kind constraint capabilities), built by walking
/// `SlashOptionKind::ALL`. Surfaces read this instead of restating the kind set.
pub async fn handle_slash_option_kinds(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    Json(SlashOptionKindsResult {
        kinds: galleon_api::skill::slash_option_kinds(),
    })
    .into_response()
}

/// `GET /api/skills/bundles`
pub async fn handle_list_bundles(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(service) = &state.skills_runner else {
        return (StatusCode::SERVICE_UNAVAILABLE, "skills service unavailable").into_response();
    };

    match service.list_bundles() {
        Ok(bundles) => Json(SkillsBundlesResult { bundles }).into_response(),
        Err(e) => service_error_response(e),
    }
}

/// `GET /api/skills/bundles/:alias/skills`
pub async fn handle_list_skills(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(alias): Path<String>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(service) = &state.skills_runner else {
        return (StatusCode::SERVICE_UNAVAILABLE, "skills service unavailable").into_response();
    };

    match service.list_skills(Some(&alias)) {
        Ok(skills) => Json(SkillsListResult { skills }).into_response(),
        Err(e) => service_error_response(e),
    }
}

pub async fn handle_agent_skills(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(alias): Path<String>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(service) = &state.skills_runner else {
        return (StatusCode::SERVICE_UNAVAILABLE, "skills service unavailable").into_response();
    };

    match service.resolve_effective_skills(&alias) {
        Ok(result) => Json(result).into_response(),
        Err(e) => service_error_response(e),
    }
}

/// Map an [`EffectiveSkill`] to its flat wire shape (`origin` string +
/// optional `plugin`/`bundle` detail). `editable`/`directory`/`shadowed` pass
/// through.
pub fn agent_skill_entry(s: EffectiveSkill) -> AgentSkillEntry {
    let (origin, plugin, bundle) = match s.origin {
        SkillOrigin::Workspace => ("workspace", None, None),
        SkillOrigin::OpenSkills => ("open-skills", None, None),
        SkillOrigin::Plugin(p) => ("plugin", Some(p), None),
        SkillOrigin::Bundle(a) => ("bundle", None, Some(a)),
    };
    AgentSkillEntry {
        name: s.name,
        description: s.description,
        origin: origin.to_string(),
        plugin,
        bundle,
        directory: s.directory.map(|d| d.display().to_string()),
        editable: s.editable,
        shadowed: s
            .shadowed
            .into_iter()
            .map(|sh| ShadowedSkillEntry {
                name: sh.name,
                origin: sh.origin_hint,
            })
            .collect(),
    }
}

/// Map a [`DroppedSkill`] to its flat wire shape, splitting the
/// [`SkillDropReason`] enum into a `(reason_kind, reason)` string pair the
/// dashboard can group on without knowing the Rust enum.
pub fn dropped_skill_entry(d: DroppedSkill) -> DroppedSkillEntry {
    let (reason_kind, reason, scripts_blocked) = match d.reason {
        SkillDropReason::AuditFindings {
            summary,
            scripts_blocked,
        } => ("audit_findings", summary, scripts_blocked),
        SkillDropReason::AuditError(s) => ("audit_error", s, false),
        SkillDropReason::ManifestParseError(s) => ("manifest_parse_error", s, false),
    };
    DroppedSkillEntry {
        name: d.name,
        origin: d.origin_hint,
        reason_kind: reason_kind.to_string(),
        reason,
        scripts_blocked,
        directory: d.location.map(|p| p.display().to_string()),
    }
}

/// `POST /api/skills/bundles/:alias/skills`
pub async fn handle_create_skill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(alias): Path<String>,
    Json(body): Json<SkillCreateBody>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(service) = &state.skills_runner else {
        return (StatusCode::SERVICE_UNAVAILABLE, "skills service unavailable").into_response();
    };

    let name = body.name.clone();
    match service.scaffold_skill(
        &alias,
        &body.name,
        body.frontmatter,
        ScaffoldOptions {
            create_optional_subdirs: !body.no_scaffold,
            body: body.body,
        },
    ) {
        Ok(path) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "bundle": alias,
                "name": name,
                "directory": path.display().to_string(),
            })),
        )
            .into_response(),
        Err(e) => service_error_response(e),
    }
}

/// `GET /api/skills/bundles/:alias/skills/:name`
pub async fn handle_read_skill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((alias, name)): Path<(String, String)>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(service) = &state.skills_runner else {
        return (StatusCode::SERVICE_UNAVAILABLE, "skills service unavailable").into_response();
    };

    match service.read_skill(&alias, &name) {
        Ok(doc) => Json(doc).into_response(),
        Err(e) => service_error_response(e),
    }
}

/// `PUT /api/skills/bundles/:alias/skills/:name`
pub async fn handle_write_skill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((alias, name)): Path<(String, String)>,
    Json(body): Json<SkillWriteBody>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(service) = &state.skills_runner else {
        return (StatusCode::SERVICE_UNAVAILABLE, "skills service unavailable").into_response();
    };

    let doc = SkillDocument {
        frontmatter: body.frontmatter,
        body: body.body,
    };
    match service.write_skill(&alias, &name, &doc) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => service_error_response(e),
    }
}

/// `DELETE /api/skills/bundles/:alias/skills/:name?purge=true`
pub async fn handle_delete_skill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((alias, name)): Path<(String, String)>,
    axum::extract::Query(q): axum::extract::Query<DeleteQuery>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }
    let Some(service) = &state.skills_runner else {
        return (StatusCode::SERVICE_UNAVAILABLE, "skills service unavailable").into_response();
    };

    let mode = if q.purge {
        RemoveMode::Purge
    } else {
        RemoveMode::Archive
    };
    match service.remove_skill(&alias, &name, mode) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => service_error_response(e),
    }
}

// ── Error mapping ───────────────────────────────────────────────────

fn service_error_response(err: ServiceError) -> Response {
    let status = match &err {
        ServiceError::Ref(_) => StatusCode::BAD_REQUEST,
        ServiceError::Bundle(_) => StatusCode::BAD_REQUEST,
        ServiceError::Scaffold(_) => StatusCode::BAD_REQUEST,
        ServiceError::DocumentParse(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
        ServiceError::NotEditable { .. } => StatusCode::FORBIDDEN,
        ServiceError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(serde_json::json!({
            "error": format!("{}", err),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use galleon_api::skill::{ShadowedSkill, SkillOrigin};
    use std::path::PathBuf;

    // the write-guard error maps to 403, distinct from 404/400.
    #[test]
    fn not_editable_maps_to_forbidden() {
        let resp = service_error_response(ServiceError::NotEditable {
            name: "alpha/foo".into(),
            origin: "non-bundle".into(),
        });
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // shadowed records ride through to the wire entry.
    #[test]
    fn agent_skill_entry_maps_shadowed() {
        let s = EffectiveSkill {
            name: "foo".into(),
            description: "d".into(),
            origin: SkillOrigin::Workspace,
            directory: None,
            editable: false,
            bundle: None,
            shadowed: vec![ShadowedSkill {
                name: "foo".into(),
                origin_hint: "bundle".into(),
            }],
        };
        let entry = agent_skill_entry(s);
        assert_eq!(entry.origin, "workspace");
        assert_eq!(entry.shadowed.len(), 1);
        assert_eq!(entry.shadowed[0].name, "foo");
        assert_eq!(entry.shadowed[0].origin, "bundle");
    }

    // each SkillDropReason arm maps to the right reason_kind tag.
    #[test]
    fn dropped_skill_entry_maps_each_reason_kind() {
        let mk = |reason| DroppedSkill {
            name: "n".into(),
            origin_hint: "workspace".into(),
            reason,
            location: Some(PathBuf::from("/x/n")),
        };
        assert_eq!(
            dropped_skill_entry(mk(SkillDropReason::AuditFindings {
                summary: "a".into(),
                scripts_blocked: true,
            }))
            .reason_kind,
            "audit_findings"
        );
        assert!(
            dropped_skill_entry(mk(SkillDropReason::AuditFindings {
                summary: "a".into(),
                scripts_blocked: true,
            }))
            .scripts_blocked,
            "scripts_blocked flag must pass through to the wire entry"
        );
        assert_eq!(
            dropped_skill_entry(mk(SkillDropReason::AuditError("b".into()))).reason_kind,
            "audit_error"
        );
        let mpe = dropped_skill_entry(mk(SkillDropReason::ManifestParseError("c".into())));
        assert_eq!(mpe.reason_kind, "manifest_parse_error");
        assert_eq!(mpe.reason, "c");
        assert_eq!(mpe.directory.as_deref(), Some("/x/n"));
    }
}
