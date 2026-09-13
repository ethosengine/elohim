import { describe, it, expect, vi } from 'vitest';
import {
  buildProjectionCommitmentBody,
  baseProjectionId,
  defaultProjectionSeeds,
  metadataDrift,
  projectionRelevantMetadata,
  regrantFingerprint,
  specToMetadata,
  findActiveRowForSpec,
  seedProjections,
  PROJECTION_RELEVANT_FIELDS,
  withHostnames,
  candidateChannelSpec,
  type ProjectionSpec,
  type ProjectionRelevantMetadata,
  type EprProjectionViewLite,
} from '../seed-projections.js';

/** Build a `Response`-like stub for the fake client. */
function fakeResponse(status: number, body: unknown): Response {
  const text = typeof body === 'string' ? body : JSON.stringify(body);
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => (typeof body === 'string' ? JSON.parse(body) : body),
    text: async () => text,
  } as unknown as Response;
}

describe('buildProjectionCommitmentBody', () => {
  const baseSpec: ProjectionSpec = {
    stewardHumanId: 'human-matthew-manager',
    stewardArchetype: 'desktop',
    doorwayId: 'alpha-elohim-host',
    eprId: 'lamad-spa',
    urlPath: '/lamad',
    mode: 'cached',
    reach: 'commons',
    baseHref: '/lamad/',
    entryFile: 'index.html',
    redirectsFrom: [],
    previewEprRef: null,
    gateHints: [],
    deadEnd: false,
    stewardDirectEndpoint: null,
    routeClaims: null,
    redirectTemplates: [],
    hostnames: [],
    channel: 'converged',
  };

  it('builds a commons-reach lamad projection at /lamad', () => {
    const body = buildProjectionCommitmentBody(baseSpec);
    expect(body.action).toBe('project-epr');
    expect(body.inScopeOf).toContain('doorway:alpha-elohim-host');
    expect(body.inScopeOf).toContain('epr:lamad-spa');
    const meta = JSON.parse(body.metadataJson);
    expect(meta.urlPath).toBe('/lamad');
    expect(meta.mode).toBe('cached');
    expect(meta.reach).toBe('commons');
  });

  it('produces deterministic id for same spec (idempotent re-seed)', () => {
    const a = buildProjectionCommitmentBody(baseSpec);
    const b = buildProjectionCommitmentBody(baseSpec);
    expect(a.id).toBe(b.id);
  });

  it('produces different ids for different (doorway, epr) pairs', () => {
    const a = buildProjectionCommitmentBody(baseSpec);
    const b = buildProjectionCommitmentBody({ ...baseSpec, doorwayId: 'elohim-host' });
    const c = buildProjectionCommitmentBody({ ...baseSpec, eprId: 'elohim-host-landing' });
    expect(a.id).not.toBe(b.id);
    expect(a.id).not.toBe(c.id);
    expect(b.id).not.toBe(c.id);
  });
});

describe('defaultProjectionSeeds', () => {
  it('default seed set has 6 commitments total (landing × 2 + lamad × 2 + portal × 2)', () => {
    expect(defaultProjectionSeeds().length).toBe(6);
  });

  it('grants lamad routeClaims + the legacy resource redirect template', () => {
    const lamad = defaultProjectionSeeds().find(
      (s) => s.eprId === 'lamad-spa' && s.doorwayId === 'alpha-elohim-host',
    )!;
    const meta = JSON.parse(buildProjectionCommitmentBody(lamad).metadataJson);
    expect(meta.routeClaims.schemaVersion).toBe(1);
    expect(meta.routeClaims.claims).toEqual([
      { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
    ]);
    expect(meta.redirectTemplates).toEqual([{ from: '/lamad/resource/{id}', to: '/epr/{id}' }]);
  });

  it('default seed set includes /auth/portal projections on both doorways', () => {
    const seeds = defaultProjectionSeeds();
    const portalSeeds = seeds.filter((s) => s.eprId === 'imagodei-portal');
    expect(portalSeeds.length).toBe(2);
    expect(portalSeeds.every((s) => s.urlPath === '/auth/portal')).toBe(true);
    expect(portalSeeds.every((s) => s.baseHref === '/auth/portal/')).toBe(true);
    expect(portalSeeds.map((s) => s.doorwayId).sort()).toEqual([
      'alpha-elohim-host',
      'apex-elohim-host',
    ]);
  });
});

// ===========================================================================
// Re-grant supersession (spec §3.2/§3.3)
// ===========================================================================

const lamadSpec: ProjectionSpec = {
  stewardHumanId: 'human-matthew-manager',
  stewardArchetype: 'desktop',
  doorwayId: 'alpha-elohim-host',
  eprId: 'lamad-spa',
  urlPath: '/lamad',
  mode: 'cached',
  reach: 'commons',
  baseHref: '/lamad/',
  entryFile: 'index.html',
  redirectsFrom: [],
  previewEprRef: null,
  gateHints: [],
  deadEnd: false,
  stewardDirectEndpoint: null,
  routeClaims: {
    schemaVersion: 1,
    claimsManifestCid: null,
    claims: [
      { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
    ],
  },
  redirectTemplates: [{ from: '/lamad/resource/{id}', to: '/epr/{id}' }],
  hostnames: [],
  channel: 'converged',
};

describe('metadataDrift — null-vs-missing normalization', () => {
  it('no drift when existing omits keys that materialize to their storage defaults', () => {
    // The live-alpha "grant-less" row, projected by storage, materializes
    // routeClaims=null, redirectTemplates=[], spaFallback=true, etc. A desired
    // spec that ALSO has those defaults must NOT register drift.
    const existing: Partial<EprProjectionViewLite> = {
      urlPath: '/lamad',
      mode: 'cached',
      reach: 'commons',
      baseHref: '/lamad/',
      entryFile: 'index.html',
      // routeClaims, redirectTemplates, spaFallback, etc. OMITTED (missing keys)
    };
    const desired = {
      urlPath: '/lamad',
      mode: 'cached' as const,
      reach: 'commons',
      baseHref: '/lamad/',
      entryFile: 'index.html',
      // also no grant — both are grant-less
    };
    expect(metadataDrift(desired, existing)).toEqual([]);
  });

  it('null routeClaims (existing) equals missing routeClaims (desired) — no drift', () => {
    expect(metadataDrift({ routeClaims: null }, {})).toEqual([]);
    expect(metadataDrift({}, { routeClaims: null })).toEqual([]);
  });

  it('missing spaFallback compares equal to materialized true', () => {
    expect(metadataDrift({}, { spaFallback: true })).toEqual([]);
    expect(metadataDrift({ spaFallback: true }, {})).toEqual([]);
  });

  it('detects routeClaims drift: grant-less existing vs grant-bearing desired', () => {
    const existing: Partial<EprProjectionViewLite> = {
      urlPath: '/lamad',
      // grant-less (routeClaims missing → materializes null)
    };
    const drifted = metadataDrift(specToMetadata(lamadSpec), existing);
    expect(drifted).toContain('routeClaims');
    expect(drifted).toContain('redirectTemplates');
  });

  it('detects a single-field drift in isolation (urlPath)', () => {
    const a = specToMetadata(lamadSpec);
    const b = { ...a, urlPath: '/lamad-v2' };
    expect(metadataDrift(b, a)).toEqual(['urlPath']);
  });

  it('PROJECTION_RELEVANT_FIELDS covers exactly the normalized keys', () => {
    const keys = Object.keys(projectionRelevantMetadata({})).sort();
    expect([...PROJECTION_RELEVANT_FIELDS].sort()).toEqual(keys);
  });
});

describe('regrantFingerprint', () => {
  it('is deterministic for identical desired metadata', () => {
    expect(regrantFingerprint(specToMetadata(lamadSpec))).toBe(
      regrantFingerprint(specToMetadata(lamadSpec)),
    );
  });

  it('differs for different desired metadata', () => {
    const a = regrantFingerprint(specToMetadata(lamadSpec));
    const b = regrantFingerprint(specToMetadata({ ...lamadSpec, urlPath: '/lamad-v2' }));
    expect(a).not.toBe(b);
  });

  it('null and missing keys fingerprint identically (normalization)', () => {
    expect(regrantFingerprint({ routeClaims: null })).toBe(regrantFingerprint({}));
  });
});

describe('buildProjectionCommitmentBody — supersede body shape', () => {
  it('first seed: no supersedes, base id, no metadata.supersedes', () => {
    const body = buildProjectionCommitmentBody(lamadSpec);
    expect(body.supersedes).toBeUndefined();
    expect(body.id).toBe(baseProjectionId(lamadSpec));
    expect((body.metadata as Record<string, unknown>).supersedes).toBeUndefined();
  });

  it('supersede: carries supersedes pointer, suffixed id, walkable metadata.supersedes', () => {
    const predecessorId = baseProjectionId(lamadSpec);
    const body = buildProjectionCommitmentBody(lamadSpec, predecessorId);
    expect(body.supersedes).toBe(predecessorId);
    expect(body.id).toBe(`${predecessorId}-r${regrantFingerprint(specToMetadata(lamadSpec))}`);
    expect(body.id).not.toBe(predecessorId); // successor id MUST differ
    // metadata.supersedes mirrors the pointer (chain walkability).
    expect((body.metadata as Record<string, unknown>).supersedes).toBe(predecessorId);
    // The granted claims ride the superseding body.
    const meta = JSON.parse(body.metadataJson);
    expect(meta.routeClaims.schemaVersion).toBe(1);
  });

  it('superseding body re-seeded with same drift derives the same successor id (idempotent)', () => {
    const predecessorId = baseProjectionId(lamadSpec);
    const a = buildProjectionCommitmentBody(lamadSpec, predecessorId);
    const b = buildProjectionCommitmentBody(lamadSpec, predecessorId);
    expect(a.id).toBe(b.id);
  });
});

describe('findActiveRowForSpec', () => {
  it('matches on long-form doorwayId (doorway:<id>)', () => {
    const rows: EprProjectionViewLite[] = [
      { commitmentId: 'c1', eprId: 'lamad-spa', doorwayId: 'doorway:alpha-elohim-host' },
      { commitmentId: 'c2', eprId: 'elohim-host-landing', doorwayId: 'doorway:alpha-elohim-host' },
    ];
    const found = findActiveRowForSpec(rows, lamadSpec);
    expect(found?.commitmentId).toBe('c1');
  });

  it('matches on bare doorwayId too', () => {
    const rows: EprProjectionViewLite[] = [
      { commitmentId: 'c1', eprId: 'lamad-spa', doorwayId: 'alpha-elohim-host' },
    ];
    expect(findActiveRowForSpec(rows, lamadSpec)?.commitmentId).toBe('c1');
  });

  it('returns undefined when no row matches the (epr, doorway) pair', () => {
    const rows: EprProjectionViewLite[] = [
      { commitmentId: 'c1', eprId: 'other-epr', doorwayId: 'doorway:alpha-elohim-host' },
    ];
    expect(findActiveRowForSpec(rows, lamadSpec)).toBeUndefined();
  });
});

describe('seedProjections — 409 drift vs idempotent handling', () => {
  it('no-drift 409 stays quiet (no re-grant POST, no supersede)', async () => {
    // Existing active row matches the desired spec exactly → idempotent re-run.
    const existingRow: EprProjectionViewLite = {
      commitmentId: baseProjectionId(lamadSpec),
      eprId: lamadSpec.eprId,
      doorwayId: `doorway:${lamadSpec.doorwayId}`,
      ...specToMetadata(lamadSpec),
    };
    const createCommitment = vi.fn(async () => fakeResponse(409, 'already exists'));
    const fetchProjections = vi.fn(async () => [existingRow]);
    const client = { createCommitment, fetchProjections } as never;

    await seedProjections(client, [lamadSpec]);

    // Exactly one POST (the initial create that 409'd) — NO second supersede POST.
    expect(createCommitment).toHaveBeenCalledTimes(1);
    expect(fetchProjections).toHaveBeenCalledTimes(1);
  });

  it('drift 409 triggers a supersede POST with supersedes + suffixed id', async () => {
    // Existing active row is the live-alpha GRANT-LESS predecessor.
    const grantlessRow: EprProjectionViewLite = {
      commitmentId: baseProjectionId(lamadSpec),
      eprId: lamadSpec.eprId,
      doorwayId: `doorway:${lamadSpec.doorwayId}`,
      urlPath: '/lamad',
      mode: 'cached',
      reach: 'commons',
      baseHref: '/lamad/',
      entryFile: 'index.html',
      // routeClaims / redirectTemplates OMITTED → grant-less
    };
    const posts: unknown[] = [];
    const createCommitment = vi.fn(async (body: unknown) => {
      posts.push(body);
      // First POST (the base create) 409s; the supersede POST succeeds.
      return posts.length === 1 ? fakeResponse(409, 'already exists') : fakeResponse(201, {});
    });
    const fetchProjections = vi.fn(async () => [grantlessRow]);
    const client = { createCommitment, fetchProjections } as never;

    await seedProjections(client, [lamadSpec]);

    expect(createCommitment).toHaveBeenCalledTimes(2);
    const supersedeBody = posts[1] as Record<string, unknown>;
    expect(supersedeBody.supersedes).toBe(baseProjectionId(lamadSpec));
    expect(supersedeBody.id).toBe(
      `${baseProjectionId(lamadSpec)}-r${regrantFingerprint(specToMetadata(lamadSpec))}`,
    );
    // The granted claims ride the superseding body.
    const meta = JSON.parse(supersedeBody.metadataJson as string);
    expect(meta.routeClaims.schemaVersion).toBe(1);
    expect(meta.supersedes).toBe(baseProjectionId(lamadSpec));
  });

  it('drift re-grant whose successor already exists is idempotent (no error exit)', async () => {
    const grantlessRow: EprProjectionViewLite = {
      commitmentId: baseProjectionId(lamadSpec),
      eprId: lamadSpec.eprId,
      doorwayId: `doorway:${lamadSpec.doorwayId}`,
      urlPath: '/lamad',
    };
    let call = 0;
    const createCommitment = vi.fn(async () => {
      call += 1;
      // base create 409s; supersede POST also 409s (already applied earlier).
      return fakeResponse(409, call === 2 ? 'already superseded' : 'already exists');
    });
    const fetchProjections = vi.fn(async () => [grantlessRow]);
    const client = { createCommitment, fetchProjections } as never;

    // Must NOT throw / process.exit — re-grant-already-applied is idempotent.
    await expect(seedProjections(client, [lamadSpec])).resolves.toBeUndefined();
    expect(createCommitment).toHaveBeenCalledTimes(2);
  });
});

// ===========================================================================
// Rung 4 slice 1 — hostnames and channel as contract terms
//
// Both are keys inside the project-epr Commitment's own `metadata_json`, so
// they are notarized by construction and cost no entry type, no link type and
// no DNA-hash move. What these tests pin is the part that is easy to get
// silently wrong: the DEFAULTS. `hostnames: []` must mean "any host" and an
// absent `channel` must mean `converged`, on both sides of the drift compare —
// otherwise the arrival of the fields re-grants every contract in the fleet
// for no routing-law change at all.
// ===========================================================================

describe('hostnames + channel — contract terms', () => {
  it('carries hostnames and channel through metadataJson', () => {
    const body = buildProjectionCommitmentBody({
      ...lamadSpec,
      hostnames: ['elohim.local', 'elohim.host'],
      channel: 'converged',
    });
    const meta = JSON.parse(body.metadataJson);
    expect(meta.hostnames).toEqual(['elohim.local', 'elohim.host']);
    expect(meta.channel).toBe('converged');
  });

  it('absent hostnames normalizes to any-host ([]) and absent channel to converged', () => {
    const m = projectionRelevantMetadata({});
    expect(m.hostnames).toEqual([]);
    expect(m.channel).toBe('converged');
  });

  it('an any-host converged seed does NOT drift against a pre-rung-4 row (no churn)', () => {
    // The existing row materialized neither key. The desired seed declares
    // both at their defaults. C10: an unknown field is never defaulted into
    // MEANING — here the default IS today's behaviour, so the compare must be
    // equal and no supersede may fire.
    const { hostnames: _h, channel: _c, ...existing } = specToMetadata(lamadSpec);
    const desired = specToMetadata({ ...lamadSpec, hostnames: [], channel: 'converged' });
    expect(metadataDrift(desired, existing as Partial<ProjectionRelevantMetadata>)).toEqual([]);
  });

  it('binding a hostname IS operative routing-law drift (re-grants)', () => {
    const before = specToMetadata(lamadSpec);
    const after = specToMetadata(withHostnames(lamadSpec, ['alpha.elohim.local']));
    expect(metadataDrift(after, before)).toEqual(['hostnames']);
  });

  it('moving the channel IS operative routing-law drift (re-grants)', () => {
    const before = specToMetadata(lamadSpec);
    const after = specToMetadata({ ...lamadSpec, channel: 'candidate' });
    expect(metadataDrift(after, before)).toEqual(['channel']);
  });

  it('PROJECTION_RELEVANT_FIELDS still covers exactly the normalized keys', () => {
    const keys = Object.keys(projectionRelevantMetadata({})).sort();
    expect([...PROJECTION_RELEVANT_FIELDS].sort()).toEqual(keys);
  });

  it('the default seed set is any-host + converged — byte-for-byte today', () => {
    const seeds = defaultProjectionSeeds();
    expect(seeds.every((s) => s.hostnames.length === 0)).toBe(true);
    expect(seeds.every((s) => s.channel === 'converged')).toBe(true);
  });

  it('hostnames and channel stay OUT of the contract id digest', () => {
    // They are contract TERMS, not contract IDENTITY: a change to either is
    // carried by the supersession ceremony (successor id `-r<fingerprint>`),
    // never by minting a second base id for the same undertaking.
    const base = baseProjectionId(lamadSpec);
    expect(baseProjectionId(withHostnames(lamadSpec, ['alpha.elohim.local']))).toBe(base);
    expect(baseProjectionId({ ...lamadSpec, channel: 'candidate' })).toBe(base);
  });

  it('but the re-grant fingerprint DOES move, so a bound contract supersedes', () => {
    expect(regrantFingerprint(specToMetadata(withHostnames(lamadSpec, ['a.local'])))).not.toBe(
      regrantFingerprint(specToMetadata(lamadSpec)),
    );
  });
});

describe('candidateChannelSpec — a candidate standing beside a converged contract', () => {
  const candidate = candidateChannelSpec(lamadSpec, 'alpha.elohim.local');

  it('is a DIFFERENT contract id (scope grows a host ref — a new row, deliberately)', () => {
    expect(baseProjectionId(candidate)).not.toBe(baseProjectionId(lamadSpec));
  });

  it('scopes itself by host so the two can never collide on one id', () => {
    const body = buildProjectionCommitmentBody(candidate);
    expect(body.inScopeOf).toBe(
      'doorway:alpha-elohim-host|epr:lamad-spa|host:alpha.elohim.local',
    );
  });

  it('serves the candidate channel at exactly the candidate hostname', () => {
    const meta = JSON.parse(buildProjectionCommitmentBody(candidate).metadataJson);
    expect(meta.channel).toBe('candidate');
    expect(meta.hostnames).toEqual(['alpha.elohim.local']);
  });

  it('never declares commons reach on a staging name', () => {
    expect(candidate.reach).not.toBe('commons');
    expect(candidateChannelSpec(lamadSpec, 'a.local', 'qahal:stewards').reach).toBe(
      'qahal:stewards',
    );
  });

  it('leaves the converged contract untouched', () => {
    expect(lamadSpec.channel).toBe('converged');
    expect(lamadSpec.hostnames).toEqual([]);
    expect(lamadSpec.scopeHost).toBeUndefined();
  });
});
