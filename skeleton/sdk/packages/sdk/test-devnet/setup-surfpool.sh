#!/usr/bin/env bash
# M5b-F12 — SPL-Token-2022 state seeder for surfpool (local mainnet-beta fork).
# Idempotent: safe to re-run against an already-primed surfpool.
# Expects surfpool to be reachable at ${RPC_URL:-http://127.0.0.1:8899}.
# Emits pubkey scratchpad at test-devnet/.surfpool-state.json for run.sh.
set -euo pipefail

RPC="${AGENTGEYSER_RPC_URL:-http://127.0.0.1:8899}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../../../../.." && pwd)"
FIX_DIR="${REPO_ROOT}/mission-fixtures"
STATE_FILE="${HERE}/.surfpool-state.json"
DEFAULT_KP="${HOME}/.config/solana/id.json"

log() { printf '[setup-surfpool-2022] %s\n' "$*"; }

if ! solana cluster-version -u "${RPC}" >/dev/null 2>&1; then
  echo "ERROR: surfpool not reachable at ${RPC}" >&2
  exit 1
fi

for f in mint.json source-owner.json dest-owner.json; do
  [[ -f "${FIX_DIR}/${f}" ]] || { echo "ERROR: missing fixture ${FIX_DIR}/${f}" >&2; exit 1; }
done

MINT=$(solana-keygen pubkey "${FIX_DIR}/mint.json")
SOURCE_OWNER=$(solana-keygen pubkey "${FIX_DIR}/source-owner.json")
DEST_OWNER=$(solana-keygen pubkey "${FIX_DIR}/dest-owner.json")
AUTHORITY=$(solana-keygen pubkey "${DEFAULT_KP}")

log "airdropping 2 SOL each to owners + authority (ignored if already funded)"
solana airdrop 2 "${SOURCE_OWNER}" -u "${RPC}" >/dev/null 2>&1 || true
solana airdrop 2 "${DEST_OWNER}"   -u "${RPC}" >/dev/null 2>&1 || true
solana airdrop 2 "${AUTHORITY}"    -u "${RPC}" >/dev/null 2>&1 || true

if ! spl-token display "${MINT}" --program-2022 --url "${RPC}" >/dev/null 2>&1; then
  log "creating Token-2022 mint ${MINT} (6 decimals)"
  spl-token create-token --program-2022 --decimals 6 \
    --mint-authority "${AUTHORITY}" --url "${RPC}" \
    "${FIX_DIR}/mint.json" >/dev/null
else
  log "reusing existing Token-2022 mint ${MINT}"
fi

create_ata() {
  local owner="$1" out_var="$2" ata
  ata=$(spl-token address --token "${MINT}" --program-2022 --owner "${owner}" \
          --url "${RPC}" --verbose 2>/dev/null \
          | awk '/^Associated token address:/ {print $4; exit}')
  [[ -n "${ata}" ]] || { echo "ERROR: could not derive ATA for ${owner}" >&2; exit 1; }
  if ! spl-token display "${ata}" --program-2022 --url "${RPC}" >/dev/null 2>&1; then
    log "creating Token-2022 ATA ${ata} for ${owner}"
    spl-token create-account "${MINT}" --program-2022 --owner "${owner}" \
      --fee-payer "${DEFAULT_KP}" --url "${RPC}" >/dev/null
  else
    log "reusing Token-2022 ATA ${ata}"
  fi
  printf -v "${out_var}" '%s' "${ata}"
}
create_ata "${SOURCE_OWNER}" SOURCE_ATA
create_ata "${DEST_OWNER}"   DEST_ATA

CUR=$(spl-token balance --address "${SOURCE_ATA}" --program-2022 --url "${RPC}" 2>/dev/null || echo 0)
CUR_INT="${CUR%.*}"
if [[ -z "${CUR_INT}" || "${CUR_INT}" -lt 1 ]]; then
  log "minting 1.0 token (=1_000_000 base units @ 6 decimals) to ${SOURCE_ATA}"
  spl-token mint "${MINT}" 1 "${SOURCE_ATA}" --program-2022 \
    --mint-authority "${DEFAULT_KP}" --url "${RPC}" >/dev/null
else
  log "source ATA already holds ${CUR}; skipping mint"
fi

cat >"${STATE_FILE}" <<JSON
{"mint":"${MINT}","source_ata":"${SOURCE_ATA}","source_owner":"${SOURCE_OWNER}","dest_ata":"${DEST_ATA}","dest_owner":"${DEST_OWNER}","authority":"${AUTHORITY}","rpc":"${RPC}"}
JSON
log "wrote ${STATE_FILE}"
log "SURFPOOL_READY ${RPC}"
