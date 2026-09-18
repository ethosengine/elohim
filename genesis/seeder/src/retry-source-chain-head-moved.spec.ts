/**
 * Unit tests for retry-source-chain-head-moved.ts — the package-local
 * equivalent of genesis/a2o/src/framework/dataplane/carried-election.ts's
 * `isSourceChainHeadMovedError` / `retryOnSourceChainHeadMoved` (see that
 * module's tests, `__tests__/carried-election.test.ts`, for the sibling
 * cases this mirrors).
 */
import { describe, expect, it } from 'vitest';

import {
  isSourceChainHeadMovedError,
  retryOnSourceChainHeadMoved,
} from './retry-source-chain-head-moved.js';

const RETRY_LABEL = 'test-leg';
const noopSleep = async (): Promise<void> => {};

describe('isSourceChainHeadMovedError', () => {
  it('matches the long-form message', () => {
    expect(
      isSourceChainHeadMovedError(
        'Source chain error: Attempted to commit a bundle to the source chain, ' +
          'but the source chain head has moved since the bundle began.',
      ),
    ).toBe(true);
  });

  it('matches the short HeadMoved form', () => {
    expect(isSourceChainHeadMovedError('HeadMoved: something')).toBe(true);
  });

  it('does not match a near-miss like NotHeadMovedPermanent', () => {
    expect(isSourceChainHeadMovedError('NotHeadMovedPermanent')).toBe(false);
  });

  it('does not match an unrelated error', () => {
    expect(isSourceChainHeadMovedError('CellDisabled')).toBe(false);
  });
});

describe('retryOnSourceChainHeadMoved', () => {
  it('retries the exact source-chain-head-moved error then resolves', async () => {
    let attempts = 0;
    const sleeps: number[] = [];
    const result = await retryOnSourceChainHeadMoved(
      RETRY_LABEL,
      async () => {
        attempts += 1;
        if (attempts <= 3) {
          throw new Error(
            'internal_error: Source chain error: Attempted to commit a bundle to the source ' +
              'chain, but the source chain head has moved since the bundle began. Bundle head: ' +
              'aaa Current head: bbb seq: 8918',
          );
        }
        return { ok: true };
      },
      async delay => {
        sleeps.push(delay);
      },
    );
    expect(result).toEqual({ ok: true });
    expect(attempts).toBe(4);
    expect(sleeps).toHaveLength(3);
  });

  it('propagates a different error immediately without retrying', async () => {
    let attempts = 0;
    await expect(
      retryOnSourceChainHeadMoved(
        RETRY_LABEL,
        async () => {
          attempts += 1;
          throw new Error('CellDisabled');
        },
        noopSleep,
      ),
    ).rejects.toThrow(/CellDisabled/);
    expect(attempts).toBe(1);
  });

  it('exhausts bounded attempts (<=6) on persistent HeadMoved and names attempts + heads', async () => {
    let attempts = 0;
    const sleeps: number[] = [];
    await expect(
      retryOnSourceChainHeadMoved(
        RETRY_LABEL,
        async () => {
          attempts += 1;
          throw new Error(
            'internal_error: Source chain error: … the source chain head has moved since the ' +
              'bundle began. Bundle head: aaa Current head: bbb seq: 8918',
          );
        },
        async delay => {
          sleeps.push(delay);
        },
      ),
    ).rejects.toThrow(/gave up after 6 attempts.*Bundle head: aaa.*Current head: bbb.*seq: 8918/s);
    expect(attempts).toBe(6);
    expect(sleeps).toHaveLength(5);
    for (const delay of sleeps) {
      expect(delay).toBeGreaterThanOrEqual(0);
    }
  });
});
