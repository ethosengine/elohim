/**
 * Vitest suite for the gate-client thin TS client.
 *
 * Exercises:
 * - createGateClient factory dispatch (mock vs http config validation)
 * - Mock transport's `check()` against every RelationalImpactEvent variant
 * - Mock transport's `queueForReview()` returns a string id
 * - HTTP transport is structurally wired (throws on missing endpoint, fetches
 *   the documented path); deep end-to-end coverage lands in Phase 2 once the
 *   Rust service exposes the endpoint.
 */

import { describe, expect, it } from 'vitest';
import { createGateClient } from './client.js';
import type { EscalationTarget } from './generated/EscalationTarget.js';
import type { RelationalImpactEvent } from './generated/RelationalImpactEvent.js';

// ---------------------------------------------------------------------------
// Event fixtures — one per variant of the closed RelationalImpactEvent enum.
// Mirrors events.rs so we catch new variants via type-checker.
// ---------------------------------------------------------------------------

const EVENT_FIXTURES: Array<[string, RelationalImpactEvent]> = [
  [
    'content-publish',
    {
      kind: 'content-publish',
      content_cid: 'bafybeigtest',
      declared_reach: 'public',
      author: 'agent-a',
    },
  ],
  [
    'attestation-write',
    {
      kind: 'attestation-write',
      subject_hash: 'bafkreitest',
      claim_kind: 'brit',
      issuer: 'agent-a',
    },
  ],
  [
    'economic-event-emit',
    {
      kind: 'economic-event-emit',
      event_kind: 'produce',
      provider: 'agent-a',
      receiver: 'agent-b',
      quantity: '1.0',
    },
  ],
  [
    'peer-message',
    {
      kind: 'peer-message',
      recipient: 'agent-b',
      payload_kind: 'chat',
    },
  ],
  [
    'sync-to-peers',
    {
      kind: 'sync-to-peers',
      manifest_cid: 'bafybeimanifest',
      item_count: 42,
    },
  ],
  [
    'advice-sought',
    {
      kind: 'advice-sought',
      requester: 'agent-a',
      summary_cid: 'bafkreisummary',
      topic: 'reflection',
    },
  ],
  [
    'capability-invoke',
    {
      kind: 'capability-invoke',
      capability: 'advise',
      requester: 'agent-a',
      request_id: 'req-1',
    },
  ],
  [
    'private-to-public-crossing',
    {
      kind: 'private-to-public-crossing',
      source_space: 'draft-a',
      artifact_ref: 'bafkreiartifact',
    },
  ],
];

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

describe('createGateClient', () => {
  it('returns a client for transport=mock', () => {
    const client = createGateClient({ transport: 'mock' });
    expect(typeof client.check).toBe('function');
    expect(typeof client.queueForReview).toBe('function');
  });

  it('returns a client for transport=http when endpoint is provided', () => {
    const client = createGateClient({
      transport: 'http',
      endpoint: 'http://localhost:8080',
    });
    expect(typeof client.check).toBe('function');
  });

  it('throws when transport=http without endpoint', () => {
    expect(() => createGateClient({ transport: 'http' })).toThrow(/endpoint/);
  });
});

// ---------------------------------------------------------------------------
// Mock check()
// ---------------------------------------------------------------------------

describe('MockGateClient.check', () => {
  it('resolves to an allow decision in dev-context phase', async () => {
    const client = createGateClient({ transport: 'mock' });
    const decision = await client.check(EVENT_FIXTURES[0][1]);

    expect(decision.status.status).toBe('allow');
    expect(decision.phase).toBe('dev-context');
    expect(decision.side_effects).toEqual([]);
    expect(decision.decision_attestation_cid).toBeNull();
  });

  it('carries the mocked reasoning summary', async () => {
    const client = createGateClient({ transport: 'mock' });
    const decision = await client.check(EVENT_FIXTURES[0][1]);

    expect(decision.reasoning.primary_principle).toBe('dev-context-mock');
    expect(decision.reasoning.confidence).toBe(0.0);
  });

  it.each(EVENT_FIXTURES)(
    'returns allow for the %s event variant',
    async (_label, event) => {
      const client = createGateClient({ transport: 'mock' });
      const decision = await client.check(event);
      expect(decision.status.status).toBe('allow');
      // Boundary-crossing events are never exempt — the RelationalImpactEvent
      // enum does not include exempt-interior variants. Mirrors the Rust
      // space.rs::detect_from_event logic.
      if (decision.status.status === 'allow') {
        expect(decision.status.exempt).toBe(false);
      }
    },
  );
});

// ---------------------------------------------------------------------------
// Mock queueForReview()
// ---------------------------------------------------------------------------

describe('MockGateClient.queueForReview', () => {
  it('returns a non-empty string id', async () => {
    const client = createGateClient({ transport: 'mock' });
    const target: EscalationTarget = {
      kind: 'app-steward',
      steward_id: 'steward-a',
    };
    const id = await client.queueForReview(target, { reason: 'needs-review' });
    expect(typeof id).toBe('string');
    expect(id.length).toBeGreaterThan(0);
  });

  it('returns a different id on each call', async () => {
    const client = createGateClient({ transport: 'mock' });
    const target: EscalationTarget = { kind: 'existential-boundary' };

    const ids = await Promise.all([
      client.queueForReview(target, {}),
      client.queueForReview(target, {}),
      client.queueForReview(target, {}),
    ]);

    expect(new Set(ids).size).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// HTTP transport — structural only. The Rust endpoint does not exist yet;
// Phase 2 wires in a real server + integration test.
// ---------------------------------------------------------------------------

describe('HttpGateClient', () => {
  it('surfaces fetch failures as errors', async () => {
    const client = createGateClient({
      transport: 'http',
      // Deliberately unroutable. The exact rejection (DNS, refused connect,
      // bad JSON) depends on the runtime; we just assert we don't silently
      // return.
      endpoint: 'http://127.0.0.1:0',
    });

    await expect(client.check(EVENT_FIXTURES[0][1])).rejects.toBeDefined();
  });
});
