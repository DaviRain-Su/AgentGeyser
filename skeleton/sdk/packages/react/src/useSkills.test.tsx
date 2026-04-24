import { act, render, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { FetchLike, Skill } from '@agentgeyser/sdk';
import { AgentGeyserProvider } from './context.js';
import { useSkills } from './useSkills.js';

type Captured = { current: ReturnType<typeof useSkills> | null };

function envelope(result: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: 'OK',
    text: async () => '',
    json: async () => ({ jsonrpc: '2.0', id: 1, result }),
  };
}

function fakeFetch(result: unknown): ReturnType<typeof vi.fn> {
  return vi.fn(async () => envelope(result));
}

function deferredFetch<T>() {
  let resolve!: (v: T) => void;
  const pending = new Promise<T>((r) => { resolve = r; });
  const fetchImpl = vi.fn(async () => envelope(await pending));
  return { fetchImpl, resolve };
}

function Harness({ captured }: { captured: Captured }): null {
  captured.current = useSkills();
  return null;
}

function renderWithFetch(fetchImpl: ReturnType<typeof vi.fn>): Captured {
  const captured: Captured = { current: null };
  render(
    <AgentGeyserProvider
      proxyUrl="http://127.0.0.1:8999"
      fetch={fetchImpl as unknown as FetchLike}
    >
      <Harness captured={captured} />
    </AgentGeyserProvider>,
  );
  return captured;
}

describe('useSkills', () => {
  it('reports loading:true on first render before resolve', async () => {
    const { fetchImpl, resolve } = deferredFetch<Skill[]>();
    const captured = renderWithFetch(fetchImpl);
    expect(captured.current?.loading).toBe(true);
    expect(captured.current?.data).toBeUndefined();
    expect(captured.current?.error).toBeUndefined();
    await act(async () => { resolve([]); });
  });

  it('populates data and flips loading:false after resolve', async () => {
    const skills: Skill[] = [
      { id: 'spl-token::transfer', name: 'Transfer', description: 'Move tokens' },
    ];
    const fetchImpl = fakeFetch(skills);
    const captured = renderWithFetch(fetchImpl);
    await waitFor(() => expect(captured.current?.loading).toBe(false));
    expect(captured.current?.data).toEqual(skills);
    expect(captured.current?.error).toBeUndefined();
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('populates error when the client throws', async () => {
    const fetchImpl = vi.fn(async () => { throw new Error('boom'); });
    const captured = renderWithFetch(fetchImpl);
    await waitFor(() => expect(captured.current?.loading).toBe(false));
    expect(captured.current?.error).toBeInstanceOf(Error);
    expect(captured.current?.data).toBeUndefined();
  });
});
