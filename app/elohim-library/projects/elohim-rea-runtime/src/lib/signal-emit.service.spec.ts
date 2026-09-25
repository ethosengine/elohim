/**
 * SignalEmitService Tests
 *
 * Ruling R-A16: a signal asks the node's write-through status once per session
 * (GET /api/v1/status/write-through, memoised) and posts to
 * POST /api/v1/signal/emit only when its (pillar, kind) pair is effective-on.
 * A pair the node declares off — or a status the app could not read — takes
 * the legacy path with no request at all: a routine 503 per page view is a
 * browser console error, and the node already said the answer. A 503 on an
 * actual post (the status went stale) still falls back as before.
 */

import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { SignalEmitService, WRITE_THROUGH_STATUS_URL } from './signal-emit.service';
import type { SignalIntent, WriteThroughStatusView } from './signal-emit-types';

const EMIT_URL = '/api/v1/signal/emit';

describe('SignalEmitService', () => {
  let service: SignalEmitService;
  let httpMock: HttpTestingController;

  const intent: SignalIntent = {
    pillar: 'shefa',
    signalType: 'EconomicEvent',
    agentCid: 'agent-cid-123',
    payload: { action: 'produce' },
    couplingRefs: { knowledge: 'node-1' },
  };

  const status = (on: boolean): WriteThroughStatusView => ({
    effective: [{ pillar: 'shefa', kind: 'EconomicEvent', on, source: 'manifest-default' }],
    integrityKinds: ['KeyRotation', 'KeyRevocation', 'RevocationAttestation', 'AgentPeerBinding'],
    adminOverride: null,
  });

  /** Let the memoised status promise settle before the post is expected. */
  const settle = (): Promise<void> => new Promise(resolve => setTimeout(resolve, 0));

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [SignalEmitService, provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(SignalEmitService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('emit_skips_the_write_through_post_when_status_says_off', async () => {
    const promise = service.tryEmit(intent);
    httpMock.expectOne(WRITE_THROUGH_STATUS_URL).flush(status(false));

    const result = await promise;
    expect(result.status).toBe('fallback');
    httpMock.expectNone(EMIT_URL);
  });

  it('status_is_read_once_per_session', async () => {
    const first = service.tryEmit(intent);
    const second = service.tryEmit(intent);
    httpMock.expectOne(WRITE_THROUGH_STATUS_URL).flush(status(false));
    await Promise.all([first, second]);

    const third = await service.tryEmit(intent);
    expect(third.status).toBe('fallback');
    httpMock.expectNone(WRITE_THROUGH_STATUS_URL);
    httpMock.expectNone(EMIT_URL);
  });

  it('emit_posts_when_status_says_on', async () => {
    const promise = service.tryEmit(intent);
    httpMock.expectOne(WRITE_THROUGH_STATUS_URL).flush(status(true));
    await settle();

    const req = httpMock.expectOne(EMIT_URL);
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(intent);
    req.flush({ eprCid: 'cid-1', eventCid: 'cid-1' }, { status: 201, statusText: 'Created' });

    const result = await promise;
    expect(result.status).toBe('emitted');
    if (result.status === 'emitted') {
      expect(result.response.eprCid).toBe('cid-1');
    }
  });

  it('status_read_failure_means_no_post', async () => {
    const promise = service.tryEmit(intent);
    httpMock
      .expectOne(WRITE_THROUGH_STATUS_URL)
      .flush({ error: 'not found' }, { status: 404, statusText: 'Not Found' });

    const result = await promise;
    expect(result.status).toBe('fallback');
    httpMock.expectNone(EMIT_URL);

    // The failed read is the session's answer too: no retry, still no post.
    const again = await service.tryEmit(intent);
    expect(again.status).toBe('fallback');
    httpMock.expectNone(WRITE_THROUGH_STATUS_URL);
    httpMock.expectNone(EMIT_URL);
  });

  it('a pair the status does not list is not on', async () => {
    const promise = service.tryEmit({ ...intent, pillar: 'lamad', signalType: 'Observation' });
    httpMock.expectOne(WRITE_THROUGH_STATUS_URL).flush(status(true));

    expect((await promise).status).toBe('fallback');
    httpMock.expectNone(EMIT_URL);
  });

  it('an integrity kind posts whatever the pillar rows say', async () => {
    const promise = service.tryEmit({ ...intent, pillar: 'imagodei', signalType: 'KeyRotation' });
    httpMock.expectOne(WRITE_THROUGH_STATUS_URL).flush(status(false));
    await settle();

    httpMock
      .expectOne(EMIT_URL)
      .flush({ eprCid: 'cid-2', eventCid: 'cid-2' }, { status: 201, statusText: 'Created' });
    expect((await promise).status).toBe('emitted');
  });

  it('a 503 on an actual post still falls back (the status went stale)', async () => {
    const promise = service.tryEmit(intent);
    httpMock.expectOne(WRITE_THROUGH_STATUS_URL).flush(status(true));
    await settle();

    httpMock
      .expectOne(EMIT_URL)
      .flush(
        { error: 'write-through is OFF for pillar=shefa' },
        { status: 503, statusText: 'Service Unavailable' }
      );

    const result = await promise;
    expect(result.status).toBe('fallback');
    if (result.status === 'fallback') {
      expect(result.reason).toContain('OFF');
    }
  });

  it('a non-503 failure on an actual post is an error', async () => {
    const promise = service.tryEmit(intent);
    httpMock.expectOne(WRITE_THROUGH_STATUS_URL).flush(status(true));
    await settle();

    httpMock
      .expectOne(EMIT_URL)
      .flush({ error: 'ingest failed' }, { status: 500, statusText: 'Internal Server Error' });

    const result = await promise;
    expect(result.status).toBe('error');
    if (result.status === 'error') {
      expect(result.status_code).toBe(500);
    }
  });
});
