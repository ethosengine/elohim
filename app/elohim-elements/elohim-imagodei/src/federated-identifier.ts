/**
 * Federated-identifier helpers — parse + resolve the `user@gateway-host` form.
 *
 * Framework-agnostic; consumed by both Angular services (via doorway.model.ts
 * re-export) and the standalone imagodei-portal bundle's StandaloneResolver.
 *
 * Pure TS — no Lit, no Angular, no DOM dependencies (fetch is available in
 * browser and Tauri webview contexts but this module does not call it; all
 * resolution is currently convention-based and synchronous).
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
}

export type ResolveOutcome =
  | { ok: true; doorway: DoorwayDescriptor }
  | { ok: false; reason: string };

/**
 * Resolve a gateway host to a doorway URL using a convention-first strategy.
 *
 * Resolution order:
 * 1. Check `knownDoorways` for an entry whose URL contains the gateway domain
 *    or whose URL contains `doorway-{first-subdomain}`.
 * 2. Convention: `sub.host.tld` (≥ 3 parts, not already prefixed) →
 *    `https://doorway-sub.host.tld`.
 * 3. Fallback: `https://{gatewayDomain}` (the domain IS the doorway).
 *
 * This is synchronous and network-free. Deeper discovery (`.well-known`,
 * federation registry) belongs in a coordinator layer that calls this first.
 *
 * @param gatewayDomain  Hostname extracted from the federated identifier.
 * @param knownDoorways  Optional list of pre-resolved doorways; defaults to [].
 */
export function resolveGatewayToDoorwayUrl(
  gatewayDomain: string,
  knownDoorways: ReadonlyArray<DoorwayDescriptor> = []
): ResolveOutcome {
  // 1. Match against known doorways (by URL content or doorway-prefix convention)
  const parts = gatewayDomain.split('.');
  const firstSubdomain = parts[0] ?? '';
  const known = knownDoorways.find(
    d => d.url.includes(gatewayDomain) || d.url.includes(`doorway-${firstSubdomain}`)
  );
  if (known) {
    return { ok: true, doorway: { url: known.url, id: known.id } };
  }

  // 2. Convention: alpha.elohim.host → https://doorway-alpha.elohim.host
  if (parts.length >= 3 && !firstSubdomain.startsWith('doorway-')) {
    return { ok: true, doorway: { url: `https://doorway-${gatewayDomain}` } };
  }

  // 3. Fallback: domain may be the doorway itself
  return { ok: true, doorway: { url: `https://${gatewayDomain}` } };
}
