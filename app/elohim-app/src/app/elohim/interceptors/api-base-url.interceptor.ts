import {
  HttpErrorResponse,
  HttpEvent,
  HttpInterceptorFn,
  HttpRequest,
  HttpResponse,
} from '@angular/common/http';

import { catchError, tap } from 'rxjs/operators';

import { Observable, throwError } from 'rxjs';

import { environment } from '../../../environments/environment';

const DOORWAY_PATH_PREFIXES = ['/api/', '/db/', '/blob/', '/apps/', '/health'];

// Methods safe to silently re-issue against a fallback host. Non-idempotent
// methods (POST/PUT/PATCH/DELETE) are never auto-retried — a network failure
// after the request left the client can't distinguish "never arrived" from
// "arrived, processed, response lost."
const RETRIABLE_METHODS = new Set(['GET', 'HEAD']);

function isAbsolute(url: string): boolean {
  return url.startsWith('http://') || url.startsWith('https://');
}

function matchesDoorwayPath(url: string): boolean {
  return DOORWAY_PATH_PREFIXES.some(p => url === p.replace(/\/$/, '') || url.startsWith(p));
}

function isTauri(): boolean {
  return typeof globalThis !== 'undefined' && '__TAURI__' in globalThis;
}

function isCheOrigin(origin: string): boolean {
  return origin.includes('.devspaces.') || origin.includes('.code.ethosengine.com');
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
 * caller should fall back to the browser's own origin instead — Che/local-dev
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
  if (isCheOrigin(origin) || isLocalDevOrigin(origin)) return '';

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

/** Test-only: reset sticky failover state between specs. */
export function resetDoorwayFailoverState(): void {
  preferredBase = null;
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
 */
function buildCandidates(effectivePrimary: string): string[] {
  const fallbacks = (environment.client?.doorwayFallbacks ?? []).map(normalizeHost);
  const raw = [
    ...(preferredBase ? [normalizeHost(preferredBase)] : []),
    normalizeHost(effectivePrimary),
    ...fallbacks,
  ];
  return dedupe(raw);
}

function isNetworkFailure(err: unknown): boolean {
  return err instanceof HttpErrorResponse && err.status === 0;
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
  const candidates = buildCandidates(effectivePrimary);
  const isRetriable = RETRIABLE_METHODS.has(req.method.toUpperCase());

  const attempt = (currentBase: string, remaining: string[]): Observable<HttpEvent<unknown>> =>
    next(rewriteTo(req, currentBase)).pipe(
      tap(event => {
        // Only a genuine HttpResponse confirms the host is reachable —
        // HttpSentEvent fires at dispatch, before any network confirmation.
        if (event instanceof HttpResponse) {
          preferredBase = currentBase;
        }
      }),
      catchError((err: unknown) => {
        if (!isNetworkFailure(err)) {
          // The host answered (even with an error status) — it's reachable,
          // never fail over on a non-zero status.
          preferredBase = currentBase;
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
          preferredBase = nextBase;
          return throwError(() => err);
        }

        return attempt(nextBase, rest);
      })
    );

  const [first, ...rest] = candidates;
  return attempt(first, rest);
};
