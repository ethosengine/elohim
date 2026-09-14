/* eslint-disable @typescript-eslint/require-await -- injected async fakes resolve synchronously by contract */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { authorizeSigningCredentialsWithRetry } from '../carried-election.js';

import type { CellId } from '@holochain/client';

const cell = [new Uint8Array([1]), new Uint8Array([2])] as unknown as CellId;

void describe('authorizeSigningCredentialsWithRetry', () => {
  void it('retries the exact transient head conflict with bounded backoff', async () => {
    let attempts = 0;
    const sleeps: number[] = [];
    await authorizeSigningCredentialsWithRetry(
      cell,
      async () => {
        attempts += 1;
        if (attempts < 3) throw new Error('source chain head has moved since the bundle began');
      },
      () => undefined,
      async delay => {
        sleeps.push(delay);
      }
    );
    assert.equal(attempts, 3);
    assert.deepEqual(sleeps, [100, 200]);
  });

  void it('exhausts after four attempts', async () => {
    let attempts = 0;
    const sleeps: number[] = [];
    await assert.rejects(
      authorizeSigningCredentialsWithRetry(
        cell,
        async () => {
          attempts += 1;
          throw new Error('HeadMoved');
        },
        () => undefined,
        async delay => {
          sleeps.push(delay);
        }
      ),
      /HeadMoved/
    );
    assert.equal(attempts, 4);
    assert.deepEqual(sleeps, [100, 200, 400]);
  });

  void it('propagates a non-retryable failure immediately', async () => {
    let attempts = 0;
    await assert.rejects(
      authorizeSigningCredentialsWithRetry(
        cell,
        async () => {
          attempts += 1;
          throw new Error('CellDisabled');
        },
        () => undefined,
        async () => undefined
      ),
      /CellDisabled/
    );
    assert.equal(attempts, 1);
  });

  void it('does not retry a symbolic near-match', async () => {
    let attempts = 0;
    await assert.rejects(
      authorizeSigningCredentialsWithRetry(
        cell,
        async () => {
          attempts += 1;
          throw new Error('NotHeadMovedPermanent');
        },
        () => undefined,
        async () => undefined
      ),
      /NotHeadMovedPermanent/
    );
    assert.equal(attempts, 1);
  });

  void it('reuses credentials cached for the exact live cell', async () => {
    let attempts = 0;
    const key = (candidate: CellId) =>
      `${Buffer.from(candidate[0]).toString('hex')}:${Buffer.from(candidate[1]).toString('hex')}`;
    const cachedKey = key(cell);
    const equalCell = [new Uint8Array([1]), new Uint8Array([2])] as unknown as CellId;
    await authorizeSigningCredentialsWithRetry(
      equalCell,
      async () => {
        attempts += 1;
      },
      candidate => (key(candidate) === cachedKey ? { cached: true } : undefined),
      async () => undefined
    );
    assert.equal(attempts, 0);
    assert.notEqual(
      key([new Uint8Array([9]), equalCell[1]] as unknown as CellId),
      cachedKey,
      'DNA bytes participate in the cache key'
    );
    assert.notEqual(
      key([equalCell[0], new Uint8Array([9])] as unknown as CellId),
      cachedKey,
      'agent bytes participate in the cache key'
    );
  });

  void it('consumes credentials populated by another writer during backoff', async () => {
    let attempts = 0;
    let populated = false;
    await authorizeSigningCredentialsWithRetry(
      cell,
      async () => {
        attempts += 1;
        throw new Error('HeadMoved');
      },
      () => (populated ? { cached: true } : undefined),
      async () => {
        populated = true;
      }
    );
    assert.equal(attempts, 1);
  });
});
