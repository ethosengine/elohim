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

import { Injectable, inject } from '@angular/core';
import { Observable, from, of } from 'rxjs';
import { map, catchError, shareReplay } from 'rxjs/operators';

import { ELOHIM_CLIENT, ElohimClient } from '../providers/elohim-client.provider';
import type { ContentQuery } from '@elohim/service/client';
import { ContentNode, ContentType, ContentReach } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

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
  relationshipType: string;  // RELATES_TO, CONTAINS, DEPENDS_ON, IMPLEMENTS, REFERENCES
  confidence: number;
  inferenceSource: string;   // explicit, path, tag, semantic
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

/**
 * Knowledge map (domain, self, person, collective)
 */
export interface KnowledgeMap {
  id: string;
  mapType: string;
  ownerId: string;
  title: string;
  description?: string;
  subjectType: string;
  subjectId: string;
  subjectName: string;
  visibility: string;
  sharedWith?: string[];
  nodes: any;  // Graph node data
  pathIds?: string[];
  overallAffinity: number;
  contentGraphId?: string;
  masteryLevels?: Record<string, number>;
  goals?: any[];
  metadata?: Record<string, any>;
  createdAt?: string;
  updatedAt?: string;
}

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

/**
 * Path extension (user customization/fork)
 */
export interface PathExtension {
  id: string;
  basePathId: string;
  basePathVersion: string;
  extendedBy: string;
  title: string;
  description?: string;
  insertions?: any[];
  annotations?: any[];
  reorderings?: any[];
  exclusions?: string[];
  visibility: string;
  sharedWith?: string[];
  forkedFrom?: string;
  forks?: string[];
  upstreamProposal?: any;
  stats?: any;
  createdAt?: string;
  updatedAt?: string;
}

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

@Injectable({ providedIn: 'root' })
export class ContentService {
  private readonly client: ElohimClient = inject(ELOHIM_CLIENT);

  // In-memory cache for hot paths
  private contentCache = new Map<string, Observable<ContentNode | null>>();
  private pathCache = new Map<string, Observable<LearningPath | null>>();

  // =========================================================================
  // Content Operations
  // =========================================================================

  /**
   * Get a single content node by ID
   */
  getContent(id: string): Observable<ContentNode | null> {
    // Check cache first
    const cached = this.contentCache.get(id);
    if (cached) return cached;

    const obs = from(this.client.get<ContentNode>('content', id)).pipe(
      map(data => data ? this.transformContent(data) : null),
      catchError(err => {
        console.debug(`[ContentService] getContent(${id}) failed:`, err);
        return of(null);
      }),
      shareReplay(1)
    );

    this.contentCache.set(id, obs);
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

    return from(this.client.query<ContentNode>(query)).pipe(
      map(items => items.map(c => this.transformContent(c))),
      map(items => this.applyLocalFilters(items, filters)),
      catchError(err => {
        console.debug('[ContentService] queryContent failed:', err);
        return of([]);
      })
    );
  }

  /**
   * Batch get multiple content nodes
   */
  batchGetContent(ids: string[]): Observable<Map<string, ContentNode>> {
    return from(this.client.getBatch<ContentNode>('content', ids)).pipe(
      map(results => {
        const map = new Map<string, ContentNode>();
        for (const [id, content] of results) {
          map.set(id, this.transformContent(content));
        }
        return map;
      }),
      catchError(err => {
        console.debug('[ContentService] batchGetContent failed:', err);
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

    const obs = from(this.client.get<LearningPath>('path', id)).pipe(
      map(data => data ? this.transformPath(data) : null),
      catchError(err => {
        console.debug(`[ContentService] getPath(${id}) failed:`, err);
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

    return from(this.client.query<LearningPath>(query)).pipe(
      map(items => items.map(p => this.transformPath(p))),
      map(items => this.applyPathFilters(items, filters)),
      catchError(err => {
        console.debug('[ContentService] queryPaths failed:', err);
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
    relationshipType?: string,
  ): Observable<Relationship[]> {
    const params = new URLSearchParams({
      content_id: contentId,
      direction,
    });
    if (relationshipType) {
      params.set('relationship_type', relationshipType);
    }

    return from(this.client.fetch<{ items: any[] }>(`/db/relationships?${params}`)).pipe(
      map(response => (response?.items || []).map(r => this.transformRelationship(r))),
      catchError(err => {
        console.debug('[ContentService] getRelationships failed:', err);
        return of([]);
      }),
    );
  }

  /**
   * Get content graph starting from a root node
   */
  getContentGraph(contentId: string, relationshipTypes?: string[]): Observable<ContentGraph | null> {
    let url = `/db/relationships/graph/${contentId}`;
    if (relationshipTypes?.length) {
      url += `?types=${relationshipTypes.join(',')}`;
    }

    return from(this.client.fetch<any>(url)).pipe(
      map(data => data ? this.transformContentGraph(data) : null),
      catchError(err => {
        console.debug(`[ContentService] getContentGraph(${contentId}) failed:`, err);
        return of(null);
      }),
    );
  }

  // =========================================================================
  // Knowledge Map Operations
  // =========================================================================

  /**
   * Get a knowledge map by ID
   */
  getKnowledgeMap(id: string): Observable<KnowledgeMap | null> {
    return from(this.client.fetch<any>(`/db/knowledge-maps/${id}`)).pipe(
      map(data => data ? this.transformKnowledgeMap(data) : null),
      catchError(err => {
        console.debug(`[ContentService] getKnowledgeMap(${id}) failed:`, err);
        return of(null);
      }),
    );
  }

  /**
   * Query knowledge maps
   */
  queryKnowledgeMaps(filters: KnowledgeMapFilters): Observable<KnowledgeMap[]> {
    const params = new URLSearchParams();
    if (filters.ownerId) params.set('owner_id', filters.ownerId);
    if (filters.mapType) params.set('map_type', filters.mapType);
    if (filters.subjectId) params.set('subject_id', filters.subjectId);
    if (filters.visibility) params.set('visibility', filters.visibility);
    if (filters.limit) params.set('limit', String(filters.limit));
    if (filters.offset) params.set('offset', String(filters.offset));

    return from(this.client.fetch<{ items: any[] }>(`/db/knowledge-maps?${params}`)).pipe(
      map(response => (response?.items || []).map((m: any) => this.transformKnowledgeMap(m))),
      catchError(err => {
        console.debug('[ContentService] queryKnowledgeMaps failed:', err);
        return of([]);
      }),
    );
  }

  // =========================================================================
  // Path Extension Operations
  // =========================================================================

  /**
   * Get a path extension by ID
   */
  getPathExtension(id: string): Observable<PathExtension | null> {
    return from(this.client.fetch<any>(`/db/path-extensions/${id}`)).pipe(
      map(data => data ? this.transformPathExtension(data) : null),
      catchError(err => {
        console.debug(`[ContentService] getPathExtension(${id}) failed:`, err);
        return of(null);
      }),
    );
  }

  /**
   * Query path extensions
   */
  queryPathExtensions(filters: PathExtensionFilters): Observable<PathExtension[]> {
    const params = new URLSearchParams();
    if (filters.basePathId) params.set('base_path_id', filters.basePathId);
    if (filters.extendedBy) params.set('extended_by', filters.extendedBy);
    if (filters.visibility) params.set('visibility', filters.visibility);
    if (filters.limit) params.set('limit', String(filters.limit));
    if (filters.offset) params.set('offset', String(filters.offset));

    return from(this.client.fetch<{ items: any[] }>(`/db/path-extensions?${params}`)).pipe(
      map(response => (response?.items || []).map((e: any) => this.transformPathExtension(e))),
      catchError(err => {
        console.debug('[ContentService] queryPathExtensions failed:', err);
        return of([]);
      }),
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
  private transformContent(data: any): ContentNode {
    return {
      id: data.id || data.doc_id,
      contentType: data.content_type || data.contentType,
      title: data.title || '',
      description: data.description || '',
      // content_body is inline storage, content is legacy/direct format
      content: data.content_body || data.content || '',
      contentFormat: data.content_format || data.contentFormat || 'markdown',
      tags: data.tags || [],
      relatedNodeIds: data.related_node_ids || data.relatedNodeIds || [],
      metadata: data.metadata || {},
      authorId: data.author_id || data.authorId,
      reach: data.reach || 'commons',
      trustScore: data.trust_score || data.trustScore,
      estimatedMinutes: data.estimated_minutes || data.estimatedMinutes,
      thumbnailUrl: data.thumbnail_url || data.thumbnailUrl,
      createdAt: data.created_at || data.createdAt,
      updatedAt: data.updated_at || data.updatedAt,
    } as ContentNode;
  }

  /**
   * Transform raw data to LearningPath model
   *
   * Handles two response formats:
   * 1. Flat: { id, title, steps, chapters, ... }
   * 2. Nested: { path: {...}, chapters: [...], ungrouped_steps: [...] }
   */
  private transformPath(data: any): LearningPath {
    // Handle nested response format from elohim-storage
    const pathData = data.path || data;
    const rawChapters = data.chapters || pathData.chapters || [];
    const ungroupedSteps = data.ungrouped_steps || [];

    // Transform chapters with their steps
    const chapters = rawChapters.map((ch: any) => ({
      id: ch.id,
      title: ch.title || '',
      description: ch.description || '',
      orderIndex: ch.order_index ?? ch.orderIndex ?? 0,
      estimatedDuration: ch.estimated_duration || ch.estimatedDuration,
      steps: (ch.steps || []).map((s: any) => this.transformStep(s)),
    }));

    // Collect all steps from chapters + ungrouped
    const allSteps = [
      ...chapters.flatMap((ch: any) => ch.steps),
      ...ungroupedSteps.map((s: any) => this.transformStep(s)),
    ];

    return {
      id: pathData.id || pathData.doc_id,
      version: pathData.version || '1.0.0',
      title: pathData.title || '',
      description: pathData.description || '',
      purpose: pathData.purpose || '',
      difficulty: pathData.difficulty || 'beginner',
      estimatedDuration: pathData.estimated_duration || pathData.estimatedDuration,
      visibility: pathData.visibility || 'public',
      pathType: pathData.path_type || pathData.pathType || 'course',
      tags: pathData.tags || [],
      createdBy: pathData.created_by || pathData.createdBy || '',
      contributors: pathData.contributors || [],
      steps: allSteps,
      chapters,
      stepCount: pathData.step_count || pathData.stepCount || allSteps.length,
      chapterCount: pathData.chapter_count || pathData.chapterCount || chapters.length,
      createdAt: pathData.created_at || pathData.createdAt,
      updatedAt: pathData.updated_at || pathData.updatedAt,
    } as LearningPath;
  }

  /**
   * Transform a step from snake_case to camelCase
   */
  private transformStep(step: any): any {
    return {
      id: step.id,
      pathId: step.path_id || step.pathId,
      chapterId: step.chapter_id || step.chapterId,
      // Map to both 'title' and 'stepTitle' for compatibility
      title: step.title || '',
      stepTitle: step.title || '',
      stepNarrative: step.description || '',
      description: step.description || '',
      stepType: step.step_type || step.stepType || 'learn',
      resourceId: step.resource_id || step.resourceId || '',
      resourceType: step.resource_type || step.resourceType || 'content',
      order: step.order_index ?? step.orderIndex ?? 0,
      orderIndex: step.order_index ?? step.orderIndex ?? 0,
      estimatedDuration: step.estimated_duration || step.estimatedDuration,
      metadata: step.metadata_json ? JSON.parse(step.metadata_json) : step.metadata,
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
      result = result.filter(c => types.includes(c.contentType as ContentType));
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
   * Transform raw relationship data
   */
  private transformRelationship(data: any): Relationship {
    return {
      id: data.id,
      sourceId: data.source_id || data.sourceId,
      targetId: data.target_id || data.targetId,
      relationshipType: data.relationship_type || data.relationshipType,
      confidence: data.confidence ?? 1.0,
      inferenceSource: data.inference_source || data.inferenceSource || 'explicit',
      metadata: data.metadata_json ? JSON.parse(data.metadata_json) : data.metadata,
      createdAt: data.created_at || data.createdAt,
    };
  }

  /**
   * Transform raw content graph data
   */
  private transformContentGraph(data: any): ContentGraph {
    return {
      rootId: data.root_id || data.rootId,
      related: (data.related || []).map((node: any) => this.transformContentGraphNode(node)),
      totalNodes: data.total_nodes || data.totalNodes || 0,
    };
  }

  /**
   * Transform content graph node recursively
   */
  private transformContentGraphNode(data: any): ContentGraphNode {
    return {
      contentId: data.content_id || data.contentId,
      relationshipType: data.relationship_type || data.relationshipType,
      confidence: data.confidence ?? 1.0,
      children: (data.children || []).map((child: any) => this.transformContentGraphNode(child)),
    };
  }

  /**
   * Transform raw knowledge map data
   */
  private transformKnowledgeMap(data: any): KnowledgeMap {
    return {
      id: data.id,
      mapType: data.map_type || data.mapType,
      ownerId: data.owner_id || data.ownerId,
      title: data.title || '',
      description: data.description,
      subjectType: data.subject_type || data.subjectType,
      subjectId: data.subject_id || data.subjectId,
      subjectName: data.subject_name || data.subjectName,
      visibility: data.visibility || 'private',
      sharedWith: data.shared_with_json ? JSON.parse(data.shared_with_json) : data.sharedWith,
      nodes: data.nodes_json ? JSON.parse(data.nodes_json) : data.nodes,
      pathIds: data.path_ids_json ? JSON.parse(data.path_ids_json) : data.pathIds,
      overallAffinity: data.overall_affinity ?? data.overallAffinity ?? 0,
      contentGraphId: data.content_graph_id || data.contentGraphId,
      masteryLevels: data.mastery_levels_json ? JSON.parse(data.mastery_levels_json) : data.masteryLevels,
      goals: data.goals_json ? JSON.parse(data.goals_json) : data.goals,
      metadata: data.metadata_json ? JSON.parse(data.metadata_json) : data.metadata,
      createdAt: data.created_at || data.createdAt,
      updatedAt: data.updated_at || data.updatedAt,
    };
  }

  /**
   * Transform raw path extension data
   */
  private transformPathExtension(data: any): PathExtension {
    return {
      id: data.id,
      basePathId: data.base_path_id || data.basePathId,
      basePathVersion: data.base_path_version || data.basePathVersion,
      extendedBy: data.extended_by || data.extendedBy,
      title: data.title || '',
      description: data.description,
      insertions: data.insertions_json ? JSON.parse(data.insertions_json) : data.insertions,
      annotations: data.annotations_json ? JSON.parse(data.annotations_json) : data.annotations,
      reorderings: data.reorderings_json ? JSON.parse(data.reorderings_json) : data.reorderings,
      exclusions: data.exclusions_json ? JSON.parse(data.exclusions_json) : data.exclusions,
      visibility: data.visibility || 'private',
      sharedWith: data.shared_with_json ? JSON.parse(data.shared_with_json) : data.sharedWith,
      forkedFrom: data.forked_from || data.forkedFrom,
      forks: data.forks_json ? JSON.parse(data.forks_json) : data.forks,
      upstreamProposal: data.upstream_proposal_json ? JSON.parse(data.upstream_proposal_json) : data.upstreamProposal,
      stats: data.stats_json ? JSON.parse(data.stats_json) : data.stats,
      createdAt: data.created_at || data.createdAt,
      updatedAt: data.updated_at || data.updatedAt,
    };
  }
}
