# AgentGeyser MCP — Claude Desktop Onboarding (macOS)

Register the non-custodial `agentgeyser-mcp-server` with Claude Desktop. Requires `agentgeyser-mcp-server` on `PATH` and the M2 proxy reachable at `http://127.0.0.1:8999`.

## Config path

`~/Library/Application Support/Claude/claude_desktop_config.json`

## Merge recipe (preserves existing `mcpServers`)

```bash
CFG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
SNIP=./claude_desktop_config.snippet.json
[ -f "$CFG" ] || echo '{}' > "$CFG"
jq -s '.[0] * .[1]' "$CFG" "$SNIP" > "$CFG.tmp" && mv "$CFG.tmp" "$CFG"
```

## Tools exposed

- `list_skills` — discovery (no args)
- `invoke_skill` — returns a `transaction_base64` to be finalized by the user-owned client in `skeleton/examples/` (non-custodial)

## Verify

1. Headless (do this first): `bash ./smoke-inspector.sh` — asserts `initialize`, `tools/list` (2 tools), and `tools/call list_skills` against the live proxy.
2. Restart Claude Desktop and confirm the MCP status indicator shows `agentgeyser` connected and lists `list_skills` + `invoke_skill`.
