/**
 * Content Resolver Service - Angular Wrapper
 *
 * Provides Angular dependency injection wrapper around the framework-agnostic
 * content resolver. Automatically initializes WASM or falls back to TypeScript.
 *
 * This service centralizes the tiered resolution logic that was previously
 * scattered throughout DataLoaderService (2400+ lines of repeated fallback patterns).
 *
 * Resolution order:
 * 1. Local (IndexedDB) - fastest, offline-capable
 * 2. Projection (Doorway's MongoDB cache) - fast, eventually consistent
 * 3. Authoritative (Conductor → Edgenode → DHT) - slow, source of truth
 * 4. External (fallback URLs) - last resort
 *
 * Usage:
 * ```typescript
 * @Component({...})
 * export class MyComponent {
 *   constructor(private resolver: ContentResolverService) {}
 *
 *   async ngOnInit() {
 *     await this.resolver.initialize();
 *
 *     // Get resolution chain for content
 *     const result = this.resolver.resolve('content', 'my-content-id');
 *     // { sourceId: 'indexeddb', tier: 0, url: null, cached: true }
 *   }
 * }
 * ```
 */

import { Injectable, OnDestroy, inject } from '@angular/core';
import { BehaviorSubject, firstValueFrom } from 'rxjs';

// Import services for actual fetching
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { ProjectionAPIService } from './projection-api.service';
import { HolochainContentService, HolochainPathWithSteps } from './holochain-content.service';

// Import models
import { ContentNode } from '../../lamad/models/content-node.model';
import { LearningPath } from '../../lamad/models/learning-path.model';

// Import from framework-agnostic content resolver
import type {
  IContentResolver,
  ResolutionResult,
  ResolutionError,
  AppResolutionResult,
  ResolverStats,
  SourceInfo,
  ResolverConfig,
} from '@elohim/service/cache/content-resolver';

import {
  SourceTier,
  createContentResolver,
  isWasmResolverAvailable,
  TsContentResolver,
} from '@elohim/service/cache/content-resolver';

// Import connection strategy types
import type {
  IConnectionStrategy,
  ConnectionConfig,
  ContentSourceConfig,
} from '@elohim/service/connection';

// Re-export types and enums for convenience
export {
  SourceTier,
  type IContentResolver,
  type ResolutionResult,
  type ResolutionError,
  type AppResolutionResult,
  type ResolverStats,
  type SourceInfo,
};

/** Service state */
export type ResolverServiceState = 'uninitialized' | 'initializing' | 'ready' | 'error';

/** Initialization result */
export interface ResolverInitializationResult {
  success: boolean;
  implementation: 'wasm' | 'typescript';
  error?: string;
}

/** Source registration config */
export interface SourceRegistration {
  id: string;
  tier: SourceTier;
  priority: number;
  contentTypes: string[];
  baseUrl?: string;
}

/** Standard source configurations for Elohim Protocol */
export const STANDARD_SOURCES: Record<string, SourceRegistration> = {
  indexeddb: {
    id: 'indexeddb',
    tier: SourceTier.Local,
    priority: 100,
    contentTypes: ['path', 'content', 'graph', 'assessment', 'profile'],
  },
  projection: {
    id: 'projection',
    tier: SourceTier.Projection,
    priority: 80,
    contentTypes: ['path', 'content', 'graph', 'assessment', 'profile', 'blob'],
    // baseUrl set dynamically based on doorway connection
  },
  /**
   * @deprecated Conductor is no longer used for content resolution.
   * Content is now served from doorway projection (SQLite).
   * Conductor remains available for agent-centric data only (identity, attestations, points).
   */
  conductor: {
    id: 'conductor',
    tier: SourceTier.Authoritative,
    priority: 50,
    contentTypes: ['identity', 'attestation', 'point-balance'], // Content types removed - use projection instead
  },
  edgenode: {
    id: 'edgenode',
    tier: SourceTier.Authoritative,
    priority: 40, // Lower priority than conductor (conductor preferred when local)
    contentTypes: ['path', 'content', 'graph', 'assessment', 'profile', 'blob', 'stream', 'app'],
    // baseUrl set dynamically based on edgenode connection
  },
  dht: {
    id: 'dht',
    tier: SourceTier.Authoritative,
    priority: 30, // Last resort for authoritative data
    contentTypes: ['identity', 'attestation', 'point-balance'],
  },
  cdn: {
    id: 'cdn',
    tier: SourceTier.External,
    priority: 20,
    contentTypes: ['blob', 'app'],
    // baseUrl set to CDN endpoint
  },
};

/**
 * Angular service providing unified content resolution.
 *
 * Features:
 * - Automatic WASM/TypeScript selection
 * - Tiered source resolution with learning
 * - Observable state for reactive UIs
 * - Pre-configured standard sources
 */
/** Resolution outcome with source info */
export interface ContentResolution<T> {
  data: T;
  sourceId: string;
  tier: SourceTier;
  durationMs: number;
}

@Injectable({
  providedIn: 'root',
})
export class ContentResolverService implements OnDestroy {
  private resolver: IContentResolver | null = null;

  private readonly stateSubject = new BehaviorSubject<ResolverServiceState>('uninitialized');
  private implementation: 'wasm' | 'typescript' = 'typescript';
  private initPromise: Promise<ResolverInitializationResult> | null = null;

  // Injected fetcher services
  private readonly idbCache = inject(IndexedDBCacheService);
  private readonly projectionApi = inject(ProjectionAPIService);
  private readonly holochainContent = inject(HolochainContentService);

  /** Observable resolver service state */
  readonly state$ = this.stateSubject.asObservable();

  /** Current state */
  get state(): ResolverServiceState {
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
   * Initialize the resolver service.
   * Call this before using any resolution operations.
   *
   * @param config - Optional configuration
   * @returns Initialization result
   */
  async initialize(config?: ResolverConfig): Promise<ResolverInitializationResult> {
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

  private async doInitialize(config?: ResolverConfig): Promise<ResolverInitializationResult> {
    try {
      const result = await createContentResolver(config);
      this.resolver = result.resolver;
      this.implementation = result.implementation;

      this.stateSubject.next('ready');
      console.log(`[ContentResolverService] Initialized with ${this.implementation} implementation`);

      return {
        success: true,
        implementation: this.implementation,
      };
    } catch (error) {
      console.error('[ContentResolverService] Initialization failed:', error);

      // Fallback to TypeScript implementation
      try {
        this.resolver = new TsContentResolver();
        this.implementation = 'typescript';
        this.stateSubject.next('ready');

        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        return {
          success: true,
          implementation: 'typescript',
          error: `WASM failed, using TypeScript fallback: ${errorMessage}`,
        };
      } catch (fallbackError) {
        this.stateSubject.next('error');
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        return {
          success: false,
          implementation: 'typescript',
          error: `All initialization attempts failed: ${errorMessage}`,
        };
      }
    }
  }

  // ==========================================================================
  // Source Management
  // ==========================================================================

  /**
   * Register a content source.
   *
   * @param id - Unique source identifier
   * @param tier - Source tier (Local, Projection, Authoritative, External)
   * @param priority - Priority within tier (higher = preferred)
   * @param contentTypes - Content types this source can provide
   * @param baseUrl - Optional base URL for URL-based sources
   */
  registerSource(
    id: string,
    tier: SourceTier,
    priority: number,
    contentTypes: string[],
    baseUrl?: string
  ): void {
    this.ensureReady();
    this.resolver!.registerSource(id, tier, priority, contentTypes, baseUrl);
  }

  /**
   * Register a standard source configuration.
   *
   * @param sourceId - Standard source ID (indexeddb, projection, conductor, etc.)
   * @param baseUrl - Optional override for base URL
   */
  registerStandardSource(sourceId: keyof typeof STANDARD_SOURCES, baseUrl?: string): void {
    const config = STANDARD_SOURCES[sourceId];
    if (!config) {
      throw new Error(`Unknown standard source: ${sourceId}`);
    }
    this.registerSource(
      config.id,
      config.tier,
      config.priority,
      config.contentTypes,
      baseUrl ?? config.baseUrl
    );
  }

  /**
   * Register all standard sources for full Elohim Protocol resolution.
   *
   * @param urls - URL overrides for URL-based sources
   * @deprecated Use initializeForMode() instead for mode-aware source registration
   */
  registerAllStandardSources(urls?: {
    projection?: string;
    edgenode?: string;
    cdn?: string;
  }): void {
    this.registerStandardSource('indexeddb');
    this.registerStandardSource('projection', urls?.projection);
    this.registerStandardSource('conductor');
    this.registerStandardSource('edgenode', urls?.edgenode);
    this.registerStandardSource('dht');
    this.registerStandardSource('cdn', urls?.cdn);
  }

  /**
   * Initialize resolver with mode-aware sources from connection strategy.
   *
   * This method configures the content resolver sources based on the current
   * connection mode (doorway or direct). Different modes have different source
   * hierarchies:
   *
   * **Doorway Mode (browser)**:
   * - indexeddb → projection → conductor
   * - Uses Projection tier (Doorway's MongoDB cache) for fast reads
   *
   * **Direct Mode (native/Tauri)**:
   * - indexeddb → conductor → elohim-storage
   * - Skips Projection tier, goes directly to Authoritative
   *
   * @param strategy - The connection strategy providing source configuration
   * @param config - Connection configuration for URL resolution
   * @returns Initialization result
   *
   * @example
   * ```typescript
   * await resolver.initialize();
   * await resolver.initializeForMode(strategy, connectionConfig);
   * ```
   */
  async initializeForMode(
    strategy: IConnectionStrategy,
    config: ConnectionConfig
  ): Promise<void> {
    // Ensure resolver is initialized first
    if (this.state !== 'ready') {
      await this.initialize();
    }

    // Get sources from strategy
    const sources = strategy.getContentSources(config);
    console.log(
      `[ContentResolverService] Configuring sources for ${strategy.name} mode:`,
      sources.map(s => s.id)
    );

    // Register each source from the strategy
    for (const source of sources) {
      this.registerSource(
        source.id,
        source.tier,
        source.priority,
        source.contentTypes,
        source.baseUrl
      );

      // Set initial availability
      if (source.available !== undefined) {
        this.setSourceAvailable(source.id, source.available);
      }
    }

    console.log(
      `[ContentResolverService] Initialized for ${strategy.mode} mode with ${sources.length} sources`
    );
  }

  /**
   * Register sources from a ContentSourceConfig array.
   * Useful for custom source configurations or testing.
   *
   * @param sources - Array of source configurations
   */
  registerSourcesFromConfig(sources: ContentSourceConfig[]): void {
    this.ensureReady();
    for (const source of sources) {
      this.registerSource(
        source.id,
        source.tier,
        source.priority,
        source.contentTypes,
        source.baseUrl
      );
      if (source.available !== undefined) {
        this.setSourceAvailable(source.id, source.available);
      }
    }
  }

  /**
   * Update source base URL.
   */
  setSourceUrl(sourceId: string, baseUrl: string | null): void {
    this.ensureReady();
    this.resolver!.setSourceUrl(sourceId, baseUrl);
  }

  /**
   * Mark source as available/unavailable.
   * When a source fails, mark it unavailable to skip it in resolution.
   */
  setSourceAvailable(sourceId: string, available: boolean): void {
    this.ensureReady();
    this.resolver!.setSourceAvailable(sourceId, available);
  }

  /**
   * Check if source is available.
   */
  isSourceAvailable(sourceId: string): boolean {
    this.ensureReady();
    return this.resolver!.isSourceAvailable(sourceId);
  }

  // ==========================================================================
  // Content Resolution
  // ==========================================================================

  /**
   * Resolve which source to try for content.
   *
   * Returns the best available source for the content type.
   * If content was previously found at a source, returns that source first.
   *
   * @param contentType - Type of content (path, content, blob, etc.)
   * @param contentId - Content identifier
   * @returns Resolution result or error
   */
  resolve(contentType: string, contentId: string): ResolutionResult | ResolutionError {
    this.ensureReady();
    return this.resolver!.resolve(contentType, contentId);
  }

  /**
   * Get the full resolution chain for a content type.
   * Useful for debugging or showing source hierarchy in UI.
   */
  getResolutionChain(contentType: string): SourceInfo[] {
    this.ensureReady();
    return this.resolver!.getResolutionChain(contentType);
  }

  /**
   * Record that content was found at a source.
   * Call this after successfully fetching content to enable content learning.
   */
  recordContentLocation(contentId: string, sourceId: string): void {
    this.ensureReady();
    this.resolver!.recordContentLocation(contentId, sourceId);
  }

  /**
   * Remove content location record.
   * Call this when content is deleted or invalidated at a source.
   */
  removeContentLocation(contentId: string, sourceId: string): void {
    this.ensureReady();
    this.resolver!.removeContentLocation(contentId, sourceId);
  }

  /**
   * Clear all content locations for a source.
   * Call this when a source is reset or cleared.
   */
  clearSourceLocations(sourceId: string): void {
    this.ensureReady();
    this.resolver!.clearSourceLocations(sourceId);
  }

  // ==========================================================================
  // App Resolution
  // ==========================================================================

  /**
   * Register an HTML5 app.
   *
   * @param appId - Unique app identifier
   * @param blobHash - Content hash of the app bundle
   * @param entryPoint - Entry point file (default: index.html)
   * @param fallbackUrl - Fallback URL if app cannot be served locally
   */
  registerApp(appId: string, blobHash: string, entryPoint: string, fallbackUrl?: string): void {
    this.ensureReady();
    this.resolver!.registerApp(appId, blobHash, entryPoint, fallbackUrl);
  }

  /**
   * Unregister an app.
   */
  unregisterApp(appId: string): void {
    this.ensureReady();
    this.resolver!.unregisterApp(appId);
  }

  /**
   * Check if app is registered.
   */
  hasApp(appId: string): boolean {
    this.ensureReady();
    return this.resolver!.hasApp(appId);
  }

  /**
   * Get app blob hash.
   */
  getAppBlobHash(appId: string): string | null {
    this.ensureReady();
    return this.resolver!.getAppBlobHash(appId);
  }

  /**
   * Resolve app URL.
   * Returns the best URL to load the app from.
   */
  resolveAppUrl(appId: string, path?: string): string {
    this.ensureReady();
    return this.resolver!.resolveAppUrl(appId, path);
  }

  /**
   * Resolve app URL with full metadata.
   * Returns URL, source, blob hash, and fallback information.
   */
  resolveAppUrlFull(appId: string, path?: string): AppResolutionResult {
    this.ensureReady();
    return this.resolver!.resolveAppUrlFull(appId, path);
  }

  // ==========================================================================
  // Statistics
  // ==========================================================================

  /**
   * Get resolver statistics.
   */
  getStats(): ResolverStats {
    this.ensureReady();
    return this.resolver!.getStats();
  }

  /**
   * Reset resolver statistics.
   */
  resetStats(): void {
    this.ensureReady();
    this.resolver!.resetStats();
  }

  /**
   * Get source count.
   */
  sourceCount(): number {
    this.ensureReady();
    return this.resolver!.sourceCount();
  }

  /**
   * Get indexed content count.
   */
  indexedContentCount(): number {
    this.ensureReady();
    return this.resolver!.indexedContentCount();
  }

  /**
   * Get registered app count.
   */
  registeredAppCount(): number {
    this.ensureReady();
    return this.resolver!.registeredAppCount();
  }

  // ==========================================================================
  // Unified Content Resolution with Fetching
  // ==========================================================================

  /**
   * Resolve and fetch content from the optimal source.
   *
   * Tries sources in priority order based on resolver's routing.
   * Records successful locations for future O(1) lookups.
   *
   * @param contentId - Content identifier
   * @returns Resolution with data, or null if not found
   */
  async resolveContent(contentId: string): Promise<ContentResolution<ContentNode> | null> {
    this.ensureReady();
    const startTime = performance.now();

    const chain = this.resolver!.getResolutionChain('content');

    for (const source of chain) {
      // For conductor, check actual availability (handles race condition during init)
      if (source.id === 'conductor') {
        if (!this.holochainContent.isAvailable()) {
          console.debug('[ContentResolver] Conductor not yet available for content, skipping');
          continue;
        }
        // Update the resolver's knowledge now that conductor is available
        this.resolver!.setSourceAvailable('conductor', true);
      } else if (!this.resolver!.isSourceAvailable(source.id)) {
        continue;
      }

      try {
        const data = await this.fetchContentFromSource(contentId, source.id);
        if (data) {
          this.resolver!.recordContentLocation(contentId, source.id);
          return {
            data,
            sourceId: source.id,
            tier: source.tier,
            durationMs: performance.now() - startTime,
          };
        }
      } catch (err) {
        console.debug(`[ContentResolver] Source ${source.id} failed for ${contentId}:`, err);
      }
    }

    return null;
  }

  /**
   * Fetch content from a specific source.
   */
  private async fetchContentFromSource(
    contentId: string,
    sourceId: string
  ): Promise<ContentNode | null> {
    switch (sourceId) {
      case 'indexeddb':
        return await this.idbCache.getContent(contentId);

      case 'projection':
        if (!this.projectionApi.enabled) return null;
        return await firstValueFrom(this.projectionApi.getContent(contentId));

      case 'conductor':
        // Conductor no longer handles content - use projection instead
        // Conductor is now reserved for agent-centric data (identity, attestations, points)
        console.debug('[ContentResolver] Conductor skipped for content - use projection');
        return null;

      default:
        return null;
    }
  }

  /**
   * Resolve and fetch a learning path from the optimal source.
   *
   * @param pathId - Path identifier
   * @returns Resolution with data, or null if not found
   */
  async resolvePath(pathId: string): Promise<ContentResolution<LearningPath> | null> {
    this.ensureReady();
    const startTime = performance.now();

    const chain = this.resolver!.getResolutionChain('path');

    for (const source of chain) {
      // For conductor, check actual availability (handles race condition during init)
      if (source.id === 'conductor') {
        if (!this.holochainContent.isAvailable()) {
          console.debug('[ContentResolver] Conductor not yet available, skipping');
          continue;
        }
        // Update the resolver's knowledge now that conductor is available
        this.resolver!.setSourceAvailable('conductor', true);
      } else if (!this.resolver!.isSourceAvailable(source.id)) {
        continue;
      }

      try {
        const data = await this.fetchPathFromSource(pathId, source.id);
        if (data) {
          this.resolver!.recordContentLocation(pathId, source.id);
          return {
            data,
            sourceId: source.id,
            tier: source.tier,
            durationMs: performance.now() - startTime,
          };
        }
      } catch (err) {
        console.debug(`[ContentResolver] Source ${source.id} failed for path ${pathId}:`, err);
      }
    }

    return null;
  }

  /**
   * Fetch path from a specific source.
   */
  private async fetchPathFromSource(
    pathId: string,
    sourceId: string
  ): Promise<LearningPath | null> {
    switch (sourceId) {
      case 'indexeddb':
        return await this.idbCache.getPath(pathId);

      case 'projection':
        if (!this.projectionApi.enabled) return null;
        return await firstValueFrom(this.projectionApi.getPath(pathId));

      case 'conductor':
        // Conductor no longer handles paths - use projection instead
        // Conductor is now reserved for agent-centric data (identity, attestations, points)
        console.debug('[ContentResolver] Conductor skipped for path - use projection');
        return null;

      default:
        return null;
    }
  }

  /**
   * Batch resolve multiple content items efficiently.
   * Groups requests by source to minimize round-trips.
   */
  async batchResolveContent(
    contentIds: string[]
  ): Promise<Map<string, ContentResolution<ContentNode>>> {
    this.ensureReady();
    const results = new Map<string, ContentResolution<ContentNode>>();
    const remaining = new Set(contentIds);
    const startTime = performance.now();

    const chain = this.resolver!.getResolutionChain('content');

    for (const source of chain) {
      if (remaining.size === 0) break;

      // For conductor, check actual availability (handles race condition during init)
      if (source.id === 'conductor') {
        if (!this.holochainContent.isAvailable()) {
          continue;
        }
        this.resolver!.setSourceAvailable('conductor', true);
      } else if (!this.resolver!.isSourceAvailable(source.id)) {
        continue;
      }

      const toFetch = Array.from(remaining);
      const batchResult = await this.batchFetchFromSource(toFetch, source.id);

      for (const [id, data] of batchResult) {
        this.resolver!.recordContentLocation(id, source.id);
        results.set(id, {
          data,
          sourceId: source.id,
          tier: source.tier,
          durationMs: performance.now() - startTime,
        });
        remaining.delete(id);
      }
    }

    return results;
  }

  /**
   * Batch fetch from a specific source.
   */
  private async batchFetchFromSource(
    ids: string[],
    sourceId: string
  ): Promise<Map<string, ContentNode>> {
    switch (sourceId) {
      case 'indexeddb':
        return await this.idbCache.getContentBatch(ids);

      case 'projection':
        if (!this.projectionApi.enabled) return new Map();
        return await firstValueFrom(this.projectionApi.batchGetContent(ids));

      case 'conductor':
        // Conductor no longer handles content batches - use projection instead
        // Conductor is now reserved for agent-centric data (identity, attestations, points)
        console.debug('[ContentResolver] Conductor skipped for batch content - use projection');
        return new Map();

      default:
        return new Map();
    }
  }

  /**
   * Cache content locally for future resolution.
   * Call after fetching from remote sources.
   */
  async cacheContent(content: ContentNode): Promise<void> {
    try {
      await this.idbCache.setContent(content);
      this.resolver?.recordContentLocation(content.id, 'indexeddb');
    } catch (err) {
      console.debug('[ContentResolver] Failed to cache content:', err);
    }
  }

  /**
   * Cache path locally for future resolution.
   */
  async cachePath(path: LearningPath): Promise<void> {
    try {
      await this.idbCache.setPath(path);
      this.resolver?.recordContentLocation(path.id, 'indexeddb');
    } catch (err) {
      console.debug('[ContentResolver] Failed to cache path:', err);
    }
  }

  /**
   * Invalidate cached content.
   */
  invalidateContent(contentId: string): void {
    this.resolver?.removeContentLocation(contentId, 'indexeddb');
    this.idbCache.removeContent(contentId).catch(() => {});
  }

  /**
   * Transform Holochain path to LearningPath model.
   */
  private transformHolochainPath(hcPath: HolochainPathWithSteps): LearningPath {
    let chapters: LearningPath['chapters'] | undefined;
    const metadata = hcPath.path.metadata ?? {};
    if (metadata && typeof metadata === 'object' && 'chapters' in metadata) {
      const metadataObj = metadata as Record<string, unknown>;
      if (Array.isArray(metadataObj['chapters'])) {
        chapters = metadataObj['chapters'] as LearningPath['chapters'];
      }
    }

    return {
      id: hcPath.path.id,
      version: hcPath.path.version,
      title: hcPath.path.title,
      description: hcPath.path.description,
      purpose: hcPath.path.purpose ?? '',
      createdBy: hcPath.path.createdBy,
      contributors: [],
      createdAt: hcPath.path.createdAt,
      updatedAt: hcPath.path.updatedAt,
      difficulty: hcPath.path.difficulty as LearningPath['difficulty'],
      estimatedDuration: hcPath.path.estimatedDuration ?? '',
      tags: hcPath.path.tags,
      visibility: hcPath.path.visibility as LearningPath['visibility'],
      chapters,
      steps: hcPath.steps.map((s, index) => ({
        order: s.step.orderIndex,
        stepType: (s.step.stepType || 'content') as 'content' | 'path' | 'external' | 'checkpoint',
        resourceId: s.step.resourceId,
        stepTitle: s.step.stepTitle ?? `Step ${index + 1}`,
        stepNarrative: s.step.stepNarrative ?? '',
        learningObjectives: [],
        optional: s.step.isOptional,
        completionCriteria: [],
      })),
    };
  }

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  /**
   * Check if WASM resolver is available.
   */
  async checkWasmAvailable(): Promise<boolean> {
    return isWasmResolverAvailable();
  }

  /**
   * Check if resolution result is an error.
   */
  isError(result: ResolutionResult | ResolutionError): result is ResolutionError {
    return 'error' in result;
  }

  /**
   * Get tier name for display.
   */
  getTierName(tier: SourceTier): string {
    switch (tier) {
      case SourceTier.Local:
        return 'Local';
      case SourceTier.Projection:
        return 'Projection';
      case SourceTier.Authoritative:
        return 'Authoritative';
      case SourceTier.External:
        return 'External';
      default:
        return 'Unknown';
    }
  }

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  private ensureReady(): void {
    if (!this.isReady) {
      throw new Error(
        '[ContentResolverService] Service not initialized. Call initialize() first.'
      );
    }
  }

  ngOnDestroy(): void {
    this.resolver?.dispose();
    this.stateSubject.complete();
  }
}
