#!/usr/bin/env bash
set -euo pipefail

# run-track-b.sh — orchestrates the AgentGeyser Track B (SPL-Token transfer) e2e
# against a running surfpool + proxy. Reads .surfpool-state.json for pubkeys.

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
STATE="$HERE/.surfpool-state.json"
FIXTURES="$REPO_ROOT/mission-fixtures"

if [ ! -f "$STATE" ]; then
  echo "error: $STATE missing — run setup-surfpool.sh first" >&2
  exit 1
fi

MINT=$(jq -r '.mint' "$STATE")
SOURCE=$(jq -r '.source' "$STATE")
DEST=$(jq -r '.destination' "$STATE")
SRC_OWNER=$(jq -r '.source_owner' "$STATE")

export AGENTGEYSER_ENDPOINT="${AGENTGEYSER_ENDPOINT:-http://127.0.0.1:8999}"
export AGENTGEYSER_RPC_URL="${AGENTGEYSER_RPC_URL:-http://127.0.0.1:8899}"
export AGENTGEYSER_DEMO_KEYPAIR="${AGENTGEYSER_DEMO_KEYPAIR:-$FIXTURES/source-owner.json}"
export AGENTGEYSER_DEMO_SOURCE="$SOURCE"
export AGENTGEYSER_DEMO_DEST="$DEST"
export AGENTGEYSER_DEMO_AUTHORITY="$SRC_OWNER"

echo "[run-track-b] mint=$MINT"
echo "[run-track-b] source=$SOURCE dest=$DEST authority=$SRC_OWNER"
echo "[run-track-b] endpoint=$AGENTGEYSER_ENDPOINT rpc=$AGENTGEYSER_RPC_URL"

exec pnpm -C "$HERE" exec tsx live-smoke.ts --e2e
