//! Invariant R-03: module size limits and file ratchet gate.
//!
//! Enforces the limits defined in standard-docs/tech-standart/code-quality/rust-module-size-and-decomposition-standard.md:
//! - Non-test source files must not exceed 5,000 lines (hard CI gate).
//! - Test files must not exceed 10,000 lines.
//! - Pre-existing legacy files that exceed limits are tracked in a dated exception baseline.
//!   They are locked to their baseline and cannot grow larger without refactoring.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

const MAX_SOURCE_FILE_LINES: usize = 5000;
const MAX_TEST_FILE_LINES: usize = 10000;

/// Baseline exception registry: (Normalized path, max allowed line cap, date recorded)
/// Invariant: Legacy files must ratchet downward over time; no new files may exceed limits.
fn legacy_size_exceptions() -> HashMap<&'static str, usize> {
    let mut m = HashMap::new();
    m.insert("src/main.rs", 10500);
    m.insert("crates/galleon-channel-discord/src/lib.rs", 9000);
    m.insert("crates/galleon-channel-lark/src/lib.rs", 8500);
    m.insert("crates/galleon-channel-matrix/src/lib.rs", 6000);
    m.insert("crates/galleon-channel-mattermost/src/lib.rs", 7000);
    m.insert("crates/clawcrew-runtime/src/acp/server.rs", 11000);
    m.insert("crates/clawcrew-channels/src/orchestrator/mod.rs", 19000);
    m.insert("crates/clawcrew-channels/src/orchestrator/tests.rs", 35000);
    m.insert("crates/galleon-channel-slack/src/lib.rs", 11000);
    m.insert("crates/galleon-channel-telegram/src/tests.rs", 16000);
    m.insert("crates/galleon-channel-telegram/src/lib.rs", 9000);
    m.insert("crates/galleon-channel-wechat/src/lib.rs", 7500);
    m.insert("crates/galleon-channel-whatsapp/src/whatsapp_web.rs", 8500);
    m.insert("crates/clawcrew-config/src/policy.rs", 11000);
    m.insert("crates/clawcrew-config/src/schema/tests.rs", 22000);
    m.insert("crates/clawcrew-config/src/schema.rs", 28000);
    m.insert("crates/clawcrew-gateway/src/api.rs", 6500);
    m.insert("crates/clawcrew-memory/src/sqlite.rs", 6000);
    m.insert("crates/clawcrew-providers/src/lib.rs", 7000);
    m.insert("crates/clawcrew-runtime/src/agent/agent/tests.rs", 13000);
    m.insert("crates/clawcrew-runtime/src/agent/loop_/tests.rs", 18000);
    m.insert("crates/clawcrew-runtime/src/agent/turn/mod.rs", 8000);
    m.insert("crates/clawcrew-runtime/src/daemon/mod.rs", 7000);
    m.insert("crates/clawcrew-runtime/src/skills/mod.rs", 6000);
    m.insert("crates/clawcrew-runtime/src/sop/engine/mod.rs", 7000);
    m.insert("crates/clawcrew-runtime/src/tools/delegate/tests.rs", 11000);
    m.insert("crates/clawcrew-runtime/src/tools/mod.rs", 6000);
    m.insert("crates/galleon-tool-call-parser/src/lib.rs", 6000);
    m.insert("crates/clawcrew-tools/src/a2a_client.rs", 6500);
    m.insert("crates/clawcrew-tools/src/git_operations.rs", 6000);
    m.insert("crates/clawcrew-tools/src/web_search_tool.rs", 6500);
    m
}

fn visit_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "target" && name != "fixtures" && name != ".git" {
                    visit_rs_files(&path, files);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
}

#[test]
fn rust_source_files_do_not_exceed_hard_size_limits() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    let mut files = Vec::new();
    let exceptions = legacy_size_exceptions();

    for search_dir in ["src", "crates"] {
        let dir_path = root.join(search_dir);
        if dir_path.exists() {
            visit_rs_files(&dir_path, &mut files);
        }
    }

    for path in files {
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        let line_count = content.lines().count();
        let is_test = rel_path.contains("/tests/")
            || rel_path.ends_with("_test.rs")
            || rel_path.ends_with("tests.rs");

        let max_allowed = if let Some(&cap) = exceptions.get(rel_path.as_str()) {
            cap
        } else if is_test {
            MAX_TEST_FILE_LINES
        } else {
            MAX_SOURCE_FILE_LINES
        };

        if line_count > max_allowed {
            violations.push(format!(
                "{} has {} lines (exceeds limit of {})",
                rel_path, line_count, max_allowed
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Rust module size limit violations detected (R-03 ratchet):\n{}",
        violations.join("\n")
    );
}
