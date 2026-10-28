//! Implementation of `galleon_api::skill::SkillsRunner` for the gateway decoupling.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;

use galleon_api::skill::{
    AgentSkillEntry, AgentSkillsResult, DroppedSkillEntry, RemoveMode, ScaffoldOptions,
    ServiceError, ShadowedSkillEntry, SkillBundleEntry, SkillDocument, SkillFrontmatter,
    SkillListEntry, SkillsReadResult, SkillsRunner,
};
use galleon_config::schema::Config;

use super::service::{EffectiveSkill, SkillsService, ServiceError as RuntimeServiceError};
use super::{DroppedSkill, SkillDropReason, SkillOrigin};

pub struct RuntimeSkillsRunner {
    config: Arc<RwLock<Config>>,
    install_root: PathBuf,
}

impl RuntimeSkillsRunner {
    pub fn new(config: Arc<RwLock<Config>>, install_root: impl Into<PathBuf>) -> Self {
        Self {
            config,
            install_root: install_root.into(),
        }
    }
}

fn map_agent_skill_entry(s: EffectiveSkill) -> AgentSkillEntry {
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

fn map_dropped_skill_entry(d: DroppedSkill) -> DroppedSkillEntry {
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

fn map_service_error(err: RuntimeServiceError) -> ServiceError {
    match err {
        RuntimeServiceError::Ref(e) => ServiceError::Ref(e.to_string()),
        RuntimeServiceError::Bundle(e) => ServiceError::Bundle(e.to_string()),
        RuntimeServiceError::Scaffold(e) => ServiceError::Scaffold(e.to_string()),
        RuntimeServiceError::DocumentParse(e) => ServiceError::DocumentParse(e.to_string()),
        RuntimeServiceError::NotFound(s) => ServiceError::NotFound(s),
        RuntimeServiceError::NotEditable { name, origin } => ServiceError::NotEditable { name, origin },
        RuntimeServiceError::Io(e) => ServiceError::Io(e.to_string()),
    }
}

impl SkillsRunner for RuntimeSkillsRunner {
    fn list_bundles(&self) -> Result<Vec<SkillBundleEntry>, ServiceError> {
        let config = self.config.read().clone();
        let service = SkillsService::new(&config, &self.install_root);
        service
            .list_bundles()
            .map(|bundles| {
                bundles
                    .into_iter()
                    .map(|b| SkillBundleEntry {
                        alias: b.alias,
                        directory: b.directory.display().to_string(),
                        include: b.include,
                        exclude: b.exclude,
                    })
                    .collect()
            })
            .map_err(map_service_error)
    }

    fn list_skills(&self, bundle: Option<&str>) -> Result<Vec<SkillListEntry>, ServiceError> {
        let config = self.config.read().clone();
        let service = SkillsService::new(&config, &self.install_root);
        service
            .list_skills(bundle)
            .map(|skills| {
                skills
                    .into_iter()
                    .map(|s| SkillListEntry {
                        bundle: s.r#ref.bundle().to_string(),
                        name: s.r#ref.name().to_string(),
                        directory: s.directory.display().to_string(),
                        frontmatter: s.frontmatter,
                    })
                    .collect()
            })
            .map_err(map_service_error)
    }

    fn resolve_effective_skills(&self, agent_alias: &str) -> Result<AgentSkillsResult, ServiceError> {
        let config = self.config.read().clone();
        let service = SkillsService::new(&config, &self.install_root);
        service
            .resolve_effective_skills(agent_alias)
            .map(|set| AgentSkillsResult {
                agent: agent_alias.to_string(),
                skills: set.skills.into_iter().map(map_agent_skill_entry).collect(),
                dropped: set.dropped.into_iter().map(map_dropped_skill_entry).collect(),
            })
            .map_err(map_service_error)
    }

    fn scaffold_skill(
        &self,
        bundle: &str,
        name: &str,
        frontmatter: SkillFrontmatter,
        options: ScaffoldOptions,
    ) -> Result<PathBuf, ServiceError> {
        let config = self.config.read().clone();
        let service = SkillsService::new(&config, &self.install_root);
        let target = service
            .resolve_ref(name, Some(bundle))
            .map_err(map_service_error)?;
        service
            .scaffold_skill(
                &target,
                frontmatter,
                super::scaffold::ScaffoldOptions {
                    create_optional_subdirs: options.create_optional_subdirs,
                    body: options.body,
                },
            )
            .map_err(map_service_error)
    }

    fn read_skill(&self, bundle: &str, name: &str) -> Result<SkillsReadResult, ServiceError> {
        let config = self.config.read().clone();
        let service = SkillsService::new(&config, &self.install_root);
        let target = service
            .resolve_ref(name, Some(bundle))
            .map_err(map_service_error)?;
        let doc = service.read_skill(&target).map_err(map_service_error)?;
        Ok(SkillsReadResult {
            bundle: target.bundle().to_string(),
            name: target.name().to_string(),
            frontmatter: doc.frontmatter,
            body: doc.body,
        })
    }

    fn write_skill(&self, bundle: &str, name: &str, doc: &SkillDocument) -> Result<(), ServiceError> {
        let config = self.config.read().clone();
        let service = SkillsService::new(&config, &self.install_root);
        let target = service
            .resolve_ref(name, Some(bundle))
            .map_err(map_service_error)?;
        let runtime_doc = super::document::SkillDocument {
            frontmatter: doc.frontmatter.clone(),
            body: doc.body.clone(),
        };
        service
            .write_skill(&target, &runtime_doc)
            .map_err(map_service_error)
    }

    fn remove_skill(&self, bundle: &str, name: &str, mode: RemoveMode) -> Result<(), ServiceError> {
        let config = self.config.read().clone();
        let service = SkillsService::new(&config, &self.install_root);
        let target = service
            .resolve_ref(name, Some(bundle))
            .map_err(map_service_error)?;
        let runtime_mode = match mode {
            RemoveMode::Purge => super::service::RemoveMode::Purge,
            RemoveMode::Archive => super::service::RemoveMode::Archive,
        };
        service
            .remove_skill(&target, runtime_mode)
            .map_err(map_service_error)
    }
}
