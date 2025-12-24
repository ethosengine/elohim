/**
 * Type declarations for holochain-cache-core WASM module.
 *
 * This module is built from holochain/holochain-cache-core using wasm-pack.
 * If the WASM module is not available, the reach-aware-cache service
 * will gracefully fall back to the TypeScript implementation.
 *
 * These types mirror the interfaces in elohim-library/projects/elohim-service/src/cache/types.ts
 */

declare module 'holochain-cache-core' {
  /**
   * Initialize the WASM module.
   * Must be called before using other functions.
   */
  export default function init(): Promise<void>;

  /**
   * Cache statistics snapshot.
   */
  export class CacheStats {
    readonly item_count: number;
    readonly total_size_bytes: bigint;
    readonly eviction_count: bigint;
    readonly hit_count: bigint;
    readonly miss_count: bigint;
    hit_rate(): number;
    free(): void;
  }

  /**
   * Cache entry metadata.
   */
  export class CacheEntryMetadata {
    readonly hash: string;
    readonly size_bytes: bigint;
    readonly created_at: bigint;
    readonly last_accessed_at: bigint;
    readonly access_count: number;
    readonly reach_level: number;
    readonly domain: string;
    readonly epic: string;
    readonly priority: number;
    free(): void;
  }

  /**
   * Basic LRU blob cache.
   * Uses O(log n) eviction via skip list.
   */
  export class BlobCache {
    constructor(max_size_bytes: bigint);

    /**
     * Add or update entry. Returns number of items evicted.
     */
    put(
      hash: string,
      size_bytes: bigint,
      reach_level: number,
      domain: string,
      epic: string,
      priority: number
    ): number;

    /** Check if entry exists */
    has(hash: string): boolean;

    /** Update access time, returns true if found */
    touch(hash: string): boolean;

    /** Delete entry, returns true if found */
    delete(hash: string): boolean;

    /** Get entry metadata as JSON string (empty if not found) */
    get_json(hash: string): string;

    /** Current cache size in bytes */
    size(): bigint;

    /** Number of items in cache */
    count(): number;

    /** Maximum cache size in bytes */
    max_size(): bigint;

    /** Get statistics snapshot */
    stats(): CacheStats;

    /** Clear all entries */
    clear(): void;

    /** Release WASM memory */
    free(): void;
  }

  /**
   * TTL-based chunk cache for streaming media.
   */
  export class ChunkCache {
    constructor(max_size_bytes: bigint, ttl_millis: bigint);

    /** Add chunk. Returns number of items evicted. */
    put(hash: string, size_bytes: bigint): number;

    /** Check if chunk exists and is not expired */
    has(hash: string): boolean;

    /** Touch chunk, returns true if found and valid */
    touch(hash: string): boolean;

    /** Delete chunk */
    delete(hash: string): boolean;

    /** Remove expired items. Returns count removed. */
    cleanup(): number;

    /** Current size in bytes */
    size(): bigint;

    /** Number of items */
    count(): number;

    /** Get statistics */
    stats(): CacheStats;

    /** Clear all */
    clear(): void;

    /** Release WASM memory */
    free(): void;
  }

  /**
   * Reach-aware cache with isolated LRU per reach level.
   * Content at different reach levels never evict each other.
   */
  export class ReachAwareCache {
    constructor(max_size_per_reach: bigint);

    /** Add entry to appropriate reach cache */
    put(
      hash: string,
      size_bytes: bigint,
      reach_level: number,
      domain: string,
      epic: string,
      priority: number
    ): number;

    /** Check if entry exists at specified reach level */
    has(hash: string, reach_level: number): boolean;

    /** Touch entry at reach level */
    touch(hash: string, reach_level: number): boolean;

    /** Delete entry from reach level */
    delete(hash: string, reach_level: number): boolean;

    /** Get stats for specific reach level */
    stats_for_reach(reach_level: number): CacheStats;

    /** Total items across all reach levels */
    total_count(): number;

    /** Total size across all reach levels */
    total_size(): bigint;

    /** Clear all caches */
    clear(): void;

    /** Release WASM memory */
    free(): void;
  }

  /**
   * Calculate content priority score.
   *
   * @param reach_level - 0-7 (private -> commons)
   * @param proximity_score - -100 to +100 (geographic proximity)
   * @param bandwidth_class - 1-4 (low -> ultra)
   * @param steward_tier - 1-4 (caretaker -> pioneer)
   * @param affinity_match - 0.0-1.0 (content relevance)
   * @param age_penalty - Penalty for aged content
   * @returns Priority score (higher = more important to cache)
   */
  export function calculate_priority(
    reach_level: number,
    proximity_score: number,
    bandwidth_class: number,
    steward_tier: number,
    affinity_match: number,
    age_penalty: number
  ): number;
}
