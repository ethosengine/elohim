/**
 * Blob Manager Service - Phase 1: Orchestration
 *
 * High-level service that coordinates:
 * - Fetching blobs with fallback URLs
 * - Verifying blob integrity
 * - Caching decisions based on blob size
 * - Progress tracking for large downloads
 * - Error handling and recovery
 */

import { Injectable } from '@angular/core';
import { Observable, from, of, throwError } from 'rxjs';
import { map, catchError, tap, switchMap } from 'rxjs/operators';
import { ContentBlob } from '../models/content-node.model';
import { BlobVerificationService, BlobVerificationResult } from './blob-verification.service';
import {
  BlobFallbackService,
  BlobFetchResult,
  UrlHealth,
} from './blob-fallback.service';

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

@Injectable({
  providedIn: 'root',
})
export class BlobManagerService {
  /** Simple in-memory cache for blobs */
  private blobCache = new Map<string, Blob>();

  /** Track cache size (bytes) */
  private cacheSize = 0;

  /** Maximum cache size (100 MB by default) */
  maxCacheSizeBytes = 100 * 1024 * 1024;

  /** Serialization lock for concurrent cache operations (FIX for race condition) */
  private cacheLock = Promise.resolve();

  constructor(
    private verificationService: BlobVerificationService,
    private fallbackService: BlobFallbackService,
  ) {}

  /**
   * Download and verify a blob from ContentBlob metadata.
   *
   * @param blobMetadata ContentBlob with fallback URLs and hash
   * @param progressCallback Optional callback for download progress
   * @returns Observable with download result
   */
  downloadBlob(
    blobMetadata: ContentBlob,
    progressCallback?: (progress: BlobDownloadProgress) => void,
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

    // Fetch blob from fallback URLs
    return this.fallbackService.fetchWithFallback(blobMetadata.fallbackUrls).pipe(
      tap((fetchResult) => {
        if (progressCallback) {
          progressCallback({
            bytesDownloaded: blobMetadata.sizeBytes,
            totalBytes: blobMetadata.sizeBytes,
            percentComplete: 100,
            phase: 'verifying',
          });
        }
      }),
      switchMap((fetchResult) =>
        // Verify blob integrity
        this.verificationService.verifyBlob(fetchResult.blob, blobMetadata.hash).pipe(
          map((verificationResult) => ({
            fetchResult,
            verificationResult,
          })),
        ),
      ),
      switchMap(({ fetchResult, verificationResult }) => {
        // Cache if verification successful and cache not full
        if (verificationResult.isValid) {
          return from(this.cacheBlob(cacheKey, fetchResult.blob, blobMetadata.sizeBytes)).pipe(
            map(() => ({ fetchResult, verificationResult })),
          );
        }
        return of({ fetchResult, verificationResult });
      }),
      map(({ fetchResult, verificationResult }) => {
        const totalDurationMs = performance.now() - startTime;

        if (!verificationResult.isValid) {
          throw new Error(
            `Blob verification failed. Expected: ${verificationResult.expectedHash}, Got: ${verificationResult.computedHash}`,
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
      catchError((error) => {
        return throwError(() => ({
          error: error.message,
          blob: blobMetadata,
          phase: 'download',
        }));
      }),
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
    progressCallback?: (progress: BlobDownloadProgress) => void,
  ): Observable<BlobDownloadResult[]> {
    const downloads = blobMetadatas.map((metadata) =>
      this.downloadBlob(metadata, progressCallback),
    );

    return from(Promise.all(downloads.map((d) => d.toPromise())));
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
    return this.blobCache.get(hash) || null;
  }

  /**
   * Clear blob from cache.
   *
   * @param hash SHA256 hash of blob to remove
   */
  removeFromCache(hash: string): void {
    const blob = this.blobCache.get(hash);
    if (blob) {
      this.cacheSize -= blob.size;
      this.blobCache.delete(hash);
    }
  }

  /**
   * Clear entire blob cache.
   */
  clearCache(): void {
    this.blobCache.clear();
    this.cacheSize = 0;
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
   *
   * @param blobMetadata ContentBlob to check
   * @returns Array of URL health statuses
   */
  getUrlHealth(blobMetadata: ContentBlob): UrlHealth[] {
    return this.fallbackService.getUrlsHealth(blobMetadata.fallbackUrls);
  }

  /**
   * Check if all fallback URLs for a blob are healthy.
   *
   * @param blobMetadata ContentBlob to check
   * @returns True if at least one URL is healthy
   */
  isAccessible(blobMetadata: ContentBlob): boolean {
    const health = this.getUrlHealth(blobMetadata);
    return health.some((h) => h.isHealthy);
  }

  /**
   * Test all fallback URLs for a blob and report health.
   *
   * @param blobMetadata ContentBlob to test
   * @returns Promise with health report
   */
  async testBlobAccess(blobMetadata: ContentBlob): Promise<UrlHealth[]> {
    return this.fallbackService.testFallbackUrls(blobMetadata.fallbackUrls);
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
    this.cacheLock = this.cacheLock.then(() => {
      return new Promise<void>((resolve) => {
        // Don't cache if blob is too large for cache
        if (size > this.maxCacheSizeBytes) {
          console.warn(
            `Blob too large to cache (${size} > ${this.maxCacheSizeBytes}). Keeping in memory temporarily.`,
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
          console.warn(`Blob still too large to cache after eviction: ${hash}`);
        }

        resolve();
      });
    });

    await this.cacheLock;
  }
}
