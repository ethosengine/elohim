/**
 * BlobBootstrapEngine - Framework-Agnostic Core
 *
 * Pure TypeScript/JavaScript implementation of blob streaming bootstrap logic.
 * Zero framework dependencies - works with Angular, React, Svelte, Flutter, vanilla JS, etc.
 *
 * This is the CORE ENGINE that orchestrates:
 * - Holochain connection waiting
 * - Cache persistence initialization (IndexedDB)
 * - Blob metadata pre-fetching
 * - Cache integrity verification startup
 * - Status tracking via events
 *
 * Framework adapters wrap this engine and connect it to their specific
 * state management systems (signals, hooks, stores, providers, etc).
 */

/**
 * Bootstrap status for UI feedback
 */
export type BlobBootstrapStatus =
  | 'initializing'
  | 'waiting-holochain'
  | 'metadata-loading'
  | 'ready'
  | 'degraded';

/**
 * Unambiguous state of bootstrap process
 */
export interface BlobBootstrapState {
  status: BlobBootstrapStatus;
  holochainConnected: boolean;
  metadataLoaded: boolean;
  cacheInitialized: boolean;
  integrityCheckStarted: boolean;
  preloadedContentIds: Set<string>;
  error?: string;
}

/**
 * Interface for Holochain connection checker.
 * Implemented differently in each framework (Angular service, React hook, Svelte store, Dart class).
 */
export interface HolochainConnectionChecker {
  isConnected(): boolean;
}

/**
 * Interface for blob metadata fetcher.
 * Implemented differently in each framework.
 */
export interface BlobMetadataFetcher {
  getBlobsForContent(contentId: string): Promise<unknown[]>;
}

/**
 * Interface for cache integrity verification.
 * Implemented differently in each framework.
 */
export interface CacheIntegrityVerifier {
  startIntegrityVerification(): void;
}

/**
 * Events emitted by the bootstrap engine.
 * Frameworks can listen to these and update their state management.
 */
export type BlobBootstrapEvent =
  | { type: 'status-changed'; status: BlobBootstrapStatus }
  | { type: 'holochain-connected' }
  | { type: 'metadata-loaded'; contentIds: string[] }
  | { type: 'cache-initialized' }
  | { type: 'integrity-started' }
  | { type: 'error'; error: string }
  | { type: 'ready' };

/**
 * Configuration for bootstrap behavior
 */
export interface BlobBootstrapConfig {
  /** Content IDs to pre-fetch on startup */
  contentIdsToPreload?: string[];

  /** Timeout waiting for Holochain connection (milliseconds) */
  holochainTimeoutMs?: number;

  /** Polling interval for Holochain connection check (milliseconds) */
  holochainPollIntervalMs?: number;

  /** Enable IndexedDB persistence */
  enableCachePersistence?: boolean;
}

/**
 * BlobBootstrapEngine - Framework-agnostic bootstrap orchestrator
 *
 * This is a pure class with no framework dependencies.
 * It emits events that frameworks can listen to and wire into their state systems.
 *
 * Usage:
 * ```typescript
 * // 1. Create engine with dependencies
 * const engine = new BlobBootstrapEngine(
 *   holochainChecker,
 *   metadataFetcher,
 *   cacheVerifier,
 *   { contentIdsToPreload: ['video-1', 'video-2'] }
 * );
 *
 * // 2. Listen to events
 * engine.on('status-changed', (event) => {
 *   updateUI(event.status);
 * });
 *
 * // 3. Start bootstrap
 * await engine.startBootstrap();
 * ```
 */
export class BlobBootstrapEngine {
  // Internal state
  private state: BlobBootstrapState = {
    status: 'initializing',
    holochainConnected: false,
    metadataLoaded: false,
    cacheInitialized: false,
    integrityCheckStarted: false,
    preloadedContentIds: new Set(),
  };

  // Event listeners (simple pub/sub)
  private readonly listeners = new Map<
    BlobBootstrapEvent['type'],
    Set<(event: BlobBootstrapEvent) => void>
  >();

  // Track initialization to prevent multiple runs
  private initializationStarted = false;

  constructor(
    private readonly holochainChecker: HolochainConnectionChecker,
    private readonly metadataFetcher: BlobMetadataFetcher,
    private readonly cacheVerifier: CacheIntegrityVerifier,
    private readonly config: BlobBootstrapConfig = {}
  ) {}

  /**
   * Start the bootstrap process (non-blocking).
   * Call this once and it runs the full sequence in background.
   */
  startBootstrap(): void {
    if (this.initializationStarted) {
      return;
    }

    this.initializationStarted = true;
    this.updateState({ status: 'initializing' });

    // Run in background - don't await
    this.runBootstrapSequence().catch(_error => {
      const message = _error instanceof Error ? _error.message : 'Unknown error';
      this.updateState({ status: 'degraded', error: message });
      this.emitEvent({ type: 'error', error: message });
    });
  }

  /**
   * Main bootstrap sequence (runs in background).
   */
  private async runBootstrapSequence(): Promise<void> {
    // Step 1: Initialize cache persistence
    try {
      await this.initializeCachePersistence();
      this.updateState({ cacheInitialized: true });
      this.emitEvent({ type: 'cache-initialized' });
    } catch {
      // Cache persistence initialization failed - will continue without persistence
    }

    // Step 2: Wait for Holochain connection
    const timeoutMs = this.config.holochainTimeoutMs ?? 30000;
    const pollIntervalMs = this.config.holochainPollIntervalMs ?? 500;

    const holochainReady = await this.waitForHolochainConnection(timeoutMs, pollIntervalMs);

    if (!holochainReady) {
      this.updateState({ status: 'degraded', holochainConnected: false });
      this.startBackgroundServices();
      this.emitEvent({ type: 'ready' });
      return;
    }

    this.updateState({ status: 'metadata-loading', holochainConnected: true });
    this.emitEvent({ type: 'holochain-connected' });

    // Step 3: Pre-fetch metadata
    const contentIds = this.config.contentIdsToPreload ?? [];
    if (contentIds.length > 0) {
      try {
        const loaded = await this.preloadMetadata(contentIds);
        this.updateState({ metadataLoaded: true, preloadedContentIds: loaded });
        this.emitEvent({ type: 'metadata-loaded', contentIds: Array.from(loaded) });
      } catch {
        // Metadata preload failed - will continue without preloading
      }
    } else {
      this.updateState({ metadataLoaded: true });
      this.emitEvent({ type: 'metadata-loaded', contentIds: [] });
    }

    // Step 4: Start background services
    this.startBackgroundServices();

    // Step 5: Mark ready
    this.updateState({ status: 'ready' });
    this.emitEvent({ type: 'ready' });
  }

  /**
   * Wait for Holochain connection with timeout and polling.
   */
  private async waitForHolochainConnection(
    timeoutMs: number,
    pollIntervalMs: number
  ): Promise<boolean> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      if (this.holochainChecker.isConnected()) {
        return true;
      }
      // Wait before next poll
      await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
    }

    return false;
  }

  /**
   * Initialize IndexedDB cache persistence.
   * Works in any JavaScript environment with IndexedDB support.
   */
  private async initializeCachePersistence(): Promise<void> {
    const enableCache = this.config.enableCachePersistence ?? true;
    if (!enableCache) {
      return;
    }

    if (typeof indexedDB === 'undefined') {
      return;
    }

    return new Promise((resolve, reject) => {
      const dbRequest = indexedDB.open('elohim-blob-cache', 1);

      dbRequest.onupgradeneeded = event => {
        const db = (event.target as IDBOpenDBRequest).result;
        if (!db.objectStoreNames.contains('blobs')) {
          db.createObjectStore('blobs', { keyPath: 'hash' });
        }
        if (!db.objectStoreNames.contains('metadata')) {
          db.createObjectStore('metadata', { keyPath: 'hash' });
        }
      };

      dbRequest.onsuccess = () => {
        resolve();
      };

      dbRequest.onerror = () => {
        reject(new Error('Failed to open IndexedDB'));
      };
    });
  }

  /**
   * Pre-fetch metadata for content IDs.
   */
  private async preloadMetadata(contentIds: string[]): Promise<Set<string>> {
    const loaded = new Set<string>();

    for (const contentId of contentIds) {
      try {
        const blobs = await this.metadataFetcher.getBlobsForContent(contentId);
        if (blobs && blobs.length > 0) {
          loaded.add(contentId);
        }
      } catch {
        // Blob fetch for individual content ID failed - continue with others
      }
    }

    return loaded;
  }

  /**
   * Start background services (cache verification, etc).
   */
  private startBackgroundServices(): void {
    this.cacheVerifier.startIntegrityVerification();
    this.updateState({ integrityCheckStarted: true });
  }

  /**
   * Check if content metadata is preloaded.
   */
  isContentPreloaded(contentId: string): boolean {
    return this.state.preloadedContentIds.has(contentId);
  }

  /**
   * Register content to preload at runtime.
   */
  preloadContent(contentIds: string[]): void {
    if (this.holochainChecker.isConnected()) {
      void this.preloadMetadata(contentIds)
        .then(loaded => {
          this.state.preloadedContentIds.forEach(id => loaded.add(id));
          this.updateState({ preloadedContentIds: loaded });
        })
        .catch(() => {
          // Runtime metadata preload failed - gracefully continue without updating state
        });
    }
  }

  /**
   * Get current state (read-only).
   */
  getState(): Readonly<BlobBootstrapState> {
    return Object.freeze({ ...this.state });
  }

  /**
   * Listen to bootstrap events.
   * Returns unsubscribe function.
   */
  on<T extends BlobBootstrapEvent['type']>(
    eventType: T,
    handler: (event: Extract<BlobBootstrapEvent, { type: T }>) => void
  ): () => void {
    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, new Set());
    }

    const handlers = this.listeners.get(eventType)!;
    handlers.add(handler as (event: BlobBootstrapEvent) => void);

    // Return unsubscribe function
    return () => handlers.delete(handler as (event: BlobBootstrapEvent) => void);
  }

  /**
   * Reset engine for testing.
   */
  reset(): void {
    this.initializationStarted = false;
    this.state = {
      status: 'initializing',
      holochainConnected: false,
      metadataLoaded: false,
      cacheInitialized: false,
      integrityCheckStarted: false,
      preloadedContentIds: new Set(),
    };
  }

  // Private helpers

  private updateState(updates: Partial<BlobBootstrapState>): void {
    this.state = { ...this.state, ...updates };
    if (updates.status) {
      this.emitEvent({ type: 'status-changed', status: updates.status });
    }
  }

  private emitEvent(event: BlobBootstrapEvent): void {
    const handlers = this.listeners.get(event.type);
    if (handlers) {
      handlers.forEach(handler => handler(event));
    }
  }
}
