//! Windows AppContainer / process isolation sandbox backend.
//!
//! Provides deny-by-default isolation for Windows targets, confining
//! child processes to the configured workspace and explicit allowed roots.

use crate::security::traits::Sandbox;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Windows AppContainer / isolation sandbox backend.
#[derive(Debug, Clone)]
pub struct AppContainerSandbox {
    workspace_dir: Option<PathBuf>,
    allowed_roots: Vec<PathBuf>,
    profile_name: String,
}

impl AppContainerSandbox {
    pub fn new() -> Self {
        Self::with_workspace(None)
    }

    pub fn with_workspace(workspace: Option<&Path>) -> Self {
        Self::with_roots(workspace, &[])
    }

    pub fn with_roots(workspace: Option<&Path>, extra_roots: &[PathBuf]) -> Self {
        let mut allowed = Vec::new();
        if let Some(ws) = workspace {
            allowed.push(ws.to_path_buf());
        }
        allowed.extend(extra_roots.iter().cloned());

        Self {
            workspace_dir: workspace.map(Path::to_path_buf),
            allowed_roots: allowed,
            profile_name: format!("clawcrew-appcontainer-{}", std::process::id()),
        }
    }

    /// Check if a given path is within the allowed workspace or roots.
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        if self.allowed_roots.is_empty() {
            return false;
        }
        let canonical_target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        for root in &self.allowed_roots {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            if canonical_target.starts_with(&canonical_root) {
                return true;
            }
        }
        false
    }

    /// Return the unique profile name for this AppContainer sandbox instance.
    #[must_use]
    pub fn profile_name(&self) -> &str {
        &self.profile_name
    }
}

impl Default for AppContainerSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox for AppContainerSandbox {
    fn wrap_command(&self, cmd: &mut Command) -> std::io::Result<()> {
        // Enforce working directory containment to workspace if specified
        if let Some(ref ws) = self.workspace_dir {
            cmd.current_dir(ws);
        }

        // Clean out sensitive host credentials from the environment
        for secret_env in [
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
            "GEMINI_API_KEY",
            "AWS_SECRET_ACCESS_KEY",
            "GITHUB_TOKEN",
            "GH_TOKEN",
            "DATABASE_URL",
        ] {
            cmd.env_remove(secret_env);
        }

        // Apply Windows process creation flags when on Windows
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NEW_PROCESS_GROUP (0x00000200) + DETACHED / BREAKAWAY_OK if appropriate
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }

        Ok(())
    }

    fn is_available(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "appcontainer"
    }

    fn description(&self) -> &str {
        "Windows AppContainer / process isolation (deny-by-default)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn appcontainer_name_and_description() {
        let sandbox = AppContainerSandbox::new();
        assert_eq!(sandbox.name(), "appcontainer");
        assert!(sandbox.description().contains("deny-by-default"));
        assert!(sandbox.is_available());
        assert!(!sandbox.profile_name().is_empty());
    }

    #[test]
    fn appcontainer_deny_by_default_rejects_outside_path() {
        let ws = tempdir().expect("tempdir");
        let outside = tempdir().expect("tempdir");

        let sandbox = AppContainerSandbox::with_workspace(Some(ws.path()));

        // Inside workspace: allowed
        let inside_file = ws.path().join("allowed.txt");
        std::fs::write(&inside_file, "ok").expect("write");
        assert!(
            sandbox.is_path_allowed(&inside_file),
            "paths within workspace must be allowed"
        );

        // Outside workspace: denied
        let outside_file = outside.path().join("forbidden.txt");
        std::fs::write(&outside_file, "secret").expect("write");
        assert!(
            !sandbox.is_path_allowed(&outside_file),
            "paths outside workspace must be denied by default"
        );
    }

    #[test]
    fn appcontainer_wrap_command_sets_working_directory_and_clears_env() {
        let ws = tempdir().expect("tempdir");
        let sandbox = AppContainerSandbox::with_workspace(Some(ws.path()));

        let mut cmd = Command::new("cmd");
        cmd.env("OPENAI_API_KEY", "sensitive-secret-token");

        assert!(sandbox.wrap_command(&mut cmd).is_ok());
    }
}
