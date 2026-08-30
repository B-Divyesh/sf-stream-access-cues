import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('@claim:hosted-guide-never-connects-obs hosted deployment explains the local control boundary and rejects OBS writes', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByText('Local control required')).toBeVisible();
  await expect(page.getByText('This public guide never reaches into your computer.')).toBeVisible();
  await page.getByRole('button', { name: 'View local setup' }).first().click();
  await expect(page.getByRole('dialog', { name: 'Run Stream Access Cues beside OBS' })).toBeVisible();
  await expect(page.getByLabel('Local Docker command')).toContainText('DEPLOYMENT_MODE=local');
  await page.keyboard.press('Escape');

  const response = await page.evaluate(async () => {
    const key = localStorage.getItem('stream-access-cues.operator-key')!;
    const response = await fetch('/api/settings', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'X-Operator-Key': key },
      body: JSON.stringify({ obs_host: '127.0.0.1', obs_port: 4455, obs_password: 'do-not-store' })
    });
    return { status: response.status, body: await response.json() };
  });
  expect(response.status).toBe(403);
  expect(response.body.error).toContain('local Stream Access Cues service');

  const results = await new AxeBuilder({ page }).analyze();
  const serious = results.violations.filter((violation) => ['serious', 'critical'].includes(violation.impact ?? ''));
  expect(serious, serious.map((item) => `${item.id}: ${item.help}`).join('\n')).toEqual([]);
});

test('@claim:hosted-workspace-stays-in-browser public checklist changes persist in this browser and hosted APIs reject state', async ({ page }) => {
  const workspaceRequests: string[] = [];
  let runtimeOperatorKey: string | null | undefined;
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/runtime') runtimeOperatorKey = request.headers()['x-operator-key'];
    else workspaceRequests.push(new URL(request.url()).pathname);
  });
  await page.goto('/');
  expect(runtimeOperatorKey).toBeUndefined();
  const firstItem = page.getByRole('checkbox', { name: 'Set stream title and category' });
  await firstItem.check();
  await page.reload();
  await expect(page.getByRole('checkbox', { name: 'Set stream title and category' })).toBeChecked();
  expect(workspaceRequests).not.toContain('/api/checklist');

  const status = await page.evaluate(async () => {
    const key = localStorage.getItem('stream-access-cues.operator-key')!;
    const response = await fetch('/api/checklist', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'X-Operator-Key': key },
      body: JSON.stringify([{ id: 'must-not-persist', text: 'Public instances must not store this', done: true }])
    });
    return { status: response.status, body: await response.json() as { error: string } };
  });
  expect(status.status).toBe(403);
  expect(status.body.error).toContain('does not store workspace data');
});
