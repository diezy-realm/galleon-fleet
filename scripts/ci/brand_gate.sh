#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Ensure base ref is fetched if running in git worktree and base ref is not yet available locally
BASE="${GITHUB_BASE_REF:-${BASE_REF:-}}"
if [ -n "$BASE" ] && git -C "$ROOT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  if ! git -C "$ROOT_DIR" rev-parse --verify --quiet "$BASE" >/dev/null 2>&1 && \
     ! git -C "$ROOT_DIR" rev-parse --verify --quiet "origin/$BASE" >/dev/null 2>&1; then
    if git -C "$ROOT_DIR" remote get-url origin >/dev/null 2>&1; then
      git -C "$ROOT_DIR" fetch origin "${BASE}:refs/remotes/origin/${BASE}" --depth=100 2>/dev/null || \
      git -C "$ROOT_DIR" fetch origin "${BASE}" 2>/dev/null || true
    fi
  fi
fi

python3 "${SCRIPT_DIR}/brand_gate.py" "$@"
