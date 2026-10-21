import assert from 'node:assert/strict';
import { test } from 'node:test';
import { HttpTier, HttpConnector } from '@elohim/storage-client/tiers/http-tier';
import { placementFor } from '@elohim/storage-client/placement';
import { MemoryTier, TierChain } from '@elohim/eprfs';
import { verifyAddress } from '@elohim/epr';

test('published placement subpaths load in plain Node ESM', async () => {
  assert.equal(typeof HttpTier, 'function');
  assert.equal(typeof HttpConnector, 'function');
  assert.equal(typeof MemoryTier, 'function');
  assert.equal(typeof TierChain, 'function');
  assert.equal(await verifyAddress('not-an-address', new Uint8Array()), false);
  const present = { need: { field: 'closure' }, state: 'present' };
  const placement = placementFor(
    'published-app',
    {
      getJson: async () => {
        throw new Error('a device need must not use a network route');
      },
    },
    { declared: async () => [present.need], status: async () => present }
  );
  assert.deepEqual(await placement.status(), [present]);
});
