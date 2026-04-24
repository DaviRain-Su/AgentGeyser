/**
 * `useSkills()` — read-only hook that fetches the skill registry via the SDK
 * client published by {@link AgentGeyserProvider}.
 *
 * Returns `{ data, loading, error, refetch }`. Uses a `cancelled` flag in the
 * `useEffect` cleanup to avoid setState after unmount, and memoizes the
 * in-flight promise in a ref so a double render (StrictMode) does not trigger
 * a second network call.
 *
 * Non-custodial: this hook never touches key material; it simply calls
 * `client.listSkills()` on the SDK client from context.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import type { Skill } from '@agentgeyser/sdk';
import { useAgentGeyser } from './context.js';

export interface UseSkillsResult {
  data: Skill[] | undefined;
  loading: boolean;
  error: Error | undefined;
  refetch: () => Promise<void>;
}

interface State {
  data: Skill[] | undefined;
  loading: boolean;
  error: Error | undefined;
}

const INITIAL: State = { data: undefined, loading: true, error: undefined };
const toError = (v: unknown): Error =>
  v instanceof Error ? v : new Error(String(v));

export function useSkills(): UseSkillsResult {
  const client = useAgentGeyser();
  const [state, setState] = useState<State>(INITIAL);
  const inFlight = useRef<Promise<Skill[]> | null>(null);
  const cancelled = useRef(false);

  const run = useCallback(async (): Promise<void> => {
    let promise = inFlight.current;
    if (!promise) {
      promise = client.listSkills();
      inFlight.current = promise;
    }
    try {
      const data = await promise;
      if (!cancelled.current) setState({ data, loading: false, error: undefined });
    } catch (err) {
      if (!cancelled.current) {
        setState({ data: undefined, loading: false, error: toError(err) });
      }
    } finally {
      if (inFlight.current === promise) inFlight.current = null;
    }
  }, [client]);

  useEffect(() => {
    cancelled.current = false;
    setState(INITIAL);
    void run();
    return () => { cancelled.current = true; };
  }, [run]);

  const refetch = useCallback(async (): Promise<void> => {
    setState((prev) => ({ ...prev, loading: true, error: undefined }));
    inFlight.current = null;
    await run();
  }, [run]);

  return { data: state.data, loading: state.loading, error: state.error, refetch };
}
