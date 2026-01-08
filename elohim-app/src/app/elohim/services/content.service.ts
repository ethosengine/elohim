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
      content: data.content || '',
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
}
