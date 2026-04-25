#!/usr/bin/env bash
# M5b-F12 — orchestrates surfpool + proxy + transfer.e2e.ts.
# Starts fresh background processes, waits for readiness, runs the e2e TS,
# captures the signature, verifies finalization, then tears everything down.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../../../../.." && pwd)"
FIX_DIR="${REPO_ROOT}/mission-fixtures"
LOG_DIR="$(mktemp -d -t m5b-f12-XXXXXX)"
SURFPOOL_LOG="${LOG_DIR}/surfpool.log"
PROXY_LOG="${LOG_DIR}/proxy.log"
SIG_FILE="/tmp/m5b-devnet-sig.txt"

SURFPOOL_PORT="${AGENTGEYSER_SURFPOOL_PORT:-8899}"
PROXY_PORT="${AGENTGEYSER_PROXY_PORT:-8999}"
RPC_URL="http://127.0.0.1:${SURFPOOL_PORT}"
PROXY_URL="http://127.0.0.1:${PROXY_PORT}"

log() { printf '[run.sh] %s\n' "$*"; }

SURFPOOL_PID=""
PROXY_PID=""
cleanup() {
  log "tearing down (logs: ${LOG_DIR})"
  [[ -n "${PROXY_PID}" ]] && kill "${PROXY_PID}" 2>/dev/null || true
  [[ -n "${SURFPOOL_PID}" ]] && kill "${SURFPOOL_PID}" 2>/dev/null || true
}
trap cleanup EXIT

wait_for_port() {
  local url="$1" label="$2" tries=60
  while (( tries-- > 0 )); do
    if curl -s -o /dev/null -w '%{http_code}' "$url" \
        -H 'content-type: application/json' \
        -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -qE '^(200|405)$'; then
      log "${label} up"
      return 0
    fi
    sleep 0.5
  done
  log "${label} failed to come up; logs:"
  tail -80 "${LOG_DIR}"/*.log >&2 || true
  return 1
}

rm -f "${SIG_FILE}"

if solana cluster-version -u "${RPC_URL}" >/dev/null 2>&1; then
  log "surfpool already running on ${RPC_URL}; reusing"
else
  log "starting surfpool on ${RPC_URL} (log: ${SURFPOOL_LOG})"
  surfpool start --port "${SURFPOOL_PORT}" --no-tui >"${SURFPOOL_LOG}" 2>&1 &
  SURFPOOL_PID=$!
  wait_for_port "${RPC_URL}" "surfpool" || exit 1
fi

log "seeding Token-2022 fixtures on surfpool"
bash "${HERE}/setup-surfpool.sh"

STATE_FILE="${HERE}/.surfpool-state.json"
MINT=$(jq -r '.mint' "${STATE_FILE}")
SRC_OWNER=$(jq -r '.source_owner' "${STATE_FILE}")
DST_OWNER=$(jq -r '.dest_owner' "${STATE_FILE}")
SRC_ATA=$(jq -r '.source_ata' "${STATE_FILE}")
DST_ATA=$(jq -r '.dest_ata' "${STATE_FILE}")

if curl -s -o /dev/null -w '%{http_code}' "${PROXY_URL}" \
    -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"ag_listSkills","params":[]}' \
    | grep -qE '^(200|405)$'; then
  log "proxy already running on ${PROXY_URL}; reusing"
else
  log "starting proxy on ${PROXY_URL} (log: ${PROXY_LOG})"
  pushd "${REPO_ROOT}/skeleton" >/dev/null
  AGENTGEYSER_RPC_URL="${RPC_URL}" AGENTGEYSER_PROXY_PORT="${PROXY_PORT}" \
    cargo run --quiet -p proxy >"${PROXY_LOG}" 2>&1 &
  PROXY_PID=$!
  popd >/dev/null
  wait_for_port "${PROXY_URL}" "proxy" || exit 1
fi

export AGENTGEYSER_PROXY_URL="${PROXY_URL}"
export AGENTGEYSER_RPC_URL="${RPC_URL}"
export AGENTGEYSER_DEVNET_MINT="${MINT}"
export AGENTGEYSER_DEVNET_SRC_OWNER="${SRC_OWNER}"
export AGENTGEYSER_DEVNET_DST_OWNER="${DST_OWNER}"
export AGENTGEYSER_DEVNET_SRC_ATA="${SRC_ATA}"
export AGENTGEYSER_DEVNET_DST_ATA="${DST_ATA}"
export AGENTGEYSER_DEVNET_AMOUNT="${AGENTGEYSER_DEVNET_AMOUNT:-10000}"
export AGENTGEYSER_DEVNET_KEYPAIR="${FIX_DIR}/source-owner.json"

log "running transfer.e2e.ts"
# tsx lives in the monorepo root devDependencies; invoke via pnpm exec.
pnpm -C "${REPO_ROOT}/skeleton" exec tsx "${HERE}/transfer.e2e.ts"

if [[ ! -s "${SIG_FILE}" ]]; then
  log "FAIL: ${SIG_FILE} missing or empty"
  exit 1
fi

SIG="$(cat "${SIG_FILE}")"
log "confirming signature ${SIG} on ${RPC_URL}"
CONFIRM_OUT="$(solana confirm "${SIG}" --url "${RPC_URL}" 2>&1)" || true
echo "${CONFIRM_OUT}"
echo "${CONFIRM_OUT}" | grep -qE '(Confirmed|Finalized)' || {
  log "FAIL: solana confirm did not report Confirmed/Finalized"
  exit 1
}

log "OK: signature=${SIG}"
