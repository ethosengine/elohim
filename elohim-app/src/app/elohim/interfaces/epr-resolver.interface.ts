/**
 * EPR Resolver Interfaces — split the monolith into focused contracts.
 *
 * The EprResolverService handles two fundamentally different concerns:
 *
 * 1. **URI resolution** (pure, synchronous) — parsing epr: URIs, building
 *    transport URLs, computing routes, context-aware navigation. No network.
 *
 * 2. **Content resolution** (async, network) — fetching metadata from storage,
 *    resolving EPR Heads with DAG-CBOR, loading three-pillar context.
 *
 * These interfaces formalize that seam. Components that only need URL building
 * depend on IEprUriResolver. Components that fetch metadata depend on
 * IEprContentResolver. Both are implemented by EprResolverService today,
 * but the interfaces enable future splits (e.g., P2P-native URI resolver
 * that doesn't go through doorway).
 */

import { Observable } from 'rxjs';

import type { EprHead } from '../models/epr-head.model';
import type {
  ResolvedEpr,
  ResolvedContent,
  ContextResolvedRoute,
  StepRef,
  CrossPathMatch,
} from '../services/epr-resolver.service';

/**
 * Pure EPR URI resolution — no network calls.
 *
 * Handles:
 * - epr: URI → transport-specific URL
 * - Blob hash → blob URL
 * - Context-aware navigation (in-path, cross-path, standalone)
 */
export interface IEprUriResolver {
  /** Resolve an epr: URI to a transport-specific URL and route. */
  resolveUrl(input: string, blobHash?: string): ResolvedEpr;

  /** Resolve a blob hash to a full URL via the current connection strategy. */
  resolveBlobUrl(hash: string): string;

  /**
   * Context-aware resolution — the HyperCard pattern.
   *
   * Same epr: link resolves differently based on navigation context:
   * - In a path containing the target → stay in path (in-path)
   * - In a path but target in another path → cross-path link
   * - No path context → standalone resource view
   *
   * This is a pure function — all data passed as parameters to keep
   * the elohim pillar independent of lamad.
   */
  resolveInContext(
    input: string,
    currentPathId: string | null,
    currentSteps: StepRef[],
    crossPathMatches?: CrossPathMatch[]
  ): ContextResolvedRoute;
}

/**
 * Async content resolution — fetches metadata from storage.
 *
 * Handles:
 * - epr: URI → full content metadata (title, description, blob URL, route)
 * - epr: URI → EPR Head with three-pillar context (DAG-CBOR)
 */
export interface IEprContentResolver {
  /** Resolve an epr: URI to full content metadata from storage. */
  resolve(input: string): Observable<ResolvedContent | null>;

  /** Resolve an EPR Head with DAG-CBOR content negotiation. */
  resolveEprHead(input: string): Observable<EprHead | null>;
}
