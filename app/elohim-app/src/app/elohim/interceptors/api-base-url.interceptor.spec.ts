import {
  HttpRequest,
  HttpHandlerFn,
  HttpEvent,
  HttpErrorResponse,
  HttpResponse,
} from '@angular/common/http';
import { NEVER, Observable, of, throwError } from 'rxjs';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import { apiBaseUrlInterceptor, resetDoorwayFailoverState } from './api-base-url.interceptor';
import { environment } from '../../../environments/environment';

vi.mock('../../../environments/environment', () => ({
  environment: {
    client: { doorwayUrl: 'https://doorway-alpha.elohim.host' },
    holochain: { storageUrl: 'http://localhost:8090' },
  },
}));

function captureUrl(): { handler: HttpHandlerFn; received: () => string | undefined } {
  let captured: string | undefined;
  const handler: HttpHandlerFn = (req): Observable<HttpEvent<unknown>> => {
    captured = req.url;
    return of({} as HttpEvent<unknown>);
  };
  return { handler, received: () => captured };
}

function setOrigin(origin: string): void {
  Object.defineProperty(globalThis, 'location', {
    value: { origin } as Location,
    configurable: true,
    writable: true,
  });
}

/** Simulates the SSR/elohim-render context: no browser `location` at all. */
function clearOrigin(): void {
  Object.defineProperty(globalThis, 'location', {
    value: undefined,
    configurable: true,
    writable: true,
  });
}

type ScriptedResult = 'network' | 'success' | { status: number };

/**
 * Handler that replays a scripted sequence of outcomes, one per call
 * (network failure = status 0, an arbitrary HTTP error status, or success),
 * recording the URL used on each call for failover assertions.
 */
function scriptedHandler(results: ScriptedResult[]): {
  handler: HttpHandlerFn;
  calls: () => string[];
} {
  const calls: string[] = [];
  let index = 0;
  const handler: HttpHandlerFn = (req): Observable<HttpEvent<unknown>> => {
    calls.push(req.url);
    const result = results[index++];
    if (result === 'network') {
      return throwError(() => new HttpErrorResponse({ status: 0, url: req.url }));
    }
    if (result && typeof result === 'object') {
      return throwError(() => new HttpErrorResponse({ status: result.status, url: req.url }));
    }
    // A real HttpResponse (not a bare object cast to HttpEvent) so the
    // interceptor's `event instanceof HttpResponse` sticky-write check
    // (FIX 3) actually exercises against these scripted successes.
    return of(new HttpResponse({ status: 200, url: req.url }) as HttpEvent<unknown>);
  };
  return { handler, calls: () => calls };
}

function setTauri(present: boolean): void {
  if (present) {
    (globalThis as unknown as { __TAURI__?: object }).__TAURI__ = {};
  } else {
    delete (globalThis as unknown as { __TAURI__?: object }).__TAURI__;
  }
}

describe('apiBaseUrlInterceptor', () => {
  const originalLocation = globalThis.location;

  afterEach(() => {
    Object.defineProperty(globalThis, 'location', {
      value: originalLocation,
      configurable: true,
      writable: true,
    });
    setTauri(false);
    resetDoorwayFailoverState();
  });

  describe('cross-origin SPA host (alpha/prod)', () => {
    beforeEach(() => setOrigin('https://alpha.elohim.host'));

    it('prefixes relative /api/v1/* with doorway URL', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('https://doorway-alpha.elohim.host/api/v1/mastery');
    });

    it('prefixes /db/, /blob/, /apps/, /health', () => {
      const cases = ['/db/content/123', '/blob/sha256-abc', '/apps/foo/index.html', '/health'];
      for (const path of cases) {
        const { handler, received } = captureUrl();
        apiBaseUrlInterceptor(new HttpRequest('GET', path), handler).subscribe();
        expect(received()).toBe(`https://doorway-alpha.elohim.host${path}`);
      }
    });

    it('leaves absolute URLs untouched', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(
        new HttpRequest('GET', 'https://other.example.com/api/v1/mastery'),
        handler
      ).subscribe();
      expect(received()).toBe('https://other.example.com/api/v1/mastery');
    });

    it('leaves non-doorway paths untouched', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/assets/icon.svg'), handler).subscribe();
      expect(received()).toBe('/assets/icon.svg');
    });
  });

  describe('Tauri desktop', () => {
    beforeEach(() => {
      setOrigin('https://tauri.localhost');
      setTauri(true);
    });

    it('prefixes /api/v1/* with storage sidecar URL (not doorway)', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('http://localhost:8090/api/v1/mastery');
    });

    it('prefixes /db/, /blob/ with storage sidecar URL', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/123'), handler).subscribe();
      expect(received()).toBe('http://localhost:8090/db/content/123');
    });

    it('does not use doorway URL even when set in environment', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/health'), handler).subscribe();
      expect(received()).not.toContain('doorway');
    });
  });

  describe('development workspace runtime (per-endpoint hostnames)', () => {
    // resolveBaseUrl() still returns '' for a workspace host (dodging the cross-origin
    // doorwayUrl), but effectivePrimary now falls back to the browser's own
    // origin so the failover ladder can engage — a same-origin absolute URL
    // behaves identically to a relative one (no CORS boundary crossed), so
    // this is a legitimate ladder-change adaptation of the prior "untouched"
    // expectation, not a functional regression.
    it('rewrites to its own origin (still same-origin, no doorwayUrl) on a .devspaces. workspace host', () => {
      setOrigin('https://workspace-angular-dev.devspaces.example.com');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('https://workspace-angular-dev.devspaces.example.com/api/v1/mastery');
    });

    it('rewrites to its own origin (still same-origin, no doorwayUrl) on a .code.ethosengine.com workspace host', () => {
      setOrigin('https://workspace-angular-dev.code.ethosengine.com');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('https://workspace-angular-dev.code.ethosengine.com/api/v1/mastery');
    });
  });

  describe('local dev server', () => {
    // Same rationale as Che above: effectivePrimary falls back to the
    // browser's own origin, so the request becomes an absolute same-origin
    // URL instead of relative — still routed through the dev-server proxy,
    // still zero CORS exposure.
    it('rewrites to its own origin on localhost', () => {
      setOrigin('http://localhost:4200');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/api/v1/mastery'), handler).subscribe();
      expect(received()).toBe('http://localhost:4200/api/v1/mastery');
    });

    it('rewrites to its own origin on 127.0.0.1', () => {
      setOrigin('http://127.0.0.1:4200');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/api/v1/mastery'), handler).subscribe();
      expect(received()).toBe('http://127.0.0.1:4200/api/v1/mastery');
    });
  });

  describe('same-origin as doorway', () => {
    // This is FIX 1's exact target: the canonical served-from-doorway
    // topology. resolveBaseUrl() still returns '' (no separate doorway
    // origin to route to), but effectivePrimary now falls back to the
    // browser's own (== doorway) origin, so the ladder engages and can fail
    // over — see the "multi-host failover" describe block below for the
    // case with doorwayFallbacks configured.
    it('rewrites to its own (doorway) origin when no fallbacks are configured', () => {
      setOrigin('https://doorway-alpha.elohim.host');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/api/v1/mastery'), handler).subscribe();
      expect(received()).toBe('https://doorway-alpha.elohim.host/api/v1/mastery');
    });
  });

  describe('multi-host failover (§3a — logical anycast)', () => {
    const PRIMARY = 'https://doorway-alpha.elohim.host';
    const FALLBACK = 'https://elohim.host';

    beforeEach(() => {
      setOrigin('https://alpha.elohim.host');
      resetDoorwayFailoverState();
      environment.client!.doorwayFallbacks = [FALLBACK];
    });

    afterEach(() => {
      delete environment.client!.doorwayFallbacks;
      resetDoorwayFailoverState();
    });

    it('rewrite + success is a single call against the primary', () => {
      const { handler, calls } = scriptedHandler(['success']);
      let completed = false;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), handler).subscribe(
        () => (completed = true)
      );
      expect(calls()).toEqual([`${PRIMARY}/db/content/1`]);
      expect(completed).toBe(true);
    });

    it('GET with status-0 on the primary retries the fallback and returns its success', () => {
      const { handler, calls } = scriptedHandler(['network', 'success']);
      let completed = false;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), handler).subscribe(
        () => (completed = true)
      );
      expect(calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);
      expect(completed).toBe(true);
    });

    it('GET with status-0 on primary AND fallback propagates the error', () => {
      const { handler, calls } = scriptedHandler(['network', 'network']);
      let error: unknown;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), handler).subscribe({
        error: e => (error = e),
      });
      expect(calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);
      expect((error as HttpErrorResponse).status).toBe(0);
    });

    it('POST with status-0 is not retried, but a following GET uses the fallback (sticky)', () => {
      const { handler: postHandler, calls: postCalls } = scriptedHandler(['network']);
      let postError: unknown;
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), postHandler).subscribe({
        error: e => (postError = e),
      });
      expect(postCalls()).toEqual([`${PRIMARY}/api/v1/mastery`]);
      expect((postError as HttpErrorResponse).status).toBe(0);

      const { handler: getHandler, calls: getCalls } = scriptedHandler(['success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), getHandler).subscribe();
      expect(getCalls()).toEqual([`${FALLBACK}/db/content/1`]);
    });

    it('HTTP 500 from the primary does not trigger failover', () => {
      const { handler, calls } = scriptedHandler([{ status: 500 }]);
      let error: unknown;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), handler).subscribe({
        error: e => (error = e),
      });
      expect(calls()).toEqual([`${PRIMARY}/db/content/1`]);
      expect((error as HttpErrorResponse).status).toBe(500);
    });

    it('leaves non-doorway-prefix URLs untouched by failover logic', () => {
      const { handler, calls } = scriptedHandler(['network']);
      let error: unknown;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/assets/icon.svg'), handler).subscribe({
        error: e => (error = e),
      });
      // Untouched means no base-url rewrite happened at all — the request
      // still reaches the handler once with its original relative URL, and
      // the interceptor's failover machinery never engages (no retry).
      expect(calls()).toEqual(['/assets/icon.svg']);
      expect((error as HttpErrorResponse).status).toBe(0);
    });

    it('with no fallbacks configured, behaves like today (single attempt, error propagates)', () => {
      environment.client!.doorwayFallbacks = undefined;
      const { handler, calls } = scriptedHandler(['network']);
      let error: unknown;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), handler).subscribe({
        error: e => (error = e),
      });
      expect(calls()).toEqual([`${PRIMARY}/db/content/1`]);
      expect((error as HttpErrorResponse).status).toBe(0);
    });

    // --- FIX 1/2 regression: the previously-inert canonical topology ---

    it('(a) same-origin-as-doorway topology fails over to the fallback on status-0 (the exact bug FIX 1 closes)', () => {
      setOrigin(PRIMARY);
      const { handler, calls } = scriptedHandler(['network', 'success']);
      let completed = false;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), handler).subscribe(
        () => (completed = true)
      );
      expect(calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);
      expect(completed).toBe(true);
    });

    it('(b) sticky returns to the primary once the fallback itself fails', () => {
      // Round 1: primary down, fallback answers — sticky pins to fallback.
      const round1 = scriptedHandler(['network', 'success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), round1.handler).subscribe();
      expect(round1.calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);

      // Round 2: candidate ladder now starts at the sticky fallback; it
      // fails too, so the ladder walks back to the primary (which stays a
      // candidate per FIX 2) and succeeds — sticky returns to the primary.
      const round2 = scriptedHandler(['network', 'success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/2'), round2.handler).subscribe();
      expect(round2.calls()).toEqual([`${FALLBACK}/db/content/2`, `${PRIMARY}/db/content/2`]);

      // Round 3: proves stickiness actually returned to the primary — a
      // single successful call, no fallback hop first.
      const round3 = scriptedHandler(['success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/3'), round3.handler).subscribe();
      expect(round3.calls()).toEqual([`${PRIMARY}/db/content/3`]);
    });

    it('(d) FIX 2: a healthy fallback does NOT get demoted before the reprobe TTL elapses', () => {
      const round1 = scriptedHandler(['network', 'success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), round1.handler).subscribe();
      expect(round1.calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);

      // Well within the reprobe TTL — sticky fallback is tried alone, no
      // speculative primary hop.
      const round2 = scriptedHandler(['success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/2'), round2.handler).subscribe();
      expect(round2.calls()).toEqual([`${FALLBACK}/db/content/2`]);
    });

    it('(e) FIX 2: primary-recovery demotion — a GET re-probes the primary once the TTL elapses, even though the fallback keeps answering', () => {
      const nowSpy = vi.spyOn(Date, 'now');
      let clock = 1_000_000;
      nowSpy.mockImplementation(() => clock);

      // Round 1: primary down, fallback answers — sticky pins to fallback at `clock`.
      const round1 = scriptedHandler(['network', 'success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), round1.handler).subscribe();
      expect(round1.calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);

      // Advance past the reprobe TTL. Without FIX 2 this call would go
      // straight to the fallback forever — the exact bug this closes: a
      // one-time blip permanently strands the session on the fallback with
      // no guaranteed content parity.
      clock += 30_000;
      const round2 = scriptedHandler(['success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/2'), round2.handler).subscribe();
      expect(round2.calls()).toEqual([`${PRIMARY}/db/content/2`]);

      // Round 3 proves the demotion stuck: sticky is back on the primary, no
      // fallback hop needed.
      const round3 = scriptedHandler(['success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/3'), round3.handler).subscribe();
      expect(round3.calls()).toEqual([`${PRIMARY}/db/content/3`]);

      nowSpy.mockRestore();
    });

    it('(f) FIX 2: reprobe never applies to a non-retriable write — a POST always trusts the current sticky preference', () => {
      const nowSpy = vi.spyOn(Date, 'now');
      let clock = 1_000_000;
      nowSpy.mockImplementation(() => clock);

      // Round 1: primary down, fallback answers a GET — sticky pins to fallback.
      const round1 = scriptedHandler(['network', 'success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), round1.handler).subscribe();
      expect(round1.calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);

      // Well past the reprobe TTL, but this is a POST: it must not gamble a
      // non-idempotent write on an unverified (possibly still-down) primary.
      clock += 60_000;
      const postHandler = scriptedHandler(['success']);
      apiBaseUrlInterceptor(
        new HttpRequest('POST', '/api/v1/mastery', {}),
        postHandler.handler
      ).subscribe();
      expect(postHandler.calls()).toEqual([`${FALLBACK}/api/v1/mastery`]);

      nowSpy.mockRestore();
    });

    it('(g) FIX 3: a black-holed (never-responding) primary still fails over via the per-attempt timeout', async () => {
      vi.useFakeTimers();
      try {
        const calls: string[] = [];
        let attempts = 0;
        const handler: HttpHandlerFn = (r): Observable<HttpEvent<unknown>> => {
          calls.push(r.url);
          attempts++;
          if (attempts === 1) {
            // Black hole: never emits a next, error, or complete.
            return NEVER;
          }
          return of(new HttpResponse({ status: 200, url: r.url }) as HttpEvent<unknown>);
        };

        let completed = false;
        apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), handler).subscribe(
          () => (completed = true)
        );

        // Primary is black-holed — no status-0 ever arrives, so nothing
        // happens yet without a per-attempt timeout.
        expect(calls).toEqual([`${PRIMARY}/db/content/1`]);
        expect(completed).toBe(false);

        // Advance past the per-attempt timeout (8000ms, matched to
        // ElohimClient's DEFAULT_ATTEMPT_TIMEOUT_MS) — the timed-out attempt
        // is treated as a network failure and fails over to the fallback.
        await vi.advanceTimersByTimeAsync(8100);

        expect(calls).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);
        expect(completed).toBe(true);
      } finally {
        vi.useRealTimers();
      }
    });

    it('(h) FIX 3: a non-retriable POST never gets a per-attempt timeout applied', async () => {
      vi.useFakeTimers();
      try {
        const calls: string[] = [];
        const handler: HttpHandlerFn = (r): Observable<HttpEvent<unknown>> => {
          calls.push(r.url);
          return NEVER; // Would time out if a timeout were (wrongly) applied.
        };

        let settled = false;
        apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe({
          next: () => (settled = true),
          error: () => (settled = true),
        });

        // Advance well past the per-attempt timeout window — a write must
        // never be aborted by it; the request should still be hanging (not
        // settled, not retried against another host).
        await vi.advanceTimersByTimeAsync(60_000);

        expect(calls).toEqual([`${PRIMARY}/api/v1/mastery`]);
        expect(settled).toBe(false);
      } finally {
        vi.useRealTimers();
      }
    });

    it('(c) SSR guard: with no browser location, passes through untouched and never reads/writes sticky state', () => {
      // Warm up a sticky preference on the fallback via a real browser call.
      const warm = scriptedHandler(['network', 'success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/1'), warm.handler).subscribe();
      expect(warm.calls()).toEqual([`${PRIMARY}/db/content/1`, `${FALLBACK}/db/content/1`]);

      // Simulate SSR/elohim-render: no `location` at all. The interceptor's
      // SSR guard is gated on `globalThis.location?.origin` being absent —
      // this is the code path that exercises it, since removing `location`
      // cleanly in vitest/jsdom is the only feasible way to simulate the
      // server context from a browser-shaped test env.
      clearOrigin();
      const ssr = scriptedHandler(['network']);
      let error: unknown;
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/2'), ssr.handler).subscribe({
        error: e => (error = e),
      });
      // Passed straight through: exactly one call, original relative URL —
      // no rewrite, no failover retry attempted.
      expect(ssr.calls()).toEqual(['/db/content/2']);
      expect((error as HttpErrorResponse).status).toBe(0);

      // Restore the browser origin and prove sticky state was left
      // untouched by the SSR call: a fresh request goes straight to the
      // fallback the warm-up pinned, with no extra primary hop in between
      // (which would appear if the SSR call had reset preferredBase).
      setOrigin('https://alpha.elohim.host');
      const verify = scriptedHandler(['success']);
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/3'), verify.handler).subscribe();
      expect(verify.calls()).toEqual([`${FALLBACK}/db/content/3`]);
    });
  });
});
