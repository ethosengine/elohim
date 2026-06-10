import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach, vi } from 'vitest';
import {
  buildCustodyCommitmentBody,
  defaultCustodyPairs,
  resolveCustodyPeerIds,
  type CustodyPair,
} from '../seed-commitments.js';
import { clearPeerIdCache, deterministicPeerId, resolvePeerId, storageUrlForHuman } from '../peer-id.js';

describe('buildCustodyCommitmentBody', () => {
  const pair: CustodyPair = {
    providerHumanId: 'human-matthew-manager',
    providerArchetype: 'desktop',
    receiverHumanId: 'human-terrance-tutor',
    receiverArchetype: 'desktop',
    blobHash: 'sha256-deadbeef',
    blobSizeBytes: 12345,
  };

  it('action is exactly "custody-blob"', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.action).toBe('custody-blob');
  });

  it('provider and receiver are 12D3KooW peer_ids, not human-* cids', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.provider).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    expect(body.receiver).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    expect(body.provider).not.toMatch(/^human-/);
    expect(body.receiver).not.toMatch(/^human-/);
  });

  it('resourceClassifiedAs is a ValueFlows list holding the sha256- blob hash', () => {
    const body = buildCustodyCommitmentBody(pair);
    // List shape per CreateReaCommitmentInputView (ValueFlows resourceClassifiedAs
    // is a classification list) — stored as a JSON array so ReaCommitmentView's
    // parse-as-array round-trips it.
    expect(body.resourceClassifiedAs).toEqual(['sha256-deadbeef']);
  });

  it('resourceQuantity uses bytes-as-integer with hasUnit "B"', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.resourceQuantity.hasNumericalValue).toBe(12345);
    expect(body.resourceQuantity.hasUnit).toBe('B');
  });

  it('id is distinct per (provider_peer, receiver_peer, blob_hash) tuple', () => {
    const a = buildCustodyCommitmentBody(pair);
    const b = buildCustodyCommitmentBody({ ...pair, blobHash: 'sha256-feedface' });
    expect(a.id).not.toBe(b.id);
  });

  it('id is deterministic — same tuple → same id (idempotent re-runs)', () => {
    const a = buildCustodyCommitmentBody(pair);
    const b = buildCustodyCommitmentBody(pair);
    expect(a.id).toBe(b.id);
  });

  it('uses resolved REAL peer ids when provided (and the content-addressed id follows)', () => {
    const resolved = {
      provider: '12D3KooWQAaKDy1JkpBNLHEP7KjazhAmDCSUzVyLUQ62eftF73N4',
      receiver: '12D3KooWBhYqzhQ8XK2v9PqQ7TZxGw7vM1nRkLs5uJcDeFgHiJkL',
    };
    const body = buildCustodyCommitmentBody(pair, resolved);
    expect(body.provider).toBe(resolved.provider);
    expect(body.receiver).toBe(resolved.receiver);
    // Different peer-id tuple → different content-addressed id than Stage 1.
    expect(body.id).not.toBe(buildCustodyCommitmentBody(pair).id);
  });
});

// =============================================================================
// Stage 2 peer-id resolution — real ids from live storage pods
// =============================================================================

describe('resolvePeerId (Stage 2)', () => {
  const REAL_ID = '12D3KooWQAaKDy1JkpBNLHEP7KjazhAmDCSUzVyLUQ62eftF73N4';

  beforeEach(() => clearPeerIdCache());
  afterEach(() => vi.restoreAllMocks());

  const okFetch = (peerId: string = REAL_ID) =>
    vi.fn(async (_url: string) => new Response(JSON.stringify({ peerId }), { status: 200 }));

  it('happy path: returns the real peerId from GET <host>/p2p/status', async () => {
    const fetchImpl = okFetch();
    const id = await resolvePeerId('human-matthew-manager', 'desktop', { fetchImpl });
    expect(id).toBe(REAL_ID);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(fetchImpl.mock.calls[0][0]).toBe(
      `${storageUrlForHuman('human-matthew-manager')}/p2p/status`
    );
  });

  it('derives the storage host from the humanId short name (alpha convention)', () => {
    expect(storageUrlForHuman('human-matthew-manager')).toBe(
      'http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090'
    );
  });

  it('caches per host within a run — one probe serves repeated resolutions', async () => {
    const fetchImpl = okFetch();
    const a = await resolvePeerId('human-matthew-manager', 'desktop', { fetchImpl });
    const b = await resolvePeerId('human-matthew-manager', 'desktop', { fetchImpl });
    expect(a).toBe(b);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('fallback: unreachable pod → deterministic Stage-1 id + loud warn naming the host', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const fetchImpl = vi.fn(async () => {
      throw new Error('connect ECONNREFUSED');
    });
    const id = await resolvePeerId('human-jessica-spouse', 'desktop', { fetchImpl });
    expect(id).toBe(deterministicPeerId('human-jessica-spouse', 'desktop'));
    expect(warn).toHaveBeenCalledTimes(1);
    const message = warn.mock.calls[0][0] as string;
    expect(message).toContain(storageUrlForHuman('human-jessica-spouse'));
    expect(message).toContain('Custody sweep will not match this peer');
  });

  it('fallback: 200 response missing peerId → deterministic Stage-1 id', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const fetchImpl = vi.fn(async () => new Response('{}', { status: 200 }));
    const id = await resolvePeerId('human-jessica-spouse', 'desktop', { fetchImpl });
    expect(id).toBe(deterministicPeerId('human-jessica-spouse', 'desktop'));
  });

  it('failed probes are cached too — seed and activate phases compute identical ids', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const fetchImpl = vi.fn(async () => {
      throw new Error('timeout');
    });
    const a = await resolvePeerId('human-james-student', 'mobile', { fetchImpl });
    const b = await resolvePeerId('human-james-student', 'mobile', { fetchImpl });
    expect(a).toBe(b);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('resolveCustodyPeerIds preserves role semantics: provider human → provider id', async () => {
    const ids: Record<string, string> = {
      'http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090/p2p/status': REAL_ID,
      'http://elohim-jessica-alpha.elohim-alpha.svc.cluster.local:8090/p2p/status':
        '12D3KooWBhYqzhQ8XK2v9PqQ7TZxGw7vM1nRkLs5uJcDeFgHiJkL',
    };
    const fetchImpl = vi.fn(
      async (url: string) => new Response(JSON.stringify({ peerId: ids[url] }), { status: 200 })
    );
    const resolved = await resolveCustodyPeerIds(
      {
        providerHumanId: 'human-matthew-manager',
        providerArchetype: 'desktop',
        receiverHumanId: 'human-jessica-spouse',
        receiverArchetype: 'desktop',
        blobHash: 'sha256-deadbeef',
        blobSizeBytes: 1,
      },
      { fetchImpl }
    );
    expect(resolved.provider).toBe(REAL_ID);
    expect(resolved.receiver).toBe('12D3KooWBhYqzhQ8XK2v9PqQ7TZxGw7vM1nRkLs5uJcDeFgHiJkL');
  });
});

describe('defaultCustodyPairs triad fixture', () => {
  let prevHash: string | undefined;
  let prevSize: string | undefined;
  beforeAll(() => {
    prevHash = process.env.CONTENT_BLOB_HASH;
    prevSize = process.env.CONTENT_BLOB_SIZE_BYTES;
    process.env.CONTENT_BLOB_HASH = 'sha256-cafebabe';
    process.env.CONTENT_BLOB_SIZE_BYTES = '64';
  });
  afterAll(() => {
    if (prevHash === undefined) delete process.env.CONTENT_BLOB_HASH;
    else process.env.CONTENT_BLOB_HASH = prevHash;
    if (prevSize === undefined) delete process.env.CONTENT_BLOB_SIZE_BYTES;
    else process.env.CONTENT_BLOB_SIZE_BYTES = prevSize;
  });

  it('includes the james fixture pairs with formation-output provenance', () => {
    const pairs = defaultCustodyPairs();
    // 2 M1 pairs (matthew<->jessica) + 4 fixture pairs (james with each parent, both directions)
    expect(pairs).toHaveLength(6);
    const jamesPairs = pairs.filter(
      p => p.providerHumanId === 'human-james-student' || p.receiverHumanId === 'human-james-student'
    );
    expect(jamesPairs).toHaveLength(4);
    for (const p of jamesPairs) expect(p.fixture).toBe('formation-output');
    const nonFixturePairs = pairs.filter(p => !p.fixture);
    expect(nonFixturePairs).toHaveLength(2);
  });

  it('stamps fixture provenance into the commitment body metadata', () => {
    const body = buildCustodyCommitmentBody({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-james-student', receiverArchetype: 'mobile',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 1, fixture: 'formation-output',
    });
    expect(body.metadata.fixture).toBe('formation-output');
    expect(body.metadata.retireAt).toBe('ceremony-landing');
  });
});
