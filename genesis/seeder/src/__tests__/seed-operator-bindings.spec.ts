import { describe, it, expect } from 'vitest';
import {
  buildOperatorCommitmentBody,
  buildHostingAgreementBody,
  type OperatorBinding,
  type HostingAgreementSpec,
} from '../seed-operator-bindings.js';

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
    const body = buildOperatorCommitmentBody(binding);
    expect(body.action).toBe('operate-doorway');
  });

  it('provider and receiver are deterministic 12D3KooW peer_ids derived from the operator', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.provider).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    expect(body.receiver).toMatch(/^12D3KooW[a-f0-9]{38}$/);
    // operate-doorway has no separate counterparty — provider == receiver by convention.
    expect(body.provider).toBe(body.receiver);
  });

  it('resourceClassifiedAs is a JSON-encoded array string of capabilities', () => {
    const body = buildOperatorCommitmentBody(binding);
    // Must be a string (not an array) — CreateReaCommitmentInput.resource_classified_as
    // is Option<String>; sending a raw JSON array causes HTTP 500.
    expect(typeof body.resourceClassifiedAs).toBe('string');
    expect(JSON.parse(body.resourceClassifiedAs)).toEqual(['*']);
  });

  it('inScopeOf is a JSON-encoded array string containing "doorway:<id>"', () => {
    const body = buildOperatorCommitmentBody(binding);
    // Must be a string — CreateReaCommitmentInput.in_scope_of is Option<String>.
    // The view layer parses it back to Vec<String> via serde_json::from_str.
    expect(typeof body.inScopeOf).toBe('string');
    expect(JSON.parse(body.inScopeOf)).toEqual(['doorway:alpha-elohim-host']);
  });

  it('metadataJson is a JSON-encoded string carrying schemaVersion, successionRole, reachScope, operatorHumanId', () => {
    const body = buildOperatorCommitmentBody(binding);
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
    const body = buildOperatorCommitmentBody({ ...binding, reachScope: undefined });
    const meta = JSON.parse(body.metadataJson) as Record<string, unknown>;
    expect(meta).toMatchObject({ reachScope: 'operator-private' });
  });

  it('id is deterministic — same (operator, doorway, action) → same id (idempotent re-runs)', () => {
    const a = buildOperatorCommitmentBody(binding);
    const b = buildOperatorCommitmentBody(binding);
    expect(a.id).toBe(b.id);
  });

  it('id is distinct across doorways for the same operator', () => {
    const alpha = buildOperatorCommitmentBody(binding);
    const apex = buildOperatorCommitmentBody({ ...binding, doorwayId: 'apex-elohim-host' });
    expect(alpha.id).not.toBe(apex.id);
  });

  it('id has the "operate-doorway-" prefix for ergonomic log scanning', () => {
    const body = buildOperatorCommitmentBody(binding);
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
