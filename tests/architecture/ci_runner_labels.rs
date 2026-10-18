//! Architecture gate for Quality Gate runner selection.
//!
//! CI runs on the self-hosted runners declared in `~/Apps/dev-ci` (outside this repo). Compile
//! jobs go to `galleon-heavy`; guards and governance go to `galleon-light` so a
//! long build never starves auto-merge. This gate keeps the job inventory and
//! the labels honest, and keeps third-party paid runners out of the workflows.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use regex::Regex;

const HEAVY_LABEL: &str = "runs-on: [self-hosted, linux, galleon-heavy]";
const LIGHT_LABEL: &str = "runs-on: [self-hosted, linux, galleon-light]";

const HEAVY_JOBS: [&str; 3] = ["lint", "check", "test"];
const LIGHT_JOBS: [&str; 5] = [
    "fmt",
    "repo-structure",
    "zerocode-rpc-boundary",
    "security",
    "gate",
];

fn read(rel: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("failed to read {rel}: {e}"))
}

/// Split the `jobs:` mapping into `job id -> job body`.
fn job_blocks(workflow: &str) -> BTreeMap<String, String> {
    let (_, jobs) = workflow
        .split_once("\njobs:\n")
        .expect("ci.yml must declare a top-level jobs mapping");
    let header = Regex::new(r"(?m)^  ([a-z0-9-]+):$").expect("valid job-header pattern");
    let starts: Vec<(usize, String)> = header
        .captures_iter(jobs)
        .map(|c| {
            (
                c.get(0).expect("match 0 always exists").start(),
                c[1].to_string(),
            )
        })
        .collect();
    assert!(!starts.is_empty(), "ci.yml must define at least one job");

    let mut blocks = BTreeMap::new();
    for (i, (start, name)) in starts.iter().enumerate() {
        let end = starts.get(i + 1).map_or(jobs.len(), |next| next.0);
        let previous = blocks.insert(name.clone(), jobs[*start..end].to_string());
        assert!(previous.is_none(), "duplicate job id {name} in ci.yml");
    }
    blocks
}

#[test]
fn quality_gate_jobs_pin_their_runner_class() {
    let blocks = job_blocks(&read(".github/workflows/ci.yml"));
    for (jobs, label) in [
        (HEAVY_JOBS.as_slice(), HEAVY_LABEL),
        (LIGHT_JOBS.as_slice(), LIGHT_LABEL),
    ] {
        for name in jobs {
            let block = blocks
                .get(*name)
                .unwrap_or_else(|| panic!("ci.yml lost job {name}"));
            assert!(block.contains(label), "{name} must use `{label}`");
        }
    }
}

#[test]
fn quality_gate_has_no_undeclared_jobs() {
    let blocks = job_blocks(&read(".github/workflows/ci.yml"));
    let declared: BTreeSet<&str> = HEAVY_JOBS
        .iter()
        .chain(LIGHT_JOBS.iter())
        .copied()
        .collect();
    let actual: BTreeSet<&str> = blocks.keys().map(String::as_str).collect();
    assert_eq!(
        actual, declared,
        "declare new ci.yml jobs in the label lists above"
    );
}

#[test]
fn the_required_gate_waits_for_every_other_job() {
    let blocks = job_blocks(&read(".github/workflows/ci.yml"));
    let gate = &blocks["gate"];
    for name in HEAVY_JOBS
        .iter()
        .chain(LIGHT_JOBS.iter())
        .filter(|n| **n != "gate")
    {
        assert!(gate.contains(name), "gate must need {name}");
    }
}

#[test]
fn workflows_do_not_depend_on_paid_third_party_runners() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github");
    let pattern = Regex::new(r"useblacksmith|blacksmith-\d").expect("valid pattern");
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("readable .github tree").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
                let text = fs::read_to_string(&path).expect("readable workflow file");
                assert!(
                    !pattern.is_match(&text),
                    "{} references a paid third-party runner",
                    path.display()
                );
            }
        }
    }
}
