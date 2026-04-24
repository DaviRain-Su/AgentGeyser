/**
 * `useInvokeSkillWithWallet()` — mutation hook taking a caller-supplied wallet.
 * `useInvokeSkill()` (in useInvokeSkillWithAdapter.ts) wraps it with wallet-adapter.
 * Non-custodial: only signing surface is `wallet.signTransaction(tx)`.
 */
import { useCallback, useState } from 'react';
import { createSolanaRpc, getBase64EncodedWireTransaction, getTransactionDecoder } from '@solana/web3.js';
import type { InvokeSkillRequest } from '@agentgeyser/sdk';
import { useAgentGeyser } from './context.js';

export interface InvokeSkillOutcome { signature: string }
export interface UseInvokeSkillOptions { rpcUrl?: string }
export interface UseInvokeSkillResult {
  mutate: (req: InvokeSkillRequest) => Promise<InvokeSkillOutcome>;
  data: InvokeSkillOutcome | undefined;
  loading: boolean;
  error: Error | undefined;
}
export type UseInvokeSkillWallet = { signTransaction?: (tx: unknown) => Promise<unknown> } & Record<string, unknown>;

const DEFAULT_RPC_URL = 'http://127.0.0.1:8899';
const toError = (v: unknown): Error => (v instanceof Error ? v : new Error(String(v)));
function b64ToBytes(b64: string): Uint8Array {
  if (typeof globalThis.Buffer !== 'undefined') return new Uint8Array(globalThis.Buffer.from(b64, 'base64'));
  const bin = atob(b64); const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

export function useInvokeSkillWithWallet(
  wallet: UseInvokeSkillWallet,
  options: UseInvokeSkillOptions = {},
): UseInvokeSkillResult {
  const client = useAgentGeyser();
  const rpcUrl = options.rpcUrl ?? DEFAULT_RPC_URL;
  const [data, setData] = useState<InvokeSkillOutcome | undefined>(undefined);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | undefined>(undefined);

  const mutate = useCallback(async (req: InvokeSkillRequest): Promise<InvokeSkillOutcome> => {
    const signTx = wallet.signTransaction;
    if (!signTx) throw new Error('useInvokeSkill: wallet does not support signTransaction');
    setLoading(true); setError(undefined);
    try {
      const { transactionBase64 } = await client.invokeSkill(req);
      const tx = getTransactionDecoder().decode(b64ToBytes(transactionBase64));
      const signed = await (signTx as (t: unknown) => Promise<unknown>)(tx);
      const wire = getBase64EncodedWireTransaction(signed as Parameters<typeof getBase64EncodedWireTransaction>[0]);
      const rpc = createSolanaRpc(rpcUrl);
      const sig = await rpc.sendTransaction(wire as unknown as Parameters<typeof rpc.sendTransaction>[0], { encoding: 'base64' }).send();
      const outcome: InvokeSkillOutcome = { signature: String(sig) };
      setData(outcome); setLoading(false);
      return outcome;
    } catch (err) {
      const e = toError(err); setError(e); setLoading(false); throw e;
    }
  }, [client, wallet, rpcUrl]);

  return { mutate, data, loading, error };
}
