import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { PeerTopologyService } from './peer-topology.service';

describe('PeerTopologyService', () => {
  let service: PeerTopologyService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), PeerTopologyService],
    });
    service = TestBed.inject(PeerTopologyService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('fetches peer topology from /api/v1/peer-topology', async () => {
    const promise = service.getMyPeerTopology();
    const req = http.expectOne('/api/v1/peer-topology');
    req.flush({
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
    });
    const view = await promise;
    expect(view.edges.length).toBe(1);
    expect(service.topology()).toEqual(view);
  });
});
