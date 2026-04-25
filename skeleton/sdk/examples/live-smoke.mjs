// SPDX: observation-only; M4-V2 no-commit live smoke.
// Drives the built @agentgeyser/sdk against a user-managed surfpool (8899) +
// AgentGeyser proxy (8999). Prints SKILLS_OK / TX_LEN=<n> / SIG=<base58> on success.
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AgentGeyserClient, signAndSend } from '../packages/sdk/dist/index.js';

const proxyUrl = process.env.AGENTGEYSER_PROXY_URL ?? 'http://127.0.0.1:8999';
const rpcUrl = process.env.AGENTGEYSER_RPC_URL ?? 'http://127.0.0.1:8899';
const here = dirname(fileURLToPath(import.meta.url));
const requireFromSdk = createRequire(resolve(here, '../packages/sdk/package.json'));
const { createKeyPairSignerFromBytes, createSolanaRpc } = await import(requireFromSdk.resolve('@solana/web3.js'));
const fallbackKeypairPath = resolve(here, '../../../mission-fixtures/source-owner.json');
const keypairPath = process.env.AGENTGEYSER_KEYPAIR_PATH && existsSync(process.env.AGENTGEYSER_KEYPAIR_PATH)
  ? process.env.AGENTGEYSER_KEYPAIR_PATH
  : existsSync(fallbackKeypairPath) ? fallbackKeypairPath : undefined;
if (!keypairPath) {
  throw new Error(
    `live-smoke: keypair not found. Set AGENTGEYSER_KEYPAIR_PATH or place source-owner.json at ${fallbackKeypairPath}`,
  );
}

function readKeypairBytes(path) {
  const arr = JSON.parse(readFileSync(path, 'utf8').trim());
  if (!Array.isArray(arr) || arr.length !== 64) {
    throw new Error(`live-smoke: expected 64-byte keypair array at ${path}`);
  }
  return Uint8Array.from(arr);
}

const statePath = resolve(here, '../packages/sdk/test-devnet/.surfpool-state.json');
const state = existsSync(statePath) ? JSON.parse(readFileSync(statePath, 'utf8')) : {};
const SRC_ATA = process.env.AGENTGEYSER_DEVNET_SRC_ATA ?? process.env.SRC_ATA ?? state.source_ata;
const DST_ATA = process.env.AGENTGEYSER_DEVNET_DST_ATA ?? process.env.DST_ATA ?? state.dest_ata;
const SRC_OWNER = process.env.AGENTGEYSER_DEVNET_SRC_OWNER ?? process.env.SRC_OWNER ?? state.source_owner;
const MINT = process.env.AGENTGEYSER_DEVNET_MINT ?? process.env.MINT ?? state.mint;

if (!SRC_ATA || !DST_ATA || !SRC_OWNER || !MINT) {
  console.error(`live-smoke: AGENTGEYSER_DEVNET_MINT/SRC_ATA/DST_ATA/SRC_OWNER env vars or ${statePath} are required`);
  process.exit(2);
}

const client = new AgentGeyserClient({ proxyUrl });

// V2.3 — listSkills must contain spl-token::transfer
const skills = await client.listSkills();
const hit = skills.find(
  (s) => s.skillId === 'spl-token::transfer',
);
if (!hit) {
  console.log('SKILLS_FAIL');
  process.exit(1);
}
console.log('SKILLS_OK');

// V2.4 — invokeSkill returns transactionBase64 (≥ 100 chars, base64)
const invokeResp = await client.invokeSkill({
  skill_id: 'spl-token::transfer',
  args: { source_ata: SRC_ATA, destination_ata: DST_ATA, owner: SRC_OWNER, amount: 1, mint: MINT, decimals: 6 },
  accounts: {},
  payer: SRC_OWNER,
});
const transactionBase64 = invokeResp.transactionBase64;
if (
  !transactionBase64 ||
  transactionBase64.length < 100 ||
  !/^[A-Za-z0-9+/=]+$/.test(transactionBase64)
) {
  console.log('TX_FAIL');
  process.exit(1);
}
console.log(`TX_LEN=${transactionBase64.length}`);

// V2.5 — signAndSend captures a base58 signature
const raw = invokeResp;
const signer = await createKeyPairSignerFromBytes(readKeypairBytes(keypairPath));
const rpc = createSolanaRpc(rpcUrl);
const connection = {
  async sendTransaction(wireB64) {
    return rpc.sendTransaction(wireB64, {
      encoding: 'base64',
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    }).send();
  },
};
const { signature } = await signAndSend({ unsignedTx: { tx: transactionBase64, message: raw.message, recent_blockhash: raw.recent_blockhash }, signer, connection });
if (!/^[1-9A-HJ-NP-Za-km-z]{64,90}$/.test(signature)) {
  console.log('SIG_FAIL');
  process.exit(1);
}
console.log(`SIG=${signature}`);
