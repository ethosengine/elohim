/**
 * Projection API Service
 *
 * Connects to Doorway's projection cache for fast reads.
 * The projection layer provides pre-computed, MongoDB-cached data
 * that's updated in real-time via DHT signals.
 *
 * This is the preferred read path for production - it never blocks
 * on Holochain conductor calls.
 */

import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams, HttpErrorResponse } from '@angular/common/http';
import { Observable, of } from 'rxjs';
import { map, catchError, timeout, shareReplay } from 'rxjs/operators';

import { environment } from '../../../environments/environment';
import { ContentNode, ContentType, ContentReach } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

/**
 * Content query filters for projection API
 */
export interface ContentQueryFilters {
  /** Single ID lookup */
  id?: string;
  /** Multiple ID lookup */
  ids?: string[];
  /** Filter by content type */
  contentType?: ContentType | ContentType[];
  /** Filter by tags (all must match) */
  tags?: string[];
  /** Filter by any of these tags */
  anyTags?: string[];
  /** Filter by reach/visibility */
  reach?: ContentReach | ContentReach[];
  /** Only public content */
  publicOnly?: boolean;
  /** Filter by author */
  author?: string;
  /** Full-text search query */
  search?: string;
  /** Maximum results */
  limit?: number;
  /** Skip for pagination */
  skip?: number;
}

/**
 * Path query filters for projection API
 */
export interface PathQueryFilters {
  /** Single ID lookup */
  id?: string;
  /** Multiple ID lookup */
  ids?: string[];
  /** Filter by difficulty */
  difficulty?: 'beginner' | 'intermediate' | 'advanced';
  /** Filter by visibility */
  visibility?: string;
  /** Only public paths */
  publicOnly?: boolean;
  /** Filter by tags */
  tags?: string[];
  /** Full-text search query */
  search?: string;
  /** Maximum results */
  limit?: number;
  /** Skip for pagination */
  skip?: number;
}

/**
 * Projection API response wrapper
 */
export interface ProjectionResponse<T> {
  data: T;
  source: 'projection' | 'conductor';
  cachedAt?: string;
}

/**
 * Projection store statistics
 */
export interface ProjectionStats {
  totalEntries: number;
  hotCacheEntries: number;
  expiredEntries: number;
  mongoConnected: boolean;
}

@Injectable({ providedIn: 'root' })
export class ProjectionAPIService {
  private readonly http = inject(HttpClient);

  /** Base URL for projection API */
  private get baseUrl(): string {
    // Use Doorway's API endpoint
    const doorwayUrl = environment.holochain?.authUrl || environment.holochain?.appUrl || 'http://localhost:8080';
    // Strip ws/wss protocol if present
    const httpUrl = doorwayUrl
      .replace('wss://', 'https://')
      .replace('ws://', 'http://');
    return `${httpUrl}/api/v1/projection`;
  }

  /** Default timeout for projection API calls */
  private readonly defaultTimeout = 5000;

  /** Whether projection API is enabled */
  get enabled(): boolean {
    return environment.projectionApi?.enabled !== false;
  }

  // =========================================================================
  // Content Queries
  // =========================================================================

  /**
   * Get a single content node by ID
   */
  getContent(id: string): Observable<ContentNode | null> {
    if (!this.enabled) {
      return of(null);
    }

    return this.http.get<ProjectionResponse<ContentNode>>(
      `${this.baseUrl}/content/${encodeURIComponent(id)}`
    ).pipe(
      timeout(this.defaultTimeout),
      map(response => this.transformContent(response.data)),
      catchError(err => this.handleContentError(err, `getContent(${id})`)),
      shareReplay(1)
    );
  }

  /**
   * Query content nodes with filters
   */
  queryContent(filters: ContentQueryFilters): Observable<ContentNode[]> {
    if (!this.enabled) {
      return of([]);
    }

    const params = this.buildContentParams(filters);

    return this.http.get<ProjectionResponse<ContentNode[]>>(
      `${this.baseUrl}/content`,
      { params }
    ).pipe(
      timeout(this.defaultTimeout),
      map(response => (response.data || []).map(c => this.transformContent(c))),
      catchError(err => this.handleContentArrayError(err, 'queryContent')),
    );
  }

  /**
   * Batch get multiple content nodes by IDs
   */
  batchGetContent(ids: string[]): Observable<Map<string, ContentNode>> {
    if (!this.enabled || ids.length === 0) {
      return of(new Map());
    }

    return this.queryContent({ ids }).pipe(
      map(contents => {
        const map = new Map<string, ContentNode>();
        contents.forEach(c => map.set(c.id, c));
        return map;
      })
    );
  }

  /**
   * Search content with full-text query
   */
  searchContent(query: string, limit = 50): Observable<ContentNode[]> {
    return this.queryContent({ search: query, limit, publicOnly: true });
  }

  // =========================================================================
  // Path Queries
  // =========================================================================

  /**
   * Get a single learning path by ID
   */
  getPath(id: string): Observable<LearningPath | null> {
    if (!this.enabled) {
      return of(null);
    }

    return this.http.get<ProjectionResponse<LearningPath>>(
      `${this.baseUrl}/path/${encodeURIComponent(id)}`
    ).pipe(
      timeout(this.defaultTimeout),
      map(response => this.transformPath(response.data)),
      catchError(err => this.handlePathError(err, `getPath(${id})`)),
      shareReplay(1)
    );
  }

  /**
   * Get path overview (minimal data for listing)
   */
  getPathOverview(id: string): Observable<Partial<LearningPath> | null> {
    if (!this.enabled) {
      return of(null);
    }

    return this.http.get<ProjectionResponse<Partial<LearningPath>>>(
      `${this.baseUrl}/path/${encodeURIComponent(id)}/overview`
    ).pipe(
      timeout(this.defaultTimeout),
      map(response => response.data),
      catchError(err => this.handlePathOverviewError(err, `getPathOverview(${id})`)),
    );
  }

  /**
   * Query paths with filters
   */
  queryPaths(filters: PathQueryFilters): Observable<LearningPath[]> {
    if (!this.enabled) {
      return of([]);
    }

    const params = this.buildPathParams(filters);

    return this.http.get<ProjectionResponse<LearningPath[]>>(
      `${this.baseUrl}/paths`,
      { params }
    ).pipe(
      timeout(this.defaultTimeout),
      map(response => (response.data || []).map(p => this.transformPath(p))),
      catchError(err => this.handlePathArrayError(err, 'queryPaths')),
    );
  }

  /**
   * Get all public paths (path index)
   */
  getAllPaths(limit = 100): Observable<LearningPath[]> {
    return this.queryPaths({ publicOnly: true, limit });
  }

  /**
   * Batch get multiple paths by IDs
   */
  batchGetPaths(ids: string[]): Observable<Map<string, LearningPath>> {
    if (!this.enabled || ids.length === 0) {
      return of(new Map());
    }

    return this.queryPaths({ ids }).pipe(
      map(paths => {
        const map = new Map<string, LearningPath>();
        paths.forEach(p => map.set(p.id, p));
        return map;
      })
    );
  }

  // =========================================================================
  // Relationships & Graph
  // =========================================================================

  /**
   * Get related content for a node
   */
  getRelated(nodeId: string, depth = 1): Observable<ContentNode[]> {
    if (!this.enabled) {
      return of([]);
    }

    const params = new HttpParams().set('depth', depth.toString());

    return this.http.get<ProjectionResponse<ContentNode[]>>(
      `${this.baseUrl}/content/${encodeURIComponent(nodeId)}/related`,
      { params }
    ).pipe(
      timeout(this.defaultTimeout),
      map(response => (response.data || []).map(c => this.transformContent(c))),
      catchError(err => this.handleContentArrayError(err, `getRelated(${nodeId})`)),
    );
  }

  // =========================================================================
  // Stats & Health
  // =========================================================================

  /**
   * Get projection store statistics
   */
  getStats(): Observable<ProjectionStats | null> {
    if (!this.enabled) {
      return of(null);
    }

    return this.http.get<ProjectionStats>(`${this.baseUrl}/stats`).pipe(
      timeout(this.defaultTimeout),
      catchError(err => this.handleStatsError(err, 'getStats')),
    );
  }

  /**
   * Check if projection API is healthy
   */
  isHealthy(): Observable<boolean> {
    return this.getStats().pipe(
      map(stats => stats !== null),
      catchError(() => of(false))
    );
  }

  // =========================================================================
  // Private Helpers
  // =========================================================================

  /**
   * Build HTTP params for content query
   */
  private buildContentParams(filters: ContentQueryFilters): HttpParams {
    let params = new HttpParams();

    if (filters.id) {
      params = params.set('id', filters.id);
    }
    if (filters.ids?.length) {
      params = params.set('ids', filters.ids.join(','));
    }
    if (filters.contentType) {
      const types = Array.isArray(filters.contentType)
        ? filters.contentType.join(',')
        : filters.contentType;
      params = params.set('content_type', types);
    }
    if (filters.tags?.length) {
      params = params.set('tags', filters.tags.join(','));
    }
    if (filters.anyTags?.length) {
      params = params.set('any_tags', filters.anyTags.join(','));
    }
    if (filters.reach) {
      const reaches = Array.isArray(filters.reach)
        ? filters.reach.join(',')
        : filters.reach;
      params = params.set('reach', reaches);
    }
    if (filters.publicOnly) {
      params = params.set('public_only', 'true');
    }
    if (filters.author) {
      params = params.set('author', filters.author);
    }
    if (filters.search) {
      params = params.set('search', filters.search);
    }
    if (filters.limit) {
      params = params.set('limit', filters.limit.toString());
    }
    if (filters.skip) {
      params = params.set('skip', filters.skip.toString());
    }

    return params;
  }

  /**
   * Build HTTP params for path query
   */
  private buildPathParams(filters: PathQueryFilters): HttpParams {
    let params = new HttpParams();

    if (filters.id) {
      params = params.set('id', filters.id);
    }
    if (filters.ids?.length) {
      params = params.set('ids', filters.ids.join(','));
    }
    if (filters.difficulty) {
      params = params.set('difficulty', filters.difficulty);
    }
    if (filters.visibility) {
      params = params.set('visibility', filters.visibility);
    }
    if (filters.publicOnly) {
      params = params.set('public_only', 'true');
    }
    if (filters.tags?.length) {
      params = params.set('tags', filters.tags.join(','));
    }
    if (filters.search) {
      params = params.set('search', filters.search);
    }
    if (filters.limit) {
      params = params.set('limit', filters.limit.toString());
    }
    if (filters.skip) {
      params = params.set('skip', filters.skip.toString());
    }

    return params;
  }

  /**
   * Transform projected content to ContentNode model
   */
  private transformContent(data: any): ContentNode {
    // Parse metadata if it's a JSON string
    let metadata = data.metadata || {};
    if (typeof data.metadata_json === 'string') {
      try {
        metadata = JSON.parse(data.metadata_json);
      } catch {
        // Ignore parse errors
      }
    }

    // Projection data may have slightly different field names
    return {
      id: data.id || data.doc_id,
      contentType: data.content_type || data.contentType,
      title: data.title || '',
      description: data.description || '',
      content: data.content || '',
      contentFormat: data.content_format || data.contentFormat || 'markdown',
      tags: data.tags || [],
      relatedNodeIds: data.related_node_ids || data.relatedNodeIds || [],
      metadata,
      authorId: data.author_id || data.author || data.authorId,
      reach: data.reach || 'private',
      trustScore: data.trust_score || data.trustScore,
      estimatedMinutes: data.estimated_minutes || data.estimatedMinutes,
      thumbnailUrl: data.thumbnail_url || data.thumbnailUrl,
      createdAt: data.created_at || data.createdAt,
      updatedAt: data.updated_at || data.updatedAt,
    } as ContentNode;
  }

  /**
   * Transform projected path to LearningPath model
   */
  private transformPath(data: any): LearningPath {
    return {
      id: data.id || data.doc_id,
      version: data.version || '1.0.0',
      title: data.title || '',
      description: data.description || '',
      purpose: data.purpose || '',
      difficulty: data.difficulty || 'beginner',
      estimatedDuration: data.estimated_duration || data.estimatedDuration,
      visibility: data.visibility || 'public',
      pathType: data.path_type || data.pathType || 'course',
      tags: data.tags || [],
      createdBy: data.created_by || data.author || data.createdBy || '',
      contributors: data.contributors || [],
      steps: data.steps || [],
      chapters: data.chapters || [],
      stepCount: data.step_count || data.stepCount || 0,
      chapterCount: data.chapter_count || data.chapterCount || 0,
      createdAt: data.created_at || data.createdAt,
      updatedAt: data.updated_at || data.updatedAt,
    } as LearningPath;
  }

  /**
   * Handle HTTP errors - returns null for single items, empty array for collections
   */
  private handleContentError(error: HttpErrorResponse, context: string): Observable<ContentNode | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }

  private handleContentArrayError(error: HttpErrorResponse, context: string): Observable<ContentNode[]> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of([]);
  }

  private handlePathError(error: HttpErrorResponse, context: string): Observable<LearningPath | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }

  private handlePathOverviewError(error: HttpErrorResponse, context: string): Observable<Partial<LearningPath> | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }

  private handlePathArrayError(error: HttpErrorResponse, context: string): Observable<LearningPath[]> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of([]);
  }

  private handleStatsError(error: HttpErrorResponse, context: string): Observable<ProjectionStats | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }
}
