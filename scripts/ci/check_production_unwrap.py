#!/usr/bin/env python3
"""
Galleon Fleet — Production Zero-Unwrap Gate (T-CI-01)
=====================================================
Audits pull request diff for `.unwrap()` and `.expect()` calls in Rust code.
Ensures that zero unwrap()/expect() calls are introduced in production paths,
while legitimately permitting them inside unit test scopes:
- Inline test modules (`mod tests { ... }`, `pub(crate) mod tests { ... }`, `#[cfg(test)]`)
- Test files (`tests.rs`, `*_tests.rs`, `tests/**`, `benches/**`)

Normative Reference: AGENTS.md, .gemini/rules/ci-infra-standards.md
"""

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

def is_test_file(path_str: str) -> bool:
    posix_path = path_str.replace("\\", "/")
    parts = posix_path.split("/")
    filename = parts[-1]
    
    if "tests" in parts or "benches" in parts:
        return True
    if filename == "tests.rs" or filename.endswith("_tests.rs"):
        return True
    return False

def get_mod_tests_start_line(file_path: Path) -> int | None:
    if not file_path.is_file():
        return None
    try:
        with open(file_path, "r", encoding="utf-8", errors="replace") as f:
            lines = f.readlines()
        
        # Check for start of test module
        for idx, line in enumerate(lines):
            # Matches: mod tests {, pub(crate) mod tests {, #[cfg(test)] mod tests
            if re.search(r"(?:pub(?:\([^)]+\))?\s+)?mod\s+tests\b", line):
                return idx + 1
            if re.search(r"#\[cfg\(test\)\]", line):
                # If #[cfg(test)] is followed closely by mod, mark this as start
                for peek in lines[idx:min(idx + 5, len(lines))]:
                    if "mod " in peek:
                        return idx + 1
        return None
    except Exception:
        return None

def main():
    parser = argparse.ArgumentParser(description="Check for unwrap/expect in production code.")
    parser.add_argument("--base", default="origin/testing", help="Base git ref for diff")
    parser.add_argument("--head", default="HEAD", help="Head git ref for diff")
    parser.add_argument("--summary-only", action="store_true", help="Print only violation count")
    args = parser.parse_args()

    base_ref = os.environ.get("BASE_REF", args.base)
    head_ref = args.head

    # Run git diff for Rust files in crates
    cmd = [
        "git", "diff", "-U0", f"{base_ref}...{head_ref}",
        "--", "crates/**/*.rs",
        ":!crates/**/tests/**",
        ":!crates/**/benches/**",
        ":!crates/**/tests.rs",
        ":!crates/**/*/tests.rs"
    ]

    try:
        proc = subprocess.run(cmd, capture_output=True, encoding="utf-8", errors="replace", check=True)
    except subprocess.CalledProcessError as e:
        # Fallback to direct two-dot diff if three-dot fails
        cmd[3] = f"{base_ref}..{head_ref}"
        proc = subprocess.run(cmd, capture_output=True, encoding="utf-8", errors="replace")
        if proc.returncode != 0:
            print(f"Error executing git diff: {proc.stderr}", file=sys.stderr)
            sys.exit(1)

    diff_lines = proc.stdout.splitlines()

    current_file = None
    hunk_new_line = 0
    violations = []
    file_mod_tests_cache = {}

    unwrap_regex = re.compile(r"\.(unwrap|expect)\(")

    for line in diff_lines:
        if line.startswith("+++ b/"):
            current_file = line[6:].strip()
        elif line.startswith("@@ "):
            # Format: @@ -old,len +new_start,len @@
            m = re.search(r"\+(\d+)", line)
            if m:
                hunk_new_line = int(m.group(1))
        elif line.startswith("+") and not line.startswith("+++"):
            if current_file and unwrap_regex.search(line):
                # Check if file is test file
                if not is_test_file(current_file):
                    file_path = Path(current_file)
                    if current_file not in file_mod_tests_cache:
                        file_mod_tests_cache[current_file] = get_mod_tests_start_line(file_path)

                    mod_tests_line = file_mod_tests_cache[current_file]
                    # If line is before mod tests or no mod tests exists, it is production code
                    if mod_tests_line is None or hunk_new_line < mod_tests_line:
                        violations.append({
                            "file": current_file,
                            "line": hunk_new_line,
                            "content": line[1:].strip()
                        })
            hunk_new_line += 1

    if args.summary_only:
        print(len(violations))
        return

    if violations:
        print(f"FAILED: Found {len(violations)} unwrap()/expect() call(s) in Rust production paths:")
        for v in violations:
            print(f"  {v['file']}:{v['line']} -> {v['content']}")
        sys.exit(1)
    else:
        print("PASSED: Zero unwrap()/expect() calls in Rust production paths.")
        sys.exit(0)

if __name__ == "__main__":
    main()
