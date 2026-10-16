import { describe, it, expect, vi, afterEach } from 'vitest';
import {
  buildOperatorCommitmentBody,
  defaultM5Bindings,
  createStewardOperatorBindingClient,
  seedOperatorBindings,
  buildHostingAgreementBody,
  type OperatorBinding,
  type HostingAgreementSpec,
} from '../seed-operator-bindings.js';
import { operatorAgentKeysFromLogin } from '../seed-humans.js';

const agentKey = 'uhCAk-real-login-agent-key';
afterEach(() => vi.unstubAllGlobals());

describe('buildOperatorCommitmentBody', () => {
  const binding: OperatorBinding = {
    operatorHumanId: 'human-matthew-manager',
    operatorArchetype: 'desktop',
    doorwayId: 'alpha-elohim-host',
    capabilities: ['*'],
    successionRole: 'primary',
    reachScope: 'stewards-only',
  };

  it('action is exactly "operate-doorway"', () => {
    const body = buildOperatorCommitmentBody(binding, agentKey);
    expect(body.action).toBe('operate-doorway');
  });

  it('provider and receiver use the real login agent key', () => {
    const body = buildOperatorCommitmentBody(binding, agentKey);
    expect(body.provider).toBe(agentKey);
    expect(body.receiver).toBe(agentKey);
    // operate-doorway has no separate counterparty — provider == receiver by convention.
    expect(body.provider).toBe(body.receiver);
  });

  it('resourceClassifiedAs is a JSON-encoded array string of capabilities', () => {
    const body = buildOperatorCommitmentBody(binding, agentKey);
    // Must be a string (not an array) — CreateReaCommitmentInput.resource_classified_as
    // is Option<String>; sending a raw JSON array causes HTTP 500.
    expect(typeof body.resourceClassifiedAs).toBe('string');
    expect(JSON.parse(body.resourceClassifiedAs)).toEqual(['*']);
  });

  it('inScopeOf is a JSON-encoded array string containing "doorway:<id>"', () => {
    const body = buildOperatorCommitmentBody(binding, agentKey);
    // Must be a string — CreateReaCommitmentInput.in_scope_of is Option<String>.
    // The view layer parses it back to Vec<String> via serde_json::from_str.
    expect(typeof body.inScopeOf).toBe('string');
    expect(JSON.parse(body.inScopeOf)).toEqual(['doorway:alpha-elohim-host']);
  });

  it('metadataJson is a JSON-encoded string carrying schemaVersion, successionRole, reachScope, operatorHumanId', () => {
    const body = buildOperatorCommitmentBody(binding, agentKey);
    // Must be metadataJson (not metadata) — CreateReaCommitmentInput.metadata_json.
    expect(typeof body.metadataJson).toBe('string');
    const meta = JSON.parse(body.metadataJson) as Record<string, unknown>;
    expect(meta).toMatchObject({
      schemaVersion: 1,
      successionRole: 'primary',
      reachScope: 'stewards-only',
      operatorHumanId: 'human-matthew-manager',
    });
  });

  it('metadataJson.reachScope defaults to "operator-private" when omitted', () => {
    const body = buildOperatorCommitmentBody({ ...binding, reachScope: undefined }, agentKey);
    const meta = JSON.parse(body.metadataJson) as Record<string, unknown>;
    expect(meta).toMatchObject({ reachScope: 'operator-private' });
  });

  it('id is deterministic — same (operator, doorway, action) → same id (idempotent re-runs)', () => {
    const a = buildOperatorCommitmentBody(binding, agentKey);
    const b = buildOperatorCommitmentBody(binding, agentKey);
    expect(a.id).toBe(b.id);
  });

  it('id is distinct across doorways for the same operator', () => {
    const alpha = buildOperatorCommitmentBody(binding, agentKey);
    const apex = buildOperatorCommitmentBody(
      { ...binding, doorwayId: 'apex-elohim-host' },
      agentKey,
    );
    expect(alpha.id).not.toBe(apex.id);
  });

  it('id has the "operate-doorway-" prefix for ergonomic log scanning', () => {
    const body = buildOperatorCommitmentBody(binding, agentKey);
    expect(body.id).toMatch(/^operate-doorway-[a-f0-9]{16}$/);
  });
});

describe('buildHostingAgreementBody', () => {
  const spec: HostingAgreementSpec = {
    provider: 'matthew',
    scopes: ['host:alpha.elohim.host', 'epr_root:elohim-host-landing'],
    signalKind: 'compute-allocation',
    triggerKind: 'subscription',
    note: "matthew's in-kind compute hosting for elohim-host-landing on alpha.elohim.host",
  };

  it('action is exactly "hosting-agreement"', () => {
    const body = buildHostingAgreementBody(spec);
    expect(body.action).toBe('hosting-agreement');
  });

  it('provider is the literal string (not a peer_id) for direct ?provider= querying', () => {
    const body = buildHostingAgreementBody(spec);
    expect(body.provider).toBe('matthew');
    expect(body.receiver).toBe('matthew');
  });

  it('inScopeOf is a JSON-encoded array containing both scope values', () => {
    const body = buildHostingAgreementBody(spec);
    expect(typeof body.inScopeOf).toBe('string');
    const scopes = JSON.parse(body.inScopeOf) as string[];
    expect(scopes).toContain('host:alpha.elohim.host');
    expect(scopes).toContain('epr_root:elohim-host-landing');
  });

  it('metadataJson carries signalKind and triggerKind', () => {
    const body = buildHostingAgreementBody(spec);
    expect(typeof body.metadataJson).toBe('string');
    const meta = JSON.parse(body.metadataJson) as Record<string, unknown>;
    expect(meta.signalKind).toBe('compute-allocation');
    expect(meta.triggerKind).toBe('subscription');
  });

  it('id is deterministic (idempotent re-runs)', () => {
    const a = buildHostingAgreementBody(spec);
    const b = buildHostingAgreementBody(spec);
    expect(a.id).toBe(b.id);
  });

  it('id has the "hosting-agreement-" prefix', () => {
    const body = buildHostingAgreementBody(spec);
    expect(body.id).toMatch(/^hosting-agreement-[a-f0-9]{16}$/);
  });
});

describe('operator ruling and anchored issuance', () => {
  const bindings = defaultM5Bindings({ DOORWAY_ID: 'household-a' });
  const keys = new Map([
    ['human-matthew-manager', agentKey],
    ['human-adam-firstman', 'uhCAk-adam-login-key'],
  ]);

  it('covers household and alpha primary, Adam apex primary, Matthew apex deputy', () => {
    expect(bindings.map(b => [b.operatorHumanId, b.doorwayId, b.successionRole])).toEqual([
      ['human-matthew-manager', 'household-a', 'primary'],
      ['human-matthew-manager', 'alpha-elohim-host', 'primary'],
      ['human-adam-firstman', 'apex-elohim-host', 'primary'],
      ['human-matthew-manager', 'apex-elohim-host', 'deputy'],
    ]);
    expect(
      bindings
        .filter(b => b.successionRole === 'primary')
        .every(b => b.capabilities.join() === '*'),
    ).toBe(true);
    expect(defaultM5Bindings({})).toHaveLength(3);
  });

  it('enumerates the exact deputy capabilities without permission mutation', () => {
    expect(bindings[3].capabilities).toEqual([
      'doorway-observe',
      'topology.read',
      'federation.peers.read',
      'federation.peers.mutate',
      'hosted_users.lifecycle',
      'graduation.record',
      'registry.read',
      'registry.mutate',
      'cache.read',
      'cache.mutate',
    ]);
  });

  it('records the issuer on every binding and Adam test-fixture basis', () => {
    const metadata = bindings.map(b =>
      JSON.parse(buildOperatorCommitmentBody(b, keys.get(b.operatorHumanId)).metadataJson),
    );
    expect(metadata.every(m => m.issuedBy === 'human-matthew-manager')).toBe(true);
    expect(metadata[2]).toMatchObject({
      standingBasis: 'test-fixture',
      testNetSteward: 'human-matthew-manager',
    });
  });

  it('fails missing agent keys and changes digest when the actual key changes', () => {
    expect(() => buildOperatorCommitmentBody(bindings[0])).toThrow(
      'OPERATOR_AGENT_KEY_MISSING: human-matthew-manager',
    );
    expect(buildOperatorCommitmentBody(bindings[0], agentKey).id).not.toBe(
      buildOperatorCommitmentBody(bindings[0], 'another-key').id,
    );
  });

  it('refuses implicit issuer routing even when other storage URLs exist', () => {
    expect(() =>
      createStewardOperatorBindingClient({
        STORAGE_URL: 'http://other',
        DOORWAY_URL: 'http://gateway',
      }),
    ).toThrow('OPERATOR_STEWARD_STORAGE_MISSING');
  });

  it('POSTs and reads back exclusively through the configured steward storage', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response('{}', { status: 201 }))
      .mockResolvedValueOnce(Response.json({ dhtAnchorHash: 'uhCkk-anchor' }));
    vi.stubGlobal('fetch', fetchMock);
    const client = createStewardOperatorBindingClient({
      OPERATOR_STEWARD_STORAGE_URL: 'http://matthew:8090',
    });
    await seedOperatorBindings(client, [bindings[2]], keys);
    expect(fetchMock.mock.calls[0][0]).toBe('http://matthew:8090/api/v1/commitments');
    const body = JSON.parse(fetchMock.mock.calls[0][1].body);
    expect(body.provider).toBe('uhCAk-adam-login-key');
    expect(fetchMock.mock.calls[1][0]).toBe(`http://matthew:8090/api/v1/commitments/${body.id}`);
  });

  it.each([201, 409])('rejects unanchored readback after HTTP %i', async status => {
    const client = {
      createCommitment: vi.fn().mockResolvedValue(new Response('{}', { status })),
      getCommitment: vi.fn().mockResolvedValue(Response.json({ dhtAnchorHash: null })),
    };
    await expect(seedOperatorBindings(client, [bindings[0]], keys)).rejects.toThrow(
      'OPERATOR_BINDING_ANCHOR_MISSING',
    );
  });

  it('surfaces conductor-required errors, even if their text mentions already exists', async () => {
    const client = {
      createCommitment: vi.fn().mockResolvedValue(
        new Response('conductor required; row already exists', {
          status: 503,
        }),
      ),
      getCommitment: vi.fn(),
    };
    await expect(seedOperatorBindings(client, [bindings[0]], keys)).rejects.toThrow(
      'HTTP 503: conductor required',
    );
    expect(client.getCommitment).not.toHaveBeenCalled();
  });

  it('fails readback HTTP errors', async () => {
    const client = {
      createCommitment: vi.fn().mockResolvedValue(new Response('{}', { status: 201 })),
      getCommitment: vi.fn().mockResolvedValue(new Response('missing', { status: 404 })),
    };
    await expect(seedOperatorBindings(client, [bindings[0]], keys)).rejects.toThrow(
      'OPERATOR_BINDING_READBACK_FAILED',
    );
  });

  it('validates every key before issuing any binding', async () => {
    const client = { createCommitment: vi.fn(), getCommitment: vi.fn() };
    await expect(
      seedOperatorBindings(client, bindings, new Map([['human-matthew-manager', agentKey]])),
    ).rejects.toThrow('OPERATOR_AGENT_KEY_MISSING: human-adam-firstman');
    expect(client.createCommitment).not.toHaveBeenCalled();
  });

  it('threads the seeder login result into provider and logs in once per human', async () => {
    const fetchMock = vi.fn().mockResolvedValue(Response.json({ agentPubKey: agentKey }));
    vi.stubGlobal('fetch', fetchMock);
    const actual = await operatorAgentKeysFromLogin('http://doorway', [
      'human-matthew-manager',
      'human-matthew-manager',
    ]);
    expect(
      buildOperatorCommitmentBody(bindings[0], actual.get('human-matthew-manager')).provider,
    ).toBe(agentKey);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe('http://doorway/auth/login');
  });

  it('refuses a successful login without an agent key', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({})));
    await expect(
      operatorAgentKeysFromLogin('http://doorway', ['human-matthew-manager']),
    ).rejects.toThrow('OPERATOR_AGENT_KEY_MISSING');
  });

  it('surfaces login failures', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response('invalid credentials', { status: 401 })),
    );
    await expect(
      operatorAgentKeysFromLogin('http://doorway', ['human-matthew-manager']),
    ).rejects.toThrow('OPERATOR_LOGIN_FAILED');
  });
});
