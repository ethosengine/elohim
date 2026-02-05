/**
 * Blob Manager Service - Phase 1: Orchestration
 *
 * High-level service that coordinates:
 * - Fetching blobs with fallback URLs
 * - Verifying blob integrity
 * - Caching decisions based on blob size
 * - Progress tracking for large downloads
 * - Error handling and recovery
 * - Metadata retrieval from Holochain DHT
 *
 * Note: Zome call payloads in this service use snake_case (e.g., content_id)
 * because Holochain zomes are Rust and expect snake_case field names.
 * This cannot be changed without updating the Rust zomes.
 */

import { Injectable, Injector } from '@angular/core';

// @coverage: 86.4% (2026-02-05)

import { map, catchError, tap, switchMap } from 'rxjs/operators';

import { Observable, from, of, throwError, firstValueFrom } from 'rxjs';

import { StorageClientService } from '../../elohim/services/storage-client.service';
import { ContentBlob } from '../models/content-node.model';

import { BlobFallbackService, BlobFetchResult, UrlHealth } from './blob-fallback.service';
import { BlobVerificationService, BlobVerificationResult } from './blob-verification.service';

// Import StorageClientService for strategy-aware blob URLs

/**
 * Full result of blob download and verification
 */
export interface BlobDownloadResult {
  /** The verified blob */
  blob: Blob;

  /** Blob metadata (from ContentBlob) */
  metadata: ContentBlob;

  /** Verification result */
  verification: BlobVerificationResult;

  /** Fetch result (which URL succeeded, etc.) */
  fetch: BlobFetchResult;

  /** Total duration in milliseconds */
  totalDurationMs: number;

  /** Whether blob was cached */
  wasCached: boolean;
}

/**
 * Download progress event
 */
export interface BlobDownloadProgress {
  /** Bytes downloaded so far */
  bytesDownloaded: number;

  /** Total bytes to download */
  totalBytes: number;

  /** Progress as percentage (0-100) */
  percentComplete: number;

  /** Estimated time remaining in seconds */
  estimatedSecondsRemaining?: number;

  /** Current download speed in Mbps */
  speedMbps?: number;

  /** Current phase ('fetching' | 'verifying') */
  phase: 'fetching' | 'verifying';
}

/**
 * Blob metadata output from Holochain zome
 * Matches BlobMetadataOutput in coordinator zome
 */
export interface BlobMetadataOutput {
  /** SHA256 hash of blob */
  hash: string;

  /** Size in bytes */
  sizeBytes: number;

  /** MIME type (video/mp4, audio/mpeg, etc.) */
  mimeType: string;

  /** Primary + fallback URLs */
  fallbackUrls: string[];

  /** Bitrate in Mbps (optional) */
  bitrateMbps?: number;

  /** Duration in seconds for audio/video */
  durationSeconds?: number;

  /** Codec (H.264, H.265, VP9, AAC, etc.) */
  codec?: string;

  /** When this blob was created */
  createdAt?: string;

  /** When this blob was last verified */
  verifiedAt?: string;
}

/**
 * Response from get_blobs_by_content_id zome call
 */
export interface BlobsForContentOutput {
  /** List of blobs for content */
  blobs: BlobMetadataOutput[];
}

@Injectable({
  providedIn: 'root',
})
export class BlobManagerService {
  /** Simple in-memory cache for blobs */
  private readonly blobCache = new Map<string, Blob>();

  /** Track cache size (bytes) */
  private cacheSize = 0;

  /** Maximum cache size (100 MB by default) */
  maxCacheSizeBytes = 100 * 1024 * 1024;

  /** Serialization lock for concurrent cache operations (FIX for race condition) */
  private cacheLock = Promise.resolve();

  /** Storage client for strategy-aware blob URLs (lazy injected) */
  private storageClient: StorageClientService | null = null;

  constructor(
    private readonly verificationService: BlobVerificationService,
    private readonly fallbackService: BlobFallbackService,
    private readonly injector: Injector
  ) {}

  // =========================================================================
  // Strategy-Aware Blob URL Methods
  // =========================================================================

  /**
   * Get blob storage URL based on connection strategy.
   *
   * Uses the StorageClientService's connection strategy to determine
   * the appropriate blob storage endpoint:
   *
   * - **Doorway mode**: `https://doorway-dev.elohim.host/api/blob/{hash}`
   * - **Direct mode**: `http://localhost:8090/store/{hash}`
   *
   * @param blobHash SHA256 hash of the blob
   * @returns URL string for the blob storage endpoint
   */
  getBlobUrl(blobHash: string): string {
    const client = this.getStorageClient();
    return client.getBlobUrl(blobHash);
  }

  /**
   * Get current connection mode for blob storage.
   *
   * @returns 'doorway' or 'direct'
   */
  get connectionMode(): 'doorway' | 'direct' {
    // Connection mode is determined by the environment
    const env = (globalThis as Record<string, unknown>)['__env'] as
      | Record<string, unknown>
      | undefined;
    return (env?.['connectionMode'] as 'doorway' | 'direct') ?? 'doorway';
  }

  /**
   * Get the priority URL for a blob based on connection strategy.
   * Combines strategy URL with existing fallback URLs for maximum availability.
   *
   * @param blobMetadata ContentBlob with fallback URLs
   * @returns Array of URLs with strategy URL first (if not already present)
   */
  getPriorityUrls(blobMetadata: ContentBlob): string[] {
    const strategyUrl = this.getBlobUrl(blobMetadata.hash);
    const fallbackUrls = blobMetadata.fallbackUrls ?? [];

    // Avoid duplicates: if strategy URL is already in fallbacks, don't add again
    if (fallbackUrls.includes(strategyUrl)) {
      return fallbackUrls;
    }

    // Strategy URL gets highest priority
    return [strategyUrl, ...fallbackUrls];
  }

  /**
   * Lazy-inject StorageClientService to avoid circular dependency.
   */
  private getStorageClient(): StorageClientService {
    this.storageClient ??= this.injector.get(StorageClientService);
    return this.storageClient;
  }

  /**
   * Download and verify a blob from ContentBlob metadata.
   *
   * @param blobMetadata ContentBlob with fallback URLs and hash
   * @param progressCallback Optional callback for download progress
   * @returns Observable with download result
   */
  downloadBlob(
    blobMetadata: ContentBlob,
    progressCallback?: (progress: BlobDownloadProgress) => void
  ): Observable<BlobDownloadResult> {
    const startTime = performance.now();
    const cacheKey = blobMetadata.hash;

    // Check cache first
    const cached = this.blobCache.get(cacheKey);
    if (cached) {
      if (progressCallback) {
        progressCallback({
          bytesDownloaded: blobMetadata.sizeBytes,
          totalBytes: blobMetadata.sizeBytes,
          percentComplete: 100,
          phase: 'fetching',
        });
      }

      return of({
        blob: cached,
        metadata: blobMetadata,
        verification: {
          isValid: true,
          computedHash: cacheKey,
          expectedHash: cacheKey,
          durationMs: 0,
        },
        fetch: {
          blob: cached,
          urlIndex: -1,
          successUrl: '(cached)',
          durationMs: 0,
          retryCount: 0,
        },
        totalDurationMs: 0,
        wasCached: true,
      });
    }

    // Fetch blob using priority URLs (strategy URL first, then fallbacks)
    const urls = this.getPriorityUrls(blobMetadata);
    return this.fallbackService.fetchWithFallback(urls).pipe(
      tap((_fetchResult: BlobFetchResult) => {
        if (progressCallback) {
          progressCallback({
            bytesDownloaded: blobMetadata.sizeBytes,
            totalBytes: blobMetadata.sizeBytes,
            percentComplete: 100,
            phase: 'verifying',
          });
        }
      }),
      switchMap((fetchResult: BlobFetchResult) =>
        // Verify blob integrity
        this.verificationService.verifyBlob(fetchResult.blob, blobMetadata.hash).pipe(
          map(verificationResult => ({
            fetchResult,
            verificationResult,
          }))
        )
      ),
      switchMap(({ fetchResult, verificationResult }) => {
        // Cache if verification successful and cache not full
        if (verificationResult.isValid) {
          return from(this.cacheBlob(cacheKey, fetchResult.blob, blobMetadata.sizeBytes)).pipe(
            map(() => ({ fetchResult, verificationResult }))
          );
        }
        return of({ fetchResult, verificationResult });
      }),
      map(({ fetchResult, verificationResult }) => {
        const totalDurationMs = performance.now() - startTime;

        if (!verificationResult.isValid) {
          throw new Error(
            `Blob verification failed. Expected: ${verificationResult.expectedHash}, Got: ${verificationResult.computedHash}`
          );
        }

        return {
          blob: fetchResult.blob,
          metadata: blobMetadata,
          verification: verificationResult,
          fetch: fetchResult,
          totalDurationMs,
          wasCached: false,
        };
      }),
      catchError(error => {
        return throwError(() => ({
          error: error.message,
          blob: blobMetadata,
          phase: 'download',
        }));
      })
    );
  }

  /**
   * Download multiple blobs in parallel.
   *
   * @param blobMetadatas Array of ContentBlob metadata
   * @param progressCallback Optional callback for overall progress
   * @returns Observable with array of download results
   */
  downloadBlobs(
    blobMetadatas: ContentBlob[],
    progressCallback?: (progress: BlobDownloadProgress) => void
  ): Observable<BlobDownloadResult[]> {
    const downloads = blobMetadatas.map(metadata => this.downloadBlob(metadata, progressCallback));

    return from(Promise.all(downloads.map(async d => firstValueFrom(d))));
  }

  /**
   * Check if blob is cached.
   *
   * @param hash SHA256 hash of blob
   * @returns True if blob is in cache
   */
  isCached(hash: string): boolean {
    return this.blobCache.has(hash);
  }

  /**
   * Get cached blob without download.
   *
   * @param hash SHA256 hash of blob
   * @returns Blob if cached, null otherwise
   */
  getCachedBlob(hash: string): Blob | null {
    return this.blobCache.get(hash) ?? null;
  }

  /**
   * Clear blob from cache.
   * Serialized with lock to prevent race conditions.
   *
   * @param hash SHA256 hash of blob to remove
   * @returns Promise that resolves when removal is complete
   */
  async removeFromCache(hash: string): Promise<void> {
    // Serialize with cache lock to prevent race conditions
    this.cacheLock = this.cacheLock.then(async () => {
      return new Promise<void>(resolve => {
        const blob = this.blobCache.get(hash);
        if (blob) {
          this.cacheSize -= blob.size;
          this.blobCache.delete(hash);
        }
        resolve();
      });
    });

    await this.cacheLock;
  }

  /**
   * Clear entire blob cache.
   * Serialized with lock to prevent race conditions.
   *
   * @returns Promise that resolves when cache is cleared
   */
  async clearCache(): Promise<void> {
    // Serialize with cache lock to prevent race conditions
    this.cacheLock = this.cacheLock.then(async () => {
      return new Promise<void>(resolve => {
        this.blobCache.clear();
        this.cacheSize = 0;
        resolve();
      });
    });

    await this.cacheLock;
  }

  /**
   * Get cache statistics.
   *
   * @returns Object with cache info
   */
  getCacheStats(): {
    entriesCount: number;
    sizeBytes: number;
    maxSizeBytes: number;
    percentFull: number;
  } {
    return {
      entriesCount: this.blobCache.size,
      sizeBytes: this.cacheSize,
      maxSizeBytes: this.maxCacheSizeBytes,
      percentFull: (this.cacheSize / this.maxCacheSizeBytes) * 100,
    };
  }

  /**
   * Get URL health information for a blob.
   * Includes strategy-aware URL as the first priority.
   *
   * @param blobMetadata ContentBlob to check
   * @returns Array of URL health statuses
   */
  getUrlHealth(blobMetadata: ContentBlob): UrlHealth[] {
    const urls = this.getPriorityUrls(blobMetadata);
    return this.fallbackService.getUrlsHealth(urls);
  }

  /**
   * Check if all fallback URLs for a blob are healthy.
   * Includes strategy-aware URL in the check.
   *
   * @param blobMetadata ContentBlob to check
   * @returns True if at least one URL is healthy
   */
  isAccessible(blobMetadata: ContentBlob): boolean {
    const health = this.getUrlHealth(blobMetadata);
    return health.some(h => h.isHealthy);
  }

  /**
   * Test all fallback URLs for a blob and report health.
   * Includes strategy-aware URL as first test.
   *
   * @param blobMetadata ContentBlob to test
   * @returns Promise with health report
   */
  async testBlobAccess(blobMetadata: ContentBlob): Promise<UrlHealth[]> {
    const urls = this.getPriorityUrls(blobMetadata);
    return this.fallbackService.testFallbackUrls(urls);
  }

  /**
   * Create a blob URL for use in HTML (video player, etc.).
   * Automatically cleaned up when blob is removed from cache.
   *
   * @param blob The Blob to create URL for
   * @returns Object URL string (blob://)
   */
  createBlobUrl(blob: Blob): string {
    return URL.createObjectURL(blob);
  }

  /**
   * Clean up blob URL (free memory).
   *
   * @param blobUrl The blob URL from createBlobUrl
   */
  revokeBlobUrl(blobUrl: string): void {
    URL.revokeObjectURL(blobUrl);
  }

  /**
   * Download blob to disk as file.
   * Uses browser's download API.
   *
   * @param blob The Blob to download
   * @param filename Desired filename for download
   */
  downloadBlobToFile(blob: Blob, filename: string): void {
    const url = this.createBlobUrl(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    this.revokeBlobUrl(url);
  }

  /**
   * Cache a blob (called internally after successful download).
   * Serialized with lock to prevent race conditions during concurrent downloads.
   *
   * @param hash SHA256 hash key
   * @param blob The Blob to cache
   * @param size Expected size (for cache planning)
   */
  private async cacheBlob(hash: string, blob: Blob, size: number): Promise<void> {
    // Serialize cache operations to prevent race conditions
    this.cacheLock = this.cacheLock.then(async () => {
      return new Promise<void>(resolve => {
        // Don't cache if blob is too large for cache
        if (size > this.maxCacheSizeBytes) {
          console.warn(
            `Blob too large to cache: ${size} bytes exceeds max ${this.maxCacheSizeBytes} bytes`
          );
          resolve();
          return;
        }

        // Re-check cache size AFTER acquiring lock (another download may have filled it)
        while (this.cacheSize + size > this.maxCacheSizeBytes && this.blobCache.size > 0) {
          // Remove first (oldest) entry (LRU)
          const firstKey = this.blobCache.keys().next().value;
          if (firstKey) {
            const evicted = this.blobCache.get(firstKey);
            if (evicted) {
              this.cacheSize -= evicted.size;
            }
            this.blobCache.delete(firstKey);
          }
        }

        // Only cache if it still fits
        if (this.cacheSize + size <= this.maxCacheSizeBytes) {
          this.blobCache.set(hash, blob);
          this.cacheSize += size;
        } else {
          // Blob too large for cache - not cached but still returned
        }

        resolve();
      });
    });

    await this.cacheLock;
  }

  // =========================================================================
  // Metadata Retrieval from Holochain DHT
  // =========================================================================

  /**
   * Retrieve all blobs for a content node from Holochain DHT.
   *
   * This method queries the Holochain DHT for all blobs associated with a content ID,
   * retrieving their metadata which includes hashes, fallback URLs, and codec info.
   *
   * @param contentId Content node ID to retrieve blobs for
   * @returns Observable with array of ContentBlob objects ready for download
   */
  getBlobsForContent(contentId: string): Observable<ContentBlob[]> {
    return from(this.callGetBlobsForContent(contentId)).pipe(
      map(output => {
        if (!output?.blobs || output.blobs.length === 0) {
          return [];
        }

        // Transform Holochain BlobMetadataOutput to ContentBlob
        return output.blobs.map(blob => this.transformBlobMetadata(blob));
      }),
      catchError(_error => {
        return of([]);
      })
    );
  }

  /**
   * Retrieve a specific blob's metadata from Holochain DHT by hash.
   *
   * Useful for getting detailed metadata about a blob before downloading,
   * or for verifying blob existence in the DHT.
   *
   * @param contentId Content node ID that owns the blob
   * @param blobHash SHA256 hash of blob to retrieve
   * @returns Observable with ContentBlob metadata or null if not found
   */
  getBlobMetadata(contentId: string, blobHash: string): Observable<ContentBlob | null> {
    return this.getBlobsForContent(contentId).pipe(
      map(blobs => {
        const found = blobs.find(b => b.hash === blobHash);
        return found ?? null;
      })
    );
  }

  /**
   * Check if a blob exists in Holochain DHT for a given content.
   *
   * @param contentId Content node ID
   * @param blobHash SHA256 hash of blob
   * @returns Observable with boolean indicating existence
   */
  blobExists(contentId: string, blobHash: string): Observable<boolean> {
    return this.getBlobMetadata(contentId, blobHash).pipe(map(metadata => metadata !== null));
  }

  /**
   * Retrieve blobs for multiple content nodes in parallel.
   *
   * @param contentIds Array of content node IDs
   * @returns Observable with map of content ID -> blobs array
   */
  getBlobsForMultipleContent(contentIds: string[]): Observable<Map<string, ContentBlob[]>> {
    const requests = contentIds.map(id =>
      this.getBlobsForContent(id).pipe(
        map(blobs => ({ contentId: id, blobs })),
        catchError(() => of({ contentId: id, blobs: [] }))
      )
    );

    return from(Promise.all(requests.map(async r => firstValueFrom(r)))).pipe(
      map(results => {
        const map = new Map<string, ContentBlob[]>();
        for (const result of results) {
          if (result) {
            map.set(result.contentId, result.blobs);
          }
        }
        return map;
      })
    );
  }

  // =========================================================================
  // Private Helper Methods
  // =========================================================================

  /**
   * Call Holochain zome function to get blobs for content.
   * Uses lazy Injector to avoid circular dependency issues.
   */
  private async callGetBlobsForContent(contentId: string): Promise<BlobsForContentOutput | null> {
    try {
      // Lazily inject HolochainClientService to avoid circular dependency
      const HolochainClientService = (await import('@app/elohim/services/holochain-client.service'))
        .HolochainClientService;
      const holochainClient = this.injector.get(HolochainClientService);

      const result = await holochainClient.callZome<BlobsForContentOutput>({
        zomeName: 'content_store',
        fnName: 'get_blobs_by_content_id',
        payload: { content_id: contentId },
      });

      if (!result.success || !result.data) {
        return null;
      }

      return result.data;
    } catch (error) {
      // Blob retrieval failure is non-critical - returns null to allow content to load without blobs
      // This can happen if Holochain is unavailable or content has no associated blobs
      if (error instanceof Error) {
        console.warn('[BlobManagerService] Failed to retrieve blobs for content:', error.message);
      }
      return null;
    }
  }

  /**
   * Transform Holochain BlobMetadataOutput to ContentBlob.
   */
  private transformBlobMetadata(metadata: BlobMetadataOutput): ContentBlob {
    return {
      hash: metadata.hash,
      sizeBytes: metadata.sizeBytes,
      mimeType: metadata.mimeType,
      fallbackUrls: metadata.fallbackUrls,
      bitrateMbps: metadata.bitrateMbps,
      durationSeconds: metadata.durationSeconds,
      codec: metadata.codec,
      createdAt: metadata.createdAt,
      verifiedAt: metadata.verifiedAt,
    };
  }
}
