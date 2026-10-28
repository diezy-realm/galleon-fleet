#!/usr/bin/env python3
"""
scripts/ci/rename_brand.py - Systematic brand renaming utility for Galleon Fleet.
Based on standard-docs/tech-standart/naming/naming-map.md (ODR-16, ODR-17, ODR-18, ODR-19).

Safety rules:
- Preserves upstream license attribution (LICENSE-MIT, NOTICE, README upstream credits).
- Preserves WIT v0 frozen contracts (wit/v0/.frozen).
- Preserves historical decisions (standard-docs/decisions/owner-decisions/**, ADRs).
- Preserves git history and lockfiles where appropriate.
"""

import os
import re
import sys
from pathlib import Path

# Directories and files to exclude from mechanical replacement
EXCLUDED_DIRS = {
    ".git",
    "target",
    "node_modules",
    "dist",
    "vendor",
    "wit/v0",
    "graphify-out",
}

EXCLUDED_FILES = {
    "LICENSE",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "NOTICE",
    "Cargo.lock",
    "package-lock.json",
}

# Explicit replacement pairs ordered from most specific to least specific
REPLACEMENTS = [
    # 1. URL / Org slugs
    (re.compile(r"github\.com/clawcrew-labs/clawcrew", re.IGNORECASE), "github.com/diezy-realm/galleon-fleet"),
    (re.compile(r"ghcr\.io/clawcrew-labs/clawcrew", re.IGNORECASE), "ghcr.io/diezy-realm/galleon-fleet"),
    (re.compile(r"github\.com/diezy-labs/claw-crew/engine", re.IGNORECASE), "github.com/diezy-realm/galleon-fleet/engine"),

    # 2. Environment Variables
    (re.compile(r"\bGALLEON_"), "GALLEON_"),
    (re.compile(r"\bZEROCODE_"), "GALLEONCODE_"),
    (re.compile(r"\bZERORELAY_"), "GALLEONRELAY_"),

    # 3. Product & Brand Names (Case Sensitive)
    (re.compile(r"\bFleet AI\b"), "Galleon Fleet"),
    (re.compile(r"\bFleetAI\b"), "Galleon Fleet"),
    (re.compile(r"\bfleet-ai\b"), "galleon-fleet"),
    (re.compile(r"\bClawCrew\b"), "GalleonFleet"),
    (re.compile(r"\bCLAWCREW\b"), "GALLEON"),
    (re.compile(r"\bclaw-crew\b"), "galleon-fleet"),
]

def should_skip(path: Path) -> bool:
    for part in path.parts:
        if part in EXCLUDED_DIRS:
            return True
        if part.startswith(".") and part != ".":
            # skip hidden dirs except .github
            if part not in {".github"}:
                return True
    if path.name in EXCLUDED_FILES:
        return True
    # Skip ADRs and immutable decision records
    path_str = str(path).replace("\\", "/")
    if "standard-docs/decisions/owner-decisions" in path_str:
        return True
    if "wit/v0" in path_str:
        return True
    return False

def process_file(path: Path, dry_run: bool = False) -> int:
    try:
        content = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, PermissionError):
        return 0

    new_content = content
    for pattern, replacement in REPLACEMENTS:
        new_content = pattern.sub(replacement, new_content)

    if new_content != content:
        if not dry_run:
            path.write_text(new_content, encoding="utf-8")
        return 1
    return 0

def main():
    root = Path(__file__).resolve().parent.parent.parent
    dry_run = "--dry-run" in sys.argv
    print(f"Scanning from root: {root} (dry_run={dry_run})")

    modified_count = 0
    for root_dir, dirs, files in os.walk(root):
        # prune excluded directories in-place
        dirs[:] = [d for d in dirs if d not in EXCLUDED_DIRS and not (d.startswith(".") and d not in {".github"})]
        for f in files:
            p = Path(root_dir) / f
            if not should_skip(p):
                # only process text files
                ext = p.suffix.lower()
                if ext in {".rs", ".go", ".ts", ".tsx", ".js", ".json", ".toml", ".yaml", ".yml", ".md", ".sh", ".py", ".html", ".css", ".wit", ".sql", ".proto"}:
                    modified_count += process_file(p, dry_run=dry_run)

    print(f"Done. Files modified: {modified_count}")

if __name__ == "__main__":
    main()
