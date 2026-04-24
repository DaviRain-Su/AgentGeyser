import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './specs',
  fullyParallel: false, retries: 0, workers: 1, reporter: 'list',
  use: { baseURL: 'http://127.0.0.1:5173', trace: 'retain-on-failure' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'], headless: true } }],
  webServer: {
    command: 'pnpm exec vite --config e2e/demo/vite.config.ts --port 5173 --strictPort',
    port: 5173, reuseExistingServer: false, timeout: 60_000,
  },
});
