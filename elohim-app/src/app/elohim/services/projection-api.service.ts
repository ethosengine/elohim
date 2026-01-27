/**
 * Projection API Service
 *
 * Connects to Doorway's cache API for fast reads.
 * Uses the generic cache endpoints: /api/v1/cache/{type}/{id}
 *
 * The app (elohim-app) owns all content structure and transformations.
 * Doorway is just a cache that serves stored content.
 */

import { HttpClient, HttpParams, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { map, catchError, timeout, shareReplay } from 'rxjs/operators';

import { Observable, of } from 'rxjs';

import { environment } from '../../../environments/environment';
import { ContentNode, ContentType, ContentReach } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

import { StorageClientService } from './storage-client.service';

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
  private readonly storageClient = inject(StorageClientService);

  /** Base URL for cache API */
  private get baseUrl(): string {
    const doorwayUrl =
      environment.holochain?.authUrl || environment.holochain?.appUrl || 'http://localhost:8080';
    const httpUrl = doorwayUrl.replace('wss://', 'https://').replace('ws://', 'http://');
    return `${httpUrl}/api/v1/cache`;
  }

  /** API key for authenticated requests */
  private get apiKey(): string | undefined {
    return environment.holochain?.proxyApiKey;
  }

  /** Build URL with optional API key */
  private buildApiUrl(path: string): string {
    const url = `${this.baseUrl}${path}`;
    return this.apiKey ? `${url}${path.includes('?') ? '&' : '?'}apiKey=${this.apiKey}` : url;
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

    const url = this.buildApiUrl(`/Content/${encodeURIComponent(id)}`);

    return this.http.get<any>(url).pipe(
      timeout(this.defaultTimeout),
      map(data => this.transformContent(data)),
      catchError(err => this.handleContentError(err, `getContent(${id})`)),
      shareReplay(1)
    );
  }

  /**
   * Query content nodes with filters
   *
   * Note: The generic cache API only supports limit/skip.
   * Client-side filtering is applied for other filters.
   */
  queryContent(filters: ContentQueryFilters): Observable<ContentNode[]> {
    if (!this.enabled) {
      return of([]);
    }

    let params = new HttpParams();
    if (filters.limit) {
      params = params.set('limit', filters.limit.toString());
    }
    if (filters.skip) {
      params = params.set('skip', filters.skip.toString());
    }

    const url = this.buildApiUrl('/Content');

    return this.http.get<any[]>(url, { params }).pipe(
      timeout(this.defaultTimeout),
      map(data => (data || []).map(c => this.transformContent(c))),
      // Apply client-side filters
      map(contents => this.applyContentFilters(contents, filters)),
      catchError(err => this.handleContentArrayError(err, 'queryContent'))
    );
  }

  /**
   * Apply client-side content filters
   */
  private applyContentFilters(
    contents: ContentNode[],
    filters: ContentQueryFilters
  ): ContentNode[] {
    let result = contents;

    if (filters.id) {
      result = result.filter(c => c.id === filters.id);
    }
    if (filters.ids?.length) {
      const idSet = new Set(filters.ids);
      result = result.filter(c => idSet.has(c.id));
    }
    if (filters.contentType) {
      const types = Array.isArray(filters.contentType)
        ? filters.contentType
        : [filters.contentType];
      result = result.filter(c => types.includes(c.contentType as ContentType));
    }
    if (filters.tags?.length) {
      result = result.filter(c => filters.tags!.every(tag => c.tags?.includes(tag)));
    }
    if (filters.anyTags?.length) {
      result = result.filter(c => filters.anyTags!.some(tag => c.tags?.includes(tag)));
    }
    if (filters.publicOnly) {
      result = result.filter(c => c.reach === 'commons');
    }
    if (filters.author) {
      result = result.filter(c => c.authorId === filters.author);
    }

    return result;
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

    const url = this.buildApiUrl(`/LearningPath/${encodeURIComponent(id)}`);

    return this.http.get<any>(url).pipe(
      timeout(this.defaultTimeout),
      map(data => this.transformPath(data)),
      catchError(err => this.handlePathError(err, `getPath(${id})`)),
      shareReplay(1)
    );
  }

  /**
   * Get path overview (minimal data for listing)
   * Note: Uses same endpoint as getPath since cache API is generic
   */
  getPathOverview(id: string): Observable<Partial<LearningPath> | null> {
    return this.getPath(id);
  }

  /**
   * Query paths with filters
   *
   * Note: The generic cache API only supports limit/skip.
   * Client-side filtering is applied for other filters.
   */
  queryPaths(filters: PathQueryFilters): Observable<LearningPath[]> {
    if (!this.enabled) {
      return of([]);
    }

    let params = new HttpParams();
    if (filters.limit) {
      params = params.set('limit', filters.limit.toString());
    }
    if (filters.skip) {
      params = params.set('skip', filters.skip.toString());
    }

    const url = this.buildApiUrl('/LearningPath');

    return this.http.get<any[]>(url, { params }).pipe(
      timeout(this.defaultTimeout),
      map(data => (data || []).map(p => this.transformPath(p))),
      // Apply client-side filters
      map(paths => this.applyPathFilters(paths, filters)),
      catchError(err => this.handlePathArrayError(err, 'queryPaths'))
    );
  }

  /**
   * Apply client-side path filters
   */
  private applyPathFilters(paths: LearningPath[], filters: PathQueryFilters): LearningPath[] {
    let result = paths;

    if (filters.id) {
      result = result.filter(p => p.id === filters.id);
    }
    if (filters.ids?.length) {
      const idSet = new Set(filters.ids);
      result = result.filter(p => idSet.has(p.id));
    }
    if (filters.difficulty) {
      result = result.filter(p => p.difficulty === filters.difficulty);
    }
    if (filters.visibility) {
      result = result.filter(p => p.visibility === filters.visibility);
    }
    if (filters.publicOnly) {
      result = result.filter(p => p.visibility === 'public');
    }
    if (filters.tags?.length) {
      result = result.filter(p => filters.tags!.some(tag => p.tags?.includes(tag)));
    }

    return result;
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

    return this.http
      .get<
        ProjectionResponse<Record<string, unknown>[]>
      >(`${this.baseUrl}/content/${encodeURIComponent(nodeId)}/related`, { params })
      .pipe(
        timeout(this.defaultTimeout),
        map(response => (response.data || []).map(c => this.transformContent(c))),
        catchError(err => this.handleContentArrayError(err, `getRelated(${nodeId})`))
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
      catchError(err => this.handleStatsError(err, 'getStats'))
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
      params = params.set('contentType', types);
    }
    if (filters.tags?.length) {
      params = params.set('tags', filters.tags.join(','));
    }
    if (filters.anyTags?.length) {
      params = params.set('anyTags', filters.anyTags.join(','));
    }
    if (filters.reach) {
      const reaches = Array.isArray(filters.reach) ? filters.reach.join(',') : filters.reach;
      params = params.set('reach', reaches);
    }
    if (filters.publicOnly) {
      params = params.set('publicOnly', 'true');
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
      params = params.set('publicOnly', 'true');
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
  private transformContent(data: Record<string, unknown>): ContentNode {
    // Rust API now returns parsed metadata directly
    const metadata = (data['metadata'] ?? {}) as Record<string, unknown>;

    // Projection data may have slightly different field names
    return {
      id: (data['id'] ?? data['docId']) as string,
      contentType: data['contentType'] as string,
      title: (data['title'] ?? '') as string,
      description: (data['description'] ?? '') as string,
      content: (data['content'] ?? '') as string,
      contentFormat: (data['contentFormat'] ?? 'markdown') as string,
      tags: (data['tags'] ?? []) as string[],
      relatedNodeIds: (data['relatedNodeIds'] ?? []) as string[],
      metadata,
      authorId: (data['authorId'] ?? data['author']) as string | undefined,
      reach: (data['reach'] ?? 'private') as string,
      trustScore: data['trustScore'] as number | undefined,
      estimatedMinutes: data['estimatedMinutes'] as number | undefined,
      thumbnailUrl: this.resolveBlobUrl(data['thumbnailUrl'] as string | null | undefined),
      createdAt: data['createdAt'] as string | undefined,
      updatedAt: data['updatedAt'] as string | undefined,
    } as ContentNode;
  }

  /**
   * Transform projected path to LearningPath model
   */
  private transformPath(data: unknown): LearningPath {
    const d = data as Record<string, unknown>;
    return {
      id: (d['id'] ?? d['docId']) as string,
      version: (d['version'] ?? '1.0.0') as string,
      title: (d['title'] ?? '') as string,
      description: (d['description'] ?? '') as string,
      purpose: (d['purpose'] ?? '') as string,
      difficulty: (d['difficulty'] ?? 'beginner') as string,
      estimatedDuration: d['estimatedDuration'] as string | undefined,
      visibility: (d['visibility'] ?? 'public') as string,
      pathType: (d['pathType'] ?? 'course') as string,
      thumbnailUrl: this.resolveBlobUrl(d['thumbnailUrl'] as string | null | undefined),
      thumbnailAlt: d['thumbnailAlt'] as string | undefined,
      tags: (d['tags'] ?? []) as string[],
      createdBy: (d['createdBy'] ?? d['author'] ?? '') as string,
      contributors: (d['contributors'] ?? []) as string[],
      steps: (d['steps'] ?? []) as unknown[],
      chapters: (d['chapters'] ?? []) as unknown[],
      stepCount: (d['stepCount'] ?? 0) as number,
      chapterCount: (d['chapterCount'] ?? 0) as number,
      createdAt: d['createdAt'] as string | undefined,
      updatedAt: d['updatedAt'] as string | undefined,
    } as LearningPath;
  }

  /**
   * Resolve a blob reference to a full URL.
   * Uses StorageClientService for strategy-aware URL construction.
   */
  private resolveBlobUrl(value: string | null | undefined): string | undefined {
    if (!value) return undefined;

    // Already a full URL - pass through
    if (value.startsWith('http://') || value.startsWith('https://')) {
      return value;
    }

    // Extract blob hash from various formats
    let blobHash = value;
    if (value.startsWith('/blob/')) {
      blobHash = value.slice(6);
    } else if (value.startsWith('blob/')) {
      blobHash = value.slice(5);
    }

    return this.storageClient.getBlobUrl(blobHash);
  }

  /**
   * Handle HTTP errors - returns null for single items, empty array for collections
   */
  private handleContentError(
    error: HttpErrorResponse,
    context: string
  ): Observable<ContentNode | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }

  private handleContentArrayError(
    error: HttpErrorResponse,
    context: string
  ): Observable<ContentNode[]> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of([]);
  }

  private handlePathError(
    error: HttpErrorResponse,
    context: string
  ): Observable<LearningPath | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }

  private handlePathOverviewError(
    error: HttpErrorResponse,
    context: string
  ): Observable<Partial<LearningPath> | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }

  private handlePathArrayError(
    error: HttpErrorResponse,
    context: string
  ): Observable<LearningPath[]> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of([]);
  }

  private handleStatsError(
    error: HttpErrorResponse,
    context: string
  ): Observable<ProjectionStats | null> {
    console.debug(`[ProjectionAPI] ${context} failed:`, error.status, error.message);
    return of(null);
  }
}
