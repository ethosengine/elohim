import {
  HttpErrorResponse,
  HttpEvent,
  HttpInterceptorFn,
  HttpRequest,
  HttpResponse,
} from '@angular/common/http';
import { inject } from '@angular/core';

import { catchError, tap, timeout } from 'rxjs/operators';

import { from, Observable, switchMap, TimeoutError, throwError } from 'rxjs';

import {
  ConfiguredDoorwayResolver,
  DOORWAY_ADDRESS_RESOLVER,
  DoorwayAddressResolver,
  DoorwayResolution,
  gatewayCandidates,
} from '@elohim/service';

import { isWorkspaceOrigin } from '@workspace/runtime';

import { environment } from '../../../environments/environment';

const DOORWAY_PATH_PREFIXES = ['/api/', '/db/', '/blob/', '/apps/', '/health'];

// Methods safe to silently re-issue against a fallback host. Non-idempotent
// methods (POST/PUT/PATCH/DELETE) are never auto-retried — a network failure
// after the request left the client can't distinguish "never arrived" from
// "arrived, processed, response lost."
const RETRIABLE_METHODS = new Set(['GET', 'HEAD']);

// Per-attempt timeout so a black-holing (packet-dropping) host that never
// produces a status-0 response still triggers failover — consistent with
// ElohimClient's per-attempt default (elohim-client.ts DEFAULT_ATTEMPT_TIMEOUT_MS).
// Applied ONLY to retriable (GET/HEAD) attempts: a non-retriable write is
// tried exactly once and must be allowed to run to completion — injecting a
// timeout here risks aborting an in-flight, non-idempotent write that would
// otherwise have succeeded, with no safe retry available (same reasoning as
// the ElohimClient SDK's non-retriable-write timeout exemption).
const PER_ATTEMPT_TIMEOUT_MS = 8000;

// How long a sticky preference for a fallback host is trusted before a
// retriable (GET/HEAD) request is used to re-probe the primary. Without
// this, a transient primary blip permanently strands the session on the
// fallback even after the primary recovers, because a healthy fallback is
// always tried first and a success never gives the primary another chance.
// Only read traffic re-probes — see `buildCandidates`'s `allowReprobe` — a
// write always trusts the current sticky preference rather than gambling a
// non-idempotent request on an unverified host.
const PRIMARY_REPROBE_INTERVAL_MS = 30000;

function isAbsolute(url: string): boolean {
  return url.startsWith('http://') || url.startsWith('https://');
}

function matchesDoorwayPath(url: string): boolean {
  return DOORWAY_PATH_PREFIXES.some(p => url === p.replace(/\/$/, '') || url.startsWith(p));
}

function isTauri(): boolean {
  return typeof globalThis !== 'undefined' && '__TAURI__' in globalThis;
}

function isLocalDevOrigin(origin: string): boolean {
  return origin.startsWith('http://localhost:') || origin.startsWith('http://127.0.0.1:');
}

function safeOrigin(url: string): string | null {
  try {
    return new URL(url).origin;
  } catch {
    return null;
  }
}

/**
 * Resolves the "preferred" absolute base for non-browser-origin cases
 * (Tauri sidecar) or the cross-origin doorway host. Returns '' whenever the
 * caller should fall back to the browser's own origin instead — workspace/local-dev
 * (dev-proxy same-origin routing, avoids CORS-header stripping) and the
 * same-origin-as-doorway topology (there is no separate doorway origin to
 * route to). '' is not "no base" here — see effectivePrimary in the
 * interceptor, which ORs this against the live browser origin.
 */
function resolveBaseUrl(): string {
  if (isTauri()) {
    return environment.client?.storageUrl ?? environment.holochain?.storageUrl ?? '';
  }

  // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof-equivalent guard, optional chaining short-circuits to undefined when globalThis.location is absent server-side, falling back to '' via ?? ''
  const origin = globalThis.location?.origin ?? '';
  if (!origin) return '';
  if (isWorkspaceOrigin(origin) || isLocalDevOrigin(origin)) return '';

  const doorwayUrl = environment.client?.doorwayUrl ?? '';
  if (!doorwayUrl) return '';

  if (origin === safeOrigin(doorwayUrl)) return '';

  return doorwayUrl;
}

/**
 * Sticky multi-host failover preference ("logical anycast" — dual-WAN
 * utility-plane failover design §3a). Null means "use effectivePrimary's
 * resolved default." Set whenever a host proves reachable (a genuine
 * HttpResponse event, even 4xx/5xx, or a successful retry) so subsequent
 * requests prefer it; also advanced (unverified) on a write-request network
 * failure so later reads route around a host that just dropped a write.
 * Session-scoped module state — cleared only via resetDoorwayFailoverState().
 * Never read or written outside a browser context (see the SSR guard in the
 * interceptor) — off-browser this state must stay untouched.
 */
let preferredBase: string | null = null;

/**
 * Timestamp (`Date.now()`) of the last time `preferredBase` was set (to a
 * fallback OR reconfirmed reachable). Drives the primary-recovery re-probe
 * in `buildCandidates` — see `PRIMARY_REPROBE_INTERVAL_MS`.
 */
let preferredSetAt = 0;

/** Test-only: reset sticky failover state between specs. */
export function resetDoorwayFailoverState(): void {
  preferredBase = null;
  preferredSetAt = 0;
}

/** Records a newly-confirmed (or newly-assumed) sticky preference with its timestamp. */
function setPreferredBase(base: string | null): void {
  preferredBase = base;
  preferredSetAt = Date.now();
}

function normalizeHost(url: string): string {
  return url.replace(/\/$/, '');
}

function dedupe(items: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const item of items) {
    if (!seen.has(item)) {
      seen.add(item);
      result.push(item);
    }
  }
  return result;
}

/**
 * Full failover candidate ladder: sticky preference first (if any), then the
 * resolved primary, then the configured fallbacks — deduped after
 * normalization so the primary is never dropped just because it's also
 * sticky, and duplicate/repeated fallback entries collapse to one hop.
 *
 * `allowReprobe` (true only for retriable GET/HEAD requests) re-orders the
 * ladder to try the PRIMARY first once the sticky fallback preference is
 * older than `PRIMARY_REPROBE_INTERVAL_MS` — otherwise a healthy fallback is
 * always tried first and a recovered primary never gets a chance to answer
 * again. The fallback stays in the ladder as the very next candidate, so a
 * still-down primary fails over exactly as before. Write-shaped calls never
 * reorder: they always trust whichever host is currently sticky rather than
 * gambling a non-idempotent request on an unverified host.
 */
function buildCandidates(resolvedCandidates: string[], allowReprobe: boolean): string[] {
  const [resolvedPrimary, ...resolvedFallbacks] = resolvedCandidates;
  const primary = normalizeHost(resolvedPrimary);
  const fallbacks = resolvedFallbacks.map(normalizeHost);
  const sticky = preferredBase ? normalizeHost(preferredBase) : null;

  const reprobeDue =
    allowReprobe &&
    sticky !== null &&
    sticky !== primary &&
    Date.now() - preferredSetAt >= PRIMARY_REPROBE_INTERVAL_MS;

  const raw = reprobeDue
    ? [primary, sticky, ...fallbacks]
    : [...(sticky ? [sticky] : []), primary, ...fallbacks];

  return dedupe(raw);
}

function isNetworkFailure(err: unknown): boolean {
  if (err instanceof HttpErrorResponse && err.status === 0) return true;
  // A per-attempt timeout (FIX 3) means the host black-holed the request —
  // no status ever arrived, so it must be treated the same as a network
  // failure (status 0) for failover purposes.
  return err instanceof TimeoutError;
}

function rewriteTo(req: HttpRequest<unknown>, base: string): HttpRequest<unknown> {
  return req.clone({ url: `${normalizeHost(base)}${req.url}` });
}

export const apiBaseUrlInterceptor: HttpInterceptorFn = (req, next) => {
  if (isAbsolute(req.url) || !matchesDoorwayPath(req.url)) {
    return next(req);
  }

  // SSR-safety: with no browser location (SSR/elohim-render context), pass
  // the request through untouched and never read or write the module-level
  // sticky state — that state must stay inert off-browser.
  // eslint-disable-next-line no-restricted-syntax -- SSR guard: optional chaining short-circuits to undefined server-side; the explicit !origin check below is the guard itself.
  const origin = globalThis.location?.origin;
  if (!origin) {
    return next(req);
  }

  const effectivePrimary = resolveBaseUrl() || origin;
  const isRetriable = RETRIABLE_METHODS.has(req.method.toUpperCase());
  const identity =
    environment.client?.doorwayIdentity ?? environment.client?.doorwayUrl ?? effectivePrimary;
  let resolver: DoorwayAddressResolver | null = null;
  if (!isTauri()) {
    try {
      // Direct unit calls run outside an Angular injection context; config is
      // the intentional fallback adapter in that case.
      resolver = inject(DOORWAY_ADDRESS_RESOLVER, { optional: true });
    } catch {
      resolver = null;
    }
  }
  resolver ??= new ConfiguredDoorwayResolver([
    {
      identity,
      primaryUrl: effectivePrimary,
      fallbackUrls: environment.client?.doorwayFallbacks,
    },
  ]);

  const dispatch = (resolution: DoorwayResolution): Observable<HttpEvent<unknown>> => {
    const resolvedCandidates = gatewayCandidates(resolution);
    if (resolvedCandidates.length === 0) {
      return throwError(
        () => new Error(`Doorway identity "${identity}" resolved without gateway endpoints`)
      );
    }
    const candidates = buildCandidates(resolvedCandidates, isRetriable);

    const attempt = (currentBase: string, remaining: string[]): Observable<HttpEvent<unknown>> => {
      // Per-attempt timeout (FIX 3) — ONLY for retriable GET/HEAD requests, so
      // a black-holing host still fails over even though it never produces a
      // status-0 response. A non-retriable write is tried exactly once and
      // must run to completion; injecting a timeout there could abort an
      // in-flight, non-idempotent write that would otherwise have succeeded.
      const response$ = next(rewriteTo(req, currentBase));
      const withTimeout$ = isRetriable
        ? response$.pipe(timeout(PER_ATTEMPT_TIMEOUT_MS))
        : response$;

      return withTimeout$.pipe(
        tap(event => {
          // Only a genuine HttpResponse confirms the host is reachable —
          // HttpSentEvent fires at dispatch, before any network confirmation.
          if (event instanceof HttpResponse) {
            setPreferredBase(currentBase);
          }
        }),
        catchError((err: unknown) => {
          if (!isNetworkFailure(err)) {
            // The host answered (even with an error status) — it's reachable,
            // never fail over on a non-zero status.
            setPreferredBase(currentBase);
            return throwError(() => err);
          }

          const [nextBase, ...rest] = remaining;
          if (!nextBase) {
            return throwError(() => err);
          }

          if (!isRetriable) {
            // Duplicate-write risk: don't re-issue. The failure already proves
            // currentBase is down, so steer subsequent requests to the
            // fallback without verifying it.
            setPreferredBase(nextBase);
            return throwError(() => err);
          }

          return attempt(nextBase, rest);
        })
      );
    };

    const [first, ...rest] = candidates;
    return attempt(first, rest);
  };

  try {
    const resolution = resolver.resolve(identity);
    return resolution instanceof Promise
      ? from(resolution).pipe(switchMap(dispatch))
      : dispatch(resolution);
  } catch (error) {
    return throwError(() => error);
  }
};
