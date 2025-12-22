/**
 * Blob Cache Tiers Service - Phase 2: Multi-Tier Cache Management
 *
 * Implements three-tier caching strategy for optimal performance:
 * - Tier 1 (Metadata): Unlimited size, unlimited TTL, DHT-verified
 * - Tier 2 (Blobs): Limited size (1 GB), LRU eviction, short TTL
 * - Tier 3 (Chunks): Limited size (10 GB), time-based cleanup, very short TTL
 *
 * This avoids cache thrashing where one large video evicts 1000 small documents.
 */

import { Injectable } from '@angular/core';
import { ContentBlob } from '../models/content-node.model';

/**
 * Cache tier configuration
 */
export interface CacheTierConfig {
  name: string;
  maxSizeBytes: number;
  ttlSeconds: number;
  evictionPolicy: 'lru' | 'lfu' | 'time-based';
}

/**
 * Cached item metadata
 */
export interface CachedItem<T> {
  hash: string;
  data: T;
  sizeBytes: number;
  createdAt: number;
  lastAccessedAt: number;
  accessCount: number;
}

/**
 * Cache statistics
 */
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

/**
 * Cache operation result
 */
export interface CacheOperationResult {
  success: boolean;
  itemSize?: number;
  evictedItems?: number;
  reason?: string;
}

@Injectable({
  providedIn: 'root',
})
export class BlobCacheTiersService {
  /** Tier 1: Metadata cache (unlimited) */
  private metadataCache = new Map<string, CachedItem<ContentBlob>>();
  private metadataStats = { hits: 0, misses: 0, evictions: 0 };

  /** Tier 2: Blob cache (1 GB limit) */
  private blobCache = new Map<string, CachedItem<Blob>>();
  private blobCacheSize = 0;
  private blobStats = { hits: 0, misses: 0, evictions: 0 };

  /** Tier 3: Chunk cache (10 GB limit) */
  private chunkCache = new Map<string, CachedItem<Uint8Array>>();
  private chunkCacheSize = 0;
  private chunkStats = { hits: 0, misses: 0, evictions: 0 };

  /** Configuration for each tier */
  private readonly tiers: { [key: string]: CacheTierConfig } = {
    metadata: {
      name: 'Metadata',
      maxSizeBytes: Infinity, // Unlimited
      ttlSeconds: Infinity, // Unlimited
      evictionPolicy: 'lru',
    },
    blob: {
      name: 'Blob',
      maxSizeBytes: 1024 * 1024 * 1024, // 1 GB
      ttlSeconds: 24 * 60 * 60, // 24 hours
      evictionPolicy: 'lru',
    },
    chunk: {
      name: 'Chunk',
      maxSizeBytes: 10 * 1024 * 1024 * 1024, // 10 GB
      ttlSeconds: 7 * 24 * 60 * 60, // 7 days
      evictionPolicy: 'time-based',
    },
  };

  constructor() {
    // Start background cleanup
    this.startCleanupTimer();
  }

  /**
   * Tier 1: Set metadata cache (unlimited).
   * Metadata should always be available since it's validated by DHT.
   */
  setMetadata(hash: string, metadata: ContentBlob): CacheOperationResult {
    const sizeBytes = JSON.stringify(metadata).length;

    this.metadataCache.set(hash, {
      hash,
      data: metadata,
      sizeBytes,
      createdAt: Date.now(),
      lastAccessedAt: Date.now(),
      accessCount: 0,
    });

    return { success: true, itemSize: sizeBytes };
  }

  /**
   * Get metadata from Tier 1 (always fast, unlimited capacity).
   */
  getMetadata(hash: string): ContentBlob | null {
    const item = this.metadataCache.get(hash);
    if (item) {
      item.lastAccessedAt = Date.now();
      item.accessCount++;
      this.metadataStats.hits++;
      return item.data;
    }
    this.metadataStats.misses++;
    return null;
  }

  /**
   * Tier 2: Set blob cache (1 GB limit, LRU eviction).
   * Full blob files go here; evicts oldest on overflow.
   */
  setBlob(hash: string, blob: Blob): CacheOperationResult {
    const sizeBytes = blob.size;

    // Don't cache oversized blobs
    if (sizeBytes > this.tiers.blob.maxSizeBytes) {
      return {
        success: false,
        reason: `Blob too large (${sizeBytes} > ${this.tiers.blob.maxSizeBytes})`,
      };
    }

    // Evict items if necessary
    const evictions = this.evictFromBlobCache(sizeBytes);

    // Add to cache
    this.blobCache.set(hash, {
      hash,
      data: blob,
      sizeBytes,
      createdAt: Date.now(),
      lastAccessedAt: Date.now(),
      accessCount: 0,
    });

    this.blobCacheSize += sizeBytes;

    return { success: true, itemSize: sizeBytes, evictedItems: evictions };
  }

  /**
   * Get blob from Tier 2 cache.
   */
  getBlob(hash: string): Blob | null {
    const item = this.blobCache.get(hash);
    if (item) {
      // Check TTL
      const age = (Date.now() - item.createdAt) / 1000;
      if (age > this.tiers.blob.ttlSeconds) {
        // Expired
        this.blobCache.delete(hash);
        this.blobCacheSize -= item.sizeBytes;
        this.blobStats.misses++;
        return null;
      }

      item.lastAccessedAt = Date.now();
      item.accessCount++;
      this.blobStats.hits++;
      return item.data;
    }
    this.blobStats.misses++;
    return null;
  }

  /**
   * Tier 3: Set chunk cache (10 GB limit, time-based cleanup).
   * Individual chunks from downloads go here.
   */
  setChunk(hash: string, chunk: Uint8Array): CacheOperationResult {
    const sizeBytes = chunk.byteLength;

    // Don't cache oversized chunks
    if (sizeBytes > this.tiers.chunk.maxSizeBytes) {
      return {
        success: false,
        reason: `Chunk too large (${sizeBytes} > ${this.tiers.chunk.maxSizeBytes})`,
      };
    }

    // Evict items if necessary (time-based)
    const evictions = this.evictFromChunkCache(sizeBytes);

    // Add to cache
    this.chunkCache.set(hash, {
      hash,
      data: chunk,
      sizeBytes,
      createdAt: Date.now(),
      lastAccessedAt: Date.now(),
      accessCount: 0,
    });

    this.chunkCacheSize += sizeBytes;

    return { success: true, itemSize: sizeBytes, evictedItems: evictions };
  }

  /**
   * Get chunk from Tier 3 cache.
   */
  getChunk(hash: string): Uint8Array | null {
    const item = this.chunkCache.get(hash);
    if (item) {
      // Check TTL
      const age = (Date.now() - item.createdAt) / 1000;
      if (age > this.tiers.chunk.ttlSeconds) {
        // Expired
        this.chunkCache.delete(hash);
        this.chunkCacheSize -= item.sizeBytes;
        this.chunkStats.misses++;
        return null;
      }

      item.lastAccessedAt = Date.now();
      item.accessCount++;
      this.chunkStats.hits++;
      return item.data;
    }
    this.chunkStats.misses++;
    return null;
  }

  /**
   * Check if item exists in any tier.
   */
  has(hash: string, tier?: 'metadata' | 'blob' | 'chunk'): boolean {
    if (tier === 'metadata' || !tier) {
      if (this.metadataCache.has(hash)) return true;
    }
    if (tier === 'blob' || !tier) {
      if (this.blobCache.has(hash)) return true;
    }
    if (tier === 'chunk' || !tier) {
      if (this.chunkCache.has(hash)) return true;
    }
    return false;
  }

  /**
   * Remove item from specific tier.
   */
  remove(hash: string, tier?: 'metadata' | 'blob' | 'chunk'): boolean {
    let removed = false;

    if (tier === 'metadata' || !tier) {
      const item = this.metadataCache.get(hash);
      if (item) {
        this.metadataCache.delete(hash);
        removed = true;
      }
    }

    if (tier === 'blob' || !tier) {
      const item = this.blobCache.get(hash);
      if (item) {
        this.blobCache.delete(hash);
        this.blobCacheSize -= item.sizeBytes;
        removed = true;
      }
    }

    if (tier === 'chunk' || !tier) {
      const item = this.chunkCache.get(hash);
      if (item) {
        this.chunkCache.delete(hash);
        this.chunkCacheSize -= item.sizeBytes;
        removed = true;
      }
    }

    return removed;
  }

  /**
   * Clear entire cache or specific tier.
   */
  clear(tier?: 'metadata' | 'blob' | 'chunk'): void {
    if (tier === 'metadata' || !tier) {
      this.metadataCache.clear();
      this.metadataStats = { hits: 0, misses: 0, evictions: 0 };
    }
    if (tier === 'blob' || !tier) {
      this.blobCache.clear();
      this.blobCacheSize = 0;
      this.blobStats = { hits: 0, misses: 0, evictions: 0 };
    }
    if (tier === 'chunk' || !tier) {
      this.chunkCache.clear();
      this.chunkCacheSize = 0;
      this.chunkStats = { hits: 0, misses: 0, evictions: 0 };
    }
  }

  /**
   * Get statistics for a tier.
   */
  getStats(tier: 'metadata' | 'blob' | 'chunk'): CacheTierStats {
    let map: Map<string, CachedItem<any>>;
    let currentSize: number;
    let stats: any;
    const config = this.tiers[tier];

    switch (tier) {
      case 'metadata':
        map = this.metadataCache;
        currentSize = 0; // Metadata size unlimited
        stats = this.metadataStats;
        break;
      case 'blob':
        map = this.blobCache;
        currentSize = this.blobCacheSize;
        stats = this.blobStats;
        break;
      case 'chunk':
        map = this.chunkCache;
        currentSize = this.chunkCacheSize;
        stats = this.chunkStats;
        break;
    }

    const totalHits = stats.hits;
    const totalMisses = stats.misses;
    const hitRate = totalHits + totalMisses > 0 ? totalHits / (totalHits + totalMisses) : 0;

    return {
      name: config.name,
      itemCount: map.size,
      totalSizeBytes: currentSize,
      maxSizeBytes: config.maxSizeBytes,
      percentFull: config.maxSizeBytes === Infinity ? 0 : (currentSize / config.maxSizeBytes) * 100,
      evictionCount: stats.evictions,
      hitCount: stats.hits,
      missCount: stats.misses,
      hitRate,
    };
  }

  /**
   * Get all tier statistics.
   */
  getAllStats(): { [tier: string]: CacheTierStats } {
    return {
      metadata: this.getStats('metadata'),
      blob: this.getStats('blob'),
      chunk: this.getStats('chunk'),
    };
  }

  /**
   * Evict items from blob cache using LRU policy.
   */
  private evictFromBlobCache(requiredBytes: number): number {
    let evicted = 0;

    while (this.blobCacheSize + requiredBytes > this.tiers.blob.maxSizeBytes && this.blobCache.size > 0) {
      // Find least recently used item
      let lruHash = '';
      let lruTime = Infinity;

      for (const [hash, item] of this.blobCache.entries()) {
        if (item.lastAccessedAt < lruTime) {
          lruTime = item.lastAccessedAt;
          lruHash = hash;
        }
      }

      if (lruHash) {
        const item = this.blobCache.get(lruHash)!;
        this.blobCache.delete(lruHash);
        this.blobCacheSize -= item.sizeBytes;
        this.blobStats.evictions++;
        evicted++;
      } else {
        break;
      }
    }

    return evicted;
  }

  /**
   * Evict items from chunk cache using time-based policy.
   */
  private evictFromChunkCache(requiredBytes: number): number {
    let evicted = 0;
    const now = Date.now();
    const ttl = this.tiers.chunk.ttlSeconds * 1000;

    // First pass: remove expired items
    for (const [hash, item] of this.chunkCache.entries()) {
      if (now - item.createdAt > ttl) {
        this.chunkCache.delete(hash);
        this.chunkCacheSize -= item.sizeBytes;
        this.chunkStats.evictions++;
        evicted++;
      }
    }

    // Second pass: if still need space, use LRU
    while (this.chunkCacheSize + requiredBytes > this.tiers.chunk.maxSizeBytes && this.chunkCache.size > 0) {
      let lruHash = '';
      let lruTime = Infinity;

      for (const [hash, item] of this.chunkCache.entries()) {
        if (item.lastAccessedAt < lruTime) {
          lruTime = item.lastAccessedAt;
          lruHash = hash;
        }
      }

      if (lruHash) {
        const item = this.chunkCache.get(lruHash)!;
        this.chunkCache.delete(lruHash);
        this.chunkCacheSize -= item.sizeBytes;
        this.chunkStats.evictions++;
        evicted++;
      } else {
        break;
      }
    }

    return evicted;
  }

  /**
   * Start background cleanup timer.
   * Periodically cleans up expired items from blob and chunk caches.
   */
  private startCleanupTimer(): void {
    // Run cleanup every 5 minutes
    setInterval(() => {
      this.cleanupExpiredItems();
    }, 5 * 60 * 1000);
  }

  /**
   * Clean up expired items.
   */
  private cleanupExpiredItems(): void {
    const now = Date.now();

    // Cleanup blob cache
    for (const [hash, item] of this.blobCache.entries()) {
      const age = (now - item.createdAt) / 1000;
      if (age > this.tiers.blob.ttlSeconds) {
        this.blobCache.delete(hash);
        this.blobCacheSize -= item.sizeBytes;
        this.blobStats.evictions++;
      }
    }

    // Cleanup chunk cache
    for (const [hash, item] of this.chunkCache.entries()) {
      const age = (now - item.createdAt) / 1000;
      if (age > this.tiers.chunk.ttlSeconds) {
        this.chunkCache.delete(hash);
        this.chunkCacheSize -= item.sizeBytes;
        this.chunkStats.evictions++;
      }
    }
  }

  /**
   * Calculate total memory usage across all tiers.
   */
  getTotalMemoryUsageBytes(): number {
    return this.blobCacheSize + this.chunkCacheSize;
  }

  /**
   * Get memory usage report.
   */
  getMemoryReport(): {
    blobCacheBytes: number;
    chunkCacheBytes: number;
    totalBytes: number;
    percentOfBlobMax: number;
    percentOfChunkMax: number;
  } {
    return {
      blobCacheBytes: this.blobCacheSize,
      chunkCacheBytes: this.chunkCacheSize,
      totalBytes: this.getTotalMemoryUsageBytes(),
      percentOfBlobMax: (this.blobCacheSize / this.tiers.blob.maxSizeBytes) * 100,
      percentOfChunkMax: (this.chunkCacheSize / this.tiers.chunk.maxSizeBytes) * 100,
    };
  }
}
