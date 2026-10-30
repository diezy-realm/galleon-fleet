//! Architecture gate: risk-tier & fleet-policy facts are served by Galleon Fleet
//! (`GET /api/fleet/policies`) with `galleon_config::policy::CommandRiskLevel` as the
//! Rust backend operational source of truth.
//! The web tier (`web/src`) and fleet domain crates (`crates/galleon-fleet-core/src`)
//! must not re-declare static forbidden tables.
//!
//! Under SPEC-013 / TASK-072:
//! - `SCAN_ROOTS` covers `web/src` and `crates/galleon-fleet-core/src`.
//! - `scan_dir` panics if any scan root directory does not exist, preventing silent passes.

use std::fs;
use std::path::{Path, PathBuf};

/// Identifiers that re-declare the risk-tier / policy tables. Their facts
/// (tier, approvalRequired, maxBudgetUSD, enforcement) live in fleet policy models.
/// Duplicate domain state is forbidden. Add `// SOT: <reason>` on the line to override.
const FORBIDDEN_TS_SYMBOLS: &[&str] = &["initialRiskTiers", "initialFleetPolicies"];

/// Roots to scan for the forbidden re-declarations.
const SCAN_ROOTS: &[&str] = &["web/src", "crates/galleon-fleet-core/src"];

#[test]
fn web_tier_does_not_redeclare_risk_tier_or_policy_state() {
    let workspace_root = workspace_root();
    let mut violations: Vec<String> = Vec::new();
    for root in SCAN_ROOTS {
        scan_dir(&workspace_root.join(root), &mut violations);
    }
    assert!(
        violations.is_empty(),
        "Duplicate risk-tier/policy state detected. These facts are served by \
         GET /api/fleet/policies; consumers must fetch them, not redefine them — \
         see .kiro/steering/galleon-product-fundamentals.md (risk-tier SSOT) \
         and docs/finalize/08 (F4-2).\n\
         To override, add `// SOT: <reason>` on the offending line.\n\n\
         Violations:\n{}",
        violations.join("\n")
    );
}

#[test]
#[should_panic(expected = "scan root directory does not exist or cannot be read")]
fn scan_dir_panics_when_root_does_not_exist() {
    let mut violations = Vec::new();
    let non_existent = workspace_root().join("non_existent_scan_root_path_verify_panic");
    scan_dir(&non_existent, &mut violations);
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn scan_dir(dir: &Path, violations: &mut Vec<String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| {
        panic!(
            "scan root directory does not exist or cannot be read: {}: {e}",
            dir.display()
        )
    });
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, violations);
            continue;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some("ts") | Some("tsx") | Some("rs") => {}
            _ => continue,
        }
        let Ok(src) = fs::read_to_string(&path) else {
            continue;
        };
        let display = path.display().to_string();
        for (lineno, line) in src.lines().enumerate() {
            if line.contains("// SOT:") {
                continue;
            }
            // Only a DECLARATION is a duplicate; a mention in a comment or an
            // import is not. Match `export const <sym>` / `const <sym>` / `<sym> =`.
            for sym in FORBIDDEN_TS_SYMBOLS {
                let decl_export = format!("export const {sym}");
                let decl_const = format!("const {sym}");
                let decl_assign = format!("{sym} =");
                let is_comment = line.trim_start().starts_with("//");
                if is_comment {
                    continue;
                }
                if line.contains(&decl_export)
                    || line.contains(&decl_const)
                    || (line.contains(&decl_assign) && !line.contains("import"))
                {
                    violations.push(format!(
                        "  {}:{}: {}",
                        display,
                        lineno + 1,
                        line.trim_start()
                    ));
                }
            }
        }
    }
}
