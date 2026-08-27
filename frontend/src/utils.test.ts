import { describe, expect, it } from 'vitest';
import { formatDuration, timerMilliseconds } from './utils';

describe('stream timer', () => {
  it('formats long sessions with tabular clock fields', () => {
    expect(formatDuration(3_661_000)).toBe('01:01:01');
  });

  it('includes time since a running timer started', () => {
    expect(timerMilliseconds({ elapsed: 2_000, startedAt: 10_000, running: true }, 13_000)).toBe(5_000);
  });

  it('never returns a negative duration', () => {
    expect(timerMilliseconds({ elapsed: -5, startedAt: null, running: false })).toBe(0);
  });
});
