/**
 * Blob Streaming Service - Thin Client for Doorway Streaming
 *
 * Delegates streaming logic to the Rust Doorway backend:
 * - HLS/DASH manifest URLs from Doorway
 * - Chunk-based downloading via Doorway APIs
 * - Custodian selection handled server-side
 * - Bandwidth detection and quality selection
 *
 * The heavy lifting (manifest generation, caching, custodian selection)
 * is done by Doorway. This service is a thin UI layer.
 */

import { HttpClient, HttpHeaders, HttpResponse } from '@angular/common/http';
import { Injectable } from '@angular/core';

// @coverage: 47.9% (2026-02-05)

import { Observable } from 'rxjs';

import { DoorwayClientService } from '../../elohim/services/doorway-client.service';
import { ContentBlob } from '../models/content-node.model';

/**
 * Range request options
 */
export interface RangeRequestOptions {
  start: number;
  end: number;
  autoRetry?: boolean;
  maxRetries?: number;
}

/**
 * Streaming progress event
 */
export interface StreamingProgress {
  bytesReceived: number;
  totalBytes: number;
  percentComplete: number;
  averageSpeedMbps: number;
  estimatedTimeRemainingSeconds: number;
  chunkIndex: number;
  totalChunks: number;
  isBuffering: boolean;
}

/**
 * Chunk download result
 */
export interface ChunkDownloadResult {
  chunkIndex: number;
  data: Uint8Array;
  startByte: number;
  endByte: number;
  durationMs: number;
}

/**
 * Chunk validation result for detecting missing/corrupted chunks
 */
export interface ChunkValidationResult {
  /** Whether all chunks were successfully downloaded */
  isValid: boolean;

  /** Total number of expected chunks */
  totalChunks: number;

  /** Number of chunks successfully downloaded */
  successfulChunks: number;

  /** Indices of missing chunks (0-based) */
  missingChunkIndices: number[];

  /** Indices of chunks with errors */
  failedChunkIndices: number[];

  /** Error messages for failed chunks */
  chunkErrors: Map<number, string>;

  /** Total expected size in bytes */
  expectedSizeBytes: number;

  /** Actual size of reassembled data */
  actualSizeBytes: number;
}

/**
 * Bandwidth probe result
 */
export interface BandwidthProbeResult {
  averageSpeedMbps: number;
  minSpeedMbps: number;
  maxSpeedMbps: number;
  probeDataSize: number;
  probeDurationMs: number;
  latencyMs: number;
}

/**
 * Recommended quality based on bandwidth
 */
export interface QualityRecommendation {
  variant: string; // "480p", "720p", "1080p", "1440p", "2160p", "4320p"
  bitrateMbps: number;
  reasoningScore: number; // 0.0-1.0 confidence
}

/**
 * Download performance metrics for auto-tuning
 */
export interface DownloadPerformanceMetrics {
  /** Number of parallel chunks used */
  parallelChunks: number;

  /** Actual bandwidth achieved in Mbps */
  achievedBandwidthMbps: number;

  /** Success rate (0.0-1.0) */
  successRate: number;

  /** Average chunk duration in milliseconds */
  avgChunkDurationMs: number;

  /** Whether network seemed congested */
  wasCongested: boolean;
}

@Injectable({
  providedIn: 'root',
})
export class BlobStreamingService {
  /** Size of each chunk for parallel downloads (5 MB default) */
  chunkSizeBytes = 5 * 1024 * 1024;

  /** Timeout for individual chunk downloads */
  chunkTimeoutMs = 30000;

  /** Maximum parallel chunks to download (auto-tuned between 1 and 16) */
  maxParallelChunks = 4;

  /** Minimum parallel chunks (for stability) */
  private readonly minParallelChunks = 1;

  /** Maximum parallel chunks (limit resource usage) */
  private readonly maxMaxParallelChunks = 16;

  /** Cached bandwidth probes (expires after 10 minutes) */
  private readonly bandwidthCache = new Map<
    string,
    { result: BandwidthProbeResult; timestamp: number }
  >();

  /** Performance metrics history for auto-tuning */
  private performanceHistory: DownloadPerformanceMetrics[] = [];

  /** Maximum history entries to keep */
  private readonly maxHistorySize = 20;

  constructor(
    private readonly http: HttpClient,
    private readonly doorway: DoorwayClientService
  ) {}

  /**
   * Get download performance history for diagnostics.
   *
   * @returns Array of recent download performance metrics
   */
  getPerformanceHistory(): DownloadPerformanceMetrics[] {
    return [...this.performanceHistory];
  }

  /**
   * Calculate optimal number of parallel chunks based on bandwidth.
   *
   * Uses heuristic:
   * - Slow networks (<5 Mbps): 1-2 chunks
   * - Medium networks (5-20 Mbps): 2-4 chunks
   * - Fast networks (20-100 Mbps): 4-8 chunks
   * - Very fast networks (>100 Mbps): 8-16 chunks
   *
   * @param bandwidthMbps Available bandwidth in Mbps
   * @param latencyMs Network latency in milliseconds
   * @returns Optimal number of parallel chunks
   */
  calculateOptimalParallelChunks(bandwidthMbps: number, latencyMs: number): number {
    // Base calculation on bandwidth
    let optimal = 1;

    if (bandwidthMbps < 5) {
      // Very slow - single chunk (optimal already = 1)
    } else if (bandwidthMbps < 20) {
      optimal = 2; // Slow
    } else if (bandwidthMbps < 50) {
      optimal = 4; // Medium
    } else if (bandwidthMbps < 100) {
      optimal = 8; // Fast
    } else {
      optimal = 12; // Very fast
    }

    // Adjust for latency (high latency benefits from more parallelism)
    if (latencyMs > 200) {
      optimal = Math.min(optimal + 2, this.maxMaxParallelChunks);
    }

    return Math.max(this.minParallelChunks, Math.min(optimal, this.maxMaxParallelChunks));
  }

  /**
   * Auto-tune parallel chunk count based on recent performance.
   *
   * Increases parallelism if:
   * - Recent downloads show high success rate (>95%)
   * - No congestion detected
   * - Bandwidth usage not maxed out
   *
   * Decreases parallelism if:
   * - Recent failures detected (<80% success rate)
   * - Congestion indicators found
   * - Timeout rate too high
   *
   * @returns New optimal parallel chunk count
   */
  autoTuneParallelChunks(): number {
    if (this.performanceHistory.length === 0) {
      return this.maxParallelChunks;
    }

    // Calculate average metrics from recent history
    const recent = this.performanceHistory.slice(-5);
    const avgSuccessRate = recent.reduce((sum, m) => sum + m.successRate, 0) / recent.length;
    const congestionCount = recent.filter(m => m.wasCongested).length;

    let newValue = this.maxParallelChunks;

    if (avgSuccessRate < 0.8 || congestionCount > 2) {
      // Network issues detected - reduce parallelism
      newValue = Math.max(this.minParallelChunks, this.maxParallelChunks - 1);
    } else if (
      avgSuccessRate > 0.95 &&
      congestionCount === 0 &&
      this.maxParallelChunks < this.maxMaxParallelChunks
    ) {
      // Network performing well - try to increase parallelism
      newValue = Math.min(this.maxMaxParallelChunks, this.maxParallelChunks + 1);
    }

    this.maxParallelChunks = newValue;
    return newValue;
  }

  /**
   * Record download performance metrics for auto-tuning feedback.
   *
   * @param metrics Performance metrics from a download
   */
  recordDownloadMetrics(metrics: DownloadPerformanceMetrics): void {
    this.performanceHistory.push(metrics);

    // Keep history size bounded
    if (this.performanceHistory.length > this.maxHistorySize) {
      this.performanceHistory.shift();
    }

    // Auto-tune based on recent history
    this.autoTuneParallelChunks();
  }

  /**
   * Get average download performance statistics.
   *
   * Useful for diagnostics and monitoring.
   *
   * @returns Average metrics across all recorded downloads
   */
  getAveragePerformance(): Partial<DownloadPerformanceMetrics> | null {
    if (this.performanceHistory.length === 0) {
      return null;
    }

    const history = this.performanceHistory;
    return {
      parallelChunks: history.reduce((sum, m) => sum + m.parallelChunks, 0) / history.length,
      achievedBandwidthMbps:
        history.reduce((sum, m) => sum + m.achievedBandwidthMbps, 0) / history.length,
      successRate: history.reduce((sum, m) => sum + m.successRate, 0) / history.length,
      avgChunkDurationMs:
        history.reduce((sum, m) => sum + m.avgChunkDurationMs, 0) / history.length,
      wasCongested: history.some(m => m.wasCongested),
    };
  }

  /**
   * Reset performance history (useful for testing or after network change).
   */
  resetPerformanceMetrics(): void {
    this.performanceHistory = [];
    this.maxParallelChunks = 4; // Reset to default
  }

  /**
   * Download blob in resumable chunks.
   * Allows pause/resume, parallel downloads, and progress tracking.
   *
   * @param blob ContentBlob metadata
   * @param url URL to download from
   * @param progressCallback Optional progress callback
   * @param abortSignal Optional abort signal for cancellation
   * @returns Observable with complete blob data
   */
  downloadInChunks(
    blob: ContentBlob,
    url: string,
    progressCallback?: (progress: StreamingProgress) => void,
    abortSignal?: AbortSignal
  ): Observable<Blob> {
    return new Observable(subscriber => {
      this.performChunkedDownload(blob, url, progressCallback, abortSignal)
        .then(data => {
          subscriber.next(new Blob([data]));
          subscriber.complete();
        })
        .catch(error => subscriber.error(error));
    });
  }

  /**
   * Internal method: perform chunked download with resumable support.
   */
  private async performChunkedDownload(
    blob: ContentBlob,
    url: string,
    progressCallback?: (progress: StreamingProgress) => void,
    abortSignal?: AbortSignal
  ): Promise<Uint8Array> {
    const totalSize = blob.sizeBytes;
    const chunkCount = Math.ceil(totalSize / this.chunkSizeBytes);
    const chunks = new Map<number, Uint8Array>();
    let downloadedBytes = 0;
    const startTime = performance.now();

    // Check if server supports Range requests
    const supportsRange = await this.checkRangeSupport(url);
    if (!supportsRange) {
      // Fallback to single request if Range not supported
      return this.downloadSingleRequest(url);
    }

    // Track chunk download errors and results
    const chunkErrors = new Map<number, string>();

    // Create array of chunk download promises
    const createChunkPromise = async (chunkIndex: number): Promise<void> => {
      try {
        const chunk = await this.downloadChunk(url, chunkIndex, this.chunkSizeBytes, totalSize);
        chunks.set(chunk.chunkIndex, chunk.data);
        downloadedBytes += chunk.data.length;

        if (progressCallback) {
          const elapsed = performance.now() - startTime;
          const speedMbps = downloadedBytes / 1024 / 1024 / (elapsed / 1000);

          progressCallback({
            bytesReceived: downloadedBytes,
            totalBytes: totalSize,
            percentComplete: (downloadedBytes / totalSize) * 100,
            averageSpeedMbps: speedMbps,
            estimatedTimeRemainingSeconds:
              (totalSize - downloadedBytes) / (speedMbps * 1024 * 1024),
            chunkIndex,
            totalChunks: chunkCount,
            isBuffering: false,
          });
        }

        abortSignal?.throwIfAborted();
      } catch (error) {
        // Capture chunk download errors instead of failing entire download
        const errorMsg = error instanceof Error ? error.message : String(error);
        chunkErrors.set(chunkIndex, errorMsg);
      }
    };

    // Download chunks in parallel (up to maxParallelChunks at a time)
    await this.downloadChunksInParallel(chunkCount, createChunkPromise, this.maxParallelChunks);

    // Validate chunks before reassembly
    const validation = this.validateChunks(chunks, chunkCount, totalSize, chunkErrors);

    if (!validation.isValid) {
      const missingList = validation.missingChunkIndices.join(', ');
      const failedList = validation.failedChunkIndices.join(', ');
      throw new Error(
        `Chunk download validation failed. Missing: [${missingList}], Failed: [${failedList}]. ` +
          `Got ${validation.successfulChunks}/${validation.totalChunks} chunks.`
      );
    }

    // Assemble chunks in order (all chunks are now validated to exist)
    const result = new Uint8Array(totalSize);
    let offset = 0;

    for (let i = 0; i < chunkCount; i++) {
      const chunk = chunks.get(i);
      // Safe to assume chunk exists due to validation above
      if (chunk) {
        result.set(chunk, offset);
        offset += chunk.length;
      }
    }

    return result;
  }

  /**
   * Download a single chunk with retries.
   */
  private async downloadChunk(
    url: string,
    chunkIndex: number,
    chunkSize: number,
    totalSize: number
  ): Promise<ChunkDownloadResult> {
    const startByte = chunkIndex * chunkSize;
    const endByte = Math.min(startByte + chunkSize - 1, totalSize - 1);

    const startTime = performance.now();

    const headers = new HttpHeaders({
      Range: `bytes=${startByte}-${endByte}`,
    });

    return new Promise((resolve, reject) => {
      this.http
        .get(url, {
          headers,
          responseType: 'arraybuffer',
        })
        .subscribe({
          next: (data: ArrayBuffer) => {
            const durationMs = performance.now() - startTime;
            resolve({
              chunkIndex,
              data: new Uint8Array(data),
              startByte,
              endByte,
              durationMs,
            });
          },
          error: err => reject(err),
        });
    });
  }

  /**
   * Validate that all expected chunks were successfully downloaded.
   *
   * Detects missing and failed chunks before reassembly to prevent silent
   * data corruption. Throws an error if chunks are missing.
   *
   * @param chunks Map of chunk index to data
   * @param totalChunks Total number of expected chunks
   * @param totalSize Total size in bytes
   * @param chunkErrors Map of chunk errors that occurred
   * @returns Validation result with details on missing/failed chunks
   */
  private validateChunks(
    chunks: Map<number, Uint8Array>,
    totalChunks: number,
    totalSize: number,
    chunkErrors: Map<number, string>
  ): ChunkValidationResult {
    const missingChunkIndices: number[] = [];
    const failedChunkIndices: number[] = [];

    // Check for missing chunks
    for (let i = 0; i < totalChunks; i++) {
      if (!chunks.has(i)) {
        missingChunkIndices.push(i);

        // Check if this chunk has a recorded error
        if (chunkErrors.has(i)) {
          failedChunkIndices.push(i);
        }
      }
    }

    // Calculate actual reassembled size
    let actualSize = 0;
    for (const chunk of chunks.values()) {
      actualSize += chunk.length;
    }

    const successfulChunks = chunks.size;
    const isValid = missingChunkIndices.length === 0 && actualSize === totalSize;

    return {
      isValid,
      totalChunks,
      successfulChunks,
      missingChunkIndices,
      failedChunkIndices,
      chunkErrors,
      expectedSizeBytes: totalSize,
      actualSizeBytes: actualSize,
    };
  }

  /**
   * Check if server supports HTTP Range requests (206 Partial Content).
   *
   * HTTP Semantics:
   * - 206 Partial Content: Server accepted Range header and returned partial content (Range SUPPORTED)
   * - 200 OK: Server ignored Range header and returned full content (Range NOT supported)
   * - 4xx/5xx: Error state
   */
  private async checkRangeSupport(url: string): Promise<boolean> {
    return new Promise(resolve => {
      const headers = new HttpHeaders({
        Range: 'bytes=0-1', // Request exactly 2 bytes
      });

      this.http
        .head(url, {
          headers,
          observe: 'response',
        })
        .subscribe({
          next: (response: HttpResponse<object>) => {
            // ONLY 206 indicates Range support
            // NEVER accept 200 as Range support - that means server ignored the Range header
            const contentRange = response.headers.get('Content-Range');
            const isRangeSupported: boolean =
              response.status === 206 && !!contentRange && contentRange.includes('0-1');

            if (response.status === 200 && response.headers.has('Accept-Ranges')) {
              // Server explicitly advertises range support but test returned 200
              // This is a server misconfiguration - trust Accept-Ranges header
              const acceptRanges = response.headers.get('Accept-Ranges');
              const canDoRanges = acceptRanges && acceptRanges.toLowerCase() !== 'none';
              resolve(!!canDoRanges);
            } else {
              resolve(isRangeSupported);
            }
          },
          error: () => resolve(false), // Assume no Range support on error
        });
    });
  }

  /**
   * Fallback: download entire blob in single request.
   */
  private async downloadSingleRequest(url: string): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      this.http.get(url, { responseType: 'arraybuffer' }).subscribe({
        next: (data: ArrayBuffer) => resolve(new Uint8Array(data)),
        error: err => reject(err),
      });
    });
  }

  /**
   * Format chunk validation result as human-readable error message.
   * Useful for logging or reporting chunk download failures.
   *
   * @param validation Validation result from chunk download
   * @returns Human-readable error message
   */
  formatValidationError(validation: ChunkValidationResult): string {
    if (validation.isValid) {
      return 'All chunks downloaded successfully';
    }

    const parts: string[] = [];

    if (validation.missingChunkIndices.length > 0) {
      parts.push(
        `Missing chunks: [${validation.missingChunkIndices.join(', ')}] ` +
          `(${validation.missingChunkIndices.length}/${validation.totalChunks})`
      );
    }

    if (validation.failedChunkIndices.length > 0) {
      const errorDetails = validation.failedChunkIndices
        .slice(0, 3) // Show first 3 errors
        .map(idx => `chunk ${idx}: ${validation.chunkErrors.get(idx)}`)
        .join('; ');

      parts.push(
        `Failed chunks: ${validation.failedChunkIndices.length} ` +
          `(${errorDetails}${validation.failedChunkIndices.length > 3 ? '...' : ''})`
      );
    }

    if (validation.expectedSizeBytes !== validation.actualSizeBytes) {
      parts.push(
        `Size mismatch: expected ${validation.expectedSizeBytes} bytes, ` +
          `got ${validation.actualSizeBytes} bytes`
      );
    }

    return parts.join('; ');
  }

  /**
   * Probe network bandwidth using small test download.
   * Cached for 10 minutes to avoid excessive probing.
   *
   * @param url Sample URL to probe (use small file if possible)
   * @param probeSizeBytes Size of probe data (default 1 MB)
   * @returns Bandwidth measurement result
   */
  async probeBandwidth(
    url: string,
    _probeSizeBytes: number = 1024 * 1024
  ): Promise<BandwidthProbeResult> {
    // Check cache
    const cached = this.bandwidthCache.get(url);
    if (cached && performance.now() - cached.timestamp < 10 * 60 * 1000) {
      return cached.result;
    }

    const startTime = performance.now();

    try {
      // Download probe data with timing
      const data = await new Promise<ArrayBuffer>((resolve, reject) => {
        this.http.get(url, { responseType: 'arraybuffer' }).subscribe({
          next: arrayBuffer => {
            resolve(arrayBuffer);
          },
          error: reject,
        });
      });

      const probeDurationMs = performance.now() - startTime;
      const sizeInMb = data.byteLength / 1024 / 1024;
      const durationInSeconds = probeDurationMs / 1000;
      const speedMbps = sizeInMb / durationInSeconds;

      const result: BandwidthProbeResult = {
        averageSpeedMbps: speedMbps,
        minSpeedMbps: speedMbps * 0.8, // Conservative estimate
        maxSpeedMbps: speedMbps * 1.2, // Optimistic estimate
        probeDataSize: data.byteLength,
        probeDurationMs,
        latencyMs: Math.round(Math.min(probeDurationMs * 0.1, 100)), // Simplified estimate - 10% of probe time, capped at 100ms
      };

      // Cache result
      this.bandwidthCache.set(url, { result, timestamp: performance.now() });

      return result;
    } catch (error) {
      throw new Error(
        `Bandwidth probe failed: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /**
   * Recommend quality based on current bandwidth.
   * Uses blob variants to select best match.
   *
   * @param blob ContentBlob with variants
   * @param bandwidthMbps Current bandwidth measurement
   * @returns Quality recommendation
   */
  recommendQuality(blob: ContentBlob, bandwidthMbps: number): QualityRecommendation {
    if (!blob.variants || blob.variants.length === 0) {
      return {
        variant: 'default',
        bitrateMbps: blob.bitrateMbps ?? 5,
        reasoningScore: 1.0,
      };
    }

    // Quality tiers and their bitrate requirements
    const qualityTiers = [
      { variant: '480p', minBitrate: 1.5 },
      { variant: '720p', minBitrate: 3 },
      { variant: '1080p', minBitrate: 5 },
      { variant: '1440p', minBitrate: 8 },
      { variant: '2160p', minBitrate: 15 },
      { variant: '4320p', minBitrate: 25 },
    ];

    // Find best match: use highest quality where bitrate <= 80% of available bandwidth
    const maxBitrate = bandwidthMbps * 0.8; // Leave 20% headroom
    let bestVariant = qualityTiers[0]; // Default to lowest quality
    let bestScore = 0;

    for (const tier of qualityTiers) {
      // Check if blob has this variant
      const hasVariant = blob.variants.some(v => v.label === tier.variant);
      if (!hasVariant) continue;

      // Only consider variants that fit within bandwidth (bitrate <= maxBitrate)
      if (tier.minBitrate > maxBitrate) continue;

      // Score: how much of available bandwidth we use (higher is better, up to 100%)
      const score = tier.minBitrate / maxBitrate;
      if (score > bestScore) {
        bestScore = score;
        bestVariant = tier;
      }
    }

    return {
      variant: bestVariant.variant,
      bitrateMbps: bestVariant.minBitrate,
      reasoningScore: bestScore,
    };
  }

  // ==========================================================================
  // Streaming URLs (Delegated to Doorway)
  // ==========================================================================

  /**
   * Get HLS master manifest URL for a content item.
   *
   * Manifest is generated by Doorway server.
   *
   * @param contentId Content identifier
   * @returns Full URL for HLS playback
   */
  getHlsManifestUrl(contentId: string): string {
    return this.doorway.getHlsManifestUrl(contentId);
  }

  /**
   * Get HLS variant manifest URL.
   *
   * @param contentId Content identifier
   * @param variant Variant label (e.g., "720p")
   * @returns Full URL for variant playlist
   */
  getHlsVariantUrl(contentId: string, variant: string): string {
    return this.doorway.getHlsVariantUrl(contentId, variant);
  }

  /**
   * Get DASH MPD manifest URL for a content item.
   *
   * Manifest is generated by Doorway server.
   *
   * @param contentId Content identifier
   * @returns Full URL for DASH playback
   */
  getDashManifestUrl(contentId: string): string {
    return this.doorway.getDashManifestUrl(contentId);
  }

  /**
   * Get chunk URL for direct chunk access.
   *
   * @param hash Blob hash
   * @param index Chunk index
   * @returns Full URL for chunk
   */
  getChunkUrl(hash: string, index: number): string {
    return this.doorway.getChunkUrl(hash, index);
  }

  /**
   * Fetch HLS manifest content (for parsing/inspection).
   *
   * @param contentId Content identifier
   * @returns Observable with manifest text
   */
  fetchHlsManifest(contentId: string): Observable<string> {
    return this.doorway.fetchHlsManifest(contentId);
  }

  /**
   * Fetch DASH MPD manifest content.
   *
   * @param contentId Content identifier
   * @returns Observable with MPD XML text
   */
  fetchDashManifest(contentId: string): Observable<string> {
    return this.doorway.fetchDashManifest(contentId);
  }

  /**
   * Helper: extract width from resolution string ("1080p" -> 1920).
   */
  private extractResolutionWidth(resolution: string): number {
    const resolutionMap: Record<string, number> = {
      '480p': 854,
      '720p': 1280,
      '1080p': 1920,
      '1440p': 2560,
      '2160p': 3840,
      '4320p': 7680,
    };
    return resolutionMap[resolution] ?? 1920;
  }

  /**
   * Helper: extract height from resolution string ("1080p" -> 1080).
   */
  private extractResolutionHeight(resolution: string): number {
    const heightMap: Record<string, number> = {
      '480p': 480,
      '720p': 720,
      '1080p': 1080,
      '1440p': 1440,
      '2160p': 2160,
      '4320p': 4320,
    };
    return heightMap[resolution] ?? 1080;
  }

  /**
   * Helper method to download chunks in parallel with concurrency limit.
   *
   * @param chunkCount Total number of chunks to download
   * @param createChunkPromise Function that creates a promise for a given chunk index
   * @param maxParallel Maximum number of parallel downloads
   */
  private async downloadChunksInParallel(
    chunkCount: number,
    createChunkPromise: (index: number) => Promise<void>,
    maxParallel: number
  ): Promise<void> {
    const allPromises: Promise<void>[] = [];

    // First, create all promises
    for (let i = 0; i < chunkCount; i++) {
      allPromises.push(createChunkPromise(i));
    }

    // Then, execute them with concurrency limit
    for (let i = 0; i < allPromises.length; i += maxParallel) {
      const batch = allPromises.slice(i, i + maxParallel);
      await Promise.all(batch);
    }
  }

  /**
   * Clear bandwidth cache.
   * Useful for testing or forcing re-probe.
   */
  clearBandwidthCache(): void {
    this.bandwidthCache.clear();
  }
}
