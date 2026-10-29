#!/usr/bin/env bash
set -euo pipefail

# check_no_go.sh — Guard against introducing or retaining Go files.
#
# Modes:
#   diff: Fails if new Go files (*.go, go.mod, go.sum, go.work) are added compared to base (default: origin/testing).
#         Modifications to existing Go files are allowed (ratchet during migration).
#   full: Fails if any Go files (*.go, go.mod, go.sum, go.work) exist in the repository index.

usage() {
  echo "Usage: $0 <diff|full> [base_ref]" >&2
  exit 2
}

mode="${1:-}"
if [ -z "$mode" ]; then
  usage
fi

case "$mode" in
  diff)
    base="${2:-${CHECK_NO_GO_BASE:-origin/testing}}"

    # Resolve base if origin/testing doesn't exist but local testing does
    if ! git rev-parse --verify "$base" >/dev/null 2>&1; then
      if [ "$base" = "origin/testing" ] && git rev-parse --verify "testing" >/dev/null 2>&1; then
        base="testing"
      else
        echo "ERROR: Base reference '$base' not found." >&2
        exit 2
      fi
    fi

    added_files=$(git diff --name-only --diff-filter=A "${base}...HEAD" 2>/dev/null || true)
    if [ -n "$added_files" ]; then
      forbidden=$(printf '%s\n' "$added_files" | grep -E '(\.go$|(^|/)(go\.mod|go\.sum|go\.work)$)' || true)
      if [ -n "$forbidden" ]; then
        echo "ERROR: New Go files or Go module files are not allowed:" >&2
        printf '%s\n' "$forbidden" | sed 's/^/  /' >&2
        exit 1
      fi
    fi
    echo "check_no_go: diff mode passed (no new Go files added against ${base})."
    ;;

  full)
    files=$(git ls-files '*.go' 'go.mod' 'go.sum' 'go.work' '*go.mod' '*go.sum' '*go.work' 2>/dev/null || true)
    if [ -n "$files" ]; then
      echo "ERROR: Go files or Go module files found in repository:" >&2
      printf '%s\n' "$files" | sort -u | sed 's/^/  /' >&2
      exit 1
    fi
    echo "check_no_go: full mode passed (0 Go files in repository)."
    ;;

  *)
    echo "ERROR: Unknown mode '$mode'" >&2
    usage
    ;;
esac
