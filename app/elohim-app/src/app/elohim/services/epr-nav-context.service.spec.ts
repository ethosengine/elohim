import { provideHttpClient } from '@angular/common/http';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { firstValueFrom } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { EprNavContextService, type EprNavContextView } from './epr-nav-context.service';

describe('EprNavContextService', () => {
  let svc: EprNavContextService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    svc = TestBed.inject(EprNavContextService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('GETs /api/v1/epr/{cid}/nav-context and returns the body', async () => {
    const promise = firstValueFrom(svc.fetch('abc'));
    const req = http.expectOne('/api/v1/epr/abc/nav-context');
    expect(req.request.method).toBe('GET');
    const view: EprNavContextView = {
      cid: 'abc',
      partOf: [],
      related: [],
      derivedFrom: [],
    };
    req.flush(view);
    expect(await promise).toEqual(view);
  });

  it('returns null on HTTP error (graceful fallback)', async () => {
    const promise = firstValueFrom(svc.fetch('abc'));
    const req = http.expectOne('/api/v1/epr/abc/nav-context');
    req.flush('boom', { status: 500, statusText: 'Server Error' });
    expect(await promise).toBeNull();
  });

  it('URI-encodes the CID', async () => {
    const promise = firstValueFrom(svc.fetch('bafkrei:abc/def'));
    const req = http.expectOne('/api/v1/epr/bafkrei%3Aabc%2Fdef/nav-context');
    req.flush(null);
    await promise;
  });
});
