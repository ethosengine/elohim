# Holochain Graceful Degradation UI & Strategy

## Overview

The Elohim app now features a comprehensive graceful degradation system for handling Holochain unavailability. When the Holochain conductor is unreachable or offline, the app continues to function in a degraded mode, providing offline read access to cached content while queuing write operations for sync when connection is restored.

**Key Achievement**: Users can keep working offline with full read access to cached content, and their write operations are automatically queued for sync.

---

## Architecture

### Three-Layer Resilience Strategy

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Connection Monitoring                              │
│ HolochainClientService                                      │
│ - Tracks connection state (connected/connecting/error)      │
│ - Exposes state via signals for reactive updates             │
│ - Handles auto-reconnection with exponential backoff         │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: User Feedback                                      │
│ HolochainAvailabilityUiComponent                            │
│ - Displays connection status banner                         │
│ - Shows feature availability in offline mode                │
│ - Provides retry/sync buttons                               │
│ - Lists queued operations                                   │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: Offline Support                                    │
│ ├─ OfflineOperationQueueService                             │
│ │  - Queues write operations while offline                  │
│ │  - Auto-syncs when connection restored                    │
│ │  - Persistent queue (IndexedDB)                           │
│ │                                                             │
│ └─ HolochainCacheService                                    │
│    - Caches read results (L1 memory + L2 IndexedDB)         │
│    - 10MB memory cache, 50MB IndexedDB                      │
│    - TTL-based expiration                                   │
│    - Preload capability                                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Components & Services

### 1. HolochainAvailabilityUiComponent

**Purpose**: Single unified UI for all connection-related messaging

**Location**: `/elohim-app/src/app/elohim/components/holochain-availability-ui/`

**States**:
- **Connected (Green)**: All features available
- **Connecting (Yellow)**: Features degraded, retry button visible
- **Error (Red)**: Error message, retry button, troubleshooting guide
- **Offline (Gray)**: Cached content only, queued operations visible

**Features**:
```typescript
// Connection state tracking
readonly isConnected: Signal<boolean>
readonly isConnecting: Signal<boolean>
readonly isError: Signal<boolean>
readonly isOffline: Signal<boolean>

// Operation queue visibility
readonly queuedOperations: Signal<number>
readonly hasQueuedOperations: Signal<boolean>

// Actions
async retryConnection(): Promise<void>
async syncQueuedOperations(): Promise<void>
dismissBanner(): void
```

**Display Elements**:
1. **Status Banner**: Main connection indicator with icon and message
2. **Degradation Warning**: Explains what's unavailable
3. **Action Buttons**: Retry, Sync, Dismiss
4. **Expandable Details**: Feature availability lists, troubleshooting guide
5. **Queue Info**: Shows pending operations count

**Integration**:
```html
<!-- Add to app.component.html at the top -->
<app-holochain-availability-ui></app-holochain-availability-ui>
<router-outlet></router-outlet>
```

---

### 2. OfflineOperationQueueService

**Purpose**: Queue and replay write operations performed while offline

**Location**: `/elohim-app/src/app/elohim/services/offline-operation-queue.service.ts`

**Key API**:
```typescript
// Enqueue operations
enqueue(operation: OfflineOperation): string
dequeue(operationId: string): void
dismissOperation(operationId: string): void

// Sync queue
async syncAll(): Promise<{succeeded: number; failed: number}>
async syncOperation(operationId: string): Promise<boolean>

// Queue info
getQueue(): OfflineOperation[]
getQueueSize(): number
getStats(): {size, totalRetries, averageRetries, oldestOperation, lastSync}

// Event callbacks
onQueueChanged(callback: (queue: OfflineOperation[]) => void): void
onSyncComplete(callback: (succeeded, failed) => void): void
```

**Operation Interface**:
```typescript
interface OfflineOperation {
  id: string;           // Unique identifier
  timestamp: number;    // When queued
  type: 'zome_call' | 'write' | 'create' | 'update' | 'delete';
  zomeName?: string;    // Zome to call on sync
  fnName?: string;      // Function to call
  payload?: any;        // Zome payload
  retryCount: number;   // Current retry count
  maxRetries: number;   // Max retries (default 3)
  description?: string; // User-friendly description
}
```

**Usage Example**:
```typescript
// In a service that makes write operations
constructor(private queue: OfflineOperationQueueService) {}

async createContent(content: any): Promise<string | null> {
  const result = await this.holochain.callZome({
    zomeName: 'content',
    fnName: 'create_content',
    payload: content
  });

  if (!result.success) {
    // Queue for retry if offline
    this.queue.enqueue({
      type: 'create',
      zomeName: 'content',
      fnName: 'create_content',
      payload: content,
      maxRetries: 5,
      description: 'Create content'
    });
    return null;
  }

  return result.data;
}
```

**Retry Strategy**:
- Automatic exponential backoff: 1s, 2s, 4s, 8s
- Max retries: 3 (configurable per operation)
- Persistent: Queue survives page reloads via IndexedDB
- Auto-sync: Syncs automatically when connection restored
- Manual sync: User can click "Sync" button

---

### 3. HolochainCacheService

**Purpose**: Multi-tier caching for offline read access

**Location**: `/elohim-app/src/app/elohim/services/holochain-cache.service.ts`

**Key API**:
```typescript
// Get/Set/Delete
async get<T>(key: string): Promise<T | null>
async set<T>(key: string, value: T, ttlMs?: number, metadata?: Record<string, any>): Promise<void>
async delete(key: string): Promise<void>
async clear(): Promise<void>

// Batch operations
async preload<T>(items: Array<{key, value, ttlMs}>): Promise<void>

// Queries
query(predicate: (entry) => boolean): CacheEntry[]
getByTag(tag: string): CacheEntry[]
getByDomain(domain: string): CacheEntry[]

// Monitoring
getStats(): CacheStats
readonly hitRate: Signal<number>
```

**Storage Tiers**:

| Tier | Storage | Size | Speed | Persistence | Use Case |
|------|---------|------|-------|-------------|----------|
| L1 | Memory | 10MB | 5-10ms | Lost on reload | Hot data |
| L2 | IndexedDB | 50MB | 50-100ms | Survives reload | Persistent cache |

**TTL Strategy**:
```typescript
// No TTL - cache forever
await cache.set('key', value);

// 1 hour TTL
await cache.set('key', value, 60 * 60 * 1000);

// 7 day TTL
await cache.set('key', value, 7 * 24 * 60 * 60 * 1000);

// Expired entries auto-removed on access
const value = await cache.get('key'); // Returns null if expired
```

**Metadata for Organization**:
```typescript
// Cache with metadata for later queries
await cache.set('content-123', contentData, 24*60*60*1000, {
  domain: 'elohim-protocol',
  epic: 'governance',
  contentType: 'video',
  tags: ['governance', 'stewardship'],
  reachLevel: 7
});

// Query by tag
const allGovernance = cache.getByTag('governance');

// Query by domain
const elohimContent = cache.getByDomain('elohim-protocol');
```

**Hit Rate Monitoring**:
```typescript
// Component or service
readonly hitRate = this.cache.hitRate; // Signal<number>
readonly hitRatePercent = computed(() => `${this.cache.hitRate().toFixed(1)}%`);

// In template
<div>Cache hit rate: {{ hitRatePercent() }}</div>

// Get full stats
const stats = this.cache.getStats();
console.log(`Total entries: ${stats.totalEntries}`);
console.log(`Size: ${stats.totalSizeBytes / 1024 / 1024}MB`);
console.log(`Hit rate: ${stats.hitRate.toFixed(1)}%`);
```

**Preload Strategy**:
```typescript
// On app startup, preload critical content
constructor(private cache: HolochainCacheService) {
  this.cache.preload([
    { key: 'app-config', value: appConfig, ttlMs: 24*60*60*1000 },
    { key: 'user-profile', value: userProfile, ttlMs: 60*60*1000 },
    { key: 'learning-paths', value: paths, ttlMs: 7*24*60*60*1000 }
  ]);
}
```

---

## Integration Points

### 1. In AppComponent

Update `app.component.ts` to use graceful degradation:

```typescript
import { HolochainAvailabilityUiComponent } from './elohim/components/...';
import { OfflineOperationQueueService } from './elohim/services/offline-operation-queue.service';
import { HolochainCacheService } from './elohim/services/holochain-cache.service';

@Component({
  imports: [HolochainAvailabilityUiComponent, RouterOutlet, ...],
  // ...
})
export class AppComponent implements OnInit {
  private readonly operationQueue = inject(OfflineOperationQueueService);
  private readonly holochainCache = inject(HolochainCacheService);

  ngOnInit(): void {
    // Existing initialization...

    // Setup operation queue auto-sync when connection restored
    this.holochainService.isConnected.subscribe(isConnected => {
      if (isConnected) {
        this.operationQueue.syncAll().catch(err => {
          console.error('Queue sync failed:', err);
        });
      }
    });
  }
}
```

### 2. In Data Services

Update services to use queue and cache:

```typescript
export class HolochainContentService {
  constructor(
    private holochain: HolochainClientService,
    private queue: OfflineOperationQueueService,
    private cache: HolochainCacheService
  ) {}

  async getContent(id: string): Promise<Content | null> {
    // Check cache first
    const cached = await this.cache.get<Content>(`content-${id}`);
    if (cached) {
      return cached;
    }

    // Try from Holochain
    const result = await this.holochain.callZome({
      zomeName: 'content',
      fnName: 'get_content',
      payload: { id }
    });

    if (result.success && result.data) {
      // Cache for offline use
      await this.cache.set(`content-${id}`, result.data, 24*60*60*1000, {
        domain: 'elohim-protocol',
        contentType: 'content'
      });
      return result.data;
    }

    // Return cached if available, even if stale
    return await this.cache.get<Content>(`content-${id}`);
  }

  async createContent(content: any): Promise<string | null> {
    const result = await this.holochain.callZome({
      zomeName: 'content',
      fnName: 'create_content',
      payload: content
    });

    if (!result.success) {
      // Queue for sync
      this.queue.enqueue({
        type: 'create',
        zomeName: 'content',
        fnName: 'create_content',
        payload: content,
        maxRetries: 5,
        description: `Create content: ${content.title}`
      });
      return null;
    }

    return result.data;
  }
}
```

### 3. In Components

Components can react to connection status:

```typescript
import { HolochainClientService } from './services/holochain-client.service';
import { OfflineOperationQueueService } from './services/offline-operation-queue.service';

@Component({
  // ...
})
export class ContentEditorComponent {
  private readonly holochain = inject(HolochainClientService);
  private readonly queue = inject(OfflineOperationQueueService);

  readonly isConnected = this.holochain.isConnected;
  readonly queuedOps = computed(() => this.queue.getQueueSize());

  async saveContent(): Promise<void> {
    // Show warning if not connected
    if (!this.isConnected()) {
      console.log('Offline - saving will be queued for sync');
    }

    // Service handles queue automatically
    await this.contentService.updateContent(this.contentId, this.formValue);
  }

  // Disable certain features when offline
  readonly canPublish = computed(() => this.isConnected());
  readonly canShare = computed(() => this.isConnected());
  readonly canTrack = computed(() => this.isConnected());
}
```

---

## User Experience Flow

### Scenario 1: User Goes Offline While Editing

```
Time  Event                          UI State                 Action
────────────────────────────────────────────────────────────────────────
T0    User creates content           Connected (green)        All features available
T+5s  Network disconnects            Connecting (yellow)      Brief message shown
T+10s Connection times out           Offline (gray)           Banner expanded
      - Create button disabled
      - Queue indicator shows "0 ops"
      - "Read cached content only" message

T+15s User clicks "Save" anyway      Operation queued         "1 operation pending"
      System queues the operation    No error to user         Shows "Sync" button

T+30s Network restored              Connected (green)         Auto-sync triggered
      Queue auto-syncs              "Syncing..."             Progress indication
      Success: Content created      Queue cleared            "All synced ✓"
```

### Scenario 2: Intermittent Connectivity

```
Network Status     Banner State    User Actions           Backend
─────────────────────────────────────────────────────────────────────
Connected          Green ✓         Can do everything      All calls succeed

Disconnected       Gray ⊗          Queue operations       Operations saved locally

Reconnects         Yellow ⟳        Auto-sync starts       Queued ops sent
                                   "Syncing 3 ops..."

1 op fails         Red ⚠            Manual retry available Shows failure reason
                                    "1 failed, 2 succeeded"

All synced         Green ✓          Queue cleared          Back to normal
```

---

## Monitoring & Observability

### Queue Monitoring

```typescript
// Get queue size for badge
const queueSize = this.queue.getQueueSize();

// Listen for queue changes
this.queue.onQueueChanged((queue) => {
  console.log('Queue updated:', queue.length, 'operations');
});

// Listen for sync completion
this.queue.onSyncComplete((succeeded, failed) => {
  console.log(`Sync: ${succeeded} succeeded, ${failed} failed`);
});

// Get detailed stats
const stats = this.queue.getStats();
console.log({
  queueSize: stats.size,
  totalRetries: stats.totalRetries,
  averageRetries: stats.averageRetries,
  oldestOpSeconds: stats.oldestOperation,
  lastSyncTime: new Date(stats.lastSync)
});
```

### Cache Monitoring

```typescript
// Get cache hit rate
const hitRate = this.cache.hitRate(); // Signal<number>

// Get full statistics
const stats = this.cache.getStats();
console.log({
  entries: stats.totalEntries,
  sizeBytes: stats.totalSizeBytes,
  sizeMB: stats.totalSizeBytes / 1024 / 1024,
  hitRate: stats.hitRate.toFixed(1) + '%',
  oldestSeconds: stats.oldestEntry,
  newestSeconds: stats.newestEntry
});

// Monitor cache by domain
const elohimContent = this.cache.getByDomain('elohim-protocol');
console.log(`${elohimContent.length} Elohim Protocol items cached`);
```

### Health Dashboard

Create a simple health component:

```typescript
@Component({
  template: `
    <div class="health-dashboard">
      <div>Connection: {{ holochainClient.state() }}</div>
      <div>Cache: {{ cacheStats().totalEntries }} entries ({{ cacheSizePercent() }}%)</div>
      <div>Queue: {{ queueSize() }} ops, {{ avgRetries() }} avg retries</div>
      <div>Hit Rate: {{ hitRate() | number:'1.1-1' }}%</div>
    </div>
  `
})
export class HealthDashboardComponent {
  readonly cacheStats = this.cache.getStats;
  readonly cacheSizePercent = computed(() => {
    const stats = this.cacheStats();
    return Math.round(stats.totalSizeBytes / (50 * 1024 * 1024) * 100);
  });
  readonly queueSize = this.queue.getQueueSize;
  readonly avgRetries = () => this.queue.getStats().averageRetries;
  readonly hitRate = this.cache.hitRate;
}
```

---

## Performance Considerations

### Memory Management

**Cache Eviction**:
- L1 (memory): 10MB limit, LRU eviction when exceeded
- L2 (IndexedDB): 50MB limit, manual cleanup via `cache.clear()`

**Optimization Tips**:
```typescript
// 1. Set appropriate TTLs to avoid stale data
await cache.set(key, value, 60*60*1000); // 1 hour TTL

// 2. Preload critical content on app startup
const paths = await api.getCommonPaths();
await cache.preload(paths.map(p => ({
  key: `path-${p.id}`,
  value: p,
  ttlMs: 7*24*60*60*1000 // 7 days
})));

// 3. Monitor cache size and hit rate
const stats = cache.getStats();
if (stats.hitRate < 0.5) {
  // Adjust preload strategy
  console.log('Low hit rate - consider preloading more content');
}

// 4. Clear old entries periodically
if (stats.oldestEntry > 30 * 24 * 60 * 60) { // 30 days
  await cache.clear(); // Or selective cleanup
}
```

### Sync Optimization

**Batch Sync**:
```typescript
// Queue groups operations
queue.enqueue({ type: 'create', /* ... */ });
queue.enqueue({ type: 'create', /* ... */ });
queue.enqueue({ type: 'create', /* ... */ });

// Single sync call handles all
await queue.syncAll(); // All 3 sent efficiently
```

**Selective Sync**:
```typescript
// Sync only high-priority operations
const queue = queue.getQueue();
const highPriority = queue.filter(op => op.metadata?.priority === 'high');

for (const op of highPriority) {
  await queue.syncOperation(op.id);
}
```

---

## Troubleshooting

### "Operations stuck in queue"

**Causes**:
- Connection never restored
- Zome calls timeout or fail
- Payload is invalid after app update

**Solutions**:
```typescript
// Check queue state
const queue = queue.getQueue();
console.log('Stuck operations:', queue);

// Manually dismiss operations user doesn't need
queue.dismissOperation(operationId);

// Or clear entire queue for restart
queue.clearQueue();
```

### "Cache hit rate is low"

**Causes**:
- Cache TTLs too short
- Not preloading content
- Users accessing new content not in cache

**Solutions**:
```typescript
// Increase TTL for stable content
await cache.set(key, value, 30*24*60*60*1000); // 30 days

// Preload content groups
const paths = await api.getPaths();
await cache.preload(paths);

// Monitor which content is accessed most
const stats = cache.getStats();
if (stats.hitRate < 0.5) {
  // Adjust preload strategy
}
```

### "IndexedDB quota exceeded"

**Causes**:
- L2 cache full (50MB limit)
- Other apps using IndexedDB
- Accumulated data over time

**Solutions**:
```typescript
// Clear L2 explicitly
await cache.clear();

// Or set more aggressive TTLs
await cache.set(key, value, 1*24*60*60*1000); // 1 day instead of 7 days

// Monitor quota
const stats = cache.getStats();
if (stats.totalSizeBytes > 40 * 1024 * 1024) { // 80% of 50MB
  console.warn('Cache near quota - clearing old entries');
  await cache.clear();
}
```

---

## Testing

### Unit Tests

```typescript
describe('OfflineOperationQueueService', () => {
  it('should queue operations when offline', () => {
    const opId = service.enqueue({
      type: 'create',
      zomeName: 'content',
      fnName: 'create',
      payload: {},
      maxRetries: 3
    });
    expect(opId).toBeDefined();
    expect(service.getQueueSize()).toBe(1);
  });

  it('should sync operations when connected', async () => {
    // Queue operation
    service.enqueue({ /* ... */ });

    // Mock successful Holochain response
    spyOn(holochain, 'callZome').and.returnValue(
      Promise.resolve({ success: true, data: 'result' })
    );

    // Sync
    const result = await service.syncAll();
    expect(result.succeeded).toBe(1);
    expect(service.getQueueSize()).toBe(0);
  });
});

describe('HolochainCacheService', () => {
  it('should cache and retrieve values', async () => {
    await service.set('key', { data: 'value' });
    const result = await service.get('key');
    expect(result).toEqual({ data: 'value' });
  });

  it('should respect TTL expiration', async () => {
    await service.set('key', 'value', 100); // 100ms TTL
    expect(await service.get('key')).toBe('value');

    await new Promise(resolve => setTimeout(resolve, 150));
    expect(await service.get('key')).toBeNull();
  });

  it('should track hit rate', async () => {
    await service.set('key', 'value');
    await service.get('key'); // hit
    await service.get('missing'); // miss
    const hitRate = service.hitRate();
    expect(hitRate).toBe(50);
  });
});
```

### E2E Tests

```typescript
// Test offline scenario
describe('Offline workflow', () => {
  it('should queue operations and sync when connection restored', async () => {
    // Simulate offline
    network.goOffline();
    expect(client.isConnected()).toBe(false);

    // User creates content
    const opId = await contentService.createContent({ title: 'Test' });
    expect(opId).toBeNull(); // Failed due to offline
    expect(queue.getQueueSize()).toBe(1);

    // Restore connection
    network.goOnline();
    await client.connect();

    // Auto-sync or manual sync
    const result = await queue.syncAll();
    expect(result.succeeded).toBe(1);
    expect(queue.getQueueSize()).toBe(0);
  });
});
```

---

## Migration Guide

### For Existing Services

**Before**:
```typescript
async createContent(data: any): Promise<any> {
  const result = await this.holochain.callZome({...});
  if (!result.success) {
    throw new Error(result.error);
  }
  return result.data;
}
```

**After**:
```typescript
async createContent(data: any): Promise<any> {
  const result = await this.holochain.callZome({...});
  if (!result.success) {
    // Queue for retry instead of throwing
    this.queue.enqueue({
      type: 'create',
      zomeName: 'content',
      fnName: 'create_content',
      payload: data,
      description: `Create content: ${data.title}`
    });
    return null; // Return null instead of throwing
  }
  return result.data;
}
```

### For Components

**Before**:
```typescript
async saveContent(): Promise<void> {
  try {
    await this.contentService.updateContent(this.id, this.form.value);
  } catch (err) {
    this.error = err.message;
  }
}
```

**After**:
```typescript
async saveContent(): Promise<void> {
  // Service handles offline queueing automatically
  const result = await this.contentService.updateContent(this.id, this.form.value);
  if (!result) {
    // Show that it's queued for sync
    this.showMessage('Saved locally, will sync when connection restored');
  }
}
```

---

## Summary

The graceful degradation system provides:

✅ **User-Friendly Feedback**: Clear status messages via HolochainAvailabilityUiComponent
✅ **Offline Read Access**: Cache layer with L1 memory + L2 IndexedDB
✅ **Write Operation Queuing**: Automatic retry with exponential backoff
✅ **Seamless Integration**: Works with existing services via dependency injection
✅ **Monitoring**: Hit rates, queue stats, cache usage visibility
✅ **Production Ready**: Persistent queue, TTL management, LRU eviction

**For Users**:
- See clear status when offline
- Can read cached content anytime
- Write operations queue automatically
- Get notifications when sync completes
- Troubleshooting guide available via UI

**For Developers**:
- Drop-in components and services
- Simple API: queue.enqueue(), cache.get/set()
- Event callbacks for custom handling
- Full type safety with TypeScript
- Comprehensive test examples

