import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { AgentGeyserProvider } from '../../src/context.js';
import { useInvokeSkillWithWallet } from '../../src/useInvokeSkill.js';

declare const __AG_SRC_ATA__: string;
declare const __AG_DST_ATA__: string;
declare const __AG_SRC_OWNER__: string;

const FAKE_SIG = new Uint8Array(64).fill(7);
const stubWallet = {
  signTransaction: async (tx: unknown) => {
    const t = tx as { messageBytes: Uint8Array; signatures: Record<string, Uint8Array> };
    return { ...t, signatures: { ...(t.signatures ?? {}), __demo__: FAKE_SIG } };
  },
};

function App(): JSX.Element {
  const { mutate, data, error } = useInvokeSkillWithWallet(stubWallet, { rpcUrl: '/stub-rpc' });
  const [clicked, setClicked] = useState(false);
  const onClick = async (): Promise<void> => {
    setClicked(true);
    try {
      await mutate({
        skill_id: 'spl-token::transfer', args: { amount: 1 },
        accounts: {
          source: __AG_SRC_ATA__,
          destination: __AG_DST_ATA__,
          authority: __AG_SRC_OWNER__,
        },
        payer: __AG_SRC_OWNER__,
      });
    } catch { /* surfaced via hook.error */ }
  };
  return (
    <div>
      <button data-testid="invoke" onClick={onClick}>invoke</button>
      {data && <p data-testid="signature">{data.signature}</p>}
      {error && clicked && <p data-testid="error">{error.message}</p>}
    </div>
  );
}

const boundFetch = ((...args: Parameters<typeof fetch>) => fetch(...args)) as typeof fetch;

createRoot(document.getElementById('root') as HTMLElement).render(
  <AgentGeyserProvider proxyUrl="/ag-proxy" fetch={boundFetch}><App /></AgentGeyserProvider>,
);
