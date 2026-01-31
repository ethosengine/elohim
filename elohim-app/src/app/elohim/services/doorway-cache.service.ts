/**
 * Doorway Cache Service
 *
 * HTTP client for the Doorway's generic cache API.
 * Provides type-safe access to cached content.
 *
 * Routes:
 * - GET /api/v1/cache/{type}/{id} - Get single document
 * - GET /api/v1/cache/{type}?limit=N - Query documents by type
 *
 * The app (elohim-app) owns all content structure and transformations.
 * Doorway is just a dumb cache that serves what's stored.
 */

import { HttpClient, HttpParams, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { map, catchError, timeout } from 'rxjs/operators';

import { Observable, of } from 'rxjs';

import { environment } from '../../../environments/environment';

/**
 * Cache query options
 */
export interface CacheQueryOptions {
  /** Maximum results (default: 100) */
  limit?: number;
  /** Offset for pagination */
  skip?: number;
}

/**
 * Cache response wrapper with metadata
 */
export interface CacheResponse<T> {
  data: T;
  reach?: string;
  cached: boolean;
}

@Injectable({ providedIn: 'root' })
export class DoorwayCacheService {
  private readonly http = inject(HttpClient);

  /** Base URL for cache API */
  private get baseUrl(): string {
    const doorwayUrl =
      environment.holochain?.authUrl ?? environment.holochain?.appUrl ?? 'http://localhost:8080';

    // Convert WebSocket URL to HTTP
    return doorwayUrl.replace('wss://', 'https://').replace('ws://', 'http://');
  }

  /** API key for authenticated requests */
  private get apiKey(): string | undefined {
    return environment.holochain?.apiKey;
  }

  /** Default request timeout */
  private readonly requestTimeout = 5000;

  // ===========================================================================
  // Generic Cache Methods
  // ===========================================================================

  /**
   * Get a single document by type and ID
   *
   * @param type - Document type (e.g., "Content", "LearningPath")
   * @param id - Document ID
   * @returns Observable of the document or null if not found
   */
  get<T>(type: string, id: string): Observable<T | null> {
    const url = this.buildUrl(`/api/v1/cache/${type}/${id}`);

    return this.http.get<T>(url).pipe(
      timeout(this.requestTimeout),
      catchError(err => this.handleError<T | null>(err, `get(${type}/${id})`, null))
    );
  }

  /**
   * Query documents by type
   *
   * @param type - Document type
   * @param options - Query options (limit, skip)
   * @returns Observable of document array
   */
  query<T>(type: string, options?: CacheQueryOptions): Observable<T[]> {
    let params = new HttpParams();

    if (options?.limit) {
      params = params.set('limit', options.limit.toString());
    }
    if (options?.skip) {
      params = params.set('skip', options.skip.toString());
    }

    const url = this.buildUrl(`/api/v1/cache/${type}`);

    return this.http.get<T[]>(url, { params }).pipe(
      timeout(this.requestTimeout),
      catchError(err => this.handleError<T[]>(err, `query(${type})`, []))
    );
  }

  // ===========================================================================
  // Typed Convenience Methods
  // ===========================================================================

  /**
   * Get all content nodes
   */
  getAllContent<T>(limit = 1000): Observable<T[]> {
    return this.query<T>('Content', { limit });
  }

  /**
   * Get a content node by ID
   */
  getContent<T>(id: string): Observable<T | null> {
    return this.get<T>('Content', id);
  }

  /**
   * Get all learning paths
   */
  getAllPaths<T>(limit = 100): Observable<T[]> {
    return this.query<T>('LearningPath', { limit });
  }

  /**
   * Get a learning path by ID
   */
  getPath<T>(id: string): Observable<T | null> {
    return this.get<T>('LearningPath', id);
  }

  /**
   * Get all relationships
   */
  getAllRelationships<T>(limit = 1000): Observable<T[]> {
    return this.query<T>('Relationship', { limit });
  }

  // ===========================================================================
  // Health Check
  // ===========================================================================

  /**
   * Check if cache service is reachable
   */
  isHealthy(): Observable<boolean> {
    const url = this.buildUrl('/health');

    return this.http.get(url, { responseType: 'text' }).pipe(
      timeout(2000),
      map(() => true),
      catchError(() => of(false))
    );
  }

  // ===========================================================================
  // Private Helpers
  // ===========================================================================

  /**
   * Build URL with API key
   */
  private buildUrl(path: string): string {
    const url = new URL(path, this.baseUrl);

    if (this.apiKey) {
      url.searchParams.set('apiKey', this.apiKey);
    }

    return url.toString();
  }

  /**
   * Handle HTTP errors gracefully
   */
  private handleError<T>(error: HttpErrorResponse, context: string, fallback: T): Observable<T> {
    // Only log non-404 errors (404 is expected for missing content)
    if (error.status !== 404) {
      // Error occurred but we're returning fallback gracefully
    }
    return of(fallback);
  }
}
