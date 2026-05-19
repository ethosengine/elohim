import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { AgentService } from '@app/elohim/services/agent.service';

import { environment } from '../../../environments/environment';

import { ClusterService } from './cluster.service';

// ---------------------------------------------------------------------------
// Shared REST fixture
// ---------------------------------------------------------------------------

const REST_VIEW = {
  agentCid: 'agent_M',
  devices: [],
  totals: {
    storageUsedBytes: 0,
    storageTotalBytes: 0,
    externalCommittedBytes: 0,
    reciprocityNetBytes: 0,
  },
  freshness: { state: 'live' },
};

// ---------------------------------------------------------------------------
// GraphQL fixture (SCREAMING_SNAKE_CASE enums, string byte counters)
// ---------------------------------------------------------------------------

const GQL_ENVELOPE = {
  data: {
    viewer: {
      agentCid: 'agent_M',
      hub: {
        agentCid: 'agent_M',
        devices: [
          {
            peerId: 'peer_1',
            archetype: 'NODE',
            displayName: 'blade-01',
            online: true,
            freshness: { state: 'LIVE', staleSinceMs: null },
            storageUsedBytes: '500000',
            storageTotalBytes: '1000000',
            memoryUsedBytes: null,
            memoryTotalBytes: null,
            hostingCount: 3,
            projectingCount: null,
            beaconAgeMs: '120',
          },
        ],
        totals: {
          storageUsedBytes: '500000',
          storageTotalBytes: '1000000',
          externalCommittedBytes: '0',
          reciprocityNetBytes: '0',
        },
        freshness: { state: 'LIVE', staleSinceMs: null },
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
      ClusterService,
      {
        provide: AgentService,
        useValue: { getCurrentAgentId: () => 'agent_M' },
      },
    ],
  });
  return {
    service: TestBed.inject(ClusterService),
    http: TestBed.inject(HttpTestingController),
  };
}

// ---------------------------------------------------------------------------
// Flag OFF (default) — REST path
// ---------------------------------------------------------------------------

describe('ClusterService — flag OFF (REST path)', () => {
  let service: ClusterService;
  let http: HttpTestingController;

  beforeEach(() => {
    environment.features = { useGraphqlTopology: false };
    ({ service, http } = setupTestBed());
  });

  afterEach(() => {
    http.verify();
    TestBed.resetTestingModule();
  });

  it('fetches the cluster view from GET /api/v1/cluster', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/cluster');
    expect(req.request.method).toBe('GET');
    req.flush(REST_VIEW);
    const view = await promise;
    expect(view.agentCid).toBe('agent_M');
    expect(service.cluster()).toEqual(view);
  });

  it('starts with cluster signal null', () => {
    expect(service.cluster()).toBeNull();
  });

  it('records error on fetch failure', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/cluster');
    req.flush('boom', { status: 500, statusText: 'Server Error' });
    await expect(promise).rejects.toBeTruthy();
    expect(service.error()).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Flag ON — GraphQL path
// ---------------------------------------------------------------------------

describe('ClusterService — flag ON (GraphQL path)', () => {
  let service: ClusterService;
  let http: HttpTestingController;

  beforeEach(() => {
    environment.features = { useGraphqlTopology: true };
    ({ service, http } = setupTestBed());
  });

  afterEach(() => {
    http.verify();
    TestBed.resetTestingModule();
  });

  it('POSTs to /api/v1/graphql with VIEWER_HUB_QUERY', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/graphql');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toMatchObject({ variables: { agentCid: 'agent_M' } });
    expect(typeof req.request.body.query).toBe('string');
    expect(req.request.body.query).toContain('ViewerHub');
    req.flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.agentCid).toBe('agent_M');
    expect(service.cluster()).toEqual(view);
  });

  it('adapts SCREAMING_SNAKE_CASE enums to lowercase', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/graphql');
    req.flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.freshness.state).toBe('live');
    expect(view.devices[0].archetype).toBe('node');
    expect(view.devices[0].freshness.state).toBe('live');
  });

  it('adapts string byte counters to numbers', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/graphql');
    req.flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.totals.storageUsedBytes).toBe(500000);
    expect(view.devices[0].storageUsedBytes).toBe(500000);
    expect(view.devices[0].beaconAgeMs).toBe(120);
  });

  it('omits null optional device fields', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/graphql');
    req.flush(GQL_ENVELOPE);
    const view = await promise;
    expect('memoryUsedBytes' in view.devices[0]).toBe(false);
    expect('projectingCount' in view.devices[0]).toBe(false);
  });

  it('sets the cluster signal on success', async () => {
    const promise = service.getMyCluster();
    http.expectOne('/api/v1/graphql').flush(GQL_ENVELOPE);
    await promise;
    expect(service.cluster()).not.toBeNull();
  });

  it('throws and sets error signal on GraphQL errors field', async () => {
    const promise = service.getMyCluster();
    http.expectOne('/api/v1/graphql').flush({ errors: [{ message: 'resolver error' }] });
    await expect(promise).rejects.toThrow('resolver error');
    expect(service.error()).toContain('resolver error');
  });

  it('throws and sets error signal when data is absent', async () => {
    const promise = service.getMyCluster();
    http.expectOne('/api/v1/graphql').flush({ data: null });
    await expect(promise).rejects.toThrow('no data');
    expect(service.error()).not.toBeNull();
  });
});
