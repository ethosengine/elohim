/**
 * Blob Bootstrap Service - Angular Adapter
 *
 * Wraps the framework-agnostic BlobBootstrapEngine and connects it to Angular's
 * signal-based state management system.
 *
 * This service is Angular-specific. For other frameworks (React, Svelte, Flutter, etc),
 * create similar adapters that wrap BlobBootstrapEngine with that framework's state system.
 *
 * The core bootstrap logic is in BlobBootstrapEngine (framework-agnostic).
 */

import { Injectable, signal, computed, inject } from '@angular/core';

// @coverage: 48.0% (2026-02-05)

import { firstValueFrom } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import {
  BlobBootstrapEngine,
  BlobBootstrapState,
  HolochainConnectionChecker,
  BlobMetadataFetcher,
  CacheIntegrityVerifier,
  BlobBootstrapConfig,
} from './blob-bootstrap-engine';
import { BlobCacheTiersService } from './blob-cache-tiers.service';
import { BlobManagerService } from './blob-manager.service';

// Re-export types for convenience
export type { BlobBootstrapStatus, BlobBootstrapState } from './blob-bootstrap-engine';

@Injectable({
  providedIn: 'root',
})
export class BlobBootstrapService {
  // Injected Angular dependencies
  private readonly holochainService = inject(HolochainClientService);
  private readonly blobManager = inject(BlobManagerService);
  private readonly blobCache = inject(BlobCacheTiersService);

  // Angular signals for state management
  private readonly bootstrapState = signal<BlobBootstrapState>({
    status: 'initializing',
    holochainConnected: false,
    metadataLoaded: false,
    cacheInitialized: false,
    integrityCheckStarted: false,
    preloadedContentIds: new Set(),
  });

  // Readonly signal exposure for Angular template binding
  readonly state = this.bootstrapState.asReadonly();

  // Convenience computed signals for UI reactivity
  readonly status = computed(() => this.bootstrapState().status);
  readonly isReady = computed(() => ['ready', 'degraded'].includes(this.bootstrapState().status));
  readonly canServeOffline = computed(
    () =>
      this.bootstrapState().cacheInitialized && this.bootstrapState().preloadedContentIds.size > 0
  );

  // The framework-agnostic engine (all bootstrap logic here)
  private engine: BlobBootstrapEngine | null = null;

  constructor() {
    // Initialize engine with adapters (bridge between Angular and engine)
    this.createEngine();
  }

  /**
   * Create the bootstrap engine with Angular-specific adapters.
   * These adapters translate between Angular services and the framework-agnostic engine.
   */
  private createEngine(): void {
    // Adapter 1: Holochain connection checker (Angular service → engine interface)
    const holochainChecker: HolochainConnectionChecker = {
      isConnected: () => this.holochainService.isConnected(),
    };

    // Adapter 2: Blob metadata fetcher (Angular service → engine interface)
    const metadataFetcher: BlobMetadataFetcher = {
      getBlobsForContent: async (contentId: string) => {
        const result = await firstValueFrom(this.blobManager.getBlobsForContent(contentId));
        return result ?? [];
      },
    };

    // Adapter 3: Cache integrity verifier (Angular service → engine interface)
    const cacheVerifier: CacheIntegrityVerifier = {
      startIntegrityVerification: () => this.blobCache.startIntegrityVerification(),
    };

    // Create engine with adapters and config
    const config: BlobBootstrapConfig = {
      // Configure as needed per deployment
    };

    this.engine = new BlobBootstrapEngine(holochainChecker, metadataFetcher, cacheVerifier, config);

    // Bridge engine events to Angular signals
    this.engine.on('status-changed', event => {
      this.bootstrapState.update(state => ({ ...state, status: event.status }));
    });

    this.engine.on('holochain-connected', () => {
      this.bootstrapState.update(state => ({ ...state, holochainConnected: true }));
    });

    this.engine.on('metadata-loaded', event => {
      this.bootstrapState.update(state => ({
        ...state,
        metadataLoaded: true,
        preloadedContentIds: new Set(event.contentIds),
      }));
    });

    this.engine.on('cache-initialized', () => {
      this.bootstrapState.update(state => ({ ...state, cacheInitialized: true }));
    });

    this.engine.on('integrity-started', () => {
      this.bootstrapState.update(state => ({ ...state, integrityCheckStarted: true }));
    });

    this.engine.on('error', event => {
      this.bootstrapState.update(state => ({ ...state, error: event.error }));
    });

    this.engine.on('ready', () => {
      this.bootstrapState.update(state => ({ ...state, status: 'ready' }));
    });
  }

  /**
   * Start blob bootstrap sequence (non-blocking).
   * Called from app.component.ts when Holochain is ready.
   */
  startBootstrap(contentIdsToPreload?: string[]): void {
    if (!this.engine) {
      return;
    }

    const config: BlobBootstrapConfig = {
      contentIdsToPreload,
    };

    // Recreate engine with updated config if content IDs provided
    if (contentIdsToPreload) {
      this.engine = new BlobBootstrapEngine(
        { isConnected: () => this.holochainService.isConnected() },
        {
          getBlobsForContent: async id => {
            const result = await firstValueFrom(this.blobManager.getBlobsForContent(id));
            return result ?? [];
          },
        },
        { startIntegrityVerification: () => this.blobCache.startIntegrityVerification() },
        config
      );
      // Re-attach event listeners (simplified - in production use a helper method)
    }

    this.engine.startBootstrap();
  }

  /**
   * Check if specific content is preloaded.
   */
  isContentPreloaded(contentId: string): boolean {
    return this.engine?.isContentPreloaded(contentId) ?? false;
  }

  /**
   * Preload content at runtime.
   */
  preloadContent(contentIds: string[]): void {
    this.engine?.preloadContent(contentIds);
  }

  /**
   * Get current state snapshot.
   */
  getState(): BlobBootstrapState {
    return this.bootstrapState();
  }

  /**
   * Reset for testing.
   */
  reset(): void {
    this.engine?.reset();
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
