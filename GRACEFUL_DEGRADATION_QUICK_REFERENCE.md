# Graceful Degradation - Quick Reference

**TL;DR**: Drop in the UI component, inject the services, your app now works offline with queued operations and cached reads.

---

## Installation (5 minutes)

### 1. Add UI to AppComponent

```typescript
// app.component.ts
import { HolochainAvailabilityUiComponent } from './elohim/components/holochain-availability-ui/holochain-availability-ui.component';

@Component({
  imports: [HolochainAvailabilityUiComponent, RouterOutlet, ...],
  // ...
})
export class AppComponent implements OnInit {
  private readonly queue = inject(OfflineOperationQueueService);
  private readonly holochain = inject(HolochainClientService);

  ngOnInit() {
    // Auto-sync when connected
    this.holochain.isConnected.subscribe(isConnected => {
      if (isConnected && this.queue.getQueueSize() > 0) {
        this.queue.syncAll();
      }
    });
  }
}
```

```html
<!-- app.component.html -->
<app-holochain-availability-ui></app-holochain-availability-ui>
<router-outlet></router-outlet>
```

---

## Usage Patterns

### Pattern 1: Queue Write Operations

```typescript
constructor(
  private holochain: HolochainClientService,
  private queue: OfflineOperationQueueService
) {}

async createContent(data: any): Promise<string | null> {
  const result = await this.holochain.callZome({
    zomeName: 'content',
    fnName: 'create_content',
    payload: data
  });

  if (!result.success) {
    // Queue for retry
    this.queue.enqueue({
      type: 'create',
      zomeName: 'content',
      fnName: 'create_content',
      payload: data,
      description: `Create: ${data.title}`
    });
    return null;
  }

  return result.data;
}
```

### Pattern 2: Cache Read Results

```typescript
constructor(
  private holochain: HolochainClientService,
  private cache: HolochainCacheService
) {}

async getContent(id: string): Promise<any | null> {
  // Try cache first
  const cached = await this.cache.get(`content-${id}`);
  if (cached) return cached;

  // Try Holochain
  const result = await this.holochain.callZome({
    zomeName: 'content',
    fnName: 'get_content',
    payload: { id }
  });

  if (result.success) {
    // Cache for offline use
    await this.cache.set(`content-${id}`, result.data, 24*60*60*1000);
    return result.data;
  }

  return null;
}
```

### Pattern 3: Disable Features When Offline

```typescript
export class MyComponent {
  private readonly holochain = inject(HolochainClientService);

  // In template: [disabled]="!canPublish()"
  readonly canPublish = computed(() => this.holochain.isConnected());
  readonly canShare = computed(() => this.holochain.isConnected());
  readonly isOffline = computed(() => !this.holochain.isConnected());
}
```

---

## Common Operations

### Queue Operations

```typescript
// Enqueue
const opId = queue.enqueue({
  type: 'create',
  zomeName: 'content',
  fnName: 'create_content',
  payload: { title: 'My Content' },
  maxRetries: 5,
  description: 'Create content'
});

// Check queue
console.log(queue.getQueueSize()); // e.g., 3
console.log(queue.getQueue());     // Full array

// Sync
await queue.syncAll();             // Sync all
await queue.syncOperation(opId);   // Sync one

// Clear
queue.dismissOperation(opId);      // Remove one
queue.clearQueue();                // Remove all

// Stats
const stats = queue.getStats();
console.log(stats.size, stats.totalRetries, stats.averageRetries);
```

### Cache Operations

```typescript
// Set
await cache.set('key', { data: 'value' });
await cache.set('key', data, 24*60*60*1000); // With 1 day TTL
await cache.set('key', data, ttl, {          // With metadata
  domain: 'elohim-protocol',
  tags: ['tag1', 'tag2']
});

// Get
const value = await cache.get('key');
const valuWithType = await cache.get<MyType>('key');

// Query
const items = cache.query(e => e.value.title.includes('search'));
const tagged = cache.getByTag('governance');
const domain = cache.getByDomain('elohim-protocol');

// Delete
await cache.delete('key');
await cache.clear(); // Clear all

// Monitor
const stats = cache.getStats();
console.log(stats.totalEntries);     // Number of items
console.log(stats.totalSizeBytes);   // Size in bytes
console.log(stats.hitRate);          // Hit rate %

// Preload
await cache.preload([
  { key: 'k1', value: v1, ttlMs: 86400000 },
  { key: 'k2', value: v2, ttlMs: 86400000 }
]);
```

### Check Connection Status

```typescript
// In service
readonly isConnected = this.holochain.isConnected; // Signal<boolean>
readonly connectionState = this.holochain.state;    // Signal<string>
readonly hasError = this.holochain.error;           // Signal<string | undefined>

// In template
<div *ngIf="!isConnected()">Offline</div>
<div [class.error]="connectionState() === 'error'"></div>
```

---

## UI Component Features

### What It Shows

```
Connected:  ✓ Connected to Holochain
            All features available

Connecting: ⟳ Connecting to Holochain...
            Some features may be temporarily unavailable

Error:      ⚠ Connection Error: (error message)
            [Retry] button, troubleshooting guide

Offline:    ⊗ Offline - Using cached content
            (if operations queued) N operations waiting
            [Sync] button
```

### How Users Interact

1. **See status** - Always visible if not connected
2. **Expand details** - Click to see feature availability
3. **Retry** - If error, click retry button
4. **Sync** - If operations queued, click sync button
5. **Dismiss** - Click X to hide (re-appears if state changes)

---

## Configuration

### Cache TTLs

```typescript
// No expiration (forever)
await cache.set('key', value);

// 1 hour
await cache.set('key', value, 60*60*1000);

// 1 day
await cache.set('key', value, 24*60*60*1000);

// 7 days
await cache.set('key', value, 7*24*60*60*1000);

// Custom (in milliseconds)
const ttlMs = 12 * 60 * 60 * 1000; // 12 hours
await cache.set('key', value, ttlMs);
```

### Queue Retry Strategy

```typescript
// Default: 3 retries with backoff
queue.enqueue({
  type: 'create',
  // ...
  maxRetries: 3  // 1s, 2s, 4s delays
});

// Custom: more retries for important operations
queue.enqueue({
  type: 'create',
  // ...
  maxRetries: 5  // 1s, 2s, 4s, 8s, 16s
});
```

### Cache Preload

```typescript
// On app startup
constructor(private cache: HolochainCacheService) {
  this.loadCriticalContent();
}

async loadCriticalContent() {
  const items = [
    {
      key: 'app-config',
      value: await api.getConfig(),
      ttlMs: 24*60*60*1000
    },
    {
      key: 'user-profile',
      value: await api.getProfile(),
      ttlMs: 1*60*60*1000
    },
    {
      key: 'learning-paths',
      value: await api.getPaths(),
      ttlMs: 7*24*60*60*1000
    }
  ];

  await this.cache.preload(items);
}
```

---

## Debugging

### Check Connection

```typescript
// In component or service
const client = inject(HolochainClientService);
console.log('Connection state:', client.state());
console.log('Is connected:', client.isConnected());
console.log('Error:', client.error());
```

### Check Queue

```typescript
const queue = inject(OfflineOperationQueueService);
console.log('Queue size:', queue.getQueueSize());
console.log('Queue:', queue.getQueue());
console.log('Stats:', queue.getStats());
```

### Check Cache

```typescript
const cache = inject(HolochainCacheService);
const stats = cache.getStats();
console.log('Entries:', stats.totalEntries);
console.log('Size:', stats.totalSizeBytes / 1024 / 1024, 'MB');
console.log('Hit rate:', stats.hitRate.toFixed(1), '%');
```

### Monitor Events

```typescript
// Queue changes
queue.onQueueChanged(q => console.log('Queue updated:', q.length));

// Sync completion
queue.onSyncComplete((s, f) => console.log(`Sync: ${s} ok, ${f} failed`));

// Connection changes
client.isConnected.subscribe(connected => {
  console.log('Connection:', connected ? 'online' : 'offline');
});
```

---

## Common Issues & Fixes

| Issue | Check | Fix |
|-------|-------|-----|
| UI not showing | Import in AppComponent? | Add to imports array |
| Queue not syncing | `client.isConnected()`? | Manually call `queue.syncAll()` |
| Cache returns null | Is key correct? | Check cache.get() vs cache.set() key |
| Stale data | TTL too long? | Set appropriate TTL |
| Memory full | Cache size? | Call `cache.clear()` |

---

## Performance Tips

```typescript
// ✅ Good: Cache with appropriate TTL
await cache.set('key', value, 24*60*60*1000); // 1 day

// ✅ Good: Preload critical content
await cache.preload(items);

// ✅ Good: Queue write operations
queue.enqueue({ type: 'create', /* ... */ });

// ❌ Avoid: No TTL (fills up cache)
await cache.set('key', value);

// ❌ Avoid: Very short TTL (defeats caching)
await cache.set('key', value, 1000); // 1 second

// ❌ Avoid: Cache everything
// Be selective about what to cache
```

---

## Testing

### Quick Test

```bash
# Simulate offline: DevTools > Network > Offline
# Or stop Holochain conductor

# Observe:
# 1. Banner shows "Offline" (gray)
# 2. Try to create content → gets queued
# 3. Banner shows "1 operation pending"
# 4. Go back online
# 5. Banner shows "Syncing..." then "Connected"
# 6. Queue clears automatically
```

### Verify Cache

```typescript
// In browser console
localStorage.clear();  // Clear session
indexedDB.databases().forEach(db => indexedDB.deleteDatabase(db.name)); // Clear DB

// Reload page
// Preload should repopulate cache

// Check DevTools > Application > IndexedDB
// Should see 'elohim-holochain-cache' database with entries
```

---

## Files Reference

| File | Purpose | Location |
|------|---------|----------|
| Component | UI display | `elohim/components/holochain-availability-ui/` |
| Queue Service | Operation queueing | `elohim/services/offline-operation-queue.service.ts` |
| Cache Service | Data caching | `elohim/services/holochain-cache.service.ts` |
| Main Guide | Complete docs | `HOLOCHAIN_GRACEFUL_DEGRADATION.md` |
| Integration | Step-by-step | `HOLOCHAIN_GRACEFUL_DEGRADATION_INTEGRATION.md` |

---

## API Summary

### HolochainAvailabilityUiComponent
Just add to your template:
```html
<app-holochain-availability-ui></app-holochain-availability-ui>
```

### OfflineOperationQueueService
```typescript
enqueue(op): string
syncAll(): Promise<{succeeded, failed}>
syncOperation(id): Promise<boolean>
dequeue(id): void
dismissOperation(id): void
getQueue(): OfflineOperation[]
getQueueSize(): number
getStats(): {size, totalRetries, ...}
onQueueChanged(callback): void
onSyncComplete(callback): void
```

### HolochainCacheService
```typescript
get<T>(key): Promise<T | null>
set<T>(key, value, ttl?, metadata): Promise<void>
delete(key): Promise<void>
clear(): Promise<void>
preload(items): Promise<void>
query(predicate): CacheEntry[]
getByTag(tag): CacheEntry[]
getByDomain(domain): CacheEntry[]
getStats(): CacheStats
readonly hitRate: Signal<number>
```

---

## Next Steps

1. **Copy files** - Component and services to your project
2. **Update AppComponent** - Add UI and auto-sync setup
3. **Update services** - Add caching and queueing
4. **Test** - Verify with offline scenario
5. **Monitor** - Track hit rates and queue sizes

**That's it! Your app is now resilient to Holochain outages.**

---

For detailed information, see:
- **Full Guide**: `HOLOCHAIN_GRACEFUL_DEGRADATION.md`
- **Integration Steps**: `HOLOCHAIN_GRACEFUL_DEGRADATION_INTEGRATION.md`
- **Delivery Summary**: `GRACEFUL_DEGRADATION_DELIVERY_SUMMARY.md`

