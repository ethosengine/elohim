/**
 * Federated-identifier helpers — parse + resolve the `user@gateway-host` form.
 *
 * Framework-agnostic; consumed by both Angular services (via doorway.model.ts
 * re-export) and the standalone imagodei-portal bundle's StandaloneResolver.
 *
 * Pure TS — no Lit, no Angular, no DOM dependencies. Resolution is LOOKUP
 * (synchronous, against doorways the caller already trusts) or PROOF
 * (`probeDoorway`, one fetch against the typed host itself). It is never
 * synthesis: a host a human typed must never become the origin a password is
 * POSTed to just because a template said so.
 */

// =============================================================================
// Parsed identifier
// =============================================================================

/**
 * A successfully parsed `user@gateway-host` identifier.
 *
 * Field names match the existing Angular contract so that `doorway.model.ts`
 * can re-export this type without breaking call sites.
 */
export interface FederatedIdentifier {
  /** The full raw identifier (trimmed, leading `@` stripped). */
  identifier: string;
  /** The local-part (before the last `@`). */
  username: string;
  /** The gateway host (after the last `@`). */
  gatewayDomain: string;
}

export type ParseOutcome =
  | { ok: true; value: FederatedIdentifier }
  | { ok: false; reason: 'no-at-sign' | 'empty-local' | 'empty-gateway' | 'malformed' };

// =============================================================================
// parseFederatedIdentifier
// =============================================================================

/**
 * Parse a federated identifier of the form `local@gateway-host`.
 *
 * - Leading `@` is stripped before parsing (ActivityPub-style mentions).
 * - The **last** `@` is the separator, so email-like local-parts
 *   (`user@email.com@gateway`) are handled correctly.
 * - Returns a discriminated-union `ParseOutcome` — callers branch without
 *   try/catch.
 *
 * @example
 * ```ts
 * const r = parseFederatedIdentifier('matthew@alpha.elohim.host');
 * if (r.ok) console.log(r.value.username, r.value.gatewayDomain);
 * ```
 */
export function parseFederatedIdentifier(input: string): ParseOutcome {
  const trimmed = input.trim().replace(/^@/, '');

  const atIndex = trimmed.lastIndexOf('@');

  if (atIndex < 0) {
    return { ok: false, reason: 'no-at-sign' };
  }

  const username = trimmed.substring(0, atIndex);
  const gatewayDomain = trimmed.substring(atIndex + 1);

  if (username.length === 0) {
    return { ok: false, reason: 'empty-local' };
  }

  if (gatewayDomain.length === 0) {
    return { ok: false, reason: 'empty-gateway' };
  }

  return {
    ok: true,
    value: { identifier: trimmed, username, gatewayDomain },
  };
}

// =============================================================================
// Doorway resolution
// =============================================================================

/**
 * Minimal doorway descriptor sufficient for URL resolution.
 *
 * The full `DoorwayInfo` in `doorway.model.ts` extends this for Angular UI
 * needs (status, region, operator, etc.). This type is deliberately slim so
 * the standalone bundle does not pull in Angular-only concerns.
 */
export interface DoorwayDescriptor {
  /** Canonical doorway URL (HTTPS). */
  url: string;
  /** Optional doorway-id slug (kebab-case). */
  id?: string;
  /**
   * Gateway host whose identifiers this doorway serves — e.g. the doorway at
   * `https://doorway-alpha.elohim.host` DECLARES that it answers for
   * `alpha.elohim.host`. Declared by the doorway entry, never derived by
   * string surgery on whatever a human typed.
   */
  gatewayDomain?: string;
}

export type ResolveOutcome =
  | { ok: true; doorway: DoorwayDescriptor }
  | { ok: false; reason: 'empty-gateway' | 'unknown-gateway' };

/** Lowercased host (with port) of a URL, or null when it does not parse. */
function hostOf(url: string): string | null {
  try {
    return new URL(url).host.toLowerCase();
  } catch {
    return null;
  }
}

/** Normalize a typed gateway host: trim, lowercase, drop any scheme/path. */
function normalizeHost(input: string): string {
  return input
    .trim()
    .toLowerCase()
    .replace(/^[a-z][a-z0-9+.-]*:\/\//, '')
    .replace(/\/.*$/, '');
}

/**
 * Resolve a gateway host to a doorway URL by LOOKUP — never by synthesis.
 *
 * A doorway is returned only when a `knownDoorways` entry vouches for the
 * host: its own URL host EQUALS the typed host, or it declares that host in
 * `gatewayDomain`. Anything else is `{ ok: false, reason: 'unknown-gateway' }`,
 * and the caller must prove the host with {@link probeDoorway} before sending
 * anything to it.
 *
 * There is deliberately no `https://doorway-{host}` convention and no
 * substring match. Both let a typed identifier NAME the origin a password is
 * POSTed to: `me@x.evil.tld` synthesized `https://doorway-x.evil.tld`, and a
 * `.includes()` match made `alpha.elohim.host.evil.tld` read as the known
 * alpha doorway.
 *
 * @param gatewayDomain  Hostname extracted from the federated identifier.
 * @param knownDoorways  Doorways already trusted by the caller; defaults to [].
 */
export function resolveGatewayToDoorwayUrl(
  gatewayDomain: string,
  knownDoorways: ReadonlyArray<DoorwayDescriptor> = []
): ResolveOutcome {
  const wanted = normalizeHost(gatewayDomain);
  if (wanted.length === 0) {
    return { ok: false, reason: 'empty-gateway' };
  }

  const known = knownDoorways.find(
    d =>
      hostOf(d.url) === wanted ||
      (d.gatewayDomain !== undefined && normalizeHost(d.gatewayDomain) === wanted)
  );
  if (known) {
    return { ok: true, doorway: { url: known.url, id: known.id } };
  }

  return { ok: false, reason: 'unknown-gateway' };
}

/**
 * Ask a host whether it is a doorway, by fetching its own auth-discovery
 * document (`GET /.well-known/elohim-auth`, served by
 * `doorway-service/src/routes/auth_discovery.rs` — unauthenticated by design).
 *
 * The proof is the TYPED host answering for itself. On success this returns
 * that host's origin and nothing else: never a `doorway-`-prefixed derivative,
 * never a host read out of the response body. An unreachable host, a non-2xx,
 * or a malformed host all return null — the caller then has no doorway, which
 * is the safe outcome.
 *
 * @param gatewayHost  Host from the identifier (scheme/path tolerated, ignored).
 * @param fetchImpl    Injectable fetch, for tests; defaults to `globalThis.fetch`.
 */
export async function probeDoorway(
  gatewayHost: string,
  fetchImpl?: typeof fetch
): Promise<string | null> {
  const host = normalizeHost(gatewayHost);
  if (host.length === 0) {
    return null;
  }

  const origin = `https://${host}`;
  let probeUrl: URL;
  try {
    probeUrl = new URL('/.well-known/elohim-auth', origin);
  } catch {
    return null;
  }
  // A host that does not survive URL parsing unchanged is not the host we
  // were asked about (userinfo/`@` smuggling), so refuse rather than probe.
  if (probeUrl.host !== host) {
    return null;
  }

  const doFetch =
    fetchImpl ??
    ((input: RequestInfo | URL, init?: RequestInit) => globalThis.fetch(input, init));

  try {
    const resp = await doFetch(probeUrl.toString(), { credentials: 'omit' });
    return resp.ok ? origin : null;
  } catch {
    return null;
  }
}
