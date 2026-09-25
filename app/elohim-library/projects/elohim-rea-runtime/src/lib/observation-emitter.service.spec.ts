/**
 * ObservationEmitterService Tests
 *
 * The emitter witnesses a person's attention on one content node as an
 * agent-private `lamad:content-viewed` observation (ruling R-A3 of the
 * post-station-4 sprint): begin → scroll → end posts once to
 * POST /api/v1/observations with the dwell (paused while the tab is hidden)
 * and the deepest scroll reached. No thresholds live here — the lifestream
 * recipe's lens table holds them.
 *
 * Ruling R-A12: the witness is written AS the signed-in person. Both paths
 * carry the session bearer — the in-app post in its headers, the page-outliving
 * flush through `fetch(…, {keepalive: true, headers: {Authorization}})`. There
 * is no `sendBeacon` path: a beacon cannot carry a header, so its write reached
 * the doorway anonymous and the node refused it (401). With no bearer the
 * emitter posts nothing at all, silently: there is nobody to witness as.
 */

import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of, throwError } from 'rxjs';

import {
  CONTENT_VIEWED_KIND,
  OBSERVATION_BEARER,
  OBSERVATION_STORAGE_BASE_URL,
  ObservationEmitterService,
  scrollDepthPct,
} from './observation-emitter.service';

describe('ObservationEmitterService', () => {
  let service: ObservationEmitterService;
  let httpMock: { post: ReturnType<typeof vi.fn> };
  let visibility: DocumentVisibilityState;

  /** The session JWT the signed-in person's app holds. */
  const BEARER = 'jwt-jessica';
  let bearer: string | null;

  const ACK = {
    observerCid: 'human-jessica',
    observerCidNamespace: 'as-asserted',
    logCid: 'blake3:00',
    logOffset: 0,
    seq: 1,
    signed: 'absent',
  };

  function setVisibility(state: DocumentVisibilityState): void {
    visibility = state;
    document.dispatchEvent(new Event('visibilitychange'));
  }

  function postedBody(call = 0): Record<string, unknown> {
    return httpMock.post.mock.calls[call][1] as Record<string, unknown>;
  }

  function postedPayload(call = 0): Record<string, unknown> {
    return JSON.parse(postedBody(call)['payloadJson'] as string) as Record<string, unknown>;
  }

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-09-24T12:00:00Z'));
    visibility = 'visible';
    vi.spyOn(document, 'visibilityState', 'get').mockImplementation(() => visibility);

    httpMock = { post: vi.fn().mockReturnValue(of(ACK)) };
    bearer = BEARER;
    TestBed.configureTestingModule({
      providers: [
        ObservationEmitterService,
        { provide: HttpClient, useValue: httpMock },
        { provide: OBSERVATION_BEARER, useValue: () => bearer },
      ],
    });
    service = TestBed.inject(ObservationEmitterService);
  });

  afterEach(() => {
    service.ngOnDestroy();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('begin_then_end_posts_content_viewed_with_dwell_and_max_scroll', () => {
    service.begin('bafy-node-1');
    vi.advanceTimersByTime(3200);
    service.noteScroll(35);
    service.noteScroll(64);
    service.noteScroll(50);
    service.end('bafy-node-1');

    expect(httpMock.post).toHaveBeenCalledOnce();
    expect(httpMock.post.mock.calls[0][0]).toBe('/api/v1/observations');
    const body = postedBody();
    expect(body['observationKind']).toBe(CONTENT_VIEWED_KIND);
    expect(body['observationKind']).toBe('lamad:content-viewed');
    expect(body['subjectCid']).toBe('bafy-node-1');
    expect(typeof body['payloadJson']).toBe('string');
    expect(body).not.toHaveProperty('observerCid');
    expect(postedPayload()).toEqual({
      ref_cid: 'bafy-node-1',
      dwell_ms: 3200,
      scroll_depth_pct: 64,
    });
    // No session id: the app's session is a random UUID, not a CID.
    expect(postedPayload()).not.toHaveProperty('session_id');
  });

  it('dwell_pauses_while_document_hidden', () => {
    service.begin('bafy-node-1');
    vi.advanceTimersByTime(1000);
    setVisibility('hidden');
    vi.advanceTimersByTime(60_000);
    setVisibility('visible');
    vi.advanceTimersByTime(500);
    service.end('bafy-node-1');

    expect(postedPayload()['dwell_ms']).toBe(1500);
  });

  it('dwell_starts_paused_when_begun_while_hidden', () => {
    setVisibility('hidden');
    service.begin('bafy-node-1');
    vi.advanceTimersByTime(10_000);
    setVisibility('visible');
    vi.advanceTimersByTime(250);
    service.end('bafy-node-1');

    expect(postedPayload()['dwell_ms']).toBe(250);
  });

  it('scroll_depth_is_monotonic_max_and_clamped_0_100', () => {
    service.begin('bafy-node-1');
    service.noteScroll(40);
    service.noteScroll(20);
    service.noteScroll(Number.NaN);
    service.noteScroll(-5);
    service.end('bafy-node-1');
    expect(postedPayload(0)['scroll_depth_pct']).toBe(40);

    service.begin('bafy-node-2');
    service.noteScroll(150);
    service.end('bafy-node-2');
    expect(postedPayload(1)['scroll_depth_pct']).toBe(100);

    service.begin('bafy-node-3');
    service.noteScroll(33.6);
    service.end('bafy-node-3');
    expect(postedPayload(2)['scroll_depth_pct']).toBe(34);

    // Never scrolled: depth 0, never NaN.
    service.begin('bafy-node-4');
    service.end('bafy-node-4');
    expect(postedPayload(3)['scroll_depth_pct']).toBe(0);

    // Depth from the page geometry: a short page reports 100, never NaN.
    expect(scrollDepthPct(0, 800, 600)).toBe(100);
    expect(scrollDepthPct(0, 800, 800)).toBe(100);
    expect(scrollDepthPct(0, 0, 0)).toBe(100);
    expect(scrollDepthPct(0, 500, 2000)).toBe(25);
    expect(scrollDepthPct(1500, 500, 2000)).toBe(100);
    expect(scrollDepthPct(9999, 500, 2000)).toBe(100);
    expect(scrollDepthPct(Number.NaN, 500, 2000)).toBe(0);
  });

  it('end_without_begin_is_noop', () => {
    service.end('bafy-never-begun');
    expect(httpMock.post).not.toHaveBeenCalled();
  });

  it('post_failure_is_swallowed', () => {
    httpMock.post.mockReturnValue(throwError(() => new Error('503 Service Unavailable')));
    service.begin('bafy-node-1');
    expect(() => service.end('bafy-node-1')).not.toThrow();
    vi.runAllTimers();

    // The emitter keeps working after a failure.
    httpMock.post.mockReturnValue(of(ACK));
    service.begin('bafy-node-2');
    service.end('bafy-node-2');
    expect(httpMock.post).toHaveBeenCalledTimes(2);
  });

  it('end_is_idempotent_per_ref', () => {
    service.begin('bafy-node-1');
    service.begin('bafy-node-2');
    vi.advanceTimersByTime(100);
    service.end('bafy-node-1');
    service.end('bafy-node-1');
    expect(httpMock.post).toHaveBeenCalledOnce();

    // The other ref is still open and ends on its own.
    service.end('bafy-node-2');
    expect(httpMock.post).toHaveBeenCalledTimes(2);
    expect(postedBody(1)['subjectCid']).toBe('bafy-node-2');
  });

  it('a second begin for an open ref keeps the first start', () => {
    service.begin('bafy-node-1');
    vi.advanceTimersByTime(700);
    service.begin('bafy-node-1');
    vi.advanceTimersByTime(300);
    service.end('bafy-node-1');
    expect(postedPayload()['dwell_ms']).toBe(1000);
  });

  it('destroy drops open views without posting and stops listening', () => {
    service.begin('bafy-node-1');
    service.ngOnDestroy();
    service.end('bafy-node-1');
    setVisibility('hidden');
    expect(httpMock.post).not.toHaveBeenCalled();
  });

  // Ruling R-A11 (review W5): the emitter writes to the same storage base the
  // lifestream reads from, never a bare path on whatever origin served the page.
  it('emitter_posts_to_the_storage_base_not_the_serving_origin', () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [
        ObservationEmitterService,
        { provide: HttpClient, useValue: httpMock },
        { provide: OBSERVATION_BEARER, useValue: () => bearer },
        { provide: OBSERVATION_STORAGE_BASE_URL, useValue: () => 'http://localhost:8090' },
      ],
    });
    service.ngOnDestroy();
    service = TestBed.inject(ObservationEmitterService);

    service.begin('bafy-node-1');
    service.end('bafy-node-1');
    expect(httpMock.post).toHaveBeenCalledOnce();
    expect(httpMock.post.mock.calls[0][0]).toBe('http://localhost:8090/api/v1/observations');
  });

  // Ruling R-A12: the witness names the person. The in-app post carries the
  // session bearer; with no bearer there is nobody to witness as, so nothing is
  // posted (the node refuses an anonymous write, so it never witnessed anything).
  it('end_posts_with_session_bearer', () => {
    service.begin('bafy-node-1');
    vi.advanceTimersByTime(1200);
    service.end('bafy-node-1');

    expect(httpMock.post).toHaveBeenCalledOnce();
    const options = httpMock.post.mock.calls[0][2] as { headers: Record<string, string> };
    expect(options.headers['Authorization']).toBe(`Bearer ${BEARER}`);
  });

  it('end_without_bearer_posts_nothing', () => {
    bearer = null;
    service.begin('bafy-node-1');
    vi.advanceTimersByTime(1200);
    expect(() => service.end('bafy-node-1')).not.toThrow();
    expect(httpMock.post).not.toHaveBeenCalled();

    // The session arriving later does not resurrect the view that was left
    // without one; the next read is witnessed normally.
    bearer = BEARER;
    service.begin('bafy-node-2');
    service.end('bafy-node-2');
    expect(httpMock.post).toHaveBeenCalledOnce();
    expect(postedBody(0)['subjectCid']).toBe('bafy-node-2');
  });

  // Ruling R-A8, as R-A12 rules it: a hard leave (tab close, cross-bundle
  // navigation) still witnesses the open view, through a keepalive fetch that
  // outlives the page AND carries the bearer. `sendBeacon` cannot carry a
  // header, so it is gone.
  describe('hard leave', () => {
    let fetchMock: ReturnType<typeof vi.fn>;
    let beacon: ReturnType<typeof vi.fn>;
    let savedBeacon: PropertyDescriptor | undefined;

    beforeEach(() => {
      fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 201 }));
      vi.stubGlobal('fetch', fetchMock);
      savedBeacon = Object.getOwnPropertyDescriptor(navigator, 'sendBeacon');
      // A beacon that WOULD be accepted, so the assertion that it is never
      // called cannot pass merely because the environment lacks one.
      beacon = vi.fn().mockReturnValue(true);
      Object.defineProperty(navigator, 'sendBeacon', { value: beacon, configurable: true });
    });

    afterEach(() => {
      vi.unstubAllGlobals();
      if (savedBeacon) {
        Object.defineProperty(navigator, 'sendBeacon', savedBeacon);
      } else {
        delete (navigator as unknown as Record<string, unknown>)['sendBeacon'];
      }
    });

    it('pagehide_flush_uses_keepalive_fetch_with_bearer', () => {
      service.begin('bafy-node-1');
      vi.advanceTimersByTime(4000);
      service.noteScroll(40);
      window.dispatchEvent(new Event('pagehide'));

      expect(httpMock.post).not.toHaveBeenCalled();
      expect(fetchMock).toHaveBeenCalledOnce();
      const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
      expect(url).toBe('/api/v1/observations');
      expect(init.method).toBe('POST');
      expect(init.keepalive).toBe(true);
      const headers = init.headers as Record<string, string>;
      expect(headers['Content-Type']).toBe('application/json');
      expect(headers['Authorization']).toBe(`Bearer ${BEARER}`);
      const body = JSON.parse(init.body as string) as Record<string, unknown>;
      expect(body['observationKind']).toBe('lamad:content-viewed');
      expect(body['subjectCid']).toBe('bafy-node-1');
      expect(JSON.parse(body['payloadJson'] as string)).toEqual({
        ref_cid: 'bafy-node-1',
        dwell_ms: 4000,
        scroll_depth_pct: 40,
      });
    });

    it('pagehide_flush_without_bearer_sends_nothing', () => {
      bearer = null;
      service.begin('bafy-node-1');
      vi.advanceTimersByTime(4000);
      expect(() => window.dispatchEvent(new Event('pagehide'))).not.toThrow();

      expect(fetchMock).not.toHaveBeenCalled();
      expect(httpMock.post).not.toHaveBeenCalled();
    });

    it('sendBeacon_is_never_called', () => {
      service.begin('bafy-node-1');
      window.dispatchEvent(new Event('pagehide'));
      expect(beacon).not.toHaveBeenCalled();
      expect(fetchMock).toHaveBeenCalledOnce();

      // Nor on the visibility-hidden fallback path.
      service.begin('bafy-node-2');
      setVisibility('hidden');
      expect(beacon).not.toHaveBeenCalled();
    });

    it('soft_leave_after_flush_does_not_double_post', () => {
      service.begin('bafy-node-1');
      window.dispatchEvent(new Event('pagehide'));
      expect(fetchMock).toHaveBeenCalledOnce();

      // The in-app leave that follows (route change, destroy) posts nothing more.
      service.end('bafy-node-1');
      window.dispatchEvent(new Event('pagehide'));
      expect(httpMock.post).not.toHaveBeenCalled();
      expect(fetchMock).toHaveBeenCalledOnce();
    });

    it('hidden_tab_does_not_flush_where_pagehide_exists', () => {
      // pagehide is the hard-leave signal; hiding a tab only pauses dwell.
      service.begin('bafy-node-1');
      setVisibility('hidden');
      expect(fetchMock).not.toHaveBeenCalled();
      setVisibility('visible');
      service.end('bafy-node-1');
      expect(httpMock.post).toHaveBeenCalledOnce();
    });

    it('destroy_stops_listening_for_pagehide', () => {
      service.begin('bafy-node-1');
      service.ngOnDestroy();
      window.dispatchEvent(new Event('pagehide'));
      expect(fetchMock).not.toHaveBeenCalled();
    });
  });
});
