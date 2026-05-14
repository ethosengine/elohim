import { describe, it, expect } from 'vitest';
import { buildCustodyCommitmentBody, type CustodyPair } from '../seed-commitments.js';

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

  it('resourceClassifiedAs is the raw blob hash with sha256- prefix', () => {
    const body = buildCustodyCommitmentBody(pair);
    expect(body.resourceClassifiedAs).toBe('sha256-deadbeef');
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
});
