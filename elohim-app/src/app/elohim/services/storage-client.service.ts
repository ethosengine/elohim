/**
 * Storage Client Service
 *
 * Client for elohim-storage backend which provides:
 * - Blob store: Content-addressed storage for images, ZIPs, media
 * - SQL metadata: Content nodes, paths, projections
 *
 * Routes requests based on connection strategy:
 * - Doorway mode (browser): https://doorway.elohim.host/api/...
 * - Direct mode (Tauri): http://localhost:8090/...
 */

import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Observable, catchError, map, of, throwError, timeout } from 'rxjs';

import { environment } from '../../../environments/environment';
import { CONNECTION_STRATEGY } from '../providers/connection-strategy.provider';
import type { IConnectionStrategy, ConnectionConfig } from '@elohim/service/connection';

/** Content node from storage */
export interface StorageContentNode {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content_body: string | null;
  content_format: string;
  blob_hash: string | null;
  blob_cid: string | null;
  metadata_json: string | null;
  tags: string[];
  created_at: string;
  updated_at: string;
}

/** Path from storage */
export interface StoragePath {
  id: string;
  version: string;
  title: string;
  description: string;
  difficulty: string;
  estimated_duration: string | null;
  path_type: string;
  thumbnail_url: string | null;
  thumbnail_blob_hash: string | null;
  metadata_json: string | null;
  tags: string[];
}

/** Content query filter */
export interface ContentFilter {
  contentType?: string;
  contentFormat?: string;
  tags?: string[];
  limit?: number;
  offset?: number;
}

@Injectable({
  providedIn: 'root',
})
export class StorageClientService {
  private readonly http = inject(HttpClient);
  private readonly strategy = inject(CONNECTION_STRATEGY);

  private readonly defaultTimeoutMs = 30000;

  // ═══════════════════════════════════════════════════════════════════════════
  // Blob Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get full URL for a blob by hash.
   * Routes based on connection strategy (doorway vs direct).
   */
  getBlobUrl(blobHash: string): string {
    if (!blobHash) return '';

    // Use strategy to determine blob URL
    // The strategy knows the appropriate base URL for current mode
    const baseUrl = this.getStorageBaseUrl();

    // Doorway mode uses /api/blob/{hash}, direct mode uses /store/{hash}
    if (this.strategy.mode === 'direct') {
      return `${baseUrl}/store/${blobHash}`;
    }
    return `${baseUrl}/api/blob/${blobHash}`;
  }

  /**
   * Fetch blob data by hash.
   */
  fetchBlob(blobHash: string): Observable<ArrayBuffer> {
    const url = this.getBlobUrl(blobHash);
    return this.http.get(url, { responseType: 'arraybuffer' }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError((error) => this.handleError('fetchBlob', error))
    );
  }

  /**
   * Check if a blob exists in storage.
   */
  blobExists(blobHash: string): Observable<boolean> {
    const url = this.getBlobUrl(blobHash);
    return this.http.head(url, { observe: 'response' }).pipe(
      timeout(5000),
      map((response) => response.status === 200),
      catchError(() => of(false))
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Content Metadata Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get content node by ID.
   */
  getContent(id: string): Observable<StorageContentNode | null> {
    const baseUrl = this.getStorageBaseUrl();
    const endpoint = this.strategy.mode === 'direct'
      ? `${baseUrl}/db/content/${encodeURIComponent(id)}`
      : `${baseUrl}/api/db/content/${encodeURIComponent(id)}`;

    return this.http.get<StorageContentNode>(endpoint).pipe(
      timeout(this.defaultTimeoutMs),
      catchError((error) => {
        if (error.status === 404) return of(null);
        return this.handleError('getContent', error);
      })
    );
  }

  /**
   * Query content nodes with filters.
   */
  queryContent(filter: ContentFilter = {}): Observable<StorageContentNode[]> {
    const baseUrl = this.getStorageBaseUrl();
    const params = new URLSearchParams();

    if (filter.contentType) params.set('content_type', filter.contentType);
    if (filter.contentFormat) params.set('content_format', filter.contentFormat);
    if (filter.tags?.length) params.set('tags', filter.tags.join(','));
    if (filter.limit) params.set('limit', String(filter.limit));
    if (filter.offset) params.set('offset', String(filter.offset));

    const queryString = params.toString();
    const basePath = this.strategy.mode === 'direct' ? '/db/content' : '/api/db/content';
    const url = queryString ? `${baseUrl}${basePath}?${queryString}` : `${baseUrl}${basePath}`;

    return this.http.get<StorageContentNode[]>(url).pipe(
      timeout(this.defaultTimeoutMs),
      catchError((error) => this.handleError('queryContent', error))
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Path Metadata Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get path by ID.
   */
  getPath(id: string): Observable<StoragePath | null> {
    const baseUrl = this.getStorageBaseUrl();
    const endpoint = this.strategy.mode === 'direct'
      ? `${baseUrl}/db/paths/${encodeURIComponent(id)}`
      : `${baseUrl}/api/db/paths/${encodeURIComponent(id)}`;

    return this.http.get<StoragePath>(endpoint).pipe(
      timeout(this.defaultTimeoutMs),
      catchError((error) => {
        if (error.status === 404) return of(null);
        return this.handleError('getPath', error);
      })
    );
  }

  /**
   * Get all paths.
   */
  getAllPaths(): Observable<StoragePath[]> {
    const baseUrl = this.getStorageBaseUrl();
    const endpoint = this.strategy.mode === 'direct'
      ? `${baseUrl}/db/paths`
      : `${baseUrl}/api/db/paths`;

    return this.http.get<StoragePath[]>(endpoint).pipe(
      timeout(this.defaultTimeoutMs),
      catchError((error) => this.handleError('getAllPaths', error))
    );
  }

  /**
   * Get thumbnail URL for a path.
   * Returns blob URL if thumbnail_blob_hash is set, otherwise returns thumbnail_url.
   */
  getPathThumbnailUrl(path: StoragePath): string | null {
    if (path.thumbnail_blob_hash) {
      return this.getBlobUrl(path.thumbnail_blob_hash);
    }
    return path.thumbnail_url;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Connection Mode
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get current connection mode.
   */
  get connectionMode(): 'doorway' | 'direct' {
    return this.strategy.mode;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get the base URL for storage API based on connection strategy.
   */
  private getStorageBaseUrl(): string {
    // Strategy provides the base URL based on mode
    // In doorway mode: returns doorway URL (e.g., https://doorway-dev.elohim.host)
    // In direct mode: returns storage URL (e.g., http://localhost:8090)
    return this.strategy.getStorageBaseUrl(this.buildConnectionConfig());
  }

  /**
   * Build ConnectionConfig from environment for strategy methods.
   */
  private buildConnectionConfig(): ConnectionConfig {
    const hc = environment.holochain;
    return {
      mode: this.strategy.mode,
      adminUrl: hc?.adminUrl ?? '',
      appUrl: hc?.appUrl ?? '',
      proxyApiKey: hc?.proxyApiKey,
      storageUrl: hc?.storageUrl,
      appId: 'elohim',
      useLocalProxy: hc?.useLocalProxy,
    };
  }

  /**
   * Handle HTTP errors with consistent logging.
   */
  private handleError(operation: string, error: HttpErrorResponse): Observable<never> {
    const message = error.error?.message || error.message || 'Unknown error';
    console.error(`StorageClient.${operation} failed:`, message);
    return throwError(() => new Error(`${operation}: ${message}`));
  }
}
