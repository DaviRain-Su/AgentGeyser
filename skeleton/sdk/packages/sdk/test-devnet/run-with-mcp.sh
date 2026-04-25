#!/usr/bin/env bash
# M5c-F15 — cross-layer MCP→proxy→tx-builder→surfpool integration harness.
# Port triangle: surfpool RPC :8899, AgentGeyser proxy :8999, MCP HTTP :9099.
# Starts fresh owned services when absent, waits for readiness, runs the MCP
# e2e test, independently confirms the signature, then tears down owned PIDs.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../../../../.." && pwd)"
FIX_DIR="${REPO_ROOT}/mission-fixtures"
LOG_DIR="$(mktemp -d -t m5c-f15-XXXXXX)"
EVIDENCE_FILE="/tmp/m5c-evidence/f15-mcp-evidence.json"

SURFPOOL_PORT="${AGENTGEYSER_SURFPOOL_PORT:-8899}"
PROXY_PORT="${AGENTGEYSER_PROXY_PORT:-8999}"
MCP_PORT="${AGENTGEYSER_MCP_PORT:-9099}"
RPC_URL="http://127.0.0.1:${SURFPOOL_PORT}"
PROXY_URL="http://127.0.0.1:${PROXY_PORT}"
MCP_URL="http://127.0.0.1:${MCP_PORT}/mcp"

log() { printf '[run-with-mcp.sh] %s\n' "$*"; }

SURFPOOL_PID=""
PROXY_PID=""
MCP_PID=""
cleanup() {
  log "tearing down owned processes (logs: ${LOG_DIR})"
  for pid in "${MCP_PID}" "${PROXY_PID}" "${SURFPOOL_PID}"; do
    [[ -n "${pid}" ]] || continue
    kill "${pid}" 2>/dev/null || true
    sleep 0.2
    kill -0 "${pid}" 2>/dev/null && kill -9 "${pid}" 2>/dev/null || true
    wait "${pid}" 2>/dev/null || true
  done
}
trap cleanup EXIT

wait_for_port() {
  local url="$1" label="$2" method="$3" tries=90
  while (( tries-- > 0 )); do
    local status
    status="$(curl -s -o /dev/null -w '%{http_code}' -X POST "$url" \
      -H 'content-type: application/json' \
      -H 'accept: application/json, text/event-stream' \
      -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":[]}" || true)"
    if [[ "${status}" != "000" ]]; then
      log "${label} up"
      return 0
    fi
    sleep 0.5
  done
  log "${label} failed to come up; logs:"
  tail -80 "${LOG_DIR}"/*.log >&2 || true
  return 1
}

mkdir -p /tmp/m5c-evidence
rm -f "${EVIDENCE_FILE}"

if solana cluster-version -u "${RPC_URL}" >/dev/null 2>&1; then
  log "surfpool already running on ${RPC_URL}; reusing"
else
  log "starting surfpool on ${RPC_URL}"
  surfpool start --port "${SURFPOOL_PORT}" --no-tui >"${LOG_DIR}/surfpool.log" 2>&1 &
  SURFPOOL_PID=$!
  wait_for_port "${RPC_URL}" "surfpool" "getHealth"
fi

log "seeding Token-2022 fixtures on surfpool"
AGENTGEYSER_RPC_URL="${RPC_URL}" bash "${HERE}/setup-surfpool.sh"

STATE_FILE="${HERE}/.surfpool-state.json"
export AGENTGEYSER_DEVNET_MINT="$(jq -r '.mint' "${STATE_FILE}")"
export AGENTGEYSER_DEVNET_SRC_OWNER="$(jq -r '.source_owner' "${STATE_FILE}")"
export AGENTGEYSER_DEVNET_DST_OWNER="$(jq -r '.dest_owner' "${STATE_FILE}")"
export AGENTGEYSER_DEVNET_SRC_ATA="$(jq -r '.source_ata' "${STATE_FILE}")"
export AGENTGEYSER_DEVNET_DST_ATA="$(jq -r '.dest_ata' "${STATE_FILE}")"
export AGENTGEYSER_DEVNET_AMOUNT="${AGENTGEYSER_DEVNET_AMOUNT:-10000}"
export AGENTGEYSER_DEVNET_KEYPAIR="${FIX_DIR}/source-owner.json"
export AGENTGEYSER_RPC_URL="${RPC_URL}"
export AGENTGEYSER_PROXY_URL="${PROXY_URL}"
export AGENTGEYSER_MCP_URL="${MCP_URL}"
export AGENTGEYSER_MCP_EVIDENCE="${EVIDENCE_FILE}"

if curl -sf -X POST "${PROXY_URL}" -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"ag_listSkills","params":[]}' >/dev/null; then
  log "proxy already running on ${PROXY_URL}; reusing"
else
  log "starting proxy on ${PROXY_URL}"
  pushd "${REPO_ROOT}/skeleton" >/dev/null
  AGENTGEYSER_RPC_URL="${RPC_URL}" AGENTGEYSER_PROXY_PORT="${PROXY_PORT}" \
    cargo run --quiet -p proxy --bin proxy >"${LOG_DIR}/proxy.log" 2>&1 &
  PROXY_PID=$!
  popd >/dev/null
  wait_for_port "${PROXY_URL}" "proxy" "ag_listSkills"
fi

log "starting MCP HTTP on ${MCP_URL}"
pushd "${REPO_ROOT}/skeleton" >/dev/null
AGENTGEYSER_PROXY_URL="${PROXY_URL}" \
  cargo run --quiet -p mcp-server --bin agentgeyser-mcp-server -- \
    --transport http --bind "127.0.0.1:${MCP_PORT}" >"${LOG_DIR}/mcp.log" 2>&1 &
MCP_PID=$!
popd >/dev/null
wait_for_port "${MCP_URL}" "mcp" "initialize"

log "running mcp-invoke.e2e.ts"
pnpm -C "${REPO_ROOT}/skeleton/sdk/packages/sdk" exec vitest run \
  --config test-devnet/vitest.mcp.config.ts

log "evidence:"
cat "${EVIDENCE_FILE}"
SIG="$(jq -r '.confirmed_signature' "${EVIDENCE_FILE}")"
log "confirming signature ${SIG} on ${RPC_URL}"
solana confirm "${SIG}" --url "${RPC_URL}" --verbose
