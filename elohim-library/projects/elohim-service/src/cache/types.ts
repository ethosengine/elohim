/**
 * Elohim Protocol Cache Types
 *
 * Framework-agnostic interfaces for reach-aware content caching.
 * Compatible with Angular, Svelte, React, Vue, or vanilla TypeScript.
 *
 * These interfaces mirror the WASM holochain-cache-core module,
 * allowing seamless switching between WASM and pure TypeScript implementations.
 */

// ============================================================================
// Reach Levels (Elohim Protocol Constants)
// ============================================================================

/**
 * Content reach levels - concentric circles of visibility.
 * Lower numbers = more private, higher = more public.
 */
export const ReachLevel = {
  /** Only visible to author */
  PRIVATE: 0,
  /** Visible to explicitly invited agents */
  INVITED: 1,
  /** Visible within household/local network */
  LOCAL: 2,
  /** Visible within neighborhood */
  NEIGHBORHOOD: 3,
  /** Visible within municipality/city */
  MUNICIPAL: 4,
  /** Visible within bioregion */
  BIOREGIONAL: 5,
  /** Visible within larger region */
  REGIONAL: 6,
  /** Visible to entire commons (global) */
  COMMONS: 7,
} as const;

export type ReachLevelType = (typeof ReachLevel)[keyof typeof ReachLevel];

// ============================================================================
// Mastery Levels (Bloom's Taxonomy for content freshness)
// ============================================================================

export const MasteryLevel = {
  NOT_STARTED: 0,
  SEEN: 1,
  REMEMBER: 2,
  UNDERSTAND: 3,
  APPLY: 4,
  ANALYZE: 5,
  EVALUATE: 6,
  CREATE: 7,
} as const;

export type MasteryLevelType = (typeof MasteryLevel)[keyof typeof MasteryLevel];

// ============================================================================
// Cache Entry Metadata
// ============================================================================

/** Metadata for a cached content entry */
export interface CacheEntryMetadata {
  hash: string;
  sizeBytes: number;
  createdAt: number;
  lastAccessedAt: number;
  accessCount: number;
  reachLevel: ReachLevelType;
  domain: string;
  epic: string;
  priority: number;
}

// ============================================================================
// Cache Statistics
// ============================================================================

/** Snapshot of cache statistics */
export interface ICacheStats {
  readonly itemCount: number;
  readonly totalSizeBytes: bigint;
  readonly evictionCount: bigint;
  readonly hitCount: bigint;
  readonly missCount: bigint;
  hitRate(): number;
}

// ============================================================================
// Core Cache Interfaces (Framework-Agnostic)
// ============================================================================

/**
 * Basic LRU blob cache interface.
 * Implementations can be WASM or pure TypeScript.
 */
export interface IBlobCache {
  /** Add or update entry. Returns number of items evicted. */
  put(
    hash: string,
    sizeBytes: bigint,
    reachLevel: number,
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
  getJson(hash: string): string;

  /** Get parsed entry metadata (null if not found) */
  getMetadata(hash: string): CacheEntryMetadata | null;

  /** Current cache size in bytes */
  size(): bigint;

  /** Number of items in cache */
  count(): number;

  /** Maximum cache size in bytes */
  maxSize(): bigint;

  /** Get statistics snapshot */
  stats(): ICacheStats;

  /** Clear all entries */
  clear(): void;

  /** Release resources (for WASM cleanup) */
  dispose(): void;
}

/**
 * TTL-based chunk cache interface.
 */
export interface IChunkCache {
  /** Add chunk. Returns number of items evicted. */
  put(hash: string, sizeBytes: bigint): number;

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
  stats(): ICacheStats;

  /** Clear all */
  clear(): void;

  /** Release resources */
  dispose(): void;
}

/**
 * Reach-aware cache with isolated LRU per reach level.
 * Content at different reach levels never evict each other.
 */
export interface IReachAwareCache {
  /** Add entry to appropriate reach cache */
  put(
    hash: string,
    sizeBytes: bigint,
    reachLevel: number,
    domain: string,
    epic: string,
    priority: number
  ): number;

  /** Check if entry exists at specified reach level */
  has(hash: string, reachLevel: number): boolean;

  /** Touch entry at reach level */
  touch(hash: string, reachLevel: number): boolean;

  /** Delete entry from reach level */
  delete(hash: string, reachLevel: number): boolean;

  /** Get stats for specific reach level */
  statsForReach(reachLevel: number): ICacheStats;

  /** Total items across all reach levels */
  totalCount(): number;

  /** Total size across all reach levels */
  totalSize(): bigint;

  /** Clear all caches */
  clear(): void;

  /** Release resources */
  dispose(): void;
}

// ============================================================================
// Priority Calculation Parameters
// ============================================================================

/** Parameters for content priority calculation */
export interface PriorityParams {
  /** 0-7 (private -> commons) */
  reachLevel: number;
  /** -100 to +100 (geographic proximity to custodian) */
  proximityScore: number;
  /** 1-4 (low -> ultra) */
  bandwidthClass: number;
  /** 1-4 (caretaker -> pioneer) */
  stewardTier: number;
  /** 0.0-1.0 (content relevance to user) */
  affinityMatch: number;
  /** Penalty for aged content */
  agePenalty: number;
}

// ============================================================================
// Factory Configuration
// ============================================================================

/** Configuration for cache factory */
export interface CacheConfig {
  /** Maximum size in bytes per reach level (for ReachAwareCache) */
  maxSizePerReach?: bigint;
  /** Maximum total size in bytes (for BlobCache) */
  maxSizeBytes?: bigint;
  /** TTL in milliseconds (for ChunkCache) */
  ttlMillis?: bigint;
  /** Prefer WASM implementation if available */
  preferWasm?: boolean;
  /** Path to WASM module (for custom loading) */
  wasmPath?: string;
}

/** Cache implementation type */
export type CacheImplementation = 'wasm' | 'typescript';

/** Result of cache initialization */
export interface CacheInitResult {
  implementation: CacheImplementation;
  cache: IReachAwareCache | IBlobCache | IChunkCache;
}

/** Cache tier configuration (for multi-tier setups) */
export interface CacheTierConfig {
  name: string;
  maxSizeBytes: number;
  ttlSeconds: number;
  evictionPolicy: 'lru' | 'time-based';
}
