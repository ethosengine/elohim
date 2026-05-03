import { provideHttpClient } from '@angular/common/http';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { ClusterService } from './cluster.service';

describe('ClusterService', () => {
  let service: ClusterService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), ClusterService],
    });
    service = TestBed.inject(ClusterService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('fetches the cluster view from /api/v1/cluster', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/cluster');
    expect(req.request.method).toBe('GET');
    req.flush({
      agentCid: 'agent_M',
      devices: [],
      totals: {
        storageUsedBytes: 0,
        storageTotalBytes: 0,
        externalCommittedBytes: 0,
        reciprocityNetBytes: 0,
      },
      freshness: { state: 'live' },
    });
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
