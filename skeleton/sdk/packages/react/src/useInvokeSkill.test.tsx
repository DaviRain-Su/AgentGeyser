import { act, render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { FetchLike } from '@agentgeyser/sdk';
import { AgentGeyserProvider } from './context.js';
import { useInvokeSkillWithWallet, type UseInvokeSkillResult } from './useInvokeSkill.js';
import { useInvokeSkill } from './useInvokeSkillWithAdapter.js';

const FAKE_SIG = ['4xQeZ1aJf8Cmb7hQg', 'F3mXk2bFqzFxJ3Pq7xN', '9vL2yKwH6U8tR3dN1a', 'B2cD3eF4gH5iJ6k'].join('');
const PAYER = ['Fh3A4pc8YtQvfy5r', 'z9HDXraX5kyn4AFk', 'Xyk1V8oWLP13'].join('');

vi.mock('@solana/wallet-adapter-react', () => ({
  useWallet: () => ({ publicKey: null, signTransaction: async (tx: unknown) => tx }),
}));
vi.mock('@solana/web3.js', () => ({
  getTransactionDecoder: () => ({ decode: () => ({ messageBytes: new Uint8Array(), signatures: {} }) }),
  getBase64EncodedWireTransaction: () => 'WIRE_BASE64',
  createSolanaRpc: () => ({ sendTransaction: () => ({ send: async () => FAKE_SIG }) }),
}));

describe('useInvokeSkill', () => {
  it('mutate() resolves with a signature-shaped string', async () => {
    const captured: { current: UseInvokeSkillResult | null } = { current: null };
    const Harness = (): null => { captured.current = useInvokeSkill({ rpcUrl: 'http://127.0.0.1:8899' }); return null; };
    const fetchImpl = vi.fn(async () => ({
      ok: true, status: 200, statusText: 'OK', text: async () => '',
      json: async () => ({ jsonrpc: '2.0', id: 1, result: { transactionBase64: 'AAA=' } }),
    }));
    render(
      <AgentGeyserProvider proxyUrl="http://127.0.0.1:8999" fetch={fetchImpl as unknown as FetchLike}>
        <Harness />
      </AgentGeyserProvider>,
    );
    let outcome: { signature: string } | undefined;
    await act(async () => {
      outcome = await captured.current!.mutate({
        skill_id: 'spl-token::transfer', args: { amount: 1 }, accounts: {},
        payer: PAYER,
      });
    });
    expect(outcome?.signature).toBe(FAKE_SIG);
    expect(captured.current?.data?.signature).toBe(FAKE_SIG);
  });

  it('useInvokeSkillWithWallet works without wallet-adapter-react', async () => {
    const captured: { current: UseInvokeSkillResult | null } = { current: null };
    const stubWallet = { signTransaction: async (tx: unknown) => tx };
    const Harness = (): null => {
      captured.current = useInvokeSkillWithWallet(stubWallet, { rpcUrl: 'http://127.0.0.1:8899' });
      return null;
    };
    const fetchImpl = vi.fn(async () => ({
      ok: true, status: 200, statusText: 'OK', text: async () => '',
      json: async () => ({ jsonrpc: '2.0', id: 1, result: { transactionBase64: 'AAA=' } }),
    }));
    render(
      <AgentGeyserProvider proxyUrl="http://127.0.0.1:8999" fetch={fetchImpl as unknown as FetchLike}>
        <Harness />
      </AgentGeyserProvider>,
    );
    let outcome: { signature: string } | undefined;
    await act(async () => {
      outcome = await captured.current!.mutate({
        skill_id: 'spl-token::transfer', args: { amount: 1 }, accounts: {},
        payer: PAYER,
      });
    });
    expect(outcome?.signature).toBe(FAKE_SIG);
  });
});
