import { Injectable, inject, afterNextRender } from '@angular/core';

// @coverage: 20.6% (2026-02-24)

import { catchError, finalize, map, shareReplay, tap, switchMap, timeout } from 'rxjs/operators';

import { Observable, of, from, defer, forkJoin } from 'rxjs';

// Models from elohim (local)

// Models from lamad pillar (will stay there - content-specific)
// Using relative imports for now; will update to @app/lamad after full migration
import { ContentAttestation } from '../../lamad/models/content-attestation.model';
import {
  ContentNode,
  ContentGraph,
  ContentRelationship,
  ContentRelationshipType,
} from '../../lamad/models/content-node.model';
import {
  KnowledgeMapIndex,
  KnowledgeMap,
  KnowledgeMapIndexEntry,
  KnowledgeMapType,
  KnowledgeNode,
} from '../../lamad/models/knowledge-map.model';
import { LearningPath, PathIndex, PathIndexEntry, parsePathView } from '../../lamad/models/learning-path.model';
import {
  PathExtensionIndex,
  PathExtension,
  PathExtensionIndexEntry,
  PathStepInsertion,
  PathStepAnnotation,
  PathStepReorder,
  PathStepExclusion,
  UpstreamProposal,
  ExtensionStats,
} from '../../lamad/models/path-extension.model';
import { Agent, AgentProgress, AgentAttestation } from '../models/agent.model';

import { ContentResolverService } from './content-resolver.service';
import { ContentService } from './content.service';
import { GOVERNANCE } from '../interfaces/governance.interface';
import { CONTENT_ATTESTATION } from '../interfaces/content-attestation.interface';

import type { IGovernance } from '../interfaces/governance.interface';
import type { IContentAttestation } from '../interfaces/content-attestation.interface';
import type { GovernanceStateView, ChallengeView, DiscussionView } from '@elohim/storage-client/generated';
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { LoggerService } from './logger.service';
import { ProjectionAPIService } from './projection-api.service';

// Content index types
export interface ContentIndexEntry {
  id: string;
  title: string;
  description: string;
  contentType: string;
  tags?: string[];
  reach?: string;
  trustScore?: number;
  createdAt?: string;
  updatedAt?: string;
}

export interface ContentIndex {
  nodes: ContentIndexEntry[];
  totalCount?: number;
  byType?: Record<string, number>;
  lastUpdated?: string;
}

/** Mutable index maps accumulated during graph traversal. */
/** Recursive graph node shape from simplified Holochain graph responses. */
interface GraphNodeData {
  contentId: string;
  relationshipType: string;
  children: GraphNodeData[];
}

// Assessment types (inline until models are expanded)
export interface AssessmentIndex {
  lastUpdated: string;
  totalCount: number;
  assessments: AssessmentIndexEntry[];
}

export interface AssessmentIndexEntry {
  id: string;
  title: string;
  domain: string;
  instrumentType: string;
  estimatedTime: string;
}

// Governance types (inline until models are expanded)
export interface GovernanceIndex {
  lastUpdated: string;
  challengeCount: number;
  proposalCount: number;
  precedentCount: number;
  discussionCount: number;
}

export interface ChallengeRecord {
  id: string;
  entityType: string;
  entityId: string;
  challenger: { agentId: string; displayName: string; standing: string };
  grounds: string;
  description: string;
  status: string;
  filedAt: string;
  slaDeadline?: string;
  assignedElohim?: string;
  resolution?: {
    outcome: string;
    reasoning: string;
    decidedBy: string;
    decidedAt: string;
  };
}

export interface ProposalRecord {
  id: string;
  title: string;
  proposalType: string;
  description: string;
  proposer: { agentId: string; displayName: string };
  status: string;
  phase: string;
  createdAt: string;
  votingConfig?: {
    mechanism: string;
    quorum: number;
    passageThreshold: number;
  };
  currentVotes?: Record<string, number>;
  outcome?: {
    decision: string;
    reasoning: string;
  };
}

export interface PrecedentRecord {
  id: string;
  title: string;
  summary: string;
  fullReasoning: string;
  binding: string;
  scope: { entityTypes: string[]; categories?: string[]; roles?: string[] };
  citations: number;
  status: string;
}

export interface DiscussionRecord {
  id: string;
  entityType: string;
  entityId: string;
  category: string;
  title: string;
  messages: {
    id: string;
    authorId: string;
    authorName: string;
    content: string;
    createdAt: string;
  }[];
  status: string;
  messageCount: number;
}

export interface GovernanceStateRecord {
  entityType: string;
  entityId: string;
  status: string;
  statusBasis: {
    method: string;
    reasoning: string;
    deciderId: string;
    deciderType: string;
    decidedAt: string;
  };
  labels: { labelType: string; severity: string; appliedBy: string }[];
  activeChallenges: string[];
  lastUpdated: string;
}

// Cluster graph types (for hierarchical graph visualization)
export interface ClusterConnectionData {
  sourceClusterId: string;
  targetClusterId: string;
  connectionCount: number;
  relationshipTypes: string[];
}

export interface ClusterConnectionSummary {
  clusterId: string;
  outgoingByCluster: Map<string, ClusterConnectionData>;
  incomingByCluster: Map<string, ClusterConnectionData>;
  totalConnections: number;
}

/**
 * DataLoaderService - Loads data from Holochain via HolochainContentService.
 *
 * This service is the ONLY place that knows about the data source.
 * All other services depend on this abstraction.
 *
 * Migration Status:
 * - Content, Paths, Steps: Fully migrated to Holochain
 * - Agents, Attestations: Zomes exist, need wiring (TODO)
 * - Knowledge Maps, Extensions, Governance: Entry types not yet created (TODO)
 *
 * Reference data for implementing missing zomes:
 * /data/lamad/ contains JSON prototypes showing data structures
 */
@Injectable({ providedIn: 'root' })
export class DataLoaderService {
  // Caches to prevent redundant calls (shareReplay pattern)
  private readonly pathCache = new Map<string, Observable<LearningPath>>();
  private readonly contentCache = new Map<string, Observable<ContentNode>>();
  private attestationCache$: Observable<ContentAttestation[]> | null = null;
  private readonly attestationsByContentCache = new Map<string, ContentAttestation[]>();
  private graphCache$: Observable<ContentGraph> | null = null;
  private pathIndexCache$: Observable<PathIndex> | null = null;

  // NOTE: LRU cache logic removed - ContentService handles caching

  /** IndexedDB cache initialized flag */
  private idbInitialized = false;

  /** Projection API service for fast cached reads */
  private readonly projectionApi = inject(ProjectionAPIService);

  /** Content Resolver for unified tiered resolution */
  private readonly contentResolver = inject(ContentResolverService);

  /** Content Service for doorway-based content operations (new pattern) */
  private readonly contentService = inject(ContentService);

  /** Structured logger */
  private readonly logger = inject(LoggerService).createChild('DataLoader');

  private readonly governance = inject(GOVERNANCE);
  private readonly attestation = inject(CONTENT_ATTESTATION);
  private readonly idbCache = inject(IndexedDBCacheService);

  constructor() {
    // Defer cache initialization until after first render to avoid async in constructor.
    // Conductor is only used for agent-centric data (identity, attestations, points).
    afterNextRender(() => void this.initCaches());
  }

  /**
   * Initialize caches and content resolver.
   * Non-blocking - app continues to work without persistent cache if it fails.
   */
  private async initCaches(): Promise<void> {
    try {
      // Initialize IndexedDB
      this.idbInitialized = await this.idbCache.init();
      if (this.idbInitialized) {
        const stats = await this.idbCache.getStats();
        this.logger.debug('IndexedDB cache initialized', stats);
      }

      // Initialize ContentResolver and register sources
      await this.contentResolver.initialize();
      this.contentResolver.registerStandardSource('indexeddb');
      if (this.projectionApi.enabled) {
        this.contentResolver.registerStandardSource('projection');
      }
      this.contentResolver.registerStandardSource('conductor');

      // Set source availability
      this.contentResolver.setSourceAvailable('indexeddb', this.idbInitialized);
      this.contentResolver.setSourceAvailable('projection', this.projectionApi.enabled);
      this.contentResolver.setSourceAvailable('conductor', false); // Conductor deprecated for content

      this.logger.debug('ContentResolver initialized with sources');
    } catch (err) {
      this.logger.warn('Cache initialization failed', {
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  // NOTE: PATH_TIMEOUT_MS removed - ContentService handles timeouts

  /**
   * Load a LearningPath (PathView) by ID.
   * Delegates to getContent() and parses via parsePathView().
   * Does NOT load the content for each step (lazy loading).
   *
   * Read path: getContent() (projection → ContentService fallback) → parsePathView.
   */
  getPath(pathId: string): Observable<LearningPath> {
    const cached = this.pathCache.get(pathId);
    if (cached) {
      return cached;
    }

    const path$ = this.getContent(pathId).pipe(
      map(node => {
        if (!node || node.contentType === 'placeholder') {
          // Distinguish true 404 from network-error placeholders:
          // Placeholders from network errors contain the original error in description
          const isNetworkError = node?.description?.includes('Error') ||
            node?.description?.includes('timeout') ||
            node?.description?.includes('could not be loaded');
          if (isNetworkError) {
            throw new Error(`Path load failed: ${pathId}`);
          }
          throw new Error(`Path not found: ${pathId}`);
        }
        return parsePathView(node);
      }),
      tap(path => {
        // Store in IndexedDB cache for offline persistence (background, non-blocking)
        if (this.idbInitialized) {
          this.idbCache.setPath(path).catch(() => {
            // Silently ignore IndexedDB errors - caching is a performance optimization
            // and should not block path loading if storage fails
          });
        }
      }),
      catchError((err: unknown) => {
        const errMsg = err instanceof Error ? err.message : String(err);

        // "Not found" is a data issue, not connectivity — don't try cache
        if (errMsg.includes('Path not found')) {
          this.logger.warn('Path not found (may be stale reference)', { pathId });
          throw err;
        }

        this.logger.warn('Error loading path, trying IDB cache', { pathId, error: errMsg });

        if (!this.idbInitialized) {
          throw err;
        }

        return from(this.idbCache.getPath(pathId)).pipe(
          map(cached => {
            if (cached) {
              this.logger.info('Served path from IDB cache (offline fallback)', { pathId });
              return cached;
            }
            this.logger.debug('IDB cache miss for path', { pathId });
            throw err;
          }),
          catchError(() => {
            throw err;
          })
        );
      }),
      shareReplay(1),
      finalize(() => this.pathCache.delete(pathId))
    );

    this.pathCache.set(pathId, path$);
    return path$;
  }

  /**
   * Load a lightweight path overview.
   *
   * Use this for:
   * - Path listings (faster than loading full paths)
   * - Initial navigation (load overview first, then full path on demand)
   * - Any UI that only needs metadata + step count
   *
   * Cache hierarchy:
   * 1. Projection API (Doorway's MongoDB cache - fastest)
   * 2. Holochain REST API (15 minute TTL cache)
   *
   * @param pathId The path ID to load
   * @returns Observable of lightweight LearningPath (steps array will be empty)
   */
  getPathOverview(pathId: string): Observable<LearningPath> {
    // Paths are now ContentNodes — delegate to getPath() which uses getContent() + parsePathView
    return this.getPath(pathId);
  }

  /**
   * Transform path overview to LearningPath model.
   * Returns path with empty steps array - use getPath() for full steps.
   */
  /**
   * Load a ContentNode by ID.
   *
   * Read path: Projection API (fast, aggregated across all peers) → ContentService fallback.
   * Writes still go to /db/ via ContentService.
   *
   * IMPORTANT: Returns a placeholder node instead of throwing for missing content.
   * This prevents one missing item from breaking entire path loading.
   */
  getContent(resourceId: string): Observable<ContentNode> {
    // Primary: projection cache (fast, serves commons content from all peers)
    // Fallback: ContentService (direct storage, includes blob resolution)
    const source$ = this.projectionApi.enabled
      ? this.projectionApi.getContentNode(resourceId).pipe(
          timeout(3000),
          switchMap(content => content
            ? of(content)
            : this.contentService.getContent(resourceId)
          ),
          catchError(() => this.contentService.getContent(resourceId))
        )
      : this.contentService.getContent(resourceId);

    return source$.pipe(
      map(content => {
        if (!content) {
          this.logger.warn('Content not found, returning placeholder', { resourceId });
          return this.createPlaceholderContent(resourceId);
        }
        return content;
      }),
      tap(content => {
        // Store in IndexedDB cache for offline persistence (background, non-blocking)
        if (this.idbInitialized && content.contentType !== 'placeholder') {
          this.idbCache.setContent(content).catch(() => {
            // Silently ignore IndexedDB errors - caching is a performance optimization
            // and should not block content loading if storage fails
          });
        }
      }),
      catchError((err: unknown) => {
        const errMsg = err instanceof Error ? err.message : String(err);
        this.logger.warn('Error loading content, trying IDB cache', { resourceId, error: errMsg });

        if (!this.idbInitialized) {
          return of(this.createPlaceholderContent(resourceId, errMsg));
        }

        return from(this.idbCache.getContent(resourceId)).pipe(
          map(cached => {
            if (cached) {
              this.logger.info('Served content from IDB cache (offline fallback)', { resourceId });
              return cached;
            }
            this.logger.debug('IDB cache miss for content', { resourceId });
            return this.createPlaceholderContent(resourceId, errMsg);
          }),
          catchError(() => of(this.createPlaceholderContent(resourceId, errMsg)))
        );
      })
    );
  }

  /**
   * Batch load multiple content items efficiently.
   * Uses ContentService which routes to doorway (browser) or local storage (Tauri).
   *
   * @param resourceIds Array of content IDs to load
   * @returns Observable of Map<id, ContentNode>
   */
  batchGetContent(resourceIds: string[]): Observable<Map<string, ContentNode>> {
    if (resourceIds.length === 0) {
      return of(new Map());
    }

    return this.contentService.batchGetContent(resourceIds).pipe(
      tap(contentMap => {
        // Store in IndexedDB cache for offline persistence (background, non-blocking)
        if (this.idbInitialized && contentMap.size > 0) {
          const toCache = Array.from(contentMap.values()).filter(
            c => c.contentType !== 'placeholder'
          );
          if (toCache.length > 0) {
            this.idbCache.setContentBatch(toCache).catch(() => {
              // Silently ignore IndexedDB errors - batch caching is a performance optimization
              // and should not block batch content loading if storage fails
            });
          }
        }
      }),
      map(contentMap => {
        // Add placeholders for any IDs not found
        for (const id of resourceIds) {
          if (!contentMap.has(id)) {
            contentMap.set(id, this.createPlaceholderContent(id));
          }
        }
        return contentMap;
      }),
      catchError((err: unknown) => {
        const errMsg = err instanceof Error ? err.message : String(err);
        this.logger.warn('Batch load error, trying IDB cache', {
          count: resourceIds.length,
          error: errMsg,
        });

        if (!this.idbInitialized) {
          const contentMap = new Map<string, ContentNode>();
          for (const id of resourceIds) {
            contentMap.set(id, this.createPlaceholderContent(id, errMsg));
          }
          return of(contentMap);
        }

        return from(this.idbCache.getContentBatch(resourceIds)).pipe(
          map(cachedMap => {
            const resultMap = new Map<string, ContentNode>(cachedMap);
            const cacheHits = cachedMap.size;
            const cacheMisses = resourceIds.length - cacheHits;

            this.logger.info('Batch IDB cache fallback', { cacheHits, cacheMisses });

            for (const id of resourceIds) {
              if (!resultMap.has(id)) {
                resultMap.set(id, this.createPlaceholderContent(id, errMsg));
              }
            }
            return resultMap;
          }),
          catchError(() => {
            const contentMap = new Map<string, ContentNode>();
            for (const id of resourceIds) {
              contentMap.set(id, this.createPlaceholderContent(id, errMsg));
            }
            return of(contentMap);
          })
        );
      })
    );
  }

  /**
   * Prefetch content for upcoming path steps.
   *
   * Call this when user starts a path to preload the first few steps,
   * or when navigating to prefetch upcoming content.
   *
   * @param resourceIds Content IDs to prefetch
   * @param prefetchCount Number of items to prefetch (default 3)
   */
  prefetchContent(resourceIds: string[], prefetchCount = 3): void {
    // Filter to uncached IDs
    const uncachedIds = resourceIds
      .filter(id => !this.contentCache.has(id))
      .slice(0, prefetchCount);

    if (uncachedIds.length === 0) {
      return;
    }

    // Prefetch via content resolver (projection tier)
    for (const id of uncachedIds) {
      this.contentResolver.resolveContent(id).catch(() => undefined);
    }
  }

  /**
   * Load path with prefetching of initial step content.
   *
   * Enhanced version of getPath that also prefetches the first few steps.
   */
  getPathWithPrefetch(pathId: string, prefetchSteps = 3): Observable<LearningPath> {
    return this.getPath(pathId).pipe(
      tap(path => {
        // Prefetch first N step content in background
        const stepResourceIds = path.steps.slice(0, prefetchSteps).map(s => s.resourceId);
        this.prefetchContent(stepResourceIds, prefetchSteps);
      })
    );
  }

  /**
   * Create a placeholder content node for missing/errored content.
   * This allows the UI to continue functioning and show useful feedback.
   */
  private createPlaceholderContent(resourceId: string, errorMessage?: string): ContentNode {
    return {
      id: resourceId,
      contentType: 'placeholder',
      title: `Content Not Found: ${resourceId}`,
      description: errorMessage ?? `The content "${resourceId}" could not be loaded.`,
      content:
        `This content is not yet available. It may not have been seeded or there was an error loading it.\n\nResource ID: ${resourceId}` +
        (errorMessage ? '\nError: ' + errorMessage : ''),
      contentFormat: 'markdown',
      tags: ['missing', 'placeholder'],
      relatedNodeIds: [],
      metadata: {},
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  }

  /** Cached content index for search/discovery */
  private contentIndexCache$: Observable<ContentIndex> | null = null;

  /** Cached readiness check */
  private readinessCache$: Observable<boolean> | null = null;

  /**
   * Lightweight readiness check for data availability.
   * Use this instead of getContentIndex() when you just need to know if data is loadable.
   * Much faster than loading 1000+ content items.
   *
   * @returns Observable<boolean> - true if data layer is ready
   */
  checkReadiness(): Observable<boolean> {
    this.readinessCache$ ??= this.contentService.queryContent({ limit: 1 }).pipe(
      timeout(5000),
      map(() => true),
      catchError(err => {
        this.logger.warn('Readiness check failed', err);
        // Clear cache so next call retries
        this.readinessCache$ = null;
        return of(false);
      }),
      shareReplay(1)
    );
    return this.readinessCache$ ?? of(false);
  }

  /**
   * Invalidate the readiness cache.
   * Call this if connection state changes.
   */
  invalidateReadinessCache(): void {
    this.readinessCache$ = null;
  }

  /**
   * Load the content index for search/discovery.
   * Returns metadata only, not full content.
   * Uses ContentService (doorway) as the source.
   * Cached with shareReplay(1) to prevent redundant calls.
   *
   * NOTE: This is a heavy operation (loads up to 1000 items).
   * Use checkReadiness() if you just need to verify data is available.
   */
  getContentIndex(): Observable<ContentIndex> {
    this.contentIndexCache$ ??= this.contentService.queryContent({ limit: 1000 }).pipe(
      map(nodes => ({
        nodes: nodes.map(node => ({
          id: node.id,
          title: node.title,
          description: node.description ?? '',
          contentType: node.contentType,
          tags: node.tags ?? [],
          reach: node.reach ?? 'commons',
          trustScore: node.trustScore,
          createdAt: node.createdAt,
          updatedAt: node.updatedAt,
        })),
        totalCount: nodes.length,
        byType: this.groupByType(nodes),
        lastUpdated: new Date().toISOString(),
      })),
      shareReplay(1),
      catchError(err => {
        this.logger.error('Failed to load content index', err);
        // Clear cache on error so next call retries
        this.contentIndexCache$ = null;
        return of({
          nodes: [],
          totalCount: 0,
          byType: {},
          lastUpdated: new Date().toISOString(),
        });
      })
    );
    return (
      this.contentIndexCache$ ??
      of({
        nodes: [],
        totalCount: 0,
        byType: {},
        lastUpdated: new Date().toISOString(),
      })
    );
  }

  /**
   * Group content nodes by type for index statistics.
   */
  private groupByType(nodes: ContentNode[]): Record<string, number> {
    const byType: Record<string, number> = {};
    for (const node of nodes) {
      byType[node.contentType] = (byType[node.contentType] ?? 0) + 1;
    }
    return byType;
  }

  /**
   * Invalidate the content index cache.
   * Call this after creating/updating/deleting content.
   */
  invalidateContentIndexCache(): void {
    this.contentIndexCache$ = null;
  }

  /**
   * Load the path index for discovery.
   * Queries content by type 'path', parses each into PathView, builds PathIndex.
   * Cached with shareReplay(1) to prevent redundant calls.
   */
  getPathIndex(): Observable<PathIndex> {
    this.pathIndexCache$ ??= this.contentService.queryContent({ contentType: 'path' }).pipe(
      map(nodes => this.transformContentNodesToPathIndex(nodes)),
      shareReplay(1),
      catchError(err => {
        this.logger.error('Failed to load path index', err);
        // Clear cache on error so next call retries
        this.pathIndexCache$ = null;
        return of({ paths: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
    return (
      this.pathIndexCache$ ??
      of({ paths: [], totalCount: 0, lastUpdated: new Date().toISOString() })
    );
  }

  /**
   * Transform ContentNode[] (type=path) to PathIndex model.
   */
  private transformContentNodesToPathIndex(nodes: ContentNode[]): PathIndex {
    const entries: PathIndexEntry[] = nodes.map(node => {
      const parsed = parsePathView(node);
      return {
        id: parsed.id,
        title: parsed.title,
        description: parsed.description ?? '',
        difficulty: (parsed.difficulty as PathIndexEntry['difficulty']) ?? 'beginner',
        estimatedDuration: parsed.estimatedDuration ?? '',
        stepCount: parsed.steps.length,
        tags: parsed.tags ?? [],
        thumbnailUrl: parsed.thumbnailUrl,
        thumbnailAlt: parsed.thumbnailAlt,
        chapterCount: parsed.chapters?.length,
        pathType: parsed.pathType as PathIndexEntry['pathType'],
        attestationsGranted: parsed.attestationsGranted,
      };
    });
    return {
      lastUpdated: new Date().toISOString(),
      totalCount: entries.length,
      paths: entries,
    };
  }

  /**
   * Invalidate the path index cache.
   * Call this after creating/updating/deleting paths.
   */
  invalidatePathIndexCache(): void {
    this.pathIndexCache$ = null;
  }

  /**
   * Load agent profile from Holochain.
   */
  // TODO: Create AgentApiService (thin HTTP client) to replace direct zome calls
  getAgent(_agentId: string): Observable<Agent | null> {
    return of(null);
  }

  /**
   * Load agent progress for a specific path.
   * Reads from IndexedDB first, falls back to localStorage, migrates on read.
   */
  getAgentProgress(agentId: string, pathId: string): Observable<AgentProgress | null> {
    if (!this.idbInitialized) {
      return of(this.getLocalProgress(agentId, pathId));
    }

    const idbKey = `progress-${agentId}-${pathId}`;
    return from(this.idbCache.getMetadata<AgentProgress>(idbKey)).pipe(
      map(idbProgress => {
        if (idbProgress) return idbProgress;

        // Fall back to localStorage and migrate if found
        const localProgress = this.getLocalProgress(agentId, pathId);
        if (localProgress) {
          // Migrate: write to IDB and remove from localStorage
          const lsKey = `lamad-progress-${agentId}-${pathId}`;
          this.idbCache
            .setMetadata(idbKey, localProgress)
            .then(() => {
              try {
                localStorage.removeItem(lsKey);
              } catch {
                /* ignore */
              }
              this.logger.debug('Migrated progress to IDB', { agentId, pathId });
            })
            .catch(() => undefined);
        }
        return localProgress;
      }),
      catchError(() => of(this.getLocalProgress(agentId, pathId)))
    );
  }

  /**
   * Save agent progress.
   * Dual-writes to IndexedDB (primary) and localStorage (fallback).
   */
  saveAgentProgress(progress: AgentProgress): Observable<void> {
    const lsKey = `lamad-progress-${progress.agentId}-${progress.pathId}`;

    // Always write to localStorage as fallback
    try {
      localStorage.setItem(lsKey, JSON.stringify(progress));
    } catch {
      // Silently ignore localStorage quota errors
    }

    // Write to IDB if available (non-blocking)
    if (this.idbInitialized) {
      const idbKey = `progress-${progress.agentId}-${progress.pathId}`;
      this.idbCache.setMetadata(idbKey, progress).catch(() => undefined);
    }

    return of(undefined);
  }

  /**
   * Load progress from localStorage (fallback).
   */
  getLocalProgress(agentId: string, pathId: string): AgentProgress | null {
    const key = `lamad-progress-${agentId}-${pathId}`;
    const data = localStorage.getItem(key);
    if (data) {
      try {
        return JSON.parse(data) as AgentProgress;
      } catch {
        return null;
      }
    }
    return null;
  }

  /**
   * Clear all caches - useful for testing or after auth changes.
   *
   * @param includeIndexedDB If true, also clears IndexedDB persistent cache
   */
  clearCache(includeIndexedDB = false): void {
    this.pathCache.clear();
    this.contentCache.clear();
    this.attestationCache$ = null;
    this.attestationsByContentCache.clear();
    this.graphCache$ = null;
    this.relationshipByNodeCache.clear();
    this.pathIndexCache$ = null;
    this.contentIndexCache$ = null;
    // Note: Holochain content cache removed — content served from projection tier

    // Optionally clear IndexedDB persistent cache
    if (includeIndexedDB && this.idbInitialized) {
      this.idbCache.clearAll().catch(() => {
        // Silently ignore IndexedDB clear errors - this is a best-effort cleanup operation
        // and should not prevent cache clearing from completing
      });
    }
  }

  /**
   * Clear only the IndexedDB persistent cache.
   * Useful when data schema changes or to force fresh data.
   */
  async clearPersistentCache(): Promise<void> {
    if (this.idbInitialized) {
      await this.idbCache.clearAll();
    }
  }

  /**
   * Get cache statistics for debugging/monitoring.
   */
  getCacheStats(): {
    pathCacheSize: number;
    contentCacheSize: number;
    relationshipCacheSize: number;
    hasGraph: boolean;
    hasPathIndex: boolean;
    hasContentIndex: boolean;
    indexedDBAvailable: boolean;
  } {
    return {
      pathCacheSize: this.pathCache.size,
      contentCacheSize: this.contentCache.size,
      relationshipCacheSize: this.relationshipByNodeCache.size,
      hasGraph: this.graphCache$ !== null,
      hasPathIndex: this.pathIndexCache$ !== null,
      hasContentIndex: this.contentIndexCache$ !== null,
      indexedDBAvailable: this.idbInitialized,
    };
  }

  /**
   * Get detailed cache statistics including IndexedDB.
   */
  async getDetailedCacheStats(): Promise<{
    memory: {
      pathCacheSize: number;
      contentCacheSize: number;
      relationshipCacheSize: number;
    };
    indexedDB: {
      available: boolean;
      contentCount: number;
      pathCount: number;
    };
  }> {
    const idbStats = this.idbInitialized
      ? await this.idbCache.getStats()
      : { contentCount: 0, pathCount: 0, isAvailable: false };

    return {
      memory: {
        pathCacheSize: this.pathCache.size,
        contentCacheSize: this.contentCache.size,
        relationshipCacheSize: this.relationshipByNodeCache.size,
      },
      indexedDB: {
        available: idbStats.isAvailable,
        contentCount: idbStats.contentCount,
        pathCount: idbStats.pathCount,
      },
    };
  }

  // =========================================================================
  // Attestation Loading (Bidirectional Trust Model)
  // =========================================================================

  /**
   * Load all content attestations.
   *
   * ContentAttestations (trust claims about content) are different from
   * Agent Attestations (credentials/achievements).
   *
   * Use getAgentAttestations() for agent credentials.
   */
  getAttestations(): Observable<ContentAttestation[]> {
    // TODO: IContentAttestation doesn't yet support queryAll with status filter.
    // For now, return empty. Wire up when attestation list endpoint is added.
    this.attestationCache$ ??= of([]);
    return this.attestationCache$;
  }

  /**
   * Load agent attestations (credentials/achievements) from Holochain.
   *
   * These are different from content attestations - they represent
   * achievements earned by agents (domain-mastery, path-completion, etc.)
   */
  // TODO: Create AgentAttestationApiService for agent credential queries
  getAgentAttestations(_agentId?: string, _category?: string): Observable<AgentAttestation[]> {
    return of([]);
  }


  /**
   * Get attestations for a specific content node.
   * Uses dedicated Holochain query for efficiency.
   */
  getAttestationsForContent(contentId: string): Observable<ContentAttestation[]> {
    // Check local cache first
    if (this.attestationsByContentCache.has(contentId)) {
      return of(this.attestationsByContentCache.get(contentId)!);
    }

    return defer(() => from(this.attestation.queryAttestationsForContent(contentId))).pipe(
      map(results => {
        const attestations: ContentAttestation[] = results.map(r => ({
          id: r.id,
          contentId: r.contentId,
          attestationType: r.attestationType as ContentAttestation['attestationType'],
          reachGranted: (r as Record<string, unknown>)['reachGranted'] as ContentAttestation['reachGranted'] ?? 'commons',
          grantedBy: (r as Record<string, unknown>)['grantedBy'] as ContentAttestation['grantedBy'] ?? { type: 'system', grantorId: 'unknown' },
          grantedAt: r.createdAt,
          status: (r.isRevoked ? 'revoked' : 'active') as ContentAttestation['status'],
          metadata: {} as ContentAttestation['metadata'],
        }));
        this.attestationsByContentCache.set(contentId, attestations);
        return attestations;
      }),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get all active attestations (not revoked or expired).
   */
  getActiveAttestations(): Observable<ContentAttestation[]> {
    return this.getAttestations().pipe(
      map(attestations => attestations.filter(att => att.status === 'active'))
    );
  }

  /**
   * Load the agent index (all known agents) from Holochain.
   */
  // TODO: Create AgentApiService (thin HTTP client) to replace direct zome calls
  getAgentIndex(): Observable<{ agents: Agent[] }> {
    return of({ agents: [] }).pipe(
      catchError(_err => {
        return of({ agents: [] });
      })
    );
  }

  // =========================================================================
  // Knowledge Map Loading
  // =========================================================================

  /**
   * Load the knowledge map index from content service.
   */
  getKnowledgeMapIndex(): Observable<KnowledgeMapIndex> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of({ maps: [], totalCount: 0, lastUpdated: new Date().toISOString() });
    }

    return this.contentService.queryKnowledgeMaps({}).pipe(
      map(results => ({
        lastUpdated: new Date().toISOString(),
        totalCount: results.length,
        maps: results.map(km => this.transformKnowledgeMapToIndex(km)),
      })),
      catchError(_err => {
        return of({ maps: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load a specific knowledge map from content service.
   */
  getKnowledgeMap(mapId: string): Observable<KnowledgeMap | null> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of(null);
    }

    return this.contentService.getKnowledgeMap(mapId).pipe(
      catchError(_err => {
        return of(null);
      })
    );
  }

  /**
   * Transform KnowledgeMap to KnowledgeMapIndexEntry.
   */
  private transformKnowledgeMapToIndex(map: KnowledgeMap): KnowledgeMapIndexEntry {
    const subjectName =
      'subject' in map
        ? map.subject.subjectName
        : (((map as unknown as Record<string, unknown>)['subjectName'] as string) ?? '');
    return {
      id: map.id,
      mapType: map.mapType,
      title: map.title,
      subjectName,
      ownerId: map.ownerId,
      ownerName: '', // Would need to look up agent name
      visibility: map.visibility,
      overallAffinity: map.overallAffinity,
      nodeCount: map.nodes.length,
      updatedAt: map.updatedAt,
    };
  }

  /**
   * Transform Holochain knowledge map entry to KnowledgeMapIndexEntry.
   */
  private transformHolochainKnowledgeMapToIndex(hcMap: {
    id: string;
    mapType: string;
    ownerId: string;
    title: string;
    subjectType: string;
    subjectId: string;
    subjectName: string;
    visibility: string;
    nodes: unknown;
    overallAffinity: number;
    updatedAt: string;
  }): KnowledgeMapIndexEntry {
    const nodes = Array.isArray(hcMap.nodes) ? hcMap.nodes : [];

    return {
      id: hcMap.id,
      mapType: hcMap.mapType as KnowledgeMapType,
      title: hcMap.title,
      subjectName: hcMap.subjectName,
      ownerId: hcMap.ownerId,
      ownerName: '', // Would need to look up agent name
      visibility: hcMap.visibility,
      overallAffinity: hcMap.overallAffinity,
      nodeCount: Array.isArray(nodes) ? nodes.length : 0,
      updatedAt: hcMap.updatedAt,
    };
  }

  /**
   * Transform Holochain knowledge map entry to full KnowledgeMap model.
   */
  private transformHolochainKnowledgeMap(hcMap: {
    id: string;
    mapType: string;
    ownerId: string;
    title: string;
    description: string | null;
    subjectType: string;
    subjectId: string;
    subjectName: string;
    visibility: string;
    sharedWith: unknown;
    nodes: unknown;
    pathIds: unknown;
    overallAffinity: number;
    contentGraphId: string | null;
    masteryLevels: unknown;
    goals: unknown;
    createdAt: string;
    updatedAt: string;
    metadata: unknown;
  }): KnowledgeMap {
    const nodes = (Array.isArray(hcMap.nodes) ? hcMap.nodes : []) as KnowledgeNode[];
    const pathIds = (Array.isArray(hcMap.pathIds) ? hcMap.pathIds : []) as string[];
    const sharedWith = (Array.isArray(hcMap.sharedWith) ? hcMap.sharedWith : []) as string[];
    const metadata = (
      hcMap.metadata && typeof hcMap.metadata === 'object' ? hcMap.metadata : {}
    ) as Record<string, unknown>;

    return {
      id: hcMap.id,
      mapType: hcMap.mapType as KnowledgeMapType,
      subject: {
        type: hcMap.subjectType as 'content-graph' | 'agent' | 'organization',
        subjectId: hcMap.subjectId,
        subjectName: hcMap.subjectName,
      },
      ownerId: hcMap.ownerId,
      title: hcMap.title,
      description: hcMap.description ?? undefined,
      visibility: hcMap.visibility as 'private' | 'mutual' | 'shared' | 'public',
      sharedWith,
      nodes,
      pathIds,
      overallAffinity: hcMap.overallAffinity,
      createdAt: hcMap.createdAt,
      updatedAt: hcMap.updatedAt,
      metadata,
    };
  }

  // =========================================================================
  // Path Extension Loading
  // =========================================================================

  /**
   * Load the path extension index from Holochain.
   */
  getPathExtensionIndex(): Observable<PathExtensionIndex> {
    return this.contentService.queryPathExtensions({}).pipe(
      map(results => ({
        lastUpdated: new Date().toISOString(),
        totalCount: results.length,
        extensions: results.map(r => this.transformPathExtensionToIndex(r)),
      })),
      catchError(_err => {
        return of({ extensions: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load a specific path extension from content service.
   */
  getPathExtension(extensionId: string): Observable<PathExtension | null> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of(null);
    }

    return this.contentService.getPathExtension(extensionId).pipe(
      catchError(_err => {
        return of(null);
      })
    );
  }

  /**
   * Get extensions for a specific base path.
   */
  getExtensionsForPath(pathId: string): Observable<PathExtension[]> {
    return this.contentService.queryPathExtensions({ basePathId: pathId }).pipe(
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Transform PathExtension to PathExtensionIndexEntry.
   */
  private transformPathExtensionToIndex(ext: PathExtension): PathExtensionIndexEntry {
    return {
      id: ext.id,
      basePathId: ext.basePathId,
      basePathTitle: '', // Would need to look up path title
      title: ext.title,
      description: ext.description,
      extendedBy: ext.extendedBy,
      extenderName: '', // Would need to look up agent name
      visibility: ext.visibility,
      insertionCount: ext.insertions?.length ?? 0,
      annotationCount: ext.annotations?.length ?? 0,
      forkCount: ext.forks?.length ?? 0,
      updatedAt: ext.updatedAt,
    };
  }

  /**
   * Transform Holochain path extension entry to PathExtensionIndexEntry.
   * @deprecated Use transformPathExtensionToIndex with ContentService data instead
   */
  private transformHolochainPathExtensionToIndex(hcExt: {
    id: string;
    basePathId: string;
    basePathVersion: string;
    extendedBy: string;
    title: string;
    description: string | null;
    visibility: string;
    insertions: unknown;
    annotations: unknown;
    updatedAt: string;
  }): PathExtensionIndexEntry {
    const insertions = Array.isArray(hcExt.insertions) ? hcExt.insertions : [];
    const annotations = Array.isArray(hcExt.annotations) ? hcExt.annotations : [];
    const insertionCount = insertions.length;
    const annotationCount = annotations.length;

    return {
      id: hcExt.id,
      basePathId: hcExt.basePathId,
      basePathTitle: '', // Would need to look up path title
      title: hcExt.title,
      description: hcExt.description ?? undefined,
      extendedBy: hcExt.extendedBy,
      extenderName: '', // Would need to look up agent name
      visibility: hcExt.visibility,
      insertionCount,
      annotationCount,
      forkCount: 0, // Would need separate query
      updatedAt: hcExt.updatedAt,
    };
  }

  /**
   * Transform Holochain path extension entry to full PathExtension model.
   */
  private transformHolochainPathExtension(hcExt: {
    id: string;
    basePathId: string;
    basePathVersion: string;
    extendedBy: string;
    title: string;
    description: string | null;
    visibility: string;
    sharedWith: unknown;
    insertions: unknown;
    annotations: unknown;
    reorderings: unknown;
    exclusions: unknown;
    forkedFrom: string | null;
    forks: unknown;
    upstreamProposal: unknown;
    stats: unknown;
    createdAt: string;
    updatedAt: string;
  }): PathExtension {
    const sharedWith = (Array.isArray(hcExt.sharedWith) ? hcExt.sharedWith : []) as string[];
    const insertions = (
      Array.isArray(hcExt.insertions) ? hcExt.insertions : []
    ) as PathStepInsertion[];
    const annotations = (
      Array.isArray(hcExt.annotations) ? hcExt.annotations : []
    ) as PathStepAnnotation[];
    const reorderings = (
      Array.isArray(hcExt.reorderings) ? hcExt.reorderings : []
    ) as PathStepReorder[];
    const exclusions = (
      Array.isArray(hcExt.exclusions) ? hcExt.exclusions : []
    ) as PathStepExclusion[];
    const forks = (Array.isArray(hcExt.forks) ? hcExt.forks : []) as string[];
    const upstreamProposal = (hcExt.upstreamProposal ?? undefined) as UpstreamProposal | undefined;
    const stats = (hcExt.stats ?? undefined) as ExtensionStats | undefined;

    return {
      id: hcExt.id,
      basePathId: hcExt.basePathId,
      basePathVersion: hcExt.basePathVersion,
      extendedBy: hcExt.extendedBy,
      title: hcExt.title,
      description: hcExt.description ?? undefined,
      insertions,
      annotations,
      reorderings,
      exclusions,
      visibility: hcExt.visibility as 'private' | 'shared' | 'public',
      sharedWith,
      forkedFrom: hcExt.forkedFrom ?? undefined,
      forks,
      upstreamProposal,
      stats,
      createdAt: hcExt.createdAt,
      updatedAt: hcExt.updatedAt,
    };
  }

  // =========================================================================
  // Graph Loading (for Exploration Service)
  // =========================================================================

  /** LRU cache for per-node relationship queries */
  private readonly relationshipByNodeCache = new Map<string, Observable<ContentRelationship[]>>();
  private readonly RELATIONSHIP_CACHE_MAX_SIZE = 100;

  /**
   * Load the full content graph for exploration.
   *
   * When Holochain is available, fetches relationships and builds the graph.
   * Falls back to empty graph if unavailable.
   *
   * Note: Prefer getRelationshipsForNode() for single-node queries to avoid
   * loading the entire graph.
   */
  getGraph(): Observable<ContentGraph> {
    if (!this.graphCache$) {
      if (true) { // Content graph served from projection tier
        // Build graph from Holochain relationships
        this.graphCache$ = this.buildGraphFromHolochain().pipe(
          shareReplay(1),
          catchError(_err => {
            return of(this.createEmptyGraph());
          })
        );
      } else {
        this.graphCache$ = of(this.createEmptyGraph());
      }
    }
    return this.graphCache$;
  }

  /**
   * Get relationships for a single node (lazy loading).
   *
   * This is more efficient than loading the full graph when you only need
   * relationships for one content node. Uses caching to prevent redundant calls.
   *
   * @param contentId - The content node ID to get relationships for
   * @param direction - 'outgoing', 'incoming', or 'both'
   * @returns Observable of ContentRelationship[]
   */
  getRelationshipsForNode(
    contentId: string,
    direction: 'outgoing' | 'incoming' | 'both' = 'both'
  ): Observable<ContentRelationship[]> {
    const cacheKey = `${contentId}:${direction}`;

    if (!this.relationshipByNodeCache.has(cacheKey)) {
      // Evict oldest entries if cache is too large
      if (this.relationshipByNodeCache.size >= this.RELATIONSHIP_CACHE_MAX_SIZE) {
        const firstKey = this.relationshipByNodeCache.keys().next().value;
        if (firstKey) {
          this.relationshipByNodeCache.delete(firstKey);
        }
      }

      const request = this.fetchRelationshipsForNode(contentId, direction).pipe(
        shareReplay(1),
        catchError(_err => {
          // Remove from cache on error
          this.relationshipByNodeCache.delete(cacheKey);
          return of([]);
        })
      );

      this.relationshipByNodeCache.set(cacheKey, request);
    }

    return this.relationshipByNodeCache.get(cacheKey)!;
  }

  /**
   * Fetch relationships for a node from Holochain.
   */
  private fetchRelationshipsForNode(
    contentId: string,
    direction: 'outgoing' | 'incoming' | 'both'
  ): Observable<ContentRelationship[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return this.contentService
      .getRelationships(contentId, direction)
      .pipe(map(results => (results ?? []).map(r => this.transformToContentRelationship(r))));
  }

  /**
   * Transform ContentService Relationship to ContentRelationship model.
   */
  private transformToContentRelationship(rel: {
    id: string;
    sourceId: string;
    targetId: string;
    relationshipType: string;
    confidence?: number;
    metadata?: Record<string, unknown>;
  }): ContentRelationship {
    return {
      id: rel.id,
      sourceNodeId: rel.sourceId,
      targetNodeId: rel.targetId,
      relationshipType: rel.relationshipType as ContentRelationshipType,
      metadata: rel.metadata,
    };
  }

  /**
   * Transform Holochain relationship entry to frontend ContentRelationship model.
   */
  private transformHolochainRelationship(hcRel: {
    id: string;
    sourceId: string;
    targetId: string;
    relationshipType: string;
    confidence: number;
    metadata: unknown;
  }): ContentRelationship {
    const metadata = (
      hcRel.metadata && typeof hcRel.metadata === 'object' ? hcRel.metadata : {}
    ) as Record<string, unknown>;

    // Intentionally untyped: relationship metadata, not domain content metadata.
    // ContentRelationship.metadata is Record<string, unknown> by design.
    metadata['confidence'] = hcRel.confidence;

    return {
      id: hcRel.id,
      sourceNodeId: hcRel.sourceId,
      targetNodeId: hcRel.targetId,
      relationshipType: hcRel.relationshipType as ContentRelationshipType,
      metadata,
    };
  }

  /**
   * Invalidate the relationship cache for a specific node.
   * Call this after creating/updating relationships.
   */
  invalidateRelationshipCache(contentId?: string): void {
    if (contentId) {
      // Remove all cache entries for this node
      for (const key of this.relationshipByNodeCache.keys()) {
        if (key.startsWith(`${contentId}:`)) {
          this.relationshipByNodeCache.delete(key);
        }
      }
    } else {
      // Clear entire cache
      this.relationshipByNodeCache.clear();
    }
  }

  /**
   * Build ContentGraph from content service.
   */
  private buildGraphFromHolochain(): Observable<ContentGraph> {
    // Get content graph starting from manifesto root
    return this.contentService.getContentGraph('manifesto').pipe(
      map(graph => {
        if (!graph) {
          return this.createEmptyGraph();
        }
        // Transform the simplified graph structure to the full ContentGraph model
        return this.transformSimplifiedGraph(graph);
      }),
      catchError(_err => {
        return of(this.createEmptyGraph());
      })
    );
  }

  /**
   * Transform simplified ContentGraph from ContentService to full model.
   */
  private transformSimplifiedGraph(simpleGraph: {
    rootId: string;
    related: {
      contentId: string;
      relationshipType: string;
      confidence: number;
      children: GraphNodeData[];
    }[];
    totalNodes: number;
  }): ContentGraph {
    const nodes = new Map<string, ContentNode>();
    const relationships = new Map<string, ContentRelationship>();
    const nodesByType = new Map<string, Set<string>>();
    const nodesByTag = new Map<string, Set<string>>();
    const nodesByCategory = new Map<string, Set<string>>();
    const adjacency = new Map<string, Set<string>>();
    const reverseAdjacency = new Map<string, Set<string>>();

    // Note: The simplified graph only has IDs, not full content nodes.
    // For full node data, would need to batch fetch via batchGetContent.
    // For now, just build the relationship structure.

    const processNode = (
      nodeData: { contentId: string; relationshipType: string; children: GraphNodeData[] },
      parentId?: string
    ): void => {
      const nodeId = nodeData.contentId;

      // Add relationship from parent if exists
      if (parentId) {
        const relId = `${parentId}-${nodeId}`;
        relationships.set(relId, {
          id: relId,
          sourceNodeId: parentId,
          targetNodeId: nodeId,
          relationshipType: nodeData.relationshipType as ContentRelationshipType,
        });

        // Update adjacency
        if (!adjacency.has(parentId)) adjacency.set(parentId, new Set());
        adjacency.get(parentId)!.add(nodeId);

        if (!reverseAdjacency.has(nodeId)) reverseAdjacency.set(nodeId, new Set());
        reverseAdjacency.get(nodeId)!.add(parentId);
      }

      // Process children recursively
      if (nodeData.children && nodeData.children.length > 0) {
        for (const child of nodeData.children) {
          processNode(child, nodeId);
        }
      }
    };

    // Process all related nodes
    for (const related of simpleGraph.related) {
      processNode(related, simpleGraph.rootId);
    }

    return {
      nodes,
      relationships,
      nodesByType,
      nodesByTag,
      nodesByCategory,
      adjacency,
      reverseAdjacency,
      metadata: {
        nodeCount: simpleGraph.totalNodes,
        relationshipCount: relationships.size,
        lastUpdated: new Date().toISOString(),
        version: '1.0.0',
      },
    };
  }

  /**
   * Create empty ContentGraph for error fallback.
   */
  private createEmptyGraph(): ContentGraph {
    return {
      nodes: new Map<string, ContentNode>(),
      relationships: new Map<string, ContentRelationship>(),
      nodesByType: new Map<string, Set<string>>(),
      nodesByTag: new Map<string, Set<string>>(),
      nodesByCategory: new Map<string, Set<string>>(),
      adjacency: new Map<string, Set<string>>(),
      reverseAdjacency: new Map<string, Set<string>>(),
      metadata: {
        nodeCount: 0,
        relationshipCount: 0,
        lastUpdated: new Date().toISOString(),
        version: '1.0.0',
      },
    };
  }

  // =========================================================================
  // Assessment Loading
  // =========================================================================

  /**
   * Load the assessment index.
   * Builds from Content entries with assessment contentType.
   */
  getAssessmentIndex(): Observable<AssessmentIndex> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of({ assessments: [], totalCount: 0, lastUpdated: new Date().toISOString() });
    }

    return this.contentService.queryContent({ contentType: 'assessment', limit: 500 }).pipe(
      map(contentNodes => {
        const assessments: AssessmentIndexEntry[] = contentNodes.map(node => {
          const meta = node.metadata ?? {};
          return {
            id: node.id,
            title: node.title,
            domain: (meta['domain'] as string) ?? 'general',
            instrumentType: (meta['instrumentType'] as string) ?? 'questionnaire',
            estimatedTime: (meta.estimatedTime as string) ?? '15 minutes',
          };
        });

        return {
          assessments,
          totalCount: assessments.length,
          lastUpdated: new Date().toISOString(),
        };
      }),
      catchError(() =>
        of({ assessments: [], totalCount: 0, lastUpdated: new Date().toISOString() })
      )
    );
  }

  /**
   * Load a specific assessment instrument.
   * Assessments are also stored as content nodes, so this uses the content loader.
   */
  getAssessment(assessmentId: string): Observable<ContentNode | null> {
    return this.getContent(assessmentId).pipe(catchError(() => of(null)));
  }

  /**
   * Get assessments by domain (values, attachment, strengths, etc.).
   */
  getAssessmentsByDomain(domain: string): Observable<AssessmentIndexEntry[]> {
    return this.getAssessmentIndex().pipe(
      map(index => index.assessments.filter(a => a.domain === domain))
    );
  }

  // =========================================================================
  // Governance Loading
  // =========================================================================

  /**
   * Load the governance index (counts and metadata).
   * Aggregates counts from all governance entity types.
   */
  getGovernanceIndex(): Observable<GovernanceIndex> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of({
        lastUpdated: new Date().toISOString(),
        challengeCount: 0,
        proposalCount: 0,
        precedentCount: 0,
        discussionCount: 0,
      });
    }

    // Query all governance types in parallel via thin API
    // Note: thin API queryChallenges/etc require a contentId — use empty string for "all"
    return defer(async () =>
      Promise.all([
        this.governance.queryGovernanceStates('challenge'),
        this.governance.queryGovernanceStates('proposal'),
        this.governance.queryGovernanceStates('precedent'),
        this.governance.queryGovernanceStates('discussion'),
      ])
    ).pipe(
      map(([challenges, proposals, precedents, discussions]) => ({
        lastUpdated: new Date().toISOString(),
        challengeCount: challenges.length,
        proposalCount: proposals.length,
        precedentCount: precedents.length,
        discussionCount: discussions.length,
      })),
      catchError(_err => {
        return of({
          lastUpdated: new Date().toISOString(),
          challengeCount: 0,
          proposalCount: 0,
          precedentCount: 0,
          discussionCount: 0,
        });
      })
    );
  }

  /**
   * Load all challenges from Holochain.
   */
  getChallenges(): Observable<ChallengeRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() => from(this.governance.queryGovernanceStates('challenge'))).pipe(
      map(results => results.map(r => this.transformGovernanceStateToChallenge(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get challenges for a specific entity.
   */
  getChallengesForEntity(entityType: string, entityId: string): Observable<ChallengeRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() =>
      from(this.governance.queryChallenges(entityId))
    ).pipe(
      map(results => results.map(r => this.transformChallengeView(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Transform GovernanceStateView to ChallengeRecord (adapter for thin API).
   */
  private transformGovernanceStateToChallenge(state: GovernanceStateView): ChallengeRecord {
    return {
      id: state.id,
      entityType: state.entityType,
      entityId: state.entityId,
      challenger: { agentId: '', displayName: '', standing: '' },
      grounds: '',
      description: '',
      status: state.votingState,
      filedAt: state.createdAt,
    };
  }

  /**
   * Transform ChallengeView to ChallengeRecord.
   */
  private transformChallengeView(view: ChallengeView): ChallengeRecord {
    return {
      id: view.id,
      entityType: view.entityType,
      entityId: view.entityId,
      challenger: { agentId: view.challengerId, displayName: '', standing: '' },
      grounds: view.groundsPrimary,
      description: view.groundsPrimary,
      status: view.state,
      filedAt: view.createdAt,
    };
  }

  /**
   * Load all proposals from Holochain.
   */
  getProposals(): Observable<ProposalRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() => from(this.governance.queryGovernanceStates('proposal'))).pipe(
      map(results => results.map(r => this.transformGovernanceStateToProposal(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get proposals by status (voting, discussion, decided).
   */
  getProposalsByStatus(status: string): Observable<ProposalRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() => from(this.governance.queryGovernanceStates('proposal'))).pipe(
      map(results => results
        .filter(r => (r as Record<string, unknown>)['status'] === status)
        .map(r => this.transformGovernanceStateToProposal(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Transform GovernanceStateView to ProposalRecord (adapter for thin API).
   */
  private transformGovernanceStateToProposal(state: GovernanceStateView): ProposalRecord {
    return {
      id: state.id,
      title: '',
      proposalType: '',
      description: '',
      proposer: { agentId: '', displayName: '' },
      status: state.votingState,
      phase: state.votingState,
      createdAt: state.createdAt,
    };
  }

  /**
   * Load all precedents from Holochain.
   */
  getPrecedents(): Observable<PrecedentRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() => from(this.governance.queryGovernanceStates('precedent'))).pipe(
      map(results => results.map(r => this.transformGovernanceStateToPrecedent(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get precedents by binding level (constitutional, binding-network, binding-local, persuasive).
   */
  getPrecedentsByBinding(binding: string): Observable<PrecedentRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() => from(this.governance.queryGovernanceStates('precedent'))).pipe(
      map(results => results
        .filter(r => (r as Record<string, unknown>)['binding'] === binding)
        .map(r => this.transformGovernanceStateToPrecedent(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Transform GovernanceStateView to PrecedentRecord (adapter for thin API).
   */
  private transformGovernanceStateToPrecedent(state: GovernanceStateView): PrecedentRecord {
    return {
      id: state.id,
      title: '',
      summary: '',
      fullReasoning: '',
      binding: '',
      scope: { entityTypes: [] },
      citations: 0,
      status: state.votingState,
    };
  }

  /**
   * Load all discussion threads from Holochain.
   */
  getDiscussions(): Observable<DiscussionRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() => from(this.governance.queryGovernanceStates('discussion'))).pipe(
      map(results => results.map(r => this.transformGovernanceStateToDiscussion(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get discussions for a specific entity.
   */
  getDiscussionsForEntity(entityType: string, entityId: string): Observable<DiscussionRecord[]> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of([]);
    }

    return defer(() =>
      from(this.governance.queryDiscussions(entityId))
    ).pipe(
      map(results => results.map(r => this.transformDiscussionView(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Transform GovernanceStateView to DiscussionRecord (adapter for thin API).
   */
  private transformGovernanceStateToDiscussion(state: GovernanceStateView): DiscussionRecord {
    return {
      id: state.id,
      entityType: state.entityType,
      entityId: state.entityId,
      category: '',
      title: '',
      messages: [],
      status: state.votingState,
      messageCount: 0,
    };
  }

  /**
   * Transform DiscussionView to DiscussionRecord.
   */
  private transformDiscussionView(view: DiscussionView): DiscussionRecord {
    return {
      id: view.id,
      entityType: 'content',
      entityId: view.contentId,
      category: '',
      title: '',
      messages: [{ id: view.id, authorId: view.authorPresenceId, authorName: '', content: view.body, createdAt: view.createdAt }] as DiscussionRecord['messages'],
      status: 'open',
      messageCount: 1,
    };
  }

  /**
   * Load governance state for a specific entity from Holochain.
   */
  getGovernanceState(
    entityType: string,
    entityId: string
  ): Observable<GovernanceStateRecord | null> {
    if (false) { // TODO: Remove dead availability guards after migration verified
      return of(null);
    }

    return defer(() =>
      from(this.governance.getGovernanceState(entityType, entityId))
    ).pipe(
      map(result =>
        result ? this.transformGovernanceStateView(result) : null
      ),
      catchError(_err => {
        return of(null);
      })
    );
  }

  /**
   * Transform GovernanceStateView to frontend GovernanceStateRecord model.
   */
  private transformGovernanceStateView(view: GovernanceStateView): GovernanceStateRecord {
    const labels = (
      Array.isArray(view.labels) ? view.labels : []
    ) as GovernanceStateRecord['labels'];

    return {
      entityType: view.entityType,
      entityId: view.entityId,
      status: view.votingState,
      statusBasis: { method: '', reasoning: '', deciderId: '', deciderType: '', decidedAt: '' },
      labels,
      activeChallenges: [],
      lastUpdated: view.updatedAt,
    };
  }

  // =========================================================================
  // Cluster Graph Methods (for hierarchical graph visualization)
  // =========================================================================

  /**
   * Get path hierarchy for cluster graph visualization.
   *
   * This is a convenience method that uses existing getPath() which loads
   * chapters from Holochain's metadata_json field. The LearningPath returned
   * contains the full hierarchy: chapters → modules → sections → conceptIds.
   *
   * @param pathId - Learning path ID (e.g., 'elohim-protocol')
   * @returns Observable of LearningPath with chapters hierarchy
   */
  getPathHierarchy(pathId: string): Observable<LearningPath> {
    return this.getPath(pathId);
  }

  /**
   * Batch load content nodes for a cluster's conceptIds.
   *
   * Uses existing batchGetContent() for efficient retrieval from Holochain.
   * This is optimized for cluster expansion where we need to load all
   * concepts in a section at once.
   *
   * @param conceptIds - Array of concept IDs to load
   * @returns Observable of Map<id, ContentNode>
   */
  getClusterConcepts(conceptIds: string[]): Observable<Map<string, ContentNode>> {
    return this.batchGetContent(conceptIds);
  }

  /**
   * Get aggregated connections for concepts within a cluster.
   *
   * Queries relationships for each concept and aggregates them by
   * target cluster. This enables showing "12 connections to Governance"
   * on collapsed clusters instead of individual relationship lines.
   *
   * @param conceptIds - Concept IDs in the source cluster
   * @param clusterMapping - Map of conceptId → clusterId for aggregation
   * @returns Observable of ClusterConnectionSummary
   */
  getClusterConnections(
    conceptIds: string[],
    clusterMapping: Map<string, string>
  ): Observable<ClusterConnectionSummary> {
    if (conceptIds.length === 0) {
      return of(this.createEmptyClusterConnectionSummary());
    }

    // Query relationships for all concepts in the cluster
    const relationshipQueries = conceptIds.map(id =>
      this.getRelationshipsForNode(id, 'both').pipe(catchError(() => of([])))
    );

    return forkJoin(relationshipQueries).pipe(
      map(relationshipArrays =>
        this.aggregateClusterConnections(conceptIds, relationshipArrays, clusterMapping)
      )
    );
  }

  /**
   * Create an empty cluster connection summary.
   */
  private createEmptyClusterConnectionSummary(): ClusterConnectionSummary {
    return {
      clusterId: '',
      outgoingByCluster: new Map(),
      incomingByCluster: new Map(),
      totalConnections: 0,
    };
  }

  /**
   * Aggregate relationship data into cluster connections.
   */
  private aggregateClusterConnections(
    conceptIds: string[],
    relationshipArrays: ContentRelationship[][],
    clusterMapping: Map<string, string>
  ): ClusterConnectionSummary {
    const outgoingByCluster = new Map<string, ClusterConnectionData>();
    const incomingByCluster = new Map<string, ClusterConnectionData>();
    let totalConnections = 0;

    for (let i = 0; i < conceptIds.length; i++) {
      const sourceConceptId = conceptIds[i];
      const relationships = relationshipArrays[i];

      for (const rel of relationships) {
        const connectionInfo = this.processRelationship(rel, sourceConceptId, clusterMapping);
        if (!connectionInfo) continue;

        const targetMap = connectionInfo.isOutgoing ? outgoingByCluster : incomingByCluster;
        this.updateClusterConnection(targetMap, connectionInfo.clusterId, rel.relationshipType);
        totalConnections++;
      }
    }

    return {
      clusterId: '', // Caller sets this
      outgoingByCluster,
      incomingByCluster,
      totalConnections,
    };
  }

  /**
   * Process a single relationship to determine cluster membership.
   * Returns null if the related node is not in the cluster mapping.
   */
  private processRelationship(
    rel: ContentRelationship,
    sourceConceptId: string,
    clusterMapping: Map<string, string>
  ): { isOutgoing: boolean; clusterId: string } | null {
    const isOutgoing = rel.sourceNodeId === sourceConceptId;
    const otherNodeId = isOutgoing ? rel.targetNodeId : rel.sourceNodeId;
    const otherClusterId = clusterMapping.get(otherNodeId);

    if (!otherClusterId) return null;

    return { isOutgoing, clusterId: otherClusterId };
  }

  /**
   * Update or create connection data for a cluster.
   */
  private updateClusterConnection(
    targetMap: Map<string, ClusterConnectionData>,
    clusterId: string,
    relationshipType: string
  ): void {
    if (!targetMap.has(clusterId)) {
      targetMap.set(clusterId, {
        sourceClusterId: '', // Will be set by caller
        targetClusterId: clusterId,
        connectionCount: 0,
        relationshipTypes: [],
      });
    }

    const connection = targetMap.get(clusterId)!;
    connection.connectionCount++;

    if (!connection.relationshipTypes.includes(relationshipType)) {
      connection.relationshipTypes.push(relationshipType);
    }
  }
}
