#!/usr/bin/env bash
set -euo pipefail
# Idempotent surfpool demo state seeder for MVP-M2-Verify.
# Targets surfpool RPC at http://127.0.0.1:8899 (managed by services.yaml).
# Writes pubkey scratchpad to skeleton/examples/.surfpool-state.json.

RPC="http://127.0.0.1:8899"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIX_DIR="${REPO_ROOT}/mission-fixtures"
STATE_FILE="${REPO_ROOT}/skeleton/examples/.surfpool-state.json"
DEFAULT_KP="${HOME}/.config/solana/id.json"
log() { printf '[setup-surfpool] %s\n' "$*"; }

log "probing surfpool at ${RPC}"
if ! solana cluster-version -u "${RPC}" >/dev/null 2>&1; then
  echo "ERROR: surfpool not reachable at ${RPC}. Start it via services.yaml." >&2
  exit 1
fi
mkdir -p "${FIX_DIR}"

ensure_kp() {
  if [[ ! -f "$1" ]]; then
    log "generating keypair $1"
    solana-keygen new --outfile "$1" --no-bip39-passphrase --force --silent >/dev/null
  else
    log "reusing keypair $1"
  fi
}
ensure_kp "${FIX_DIR}/mint.json"
ensure_kp "${FIX_DIR}/source-owner.json"
ensure_kp "${FIX_DIR}/dest-owner.json"

MINT=$(solana-keygen pubkey "${FIX_DIR}/mint.json")
SOURCE_OWNER=$(solana-keygen pubkey "${FIX_DIR}/source-owner.json")
DEST_OWNER=$(solana-keygen pubkey "${FIX_DIR}/dest-owner.json")
AUTHORITY=$(solana-keygen pubkey "${DEFAULT_KP}")

log "airdropping 2 SOL to owners (errors ignored if already funded)"
solana airdrop 2 "${SOURCE_OWNER}" -u "${RPC}" >/dev/null 2>&1 || true
solana airdrop 2 "${DEST_OWNER}"   -u "${RPC}" >/dev/null 2>&1 || true

if ! spl-token display "${MINT}" --url "${RPC}" >/dev/null 2>&1; then
  log "creating SPL-Token mint ${MINT}"
  spl-token create-token --mint-authority "${AUTHORITY}" --url "${RPC}" "${FIX_DIR}/mint.json" >/dev/null
else
  log "reusing existing mint ${MINT}"
fi

create_ata() {
  local owner="$1" out_var="$2" ata
  ata=$(spl-token address --token "${MINT}" --owner "${owner}" --url "${RPC}" --verbose 2>/dev/null \
    | awk '/^Associated token address:/ {print $4; exit}')
  [[ -n "${ata}" ]] || { echo "ERROR: could not derive ATA for ${owner}" >&2; exit 1; }
  if spl-token display "${ata}" --url "${RPC}" >/dev/null 2>&1; then
    log "reusing ATA ${ata} for ${owner}"
  else
    log "creating ATA ${ata} for ${owner}"
    spl-token create-account "${MINT}" --owner "${owner}" --fee-payer "${DEFAULT_KP}" --url "${RPC}" >/dev/null
  fi
  printf -v "${out_var}" '%s' "${ata}"
}
create_ata "${SOURCE_OWNER}" SOURCE_ATA
create_ata "${DEST_OWNER}"   DEST_ATA

CUR=$(spl-token balance --address "${SOURCE_ATA}" --url "${RPC}" 2>/dev/null || echo 0)
if [[ "${CUR%.*}" -lt 1000 ]]; then
  log "minting 1000 tokens to ${SOURCE_ATA}"
  spl-token mint "${MINT}" 1000 "${SOURCE_ATA}" --mint-authority "${DEFAULT_KP}" --url "${RPC}" >/dev/null
else
  log "source ATA already holds ${CUR}; skipping mint"
fi

mkdir -p "$(dirname "${STATE_FILE}")"
cat >"${STATE_FILE}" <<JSON
{"mint":"${MINT}","source":"${SOURCE_ATA}","source_owner":"${SOURCE_OWNER}","destination":"${DEST_ATA}","dest_owner":"${DEST_OWNER}","authority":"${AUTHORITY}"}
JSON
log "wrote ${STATE_FILE}"
log "done."
