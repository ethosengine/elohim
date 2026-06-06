/**
 * EprRef — Parse and produce Elohim Protocol Reference URIs.
 *
 * The `epr:` URI is the canonical content reference across all components.
 * Transport-agnostic: resolves to HTTP, libp2p, or local depending on context.
 *
 * Syntax:
 *   epr:{id}[@{version}][/{tier}][?{query}][#{fragment}]
 *
 * Examples:
 *   epr:manifesto-foundations           → EPR Head for manifesto-foundations
 *   epr:manifesto-foundations@3         → Version 3
 *   epr:manifesto-foundations/blob      → Content bytes (Tier 3)
 *   epr:manifesto-foundations/doc       → Full EPR Document (Tier 2)
 *   epr:elohim-protocol#step/2         → Step 2 of a learning path
 *   epr:systems-thinking#rel/PREREQUISITE/feedback-loops → Graph edge
 *   epr:manifesto?via=did:web:alpha.elohim.host → Resolution hint
 *
 * See: protocol-specification.md Appendix E
 */

export type EprTier = 'head' | 'doc' | 'blob';

export interface EprFragment {
  type: 'step' | 'chapter' | 'rel';
  /** Step index, chapter ID, or relationship target ID */
  value: string;
  /** For rel fragments: the relationship type (PREREQUISITE, CONTAINS, etc.) */
  relType?: string;
}

export interface EprRef {
  /** Content ID (stable slug) */
  id: string;
  /** Specific version (undefined = latest) */
  version?: number;
  /** Resolution tier: head (default), doc, or blob */
  tier: EprTier;
  /** Resolution hint: prefer this doorway or peer */
  via?: string;
  /** Access level hint */
  reach?: string;
  /** Graph position: step, chapter, or relationship */
  fragment?: EprFragment;
}

const EPR_PREFIX = 'epr:';
const DID_CONTENT_PATTERN = /^did:web:[\w.-]+:content:(.+)$/;
const DID_PATHS_PATTERN = /^did:web:[\w.-]+:paths:(.+)$/;

/**
 * Parse any content reference into an EprRef.
 *
 * Accepts:
 *   - epr:{id}[@version][/tier][?query][#fragment]
 *   - did:web:{host}:content:{id}
 *   - did:web:{host}:paths:{id}
 *   - bare ID string (treated as epr:{id})
 */
export function parseEpr(input: string): EprRef {
  // DID → EPR bridge
  const contentMatch = DID_CONTENT_PATTERN.exec(input);
  if (contentMatch) {
    return { id: contentMatch[1], tier: 'head' };
  }
  const pathMatch = DID_PATHS_PATTERN.exec(input);
  if (pathMatch) {
    return { id: pathMatch[1], tier: 'head' };
  }

  // Strip epr: prefix if present
  let raw = input.startsWith(EPR_PREFIX) ? input.slice(EPR_PREFIX.length) : input;

  // Parse fragment (#step/2, #chapter/economic, #rel/PREREQUISITE/feedback-loops)
  let fragment: EprFragment | undefined;
  const hashIdx = raw.indexOf('#');
  if (hashIdx !== -1) {
    fragment = parseFragment(raw.slice(hashIdx + 1));
    raw = raw.slice(0, hashIdx);
  }

  // Parse query (?via=...&reach=...)
  let via: string | undefined;
  let reach: string | undefined;
  const qIdx = raw.indexOf('?');
  if (qIdx !== -1) {
    const params = new URLSearchParams(raw.slice(qIdx + 1));
    via = params.get('via') ?? undefined;
    reach = params.get('reach') ?? undefined;
    raw = raw.slice(0, qIdx);
  }

  // Parse tier (/doc, /blob)
  let tier: EprTier = 'head';
  if (raw.endsWith('/doc')) {
    tier = 'doc';
    raw = raw.slice(0, -4);
  } else if (raw.endsWith('/blob')) {
    tier = 'blob';
    raw = raw.slice(0, -5);
  }

  // Parse version (@3)
  let version: number | undefined;
  const atIdx = raw.indexOf('@');
  if (atIdx !== -1) {
    version = Number.parseInt(raw.slice(atIdx + 1), 10);
    raw = raw.slice(0, atIdx);
  }

  return { id: raw, version, tier, via, reach, fragment };
}

/**
 * Produce a canonical epr: URI string from an EprRef.
 */
export function formatEpr(ref: EprRef): string {
  let uri = `epr:${ref.id}`;

  if (ref.version !== undefined) {
    uri += `@${ref.version}`;
  }

  if (ref.tier === 'doc') {
    uri += '/doc';
  } else if (ref.tier === 'blob') {
    uri += '/blob';
  }

  const params: string[] = [];
  if (ref.via) params.push(`via=${ref.via}`);
  if (ref.reach) params.push(`reach=${ref.reach}`);
  if (params.length) {
    uri += `?${params.join('&')}`;
  }

  if (ref.fragment) {
    uri += `#${formatFragment(ref.fragment)}`;
  }

  return uri;
}

/**
 * Shorthand: create an EprRef from a content ID.
 */
export function epr(id: string, tier: EprTier = 'head'): EprRef {
  return { id, tier };
}

// ── §12.3 Mount-agnostic link minting ──────────────────────────────────────

/** A contentType this bundle renders natively, with its in-bundle route shape. */
export interface RouteClaim {
  contentType: string;
  /** Mint in-bundle router commands for a claimed ref. */
  commands(ref: EprRef): string[];
}

/** Serializable claim shape (spec §3.1) — what bundle manifests DECLARE and
 *  commitment grants carry. {id}/{n} placeholders, segment-safe. */
export interface RouteClaimTemplate {
  contentType: string;
  template: string;
  fragments?: Record<string, string>;
}

/**
 * Interpret serializable claim templates into executable RouteClaims (§8.3).
 * The single authoring home: bundles declare templates; this turns them into
 * the commands() the router consumes. A fragment whose type has a template
 * uses it ({n} = fragment value); otherwise the base template applies.
 */
export function claimsFromDeclaration(decl: readonly RouteClaimTemplate[]): RouteClaim[] {
  return decl.map(d => ({
    contentType: d.contentType,
    commands: (ref: EprRef): string[] => {
      const fragTpl = ref.fragment ? d.fragments?.[ref.fragment.type] : undefined;
      const tpl = fragTpl ?? d.template;
      const substituted = tpl
        .replace('{id}', ref.id)
        .replace('{n}', ref.fragment?.value ?? '');
      const segments = substituted.split('/').filter(s => s.length > 0);
      return ['/' + segments[0], ...segments.slice(1)];
    },
  }));
}

/**
 * Declared by each bundle's composition root (provide via BUNDLE_ROUTE_CONTEXT):
 * which EPR shapes this bundle renders natively. Unclaimed → /epr/{id}.
 */
export interface BundleRouteContext {
  claims: readonly RouteClaim[];
  /** The shell owns the universal epr/:resourceId route. */
  ownsUniversalRoute?: boolean;
}

/**
 * Result of claims-aware route minting. `commands` is non-null iff THIS bundle
 * claims the target (routerLink/router.navigate is safe); otherwise navigate
 * cross-bundle via `href` (plain anchor → epr-link interceptor, or
 * EprNavService.navigate → full doorway load).
 */
export interface EprRouteResolution {
  commands: string[] | null;
  /** Origin-absolute universal address — always present, safe in every bundle. */
  href: string;
  claimed: boolean;
}

/**
 * Mint the universal EPR address (§12.1): /epr/{id}[#fragment].
 * The fragment is deliberately NOT encoded: its structural slashes
 * (step/2, rel/PREREQUISITE/x) must stay raw for parseEpr round-trips and
 * the Slice-3 doorway resolver. Fragment VALUES are assumed URL-safe slugs.
 */
export function eprToUniversalHref(ref: EprRef): string {
  const frag = ref.fragment ? `#${formatFragment(ref.fragment)}` : '';
  return `/epr/${encodeURIComponent(ref.id)}${frag}`;
}

/**
 * Convert an EprRef to a navigable resolution for THIS bundle (§12.3):
 * in-mount targets → relative commands; everything else → /epr/{id}.
 * Route shape comes from the EPR head's contentType (caller-supplied when
 * known) — never guessed from the id. The single sanctioned structural
 * inference: a `step` fragment implies contentType 'path'.
 * Returns null for blob tier (no page address).
 */
export function eprToRoute(
  ref: EprRef,
  ctx: BundleRouteContext,
  contentType?: string | null
): EprRouteResolution | null {
  if (ref.tier === 'blob') return null;

  const effectiveType = contentType ?? (ref.fragment?.type === 'step' ? 'path' : null);
  const href = eprToUniversalHref(ref);

  if (effectiveType) {
    const claim = ctx.claims.find(c => c.contentType === effectiveType);
    if (claim) {
      return { commands: claim.commands(ref), href, claimed: true };
    }
  }

  if (ctx.ownsUniversalRoute) {
    return { commands: ['/epr', ref.id], href, claimed: false };
  }

  return { commands: null, href, claimed: false };
}

/**
 * Convert an EPR content ID to its W3C DID equivalent.
 */
export function eprToDid(id: string, host = 'hosted.elohim.host', namespace = 'content'): string {
  return `did:web:${host}:${namespace}:${id}`;
}

// ── Internal ────────────────────────────────────────────────────────────────

function parseFragment(raw: string): EprFragment | undefined {
  if (raw.startsWith('step/')) {
    return { type: 'step', value: raw.slice(5) };
  }
  if (raw.startsWith('chapter/')) {
    return { type: 'chapter', value: raw.slice(8) };
  }
  if (raw.startsWith('rel/')) {
    const parts = raw.slice(4).split('/');
    if (parts.length >= 2) {
      return { type: 'rel', relType: parts[0], value: parts.slice(1).join('/') };
    }
  }
  return undefined;
}

function formatFragment(f: EprFragment): string {
  switch (f.type) {
    case 'step':
      return `step/${f.value}`;
    case 'chapter':
      return `chapter/${f.value}`;
    case 'rel':
      return `rel/${f.relType}/${f.value}`;
  }
}
