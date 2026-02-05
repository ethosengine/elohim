/**
 * Doorway Client Service - Thin HTTP Client for Rust Backend
 *
 * Unified API client for all Doorway endpoints. Delegates heavy lifting
 * (blob caching, HLS/DASH generation, verification) to the Rust server,
 * making Angular a thin UI layer.
 *
 * ## Architecture
 *
 * ```
 * Angular (thin UI)
 *     │
 *     ▼
 * DoorwayClientService  ◄── This service
 *     │
 *     ▼
 * Doorway (Rust) ─────► Holochain DHT
 *     │                      │
 *     ▼                      ▼
 * Tiered Cache         Content Graph
 * ```
 *
 * ## Endpoints Used
 *
 * - GET /api/stream/hls/{content_id} - HLS master playlist
 * - GET /api/stream/dash/{content_id} - DASH MPD manifest
 * - GET /api/stream/chunk/{hash}/{index} - Media chunks
 * - POST /api/blob/verify - Server-side verification
 * - GET /api/custodian/blob/{hash} - Custodian list
 * - GET /api/custodian/blob/{hash}/best - Best custodian URL
 */

import { HttpClient, HttpHeaders, HttpParams } from '@angular/common/http';
import { Injectable } from '@angular/core';

// @coverage: 97.9% (2026-02-05)

import { map, catchError, timeout, retry } from 'rxjs/operators';

import { Observable, throwError } from 'rxjs';

import { environment } from '../../../environments/environment';

/**
 * Blob verification request
 */
export interface VerifyBlobRequest {
  /** Expected SHA256 hash (hex string, 64 chars) */
  expectedHash: string;
  /** Blob data as base64 (for small blobs) */
  dataBase64?: string;
  /** URL to fetch blob from (alternative to inline data) */
  fetchUrl?: string;
  /** Content ID for logging/tracing */
  contentId?: string;
}

/**
 * Blob verification response from server
 */
export interface VerifyBlobResponse {
  /** Whether the hash matches */
  isValid: boolean;
  /** Computed SHA256 hash (hex string) */
  computedHash: string;
  /** Expected hash (echoed back) */
  expectedHash: string;
  /** Size of data in bytes */
  sizeBytes: number;
  /** Time taken to verify in milliseconds */
  durationMs: number;
  /** Error message if verification failed */
  error?: string;
}

/**
 * Custodian information
 */
export interface CustodianInfo {
  /** Agent public key */
  agentId: string;
  /** Base URL for blob retrieval */
  baseUrl: string;
  /** Bandwidth capacity in Mbps */
  bandwidthMbps: number;
  /** Network latency in milliseconds */
  latencyMs: number;
  /** Uptime ratio (0.0 - 1.0) */
  uptimeRatio: number;
  /** Geographic region (if known) */
  region?: string;
  /** Selection score (higher is better) */
  score: number;
}

/**
 * Best custodian URL response
 */
export interface BestCustodianResponse {
  /** Direct URL to fetch blob */
  url: string;
  /** Selected custodian info */
  custodian: CustodianInfo;
  /** Fallback URLs if primary fails */
  fallbackUrls: string[];
}

/**
 * Streaming quality variant
 */
export interface StreamingVariant {
  /** Variant label (e.g., "720p") */
  label: string;
  /** Video width in pixels */
  width: number;
  /** Video height in pixels */
  height: number;
  /** Bitrate in Mbps */
  bitrateMbps: number;
  /** Variant hash */
  hash: string;
}

/**
 * Streaming manifest info
 */
export interface StreamingManifest {
  /** Content ID */
  contentId: string;
  /** Duration in seconds */
  durationSecs: number;
  /** Available variants */
  variants: StreamingVariant[];
  /** Master playlist URL (HLS) */
  hlsUrl: string;
  /** MPD manifest URL (DASH) */
  dashUrl: string;
}

@Injectable({
  providedIn: 'root',
})
export class DoorwayClientService {
  /** Base URL for Doorway API */
  private baseUrl: string;

  /** Default request timeout in milliseconds */
  private defaultTimeoutMs = 30000;

  /** Maximum retries for failed requests */
  private maxRetries = 3;

  constructor(private readonly http: HttpClient) {
    // Use environment config or default to same origin
    this.baseUrl = environment.doorwayUrl ?? '';
  }

  // ==========================================================================
  // Blob Retrieval
  // ==========================================================================

  /**
   * Get a blob by hash with optional byte range.
   *
   * @param hash Blob SHA256 hash
   * @param range Optional byte range (start, end)
   * @returns Observable with blob data
   */
  getBlob(hash: string, range?: { start: number; end: number }): Observable<ArrayBuffer> {
    let headers = new HttpHeaders();

    if (range) {
      headers = headers.set('Range', `bytes=${range.start}-${range.end}`);
    }

    return this.http
      .get(`${this.baseUrl}/api/blob/${hash}`, {
        headers,
        responseType: 'arraybuffer',
      })
      .pipe(
        timeout(this.defaultTimeoutMs),
        retry(this.maxRetries),
        catchError(error => this.handleError('getBlob', error))
      );
  }

  /**
   * Get a specific chunk of a blob.
   *
   * @param hash Blob hash
   * @param index Chunk index (0-based)
   * @returns Observable with chunk data
   */
  getChunk(hash: string, index: number): Observable<ArrayBuffer> {
    return this.http
      .get(`${this.baseUrl}/api/stream/chunk/${hash}/${index}`, {
        responseType: 'arraybuffer',
      })
      .pipe(
        timeout(this.defaultTimeoutMs),
        retry(this.maxRetries),
        catchError(error => this.handleError('getChunk', error))
      );
  }

  // ==========================================================================
  // Streaming URLs
  // ==========================================================================

  /**
   * Get HLS master manifest URL for a content item.
   *
   * @param contentId Content identifier
   * @returns Full URL for HLS playback
   */
  getHlsManifestUrl(contentId: string): string {
    return `${this.baseUrl}/api/stream/hls/${encodeURIComponent(contentId)}`;
  }

  /**
   * Get HLS variant manifest URL.
   *
   * @param contentId Content identifier
   * @param variant Variant label (e.g., "720p")
   * @returns Full URL for variant playlist
   */
  getHlsVariantUrl(contentId: string, variant: string): string {
    return `${this.baseUrl}/api/stream/hls/${encodeURIComponent(contentId)}/${encodeURIComponent(variant)}.m3u8`;
  }

  /**
   * Get DASH MPD manifest URL for a content item.
   *
   * @param contentId Content identifier
   * @returns Full URL for DASH playback
   */
  getDashManifestUrl(contentId: string): string {
    return `${this.baseUrl}/api/stream/dash/${encodeURIComponent(contentId)}`;
  }

  /**
   * Get chunk URL for direct chunk access.
   *
   * @param hash Blob hash
   * @param index Chunk index
   * @returns Full URL for chunk
   */
  getChunkUrl(hash: string, index: number): string {
    return `${this.baseUrl}/api/stream/chunk/${encodeURIComponent(hash)}/${index}`;
  }

  // ==========================================================================
  // Verification
  // ==========================================================================

  /**
   * Verify blob integrity using server-side SHA256.
   *
   * Use this as fallback when client-side verification is unavailable.
   *
   * @param request Verification request
   * @returns Observable with verification response
   */
  verifyBlob(request: VerifyBlobRequest): Observable<VerifyBlobResponse> {
    return this.http
      .post<VerifyBlobResponse>(`${this.baseUrl}/api/blob/verify`, {
        expected_hash: request.expectedHash,
        data_base64: request.dataBase64,
        fetch_url: request.fetchUrl,
        content_id: request.contentId,
      })
      .pipe(
        timeout(this.defaultTimeoutMs),
        map(response => ({
          isValid: response.isValid,
          computedHash: response.computedHash,
          expectedHash: response.expectedHash,
          sizeBytes: response.sizeBytes,
          durationMs: response.durationMs,
          error: response.error,
        })),
        catchError(error => this.handleError('verifyBlob', error))
      );
  }

  /**
   * Verify blob data directly (base64 encoded).
   *
   * @param data Blob data as Uint8Array or ArrayBuffer
   * @param expectedHash Expected SHA256 hash
   * @returns Observable with verification response
   */
  verifyBlobData(
    data: Uint8Array | ArrayBuffer,
    expectedHash: string
  ): Observable<VerifyBlobResponse> {
    const bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : data;
    const base64 = this.arrayBufferToBase64(bytes);

    return this.verifyBlob({
      expectedHash,
      dataBase64: base64,
    });
  }

  // ==========================================================================
  // Custodian Selection
  // ==========================================================================

  /**
   * Get list of custodians for a blob.
   *
   * @param hash Blob hash
   * @returns Observable with custodian list
   */
  getCustodiansForBlob(hash: string): Observable<CustodianInfo[]> {
    return this.http
      .get<{
        custodians: CustodianInfo[];
      }>(`${this.baseUrl}/api/custodian/blob/${encodeURIComponent(hash)}`)
      .pipe(
        timeout(this.defaultTimeoutMs),
        map(response => response.custodians),
        catchError(error => this.handleError('getCustodiansForBlob', error))
      );
  }

  /**
   * Get the best custodian URL for a blob.
   *
   * Server selects based on bandwidth, latency, uptime, and region.
   *
   * @param hash Blob hash
   * @param preferredRegion Optional preferred geographic region
   * @returns Observable with best URL and fallbacks
   */
  getBestCustodianUrl(hash: string, preferredRegion?: string): Observable<BestCustodianResponse> {
    let params = new HttpParams();
    if (preferredRegion) {
      params = params.set('region', preferredRegion);
    }

    return this.http
      .get<BestCustodianResponse>(
        `${this.baseUrl}/api/custodian/blob/${encodeURIComponent(hash)}/best`,
        { params }
      )
      .pipe(
        timeout(this.defaultTimeoutMs),
        catchError(error => this.handleError('getBestCustodianUrl', error))
      );
  }

  // ==========================================================================
  // Streaming Manifest (Optional - for when manifest content is needed)
  // ==========================================================================

  /**
   * Fetch HLS manifest content (useful for parsing).
   *
   * @param contentId Content identifier
   * @returns Observable with manifest text
   */
  fetchHlsManifest(contentId: string): Observable<string> {
    return this.http.get(this.getHlsManifestUrl(contentId), { responseType: 'text' }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('fetchHlsManifest', error))
    );
  }

  /**
   * Fetch DASH MPD manifest content.
   *
   * @param contentId Content identifier
   * @returns Observable with MPD XML text
   */
  fetchDashManifest(contentId: string): Observable<string> {
    return this.http.get(this.getDashManifestUrl(contentId), { responseType: 'text' }).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('fetchDashManifest', error))
    );
  }

  // ==========================================================================
  // Health & Status
  // ==========================================================================

  /**
   * Check Doorway health status.
   *
   * @returns Observable with health status
   */
  checkHealth(): Observable<{ status: string }> {
    return this.http.get<{ status: string }>(`${this.baseUrl}/health`).pipe(
      timeout(5000),
      catchError(error => this.handleError('checkHealth', error))
    );
  }

  /**
   * Get Doorway status with runtime info.
   *
   * @returns Observable with status info
   */
  getStatus(): Observable<Record<string, unknown>> {
    return this.http.get<Record<string, unknown>>(`${this.baseUrl}/status`).pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getStatus', error))
    );
  }

  // ==========================================================================
  // Configuration
  // ==========================================================================

  /**
   * Set the base URL for Doorway API.
   *
   * @param url New base URL
   */
  setBaseUrl(url: string): void {
    this.baseUrl = url;
  }

  /**
   * Get current base URL.
   *
   * @returns Current Doorway base URL
   */
  getBaseUrl(): string {
    return this.baseUrl;
  }

  /**
   * Set request timeout.
   *
   * @param timeoutMs Timeout in milliseconds
   */
  setDefaultTimeout(timeoutMs: number): void {
    this.defaultTimeoutMs = timeoutMs;
  }

  /**
   * Set maximum retries for failed requests.
   *
   * @param retries Maximum retry count
   */
  setMaxRetries(retries: number): void {
    this.maxRetries = retries;
  }

  // ==========================================================================
  // Private Helpers
  // ==========================================================================

  /**
   * Handle HTTP errors consistently.
   */
  private handleError(operation: string, error: unknown): Observable<never> {
    const message = error instanceof Error ? error.message : String(error);

    return throwError(() => new Error(`${operation} failed: ${message}`));
  }

  /**
   * Convert ArrayBuffer/Uint8Array to base64 string.
   */
  private arrayBufferToBase64(bytes: Uint8Array): string {
    let binary = '';
    for (let i = 0; i < bytes.byteLength; i++) {
      binary += String.fromCodePoint(bytes[i]);
    }
    return btoa(binary);
  }
}
