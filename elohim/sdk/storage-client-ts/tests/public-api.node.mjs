import assert from 'node:assert/strict';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
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

// Run each require consumer in a separate process: TSX installs global loader
// hooks, and the ESM test above must continue to exercise native ESM resolution.
for (const mode of ['native', 'tsx']) {
  test(`published storage-client root loads through ${mode} CommonJS`, () => {
    const args = mode === 'tsx' ? ['--require', 'tsx'] : [];
    const result = spawnSync(
      process.execPath,
      [
        ...args,
        '--eval',
        `
      const assert = require('node:assert/strict');
      const { StorageClient, AutomergeSync, HttpTier, HttpConnector, placementFor } =
        require('@elohim/storage-client');
      assert.equal(typeof StorageClient, 'function');
      assert.equal(typeof AutomergeSync, 'function');
      assert.equal(typeof HttpTier, 'function');
      assert.equal(typeof HttpConnector, 'function');
      const present = { need: { field: 'closure' }, state: 'present' };
      const placement = placementFor('published-app', {
        getJson: async () => { throw new Error('a device need must not use a network route'); },
      }, { declared: async () => [present.need], status: async () => present });
      placement.status().then((status) => assert.deepEqual(status, [present]))
        .catch((error) => { console.error(error); process.exitCode = 1; });
    `,
      ],
      {
        cwd: fileURLToPath(new URL('..', import.meta.url)),
        encoding: 'utf8',
      },
    );
    assert.equal(
      result.status,
      0,
      result.error?.message ?? result.stdout + result.stderr,
    );
  });
}
