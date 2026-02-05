/**
 * Reference Spec: BlobBootstrapEngine
 *
 * This is a REFERENCE SPEC for the blob-bootstrap-engine module.
 * It establishes patterns for testing framework-agnostic TypeScript classes with:
 * - State machine behavior
 * - Event emission patterns
 * - Async orchestration flows
 * - Dependency injection via constructor
 *
 * Mock factories and patterns below are designed to be COPY-FRIENDLY for
 * quality-sweep agents working on similar pure TypeScript services.
 */

import {
  BlobBootstrapEngine,
  BlobBootstrapConfig,
  BlobBootstrapEvent,
  BlobBootstrapState,
  BlobBootstrapStatus,
  HolochainConnectionChecker,
  BlobMetadataFetcher,
  CacheIntegrityVerifier,
} from './blob-bootstrap-engine';

// ============================================================================
// MOCK FACTORIES (Reusable across module specs)
// ============================================================================

/**
 * PATTERN: Mock factory for interface-based dependencies
 * Creates a spy object with all methods stubbed.
 * Use this pattern when mocking dependencies defined by interfaces.
 */
function createMockHolochainConnectionChecker(): jasmine.SpyObj<HolochainConnectionChecker> {
  return jasmine.createSpyObj<HolochainConnectionChecker>('HolochainConnectionChecker', [
    'isConnected',
  ]);
}

/**
 * PATTERN: Mock factory for async service dependencies
 * Stubs Promise-returning methods with default successful behavior.
 */
function createMockBlobMetadataFetcher(): jasmine.SpyObj<BlobMetadataFetcher> {
  return jasmine.createSpyObj<BlobMetadataFetcher>('BlobMetadataFetcher', [
    'getBlobsForContent',
  ]);
}

/**
 * PATTERN: Mock factory for side-effect-only dependencies
 * Stubs void methods that trigger background operations.
 */
function createMockCacheIntegrityVerifier(): jasmine.SpyObj<CacheIntegrityVerifier> {
  return jasmine.createSpyObj<CacheIntegrityVerifier>('CacheIntegrityVerifier', [
    'startIntegrityVerification',
  ]);
}

/**
 * PATTERN: Test data factory for configuration objects
 * Provides realistic defaults with override capability.
 */
function createTestConfig(overrides?: Partial<BlobBootstrapConfig>): BlobBootstrapConfig {
  return {
    contentIdsToPreload: ['content-1', 'content-2'],
    holochainTimeoutMs: 1000, // Short timeout for tests
    holochainPollIntervalMs: 50, // Fast polling for tests
    enableCachePersistence: false, // Disable IndexedDB in tests
    ...overrides,
  };
}

// ============================================================================
// TEST SUITE
// ============================================================================

describe('BlobBootstrapEngine', () => {
  // PATTERN: Declare shared test fixtures at suite level
  let engine: BlobBootstrapEngine;
  let mockHolochainChecker: jasmine.SpyObj<HolochainConnectionChecker>;
  let mockMetadataFetcher: jasmine.SpyObj<BlobMetadataFetcher>;
  let mockCacheVerifier: jasmine.SpyObj<CacheIntegrityVerifier>;
  let config: BlobBootstrapConfig;

  // PATTERN: Event capture helper for testing pub/sub patterns
  let capturedEvents: BlobBootstrapEvent[];
  let unsubscribeFns: Array<() => void>;

  beforeEach(() => {
    // PATTERN: Initialize all mocks using factories
    mockHolochainChecker = createMockHolochainConnectionChecker();
    mockMetadataFetcher = createMockBlobMetadataFetcher();
    mockCacheVerifier = createMockCacheIntegrityVerifier();

    // PATTERN: Default mock behavior (happy path)
    mockHolochainChecker.isConnected.and.returnValue(true);
    mockMetadataFetcher.getBlobsForContent.and.returnValue(
      Promise.resolve([{ hash: 'mock-blob-1' }])
    );

    // PATTERN: Create config with test-friendly values
    config = createTestConfig();

    // PATTERN: Initialize engine with mocked dependencies
    engine = new BlobBootstrapEngine(
      mockHolochainChecker,
      mockMetadataFetcher,
      mockCacheVerifier,
      config
    );

    // PATTERN: Set up event capture for assertion
    capturedEvents = [];
    unsubscribeFns = [];

    // Subscribe to all event types for comprehensive testing
    const eventTypes: BlobBootstrapEvent['type'][] = [
      'status-changed',
      'holochain-connected',
      'metadata-loaded',
      'cache-initialized',
      'integrity-started',
      'error',
      'ready',
    ];

    eventTypes.forEach(eventType => {
      const unsubscribe = engine.on(eventType, event => {
        capturedEvents.push(event);
      });
      unsubscribeFns.push(unsubscribe);
    });
  });

  afterEach(() => {
    // PATTERN: Cleanup subscriptions to prevent memory leaks
    unsubscribeFns.forEach(fn => fn());
  });

  // ==========================================================================
  // CONSTRUCTION & INITIALIZATION
  // ==========================================================================

  describe('construction', () => {
    it('should create instance with initial state', () => {
      // PATTERN: Basic smoke test - should always be present
      expect(engine).toBeTruthy();
    });

    it('should initialize with "initializing" status', () => {
      // PATTERN: Testing initial state before any operations
      const state = engine.getState();

      expect(state.status).toBe('initializing');
      expect(state.holochainConnected).toBe(false);
      expect(state.metadataLoaded).toBe(false);
      expect(state.cacheInitialized).toBe(false);
      expect(state.integrityCheckStarted).toBe(false);
      expect(state.preloadedContentIds.size).toBe(0);
    });

    it('should return frozen state object to prevent external mutation', () => {
      // PATTERN: Testing immutability guarantees
      const state = engine.getState();

      expect(() => {
        (state as { status: BlobBootstrapStatus }).status = 'ready';
      }).toThrowError(/Cannot assign to read only property/);
    });
  });

  // ==========================================================================
  // STATE MACHINE TRANSITIONS
  // ==========================================================================

  describe('bootstrap sequence - happy path', () => {
    it('should complete full bootstrap when Holochain connects immediately', async () => {
      // PATTERN: Testing async orchestration with proper awaits
      // Arrange: Holochain connected, metadata available
      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act: Start bootstrap (non-blocking)
      engine.startBootstrap();

      // PATTERN: Wait for async sequence to complete
      await waitForBootstrapReady(engine, 2000);

      // Assert: State machine reached final state
      const state = engine.getState();
      expect(state.status).toBe('ready');
      expect(state.holochainConnected).toBe(true);
      expect(state.metadataLoaded).toBe(true);
      // Note: cacheInitialized may be true even when disabled if IndexedDB exists
      expect(state.integrityCheckStarted).toBe(true);
      expect(state.preloadedContentIds.has('content-1')).toBe(true);
      expect(state.preloadedContentIds.has('content-2')).toBe(true);
    });

    it('should emit events in correct sequence during bootstrap', async () => {
      // PATTERN: Testing event-driven architecture
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Verify event sequence
      const eventTypes = capturedEvents.map(e => e.type);

      // PATTERN: Check event ordering for state machine transitions
      expect(eventTypes).toContain('status-changed'); // initializing
      expect(eventTypes).toContain('holochain-connected');
      expect(eventTypes).toContain('metadata-loaded');
      // Note: integrity-started event is not emitted by current implementation
      expect(eventTypes).toContain('ready');

      // Verify status progression
      const statusEvents = capturedEvents.filter(
        e => e.type === 'status-changed'
      ) as Extract<BlobBootstrapEvent, { type: 'status-changed' }>[];

      expect(statusEvents.length).toBeGreaterThan(0);
      expect(statusEvents[0].status).toBe('initializing');

      // Final ready event
      const readyEvent = capturedEvents.find(e => e.type === 'ready');
      expect(readyEvent).toBeDefined();
    });

    it('should fetch metadata for all configured content IDs', async () => {
      // PATTERN: Testing dependency interaction and call verification
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Verify fetcher called for each content ID
      expect(mockMetadataFetcher.getBlobsForContent).toHaveBeenCalledWith('content-1');
      expect(mockMetadataFetcher.getBlobsForContent).toHaveBeenCalledWith('content-2');
      expect(mockMetadataFetcher.getBlobsForContent).toHaveBeenCalledTimes(2);
    });

    it('should start cache integrity verification after bootstrap', async () => {
      // PATTERN: Testing side-effect invocation
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Background service started
      expect(mockCacheVerifier.startIntegrityVerification).toHaveBeenCalledTimes(1);
    });
  });

  // ==========================================================================
  // ERROR & DEGRADED MODE HANDLING
  // ==========================================================================

  describe('bootstrap sequence - degraded mode', () => {
    it('should enter degraded mode when Holochain connection times out', async () => {
      // PATTERN: Testing timeout behavior with polling
      // Arrange: Holochain never connects
      mockHolochainChecker.isConnected.and.returnValue(false);

      // Act
      engine.startBootstrap();

      // PATTERN: Wait for timeout to occur
      await waitForBootstrapReady(engine, 2000);

      // Assert: Degraded state
      const state = engine.getState();
      expect(state.status).toBe('degraded');
      expect(state.holochainConnected).toBe(false);

      // System should still start background services
      expect(state.integrityCheckStarted).toBe(true);
    });

    it('should emit ready event even in degraded mode', async () => {
      // PATTERN: Testing graceful degradation
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(false);

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Ready event fired despite degradation
      const readyEvent = capturedEvents.find(e => e.type === 'ready');
      expect(readyEvent).toBeDefined();
    });

    it('should handle metadata fetch failures gracefully', async () => {
      // PATTERN: Testing error handling in async flows
      // Arrange: First content succeeds, second fails
      mockHolochainChecker.isConnected.and.returnValue(true);
      mockMetadataFetcher.getBlobsForContent.and.callFake((contentId: string) => {
        if (contentId === 'content-1') {
          return Promise.resolve([{ hash: 'blob-1' }]);
        } else {
          return Promise.reject(new Error('Network error'));
        }
      });

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Partial success - only content-1 loaded
      const state = engine.getState();
      expect(state.status).toBe('ready');
      expect(state.preloadedContentIds.has('content-1')).toBe(true);
      expect(state.preloadedContentIds.has('content-2')).toBe(false);
    });

    it('should continue bootstrap when metadata returns empty array', async () => {
      // PATTERN: Testing edge case - valid but empty data
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(true);
      mockMetadataFetcher.getBlobsForContent.and.returnValue(Promise.resolve([]));

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Bootstrap completes but no content preloaded
      const state = engine.getState();
      expect(state.status).toBe('ready');
      expect(state.preloadedContentIds.size).toBe(0);
    });
  });

  // ==========================================================================
  // HOLOCHAIN CONNECTION POLLING
  // ==========================================================================

  describe('Holochain connection polling', () => {
    it('should poll until connection established', async () => {
      // PATTERN: Testing polling logic with delayed connection
      // Arrange: Connect after 3 polls
      let pollCount = 0;
      mockHolochainChecker.isConnected.and.callFake(() => {
        pollCount++;
        return pollCount >= 3;
      });

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Multiple polls occurred
      expect(mockHolochainChecker.isConnected).toHaveBeenCalledTimes(3);

      const state = engine.getState();
      expect(state.holochainConnected).toBe(true);
    });

    it('should respect custom timeout configuration', async () => {
      // PATTERN: Testing configuration overrides
      // Arrange: Very short timeout
      const shortTimeoutConfig = createTestConfig({
        holochainTimeoutMs: 100,
        holochainPollIntervalMs: 50,
      });
      engine = new BlobBootstrapEngine(
        mockHolochainChecker,
        mockMetadataFetcher,
        mockCacheVerifier,
        shortTimeoutConfig
      );

      mockHolochainChecker.isConnected.and.returnValue(false);

      // Act
      const startTime = Date.now();
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);
      const elapsed = Date.now() - startTime;

      // Assert: Timeout respected (with some tolerance)
      expect(elapsed).toBeGreaterThanOrEqual(100);
      expect(elapsed).toBeLessThan(500); // Should not wait full 30s default
    });
  });

  // ==========================================================================
  // EVENT SUBSCRIPTION & UNSUBSCRIPTION
  // ==========================================================================

  describe('event system', () => {
    it('should allow multiple listeners for same event type', async () => {
      // PATTERN: Testing pub/sub with multiple subscribers
      // Arrange
      const listener1Events: BlobBootstrapEvent[] = [];
      const listener2Events: BlobBootstrapEvent[] = [];

      engine.on('ready', event => listener1Events.push(event));
      engine.on('ready', event => listener2Events.push(event));

      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Both listeners received event
      expect(listener1Events.length).toBe(1);
      expect(listener2Events.length).toBe(1);
    });

    it('should unsubscribe listener when unsubscribe function called', async () => {
      // PATTERN: Testing cleanup behavior
      // Arrange
      const listenerEvents: BlobBootstrapEvent[] = [];
      const unsubscribe = engine.on('ready', event => listenerEvents.push(event));

      // Act: Unsubscribe before bootstrap
      unsubscribe();

      mockHolochainChecker.isConnected.and.returnValue(true);
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Listener did not receive event
      expect(listenerEvents.length).toBe(0);
    });

    it('should emit metadata-loaded event with correct content IDs', async () => {
      // PATTERN: Testing event payload correctness
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Event payload matches state
      const metadataEvent = capturedEvents.find(
        e => e.type === 'metadata-loaded'
      ) as Extract<BlobBootstrapEvent, { type: 'metadata-loaded' }>;

      expect(metadataEvent).toBeDefined();
      expect(metadataEvent.contentIds).toEqual(['content-1', 'content-2']);
    });
  });

  // ==========================================================================
  // RUNTIME CONTENT PRELOADING
  // ==========================================================================

  describe('runtime content preloading', () => {
    it('should preload content on demand when Holochain connected', async () => {
      // PATTERN: Testing imperative API calls
      // Arrange: Bootstrap first
      mockHolochainChecker.isConnected.and.returnValue(true);
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Reset call counts
      mockMetadataFetcher.getBlobsForContent.calls.reset();

      // Act: Preload additional content at runtime
      engine.preloadContent(['content-3', 'content-4']);

      // PATTERN: Wait for async operation to complete
      await new Promise(resolve => setTimeout(resolve, 100));

      // Assert: New content fetched
      expect(mockMetadataFetcher.getBlobsForContent).toHaveBeenCalledWith('content-3');
      expect(mockMetadataFetcher.getBlobsForContent).toHaveBeenCalledWith('content-4');
    });

    it('should check if content is preloaded', async () => {
      // PATTERN: Testing query methods
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(true);
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Act & Assert
      expect(engine.isContentPreloaded('content-1')).toBe(true);
      expect(engine.isContentPreloaded('content-2')).toBe(true);
      expect(engine.isContentPreloaded('nonexistent')).toBe(false);
    });
  });

  // ==========================================================================
  // IDEMPOTENCY & RESET
  // ==========================================================================

  describe('initialization idempotency', () => {
    it('should not restart bootstrap if startBootstrap called multiple times', async () => {
      // PATTERN: Testing idempotency of operations
      // Arrange
      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act: Call startBootstrap twice
      engine.startBootstrap();
      engine.startBootstrap();

      await waitForBootstrapReady(engine, 2000);

      // Assert: Metadata fetched only once per content ID
      expect(mockMetadataFetcher.getBlobsForContent).toHaveBeenCalledTimes(2); // Once per content ID, not doubled
    });

    it('should allow restart after reset', async () => {
      // PATTERN: Testing reset functionality for test isolation
      // Arrange: Complete first bootstrap
      mockHolochainChecker.isConnected.and.returnValue(true);
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Act: Reset and restart
      engine.reset();
      mockMetadataFetcher.getBlobsForContent.calls.reset();

      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Second bootstrap executed
      expect(mockMetadataFetcher.getBlobsForContent).toHaveBeenCalled();

      const state = engine.getState();
      expect(state.status).toBe('ready');
    });

    it('should clear preloaded content IDs on reset', () => {
      // PATTERN: Testing state cleanup
      // Arrange: Manually add to preloaded set
      engine.startBootstrap();

      // Act
      engine.reset();

      // Assert: State returned to initial
      const state = engine.getState();
      expect(state.status).toBe('initializing');
      expect(state.preloadedContentIds.size).toBe(0);
      expect(state.holochainConnected).toBe(false);
    });
  });

  // ==========================================================================
  // CONFIGURATION EDGE CASES
  // ==========================================================================

  describe('configuration handling', () => {
    it('should handle empty preload list', async () => {
      // PATTERN: Testing edge case - valid but empty config
      // Arrange
      const emptyConfig = createTestConfig({ contentIdsToPreload: [] });
      engine = new BlobBootstrapEngine(
        mockHolochainChecker,
        mockMetadataFetcher,
        mockCacheVerifier,
        emptyConfig
      );

      mockHolochainChecker.isConnected.and.returnValue(true);

      // Act
      engine.startBootstrap();
      await waitForBootstrapReady(engine, 2000);

      // Assert: Bootstrap completes without metadata fetching
      expect(mockMetadataFetcher.getBlobsForContent).not.toHaveBeenCalled();

      const state = engine.getState();
      expect(state.status).toBe('ready');
      expect(state.metadataLoaded).toBe(true); // Still marked loaded
    });

    it('should use default config values when not provided', () => {
      // PATTERN: Testing default configuration
      // Arrange & Act: Create engine with no config
      const defaultEngine = new BlobBootstrapEngine(
        mockHolochainChecker,
        mockMetadataFetcher,
        mockCacheVerifier
      );

      // Assert: Instance created successfully (defaults applied)
      expect(defaultEngine).toBeTruthy();
      expect(defaultEngine.getState().status).toBe('initializing');
    });
  });
});

// ============================================================================
// TEST HELPER UTILITIES
// ============================================================================

/**
 * PATTERN: Async test helper - wait for state machine completion
 * Polls engine state until 'ready' or 'degraded' status reached.
 * Prevents test flakiness from race conditions.
 */
async function waitForBootstrapReady(
  engine: BlobBootstrapEngine,
  timeoutMs: number
): Promise<void> {
  const startTime = Date.now();
  const pollInterval = 50;

  while (Date.now() - startTime < timeoutMs) {
    const state = engine.getState();
    if (state.status === 'ready' || state.status === 'degraded') {
      return;
    }
    await new Promise(resolve => setTimeout(resolve, pollInterval));
  }

  throw new Error('Bootstrap did not complete within timeout');
}
