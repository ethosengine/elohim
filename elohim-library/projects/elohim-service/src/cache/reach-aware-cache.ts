/**
 * Reach-Aware Cache - Framework-Agnostic Implementation
 *
 * Provides reach-level isolated caching for Elohim Protocol content.
 * Supports both WASM (high-performance) and pure TypeScript (portable) backends.
 *
 * Usage (any framework):
 * ```typescript
 * import { createReachAwareCache } from '@aspect/elohim-service/cache';
 *
 * const cache = await createReachAwareCache({
 *   maxSizePerReach: BigInt(128 * 1024 * 1024), // 128MB per reach
 *   preferWasm: true
 * });
 *
 * cache.put('content-hash', BigInt(1024), ReachLevel.COMMONS, 'elohim', 'governance', 50);
 * ```
 */

import type {
  IReachAwareCache,
  IBlobCache,
  IChunkCache,
  ICacheStats,
  CacheEntryMetadata,
  CacheConfig,
  CacheInitResult,
  PriorityParams,
} from './types';
import { ReachLevel } from './types';

// ============================================================================
// WASM Module Types (imported dynamically)
// ============================================================================

interface WasmModule {
  default: (input?: { module_or_path?: string }) => Promise<void>;
  ReachAwareCache: new (maxSizePerReach: bigint) => WasmReachAwareCache;
  BlobCache: new (maxSizeBytes: bigint) => WasmBlobCache;
  ChunkCache: new (maxSizeBytes: bigint, ttlMillis: bigint) => WasmChunkCache;
  calculate_priority: (
    reachLevel: number,
    proximityScore: number,
    bandwidthClass: number,
    stewardTier: number,
    affinityMatch: number,
    agePenalty: number
  ) => number;
  calculate_freshness: (masteryLevel: number, ageSeconds: number) => number;
}

interface WasmCacheStats {
  readonly item_count: number;
  readonly total_size_bytes: bigint;
  readonly eviction_count: bigint;
  readonly hit_count: bigint;
  readonly miss_count: bigint;
  hit_rate(): number;
  free(): void;
}

interface WasmReachAwareCache {
  put(hash: string, sizeBytes: bigint, reachLevel: number, domain: string, epic: string, priority: number): number;
  has(hash: string, reachLevel: number): boolean;
  touch(hash: string, reachLevel: number): boolean;
  delete(hash: string, reachLevel: number): boolean;
  stats_for_reach(reachLevel: number): WasmCacheStats;
  total_count(): number;
  total_size(): bigint;
  clear(): void;
  free(): void;
}

interface WasmBlobCache {
  put(hash: string, sizeBytes: bigint, reachLevel: number, domain: string, epic: string, priority: number): number;
  has(hash: string): boolean;
  touch(hash: string): boolean;
  delete(hash: string): boolean;
  get_json(hash: string): string;
  size(): bigint;
  count(): number;
  max_size(): bigint;
  stats(): WasmCacheStats;
  clear(): void;
  free(): void;
}

interface WasmChunkCache {
  put(hash: string, sizeBytes: bigint): number;
  has(hash: string): boolean;
  touch(hash: string): boolean;
  delete(hash: string): boolean;
  cleanup(): number;
  size(): bigint;
  count(): number;
  stats(): WasmCacheStats;
  clear(): void;
  free(): void;
}

// ============================================================================
// Pure TypeScript Implementation (Fallback)
// ============================================================================

/** TypeScript cache stats implementation */
class TsCacheStats implements ICacheStats {
  constructor(
    public readonly itemCount: number,
    public readonly totalSizeBytes: bigint,
    public readonly evictionCount: bigint,
    public readonly hitCount: bigint,
    public readonly missCount: bigint
  ) {}

  hitRate(): number {
    const total = this.hitCount + this.missCount;
    if (total === 0n) return 0;
    return Number((this.hitCount * 100n) / total);
  }
}

/** Internal cache entry */
interface InternalEntry {
  hash: string;
  sizeBytes: bigint;
  createdAt: number;
  lastAccessedAt: number;
  accessCount: number;
  reachLevel: number;
  domain: string;
  epic: string;
  priority: number;
}

/**
 * Pure TypeScript LRU cache using Map insertion order.
 * O(1) for all operations.
 */
class TsBlobCache implements IBlobCache {
  private entries = new Map<string, InternalEntry>();
  private currentSize = 0n;
  private hits = 0n;
  private misses = 0n;
  private evictions = 0n;

  constructor(private readonly maxBytes: bigint) {}

  put(
    hash: string,
    sizeBytes: bigint,
    reachLevel: number,
    domain: string,
    epic: string,
    priority: number
  ): number {
    // Remove existing to update position
    if (this.entries.has(hash)) {
      this.delete(hash);
    }

    // Evict LRU until space available
    let evicted = 0;
    while (this.currentSize + sizeBytes > this.maxBytes && this.entries.size > 0) {
      const lruHash = this.entries.keys().next().value;
      if (lruHash) {
        const entry = this.entries.get(lruHash)!;
        this.entries.delete(lruHash);
        this.currentSize -= entry.sizeBytes;
        this.evictions++;
        evicted++;
      } else {
        break;
      }
    }

    // Insert at end (MRU position)
    const now = Date.now();
    this.entries.set(hash, {
      hash,
      sizeBytes,
      createdAt: now,
      lastAccessedAt: now,
      accessCount: 0,
      reachLevel,
      domain,
      epic,
      priority,
    });
    this.currentSize += sizeBytes;

    return evicted;
  }

  has(hash: string): boolean {
    return this.entries.has(hash);
  }

  touch(hash: string): boolean {
    const entry = this.entries.get(hash);
    if (!entry) {
      this.misses++;
      return false;
    }

    // Move to end (MRU) by delete + re-insert
    this.entries.delete(hash);
    entry.lastAccessedAt = Date.now();
    entry.accessCount++;
    this.entries.set(hash, entry);
    this.hits++;
    return true;
  }

  delete(hash: string): boolean {
    const entry = this.entries.get(hash);
    if (entry) {
      this.entries.delete(hash);
      this.currentSize -= entry.sizeBytes;
      return true;
    }
    return false;
  }

  getJson(hash: string): string {
    const entry = this.entries.get(hash);
    if (!entry) {
      this.misses++;
      return '';
    }
    this.hits++;
    return JSON.stringify({
      hash: entry.hash,
      sizeBytes: Number(entry.sizeBytes),
      createdAt: entry.createdAt,
      lastAccessedAt: entry.lastAccessedAt,
      accessCount: entry.accessCount,
      reachLevel: entry.reachLevel,
      domain: entry.domain,
      epic: entry.epic,
      priority: entry.priority,
    });
  }

  getMetadata(hash: string): CacheEntryMetadata | null {
    const json = this.getJson(hash);
    if (!json) return null;
    return JSON.parse(json);
  }

  size(): bigint {
    return this.currentSize;
  }

  count(): number {
    return this.entries.size;
  }

  maxSize(): bigint {
    return this.maxBytes;
  }

  stats(): ICacheStats {
    return new TsCacheStats(
      this.entries.size,
      this.currentSize,
      this.evictions,
      this.hits,
      this.misses
    );
  }

  clear(): void {
    this.entries.clear();
    this.currentSize = 0n;
  }

  dispose(): void {
    this.clear();
  }
}

/**
 * Pure TypeScript TTL-based chunk cache.
 */
class TsChunkCache implements IChunkCache {
  private entries = new Map<string, { hash: string; sizeBytes: bigint; createdAt: number }>();
  private currentSize = 0n;
  private hits = 0n;
  private misses = 0n;
  private evictions = 0n;

  constructor(
    private readonly maxBytes: bigint,
    private readonly ttlMs: bigint
  ) {}

  put(hash: string, sizeBytes: bigint): number {
    if (this.entries.has(hash)) {
      this.delete(hash);
    }

    // Cleanup expired first
    this.cleanup();

    // Evict oldest until space available
    let evicted = 0;
    while (this.currentSize + sizeBytes > this.maxBytes && this.entries.size > 0) {
      const oldestHash = this.entries.keys().next().value;
      if (oldestHash) {
        const entry = this.entries.get(oldestHash)!;
        this.entries.delete(oldestHash);
        this.currentSize -= entry.sizeBytes;
        this.evictions++;
        evicted++;
      } else {
        break;
      }
    }

    this.entries.set(hash, {
      hash,
      sizeBytes,
      createdAt: Date.now(),
    });
    this.currentSize += sizeBytes;

    return evicted;
  }

  has(hash: string): boolean {
    const entry = this.entries.get(hash);
    if (!entry) return false;
    const age = BigInt(Date.now() - entry.createdAt);
    return age <= this.ttlMs;
  }

  touch(hash: string): boolean {
    const entry = this.entries.get(hash);
    if (!entry) {
      this.misses++;
      return false;
    }
    const age = BigInt(Date.now() - entry.createdAt);
    if (age > this.ttlMs) {
      this.delete(hash);
      this.misses++;
      return false;
    }
    this.hits++;
    return true;
  }

  delete(hash: string): boolean {
    const entry = this.entries.get(hash);
    if (entry) {
      this.entries.delete(hash);
      this.currentSize -= entry.sizeBytes;
      return true;
    }
    return false;
  }

  cleanup(): number {
    const now = Date.now();
    const ttlMs = Number(this.ttlMs);
    let cleaned = 0;

    for (const [hash, entry] of this.entries.entries()) {
      if (now - entry.createdAt > ttlMs) {
        this.entries.delete(hash);
        this.currentSize -= entry.sizeBytes;
        this.evictions++;
        cleaned++;
      }
    }

    return cleaned;
  }

  size(): bigint {
    return this.currentSize;
  }

  count(): number {
    return this.entries.size;
  }

  stats(): ICacheStats {
    return new TsCacheStats(
      this.entries.size,
      this.currentSize,
      this.evictions,
      this.hits,
      this.misses
    );
  }

  clear(): void {
    this.entries.clear();
    this.currentSize = 0n;
  }

  dispose(): void {
    this.clear();
  }
}

/**
 * Pure TypeScript reach-aware cache.
 * Maintains 8 isolated LRU caches (one per reach level).
 */
class TsReachAwareCache implements IReachAwareCache {
  private reachCaches: TsBlobCache[];

  constructor(maxSizePerReach: bigint) {
    this.reachCaches = Array.from(
      { length: 8 },
      () => new TsBlobCache(maxSizePerReach)
    );
  }

  put(
    hash: string,
    sizeBytes: bigint,
    reachLevel: number,
    domain: string,
    epic: string,
    priority: number
  ): number {
    const reach = Math.min(Math.max(0, reachLevel), 7);
    return this.reachCaches[reach].put(hash, sizeBytes, reachLevel, domain, epic, priority);
  }

  has(hash: string, reachLevel: number): boolean {
    const reach = Math.min(Math.max(0, reachLevel), 7);
    return this.reachCaches[reach].has(hash);
  }

  touch(hash: string, reachLevel: number): boolean {
    const reach = Math.min(Math.max(0, reachLevel), 7);
    return this.reachCaches[reach].touch(hash);
  }

  delete(hash: string, reachLevel: number): boolean {
    const reach = Math.min(Math.max(0, reachLevel), 7);
    return this.reachCaches[reach].delete(hash);
  }

  statsForReach(reachLevel: number): ICacheStats {
    const reach = Math.min(Math.max(0, reachLevel), 7);
    return this.reachCaches[reach].stats();
  }

  totalCount(): number {
    return this.reachCaches.reduce((sum, c) => sum + c.count(), 0);
  }

  totalSize(): bigint {
    return this.reachCaches.reduce((sum, c) => sum + c.size(), 0n);
  }

  clear(): void {
    this.reachCaches.forEach((c) => c.clear());
  }

  dispose(): void {
    this.reachCaches.forEach((c) => c.dispose());
  }
}

// ============================================================================
// WASM Wrapper Classes
// ============================================================================

/** WASM CacheStats wrapper */
class WasmCacheStatsWrapper implements ICacheStats {
  constructor(private wasm: WasmCacheStats) {}

  get itemCount(): number {
    return this.wasm.item_count;
  }
  get totalSizeBytes(): bigint {
    return this.wasm.total_size_bytes;
  }
  get evictionCount(): bigint {
    return this.wasm.eviction_count;
  }
  get hitCount(): bigint {
    return this.wasm.hit_count;
  }
  get missCount(): bigint {
    return this.wasm.miss_count;
  }
  hitRate(): number {
    return this.wasm.hit_rate();
  }
}

/** WASM ReachAwareCache wrapper */
class WasmReachAwareCacheWrapper implements IReachAwareCache {
  constructor(private wasm: WasmReachAwareCache) {}

  put(hash: string, sizeBytes: bigint, reachLevel: number, domain: string, epic: string, priority: number): number {
    return this.wasm.put(hash, sizeBytes, reachLevel, domain, epic, priority);
  }

  has(hash: string, reachLevel: number): boolean {
    return this.wasm.has(hash, reachLevel);
  }

  touch(hash: string, reachLevel: number): boolean {
    return this.wasm.touch(hash, reachLevel);
  }

  delete(hash: string, reachLevel: number): boolean {
    return this.wasm.delete(hash, reachLevel);
  }

  statsForReach(reachLevel: number): ICacheStats {
    return new WasmCacheStatsWrapper(this.wasm.stats_for_reach(reachLevel));
  }

  totalCount(): number {
    return this.wasm.total_count();
  }

  totalSize(): bigint {
    return this.wasm.total_size();
  }

  clear(): void {
    this.wasm.clear();
  }

  dispose(): void {
    this.wasm.free();
  }
}

/** WASM BlobCache wrapper */
class WasmBlobCacheWrapper implements IBlobCache {
  constructor(private wasm: WasmBlobCache) {}

  put(hash: string, sizeBytes: bigint, reachLevel: number, domain: string, epic: string, priority: number): number {
    return this.wasm.put(hash, sizeBytes, reachLevel, domain, epic, priority);
  }

  has(hash: string): boolean {
    return this.wasm.has(hash);
  }

  touch(hash: string): boolean {
    return this.wasm.touch(hash);
  }

  delete(hash: string): boolean {
    return this.wasm.delete(hash);
  }

  getJson(hash: string): string {
    return this.wasm.get_json(hash);
  }

  getMetadata(hash: string): CacheEntryMetadata | null {
    const json = this.getJson(hash);
    if (!json) return null;
    return JSON.parse(json);
  }

  size(): bigint {
    return this.wasm.size();
  }

  count(): number {
    return this.wasm.count();
  }

  maxSize(): bigint {
    return this.wasm.max_size();
  }

  stats(): ICacheStats {
    return new WasmCacheStatsWrapper(this.wasm.stats());
  }

  clear(): void {
    this.wasm.clear();
  }

  dispose(): void {
    this.wasm.free();
  }
}

/** WASM ChunkCache wrapper */
class WasmChunkCacheWrapper implements IChunkCache {
  constructor(private wasm: WasmChunkCache) {}

  put(hash: string, sizeBytes: bigint): number {
    return this.wasm.put(hash, sizeBytes);
  }

  has(hash: string): boolean {
    return this.wasm.has(hash);
  }

  touch(hash: string): boolean {
    return this.wasm.touch(hash);
  }

  delete(hash: string): boolean {
    return this.wasm.delete(hash);
  }

  cleanup(): number {
    return this.wasm.cleanup();
  }

  size(): bigint {
    return this.wasm.size();
  }

  count(): number {
    return this.wasm.count();
  }

  stats(): ICacheStats {
    return new WasmCacheStatsWrapper(this.wasm.stats());
  }

  clear(): void {
    this.wasm.clear();
  }

  dispose(): void {
    this.wasm.free();
  }
}

// ============================================================================
// Utility Functions (Framework-Agnostic)
// ============================================================================

/**
 * Calculate priority score for content.
 * Pure TypeScript implementation matching WASM.
 *
 * Priority = reach_level * 12 + proximity + bandwidth_bonus + steward_bonus + affinity * 10 - age_penalty
 */
export function calculatePriority(params: PriorityParams): number {
  let score = 0;

  // Base reach (0-84 points)
  score += Math.min(7, Math.max(0, params.reachLevel)) * 12;

  // Proximity (-100 to +100)
  score += Math.min(100, Math.max(-100, params.proximityScore));

  // Bandwidth bonus
  switch (params.bandwidthClass) {
    case 4: score += 20; break;  // Ultra
    case 3: score += 10; break;  // High
    case 2: score += 5; break;   // Medium
    case 1: score -= 5; break;   // Low
  }

  // Steward bonus
  switch (params.stewardTier) {
    case 4: score += 50; break;  // Pioneer
    case 3: score += 30; break;  // Expert
    case 2: score += 15; break;  // Curator
    case 1: score += 5; break;   // Caretaker
  }

  // Affinity (0-10 points)
  score += Math.floor(Math.min(1, Math.max(0, params.affinityMatch)) * 10);

  // Age penalty
  score -= params.agePenalty;

  return Math.min(200, Math.max(0, score));
}

/**
 * Calculate mastery freshness (0.0-1.0) based on age and mastery level.
 * Higher mastery levels decay slower.
 */
export function calculateFreshness(masteryLevel: number, ageSeconds: number): number {
  const decayPerDay: Record<number, number> = {
    0: 0,      // NotStarted - no decay
    1: 0.05,   // Seen
    2: 0.03,   // Remember
    3: 0.02,   // Understand
    4: 0.015,  // Apply
    5: 0.01,   // Analyze
    6: 0.008,  // Evaluate
    7: 0.005,  // Create
  };

  const decay = decayPerDay[Math.min(7, Math.max(0, masteryLevel))] ?? 0;
  const decayPerSecond = decay / 86400;
  return Math.max(0, 1 - decayPerSecond * ageSeconds);
}

// ============================================================================
// Factory Functions (Framework-Agnostic)
// ============================================================================

let wasmModule: WasmModule | null = null;
let wasmLoadAttempted = false;

/**
 * Load WASM module (internal).
 * Attempts once per session — WASM unavailability is expected in most environments.
 */
async function loadWasmModule(wasmPath?: string): Promise<WasmModule | null> {
  if (wasmModule) return wasmModule;
  if (wasmLoadAttempted) return null;

  wasmLoadAttempted = true;

  try {
    const path = wasmPath || '/wasm/holochain-cache-core/holochain_cache_core.js';
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const module: any = await import(/* webpackIgnore: true */ path);
    await module.default();
    wasmModule = module as WasmModule;
    return wasmModule;
  } catch {
    // WASM unavailability is expected — TypeScript fallback is used
    return null;
  }
}

/**
 * Create a reach-aware cache instance.
 *
 * @param config - Cache configuration
 * @returns Promise resolving to cache instance
 *
 * @example
 * ```typescript
 * // Works in any framework (Angular, Svelte, React, Vue, Node.js)
 * const cache = await createReachAwareCache({
 *   maxSizePerReach: BigInt(128 * 1024 * 1024), // 128MB per reach level
 *   preferWasm: true
 * });
 *
 * // Add content (reach level 7 = commons)
 * cache.put('sha256-abc123', BigInt(1024), 7, 'elohim', 'governance', 50);
 *
 * // Check existence
 * if (cache.has('sha256-abc123', 7)) {
 *   cache.touch('sha256-abc123', 7);
 * }
 * ```
 */
export async function createReachAwareCache(
  config: CacheConfig = {}
): Promise<{ cache: IReachAwareCache; implementation: 'wasm' | 'typescript' }> {
  const maxSize = config.maxSizePerReach ?? BigInt(128 * 1024 * 1024); // 128MB default

  if (config.preferWasm !== false) {
    const wasm = await loadWasmModule(config.wasmPath);
    if (wasm) {
      return {
        cache: new WasmReachAwareCacheWrapper(new wasm.ReachAwareCache(maxSize)),
        implementation: 'wasm',
      };
    }
  }

  return {
    cache: new TsReachAwareCache(maxSize),
    implementation: 'typescript',
  };
}

/**
 * Create a blob cache instance.
 */
export async function createBlobCache(
  config: CacheConfig = {}
): Promise<{ cache: IBlobCache; implementation: 'wasm' | 'typescript' }> {
  const maxSize = config.maxSizeBytes ?? BigInt(1024 * 1024 * 1024); // 1GB default

  if (config.preferWasm !== false) {
    const wasm = await loadWasmModule(config.wasmPath);
    if (wasm) {
      return {
        cache: new WasmBlobCacheWrapper(new wasm.BlobCache(maxSize)),
        implementation: 'wasm',
      };
    }
  }

  return {
    cache: new TsBlobCache(maxSize),
    implementation: 'typescript',
  };
}

/**
 * Create a chunk cache instance.
 */
export async function createChunkCache(
  config: CacheConfig = {}
): Promise<{ cache: IChunkCache; implementation: 'wasm' | 'typescript' }> {
  const maxSize = config.maxSizeBytes ?? BigInt(10 * 1024 * 1024 * 1024); // 10GB default
  const ttl = config.ttlMillis ?? BigInt(7 * 24 * 60 * 60 * 1000); // 7 days default

  if (config.preferWasm !== false) {
    const wasm = await loadWasmModule(config.wasmPath);
    if (wasm) {
      return {
        cache: new WasmChunkCacheWrapper(new wasm.ChunkCache(maxSize, ttl)),
        implementation: 'wasm',
      };
    }
  }

  return {
    cache: new TsChunkCache(maxSize, ttl),
    implementation: 'typescript',
  };
}

/**
 * Check if WASM module is available.
 */
export async function isWasmAvailable(wasmPath?: string): Promise<boolean> {
  const module = await loadWasmModule(wasmPath);
  return module !== null;
}

// Export TypeScript implementations for direct use if needed
export { TsBlobCache, TsChunkCache, TsReachAwareCache, TsCacheStats };
