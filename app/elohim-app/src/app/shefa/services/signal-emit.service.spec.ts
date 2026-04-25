import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { SignalEmitService, type SignalIntent } from './signal-emit.service';

describe('SignalEmitService', () => {
  let service: SignalEmitService;
  let httpMock: HttpTestingController;

  const stubIntent: SignalIntent = {
    pillar: 'shefa',
    signalType: 'EconomicEvent',
    agentCid: 'agent-cid-123',
    payload: { action: 'produce' },
    couplingRefs: { knowledge: 'node-1' },
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [SignalEmitService, provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(SignalEmitService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('returns emitted on HTTP 201', async () => {
    const promise = service.tryEmit(stubIntent);
    const req = httpMock.expectOne('/api/v1/signal/emit');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(stubIntent);
    req.flush({ eprCid: 'cid-1', eventCid: 'cid-1' }, { status: 201, statusText: 'Created' });

    const result = await promise;
    expect(result.status).toBe('emitted');
    if (result.status === 'emitted') {
      expect(result.response.eprCid).toBe('cid-1');
    }
  });

  it('returns fallback on HTTP 503 (write-through OFF)', async () => {
    const promise = service.tryEmit(stubIntent);
    const req = httpMock.expectOne('/api/v1/signal/emit');
    req.flush(
      { error: 'write-through is OFF for pillar=shefa' },
      { status: 503, statusText: 'Service Unavailable' }
    );

    const result = await promise;
    expect(result.status).toBe('fallback');
    if (result.status === 'fallback') {
      expect(result.reason).toContain('OFF');
    }
  });

  it('returns error on HTTP 400', async () => {
    const promise = service.tryEmit(stubIntent);
    const req = httpMock.expectOne('/api/v1/signal/emit');
    req.flush({ error: 'invalid agent cid' }, { status: 400, statusText: 'Bad Request' });

    const result = await promise;
    expect(result.status).toBe('error');
    if (result.status === 'error') {
      expect(result.status_code).toBe(400);
    }
  });

  it('returns error on HTTP 500', async () => {
    const promise = service.tryEmit(stubIntent);
    const req = httpMock.expectOne('/api/v1/signal/emit');
    req.flush(
      { error: 'ingest failed' },
      { status: 500, statusText: 'Internal Server Error' }
    );

    const result = await promise;
    expect(result.status).toBe('error');
    if (result.status === 'error') {
      expect(result.status_code).toBe(500);
    }
  });
});
