//! Contract and unit tests for project git-status domain model and parser.
//!
//! Under SPEC-013 §Model Data (TASK-075):
//! - Parity with Go fixture `GET_api_project_git_status.json`.
//! - Strict compliance with TS `ProjectGitStatus` contract in `web/src/utils/apiClient.ts`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use galleon_fleet_core::project::parse_git_status;

#[test]
fn test_project_git_status_matches_go_fixture() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/go-fleet/GET_api_project_git_status.json");
    let fixture_str = std::fs::read_to_string(&fixture_path)
        .expect("fixtures/go-fleet/GET_api_project_git_status.json must exist");
    let fixture_val: serde_json::Value = serde_json::from_str(&fixture_str).unwrap();

    let raw_git_status = "## main...origin/main\n";
    let status = parse_git_status(raw_git_status);
    let serialized = serde_json::to_value(&status).unwrap();

    assert_eq!(serialized, fixture_val);
}

#[test]
fn test_parse_git_status_complex_cases() {
    let stdout = r#"## feat/migration...origin/feat/migration [ahead 2, behind 1]
M  crates/core/src/lib.rs
 M crates/gateway/src/main.rs
MM crates/both/src/state.rs
A  new_staged.txt
D  deleted_staged.txt
 D deleted_unstaged.txt
?? untracked1.rs
?? "path with spaces/file.txt"
"#;
    let res = parse_git_status(stdout);
    assert_eq!(res.branch, "feat/migration");
    assert!(!res.clean);
    assert_eq!(
        res.staged,
        vec![
            "crates/core/src/lib.rs",
            "crates/both/src/state.rs",
            "new_staged.txt",
            "deleted_staged.txt"
        ]
    );
    assert_eq!(
        res.unstaged,
        vec![
            "crates/gateway/src/main.rs",
            "crates/both/src/state.rs",
            "deleted_unstaged.txt"
        ]
    );
    assert_eq!(
        res.untracked,
        vec!["untracked1.rs", "path with spaces/file.txt"]
    );
    assert_eq!(res.risk_tier, "READ");
    assert_eq!(res.policy_check, "evaluated-pass");
}

#[test]
fn test_parse_git_status_no_commits_yet() {
    let stdout = "## No commits yet on staging\n?? file.txt\n";
    let res = parse_git_status(stdout);
    assert_eq!(res.branch, "staging");
    assert!(!res.clean);
    assert_eq!(res.untracked, vec!["file.txt"]);
}

#[test]
fn test_parse_git_status_detached_head() {
    let stdout = "## HEAD (no branch)\n";
    let res = parse_git_status(stdout);
    assert_eq!(res.branch, "HEAD");
    assert!(res.clean);
}
