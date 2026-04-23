/**
 * examples/sign-and-send.ts — sign + broadcast an unsigned base64 Solana TX.
 * Reads AGENTGEYSER_DEMO_KEYPAIR (JSON-array file path OR base58 secret) and
 * AGENTGEYSER_RPC_URL (default https://api.devnet.solana.com). Module export
 * `signAndSend` is reused by live-smoke.ts. Keypair material is never printed.
 */

import { Connection, Keypair, Transaction } from '@solana/web3.js';
import { readFileSync } from 'node:fs';

const DEFAULT_RPC_URL = 'https://api.devnet.solana.com';
const DEFAULT_TIMEOUT_MS = 60_000;
const POLL_MS = 1500;
const B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

export interface SignAndSendOpts {
  rpcUrl?: string;
  keypairSource?: string;
  timeoutMs?: number;
}

export interface SignAndSendResult {
  signature: string;
  confirmed: boolean;
}

function base58Decode(s: string): Uint8Array {
  let num = 0n;
  for (const ch of s) {
    const v = B58.indexOf(ch);
    if (v < 0) throw new Error('invalid base58');
    num = num * 58n + BigInt(v);
  }
  const out: number[] = [];
  while (num > 0n) { out.push(Number(num & 0xffn)); num >>= 8n; }
  for (const ch of s) { if (ch !== '1') break; out.push(0); }
  return Uint8Array.from(out.reverse());
}

function loadKeypair(src: string): Keypair {
  const t = src.trim();
  if (t.startsWith('[')) {
    return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(t) as number[]));
  }
  try {
    const raw = readFileSync(t, 'utf8').trim();
    if (raw.startsWith('[')) {
      return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(raw) as number[]));
    }
    return Keypair.fromSecretKey(base58Decode(raw));
  } catch {
    return Keypair.fromSecretKey(base58Decode(t));
  }
}

const sleep = (ms: number): Promise<void> =>
  new Promise((r) => setTimeout(r, ms));

export async function signAndSend(
  unsignedTxB64: string,
  opts: SignAndSendOpts = {},
): Promise<SignAndSendResult> {
  const rpcUrl = opts.rpcUrl ?? process.env.AGENTGEYSER_RPC_URL ?? DEFAULT_RPC_URL;
  const kpSrc = opts.keypairSource ?? process.env.AGENTGEYSER_DEMO_KEYPAIR;
  if (!kpSrc) throw new Error('AGENTGEYSER_DEMO_KEYPAIR not set');
  let kp: Keypair;
  try { kp = loadKeypair(kpSrc); }
  catch { throw new Error('failed to load keypair: <redacted>'); }
  const conn = new Connection(rpcUrl, 'confirmed');
  const tx = Transaction.from(Buffer.from(unsignedTxB64, 'base64'));
  tx.partialSign(kp);
  const sig = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: false });
  const start = Date.now();
  const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  while (Date.now() - start < timeoutMs) {
    const cs = (await conn.getSignatureStatus(sig)).value?.confirmationStatus;
    if (cs === 'confirmed' || cs === 'finalized') {
      return { signature: sig, confirmed: true };
    }
    await sleep(POLL_MS);
  }
  return { signature: sig, confirmed: false };
}

function usage(): void {
  console.log([
    'Usage: tsx examples/sign-and-send.ts [--help] [--tx-file <path>]',
    '',
    '  Signs a base64 unsigned Solana TX (--tx-file or stdin) and broadcasts.',
    '',
    'Env: AGENTGEYSER_DEMO_KEYPAIR (JSON file OR base58 secret),',
    `     AGENTGEYSER_RPC_URL (default ${DEFAULT_RPC_URL}).`,
  ].join('\n'));
}

async function readStdin(): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(typeof chunk === 'string' ? Buffer.from(chunk) : chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

async function cliMain(): Promise<void> {
  const argv = process.argv.slice(2);
  if (argv.includes('--help') || argv.includes('-h')) { usage(); return; }
  const i = argv.indexOf('--tx-file');
  let txB64: string;
  if (i !== -1) {
    const p = argv[i + 1];
    if (!p) throw new Error('--tx-file requires a path');
    txB64 = readFileSync(p, 'utf8').trim();
  } else {
    txB64 = (await readStdin()).trim();
  }
  if (!txB64) throw new Error('no transaction bytes provided');
  const res = await signAndSend(txB64);
  console.log(JSON.stringify({
    signature: res.signature,
    confirmed: res.confirmed,
    explorer: `https://explorer.solana.com/tx/${res.signature}?cluster=devnet`,
  }));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  cliMain().catch((err: unknown) => {
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[sign-and-send] ${msg}\n`);
    process.exit(1);
  });
}
