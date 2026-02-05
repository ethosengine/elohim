/**
 * WASM Cache Service - Angular Wrapper
 *
 * Provides Angular dependency injection wrapper around the framework-agnostic
 * reach-aware cache. Automatically initializes WASM or falls back to TypeScript.
 *
 * Usage:
 * ```typescript
 * @Component({...})
 * export class MyComponent {
 *   constructor(private cacheService: WasmCacheService) {}
 *
 *   async ngOnInit() {
 *     await this.cacheService.initialize();
 *
 *     // Use reach-aware caching
 *     this.cacheService.put('hash', 1024, ReachLevel.COMMONS, 'elohim', 'governance');
 *   }
 * }
 * ```
 */

import { Injectable, OnDestroy } from '@angular/core';

// @coverage: 33.7% (2026-02-05)

import { BehaviorSubject } from 'rxjs';

// Import from framework-agnostic cache module
import {
  createReachAwareCache,
  createBlobCache,
  createChunkCache,
  calculatePriority,
  calculateFreshness,
  isWasmAvailable,
  TsReachAwareCache,
  TsBlobCache,
  TsChunkCache,
} from '@elohim/service/cache/reach-aware-cache';
import { ReachLevel, MasteryLevel } from '@elohim/service/cache/types';

import type {
  IReachAwareCache,
  IBlobCache,
  IChunkCache,
  ICacheStats,
  CacheEntryMetadata,
  PriorityParams,
  CacheConfig,
} from '@elohim/service/cache/types';

// Re-export constants for convenience
export { ReachLevel, MasteryLevel };

/** Service state */
export type CacheServiceState = 'uninitialized' | 'initializing' | 'ready' | 'error';

/** Initialization result */
export interface CacheInitializationResult {
  success: boolean;
  implementation: 'wasm' | 'typescript';
  error?: string;
}

/**
 * Angular service providing reach-aware content caching.
 *
 * Features:
 * - Automatic WASM/TypeScript selection
 * - Reach-level isolation (private content never evicts commons)
 * - Priority-based eviction with Elohim Protocol scoring
 * - Observable state for reactive UIs
 */
@Injectable({
  providedIn: 'root',
})
export class WasmCacheService implements OnDestroy {
  private reachCache: IReachAwareCache | null = null;
  private blobCache: IBlobCache | null = null;
  private chunkCache: IChunkCache | null = null;

  private readonly stateSubject = new BehaviorSubject<CacheServiceState>('uninitialized');
  private implementation: 'wasm' | 'typescript' = 'typescript';
  private initPromise: Promise<CacheInitializationResult> | null = null;

  /** Observable cache service state */
  readonly state$ = this.stateSubject.asObservable();

  /** Current state */
  get state(): CacheServiceState {
    return this.stateSubject.value;
  }

  /** Current implementation type */
  get implementationType(): 'wasm' | 'typescript' {
    return this.implementation;
  }

  /** Check if service is ready */
  get isReady(): boolean {
    return this.state === 'ready';
  }

  /**
   * Initialize the cache service.
   * Call this before using any cache operations.
   *
   * @param config - Optional configuration
   * @returns Initialization result
   */
  async initialize(config?: CacheConfig): Promise<CacheInitializationResult> {
    // Return existing promise if already initializing
    if (this.initPromise) {
      return this.initPromise;
    }

    // Already initialized
    if (this.state === 'ready') {
      return {
        success: true,
        implementation: this.implementation,
      };
    }

    this.stateSubject.next('initializing');

    this.initPromise = this.doInitialize(config);
    return this.initPromise;
  }

  private async doInitialize(config?: CacheConfig): Promise<CacheInitializationResult> {
    try {
      // Default configuration
      const defaultConfig: CacheConfig = {
        maxSizePerReach: BigInt(128 * 1024 * 1024), // 128MB per reach level (1GB total)
        maxSizeBytes: BigInt(1024 * 1024 * 1024), // 1GB for blob cache
        ttlMillis: BigInt(7 * 24 * 60 * 60 * 1000), // 7 days for chunk cache
        preferWasm: true,
        ...config,
      };

      // Try to create reach-aware cache (prefers WASM)
      const reachResult = await createReachAwareCache(defaultConfig);
      this.reachCache = reachResult.cache;
      this.implementation = reachResult.implementation;

      // Create supporting caches
      const blobResult = await createBlobCache(defaultConfig);
      this.blobCache = blobResult.cache;

      const chunkResult = await createChunkCache(defaultConfig);
      this.chunkCache = chunkResult.cache;

      this.stateSubject.next('ready');

      return {
        success: true,
        implementation: this.implementation,
      };
    } catch (error) {
      this.stateSubject.next('error');
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';

      // Fallback to TypeScript implementation
      try {
        this.reachCache = new TsReachAwareCache(BigInt(128 * 1024 * 1024));
        this.blobCache = new TsBlobCache(BigInt(1024 * 1024 * 1024));
        this.chunkCache = new TsChunkCache(
          BigInt(10 * 1024 * 1024 * 1024),
          BigInt(7 * 24 * 60 * 60 * 1000)
        );
        this.implementation = 'typescript';
        this.stateSubject.next('ready');

        return {
          success: true,
          implementation: 'typescript',
          error: `WASM failed, using TypeScript fallback: ${errorMessage}`,
        };
      } catch {
        return {
          success: false,
          implementation: 'typescript',
          error: `All initialization attempts failed: ${errorMessage}`,
        };
      }
    }
  }

  // ==========================================================================
  // Reach-Aware Cache Operations
  // ==========================================================================

  /**
   * Add content to the reach-aware cache.
   *
   * @param hash - Content hash
   * @param sizeBytes - Content size in bytes
   * @param reachLevel - Reach level (0-7)
   * @param domain - Content domain
   * @param epic - Content epic
   * @param priority - Optional priority (calculated if not provided)
   * @returns Number of items evicted
   */
  put(
    hash: string,
    sizeBytes: number,
    reachLevel: number,
    domain: string,
    epic: string,
    priority?: number
  ): number {
    this.ensureReady();

    const calculatedPriority =
      priority ??
      calculatePriority({
        reachLevel,
        proximityScore: 0,
        bandwidthClass: 2,
        stewardTier: 1,
        affinityMatch: 0.5,
        agePenalty: 0,
      });

    return this.reachCache!.put(
      hash,
      BigInt(sizeBytes),
      reachLevel,
      domain,
      epic,
      calculatedPriority
    );
  }

  /**
   * Check if content exists at reach level.
   */
  has(hash: string, reachLevel: number): boolean {
    this.ensureReady();
    return this.reachCache!.has(hash, reachLevel);
  }

  /**
   * Touch content (update access time).
   */
  touch(hash: string, reachLevel: number): boolean {
    this.ensureReady();
    return this.reachCache!.touch(hash, reachLevel);
  }

  /**
   * Delete content from reach level.
   */
  delete(hash: string, reachLevel: number): boolean {
    this.ensureReady();
    return this.reachCache!.delete(hash, reachLevel);
  }

  /**
   * Get statistics for a reach level.
   */
  statsForReach(reachLevel: number): ICacheStats {
    this.ensureReady();
    return this.reachCache!.statsForReach(reachLevel);
  }

  /**
   * Get total count across all reach levels.
   */
  totalCount(): number {
    this.ensureReady();
    return this.reachCache!.totalCount();
  }

  /**
   * Get total size across all reach levels.
   */
  totalSize(): bigint {
    this.ensureReady();
    return this.reachCache!.totalSize();
  }

  // ==========================================================================
  // Blob Cache Operations
  // ==========================================================================

  /**
   * Add blob to cache.
   */
  putBlob(
    hash: string,
    sizeBytes: number,
    reachLevel: number,
    domain: string,
    epic: string,
    priority?: number
  ): number {
    this.ensureReady();
    const p =
      priority ??
      calculatePriority({
        reachLevel,
        proximityScore: 0,
        bandwidthClass: 2,
        stewardTier: 1,
        affinityMatch: 0.5,
        agePenalty: 0,
      });
    return this.blobCache!.put(hash, BigInt(sizeBytes), reachLevel, domain, epic, p);
  }

  /**
   * Get blob metadata.
   */
  getBlobMetadata(hash: string): CacheEntryMetadata | null {
    this.ensureReady();
    return this.blobCache!.getMetadata(hash);
  }

  /**
   * Get blob cache stats.
   */
  blobStats(): ICacheStats {
    this.ensureReady();
    return this.blobCache!.stats();
  }

  // ==========================================================================
  // Chunk Cache Operations
  // ==========================================================================

  /**
   * Add chunk to cache.
   */
  putChunk(hash: string, sizeBytes: number): number {
    this.ensureReady();
    return this.chunkCache!.put(hash, BigInt(sizeBytes));
  }

  /**
   * Check if chunk exists.
   */
  hasChunk(hash: string): boolean {
    this.ensureReady();
    return this.chunkCache!.has(hash);
  }

  /**
   * Cleanup expired chunks.
   */
  cleanupChunks(): number {
    this.ensureReady();
    return this.chunkCache!.cleanup();
  }

  /**
   * Get chunk cache stats.
   */
  chunkStats(): ICacheStats {
    this.ensureReady();
    return this.chunkCache!.stats();
  }

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  /**
   * Calculate priority score using Elohim Protocol factors.
   */
  calculatePriority(params: PriorityParams): number {
    return calculatePriority(params);
  }

  /**
   * Calculate mastery freshness.
   */
  calculateFreshness(masteryLevel: number, ageSeconds: number): number {
    return calculateFreshness(masteryLevel, ageSeconds);
  }

  /**
   * Check if WASM is available.
   */
  async checkWasmAvailable(): Promise<boolean> {
    return isWasmAvailable();
  }

  /**
   * Get comprehensive statistics.
   */
  getAllStats(): {
    reach: Record<number, ICacheStats>;
    blob: ICacheStats;
    chunk: ICacheStats;
    implementation: 'wasm' | 'typescript';
  } {
    this.ensureReady();

    const reachStats: Record<number, ICacheStats> = {};
    for (let i = 0; i <= 7; i++) {
      reachStats[i] = this.reachCache!.statsForReach(i);
    }

    return {
      reach: reachStats,
      blob: this.blobCache!.stats(),
      chunk: this.chunkCache!.stats(),
      implementation: this.implementation,
    };
  }

  /**
   * Clear all caches.
   */
  clearAll(): void {
    this.reachCache?.clear();
    this.blobCache?.clear();
    this.chunkCache?.clear();
  }

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  private ensureReady(): void {
    if (!this.isReady) {
      throw new Error('[WasmCacheService] Service not initialized. Call initialize() first.');
    }
  }

  ngOnDestroy(): void {
    this.reachCache?.dispose();
    this.blobCache?.dispose();
    this.chunkCache?.dispose();
    this.stateSubject.complete();
  }
}
