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

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

// @coverage: 68.6% (2026-03-03)

import { Observable, of, switchMap, map, catchError } from 'rxjs';

import {
  type IEprContentResolver,
  type IEprUriResolver,
} from '../interfaces/epr-resolver.interface';
import { decodeEprHead } from '../utils/epr-codec';
import { type EprRef, parseEpr, eprToRoute } from '../utils/epr-ref';

import { StorageClientService, type StorageContentNode } from './storage-client.service';

import type { EprHead } from '../models/epr-head.model';

export interface ResolvedEpr {
  /** The parsed EPR reference */
  ref: EprRef;
  /** Transport-specific URL for HTTP fetch (null for P2P-only) */
  url: string;
  /** Angular route for in-app navigation (null for blob tier) */
  route: string[] | null;
}

export interface ResolvedContent {
  /** The parsed EPR reference */
  ref: EprRef;
  /** Resolved content metadata (Tier 1/2) */
  content: StorageContentNode;
  /** Blob URL if content has a blob reference */
  blobUrl: string | null;
  /** Angular route for this content */
  route: string[];
}

/**
 * Result of context-aware EPR resolution.
 *
 * The same epr: link resolves differently depending on WHERE you click it:
 * - In a path containing the target content → navigate to that step (stay in path)
 * - In a path but content found in another path → cross-path link
 * - No path context or not found → standalone resource view
 */
export interface ContextResolvedRoute {
  /** Angular route segments for navigation */
  route: string[];
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
   *   // → "https://doorway.host/store/sha256-abc..." (doorway)
   *   // → "http://localhost:8090/blob/sha256-abc..." (P2P-native)
   */
  resolveUrl(input: string, blobHash?: string): ResolvedEpr {
    const ref = parseEpr(input);
    return {
      ref,
      url: this.buildUrl(ref, blobHash),
      route: eprToRoute(ref),
    };
  }

  /**
   * Full two-step resolution: resolve EPR → fetch metadata → resolve blob if needed.
   *
   * This is the async path — fetches the content metadata, then resolves the
   * blob URL from the metadata's blobHash. Returns everything a component needs
   * to render a content link with preview.
   *
   * @example
   *   resolver.resolve('epr:manifesto-foundations').subscribe(resolved => {
   *     // resolved.content  → { id, title, description, contentType, blobHash, ... }
   *     // resolved.blobUrl  → "https://doorway.host/store/sha256-abc..." or null
   *     // resolved.route    → ['/resource', 'manifesto-foundations']
   *   });
   */
  resolve(input: string): Observable<ResolvedContent | null> {
    const ref = parseEpr(input);

    return this.storage.getContent(ref.id).pipe(
      switchMap(content => {
        if (!content) return of(null);

        const blobHash = this.extractBlobHash(content);
        const blobUrl = blobHash ? this.storage.getBlobUrl(blobHash) : null;
        const route = eprToRoute(ref) ?? ['/resource', ref.id];

        return of({ ref, content, blobUrl, route });
      })
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

    // 1. Check current path for the target content
    if (currentPathId) {
      const stepIndex = currentSteps.findIndex(s => s.resourceId === targetId);
      if (stepIndex >= 0) {
        return {
          route: ['/lamad/path', currentPathId, 'step', String(stepIndex)],
          resolution: 'in-path',
          stepIndex,
        };
      }
    }

    // 2. Check cross-path matches (if provided by caller)
    if (crossPathMatches && crossPathMatches.length > 0) {
      const match = crossPathMatches[0];
      return {
        route: ['/lamad/path', match.pathId, 'step', String(match.stepIndex)],
        resolution: 'cross-path',
        crossPath: match,
      };
    }

    // 3. Standalone resource view (fallback)
    return {
      route: eprToRoute(ref) ?? ['/resource', targetId],
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

  private extractBlobHash(content: StorageContentNode): string | null {
    // Check explicit blobHash field (may be CID or sha256-{hex})
    if (content.blobHash) return normalizeContentAddress(content.blobHash);

    // Check if contentBody is a blob reference
    const body = content.contentBody ?? '';
    if (isContentAddress(body)) {
      return normalizeContentAddress(body);
    }

    return null;
  }
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
