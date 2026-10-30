#!/usr/bin/env bash
# ==============================================================================
# scripts/e2e/smoke_dashboard.sh
# Smoke test end-to-end dashboard tanpa dependensi proses Go (agent-engine).
# SPEC-014 / BL-089 / TASK-087
#
# Alur pengujian:
# 1. Verifikasi ketiadaan proses Go agent-engine (pgrep -f agent-engine).
# 2. Start galleon daemon dengan GALLEON_CONFIG_DIR sementara.
# 3. Ambil pairing code via galleon gateway get-paircode.
# 4. Lakukan POST /pair dengan header X-Pairing-Code untuk mendapatkan bearer token.
# 5. Panggil setiap rute port/restore di consumer-inventory.md:
#    - Tanpa token: verifikasi HTTP 401 Unauthorized (pada rute berotentikasi Bearer).
#    - Dengan token: verifikasi HTTP 2xx (pada rute berotentikasi Bearer).
#    - Rute publik dan admin lokal diverifikasi sesuai perilakunya.
# 6. Bersihkan proses daemon dan direktori konfigurasi sementara saat exit.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Cari binary galleon
GALLEON_BIN="${REPO_ROOT}/target/debug/galleon"
if [ ! -x "${GALLEON_BIN}" ]; then
    if command -v galleon >/dev/null 2>&1; then
        GALLEON_BIN="$(command -v galleon)"
    elif [ -x "${REPO_ROOT}/target/release/galleon" ]; then
        GALLEON_BIN="${REPO_ROOT}/target/release/galleon"
    else
        echo "ERROR: binary galleon tidak ditemukan di ${GALLEON_BIN}. Harap build terlebih dahulu." >&2
        exit 1
    fi
fi

# Port daemon yang digunakan untuk smoke test (bisa di-override lewat env SMOKE_PORT)
PORT="${SMOKE_PORT:-42617}"
HOST="127.0.0.1"
BASE_URL="http://${HOST}:${PORT}"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

record_result() {
    local name="$1"
    local expected="$2"
    local actual="$3"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    if [[ "|${expected}|" =~ \|${actual}\| ]]; then
        echo "  [PASS] ${name} (expected: ${expected}, got: ${actual})"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo "  [FAIL] ${name} (expected: ${expected}, got: ${actual})" >&2
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

echo "=== Galleon Dashboard E2E Smoke Test (SPEC-014 / TASK-087) ==="
echo "Repo root  : ${REPO_ROOT}"
echo "Binary     : ${GALLEON_BIN}"
echo "Target host: ${BASE_URL}"
echo ""

# ------------------------------------------------------------------------------
# 1. Verifikasi proses Go (agent-engine) harus kosong
# ------------------------------------------------------------------------------
echo "--- Step 1: Memeriksa ketiadaan proses Go agent-engine ---"
GO_PROCS=$(pgrep -f "agent-engine" 2>/dev/null | while read -r p; do
    cmd=$(tr '\0' ' ' < "/proc/$p/cmdline" 2>/dev/null || true)
    case "$cmd" in
        *agy*|*smoke_dashboard*) ;;
        *) echo "$p: $cmd" ;;
    esac
done || true)

if [ -n "${GO_PROCS}" ]; then
    echo "ERROR: Proses Go agent-engine masih berjalan di sistem:" >&2
    echo "${GO_PROCS}" >&2
    exit 1
fi
echo "OK: Tidak ada proses Go (agent-engine) yang berjalan."
echo ""

# ------------------------------------------------------------------------------
# 2. Siapkan GALLEON_CONFIG_DIR sementara dan jalankan galleon daemon
# ------------------------------------------------------------------------------
echo "--- Step 2: Menjalankan galleon daemon dengan GALLEON_CONFIG_DIR sementara ---"
TEMP_CONFIG_DIR="$(mktemp -d -t galleon-smoke-XXXXXX)"
export GALLEON_CONFIG_DIR="${TEMP_CONFIG_DIR}"

# Inisialisasi git repository minimal di direktori sementara agar endpoint git-status valid
git init -q "${GALLEON_CONFIG_DIR}"

# Buat agent minimal agar WebSocket chat route (/ws, /ws/chat) dapat mengikat agent
cat << 'EOF' > "${GALLEON_CONFIG_DIR}/config.toml"
[agents.main]
EOF

DAEMON_LOG="${TEMP_CONFIG_DIR}/daemon.log"
DAEMON_PID=""

cleanup() {
    local exit_code=$?
    echo ""
    echo "--- Cleanup: Menutup galleon daemon ---"
    if [ -n "${DAEMON_PID}" ] && kill -0 "${DAEMON_PID}" 2>/dev/null; then
        echo "Menghentikan daemon (PID: ${DAEMON_PID})..."
        kill "${DAEMON_PID}" 2>/dev/null || true
        wait "${DAEMON_PID}" 2>/dev/null || true
    fi
    if [ -d "${TEMP_CONFIG_DIR}" ]; then
        rm -rf "${TEMP_CONFIG_DIR}"
    fi
    echo "Cleanup selesai."
    exit ${exit_code}
}
trap cleanup EXIT INT TERM

# Start galleon daemon
"${GALLEON_BIN}" daemon -p "${PORT}" --host "${HOST}" >"${DAEMON_LOG}" 2>&1 &
DAEMON_PID=$!
echo "Daemon berjalan di background (PID: ${DAEMON_PID})."

# Tunggu sampai gateway ready
READY=0
for i in $(seq 1 30); do
    if curl -s -f -m 1 "${BASE_URL}/health" >/dev/null 2>&1; then
        READY=1
        break
    fi
    sleep 0.5
done

if [ "${READY}" -ne 1 ]; then
    echo "ERROR: Galleon daemon gagal merespons /health dalam 15 detik." >&2
    echo "Log daemon:" >&2
    cat "${DAEMON_LOG}" >&2
    exit 1
fi
echo "Galleon daemon siap menerima request."
echo ""

# ------------------------------------------------------------------------------
# 3. Ambil kode pairing via galleon gateway get-paircode
# ------------------------------------------------------------------------------
echo "--- Step 3: Mengambil kode pairing via galleon gateway get-paircode ---"
PAIRCODE_OUTPUT=$("${GALLEON_BIN}" gateway get-paircode -p "${PORT}" --host "${HOST}")
PAIRING_CODE=$(echo "${PAIRCODE_OUTPUT}" | grep -E '│\s+[0-9A-Za-z-]+\s+│' | sed -E 's/.*│\s+([0-9A-Za-z-]+)\s+│.*/\1/' || true)

if [ -z "${PAIRING_CODE}" ]; then
    echo "ERROR: Gagal mengekstrak pairing code dari output CLI:" >&2
    echo "${PAIRCODE_OUTPUT}" >&2
    exit 1
fi
echo "Pairing code didapatkan: ${PAIRING_CODE}"
echo ""

# ------------------------------------------------------------------------------
# 4. Lakukan POST /pair untuk mendapatkan Bearer token
# ------------------------------------------------------------------------------
echo "--- Step 4: POST /pair dengan kode pairing ---"
PAIR_RESP=$(curl -s -X POST -H "X-Pairing-Code: ${PAIRING_CODE}" "${BASE_URL}/pair")
TOKEN=$(echo "${PAIR_RESP}" | jq -r '.token // empty')

if [ -z "${TOKEN}" ]; then
    echo "ERROR: Gagal melakukan pairing; respons tidak memuat token:" >&2
    echo "${PAIR_RESP}" >&2
    exit 1
fi
echo "Pairing berhasil! Bearer token: ${TOKEN:0:16}..."
echo ""

# Siapkan device kedua & ketiga untuk pengujian rotasi token dan pencabutan device (DELETE /api/devices/{id})
PAIRCODE_2=$(curl -s -X POST -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/api/pairing/initiate" | jq -r '.pairing_code // empty')
if [ -n "${PAIRCODE_2}" ]; then
    curl -s -X POST -H "X-Pairing-Code: ${PAIRCODE_2}" "${BASE_URL}/pair" >/dev/null 2>&1 || true
fi
DEVICE_2_ID=$(curl -s -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/api/devices" | jq -r '.devices[-1].id // empty')

PAIRCODE_3=$(curl -s -X POST -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/api/pairing/initiate" | jq -r '.pairing_code // empty')
if [ -n "${PAIRCODE_3}" ]; then
    curl -s -X POST -H "X-Pairing-Code: ${PAIRCODE_3}" "${BASE_URL}/pair" >/dev/null 2>&1 || true
fi
DEVICE_3_ID=$(curl -s -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/api/devices" | jq -r '.devices[-1].id // empty')

# ------------------------------------------------------------------------------
# 5. Panggil setiap rute port/restore di consumer-inventory.md
# ------------------------------------------------------------------------------
echo "--- Step 5: Pengujian rute consumer-inventory.md (tanpa token vs dengan token) ---"

test_route() {
    local method="$1"
    local path="$2"
    local exp_unauth="$3"
    local exp_auth="$4"
    local data="${5:-}"
    local extra_headers="${6:-}"

    local url="${BASE_URL}${path}"

    # 1. Panggilan tanpa token
    local code_unauth
    if [ -n "${data}" ]; then
        code_unauth=$(curl -s -o /dev/null -w "%{http_code}" -X "${method}" \
            -H "Content-Type: application/json" ${extra_headers} \
            --data "${data}" "${url}" || true)
    else
        code_unauth=$(curl -s -o /dev/null -w "%{http_code}" -X "${method}" \
            ${extra_headers} "${url}" || true)
    fi
    record_result "${method} ${path} [Unauthenticated]" "${exp_unauth}" "${code_unauth}"

    # 2. Panggilan dengan token
    local code_auth
    if [ -n "${data}" ]; then
        code_auth=$(curl -s -o /dev/null -w "%{http_code}" -X "${method}" \
            -H "Authorization: Bearer ${TOKEN}" -H "Content-Type: application/json" ${extra_headers} \
            --data "${data}" "${url}" || true)
    else
        code_auth=$(curl -s -o /dev/null -w "%{http_code}" -X "${method}" \
            -H "Authorization: Bearer ${TOKEN}" ${extra_headers} "${url}" || true)
    fi
    record_result "${method} ${path} [Authenticated]" "${exp_auth}" "${code_auth}"
}

# ------------------------------------------------------------------------------
# Rute Publik & Admin
# ------------------------------------------------------------------------------
echo "A. Rute Publik & Admin (Localhost)"
test_route "GET"  "/health"                      "200" "200"
test_route "GET"  "/ready"                       "200" "200"
test_route "GET"  "/api/health"                  "200" "200"
test_route "GET"  "/api/auth/ratelimit"          "200" "200"
test_route "GET"  "/pair/code"                   "200" "200"
test_route "GET"  "/admin/paircode"              "200" "200"
test_route "POST" "/admin/paircode/new"          "200" "200"
test_route "GET"  "/admin/sop/pending"           "503" "503"

# ------------------------------------------------------------------------------
# Rute Fleet / Dashboard (Port dari Go ke Rust galleon-fleet-core)
# ------------------------------------------------------------------------------
echo "B. Rute Fleet / Dashboard"
test_route "GET"  "/api/fleet/metrics"                 "401" "200"
test_route "POST" "/api/fleet/deck-bell"               "401" "200" "{}"
test_route "GET"  "/api/collections/squads"            "401" "200"
test_route "PUT"  "/api/collections/squads"            "401" "200" "[]"
test_route "GET"  "/api/system/metrics"                "401" "200"
test_route "GET"  "/api/network/qrcode"                "401" "200"
test_route "GET"  "/api/system/executive-briefing"     "401" "200"
test_route "GET"  "/api/providers/harbor"              "401" "200"
test_route "GET"  "/api/diagnostics"                   "401" "200"
test_route "GET"  "/api/snapshots"                     "401" "200"
test_route "POST" "/api/snapshots"                     "401" "201" '{"label":"smoke-test"}'
test_route "GET"  "/api/engine/processes"              "401" "200"
test_route "POST" "/api/terminal/exec"                 "401" "200" '{"command":"echo smoke"}'
test_route "GET"  "/api/fleet/policies"                "401" "200"
test_route "POST" "/api/chat/quartermaster"            "401" "200" '{"message":"status report"}'
test_route "GET"  "/api/system/network"                "401" "200"
test_route "GET"  "/api/project/git-status"            "401" "200"
test_route "GET"  "/api/v1/audit/trail"                "401" "200"
test_route "GET"  "/api/fleet/seed"                    "401" "200"

# ------------------------------------------------------------------------------
# Rute Section & Config Gateway (Port/Pertahankan)
# ------------------------------------------------------------------------------
echo "C. Rute Sections & Gateway Config"
test_route "GET"  "/api/sections"                                "401" "200"
test_route "GET"  "/api/sections/status"                         "401" "200"
test_route "GET"  "/api/sections/agent-options"                  "401" "200"
test_route "GET"  "/api/sections/catalog"                        "401" "200"
test_route "GET"  "/api/sections/catalog/models?model_provider=ollama" "401" "200"
test_route "GET"  "/api/sections/memory"                         "401" "200"
test_route "POST" "/api/sections/memory/items/sqlite"            "401" "200" "{}"
test_route "GET"  "/api/config/sections"                         "401" "200"
test_route "GET"  "/api/config/sections/memory"                  "401" "200"
test_route "POST" "/api/config/sections/memory/items/sqlite"     "401" "200" "{}"
test_route "GET"  "/api/config/catalog"                          "401" "200"
test_route "GET"  "/api/config/catalog/models?model_provider=ollama" "401" "200"
test_route "GET"  "/api/config/status"                           "401" "200"
test_route "GET"  "/api/config/agent-options"                    "401" "200"
test_route "GET"  "/api/status"                                  "401" "200"
test_route "GET"  "/api/config"                                  "401" "200"
test_route "PATCH" "/api/config"                                 "401" "200" "[]"
test_route "OPTIONS" "/api/config"                               "200" "200"
test_route "GET"  "/api/config/drift"                            "401" "200"
test_route "GET"  "/api/logs"                                    "401" "200"

# ------------------------------------------------------------------------------
# Rute Pairing & Device Management
# ------------------------------------------------------------------------------
echo "D. Rute Pairing & Device Lifecycle"
test_route "POST"  "/api/pairing/initiate"          "401" "200"
test_route "GET"   "/api/devices"                   "401" "200"
test_route "PATCH" "/api/devices/me/capabilities"   "401" "200" '{"capabilities":["terminal"]}'

if [ -n "${DEVICE_2_ID}" ]; then
    test_route "POST" "/api/devices/${DEVICE_2_ID}/token/rotate" "401" "200"
fi
if [ -n "${DEVICE_3_ID}" ]; then
    test_route "DELETE" "/api/devices/${DEVICE_3_ID}"            "401" "200"
fi

# ------------------------------------------------------------------------------
# Rute Streaming (SSE) & WebSocket
# ------------------------------------------------------------------------------
echo "E. Rute Streaming & WebSocket Handshake"
# SSE /api/events
code_sse_unauth=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/api/events" || true)
record_result "GET /api/events [Unauthenticated]" "401" "${code_sse_unauth}"

code_sse_auth=$(curl -s -N -H "Authorization: Bearer ${TOKEN}" --max-time 1 -o /dev/null -w "%{http_code}" "${BASE_URL}/api/events" || true)
record_result "GET /api/events [Authenticated]" "200" "${code_sse_auth}"

# WebSocket handshake headers
WS_HEADERS=(-H "Upgrade: websocket" -H "Connection: Upgrade" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" -H "Sec-WebSocket-Version: 13")

# /ws
code_ws_unauth=$(curl -s -o /dev/null -w "%{http_code}" "${WS_HEADERS[@]}" "${BASE_URL}/ws?agent=main" || true)
record_result "GET /ws?agent=main [Unauthenticated]" "401" "${code_ws_unauth}"

code_ws_auth=$(curl -s -N -H "Authorization: Bearer ${TOKEN}" --max-time 1 -o /dev/null -w "%{http_code}" "${WS_HEADERS[@]}" "${BASE_URL}/ws?agent=main" || true)
record_result "GET /ws?agent=main [Authenticated Handshake]" "101" "${code_ws_auth}"

# /ws/chat
code_wschat_unauth=$(curl -s -o /dev/null -w "%{http_code}" "${WS_HEADERS[@]}" "${BASE_URL}/ws/chat?agent=main" || true)
record_result "GET /ws/chat?agent=main [Unauthenticated]" "401" "${code_wschat_unauth}"

code_wschat_auth=$(curl -s -N -H "Authorization: Bearer ${TOKEN}" --max-time 1 -o /dev/null -w "%{http_code}" "${WS_HEADERS[@]}" "${BASE_URL}/ws/chat?agent=main" || true)
record_result "GET /ws/chat?agent=main [Authenticated Handshake]" "101" "${code_wschat_auth}"

# /acp
code_acp_unauth=$(curl -s -o /dev/null -w "%{http_code}" "${WS_HEADERS[@]}" "${BASE_URL}/acp" || true)
record_result "GET /acp [Unauthenticated]" "401" "${code_acp_unauth}"

code_acp_auth=$(curl -s -N -H "Authorization: Bearer ${TOKEN}" --max-time 1 -o /dev/null -w "%{http_code}" "${WS_HEADERS[@]}" "${BASE_URL}/acp" || true)
record_result "GET /acp [Authenticated Handshake]" "101" "${code_acp_auth}"

# ------------------------------------------------------------------------------
# Rute Webhook (Verifikasi otentikasi signature / auth)
# ------------------------------------------------------------------------------
echo "F. Rute Webhook Signature Ingress"
test_route "POST" "/sop/smoke"          "401" "503"
test_route "POST" "/whatsapp"           "401" "404"
test_route "POST" "/linq"               "401" "404"
test_route "POST" "/nextcloud-talk"     "401" "404"
test_route "POST" "/webhook/gmail"      "404" "404"

# ------------------------------------------------------------------------------
# Admin reload (dijalankan di akhir agar tidak memutus sesi pengujian sebelumnya)
# ------------------------------------------------------------------------------
echo "G. Admin Reload Lifecycle"
test_route "POST" "/admin/reload"       "200" "200"

echo ""
echo "=== Ringkasan Hasil Uji Smoke E2E ==="
echo "Total Uji  : ${TOTAL_TESTS}"
echo "Lolos (OK) : ${PASSED_TESTS}"
echo "Gagal (ERR): ${FAILED_TESTS}"
echo ""

if [ "${FAILED_TESTS}" -gt 0 ]; then
    echo "STATUS: FAIL (${FAILED_TESTS} uji gagal)" >&2
    exit 1
fi

echo "STATUS: SUCCESS (Seluruh rute port/restore lulus pengujian smoke)"
exit 0
