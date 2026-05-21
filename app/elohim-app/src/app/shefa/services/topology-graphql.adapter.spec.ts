import { describe, it, expect } from 'vitest';

import type { ViewerHubResponse, ViewerPeersResponse } from '@elohim/storage-client/graphql';

import { adaptHubResponse, adaptPeersResponse } from './topology-graphql.adapter';

// ---------------------------------------------------------------------------
// Shared fixture builders
// ---------------------------------------------------------------------------

function makeFreshnessGql(
  state: 'LIVE' | 'STALE' | 'OFFLINE' | 'CACHED_OFFLINE_UNTIL_RECONNECT' | 'UNVERIFIABLE' | 'ALL_OFFLINE',
  staleSinceMs: string | null = null,
) {
  return { state, staleSinceMs };
}

function makeHubResponse(overrides: Partial<ViewerHubResponse['viewer']['hub']> = {}): ViewerHubResponse {
  return {
    viewer: {
      agentCid: 'agent_test',
      hub: {
        agentCid: 'agent_test',
        devices: [],
        totals: {
          storageUsedBytes: '1000',
          storageTotalBytes: '2000',
          externalCommittedBytes: '300',
          reciprocityNetBytes: '-50',
        },
        freshness: makeFreshnessGql('LIVE'),
        ...overrides,
      },
    },
  };
}

function makePeersResponse(overrides: Partial<ViewerPeersResponse['viewer']['peers']> = {}): ViewerPeersResponse {
  return {
    viewer: {
      agentCid: 'agent_test',
      peers: {
        agentCid: 'agent_test',
        edges: [],
        reciprocationCount: 0,
        resilienceCliffs: [],
        freshness: makeFreshnessGql('LIVE'),
        ...overrides,
      },
    },
  };
}

// ---------------------------------------------------------------------------
// adaptHubResponse
// ---------------------------------------------------------------------------

describe('adaptHubResponse', () => {
  it('passes agentCid through', () => {
    const view = adaptHubResponse(makeHubResponse());
    expect(view.agentCid).toBe('agent_test');
  });

  it('lowercases LIVE freshness state', () => {
    const view = adaptHubResponse(makeHubResponse({ freshness: makeFreshnessGql('LIVE') }));
    expect(view.freshness.state).toBe('live');
  });

  it('lowercases STALE freshness state', () => {
    const view = adaptHubResponse(makeHubResponse({ freshness: makeFreshnessGql('STALE', '1234567890') }));
    expect(view.freshness.state).toBe('stale');
    expect(view.freshness.staleSinceMs).toBe(1234567890);
  });

  it('lowercases CACHED_OFFLINE_UNTIL_RECONNECT freshness state', () => {
    const view = adaptHubResponse(makeHubResponse({ freshness: makeFreshnessGql('CACHED_OFFLINE_UNTIL_RECONNECT') }));
    expect(view.freshness.state).toBe('cached_offline_until_reconnect');
  });

  it('converts totals string fields to numbers', () => {
    const view = adaptHubResponse(makeHubResponse());
    expect(view.totals.storageUsedBytes).toBe(1000);
    expect(view.totals.storageTotalBytes).toBe(2000);
    expect(view.totals.externalCommittedBytes).toBe(300);
    expect(view.totals.reciprocityNetBytes).toBe(-50);
  });

  it('converts device byte fields from nullable string to optional number', () => {
    const view = adaptHubResponse(makeHubResponse({
      devices: [{
        peerId: 'peer_1',
        archetype: 'NODE',
        displayName: null,
        online: true,
        freshness: makeFreshnessGql('LIVE'),
        storageUsedBytes: '512000',
        storageTotalBytes: '1024000',
        memoryUsedBytes: null,
        memoryTotalBytes: null,
        hostingCount: 5,
        projectingCount: null,
        beaconAgeMs: '250',
        compute: null,
      }],
    }));
    const device = view.devices[0];
    expect(device.storageUsedBytes).toBe(512000);
    expect(device.storageTotalBytes).toBe(1024000);
    expect(device.memoryUsedBytes).toBeUndefined();
    expect(device.memoryTotalBytes).toBeUndefined();
    expect(device.hostingCount).toBe(5);
    expect(device.projectingCount).toBeUndefined();
    expect(device.beaconAgeMs).toBe(250);
  });

  it('lowercases device archetype', () => {
    const view = adaptHubResponse(makeHubResponse({
      devices: [{
        peerId: 'peer_1',
        archetype: 'DESKTOP',
        displayName: 'My Laptop',
        online: false,
        freshness: makeFreshnessGql('OFFLINE'),
        storageUsedBytes: null,
        storageTotalBytes: null,
        memoryUsedBytes: null,
        memoryTotalBytes: null,
        hostingCount: null,
        projectingCount: null,
        beaconAgeMs: null,
        compute: null,
      }],
    }));
    expect(view.devices[0].archetype).toBe('desktop');
  });

  it('sets displayName when non-null', () => {
    const view = adaptHubResponse(makeHubResponse({
      devices: [{
        peerId: 'peer_1',
        archetype: 'MOBILE',
        displayName: 'Phone',
        online: true,
        freshness: makeFreshnessGql('LIVE'),
        storageUsedBytes: null,
        storageTotalBytes: null,
        memoryUsedBytes: null,
        memoryTotalBytes: null,
        hostingCount: null,
        projectingCount: null,
        beaconAgeMs: null,
        compute: null,
      }],
    }));
    expect(view.devices[0].displayName).toBe('Phone');
  });

  it('omits displayName when null', () => {
    const view = adaptHubResponse(makeHubResponse({
      devices: [{
        peerId: 'peer_1',
        archetype: 'NODE',
        displayName: null,
        online: true,
        freshness: makeFreshnessGql('LIVE'),
        storageUsedBytes: null,
        storageTotalBytes: null,
        memoryUsedBytes: null,
        memoryTotalBytes: null,
        hostingCount: null,
        projectingCount: null,
        beaconAgeMs: null,
        compute: null,
      }],
    }));
    expect('displayName' in view.devices[0]).toBe(false);
  });

  it('converts compute triptych string bytes to numbers', () => {
    // After the HubComputeAggregateView landing (92cad4734) the canonical
    // MyClusterView['devices'][number].compute is `ComputeTriptych` with
    // number-typed bytes. The adapter converts gql string bytes via Number()
    // following the same precision trade-off as the storage/memory counters.
    const view = adaptHubResponse(makeHubResponse({
      devices: [{
        peerId: 'peer_M',
        archetype: 'DESKTOP',
        displayName: 'laptop',
        online: true,
        freshness: makeFreshnessGql('LIVE'),
        storageUsedBytes: '500000',
        storageTotalBytes: '10000000',
        memoryUsedBytes: null,
        memoryTotalBytes: null,
        hostingCount: null,
        projectingCount: null,
        beaconAgeMs: null,
        compute: { free: '9500000', used: '500000', stewarded: '250000' },
      }],
    }));
    expect(view.devices[0].compute).toEqual({ free: 9500000, used: 500000, stewarded: 250000 });
  });

  it('omits compute when GraphQL compute is null', () => {
    const view = adaptHubResponse(makeHubResponse({
      devices: [{
        peerId: 'peer_M',
        archetype: 'NODE',
        displayName: null,
        online: true,
        freshness: makeFreshnessGql('LIVE'),
        storageUsedBytes: null,
        storageTotalBytes: null,
        memoryUsedBytes: null,
        memoryTotalBytes: null,
        hostingCount: null,
        projectingCount: null,
        beaconAgeMs: null,
        compute: null,
      }],
    }));
    expect('compute' in view.devices[0]).toBe(false);
  });

  it('omits staleSinceMs when null', () => {
    const view = adaptHubResponse(makeHubResponse({ freshness: makeFreshnessGql('LIVE', null) }));
    expect('staleSinceMs' in view.freshness).toBe(false);
  });

  it('returns staleSinceMs as number when present', () => {
    const view = adaptHubResponse(makeHubResponse({ freshness: makeFreshnessGql('STALE', '9999') }));
    expect(view.freshness.staleSinceMs).toBe(9999);
  });
});

// ---------------------------------------------------------------------------
// adaptPeersResponse
// ---------------------------------------------------------------------------

describe('adaptPeersResponse', () => {
  it('passes agentCid through', () => {
    const view = adaptPeersResponse(makePeersResponse());
    expect(view.agentCid).toBe('agent_test');
  });

  it('lowercases freshness state', () => {
    const view = adaptPeersResponse(makePeersResponse({ freshness: makeFreshnessGql('ALL_OFFLINE') }));
    expect(view.freshness.state).toBe('all_offline');
  });

  it('passes reciprocationCount through', () => {
    const view = adaptPeersResponse(makePeersResponse({ reciprocationCount: 3 }));
    expect(view.reciprocationCount).toBe(3);
  });

  it('maps resilienceCliffs correctly', () => {
    const view = adaptPeersResponse(makePeersResponse({
      resilienceCliffs: [{ householdId: 'hh_adam', soleReplicaCidCount: 12 }],
    }));
    expect(view.resilienceCliffs).toEqual([{ householdId: 'hh_adam', soleReplicaCidCount: 12 }]);
  });

  it('converts edge lastSyncSec from nullable string to optional number', () => {
    const view = adaptPeersResponse(makePeersResponse({
      edges: [{
        householdId: 'hh_1',
        displayName: null,
        online: true,
        lastSyncSec: '42',
        myCidsHostedByThem: 7,
        theirCidsHostedByMe: 3,
        netDiff: '-4',
        isCriticalForMe: false,
        iAmCriticalForThem: null,
      }],
    }));
    const edge = view.edges[0];
    expect(edge.lastSyncSec).toBe(42);
    expect(edge.netDiff).toBe(-4);
    expect(edge.isCriticalForMe).toBe(false);
    expect('iAmCriticalForThem' in edge).toBe(false);
  });

  it('omits edge lastSyncSec when null', () => {
    const view = adaptPeersResponse(makePeersResponse({
      edges: [{
        householdId: 'hh_1',
        displayName: null,
        online: false,
        lastSyncSec: null,
        myCidsHostedByThem: 0,
        theirCidsHostedByMe: 0,
        netDiff: null,
        isCriticalForMe: null,
        iAmCriticalForThem: null,
      }],
    }));
    expect('lastSyncSec' in view.edges[0]).toBe(false);
    expect('netDiff' in view.edges[0]).toBe(false);
  });

  it('sets edge displayName when non-null', () => {
    const view = adaptPeersResponse(makePeersResponse({
      edges: [{
        householdId: 'hh_1',
        displayName: 'Adam household',
        online: true,
        lastSyncSec: null,
        myCidsHostedByThem: 1,
        theirCidsHostedByMe: 1,
        netDiff: null,
        isCriticalForMe: null,
        iAmCriticalForThem: null,
      }],
    }));
    expect(view.edges[0].displayName).toBe('Adam household');
  });
});
