#!/usr/bin/env python3
"""Debug script for unwrap check - print relevant diff lines."""
import subprocess
import re

base_ref = "origin/testing"
head_ref = "HEAD"

cmd = [
    "git", "diff", "-U0", f"{base_ref}...{head_ref}",
    "--", "crates/**/*.rs",
    ":!crates/**/tests/**",
    ":!crates/**/benches/**",
    ":!crates/**/tests.rs",
    ":!crates/**/*/tests.rs"
]

proc = subprocess.run(cmd, capture_output=True, encoding="utf-8", errors="replace", check=True)
diff_lines = proc.stdout.splitlines()

current_file = None
unwrap_regex = re.compile(r"\.(unwrap|expect)\(")

for idx, line in enumerate(diff_lines):
    if line.startswith("+++ b/"):
        current_file = line[6:].strip()
    elif line.startswith("@@ "):
        m = re.search(r"\+(\d+)", line)
        if m:
            hunk_new_line = int(m.group(1))
    elif line.startswith("+") and not line.startswith("+++"):
        if current_file and unwrap_regex.search(line):
            print(f"File: {current_file}")
            print(f"Line: {line}")
            # Show previous 3 lines
            print("Previous lines:")
            for i in range(max(0, idx - 3), idx):
                print(f"  {diff_lines[i]}")
            print()
