import { Injectable, inject } from '@angular/core';
import { Observable, of, from, defer, forkJoin } from 'rxjs';
import { catchError, map, shareReplay, tap, switchMap, timeout } from 'rxjs/operators';
import { LoggerService } from './logger.service';
import {
  HolochainContentService,
  HolochainPathOverview,
  HolochainAgentEntry,
  HolochainAttestationEntry,
  HolochainContentGraph,
  HolochainContentGraphNode,
  HolochainContentAttestationEntry,
} from './holochain-content.service';
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { ProjectionAPIService } from './projection-api.service';
import { ContentResolverService } from './content-resolver.service';
import { ContentService } from './content.service';

// Models from elohim (local)
import { Agent, AgentProgress, AgentAttestation } from '../models/agent.model';

// Models from lamad pillar (will stay there - content-specific)
// Using relative imports for now; will update to @app/lamad after full migration
import { LearningPath, PathIndex } from '../../lamad/models/learning-path.model';
import { ContentNode, ContentGraph, ContentRelationship, ContentRelationshipType } from '../../lamad/models/content-node.model';
import { ContentAttestation } from '../../lamad/models/content-attestation.model';
import {
  KnowledgeMapIndex,
  KnowledgeMap,
  KnowledgeMapIndexEntry,
  KnowledgeMapType,
  KnowledgeNode
} from '../../lamad/models/knowledge-map.model';
import {
  PathExtensionIndex,
  PathExtension,
  PathExtensionIndexEntry,
  PathStepInsertion,
  PathStepAnnotation,
  PathStepReorder,
  PathStepExclusion,
  UpstreamProposal,
  ExtensionStats
} from '../../lamad/models/path-extension.model';

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
  messages: Array<{
    id: string;
    authorId: string;
    authorName: string;
    content: string;
    createdAt: string;
  }>;
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
  labels: Array<{ labelType: string; severity: string; appliedBy: string }>;
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

  constructor(
    private readonly holochainContent: HolochainContentService,
    private readonly idbCache: IndexedDBCacheService
  ) {
    // Initialize caches in background
    this.initCaches();

    // NOTE: Conductor availability tracking removed.
    // Conductor is no longer used for content resolution - content comes from doorway projection.
    // Holochain conductor is only used for agent-centric data (identity, attestations, points).
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
      this.contentResolver.setSourceAvailable('conductor', this.holochainContent.isAvailable());

      this.logger.debug('ContentResolver initialized with sources');
    } catch (err) {
      this.logger.warn('Cache initialization failed', { error: err instanceof Error ? err.message : String(err) });
    }
  }

  // NOTE: PATH_TIMEOUT_MS removed - ContentService handles timeouts

  /**
   * Load a LearningPath by ID.
   * Does NOT load the content for each step (lazy loading).
   * Uses ContentService which routes to doorway (browser) or local storage (Tauri).
   */
  getPath(pathId: string): Observable<LearningPath> {
    return this.contentService.getPath(pathId).pipe(
      map(path => {
        if (!path) {
          throw new Error(`Path not found: ${pathId}`);
        }
        return path;
      }),
      tap(path => {
        // Store in IndexedDB cache for offline persistence (background, non-blocking)
        if (this.idbInitialized) {
          this.idbCache.setPath(path).catch(() => {
            // Ignore IndexedDB errors
          });
        }
      }),
      catchError(err => {
        const errMsg = err.message || String(err);
        if (errMsg.includes('Path not found') || errMsg.includes('not found')) {
          this.logger.warn('Path not found (may be stale reference)', { pathId });
        } else {
          this.logger.error('Error loading path', err, { pathId });
        }
        throw err; // Re-throw - paths are critical, can't use placeholder
      })
    );
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
    // Try projection API first if enabled
    if (this.projectionApi.enabled) {
      return this.projectionApi.getPathOverview(pathId).pipe(
        timeout(5000),
        switchMap(result => {
          if (result) {
            return of(result as LearningPath);
          }
          // Fall back to Holochain REST
          return this.getPathOverviewFromHolochain(pathId);
        }),
        catchError(() => this.getPathOverviewFromHolochain(pathId))
      );
    }

    return this.getPathOverviewFromHolochain(pathId);
  }

  /**
   * Load path overview from Holochain REST API (fallback).
   */
  private getPathOverviewFromHolochain(pathId: string): Observable<LearningPath> {
    return defer(() => from(this.holochainContent.getPathOverviewRest(pathId))).pipe(
      timeout(10000),
      map(result => {
        if (!result) {
          throw new Error(`Path not found: ${pathId}`);
        }
        return this.transformHolochainPathOverview(result);
      }),
      catchError(err => {
        this.logger.warn('Path overview failed', { pathId, error: err.message || err });
        throw err;
      }),
      shareReplay(1)
    );
  }

  /**
   * Transform path overview to LearningPath model.
   * Returns path with empty steps array - use getPath() for full steps.
   */
  private transformHolochainPathOverview(hcPath: HolochainPathOverview): LearningPath {
    // Extract chapters from metadata if available
    let chapters: LearningPath['chapters'] | undefined;
    const metadata = hcPath.path.metadata ?? {};
    if (metadata && typeof metadata === 'object' && 'chapters' in metadata) {
      const metadataObj = metadata as Record<string, unknown>;
      if (Array.isArray(metadataObj['chapters'])) {
        chapters = metadataObj['chapters'] as LearningPath['chapters'];
      }
    }

    return {
      id: hcPath.path.id,
      version: hcPath.path.version,
      title: hcPath.path.title,
      description: hcPath.path.description,
      purpose: hcPath.path.purpose ?? '',
      createdBy: hcPath.path.createdBy,
      contributors: [],
      createdAt: hcPath.path.createdAt,
      updatedAt: hcPath.path.updatedAt,
      difficulty: hcPath.path.difficulty as LearningPath['difficulty'],
      estimatedDuration: hcPath.path.estimatedDuration ?? '',
      tags: hcPath.path.tags,
      visibility: hcPath.path.visibility as LearningPath['visibility'],
      chapters,
      // Empty steps - use getPath() for full step data
      steps: [],
      // Store step count in metadata for UI display
      stepCount: hcPath.stepCount,
    } as LearningPath & { stepCount?: number };
  }

  /**
   * Load a ContentNode by ID.
   * Uses ContentService which routes to doorway (browser) or local storage (Tauri).
   *
   * IMPORTANT: Returns a placeholder node instead of throwing for missing content.
   * This prevents one missing item from breaking entire path loading.
   */
  getContent(resourceId: string): Observable<ContentNode> {
    return this.contentService.getContent(resourceId).pipe(
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
            // Ignore IndexedDB errors
          });
        }
      }),
      catchError(err => {
        this.logger.warn('Error loading content', { resourceId, error: err.message || err });
        return of(this.createPlaceholderContent(resourceId, err.message));
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
          const toCache = Array.from(contentMap.values()).filter(c => c.contentType !== 'placeholder');
          if (toCache.length > 0) {
            this.idbCache.setContentBatch(toCache).catch(() => {});
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
      catchError(err => {
        this.logger.warn('Batch load error', { count: resourceIds.length, error: err.message || err });
        // Return placeholders for all
        const contentMap = new Map<string, ContentNode>();
        for (const id of resourceIds) {
          contentMap.set(id, this.createPlaceholderContent(id, err.message));
        }
        return of(contentMap);
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

    // Fire and forget - don't block the UI
    this.holochainContent.prefetchRelatedContent(uncachedIds);
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
        const stepResourceIds = path.steps
          .slice(0, prefetchSteps)
          .map(s => s.resourceId);
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
      description: errorMessage || `The content "${resourceId}" could not be loaded.`,
      content: `This content is not yet available. It may not have been seeded or there was an error loading it.\n\nResource ID: ${resourceId}${errorMessage ? `\nError: ${errorMessage}` : ''}`,
      contentFormat: 'markdown',
      tags: ['missing', 'placeholder'],
      relatedNodeIds: [],
      metadata: {},
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  }

  /** Cached content index for search/discovery */
  private contentIndexCache$: Observable<any> | null = null;

  /**
   * Load the content index for search/discovery.
   * Returns metadata only, not full content.
   * Uses ContentService (doorway) as the source.
   * Cached with shareReplay(1) to prevent redundant calls.
   */
  getContentIndex(): Observable<any> {
    if (!this.contentIndexCache$) {
      this.contentIndexCache$ = this.contentService.queryContent({ limit: 1000 }).pipe(
        map(nodes => ({
          nodes: nodes.map(node => ({
            id: node.id,
            title: node.title,
            description: node.description || '',
            contentType: node.contentType,
            tags: node.tags || [],
            reach: node.reach || 'commons',
            trustScore: node.trustScore,
            createdAt: node.createdAt,
            updatedAt: node.updatedAt,
          })),
          totalCount: nodes.length,
          byType: this.groupByType(nodes),
          lastUpdated: new Date().toISOString()
        })),
        shareReplay(1),
        catchError(err => {
          this.logger.error('Failed to load content index', err);
          // Clear cache on error so next call retries
          this.contentIndexCache$ = null;
          return of({ nodes: [], totalCount: 0, byType: {}, lastUpdated: new Date().toISOString() });
        })
      );
    }
    return this.contentIndexCache$;
  }

  /**
   * Group content nodes by type for index statistics.
   */
  private groupByType(nodes: ContentNode[]): Record<string, number> {
    const byType: Record<string, number> = {};
    for (const node of nodes) {
      byType[node.contentType] = (byType[node.contentType] || 0) + 1;
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
   * Uses ContentService (doorway projection) as the source.
   * Cached with shareReplay(1) to prevent redundant calls.
   */
  getPathIndex(): Observable<PathIndex> {
    if (!this.pathIndexCache$) {
      this.pathIndexCache$ = this.contentService.queryPaths({}).pipe(
        map(paths => this.transformPathsToIndex(paths)),
        shareReplay(1),
        catchError(err => {
          this.logger.error('Failed to load path index', err);
          // Clear cache on error so next call retries
          this.pathIndexCache$ = null;
          return of({ paths: [], totalCount: 0, lastUpdated: new Date().toISOString() });
        })
      );
    }
    return this.pathIndexCache$;
  }

  /**
   * Transform LearningPath[] to PathIndex model.
   */
  private transformPathsToIndex(paths: LearningPath[]): PathIndex {
    return {
      lastUpdated: new Date().toISOString(),
      totalCount: paths.length,
      paths: paths.map(p => ({
        id: p.id,
        title: p.title,
        description: p.description || '',
        difficulty: p.difficulty as any,
        estimatedDuration: p.estimatedDuration ?? '',
        stepCount: this.calculateStepCount(p),
        tags: p.tags || [],
        thumbnailUrl: p.thumbnailUrl,
        thumbnailAlt: p.thumbnailAlt,
        chapterCount: p.chapters?.length,
        pathType: p.pathType,
        attestationsGranted: p.attestationsGranted,
      })),
    };
  }

  /**
   * Calculate total step count for a path (handles both flat and chapter-based paths).
   */
  private calculateStepCount(path: LearningPath): number {
    // Check if raw data includes pre-computed stepCount (from doorway)
    const rawStepCount = (path as any).stepCount ?? (path as any).stepCount;
    if (typeof rawStepCount === 'number' && rawStepCount > 0) {
      return rawStepCount;
    }

    // Calculate from structure
    if (path.chapters && path.chapters.length > 0) {
      return path.chapters.reduce((total, chapter) => {
        if (chapter.steps) {
          return total + chapter.steps.length;
        }
        if (chapter.modules) {
          return total + chapter.modules.reduce((modTotal, mod) =>
            modTotal + mod.sections.reduce((secTotal, sec) =>
              secTotal + (sec.conceptIds?.length ?? 0), 0), 0);
        }
        return total;
      }, 0);
    }

    return path.steps?.length ?? 0;
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
  getAgent(agentId: string): Observable<Agent | null> {
    if (!this.holochainContent.isAvailable()) {
      return of(null);
    }

    return defer(() => from(this.holochainContent.getAgentById(agentId))).pipe(
      map(result => result ? this.transformHolochainAgent(result.agent) : null),
      catchError((err) => {
        this.logger.warn('Failed to load agent', { agentId, error: err.message || err });
        return of(null);
      })
    );
  }

  /**
   * Load agent progress for a specific path.
   * Uses localStorage for prototype. In Holochain, this will read from private source chain.
   */
  getAgentProgress(agentId: string, pathId: string): Observable<AgentProgress | null> {
    // Use localStorage directly instead of fetching from JSON files
    // This avoids 404 errors and keeps all progress client-side
    const progress = this.getLocalProgress(agentId, pathId);
    return of(progress);
  }

  /**
   * Save agent progress.
   * In prototype: Updates localStorage (JSON files are read-only).
   * In Holochain: Writes to private source chain.
   */
  saveAgentProgress(progress: AgentProgress): Observable<void> {
    const key = `lamad-progress-${progress.agentId}-${progress.pathId}`;
    try {
      localStorage.setItem(key, JSON.stringify(progress));
    } catch {
      // Silently ignore localStorage quota errors - progress will be lost on refresh
      // but the app continues to function
    }
    return of(undefined);
  }

  /**
   * Load progress from localStorage (prototype fallback).
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
    // Also clear Holochain content cache
    this.holochainContent.clearCache();

    // Optionally clear IndexedDB persistent cache
    if (includeIndexedDB && this.idbInitialized) {
      this.idbCache.clearAll().catch(() => {
        // Ignore errors
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
      indexedDBAvailable: this.idbInitialized
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
    if (!this.holochainContent.isAvailable()) {
      this.attestationCache$ ??= of([]);
      return this.attestationCache$;
    }

    this.attestationCache$ ??= defer(() =>
      from(this.holochainContent.queryContentAttestations({ status: 'active' }))
    ).pipe(
      map(results => results.map(r => this.transformHolochainContentAttestation(r.contentAttestation))),
      shareReplay(1),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load content attestations:', err);
        return of([]);
      })
    );
    return this.attestationCache$;
  }

  /**
   * Load agent attestations (credentials/achievements) from Holochain.
   *
   * These are different from content attestations - they represent
   * achievements earned by agents (domain-mastery, path-completion, etc.)
   */
  getAgentAttestations(agentId?: string, category?: string): Observable<AgentAttestation[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.getAttestations({
      agentId: agentId,
      category: category,
    }))).pipe(
      map(results => results.map(r => this.transformHolochainAttestation(r.attestation))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load agent attestations:', err);
        return of([]);
      })
    );
  }

  /**
   * Transform Holochain attestation entry to frontend AgentAttestation model.
   */
  private transformHolochainAttestation(hcAtt: HolochainAttestationEntry): AgentAttestation {
    const earnedVia = (hcAtt.earnedVia ?? {}) as AgentAttestation['earnedVia'];

    return {
      id: hcAtt.id,
      agentId: hcAtt.agentId,
      category: hcAtt.category as AgentAttestation['category'],
      attestationType: hcAtt.attestationType,
      displayName: hcAtt.displayName,
      description: hcAtt.description,
      iconUrl: hcAtt.iconUrl ?? undefined,
      tier: hcAtt.tier as AgentAttestation['tier'],
      earnedVia,
      issuedAt: hcAtt.issuedAt,
      issuedBy: hcAtt.issuedBy,
      expiresAt: hcAtt.expiresAt ?? undefined,
      proof: hcAtt.proof ?? undefined,
    };
  }

  /**
   * Transform Holochain content attestation entry to frontend ContentAttestation model.
   */
  private transformHolochainContentAttestation(hcAtt: HolochainContentAttestationEntry): ContentAttestation {
    const grantedBy = (hcAtt.grantedBy ?? { type: 'system', grantorId: 'unknown' }) as ContentAttestation['grantedBy'];
    const revocation = (hcAtt.revocation ?? undefined) as ContentAttestation['revocation'];
    const evidence = (hcAtt.evidence ?? undefined) as ContentAttestation['evidence'];
    const scope = (hcAtt.scope ?? undefined) as ContentAttestation['scope'];
    const metadata = (hcAtt.metadata ?? {}) as ContentAttestation['metadata'];

    return {
      id: hcAtt.id,
      contentId: hcAtt.contentId,
      attestationType: hcAtt.attestationType as ContentAttestation['attestationType'],
      reachGranted: hcAtt.reachGranted as ContentAttestation['reachGranted'],
      grantedBy,
      grantedAt: hcAtt.grantedAt,
      expiresAt: hcAtt.expiresAt ?? undefined,
      status: hcAtt.status as ContentAttestation['status'],
      revocation,
      evidence,
      scope,
      metadata,
    };
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

    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() =>
      from(this.holochainContent.getAttestationsForContent(contentId))
    ).pipe(
      map(results => {
        const attestations = results.map(r => this.transformHolochainContentAttestation(r.contentAttestation));
        this.attestationsByContentCache.set(contentId, attestations);
        return attestations;
      }),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load attestations for content:', err);
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
  getAgentIndex(): Observable<{ agents: Agent[] }> {
    if (!this.holochainContent.isAvailable()) {
      return of({ agents: [] });
    }

    return defer(() => from(this.holochainContent.queryAgents({}))).pipe(
      map(results => ({
        agents: results.map(r => this.transformHolochainAgent(r.agent))
      })),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load agent index:', err);
        return of({ agents: [] });
      })
    );
  }

  /**
   * Transform Holochain agent entry to frontend Agent model.
   */
  private transformHolochainAgent(hcAgent: HolochainAgentEntry): Agent {
    return {
      id: hcAgent.id,
      displayName: hcAgent.displayName,
      type: hcAgent.agentType as Agent['type'],
      bio: hcAgent.bio ?? undefined,
      avatar: hcAgent.avatar ?? undefined,
      visibility: hcAgent.visibility as Agent['visibility'],
      createdAt: hcAgent.createdAt,
      updatedAt: hcAgent.updatedAt,
      did: hcAgent.did ?? undefined,
      activityPubType: hcAgent.activityPubType as Agent['activityPubType'],
    };
  }

  // =========================================================================
  // Knowledge Map Loading
  // =========================================================================

  /**
   * Load the knowledge map index from Holochain.
   */
  getKnowledgeMapIndex(): Observable<KnowledgeMapIndex> {
    if (!this.holochainContent.isAvailable()) {
      return of({ maps: [], totalCount: 0, lastUpdated: new Date().toISOString() });
    }

    return defer(() => from(this.holochainContent.queryKnowledgeMaps({}))).pipe(
      map(results => ({
        lastUpdated: new Date().toISOString(),
        totalCount: results.length,
        maps: results.map(r => this.transformHolochainKnowledgeMapToIndex(r.knowledgeMap))
      })),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load knowledge map index:', err);
        return of({ maps: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load a specific knowledge map from Holochain.
   */
  getKnowledgeMap(mapId: string): Observable<KnowledgeMap | null> {
    if (!this.holochainContent.isAvailable()) {
      return of(null);
    }

    return defer(() => from(this.holochainContent.getKnowledgeMapById(mapId))).pipe(
      map(result => result ? this.transformHolochainKnowledgeMap(result.knowledgeMap) : null),
      catchError((err) => {
        console.warn(`[DataLoader] Failed to load knowledge map "${mapId}":`, err);
        return of(null);
      })
    );
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
      updatedAt: hcMap.updatedAt
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
    const metadata = (hcMap.metadata && typeof hcMap.metadata === 'object' ? hcMap.metadata : {}) as Record<string, unknown>;

    return {
      id: hcMap.id,
      mapType: hcMap.mapType as KnowledgeMapType,
      subject: {
        type: hcMap.subjectType as 'content-graph' | 'agent' | 'organization',
        subjectId: hcMap.subjectId,
        subjectName: hcMap.subjectName
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
      metadata
    };
  }

  // =========================================================================
  // Path Extension Loading
  // =========================================================================

  /**
   * Load the path extension index from Holochain.
   */
  getPathExtensionIndex(): Observable<PathExtensionIndex> {
    if (!this.holochainContent.isAvailable()) {
      return of({ extensions: [], totalCount: 0, lastUpdated: new Date().toISOString() });
    }

    return defer(() => from(this.holochainContent.queryPathExtensions({}))).pipe(
      map(results => ({
        lastUpdated: new Date().toISOString(),
        totalCount: results.length,
        extensions: results.map(r => this.transformHolochainPathExtensionToIndex(r.pathExtension))
      })),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load path extension index:', err);
        return of({ extensions: [], totalCount: 0, lastUpdated: new Date().toISOString() });
      })
    );
  }

  /**
   * Load a specific path extension from Holochain.
   */
  getPathExtension(extensionId: string): Observable<PathExtension | null> {
    if (!this.holochainContent.isAvailable()) {
      return of(null);
    }

    return defer(() => from(this.holochainContent.getPathExtensionById(extensionId))).pipe(
      map(result => result ? this.transformHolochainPathExtension(result.pathExtension) : null),
      catchError((err) => {
        console.warn(`[DataLoader] Failed to load path extension "${extensionId}":`, err);
        return of(null);
      })
    );
  }

  /**
   * Get extensions for a specific base path.
   */
  getExtensionsForPath(pathId: string): Observable<PathExtension[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryPathExtensions({ basePathId: pathId }))).pipe(
      map(results => results.map(r => this.transformHolochainPathExtension(r.pathExtension))),
      catchError((err) => {
        console.warn(`[DataLoader] Failed to load extensions for path "${pathId}":`, err);
        return of([]);
      })
    );
  }

  /**
   * Transform Holochain path extension entry to PathExtensionIndexEntry.
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
      updatedAt: hcExt.updatedAt
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
    const insertions = (Array.isArray(hcExt.insertions) ? hcExt.insertions : []) as PathStepInsertion[];
    const annotations = (Array.isArray(hcExt.annotations) ? hcExt.annotations : []) as PathStepAnnotation[];
    const reorderings = (Array.isArray(hcExt.reorderings) ? hcExt.reorderings : []) as PathStepReorder[];
    const exclusions = (Array.isArray(hcExt.exclusions) ? hcExt.exclusions : []) as PathStepExclusion[];
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
      updatedAt: hcExt.updatedAt
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
      if (this.holochainContent.isAvailable()) {
        // Build graph from Holochain relationships
        this.graphCache$ = this.buildGraphFromHolochain().pipe(
          shareReplay(1),
          catchError((err) => {
            console.warn('[DataLoader] Failed to load graph from Holochain:', err);
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
        catchError((err) => {
          console.warn(`[DataLoader] Failed to load relationships for "${contentId}":`, err);
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
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() =>
      from(this.holochainContent.getRelationships({ contentId: contentId, direction }))
    ).pipe(
      map(results => results.map(r => this.transformHolochainRelationship(r.relationship)))
    );
  }

  /**
   * Transform Holochain relationship entry to frontend ContentRelationship model.
   */
  private transformHolochainRelationship(
    hcRel: { id: string; sourceId: string; targetId: string; relationshipType: string; confidence: number; metadata: unknown }
  ): ContentRelationship {
    const metadata = (hcRel.metadata && typeof hcRel.metadata === 'object' ? hcRel.metadata : {}) as Record<string, unknown>;

    // Store confidence in metadata since ContentRelationship doesn't have a confidence field
    if (hcRel.confidence !== undefined && hcRel.confidence !== null) {
      metadata['confidence'] = hcRel.confidence;
    }

    return {
      id: hcRel.id,
      sourceNodeId: hcRel.sourceId,
      targetNodeId: hcRel.targetId,
      relationshipType: hcRel.relationshipType as ContentRelationshipType,
      metadata
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
   * Build ContentGraph from Holochain content and relationships.
   */
  private buildGraphFromHolochain(): Observable<ContentGraph> {
    // Get content graph starting from manifesto root
    return defer(() => from(this.holochainContent.getContentGraph('manifesto'))).pipe(
      map(hcGraph => {
        if (!hcGraph) {
          return this.createEmptyGraph();
        }
        return this.transformHolochainGraph(hcGraph);
      })
    );
  }

  /**
   * Transform Holochain ContentGraph to frontend ContentGraph structure.
   */
  private transformHolochainGraph(hcGraph: HolochainContentGraph): ContentGraph {
    const nodes = new Map<string, ContentNode>();
    const relationships = new Map<string, ContentRelationship>();
    const nodesByType = new Map<string, Set<string>>();
    const nodesByTag = new Map<string, Set<string>>();
    const nodesByCategory = new Map<string, Set<string>>();
    const adjacency = new Map<string, Set<string>>();
    const reverseAdjacency = new Map<string, Set<string>>();

    // Add root node if present
    if (hcGraph.root) {
      const rootNode = this.transformHolochainContentToNode(hcGraph.root);
      this.addNodeToGraphIndexes(rootNode, nodes, nodesByType, nodesByTag, nodesByCategory, adjacency, reverseAdjacency);
    }

    // Process related nodes recursively
    this.processHolochainGraphNodes(hcGraph.related, nodes, relationships, nodesByType, nodesByTag, nodesByCategory, adjacency, reverseAdjacency, hcGraph.root?.content.id);

    return {
      nodes,
      relationships,
      nodesByType,
      nodesByTag,
      nodesByCategory,
      adjacency,
      reverseAdjacency,
      metadata: {
        nodeCount: hcGraph.totalNodes,
        relationshipCount: relationships.size,
        lastUpdated: new Date().toISOString(),
        version: '1.0.0'
      }
    };
  }

  /**
   * Transform HolochainContentOutput to ContentNode.
   */
  private transformHolochainContentToNode(output: { content: { id: string; contentType: string; title: string; description: string; content: string; contentFormat: string; tags: string[]; sourcePath: string | null; relatedNodeIds: string[]; metadata: unknown; createdAt: string; updatedAt: string } }): ContentNode {
    const entry = output.content;
    const metadata = (entry.metadata && typeof entry.metadata === 'object' ? entry.metadata : {}) as Record<string, unknown>;

    return {
      id: entry.id,
      contentType: entry.contentType as ContentNode['contentType'],
      title: entry.title,
      description: entry.description,
      content: entry.content,
      contentFormat: entry.contentFormat as ContentNode['contentFormat'],
      tags: entry.tags,
      sourcePath: entry.sourcePath ?? undefined,
      relatedNodeIds: entry.relatedNodeIds,
      metadata,
      createdAt: entry.createdAt,
      updatedAt: entry.updatedAt,
    };
  }

  /**
   * Process graph nodes recursively from Holochain tree structure.
   */
  private processHolochainGraphNodes(
    graphNodes: HolochainContentGraphNode[],
    nodes: Map<string, ContentNode>,
    relationships: Map<string, ContentRelationship>,
    nodesByType: Map<string, Set<string>>,
    nodesByTag: Map<string, Set<string>>,
    nodesByCategory: Map<string, Set<string>>,
    adjacency: Map<string, Set<string>>,
    reverseAdjacency: Map<string, Set<string>>,
    parentId?: string
  ): void {
    for (const graphNode of graphNodes) {
      const node = this.transformHolochainContentToNode(graphNode.content);
      this.addNodeToGraphIndexes(node, nodes, nodesByType, nodesByTag, nodesByCategory, adjacency, reverseAdjacency);

      // Add relationship from parent to this node
      if (parentId) {
        const relId = `${parentId}-${node.id}`;
        relationships.set(relId, {
          id: relId,
          sourceNodeId: parentId,
          targetNodeId: node.id,
          relationshipType: graphNode.relationshipType as ContentRelationshipType
        });

        // Update adjacency
        if (!adjacency.has(parentId)) adjacency.set(parentId, new Set());
        adjacency.get(parentId)!.add(node.id);

        if (!reverseAdjacency.has(node.id)) reverseAdjacency.set(node.id, new Set());
        reverseAdjacency.get(node.id)!.add(parentId);
      }

      // Process children recursively
      if (graphNode.children.length > 0) {
        this.processHolochainGraphNodes(graphNode.children, nodes, relationships, nodesByType, nodesByTag, nodesByCategory, adjacency, reverseAdjacency, node.id);
      }
    }
  }

  /**
   * Add a node to all graph indexes.
   */
  private addNodeToGraphIndexes(
    node: ContentNode,
    nodes: Map<string, ContentNode>,
    nodesByType: Map<string, Set<string>>,
    nodesByTag: Map<string, Set<string>>,
    nodesByCategory: Map<string, Set<string>>,
    adjacency: Map<string, Set<string>>,
    reverseAdjacency: Map<string, Set<string>>
  ): void {
    nodes.set(node.id, node);
    this.addToSetMap(nodesByType, node.contentType, node.id);
    for (const tag of node.tags || []) {
      this.addToSetMap(nodesByTag, tag, node.id);
    }
    const category = (node.metadata as Record<string, unknown>)?.['category'] as string ?? 'uncategorized';
    this.addToSetMap(nodesByCategory, category, node.id);
    if (!adjacency.has(node.id)) adjacency.set(node.id, new Set());
    if (!reverseAdjacency.has(node.id)) reverseAdjacency.set(node.id, new Set());
  }

  private addToSetMap(map: Map<string, Set<string>>, key: string, value: string): void {
    if (!map.has(key)) {
      map.set(key, new Set());
    }
    map.get(key)!.add(value);
  }

  /**
   * Create empty ContentGraph for error fallback.
   */
  private createEmptyGraph(): ContentGraph {
    return {
      nodes: new Map<string, ContentNode>(),
      relationships: new Map<string, any>(),
      nodesByType: new Map<string, Set<string>>(),
      nodesByTag: new Map<string, Set<string>>(),
      nodesByCategory: new Map<string, Set<string>>(),
      adjacency: new Map<string, Set<string>>(),
      reverseAdjacency: new Map<string, Set<string>>(),
      metadata: {
        nodeCount: 0,
        relationshipCount: 0,
        lastUpdated: new Date().toISOString(),
        version: '1.0.0'
      }
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
    if (!this.holochainContent.isAvailable()) {
      return of({ assessments: [], totalCount: 0, lastUpdated: new Date().toISOString() });
    }

    return this.holochainContent.getContentByType('assessment', 500).pipe(
      map(contentNodes => {
        const assessments: AssessmentIndexEntry[] = contentNodes.map(node => ({
          id: node.id,
          title: node.title,
          domain: node.metadata?.['domain'] ?? 'general',
          instrumentType: node.metadata?.['instrumentType'] ?? 'questionnaire',
          estimatedTime: node.metadata?.['estimatedTime'] ?? '15 minutes',
        }));

        return {
          assessments,
          totalCount: assessments.length,
          lastUpdated: new Date().toISOString(),
        };
      }),
      catchError(() => of({ assessments: [], totalCount: 0, lastUpdated: new Date().toISOString() }))
    );
  }

  /**
   * Load a specific assessment instrument.
   * Assessments are also stored as content nodes, so this uses the content loader.
   */
  getAssessment(assessmentId: string): Observable<ContentNode | null> {
    return this.getContent(assessmentId).pipe(
      catchError(() => of(null))
    );
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
    if (!this.holochainContent.isAvailable()) {
      return of({
        lastUpdated: new Date().toISOString(),
        challengeCount: 0,
        proposalCount: 0,
        precedentCount: 0,
        discussionCount: 0
      });
    }

    // Query all governance types in parallel
    return defer(() => Promise.all([
      this.holochainContent.queryChallenges({}),
      this.holochainContent.queryProposals({}),
      this.holochainContent.queryPrecedents({}),
      this.holochainContent.queryDiscussions({})
    ])).pipe(
      map(([challenges, proposals, precedents, discussions]) => ({
        lastUpdated: new Date().toISOString(),
        challengeCount: challenges.length,
        proposalCount: proposals.length,
        precedentCount: precedents.length,
        discussionCount: discussions.length
      })),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load governance index:', err);
        return of({
          lastUpdated: new Date().toISOString(),
          challengeCount: 0,
          proposalCount: 0,
          precedentCount: 0,
          discussionCount: 0
        });
      })
    );
  }

  /**
   * Load all challenges from Holochain.
   */
  getChallenges(): Observable<ChallengeRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryChallenges({}))).pipe(
      map(results => results.map(r => this.transformHolochainChallenge(r.challenge))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load challenges:', err);
        return of([]);
      })
    );
  }

  /**
   * Get challenges for a specific entity.
   */
  getChallengesForEntity(entityType: string, entityId: string): Observable<ChallengeRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryChallenges({
      entityType: entityType,
      entityId: entityId
    }))).pipe(
      map(results => results.map(r => this.transformHolochainChallenge(r.challenge))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load challenges for entity:', err);
        return of([]);
      })
    );
  }

  /**
   * Transform Holochain challenge entry to frontend ChallengeRecord model.
   */
  private transformHolochainChallenge(hcChallenge: {
    id: string;
    entityType: string;
    entityId: string;
    challengerId: string;
    challengerName: string;
    challengerStanding: string;
    grounds: string;
    description: string;
    evidence: unknown;
    status: string;
    filedAt: string;
    slaDeadline: string | null;
    assignedElohim: string | null;
    resolution: unknown;
  }): ChallengeRecord {
    const resolution = (hcChallenge.resolution ?? undefined) as ChallengeRecord['resolution'];

    return {
      id: hcChallenge.id,
      entityType: hcChallenge.entityType,
      entityId: hcChallenge.entityId,
      challenger: {
        agentId: hcChallenge.challengerId,
        displayName: hcChallenge.challengerName,
        standing: hcChallenge.challengerStanding
      },
      grounds: hcChallenge.grounds,
      description: hcChallenge.description,
      status: hcChallenge.status,
      filedAt: hcChallenge.filedAt,
      slaDeadline: hcChallenge.slaDeadline ?? undefined,
      assignedElohim: hcChallenge.assignedElohim ?? undefined,
      resolution
    };
  }

  /**
   * Load all proposals from Holochain.
   */
  getProposals(): Observable<ProposalRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryProposals({}))).pipe(
      map(results => results.map(r => this.transformHolochainProposal(r.proposal))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load proposals:', err);
        return of([]);
      })
    );
  }

  /**
   * Get proposals by status (voting, discussion, decided).
   */
  getProposalsByStatus(status: string): Observable<ProposalRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryProposals({ status }))).pipe(
      map(results => results.map(r => this.transformHolochainProposal(r.proposal))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load proposals by status:', err);
        return of([]);
      })
    );
  }

  /**
   * Transform Holochain proposal entry to frontend ProposalRecord model.
   */
  private transformHolochainProposal(hcProposal: {
    id: string;
    title: string;
    proposalType: string;
    description: string;
    proposerId: string;
    proposerName: string;
    status: string;
    phase: string;
    votingConfig: unknown;
    currentVotes: unknown;
    outcome: unknown;
    createdAt: string;
  }): ProposalRecord {
    const votingConfig = (hcProposal.votingConfig ?? undefined) as ProposalRecord['votingConfig'];
    const currentVotes = (hcProposal.currentVotes ?? undefined) as ProposalRecord['currentVotes'];
    const outcome = (hcProposal.outcome ?? undefined) as ProposalRecord['outcome'];

    return {
      id: hcProposal.id,
      title: hcProposal.title,
      proposalType: hcProposal.proposalType,
      description: hcProposal.description,
      proposer: {
        agentId: hcProposal.proposerId,
        displayName: hcProposal.proposerName
      },
      status: hcProposal.status,
      phase: hcProposal.phase,
      createdAt: hcProposal.createdAt,
      votingConfig,
      currentVotes,
      outcome
    };
  }

  /**
   * Load all precedents from Holochain.
   */
  getPrecedents(): Observable<PrecedentRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryPrecedents({}))).pipe(
      map(results => results.map(r => this.transformHolochainPrecedent(r.precedent))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load precedents:', err);
        return of([]);
      })
    );
  }

  /**
   * Get precedents by binding level (constitutional, binding-network, binding-local, persuasive).
   */
  getPrecedentsByBinding(binding: string): Observable<PrecedentRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryPrecedents({ binding }))).pipe(
      map(results => results.map(r => this.transformHolochainPrecedent(r.precedent))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load precedents by binding:', err);
        return of([]);
      })
    );
  }

  /**
   * Transform Holochain precedent entry to frontend PrecedentRecord model.
   */
  private transformHolochainPrecedent(hcPrecedent: {
    id: string;
    title: string;
    summary: string;
    fullReasoning: string;
    binding: string;
    scope: unknown;
    citations: number;
    status: string;
  }): PrecedentRecord {
    const scope = (hcPrecedent.scope ?? { entityTypes: [] }) as PrecedentRecord['scope'];

    return {
      id: hcPrecedent.id,
      title: hcPrecedent.title,
      summary: hcPrecedent.summary,
      fullReasoning: hcPrecedent.fullReasoning,
      binding: hcPrecedent.binding,
      scope,
      citations: hcPrecedent.citations,
      status: hcPrecedent.status
    };
  }

  /**
   * Load all discussion threads from Holochain.
   */
  getDiscussions(): Observable<DiscussionRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryDiscussions({}))).pipe(
      map(results => results.map(r => this.transformHolochainDiscussion(r.discussion))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load discussions:', err);
        return of([]);
      })
    );
  }

  /**
   * Get discussions for a specific entity.
   */
  getDiscussionsForEntity(entityType: string, entityId: string): Observable<DiscussionRecord[]> {
    if (!this.holochainContent.isAvailable()) {
      return of([]);
    }

    return defer(() => from(this.holochainContent.queryDiscussions({
      entityType: entityType,
      entityId: entityId
    }))).pipe(
      map(results => results.map(r => this.transformHolochainDiscussion(r.discussion))),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load discussions for entity:', err);
        return of([]);
      })
    );
  }

  /**
   * Transform Holochain discussion entry to frontend DiscussionRecord model.
   */
  private transformHolochainDiscussion(hcDiscussion: {
    id: string;
    entityType: string;
    entityId: string;
    category: string;
    title: string;
    messages: unknown;
    status: string;
    messageCount: number;
  }): DiscussionRecord {
    const messages = (Array.isArray(hcDiscussion.messages) ? hcDiscussion.messages : []) as DiscussionRecord['messages'];

    return {
      id: hcDiscussion.id,
      entityType: hcDiscussion.entityType,
      entityId: hcDiscussion.entityId,
      category: hcDiscussion.category,
      title: hcDiscussion.title,
      messages,
      status: hcDiscussion.status,
      messageCount: hcDiscussion.messageCount
    };
  }

  /**
   * Load governance state for a specific entity from Holochain.
   */
  getGovernanceState(entityType: string, entityId: string): Observable<GovernanceStateRecord | null> {
    if (!this.holochainContent.isAvailable()) {
      return of(null);
    }

    return defer(() => from(this.holochainContent.getGovernanceState({
      entityType: entityType,
      entityId: entityId
    }))).pipe(
      map(result => result ? this.transformHolochainGovernanceState(result.governanceState) : null),
      catchError((err) => {
        console.warn('[DataLoader] Failed to load governance state:', err);
        return of(null);
      })
    );
  }

  /**
   * Transform Holochain governance state entry to frontend GovernanceStateRecord model.
   */
  private transformHolochainGovernanceState(hcState: {
    entityType: string;
    entityId: string;
    status: string;
    statusBasis: unknown;
    labels: unknown;
    activeChallenges: unknown;
    lastUpdated: string;
  }): GovernanceStateRecord {
    const statusBasis = (hcState.statusBasis ?? {
      method: '',
      reasoning: '',
      deciderId: '',
      deciderType: '',
      decidedAt: ''
    }) as GovernanceStateRecord['statusBasis'];
    const labels = (Array.isArray(hcState.labels) ? hcState.labels : []) as GovernanceStateRecord['labels'];
    const activeChallenges = (Array.isArray(hcState.activeChallenges) ? hcState.activeChallenges : []) as string[];

    return {
      entityType: hcState.entityType,
      entityId: hcState.entityId,
      status: hcState.status,
      statusBasis,
      labels,
      activeChallenges,
      lastUpdated: hcState.lastUpdated
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
      return of({
        clusterId: '',
        outgoingByCluster: new Map(),
        incomingByCluster: new Map(),
        totalConnections: 0
      });
    }

    // Query relationships for all concepts in the cluster
    const relationshipQueries = conceptIds.map(id =>
      this.getRelationshipsForNode(id, 'both').pipe(
        catchError(() => of([]))
      )
    );

    return forkJoin(relationshipQueries).pipe(
      map(relationshipArrays => {
        const outgoingByCluster = new Map<string, ClusterConnectionData>();
        const incomingByCluster = new Map<string, ClusterConnectionData>();
        let totalConnections = 0;

        for (let i = 0; i < conceptIds.length; i++) {
          const sourceConceptId = conceptIds[i];
          const relationships = relationshipArrays[i];

          for (const rel of relationships) {
            const isOutgoing = rel.sourceNodeId === sourceConceptId;
            const otherNodeId = isOutgoing ? rel.targetNodeId : rel.sourceNodeId;

            // Look up which cluster the other node belongs to
            const otherClusterId = clusterMapping.get(otherNodeId);
            if (!otherClusterId) continue;  // Skip nodes not in our cluster mapping

            const targetMap = isOutgoing ? outgoingByCluster : incomingByCluster;

            if (!targetMap.has(otherClusterId)) {
              targetMap.set(otherClusterId, {
                sourceClusterId: '',  // Will be set by caller
                targetClusterId: otherClusterId,
                connectionCount: 0,
                relationshipTypes: []
              });
            }

            const connection = targetMap.get(otherClusterId)!;
            connection.connectionCount++;
            totalConnections++;

            if (!connection.relationshipTypes.includes(rel.relationshipType)) {
              connection.relationshipTypes.push(rel.relationshipType);
            }
          }
        }

        return {
          clusterId: '',  // Caller sets this
          outgoingByCluster,
          incomingByCluster,
          totalConnections
        };
      })
    );
  }
}
