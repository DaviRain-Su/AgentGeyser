/**
 * React context wiring for `@agentgeyser/react`.
 *
 * `AgentGeyserProvider` instantiates an `AgentGeyserClient` (from
 * `@agentgeyser/sdk`) once per `proxyUrl` (via `useMemo`) and publishes it on a
 * React context. `useAgentGeyser()` reads the client from that context and
 * throws a clear error when called outside a provider.
 *
 * Non-custodial: this module never touches key material. Signing lives in the
 * consumer's `@solana/wallet-adapter-react` wallet + `useInvokeSkill` hook.
 */

import { AgentGeyserClient } from '@agentgeyser/sdk';
import type { FetchLike } from '@agentgeyser/sdk';
import { createContext, useContext, useMemo, type ReactNode } from 'react';

const DEFAULT_PROXY_URL = 'http://127.0.0.1:8999';

const AgentGeyserContext = createContext<AgentGeyserClient | null>(null);

export interface AgentGeyserProviderProps {
  /** Optional override for the proxy JSON-RPC endpoint. */
  proxyUrl?: string;
  /** Optional fetch impl; falls back to global fetch inside the SDK. */
  fetch?: FetchLike;
  children: ReactNode;
}

export function AgentGeyserProvider(props: AgentGeyserProviderProps): JSX.Element {
  const { proxyUrl = DEFAULT_PROXY_URL, fetch, children } = props;
  const client = useMemo(
    () => new AgentGeyserClient({ proxyUrl, fetch }),
    [proxyUrl, fetch],
  );
  return (
    <AgentGeyserContext.Provider value={client}>{children}</AgentGeyserContext.Provider>
  );
}

export function useAgentGeyser(): AgentGeyserClient {
  const client = useContext(AgentGeyserContext);
  if (!client) {
    throw new Error(
      'useAgentGeyser() must be called inside <AgentGeyserProvider>.',
    );
  }
  return client;
}
