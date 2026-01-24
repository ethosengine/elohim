/**
 * Storage Client Service
 *
 * Client for elohim-storage backend which provides:
 * - Blob store: Content-addressed storage for images, ZIPs, media
 * - SQL metadata: Content nodes, paths, projections
 *
 * Routes requests based on connection strategy:
 * - Doorway mode (browser): Blobs via /api/blob/{hash}, DB via /db/{table}
 * - Direct mode (Tauri): http://localhost:8090/blob/{hash}, /db/{table}
 *
 * In Eclipse Che, doorway is accessed via the hc-dev endpoint URL.
 */

import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Observable, catchError, map, of, throwError, timeout } from 'rxjs';

import { environment } from '../../../environments/environment';
import { CONNECTION_STRATEGY } from '../providers/connection-strategy.provider';
import type { IConnectionStrategy, ConnectionConfig } from '@elohim/service/connection';
import type { ListResponse, BulkCreateResult } from '../models/storage-response.model';

/** Content node from storage (matches backend ContentWithTags) */
export interface StorageContentNode {
  id: string;
  contentType: string;
  title: string;
  description: string;
  contentBody: string | null;
  contentFormat: string;
  blobHash: string | null;
  blobCid: string | null;
  metadataJson: string | null;
  tags: string[];
  createdAt: string;
  updatedAt: string;
  // Additional fields from backend
  reach?: string;              // visibility scope (commons, private, etc.)
  validationStatus?: string;  // draft, approved, etc.
  createdBy?: string;         // agent who created the content
  contentSizeBytes?: number; // size of blob content
}

/** Path from storage */
export interface StoragePath {
  id: string;
  version: string;
  title: string;
  description: string;
  difficulty: string;
  estimatedDuration: string | null;
  pathType: string;
  thumbnailUrl: string | null;
  thumbnailBlobHash: string | null;
  metadataJson: string | null;
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

/** Relationship between content nodes - camelCase for API */
export interface StorageRelationship {
  id?: string;               // Optional for creates
  sourceId: string;
  targetId: string;
  relationshipType: string; // RELATES_TO, CONTAINS, DEPENDS_ON, IMPLEMENTS, REFERENCES
  confidence?: number;       // 0.0-1.0
  inferenceSource?: string; // explicit, path, tag, semantic
  metadata?: Record<string, unknown>;  // Parsed JSON object
  createdAt?: string;
  updatedAt?: string;
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

    // Route based on connection mode:
    // - Direct mode: /blob/{hash} (directly to elohim-storage)
    // - Doorway mode: /api/blob/{hash} (doorway proxies to storage's /blob/{hash})
    if (this.strategy.mode === 'direct') {
      return `${baseUrl}/blob/${blobHash}`;
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
    // Doorway proxies /db/* routes to elohim-storage (no /api/ prefix for db)
    const endpoint = `${baseUrl}/db/content/${encodeURIComponent(id)}`;

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
   * @endpoint GET /db/content
   * @returns ListResponse with items, count, limit, offset
   */
  queryContent(filter: ContentFilter = {}): Observable<ListResponse<StorageContentNode>> {
    const baseUrl = this.getStorageBaseUrl();
    const params = new URLSearchParams();

    if (filter.contentType) params.set('contentType', filter.contentType);
    if (filter.contentFormat) params.set('contentFormat', filter.contentFormat);
    if (filter.tags?.length) params.set('tags', filter.tags.join(','));
    if (filter.limit) params.set('limit', String(filter.limit));
    if (filter.offset) params.set('offset', String(filter.offset));

    const queryString = params.toString();
    // Doorway proxies /db/* routes (no /api/ prefix for db)
    const url = queryString ? `${baseUrl}/db/content?${queryString}` : `${baseUrl}/db/content`;

    return this.http.get<ListResponse<StorageContentNode>>(url).pipe(
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
    // Doorway proxies /db/* routes (no /api/ prefix for db)
    const endpoint = `${baseUrl}/db/paths/${encodeURIComponent(id)}`;

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
   * @endpoint GET /db/paths
   * @returns ListResponse with items, count, limit, offset
   */
  getAllPaths(): Observable<ListResponse<StoragePath>> {
    const baseUrl = this.getStorageBaseUrl();
    // Doorway proxies /db/* routes (no /api/ prefix for db)
    const endpoint = `${baseUrl}/db/paths`;

    return this.http.get<ListResponse<StoragePath>>(endpoint).pipe(
      timeout(this.defaultTimeoutMs),
      catchError((error) => this.handleError('getAllPaths', error))
    );
  }

  /**
   * Get thumbnail URL for a path.
   * Returns blob URL if thumbnailBlobHash is set, otherwise returns thumbnailUrl.
   */
  getPathThumbnailUrl(path: StoragePath): string | null {
    if (path.thumbnailBlobHash) {
      return this.getBlobUrl(path.thumbnailBlobHash);
    }
    return path.thumbnailUrl;
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
  getStorageBaseUrl(): string {
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
   * Backend returns errors as: { "error": "message" }
   */
  private handleError(operation: string, error: HttpErrorResponse): Observable<never> {
    const errorBody = error.error;
    // Backend returns {"error": "..."}, not {"message": "..."}
    const message = errorBody?.error || errorBody?.message || error.message || 'Request failed';
    console.error(`StorageClient.${operation} failed:`, message, { status: error.status });
    return throwError(() => new Error(`${operation}: ${message}`));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Bulk Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Bulk create content items.
   * @endpoint POST /db/content/bulk
   * @returns BulkCreateResult with inserted/skipped counts
   */
  bulkCreateContent(items: Partial<StorageContentNode>[]): Observable<BulkCreateResult> {
    const baseUrl = this.getStorageBaseUrl();
    // Doorway proxies /db/* routes (no /api/ prefix for db)
    const endpoint = `${baseUrl}/db/content/bulk`;

    return this.http.post<BulkCreateResult>(endpoint, items).pipe(
      timeout(120000), // 2 min for bulk ops
      catchError((error) => this.handleError('bulkCreateContent', error))
    );
  }

  /**
   * Bulk create paths.
   * @endpoint POST /db/paths/bulk
   * @returns BulkCreateResult with inserted/skipped counts
   */
  bulkCreatePaths(items: Partial<StoragePath>[]): Observable<BulkCreateResult> {
    const baseUrl = this.getStorageBaseUrl();
    // Doorway proxies /db/* routes (no /api/ prefix for db)
    const endpoint = `${baseUrl}/db/paths/bulk`;

    return this.http.post<BulkCreateResult>(endpoint, items).pipe(
      timeout(120000),
      catchError((error) => this.handleError('bulkCreatePaths', error))
    );
  }

  /**
   * Bulk create relationships.
   * @endpoint POST /db/relationships/bulk
   * @returns BulkCreateResult with inserted/skipped counts
   */
  bulkCreateRelationships(items: StorageRelationship[]): Observable<BulkCreateResult> {
    const baseUrl = this.getStorageBaseUrl();
    // Doorway proxies /db/* routes (no /api/ prefix for db)
    const endpoint = `${baseUrl}/db/relationships/bulk`;

    return this.http.post<BulkCreateResult>(endpoint, items).pipe(
      timeout(120000),
      catchError((error) => this.handleError('bulkCreateRelationships', error))
    );
  }
}
