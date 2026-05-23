import { describe, it, expect } from 'vitest';
import { buildOperatorCommitmentBody, type OperatorBinding } from '../seed-operator-bindings.js';

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

  it('resourceClassifiedAs is the capability array verbatim', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.resourceClassifiedAs).toEqual(['*']);
  });

  it('inScopeOf is exactly ["doorway:<id>"]', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.inScopeOf).toEqual(['doorway:alpha-elohim-host']);
  });

  it('metadata carries schemaVersion=1, successionRole, reachScope, and operator humanId', () => {
    const body = buildOperatorCommitmentBody(binding);
    expect(body.metadata).toMatchObject({
      schemaVersion: 1,
      successionRole: 'primary',
      reachScope: 'stewards-only',
      operatorHumanId: 'human-matthew-manager',
    });
  });

  it('metadata.reachScope defaults to "operator-private" when omitted', () => {
    const body = buildOperatorCommitmentBody({ ...binding, reachScope: undefined });
    expect(body.metadata).toMatchObject({ reachScope: 'operator-private' });
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
