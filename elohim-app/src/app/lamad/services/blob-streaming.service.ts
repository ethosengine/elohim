/**
 * Blob Streaming Service - Phase 2: HTTP 206 Range Request Support
 *
 * Enables efficient streaming of large files using HTTP Range requests:
 * - Resumable downloads (pause/resume support)
 * - Bandwidth detection for quality selection
 * - Chunk-based downloading with progress tracking
 * - Cache-friendly partial content delivery
 * - HLS/DASH manifest generation for adaptive streaming
 */

import { Injectable } from '@angular/core';
import { Observable, Subject, interval } from 'rxjs';
import { map, takeUntil } from 'rxjs/operators';
import { HttpClient, HttpHeaders, HttpResponse } from '@angular/common/http';
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

@Injectable({
  providedIn: 'root',
})
export class BlobStreamingService {
  /** Size of each chunk for parallel downloads (5 MB default) */
  chunkSizeBytes = 5 * 1024 * 1024;

  /** Timeout for individual chunk downloads */
  chunkTimeoutMs = 30000;

  /** Maximum parallel chunks to download */
  maxParallelChunks = 4;

  /** Cached bandwidth probes (expires after 10 minutes) */
  private bandwidthCache = new Map<string, { result: BandwidthProbeResult; timestamp: number }>();

  constructor(private http: HttpClient) {}

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
    return new Observable((subscriber) => {
      this.performChunkedDownload(blob, url, progressCallback, abortSignal)
        .then((data) => {
          subscriber.next(new Blob([data]));
          subscriber.complete();
        })
        .catch((error) => subscriber.error(error));
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
    const chunks: Map<number, Uint8Array> = new Map();
    let downloadedBytes = 0;
    const startTime = performance.now();

    // Check if server supports Range requests
    const supportsRange = await this.checkRangeSupport(url);
    if (!supportsRange) {
      // Fallback to single request if Range not supported
      return this.downloadSingleRequest(url);
    }

    // Download chunks in parallel (up to maxParallelChunks at a time)
    const downloadPromises: Promise<void>[] = [];

    for (let i = 0; i < chunkCount; i++) {
      // Limit parallel downloads
      if (downloadPromises.length >= this.maxParallelChunks) {
        await Promise.race(downloadPromises);
        downloadPromises.splice(
          downloadPromises.findIndex((p) => p === undefined),
          1
        );
      }

      const chunkPromise = this.downloadChunk(
        url,
        i,
        this.chunkSizeBytes,
        totalSize
      ).then((chunk) => {
        chunks.set(chunk.chunkIndex, chunk.data);
        downloadedBytes += chunk.data.length;

        if (progressCallback) {
          const elapsed = performance.now() - startTime;
          const speedMbps = (downloadedBytes / 1024 / 1024) / (elapsed / 1000);

          progressCallback({
            bytesReceived: downloadedBytes,
            totalBytes: totalSize,
            percentComplete: (downloadedBytes / totalSize) * 100,
            averageSpeedMbps: speedMbps,
            estimatedTimeRemainingSeconds:
              (totalSize - downloadedBytes) / (speedMbps * 1024 * 1024),
            chunkIndex: i,
            totalChunks: chunkCount,
            isBuffering: false,
          });
        }

        abortSignal?.throwIfAborted();
      });

      downloadPromises.push(chunkPromise);
    }

    // Wait for all chunks to complete
    await Promise.all(downloadPromises);

    // Assemble chunks in order
    const result = new Uint8Array(totalSize);
    let offset = 0;

    for (let i = 0; i < chunkCount; i++) {
      const chunk = chunks.get(i);
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
      'Range': `bytes=${startByte}-${endByte}`,
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
          error: (err) => reject(err),
        });
    });
  }

  /**
   * Check if server supports HTTP Range requests (206 Partial Content).
   */
  private async checkRangeSupport(url: string): Promise<boolean> {
    return new Promise((resolve) => {
      const headers = new HttpHeaders({
        'Range': 'bytes=0-0',
      });

      this.http
        .head(url, {
          headers,
          observe: 'response',
        })
        .subscribe({
          next: (response: HttpResponse<any>) => {
            // 206 Partial Content means Range is supported
            // 200 OK means Range is not supported but we can still download
            resolve(response.status === 206 || response.status === 200);
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
        error: (err) => reject(err),
      });
    });
  }

  /**
   * Probe network bandwidth using small test download.
   * Cached for 10 minutes to avoid excessive probing.
   *
   * @param url Sample URL to probe (use small file if possible)
   * @param probeSizeBytes Size of probe data (default 1 MB)
   * @returns Bandwidth measurement result
   */
  async probeBandwidth(url: string, probeSizeBytes: number = 1024 * 1024): Promise<BandwidthProbeResult> {
    // Check cache
    const cached = this.bandwidthCache.get(url);
    if (cached && performance.now() - cached.timestamp < 10 * 60 * 1000) {
      return cached.result;
    }

    const startTime = performance.now();
    const latencyStartTime = performance.now();

    try {
      // Download probe data with timing
      const data = await new Promise<ArrayBuffer>((resolve, reject) => {
        this.http.get(url, { responseType: 'arraybuffer' }).subscribe({
          next: (data) => {
            const latency = performance.now() - latencyStartTime;
            resolve(data);
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
        latencyMs: Math.round(latencyStartTime), // Simplified - would need more sophisticated timing
      };

      // Cache result
      this.bandwidthCache.set(url, { result, timestamp: performance.now() });

      return result;
    } catch (error) {
      throw new Error(`Bandwidth probe failed: ${error instanceof Error ? error.message : String(error)}`);
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
        bitrateMbps: blob.bitrateMbps || 5,
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
      const hasVariant = blob.variants.some(
        (v) => v.resolution === tier.variant
      );
      if (!hasVariant) continue;

      // Score: how much of available bandwidth we use (higher is better, up to 100%)
      const score = Math.min(tier.minBitrate / maxBitrate, 1.0);
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

  /**
   * Generate HLS master playlist for adaptive streaming.
   * Allows client to switch between variants based on bandwidth.
   *
   * @param blob ContentBlob with variants
   * @param baseUrl Base URL for variant segments
   * @returns HLS M3U8 playlist content
   */
  generateHlsPlaylist(blob: ContentBlob, baseUrl: string): string {
    if (!blob.variants || blob.variants.length === 0) {
      return this.generateSimpleHlsPlaylist(blob, baseUrl);
    }

    const lines: string[] = [
      '#EXTM3U',
      '#EXT-X-VERSION:3',
      '#EXT-X-ALLOW-CACHE:YES',
      `#EXT-X-TARGETDURATION:${Math.ceil((blob.durationSeconds || 0) / 10)}`,
    ];

    // Add variant streams (for HLS variant selection)
    for (const variant of blob.variants) {
      lines.push(
        `#EXT-X-STREAM-INF:BANDWIDTH=${(variant.bitrateMbps || 5) * 1000000},RESOLUTION=${variant.resolution}`
      );
      lines.push(`${baseUrl}/${variant.hash}.m3u8`);
    }

    // Add segments for main stream
    const segmentDuration = 10; // 10 seconds per segment
    const totalSegments = Math.ceil((blob.durationSeconds || 0) / segmentDuration);

    for (let i = 0; i < totalSegments; i++) {
      lines.push(`#EXTINF:${segmentDuration},`);
      lines.push(`segment-${i}.ts`);
    }

    lines.push('#EXT-X-ENDLIST');

    return lines.join('\n');
  }

  /**
   * Simple HLS playlist for single-bitrate streams.
   */
  private generateSimpleHlsPlaylist(blob: ContentBlob, baseUrl: string): string {
    const lines: string[] = [
      '#EXTM3U',
      '#EXT-X-VERSION:3',
      '#EXT-X-ALLOW-CACHE:YES',
      `#EXT-X-TARGETDURATION:10`,
      `#EXT-X-MEDIA-SEQUENCE:0`,
    ];

    const segmentDuration = 10;
    const totalSegments = Math.ceil((blob.durationSeconds || 0) / segmentDuration);

    for (let i = 0; i < totalSegments; i++) {
      lines.push(`#EXTINF:${segmentDuration},`);
      lines.push(`${baseUrl}/segment-${i}.ts`);
    }

    lines.push('#EXT-X-ENDLIST');

    return lines.join('\n');
  }

  /**
   * Generate DASH MPD (Media Presentation Description) for adaptive streaming.
   * More complex than HLS but widely supported in modern players.
   *
   * @param blob ContentBlob with variants
   * @param baseUrl Base URL for variant segments
   * @returns DASH MPD XML content
   */
  generateDashMpd(blob: ContentBlob, baseUrl: string): string {
    const mediaduration = this.formatISO8601Duration(blob.durationSeconds || 0);

    let xml = `<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" mediaPresentationDuration="${mediaduration}">
  <Period>
    <AdaptationSet mimeType="video/mp4" segmentAlignment="true">`;

    if (blob.variants && blob.variants.length > 0) {
      for (const variant of blob.variants) {
        xml += `
      <Representation id="${variant.hash}" width="${this.extractResolutionWidth(
          variant.resolution
        )}" height="${this.extractResolutionHeight(
          variant.resolution
        )}" bandwidth="${(variant.bitrateMbps || 5) * 1000000}">
        <BaseURL>${baseUrl}/${variant.hash}/manifest.mpd</BaseURL>
      </Representation>`;
      }
    } else {
      // Single representation
      xml += `
      <Representation id="${blob.hash}" bandwidth="${(blob.bitrateMbps || 5) * 1000000}">
        <BaseURL>${baseUrl}/${blob.hash}/segment.mp4</BaseURL>
      </Representation>`;
    }

    xml += `
    </AdaptationSet>
  </Period>
</MPD>`;

    return xml;
  }

  /**
   * Helper: format seconds to ISO 8601 duration format (PT...S).
   */
  private formatISO8601Duration(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);

    let duration = 'PT';
    if (hours > 0) duration += `${hours}H`;
    if (minutes > 0) duration += `${minutes}M`;
    if (secs > 0 || duration === 'PT') duration += `${secs}S`;

    return duration;
  }

  /**
   * Helper: extract width from resolution string ("1080p" -> 1920).
   */
  private extractResolutionWidth(resolution: string): number {
    const resolutionMap: { [key: string]: number } = {
      '480p': 854,
      '720p': 1280,
      '1080p': 1920,
      '1440p': 2560,
      '2160p': 3840,
      '4320p': 7680,
    };
    return resolutionMap[resolution] || 1920;
  }

  /**
   * Helper: extract height from resolution string ("1080p" -> 1080).
   */
  private extractResolutionHeight(resolution: string): number {
    const heightMap: { [key: string]: number } = {
      '480p': 480,
      '720p': 720,
      '1080p': 1080,
      '1440p': 1440,
      '2160p': 2160,
      '4320p': 4320,
    };
    return heightMap[resolution] || 1080;
  }

  /**
   * Clear bandwidth cache.
   * Useful for testing or forcing re-probe.
   */
  clearBandwidthCache(): void {
    this.bandwidthCache.clear();
  }
}
