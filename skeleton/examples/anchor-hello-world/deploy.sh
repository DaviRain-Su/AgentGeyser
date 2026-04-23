#!/usr/bin/env bash
set -euo pipefail
# ⚠️  requires a funded devnet keypair; DO NOT run in CI
#
# One-shot script: build + deploy the hello_world Anchor program to devnet
# and publish its IDL on-chain. Writes the deployed program ID into
# PROGRAM_ID.txt, replacing the <PENDING_DEPLOY> placeholder.
#
# Prereqs:
#   - anchor --version >= 0.30
#   - solana --version (CLI installed and configured for devnet)
#   - ~/.config/solana/id.json exists and is funded (airdrop: `solana airdrop 2`)
#
# Usage:
#   cd skeleton/examples/anchor-hello-world
#   ./deploy.sh

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

echo "==> anchor deploy --provider.cluster devnet"
anchor deploy --provider.cluster devnet

echo "==> anchor idl init --provider.cluster devnet"
anchor idl init "$PROGRAM_ID" \
  --filepath target/idl/hello_world.json \
  --provider.cluster devnet

echo "==> writing $PROGRAM_ID to PROGRAM_ID.txt"
echo "$PROGRAM_ID" > PROGRAM_ID.txt

echo "done. program id: $PROGRAM_ID"
