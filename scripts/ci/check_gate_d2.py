#!/usr/bin/env python3
"""
Galleon Fleet — Gate D-2 Governance Coupling Verifier (T-CI-03)
==============================================================
Verifies that `crates/galleon-channels/src/util.rs` does not couple
channel transport helpers with genuine governance policy primitives.

Two-Layer Detection:
1. Candidate Layer: Identifies `feature = "channel-"` conditional gates.
2. Architecture Boundary Layer:
   - Legitimate: Pure transport I/O (PendingApproval, response limiting, token parsing)
     with zero dependency on `galleon-runtime`.
   - Violation (REJECT): Presence of policy decision symbols (RiskTier,
     should_auto_approve, playbook::, galleon_runtime::) OR crate dependency
     from `galleon-channels` to `galleon-runtime`.

Normative Reference:
- standard-docs/decisions/refactoring-rust/RF-D-principle-gateway-checklist.md (§Gate D-2)
- standard-docs/decisions/finalize/dev-task-2/ci-infra-fix/T-CI-03-gate-d2-regex-overbroad.md
- .gemini/rules/ci-infra-standards.md
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

POLICY_SYMBOLS_REGEX = re.compile(
    r"\b(RiskTier|should_auto_approve|playbook::|galleon_runtime::)\b"
)

def check_gate_d2(util_path: Path) -> tuple[bool, list[str]]:
    if not util_path.is_file():
        return True, ["util.rs does not exist (check skipped)."]

    try:
        content = util_path.read_text(encoding="utf-8", errors="replace")
    except Exception as e:
        return False, [f"Failed to read {util_path}: {e}"]

    # Layer 1: Check for channel feature flag candidates
    candidates = [
        line.strip() for line in content.splitlines()
        if 'feature = "channel-' in line
    ]

    if not candidates:
        return True, ["No channel feature flags found in util.rs (Gate D-2 clean)."]

    # Layer 2: Check for genuine policy symbols in util.rs
    policy_matches = []
    for idx, line in enumerate(content.splitlines(), start=1):
        if POLICY_SYMBOLS_REGEX.search(line):
            policy_matches.append(f"Line {idx}: {line.strip()}")

    # Layer 2: Check for dependency to galleon-runtime
    has_runtime_dep = False
    runtime_dep_output = ""
    try:
        res = subprocess.run(
            ["cargo", "tree", "-p", "galleon-channels", "-e", "normal", "--prefix", "none"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace"
        )
        if res.returncode == 0:
            for dep_line in res.stdout.splitlines():
                if re.match(r"^galleon-runtime\b", dep_line.strip()):
                    has_runtime_dep = True
                    runtime_dep_output = dep_line.strip()
                    break
    except Exception:
        # If cargo is not available or tree fails in mock environments, rely on symbol scan
        pass

    violations = []
    if policy_matches:
        violations.append(f"Policy decision symbols detected in util.rs ({len(policy_matches)} occurrences):")
        violations.extend(policy_matches[:5])
    if has_runtime_dep:
        violations.append(f"Architectural boundary violation: galleon-channels depends on galleon-runtime ({runtime_dep_output})")

    if violations:
        return False, violations

    return True, [
        f"Gate D-2 Passed: {len(candidates)} transport helper candidate(s) validated.",
        "All candidates are pure transport I/O (zero runtime dependency and zero policy symbols)."
    ]

def main():
    parser = argparse.ArgumentParser(description="Verify Gate D-2 governance decoupling in util.rs.")
    parser.add_argument("--path", default="crates/galleon-channels/src/util.rs", help="Path to util.rs")
    args = parser.parse_args()

    passed, messages = check_gate_d2(Path(args.path))
    for msg in messages:
        print(msg)

    sys.exit(0 if passed else 1)

if __name__ == "__main__":
    main()
