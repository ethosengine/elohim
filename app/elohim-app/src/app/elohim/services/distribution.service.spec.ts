import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { DistributionService } from './distribution.service';

describe('DistributionService', () => {
  let service: DistributionService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), DistributionService],
    });
    service = TestBed.inject(DistributionService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('fetches details from /api/v1/blob/:hash/distribution/details', async () => {
    const promise = service.getDetails('hash_xyz');
    const req = http.expectOne('/api/v1/blob/hash_xyz/distribution/details');
    expect(req.request.method).toBe('GET');
    req.flush({
      summary: { replicaCount: 5 },
      replicaPeers: [],
      projectorIdentities: [],
      placementGaps: [],
      recentProjectionEvents: [],
    });
    const result = await promise;
    expect(result.summary.replicaCount).toBe(5);
  });
});
