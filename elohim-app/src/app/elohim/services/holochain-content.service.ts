/**
 * Holochain Content Service
 *
 * Provides content retrieval from Holochain conductor.
 * Uses the HolochainClientService for WebSocket communication.
 *
 * Architecture:
 *   Browser → HolochainClientService → Holochain Conductor → DHT
 *
 * NOTE: This service is prepared for Holochain integration but currently returns
 * unavailable status until the app interface proxy (Phase 2) is implemented.
 * The admin interface works for testing, but zome calls require app interface.
 *
 * Once the proxy is ready, this service will transparently provide content
 * from the Holochain DHT instead of JSON files or Kuzu.
 *
 * @see HolochainClientService for connection management
 * @see DataLoaderService for the primary content abstraction (delegates here when ready)
 */

import { Injectable, computed, signal } from '@angular/core';
import { Observable, of, from, defer } from 'rxjs';
import { map, catchError, shareReplay, switchMap, tap } from 'rxjs/operators';
import { HolochainClientService } from './holochain-client.service';
import { ContentNode, ContentType, ContentFormat, ContentMetadata } from '../../lamad/models/content-node.model';

// =============================================================================
// Holochain Content Types (match Rust DNA structures)
// =============================================================================

/**
 * Content entry as stored in Holochain
 * Matches Content struct in integrity zome (extended schema)
 */
export interface HolochainContentEntry {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  author_id: string | null;
  reach: string;
  trust_score: number;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/**
 * Output from content retrieval zome calls
 */
export interface HolochainContentOutput {
  action_hash: Uint8Array;
  entry_hash: Uint8Array;
  content: HolochainContentEntry;
}

/**
 * Content statistics from get_content_stats
 */
export interface HolochainContentStats {
  total_count: number;
  by_type: Record<string, number>;
}

/**
 * Query input for content by type
 */
export interface QueryByTypeInput {
  content_type: string;
  limit?: number;
}

/**
 * Query input for content by ID
 */
export interface QueryByIdInput {
  id: string;
}

// =============================================================================
// Service Implementation
// =============================================================================

@Injectable({
  providedIn: 'root',
})
export class HolochainContentService {
  /**
   * Whether Holochain content service is available.
   *
   * Currently returns false because the app interface proxy (Phase 2) is not
   * yet implemented. The HolochainClientService can connect to admin interface,
   * but zome calls require app interface which returns localhost ports.
   *
   * Set to true once:
   * 1. App interface proxy is deployed
   * 2. Connection flow supports browser-based zome calls
   */
  private readonly availableSignal = signal(false);
  readonly available = this.availableSignal.asReadonly();

  /** Computed: true when Holochain client is connected AND available */
  readonly ready = computed(() => this.available() && this.holochainClient.isConnected());

  /** Cache for content by ID */
  private readonly contentCache = new Map<string, Observable<ContentNode | null>>();

  /** Cache for stats (refreshed periodically) */
  private statsCache$: Observable<HolochainContentStats> | null = null;

  constructor(private readonly holochainClient: HolochainClientService) {}

  /**
   * Check if Holochain content is available for use.
   *
   * This returns false until the app interface proxy is implemented.
   * DataLoaderService should check this before delegating to this service.
   */
  isAvailable(): boolean {
    return this.availableSignal();
  }

  /**
   * Get content by ID from Holochain.
   *
   * Returns null if content not found or service unavailable.
   * Uses caching to avoid redundant zome calls.
   */
  getContent(resourceId: string): Observable<ContentNode | null> {
    if (!this.isAvailable()) {
      return of(null);
    }

    if (!this.contentCache.has(resourceId)) {
      const request = defer(() =>
        from(this.fetchContentById(resourceId))
      ).pipe(
        shareReplay(1),
        catchError((err) => {
          console.warn(`[HolochainContent] Failed to fetch "${resourceId}":`, err);
          return of(null);
        })
      );

      this.contentCache.set(resourceId, request);
    }

    return this.contentCache.get(resourceId)!;
  }

  /**
   * Get content by type from Holochain.
   *
   * Returns empty array if service unavailable.
   */
  getContentByType(contentType: string, limit = 100): Observable<ContentNode[]> {
    if (!this.isAvailable()) {
      return of([]);
    }

    return defer(() =>
      from(this.fetchContentByType(contentType, limit))
    ).pipe(
      catchError((err) => {
        console.warn(`[HolochainContent] Failed to fetch type "${contentType}":`, err);
        return of([]);
      })
    );
  }

  /**
   * Get content statistics from Holochain.
   *
   * Cached with shareReplay for efficiency.
   */
  getStats(): Observable<HolochainContentStats> {
    if (!this.isAvailable()) {
      return of({ total_count: 0, by_type: {} });
    }

    this.statsCache$ ??= defer(() =>
      from(this.fetchStats())
    ).pipe(
      shareReplay(1),
      catchError(() => of({ total_count: 0, by_type: {} }))
    );

    return this.statsCache$;
  }

  /**
   * Clear all caches (useful after imports or when data changes)
   */
  clearCache(): void {
    this.contentCache.clear();
    this.statsCache$ = null;
  }

  /**
   * Test if Holochain content API is reachable.
   *
   * Attempts a simple zome call to verify connectivity.
   * Updates the available signal based on result.
   */
  async testAvailability(): Promise<boolean> {
    try {
      const result = await this.holochainClient.callZome<HolochainContentStats>({
        zomeName: 'content_store',
        fnName: 'get_content_stats',
        payload: null,
      });

      if (result.success) {
        this.availableSignal.set(true);
        console.log('[HolochainContent] Service available, content count:', result.data?.total_count);
        return true;
      }

      this.availableSignal.set(false);
      return false;
    } catch (err) {
      console.warn('[HolochainContent] Availability test failed:', err);
      this.availableSignal.set(false);
      return false;
    }
  }

  // ===========================================================================
  // Private Methods - Zome Calls
  // ===========================================================================

  /**
   * Fetch single content by ID from Holochain
   */
  private async fetchContentById(id: string): Promise<ContentNode | null> {
    const result = await this.holochainClient.callZome<HolochainContentOutput | null>({
      zomeName: 'content_store',
      fnName: 'get_content_by_id',
      payload: { id } as QueryByIdInput,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return this.transformToContentNode(result.data);
  }

  /**
   * Fetch content by type from Holochain
   */
  private async fetchContentByType(contentType: string, limit: number): Promise<ContentNode[]> {
    const result = await this.holochainClient.callZome<HolochainContentOutput[]>({
      zomeName: 'content_store',
      fnName: 'get_content_by_type',
      payload: { content_type: contentType, limit } as QueryByTypeInput,
    });

    if (!result.success || !result.data) {
      return [];
    }

    return result.data.map((output) => this.transformToContentNode(output));
  }

  /**
   * Fetch content statistics from Holochain
   */
  private async fetchStats(): Promise<HolochainContentStats> {
    const result = await this.holochainClient.callZome<HolochainContentStats>({
      zomeName: 'content_store',
      fnName: 'get_content_stats',
      payload: null,
    });

    if (!result.success || !result.data) {
      return { total_count: 0, by_type: {} };
    }

    return result.data;
  }

  // ===========================================================================
  // Transformation - Holochain Entry → ContentNode
  // ===========================================================================

  /**
   * Transform Holochain content output to ContentNode
   *
   * Maps snake_case Rust fields to camelCase TypeScript fields.
   * Parses metadata_json back to ContentMetadata object.
   */
  private transformToContentNode(output: HolochainContentOutput): ContentNode {
    const entry = output.content;

    // Parse metadata JSON
    let metadata: ContentMetadata = {};
    try {
      metadata = JSON.parse(entry.metadata_json || '{}');
    } catch {
      console.warn(`[HolochainContent] Failed to parse metadata for "${entry.id}"`);
    }

    return {
      id: entry.id,
      contentType: entry.content_type as ContentType,
      title: entry.title,
      description: entry.description,
      content: entry.content,
      contentFormat: entry.content_format as ContentFormat,
      tags: entry.tags,
      sourcePath: entry.source_path ?? undefined,
      relatedNodeIds: entry.related_node_ids,
      metadata,
      authorId: entry.author_id ?? undefined,
      reach: this.mapReachLevel(entry.reach),
      trustScore: entry.trust_score,
      createdAt: entry.created_at,
      updatedAt: entry.updated_at,
    };
  }

  /**
   * Map Holochain reach string to ContentReach type
   */
  private mapReachLevel(reach: string): ContentNode['reach'] {
    const validReaches: ContentNode['reach'][] = [
      'private', 'invited', 'local', 'community', 'federated', 'commons'
    ];

    // Handle Holochain's neighborhood/municipal/etc. mappings
    const reachMap: Record<string, ContentNode['reach']> = {
      'private': 'private',
      'invited': 'invited',
      'local': 'local',
      'neighborhood': 'local',        // Map to local
      'municipal': 'community',       // Map to community
      'community': 'community',
      'bioregional': 'community',     // Map to community
      'regional': 'federated',        // Map to federated
      'federated': 'federated',
      'commons': 'commons',
    };

    return reachMap[reach.toLowerCase()] ?? 'commons';
  }
}
