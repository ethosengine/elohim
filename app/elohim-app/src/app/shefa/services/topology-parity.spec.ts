/**
 * topology-parity.spec.ts
 *
 * Parity guard: asserts that the GraphQL adapter (topology-graphql.adapter.ts,
 * landed in Epic A / A6) produces structurally-identical output to the REST
 * endpoints when given logically-equivalent fixtures.
 *
 * Three variants per view type:
 *   (a) empty fixture   — no devices / no edges, all-zero totals
 *   (b) full fixture    — one device + one peer edge, all optional fields populated
 *   (c) null-omission   — one device with all optional fields null/undefined;
 *                         verifies that the adapter does NOT add the key at all
 *                         when the GQL value is null (`'key' in obj === false`)
 *
 * If someone changes the adapter or the GQL types and breaks the REST↔GQL
 * invariant, this file will fail.  That is its purpose.
 */

import { describe, it, expect } from 'vitest';

import type { MyClusterView } from '@app/generated/my-cluster-view';
import type { PeerTopologyView } from '@app/generated/peer-topology-view';
import type { ViewerHubResponse, ViewerPeersResponse } from '@elohim/storage-client/graphql';

import { adaptHubResponse, adaptPeersResponse } from './topology-graphql.adapter';

// ---------------------------------------------------------------------------
// Canonical fixture pairs — MyClusterView
// ---------------------------------------------------------------------------

// (a) Empty cluster fixture ------------------------------------------------

const GQL_HUB_EMPTY: ViewerHubResponse = {
  viewer: {
    agentCid: 'cid_agent_empty',
    hub: {
      agentCid: 'cid_agent_empty',
      devices: [],
      totals: {
        storageUsedBytes: '0',
        storageTotalBytes: '0',
        externalCommittedBytes: '0',
        reciprocityNetBytes: '0',
      },
      freshness: { state: 'LIVE', staleSinceMs: null },
    },
  },
};

const REST_HUB_EMPTY: MyClusterView = {
  agentCid: 'cid_agent_empty',
  devices: [],
  totals: {
    storageUsedBytes: 0,
    storageTotalBytes: 0,
    externalCommittedBytes: 0,
    reciprocityNetBytes: 0,
  },
  freshness: { state: 'live' },
};

// (b) Full cluster fixture — one device, all optional fields populated ------

const GQL_HUB_FULL: ViewerHubResponse = {
  viewer: {
    agentCid: 'cid_agent_matthew',
    hub: {
      agentCid: 'cid_agent_matthew',
      devices: [
        {
          peerId: 'peer_laptop',
          archetype: 'DESKTOP',
          displayName: 'Matthews Laptop',
          online: true,
          freshness: { state: 'STALE', staleSinceMs: '5000' },
          storageUsedBytes: '1073741824',
          storageTotalBytes: '107374182400',
          memoryUsedBytes: '2147483648',
          memoryTotalBytes: '17179869184',
          hostingCount: 12,
          projectingCount: 3,
          beaconAgeMs: '750',
        },
      ],
      totals: {
        storageUsedBytes: '1073741824',
        storageTotalBytes: '107374182400',
        externalCommittedBytes: '536870912',
        reciprocityNetBytes: '-268435456',
      },
      freshness: { state: 'STALE', staleSinceMs: '5000' },
    },
  },
};

const REST_HUB_FULL: MyClusterView = {
  agentCid: 'cid_agent_matthew',
  devices: [
    {
      peerId: 'peer_laptop',
      archetype: 'desktop',
      displayName: 'Matthews Laptop',
      online: true,
      freshness: { state: 'stale', staleSinceMs: 5000 },
      storageUsedBytes: 1073741824,
      storageTotalBytes: 107374182400,
      memoryUsedBytes: 2147483648,
      memoryTotalBytes: 17179869184,
      hostingCount: 12,
      projectingCount: 3,
      beaconAgeMs: 750,
    },
  ],
  totals: {
    storageUsedBytes: 1073741824,
    storageTotalBytes: 107374182400,
    externalCommittedBytes: 536870912,
    reciprocityNetBytes: -268435456,
  },
  freshness: { state: 'stale', staleSinceMs: 5000 },
};

// (c) Null-omission cluster fixture — all optional device fields null -------
// The REST shape must NOT contain the optional keys at all (not undefined — absent).

const GQL_HUB_NULL: ViewerHubResponse = {
  viewer: {
    agentCid: 'cid_agent_sparse',
    hub: {
      agentCid: 'cid_agent_sparse',
      devices: [
        {
          peerId: 'peer_node',
          archetype: 'NODE',
          displayName: null,
          online: false,
          freshness: { state: 'OFFLINE', staleSinceMs: null },
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
      freshness: { state: 'OFFLINE', staleSinceMs: null },
    },
  },
};

const REST_HUB_NULL: MyClusterView = {
  agentCid: 'cid_agent_sparse',
  devices: [
    {
      peerId: 'peer_node',
      archetype: 'node',
      online: false,
      freshness: { state: 'offline' },
      // displayName, storageUsedBytes, storageTotalBytes, memoryUsedBytes,
      // memoryTotalBytes, hostingCount, projectingCount, beaconAgeMs — all absent
    },
  ],
  totals: {
    storageUsedBytes: 0,
    storageTotalBytes: 0,
    externalCommittedBytes: 0,
    reciprocityNetBytes: 0,
  },
  freshness: { state: 'offline' },
};

// ---------------------------------------------------------------------------
// Canonical fixture pairs — PeerTopologyView
// ---------------------------------------------------------------------------

// (a) Empty topology fixture ------------------------------------------------

const GQL_PEERS_EMPTY: ViewerPeersResponse = {
  viewer: {
    agentCid: 'cid_agent_empty',
    peers: {
      agentCid: 'cid_agent_empty',
      edges: [],
      reciprocationCount: 0,
      resilienceCliffs: [],
      freshness: { state: 'LIVE', staleSinceMs: null },
    },
  },
};

const REST_PEERS_EMPTY: PeerTopologyView = {
  agentCid: 'cid_agent_empty',
  edges: [],
  reciprocationCount: 0,
  resilienceCliffs: [],
  freshness: { state: 'live' },
};

// (b) Full topology fixture — one edge, all optional fields populated --------

const GQL_PEERS_FULL: ViewerPeersResponse = {
  viewer: {
    agentCid: 'cid_agent_matthew',
    peers: {
      agentCid: 'cid_agent_matthew',
      edges: [
        {
          householdId: 'hh_adam',
          displayName: 'Adam household',
          online: true,
          lastSyncSec: '42',
          myCidsHostedByThem: 7,
          theirCidsHostedByMe: 3,
          netDiff: '-4',
          isCriticalForMe: true,
          iAmCriticalForThem: false,
        },
      ],
      reciprocationCount: 1,
      resilienceCliffs: [{ householdId: 'hh_adam', soleReplicaCidCount: 2 }],
      freshness: { state: 'LIVE', staleSinceMs: null },
    },
  },
};

const REST_PEERS_FULL: PeerTopologyView = {
  agentCid: 'cid_agent_matthew',
  edges: [
    {
      householdId: 'hh_adam',
      displayName: 'Adam household',
      online: true,
      lastSyncSec: 42,
      myCidsHostedByThem: 7,
      theirCidsHostedByMe: 3,
      netDiff: -4,
      isCriticalForMe: true,
      iAmCriticalForThem: false,
    },
  ],
  reciprocationCount: 1,
  resilienceCliffs: [{ householdId: 'hh_adam', soleReplicaCidCount: 2 }],
  freshness: { state: 'live' },
};

// (c) Null-omission topology fixture — all optional edge fields null ---------

const GQL_PEERS_NULL: ViewerPeersResponse = {
  viewer: {
    agentCid: 'cid_agent_sparse',
    peers: {
      agentCid: 'cid_agent_sparse',
      edges: [
        {
          householdId: 'hh_silent',
          displayName: null,
          online: false,
          lastSyncSec: null,
          myCidsHostedByThem: 0,
          theirCidsHostedByMe: 0,
          netDiff: null,
          isCriticalForMe: null,
          iAmCriticalForThem: null,
        },
      ],
      reciprocationCount: 0,
      resilienceCliffs: [],
      freshness: { state: 'ALL_OFFLINE', staleSinceMs: null },
    },
  },
};

const REST_PEERS_NULL: PeerTopologyView = {
  agentCid: 'cid_agent_sparse',
  edges: [
    {
      householdId: 'hh_silent',
      online: false,
      myCidsHostedByThem: 0,
      theirCidsHostedByMe: 0,
      // displayName, lastSyncSec, netDiff, isCriticalForMe, iAmCriticalForThem — all absent
    },
  ],
  reciprocationCount: 0,
  resilienceCliffs: [],
  freshness: { state: 'all_offline' },
};

// ---------------------------------------------------------------------------
// Cluster view parity tests
// ---------------------------------------------------------------------------

describe('cluster view: empty fixture parity', () => {
  it('adapter output equals REST shape', () => {
    expect(adaptHubResponse(GQL_HUB_EMPTY)).toEqual(REST_HUB_EMPTY);
  });
});

describe('cluster view: full fixture parity', () => {
  it('adapter output equals REST shape', () => {
    expect(adaptHubResponse(GQL_HUB_FULL)).toEqual(REST_HUB_FULL);
  });
});

describe('cluster view: null-omission parity', () => {
  it('adapter output equals REST shape', () => {
    expect(adaptHubResponse(GQL_HUB_NULL)).toEqual(REST_HUB_NULL);
  });

  it('optional keys are absent (not merely undefined) when GQL value is null', () => {
    const adapted = adaptHubResponse(GQL_HUB_NULL);
    const device = adapted.devices[0];
    expect('displayName' in device).toBe(false);
    expect('storageUsedBytes' in device).toBe(false);
    expect('storageTotalBytes' in device).toBe(false);
    expect('memoryUsedBytes' in device).toBe(false);
    expect('memoryTotalBytes' in device).toBe(false);
    expect('hostingCount' in device).toBe(false);
    expect('projectingCount' in device).toBe(false);
    expect('beaconAgeMs' in device).toBe(false);
    expect('staleSinceMs' in device.freshness).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Peer topology parity tests
// ---------------------------------------------------------------------------

describe('peer topology: empty fixture parity', () => {
  it('adapter output equals REST shape', () => {
    expect(adaptPeersResponse(GQL_PEERS_EMPTY)).toEqual(REST_PEERS_EMPTY);
  });
});

describe('peer topology: full fixture parity', () => {
  it('adapter output equals REST shape', () => {
    expect(adaptPeersResponse(GQL_PEERS_FULL)).toEqual(REST_PEERS_FULL);
  });
});

describe('peer topology: null-omission parity', () => {
  it('adapter output equals REST shape', () => {
    expect(adaptPeersResponse(GQL_PEERS_NULL)).toEqual(REST_PEERS_NULL);
  });

  it('optional edge keys are absent (not merely undefined) when GQL value is null', () => {
    const adapted = adaptPeersResponse(GQL_PEERS_NULL);
    const edge = adapted.edges[0];
    expect('displayName' in edge).toBe(false);
    expect('lastSyncSec' in edge).toBe(false);
    expect('netDiff' in edge).toBe(false);
    expect('isCriticalForMe' in edge).toBe(false);
    expect('iAmCriticalForThem' in edge).toBe(false);
    expect('staleSinceMs' in adapted.freshness).toBe(false);
  });
});
