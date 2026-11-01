#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
runner="${script_dir}/brand_gate.sh"
root_dir="$(cd "${script_dir}/../.." && pwd)"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# 1. Setup mock git repo
mock_dir="$(mktemp -d)"
trap 'rm -rf "$mock_dir"' EXIT

repo="${mock_dir}/repo"
git init --quiet --initial-branch=testing "$repo"
git -C "$repo" config user.name "Galleon Test"
git -C "$repo" config user.email "test@galleon.invalid"

# Copy brand_gate scripts into mock repo
mkdir -p "${repo}/scripts/ci" "${repo}/src"
cp "${script_dir}/brand_gate.py" "${repo}/scripts/ci/brand_gate.py"
cp "${script_dir}/brand_gate.sh" "${repo}/scripts/ci/brand_gate.sh"
chmod +x "${repo}/scripts/ci/brand_gate.sh"

cat > "${repo}/src/lib.rs" <<'EOF'
pub fn galleon_feature() -> &'static str {
    "galleon"
}
EOF

git -C "$repo" add scripts/ci src/lib.rs
git -C "$repo" commit --quiet -m "initial commit on testing"
base_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" update-ref refs/remotes/origin/testing "$base_sha"

# 2. Test clean diff passes
(cd "$repo" && bash scripts/ci/brand_gate.sh) >/dev/null || \
  fail "brand_gate should pass on clean diff"

# 3. Test adding clean new code passes
cat >> "${repo}/src/lib.rs" <<'EOF'
pub fn another_galleon_func() {}
EOF
(cd "$repo" && bash scripts/ci/brand_gate.sh) >/dev/null || \
  fail "brand_gate should pass when clean Galleon code is added"

# 4. Test introducing unauthorized brand name fails
cat >> "${repo}/src/lib.rs" <<'EOF'
pub const BAD: &str = "clawcrew";
EOF

set +e
output="$(cd "$repo" && bash scripts/ci/brand_gate.sh 2>&1)"
status=$?
set -e

if [ "$status" -ne 1 ]; then
  fail "brand_gate must fail with exit 1 when unauthorized brand name is added, got $status"
fi

# 5. Test permitted brand-compat comment passes
cat > "${repo}/src/lib.rs" <<'EOF'
// BRAND-COMPAT: legacy binary support
pub const OLD: &str = "clawcrew";
EOF
(cd "$repo" && bash scripts/ci/brand_gate.sh) >/dev/null || \
  fail "brand_gate should pass when line contains BRAND-COMPAT marker"

# Revert working tree for subsequent contract tests
git -C "$repo" checkout -- src/lib.rs

# 6. Contract Test: PR checkout environment where local branch doesn't exist
# In GitHub Actions PR checkout, only refs/remotes/origin/testing exists and GITHUB_BASE_REF="testing"
git -C "$repo" checkout -b feat/contract-pr --quiet
git -C "$repo" branch -D testing --quiet

# Verify local testing branch does not exist, but origin/testing exists
if git -C "$repo" rev-parse --verify --quiet testing >/dev/null 2>&1; then
  fail "local testing branch should not exist in PR checkout simulation"
fi
if ! git -C "$repo" rev-parse --verify --quiet origin/testing >/dev/null 2>&1; then
  fail "origin/testing must exist in PR checkout simulation"
fi

# 6a. Clean PR checkout passes
(cd "$repo" && GITHUB_BASE_REF="testing" bash scripts/ci/brand_gate.sh) >/dev/null || \
  fail "brand_gate must pass in PR checkout environment resolving GITHUB_BASE_REF to origin/testing"

# 6b. Unauthorized brand name in PR checkout fails
cat >> "${repo}/src/lib.rs" <<'EOF'
pub const PR_BAD: &str = "clawcrew";
EOF

set +e
output="$(cd "$repo" && GITHUB_BASE_REF="testing" bash scripts/ci/brand_gate.sh 2>&1)"
status=$?
set -e

if [ "$status" -ne 1 ]; then
  fail "brand_gate must fail with exit 1 on unauthorized brand name in PR checkout environment, got $status"
fi
git -C "$repo" checkout -- src/lib.rs

# 7. Contract Test: Unavailable / unresolvable base ref must fail with exit nonzero
set +e
output="$(cd "$repo" && GITHUB_BASE_REF="nonexistent-target" bash scripts/ci/brand_gate.sh 2>&1)"
status=$?
set -e
if [ "$status" -eq 0 ]; then
  fail "brand_gate must exit nonzero when GITHUB_BASE_REF cannot be resolved"
fi

set +e
output="$(cd "$repo" && bash scripts/ci/brand_gate.sh --base nonexistent-target 2>&1)"
status=$?
set -e
if [ "$status" -eq 0 ]; then
  fail "brand_gate must exit nonzero when explicit --base cannot be resolved"
fi

set +e
output="$(cd "$repo" && bash scripts/ci/brand_gate.sh --mode diff --base nonexistent-target 2>&1)"
status=$?
set -e
if [ "$status" -eq 0 ]; then
  fail "brand_gate must exit nonzero when diff mode has unavailable base ref"
fi

echo "brand_gate tests passed"
