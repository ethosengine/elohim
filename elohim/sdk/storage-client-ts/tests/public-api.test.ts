import { describe, expect, it } from 'vitest';

// Self-reference resolves package.json exports to dist, without Vite/tsconfig source aliases.
import { HttpTier, placementFor, type NeedStatus } from '@elohim/storage-client';

describe('built placement public API', () => {
  it('ships transport and placement through the package barrel', async () => {
    expect(HttpTier).toBeTypeOf('function');
    const present: NeedStatus = {
      need: { field: 'closure' },
      state: 'present',
    };
    const placement = placementFor(
      'published-app',
      {
        getJson: async () => {
          throw new Error('device need must not use a network route');
        },
      },
      { declared: async () => [present.need], status: async () => present }
    );
    expect(await placement.status()).toEqual([present]);
  });
});
