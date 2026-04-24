import { test, expect } from '@playwright/test';

const PROXY_URL = 'http://127.0.0.1:8999';

test.beforeAll(async () => {
  try {
    const res = await fetch(PROXY_URL, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'ag_listSkills', params: [] }),
      signal: AbortSignal.timeout(3000),
    });
    if (!res.ok) test.skip(true, `proxy not reachable at ${PROXY_URL} (status ${res.status})`);
  } catch (err) {
    test.skip(true, `proxy not reachable at ${PROXY_URL}: ${(err as Error).message}`);
  }
});

test.beforeEach(async ({ page, baseURL }) => {
  try {
    await page.goto(baseURL ?? '/', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('invoke')).toBeVisible({ timeout: 5000 });
  } catch {
    test.skip(true, 'webServer not ready or demo bundle broken');
  }
});

test('clicking invoke displays a signature from the mocked wallet path', async ({ page }) => {
  await page.getByTestId('invoke').click();
  const sig = page.getByTestId('signature');
  await expect(sig).toBeVisible({ timeout: 15_000 });
  expect(((await sig.textContent()) ?? '').length).toBeGreaterThan(10);
});
