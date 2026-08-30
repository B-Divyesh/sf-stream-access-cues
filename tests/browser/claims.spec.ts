import { expect, test } from '@playwright/test';

test('@claim:demo-sample-isolated @claim:sample-cues-never-contact-obs one click opens a realistic sample without calling workspace APIs', async ({ page }) => {
  const requests: string[] = [];
  page.on('request', (request) => requests.push(request.url()));

  await page.goto('/');
  await expect(page.getByRole('checkbox', { name: 'Set stream title and category' })).toBeVisible();
  requests.length = 0;
  await page.getByRole('link', { name: 'Try it with sample data' }).click();
  await expect(page).toHaveURL(/\/demo$/);
  await expect(page.getByText('Demo — sample data, nothing is saved.')).toBeVisible();
  await expect(page.getByRole('checkbox', { name: 'Set the Friday community stream title' })).toBeChecked();
  await expect(page.getByRole('button', { name: 'Starting soon' })).toBeEnabled();

  const storage = await page.evaluate(() => localStorage.getItem('demo:stream-access-cues.workspace.v1'));
  expect(storage).toContain('demo-title');
  expect(requests.every((url) => new URL(url).origin === 'http://127.0.0.1:4173')).toBe(true);
  expect(requests.map((url) => new URL(url).pathname)).not.toContain('/api/checklist');
  expect(requests.map((url) => new URL(url).pathname)).not.toContain('/api/cues');
  expect(requests.map((url) => new URL(url).pathname)).not.toContain('/api/links');
  expect(requests.map((url) => new URL(url).pathname)).not.toContain('/api/settings');

  await page.goto('/?demo=1');
  await expect(page.getByText('Demo — sample data, nothing is saved.')).toBeVisible();
});

test('@claim:demo-reset reset restores the bundled sample and leaving discards it', async ({ page }) => {
  await page.goto('/demo');
  const changed = page.getByRole('checkbox', { name: 'Check the recording folder has space' });
  await expect(changed).not.toBeChecked();
  await changed.check();
  await expect(changed).toBeChecked();
  await page.getByRole('button', { name: 'Reset demo' }).click();
  await expect(changed).not.toBeChecked();

  await page.getByRole('link', { name: 'Start for real' }).first().click();
  await expect(page).toHaveURL(/\/$/);
  expect(await page.evaluate(() => localStorage.getItem('demo:stream-access-cues.workspace.v1'))).toBeNull();
});

test('@claim:demo-offline-reload the sample dashboard reloads offline after its first visit', async ({ browser }) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  try {
    await page.goto('/demo', { waitUntil: 'networkidle' });
    await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
    await context.setOffline(true);
    await page.reload({ waitUntil: 'domcontentloaded' });
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('Control your stream with a keyboard');
    await expect(page.getByRole('checkbox', { name: 'Set the Friday community stream title' })).toBeChecked();
  } finally {
    await context.close();
  }
});

test('@claim:keyboard-shortcuts keyboard scene cues and timer actions work in the demo', async ({ page }) => {
  await page.goto('/demo');
  await page.keyboard.press('Control+Shift+1');
  await expect(page.locator('.live-region')).toContainText('sample cue changed to Starting Soon');
  await page.keyboard.press('Control+Shift+T');
  await expect(page.getByRole('button', { name: 'Pause timer' })).toBeVisible();
  await page.keyboard.press('?');
  await expect(page.getByRole('dialog', { name: 'Shortcuts' })).toBeVisible();
  await page.keyboard.press('Escape');
});

test('@claim:privacy-no-third-parties the demo starts without an account and only requests this origin', async ({ page }) => {
  const requests: string[] = [];
  page.on('request', (request) => requests.push(request.url()));
  await page.goto('/demo');
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  await expect(page.getByRole('button', { name: /sign in|log in/i })).toHaveCount(0);
  expect(requests).not.toEqual([]);
  expect(requests.every((url) => new URL(url).origin === 'http://127.0.0.1:4173')).toBe(true);
});

test('@claim:local-workspace-persistence local checklist changes survive a reload', async ({ page }) => {
  await page.goto('/');
  const item = page.getByRole('checkbox', { name: 'Set stream title and category' });
  const before = await item.isChecked();
  await item.click();
  await page.reload();
  await expect(page.getByRole('checkbox', { name: 'Set stream title and category' })).toBeChecked({ checked: !before });
});

test('@claim:rate-limit public API responses return Retry-After when the burst is exhausted', async ({ request }) => {
  const responses = await Promise.all(Array.from({ length: 80 }, async () => {
    const response = await request.get('/api/runtime', { headers: { 'X-Forwarded-For': '198.51.100.250' } });
    return { status: response.status(), retryAfter: response.headers()['retry-after'] };
  }));
  expect(responses.some((response) => response.status === 429 && response.retryAfter === '1')).toBe(true);
});
