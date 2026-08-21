import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach, vi } from 'vitest';
import {
  activateCustodyCommitments,
  activationDecision,
  buildCustodyCommitmentBody,
  defaultCustodyPairs,
  resolveCustodyBlobDescriptor,
  resolveCustodyPeerIds,
  seedCapacityPledge,
  seedCustodyCommitments,
  type CapacityRatioAttestation,
  type CommitmentClient,
  type CustodyPair,
  type CustodyPeerIds,
  type ResolvedCustodyPair,
} from '../seed-commitments.js';
import { clearPeerIdCache, deterministicPeerId, resolvePeerId, storageUrlForHuman } from '../peer-id.js';

describe('buildCustodyCommitmentBody', () => {
  const pair: ResolvedCustodyPair = {
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

  it('preserves content identity and artifact role for stale-pledge retirement', () => {
    const body = buildCustodyCommitmentBody(
      {
        ...pair,
        contentId: 'elohim-host-landing',
        artifactRole: 'ssr-server',
      },
      agentCids,
    );
    expect(body.metadata).toMatchObject({
      contentId: 'elohim-host-landing',
      artifactRole: 'ssr-server',
    });
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
// resolveCustodyBlobDescriptor — contentId + explicit artifact role.
// A content row's blobHash (browser/primary artifact) and serverBlobHash
// (optional SSR server bundle) intentionally name different bytes. The
// custody join uses the exact artifact selected by the caller; no fallback
// can silently switch the pledge to a different artifact.
// =============================================================================

describe('resolveCustodyBlobDescriptor', () => {
  const basePair = {
    providerHumanId: 'human-matthew-manager',
    providerArchetype: 'desktop' as const,
    receiverHumanId: 'human-matthew-manager',
    receiverArchetype: 'desktop' as const,
  };

  it('blobHash pair: passes the declared hash/size through unchanged (back-compat, no fetch)', async () => {
    const fetchImpl = vi.fn();
    const resolved = await resolveCustodyBlobDescriptor(
      { ...basePair, blobHash: 'sha256-deadbeef', blobSizeBytes: 42 },
      { fetchImpl },
    );
    expect(resolved).toEqual({
      blobHash: 'sha256-deadbeef',
      blobSizeBytes: 42,
    });
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('ssr-server pair: selects serverBlobHash even when blobHash is present', async () => {
    const fetchImpl = vi.fn(async (url: string) => {
      expect(url).toBe(`${storageUrlForHuman('human-matthew-manager')}/db/content/elohim-host-landing`);
      return new Response(
        JSON.stringify({
          serverBlobHash: 'sha256-56adaebb',
          blobHash: 'sha256-a12218a5',
          contentSizeBytes: 999,
        }),
        { status: 200 },
      );
    });
    const resolved = await resolveCustodyBlobDescriptor(
      {
        ...basePair,
        contentId: 'elohim-host-landing',
        artifactRole: 'ssr-server',
      },
      { fetchImpl },
    );
    expect(resolved.blobHash).toBe('sha256-56adaebb');
    expect(resolved.blobSizeBytes).toBe(999);
  });

  it('content pair: selects blobHash even when serverBlobHash is present', async () => {
    const fetchImpl = vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            blobHash: 'sha256-a12218a5',
            serverBlobHash: 'sha256-56adaebb',
            contentSizeBytes: 10,
          }),
          { status: 200 },
        ),
    );
    const resolved = await resolveCustodyBlobDescriptor(
      {
        ...basePair,
        contentId: 'elohim-host-landing',
        artifactRole: 'content',
      },
      { fetchImpl },
    );
    expect(resolved.blobHash).toBe('sha256-a12218a5');
  });

  it('contentId pair requires an explicit artifact role', async () => {
    await expect(
      resolveCustodyBlobDescriptor({
        ...basePair,
        contentId: 'elohim-host-landing',
      }),
    ).rejects.toThrow(/requires artifactRole/);
  });

  it('does not fall back from a missing selected artifact to the other field', async () => {
    const fetchImpl = vi.fn(
      async () =>
        new Response(JSON.stringify({ blobHash: 'sha256-a12218a5', contentSizeBytes: 10 }), {
          status: 200,
        }),
    );
    await expect(
      resolveCustodyBlobDescriptor(
        {
          ...basePair,
          contentId: 'elohim-host-landing',
          artifactRole: 'ssr-server',
        },
        { fetchImpl },
      ),
    ).rejects.toThrow(/has no serverBlobHash/);
  });

  it("contentId pair: an explicitly declared blobSizeBytes wins over the row's contentSizeBytes", async () => {
    const fetchImpl = vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            serverBlobHash: 'sha256-56adaebb',
            contentSizeBytes: 999,
          }),
          { status: 200 },
        ),
    );
    const resolved = await resolveCustodyBlobDescriptor(
      {
        ...basePair,
        contentId: 'elohim-host-landing',
        artifactRole: 'ssr-server',
        blobSizeBytes: 5,
      },
      { fetchImpl },
    );
    expect(resolved.blobSizeBytes).toBe(5);
  });

  it('declaring BOTH blobHash and contentId is a fail-fast authoring error', async () => {
    await expect(
      resolveCustodyBlobDescriptor({
        ...basePair,
        blobHash: 'sha256-deadbeef',
        blobSizeBytes: 1,
        contentId: 'x',
      }),
    ).rejects.toThrow(/declares BOTH blobHash and contentId/);
  });

  it('declaring NEITHER blobHash nor contentId is a fail-fast authoring error', async () => {
    await expect(resolveCustodyBlobDescriptor({ ...basePair })).rejects.toThrow(
      /declares NEITHER blobHash nor contentId/,
    );
  });

  it('an unresolvable content row (fetch failure) is an honest seed failure, no fallback', async () => {
    const fetchImpl = vi.fn(async () => {
      throw new Error('connect ECONNREFUSED');
    });
    await expect(
      resolveCustodyBlobDescriptor(
        {
          ...basePair,
          contentId: 'elohim-host-landing',
          artifactRole: 'ssr-server',
        },
        { fetchImpl },
      ),
    ).rejects.toThrow('connect ECONNREFUSED');
  });

  it('a content row without the selected artifact is a fail-fast error', async () => {
    const fetchImpl = vi.fn(async () => new Response(JSON.stringify({ contentSizeBytes: 5 }), { status: 200 }));
    await expect(
      resolveCustodyBlobDescriptor(
        {
          ...basePair,
          contentId: 'elohim-host-landing',
          artifactRole: 'ssr-server',
        },
        { fetchImpl },
      ),
    ).rejects.toThrow(/has no serverBlobHash/);
  });

  it('an unresolvable blobSizeBytes (no declared value, no contentSizeBytes on the row, no matching env, blob GET 404s) fails fast', async () => {
    let prevHash: string | undefined;
    let prevSize: string | undefined;
    prevHash = process.env.CONTENT_BLOB_HASH;
    prevSize = process.env.CONTENT_BLOB_SIZE_BYTES;
    delete process.env.CONTENT_BLOB_HASH;
    delete process.env.CONTENT_BLOB_SIZE_BYTES;
    try {
      const fetchImpl = vi.fn(async (url: string) => {
        if (url.endsWith('/db/content/elohim-host-landing')) {
          return new Response(JSON.stringify({ serverBlobHash: 'sha256-56adaebb' }), { status: 200 });
        }
        if (url.includes('/blob/')) {
          return new Response('not found', { status: 404 });
        }
        throw new Error(`unexpected fetch: ${url}`);
      });
      await expect(
        resolveCustodyBlobDescriptor(
          {
            ...basePair,
            contentId: 'elohim-host-landing',
            artifactRole: 'ssr-server',
          },
          { fetchImpl },
        ),
      ).rejects.toThrow(/cannot resolve blobSizeBytes/);
    } finally {
      if (prevHash === undefined) delete process.env.CONTENT_BLOB_HASH;
      else process.env.CONTENT_BLOB_HASH = prevHash;
      if (prevSize === undefined) delete process.env.CONTENT_BLOB_SIZE_BYTES;
      else process.env.CONTENT_BLOB_SIZE_BYTES = prevSize;
    }
  });

  it('falls back to CONTENT_BLOB_SIZE_BYTES when CONTENT_BLOB_HASH names the same resolved blob', async () => {
    let prevHash: string | undefined;
    let prevSize: string | undefined;
    prevHash = process.env.CONTENT_BLOB_HASH;
    prevSize = process.env.CONTENT_BLOB_SIZE_BYTES;
    process.env.CONTENT_BLOB_HASH = 'sha256-56adaebb';
    process.env.CONTENT_BLOB_SIZE_BYTES = '17905368';
    try {
      const fetchImpl = vi.fn(async (url: string) => {
        if (url.endsWith('/db/content/elohim-host-landing')) {
          return new Response(JSON.stringify({ serverBlobHash: 'sha256-56adaebb' }), { status: 200 });
        }
        throw new Error(`unexpected fetch (should not need /blob/): ${url}`);
      });
      const resolved = await resolveCustodyBlobDescriptor(
        {
          ...basePair,
          contentId: 'elohim-host-landing',
          artifactRole: 'ssr-server',
        },
        { fetchImpl },
      );
      expect(resolved.blobSizeBytes).toBe(17905368);
      // Never needed the /blob/ GET fallback — the env tier resolved it first.
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    } finally {
      if (prevHash === undefined) delete process.env.CONTENT_BLOB_HASH;
      else process.env.CONTENT_BLOB_HASH = prevHash;
      if (prevSize === undefined) delete process.env.CONTENT_BLOB_SIZE_BYTES;
      else process.env.CONTENT_BLOB_SIZE_BYTES = prevSize;
    }
  });

  it('CONTENT_BLOB_SIZE_BYTES is ignored when CONTENT_BLOB_HASH names a DIFFERENT blob (falls through to the GET tier)', async () => {
    let prevHash: string | undefined;
    let prevSize: string | undefined;
    prevHash = process.env.CONTENT_BLOB_HASH;
    prevSize = process.env.CONTENT_BLOB_SIZE_BYTES;
    process.env.CONTENT_BLOB_HASH = 'sha256-someOTHERblob';
    process.env.CONTENT_BLOB_SIZE_BYTES = '999';
    try {
      const fetchImpl = vi.fn(async (url: string) => {
        if (url.endsWith('/db/content/elohim-host-landing')) {
          return new Response(JSON.stringify({ serverBlobHash: 'sha256-56adaebb' }), { status: 200 });
        }
        if (url.includes('/blob/sha256-56adaebb')) {
          return new Response(null, { status: 200, headers: { 'content-length': '17905368' } });
        }
        throw new Error(`unexpected fetch: ${url}`);
      });
      const resolved = await resolveCustodyBlobDescriptor(
        {
          ...basePair,
          contentId: 'elohim-host-landing',
          artifactRole: 'ssr-server',
        },
        { fetchImpl },
      );
      // NOT 999 — the mismatched env value must never be trusted for a
      // different blob; the real size came from the GET fallback instead.
      expect(resolved.blobSizeBytes).toBe(17905368);
    } finally {
      if (prevHash === undefined) delete process.env.CONTENT_BLOB_HASH;
      else process.env.CONTENT_BLOB_HASH = prevHash;
      if (prevSize === undefined) delete process.env.CONTENT_BLOB_SIZE_BYTES;
      else process.env.CONTENT_BLOB_SIZE_BYTES = prevSize;
    }
  });

  it('falls back to GET /blob/<hash> Content-Length when neither the row nor a matching env has a usable size', async () => {
    let prevHash: string | undefined;
    let prevSize: string | undefined;
    prevHash = process.env.CONTENT_BLOB_HASH;
    prevSize = process.env.CONTENT_BLOB_SIZE_BYTES;
    delete process.env.CONTENT_BLOB_HASH;
    delete process.env.CONTENT_BLOB_SIZE_BYTES;
    try {
      const fetchImpl = vi.fn(async (url: string) => {
        if (url.endsWith('/db/content/elohim-host-landing')) {
          return new Response(JSON.stringify({ serverBlobHash: 'sha256-56adaebb' }), { status: 200 });
        }
        if (url.includes('/blob/sha256-56adaebb')) {
          // GET, not HEAD — content-addressed blob routes 404 on HEAD even
          // when GET 200s on this mesh; the resolver must never issue HEAD.
          return new Response(null, { status: 200, headers: { 'content-length': '17905368' } });
        }
        throw new Error(`unexpected fetch: ${url}`);
      });
      const resolved = await resolveCustodyBlobDescriptor(
        {
          ...basePair,
          contentId: 'elohim-host-landing',
          artifactRole: 'ssr-server',
        },
        { fetchImpl },
      );
      expect(resolved.blobSizeBytes).toBe(17905368);
      expect(fetchImpl).toHaveBeenCalledTimes(2);
    } finally {
      if (prevHash === undefined) delete process.env.CONTENT_BLOB_HASH;
      else process.env.CONTENT_BLOB_HASH = prevHash;
      if (prevSize === undefined) delete process.env.CONTENT_BLOB_SIZE_BYTES;
      else process.env.CONTENT_BLOB_SIZE_BYTES = prevSize;
    }
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
  const pair: ResolvedCustodyPair = {
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

describe('seedCustodyCommitments (offline) — contentId pair drives create + activate', () => {
  const REAL = 'uhCAkmatthewagentkey';
  const contentIdPair: CustodyPair = {
    providerHumanId: 'human-matthew-manager',
    providerArchetype: 'desktop',
    receiverHumanId: 'human-matthew-manager',
    receiverArchetype: 'desktop',
    contentId: 'elohim-host-landing',
    artifactRole: 'ssr-server',
  };

  beforeEach(() => clearPeerIdCache());
  afterEach(() => vi.restoreAllMocks());

  it('resolves serverBlobHash via contentId, creates the commitment, then issues the activation PATCH', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const fetchImpl = vi.fn(async (url: string) => {
      if (url.endsWith('/db/content/elohim-host-landing')) {
        return new Response(
          JSON.stringify({
            serverBlobHash: 'sha256-56adaebb',
            blobHash: 'sha256-a12218a5',
            contentSizeBytes: 999,
          }),
          { status: 200 },
        );
      }
      if (url.endsWith('/auth/me')) {
        return new Response(JSON.stringify({ agentPubKey: REAL }), {
          status: 200,
        });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    // Stateful, mirroring the real storage/doorway backend: 404 before the row
    // exists (the seedCustodyCommitments create-phase pre-check), 200
    // proposed once createCommitment has run (the activation phase's read).
    let rowCreated = false;
    const createCommitment = vi.fn(async (_body: { resourceClassifiedAs: string[] }) => {
      rowCreated = true;
      return new Response('{}', { status: 200 });
    });
    const getCommitment = vi.fn(async (id: string) => {
      if (!rowCreated) return new Response('not found', { status: 404 });
      return new Response(JSON.stringify({ id, state: 'proposed' }), { status: 200 });
    });
    const patchCommitmentState = vi.fn(async (_id: string, _state: string) => new Response('{}', { status: 200 }));
    const client = {
      createCommitment,
      getCommitment,
      patchCommitmentState,
    } as unknown as CommitmentClient;

    await seedCustodyCommitments(client, [contentIdPair], { fetchImpl });

    // Created with the explicitly selected SSR-server artifact address.
    expect(createCommitment).toHaveBeenCalledTimes(1);
    const sentBody = createCommitment.mock.calls[0][0] as {
      resourceClassifiedAs: string[];
    };
    expect(sentBody.resourceClassifiedAs).toEqual(['sha256-56adaebb']);

    // getCommitment runs TWICE: once as the create-phase pre-check (404 —
    // the row doesn't exist yet) and once as the activation-phase read (200
    // proposed, after createCommitment ran). The activation step then issued
    // the PATCH for that same (proposed) row.
    expect(getCommitment).toHaveBeenCalledTimes(2);
    expect(patchCommitmentState).toHaveBeenCalledTimes(1);
    expect(patchCommitmentState.mock.calls[0][1]).toBe('active');
  });

  it('a pair whose id already exists is recognized via GET and never re-POSTed — a re-run reports already-exists, not created', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const fetchImpl = vi.fn(async (url: string) => {
      if (url.endsWith('/db/content/elohim-host-landing')) {
        return new Response(
          JSON.stringify({ serverBlobHash: 'sha256-56adaebb', contentSizeBytes: 999 }),
          { status: 200 },
        );
      }
      if (url.endsWith('/auth/me')) {
        return new Response(JSON.stringify({ agentPubKey: REAL }), { status: 200 });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    // The row already exists — GET returns it 200/active on every call, the
    // same shape a re-POST to the real backend upserts-and-returns-201 for
    // (verified live: re-POSTing an existing id returns HTTP 201 carrying
    // the ORIGINAL row unchanged — never 409). createCommitment must never
    // be reached at all once the pre-check finds the row.
    const createCommitment = vi.fn(async () => new Response('{}', { status: 201 }));
    const getCommitment = vi.fn(
      async (id: string) => new Response(JSON.stringify({ id, state: 'active' }), { status: 200 }),
    );
    const patchCommitmentState = vi.fn(async () => new Response('{}', { status: 200 }));
    const client = {
      createCommitment,
      getCommitment,
      patchCommitmentState,
    } as unknown as CommitmentClient;

    const result = await seedCustodyCommitments(client, [contentIdPair], { fetchImpl });

    expect(createCommitment).not.toHaveBeenCalled();
    expect(result.created).toBe(0);
    expect(result.alreadyExists).toBe(1);
    // Activation still runs and recognizes the already-active row (no PATCH).
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
    // 2 M1 pairs (matthew<->jessica) + 4 fixture pairs (james with each parent,
    // both directions) + 1 self-custody contentId pair (elohim-host-landing)
    expect(pairs).toHaveLength(7);
    const jamesPairs = pairs.filter(
      (p) => p.providerHumanId === 'human-james-son' || p.receiverHumanId === 'human-james-son',
    );
    expect(jamesPairs).toHaveLength(4);
    for (const p of jamesPairs) expect(p.fixture).toBe('formation-output');
    const nonFixturePairs = pairs.filter((p) => !p.fixture);
    expect(nonFixturePairs).toHaveLength(3);
  });

  it('includes the elohim-host-landing self-custody pair, resolved via contentId not a fixture blobHash', () => {
    const pairs = defaultCustodyPairs();
    const selfCustody = pairs.find((p) => p.contentId === 'elohim-host-landing');
    expect(selfCustody).toBeDefined();
    expect(selfCustody?.providerHumanId).toBe('human-matthew-manager');
    expect(selfCustody?.receiverHumanId).toBe('human-matthew-manager');
    expect(selfCustody?.blobHash).toBeUndefined();
    expect(selfCustody?.artifactRole).toBe('ssr-server');
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

// =============================================================================
// seedCapacityPledge — fail-soft consent-legibility read + idempotent pledge
// =============================================================================

describe('seedCapacityPledge (offline, injected mock client)', () => {
  const RATIOS: CapacityRatioAttestation = {
    commonsPct: 20,
    dwellingPct: 40,
    collectivePct: 25,
    freePct: 15,
    effectiveRatioCid: 'bafkrei-effective-ratios',
  };

  afterEach(() => vi.restoreAllMocks());

  it('happy path: posts the attestation verbatim from the GET', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    const getCapacityRatios = vi.fn(async () => new Response(JSON.stringify(RATIOS), { status: 200 }));
    const createCapacityPledge = vi.fn(async (_body: unknown) => new Response('{}', { status: 200 }));
    const client = {
      getCapacityRatios,
      createCapacityPledge,
    } as unknown as CommitmentClient;

    await seedCapacityPledge(client);

    expect(getCapacityRatios).toHaveBeenCalledTimes(1);
    expect(createCapacityPledge).toHaveBeenCalledTimes(1);
    const sentBody = createCapacityPledge.mock.calls[0][0] as {
      variant: string;
      commonsBytes: number;
      bounds: { ratePerMinute: number; reachCeiling: string };
      ratioAttestation: CapacityRatioAttestation;
    };
    expect(sentBody.variant).toBe('capacity');
    expect(sentBody.commonsBytes).toBe(25 * 1024 * 1024 * 1024);
    expect(sentBody.bounds).toEqual({ ratePerMinute: 12, reachCeiling: 'commons' });
    // The attestation MUST be exactly what the GET returned — the exact-match
    // donut check in replicates_commons_validator rejects anything else.
    expect(sentBody.ratioAttestation).toEqual(RATIOS);
  });

  it('404 on GET: skips without throwing, never calls the POST', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const getCapacityRatios = vi.fn(async () => new Response('not found', { status: 404 }));
    const createCapacityPledge = vi.fn();
    const client = {
      getCapacityRatios,
      createCapacityPledge,
    } as unknown as CommitmentClient;

    await expect(seedCapacityPledge(client)).resolves.toBeUndefined();

    expect(createCapacityPledge).not.toHaveBeenCalled();
  });

  it('501 on GET: skips without throwing, never calls the POST', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const getCapacityRatios = vi.fn(async () => new Response('not implemented', { status: 501 }));
    const createCapacityPledge = vi.fn();
    const client = {
      getCapacityRatios,
      createCapacityPledge,
    } as unknown as CommitmentClient;

    await expect(seedCapacityPledge(client)).resolves.toBeUndefined();

    expect(createCapacityPledge).not.toHaveBeenCalled();
  });

  it('GET throws (network error): logs and returns without throwing', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const getCapacityRatios = vi.fn(async () => {
      throw new Error('connect ECONNREFUSED');
    });
    const createCapacityPledge = vi.fn();
    const client = {
      getCapacityRatios,
      createCapacityPledge,
    } as unknown as CommitmentClient;

    await expect(seedCapacityPledge(client)).resolves.toBeUndefined();

    expect(createCapacityPledge).not.toHaveBeenCalled();
  });

  it('POST failure: logs and does not throw', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const getCapacityRatios = vi.fn(async () => new Response(JSON.stringify(RATIOS), { status: 200 }));
    const createCapacityPledge = vi.fn(
      async () => new Response(JSON.stringify({ error: 'donut check failed' }), { status: 400 }),
    );
    const client = {
      getCapacityRatios,
      createCapacityPledge,
    } as unknown as CommitmentClient;

    await expect(seedCapacityPledge(client)).resolves.toBeUndefined();

    expect(createCapacityPledge).toHaveBeenCalledTimes(1);
  });

  it('POST throws (network error): logs and does not throw', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const getCapacityRatios = vi.fn(async () => new Response(JSON.stringify(RATIOS), { status: 200 }));
    const createCapacityPledge = vi.fn(async () => {
      throw new Error('connect ECONNREFUSED');
    });
    const client = {
      getCapacityRatios,
      createCapacityPledge,
    } as unknown as CommitmentClient;

    await expect(seedCapacityPledge(client)).resolves.toBeUndefined();
  });
});
