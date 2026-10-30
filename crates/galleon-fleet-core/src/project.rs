//! Domain model and parser for project git status.
//!
//! Under SPEC-013 §Model Data (TASK-075):
//! - `GET /api/project/git-status?path=`:
//!   - Parses porcelain v1 output from `git status --porcelain=v1 -b`.
//!   - Accurately tracks branch name, staged files, unstaged files, and untracked files.
//!   - Evaluates cleanliness (`clean: true` iff no staged, unstaged, or untracked changes).
//!   - Returns honest operational risk and policy tier (`risk_tier: "READ"`, `policy_check: "evaluated-pass"`).

use serde::{Deserialize, Serialize};

/// Project git status model matching `web/src/utils/apiClient.ts` `ProjectGitStatus` contract
/// and Go fixture `GET_api_project_git_status.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectGitStatus {
    /// Active branch name or HEAD commit description.
    pub branch: String,
    /// Whether working tree and index are clean.
    pub clean: bool,
    /// List of staged files (changes added to the index).
    pub staged: Vec<String>,
    /// List of unstaged files (tracked changes not added to index).
    pub unstaged: Vec<String>,
    /// List of untracked files.
    pub untracked: Vec<String>,
    /// Evaluated terminal risk tier (defaults to "READ" for git status).
    pub risk_tier: String,
    /// Security policy evaluation status ("evaluated-pass").
    pub policy_check: String,
}

/// Parses standard `git status --porcelain=v1 -b` output.
#[must_use]
pub fn parse_git_status(stdout: &str) -> ProjectGitStatus {
    let mut branch = "main".to_string();
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim_end_matches('\r');
        if trimmed.is_empty() {
            continue;
        }

        // Branch line format: `## <branch>...<upstream>` or `## <branch>`
        if let Some(rest) = trimmed.strip_prefix("## ") {
            let branch_str = if let Some(b) = rest.strip_prefix("No commits yet on ") {
                b.trim()
            } else if let Some(b) = rest.strip_prefix("Initial commit on ") {
                b.trim()
            } else {
                let first_part = rest.split("...").next().unwrap_or(rest);
                first_part.split_whitespace().next().unwrap_or(first_part)
            };
            if !branch_str.is_empty() {
                branch = branch_str.to_string();
            }
            continue;
        }

        // Status lines: XY PATH
        // X = staged (index status), Y = unstaged (work tree status)
        if trimmed.len() >= 3 {
            let bytes = trimmed.as_bytes();
            let x = bytes[0] as char;
            let y = bytes[1] as char;
            let file = trimmed[2..].trim_start();
            let file_clean = if file.starts_with('"') && file.ends_with('"') && file.len() >= 2 {
                &file[1..file.len() - 1]
            } else {
                file
            };

            if x == '?' && y == '?' {
                untracked.push(file_clean.to_string());
            } else {
                if x != ' ' && x != '?' && x != '!' {
                    staged.push(file_clean.to_string());
                }
                if y != ' ' && y != '?' && y != '!' {
                    unstaged.push(file_clean.to_string());
                }
            }
        }
    }

    let clean = staged.is_empty() && unstaged.is_empty() && untracked.is_empty();

    ProjectGitStatus {
        branch,
        clean,
        staged,
        unstaged,
        untracked,
        risk_tier: "READ".to_string(),
        policy_check: "evaluated-pass".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_status() {
        let raw = "## main...origin/main [ahead 1]\n";
        let status = parse_git_status(raw);
        assert_eq!(status.branch, "main");
        assert!(status.clean);
        assert!(status.staged.is_empty());
        assert!(status.unstaged.is_empty());
        assert!(status.untracked.is_empty());
        assert_eq!(status.risk_tier, "READ");
        assert_eq!(status.policy_check, "evaluated-pass");
    }

    #[test]
    fn test_parse_modified_and_untracked() {
        let raw = r#"## feat/testing
M  crates/core/src/lib.rs
 M crates/gateway/src/main.rs
MM crates/both/src/mod.rs
?? new_file.txt
"#;
        let status = parse_git_status(raw);
        assert_eq!(status.branch, "feat/testing");
        assert!(!status.clean);
        assert_eq!(
            status.staged,
            vec!["crates/core/src/lib.rs", "crates/both/src/mod.rs"]
        );
        assert_eq!(
            status.unstaged,
            vec!["crates/gateway/src/main.rs", "crates/both/src/mod.rs"]
        );
        assert_eq!(status.untracked, vec!["new_file.txt"]);
    }

    #[test]
    fn test_parse_initial_commit() {
        let raw = "## Initial commit on master\n?? README.md\n";
        let status = parse_git_status(raw);
        assert_eq!(status.branch, "master");
        assert!(!status.clean);
        assert_eq!(status.untracked, vec!["README.md"]);
    }
}
