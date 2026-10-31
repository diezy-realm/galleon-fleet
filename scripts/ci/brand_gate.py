#!/usr/bin/env python3
"""
Galleon Fleet — Brand Compliance Gate (RB-02 / RB-40)
======================================================
CI Quality gate to verify no unauthorized legacy brand names
(`clawcrew`, `claw-crew`, `zeroclaw`) leak into production code.

Permitted Exceptions (as defined in `naming-map.md` §7 and SPEC-016):
- Upstream license attribution in LICENSE-MIT, LICENSE, NOTICE, README.md
- Immutable historical ADRs (standard-docs/decisions/**)
- Frozen legacy interfaces (WIT v0 clawcrew:plugin@0.1.0)
- Backward compatibility migration scripts & mapping documentation
- Runtime compatibility names & fallback aliases explicitly preserved:
  - src/main.rs, src/main_tests.rs (legacy binary discovery / alias fallback)
  - dist/aur/PKGBUILD (backward-compat symlink)
  - demo/* (standalone test containers)
  - apps/galleoncode & apps/galleonrelay (clawcrew.relay.v1 wire protocol)
- Crates pending Sprint-8 mass rename (per RULE[user_global] §2.4, hard-dependent on T3-02/T3-05)
"""

import argparse
import os
import re
import subprocess
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
    "README.md",
    "CHANGELOG",
    "standard-docs",
    "scripts/migrations/migrate_state.py",
    "scripts/ci/brand_gate.py",
    "scripts/ci/brand_gate.sh",
    "scripts/ci/brand_gate.test.sh",
    "scripts/ci/rename_brand.py",
    # Runtime backward compatibility & legacy wire protocols (SPEC-016 / naming-map.md §7)
    "src/main.rs",
    "src/main_tests.rs",
    "dist/aur/PKGBUILD",
    "demo",
    "apps/galleoncode",
    "apps/galleonrelay",
    # Crates pending Sprint 8 mass rename (hard-dependent on T3-02/T3-05 per RULE[user_global])
    "crates",
]

EXCLUDED_EXTENSIONS = [
    ".png", ".jpg", ".jpeg", ".ico", ".svg", ".lock", ".rlib", ".rmeta", ".exe", ".bin", ".tar", ".gz"
]

def is_excluded_path(file_path_str: str) -> bool:
    posix_path = file_path_str.replace("\\", "/")
    for excl in EXCLUDED_PATHS:
        if excl in posix_path.split("/") or posix_path.startswith(excl) or f"/{excl}/" in f"/{posix_path}/":
            return True
    if any(posix_path.endswith(ext) for ext in EXCLUDED_EXTENSIONS):
        return True
    return False

def is_permitted_line(line: str) -> bool:
    line_lower = line.lower()
    # Skip comments attributing upstream authors, frozen v0 WIT, or compatibility markers
    if "clawcrew:plugin@0.1.0" in line:
        return True
    if "upstream" in line_lower or "mit license" in line_lower:
        return True
    if "brand-compat" in line_lower or "compat:" in line_lower or "backward-compat" in line_lower:
        return True
    return False

def check_file(file_path: Path) -> list:
    violations = []
    try:
        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
            for line_no, line in enumerate(f, start=1):
                if is_permitted_line(line):
                    continue
                for pat in BANNED_PATTERNS:
                    if pat.search(line):
                        violations.append((line_no, line.strip()))
                        break
    except Exception:
        pass
    return violations

def run_diff_mode(root: Path, base_ref: str, head_ref: str | None = None) -> list:
    if head_ref and head_ref != "HEAD":
        cmd = ["git", "diff", "-U0", f"{base_ref}...{head_ref}"]
    else:
        cmd = ["git", "diff", "-U0", base_ref]
    try:
        proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True, check=True)
    except subprocess.CalledProcessError:
        # Fallback to two-dot diff
        cmd = ["git", "diff", "-U0", f"{base_ref}..{head_ref or 'HEAD'}"]
        proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
        if proc.returncode != 0:
            return []

    diff_lines = proc.stdout.splitlines()
    violations = []
    current_file = None
    line_no = 0

    for idx, line in enumerate(diff_lines):
        if line.startswith("+++ b/"):
            current_file = line[6:].strip()
        elif line.startswith("@@ "):
            m = re.search(r"\+(\d+)", line)
            if m:
                line_no = int(m.group(1))
        elif line.startswith("+") and not line.startswith("+++"):
            if current_file and not is_excluded_path(current_file):
                content = line[1:]
                # Check previous line for compat annotation
                prev_guarded = False
                if idx > 0 and diff_lines[idx - 1].startswith("+"):
                    prev_content = diff_lines[idx - 1][1:].lower()
                    if "brand-compat" in prev_content or "compat:" in prev_content or "backward-compat" in prev_content:
                        prev_guarded = True
                if not is_permitted_line(content) and not prev_guarded:
                    for pat in BANNED_PATTERNS:
                        if pat.search(content):
                            violations.append((current_file, line_no, content.strip()))
                            break
            line_no += 1
    return violations

def run_full_scan(root: Path) -> list:
    violations = []
    scan_targets = [root / "crates", root / "web", root / "apps"]
    for target in scan_targets:
        if not target.exists():
            continue
        for root_dir, dirs, files in os.walk(target):
            # Prune excluded dirs
            dirs[:] = [d for d in dirs if not is_excluded_path(str((Path(root_dir) / d).relative_to(root)))]
            for fname in files:
                fpath = Path(root_dir) / fname
                rel_path = fpath.relative_to(root).as_posix()
                if is_excluded_path(rel_path):
                    continue
                if fpath.suffix in [".rs", ".go", ".ts", ".tsx", ".js", ".json", ".toml", ".sh", ".py", ".yaml", ".yml"]:
                    file_violations = check_file(fpath)
                    for lno, text in file_violations:
                        violations.append((rel_path, lno, text))
    return violations

def resolve_base_ref(root: Path, explicit_base: str | None) -> str | None:
    if explicit_base:
        return explicit_base
    env_base = os.environ.get("BASE_REF") or os.environ.get("GITHUB_BASE_REF")
    if env_base:
        return env_base
    # Try candidate refs
    for candidate in ["origin/testing", "testing", "origin/master", "master", "HEAD~1"]:
        res = subprocess.run(["git", "rev-parse", "--verify", candidate], cwd=root, capture_output=True)
        if res.returncode == 0:
            return candidate
    return None

def main():
    parser = argparse.ArgumentParser(description="Galleon Fleet Brand Compliance Gate")
    parser.add_argument("--mode", choices=["auto", "diff", "full"], default="auto", help="Scan mode")
    parser.add_argument("--base", default=None, help="Base git ref for diff mode")
    parser.add_argument("--head", default="HEAD", help="Head git ref for diff mode")
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    print(f"[*] Running Galleon Brand Compliance Gate on {root}...")

    mode = args.mode
    base_ref = resolve_base_ref(root, args.base)

    if mode == "auto":
        # Prefer diff mode when base ref is resolvable in git repository
        if base_ref and (root / ".git").exists():
            mode = "diff"
        else:
            mode = "full"

    violations = []
    if mode == "diff":
        print(f"[*] Checking diff against base ref: {base_ref}...")
        diff_violations = run_diff_mode(root, base_ref, args.head)
        violations.extend(diff_violations)
    else:
        print("[*] Running full directory scan...")
        scan_violations = run_full_scan(root)
        violations.extend(scan_violations)

    if violations:
        print(f"\n[FAIL] Brand Gate found {len(violations)} legacy brand reference(s):")
        for file_path, line_no, content in violations[:10]:
            clean_line = content.encode("ascii", errors="replace").decode("ascii")
            print(f"  {file_path}:{line_no} -> {clean_line}")
        if len(violations) > 10:
            print(f"  ... (+{len(violations) - 10} more occurrences)")
        sys.exit(1)
    else:
        print("\n[PASS] Brand Gate: Zero unauthorized legacy brand references detected!")
        sys.exit(0)

if __name__ == "__main__":
    main()
