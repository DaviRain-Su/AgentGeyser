#!/usr/bin/env bash
set -euo pipefail

# AgentGeyser MCP headless smoke test.
# Uses `npx -y @modelcontextprotocol/inspector@latest --cli` — no install step.
# Exercises only read-only discovery tools; never touches custodial material.

PROXY_URL="${AGENTGEYSER_PROXY_URL:-http://127.0.0.1:8999}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# (1) Preflight: proxy must be reachable via JSON-RPC.
PING_BODY='{"jsonrpc":"2.0","id":1,"method":"getVersion"}'
if ! curl -fsS -X POST -H 'content-type: application/json' \
     --data "${PING_BODY}" "${PROXY_URL}" >/dev/null; then
  echo "AgentGeyser proxy not reachable at ${PROXY_URL}" >&2
  exit 1
fi

# (2) Locate the MCP server binary (prefer release, fall back to debug).
BIN_RELEASE="${REPO_ROOT}/target/release/agentgeyser-mcp-server"
BIN_DEBUG="${REPO_ROOT}/target/debug/agentgeyser-mcp-server"
if   [ -x "${BIN_RELEASE}" ]; then MCP_BIN="${BIN_RELEASE}"
elif [ -x "${BIN_DEBUG}"   ]; then MCP_BIN="${BIN_DEBUG}"
else
  echo "agentgeyser-mcp-server not built; run: cargo build -p mcp-server --release" >&2
  exit 1
fi

# (3) Track the server process under a PID and clean up on any exit.
MCP_PID=""
cleanup() {
  if [ -n "${MCP_PID}" ] && kill -0 "${MCP_PID}" 2>/dev/null; then
    kill "${MCP_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# Liveness probe — Inspector spawns its own child per call, but we also guard
# a background sentinel so a premature crash surfaces as a hard failure.
"${MCP_BIN}" >/dev/null 2>&1 &
MCP_PID=$!
sleep 1
if ! kill -0 "${MCP_PID}" 2>/dev/null; then
  echo "MCP server exited prematurely" >&2
  exit 1
fi

INSPECTOR=(npx -y @modelcontextprotocol/inspector@latest --cli)

# (4a) initialize — server name + protocolVersion must be present.
INIT_OUT="$("${INSPECTOR[@]}" --method initialize -- "${MCP_BIN}")"
echo "${INIT_OUT}" | grep -q 'agentgeyser-mcp-server' \
  || { echo "initialize: server name missing" >&2; exit 1; }
echo "${INIT_OUT}" | grep -q 'protocolVersion' \
  || { echo "initialize: protocolVersion missing" >&2; exit 1; }

# (4b) tools/list — exactly 2 tools.
LIST_OUT="$("${INSPECTOR[@]}" --method tools/list -- "${MCP_BIN}")"
TOOL_COUNT="$(printf '%s' "${LIST_OUT}" | jq '.tools | length')"
if [ "${TOOL_COUNT}" != "2" ]; then
  echo "tools/list: expected 2 tools, got ${TOOL_COUNT}" >&2
  exit 1
fi

# (4c) tools/call list_skills — returns non-empty array of skill entries.
CALL_OUT="$("${INSPECTOR[@]}" --method tools/call \
            --tool-name list_skills --tool-arg '{}' -- "${MCP_BIN}")"
printf '%s' "${CALL_OUT}" \
  | jq -e '[.. | .text? // empty | fromjson? // empty | arrays] | add | length > 0' \
    >/dev/null \
  || { echo "list_skills: empty or malformed result" >&2; exit 1; }

# (5) Final liveness check before success.
if ! kill -0 "${MCP_PID}" 2>/dev/null; then
  echo "MCP server exited prematurely" >&2
  exit 1
fi

echo "smoke-inspector: OK (proxy=${PROXY_URL}, bin=${MCP_BIN}, tools=${TOOL_COUNT})"
