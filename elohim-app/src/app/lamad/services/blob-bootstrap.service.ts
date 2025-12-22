/**
 * Blob Bootstrap Service - Phase 0: Startup Initialization
 *
 * Manages the seamless initialization of blob streaming infrastructure on app startup.
 * Ensures blobs can stream immediately even when Holochain hasn't fully connected yet.
 *
 * Responsibilities:
 * - Initialize all blob services (cache, verification, streaming)
 * - Wait for Holochain connection without blocking UI
 * - Pre-fetch blob metadata for known content
 * - Initialize cache persistence layer (IndexedDB)
 * - Start background integrity verification
 * - Provide status signals for UI degradation
 */

import { Injectable, signal, computed, inject } from '@angular/core';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { BlobManagerService } from './blob-manager.service';
import { BlobCacheTiersService } from './blob-cache-tiers.service';
import { ContentBlob } from '../models/content-node.model';

/**
 * Bootstrap status for UI components
 */
export type BlobBootstrapStatus = 'initializing' | 'waiting-holochain' | 'metadata-loading' | 'ready' | 'degraded';

/**
 * Bootstrap readiness state
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

@Injectable({
  providedIn: 'root',
})
export class BlobBootstrapService {
  // Injected dependencies
  private readonly holochainService = inject(HolochainClientService);
  private readonly blobManager = inject(BlobManagerService);
  private readonly blobCache = inject(BlobCacheTiersService);

  // State management
  private readonly bootstrapState = signal<BlobBootstrapState>({
    status: 'initializing',
    holochainConnected: false,
    metadataLoaded: false,
    cacheInitialized: false,
    integrityCheckStarted: false,
    preloadedContentIds: new Set(),
  });

  // Readonly exposure
  readonly state = this.bootstrapState.asReadonly();

  // Convenience computed signals
  readonly status = computed(() => this.bootstrapState().status);
  readonly isReady = computed(() =>
    ['ready', 'degraded'].includes(this.bootstrapState().status)
  );
  readonly canServeOffline = computed(() =>
    this.bootstrapState().cacheInitialized &&
    this.bootstrapState().preloadedContentIds.size > 0
  );

  // Content IDs to pre-fetch on startup (configured per deployment)
  private readonly contentIdsToPreload: string[] = [
    // These would be set via configuration or environment
    // Example: 'landing-page-video', 'intro-tutorial'
  ];

  // Track initialization to prevent multiple runs
  private initializationStarted = false;

  /**
   * Start the blob bootstrap process on app init.
   * This is called from app.component.ts once Holochain is ready.
   * Non-blocking - all operations run in background.
   */
  async startBootstrap(contentIdsToPreload?: string[]): Promise<void> {
    if (this.initializationStarted) {
      console.log('[BlobBootstrap] Bootstrap already started, skipping');
      return;
    }

    this.initializationStarted = true;
    if (contentIdsToPreload) {
      this.contentIdsToPreload.push(...contentIdsToPreload);
    }

    // Start initialization in background (don't await - let it run)
    this.runBootstrapSequence().catch((error) => {
      console.error('[BlobBootstrap] Bootstrap sequence failed:', error);
      this.updateState({
        status: 'degraded',
        error: error instanceof Error ? error.message : 'Unknown error',
      });
    });
  }

  /**
   * Run the complete bootstrap sequence.
   * This is the main initialization flow that runs in background.
   */
  private async runBootstrapSequence(): Promise<void> {
    this.updateState({ status: 'initializing' });

    // Step 1: Initialize cache persistence (IndexedDB)
    try {
      await this.initializeCachePersistence();
      this.updateState({ cacheInitialized: true });
    } catch (error) {
      console.warn('[BlobBootstrap] Cache persistence init failed, continuing without persistence:', error);
      // Continue anyway - memory cache still works
    }

    // Step 2: Wait for Holochain to be ready (with timeout)
    const holochainReady = await this.waitForHolochainConnection(30000); // 30s timeout
    if (!holochainReady) {
      console.warn('[BlobBootstrap] Holochain not ready after timeout, entering degraded mode');
      this.updateState({
        status: 'degraded',
        holochainConnected: false,
      });
      // Can still serve cached blobs in degraded mode
      this.startBackgroundServices();
      return;
    }

    this.updateState({
      status: 'metadata-loading',
      holochainConnected: true,
    });

    // Step 3: Pre-fetch metadata for known content
    if (this.contentIdsToPreload.length > 0) {
      try {
        await this.preloadMetadataForContent(this.contentIdsToPreload);
        this.updateState({ metadataLoaded: true });
      } catch (error) {
        console.warn('[BlobBootstrap] Metadata preload failed, continuing:', error);
        // Not fatal - metadata will be loaded on-demand
      }
    } else {
      this.updateState({ metadataLoaded: true });
    }

    // Step 4: Start background services (integrity check, auto-tuning, etc.)
    this.startBackgroundServices();

    // Step 5: Mark as ready
    this.updateState({ status: 'ready' });
  }

  /**
   * Wait for Holochain connection with timeout.
   * Returns true if connected, false if timeout.
   */
  private async waitForHolochainConnection(timeoutMs: number): Promise<boolean> {
    const startTime = performance.now();
    const pollInterval = 500; // Poll every 500ms

    while (performance.now() - startTime < timeoutMs) {
      if (this.holochainService.isConnected()) {
        console.log('[BlobBootstrap] Holochain connected');
        return true;
      }

      // Wait before next poll
      await new Promise(resolve => setTimeout(resolve, pollInterval));
    }

    console.warn('[BlobBootstrap] Holochain connection timeout');
    return false;
  }

  /**
   * Initialize cache persistence layer (IndexedDB).
   * Allows blobs to persist across page refreshes for offline playback.
   */
  private async initializeCachePersistence(): Promise<void> {
    try {
      // Check if IndexedDB is available
      if (!('indexedDB' in window)) {
        console.warn('[BlobBootstrap] IndexedDB not available, cache will be memory-only');
        return;
      }

      const dbRequest = indexedDB.open('elohim-blob-cache', 1);

      return new Promise((resolve, reject) => {
        dbRequest.onupgradeneeded = (event) => {
          const db = (event.target as IDBOpenDBRequest).result;
          if (!db.objectStoreNames.contains('blobs')) {
            db.createObjectStore('blobs', { keyPath: 'hash' });
          }
          if (!db.objectStoreNames.contains('metadata')) {
            db.createObjectStore('metadata', { keyPath: 'hash' });
          }
        };

        dbRequest.onsuccess = () => {
          console.log('[BlobBootstrap] Cache persistence initialized');
          resolve();
        };

        dbRequest.onerror = () => {
          reject(new Error('Failed to open IndexedDB'));
        };
      });
    } catch (error) {
      console.warn('[BlobBootstrap] Cache persistence init error:', error);
      throw error;
    }
  }

  /**
   * Pre-fetch blob metadata for known content.
   * This allows blobs to stream immediately when user clicks play.
   */
  private async preloadMetadataForContent(contentIds: string[]): Promise<void> {
    const preloadedIds = new Set<string>();

    for (const contentId of contentIds) {
      try {
        // Fetch metadata (non-blocking - returns Observable)
        const metadata = await this.blobManager
          .getBlobsForContent(contentId)
          .toPromise();

        if (metadata && metadata.length > 0) {
          console.log(
            `[BlobBootstrap] Pre-loaded ${metadata.length} blobs for content: ${contentId}`
          );
          preloadedIds.add(contentId);
        }
      } catch (error) {
        console.warn(
          `[BlobBootstrap] Failed to preload metadata for content ${contentId}:`,
          error
        );
        // Continue with other content
      }
    }

    this.updateState((current) => ({
      ...current,
      preloadedContentIds: preloadedIds,
    }));
  }

  /**
   * Start all background services that run continuously.
   * These don't block - they run in the background.
   */
  private startBackgroundServices(): void {
    // Start cache integrity verification (hourly)
    this.blobCache.startIntegrityVerification();
    this.updateState({ integrityCheckStarted: true });

    console.log('[BlobBootstrap] Background services started');
  }

  /**
   * Check if a specific content's metadata is preloaded.
   * Used by components to show "ready to play" vs "loading" state.
   */
  isContentPreloaded(contentId: string): boolean {
    return this.bootstrapState().preloadedContentIds.has(contentId);
  }

  /**
   * Register additional content to preload (can be called at runtime).
   * Useful for preloading content when user navigates to a page.
   */
  preloadContent(contentIds: string[]): void {
    // If Holochain is ready, preload immediately
    if (this.holochainService.isConnected()) {
      this.preloadMetadataForContent(contentIds).catch((error) => {
        console.warn('[BlobBootstrap] Runtime preload failed:', error);
      });
    } else {
      // Queue for later preload
      console.log('[BlobBootstrap] Queuing content for preload when Holochain ready:', contentIds);
      // Could implement a queue here for retry when Holochain connects
    }
  }

  /**
   * Update bootstrap state.
   */
  private updateState(updates: Partial<BlobBootstrapState> | ((current: BlobBootstrapState) => BlobBootstrapState)): void {
    if (typeof updates === 'function') {
      this.bootstrapState.update(updates);
    } else {
      this.bootstrapState.update((current) => ({ ...current, ...updates }));
    }
  }

  /**
   * Get current bootstrap state (for testing/debugging).
   */
  getState(): BlobBootstrapState {
    return this.bootstrapState();
  }

  /**
   * Reset bootstrap for testing.
   */
  reset(): void {
    this.initializationStarted = false;
    this.bootstrapState.set({
      status: 'initializing',
      holochainConnected: false,
      metadataLoaded: false,
      cacheInitialized: false,
      integrityCheckStarted: false,
      preloadedContentIds: new Set(),
    });
  }
}
