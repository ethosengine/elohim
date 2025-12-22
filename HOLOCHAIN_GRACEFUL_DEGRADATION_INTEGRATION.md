# Graceful Degradation Integration Guide

Quick start guide for integrating graceful degradation UI and offline support into the Elohim app.

## Step 1: Add HolochainAvailabilityUiComponent to AppComponent

### Update app.component.ts

```typescript
import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { CommonModule } from '@angular/common';
import { HolochainAvailabilityUiComponent } from './elohim/components/holochain-availability-ui/holochain-availability-ui.component';
import { HolochainClientService } from './elohim/services/holochain-client.service';
import { OfflineOperationQueueService } from './elohim/services/offline-operation-queue.service';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    RouterOutlet,
    CommonModule,
    HolochainAvailabilityUiComponent  // Add this
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit, OnDestroy {
  // Existing code...

  private readonly holochainService = inject(HolochainClientService);
  private readonly operationQueue = inject(OfflineOperationQueueService);

  ngOnInit(): void {
    // Existing initialization...

    // NEW: Setup automatic sync when connection restored
    this.setupAutoSync();
  }

  /**
   * Auto-sync queued operations when connection is restored
   */
  private setupAutoSync(): void {
    // Watch connection state - when it changes to connected, trigger sync
    const subscription = this.holochainService.isConnected.subscribe(isConnected => {
      if (isConnected && this.operationQueue.getQueueSize() > 0) {
        console.log('[AppComponent] Connection restored, syncing queued operations...');
        this.operationQueue.syncAll().catch(err => {
          console.error('[AppComponent] Sync failed:', err);
        });
      }
    });

    // Note: In production, manage subscription lifecycle properly
    // For simplicity here, we let it live for app lifetime
  }
}
```

### Update app.component.html

```html
<!-- Add availability UI at the very top of the template -->
<app-holochain-availability-ui></app-holochain-availability-ui>

<!-- Existing content -->
<router-outlet></router-outlet>
```

---

## Step 2: Update Services to Use Graceful Degradation

### Example: Update HolochainContentService

```typescript
import { Injectable, signal, computed, inject } from '@angular/core';
import { HolochainClientService } from './holochain-client.service';
import { OfflineOperationQueueService } from './offline-operation-queue.service';
import { HolochainCacheService } from './holochain-cache.service';

@Injectable({
  providedIn: 'root'
})
export class HolochainContentService {
  private readonly holochain = inject(HolochainClientService);
  private readonly queue = inject(OfflineOperationQueueService);
  private readonly cache = inject(HolochainCacheService);

  /**
   * Get content - tries cache first, then Holochain
   */
  async getContent(id: string): Promise<any | null> {
    const cacheKey = `content-${id}`;

    // Check cache first (works offline)
    const cached = await this.cache.get(cacheKey);
    if (cached) {
      return cached;
    }

    // Try to fetch from Holochain
    const result = await this.holochain.callZome({
      zomeName: 'content',
      fnName: 'get_content',
      payload: { id }
    });

    if (result.success && result.data) {
      // Cache for offline use (24 hour TTL)
      await this.cache.set(cacheKey, result.data, 24 * 60 * 60 * 1000, {
        domain: 'elohim-protocol',
        contentType: 'content',
        contentId: id
      });
      return result.data;
    }

    // Return null if both cache and Holochain fail
    return null;
  }

  /**
   * Create content - queues if offline, syncs when online
   */
  async createContent(data: any): Promise<string | null> {
    const result = await this.holochain.callZome({
      zomeName: 'content',
      fnName: 'create_content',
      payload: data
    });

    if (!result.success) {
      // Queue for retry instead of failing
      const opId = this.queue.enqueue({
        type: 'create',
        zomeName: 'content',
        fnName: 'create_content',
        payload: data,
        maxRetries: 5,
        description: `Create content: ${data.title}`
      });

      console.log('[HolochainContent] Create queued for offline sync', { opId });
      return null; // Indicate queued, not successful
    }

    const contentId = result.data;

    // Cache the newly created content
    await this.cache.set(
      `content-${contentId}`,
      { id: contentId, ...data },
      24 * 60 * 60 * 1000,
      { domain: 'elohim-protocol', contentType: 'content' }
    );

    return contentId;
  }

  /**
   * Update content - queues if offline
   */
  async updateContent(id: string, data: any): Promise<boolean> {
    const result = await this.holochain.callZome({
      zomeName: 'content',
      fnName: 'update_content',
      payload: { id, ...data }
    });

    if (!result.success) {
      // Queue for retry
      this.queue.enqueue({
        type: 'update',
        zomeName: 'content',
        fnName: 'update_content',
        payload: { id, ...data },
        maxRetries: 3,
        description: `Update content: ${id}`
      });

      return false; // Queued
    }

    // Update cache
    const cached = await this.cache.get(`content-${id}`);
    if (cached) {
      await this.cache.set(
        `content-${id}`,
        { ...cached, ...data },
        24 * 60 * 60 * 1000
      );
    }

    return true; // Success
  }
}
```

### Update Other Write Services

Apply the same pattern to other services:
- `LearnerBackendService`: Queue mastery updates
- `EconomicService`: Queue economic events
- `AppreciationService`: Queue appreciation events
- `StewardService`: Queue steward operations

---

## Step 3: Update Components to React to Offline State

### Example: Content Editor Component

```typescript
import { Component, OnInit, inject, computed } from '@angular/core';
import { FormBuilder, FormGroup, ReactiveFormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';
import { HolochainClientService } from '../services/holochain-client.service';
import { OfflineOperationQueueService } from '../services/offline-operation-queue.service';
import { HolochainContentService } from '../services/holochain-content.service';

@Component({
  selector: 'app-content-editor',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  template: `
    <div class="content-editor">
      <!-- Offline warning -->
      <div *ngIf="!isConnected()" class="offline-warning">
        <p>You're working offline. Changes will be synced when connection is restored.</p>
        <p *ngIf="queuedOps() > 0">{{ queuedOps() }} operation(s) pending sync</p>
      </div>

      <!-- Form -->
      <form [formGroup]="form">
        <input formControlName="title" placeholder="Title" />
        <textarea formControlName="description"></textarea>

        <!-- Buttons -->
        <button (click)="saveDraft()" class="btn-draft">
          Save Draft
        </button>

        <button
          (click)="publish()"
          [disabled]="!canPublish()"
          class="btn-publish"
        >
          {{ canPublish() ? 'Publish' : 'Cannot Publish (Offline)' }}
        </button>
      </form>
    </div>
  `,
  styles: [`
    .offline-warning {
      background-color: #fff3cd;
      border: 1px solid #ffc107;
      padding: 12px;
      border-radius: 4px;
      margin-bottom: 16px;
    }

    .btn-draft { background: #6c757d; color: white; }
    .btn-publish { background: #007bff; color: white; }
    .btn-publish:disabled { background: #ccc; cursor: not-allowed; }
  `]
})
export class ContentEditorComponent implements OnInit {
  private readonly holochain = inject(HolochainClientService);
  private readonly contentService = inject(HolochainContentService);
  private readonly queue = inject(OfflineOperationQueueService);
  private readonly fb = inject(FormBuilder);

  form: FormGroup;

  // Expose connection state to template
  readonly isConnected = this.holochain.isConnected;
  readonly queuedOps = computed(() => this.queue.getQueueSize());

  // Disable certain actions when offline
  readonly canPublish = computed(() => this.isConnected());
  readonly canShare = computed(() => this.isConnected());

  constructor() {
    this.form = this.fb.group({
      title: [''],
      description: ['']
    });
  }

  ngOnInit(): void {
    // Setup
  }

  /**
   * Save draft - works offline, queues if needed
   */
  async saveDraft(): Promise<void> {
    if (!this.form.valid) return;

    try {
      const result = await this.contentService.updateContent(
        'current-id',
        this.form.value
      );

      if (result) {
        console.log('Draft saved successfully');
      } else {
        console.log('Draft queued for sync');
      }
    } catch (err) {
      console.error('Save failed:', err);
    }
  }

  /**
   * Publish - requires connection
   */
  async publish(): Promise<void> {
    if (!this.canPublish()) {
      alert('Cannot publish while offline');
      return;
    }

    // Publish logic here
  }
}
```

---

## Step 4: Add Health Monitoring Component (Optional)

Create a simple health dashboard for admin/debugging:

```typescript
import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { HolochainClientService } from '../services/holochain-client.service';
import { OfflineOperationQueueService } from '../services/offline-operation-queue.service';
import { HolochainCacheService } from '../services/holochain-cache.service';

@Component({
  selector: 'app-health-dashboard',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="health-dashboard">
      <h3>System Health</h3>

      <!-- Connection -->
      <div class="metric">
        <label>Connection:</label>
        <span [class]="'status-' + holochain.state()">
          {{ holochain.state() }}
        </span>
      </div>

      <!-- Queue -->
      <div class="metric">
        <label>Queued Ops:</label>
        <span>{{ queueStats().size }}</span>
        <span *ngIf="queueStats().size > 0" class="secondary">
          ({{ queueStats().totalRetries }} retries)
        </span>
      </div>

      <!-- Cache -->
      <div class="metric">
        <label>Cache:</label>
        <span>{{ cacheStats().totalEntries }} entries</span>
        <span class="secondary">{{ (cacheStats().totalSizeBytes / 1024 / 1024).toFixed(1) }}MB</span>
      </div>

      <!-- Hit Rate -->
      <div class="metric">
        <label>Hit Rate:</label>
        <span>{{ (cache.hitRate() as any | number:'1.1-1') }}%</span>
      </div>
    </div>
  `,
  styles: [`
    .health-dashboard {
      background: #f5f5f5;
      padding: 16px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
    }

    .metric {
      display: flex;
      gap: 8px;
      padding: 4px 0;
      align-items: center;
    }

    label {
      font-weight: bold;
      min-width: 100px;
    }

    .status-connected { color: green; }
    .status-connecting { color: orange; }
    .status-error { color: red; }
    .status-disconnected { color: gray; }

    .secondary { color: #666; font-size: 11px; }
  `]
})
export class HealthDashboardComponent {
  readonly holochain = inject(HolochainClientService);
  private readonly queue = inject(OfflineOperationQueueService);
  readonly cache = inject(HolochainCacheService);

  queueStats() {
    return this.queue.getStats();
  }

  cacheStats() {
    return this.cache.getStats();
  }
}
```

---

## Step 5: Testing the Integration

### Manual Test Checklist

- [ ] **Connection Lost**
  - [ ] Stop Holochain conductor
  - [ ] Verify yellow "Connecting..." banner appears
  - [ ] After timeout, verify red "Error" banner with retry button
  - [ ] Click "Retry" button and it attempts to reconnect

- [ ] **Offline Operations**
  - [ ] Create/edit content while offline
  - [ ] Verify operation queued (no error to user)
  - [ ] Verify "X operations pending" shows in banner
  - [ ] Restart Holochain
  - [ ] Verify auto-sync or click "Sync" button
  - [ ] Verify operations succeed and queue clears

- [ ] **Cache Functionality**
  - [ ] Read cached content while offline
  - [ ] Verify cache hit rate increases
  - [ ] Clear browser storage and verify cache persists in IndexedDB
  - [ ] Check DevTools→Application→IndexedDB for entries

- [ ] **UI Responsiveness**
  - [ ] Banner dismisses and re-appears on state change
  - [ ] Feature availability list is accurate
  - [ ] Buttons are enabled/disabled appropriately
  - [ ] Mobile layout is responsive

### Automated Test Example

```typescript
describe('Graceful Degradation Integration', () => {
  it('should queue operations while offline', async () => {
    // Simulate offline
    holochain.disconnect();

    // Try to create content
    const result = await contentService.createContent({ title: 'Test' });

    // Should queue, not fail
    expect(result).toBeNull();
    expect(queue.getQueueSize()).toBe(1);
  });

  it('should auto-sync when connection restored', async () => {
    // Queue an operation
    queue.enqueue({ type: 'create', /* ... */ });

    // Simulate connection restored
    await holochain.connect();

    // Wait for auto-sync
    await new Promise(r => setTimeout(r, 1000));

    // Queue should be empty
    expect(queue.getQueueSize()).toBe(0);
  });

  it('should serve cached content when offline', async () => {
    // Cache content
    await cache.set('key', { data: 'cached' });

    // Go offline
    holochain.disconnect();

    // Retrieve should work
    const result = await cache.get('key');
    expect(result).toEqual({ data: 'cached' });
  });
});
```

---

## Troubleshooting Integration

### Issue: HolochainAvailabilityUiComponent not showing

**Solution**:
- Verify component is imported in AppComponent
- Check CSS is loaded (inspect in DevTools)
- Verify banner is above router-outlet in template

### Issue: Operations not syncing automatically

**Solution**:
- Check isConnected signal is working: `console.log(holochain.isConnected())`
- Manually call `queue.syncAll()` to test
- Check browser console for errors
- Verify connection state actually changes

### Issue: Cache returns stale data

**Solution**:
- Set appropriate TTLs: `cache.set(key, value, ttlMs)`
- Monitor expiration: `cache.getStats().oldestEntry`
- Clear cache if needed: `cache.clear()`
- Check TTL logic: expired entries should return null

### Issue: Queue grows but doesn't clear

**Solution**:
- Check if connection is actually succeeded: `holochain.isConnected()`
- Monitor sync: `queue.onSyncComplete((s, f) => console.log(s, f))`
- Check if zome calls are actually succeeding
- Review operation payloads for errors

---

## Performance Tuning

### Cache Size

```typescript
// Increase preload for faster cold start
const commonContent = await api.getCommonContent();
await cache.preload(
  commonContent.map(c => ({
    key: `content-${c.id}`,
    value: c,
    ttlMs: 7 * 24 * 60 * 60 * 1000 // 7 days
  }))
);
```

### Queue Processing

```typescript
// Batch sync every 5 seconds instead of manually
setInterval(() => {
  if (holochain.isConnected() && queue.getQueueSize() > 0) {
    queue.syncAll();
  }
}, 5000);
```

### Selective Caching

```typescript
// Only cache high-value content
const result = await holochain.callZome({...});
if (result.data && result.data.cacheWorthy) {
  await cache.set(key, result.data, ttlMs);
}
```

---

## Next Steps

1. **Deploy**: Follow the checklist above to test
2. **Monitor**: Track cache hit rates and queue sizes
3. **Optimize**: Adjust TTLs and preload based on usage
4. **Document**: Add offline mode to user docs
5. **Enhance**: Add custom sync UI/notifications as needed

---

## Support

For issues with graceful degradation:
1. Check browser console for errors
2. Use Health Dashboard component to diagnose
3. Review queue/cache stats
4. Refer to `HOLOCHAIN_GRACEFUL_DEGRADATION.md` for detailed guide
5. Check tests in `/src/app/elohim/services/*.spec.ts`

