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
 * from the Holochain DHT.
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
// Holochain Learning Path Types (match Rust DNA structures)
// =============================================================================

/**
 * Path index entry from get_all_paths
 */
export interface HolochainPathIndexEntry {
  id: string;
  title: string;
  description: string;
  difficulty: string;
  estimated_duration: string | null;
  step_count: number;
  tags: string[];
}

/**
 * Path index output from get_all_paths
 */
export interface HolochainPathIndex {
  paths: HolochainPathIndexEntry[];
  total_count: number;
  last_updated: string;
}

/**
 * Path step from Holochain
 */
export interface HolochainPathStep {
  id: string;
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title: string | null;
  step_narrative: string | null;
  is_optional: boolean;
}

/**
 * Learning path entry from Holochain
 */
export interface HolochainLearningPath {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  created_by: string;
  difficulty: string;
  estimated_duration: string | null;
  visibility: string;
  path_type: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

/**
 * Path with steps output
 */
export interface HolochainPathWithSteps {
  action_hash: Uint8Array;
  path: HolochainLearningPath;
  steps: Array<{
    action_hash: Uint8Array;
    step: HolochainPathStep;
  }>;
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
   * Starts as false, set to true after successful testAvailability() call.
   * The HolochainClientService now properly discovers existing app interfaces
   * and authorizes signing credentials, enabling zome calls from the browser.
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
  // Learning Path Methods
  // ===========================================================================

  /**
   * Get all learning paths (path index).
   */
  async getPathIndex(): Promise<HolochainPathIndex> {
    console.log('[HolochainContent] Calling get_all_paths...');
    const result = await this.holochainClient.callZome<HolochainPathIndex>({
      zomeName: 'content_store',
      fnName: 'get_all_paths',
      payload: null,
    });

    console.log('[HolochainContent] get_all_paths result:', result);

    if (!result.success || !result.data) {
      console.warn('[HolochainContent] get_all_paths failed or empty:', result.error);
      return { paths: [], total_count: 0, last_updated: new Date().toISOString() };
    }

    console.log('[HolochainContent] Found paths:', result.data.total_count);
    return result.data;
  }

  /**
   * Get a learning path with all its steps.
   */
  async getPathWithSteps(pathId: string): Promise<HolochainPathWithSteps | null> {
    const result = await this.holochainClient.callZome<HolochainPathWithSteps | null>({
      zomeName: 'content_store',
      fnName: 'get_path_with_steps',
      payload: pathId,
    });

    if (!result.success || !result.data) {
      return null;
    }

    return result.data;
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
   * Map Holochain reach string to ReachLevel type.
   * Uses the ReachLevel values from protocol-core.model.ts:
   * private, invited, local, neighborhood, municipal, bioregional, regional, commons
   */
  private mapReachLevel(reach: string): ContentNode['reach'] {
    // Handle various reach string formats
    const reachMap: Record<string, ContentNode['reach']> = {
      'private': 'private',
      'invited': 'invited',
      'local': 'local',
      'neighborhood': 'neighborhood',
      'municipal': 'municipal',
      'community': 'municipal',       // Alias: community → municipal
      'bioregional': 'bioregional',
      'regional': 'regional',
      'federated': 'regional',        // Alias: federated → regional
      'commons': 'commons',
      'public': 'commons',            // Alias: public → commons
    };

    return reachMap[reach.toLowerCase()] ?? 'commons';
  }
}
