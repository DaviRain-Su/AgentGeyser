import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AgentGeyserProvider, useAgentGeyser } from './context.js';

describe('AgentGeyserProvider', () => {
  it('exposes a non-null AgentGeyserClient to a consumer', () => {
    let captured: unknown = null;
    function Consumer(): null {
      captured = useAgentGeyser();
      return null;
    }
    render(
      <AgentGeyserProvider proxyUrl="http://127.0.0.1:8999">
        <Consumer />
      </AgentGeyserProvider>,
    );
    expect(captured).not.toBeNull();
    expect(typeof (captured as { listSkills?: unknown }).listSkills).toBe(
      'function',
    );
  });

  it('throws a clear error when useAgentGeyser is used without a provider', () => {
    function Consumer(): null {
      useAgentGeyser();
      return null;
    }
    expect(() => render(<Consumer />)).toThrow(/AgentGeyserProvider/);
  });
});
