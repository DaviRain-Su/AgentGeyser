import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['test-devnet/mcp-invoke.e2e.ts'],
  },
});
