#!/usr/bin/env python3
"""
Galleon Fleet — Upstream ZeroClaw License & Legal Attribution Audit Gate (SPEC-009 / RB-29 / RB-30)
==================================================================================================
Verifies 100% preservation of upstream ZeroClaw legal copyright, attribution,
license notices, and immutable decisions compliance.
"""

import sys
import subprocess
from pathlib import Path

REQUIRED_LICENSE_MIT_SNIPPETS = [
    "MIT License",
    "Copyright (c) 2025 ZeroClaw Labs",
    "Copyright (c) 2026 Galleon Fleet contributors",
    "Permission is hereby granted, free of charge",
    "The above copyright notice and this permission notice shall be included in all",
]

REQUIRED_NOTICE_SNIPPETS = [
    "Galleon Fleet",
    "Copyright 2026 Galleon Fleet contributors",
    "Origin and Attribution",
    "Galleon Fleet is a derivative work of ZeroClaw, created by ZeroClaw Labs and",
    "https://github.com/zeroclaw-labs/zeroclaw",
    "Copyright 2025 ZeroClaw Labs",
    "MIT OR Apache-2.0",
    "ZeroClaw name and logo are trademarks of ZeroClaw Labs",
    "Upstream contributors retain copyright ownership",
    "https://github.com/zeroclaw-labs/zeroclaw/graphs/contributors",
]

REQUIRED_README_LEGAL_SNIPPETS = [
    "Dual-licensed: [MIT](LICENSE-MIT) OR [Apache 2.0](LICENSE-APACHE)",
    "Both license files retain the upstream copyright notice of ZeroClaw Labs; see [NOTICE](NOTICE) for the full attribution.",
    "The **ZeroClaw** name and logo are trademarks of ZeroClaw Labs.",
    "This project is a derivative of [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) by ZeroClaw Labs",
    "@theonlyhennygod",
    "@JordanTheJet",
]

def audit_file_content(path: Path, required_snippets: list[str]) -> list[str]:
    errors = []
    if not path.exists():
        return [f"Required file does not exist: {path}"]
    content = path.read_text(encoding="utf-8")
    for snippet in required_snippets:
        if snippet not in content:
            errors.append(f"Missing mandatory legal snippet in {path.name}: {snippet!r}")
    return errors

def audit_decisions_immutability(repo_root: Path) -> list[str]:
    errors = []
    decisions_dir = repo_root / "standard-docs" / "decisions"
    if not decisions_dir.exists():
        return [f"Decisions directory does not exist: {decisions_dir}"]
    
    # Check git diff for any modifications in decisions directory
    res = subprocess.run(
        ["git", "status", "--porcelain", "--", "standard-docs/decisions/"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    if res.stdout.strip():
        errors.append(f"Immutable decisions directory modified:\n{res.stdout.strip()}")
    return errors

def audit_cargo_license(repo_root: Path) -> list[str]:
    errors = []
    cargo_toml = repo_root / "Cargo.toml"
    if not cargo_toml.exists():
        return [f"Cargo.toml not found: {cargo_toml}"]
    content = cargo_toml.read_text(encoding="utf-8")
    if 'license = "MIT OR Apache-2.0"' not in content:
        errors.append("Root Cargo.toml missing [workspace.package] license = \"MIT OR Apache-2.0\"")
    return errors

def main():
    repo_root = Path(__file__).resolve().parent.parent.parent
    print(f"[*] Auditing Upstream ZeroClaw License Compliance & Legal Invariants in {repo_root}...")

    all_errors = []

    # 1. LICENSE-MIT
    errs = audit_file_content(repo_root / "LICENSE-MIT", REQUIRED_LICENSE_MIT_SNIPPETS)
    if errs:
        all_errors.extend(errs)
    else:
        print("  [PASS] LICENSE-MIT: 100% upstream ZeroClaw copyright & MIT terms intact")

    # 2. NOTICE
    errs = audit_file_content(repo_root / "NOTICE", REQUIRED_NOTICE_SNIPPETS)
    if errs:
        all_errors.extend(errs)
    else:
        print("  [PASS] NOTICE: 100% upstream ZeroClaw origin and attribution intact")

    # 3. README.md
    errs = audit_file_content(repo_root / "README.md", REQUIRED_README_LEGAL_SNIPPETS)
    if errs:
        all_errors.extend(errs)
    else:
        print("  [PASS] README.md: Upstream ZeroClaw license, trademark, and credits intact")

    # 4. Cargo workspace license
    errs = audit_cargo_license(repo_root)
    if errs:
        all_errors.extend(errs)
    else:
        print("  [PASS] Cargo.toml: Workspace dual-license expression verified")

    # 5. Decisions Immutability
    errs = audit_decisions_immutability(repo_root)
    if errs:
        all_errors.extend(errs)
    else:
        print("  [PASS] standard-docs/decisions/: Zero modifications detected (immutable invariant preserved)")

    if all_errors:
        print(f"\n[FAIL] License compliance audit failed with {len(all_errors)} violation(s):")
        for err in all_errors:
            print(f"  - {err}")
        sys.exit(1)
    else:
        print("\n[SUCCESS] All legal, license, and attribution checks passed with 100% compliance!")
        sys.exit(0)

if __name__ == "__main__":
    main()
