/* eslint-disable @typescript-eslint/promise-function-async */
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { resolveOwnedCommitmentCreate } from '../owned-commitment-create.js';

const expected = {
  id: 'owned-doorway-a',
  action: 'project-epr',
  provider: 'human-matthew-manager',
  receiver: 'human-matthew-manager',
  inScopeOf: 'doorway:a|epr:owned',
  metadataJson: '{"urlPath":"/owned","reach":"commons"}',
  metadata: { urlPath: '/owned', reach: 'commons' },
};
const anchored = {
  ...expected,
  inScopeOf: [expected.inScopeOf],
  dhtAnchorHash: 'uhCkk-action',
};
const deferred = {
  ok: false,
  status: 400,
  text: '{"error":"REA projection changed during authority read; deferred"}',
};

void describe('resolveOwnedCommitmentCreate', () => {
  void it('accepts the exact already-created anchored row without another POST', async () => {
    let reads = 0;
    const actual = await resolveOwnedCommitmentCreate(
      deferred,
      expected,
      () => {
        reads += 1;
        return Promise.resolve({ ok: true, status: 200, text: JSON.stringify(anchored) });
      },
      Date.now() + 1_000
    );
    assert.equal(actual.dhtAnchorHash, 'uhCkk-action');
    assert.equal(reads, 1);
  });

  void it('rejects wrong terms and a missing anchor', async () => {
    await assert.rejects(
      resolveOwnedCommitmentCreate(
        deferred,
        expected,
        () =>
          Promise.resolve({
            ok: true,
            status: 200,
            text: JSON.stringify({ ...anchored, receiver: 'someone-else' }),
          }),
        Date.now() + 1_000
      )
    );
    await assert.rejects(
      resolveOwnedCommitmentCreate(
        deferred,
        expected,
        () =>
          Promise.resolve({
            ok: true,
            status: 200,
            text: JSON.stringify({ ...anchored, dhtAnchorHash: '' }),
          }),
        Date.now() + 1_000
      )
    );
  });

  void it('fails when exact readback never arrives before the immutable deadline', async () => {
    let now = 0;
    await assert.rejects(
      resolveOwnedCommitmentCreate(
        deferred,
        expected,
        () => Promise.resolve({ ok: false, status: 404, text: 'missing' }),
        3,
        () => {
          now += 1;
          return Promise.resolve();
        },
        () => now
      ),
      /not readable before the setup deadline/
    );
  });

  void it('propagates malformed, unauthorized, and nonretryable responses', async () => {
    for (const result of [
      { ok: false, status: 401, text: 'unauthorized' },
      { ok: false, status: 400, text: '{bad json' },
      { ok: false, status: 400, text: '{"error":"different"}' },
    ]) {
      await assert.rejects(
        resolveOwnedCommitmentCreate(
          result,
          expected,
          () => Promise.reject(new Error('readback must not run')),
          Date.now() + 1_000
        )
      );
    }
  });
});
