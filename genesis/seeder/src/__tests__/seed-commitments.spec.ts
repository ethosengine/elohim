import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach, vi } from 'vitest';
import {
  activateCustodyCommitments,
  activationDecision,
  buildCustodyCommitmentBody,
  defaultCustodyPairs,
  resolveCustodyPeerIds,
  type CommitmentClient,
  type CustodyPair,
  type CustodyPeerIds,
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
  const agentCids: CustodyPeerIds = {
    provider: 'uhCAkmatthewagentkey',
    receiver: 'uhCAkterrenceagentkey',
  };

  it('action is exactly "custody-blob"', () => {
    const body = buildCustodyCommitmentBody(pair, agentCids);
    expect(body.action).toBe('custody-blob');
  });

  it('provider and receiver are Holochain agent CIDs, not human slugs or transport IDs', () => {
    const body = buildCustodyCommitmentBody(pair, agentCids);
    expect(body.provider).toMatch(/^uhCAk/);
    expect(body.receiver).toMatch(/^uhCAk/);
    expect(body.provider).not.toMatch(/^human-/);
    expect(body.receiver).not.toMatch(/^human-/);
    expect(body.provider).not.toMatch(/^12D3/);
    expect(body.receiver).not.toMatch(/^12D3/);
  });

  it('resourceClassifiedAs is a ValueFlows list holding the sha256- blob hash', () => {
    const body = buildCustodyCommitmentBody(pair, agentCids);
    // List shape per CreateReaCommitmentInputView (ValueFlows resourceClassifiedAs
    // is a classification list) — stored as a JSON array so ReaCommitmentView's
    // parse-as-array round-trips it.
    expect(body.resourceClassifiedAs).toEqual(['sha256-deadbeef']);
  });

  it('resourceQuantity uses bytes-as-integer with hasUnit "B"', () => {
    const body = buildCustodyCommitmentBody(pair, agentCids);
    expect(body.resourceQuantity.hasNumericalValue).toBe(12345);
    expect(body.resourceQuantity.hasUnit).toBe('B');
  });

  it('id is distinct per (provider_peer, receiver_peer, blob_hash) tuple', () => {
    const a = buildCustodyCommitmentBody(pair, agentCids);
    const b = buildCustodyCommitmentBody(
      {
        ...pair,
        blobHash: 'sha256-feedface',
      },
      agentCids,
    );
    expect(a.id).not.toBe(b.id);
  });

  it('id is deterministic — same tuple → same id (idempotent re-runs)', () => {
    const a = buildCustodyCommitmentBody(pair, agentCids);
    const b = buildCustodyCommitmentBody(pair, agentCids);
    expect(a.id).toBe(b.id);
  });

  it('uses resolved Holochain agent keys when provided (and the content-addressed id follows)', () => {
    const resolved = {
      provider: 'uhCAkmatthewagentkey',
      receiver: 'uhCAkjessicaagentkey',
    };
    const body = buildCustodyCommitmentBody(pair, resolved);
    expect(body.provider).toBe(resolved.provider);
    expect(body.receiver).toBe(resolved.receiver);
    expect(body.id).not.toBe(buildCustodyCommitmentBody(pair, agentCids).id);
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
    const id = await resolvePeerId('human-matthew-manager', 'desktop', {
      fetchImpl,
    });
    expect(id).toBe(REAL_ID);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(fetchImpl.mock.calls[0][0]).toBe(`${storageUrlForHuman('human-matthew-manager')}/p2p/status`);
  });

  it('derives the storage host from the humanId short name (alpha convention)', () => {
    expect(storageUrlForHuman('human-matthew-manager')).toBe(
      'http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090',
    );
  });

  it('caches per host within a run — one probe serves repeated resolutions', async () => {
    const fetchImpl = okFetch();
    const a = await resolvePeerId('human-matthew-manager', 'desktop', {
      fetchImpl,
    });
    const b = await resolvePeerId('human-matthew-manager', 'desktop', {
      fetchImpl,
    });
    expect(a).toBe(b);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('fallback: unreachable pod → deterministic Stage-1 id + loud warn naming the host', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const fetchImpl = vi.fn(async () => {
      throw new Error('connect ECONNREFUSED');
    });
    const id = await resolvePeerId('human-jessica-spouse', 'desktop', {
      fetchImpl,
    });
    expect(id).toBe(deterministicPeerId('human-jessica-spouse', 'desktop'));
    expect(warn).toHaveBeenCalledTimes(1);
    const message = warn.mock.calls[0][0] as string;
    expect(message).toContain(storageUrlForHuman('human-jessica-spouse'));
    expect(message).toContain('Custody sweep will not match this peer');
  });

  it('fallback: 200 response missing peerId → deterministic Stage-1 id', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const fetchImpl = vi.fn(async () => new Response('{}', { status: 200 }));
    const id = await resolvePeerId('human-jessica-spouse', 'desktop', {
      fetchImpl,
    });
    expect(id).toBe(deterministicPeerId('human-jessica-spouse', 'desktop'));
  });

  it('failed probes are cached too — seed and activate phases compute identical ids', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const fetchImpl = vi.fn(async () => {
      throw new Error('timeout');
    });
    const a = await resolvePeerId('human-james-son', 'mobile', { fetchImpl });
    const b = await resolvePeerId('human-james-son', 'mobile', { fetchImpl });
    expect(a).toBe(b);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('resolveCustodyPeerIds preserves role semantics in the agent-CID namespace', async () => {
    const ids: Record<string, string> = {
      'http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090/auth/me': 'uhCAkmatthewagentkey',
      'http://elohim-jessica-alpha.elohim-alpha.svc.cluster.local:8090/auth/me': 'uhCAkjessicaagentkey',
    };
    const fetchImpl = vi.fn(
      async (url: string) =>
        new Response(JSON.stringify({ agentPubKey: ids[url] }), {
          status: 200,
        }),
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
      { fetchImpl },
    );
    expect(resolved.provider).toBe('uhCAkmatthewagentkey');
    expect(resolved.receiver).toBe('uhCAkjessicaagentkey');
  });

  it('refuses a transport peer ID rather than creating an unjoinable commitment', async () => {
    const fetchImpl = vi.fn(async () => new Response(JSON.stringify({ agentPubKey: REAL_ID }), { status: 200 }));
    await expect(
      resolveCustodyPeerIds(
        {
          providerHumanId: 'human-matthew-manager',
          providerArchetype: 'desktop',
          receiverHumanId: 'human-jessica-spouse',
          receiverArchetype: 'desktop',
          blobHash: 'sha256-deadbeef',
          blobSizeBytes: 1,
        },
        { fetchImpl },
      ),
    ).rejects.toThrow('Holochain agentPubKey');
  });
});

// =============================================================================
// Idempotent activation — recognize an already-active commitment as a
// meaningful success instead of re-submitting it (a PATCH active→active still
// routes through the conductor/projection write path and gets shed under seed
// back-pressure with 503 {"status":"catching-up"} — the genesis stage went
// Unstable on exactly this no-op re-activation of rows a prior run had already
// activated).
// =============================================================================

describe('activationDecision (idempotent activation rule)', () => {
  it('an already-active row is a meaningful success — skip the redundant write', () => {
    expect(activationDecision('active')).toBe('skip-active');
  });

  it('a proposed row still needs the activation write', () => {
    expect(activationDecision('proposed')).toBe('activate');
  });

  it('a missing row (404 → null state) is reported missing, never activated', () => {
    expect(activationDecision(null)).toBe('missing');
  });

  it('an unknown/empty state falls through to activate (fail toward the write, not silent skip)', () => {
    expect(activationDecision('')).toBe('activate');
  });
});

describe('activateCustodyCommitments (offline, injected client + agent-key probe)', () => {
  const REAL = 'uhCAkmatthewagentkey';
  const pair: CustodyPair = {
    providerHumanId: 'human-matthew-manager',
    providerArchetype: 'desktop',
    receiverHumanId: 'human-jessica-spouse',
    receiverArchetype: 'desktop',
    blobHash: 'sha256-deadbeef',
    blobSizeBytes: 1,
  };

  beforeEach(() => clearPeerIdCache());
  afterEach(() => vi.restoreAllMocks());

  // Every /auth/me probe returns the same agent key → deterministic body ids
  // computed offline; no live pods touched.
  const peerFetch = () => vi.fn(async () => new Response(JSON.stringify({ agentPubKey: REAL }), { status: 200 }));

  it('already-active commitment: recognized via GET, NOT re-PATCHed', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    const expectedId = buildCustodyCommitmentBody(pair, {
      provider: REAL,
      receiver: REAL,
    }).id;

    const getCommitment = vi.fn(
      async (id: string) => new Response(JSON.stringify({ id, state: 'active' }), { status: 200 }),
    );
    const patchCommitmentState = vi.fn();
    const client = {
      getCommitment,
      patchCommitmentState,
    } as unknown as CommitmentClient;

    await activateCustodyCommitments(client, [pair], {
      fetchImpl: peerFetch(),
    });

    expect(getCommitment).toHaveBeenCalledWith(expectedId);
    expect(patchCommitmentState).not.toHaveBeenCalled();
  });

  it('proposed commitment: PATCHed to active exactly once', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    const getCommitment = vi.fn(
      async (id: string) =>
        new Response(JSON.stringify({ id, state: 'proposed' }), {
          status: 200,
        }),
    );
    const patchCommitmentState = vi.fn(async (_id: string, _state: string) => new Response('{}', { status: 200 }));
    const client = {
      getCommitment,
      patchCommitmentState,
    } as unknown as CommitmentClient;

    await activateCustodyCommitments(client, [pair], {
      fetchImpl: peerFetch(),
    });

    expect(patchCommitmentState).toHaveBeenCalledTimes(1);
    expect(patchCommitmentState.mock.calls[0][1]).toBe('active');
  });

  it('missing commitment (GET 404): non-fatal, no PATCH attempted', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const getCommitment = vi.fn(async () => new Response('not found', { status: 404 }));
    const patchCommitmentState = vi.fn();
    const client = {
      getCommitment,
      patchCommitmentState,
    } as unknown as CommitmentClient;

    await activateCustodyCommitments(client, [pair], {
      fetchImpl: peerFetch(),
    });

    expect(patchCommitmentState).not.toHaveBeenCalled();
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
      (p) => p.providerHumanId === 'human-james-son' || p.receiverHumanId === 'human-james-son',
    );
    expect(jamesPairs).toHaveLength(4);
    for (const p of jamesPairs) expect(p.fixture).toBe('formation-output');
    const nonFixturePairs = pairs.filter((p) => !p.fixture);
    expect(nonFixturePairs).toHaveLength(2);
  });

  it('stamps fixture provenance into the commitment body metadata', () => {
    const body = buildCustodyCommitmentBody(
      {
        providerHumanId: 'human-jessica-spouse',
        providerArchetype: 'desktop',
        receiverHumanId: 'human-james-son',
        receiverArchetype: 'mobile',
        blobHash: 'sha256-deadbeef',
        blobSizeBytes: 1,
        fixture: 'formation-output',
      },
      { provider: 'uhCAkjessicaagentkey', receiver: 'uhCAkjamesagentkey' },
    );
    expect(body.metadata.fixture).toBe('formation-output');
    expect(body.metadata.retireAt).toBe('ceremony-landing');
  });
});
