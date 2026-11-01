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
- Specific runtime compatibility names & fallback aliases:
  - src/main.rs, src/main_tests.rs (legacy binary discovery / desktop launcher)
  - dist/aur/PKGBUILD (backward-compat symlink)
  - apps/galleoncode & apps/galleonrelay (clawcrew.relay.v1 wire protocol)
- Pre-existing legacy brand references tracked in base ref (baseline)
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
]

EXCLUDED_EXTENSIONS = [
    ".png", ".jpg", ".jpeg", ".ico", ".svg", ".lock", ".rlib", ".rmeta", ".exe", ".bin", ".tar", ".gz"
]

_BASE_FILE_CACHE: dict[tuple[str, str], set[str]] = {}

def is_excluded_path(file_path_str: str) -> bool:
    posix_path = file_path_str.replace("\\", "/").strip("/")
    parts = posix_path.split("/")
    for excl in EXCLUDED_PATHS:
        excl_clean = excl.strip("/")
        # Exact component match (e.g. '.git', 'target', 'node_modules')
        if excl_clean in parts:
            return True
        # Exact path or directory prefix match (e.g. 'standard-docs/...')
        if posix_path == excl_clean or posix_path.startswith(excl_clean + "/"):
            return True
    if any(posix_path.endswith(ext) for ext in EXCLUDED_EXTENSIONS):
        return True
    return False

def is_permitted_line(file_path_str: str, line: str) -> bool:
    line_lower = line.lower()
    # Skip comments attributing upstream authors, frozen v0 WIT, or compatibility markers
    if "clawcrew:plugin@0.1.0" in line:
        return True
    if "upstream" in line_lower or "mit license" in line_lower:
        return True
    if any(marker in line_lower for marker in ("brand-compat", "compat:", "backward-compat", "ratchet(")):
        return True

    posix_path = file_path_str.replace("\\", "/")

    # Specific backward compatibility wire protocols
    if "clawcrew.relay.v1" in line:
        return True

    # AUR package build symlink for backward compatibility (dist/aur/PKGBUILD)
    if posix_path.endswith("dist/aur/PKGBUILD") and "ln -sf" in line and "clawcrew" in line:
        return True

    # Standalone demo container backward compatibility (SPEC-016)
    if posix_path.startswith("demo/"):
        if any(pat in line for pat in (
            "/tmp/clawcrew",
            "/usr/local/bin/clawcrew",
            "run-clawcrew.sh",
            "clawcrew daemon",
            "clawcrew agent",
            "clawcrew.toml",
            "clawcrew-demo",
        )):
            return True

    # Runtime backward compatibility in main binary / launcher (src/main.rs, src/main_tests.rs)
    if posix_path.endswith("src/main.rs") or posix_path.endswith("src/main_tests.rs"):
        # Match legacy desktop binary discovery and compatibility helpers without literal forbidden substrings in script
        if ("clawcrew" in line_lower and "desktop" in line_lower) or any(pat in line for pat in (
            "is_clawcrew_program",
            "is_clawcrew_appimage_name",
            "is_clawcrew_name",
            "clawcrew_desktop_exec",
            "parse_exec_program",
            "which::which(\"clawcrew\")",
            'which("clawcrew")',
            "find_clawcrew_binary",
            "is_clawcrew_binary",
            'exe.with_file_name("clawcrew")',
        )):
            return True

    return False

def get_base_file_lines(root: Path, base_ref: str, file_path_str: str) -> set[str]:
    posix_path = file_path_str.replace("\\", "/")
    key = (base_ref, posix_path)
    if key in _BASE_FILE_CACHE:
        return _BASE_FILE_CACHE[key]
    lines_set = set()
    try:
        res = subprocess.run(
            ["git", "show", f"{base_ref}:{posix_path}"],
            cwd=root,
            capture_output=True,
            text=True,
        )
        if res.returncode == 0:
            for l in res.stdout.splitlines():
                stripped = l.strip()
                if stripped:
                    lines_set.add(stripped)
    except Exception:
        pass
    _BASE_FILE_CACHE[key] = lines_set
    return lines_set

def is_in_baseline(
    root: Path,
    base_ref: str | None,
    file_path_str: str,
    content: str,
    baseline_set: set[str] | None = None,
) -> bool:
    stripped = content.strip()
    if not stripped:
        return True

    posix_path = file_path_str.replace("\\", "/")

    # Check static baseline set if provided
    if baseline_set is not None:
        if f"{posix_path}:{stripped}" in baseline_set:
            return True
        if stripped in baseline_set:
            return True

    if not base_ref:
        return False

    # Check if the line existed in base_ref for this file
    base_lines = get_base_file_lines(root, base_ref, posix_path)
    if stripped in base_lines:
        return True

    # Fallback: check if the exact line existed anywhere in base_ref (e.g. moved line)
    try:
        grep_proc = subprocess.run(
            ["git", "grep", "-F", "-q", stripped, base_ref],
            cwd=root,
            capture_output=True,
        )
        if grep_proc.returncode == 0:
            return True
    except Exception:
        pass

    return False

def load_baseline(root: Path, baseline_arg: str | None) -> set[str] | None:
    path = None
    if baseline_arg:
        path = Path(baseline_arg)
        if not path.is_absolute():
            path = root / path
    else:
        default_baseline = root / "scripts" / "ci" / "brand_gate_baseline.txt"
        if default_baseline.exists():
            path = default_baseline

    if path and path.exists():
        entries = set()
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            for line in f:
                s = line.strip()
                if s and not s.startswith("#"):
                    entries.add(s)
        return entries
    return None

def check_file(rel_path: str, file_path: Path) -> list:
    violations = []
    try:
        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
            lines = f.readlines()
            for idx, line in enumerate(lines, start=1):
                prev_line = lines[idx - 2] if idx > 1 else ""
                prev_guarded = any(m in prev_line.lower() for m in ("brand-compat", "compat:", "backward-compat", "ratchet("))
                if is_permitted_line(rel_path, line) or prev_guarded:
                    continue
                for pat in BANNED_PATTERNS:
                    if pat.search(line):
                        violations.append((idx, line.strip()))
                        break
    except Exception:
        pass
    return violations

def run_diff_mode(root: Path, base_ref: str, head_ref: str | None = None, baseline_set: set[str] | None = None) -> list:
    head = head_ref if head_ref and head_ref != "HEAD" else "HEAD"
    mb_res = subprocess.run(["git", "merge-base", base_ref, head], cwd=root, capture_output=True, text=True)
    proc = None
    if mb_res.returncode == 0 and mb_res.stdout.strip():
        mb = mb_res.stdout.strip()
        if head_ref and head_ref != "HEAD":
            cmd = ["git", "diff", "-U0", mb, head]
        else:
            cmd = ["git", "diff", "-U0", mb]
        proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)

    if proc is None or proc.returncode != 0:
        # Fallback to three-dot or two-dot diff
        cmd = ["git", "diff", "-U0", f"{base_ref}...{head}"]
        proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
        if proc.returncode != 0:
            cmd = ["git", "diff", "-U0", f"{base_ref}..{head}"]
            proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
            if proc.returncode != 0 and head == "HEAD":
                cmd = ["git", "diff", "-U0", base_ref]
                proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)

    if proc.returncode != 0:
        print(f"[FAIL] git diff failed against base ref '{base_ref}': {proc.stderr.strip()}", file=sys.stderr)
        sys.exit(1)

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
                    if any(m in prev_content for m in ("brand-compat", "compat:", "backward-compat", "ratchet(")):
                        prev_guarded = True
                if not is_permitted_line(current_file, content) and not prev_guarded:
                    for pat in BANNED_PATTERNS:
                        if pat.search(content):
                            if is_in_baseline(root, base_ref, current_file, content, baseline_set):
                                continue
                            violations.append((current_file, line_no, content.strip()))
                            break
            line_no += 1

    # Also inspect untracked files if inspecting local working tree
    if head == "HEAD":
        untracked_proc = subprocess.run(
            ["git", "ls-files", "--others", "--exclude-standard"],
            cwd=root,
            capture_output=True,
            text=True,
        )
        if untracked_proc.returncode == 0:
            for rel_file in untracked_proc.stdout.splitlines():
                rel_file = rel_file.strip()
                if rel_file and not is_excluded_path(rel_file):
                    file_path = root / rel_file
                    if file_path.is_file():
                        file_violations = check_file(rel_file, file_path)
                        for lno, text in file_violations:
                            if not is_in_baseline(root, base_ref, rel_file, text, baseline_set):
                                violations.append((rel_file, lno, text))

    return violations

def run_full_scan(root: Path, base_ref: str | None = None, baseline_set: set[str] | None = None) -> list:
    violations = []
    scan_targets = [root / "crates", root / "web", root / "apps", root / "src", root / ".github"]
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
                    file_violations = check_file(rel_path, fpath)
                    for lno, text in file_violations:
                        if is_in_baseline(root, base_ref, rel_path, text, baseline_set):
                            continue
                        violations.append((rel_path, lno, text))
    return violations

def verify_git_ref(root: Path, ref: str) -> bool:
    res = subprocess.run(["git", "rev-parse", "--verify", "--quiet", ref], cwd=root, capture_output=True)
    return res.returncode == 0

def resolve_base_ref(root: Path, explicit_base: str | None) -> str | None:
    if explicit_base:
        candidates = [explicit_base]
        if not explicit_base.startswith("origin/"):
            candidates.append(f"origin/{explicit_base}")
        for candidate in candidates:
            if verify_git_ref(root, candidate):
                return candidate
        return None

    env_base = os.environ.get("BASE_REF") or os.environ.get("GITHUB_BASE_REF")
    if env_base:
        candidates = [env_base]
        if not env_base.startswith("origin/"):
            candidates.append(f"origin/{env_base}")
        for candidate in candidates:
            if verify_git_ref(root, candidate):
                return candidate
        # If candidate ref is not resolved locally, attempt fetch from remote if origin exists
        ref_to_fetch = env_base.replace("origin/", "")
        try:
            subprocess.run(
                ["git", "fetch", "origin", f"{ref_to_fetch}:refs/remotes/origin/{ref_to_fetch}", "--depth=100"],
                cwd=root,
                capture_output=True,
                timeout=15,
            )
            if verify_git_ref(root, f"origin/{ref_to_fetch}"):
                return f"origin/{ref_to_fetch}"
            if verify_git_ref(root, ref_to_fetch):
                return ref_to_fetch
        except Exception:
            pass
        return None

    # Try candidate refs
    for candidate in ["origin/testing", "testing", "origin/master", "master", "HEAD~1"]:
        if verify_git_ref(root, candidate):
            return candidate
    return None

def main():
    parser = argparse.ArgumentParser(description="Galleon Fleet Brand Compliance Gate")
    parser.add_argument("--mode", choices=["auto", "diff", "full"], default="auto", help="Scan mode")
    parser.add_argument("--base", default=None, help="Base git ref for diff mode")
    parser.add_argument("--head", default="HEAD", help="Head git ref for diff mode")
    parser.add_argument("--baseline", default=None, help="Path to baseline file containing legacy brand references")
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    print(f"[*] Running Galleon Brand Compliance Gate on {root}...")

    mode = args.mode
    base_ref = resolve_base_ref(root, args.base)

    if args.base and not base_ref:
        print(f"[FAIL] Explicit base ref '{args.base}' could not be resolved.", file=sys.stderr)
        sys.exit(1)

    env_base = os.environ.get("BASE_REF") or os.environ.get("GITHUB_BASE_REF")
    if env_base and not base_ref:
        env_key = "BASE_REF" if "BASE_REF" in os.environ else "GITHUB_BASE_REF"
        print(f"[FAIL] Environment base ref {env_key}='{env_base}' could not be resolved.", file=sys.stderr)
        sys.exit(1)

    if mode == "diff" and not base_ref:
        print(f"[FAIL] Unable to resolve base ref for diff mode.", file=sys.stderr)
        sys.exit(1)

    if mode == "auto":
        # Prefer diff mode when base ref is resolvable in git repository
        if base_ref and (root / ".git").exists():
            mode = "diff"
        else:
            mode = "full"

    baseline_set = load_baseline(root, args.baseline)

    violations = []
    if mode == "diff":
        print(f"[*] Checking diff against base ref: {base_ref}...")
        diff_violations = run_diff_mode(root, base_ref, args.head, baseline_set)
        violations.extend(diff_violations)
    else:
        print("[*] Running full directory scan...")
        scan_violations = run_full_scan(root, base_ref, baseline_set)
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
