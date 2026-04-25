/**
 * M5b-F12 — end-to-end SPL-Token-2022 transfer harness.
 *
 * Pivoted from public devnet to **surfpool** (local mainnet-beta fork)
 * because canonical fixtures are surfpool-only and devnet USDC-2022
 * faucet provisioning is out of scope. The signature produced here is a
 * REAL Solana-format ledger signature (validated by solana-cli), but is
 * not publicly verifiable via explorer.solana.com.
 *
 * Flow:
 *   1. Build unsigned tx via proxy `ag_invokeSkill("spl-token::transfer")`
 *   2. Sign with source-owner keypair (loaded from fixture JSON; disk load
 *      is allowed in the harness per F11 — library stays non-custodial)
 *   3. Submit via surfpool RPC, write signature to /tmp/m5b-devnet-sig.txt
 *
 * All addresses arrive via env vars (VX.4: no base58 literals in source).
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { createSolanaRpc, createKeyPairSignerFromBytes } from '@solana/web3.js';
import { AgentGeyserClient, signAndSend, type Connection } from '../src/index.js';

const SIG_OUT = '/tmp/m5b-devnet-sig.txt';

function env(name: string, fallback?: string): string {
  const v = process.env[name] ?? fallback;
  if (!v) throw new Error(`missing required env: ${name}`);
  return v;
}

function loadKeypairBytes(path: string): Uint8Array {
  const raw = readFileSync(path, 'utf8').trim();
  const arr = JSON.parse(raw) as number[];
  if (!Array.isArray(arr) || arr.length !== 64) {
    throw new Error(`expected 64-byte keypair array at ${path}`);
  }
  return Uint8Array.from(arr);
}

async function main(): Promise<void> {
  const proxyUrl = env('AGENTGEYSER_PROXY_URL');
  const rpcUrl = env('AGENTGEYSER_RPC_URL');
  const srcOwner = env('AGENTGEYSER_DEVNET_SRC_OWNER');
  const dstOwner = env('AGENTGEYSER_DEVNET_DST_OWNER');
  const srcAta = env('AGENTGEYSER_DEVNET_SRC_ATA');
  const dstAta = env('AGENTGEYSER_DEVNET_DST_ATA');
  const mint = env('AGENTGEYSER_DEVNET_MINT');
  const amount = Number(env('AGENTGEYSER_DEVNET_AMOUNT', '10000'));
  const keypairPath = env('AGENTGEYSER_DEVNET_KEYPAIR');

  const client = new AgentGeyserClient({ proxyUrl });
  const resp = await client.invokeSkill({
    skill_id: 'spl-token::transfer',
    payer: srcOwner,
    args: {
      source_ata: srcAta,
      destination_ata: dstAta,
      owner: srcOwner,
      amount,
      mint,
      decimals: 6,
    },
    accounts: {},
  });
  const raw = resp as unknown as Record<string, unknown>;
  const tx = resp.transactionBase64;
  const message = typeof raw.message === 'string' ? (raw.message as string) : undefined;
  const recentBlockhash = typeof raw.recent_blockhash === 'string'
    ? (raw.recent_blockhash as string)
    : typeof raw.recentBlockhash === 'string' ? (raw.recentBlockhash as string) : undefined;

  // Non-custodial library invariant preserved: keypair bytes live only in
  // this harness process and never cross the SDK public surface.
  const signer = await createKeyPairSignerFromBytes(loadKeypairBytes(keypairPath));
  if (signer.address !== srcOwner) {
    throw new Error(`signer address ${signer.address} ≠ expected ${srcOwner}`);
  }

  const rpc = createSolanaRpc(rpcUrl);
  const connection: Connection = {
    async sendTransaction(wireB64: string): Promise<string> {
      return rpc
        .sendTransaction(wireB64 as unknown as Parameters<typeof rpc.sendTransaction>[0], {
          encoding: 'base64',
          skipPreflight: false,
          preflightCommitment: 'confirmed',
        })
        .send();
    },
  };

  const { signature } = await signAndSend({
    unsignedTx: { tx, message, recent_blockhash: recentBlockhash },
    signer,
    connection,
  });

  writeFileSync(SIG_OUT, `${signature}\n`);
  process.stdout.write(`SIGNATURE ${signature}\n`);
  process.stdout.write(`WROTE ${SIG_OUT}\n`);
  // Record destination ownership proof alongside sig (used by run.sh to
  // call `solana confirm` while surfpool is still alive).
  process.stdout.write(`RPC_URL ${rpcUrl}\n`);
}

main().catch((err: unknown) => {
  const msg = err instanceof Error ? err.stack ?? err.message : String(err);
  process.stderr.write(`[transfer.e2e] ${msg}\n`);
  process.exit(1);
});
