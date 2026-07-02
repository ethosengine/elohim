/**
 * EprResolverService — Resolve epr: URIs to transport-specific URLs.
 *
 * This is the bridge between the protocol-level EPR URI and the
 * connection strategy (doorway vs P2P-native device). A developer
 * writes `epr:manifesto-foundations` — this service figures out
 * whether that means GET https://doorway.host/db/content/... or
 * GET http://localhost:8090/db/content/... based on runtime context.
 *
 * Content addressing uses IPFS-compatible CIDv1 as the canonical format.
 * Legacy sha256-{hex} is accepted on input and passed through — the
 * Rust backend (blob_store.rs:parse_content_address) handles all formats.
 *
 * See: protocol-specification.md Appendix E (Resolution Matrix)
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

// @coverage: 68.6% (2026-03-03)

import { Observable, of, map, catchError } from 'rxjs';

import {
  type EprRef,
  parseEpr,
  eprToRoute,
  eprToUniversalHref,
  BUNDLE_ROUTE_CONTEXT,
} from '@elohim/service';

import {
  type IEprContentResolver,
  type IEprUriResolver,
} from '../interfaces/epr-resolver.interface';
import { decodeEprHead } from '../utils/epr-codec';

import { StorageClientService } from './storage-client.service';

import type { EprHead } from '../models/epr-head.model';

export interface ResolvedEpr {
  /** The parsed EPR reference */
  ref: EprRef;
  /** Transport-specific URL for HTTP fetch (null for P2P-only) */
  url: string;
  /** In-bundle router commands (null = cross-bundle or blob tier) */
  route: string[] | null;
  /** Universal address — always navigable, in every bundle (§12.1) */
  href: string;
}

/**
 * Preview-tier content metadata — the subset a link/card needs to render,
 * sourced from the anonymous-safe EPR Head (never the reach-gated body).
 *
 * "Heads flow freely, bodies are gated": a preview must render from head
 * metadata alone, so a body 403/404 can never blank a card. Consumers that
 * genuinely need the content BYTES fetch them explicitly via
 * `StorageClientService.getContent()` at their own call site.
 */
export interface ResolvedContentPreview {
  /** Stable content id (slug) */
  id: string;
  /** Human-readable title */
  title: string;
  /** Short description (empty string when the head carries none) */
  description: string;
  /** Access reach (commons/community/…) — governs routing/provide decisions */
  reach?: string;
  /** Content type (epic/concept/path/…) — feeds eprToRoute routing */
  contentType?: string;
  /** Content format (markdown/sophia-quiz-json/…) */
  contentFormat?: string;
  /** Discovery tags */
  tags: string[];
}

export interface ResolvedContent {
  /** The parsed EPR reference */
  ref: EprRef;
  /** Resolved preview metadata (head-first — no body bytes) */
  content: ResolvedContentPreview;
  /** Blob URL if content has a blob reference (always null in head-first preview) */
  blobUrl: string | null;
  /** In-bundle router commands (null = cross-bundle or blob tier) */
  route: string[] | null;
  /** Universal address — always navigable, in every bundle (§12.1) */
  href: string;
}

/** Typed degradation state for a head-first preview resolution. */
export type EprPreviewState = 'resolved' | 'forbidden' | 'missing' | 'error';

/**
 * Everything a link/card preview needs, computed from the EPR Head.
 * Carries route/href so navigation never re-fetches to route.
 */
export interface EprPreview {
  ref: EprRef;
  id: string;
  title: string;
  description: string | null;
  reach: string | null;
  contentType: string | null;
  contentFormat: string | null;
  tags: string[];
  route: string[] | null;
  href: string;
}

/**
 * Discriminated outcome of a head-first preview resolution.
 *
 * The body being unreachable (403/404) can NEVER surface here — only the
 * anonymous-safe HEAD itself failing degrades, and it degrades to a typed,
 * legible state the UI can render honestly:
 * - `forbidden` — the head is gated at the reader's reach (rare, heads are
 *   anonymous-safe by design); render an "unavailable at your reach" affordance.
 * - `missing`   — 404; render an honest missing state.
 * - `error`     — network/other; render an honest error state.
 */
export type EprPreviewOutcome =
  | { state: 'resolved'; preview: EprPreview }
  | { state: 'forbidden'; preview?: EprPreview }
  | { state: 'missing' }
  | { state: 'error' };

/**
 * Result of context-aware EPR resolution.
 *
 * The same epr: link resolves differently depending on WHERE you click it:
 * - In a path containing the target content → navigate to that step (stay in path)
 * - In a path but content found in another path → cross-path link
 * - No path context or not found → standalone resource view
 */
export interface ContextResolvedRoute {
  /** In-bundle router commands (null = cross-bundle or blob tier) */
  route: string[] | null;
  /** Universal address — always navigable, in every bundle (§12.1) */
  href: string;
  /** How the link was resolved */
  resolution: 'in-path' | 'cross-path' | 'standalone';
  /** For in-path resolution: the step index within the current path */
  stepIndex?: number;
  /** For cross-path resolution: the target path and step */
  crossPath?: { pathId: string; stepIndex: number };
}

/**
 * Lightweight step reference for context-aware resolution.
 * Only the fields we need — avoids coupling to the full PathStep model.
 */
export interface StepRef {
  resourceId: string;
  order: number;
}

/**
 * Cross-path lookup result — where a content ID appears in other paths.
 * The caller (lamad pillar) provides these from PathService.
 */
export interface CrossPathMatch {
  pathId: string;
  stepIndex: number;
}

@Injectable({ providedIn: 'root' })
export class EprResolverService implements IEprUriResolver, IEprContentResolver {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);
  private readonly routeCtx = inject(BUNDLE_ROUTE_CONTEXT);

  /**
   * Resolve an epr: URI (or any accepted format) to a transport-specific URL.
   *
   * This is the synchronous path — returns the URL without fetching anything.
   * Use this when you already know the tier you want.
   *
   * @example
   *   // In a component:
   *   const url = resolver.resolveUrl('epr:manifesto-foundations');
   *   // → "https://doorway.host/db/content/manifesto-foundations" (doorway mode)
   *   // → "http://localhost:8090/db/content/manifesto-foundations" (P2P-native)
   *
   *   const blobUrl = resolver.resolveUrl('epr:manifesto-foundations/blob', 'sha256-abc...');
   *   // → "https://doorway.host/blob/sha256-abc..." (doorway)
   *   // → "http://localhost:8090/blob/sha256-abc..." (P2P-native)
   */
  resolveUrl(input: string, blobHash?: string): ResolvedEpr {
    const ref = parseEpr(input);
    const res = eprToRoute(ref, this.routeCtx);
    return {
      ref,
      url: this.buildUrl(ref, blobHash),
      route: res?.commands ?? null,
      href: res?.href ?? eprToUniversalHref(ref),
    };
  }

  /**
   * Head-first content resolution — everything a link/card preview needs.
   *
   * Resolution is served from the anonymous-safe EPR Head (`/epr-head/{id}`),
   * NOT the reach-gated body (`/db/content/{id}`). "Heads flow freely, bodies
   * are gated": a community-reach body 403s for an anonymous reader, but its
   * head still carries title/description/reach/contentType — everything a
   * preview and its route need. A body 403/404 therefore can never blank a
   * card. Consumers that need the content BYTES fetch them explicitly via
   * `StorageClientService.getContent()` at their own call site.
   *
   * Returns `null` only when the HEAD itself is unresolvable (forbidden /
   * missing / network) — the genuinely-unavailable case. Callers that want to
   * DISTINGUISH those states (to render a typed affordance) use
   * {@link resolvePreview} instead.
   *
   * @example
   *   resolver.resolve('epr:manifesto-foundations').subscribe(resolved => {
   *     // resolved.content  → { id, title, description, contentType, reach, ... }
   *     // resolved.route    → ['/epr', 'manifesto-foundations'] (shell) or in-bundle commands
   *     // resolved.href     → '/epr/manifesto-foundations' // route-literal-ok: JSDoc @example illustrating minted output, not a literal mint
   *   });
   */
  resolve(input: string): Observable<ResolvedContent | null> {
    return this.resolvePreview(input).pipe(
      map(outcome => (outcome.state === 'resolved' ? toResolvedContent(outcome.preview) : null))
    );
  }

  /**
   * Head-first preview resolution with TYPED degradation.
   *
   * Fetches the EPR Head and maps it to preview fields + route/href. The head
   * endpoint is anonymous-safe by design, so success is the common path even
   * for community-reach content whose body is gated. On failure it never
   * throws — it returns a discriminated {@link EprPreviewOutcome} so the UI can
   * render an honest state (forbidden / missing / error) instead of collapsing
   * to a raw `epr:{id}` string.
   */
  resolvePreview(input: string): Observable<EprPreviewOutcome> {
    const ref = parseEpr(input);
    const base = this.storage.getStorageBaseUrl();
    const url = `${base}/epr-head/${encodeURIComponent(ref.id)}`;

    return this.http
      .get(url, {
        responseType: 'arraybuffer',
        headers: { Accept: 'application/vnd.ipld.dag-cbor' },
      })
      .pipe(
        map((buffer): EprPreviewOutcome => {
          const head = decodeEprHead(new Uint8Array(buffer));
          return { state: 'resolved', preview: this.previewFromHead(ref, head) };
        }),
        catchError((err: unknown) => of(degradedOutcome(err)))
      );
  }

  /**
   * Resolve just the blob URL for a known hash.
   * Delegates to StorageClientService which handles doorway vs P2P-native routing.
   */
  resolveBlobUrl(hash: string): string {
    return this.storage.getBlobUrl(normalizeContentAddress(hash));
  }

  /**
   * Context-aware EPR resolution — the HyperCard recovery.
   *
   * Same `epr:rea-foundations` link resolves differently depending on WHERE you click:
   *
   * 1. **In a path** containing the target → stay in path, go to that step
   * 2. **In a path** but target in another path → cross-path link
   * 3. **No path context** → standalone resource view
   *
   * This is a pure function — all data passed as parameters to keep the elohim
   * pillar independent of lamad. The caller (markdown renderer, epr-link component)
   * provides path steps and cross-path results from lamad services.
   *
   * Route shape comes from the BundleRouteContext claims (§12.3).
   * In the shell (ownsUniversalRoute, no path claim), in-path/cross-path
   * targets resolve to /epr/{pathId}#step/{n} so the universal route handles
   * them. In a lamad bundle (path claim present), they resolve to in-bundle
   * router commands (['/path', pathId, 'step', n]).
   *
   * @param input - EPR URI string (e.g., 'epr:rea-foundations')
   * @param currentPathId - Current path ID (null if not in a path)
   * @param currentSteps - Steps of the current path (empty if not in a path)
   * @param crossPathMatches - Where this content appears in OTHER paths (optional)
   */
  resolveInContext(
    input: string,
    currentPathId: string | null,
    currentSteps: StepRef[],
    crossPathMatches?: CrossPathMatch[]
  ): ContextResolvedRoute {
    const ref = parseEpr(input);
    const targetId = ref.id;

    const stepResolution = (
      pathId: string,
      stepIndex: number
    ): { route: string[] | null; href: string } => {
      const stepRef: EprRef = {
        id: pathId,
        tier: 'head',
        fragment: { type: 'step', value: String(stepIndex) },
      };
      const res = eprToRoute(stepRef, this.routeCtx, 'path');
      return {
        route: res?.commands ?? null,
        href: res?.href ?? eprToUniversalHref(stepRef),
      };
    };

    // 1. Check current path for the target content
    if (currentPathId) {
      const stepIndex = currentSteps.findIndex(s => s.resourceId === targetId);
      if (stepIndex >= 0) {
        return { ...stepResolution(currentPathId, stepIndex), resolution: 'in-path', stepIndex };
      }
    }

    // 2. Check cross-path matches (if provided by caller)
    if (crossPathMatches && crossPathMatches.length > 0) {
      const match = crossPathMatches[0];
      return {
        ...stepResolution(match.pathId, match.stepIndex),
        resolution: 'cross-path',
        crossPath: match,
      };
    }

    // 3. Standalone resource view (fallback)
    const res = eprToRoute(ref, this.routeCtx);
    return {
      route: res?.commands ?? null,
      href: res?.href ?? eprToUniversalHref(ref),
      resolution: 'standalone',
    };
  }

  /**
   * Resolve an EPR Head directly, with DAG-CBOR content negotiation.
   *
   * Fetches from `/epr-head/{id}`, handling both JSON and DAG-CBOR responses
   * via the Accept header. Returns the typed EprHead metadata.
   */
  resolveEprHead(input: string): Observable<EprHead | null> {
    const ref = parseEpr(input);
    const base = this.storage.getStorageBaseUrl();
    const url = `${base}/epr-head/${encodeURIComponent(ref.id)}`;

    return this.http
      .get(url, {
        responseType: 'arraybuffer',
        headers: { Accept: 'application/vnd.ipld.dag-cbor' },
      })
      .pipe(
        map(buffer => {
          const bytes = new Uint8Array(buffer);
          return decodeEprHead(bytes);
        }),
        catchError(() => of(null))
      );
  }

  // ── Internal ──────────────────────────────────────────────────────────────

  private buildUrl(ref: EprRef, blobHash?: string): string {
    const base = this.storage.getStorageBaseUrl();

    switch (ref.tier) {
      case 'blob': {
        const hash = blobHash ?? '';
        return hash ? this.storage.getBlobUrl(hash) : '';
      }
      case 'doc':
      case 'head':
        return `${base}/db/content/${encodeURIComponent(ref.id)}`;
    }
  }

  /** Map an EPR Head to the preview fields + route/href a link/card renders. */
  private previewFromHead(ref: EprRef, head: EprHead): EprPreview {
    const contentType = head.lamad?.contentType;
    const res = eprToRoute(ref, this.routeCtx, contentType);
    return {
      ref,
      id: head.id || ref.id,
      title: head.lamad?.title ?? ref.id,
      description: head.lamad?.description ?? null,
      reach: head.qahal?.reach ?? null,
      contentType: contentType ?? null,
      contentFormat: head.lamad?.contentFormat ?? null,
      tags: head.lamad?.tags ?? [],
      route: res?.commands ?? null,
      href: res?.href ?? eprToUniversalHref(ref),
    };
  }
}

/** Project a head-first preview onto the ResolvedContent shape consumers use. */
function toResolvedContent(preview: EprPreview): ResolvedContent {
  return {
    ref: preview.ref,
    content: {
      id: preview.id,
      title: preview.title,
      description: preview.description ?? '',
      reach: preview.reach ?? undefined,
      contentType: preview.contentType ?? undefined,
      contentFormat: preview.contentFormat ?? undefined,
      tags: preview.tags,
    },
    // Head-first preview never fetches the body, so there is no blob URL here;
    // a consumer that needs the bytes fetches them explicitly (getContent).
    blobUrl: null,
    route: preview.route,
    href: preview.href,
  };
}

/** Classify a failed head fetch into a typed degradation outcome. */
function degradedOutcome(err: unknown): EprPreviewOutcome {
  const status = err instanceof HttpErrorResponse ? err.status : 0;
  if (status === 401 || status === 403) return { state: 'forbidden' };
  if (status === 404) return { state: 'missing' };
  return { state: 'error' };
}

/**
 * Check if a string looks like a content address (CID, sha256-{hex}, sha256:, or raw hex).
 */
export function isContentAddress(input: string): boolean {
  if (!input) return false;
  // CIDv1 base32 (bafkrei...) or base58 (Qm...)
  if (input.startsWith('bafk') || input.startsWith('Qm')) return true;
  // SHA256 with prefix
  if (input.startsWith('sha256-') || input.startsWith('sha256:')) return true;
  // Raw 64-char hex
  if (input.length === 64 && /^[\da-f]+$/i.test(input)) return true;
  return false;
}

/**
 * Normalize a content address for use in blob URLs.
 *
 * CIDv1 strings (bafkrei...) pass through unchanged — the Rust backend's
 * parse_content_address() handles all formats. Legacy sha256 formats are
 * normalized to sha256-{hex} for backward compatibility.
 *
 * See: blob_store.rs:parse_content_address(), protocol-specification.md Appendix E.5
 */
export function normalizeContentAddress(input: string): string {
  // CIDv1 base32 — pass through (canonical IPLD format)
  if (input.startsWith('bafk')) return input;
  // CIDv0 base58 — pass through (backend handles conversion)
  if (input.startsWith('Qm')) return input;
  // sha256-{hex} — already in legacy canonical form
  if (input.startsWith('sha256-') && input.length === 71) return input;
  // sha256:{hex} — normalize colon to hyphen
  if (input.startsWith('sha256:')) return `sha256-${input.slice(7)}`;
  // Raw 64-char hex — add sha256- prefix
  if (input.length === 64 && /^[\da-f]+$/i.test(input)) return `sha256-${input}`;
  return input;
}
