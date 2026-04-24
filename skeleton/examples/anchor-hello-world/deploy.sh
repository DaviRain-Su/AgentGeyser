#!/usr/bin/env bash
set -euo pipefail
# ⚠️  requires a funded keypair; DO NOT run in CI
#
# One-shot script: build + deploy the hello_world Anchor program to the
# target cluster and publish its IDL on-chain. Writes the deployed program
# ID into PROGRAM_ID.txt, replacing the <PENDING_DEPLOY> placeholder.
#
# Prereqs:
#   - anchor --version >= 0.30
#   - solana --version (CLI installed)
#   - ~/.config/solana/id.json exists and is funded
#
# Usage:
#   cd skeleton/examples/anchor-hello-world
#   ./deploy.sh [--cluster-url <url>]
#     default cluster-url: http://127.0.0.1:8899 (local surfpool)

CLUSTER_URL="http://127.0.0.1:8899"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --cluster-url) CLUSTER_URL="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

export PATH="$HOME/.cargo/bin:$PATH"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"

echo "==> anchor build"
anchor build

KEYPAIR="target/deploy/hello_world-keypair.json"
if [[ ! -f "$KEYPAIR" ]]; then
  echo "ERROR: expected $KEYPAIR after anchor build" >&2
  exit 1
fi

echo "==> verify program keypair pubkey"
PROGRAM_ID="$(solana-keygen pubkey "$KEYPAIR")"
echo "program id: $PROGRAM_ID"

echo "==> solana program deploy --url $CLUSTER_URL"
[[ -f target/deploy/hello_world.so ]] || cp programs/hello_world/target/deploy/hello_world.so target/deploy/hello_world.so
solana program deploy "target/deploy/hello_world.so" --program-id "$KEYPAIR" --url "$CLUSTER_URL" --keypair "$HOME/.config/solana/id.json" --use-rpc

echo "==> anchor idl init --provider.cluster $CLUSTER_URL"
anchor idl init "$PROGRAM_ID" \
  --filepath target/idl/hello_world.json \
  --provider.cluster "$CLUSTER_URL"

echo "==> writing $PROGRAM_ID to PROGRAM_ID.txt"
echo "$PROGRAM_ID" > PROGRAM_ID.txt

echo "done. program id: $PROGRAM_ID"
