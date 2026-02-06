/**
 * Content Service
 *
 * Domain-focused content operations using ElohimClient.
 * This service handles content retrieval and caching,
 * while ElohimClient handles the mode-aware backend routing.
 *
 * This is the NEW pattern - compare with ProjectionAPIService
 * which uses HttpClient directly.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

// @coverage: 83.1% (2026-02-05)

import { map, catchError, shareReplay, switchMap } from 'rxjs/operators';

import { Observable, from, of } from 'rxjs';

import { ContentNode, ContentType, ContentReach } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';
import { ELOHIM_CLIENT, ElohimClient } from '../providers/elohim-client.provider';

import { StorageClientService } from './storage-client.service';

import type { KnowledgeMap, KnowledgeMapType } from '../../lamad/models/knowledge-map.model';
import type { PathExtension, UpstreamProposal } from '../../lamad/models/path-extension.model';
import type { ContentQuery } from '@elohim/service/client';

/**
 * Content query filters
 */
export interface ContentFilters {
  contentType?: ContentType | ContentType[];
  tags?: string[];
  reach?: ContentReach;
  search?: string;
  limit?: number;
  offset?: number;
}

/**
 * Path query filters
 */
export interface PathFilters {
  difficulty?: 'beginner' | 'intermediate' | 'advanced';
  visibility?: 'public' | 'private';
  tags?: string[];
  search?: string;
  limit?: number;
  offset?: number;
}

// =============================================================================
// Relationship/Graph Types
// =============================================================================

/**
 * Content relationship (edge in knowledge graph)
 */
export interface Relationship {
  id: string;
  sourceId: string;
  targetId: string;
  relationshipType: string; // RELATES_TO, CONTAINS, DEPENDS_ON, IMPLEMENTS, REFERENCES
  confidence: number;
  inferenceSource: string; // explicit, path, tag, semantic
  metadata?: Record<string, any>;
  createdAt?: string;
}

/**
 * Content graph node
 */
export interface ContentGraphNode {
  contentId: string;
  relationshipType: string;
  confidence: number;
  children: ContentGraphNode[];
}

/**
 * Content graph from a root node
 */
export interface ContentGraph {
  rootId: string;
  related: ContentGraphNode[];
  totalNodes: number;
}

// =============================================================================
// Knowledge Map Types
// =============================================================================

// Re-export canonical KnowledgeMap from lamad module
export type { KnowledgeMap } from '../../lamad/models/knowledge-map.model';

/**
 * Knowledge map query filters
 */
export interface KnowledgeMapFilters {
  ownerId?: string;
  mapType?: string;
  subjectId?: string;
  visibility?: string;
  limit?: number;
  offset?: number;
}

// =============================================================================
// Path Extension Types
// =============================================================================

// Re-export canonical types from lamad module
export type { PathExtension } from '../../lamad/models/path-extension.model';
export type {
  PathStepInsertion,
  PathStepAnnotation,
  PathStepReorder,
  PathStepExclusion,
} from '../../lamad/models/path-extension.model';

/**
 * Path extension query filters
 */
export interface PathExtensionFilters {
  basePathId?: string;
  extendedBy?: string;
  visibility?: string;
  limit?: number;
  offset?: number;
}

// =============================================================================
// Raw API Response Types (for type-safe transforms)
// =============================================================================

/** Raw content data from storage/API before transformation */
interface RawContentData {
  id: string;
  docId?: string;
  contentType?: string;
  title?: string;
  description?: string;
  contentBody?: string;
  content?: string;
  contentFormat?: string;
  tags?: string[];
  relatedNodeIds?: string[];
  metadata?: Record<string, unknown>;
  authorId?: string;
  reach?: string;
  trustScore?: number;
  estimatedMinutes?: number;
  thumbnailUrl?: string | null;
  blobCid?: string;
  createdAt?: string;
  updatedAt?: string;
}

/** Raw path step from storage/API */
interface RawStepData {
  id?: string;
  pathId?: string;
  chapterId?: string;
  title?: string;
  description?: string;
  stepType?: string;
  resourceId?: string;
  resourceType?: string;
  orderIndex?: number;
  estimatedDuration?: string;
  metadata?: Record<string, unknown>;
}

/** Raw chapter from storage/API */
interface RawChapterData {
  id?: string;
  title?: string;
  description?: string;
  orderIndex?: number;
  estimatedDuration?: string;
  steps?: RawStepData[];
}

/** Raw path data from storage/API before transformation */
interface RawPathData {
  id: string;
  docId?: string;
  version?: string;
  title?: string;
  description?: string;
  purpose?: string;
  difficulty?: string;
  estimatedDuration?: string;
  visibility?: string;
  pathType?: string;
  thumbnailUrl?: string | null;
  thumbnailAlt?: string;
  tags?: string[];
  createdBy?: string;
  contributors?: string[];
  steps?: RawStepData[];
  chapters?: RawChapterData[];
  stepCount?: number;
  chapterCount?: number;
  createdAt?: string;
  updatedAt?: string;
  metadata?: Record<string, unknown>;
  path?: RawPathData;
  ungroupedSteps?: RawStepData[];
}

/** Raw relationship from storage/API */
interface RawRelationshipData {
  id?: string;
  sourceId?: string;
  targetId?: string;
  relationshipType?: string;
  confidence?: number;
  inferenceSource?: string;
  metadata?: Record<string, unknown>;
  createdAt?: string;
}

/** Raw content graph node */
interface RawContentGraphNodeData {
  contentId?: string;
  relationshipType?: string;
  confidence?: number;
  children?: RawContentGraphNodeData[];
}

/** Raw content graph from storage/API */
interface RawContentGraphData {
  rootId?: string;
  related?: RawContentGraphNodeData[];
  totalNodes?: number;
}

/** Raw knowledge map from storage/API */
interface RawKnowledgeMapData {
  id: string;
  mapType?: string;
  ownerId?: string;
  title?: string;
  description?: string;
  subjectType?: string;
  subjectId?: string;
  subjectName?: string;
  visibility?: string;
  sharedWith?: string[];
  nodes?: KnowledgeMap['nodes'];
  pathIds?: string[];
  overallAffinity?: number;
  metadata?: Record<string, unknown>;
  createdAt?: string;
  updatedAt?: string;
}

/** Raw path extension from storage/API */
interface RawPathExtensionData {
  id: string;
  basePathId?: string;
  basePathVersion?: string;
  extendedBy?: string;
  title?: string;
  description?: string;
  insertions?: PathExtension['insertions'];
  annotations?: PathExtension['annotations'];
  reorderings?: PathExtension['reorderings'];
  exclusions?: PathExtension['exclusions'];
  visibility?: string;
  sharedWith?: string[];
  forkedFrom?: string;
  forks?: string[];
  upstreamProposal?: UpstreamProposal;
  stats?: PathExtension['stats'];
  createdAt?: string;
  updatedAt?: string;
}

@Injectable({ providedIn: 'root' })
export class ContentService {
  private readonly client: ElohimClient = inject(ELOHIM_CLIENT);
  private readonly storageClient = inject(StorageClientService);
  private readonly http = inject(HttpClient);

  // In-memory cache for hot paths
  private readonly contentCache = new Map<string, Observable<ContentNode | null>>();
  private readonly pathCache = new Map<string, Observable<LearningPath | null>>();
  private readonly blobCache = new Map<string, Observable<string>>();

  // =========================================================================
  // Content Operations
  // =========================================================================

  /**
   * Get a single content node by ID
   * Automatically fetches blob content when contentBody is null but blobCid exists
   */
  getContent(id: string): Observable<ContentNode | null> {
    // Check cache first
    const cached = this.contentCache.get(id);
    if (cached) return cached;

    const obs = from(this.client.get<RawContentData>('content', id)).pipe(
      switchMap(data => {
        if (!data) return of(null);

        // Check if we need to fetch blob content
        // contentBody may be a blob reference (sha256:... or sha256-...) instead of actual content
        const contentBody = data.contentBody ?? '';

        const isBlobReference =
          contentBody.startsWith('sha256:') || contentBody.startsWith('sha256-');
        const blobCid = isBlobReference ? contentBody : (data.blobCid ?? undefined);
        const needsBlobFetch = isBlobReference || (!contentBody && data.blobCid);

        if (needsBlobFetch && blobCid) {
          return this.fetchBlobContent(blobCid).pipe(
            map(blobContent => {
              // Inject blob content as contentBody
              return this.transformContent({ ...data, contentBody: blobContent });
            }),
            catchError(_err => {
              // Fall back to transforming without blob content
              return of(this.transformContent(data));
            })
          );
        }

        return of(this.transformContent(data));
      }),
      catchError(_err => {
        return of(null);
      }),
      shareReplay(1)
    );

    this.contentCache.set(id, obs);
    return obs;
  }

  /**
   * Fetch blob content by CID from storage
   */
  private fetchBlobContent(blobCid: string): Observable<string> {
    // Normalize hash format: storage expects sha256- (hyphen) not sha256: (colon)
    const normalizedCid = blobCid.replace('sha256:', 'sha256-');

    // Check cache first
    const cached = this.blobCache.get(normalizedCid);
    if (cached) return cached;

    // Use StorageClientService to get the correct blob URL (handles doorway vs direct mode)
    const blobUrl = this.storageClient.getBlobUrl(normalizedCid);

    const obs = this.http.get(blobUrl, { responseType: 'text' }).pipe(shareReplay(1));

    this.blobCache.set(normalizedCid, obs);
    return obs;
  }

  /**
   * Query content with filters
   */
  queryContent(filters: ContentFilters): Observable<ContentNode[]> {
    const query: ContentQuery = {
      contentType: 'content',
      tags: filters.tags,
      search: filters.search,
      limit: filters.limit,
      offset: filters.offset,
    };

    return from(this.client.query<RawContentData>(query)).pipe(
      map(items => items.map(c => this.transformContent(c))),
      map(items => this.applyLocalFilters(items, filters)),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Batch get multiple content nodes
   */
  batchGetContent(ids: string[]): Observable<Map<string, ContentNode>> {
    return from(this.client.getBatch<RawContentData>('content', ids)).pipe(
      map(results => {
        const map = new Map<string, ContentNode>();
        for (const [id, content] of results) {
          map.set(id, this.transformContent(content));
        }
        return map;
      }),
      catchError(_err => {
        return of(new Map());
      })
    );
  }

  /**
   * Search content with full-text query
   */
  searchContent(query: string, limit = 50): Observable<ContentNode[]> {
    return this.queryContent({
      search: query,
      limit,
      reach: 'commons',
    });
  }

  // =========================================================================
  // Path Operations
  // =========================================================================

  /**
   * Get a single learning path by ID
   */
  getPath(id: string): Observable<LearningPath | null> {
    // Check cache first
    const cached = this.pathCache.get(id);
    if (cached) return cached;

    const obs = from(this.client.get<RawPathData>('path', id)).pipe(
      map(data => (data ? this.transformPath(data) : null)),
      catchError(_err => {
        return of(null);
      }),
      shareReplay(1)
    );

    this.pathCache.set(id, obs);
    return obs;
  }

  /**
   * Query paths with filters
   */
  queryPaths(filters: PathFilters): Observable<LearningPath[]> {
    const query: ContentQuery = {
      contentType: 'path',
      tags: filters.tags,
      search: filters.search,
      limit: filters.limit,
      offset: filters.offset,
    };

    return from(this.client.query<RawPathData>(query)).pipe(
      map(items => items.map(p => this.transformPath(p))),
      map(items => this.applyPathFilters(items, filters)),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get all public paths
   */
  getAllPaths(limit = 100): Observable<LearningPath[]> {
    return this.queryPaths({ visibility: 'public', limit });
  }

  // =========================================================================
  // Relationship/Graph Operations
  // =========================================================================

  /**
   * Get relationships for a content node
   */
  getRelationships(
    contentId: string,
    direction: 'outgoing' | 'incoming' | 'both' = 'both',
    relationshipType?: string
  ): Observable<Relationship[]> {
    const params = new URLSearchParams({
      contentId: contentId,
      direction,
    });
    if (relationshipType) {
      params.set('relationshipType', relationshipType);
    }

    return from(
      this.client.fetch<{ items: RawRelationshipData[] }>(`/db/relationships?${params}`)
    ).pipe(
      map(response => (response?.items ?? []).map(r => this.transformRelationship(r))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  /**
   * Get content graph starting from a root node
   */
  getContentGraph(
    contentId: string,
    relationshipTypes?: string[]
  ): Observable<ContentGraph | null> {
    let url = `/db/relationships/graph/${contentId}`;
    if (relationshipTypes?.length) {
      url += `?types=${relationshipTypes.join(',')}`;
    }

    return from(this.client.fetch<RawContentGraphData>(url)).pipe(
      map(data => (data ? this.transformContentGraph(data) : null)),
      catchError(_err => {
        return of(null);
      })
    );
  }

  // =========================================================================
  // Knowledge Map Operations
  // =========================================================================

  /**
   * Get a knowledge map by ID
   */
  getKnowledgeMap(id: string): Observable<KnowledgeMap | null> {
    return from(this.client.fetch<RawKnowledgeMapData>(`/db/knowledge-maps/${id}`)).pipe(
      map(data => (data ? this.transformKnowledgeMap(data) : null)),
      catchError(_err => {
        return of(null);
      })
    );
  }

  /**
   * Query knowledge maps
   */
  queryKnowledgeMaps(filters: KnowledgeMapFilters): Observable<KnowledgeMap[]> {
    const params = new URLSearchParams();
    if (filters.ownerId) params.set('ownerId', filters.ownerId);
    if (filters.mapType) params.set('mapType', filters.mapType);
    if (filters.subjectId) params.set('subjectId', filters.subjectId);
    if (filters.visibility) params.set('visibility', filters.visibility);
    if (filters.limit) params.set('limit', String(filters.limit));
    if (filters.offset) params.set('offset', String(filters.offset));

    return from(
      this.client.fetch<{ items: RawKnowledgeMapData[] }>(`/db/knowledge-maps?${params}`)
    ).pipe(
      map(response => (response?.items ?? []).map(m => this.transformKnowledgeMap(m))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  // =========================================================================
  // Path Extension Operations
  // =========================================================================

  /**
   * Get a path extension by ID
   */
  getPathExtension(id: string): Observable<PathExtension | null> {
    return from(this.client.fetch<RawPathExtensionData>(`/db/path-extensions/${id}`)).pipe(
      map(data => (data ? this.transformPathExtension(data) : null)),
      catchError(_err => {
        return of(null);
      })
    );
  }

  /**
   * Query path extensions
   */
  queryPathExtensions(filters: PathExtensionFilters): Observable<PathExtension[]> {
    const params = new URLSearchParams();
    if (filters.basePathId) params.set('basePathId', filters.basePathId);
    if (filters.extendedBy) params.set('extendedBy', filters.extendedBy);
    if (filters.visibility) params.set('visibility', filters.visibility);
    if (filters.limit) params.set('limit', String(filters.limit));
    if (filters.offset) params.set('offset', String(filters.offset));

    return from(
      this.client.fetch<{ items: RawPathExtensionData[] }>(`/db/path-extensions?${params}`)
    ).pipe(
      map(response => (response?.items ?? []).map(e => this.transformPathExtension(e))),
      catchError(_err => {
        return of([]);
      })
    );
  }

  // =========================================================================
  // Cache Management
  // =========================================================================

  /**
   * Clear the content cache
   */
  clearCache(): void {
    this.contentCache.clear();
    this.pathCache.clear();
  }

  /**
   * Invalidate a specific content item
   */
  invalidateContent(id: string): void {
    this.contentCache.delete(id);
  }

  /**
   * Invalidate a specific path
   */
  invalidatePath(id: string): void {
    this.pathCache.delete(id);
  }

  // =========================================================================
  // Client Info
  // =========================================================================

  /**
   * Check if offline mode is supported
   */
  supportsOffline(): boolean {
    return this.client.supportsOffline();
  }

  /**
   * Check current backpressure level
   */
  async backpressure(): Promise<number> {
    return this.client.backpressure();
  }

  // =========================================================================
  // Private Helpers
  // =========================================================================

  /**
   * Transform raw data to ContentNode model
   */
  private transformContent(data: RawContentData): ContentNode {
    const contentFormat = data.contentFormat ?? 'markdown';
    const rawContent = data.contentBody ?? data.content ?? '';

    return {
      id: data.id ?? data.docId,
      contentType: data.contentType,
      title: data.title ?? '',
      description: data.description ?? '',
      // Parse content for structured formats (html5-app, perseus, quiz-json, etc.)
      content: this.parseContentBody(rawContent, contentFormat),
      contentFormat,
      tags: data.tags ?? [],
      relatedNodeIds: data.relatedNodeIds ?? [],
      metadata: data.metadata ?? {},
      authorId: data.authorId,
      reach: data.reach ?? 'commons',
      trustScore: data.trustScore,
      estimatedMinutes: data.estimatedMinutes,
      thumbnailUrl: this.resolveBlobUrl(data.thumbnailUrl),
      createdAt: data.createdAt,
      updatedAt: data.updatedAt,
    } as ContentNode;
  }

  /**
   * Parse content body for structured formats.
   * Formats like html5-app, perseus, quiz-json store JSON objects as strings.
   */
  // eslint-disable-next-line sonarjs/function-return-type -- Intentionally returns string or parsed object
  private parseContentBody(content: string, contentFormat: string): string | object {
    // Formats that store structured JSON content
    const structuredFormats = [
      'html5-app',
      'perseus',
      'quiz-json',
      'assessment',
      'perseus-json',
      'perseus-quiz-json',
      'sophia-quiz-json',
      'sophia',
    ];

    if (!structuredFormats.includes(contentFormat) || !content) {
      return content;
    }

    // If content is already an object (pre-parsed), return as-is
    if (typeof content === 'object') {
      return content;
    }

    // Try to parse as JSON
    try {
      const parsed: unknown = JSON.parse(content);
      // Only return parsed object if it's actually an object
      if (parsed && typeof parsed === 'object') {
        return parsed;
      }
    } catch {
      // Not valid JSON, return as-is
    }

    return content;
  }

  /**
   * Transform raw data to LearningPath model
   *
   * Handles two response formats:
   * 1. Flat: { id, title, steps, chapters, ... }
   * 2. Nested: { path: {...}, chapters: [...], ungrouped_steps: [...] }
   * 3. Metadata: { path: {..., metadata: { chapters: [...] } } }
   */
  private transformPath(data: RawPathData): LearningPath {
    // Handle nested response format from elohim-storage
    const pathData = data.path ?? data;
    // Chapters can be at top-level, in path data, or in metadata
    const metadataChapters = (pathData.metadata?.['chapters'] ?? []) as RawChapterData[];
    const rawChapters = data.chapters ?? pathData.chapters ?? metadataChapters;
    const ungroupedSteps = data.ungroupedSteps ?? [];

    // Transform chapters with their steps
    const chapters = rawChapters.map((ch: RawChapterData) => ({
      id: ch.id,
      title: ch.title ?? '',
      description: ch.description ?? '',
      order: ch.orderIndex ?? 0,
      orderIndex: ch.orderIndex ?? 0,
      estimatedDuration: ch.estimatedDuration ?? ch.estimatedDuration,
      steps: (ch.steps ?? []).map((s: RawStepData) => this.transformStep(s)),
    }));

    // Collect all steps from chapters + ungrouped
    type TransformedChapter = (typeof chapters)[number];
    const allSteps = [
      ...chapters.flatMap((ch: TransformedChapter) => ch.steps),
      ...ungroupedSteps.map((s: RawStepData) => this.transformStep(s)),
    ];

    return {
      id: pathData.id ?? pathData.docId,
      version: pathData.version ?? '1.0.0',
      title: pathData.title ?? '',
      description: pathData.description ?? '',
      purpose: pathData.purpose ?? '',
      difficulty: pathData.difficulty ?? 'beginner',
      estimatedDuration: pathData.estimatedDuration ?? pathData.estimatedDuration,
      visibility: pathData.visibility ?? 'public',
      pathType: pathData.pathType ?? 'course',
      thumbnailUrl: this.resolveBlobUrl(pathData.thumbnailUrl),
      thumbnailAlt: pathData.thumbnailAlt,
      tags: pathData.tags ?? [],
      createdBy: pathData.createdBy ?? pathData.createdBy ?? '',
      contributors: pathData.contributors ?? [],
      steps: allSteps,
      chapters,
      stepCount: pathData.stepCount ?? pathData.stepCount ?? allSteps.length,
      chapterCount: pathData.chapterCount ?? pathData.chapterCount ?? chapters.length,
      createdAt: pathData.createdAt,
      updatedAt: pathData.updatedAt,
    } as LearningPath;
  }

  /**
   * Transform a step from snake_case to camelCase
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- Bridge type: raw storage steps have extra fields beyond PathStep
  private transformStep(step: RawStepData): any {
    return {
      id: step.id,
      pathId: step.pathId ?? step.pathId,
      chapterId: step.chapterId,
      // Map to both 'title' and 'stepTitle' for compatibility
      title: step.title ?? '',
      stepTitle: step.title ?? '',
      stepNarrative: step.description ?? '',
      description: step.description ?? '',
      stepType: step.stepType ?? 'content',
      resourceId: step.resourceId ?? step.resourceId ?? '',
      resourceType: step.resourceType ?? step.resourceType ?? 'content',
      order: step.orderIndex ?? 0,
      orderIndex: step.orderIndex ?? 0,
      estimatedDuration: step.estimatedDuration ?? step.estimatedDuration,
      metadata: step.metadata,
    };
  }

  /**
   * Apply local content filters (for fields not supported by backend query)
   */
  private applyLocalFilters(items: ContentNode[], filters: ContentFilters): ContentNode[] {
    let result = items;

    if (filters.contentType) {
      const types = Array.isArray(filters.contentType)
        ? filters.contentType
        : [filters.contentType];
      result = result.filter(c => types.includes(c.contentType));
    }

    if (filters.reach) {
      result = result.filter(c => c.reach === filters.reach);
    }

    return result;
  }

  /**
   * Apply local path filters
   */
  private applyPathFilters(items: LearningPath[], filters: PathFilters): LearningPath[] {
    let result = items;

    if (filters.difficulty) {
      result = result.filter(p => p.difficulty === filters.difficulty);
    }

    if (filters.visibility) {
      result = result.filter(p => p.visibility === filters.visibility);
    }

    return result;
  }

  /**
   * Resolve a blob reference to a full URL.
   *
   * Handles multiple formats:
   * - Full URL (https://...): pass through unchanged
   * - Relative asset path (/images/..., /assets/...): pass through unchanged
   * - Relative path (/blob/sha256-...): extract hash, build URL
   * - Blob hash (sha256-...): build URL directly
   * - Null/undefined: return undefined
   *
   * Uses StorageClientService for strategy-aware URL construction
   * (doorway vs direct/tauri mode).
   */
  private resolveBlobUrl(value: string | null | undefined): string | undefined {
    if (!value) return undefined;

    // Already a full URL - pass through
    if (value.startsWith('http://') || value.startsWith('https://')) {
      return value;
    }

    // Relative asset paths (static assets served by Angular or CDN) - pass through
    // These are NOT blob references; they're served from public/images, public/assets, etc.
    if (value.startsWith('/images/') || value.startsWith('/assets/')) {
      return value;
    }

    // Extract blob hash from various formats
    let blobHash = value;

    // Handle /blob/{hash} format
    if (value.startsWith('/blob/')) {
      blobHash = value.slice(6);
    }
    // Handle blob/{hash} format (no leading slash)
    else if (value.startsWith('blob/')) {
      blobHash = value.slice(5);
    }

    // Only convert to blob URL if it looks like a hash (sha256-... or sha256:...)
    // Other relative paths should pass through unchanged
    if (!blobHash.startsWith('sha256-') && !blobHash.startsWith('sha256:')) {
      return value;
    }

    // Build full URL using strategy-aware service
    return this.storageClient.getBlobUrl(blobHash);
  }

  /**
   * Transform raw relationship data
   */
  private transformRelationship(data: RawRelationshipData): Relationship {
    return {
      id: data.id ?? '',
      sourceId: data.sourceId ?? '',
      targetId: data.targetId ?? '',
      relationshipType: data.relationshipType ?? '',
      confidence: data.confidence ?? 1,
      inferenceSource: data.inferenceSource ?? 'explicit',
      metadata: data.metadata,
      createdAt: data.createdAt,
    };
  }

  /**
   * Transform raw content graph data
   */
  private transformContentGraph(data: RawContentGraphData): ContentGraph {
    return {
      rootId: data.rootId ?? '',
      related: (data.related ?? []).map(node => this.transformContentGraphNode(node)),
      totalNodes: data.totalNodes ?? 0,
    };
  }

  /**
   * Transform content graph node recursively
   */
  private transformContentGraphNode(data: RawContentGraphNodeData): ContentGraphNode {
    return {
      contentId: data.contentId ?? '',
      relationshipType: data.relationshipType ?? '',
      confidence: data.confidence ?? 1,
      children: (data.children ?? []).map(child => this.transformContentGraphNode(child)),
    };
  }

  /**
   * Transform raw knowledge map data
   */
  private transformKnowledgeMap(data: RawKnowledgeMapData): KnowledgeMap {
    return {
      id: data.id ?? '',
      mapType: (data.mapType ?? 'content-graph') as KnowledgeMapType,
      ownerId: data.ownerId ?? '',
      title: data.title ?? '',
      description: data.description,
      subject: {
        type: (data.subjectType ?? 'content-graph') as 'content-graph' | 'agent' | 'organization',
        subjectId: data.subjectId ?? '',
        subjectName: data.subjectName ?? '',
      },
      visibility: (data.visibility ?? 'private') as 'private' | 'mutual' | 'shared' | 'public',
      sharedWith: data.sharedWith,
      nodes: data.nodes ?? [],
      pathIds: data.pathIds ?? [],
      overallAffinity: data.overallAffinity ?? 0,
      metadata: data.metadata,
      createdAt: data.createdAt ?? '',
      updatedAt: data.updatedAt ?? '',
    };
  }

  /**
   * Transform raw path extension data
   */
  private transformPathExtension(data: RawPathExtensionData): PathExtension {
    return {
      id: data.id ?? '',
      basePathId: data.basePathId ?? '',
      basePathVersion: data.basePathVersion ?? '',
      extendedBy: data.extendedBy ?? '',
      title: data.title ?? '',
      description: data.description,
      // Transform loose arrays to typed arrays (canonical model requires these)
      insertions: Array.isArray(data.insertions) ? data.insertions : [],
      annotations: Array.isArray(data.annotations) ? data.annotations : [],
      reorderings: Array.isArray(data.reorderings) ? data.reorderings : [],
      exclusions: Array.isArray(data.exclusions) ? data.exclusions : [],
      visibility: (data.visibility ?? 'private') as 'private' | 'shared' | 'public',
      sharedWith: data.sharedWith,
      forkedFrom: data.forkedFrom,
      forks: data.forks,
      upstreamProposal: data.upstreamProposal,
      stats: data.stats,
      createdAt: data.createdAt ?? '',
      updatedAt: data.updatedAt ?? '',
    };
  }
}
