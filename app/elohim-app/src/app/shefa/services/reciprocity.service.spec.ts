import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { ReciprocityService } from './reciprocity.service';

describe('ReciprocityService', () => {
  let service: ReciprocityService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), ReciprocityService],
    });
    service = TestBed.inject(ReciprocityService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('fetches reciprocity from /api/v1/reciprocity', async () => {
    const promise = service.getMyReciprocity();
    const req = http.expectOne('/api/v1/reciprocity');
    req.flush({
      agentCid: 'agent_M',
      inflow: [],
      outflow: [],
      netHostedBytes: 0,
      capacityAvailableBytes: 1_000_000_000,
    });
    const view = await promise;
    expect(view.capacityAvailableBytes).toBe(1_000_000_000);
    expect(service.reciprocity()).toEqual(view);
  });
});
