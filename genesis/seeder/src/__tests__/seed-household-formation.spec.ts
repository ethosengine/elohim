import { createHash } from 'node:crypto';
import { describe, it, expect, vi, afterEach } from 'vitest';
import {
  buildHouseholdCharter, buildCeremonyCustodyInput, HOUSEHOLD_MEMBERS,
  resolveExistingCollectiveCid,
} from '../seed-household-formation.js';
import {
  clearPeerIdCache, deterministicPeerId, resolvePeerId, storageUrlForHuman,
} from '../peer-id.js';

describe('buildHouseholdCharter', () => {
  it('declares household kind, rubric, and the family-dowell slug alias', () => {
    const charter = JSON.parse(buildHouseholdCharter());
    expect(charter.kind).toBe('household');
    expect(charter.rubric).toBe('recognition-of-given');
    expect(charter.slugAlias).toBe('family-dowell');
  });
});

describe('buildCeremonyCustodyInput', () => {
  it('builds a snake_case zome input with ceremony provenance and collective scope', () => {
    const input = buildCeremonyCustodyInput({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 64,
      collectiveCid: 'collective:uhCkkFAKE',
    });
    expect(input.action).toBe('custody-blob');
    expect(input.provider.startsWith('12D3KooW')).toBe(true);
    expect(input.in_scope_of).toEqual(['collective:uhCkkFAKE']);
    expect(JSON.parse(input.metadata_json!).seedGeneration).toBe('ceremony');
    expect(JSON.parse(input.metadata_json!).providerHumanId).toBe('human-jessica-spouse');
  });

  it('uses resolved REAL peer ids when provided and content-addresses the id from them', () => {
    // Same contract as buildCustodyCommitmentBody(pair, peerIds): the live
    // seed path resolves Stage-2 ids first; the builder must carry them
    // verbatim or the storage custody sweep (provider == self peer id) and
    // the receiver placement-gap leg can never match this row.
    const peerIds = {
      provider: '12D3KooWLiveJessicaPodRealPeerIdAAAAAAAAAAAAAA',
      receiver: '12D3KooWLiveMatthewPodRealPeerIdBBBBBBBBBBBBBB',
    };
    const input = buildCeremonyCustodyInput({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 64,
      collectiveCid: 'collective:uhCkkFAKE',
    }, peerIds);
    expect(input.provider).toBe(peerIds.provider);
    expect(input.receiver).toBe(peerIds.receiver);
    const expectedDigest = createHash('sha256')
      .update(`${peerIds.provider}|${peerIds.receiver}|sha256-deadbeef`)
      .digest('hex')
      .slice(0, 16);
    expect(input.id).toBe(`custody-blob-${expectedDigest}`);
    // Provenance metadata keeps naming the humans, not the peer ids.
    expect(JSON.parse(input.metadata_json!).providerHumanId).toBe('human-jessica-spouse');
  });

  it('custody peer ids cohere with the agent-peer binding ids for the same human/archetype', async () => {
    // N9 binding-coherence guardrail. Two seeding surfaces resolve peer ids
    // independently: seed-agent-bindings (peer_identity_bindings entries) and
    // the ceremony custody input built here. Both MUST go through
    // resolvePeerId(humanId, archetype) against the same /p2p/status — drift
    // between them disarms the storage custody sweep silently (the sweep kicks
    // a blob fetch iff commitment.provider == the node's REAL peer id, so a
    // custody row keyed differently from the binding never matches anything).
    clearPeerIdCache();
    const REAL_JESSICA = '12D3KooWLiveJessicaPodRealPeerIdAAAAAAAAAAAAAA';
    const REAL_MATTHEW = '12D3KooWLiveMatthewPodRealPeerIdBBBBBBBBBBBBBB';
    // Per-host /p2p/status responses, keyed exactly like the live alpha pods.
    const ids: Record<string, string> = {
      [`${storageUrlForHuman('human-jessica-spouse')}/p2p/status`]: REAL_JESSICA,
      [`${storageUrlForHuman('human-matthew-manager')}/p2p/status`]: REAL_MATTHEW,
    };
    const fetchImpl = vi.fn(
      async (url: string) => new Response(JSON.stringify({ peerId: ids[url] }), { status: 200 })
    );

    // Surface 1 — what an agent-peer binding carries for (humanId, archetype).
    const bindingProvider = await resolvePeerId('human-jessica-spouse', 'desktop', { fetchImpl });
    const bindingReceiver = await resolvePeerId('human-matthew-manager', 'desktop', { fetchImpl });

    // Surface 2 — the ceremony custody input, resolved the way the live seed
    // path does (resolvePeerId per member, then handed to the builder).
    const custodyPeerIds = {
      provider: await resolvePeerId('human-jessica-spouse', 'desktop', { fetchImpl }),
      receiver: await resolvePeerId('human-matthew-manager', 'desktop', { fetchImpl }),
    };
    const input = buildCeremonyCustodyInput({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 64,
      collectiveCid: 'collective:uhCkkFAKE',
    }, custodyPeerIds);

    // Coherence: custody.provider == the binding peer id for the same
    // (humanId, archetype) resolved from the same mocked /p2p/status.
    expect(input.provider).toBe(bindingProvider);
    expect(input.receiver).toBe(bindingReceiver);

    // Both surfaces resolved the REAL pod ids — not the Stage-1 deterministic
    // fallback (both surfaces falling back in lockstep would mask drift).
    expect(input.provider).toBe(REAL_JESSICA);
    expect(input.receiver).toBe(REAL_MATTHEW);
    expect(input.provider).not.toBe(deterministicPeerId('human-jessica-spouse', 'desktop'));

    // The per-run promise cache is the mechanism guaranteeing both surfaces
    // see the SAME answer per host: one /p2p/status probe per pod, shared.
    expect(fetchImpl).toHaveBeenCalledTimes(2);
    clearPeerIdCache();
  });

  it('falls back to Stage-1 deterministic ids only when peerIds is omitted (shape-tests only)', () => {
    const input = buildCeremonyCustodyInput({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 64,
      collectiveCid: 'collective:uhCkkFAKE',
    });
    expect(input.provider).toBe(deterministicPeerId('human-jessica-spouse', 'desktop'));
    expect(input.receiver).toBe(deterministicPeerId('human-matthew-manager', 'desktop'));
  });
});

describe('HOUSEHOLD_MEMBERS', () => {
  it('is the canonical triad with the founder first', () => {
    expect(HOUSEHOLD_MEMBERS.map(m => m.humanId)).toEqual([
      'human-matthew-manager', 'human-jessica-spouse', 'human-james-son',
    ]);
    expect(HOUSEHOLD_MEMBERS[2].minor).toBe(true);
  });
});

describe('resolveExistingCollectiveCid', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns the cid when collectiveCid is present and starts with collective:', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collectiveCid: 'collective:uhCkkABCDEF' }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBe('collective:uhCkkABCDEF');
  });

  it('returns null when collectiveCid is null', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collectiveCid: null }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('returns null on a non-OK response', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('returns null when fetch throws', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network error')));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('returns null when cid does not start with collective:', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collectiveCid: 'uhCkkBadPrefix' }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('also handles the snake_case collective_cid field', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collective_cid: 'collective:uhCkkSNAKE' }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBe('collective:uhCkkSNAKE');
  });
});
