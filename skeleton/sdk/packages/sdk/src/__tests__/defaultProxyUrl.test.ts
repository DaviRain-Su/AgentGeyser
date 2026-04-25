import { afterEach, describe, expect, it, vi } from 'vitest';
import { defaultProxyUrl } from '../index.js';

describe('defaultProxyUrl', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('respects env var', () => {
    vi.stubEnv('AGENTGEYSER_PROXY_PORT', '12345');
    expect(defaultProxyUrl()).toBe('http://127.0.0.1:12345');
  });

  it('falls back to 8999', () => {
    vi.stubEnv('AGENTGEYSER_PROXY_PORT', undefined);
    expect(defaultProxyUrl()).toBe('http://127.0.0.1:8999');
  });
});
