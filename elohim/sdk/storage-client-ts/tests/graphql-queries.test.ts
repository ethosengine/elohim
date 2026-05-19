/**
 * Smoke tests for the GraphQL query documents + response types exported
 * from `@elohim/storage-client/graphql`.
 *
 * Verifies:
 * 1. Both query strings are non-empty.
 * 2. Each query selects the field names the Angular consumer (A6) expects —
 *    catches accidental deletion or rename.
 * 3. The TS response interfaces compile against representative literals
 *    (the type-check itself is the assertion).
 *
 * No live HTTP — pure string + type-shape checks.
 */

import { describe, expect, it } from 'vitest';

import {
  VIEWER_HUB_QUERY,
  VIEWER_PEERS_QUERY,
  VIEWER_RECIPROCITY_QUERY,
  type DeviceArchetype,
  type FreshnessState,
  type GraphQLEnvelope,
  type ViewerHubResponse,
  type ViewerPeersResponse,
  type ViewerQueryVariables,
  type ViewerReciprocityResponse,
} from '../src/graphql';

// ---------------------------------------------------------------------------
// Query string smoke checks
// ---------------------------------------------------------------------------

describe('VIEWER_HUB_QUERY', () => {
  it('is a non-empty string', () => {
    expect(typeof VIEWER_HUB_QUERY).toBe('string');
    expect(VIEWER_HUB_QUERY.trim().length).toBeGreaterThan(0);
  });

  it('declares the ViewerHub operation with $agentCid: ID! variable', () => {
    expect(VIEWER_HUB_QUERY).toMatch(/query\s+ViewerHub\s*\(\s*\$agentCid:\s*ID!\s*\)/);
    expect(VIEWER_HUB_QUERY).toContain('viewer(agentCid: $agentCid)');
  });

  it('selects all hub-level fields', () => {
    const required = [
      'agentCid',
      'hub',
      'devices',
      'totals',
      'freshness',
      'state',
      'staleSinceMs',
    ];
    for (const field of required) {
      expect(VIEWER_HUB_QUERY).toContain(field);
    }
  });

  it('selects all DeviceSummary fields the resolver exposes', () => {
    const fields = [
      'peerId',
      'archetype',
      'displayName',
      'online',
      'storageUsedBytes',
      'storageTotalBytes',
      'memoryUsedBytes',
      'memoryTotalBytes',
      'hostingCount',
      'projectingCount',
      'beaconAgeMs',
    ];
    for (const field of fields) {
      expect(VIEWER_HUB_QUERY).toContain(field);
    }
  });

  it('selects all DeviceTotals fields', () => {
    expect(VIEWER_HUB_QUERY).toContain('externalCommittedBytes');
    expect(VIEWER_HUB_QUERY).toContain('reciprocityNetBytes');
  });
});

describe('VIEWER_PEERS_QUERY', () => {
  it('is a non-empty string', () => {
    expect(typeof VIEWER_PEERS_QUERY).toBe('string');
    expect(VIEWER_PEERS_QUERY.trim().length).toBeGreaterThan(0);
  });

  it('declares the ViewerPeers operation with $agentCid: ID! variable', () => {
    expect(VIEWER_PEERS_QUERY).toMatch(/query\s+ViewerPeers\s*\(\s*\$agentCid:\s*ID!\s*\)/);
    expect(VIEWER_PEERS_QUERY).toContain('viewer(agentCid: $agentCid)');
  });

  it('selects all PeerTopology top-level fields', () => {
    const required = [
      'peers',
      'edges',
      'reciprocationCount',
      'resilienceCliffs',
      'freshness',
    ];
    for (const field of required) {
      expect(VIEWER_PEERS_QUERY).toContain(field);
    }
  });

  it('selects all PeerHouseholdEdge fields the resolver exposes', () => {
    const fields = [
      'householdId',
      'displayName',
      'online',
      'lastSyncSec',
      'myCidsHostedByThem',
      'theirCidsHostedByMe',
      'netDiff',
      'isCriticalForMe',
      'iAmCriticalForThem',
    ];
    for (const field of fields) {
      expect(VIEWER_PEERS_QUERY).toContain(field);
    }
  });

  it('selects all ResilienceCliff fields', () => {
    expect(VIEWER_PEERS_QUERY).toContain('soleReplicaCidCount');
  });
});

describe('VIEWER_RECIPROCITY_QUERY', () => {
  it('is a non-empty string', () => {
    expect(typeof VIEWER_RECIPROCITY_QUERY).toBe('string');
    expect(VIEWER_RECIPROCITY_QUERY.trim().length).toBeGreaterThan(0);
  });

  it('declares the ViewerReciprocity operation with $agentCid: ID! variable', () => {
    expect(VIEWER_RECIPROCITY_QUERY).toMatch(
      /query\s+ViewerReciprocity\s*\(\s*\$agentCid:\s*ID!\s*\)/,
    );
    expect(VIEWER_RECIPROCITY_QUERY).toContain('viewer(agentCid: $agentCid)');
  });

  it('selects all reciprocity-level fields', () => {
    const required = [
      'reciprocity',
      'inflow',
      'outflow',
      'netHostedBytes',
      'capacityAvailableBytes',
    ];
    for (const field of required) {
      expect(VIEWER_RECIPROCITY_QUERY).toContain(field);
    }
  });

  it('selects all ReciprocityRow fields the resolver exposes', () => {
    const fields = [
      'counterpartyHouseholdId',
      'displayName',
      'committedBytes',
      'deliveredBytes',
      'honoredPercent',
      'online',
    ];
    for (const field of fields) {
      expect(VIEWER_RECIPROCITY_QUERY).toContain(field);
    }
  });
});

// ---------------------------------------------------------------------------
// Type-shape compile-time assertions
// ---------------------------------------------------------------------------

describe('viewer GraphQL response types', () => {
  it('ViewerHubResponse accepts a fully-populated literal', () => {
    const envelope: GraphQLEnvelope<ViewerHubResponse> = {
      data: {
        viewer: {
          agentCid: 'agent-cid-abc',
          hub: {
            agentCid: 'agent-cid-abc',
            devices: [
              {
                peerId: 'peer-1',
                archetype: 'DESKTOP',
                displayName: 'Workstation',
                online: true,
                freshness: { state: 'LIVE', staleSinceMs: null },
                storageUsedBytes: '1024',
                storageTotalBytes: '4096',
                memoryUsedBytes: '512',
                memoryTotalBytes: '2048',
                hostingCount: 3,
                projectingCount: 1,
                beaconAgeMs: '120',
              },
            ],
            totals: {
              storageUsedBytes: '1024',
              storageTotalBytes: '4096',
              externalCommittedBytes: '0',
              reciprocityNetBytes: '0',
            },
            freshness: { state: 'LIVE', staleSinceMs: null },
          },
        },
      },
    };

    expect(envelope.data?.viewer.hub.devices[0]?.archetype).toBe('DESKTOP');
    expect(envelope.data?.viewer.hub.freshness.state).toBe('LIVE');
  });

  it('ViewerHubResponse accepts nullable device-summary fields', () => {
    const envelope: GraphQLEnvelope<ViewerHubResponse> = {
      data: {
        viewer: {
          agentCid: 'agent-cid-abc',
          hub: {
            agentCid: 'agent-cid-abc',
            devices: [
              {
                peerId: 'peer-1',
                archetype: 'NODE',
                displayName: null,
                online: false,
                freshness: { state: 'ALL_OFFLINE', staleSinceMs: '1700000000000' },
                storageUsedBytes: null,
                storageTotalBytes: null,
                memoryUsedBytes: null,
                memoryTotalBytes: null,
                hostingCount: null,
                projectingCount: null,
                beaconAgeMs: null,
              },
            ],
            totals: {
              storageUsedBytes: '0',
              storageTotalBytes: '0',
              externalCommittedBytes: '0',
              reciprocityNetBytes: '0',
            },
            freshness: { state: 'ALL_OFFLINE', staleSinceMs: '1700000000000' },
          },
        },
      },
    };

    expect(envelope.data?.viewer.hub.devices[0]?.online).toBe(false);
    expect(envelope.data?.viewer.hub.devices[0]?.hostingCount).toBeNull();
  });

  it('ViewerPeersResponse accepts a fully-populated literal', () => {
    const envelope: GraphQLEnvelope<ViewerPeersResponse> = {
      data: {
        viewer: {
          agentCid: 'agent-cid-abc',
          peers: {
            agentCid: 'agent-cid-abc',
            edges: [
              {
                householdId: 'household-xyz',
                displayName: 'Bay Area Dawn Runners',
                online: true,
                lastSyncSec: '1700000000',
                myCidsHostedByThem: 12,
                theirCidsHostedByMe: 8,
                netDiff: '4',
                isCriticalForMe: false,
                iAmCriticalForThem: true,
              },
            ],
            reciprocationCount: 1,
            resilienceCliffs: [
              { householdId: 'household-xyz', soleReplicaCidCount: 2 },
            ],
            freshness: { state: 'LIVE', staleSinceMs: null },
          },
        },
      },
    };

    expect(envelope.data?.viewer.peers.reciprocationCount).toBe(1);
    expect(envelope.data?.viewer.peers.resilienceCliffs[0]?.soleReplicaCidCount).toBe(2);
  });

  it('GraphQLEnvelope accepts errors-only responses', () => {
    const envelope: GraphQLEnvelope<ViewerHubResponse> = {
      errors: [
        {
          message: 'Forbidden: agent cid not visible',
          path: ['viewer', 'hub'],
        },
      ],
    };

    expect(envelope.errors?.[0]?.message).toContain('Forbidden');
    expect(envelope.data).toBeUndefined();
  });

  it('ViewerQueryVariables enforces the agentCid contract', () => {
    const vars: ViewerQueryVariables = { agentCid: 'agent-cid-abc' };
    expect(vars.agentCid).toBe('agent-cid-abc');
  });

  it('FreshnessState string-union covers every Rust variant', () => {
    // Compile-time assertion: each literal must be assignable to FreshnessState.
    const variants: FreshnessState[] = [
      'LIVE',
      'STALE',
      'OFFLINE',
      'CACHED_OFFLINE_UNTIL_RECONNECT',
      'UNVERIFIABLE',
      'ALL_OFFLINE',
    ];
    expect(variants).toHaveLength(6);
  });

  it('DeviceArchetype string-union covers every Rust variant', () => {
    const variants: DeviceArchetype[] = ['NODE', 'DESKTOP', 'MOBILE', 'STEWARD'];
    expect(variants).toHaveLength(4);
  });

  it('ViewerReciprocityResponse accepts a fully-populated literal', () => {
    const envelope: GraphQLEnvelope<ViewerReciprocityResponse> = {
      data: {
        viewer: {
          agentCid: 'agent-cid-abc',
          reciprocity: {
            agentCid: 'agent-cid-abc',
            inflow: [
              {
                counterpartyHouseholdId: 'household-terrance',
                displayName: 'Terrance',
                committedBytes: '1024',
                deliveredBytes: '768',
                honoredPercent: 0.75,
                online: true,
              },
            ],
            outflow: [
              {
                counterpartyHouseholdId: 'household-jessica',
                displayName: null,
                committedBytes: '2048',
                deliveredBytes: '2048',
                honoredPercent: 1.0,
                online: false,
              },
            ],
            netHostedBytes: '-1280',
            capacityAvailableBytes: '8192',
          },
        },
      },
    };

    expect(envelope.data?.viewer.reciprocity.inflow[0]?.counterpartyHouseholdId).toBe(
      'household-terrance',
    );
    expect(envelope.data?.viewer.reciprocity.outflow[0]?.displayName).toBeNull();
    expect(envelope.data?.viewer.reciprocity.netHostedBytes).toBe('-1280');
  });

  it('ViewerReciprocityResponse accepts cold-cache nullable online + empty arrays', () => {
    const envelope: GraphQLEnvelope<ViewerReciprocityResponse> = {
      data: {
        viewer: {
          agentCid: 'agent-cid-empty',
          reciprocity: {
            agentCid: 'agent-cid-empty',
            inflow: [],
            outflow: [],
            netHostedBytes: '0',
            capacityAvailableBytes: '0',
          },
        },
      },
    };

    expect(envelope.data?.viewer.reciprocity.inflow).toHaveLength(0);
    expect(envelope.data?.viewer.reciprocity.netHostedBytes).toBe('0');
  });
});
