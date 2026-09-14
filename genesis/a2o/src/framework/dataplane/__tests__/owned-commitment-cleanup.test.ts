/* eslint-disable @typescript-eslint/require-await -- injected async fakes resolve synchronously by contract */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { cancelOwnedCommitmentWithReadback } from '../owned-commitment-cleanup.js';

const deferred = {
  ok: false,
  status: 400,
  text: '{"error":"REA projection changed during authority read; deferred"}',
};

void describe('cancelOwnedCommitmentWithReadback', () => {
  void it('accepts only the expected owned cancellation after an exact projection deferral', async () => {
    let cancels = 0;
    const result = await cancelOwnedCommitmentWithReadback(
      'owned-id',
      async () => {
        cancels += 1;
        return deferred;
      },
      async () => ({ id: 'owned-id', state: 'cancelled', finished: true }),
      async () => undefined
    );
    assert.equal(result.ok, true);
    assert.equal(cancels, 1);
  });

  void it('retries the idempotent cancel when readback has not caught up', async () => {
    let cancels = 0;
    const sleeps: number[] = [];
    const result = await cancelOwnedCommitmentWithReadback(
      'owned-id',
      async () => {
        cancels += 1;
        return cancels < 3 ? deferred : { ok: true, status: 200, text: '{}' };
      },
      async () =>
        cancels < 3
          ? { id: 'owned-id', state: 'proposed', finished: false }
          : { id: 'owned-id', state: 'cancelled', finished: true },
      async delay => {
        sleeps.push(delay);
      }
    );
    assert.equal(result.ok, true);
    assert.equal(cancels, 3);
    assert.deepEqual(sleeps, [100, 200]);
  });

  void it('does not accept a different id or broaden a non-retryable response', async () => {
    let cancels = 0;
    const result = await cancelOwnedCommitmentWithReadback(
      'owned-id',
      async () => {
        cancels += 1;
        return { ok: false, status: 400, text: '{"error":"not authorized"}' };
      },
      async () => ({ id: 'other-id', state: 'cancelled', finished: true }),
      async () => undefined
    );
    assert.equal(result.ok, false);
    assert.equal(cancels, 1);
  });

  void it('does not retry a 400 that merely contains the deferral text', async () => {
    let cancels = 0;
    const result = await cancelOwnedCommitmentWithReadback(
      'owned-id',
      async () => {
        cancels += 1;
        return {
          ...deferred,
          text: '{"error":"prefix: REA projection changed during authority read; deferred"}',
        };
      },
      async () => ({ id: 'owned-id', state: 'cancelled', finished: true }),
      async () => undefined
    );
    assert.equal(result.ok, false);
    assert.equal(cancels, 1);
  });

  void it('requires exact cancelled readback after a successful PATCH', async () => {
    let cancels = 0;
    const result = await cancelOwnedCommitmentWithReadback(
      'owned-id',
      async () => {
        cancels += 1;
        return { ok: true, status: 200, text: '{}' };
      },
      async () =>
        cancels < 2
          ? { id: 'owned-id', state: 'proposed', finished: false }
          : { id: 'owned-id', state: 'cancelled', finished: true },
      async () => undefined
    );
    assert.equal(result.ok, true);
    assert.equal(cancels, 2);
  });

  void it('exhausts after four cancellation attempts', async () => {
    let cancels = 0;
    const sleeps: number[] = [];
    const result = await cancelOwnedCommitmentWithReadback(
      'owned-id',
      async () => {
        cancels += 1;
        return { ok: true, status: 200, text: '{}' };
      },
      async () => ({ id: 'owned-id', state: 'proposed', finished: false }),
      async delay => {
        sleeps.push(delay);
      }
    );
    assert.equal(result.ok, false);
    assert.equal(cancels, 4);
    assert.deepEqual(sleeps, [100, 200, 400]);
  });
});
