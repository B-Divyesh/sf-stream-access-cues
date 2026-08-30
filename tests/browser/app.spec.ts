import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('live surface has landmarks, one heading, and no serious accessibility violations', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveTitle(/Stream Access Cues/);
  await expect(page.locator('main')).toBeVisible();
  await expect(page.locator('h1')).toHaveCount(1);
  const results = await new AxeBuilder({ page }).analyze();
  const serious = results.violations.filter((violation) => ['serious', 'critical'].includes(violation.impact ?? ''));
  expect(serious, serious.map((item) => `${item.id}: ${item.help}`).join('\n')).toEqual([]);
});

test('keyboard guide and timer work without a pointer', async ({ page }) => {
  await page.goto('/');
  await page.keyboard.press('?');
  await expect(page.getByRole('dialog', { name: 'Shortcuts' })).toBeVisible();
  await page.keyboard.press('Escape');
  await page.keyboard.press('Control+Shift+T');
  await expect(page.getByRole('button', { name: 'Pause timer' })).toBeVisible();
  await page.keyboard.press('Control+Shift+T');
  await expect(page.getByRole('button', { name: 'Resume timer' })).toBeVisible();
});

test('checklist changes persist through the local service', async ({ page }) => {
  await page.goto('/');
  const firstItem = page.getByRole('checkbox', { name: 'Set stream title and category' });
  const before = await firstItem.isChecked();
  await firstItem.click();
  await expect(firstItem).toBeChecked({ checked: !before });
  await page.reload();
  await expect(page.getByRole('checkbox', { name: 'Set stream title and category' })).toBeChecked({ checked: !before });
});

test('private workspace data cannot be read or replaced by a separate browser context', async ({ browser }) => {
  const first = await browser.newContext();
  const second = await browser.newContext();
  const firstPage = await first.newPage();
  const secondPage = await second.newPage();
  await firstPage.goto('/');
  await secondPage.goto('/');

  const saved = await firstPage.evaluate(async () => {
    const key = localStorage.getItem('stream-access-cues.operator-key')!;
    const response = await fetch('/api/checklist', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'X-Operator-Key': key },
      body: JSON.stringify([{ id: 'operator-one-only', text: 'Private first workspace item', done: true }])
    });
    return response.status;
  });
  expect(saved).toBe(200);

  const secondItems = await secondPage.evaluate(async () => {
    const key = localStorage.getItem('stream-access-cues.operator-key')!;
    const response = await fetch('/api/checklist', { headers: { 'X-Operator-Key': key } });
    return response.json() as Promise<Array<{ id: string }>>;
  });
  expect(secondItems.map((item) => item.id)).not.toContain('operator-one-only');

  await first.close();
  await second.close();
});

test('390px layout does not scroll sideways', async ({ page }) => {
  await page.goto('/');
  const widths = await page.evaluate(() => ({ body: document.body.scrollWidth, viewport: document.documentElement.clientWidth }));
  expect(widths.body).toBeLessThanOrEqual(widths.viewport);
});

test('checklist and footer controls meet the 44px pointer-target contract', async ({ page }) => {
  await page.goto('/');
  const undersizedTargets = await page.locator('.checklist input[type="checkbox"], .checklist label, footer nav a').evaluateAll((targets) =>
    targets.map((target) => {
      const { width, height } = target.getBoundingClientRect();
      return { name: target.textContent?.trim() || target.getAttribute('aria-label') || target.tagName, width, height };
    }).filter((target) => target.width < 44 || target.height < 44)
  );
  expect(undersizedTargets).toEqual([]);
});

test('direct legal page navigation returns 200 without browser console errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => errors.push(error.message));

  for (const route of ['/privacy', '/terms']) {
    const response = await page.goto(route);
    expect(response?.status(), route).toBe(200);
    await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
  }
  expect(errors).toEqual([]);
});

test('offline shell loads and an updated worker removes the previous release cache', async ({ page, context }) => {
  await page.goto('/', { waitUntil: 'networkidle' });
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  await context.setOffline(true);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('h1')).toHaveCount(1);
  await context.setOffline(false);

  await page.evaluate(async () => {
    await navigator.serviceWorker.register('/sw.js?build=playwright-update-check');
    await new Promise<void>((resolve, reject) => {
      const timeout = window.setTimeout(() => reject(new Error('Updated worker did not take control.')), 5_000);
      navigator.serviceWorker.addEventListener('controllerchange', () => {
        window.clearTimeout(timeout);
        resolve();
      }, { once: true });
    });
    // The first new-worker request sweeps a cache that an outgoing worker may
    // have completed while the update was installing.
    await fetch('/icon.svg');
  });
  await page.waitForFunction(async () => (await caches.keys()).length === 1);
  const cacheState = await page.evaluate(async () => ({
    controller: navigator.serviceWorker.controller?.scriptURL,
    caches: await caches.keys()
  }));
  expect(cacheState.controller).toContain('playwright-update-check');
  expect(cacheState.caches).toEqual(['stream-access-cues-playwright-update-check']);
});
