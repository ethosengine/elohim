import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import { firstValueFrom } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { ResilienceDetailService } from './resilience-detail.service';
import { StorageClientService } from './storage-client.service';

describe('ResilienceDetailService', () => {
  let service: ResilienceDetailService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        {
          provide: StorageClientService,
          useValue: { getStorageBaseUrl: () => 'https://doorway.test' },
        },
      ],
    });
    service = TestBed.inject(ResilienceDetailService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('reads the resilience view from the storage base', async () => {
    const pending = firstValueFrom(service.getResilience('evolution of trust'));
    const req = http.expectOne('https://doorway.test/api/v1/resilience/evolution%20of%20trust');
    expect(req.request.method).toBe('GET');
    req.flush({ contentId: 'evolution of trust' });
    await expect(pending).resolves.toEqual({ contentId: 'evolution of trust' });
  });

  it('asks the storage peer to check the atom is still whole', async () => {
    const pending = firstValueFrom(service.verifyResilience('evolution-of-trust'));
    const req = http.expectOne('https://doorway.test/api/v1/resilience/evolution-of-trust/verify');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual({});
    req.flush({ contentId: 'evolution-of-trust', verified: true });
    await expect(pending).resolves.toMatchObject({ verified: true });
  });
});
