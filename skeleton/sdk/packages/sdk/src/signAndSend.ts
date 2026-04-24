/** M5b-F11 — non-custodial `signAndSend`. No filesystem, no key material. */
import {
  getBase64EncodedWireTransaction,
  getTransactionDecoder,
  type Transaction,
} from '@solana/web3.js';

export interface UnsignedTxPayload { tx: string; message?: string; recent_blockhash?: string; }
export interface Signer {
  readonly address: string;
  signTransactions(
    txs: ReadonlyArray<Transaction>,
  ): Promise<ReadonlyArray<Readonly<Record<string, Uint8Array>>>>;
}
export interface Connection { sendTransaction(wireTransactionBase64: string): Promise<string>; }

export class SignAndSendError extends Error {
  constructor(message: string, public readonly cause?: unknown) {
    super(message);
    this.name = 'SignAndSendError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

function b64ToBytes(b64: string): Uint8Array {
  if (typeof globalThis.Buffer !== 'undefined') return new Uint8Array(globalThis.Buffer.from(b64, 'base64'));
  const bin = atob(b64); const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
const trunc = (m: string, max = 200) => (m.length > max ? `${m.slice(0, max)}…` : m);
const msgOf = (e: unknown) => (e instanceof Error ? e.message : String(e));

export async function signAndSend(params: {
  unsignedTx: UnsignedTxPayload; signer: Signer; connection: Connection;
}): Promise<{ signature: string }> {
  const { unsignedTx, signer, connection } = params;

  let decoded: Transaction;
  try {
    const bytes = b64ToBytes(unsignedTx.tx);
    if (bytes.length === 0) throw new Error('empty tx bytes');
    decoded = getTransactionDecoder().decode(bytes) as Transaction;
  } catch (err) {
    throw new SignAndSendError('Invalid unsigned transaction payload', err);
  }

  let signed: Transaction;
  try {
    const [newSigs] = await signer.signTransactions([decoded]);
    if (!newSigs) throw new Error('signer returned no signature dictionary');
    const merged: Record<string, Uint8Array | null> = { ...decoded.signatures };
    for (const [addr, sig] of Object.entries(newSigs)) if (sig) merged[addr] = sig;
    signed = { messageBytes: decoded.messageBytes, signatures: merged } as unknown as Transaction;
  } catch (err) {
    throw new SignAndSendError(`Signer rejected transaction: ${trunc(msgOf(err))}`, err);
  }

  try {
    const signature = await connection.sendTransaction(getBase64EncodedWireTransaction(signed));
    return { signature };
  } catch (err) {
    throw new SignAndSendError(`sendTransaction failed: ${trunc(msgOf(err))}`, err);
  }
}
