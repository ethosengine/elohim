import { describe, expect, it } from 'vitest';

import {
  absent,
  answerFromStatus,
  isPresent,
  present,
  unreachable,
  unreachableFromError,
  valueOr,
} from './answer.js';

describe('Answer — the distinction that matters', () => {
  it('absent is a positive claim; unreachable establishes nothing', () => {
    expect(absent().state).toBe('absent');
    expect(unreachable('timeout').state).toBe('unreachable');
    // Both carry a reason. A non-present answer that cannot say why is how
    // "no such thing" and "we could not ask" get conflated downstream.
    expect(absent().reason).toBe('not-found');
    expect(unreachable('refused').reason).toBe('refused');
  });

  it('only a 404 yields absent', () => {
    expect(answerFromStatus(404).state).toBe('absent');
    for (const status of [401, 403]) {
      const a = answerFromStatus(status);
      expect(a.state, `status ${status}`).toBe('unreachable');
      expect(a.reason).toBe('refused');
    }
    for (const status of [500, 502, 418]) {
      expect(answerFromStatus(status).state, `status ${status}`).toBe('unreachable');
    }
  });

  it('separates a deadline we imposed from a transport failure', () => {
    const abort = new Error('aborted');
    abort.name = 'AbortError';
    expect(unreachableFromError(abort).reason).toBe('timeout');
    expect(unreachableFromError(new TypeError('Failed to fetch')).reason).toBe('transport');
    // Non-Error throws must not crash the classifier.
    expect(unreachableFromError('something odd').reason).toBe('transport');
  });

  it('valueOr never throws and isPresent narrows', () => {
    const p = present(42);
    expect(isPresent(p) && p.value).toBe(42);
    expect(valueOr(p, 0)).toBe(42);
    expect(valueOr(absent(), 7)).toBe(7);
    expect(valueOr(unreachable('transport'), 7)).toBe(7);
  });
});
