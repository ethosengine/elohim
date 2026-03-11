# Integrity Anchors & Dead Code Cleanup — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Delete 2 dead services (~1,355 lines), create integrity anchor module, extract 3 zome calls into named anchors.

**Architecture:** Dead code deletion first (safe, no dependencies), then create integrity module with IIntegrityAnchor interface, then extract each zome call into an anchor and rewire the consuming service.

**Tech Stack:** Angular 19, TypeScript, inject() pattern, Holochain zome calls via HolochainClientService

---

### Task 1: Delete CircuitBreakerService

**Files:**
- Delete: `elohim-app/src/app/elohim/services/circuit-breaker.service.ts`
- Delete: `elohim-app/src/app/elohim/services/circuit-breaker.service.spec.ts`

**Step 1: Verify zero consumers**

Run: `cd /projects/elohim && grep -r "CircuitBreakerService\|circuit-breaker.service" elohim-app/src/app --include="*.ts" -l`

Expected: Only the two files being deleted (service + spec). If other files appear, STOP and investigate.

**Step 2: Delete the files**

```bash
rm elohim-app/src/app/elohim/services/circuit-breaker.service.ts
rm elohim-app/src/app/elohim/services/circuit-breaker.service.spec.ts
```

**Step 3: Verify no barrel export**

Check `elohim-app/src/app/elohim/services/index.ts` — CircuitBreakerService should NOT be exported there. If it is, remove the export line.

**Step 4: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -5`

Expected: All tests pass (no imports of deleted service).

**Step 5: Commit**

```
feat(elohim): delete unused CircuitBreakerService

Zero production consumers in elohim-app. Pattern exists in
doorway-app if needed.
```

---

### Task 2: Delete OfflineOperationQueueService + clean up consumer

**Files:**
- Delete: `elohim-app/src/app/elohim/services/offline-operation-queue.service.ts`
- Delete: `elohim-app/src/app/elohim/services/offline-operation-queue.service.spec.ts`
- Modify: `elohim-app/src/app/elohim/components/holochain-availability-ui/holochain-availability-ui.component.ts`
- Modify: `elohim-app/src/app/elohim/components/holochain-availability-ui/holochain-availability-ui.component.spec.ts`

**Step 1: Clean up HolochainAvailabilityUiComponent**

The component injects OfflineOperationQueueService and uses it for:
- `queuedOperations` signal (line 48) — always 0 since enqueue() is never called
- `hasQueuedOperations` signal (line 49)
- `syncQueuedOperations()` method (line 159) — calls `operationQueue.syncAll()` which no-ops
- Status messages referencing queue size (lines 89-90)
- Degradation message referencing "Write operations will be queued" (line 110)

Remove the import, injection, and all queue-related computed signals and methods. Simplify status messages. The component still shows connection state via HolochainClientService — that stays.

Updated component (full replacement):

```typescript
import { CommonModule } from '@angular/common';
import { Component, inject, computed, signal } from '@angular/core';

import { HolochainClientService } from '../../services/holochain-client.service';
import { HolochainContentService } from '../../services/holochain-content.service';

@Component({
  selector: 'app-holochain-availability-ui',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './holochain-availability-ui.component.html',
  styleUrl: './holochain-availability-ui.component.css',
})
export class HolochainAvailabilityUiComponent {
  private readonly holochainClient = inject(HolochainClientService);
  private readonly holochainContent = inject(HolochainContentService);

  readonly connectionState = this.holochainClient.state;
  readonly isConnected = this.holochainClient.isConnected;
  readonly error = this.holochainClient.error;
  readonly contentAvailable = this.holochainContent.available;

  readonly isDismissed = signal(false);

  readonly isConnecting = computed(() => this.connectionState() === 'connecting');
  readonly isError = computed(() => this.connectionState() === 'error');
  readonly isOffline = computed(
    () => this.connectionState() === 'disconnected' || this.connectionState() === 'error'
  );

  readonly shouldShow = computed(() => {
    const state = this.connectionState();
    return (
      !this.isDismissed() &&
      (state === 'connecting' || state === 'error' || state === 'disconnected')
    );
  });

  readonly statusMessage = computed(() => {
    const state = this.connectionState();
    if (state === 'connected') return 'Connected to Holochain';
    if (state === 'connecting') return 'Connecting to Holochain...';
    if (state === 'error') {
      const errorMsg = this.error();
      return `Connection Error: ${errorMsg ?? 'Unknown error'}`;
    }
    if (state === 'disconnected') return 'Offline - Using cached content';
    return 'Unknown connection state';
  });

  readonly degradationMessage = computed(() => {
    if (this.isConnected()) return '';
    if (this.isConnecting()) {
      return 'Some features may be temporarily unavailable while connecting.';
    }
    return 'Working in offline mode. Some features are unavailable.';
  });

  readonly bannerClass = computed(() => {
    if (this.isConnected()) return 'connected';
    if (this.isConnecting()) return 'connecting';
    if (this.isError()) return 'error';
    return 'offline';
  });

  readonly bannerIcon = computed(() => {
    if (this.isConnected()) return '✓';
    if (this.isConnecting()) return '⟳';
    if (this.isError()) return '⚠';
    return '⊗';
  });

  dismissBanner(): void {
    this.isDismissed.set(true);
  }

  async retryConnection(): Promise<void> {
    this.isDismissed.set(false);
    try {
      await this.holochainClient.connect();
    } catch {
      // Connection retry failed - user can try again
    }
  }

  getDegradedFeatures(): string[] {
    if (this.isConnected()) return [];
    return [
      'Creating new content',
      'Submitting mastery progress',
      'Recording appreciation',
      'Accessing real-time data',
    ];
  }

  getAvailableFeatures(): string[] {
    return [
      'Reading cached content',
      'Browsing learning paths',
      'Viewing cached blobs',
      'Offline caching',
    ];
  }
}
```

**Step 2: Update spec**

Read `holochain-availability-ui.component.spec.ts` and remove:
- `OfflineOperationQueueService` import and mock provider
- Any tests referencing `queuedOperations`, `hasQueuedOperations`, `syncQueuedOperations`

**Step 3: Verify no other consumers**

Run: `grep -r "OfflineOperationQueueService\|offline-operation-queue.service" elohim-app/src/app --include="*.ts" -l`

Expected: Only the two files being deleted + the two component files being modified.

**Step 4: Delete the service files**

```bash
rm elohim-app/src/app/elohim/services/offline-operation-queue.service.ts
rm elohim-app/src/app/elohim/services/offline-operation-queue.service.spec.ts
```

**Step 5: Verify no barrel export**

Check `elohim-app/src/app/elohim/services/index.ts` — OfflineOperationQueueService should NOT be exported. If it is, remove the line.

**Step 6: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -5`

Expected: All tests pass.

**Step 7: Commit**

```
feat(elohim): delete orphaned OfflineOperationQueueService

enqueue() had zero callers — queue was never populated.
Simplified HolochainAvailabilityUiComponent to remove dead queue UI.
```

---

### Task 3: Create integrity anchor module + interface

**Files:**
- Create: `elohim-app/src/app/elohim/integrity/integrity-anchor.interface.ts`
- Create: `elohim-app/src/app/elohim/integrity/index.ts`

**Step 1: Create the interface**

```typescript
// integrity-anchor.interface.ts

/**
 * IIntegrityAnchor — A single Holochain zome call that provides
 * cryptographic proof of network agreement.
 *
 * Holochain DNAs are cryptographically signed to their schema,
 * requiring parallel conductors to agree on upgrade paths.
 * Each anchor is a verification point — not a data fetch.
 *
 * The verify() method calls one zome function and returns the
 * DHT-attested result. Services wrap anchors with caching,
 * fallback, and orchestration logic.
 */
export interface IIntegrityAnchor<TInput, TOutput> {
  readonly zomeName: string;
  readonly fnName: string;

  /**
   * Verify data against the DHT's cryptographic integrity.
   * Returns the network-attested result.
   */
  verify(input: TInput): Promise<TOutput>;
}
```

**Step 2: Create barrel export**

```typescript
// index.ts
export type { IIntegrityAnchor } from './integrity-anchor.interface';
```

**Step 3: Commit**

```
feat(elohim): create integrity anchor module

IIntegrityAnchor<TInput, TOutput> interface for single zome calls
that provide cryptographic proof of network agreement.
```

---

### Task 4: Extract blob-metadata integrity anchor

**Files:**
- Create: `elohim-app/src/app/elohim/integrity/blob-metadata.anchor.ts`
- Modify: `elohim-app/src/app/elohim/integrity/index.ts`
- Modify: `elohim-app/src/app/lamad/services/blob-manager.service.ts`

**Step 1: Read BlobManagerService to find exact zome call**

Read `blob-manager.service.ts` around line 630 to capture exact types and call signature.

**Step 2: Create the anchor**

```typescript
// blob-metadata.anchor.ts
import { Injectable, inject } from '@angular/core';

import { HolochainClientService } from '../services/holochain-client.service';
import type { IIntegrityAnchor } from './integrity-anchor.interface';

// Re-export from BlobManagerService's types — these are the DHT wire types
import type { BlobsForContentOutput } from '../../lamad/services/blob-manager.service';

@Injectable({ providedIn: 'root' })
export class BlobMetadataAnchor implements IIntegrityAnchor<string, BlobsForContentOutput | null> {
  readonly zomeName = 'content_store';
  readonly fnName = 'get_blobs_by_content_id';

  private readonly holochainClient = inject(HolochainClientService);

  async verify(contentId: string): Promise<BlobsForContentOutput | null> {
    try {
      return await this.holochainClient.callZome(
        this.zomeName,
        this.fnName,
        { content_id: contentId }
      );
    } catch {
      return null;
    }
  }
}
```

NOTE: The exact `callZome` signature and import path for `HolochainClientService` must be verified by reading the actual file. The implementer should check how BlobManagerService currently calls zomes (line ~632) and replicate that exact pattern.

**Step 3: Update barrel**

Add to `integrity/index.ts`:
```typescript
export { BlobMetadataAnchor } from './blob-metadata.anchor';
```

**Step 4: Rewire BlobManagerService**

In `blob-manager.service.ts`, replace the direct zome call in `callGetBlobsForContent()` (~line 632) with:

```typescript
private readonly blobMetadataAnchor = inject(BlobMetadataAnchor);

private async callGetBlobsForContent(contentId: string): Promise<BlobsForContentOutput | null> {
  return this.blobMetadataAnchor.verify(contentId);
}
```

Remove the `HolochainClientService` import/injection from BlobManagerService if it was only used for this one call. Check for other usages first.

**Step 5: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts "blob" 2>&1 | tail -10`

Expected: blob-manager and blob-bootstrap tests pass.

**Step 6: Commit**

```
feat(elohim): extract blob-metadata integrity anchor

BlobManagerService now delegates its single zome call to
BlobMetadataAnchor.verify() — making the DHT integrity role
explicit and the service a pure orchestrator.
```

---

### Task 5: Extract federation-registry integrity anchor

**Files:**
- Create: `elohim-app/src/app/elohim/integrity/federation-registry.anchor.ts`
- Modify: `elohim-app/src/app/elohim/integrity/index.ts`
- Modify: `elohim-app/src/app/imagodei/services/doorway-registry.service.ts`

**Step 1: Read DoorwayRegistryService to find exact zome call**

Read `doorway-registry.service.ts` around line 381 to capture exact types and call pattern. Note: this uses `roleName: 'infrastructure'` which is a different role than `content_store`.

**Step 2: Create the anchor**

```typescript
// federation-registry.anchor.ts
import { Injectable, inject } from '@angular/core';

import { HolochainClientService } from '../services/holochain-client.service';
import type { IIntegrityAnchor } from './integrity-anchor.interface';
import type { DoorwayInfo } from '../../imagodei/models/doorway.model';

@Injectable({ providedIn: 'root' })
export class FederationRegistryAnchor implements IIntegrityAnchor<string, DoorwayInfo[]> {
  readonly zomeName = 'infrastructure';
  readonly fnName = 'get_doorways_by_region';

  private readonly holochainClient = inject(HolochainClientService);

  async verify(region: string): Promise<DoorwayInfo[]> {
    try {
      return await this.holochainClient.callZome(
        this.zomeName,
        this.fnName,
        region,
        { roleName: 'infrastructure' }
      );
    } catch {
      return [];
    }
  }
}
```

NOTE: The exact `callZome` signature for infrastructure role must be verified. Check how DoorwayRegistryService calls it (line ~383) — it may pass `roleName` differently.

**Step 3: Update barrel**

Add to `integrity/index.ts`:
```typescript
export { FederationRegistryAnchor } from './federation-registry.anchor';
```

**Step 4: Rewire DoorwayRegistryService**

In `doorway-registry.service.ts`, replace the zome call in `fetchFromDHT()` (~line 381) with:

```typescript
private readonly federationAnchor = inject(FederationRegistryAnchor);

private async fetchFromDHT(): Promise<DoorwayInfo[]> {
  return this.federationAnchor.verify('global');
}
```

Remove the `HolochainClientService` import/injection if only used for this call.

**Step 5: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts "doorway-registry" 2>&1 | tail -10`

Expected: doorway-registry tests pass.

**Step 6: Commit**

```
feat(elohim): extract federation-registry integrity anchor

DoorwayRegistryService now delegates DHT federation lookup to
FederationRegistryAnchor.verify() — the zome call was already
a fallback behind HTTP; now its integrity role is named.
```

---

### Task 6: Extract node-registry integrity anchor

**Files:**
- Create: `elohim-app/src/app/elohim/integrity/node-registry.anchor.ts`
- Modify: `elohim-app/src/app/elohim/integrity/index.ts`
- Modify: `elohim-app/src/app/doorway/services/running-context.service.ts`

**Step 1: Read RunningContextService to find exact zome call**

Read `running-context.service.ts` around line 225 to capture exact types. Note: uses `node_registry_coordinator` zome and returns `RawRegisteredNode[]` (internal interface, snake_case).

**Step 2: Create the anchor**

```typescript
// node-registry.anchor.ts
import { Injectable, inject } from '@angular/core';

import { HolochainClientService } from '../services/holochain-client.service';
import type { IIntegrityAnchor } from './integrity-anchor.interface';

/**
 * Raw node registration from the DHT (snake_case wire format).
 * Transformed to RegisteredNode (camelCase) by RunningContextService.
 */
export interface RawRegisteredNode {
  node_id: string;
  display_name: string;
  node_type: string;
  status: string;
  last_heartbeat: string;
  doorway_url?: string;
}

@Injectable({ providedIn: 'root' })
export class NodeRegistryAnchor implements IIntegrityAnchor<null, RawRegisteredNode[]> {
  readonly zomeName = 'node_registry_coordinator';
  readonly fnName = 'get_my_nodes';

  private readonly holochainClient = inject(HolochainClientService);

  async verify(_input: null): Promise<RawRegisteredNode[]> {
    try {
      return await this.holochainClient.callZome(
        this.zomeName,
        this.fnName,
        null
      );
    } catch {
      return [];
    }
  }
}
```

NOTE: Verify exact `RawRegisteredNode` fields by reading `running-context.service.ts` line ~49. The interface above is approximate — match the actual wire type.

**Step 3: Update barrel**

Add to `integrity/index.ts`:
```typescript
export { NodeRegistryAnchor } from './node-registry.anchor';
export type { RawRegisteredNode } from './node-registry.anchor';
```

**Step 4: Rewire RunningContextService**

In `running-context.service.ts`, replace the zome call in `getRegisteredNodes()` (~line 225) with:

```typescript
private readonly nodeAnchor = inject(NodeRegistryAnchor);

private async getRegisteredNodes(): Promise<RawRegisteredNode[]> {
  return this.nodeAnchor.verify(null);
}
```

Remove the internal `RawRegisteredNode` interface (now imported from anchor). Remove `HolochainClientService` if only used for this call.

**Step 5: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts "running-context" 2>&1 | tail -10`

Expected: running-context tests pass.

**Step 6: Commit**

```
feat(elohim): extract node-registry integrity anchor

RunningContextService now delegates node discovery to
NodeRegistryAnchor.verify() — making the DHT integrity
verification explicit.
```

---

### Task 7: Final verification + squash

**Step 1: Full test suite**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -5`

Expected: All 238+ test files pass, 7,134+ tests pass.

**Step 2: Lint check**

Run: `cd /projects/elohim/elohim-app && pnpm run lint 2>&1 | tail -5`

Expected: Clean.

**Step 3: Verify integrity module structure**

```bash
ls -la elohim-app/src/app/elohim/integrity/
```

Expected:
```
integrity-anchor.interface.ts
blob-metadata.anchor.ts
federation-registry.anchor.ts
node-registry.anchor.ts
index.ts
```

**Step 4: Squash and push**

Squash all commits into one:

```
refactor(elohim): dead code cleanup + integrity anchor module

Tier 1 — Deleted dead services:
- CircuitBreakerService (333 lines, 0 consumers)
- OfflineOperationQueueService (511 lines, orphaned enqueue path)
- Simplified HolochainAvailabilityUiComponent

Tier 2 — Created elohim/integrity/ module:
- IIntegrityAnchor<TInput, TOutput> interface
- BlobMetadataAnchor (content_store.get_blobs_by_content_id)
- FederationRegistryAnchor (infrastructure.get_doorways_by_region)
- NodeRegistryAnchor (node_registry_coordinator.get_my_nodes)

Rewired BlobManagerService, DoorwayRegistryService, and
RunningContextService to delegate zome calls to named anchors.
```
