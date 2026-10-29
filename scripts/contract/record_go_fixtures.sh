#!/usr/bin/env bash
# scripts/contract/record_go_fixtures.sh
# Record HTTP contract response fixtures from Go Agent Engine (TASK-065, SPEC-012, BL-087).
#
# Requirements:
#   - bash, curl, jq, and Go toolchain (go 1.27+)
#   - Copies engine/data to a temporary directory before startup
#   - Runs 'go run ./cmd/agent-engine' from engine/
#   - Queries each Go endpoint with 'port' status in consumer-inventory.md
#   - Normalizes responses with `jq -S .` and writes to tests/fixtures/go-fleet/<method>_<path_slug>.json
#   - Documents volatile fields in tests/fixtures/go-fleet/VOLATILE.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INVENTORY_FILE="$REPO_ROOT/standard-docs/tech-standart/api-contracts/consumer-inventory.md"
FIXTURES_DIR="$REPO_ROOT/tests/fixtures/go-fleet"
ENGINE_DIR="$REPO_ROOT/engine"

echo "=== Go Fleet Contract Fixture Recorder ==="
echo "Repo root:     $REPO_ROOT"
echo "Inventory:     $INVENTORY_FILE"
echo "Output dir:    $FIXTURES_DIR"

# 1. Verify required tools
for tool in curl jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "ERROR: Required tool '$tool' is not installed or not in PATH." >&2
    exit 1
  fi
done

if ! command -v go >/dev/null 2>&1; then
  echo "ERROR: Go toolchain not found on this machine." >&2
  echo "If running in an agent environment without Go, the Owner must run this script locally." >&2
  exit 1
fi

GO_VERSION=$(go version)
echo "Go toolchain:  $GO_VERSION"

# 2. Prepare output directory
mkdir -p "$FIXTURES_DIR"

# 3. Prepare temporary data directory
TMP_DATA_DIR="$(mktemp -d -t galleon-contract-data-XXXXXX)"
echo "Temp data dir: $TMP_DATA_DIR"

if [ -d "$ENGINE_DIR/data" ]; then
  cp -r "$ENGINE_DIR/data"/* "$TMP_DATA_DIR/"
else
  echo "ERROR: $ENGINE_DIR/data not found." >&2
  exit 1
fi

# Pick available port
METRICS_PORT="${GALLEON_ENGINE_METRICS_PORT:-9092}"
GRPC_PORT="${GALLEON_ENGINE_GRPC_PORT:-50054}"
AUTH_TOKEN="test-token"

cleanup() {
  echo "Cleaning up..."
  if [ -n "${ENGINE_PID:-}" ] && kill -0 "$ENGINE_PID" 2>/dev/null; then
    echo "Stopping Agent Engine (PID $ENGINE_PID)..."
    kill "$ENGINE_PID" 2>/dev/null || true
    wait "$ENGINE_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DATA_DIR"
  echo "Cleanup complete."
}
trap cleanup EXIT INT TERM

# 4. Start Agent Engine from engine/
echo "Starting Agent Engine on HTTP :$METRICS_PORT (gRPC :$GRPC_PORT)..."
(
  cd "$ENGINE_DIR"
  export FLEET_DATA_DIR="$TMP_DATA_DIR"
  export GALLEON_DATA_DIR="$TMP_DATA_DIR"
  export GALLEON_ENGINE_DATA_DIR="$TMP_DATA_DIR"
  export GALLEON_ENGINE_TOKEN="$AUTH_TOKEN"
  exec go run ./cmd/agent-engine \
    -metrics-port "$METRICS_PORT" \
    -grpc-port "$GRPC_PORT" \
    -data-dir "$TMP_DATA_DIR" \
    -console-out=false
) >/dev/null 2>&1 &
ENGINE_PID=$!

echo "Agent Engine spawned with PID $ENGINE_PID. Waiting for readiness..."
READY=0
for i in $(seq 1 30); do
  if curl -s -f "http://127.0.0.1:$METRICS_PORT/api/health" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 0.5
done

if [ "$READY" -ne 1 ]; then
  echo "ERROR: Agent Engine failed to become ready within 15 seconds." >&2
  exit 1
fi
echo "Agent Engine is ready."

# 5. Define endpoints to record (19 Go endpoints marked as 'port' in consumer-inventory.md)
# Format: METHOD | PATH_IN_INVENTORY | REQUEST_PATH | REQUEST_BODY (or empty) | SLUG
ENDPOINTS=(
  "GET|/api/fleet/metrics|/api/fleet/metrics||api_fleet_metrics"
  "POST|/api/fleet/deck-bell|/api/fleet/deck-bell|{}|api_fleet_deck_bell"
  "GET|/api/collections/{name}|/api/collections/squads||api_collections_name"
  "PUT|/api/collections/{name}|/api/collections/squads|[{\"id\":\"squad-dev\",\"name\":\"Core Software Delivery Squad\"}]|api_collections_name"
  "GET|/api/system/metrics|/api/system/metrics||api_system_metrics"
  "GET|/api/network/qrcode|/api/network/qrcode?url=http://127.0.0.1:9090||api_network_qrcode"
  "GET|/api/system/executive-briefing|/api/system/executive-briefing||api_system_executive_briefing"
  "GET|/api/providers/harbor|/api/providers/harbor||api_providers_harbor"
  "GET|/api/diagnostics|/api/diagnostics||api_diagnostics"
  "GET|/api/snapshots|/api/snapshots||api_snapshots"
  "POST|/api/snapshots|/api/snapshots|{\"label\":\"Genesis Commissioning State\"}|api_snapshots"
  "GET|/api/engine/processes|/api/engine/processes||api_engine_processes"
  "POST|/api/terminal/exec|/api/terminal/exec|{\"command\":\"git status\"}|api_terminal_exec"
  "GET|/api/fleet/policies|/api/fleet/policies||api_fleet_policies"
  "POST|/api/chat/quartermaster|/api/chat/quartermaster|{\"message\":\"report status\"}|api_chat_quartermaster"
  "GET|/api/system/network|/api/system/network||api_system_network"
  "GET|/api/project/git-status|/api/project/git-status?path=.||api_project_git_status"
  "GET|/api/v1/audit/trail|/api/v1/audit/trail||api_v1_audit_trail"
  "GET|/api/fleet/seed|/api/fleet/seed||api_fleet_seed"
)

RECORDED_COUNT=0
TOTAL_COUNT=${#ENDPOINTS[@]}

echo "Recording $TOTAL_COUNT endpoint fixtures..."

for entry in "${ENDPOINTS[@]}"; do
  IFS="|" read -r method inv_path req_path req_body slug <<< "$entry"
  target_file="$FIXTURES_DIR/${method}_${slug}.json"
  url="http://127.0.0.1:$METRICS_PORT$req_path"

  echo -n "  [$method] $inv_path -> ${method}_${slug}.json ... "

  curl_args=(
    -sS -f
    -X "$method"
    -H "Authorization: Bearer $AUTH_TOKEN"
  )

  if [ -n "$req_body" ]; then
    curl_args+=(-H "Content-Type: application/json" -d "$req_body")
  fi

  raw_resp=$(curl "${curl_args[@]}" "$url")
  if [ -z "$raw_resp" ]; then
    echo "FAILED (empty response)" >&2
    exit 1
  fi

  # Validate and format with jq -S
  echo "$raw_resp" | jq -S . > "$target_file"

  # If this was parameterized collections, also save the concrete slug copy
  if [ "$inv_path" = "/api/collections/{name}" ]; then
    cp "$target_file" "$FIXTURES_DIR/${method}_api_collections_squads.json"
  fi

  file_size=$(stat -c%s "$target_file")
  echo "OK (${file_size} bytes)"
  RECORDED_COUNT=$((RECORDED_COUNT + 1))
done

echo "Successfully recorded $RECORDED_COUNT/$TOTAL_COUNT fixtures in $FIXTURES_DIR."
