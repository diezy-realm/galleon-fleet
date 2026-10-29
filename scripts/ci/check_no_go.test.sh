#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
runner="${script_dir}/check_no_go.sh"
root_dir="$(cd "${script_dir}/../.." && pwd)"
workflow="${root_dir}/.github/workflows/ci.yml"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# 1. Test argument handling
set +e
"$runner" >/dev/null 2>&1
status=$?
set -e
if [ "$status" -ne 2 ]; then
  fail "expected exit 2 when no mode is provided, got $status"
fi

set +e
"$runner" invalid_mode >/dev/null 2>&1
status=$?
set -e
if [ "$status" -ne 2 ]; then
  fail "expected exit 2 for invalid mode, got $status"
fi

# 2. Setup temporary git repository
mock_dir="$(mktemp -d)"
trap 'rm -rf "$mock_dir"' EXIT

repo="${mock_dir}/repo"
git init --quiet --initial-branch=testing "$repo"
git -C "$repo" config user.name "Galleon Test"
git -C "$repo" config user.email "test@galleon.invalid"

# Create base state with an existing Go file and a Rust file
mkdir -p "${repo}/engine" "${repo}/src"
cat > "${repo}/engine/legacy.go" <<'EOF'
package engine
func Legacy() {}
EOF
cat > "${repo}/go.work" <<'EOF'
go 1.22
EOF
cat > "${repo}/src/main.rs" <<'EOF'
fn main() {}
EOF

git -C "$repo" add engine/legacy.go go.work src/main.rs
git -C "$repo" commit --quiet -m "base commit on testing"
base_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" update-ref refs/remotes/origin/testing "$base_sha"

# 3. Test diff mode with no changes
(cd "$repo" && "$runner" diff origin/testing) >/dev/null || \
  fail "diff mode should pass when there are no changes"

# 4. Test diff mode: modifying existing Go file should pass (ratchet)
git -C "$repo" checkout --quiet -b feat/modify-existing
cat >> "${repo}/engine/legacy.go" <<'EOF'
// safe edit to existing Go code
EOF
git -C "$repo" commit --quiet -am "modify legacy.go"
(cd "$repo" && "$runner" diff origin/testing) >/dev/null || \
  fail "diff mode should pass when existing Go file is modified"

# 5. Test diff mode: adding a new Rust file should pass
cat > "${repo}/src/lib.rs" <<'EOF'
pub fn new_feature() {}
EOF
git -C "$repo" add src/lib.rs
git -C "$repo" commit --quiet -m "add new rust file"
(cd "$repo" && "$runner" diff origin/testing) >/dev/null || \
  fail "diff mode should pass when new Rust file is added"

# 6. Test diff mode: adding a new .go file must fail
git -C "$repo" checkout --quiet -b feat/add-new-go origin/testing
cat > "${repo}/src/new.go" <<'EOF'
package main
EOF
git -C "$repo" add src/new.go
git -C "$repo" commit --quiet -m "add new go file"

set +e
output="$(cd "$repo" && "$runner" diff origin/testing 2>&1)"
status=$?
set -e

if [ "$status" -ne 1 ]; then
  fail "diff mode must fail with exit 1 when a new .go file is added, got $status"
fi
if ! printf '%s\n' "$output" | grep -q "src/new.go"; then
  fail "diff mode output should mention src/new.go, got: $output"
fi

# 7. Test diff mode: adding a nested .go file must fail
git -C "$repo" checkout --quiet -b feat/add-nested-go origin/testing
mkdir -p "${repo}/pkg/nested"
cat > "${repo}/pkg/nested/service.go" <<'EOF'
package nested
EOF
git -C "$repo" add pkg/nested/service.go
git -C "$repo" commit --quiet -m "add nested go file"

set +e
status=0
(cd "$repo" && "$runner" diff origin/testing) >/dev/null 2>&1 || status=$?
set -e
if [ "$status" -ne 1 ]; then
  fail "diff mode must fail when nested .go file is added, got $status"
fi

# 8. Test diff mode: adding go.mod / go.sum / go.work must fail
for mod_file in "go.mod" "engine/go.mod" "go.sum" "sub/go.work"; do
  git -C "$repo" checkout --quiet -b "feat/add-${mod_file//\//-}" origin/testing
  mkdir -p "${repo}/$(dirname "$mod_file")"
  echo "test" > "${repo}/${mod_file}"
  git -C "$repo" add "$mod_file"
  git -C "$repo" commit --quiet -m "add ${mod_file}"

  set +e
  status=0
  (cd "$repo" && "$runner" diff origin/testing) >/dev/null 2>&1 || status=$?
  set -e
  if [ "$status" -ne 1 ]; then
    fail "diff mode must fail when ${mod_file} is added, got $status"
  fi
done

# 9. Test full mode when Go files exist -> must fail
set +e
output="$(cd "$repo" && "$runner" full 2>&1)"
status=$?
set -e
if [ "$status" -ne 1 ]; then
  fail "full mode must fail when Go files exist, got $status"
fi

# 10. Test full mode when all Go files are removed -> must pass
git -C "$repo" checkout --quiet -b feat/clean-go origin/testing
git -C "$repo" rm --quiet engine/legacy.go go.work
git -C "$repo" commit --quiet -m "remove all go files"
(cd "$repo" && "$runner" full) >/dev/null || \
  fail "full mode should pass when repository has zero Go files"

# 11. Assert workflow ci.yml wiring
if [ -f "$workflow" ]; then
  grep -Fq 'bash scripts/ci/check_no_go.sh diff' "$workflow" || \
    fail "ci.yml must contain 'bash scripts/ci/check_no_go.sh diff'"
fi

echo "check_no_go tests passed"
