import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { DistributionService } from './distribution.service';

describe('DistributionService', () => {
  let service: DistributionService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(DistributionService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('GETs /api/v1/blob/{hash}/distribution/details', async () => {
    const promise = service.getDetails('sha256-abc');
    const req = http.expectOne('/api/v1/blob/sha256-abc/distribution/details');
    expect(req.request.method).toBe('GET');
    req.flush({
      summary: {
        replicaCount: 2,
        replicaTarget: 3,
        replicaHealth: 'at_risk',
        projectorCount: 0,
        reachClass: 'household',
        diversityHint: { kind: 'none', value: null },
        thisFetchSource: 'projected_via_doorway',
        lastVerifiedSeconds: 30,
      },
      replicaPeers: [],
      projectorIdentities: [],
      placementGaps: [],
      recentProjectionEvents: [],
    });
    const result = await promise;
    expect(result.summary.replicaCount).toBe(2);
  });
});
