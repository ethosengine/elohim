# Blob Bootstrap - Multi-Framework Adapter Pattern

The `BlobBootstrapEngine` is a framework-agnostic core library that can be wrapped by any framework-specific adapter. This document shows how to implement adapters for different frameworks.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│          BlobBootstrapEngine (Pure TypeScript)      │
│  - Orchestrates blob streaming bootstrap sequence   │
│  - Emits events, not observables                    │
│  - Zero framework dependencies                      │
└────────────────┬────────────────────────────────────┘
                 │
        ┌────────┴────────┬───────────┬─────────┐
        ▼                 ▼           ▼         ▼
   ┌─────────┐      ┌──────────┐ ┌─────────┐ ┌────────┐
   │ Angular │      │  React   │ │ Svelte  │ │ Flutter│
   │ Service │      │  Hooks   │ │ Store   │ │  Dart  │
   └─────────┘      └──────────┘ └─────────┘ └────────┘
```

## 1. Angular Service (blob-bootstrap.service.ts) ✅ IMPLEMENTED

Uses Angular signals for reactive state management.

```typescript
@Injectable({ providedIn: 'root' })
export class BlobBootstrapService {
  private engine: BlobBootstrapEngine;
  private bootstrapState = signal<BlobBootstrapState>({...});

  readonly status = computed(() => this.bootstrapState().status);
  readonly isReady = computed(() => ['ready', 'degraded'].includes(...));

  constructor() {
    this.engine = new BlobBootstrapEngine(
      { isConnected: () => this.holochainService.isConnected() },
      { getBlobsForContent: async (id) => {...} },
      { startIntegrityVerification: () => {...} }
    );

    // Bridge engine events to signals
    this.engine.on('status-changed', (e) => {
      this.bootstrapState.update(s => ({...s, status: e.status}));
    });
  }

  async startBootstrap(): Promise<void> {
    await this.engine.startBootstrap();
  }
}
```

**Usage in Angular components:**
```typescript
export class VideoPlayerComponent {
  private blobBootstrap = inject(BlobBootstrapService);

  // Reactive state in templates
  status = this.blobBootstrap.status;
  isReady = this.blobBootstrap.isReady;

  onPlayClick() {
    if (this.isReady()) {
      this.playVideo();
    } else {
      console.log(`Status: ${this.status()}`);
    }
  }
}
```

## 2. React Hook (use-blob-bootstrap.ts) - EXAMPLE

Uses React hooks for state management.

```typescript
// use-blob-bootstrap.ts
import { useState, useEffect, useCallback } from 'react';
import {
  BlobBootstrapEngine,
  BlobBootstrapState,
  BlobBootstrapStatus,
} from '@elohim/blob-bootstrap-engine';

interface UseBlobBootstrapResult {
  status: BlobBootstrapStatus;
  isReady: boolean;
  canServeOffline: boolean;
  error?: string;
  startBootstrap: (contentIds?: string[]) => Promise<void>;
  isContentPreloaded: (contentId: string) => boolean;
  preloadContent: (contentIds: string[]) => void;
}

export function useBlobBootstrap(
  holochainChecker: () => boolean,
  metadataFetcher: (contentId: string) => Promise<any[]>,
  cacheVerifier: () => void,
): UseBlobBootstrapResult {
  const [state, setState] = useState<BlobBootstrapState>({
    status: 'initializing',
    holochainConnected: false,
    metadataLoaded: false,
    cacheInitialized: false,
    integrityCheckStarted: false,
    preloadedContentIds: new Set(),
  });

  // Create engine once
  const engine = useRef<BlobBootstrapEngine | null>(null);

  useEffect(() => {
    if (!engine.current) {
      engine.current = new BlobBootstrapEngine(
        { isConnected: holochainChecker },
        { getBlobsForContent: metadataFetcher },
        { startIntegrityVerification: cacheVerifier }
      );

      // Bridge events to React state
      engine.current.on('status-changed', (e) => {
        setState(prev => ({...prev, status: e.status}));
      });

      engine.current.on('holochain-connected', () => {
        setState(prev => ({...prev, holochainConnected: true}));
      });

      engine.current.on('metadata-loaded', (e) => {
        setState(prev => ({
          ...prev,
          metadataLoaded: true,
          preloadedContentIds: new Set(e.contentIds),
        }));
      });

      engine.current.on('cache-initialized', () => {
        setState(prev => ({...prev, cacheInitialized: true}));
      });

      engine.current.on('error', (e) => {
        setState(prev => ({...prev, error: e.error}));
      });
    }

    return () => {
      // Cleanup if needed
    };
  }, [holochainChecker, metadataFetcher, cacheVerifier]);

  const startBootstrap = useCallback(async (contentIds?: string[]) => {
    if (engine.current) {
      await engine.current.startBootstrap();
    }
  }, []);

  const isContentPreloaded = useCallback((contentId: string) => {
    return engine.current?.isContentPreloaded(contentId) ?? false;
  }, []);

  const preloadContent = useCallback((contentIds: string[]) => {
    engine.current?.preloadContent(contentIds);
  }, []);

  return {
    status: state.status,
    isReady: ['ready', 'degraded'].includes(state.status),
    canServeOffline: state.cacheInitialized && state.preloadedContentIds.size > 0,
    error: state.error,
    startBootstrap,
    isContentPreloaded,
    preloadContent,
  };
}
```

**Usage in React components:**
```typescript
function VideoPlayer({ contentId, holochainService, blobService, cacheService }) {
  const bootstrap = useBlobBootstrap(
    () => holochainService.isConnected(),
    async (id) => blobService.getBlobsForContent(id),
    () => cacheService.startIntegrityVerification()
  );

  useEffect(() => {
    bootstrap.startBootstrap(['landing-video']);
  }, []);

  if (bootstrap.status === 'initializing') {
    return <div>Loading...</div>;
  }

  if (bootstrap.status === 'degraded') {
    return <div>⚠️ Offline Mode - Only cached content available</div>;
  }

  return bootstrap.isReady ? <VideoComponent /> : <Spinner />;
}
```

## 3. Svelte Store (blob-bootstrap.store.ts) - EXAMPLE

Uses Svelte reactive stores.

```typescript
// blob-bootstrap.store.ts
import { writable, derived, get } from 'svelte/store';
import {
  BlobBootstrapEngine,
  BlobBootstrapState,
  BlobBootstrapStatus,
} from '@elohim/blob-bootstrap-engine';

interface BlobBootstrapStores {
  status: Readable<BlobBootstrapStatus>;
  isReady: Readable<boolean>;
  canServeOffline: Readable<boolean>;
  error: Readable<string | undefined>;
  startBootstrap: (contentIds?: string[]) => Promise<void>;
  isContentPreloaded: (contentId: string) => boolean;
  preloadContent: (contentIds: string[]) => void;
}

export function createBlobBootstrapStore(
  holochainChecker: () => boolean,
  metadataFetcher: (contentId: string) => Promise<any[]>,
  cacheVerifier: () => void,
): BlobBootstrapStores {
  // Create writable store
  const stateStore = writable<BlobBootstrapState>({
    status: 'initializing',
    holochainConnected: false,
    metadataLoaded: false,
    cacheInitialized: false,
    integrityCheckStarted: false,
    preloadedContentIds: new Set(),
  });

  // Create engine
  const engine = new BlobBootstrapEngine(
    { isConnected: holochainChecker },
    { getBlobsForContent: metadataFetcher },
    { startIntegrityVerification: cacheVerifier }
  );

  // Bridge events to store
  engine.on('status-changed', (e) => {
    stateStore.update(s => ({...s, status: e.status}));
  });

  engine.on('holochain-connected', () => {
    stateStore.update(s => ({...s, holochainConnected: true}));
  });

  engine.on('metadata-loaded', (e) => {
    stateStore.update(s => ({
      ...s,
      metadataLoaded: true,
      preloadedContentIds: new Set(e.contentIds),
    }));
  });

  // Create derived stores
  const status = derived(stateStore, $state => $state.status);
  const isReady = derived(stateStore, $state =>
    ['ready', 'degraded'].includes($state.status)
  );
  const canServeOffline = derived(stateStore, $state =>
    $state.cacheInitialized && $state.preloadedContentIds.size > 0
  );
  const error = derived(stateStore, $state => $state.error);

  return {
    status,
    isReady,
    canServeOffline,
    error,
    startBootstrap: async (contentIds) => {
      await engine.startBootstrap();
    },
    isContentPreloaded: (contentId) =>
      engine.isContentPreloaded(contentId),
    preloadContent: (contentIds) =>
      engine.preloadContent(contentIds),
  };
}
```

**Usage in Svelte components:**
```svelte
<script>
  import { createBlobBootstrapStore } from './blob-bootstrap.store';

  const bootstrap = createBlobBootstrapStore(
    () => holochainService.isConnected(),
    (id) => blobService.getBlobsForContent(id),
    () => cacheService.startIntegrityVerification()
  );

  onMount(() => {
    bootstrap.startBootstrap(['landing-video']);
  });
</script>

{#if $bootstrap.status === 'initializing'}
  <div>Loading...</div>
{:else if $bootstrap.status === 'degraded'}
  <div>⚠️ Offline Mode</div>
{:else if $bootstrap.isReady}
  <VideoComponent />
{:else}
  <Spinner />
{/if}
```

## 4. Flutter Port (blob_bootstrap_engine.dart) - EXAMPLE

For Flutter/Dart applications:

```dart
// lib/services/blob_bootstrap/blob_bootstrap_engine.dart
import 'package:flutter/foundation.dart';

typedef IsConnectedChecker = bool Function();
typedef GetBlobsForContent = Future<List<dynamic>> Function(String contentId);
typedef StartIntegrityVerifier = void Function();

enum BlobBootstrapStatus {
  initializing,
  waitingHolochain,
  metadataLoading,
  ready,
  degraded,
}

class BlobBootstrapState {
  final BlobBootstrapStatus status;
  final bool holochainConnected;
  final bool metadataLoaded;
  final bool cacheInitialized;
  final bool integrityCheckStarted;
  final Set<String> preloadedContentIds;
  final String? error;

  BlobBootstrapState({
    required this.status,
    required this.holochainConnected,
    required this.metadataLoaded,
    required this.cacheInitialized,
    required this.integrityCheckStarted,
    required this.preloadedContentIds,
    this.error,
  });

  BlobBootstrapState copyWith({
    BlobBootstrapStatus? status,
    bool? holochainConnected,
    bool? metadataLoaded,
    bool? cacheInitialized,
    bool? integrityCheckStarted,
    Set<String>? preloadedContentIds,
    String? error,
  }) {
    return BlobBootstrapState(
      status: status ?? this.status,
      holochainConnected: holochainConnected ?? this.holochainConnected,
      metadataLoaded: metadataLoaded ?? this.metadataLoaded,
      cacheInitialized: cacheInitialized ?? this.cacheInitialized,
      integrityCheckStarted: integrityCheckStarted ?? this.integrityCheckStarted,
      preloadedContentIds: preloadedContentIds ?? this.preloadedContentIds,
      error: error ?? this.error,
    );
  }
}

class BlobBootstrapEngine extends ChangeNotifier {
  late BlobBootstrapState _state = BlobBootstrapState(
    status: BlobBootstrapStatus.initializing,
    holochainConnected: false,
    metadataLoaded: false,
    cacheInitialized: false,
    integrityCheckStarted: false,
    preloadedContentIds: {},
  );

  BlobBootstrapState get state => _state;

  final IsConnectedChecker _holochainChecker;
  final GetBlobsForContent _metadataFetcher;
  final StartIntegrityVerifier _cacheVerifier;
  final Map<String, dynamic>? config;

  bool _initializationStarted = false;

  BlobBootstrapEngine({
    required IsConnectedChecker holochainChecker,
    required GetBlobsForContent metadataFetcher,
    required StartIntegrityVerifier cacheVerifier,
    this.config,
  })  : _holochainChecker = holochainChecker,
        _metadataFetcher = metadataFetcher,
        _cacheVerifier = cacheVerifier;

  Future<void> startBootstrap({List<String>? contentIdsToPreload}) async {
    if (_initializationStarted) {
      debugPrint('[BlobBootstrapEngine] Bootstrap already started');
      return;
    }

    _initializationStarted = true;
    _updateState(_state.copyWith(status: BlobBootstrapStatus.initializing));

    // Run in background via compute or isolate
    unawaited(_runBootstrapSequence(contentIdsToPreload ?? []));
  }

  Future<void> _runBootstrapSequence(List<String> contentIds) async {
    // Initialize cache persistence
    try {
      // Use shared_preferences or local storage
      _updateState(_state.copyWith(cacheInitialized: true));
    } catch (e) {
      debugPrint('[BlobBootstrapEngine] Cache init failed: $e');
    }

    // Wait for Holochain
    final holochainReady =
        await _waitForHolochainConnection(Duration(seconds: 30));

    if (!holochainReady) {
      _updateState(
        _state.copyWith(
          status: BlobBootstrapStatus.degraded,
          holochainConnected: false,
        ),
      );
      _startBackgroundServices();
      return;
    }

    _updateState(
      _state.copyWith(
        status: BlobBootstrapStatus.metadataLoading,
        holochainConnected: true,
      ),
    );

    // Preload metadata
    if (contentIds.isNotEmpty) {
      try {
        final loaded = await _preloadMetadata(contentIds);
        _updateState(
          _state.copyWith(
            metadataLoaded: true,
            preloadedContentIds: loaded,
          ),
        );
      } catch (e) {
        debugPrint('[BlobBootstrapEngine] Metadata preload failed: $e');
      }
    }

    _startBackgroundServices();
    _updateState(_state.copyWith(status: BlobBootstrapStatus.ready));
  }

  Future<bool> _waitForHolochainConnection(Duration timeout) async {
    final startTime = DateTime.now();
    const pollInterval = Duration(milliseconds: 500);

    while (DateTime.now().difference(startTime) < timeout) {
      if (_holochainChecker()) {
        return true;
      }
      await Future.delayed(pollInterval);
    }

    return false;
  }

  Future<Set<String>> _preloadMetadata(List<String> contentIds) async {
    final loaded = <String>{};

    for (final contentId in contentIds) {
      try {
        final blobs = await _metadataFetcher(contentId);
        if (blobs.isNotEmpty) {
          loaded.add(contentId);
        }
      } catch (e) {
        debugPrint('[BlobBootstrapEngine] Failed to preload $contentId: $e');
      }
    }

    return loaded;
  }

  void _startBackgroundServices() {
    _cacheVerifier();
    _updateState(_state.copyWith(integrityCheckStarted: true));
  }

  bool isContentPreloaded(String contentId) =>
      _state.preloadedContentIds.contains(contentId);

  void preloadContent(List<String> contentIds) {
    if (_holochainChecker()) {
      unawaited(_preloadMetadata(contentIds).then((loaded) {
        _updateState(
          _state.copyWith(
            preloadedContentIds: {..._state.preloadedContentIds, ...loaded},
          ),
        );
      }));
    }
  }

  void _updateState(BlobBootstrapState newState) {
    _state = newState;
    notifyListeners();
  }

  void reset() {
    _initializationStarted = false;
    _updateState(
      BlobBootstrapState(
        status: BlobBootstrapStatus.initializing,
        holochainConnected: false,
        metadataLoaded: false,
        cacheInitialized: false,
        integrityCheckStarted: false,
        preloadedContentIds: {},
      ),
    );
  }
}
```

**Usage with Provider pattern in Flutter:**
```dart
// lib/providers/blob_bootstrap_provider.dart
import 'package:flutter_riverpod/flutter_riverpod.dart';

final blobBootstrapProvider =
    ChangeNotifierProvider((ref) {
  final holochain = ref.watch(holochainServiceProvider);
  final blobManager = ref.watch(blobManagerProvider);
  final cache = ref.watch(cacheProvider);

  return BlobBootstrapEngine(
    holochainChecker: () => holochain.isConnected,
    metadataFetcher: (id) => blobManager.getBlobsForContent(id),
    cacheVerifier: () => cache.startIntegrityVerification(),
  )..startBootstrap(contentIdsToPreload: ['landing-video']);
});
```

## Summary

**Key Benefits of This Architecture**:

1. **Code Reuse**: The core bootstrap logic (`BlobBootstrapEngine`) is identical across all frameworks
2. **Framework Independence**: Engine has zero framework dependencies
3. **Easy Porting**: To support a new framework, just create a new adapter (200-300 LOC)
4. **Testability**: Engine can be unit tested in isolation
5. **Type Safety**: Each framework adapter maintains full type safety
6. **Performance**: Event-based architecture avoids unnecessary state updates
7. **Maintenance**: Bug fixes in core logic automatically benefit all frameworks

## Implementation Checklist

- [x] Angular Service (`BlobBootstrapService`)
- [ ] React Hook (`useBlobBootstrap`)
- [ ] Svelte Store (`createBlobBootstrapStore`)
- [ ] Flutter Engine (`BlobBootstrapEngine` in Dart)
- [ ] Vue 3 Composable
- [ ] SolidJS Hook
- [ ] Web Components

Each implementation follows the same pattern: wrap `BlobBootstrapEngine` with framework-specific state management and adapt the service interfaces.
