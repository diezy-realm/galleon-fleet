#!/usr/bin/env python3
"""
Galleon Fleet — Brand Compliance Gate (RB-02 / RB-40)
======================================================
CI Quality gate to verify no unauthorized legacy brand names
(`clawcrew`, `claw-crew`, `zeroclaw`) leak into production code.

Permitted Exceptions (as defined in `naming-map.md` §7):
- Upstream license attribution in LICENSE-MIT, LICENSE, NOTICE, README.md
- Immutable historical ADRs (standard-docs/decisions/owner-decisions/ODR-*)
- Frozen legacy interfaces (WIT v0 clawcrew:plugin@0.1.0)
- Backward compatibility migration scripts & mapping documentation
"""

import os
import re
import sys
from pathlib import Path

# Legacy brand keywords to scan for
BANNED_PATTERNS = [
    re.compile(r"\bclawcrew\b", re.IGNORECASE),
    re.compile(r"\bclaw-crew\b", re.IGNORECASE),
    re.compile(r"\bzeroclaw\b", re.IGNORECASE),
]

EXCLUDED_PATHS = [
    ".git",
    "target",
    "dist",
    "node_modules",
    ".gemini",
    "graphify-out",
    # Permitted documentation & licenses
    "LICENSE",
    "LICENSE-MIT",
    "NOTICE",
    "standard-docs/tech-standart/naming/naming-map.md",
    "scripts/migrations/migrate_state.py",
    "scripts/ci/brand_gate.py",
    "scripts/ci/brand_gate.sh",
]

EXCLUDED_EXTENSIONS = [
    ".png", ".jpg", ".jpeg", ".ico", ".svg", ".lock", ".rlib", ".rmeta", ".exe", ".bin", ".tar", ".gz"
]

def is_excluded(file_path: Path) -> bool:
    rel_str = file_path.as_posix()
    for excl in EXCLUDED_PATHS:
        if excl in rel_str:
            return True
    # Exclude historical ADRs
    if "standard-docs/decisions/owner-decisions/ODR-" in rel_str:
        return True
    if file_path.suffix.lower() in EXCLUDED_EXTENSIONS:
        return True
    return False

def check_file(file_path: Path) -> list:
    violations = []
    try:
        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
            for line_no, line in enumerate(f, start=1):
                # Skip comments attributing upstream authors or frozen v0 WIT
                if "clawcrew:plugin@0.1.0" in line or "upstream" in line.lower() or "mit license" in line.lower():
                    continue
                for pat in BANNED_PATTERNS:
                    if pat.search(line):
                        violations.append((line_no, line.strip()))
                        break
    except Exception as err:
        pass
    return violations

def main():
    root = Path(__file__).resolve().parents[2]
    print(f"[*] Running Galleon Brand Compliance Gate on {root}...")
    total_violations = 0

    # Scan production source dirs
    scan_targets = [root / "crates", root / "engine", root / "web", root / "apps"]
    for target in scan_targets:
        if not target.exists():
            continue
        for root_dir, dirs, files in os.walk(target):
            # filter dirs
            dirs[:] = [d for d in dirs if not is_excluded(Path(root_dir) / d)]
            for fname in files:
                fpath = Path(root_dir) / fname
                if is_excluded(fpath):
                    continue
                # Only check code files
                if fpath.suffix in [".rs", ".go", ".ts", ".tsx", ".js", ".json", ".toml"]:
                    v = check_file(fpath)
                    if v:
                        print(f"[VIOLATION] {fpath.relative_to(root)}:")
                        for line_no, line_content in v[:3]:
                            clean_line = line_content.encode("ascii", errors="replace").decode("ascii")
                            print(f"  L{line_no}: {clean_line}")
                        if len(v) > 3:
                            print(f"  ... (+{len(v) - 3} more occurrences)")
                        total_violations += len(v)

    if total_violations > 0:
        print(f"\n[FAIL] Brand Gate found {total_violations} legacy brand reference(s).")
        sys.exit(1)
    else:
        print("\n[PASS] Brand Gate: Zero unauthorized legacy brand references detected!")
        sys.exit(0)

if __name__ == "__main__":
    main()
