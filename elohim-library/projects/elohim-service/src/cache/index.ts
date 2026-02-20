/**
 * Elohim Protocol Cache Module
 *
 * Framework-agnostic reach-aware content caching for Elohim Protocol.
 * Works with Angular, Svelte, React, Vue, Node.js, Deno, or vanilla TypeScript.
 *
 * @example
 * ```typescript
 * import {
 *   createReachAwareCache,
 *   ReachLevel,
 *   calculatePriority
 * } from '@aspect/elohim-service/cache';
 *
 * // Create cache (prefers WASM, falls back to TypeScript)
 * const { cache, implementation } = await createReachAwareCache({
 *   maxSizePerReach: BigInt(128 * 1024 * 1024)
 * });
 *
 * console.log(`Using ${implementation} implementation`);
 *
 * // Add content with reach-level isolation
 * const priority = calculatePriority({
 *   reachLevel: ReachLevel.COMMONS,
 *   proximityScore: 50,
 *   bandwidthClass: 3,
 *   stewardTier: 2,
 *   affinityMatch: 0.8,
 *   agePenalty: 0
 * });
 *
 * cache.put('content-hash', BigInt(1024), ReachLevel.COMMONS, 'elohim', 'governance', priority);
 * ```
 *
 * @packageDocumentation
 */

// Types and constants
export {
  // Reach levels
  ReachLevel,
  type ReachLevelType,

  // Mastery levels
  MasteryLevel,
  type MasteryLevelType,

  // Cache interfaces
  type ICacheStats,
  type IBlobCache,
  type IChunkCache,
  type IReachAwareCache,

  // Metadata and configuration
  type CacheEntryMetadata,
  type CacheConfig,
  type CacheInitResult,
  type CacheImplementation,
  type PriorityParams,

  // Tier configuration
  type CacheTierConfig,
} from './types';

// Factory functions
export {
  createReachAwareCache,
  createBlobCache,
  createChunkCache,
  isWasmAvailable,
} from './reach-aware-cache';

// Utility functions
export { calculatePriority, calculateFreshness } from './reach-aware-cache';

// TypeScript implementations (for direct use without async factory)
export { TsBlobCache, TsChunkCache, TsReachAwareCache, TsCacheStats } from './reach-aware-cache';

// Content Resolver - Unified tiered source resolution
export {
  // Types
  SourceTier,
  type IContentResolver,
  type ResolutionResult,
  type ResolutionError,
  type AppResolutionResult,
  type ResolverStats,
  type SourceInfo,
  type ResolverConfig,
  type ResolverInitResult,

  // Factory functions
  createContentResolver,
  isWasmResolverAvailable,

  // TypeScript implementation
  TsContentResolver,
} from './content-resolver';

// Write Buffer - Batched write operations with priority queues
export {
  // Types
  WritePriority,
  WriteOpType,
  type IWriteBuffer,
  type WriteOperation,
  type WriteBatch,
  type BatchResult,
  type WriteBufferStats,
  type WriteBufferConfig,
  type WriteBufferInitResult,

  // Factory functions
  createWriteBuffer,
  createSeedingBuffer,
  createInteractiveBuffer,
  createRecoveryBuffer,
  isWasmBufferAvailable,

  // TypeScript implementation
  TsWriteBuffer,
} from './write-buffer';
