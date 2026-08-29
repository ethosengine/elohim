/**
 * Reading `GET /.well-known/elohim-auth` — the first production reader.
 *
 * The document exists so an app carries no auth configuration at all: the one
 * fact a page always knows is the origin it was served from, and this turns
 * that single fact into every path a client needs. The doorway has served it
 * since 2026-08-28 (`doorway-service/src/routes/auth_discovery.rs`), and until
 * this file its only reader anywhere was an a2o test step — infrastructure
 * waiting for a consumer.
 *
 * # Trusting it, and the exact limits of that trust
 *
 * The document is unsigned. Its safety comes from SHAPE, not signature: every
 * value is an origin-relative path, so a foreign origin is unexpressible rather
 * than merely rejected, and the client resolves what it reads against the
 * origin it already trusted enough to load from. That property is enforced on
 * the server (`auth_discovery.rs`, with its own detector control) and in the
 * view schema (`relativePath`'s `^/(?!/).*$`, which rejects protocol-relative
 * `//host/x`).
 *
 * It is re-enforced HERE anyway, because a client that trusts a server-side
 * invariant it did not check is trusting the network path too. An attacker who
 * can rewrite this response — MITM, stale DNS, a compromised edge — otherwise
 * gets to aim a Login button.
 *
 * # Why shape-checking is not enough on its own
 *
 * A relative path can still be hostile. `/apps/{epr_id}/{sub_path}` is a
 * service prefix that proxies to storage's bundle-serving surface
 * (`server/http.rs`, `is_service_path` lists `/apps/`) — third-party EPR bundle
 * content. A document advertising `login: "/apps/evil/login.html"` is
 * origin-relative, passes the schema, and points a password form at content an
 * attacker uploaded. So every advertised location must ALSO sit under a prefix
 * the auth layer owns.
 *
 * # What this deliberately does not do
 *
 * It does not become the source of the paths a client calls.
 * `DoorwaySessionClientOptions` accepts only `baseUrl`, `fetchImpl` and
 * `tokenStore` — there is no path map to override — so `DoorwaySessionClient`
 * keeps its hardcoded `/auth/login`, `/auth/me` and the rest. Claiming
 * "every path is discovered" while composing that client would be false. The
 * document is a CHECK today: `pathDrift` reports where the two disagree, which
 * is exactly the signal that says when it is safe to make it the source.
 */

import { normalizeDoorwayUrl } from '../client/doorway-address-resolver.js';

import { docRejected, answerFromStatus, present, unreachableFromError } from './answer.js';

import type { Answer } from './answer.js';
import type { AuthDiscovery } from '../generated/auth-discovery.js';

export const DISCOVERY_PATH = '/.well-known/elohim-auth';

/**
 * Prefixes the auth layer owns.
 *
 * `/auth/` is `AUTH_OWNED_PATHS` (the doorway asserts at `cargo test` that every
 * path this document advertises is one it owns), and `/threshold/` is where the
 * doorway forwards the sign-in portal. Anything else is refused here — not
 * because it is necessarily hostile, but because a client cannot tell the
 * difference, and the cost of being wrong is a credential.
 */
const OWNED_PREFIXES = ['/auth/', '/threshold/'] as const;

/** Is this an origin-relative path that cannot name another origin? */
export function isOriginRelative(value: string): boolean {
  // `//host/x` is protocol-relative: it names ANOTHER origin while passing a
  // naive leading-slash check. This is the bypass the server-side walker names
  // and the schema pattern excludes.
  return value.startsWith('/') && !value.startsWith('//');
}

function isOwned(value: string): boolean {
  return OWNED_PREFIXES.some(prefix => value.startsWith(prefix));
}

/** Every location-shaped value in the document, as `field = value` pairs. */
function locations(doc: unknown): { field: string; value: string }[] {
  const out: { field: string; value: string }[] = [];
  const walk = (node: unknown, path: string): void => {
    if (typeof node === 'string') {
      // `doorwayId` is a name, not a location — only location-SHAPED values are
      // judged, exactly as the server-side walker does it.
      if (node.startsWith('/') || node.includes('://')) out.push({ field: path, value: node });
    } else if (node && typeof node === 'object') {
      for (const [key, sub] of Object.entries(node as Record<string, unknown>)) {
        walk(sub, `${path}.${key}`);
      }
    }
  };
  walk(doc, '$');
  return out;
}

/**
 * Reject a document that names anywhere it should not.
 *
 * Returns the offending fields; empty means the document is safe to follow.
 */
export function rejectionsIn(doc: unknown): string[] {
  const found = locations(doc);
  const bad: string[] = [];
  for (const { field, value } of found) {
    if (!isOriginRelative(value)) {
      bad.push(`${field}=${value} (escapes this origin)`);
    } else if (!isOwned(value)) {
      bad.push(`${field}=${value} (outside the auth layer's own prefixes)`);
    }
  }
  return bad;
}

function looksLikeDiscovery(doc: unknown): doc is AuthDiscovery {
  if (!doc || typeof doc !== 'object') return false;
  const d = doc as Partial<AuthDiscovery>;
  return typeof d.version === 'number' && typeof d.portal === 'string' && !!d.endpoints;
}

export interface ReadDiscoveryOptions {
  fetchImpl?: typeof fetch;
  timeoutMs?: number;
}

const DEFAULT_TIMEOUT_MS = 5000;

/**
 * Fetch and validate the discovery document for an origin.
 *
 * Never throws. A 404 is `absent` — the doorway answered our question and said
 * it has no document. Everything else that fails is `unreachable` with a reason
 * naming which failure it was, so "this doorway has no document" is never
 * confused with "we could not ask".
 */
export async function readAuthDiscovery(
  origin: string,
  options: ReadDiscoveryOptions = {}
): Promise<Answer<AuthDiscovery>> {
  const doFetch = options.fetchImpl ?? globalThis.fetch?.bind(globalThis);
  if (!doFetch) return docRejected('no fetch implementation available in this runtime');

  const base = normalizeDoorwayUrl(origin);
  const controller = new AbortController();
  const timer = setTimeout(() => {
    controller.abort();
  }, options.timeoutMs ?? DEFAULT_TIMEOUT_MS);

  try {
    const response = await doFetch(`${base}${DISCOVERY_PATH}`, {
      method: 'GET',
      signal: controller.signal,
    });
    if (!response.ok) {
      return answerFromStatus(response.status, `${base}${DISCOVERY_PATH}`);
    }

    let body: unknown;
    try {
      body = await response.json();
    } catch {
      // 200 with a non-JSON body is the app-shell-answering shape this
      // document lives under /.well-known/ specifically to avoid. Worth naming
      // distinctly, because it means a route stopped being owned.
      return docRejected(
        'answered 200 with a body that is not JSON (an app shell, not a document)'
      );
    }

    if (!looksLikeDiscovery(body)) {
      return docRejected('answered a JSON body that is not an auth discovery document');
    }

    const rejections = rejectionsIn(body);
    if (rejections.length > 0) {
      return docRejected(
        `document names locations a client must not follow: ${rejections.join('; ')}`
      );
    }

    return present(body);
  } catch (error) {
    return unreachableFromError(error);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Where to send a human to sign in, as an absolute URL on the origin that
 * served the document.
 *
 * Replaces an inlined `/threshold/login` literal. It does NOT replace an app's
 * own in-app login route — that is application routing and belongs to the app.
 */
// eslint-disable-next-line sonarjs/function-return-type -- intentional `T | null` API; rule misfires on nullable unions in this toolchain
export function portalUrl(
  discovery: AuthDiscovery,
  origin: string,
  returnUrl?: string
): string | null {
  if (!isOriginRelative(discovery.portal) || !isOwned(discovery.portal)) return null;
  const base = normalizeDoorwayUrl(origin);
  const url = `${base}${discovery.portal}`;
  if (!returnUrl) return url;
  return `${url}?returnUrl=${encodeURIComponent(returnUrl)}`;
}

/** One place the document and the client disagree about a path. */
export interface PathDrift {
  endpoint: string;
  advertised: string;
  clientUses: string;
}

/**
 * The paths `DoorwaySessionClient` hardcodes today, as it hardcodes them.
 *
 * This is a mirror, and normally that would be the anti-pattern — but its
 * purpose is precisely to DETECT divergence rather than to be authoritative,
 * and a drift report that cannot go stale would be reporting nothing. When the
 * session client grows a path map, this constant is deleted and the document
 * becomes the source.
 */
export const CLIENT_AUTH_PATHS: Readonly<Record<string, string>> = {
  register: '/auth/register',
  login: '/auth/login',
  logout: '/auth/logout',
  refresh: '/auth/refresh',
  me: '/auth/me',
  sessionToken: '/auth/session-token',
  exchangeSession: '/auth/exchange-session',
};

/**
 * Where the document and the client disagree.
 *
 * Reported as DATA, never thrown: a drifted path means the client is calling
 * the wrong place, and taking the app down at boot over it would turn a
 * degraded login into no app at all.
 */
export function pathDrift(discovery: AuthDiscovery): PathDrift[] {
  const advertised = discovery.endpoints as unknown as Record<string, string>;
  const drift: PathDrift[] = [];
  for (const [endpoint, clientUses] of Object.entries(CLIENT_AUTH_PATHS)) {
    const value = advertised[endpoint];
    if (typeof value === 'string' && value !== clientUses) {
      drift.push({ endpoint, advertised: value, clientUses });
    }
  }
  return drift;
}
