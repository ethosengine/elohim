/**
 * Blob Cache Tiers Service - O(1) LRU Multi-Tier Cache
 *
 * Three-tier caching strategy optimized for Elohim Protocol content:
 * - Tier 1 (Metadata): Unlimited, DHT-verified content metadata
 * - Tier 2 (Blobs): 1 GB limit, O(1) LRU eviction, 24h TTL
 * - Tier 3 (Chunks): 10 GB limit, time-based cleanup, 7-day TTL
 *
 * Performance: O(1) for all operations using Map insertion-order LRU.
 * Large blobs won't evict many small documents (tier isolation).
 *
 * NEW: Integrates with WasmCacheService for reach-aware caching.
 * Content at different reach levels (private -> commons) never evict each other.
 */

import { Injectable, Injector } from '@angular/core';

import { ContentBlob } from '../models/content-node.model';

import { WasmCacheService, ReachLevel } from './wasm-cache.service';

// ============================================================================
// Types
// ============================================================================

/** Cache tier configuration */
export interface CacheTierConfig {
  name: string;
  maxSizeBytes: number;
  ttlSeconds: number;
  evictionPolicy: 'lru' | 'time-based';
}

/** Cached item with metadata */
export interface CachedItem<T> {
  hash: string;
  data: T;
  sizeBytes: number;
  createdAt: number;
}

/** Cache statistics snapshot */
export interface CacheTierStats {
  name: string;
  itemCount: number;
  totalSizeBytes: number;
  maxSizeBytes: number;
  percentFull: number;
  evictionCount: number;
  hitCount: number;
  missCount: number;
  hitRate: number;
}

/** Cache operation result */
export interface CacheOperationResult {
  success: boolean;
  itemSize?: number;
  evictedItems?: number;
  reason?: string;
}

/** Integrity check result */
export interface CacheIntegrityCheckResult {
  isValid: boolean;
  itemsChecked: number;
  corruptedItems: string[];
  missingMetadata: string[];
  durationMs: number;
  checkedAt: number;
}

// ============================================================================
// O(1) LRU Cache Implementation
// ============================================================================

/**
 * Generic O(1) LRU cache using Map insertion order.
 *
 * JavaScript Map maintains insertion order, so:
 * - First item = Least Recently Used (evict this)
 * - Last item = Most Recently Used
 * - On access: delete + re-insert moves item to end
 *
 * All operations are O(1).
 */
class LRUCache<T> {
  private readonly cache = new Map<string, CachedItem<T>>();
  private currentSize = 0;
  private stats = { hits: 0, misses: 0, evictions: 0 };

  constructor(
    private readonly maxSizeBytes: number,
    private readonly ttlSeconds: number
  ) {}

  /** Add or update item. Returns eviction count. O(1). */
  set(hash: string, data: T, sizeBytes: number): number {
    // Remove existing to update position
    if (this.cache.has(hash)) {
      this.delete(hash);
    }

    // Evict LRU items until we have space
    const evicted = this.evictUntilFits(sizeBytes);

    // Insert at end (most recently used position)
    this.cache.set(hash, {
      hash,
      data,
      sizeBytes,
      createdAt: Date.now(),
    });
    this.currentSize += sizeBytes;

    return evicted;
  }

  /** Get item, moving it to MRU position. O(1). */
  get(hash: string): T | null {
    const item = this.cache.get(hash);
    if (!item) {
      this.stats.misses++;
      return null;
    }

    // Check TTL
    const ageSeconds = (Date.now() - item.createdAt) / 1000;
    if (ageSeconds > this.ttlSeconds) {
      this.delete(hash);
      this.stats.misses++;
      return null;
    }

    // Move to end (MRU) by delete + re-insert
    this.cache.delete(hash);
    this.cache.set(hash, item);

    this.stats.hits++;
    return item.data;
  }

  /** Check existence without updating LRU position. O(1). */
  has(hash: string): boolean {
    return this.cache.has(hash);
  }

  /** Delete item. O(1). */
  delete(hash: string): boolean {
    const item = this.cache.get(hash);
    if (item) {
      this.cache.delete(hash);
      this.currentSize -= item.sizeBytes;
      return true;
    }
    return false;
  }

  /** Evict LRU items until space available. O(k) where k = evictions. */
  private evictUntilFits(requiredBytes: number): number {
    let evicted = 0;

    while (this.currentSize + requiredBytes > this.maxSizeBytes && this.cache.size > 0) {
      // Map.keys().next() returns first (LRU) item in O(1)
      const lruHash = this.cache.keys().next().value;
      if (lruHash) {
        const item = this.cache.get(lruHash)!;
        this.cache.delete(lruHash);
        this.currentSize -= item.sizeBytes;
        this.stats.evictions++;
        evicted++;
      } else {
        break;
      }
    }

    return evicted;
  }

  /** Remove expired items. O(n) but only runs periodically. */
  cleanupExpired(): number {
    const now = Date.now();
    const ttlMs = this.ttlSeconds * 1000;
    let cleaned = 0;

    for (const [hash, item] of this.cache.entries()) {
      if (now - item.createdAt > ttlMs) {
        this.cache.delete(hash);
        this.currentSize -= item.sizeBytes;
        this.stats.evictions++;
        cleaned++;
      }
    }

    return cleaned;
  }

  /** Clear all items */
  clear(): void {
    this.cache.clear();
    this.currentSize = 0;
    this.stats = { hits: 0, misses: 0, evictions: 0 };
  }

  /** Get current size in bytes */
  get size(): number {
    return this.currentSize;
  }

  /** Get item count */
  get count(): number {
    return this.cache.size;
  }

  /** Get statistics */
  getStats(name: string): CacheTierStats {
    const total = this.stats.hits + this.stats.misses;
    return {
      name,
      itemCount: this.cache.size,
      totalSizeBytes: this.currentSize,
      maxSizeBytes: this.maxSizeBytes,
      percentFull:
        this.maxSizeBytes === Infinity ? 0 : (this.currentSize / this.maxSizeBytes) * 100,
      evictionCount: this.stats.evictions,
      hitCount: this.stats.hits,
      missCount: this.stats.misses,
      hitRate: total > 0 ? this.stats.hits / total : 0,
    };
  }

  /** Iterate over entries (for integrity checks) */
  entries(): IterableIterator<[string, CachedItem<T>]> {
    return this.cache.entries();
  }
}

// ============================================================================
// BlobCacheTiersService
// ============================================================================

@Injectable({
  providedIn: 'root',
})
export class BlobCacheTiersService {
  // Tier 1: Metadata (unlimited, no eviction)
  private readonly metadataCache = new Map<string, CachedItem<ContentBlob>>();
  private metadataStats = { hits: 0, misses: 0, evictions: 0 };

  // Tier 2: Blobs (1 GB, LRU, 24h TTL)
  private readonly blobCache = new LRUCache<Blob>(
    1024 * 1024 * 1024, // 1 GB
    24 * 60 * 60 // 24 hours
  );

  // Tier 3: Chunks (10 GB, LRU, 7-day TTL)
  private readonly chunkCache = new LRUCache<Uint8Array>(
    10 * 1024 * 1024 * 1024, // 10 GB
    7 * 24 * 60 * 60 // 7 days
  );

  private lastIntegrityCheck: CacheIntegrityCheckResult | null = null;
  private integrityCheckIntervalId: ReturnType<typeof setInterval> | null = null;
  private cleanupIntervalId: ReturnType<typeof setInterval> | null = null;

  // NEW: WASM-backed reach-aware cache for high-performance operations
  private wasmCacheInitialized = false;

  constructor(
    private readonly injector: Injector,
    private readonly wasmCache: WasmCacheService
  ) {
    this.startCleanupTimer();
    this.startIntegrityVerification();
    this.initializeWasmCache();
  }

  /**
   * Initialize WASM cache (async, non-blocking).
   * Operations fall back to TypeScript if WASM isn't ready.
   */
  private async initializeWasmCache(): Promise<void> {
    try {
      const result = await this.wasmCache.initialize({
        maxSizePerReach: BigInt(128 * 1024 * 1024), // 128MB per reach level
        preferWasm: true,
      });
      this.wasmCacheInitialized = true;
      console.log(`[BlobCacheTiersService] WASM cache initialized (${result.implementation})`);
    } catch (error) {
      console.warn('[BlobCacheTiersService] WASM cache init failed, using fallback:', error);
    }
  }

  /** Check if WASM cache is ready */
  get isWasmReady(): boolean {
    return this.wasmCacheInitialized && this.wasmCache.isReady;
  }

  /** Get current implementation type */
  get cacheImplementation(): 'wasm' | 'typescript' | 'initializing' {
    if (!this.wasmCacheInitialized) return 'initializing';
    return this.wasmCache.implementationType;
  }

  // ==========================================================================
  // Tier 1: Metadata (unlimited, no eviction)
  // ==========================================================================

  /** Cache metadata. Always succeeds. */
  setMetadata(hash: string, metadata: ContentBlob): CacheOperationResult {
    const sizeBytes = JSON.stringify(metadata).length;

    this.metadataCache.set(hash, {
      hash,
      data: metadata,
      sizeBytes,
      createdAt: Date.now(),
    });

    return { success: true, itemSize: sizeBytes };
  }

  /** Get metadata. O(1). */
  getMetadata(hash: string): ContentBlob | null {
    const item = this.metadataCache.get(hash);
    if (item) {
      this.metadataStats.hits++;
      return item.data;
    }
    this.metadataStats.misses++;
    return null;
  }

  // ==========================================================================
  // Tier 2: Blobs (1 GB, O(1) LRU)
  // ==========================================================================

  /** Cache blob with O(1) LRU eviction. */
  setBlob(hash: string, blob: Blob): CacheOperationResult {
    const sizeBytes = blob.size;

    if (sizeBytes > 1024 * 1024 * 1024) {
      return { success: false, reason: 'Blob exceeds 1 GB limit' };
    }

    const evicted = this.blobCache.set(hash, blob, sizeBytes);
    return { success: true, itemSize: sizeBytes, evictedItems: evicted };
  }

  /** Get blob, updating LRU position. O(1). */
  getBlob(hash: string): Blob | null {
    return this.blobCache.get(hash);
  }

  // ==========================================================================
  // Tier 3: Chunks (10 GB, O(1) LRU)
  // ==========================================================================

  /** Cache chunk with O(1) LRU eviction. */
  setChunk(hash: string, chunk: Uint8Array): CacheOperationResult {
    const sizeBytes = chunk.byteLength;

    if (sizeBytes > 10 * 1024 * 1024 * 1024) {
      return { success: false, reason: 'Chunk exceeds 10 GB limit' };
    }

    const evicted = this.chunkCache.set(hash, chunk, sizeBytes);
    return { success: true, itemSize: sizeBytes, evictedItems: evicted };
  }

  /** Get chunk, updating LRU position. O(1). */
  getChunk(hash: string): Uint8Array | null {
    return this.chunkCache.get(hash);
  }

  // ==========================================================================
  // Common Operations
  // ==========================================================================

  /** Check if item exists in any/specific tier. O(1). */
  has(hash: string, tier?: 'metadata' | 'blob' | 'chunk'): boolean {
    if (!tier || tier === 'metadata') {
      if (this.metadataCache.has(hash)) return true;
    }
    if (!tier || tier === 'blob') {
      if (this.blobCache.has(hash)) return true;
    }
    if (!tier || tier === 'chunk') {
      if (this.chunkCache.has(hash)) return true;
    }
    return false;
  }

  /** Remove item from tier(s). O(1). */
  remove(hash: string, tier?: 'metadata' | 'blob' | 'chunk'): boolean {
    let removed = false;

    if (!tier || tier === 'metadata') {
      if (this.metadataCache.delete(hash)) removed = true;
    }
    if (!tier || tier === 'blob') {
      if (this.blobCache.delete(hash)) removed = true;
    }
    if (!tier || tier === 'chunk') {
      if (this.chunkCache.delete(hash)) removed = true;
    }

    return removed;
  }

  /** Clear all or specific tier. */
  clear(tier?: 'metadata' | 'blob' | 'chunk'): void {
    if (!tier || tier === 'metadata') {
      this.metadataCache.clear();
      this.metadataStats = { hits: 0, misses: 0, evictions: 0 };
    }
    if (!tier || tier === 'blob') {
      this.blobCache.clear();
    }
    if (!tier || tier === 'chunk') {
      this.chunkCache.clear();
    }
  }

  // ==========================================================================
  // Statistics
  // ==========================================================================

  /** Get tier statistics. */
  getStats(tier: 'metadata' | 'blob' | 'chunk'): CacheTierStats {
    switch (tier) {
      case 'metadata': {
        const total = this.metadataStats.hits + this.metadataStats.misses;
        return {
          name: 'Metadata',
          itemCount: this.metadataCache.size,
          totalSizeBytes: 0, // Not tracked for metadata
          maxSizeBytes: Infinity,
          percentFull: 0,
          evictionCount: this.metadataStats.evictions,
          hitCount: this.metadataStats.hits,
          missCount: this.metadataStats.misses,
          hitRate: total > 0 ? this.metadataStats.hits / total : 0,
        };
      }
      case 'blob':
        return this.blobCache.getStats('Blob');
      case 'chunk':
        return this.chunkCache.getStats('Chunk');
    }
  }

  /** Get all tier statistics. */
  getAllStats(): Record<string, CacheTierStats> {
    return {
      metadata: this.getStats('metadata'),
      blob: this.getStats('blob'),
      chunk: this.getStats('chunk'),
    };
  }

  /** Get total memory usage. */
  getTotalMemoryUsageBytes(): number {
    return this.blobCache.size + this.chunkCache.size;
  }

  /** Get memory report. */
  getMemoryReport(): {
    blobCacheBytes: number;
    chunkCacheBytes: number;
    totalBytes: number;
    percentOfBlobMax: number;
    percentOfChunkMax: number;
  } {
    const blobMax = 1024 * 1024 * 1024;
    const chunkMax = 10 * 1024 * 1024 * 1024;
    return {
      blobCacheBytes: this.blobCache.size,
      chunkCacheBytes: this.chunkCache.size,
      totalBytes: this.getTotalMemoryUsageBytes(),
      percentOfBlobMax: (this.blobCache.size / blobMax) * 100,
      percentOfChunkMax: (this.chunkCache.size / chunkMax) * 100,
    };
  }

  // ==========================================================================
  // Background Cleanup
  // ==========================================================================

  private startCleanupTimer(): void {
    // Run every 5 minutes
    this.cleanupIntervalId = setInterval(
      () => {
        this.blobCache.cleanupExpired();
        this.chunkCache.cleanupExpired();
      },
      5 * 60 * 1000
    );
  }

  // ==========================================================================
  // Integrity Verification
  // ==========================================================================

  getLastIntegrityCheck(): CacheIntegrityCheckResult | null {
    return this.lastIntegrityCheck;
  }

  async verifyBlobIntegrity(hash: string): Promise<boolean> {
    const blob = this.blobCache.get(hash);
    if (!blob) return false;

    try {
      const { BlobVerificationService } = await import('./blob-verification.service');
      const verificationService = this.injector.get(BlobVerificationService);
      const result = await verificationService.verifyBlob(blob, hash).toPromise();
      return result?.isValid ?? false;
    } catch {
      return false;
    }
  }

  async verifyAllBlobIntegrity(): Promise<CacheIntegrityCheckResult> {
    const startTime = performance.now();
    const corruptedItems: string[] = [];
    let itemsChecked = 0;

    try {
      const { BlobVerificationService } = await import('./blob-verification.service');
      const verificationService = this.injector.get(BlobVerificationService);

      for (const [hash, item] of this.blobCache.entries()) {
        itemsChecked++;
        try {
          const result = await verificationService.verifyBlob(item.data, hash).toPromise();
          if (!result?.isValid) {
            corruptedItems.push(hash);
          }
        } catch {
          corruptedItems.push(hash);
        }
      }

      // Remove corrupted items
      for (const hash of corruptedItems) {
        this.blobCache.delete(hash);
      }

      this.lastIntegrityCheck = {
        isValid: corruptedItems.length === 0,
        itemsChecked,
        corruptedItems,
        missingMetadata: [],
        durationMs: Math.round(performance.now() - startTime),
        checkedAt: Date.now(),
      };

      return this.lastIntegrityCheck;
    } catch (error) {
      this.lastIntegrityCheck = {
        isValid: false,
        itemsChecked,
        corruptedItems,
        missingMetadata: [],
        durationMs: Math.round(performance.now() - startTime),
        checkedAt: Date.now(),
      };
      return this.lastIntegrityCheck;
    }
  }

  startIntegrityVerification(): void {
    // Every hour
    this.integrityCheckIntervalId = setInterval(
      () => {
        this.verifyAllBlobIntegrity().catch(() => {});
      },
      60 * 60 * 1000
    );

    // Initial check after 5 minutes
    setTimeout(
      () => {
        this.verifyAllBlobIntegrity().catch(() => {});
      },
      5 * 60 * 1000
    );
  }

  stopIntegrityVerification(): void {
    if (this.integrityCheckIntervalId) {
      clearInterval(this.integrityCheckIntervalId);
      this.integrityCheckIntervalId = null;
    }
  }

  // ==========================================================================
  // Reach-Aware Cache Operations (NEW)
  // ==========================================================================

  /**
   * Add metadata with reach-level awareness.
   * Content at different reach levels never evict each other.
   *
   * @param hash - Content hash
   * @param metadata - ContentBlob metadata
   * @param reachLevel - 0=Private, 7=Commons (default)
   * @param domain - Content domain (e.g., 'elohim-protocol')
   * @param epic - Content epic (e.g., 'governance')
   */
  setMetadataWithReach(
    hash: string,
    metadata: ContentBlob,
    reachLevel: number = ReachLevel.COMMONS,
    domain = '',
    epic = ''
  ): CacheOperationResult {
    // Always store in legacy metadata cache for backwards compatibility
    const legacyResult = this.setMetadata(hash, metadata);

    // Also store in WASM reach-aware cache if available
    if (this.isWasmReady) {
      const sizeBytes = JSON.stringify(metadata).length;
      this.wasmCache.put(hash, sizeBytes, reachLevel, domain, epic);
    }

    return legacyResult;
  }

  /**
   * Check if content exists at specified reach level.
   */
  hasAtReach(hash: string, reachLevel: number): boolean {
    if (this.isWasmReady) {
      return this.wasmCache.has(hash, reachLevel);
    }
    // Fallback to legacy check (ignores reach)
    return this.has(hash);
  }

  /**
   * Touch content at reach level (update LRU position).
   */
  touchAtReach(hash: string, reachLevel: number): boolean {
    if (this.isWasmReady) {
      return this.wasmCache.touch(hash, reachLevel);
    }
    return false;
  }

  /**
   * Delete content from specific reach level.
   */
  deleteAtReach(hash: string, reachLevel: number): boolean {
    // Remove from legacy cache
    this.remove(hash);

    // Remove from WASM cache
    if (this.isWasmReady) {
      return this.wasmCache.delete(hash, reachLevel);
    }
    return true;
  }

  /**
   * Get statistics for a specific reach level.
   */
  getReachStats(reachLevel: number): CacheTierStats | null {
    if (!this.isWasmReady) return null;

    const stats = this.wasmCache.statsForReach(reachLevel);
    return {
      name: `Reach-${reachLevel}`,
      itemCount: stats.itemCount,
      totalSizeBytes: Number(stats.totalSizeBytes),
      maxSizeBytes: 128 * 1024 * 1024, // 128MB per reach
      percentFull: Number((stats.totalSizeBytes * 100n) / BigInt(128 * 1024 * 1024)),
      evictionCount: Number(stats.evictionCount),
      hitCount: Number(stats.hitCount),
      missCount: Number(stats.missCount),
      hitRate: stats.hitRate() / 100,
    };
  }

  /**
   * Get all reach-level statistics.
   */
  getAllReachStats(): Record<number, CacheTierStats> | null {
    if (!this.isWasmReady) return null;

    const result: Record<number, CacheTierStats> = {};
    for (let i = 0; i <= 7; i++) {
      const stats = this.getReachStats(i);
      if (stats) result[i] = stats;
    }
    return result;
  }

  /**
   * Get combined stats including reach-aware cache.
   */
  getExtendedStats(): {
    legacy: Record<string, CacheTierStats>;
    reachAware: Record<number, CacheTierStats> | null;
    implementation: 'wasm' | 'typescript' | 'initializing';
  } {
    return {
      legacy: this.getAllStats(),
      reachAware: this.getAllReachStats(),
      implementation: this.cacheImplementation,
    };
  }

  /**
   * Calculate priority using Elohim Protocol factors.
   */
  calculateContentPriority(params: {
    reachLevel: number;
    proximityScore?: number;
    bandwidthClass?: number;
    stewardTier?: number;
    affinityMatch?: number;
    agePenalty?: number;
  }): number {
    if (this.isWasmReady) {
      return this.wasmCache.calculatePriority({
        reachLevel: params.reachLevel,
        proximityScore: params.proximityScore ?? 0,
        bandwidthClass: params.bandwidthClass ?? 2,
        stewardTier: params.stewardTier ?? 1,
        affinityMatch: params.affinityMatch ?? 0.5,
        agePenalty: params.agePenalty ?? 0,
      });
    }
    // Simple fallback: just use reach level
    return params.reachLevel * 12;
  }
}
