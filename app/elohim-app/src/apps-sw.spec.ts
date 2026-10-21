// @vitest-environment node
// The service worker's own tier (placement interfaces spec §H.3, rails R-T2 and H5): CacheTier
// caches only bytes that verify (put checks them), verifies every hit, and lives outside
// activate's purge.
//
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

import { absence } from '@elohim/eprfs';

import type { BatchItem, TierAnswer } from '@elohim/eprfs';

/** An in-memory Cache Storage entry set, keyed by URL as the real one is. */
class FakeCache {
  readonly entries = new Map<string, Response>();

  async match(request: Request | string): Promise<Response | undefined> {
    const url = typeof request === 'string' ? request : request.url;
    return this.entries.get(url)?.clone();
  }

  async put(request: Request | string, response: Response): Promise<void> {
    const url = typeof request === 'string' ? request : request.url;
    this.entries.set(url, response.clone());
  }

  async delete(request: Request | string): Promise<boolean> {
    const url = typeof request === 'string' ? request : request.url;
    return this.entries.delete(url);
  }
}

const ORIGIN = 'https://doorway.example';
const enc = new TextEncoder();

async function rawCid(bytes: Uint8Array): Promise<CID> {
  return CID.create(1, 0x55, await sha256.digest(bytes));
}

type SwModule = typeof import('./apps-sw');

async function loadWorker(): Promise<SwModule> {
  // The worker registers its lifecycle listeners on `self` and opens its invalidation channel at
  // load; give it an inert scope and channel (this spec runs in node: the worker has no DOM).
  vi.stubGlobal('self', { addEventListener: () => undefined, location: { origin: ORIGIN } });
  vi.stubGlobal(
    'BroadcastChannel',
    class {
      onmessage: unknown = null;
      close(): void {
        /* inert */
      }
    }
  );
  return import('./apps-sw');
}

afterAll(() => {
  vi.unstubAllGlobals();
});

describe('CacheTier', () => {
  let sw: SwModule;
  beforeAll(async () => {
    sw = await loadWorker();
  });

  function tierOver(cache: FakeCache) {
    return new sw.CacheTier(cache as unknown as Cache, ORIGIN);
  }

  it('stores verified bytes under /_eprfs/<cid> and answers them back', async () => {
    const cache = new FakeCache();
    const tier = tierOver(cache);
    const bytes = enc.encode('index.html bytes');
    const cid = await rawCid(bytes);
    await tier.put(cid, bytes);
    expect([...cache.entries.keys()]).toEqual([`${ORIGIN}/_eprfs/${cid.toString()}`]);
    expect(await tier.fetchAt(cid, { cid, path: [] }, [])).toEqual(bytes);
    expect(tier.record).toMatchObject({ id: 'own', class: 'own', endpoint: null });
  });

  it('put verifies before it writes: bytes that do not match the CID throw and nothing is cached (R-T2)', async () => {
    const cache = new FakeCache();
    const tier = tierOver(cache);
    const cid = await rawCid(enc.encode('the real bytes'));
    await expect(tier.put(cid, enc.encode('bytes a broken caller handed over'))).rejects.toThrow(
      /eprfs-integrity/
    );
    expect(cache.entries.size).toBe(0);
  });

  it('a forged entry a page wrote under /_eprfs/<cid> is integrity, deleted, never served (H5)', async () => {
    const cache = new FakeCache();
    const tier = tierOver(cache);
    const cid = await rawCid(enc.encode('the real bytes'));
    const key = `${ORIGIN}/_eprfs/${cid.toString()}`;
    await cache.put(key, new Response(enc.encode('forged by a same-origin page')));
    const path = [enc.encode('help.html')];
    expect(await tier.fetchAt(cid, { cid, path }, [])).toEqual({
      kind: 'absent',
      cid: cid.toString(),
      path,
      unanswered: absence.integrity('own'),
    });
    expect(cache.entries.has(key)).toBe(false);
  });

  it('a miss is not-held with no tier tried, and a failing cache is unreachable, never not-held', async () => {
    const cid = await rawCid(enc.encode('never stored'));
    expect(await tierOver(new FakeCache()).fetchAt(cid, { cid, path: [] }, [])).toEqual({
      kind: 'absent',
      cid: cid.toString(),
      path: [],
      unanswered: absence.notHeld([]),
    });
    const broken = new FakeCache();
    broken.match = () => Promise.reject(new DOMException('storage unavailable', 'UnknownError'));
    expect(await tierOver(broken).fetchAt(cid, { cid, path: [] }, [])).toMatchObject({
      kind: 'absent',
      unanswered: { why: { kind: 'unreachable', tier: 'own' } },
    });
  });

  it('fetchBlobs answers every item, in order, over fetchAt; a written-back file serves offline', async () => {
    const cache = new FakeCache();
    const tier = tierOver(cache);
    const held = enc.encode('app.js');
    const heldCid = await rawCid(held);
    const missing = await rawCid(enc.encode('not here'));
    // What TierChain does after an outer tier's bytes verified: write back into the own tier.
    await tier.put(heldCid, held);
    const answers: [string, TierAnswer][] = [];
    const items: BatchItem[] = [
      { cid: heldCid, path: [enc.encode('app.js')] },
      { cid: missing, path: [enc.encode('b')] },
    ];
    await tier.fetchBlobs(heldCid, items, [], (item, r) => answers.push([item.cid.toString(), r]));
    expect(answers.map(([cid]) => cid)).toEqual([heldCid.toString(), missing.toString()]);
    expect(answers[0][1]).toEqual(held);
    expect(answers[1][1]).toMatchObject({
      kind: 'absent',
      unanswered: { why: { kind: 'not-held' } },
    });
  });

  it('answers a failed cached-body read as unreachable and continues the batch', async () => {
    const cache = new FakeCache();
    const tier = tierOver(cache);
    const brokenCid = await rawCid(enc.encode('unreadable body'));
    const held = enc.encode('readable next item');
    const heldCid = await rawCid(held);
    await tier.put(heldCid, held);
    const match = cache.match.bind(cache);
    cache.match = async request => {
      const url = typeof request === 'string' ? request : request.url;
      if (url.endsWith(brokenCid.toString())) {
        const response = new Response('unreadable body');
        response.arrayBuffer = () => Promise.reject(new TypeError('cached body unavailable'));
        return response;
      }
      return match(request);
    };
    const items: BatchItem[] = [
      { cid: brokenCid, path: [enc.encode('broken.js')] },
      { cid: heldCid, path: [enc.encode('held.js')] },
    ];
    const answers: TierAnswer[] = [];
    await expect(
      tier.fetchBlobs(heldCid, items, [], (_item, answer) => answers.push(answer))
    ).resolves.toBeUndefined();
    expect(answers).toEqual([
      {
        kind: 'absent',
        cid: brokenCid.toString(),
        path: items[0].path,
        unanswered: absence.unreachable('own'),
      },
      held,
    ]);
  });
});

describe('the eprfs-objects cache outlives a worker update (H19)', () => {
  it('activate drops older apps-* caches and never CacheTier’s', async () => {
    const sw = await loadWorker();
    expect(sw.EPRFS_CACHE_NAME).toBe('eprfs-objects');
    expect(sw.staleAppCaches(['apps-v1', 'apps-v2', 'apps-v3', 'eprfs-objects', 'other'])).toEqual([
      'apps-v1',
      'apps-v2',
    ]);
  });
});
