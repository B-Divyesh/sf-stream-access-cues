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

test('390px layout does not scroll sideways', async ({ page }) => {
  await page.goto('/');
  const widths = await page.evaluate(() => ({ body: document.body.scrollWidth, viewport: document.documentElement.clientWidth }));
  expect(widths.body).toBeLessThanOrEqual(widths.viewport);
});
