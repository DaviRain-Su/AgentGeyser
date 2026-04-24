/**
 * `signAndSend` — relay an AgentGeyser-proxy transaction blob to Solana.
 *
 * Two runtime branches via a discriminated union on the options:
 *  - Node ({@link SignAndSendNodeOptions}): dynamically imports `node:fs/promises`
 *    (never a top-level import), loads a JSON-array keypair file at the
 *    caller-supplied `keypairPath`, signs with `@solana/web3.js` v2 primitives,
 *    and posts the wire-encoded tx to the RPC.
 *  - Browser ({@link SignAndSendBrowserOptions}): never touches `fs` nor key
 *    material — the caller has already signed via a wallet adapter, and this
 *    helper simply relays `signedTransactionBase64` to the RPC.
 *
 * Non-custodial: this file MUST NOT construct v1-style secret-key instances or
 * reference any pass-phrase / seed-phrase identifier. The v2 helper
 * `createKeyPairSignerFromBytes` below only reads the caller-supplied file.
 */

import {
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  getBase64EncodedWireTransaction,
  getTransactionDecoder,
  type Signature,
  type Transaction,
} from '@solana/web3.js';
import type { AgentGeyserClient } from './client.js';
import { NetworkError, ValidationError } from './errors.js';

interface SignAndSendBase {
  client: AgentGeyserClient;
  transactionBase64: string;
  rpcUrl: string;
  /** Max poll time before returning `'pending'`. Default 30s. */
  confirmTimeoutMs?: number;
}

/** Node branch — read a JSON-array keypair file and sign locally. */
export interface SignAndSendNodeOptions extends SignAndSendBase {
  keypairPath: string;
  signedTransactionBase64?: undefined;
}

/** Browser branch — the caller (wallet adapter) has already signed. */
export interface SignAndSendBrowserOptions extends SignAndSendBase {
  signedTransactionBase64: string;
  keypairPath?: undefined;
}

export type SignAndSendOptions = SignAndSendNodeOptions | SignAndSendBrowserOptions;
export type ConfirmationState = 'confirmed' | 'finalized' | 'pending';
export interface SignAndSendResult { signature: string; confirmation: ConfirmationState; }

const DEFAULT_CONFIRM_TIMEOUT_MS = 30_000;
const POLL_INTERVAL_MS = 1_500;

/** Runtime detection so bundlers can drop the Node branch from browser builds. */
export function isNodeEnvironment(): boolean {
  return typeof process !== 'undefined' && typeof process.versions?.node === 'string';
}

function requireString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new ValidationError(`signAndSend: \`${field}\` must be a non-empty string`);
  }
  return value;
}

function base64ToBytes(b64: string): Uint8Array {
  if (typeof globalThis.Buffer !== 'undefined') {
    return new Uint8Array(globalThis.Buffer.from(b64, 'base64'));
  }
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Load a Solana JSON keypair file (array of 64 bytes) via dynamic fs import. */
async function loadKeypairBytes(path: string): Promise<Uint8Array> {
  const fs = await import('node:fs/promises');
  const raw = await fs.readFile(path, 'utf8');
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed) || parsed.some((n) => typeof n !== 'number')) {
      throw new Error('expected JSON array of numbers');
    }
    return Uint8Array.from(parsed as number[]);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    throw new ValidationError(`signAndSend: failed to parse keypair file: ${msg}`);
  }
}

/** Node branch: load keypair, decode+sign+encode the tx, and submit. */
async function signAndSendNode(opts: SignAndSendNodeOptions): Promise<SignAndSendResult> {
  requireString(opts.keypairPath, 'keypairPath');
  const transactionBase64 = requireString(opts.transactionBase64, 'transactionBase64');

  const bytes = await loadKeypairBytes(opts.keypairPath);
  const signer = await createKeyPairSignerFromBytes(bytes);
  const decoded = getTransactionDecoder().decode(base64ToBytes(transactionBase64)) as Transaction;

  const [newSignatures] = await signer.signTransactions([decoded]);
  if (!newSignatures) {
    throw new ValidationError('signAndSend: signer returned no signature dictionary');
  }
  const merged: Record<string, Uint8Array | null> = { ...decoded.signatures };
  for (const [addr, sig] of Object.entries(newSignatures)) if (sig) merged[addr] = sig;
  const signed = { messageBytes: decoded.messageBytes, signatures: merged } as unknown as Transaction;

  return submitWire(opts.rpcUrl, getBase64EncodedWireTransaction(signed), opts.confirmTimeoutMs);
}

/** Post the wire transaction and (best-effort) poll for confirmation. */
async function submitWire(
  rpcUrl: string,
  wireBase64: string,
  confirmTimeoutMs?: number,
): Promise<SignAndSendResult> {
  requireString(rpcUrl, 'rpcUrl');
  const rpc = createSolanaRpc(rpcUrl);
  let sig: Signature;
  try {
    // Localized cast: the v2 type brands base64 strings, but runtime accepts a plain string.
    sig = await rpc
      .sendTransaction(wireBase64 as unknown as Parameters<typeof rpc.sendTransaction>[0], {
        encoding: 'base64',
      })
      .send();
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    throw new NetworkError(`sendTransaction failed: ${msg}`);
  }
  const confirmation = await pollConfirmation(rpc, sig, confirmTimeoutMs);
  return { signature: String(sig), confirmation };
}

async function pollConfirmation(
  rpc: ReturnType<typeof createSolanaRpc>,
  sig: Signature,
  timeoutMs: number = DEFAULT_CONFIRM_TIMEOUT_MS,
): Promise<ConfirmationState> {
  const deadline = Date.now() + Math.max(0, timeoutMs);
  while (Date.now() < deadline) {
    try {
      const resp = await rpc.getSignatureStatuses([sig]).send();
      const status = resp.value[0]?.confirmationStatus;
      if (status === 'confirmed' || status === 'finalized') return status;
    } catch { /* transient — bounded by deadline */ }
    await new Promise<void>((r) => setTimeout(r, POLL_INTERVAL_MS));
  }
  return 'pending';
}

/**
 * Sign (or relay a pre-signed) proxy-built transaction blob and submit it.
 * See {@link SignAndSendNodeOptions} / {@link SignAndSendBrowserOptions}.
 */
export async function signAndSend(opts: SignAndSendOptions): Promise<SignAndSendResult> {
  if (!opts || typeof opts !== 'object') {
    throw new ValidationError('signAndSend: options object required');
  }
  if ('signedTransactionBase64' in opts && typeof opts.signedTransactionBase64 === 'string') {
    requireString(opts.signedTransactionBase64, 'signedTransactionBase64');
    return submitWire(opts.rpcUrl, opts.signedTransactionBase64, opts.confirmTimeoutMs);
  }
  if ('keypairPath' in opts && typeof opts.keypairPath === 'string') {
    if (!isNodeEnvironment()) {
      throw new ValidationError('signAndSend: `keypairPath` requires a Node runtime');
    }
    return signAndSendNode(opts);
  }
  throw new ValidationError(
    'signAndSend: options must include either `keypairPath` (Node) or `signedTransactionBase64` (Browser)',
  );
}
