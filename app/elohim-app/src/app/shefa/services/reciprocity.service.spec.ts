import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { AgentService } from '@app/elohim/services/agent.service';

import { environment } from '../../../environments/environment';

import { ReciprocityService } from './reciprocity.service';

// ---------------------------------------------------------------------------
// Shared REST fixture (matches `/api/v1/reciprocity` response shape)
// ---------------------------------------------------------------------------

const REST_VIEW = {
  agentCid: 'agent_M',
  inflow: [
    {
      counterpartyHouseholdId: 'household-terrance',
      displayName: 'Terrance',
      committedBytes: 1_000_000,
      deliveredBytes: 500_000,
      honoredPercent: 0.5,
      online: true,
    },
  ],
  outflow: [],
  netHostedBytes: 500_000,
  capacityAvailableBytes: 1_000_000_000,
};

// ---------------------------------------------------------------------------
// GraphQL fixture (stringified u64 counters per resolver contract)
// ---------------------------------------------------------------------------

const GQL_ENVELOPE = {
  data: {
    viewer: {
      agentCid: 'agent_M',
      reciprocity: {
        agentCid: 'agent_M',
        inflow: [
          {
            counterpartyHouseholdId: 'household-terrance',
            displayName: 'Terrance',
            committedBytes: '1000000',
            deliveredBytes: '500000',
            honoredPercent: 0.5,
            online: true,
          },
        ],
        outflow: [],
        netHostedBytes: '500000',
        capacityAvailableBytes: '1000000000',
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
      ReciprocityService,
      {
        provide: AgentService,
        useValue: { getCurrentAgentId: () => 'agent_M' },
      },
    ],
  });
  return {
    service: TestBed.inject(ReciprocityService),
    http: TestBed.inject(HttpTestingController),
  };
}

// ---------------------------------------------------------------------------
// Flag OFF — REST path
// ---------------------------------------------------------------------------

describe('ReciprocityService — flag OFF (REST path)', () => {
  let service: ReciprocityService;
  let http: HttpTestingController;

  beforeEach(() => {
    environment.features = { useGraphqlTopology: false };
    ({ service, http } = setupTestBed());
  });

  afterEach(() => {
    http.verify();
    TestBed.resetTestingModule();
  });

  it('fetches reciprocity from GET /api/v1/reciprocity', async () => {
    const promise = service.getReciprocity();
    const req = http.expectOne('/api/v1/reciprocity');
    expect(req.request.method).toBe('GET');
    req.flush(REST_VIEW);
    const view = await promise;
    expect(view.capacityAvailableBytes).toBe(1_000_000_000);
    expect(view.inflow[0].counterpartyHouseholdId).toBe('household-terrance');
    expect(service.reciprocity()).toEqual(view);
  });

  it('records error on fetch failure', async () => {
    const promise = service.getReciprocity();
    const req = http.expectOne('/api/v1/reciprocity');
    req.flush('boom', { status: 500, statusText: 'Server Error' });
    await expect(promise).rejects.toBeTruthy();
    expect(service.error()).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Flag ON — GraphQL path
// ---------------------------------------------------------------------------

describe('ReciprocityService — flag ON (GraphQL path)', () => {
  let service: ReciprocityService;
  let http: HttpTestingController;

  beforeEach(() => {
    environment.features = { useGraphqlTopology: true };
    ({ service, http } = setupTestBed());
  });

  afterEach(() => {
    http.verify();
    TestBed.resetTestingModule();
  });

  it('POSTs to /api/v1/graphql with VIEWER_RECIPROCITY_QUERY', async () => {
    const promise = service.getReciprocity();
    const req = http.expectOne('/api/v1/graphql');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toMatchObject({ variables: { agentCid: 'agent_M' } });
    expect(typeof req.request.body.query).toBe('string');
    expect(req.request.body.query).toContain('ViewerReciprocity');
    req.flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.agentCid).toBe('agent_M');
    expect(service.reciprocity()).toEqual(view);
  });

  it('adapts string byte counters to numbers', async () => {
    const promise = service.getReciprocity();
    const req = http.expectOne('/api/v1/graphql');
    req.flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.inflow[0].committedBytes).toBe(1_000_000);
    expect(view.inflow[0].deliveredBytes).toBe(500_000);
    expect(view.netHostedBytes).toBe(500_000);
    expect(view.capacityAvailableBytes).toBe(1_000_000_000);
  });

  it('passes through honoredPercent as a number', async () => {
    const promise = service.getReciprocity();
    http.expectOne('/api/v1/graphql').flush(GQL_ENVELOPE);
    const view = await promise;
    expect(view.inflow[0].honoredPercent).toBe(0.5);
  });

  it('omits null optional row fields (displayName / online)', async () => {
    const envelope = {
      data: {
        viewer: {
          agentCid: 'agent_M',
          reciprocity: {
            agentCid: 'agent_M',
            inflow: [
              {
                counterpartyHouseholdId: 'household-anon',
                displayName: null,
                committedBytes: '0',
                deliveredBytes: '0',
                honoredPercent: 0,
                online: null,
              },
            ],
            outflow: [],
            netHostedBytes: '0',
            capacityAvailableBytes: '0',
          },
        },
      },
    };
    const promise = service.getReciprocity();
    http.expectOne('/api/v1/graphql').flush(envelope);
    const view = await promise;
    expect('displayName' in view.inflow[0]).toBe(false);
    expect('online' in view.inflow[0]).toBe(false);
  });

  it('throws and sets error signal on GraphQL errors field', async () => {
    const promise = service.getReciprocity();
    http.expectOne('/api/v1/graphql').flush({ errors: [{ message: 'resolver error' }] });
    await expect(promise).rejects.toThrow('resolver error');
    expect(service.error()).toContain('resolver error');
  });

  it('throws and sets error signal when data is absent', async () => {
    const promise = service.getReciprocity();
    http.expectOne('/api/v1/graphql').flush({ data: null });
    await expect(promise).rejects.toThrow('no data');
    expect(service.error()).not.toBeNull();
  });
});
