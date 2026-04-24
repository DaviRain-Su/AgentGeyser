/** M5b-F11 unit tests — wallet-agnostic, non-custodial. */
import { describe, expect, it, vi } from 'vitest';
import { signAndSend, SignAndSendError, type Signer, type Connection } from '../signAndSend.js';

const u8 = (x: ArrayLike<number> | number) => (typeof x === 'number' ? new Uint8Array(x) : Uint8Array.from(x));

/** Minimal v2 Transaction wire blob: 1 sig slot + 1 account + no ixs. */
function txBase64(addrBytes: Uint8Array): string {
  const parts = [u8([1]), u8(64), u8([1, 0, 0]), u8([1]), addrBytes, u8(32), u8([0])];
  const out = u8(parts.reduce((n, p) => n + p.length, 0));
  let o = 0; for (const p of parts) { out.set(p, o); o += p.length; }
  return Buffer.from(out).toString('base64');
}

const ADDR = '11111111111111111111111111111111';
const SIG = 'TestSignature1111111111111111111111111111111111111111111111111111';
const PK = u8(32).fill(7);
const mkSigner = (): Signer => ({
  address: ADDR,
  signTransactions: vi.fn(async () => [{ [ADDR]: u8(64).fill(0x42) }]),
});
const mkConn = (r: string | Error = SIG): Connection => ({
  sendTransaction: vi.fn(async () => { if (r instanceof Error) throw r; return r; }),
});

describe('signAndSend (F11)', () => {
  it('happy path: signs and returns the RPC signature', async () => {
    const signer = mkSigner(); const connection = mkConn(SIG);
    const result = await signAndSend({ unsignedTx: { tx: txBase64(PK) }, signer, connection });
    expect(result).toEqual({ signature: SIG });
    expect(signer.signTransactions).toHaveBeenCalledTimes(1);
    expect(connection.sendTransaction).toHaveBeenCalledTimes(1);
  });

  it('malformed base64 → SignAndSendError "Invalid unsigned transaction payload"', async () => {
    const err = await signAndSend({
      unsignedTx: { tx: '!!!not-valid-base64!!!' }, signer: mkSigner(), connection: mkConn(),
    }).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(SignAndSendError);
    expect((err as Error).message).toBe('Invalid unsigned transaction payload');
  });

  it('signer rejects → re-thrown with "Signer rejected transaction" prefix', async () => {
    const signer: Signer = {
      address: ADDR,
      signTransactions: vi.fn(async () => { throw new Error('user declined in wallet popup'); }),
    };
    const err = await signAndSend({ unsignedTx: { tx: txBase64(PK) }, signer, connection: mkConn() })
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(SignAndSendError);
    expect((err as Error).message).toMatch(/^Signer rejected transaction: /);
    expect((err as Error).message).toContain('user declined in wallet popup');
  });

  it('sendTransaction throws → wrapped as "sendTransaction failed" (truncated ≤200)', async () => {
    const rpcErr = new Error('401 Unauthorized: blockhash not found: ' + 'x'.repeat(400));
    const err = await signAndSend({ unsignedTx: { tx: txBase64(PK) }, signer: mkSigner(), connection: mkConn(rpcErr) })
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(SignAndSendError);
    const suffix = (err as Error).message.replace(/^sendTransaction failed: /, '');
    expect(suffix.length).toBeLessThanOrEqual(201);
    expect(suffix).toContain('401 Unauthorized');
  });
});
