import { afterEach, describe, expect, it } from 'vitest';
import { browserWorkspace, resetBrowserWorkspace } from './api';

afterEach(() => {
  localStorage.clear();
});

describe('browser-local workspaces', () => {
  it('seeds demo data in an isolated namespace and resets it exactly', async () => {
    const demo = browserWorkspace('demo');
    const original = await demo.checklist();
    expect(original).toHaveLength(5);
    expect(original[0]?.id).toBe('demo-title');

    await demo.saveChecklist([{ id: 'changed', text: 'Changed only in demo', done: true }]);
    expect(await demo.checklist()).toEqual([{ id: 'changed', text: 'Changed only in demo', done: true }]);
    expect(await browserWorkspace('hosted').checklist()).toHaveLength(5);

    resetBrowserWorkspace('demo');
    expect(await demo.checklist()).toEqual(original);
  });

  it('keeps browser workspaces within the same validation boundary as the local API', async () => {
    const hosted = browserWorkspace('hosted');
    const original = await hosted.links();
    await expect(hosted.saveLinks([{ id: 'bad-link', label: 'Unsafe link', url: 'javascript:alert(1)' }])).rejects.toThrow('Metadata links');
    expect(await hosted.links()).toEqual(original);
  });
});
