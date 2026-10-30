//! HTTP handlers for Galleon Fleet project endpoints (`/api/project/*`).
//!
//! Under SPEC-013 §Model Data (TASK-075):
//! - `GET /api/project/git-status?path=`:
//!   - Executes `git -C <path> status --porcelain=v1 -b` via `tokio::process::Command` without shell.
//!   - Target path is canonicalized and strictly required to reside within configured workspace roots.
//!   - Path outside workspace (including `..` traversal and symlink escape) is rejected with 400 Bad Request.
//!   - Command failure or non-git directory returns an honest 4xx/5xx error (fixing Go's fake success pattern).

use std::path::PathBuf;

use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use galleon_fleet_core::{parse_git_status, FLEET_MAX_BODY};
use serde::Deserialize;
use serde_json::json;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

/// Query parameters for `GET /api/project/git-status`.
#[derive(Debug, Deserialize, Default)]
pub struct GitStatusQuery {
    /// Optional path to the project directory. Default = primary workspace root.
    #[serde(default)]
    pub path: Option<String>,
}

/// Standalone configuration for testing project endpoints.
#[derive(Debug, Clone)]
pub struct ProjectApiConfig {
    pub allowed_workspace_roots: Vec<PathBuf>,
}

impl Default for ProjectApiConfig {
    fn default() -> Self {
        Self {
            allowed_workspace_roots: vec![PathBuf::from(".")],
        }
    }
}

/// Assembles project routes for the global AppState.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/project/git-status",
            get(handle_get_project_git_status_state),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
        .with_state(state)
}

/// Standalone router for testing with dedicated configuration.
pub fn router_for_config(config: ProjectApiConfig) -> Router {
    Router::new()
        .route(
            "/api/project/git-status",
            get({
                let cfg = config;
                move |Query(query): Query<GitStatusQuery>| {
                    let roots = cfg.allowed_workspace_roots.clone();
                    async move { handle_get_project_git_status_impl(query, roots).await }
                }
            }),
        )
        .layer(RequestBodyLimitLayer::new(FLEET_MAX_BODY))
}

/// Standalone router with default configuration.
pub fn router() -> Router {
    router_for_config(ProjectApiConfig::default())
}

/// Resolves all valid workspace roots configured in the system.
pub fn resolve_configured_workspace_roots(config: &galleon_config::schema::Config) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    roots.push(config.install_root_dir());
    roots.push(config.shared_workspace_dir());
    roots.push(config.data_dir.clone());
    for alias in config.agents.keys() {
        roots.push(config.agent_workspace_dir(alias));
    }
    if config.agents.is_empty() {
        roots.push(config.agent_workspace_dir("default"));
    }
    for root in &config.runtime.docker.allowed_workspace_roots {
        roots.push(PathBuf::from(root));
    }
    roots
}

async fn handle_get_project_git_status_state(
    State(state): State<AppState>,
    Query(query): Query<GitStatusQuery>,
) -> Response {
    let roots = {
        let cfg = state.config.read();
        resolve_configured_workspace_roots(&cfg)
    };
    handle_get_project_git_status_impl(query, roots).await
}

/// Implementation handler for project git-status with explicit allowed workspace roots.
pub async fn handle_get_project_git_status_impl(
    query: GitStatusQuery,
    allowed_roots: Vec<PathBuf>,
) -> Response {
    let primary_root = allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));

    let raw_path = query.path.as_deref().unwrap_or("").trim();
    let candidate = if raw_path.is_empty() || raw_path == "." {
        primary_root
    } else {
        let p = std::path::Path::new(raw_path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            primary_root.join(p)
        }
    };

    let canonical_target = match candidate.canonicalize() {
        Ok(p) => p,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "application/json")],
                json!({
                    "error": format!("path does not exist or cannot be resolved: {err}")
                })
                .to_string(),
            )
                .into_response();
        }
    };

    if !canonical_target.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            json!({
                "error": format!("path '{}' is not a directory", canonical_target.display())
            })
            .to_string(),
        )
            .into_response();
    }

    let is_allowed = allowed_roots.iter().any(|root| {
        if let Ok(canonical_root) = root.canonicalize() {
            canonical_target.starts_with(&canonical_root)
        } else {
            false
        }
    });

    if !is_allowed {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            json!({
                "error": format!("path '{}' is outside configured workspace", candidate.display())
            })
            .to_string(),
        )
            .into_response();
    }

    // Direct invocation without shell (tokio::process::Command)
    let git_output = match tokio::process::Command::new("git")
        .arg("-C")
        .arg(&canonical_target)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("-b")
        .output()
        .await
    {
        Ok(out) => out,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                json!({
                    "error": format!("failed to invoke git: {err}")
                })
                .to_string(),
            )
                .into_response();
        }
    };

    if !git_output.status.success() {
        let stderr = String::from_utf8_lossy(&git_output.stderr);
        let code = git_output.status.code().unwrap_or(-1);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            json!({
                "error": format!("git status failed with exit code {code}: {}", stderr.trim())
            })
            .to_string(),
        )
            .into_response();
    }

    let stdout = String::from_utf8_lossy(&git_output.stdout);
    let git_status = parse_git_status(&stdout);

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json!(git_status).to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_git_status_success_in_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let ws_root = tmp.path().to_path_buf();

        // Initialize a clean git repo inside workspace
        let init_status = std::process::Command::new("git")
            .arg("-C")
            .arg(&ws_root)
            .args(["init", "-b", "main"])
            .status()
            .unwrap();
        assert!(init_status.success());

        // Configure git identity for commit
        let _ = std::process::Command::new("git")
            .arg("-C")
            .arg(&ws_root)
            .args(["config", "user.name", "Test User"])
            .status();
        let _ = std::process::Command::new("git")
            .arg("-C")
            .arg(&ws_root)
            .args(["config", "user.email", "test@example.com"])
            .status();

        // Write a file and commit it
        std::fs::write(ws_root.join("README.md"), "# Test Repo").unwrap();
        let _ = std::process::Command::new("git")
            .arg("-C")
            .arg(&ws_root)
            .args(["add", "README.md"])
            .status();
        let _ = std::process::Command::new("git")
            .arg("-C")
            .arg(&ws_root)
            .args(["commit", "-m", "initial commit"])
            .status();

        let cfg = ProjectApiConfig {
            allowed_workspace_roots: vec![ws_root.clone()],
        };
        let app = router_for_config(cfg);

        let req = Request::builder()
            .uri(format!(
                "/api/project/git-status?path={}",
                ws_root.display()
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(val["branch"], "main");
        assert_eq!(val["clean"], true);
        assert_eq!(val["risk_tier"], "READ");
        assert_eq!(val["policy_check"], "evaluated-pass");
        assert_eq!(val["staged"], json!([]));
        assert_eq!(val["unstaged"], json!([]));
        assert_eq!(val["untracked"], json!([]));
    }

    #[tokio::test]
    async fn test_git_status_rejects_path_outside_workspace() {
        let ws_tmp = tempfile::tempdir().unwrap();
        let outside_tmp = tempfile::tempdir().unwrap();

        let cfg = ProjectApiConfig {
            allowed_workspace_roots: vec![ws_tmp.path().to_path_buf()],
        };
        let app = router_for_config(cfg);

        let req = Request::builder()
            .uri(format!(
                "/api/project/git-status?path={}",
                outside_tmp.path().display()
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(val["error"]
            .as_str()
            .unwrap()
            .contains("outside configured workspace"));
    }

    #[tokio::test]
    async fn test_git_status_rejects_parent_traversal() {
        let ws_tmp = tempfile::tempdir().unwrap();
        let outside_dir = ws_tmp.path().parent().unwrap();

        let cfg = ProjectApiConfig {
            allowed_workspace_roots: vec![ws_tmp.path().to_path_buf()],
        };
        let app = router_for_config(cfg);

        let traversal_path = ws_tmp.path().join("../");
        let req = Request::builder()
            .uri(format!(
                "/api/project/git-status?path={}",
                traversal_path.display()
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(val["error"]
            .as_str()
            .unwrap()
            .contains("outside configured workspace"));
    }

    #[tokio::test]
    async fn test_git_status_rejects_symlink_escape() {
        let ws_tmp = tempfile::tempdir().unwrap();
        let outside_tmp = tempfile::tempdir().unwrap();

        let symlink_path = ws_tmp.path().join("escape_link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside_tmp.path(), &symlink_path).unwrap();

        let cfg = ProjectApiConfig {
            allowed_workspace_roots: vec![ws_tmp.path().to_path_buf()],
        };
        let app = router_for_config(cfg);

        let req = Request::builder()
            .uri(format!(
                "/api/project/git-status?path={}",
                symlink_path.display()
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(val["error"]
            .as_str()
            .unwrap()
            .contains("outside configured workspace"));
    }

    #[tokio::test]
    async fn test_git_status_honest_error_when_not_a_git_repo() {
        let ws_tmp = tempfile::tempdir().unwrap();
        let not_git_dir = ws_tmp.path().join("not_a_repo");
        std::fs::create_dir_all(&not_git_dir).unwrap();

        let cfg = ProjectApiConfig {
            allowed_workspace_roots: vec![ws_tmp.path().to_path_buf()],
        };
        let app = router_for_config(cfg);

        let req = Request::builder()
            .uri(format!(
                "/api/project/git-status?path={}",
                not_git_dir.display()
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        // Honest 5xx error instead of Go's fake 200 OK success
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(val["error"]
            .as_str()
            .unwrap()
            .contains("git status failed with exit code"));
    }

    #[tokio::test]
    async fn test_git_status_rejects_non_existent_path() {
        let ws_tmp = tempfile::tempdir().unwrap();
        let non_existent = ws_tmp.path().join("does_not_exist");

        let cfg = ProjectApiConfig {
            allowed_workspace_roots: vec![ws_tmp.path().to_path_buf()],
        };
        let app = router_for_config(cfg);

        let req = Request::builder()
            .uri(format!(
                "/api/project/git-status?path={}",
                non_existent.display()
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(val["error"]
            .as_str()
            .unwrap()
            .contains("path does not exist or cannot be resolved"));
    }

    #[tokio::test]
    async fn test_git_status_rejects_file_target() {
        let ws_tmp = tempfile::tempdir().unwrap();
        let file_path = ws_tmp.path().join("a_file.txt");
        std::fs::write(&file_path, "contents").unwrap();

        let cfg = ProjectApiConfig {
            allowed_workspace_roots: vec![ws_tmp.path().to_path_buf()],
        };
        let app = router_for_config(cfg);

        let req = Request::builder()
            .uri(format!(
                "/api/project/git-status?path={}",
                file_path.display()
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let val: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(val["error"]
            .as_str()
            .unwrap()
            .contains("is not a directory"));
    }
}
