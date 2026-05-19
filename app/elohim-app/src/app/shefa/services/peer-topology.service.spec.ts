import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { AgentService } from '@app/elohim/services/agent.service';

import { environment } from '../../../environments/environment';

import { PeerTopologyService } from './peer-topology.service';

// ---------------------------------------------------------------------------
// REST fixture
// ---------------------------------------------------------------------------

const REST_VIEW = {
  agentCid: 'agent_M',
  edges: [
    {
      householdId: 'adam',
      online: true,
      myCidsHostedByThem: 7,
      theirCidsHostedByMe: 12,
      netDiff: 5,
    },
  ],
  reciprocationCount: 1,
  resilienceCliffs: [],
  freshness: { state: 'live' },
};

// ---------------------------------------------------------------------------
// GraphQL fixture
// ---------------------------------------------------------------------------

const GQL_ENVELOPE = {
  data: {
    viewer: {
      agentCid: 'agent_M',
      peers: {
        agentCid: 'agent_M',
        edges: [
          {
            householdId: 'adam',
            displayName: 'Adam household',
            online: true,
            lastSyncSec: '30',
            myCidsHostedByThem: 7,
            theirCidsHostedByMe: 12,
            netDiff: '5',
            isCriticalForMe: false,
            iAmCriticalForThem: null,
          },
        ],
        reciprocationCount: 1,
        resilienceCliffs: [{ householdId: 'adam', soleReplicaCidCount: 2 }],
        freshness: { state: 'STALE', staleSinceMs: '12345' },
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Shared TestBed setup
// ---------------------------------------------------------------------------

function setupTestBed() {
  TestBed.configureTestingModule({
    providers: [
      provideHttpClient(),
      provideHttpClientTesting(),
      PeerTopologyService,
      {
        provide: AgentService,
        useValue: { getCurrentAgentId: () => 'agent_M' },
      },
    ],
  });
  return {
    service: TestBed.inject(PeerTopologyService),
    http: TestBed.inject(HttpTestingController),
  };
}

// ---------------------------------------------------------------------------
// Flag OFF (default) — REST path
// ---------------------------------------------------------------------------

describe('PeerTopologyService — flag OFF (REST path)', () => {
  let service: PeerTopologyService;
  let http: HttpTestingController;

  beforeEach(() => {
    environment.features = { useGraphqlTopology: false };
    ({ service, http } = setupTestBed());
  });

  afterEach(() => {
    http.verify();
    TestBed.resetTestingModule();
  });

  it('fetches peer topology from GET /api/v1/peer-topology', async () => {
    const promise = service.getMyPeerTopology();
    const req = http.expectOne('/api/v1/peer-topology');
    expect(req.request.method).toBe('GET');
    req.flush(REST_VIEW);
    const view = await promise;
    expect(view.edges.length).toBe(1);
    expect(service.topology()).toEqual(view);
  });

  it('starts with topology signal null', () => {
    expect(service.topology()).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Flag ON — GraphQL path
// ---------------------------------------------------------------------------

describe('PeerTopologyService — flag ON (GraphQL path)', () => {
  let service: PeerTopologyService;
  let http: HttpTestingController;

  beforeEach(() => {
    environment.features = { useGraphqlTopology: true };
    ({ service, http } = setupTestBed());
  });

  afterEach(() => {
    http.verify();
    TestBed.resetTestingModule();
  });

  it('POSTs to /api/v1/graphql with VIEWER_PEERS_QUERY', async () => {
    const promise = service.getMyPeerTopology();
    const req = http.expectOne('/api/v1/graphql');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toMatchObject({ variables: { agentCid: 'agent_M' } });
    expect(req.request.body.query).toContain('ViewerPeers');
    req.flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.agentCid).toBe('agent_M');
    expect(service.topology()).toEqual(view);
  });

  it('adapts freshness state to lowercase', async () => {
    const promise = service.getMyPeerTopology();
    http.expectOne('/api/v1/graphql').flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.freshness.state).toBe('stale');
    expect(view.freshness.staleSinceMs).toBe(12345);
  });

  it('adapts edge lastSyncSec and netDiff from string to number', async () => {
    const promise = service.getMyPeerTopology();
    http.expectOne('/api/v1/graphql').flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.edges[0].lastSyncSec).toBe(30);
    expect(view.edges[0].netDiff).toBe(5);
  });

  it('omits iAmCriticalForThem when null', async () => {
    const promise = service.getMyPeerTopology();
    http.expectOne('/api/v1/graphql').flush(GQL_ENVELOPE);
    const view = await promise;
    expect('iAmCriticalForThem' in view.edges[0]).toBe(false);
  });

  it('maps resilienceCliffs correctly', async () => {
    const promise = service.getMyPeerTopology();
    http.expectOne('/api/v1/graphql').flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.resilienceCliffs).toEqual([{ householdId: 'adam', soleReplicaCidCount: 2 }]);
  });

  it('sets topology signal on success', async () => {
    const promise = service.getMyPeerTopology();
    http.expectOne('/api/v1/graphql').flush(GQL_ENVELOPE);
    await promise;
    expect(service.topology()).not.toBeNull();
  });

  it('throws on GraphQL errors field', async () => {
    const promise = service.getMyPeerTopology();
    http.expectOne('/api/v1/graphql').flush({ errors: [{ message: 'peers resolver failed' }] });
    await expect(promise).rejects.toThrow('peers resolver failed');
  });

  it('throws when data is absent', async () => {
    const promise = service.getMyPeerTopology();
    http.expectOne('/api/v1/graphql').flush({ data: null });
    await expect(promise).rejects.toThrow('no data');
  });
});
