import { Injectable, inject } from '@angular/core';
import { Observable, of, from, defer, throwError, timer, forkJoin } from 'rxjs';
import { catchError, map, shareReplay, timeout, retry, tap, switchMap, take } from 'rxjs/operators';
import {
  HolochainContentService,
  HolochainPathWithSteps,
  HolochainPathOverview,
  HolochainPathIndex,
  HolochainAgentEntry,
  HolochainAttestationEntry,
  HolochainContentGraph,
  HolochainContentGraphNode,
  HolochainContentAttestationEntry,
} from './holochain-content.service';
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { ProjectionAPIService } from './projection-api.service';
import { ContentResolverService, SourceTier } from './content-resolver.service';

// Models from elohim (local)
import { Agent, AgentProgress, AgentAttestation } from '../models/agent.model';

// Models from lamad pillar (will stay there - content-specific)
// Using relative imports for now; will update to @app/lamad after full migration
import { LearningPath, PathIndex } from '../../lamad/models/learning-path.model';
import { ContentNode, ContentGraph, ContentGraphMetadata, ContentRelationship, ContentRelationshipType } from '../../lamad/models/content-node.model';
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

  /** Maximum number of content items to keep in cache */
  private readonly CONTENT_CACHE_MAX_SIZE = 500;

  /** Maximum number of paths to keep in cache */
  private readonly PATH_CACHE_MAX_SIZE = 50;

  /** IndexedDB cache initialized flag */
  private idbInitialized = false;

  /** Projection API service for fast cached reads */
  private readonly projectionApi = inject(ProjectionAPIService);

  /** Content Resolver for unified tiered resolution */
  private readonly contentResolver = inject(ContentResolverService);

  constructor(
    private readonly holochainContent: HolochainContentService,
    private readonly idbCache: IndexedDBCacheService
  ) {
    // Initialize caches in background
    this.initCaches();
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
        console.log('[DataLoader] IndexedDB cache initialized:', stats);
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

      console.log('[DataLoader] ContentResolver initialized with sources');
    } catch (err) {
      console.warn('[DataLoader] Cache initialization failed:', err);
    }
  }

  /** Path loading timeout in milliseconds (30s for heavy paths) */
  private readonly PATH_TIMEOUT_MS = 30000;

  /**
   * Load a LearningPath by ID.
   * Does NOT load the content for each step (lazy loading).
   * Uses Holochain as the only source.
   *
   * Cache hierarchy:
   * 1. In-memory cache (fastest, LRU eviction)
   * 2. IndexedDB cache (persistent across refreshes)
   * 3. Holochain zome call (network)
   *
   * Uses LRU-style cache eviction to prevent unbounded memory growth.
   */
  getPath(pathId: string): Observable<LearningPath> {
    // Check if already cached (and move to end for LRU)
    if (this.pathCache.has(pathId)) {
      const existing = this.pathCache.get(pathId)!;
      // Move to end (most recently used) - delete and re-add
      this.pathCache.delete(pathId);
      this.pathCache.set(pathId, existing);
      return existing;
    }

    // Evict oldest entries if cache is at capacity
    while (this.pathCache.size >= this.PATH_CACHE_MAX_SIZE) {
      const firstKey = this.pathCache.keys().next().value;
      if (firstKey) {
        this.pathCache.delete(firstKey);
      } else {
        break;
      }
    }

    const request = defer(() => this.loadPathWithIDBFallback(pathId)).pipe(
        timeout(this.PATH_TIMEOUT_MS),
        map(result => {
          if (!result) {
            throw new Error(`Path not found: ${pathId}`);
          }
          return result;
        }),
        tap(path => {
          // Store in IndexedDB cache (background, non-blocking)
          if (this.idbInitialized) {
            this.idbCache.setPath(path).catch(() => {
              // Ignore IndexedDB errors
            });
          }
        }),
        catchError(err => {
          const errMsg = err.message || String(err);
          // "Path not found" is expected for stale references - log as warning, not error
          if (errMsg.includes('Path not found') || errMsg.includes('not found')) {
            console.warn(`[DataLoader] Path "${pathId}" not found (may be stale reference)`);
          } else {
            console.error(`[DataLoader] Error loading path "${pathId}":`, errMsg);
          }
          // Remove from cache so next request retries
          this.pathCache.delete(pathId);
          throw err; // Re-throw - paths are critical, can't use placeholder
        }),
        shareReplay(1)
      );

    this.pathCache.set(pathId, request);
    return request;
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
        console.warn(`[DataLoader] Path overview "${pathId}" failed:`, err.message || err);
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
    // Parse metadata to extract chapters if available
    let chapters: LearningPath['chapters'] | undefined;
    try {
      const metadata = JSON.parse(hcPath.path.metadata_json || '{}');
      if (metadata.chapters && Array.isArray(metadata.chapters)) {
        chapters = metadata.chapters;
      }
    } catch {
      // Ignore JSON parse errors
    }

    return {
      id: hcPath.path.id,
      version: hcPath.path.version,
      title: hcPath.path.title,
      description: hcPath.path.description,
      purpose: hcPath.path.purpose ?? '',
      createdBy: hcPath.path.created_by,
      contributors: [],
      createdAt: hcPath.path.created_at,
      updatedAt: hcPath.path.updated_at,
      difficulty: hcPath.path.difficulty as LearningPath['difficulty'],
      estimatedDuration: hcPath.path.estimated_duration ?? '',
      tags: hcPath.path.tags,
      visibility: hcPath.path.visibility as LearningPath['visibility'],
      chapters,
      // Empty steps - use getPath() for full step data
      steps: [],
      // Store step count in metadata for UI display
      stepCount: hcPath.step_count,
    } as LearningPath & { stepCount?: number };
  }

  /**
   * Load path with unified cache resolution.
   *
   * Uses ContentResolver for intelligent tiered source selection.
   * Cache hierarchy:
   * 1. IndexedDB (local persistent cache)
   * 2. Projection API (Doorway's MongoDB cache - fast HTTP)
   * 3. Holochain (direct conductor - slow, authoritative)
   */
  private async loadPathWithIDBFallback(pathId: string): Promise<LearningPath | null> {
    // Ensure ContentResolver is initialized
    if (!this.contentResolver.isReady) {
      await this.contentResolver.initialize();
    }

    const resolution = await this.contentResolver.resolvePath(pathId);
    if (resolution) {
      console.log(`[DataLoader] Path "${pathId}" loaded from ${resolution.sourceId} (${resolution.durationMs.toFixed(0)}ms)`);
      // Cache in IndexedDB if loaded from remote source
      if (resolution.tier !== SourceTier.Local && this.idbInitialized) {
        this.contentResolver.cachePath(resolution.data).catch(() => {});
      }
      return resolution.data;
    }
    return null;
  }

  /**
   * Transform Holochain path response to LearningPath model.
   * Maps snake_case Rust fields to camelCase TypeScript fields.
   *
   * Extracts chapters from metadata_json if present, preserving
   * the hierarchical structure for UI display.
   */
  private transformHolochainPath(hcPath: HolochainPathWithSteps): LearningPath {
    // Parse metadata to extract chapters if available
    let chapters: LearningPath['chapters'] | undefined;
    try {
      const metadataJson = hcPath.path.metadata_json;
      console.log(`[DataLoader] metadata_json for "${hcPath.path.id}":`, metadataJson?.substring(0, 100) ?? 'MISSING');
      const metadata = JSON.parse(metadataJson || '{}');
      if (metadata.chapters && Array.isArray(metadata.chapters)) {
        chapters = metadata.chapters;
        console.log(`[DataLoader] Extracted ${metadata.chapters.length} chapters from metadata`);
      }
    } catch (e) {
      console.error('[DataLoader] Error parsing metadata_json:', e);
      // Ignore JSON parse errors - chapters will remain undefined
    }

    return {
      id: hcPath.path.id,
      version: hcPath.path.version,
      title: hcPath.path.title,
      description: hcPath.path.description,
      purpose: hcPath.path.purpose ?? '',
      createdBy: hcPath.path.created_by,
      contributors: [],
      createdAt: hcPath.path.created_at,
      updatedAt: hcPath.path.updated_at,
      difficulty: hcPath.path.difficulty as LearningPath['difficulty'],
      estimatedDuration: hcPath.path.estimated_duration ?? '',
      tags: hcPath.path.tags,
      visibility: hcPath.path.visibility as LearningPath['visibility'],
      // Include chapters from metadata (hierarchical structure)
      chapters,
      // Steps remain flattened for backward compatibility and progress tracking
      steps: hcPath.steps.map((s, index) => ({
        order: s.step.order_index,
        stepType: (s.step.step_type || 'content') as 'content' | 'path' | 'external' | 'checkpoint',
        resourceId: s.step.resource_id,
        stepTitle: s.step.step_title ?? `Step ${index + 1}`,
        stepNarrative: s.step.step_narrative ?? '',
        learningObjectives: [],
        optional: s.step.is_optional,
        completionCriteria: [],
      })),
    };
  }

  /** Content loading timeout in milliseconds (15s for slow responses) */
  private readonly CONTENT_TIMEOUT_MS = 15000;

  /**
   * Load a ContentNode by ID.
   * This is the only way to get content - enforces lazy loading.
   * Uses Holochain as the only source.
   *
   * IMPORTANT: Returns a placeholder node instead of throwing for missing content.
   * This prevents one missing item from breaking entire path loading.
   *
   * Cache hierarchy:
   * 1. In-memory cache (fastest, LRU eviction)
   * 2. IndexedDB cache (persistent across refreshes)
   * 3. Holochain zome call (network)
   *
   * Uses LRU-style cache eviction to prevent unbounded memory growth.
   */
  getContent(resourceId: string): Observable<ContentNode> {
    // Check if already in memory cache (and move to end for LRU)
    if (this.contentCache.has(resourceId)) {
      const existing = this.contentCache.get(resourceId)!;
      // Move to end (most recently used) - delete and re-add
      this.contentCache.delete(resourceId);
      this.contentCache.set(resourceId, existing);
      return existing;
    }

    // Evict oldest entries if cache is at capacity
    while (this.contentCache.size >= this.CONTENT_CACHE_MAX_SIZE) {
      const firstKey = this.contentCache.keys().next().value;
      if (firstKey) {
        this.contentCache.delete(firstKey);
      } else {
        break;
      }
    }

    // Check IndexedDB cache first, then fall back to Holochain
    const request = defer(() => this.loadContentWithIDBFallback(resourceId)).pipe(
      timeout(this.CONTENT_TIMEOUT_MS),
      map(content => {
        if (!content) {
          console.warn(`[DataLoader] Content not found: ${resourceId}, returning placeholder`);
          return this.createPlaceholderContent(resourceId);
        }
        return content;
      }),
      tap(content => {
        // Store in IndexedDB cache (background, non-blocking)
        if (this.idbInitialized && content.contentType !== 'placeholder') {
          this.idbCache.setContent(content).catch(() => {
            // Ignore IndexedDB errors
          });
        }
      }),
      catchError(err => {
        // Don't cache errors - return placeholder and don't cache this result
        console.warn(`[DataLoader] Error loading "${resourceId}":`, err.message || err);
        // Remove from cache so next request retries
        this.contentCache.delete(resourceId);
        return of(this.createPlaceholderContent(resourceId, err.message));
      }),
      shareReplay(1)
    );

    this.contentCache.set(resourceId, request);
    return request;
  }

  /**
   * Load content with unified cache resolution.
   *
   * Uses ContentResolver for intelligent tiered source selection.
   * Cache hierarchy:
   * 1. IndexedDB (local persistent cache)
   * 2. Projection API (Doorway's MongoDB cache - fast HTTP)
   * 3. Holochain (direct conductor - slow, authoritative)
   */
  private async loadContentWithIDBFallback(resourceId: string): Promise<ContentNode | null> {
    // Ensure ContentResolver is initialized
    if (!this.contentResolver.isReady) {
      await this.contentResolver.initialize();
    }

    const resolution = await this.contentResolver.resolveContent(resourceId);
    if (resolution) {
      // Cache in IndexedDB if loaded from remote source
      if (resolution.tier !== SourceTier.Local && this.idbInitialized) {
        this.contentResolver.cacheContent(resolution.data).catch(() => {});
      }
      return resolution.data;
    }
    return null;
  }

  /**
   * Batch load multiple content items efficiently.
   *
   * Uses a single zome call to fetch all content, then populates the cache.
   * Much more efficient than calling getContent() multiple times.
   *
   * Cache hierarchy:
   * 1. In-memory cache (checked individually)
   * 2. IndexedDB batch lookup
   * 3. Holochain batch zome call
   *
   * @param resourceIds Array of content IDs to load
   * @returns Observable of Map<id, ContentNode>
   */
  batchGetContent(resourceIds: string[]): Observable<Map<string, ContentNode>> {
    if (resourceIds.length === 0) {
      return of(new Map());
    }

    return defer(() => this.batchGetContentWithIDB(resourceIds)).pipe(
      timeout(this.CONTENT_TIMEOUT_MS * 2), // Allow more time for batch
      catchError(err => {
        console.warn('[DataLoader] Batch load error:', err);
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
   * Internal batch get with unified cache resolution.
   *
   * Uses ContentResolver for intelligent tiered source selection.
   * Cache hierarchy:
   * 1. In-memory cache (checked individually)
   * 2. ContentResolver (IndexedDB → Projection → Holochain)
   */
  private async batchGetContentWithIDB(resourceIds: string[]): Promise<Map<string, ContentNode>> {
    const contentMap = new Map<string, ContentNode>();
    const uncachedIds: string[] = [];

    // 1. First pass: check in-memory cache
    for (const id of resourceIds) {
      if (this.contentCache.has(id)) {
        try {
          const content = await this.contentCache.get(id)!.toPromise();
          if (content) {
            contentMap.set(id, content);
            continue;
          }
        } catch {
          // Continue to next cache layer
        }
      }
      uncachedIds.push(id);
    }

    if (uncachedIds.length === 0) {
      return contentMap;
    }

    // Ensure ContentResolver is initialized
    if (!this.contentResolver.isReady) {
      await this.contentResolver.initialize();
    }

    // 2. Use unified resolver for remaining IDs
    const resolved = await this.contentResolver.batchResolveContent(uncachedIds);
    const toCache: ContentNode[] = [];

    for (const [id, resolution] of resolved) {
      contentMap.set(id, resolution.data);
      this.contentCache.set(id, of(resolution.data).pipe(shareReplay(1)));
      // Queue for local caching if from remote source
      if (resolution.tier !== SourceTier.Local) {
        toCache.push(resolution.data);
      }
    }

    // Cache remotely-fetched content in IndexedDB
    if (this.idbInitialized && toCache.length > 0) {
      this.idbCache.setContentBatch(toCache).catch(() => {});
    }

    // Add placeholders for not found
    for (const id of uncachedIds) {
      if (!resolved.has(id)) {
        const placeholder = this.createPlaceholderContent(id);
        contentMap.set(id, placeholder);
      }
    }

    return contentMap;
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

  /**
   * Load the content index for search/discovery.
   * Returns metadata only, not full content.
   *
   * TODO: Implement getContentIndex in HolochainContentService
   */
  getContentIndex(): Observable<any> {
    // TODO: Implement content index via Holochain (get all content metadata)
    return this.holochainContent.getStats().pipe(
      map(stats => ({
        nodes: [],  // TODO: Fetch actual content list
        totalCount: stats.total_count,
        byType: stats.by_type,
        lastUpdated: new Date().toISOString()
      }))
    );
  }

  /**
   * Load the path index for discovery.
   * Uses Holochain as the only source.
   * Cached with shareReplay(1) to prevent redundant Holochain calls.
   */
  getPathIndex(): Observable<PathIndex> {
    if (!this.pathIndexCache$) {
      this.pathIndexCache$ = defer(() =>
        from(this.holochainContent.getPathIndex())
      ).pipe(
        map(hcIndex => this.transformHolochainPathIndex(hcIndex)),
        shareReplay(1),
        catchError(err => {
          console.error('[DataLoader] Failed to load path index:', err);
          // Clear cache on error so next call retries
          this.pathIndexCache$ = null;
          return of({ paths: [], totalCount: 0, lastUpdated: new Date().toISOString() });
        })
      );
    }
    return this.pathIndexCache$;
  }

  /**
   * Invalidate the path index cache.
   * Call this after creating/updating/deleting paths.
   */
  invalidatePathIndexCache(): void {
    this.pathIndexCache$ = null;
  }

  /**
   * Transform Holochain path index to PathIndex model.
   */
  private transformHolochainPathIndex(hcIndex: HolochainPathIndex): PathIndex {
    return {
      lastUpdated: hcIndex.last_updated,
      totalCount: hcIndex.total_count,
      paths: hcIndex.paths.map(p => ({
        id: p.id,
        title: p.title,
        description: p.description,
        difficulty: p.difficulty as any,
        estimatedDuration: p.estimated_duration ?? '',
        stepCount: p.step_count,
        tags: p.tags,
      })),
    };
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
        console.warn(`[DataLoader] Failed to load agent "${agentId}":`, err);
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
    indexedDBAvailable: boolean;
  } {
    return {
      pathCacheSize: this.pathCache.size,
      contentCacheSize: this.contentCache.size,
      relationshipCacheSize: this.relationshipByNodeCache.size,
      hasGraph: this.graphCache$ !== null,
      hasPathIndex: this.pathIndexCache$ !== null,
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
      map(results => results.map(r => this.transformHolochainContentAttestation(r.content_attestation))),
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
      agent_id: agentId,
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
    let earnedVia: AgentAttestation['earnedVia'] = {};
    try {
      earnedVia = JSON.parse(hcAtt.earned_via_json || '{}');
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcAtt.id,
      agentId: hcAtt.agent_id,
      category: hcAtt.category as AgentAttestation['category'],
      attestationType: hcAtt.attestation_type,
      displayName: hcAtt.display_name,
      description: hcAtt.description,
      iconUrl: hcAtt.icon_url ?? undefined,
      tier: hcAtt.tier as AgentAttestation['tier'],
      earnedVia,
      issuedAt: hcAtt.issued_at,
      issuedBy: hcAtt.issued_by,
      expiresAt: hcAtt.expires_at ?? undefined,
      proof: hcAtt.proof ?? undefined,
    };
  }

  /**
   * Transform Holochain content attestation entry to frontend ContentAttestation model.
   */
  private transformHolochainContentAttestation(hcAtt: HolochainContentAttestationEntry): ContentAttestation {
    let grantedBy: ContentAttestation['grantedBy'] = { type: 'system', grantorId: 'unknown' };
    let revocation: ContentAttestation['revocation'] = undefined;
    let evidence: ContentAttestation['evidence'] = undefined;
    let scope: ContentAttestation['scope'] = undefined;
    let metadata: ContentAttestation['metadata'] = {};

    try {
      grantedBy = JSON.parse(hcAtt.granted_by_json || '{}');
    } catch { /* ignore */ }

    try {
      if (hcAtt.revocation_json) {
        revocation = JSON.parse(hcAtt.revocation_json);
      }
    } catch { /* ignore */ }

    try {
      if (hcAtt.evidence_json) {
        evidence = JSON.parse(hcAtt.evidence_json);
      }
    } catch { /* ignore */ }

    try {
      if (hcAtt.scope_json) {
        scope = JSON.parse(hcAtt.scope_json);
      }
    } catch { /* ignore */ }

    try {
      metadata = JSON.parse(hcAtt.metadata_json || '{}');
    } catch { /* ignore */ }

    return {
      id: hcAtt.id,
      contentId: hcAtt.content_id,
      attestationType: hcAtt.attestation_type as ContentAttestation['attestationType'],
      reachGranted: hcAtt.reach_granted as ContentAttestation['reachGranted'],
      grantedBy,
      grantedAt: hcAtt.granted_at,
      expiresAt: hcAtt.expires_at ?? undefined,
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
        const attestations = results.map(r => this.transformHolochainContentAttestation(r.content_attestation));
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
      displayName: hcAgent.display_name,
      type: hcAgent.agent_type as Agent['type'],
      bio: hcAgent.bio ?? undefined,
      avatar: hcAgent.avatar ?? undefined,
      visibility: hcAgent.visibility as Agent['visibility'],
      createdAt: hcAgent.created_at,
      updatedAt: hcAgent.updated_at,
      did: hcAgent.did ?? undefined,
      activityPubType: hcAgent.activity_pub_type as Agent['activityPubType'],
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
        maps: results.map(r => this.transformHolochainKnowledgeMapToIndex(r.knowledge_map))
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
      map(result => result ? this.transformHolochainKnowledgeMap(result.knowledge_map) : null),
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
    map_type: string;
    owner_id: string;
    title: string;
    subject_type: string;
    subject_id: string;
    subject_name: string;
    visibility: string;
    nodes_json: string;
    overall_affinity: number;
    updated_at: string;
  }): KnowledgeMapIndexEntry {
    let nodes: unknown[] = [];
    try {
      nodes = hcMap.nodes_json ? JSON.parse(hcMap.nodes_json) : [];
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcMap.id,
      mapType: hcMap.map_type as KnowledgeMapType,
      title: hcMap.title,
      subjectName: hcMap.subject_name,
      ownerId: hcMap.owner_id,
      ownerName: '', // Would need to look up agent name
      visibility: hcMap.visibility,
      overallAffinity: hcMap.overall_affinity,
      nodeCount: Array.isArray(nodes) ? nodes.length : 0,
      updatedAt: hcMap.updated_at
    };
  }

  /**
   * Transform Holochain knowledge map entry to full KnowledgeMap model.
   */
  private transformHolochainKnowledgeMap(hcMap: {
    id: string;
    map_type: string;
    owner_id: string;
    title: string;
    description: string | null;
    subject_type: string;
    subject_id: string;
    subject_name: string;
    visibility: string;
    shared_with_json: string;
    nodes_json: string;
    path_ids_json: string;
    overall_affinity: number;
    content_graph_id: string | null;
    mastery_levels_json: string;
    goals_json: string;
    created_at: string;
    updated_at: string;
    metadata_json: string;
  }): KnowledgeMap {
    let nodes: KnowledgeNode[] = [];
    let pathIds: string[] = [];
    let sharedWith: string[] = [];
    let metadata: Record<string, unknown> = {};

    try {
      nodes = hcMap.nodes_json ? JSON.parse(hcMap.nodes_json) : [];
      pathIds = hcMap.path_ids_json ? JSON.parse(hcMap.path_ids_json) : [];
      sharedWith = hcMap.shared_with_json ? JSON.parse(hcMap.shared_with_json) : [];
      metadata = hcMap.metadata_json ? JSON.parse(hcMap.metadata_json) : {};
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcMap.id,
      mapType: hcMap.map_type as KnowledgeMapType,
      subject: {
        type: hcMap.subject_type as 'content-graph' | 'agent' | 'organization',
        subjectId: hcMap.subject_id,
        subjectName: hcMap.subject_name
      },
      ownerId: hcMap.owner_id,
      title: hcMap.title,
      description: hcMap.description ?? undefined,
      visibility: hcMap.visibility as 'private' | 'mutual' | 'shared' | 'public',
      sharedWith,
      nodes,
      pathIds,
      overallAffinity: hcMap.overall_affinity,
      createdAt: hcMap.created_at,
      updatedAt: hcMap.updated_at,
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
        extensions: results.map(r => this.transformHolochainPathExtensionToIndex(r.path_extension))
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
      map(result => result ? this.transformHolochainPathExtension(result.path_extension) : null),
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

    return defer(() => from(this.holochainContent.queryPathExtensions({ base_path_id: pathId }))).pipe(
      map(results => results.map(r => this.transformHolochainPathExtension(r.path_extension))),
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
    base_path_id: string;
    base_path_version: string;
    extended_by: string;
    title: string;
    description: string | null;
    visibility: string;
    insertions_json: string;
    annotations_json: string;
    updated_at: string;
  }): PathExtensionIndexEntry {
    let insertionCount = 0;
    let annotationCount = 0;
    try {
      const insertions = hcExt.insertions_json ? JSON.parse(hcExt.insertions_json) : [];
      const annotations = hcExt.annotations_json ? JSON.parse(hcExt.annotations_json) : [];
      insertionCount = Array.isArray(insertions) ? insertions.length : 0;
      annotationCount = Array.isArray(annotations) ? annotations.length : 0;
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcExt.id,
      basePathId: hcExt.base_path_id,
      basePathTitle: '', // Would need to look up path title
      title: hcExt.title,
      description: hcExt.description ?? undefined,
      extendedBy: hcExt.extended_by,
      extenderName: '', // Would need to look up agent name
      visibility: hcExt.visibility,
      insertionCount,
      annotationCount,
      forkCount: 0, // Would need separate query
      updatedAt: hcExt.updated_at
    };
  }

  /**
   * Transform Holochain path extension entry to full PathExtension model.
   */
  private transformHolochainPathExtension(hcExt: {
    id: string;
    base_path_id: string;
    base_path_version: string;
    extended_by: string;
    title: string;
    description: string | null;
    visibility: string;
    shared_with_json: string;
    insertions_json: string;
    annotations_json: string;
    reorderings_json: string;
    exclusions_json: string;
    forked_from: string | null;
    forks_json: string;
    upstream_proposal_json: string | null;
    stats_json: string;
    created_at: string;
    updated_at: string;
  }): PathExtension {
    let sharedWith: string[] = [];
    let insertions: PathStepInsertion[] = [];
    let annotations: PathStepAnnotation[] = [];
    let reorderings: PathStepReorder[] = [];
    let exclusions: PathStepExclusion[] = [];
    let forks: string[] = [];
    let upstreamProposal: UpstreamProposal | undefined;
    let stats: ExtensionStats | undefined;

    try {
      sharedWith = hcExt.shared_with_json ? JSON.parse(hcExt.shared_with_json) : [];
      insertions = hcExt.insertions_json ? JSON.parse(hcExt.insertions_json) : [];
      annotations = hcExt.annotations_json ? JSON.parse(hcExt.annotations_json) : [];
      reorderings = hcExt.reorderings_json ? JSON.parse(hcExt.reorderings_json) : [];
      exclusions = hcExt.exclusions_json ? JSON.parse(hcExt.exclusions_json) : [];
      forks = hcExt.forks_json ? JSON.parse(hcExt.forks_json) : [];
      upstreamProposal = hcExt.upstream_proposal_json ? JSON.parse(hcExt.upstream_proposal_json) : undefined;
      stats = hcExt.stats_json ? JSON.parse(hcExt.stats_json) : undefined;
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcExt.id,
      basePathId: hcExt.base_path_id,
      basePathVersion: hcExt.base_path_version,
      extendedBy: hcExt.extended_by,
      title: hcExt.title,
      description: hcExt.description ?? undefined,
      insertions,
      annotations,
      reorderings,
      exclusions,
      visibility: hcExt.visibility as 'private' | 'shared' | 'public',
      sharedWith,
      forkedFrom: hcExt.forked_from ?? undefined,
      forks,
      upstreamProposal,
      stats,
      createdAt: hcExt.created_at,
      updatedAt: hcExt.updated_at
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
      from(this.holochainContent.getRelationships({ content_id: contentId, direction }))
    ).pipe(
      map(results => results.map(r => this.transformHolochainRelationship(r.relationship)))
    );
  }

  /**
   * Transform Holochain relationship entry to frontend ContentRelationship model.
   */
  private transformHolochainRelationship(
    hcRel: { id: string; source_id: string; target_id: string; relationship_type: string; confidence: number; metadata_json: string | null }
  ): ContentRelationship {
    let metadata: Record<string, unknown> = {};
    try {
      metadata = JSON.parse(hcRel.metadata_json || '{}');
    } catch {
      // Ignore parse errors
    }

    // Store confidence in metadata since ContentRelationship doesn't have a confidence field
    if (hcRel.confidence !== undefined && hcRel.confidence !== null) {
      metadata['confidence'] = hcRel.confidence;
    }

    return {
      id: hcRel.id,
      sourceNodeId: hcRel.source_id,
      targetNodeId: hcRel.target_id,
      relationshipType: hcRel.relationship_type as ContentRelationshipType,
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
        nodeCount: hcGraph.total_nodes,
        relationshipCount: relationships.size,
        lastUpdated: new Date().toISOString(),
        version: '1.0.0'
      }
    };
  }

  /**
   * Transform HolochainContentOutput to ContentNode.
   */
  private transformHolochainContentToNode(output: { content: { id: string; content_type: string; title: string; description: string; content: string; content_format: string; tags: string[]; source_path: string | null; related_node_ids: string[]; metadata_json: string; created_at: string; updated_at: string } }): ContentNode {
    const entry = output.content;
    let metadata = {};
    try {
      metadata = JSON.parse(entry.metadata_json || '{}');
    } catch {
      // Ignore parse errors
    }

    return {
      id: entry.id,
      contentType: entry.content_type as ContentNode['contentType'],
      title: entry.title,
      description: entry.description,
      content: entry.content,
      contentFormat: entry.content_format as ContentNode['contentFormat'],
      tags: entry.tags,
      sourcePath: entry.source_path ?? undefined,
      relatedNodeIds: entry.related_node_ids,
      metadata,
      createdAt: entry.created_at,
      updatedAt: entry.updated_at,
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
          relationshipType: graphNode.relationship_type as ContentRelationshipType
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

  /**
   * Build ContentGraph structure from raw data.
   */
  private buildContentGraph(
    metadata: ContentGraphMetadata,
    contentIndex: { nodes?: ContentNode[] },
    relationshipData: { relationships: Array<{ id: string; source: string; target: string; type: string }> }
  ): ContentGraph {
    const nodes = new Map<string, ContentNode>();
    const nodesByType = new Map<string, Set<string>>();
    const nodesByTag = new Map<string, Set<string>>();
    const nodesByCategory = new Map<string, Set<string>>();
    const adjacency = new Map<string, Set<string>>();
    const reverseAdjacency = new Map<string, Set<string>>();

    this.indexNodes(contentIndex.nodes || [], nodes, nodesByType, nodesByTag, nodesByCategory, adjacency, reverseAdjacency);
    const relationshipsMap = this.buildRelationships(relationshipData.relationships || [], nodes, adjacency, reverseAdjacency);

    return { nodes, relationships: relationshipsMap, nodesByType, nodesByTag, nodesByCategory, adjacency, reverseAdjacency, metadata };
  }

  private indexNodes(
    nodeList: ContentNode[],
    nodes: Map<string, ContentNode>,
    nodesByType: Map<string, Set<string>>,
    nodesByTag: Map<string, Set<string>>,
    nodesByCategory: Map<string, Set<string>>,
    adjacency: Map<string, Set<string>>,
    reverseAdjacency: Map<string, Set<string>>
  ): void {
    for (const node of nodeList) {
      nodes.set(node.id, node);
      this.addToSetMap(nodesByType, node.contentType, node.id);
      for (const tag of node.tags || []) {
        this.addToSetMap(nodesByTag, tag, node.id);
      }
      const category = (node.metadata as any)?.category ?? 'uncategorized';
      this.addToSetMap(nodesByCategory, category, node.id);
      adjacency.set(node.id, new Set());
      reverseAdjacency.set(node.id, new Set());
    }
  }

  private addToSetMap(map: Map<string, Set<string>>, key: string, value: string): void {
    if (!map.has(key)) {
      map.set(key, new Set());
    }
    map.get(key)!.add(value);
  }

  private buildRelationships(
    relationships: Array<{ id: string; source: string; target: string; type: string }>,
    nodes: Map<string, ContentNode>,
    adjacency: Map<string, Set<string>>,
    reverseAdjacency: Map<string, Set<string>>
  ): Map<string, any> {
    const relationshipsMap = new Map<string, any>();

    for (const rel of relationships) {
      const relId = rel.id || `${rel.source}-${rel.target}`;
      relationshipsMap.set(relId, { id: relId, sourceId: rel.source, targetId: rel.target, type: rel.type });

      adjacency.get(rel.source)?.add(rel.target);
      reverseAdjacency.get(rel.target)?.add(rel.source);

      const sourceNode = nodes.get(rel.source);
      const targetNode = nodes.get(rel.target);
      if (sourceNode && !sourceNode.relatedNodeIds.includes(rel.target)) {
        sourceNode.relatedNodeIds.push(rel.target);
      }
      if (targetNode && !targetNode.relatedNodeIds.includes(rel.source)) {
        targetNode.relatedNodeIds.push(rel.source);
      }
    }

    return relationshipsMap;
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
      entity_type: entityType,
      entity_id: entityId
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
    entity_type: string;
    entity_id: string;
    challenger_id: string;
    challenger_name: string;
    challenger_standing: string;
    grounds: string;
    description: string;
    evidence_json: string;
    status: string;
    filed_at: string;
    sla_deadline: string | null;
    assigned_elohim: string | null;
    resolution_json: string | null;
  }): ChallengeRecord {
    let resolution: ChallengeRecord['resolution'];
    try {
      resolution = hcChallenge.resolution_json ? JSON.parse(hcChallenge.resolution_json) : undefined;
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcChallenge.id,
      entityType: hcChallenge.entity_type,
      entityId: hcChallenge.entity_id,
      challenger: {
        agentId: hcChallenge.challenger_id,
        displayName: hcChallenge.challenger_name,
        standing: hcChallenge.challenger_standing
      },
      grounds: hcChallenge.grounds,
      description: hcChallenge.description,
      status: hcChallenge.status,
      filedAt: hcChallenge.filed_at,
      slaDeadline: hcChallenge.sla_deadline ?? undefined,
      assignedElohim: hcChallenge.assigned_elohim ?? undefined,
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
    proposal_type: string;
    description: string;
    proposer_id: string;
    proposer_name: string;
    status: string;
    phase: string;
    voting_config_json: string;
    current_votes_json: string;
    outcome_json: string | null;
    created_at: string;
  }): ProposalRecord {
    let votingConfig: ProposalRecord['votingConfig'];
    let currentVotes: ProposalRecord['currentVotes'];
    let outcome: ProposalRecord['outcome'];

    try {
      votingConfig = hcProposal.voting_config_json ? JSON.parse(hcProposal.voting_config_json) : undefined;
      currentVotes = hcProposal.current_votes_json ? JSON.parse(hcProposal.current_votes_json) : undefined;
      outcome = hcProposal.outcome_json ? JSON.parse(hcProposal.outcome_json) : undefined;
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcProposal.id,
      title: hcProposal.title,
      proposalType: hcProposal.proposal_type,
      description: hcProposal.description,
      proposer: {
        agentId: hcProposal.proposer_id,
        displayName: hcProposal.proposer_name
      },
      status: hcProposal.status,
      phase: hcProposal.phase,
      createdAt: hcProposal.created_at,
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
    full_reasoning: string;
    binding: string;
    scope_json: string;
    citations: number;
    status: string;
  }): PrecedentRecord {
    let scope: PrecedentRecord['scope'] = { entityTypes: [] };
    try {
      scope = hcPrecedent.scope_json ? JSON.parse(hcPrecedent.scope_json) : { entityTypes: [] };
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcPrecedent.id,
      title: hcPrecedent.title,
      summary: hcPrecedent.summary,
      fullReasoning: hcPrecedent.full_reasoning,
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
      entity_type: entityType,
      entity_id: entityId
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
    entity_type: string;
    entity_id: string;
    category: string;
    title: string;
    messages_json: string;
    status: string;
    message_count: number;
  }): DiscussionRecord {
    let messages: DiscussionRecord['messages'] = [];
    try {
      messages = hcDiscussion.messages_json ? JSON.parse(hcDiscussion.messages_json) : [];
    } catch {
      // Ignore parse errors
    }

    return {
      id: hcDiscussion.id,
      entityType: hcDiscussion.entity_type,
      entityId: hcDiscussion.entity_id,
      category: hcDiscussion.category,
      title: hcDiscussion.title,
      messages,
      status: hcDiscussion.status,
      messageCount: hcDiscussion.message_count
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
      entity_type: entityType,
      entity_id: entityId
    }))).pipe(
      map(result => result ? this.transformHolochainGovernanceState(result.governance_state) : null),
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
    entity_type: string;
    entity_id: string;
    status: string;
    status_basis_json: string;
    labels_json: string;
    active_challenges_json: string;
    last_updated: string;
  }): GovernanceStateRecord {
    let statusBasis: GovernanceStateRecord['statusBasis'] = {
      method: '',
      reasoning: '',
      deciderId: '',
      deciderType: '',
      decidedAt: ''
    };
    let labels: GovernanceStateRecord['labels'] = [];
    let activeChallenges: string[] = [];

    try {
      statusBasis = hcState.status_basis_json ? JSON.parse(hcState.status_basis_json) : statusBasis;
      labels = hcState.labels_json ? JSON.parse(hcState.labels_json) : [];
      activeChallenges = hcState.active_challenges_json ? JSON.parse(hcState.active_challenges_json) : [];
    } catch {
      // Ignore parse errors
    }

    return {
      entityType: hcState.entity_type,
      entityId: hcState.entity_id,
      status: hcState.status,
      statusBasis,
      labels,
      activeChallenges,
      lastUpdated: hcState.last_updated
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
