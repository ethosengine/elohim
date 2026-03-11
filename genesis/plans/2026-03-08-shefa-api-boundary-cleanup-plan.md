# Shefa API Boundary Cleanup — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate 4 fat Angular services (~2,580 lines) from the shefa pillar, completing the data boundary migration to the thin API + interface + injection token pattern.

**Architecture:** Each fat service either has zero consumers (pure delete) or gets replaced by an injection token pointing to a thin HTTP service. The pattern follows the stewardship migration: `InjectionToken<Interface>` with factory defaulting to the thin API service. Consumers inject the token, never the concrete class.

**Tech Stack:** Angular 19, TypeScript, RxJS, Vitest

**Design doc:** `genesis/plans/2026-03-08-shefa-api-boundary-cleanup-design.md`

---

### Task 1: Delete exchange.service (0 consumers)

**Files:**
- Delete: `elohim-app/src/app/shefa/services/exchange.service.ts` (1,341 lines)
- Delete: `elohim-app/src/app/shefa/services/exchange.service.spec.ts`

**Step 1: Verify no imports exist**

Run: `cd /projects/elohim && grep -r "ExchangeService\b" elohim-app/src/ --include="*.ts" -l | grep -v exchange.service | grep -v exchange-api.service | grep -v exchange.interface`

Expected: No results (only the service itself, its API replacement, and interface reference it)

**Step 2: Delete the fat service and its spec**

```bash
rm elohim-app/src/app/shefa/services/exchange.service.ts
rm elohim-app/src/app/shefa/services/exchange.service.spec.ts
```

**Step 3: Update the exchange interface to remove stale doc reference**

In `elohim-app/src/app/shefa/interfaces/exchange.interface.ts`, update the factory doc comment from "resolves to ExchangeService" to "resolves to ExchangeApiService" (the factory already points to ExchangeApiService, just the comment is stale).

**Step 4: Verify build**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Build succeeds with no errors.

**Step 5: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20`
Expected: No new failures related to exchange.

**Step 6: Commit**

```bash
git add -A elohim-app/src/app/shefa/services/exchange.service.ts \
  elohim-app/src/app/shefa/services/exchange.service.spec.ts \
  elohim-app/src/app/shefa/interfaces/exchange.interface.ts
git commit -m "refactor(shefa): delete fat exchange.service (0 consumers)

EXCHANGE token already defaults to ExchangeApiService.
No consumers imported the fat service directly.
-1,341 lines."
```

---

### Task 2: Delete economic-event-bridge.service (0 consumers)

**Files:**
- Delete: `elohim-app/src/app/shefa/banking-bridge/services/economic-event-bridge.service.ts` (314 lines)
- Delete: `elohim-app/src/app/shefa/banking-bridge/services/economic-event-bridge.service.spec.ts` (~697 lines)
- Modify: `elohim-app/src/app/shefa/banking-bridge/index.ts` (remove barrel export)

**Step 1: Verify no imports exist**

Run: `cd /projects/elohim && grep -r "EconomicEventBridgeService\b" elohim-app/src/ --include="*.ts" -l | grep -v economic-event-bridge.service`

Expected: Only `banking-bridge/index.ts` (the barrel export). No actual consumers.

**Step 2: Delete the service and its spec**

```bash
rm elohim-app/src/app/shefa/banking-bridge/services/economic-event-bridge.service.ts
rm elohim-app/src/app/shefa/banking-bridge/services/economic-event-bridge.service.spec.ts
```

**Step 3: Clean the barrel export**

In `elohim-app/src/app/shefa/banking-bridge/index.ts`, remove lines 26-32:

```typescript
// Remove these lines:
// Bridge service
export {
  EconomicEventBridgeService,
  type EconomicEventPayload,
  type CommitResult,
  type BatchCommitResult,
} from './services/economic-event-bridge.service';
```

Keep the Store exports (lines 14-24) intact.

**Step 4: Verify build**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Build succeeds.

**Step 5: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20`
Expected: No new failures.

**Step 6: Commit**

```bash
git add -A elohim-app/src/app/shefa/banking-bridge/
git commit -m "refactor(shefa): delete orphaned economic-event-bridge.service

Superseded by EconomicEventsApiService (HTTP API).
No consumers — was exported from barrel but never injected.
-1,011 lines (service + spec)."
```

---

### Task 3: Rewire TransactionImportService to use ECONOMIC_EVENT_FACTORY token

**Files:**
- Modify: `elohim-app/src/app/shefa/services/transaction-import.service.ts:41,105`
- Modify: `elohim-app/src/app/shefa/services/transaction-import.service.spec.ts:23,65`

**Step 1: Update the production service import and injection**

In `elohim-app/src/app/shefa/services/transaction-import.service.ts`:

Replace line 41:
```typescript
// Old:
import { EconomicEventFactoryService } from './economic-event-factory.service';
// New:
import { ECONOMIC_EVENT_FACTORY } from '../interfaces';
```

Replace line 105:
```typescript
// Old:
private readonly eventFactory = inject(EconomicEventFactoryService);
// New:
private readonly eventFactory = inject(ECONOMIC_EVENT_FACTORY);
```

**Step 2: Update the test spec**

In `elohim-app/src/app/shefa/services/transaction-import.service.spec.ts`:

Replace line 23:
```typescript
// Old:
import { EconomicEventFactoryService } from './economic-event-factory.service';
// New:
import { ECONOMIC_EVENT_FACTORY } from '../interfaces';
```

Replace line 65:
```typescript
// Old:
{ provide: EconomicEventFactoryService, useValue: mockEventFactory },
// New:
{ provide: ECONOMIC_EVENT_FACTORY, useValue: mockEventFactory },
```

**Step 3: Verify build**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Build succeeds.

**Step 4: Run the specific test**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts "transaction-import"`
Expected: All transaction-import tests pass.

**Step 5: Commit**

```bash
git add elohim-app/src/app/shefa/services/transaction-import.service.ts \
  elohim-app/src/app/shefa/services/transaction-import.service.spec.ts
git commit -m "refactor(shefa): rewire TransactionImportService to ECONOMIC_EVENT_FACTORY token

Switch from direct inject(EconomicEventFactoryService) to
inject(ECONOMIC_EVENT_FACTORY) which defaults to EconomicEventsApiService.
Prepares for fat service deletion."
```

---

### Task 4: Delete economic-event-factory.service

**Files:**
- Delete: `elohim-app/src/app/shefa/services/economic-event-factory.service.ts` (394 lines)
- Delete: `elohim-app/src/app/shefa/services/economic-event-factory.service.spec.ts`

**Step 1: Verify no remaining imports**

Run: `cd /projects/elohim && grep -r "EconomicEventFactoryService\b" elohim-app/src/ --include="*.ts" -l | grep -v economic-event-factory.service`

Expected: No results (TransactionImportService was rewired in Task 3).

**Step 2: Delete the fat service and spec**

```bash
rm elohim-app/src/app/shefa/services/economic-event-factory.service.ts
rm elohim-app/src/app/shefa/services/economic-event-factory.service.spec.ts
```

**Step 3: Verify build**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Build succeeds.

**Step 4: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20`
Expected: No new failures.

**Step 5: Commit**

```bash
git add -A elohim-app/src/app/shefa/services/economic-event-factory.service.ts \
  elohim-app/src/app/shefa/services/economic-event-factory.service.spec.ts
git commit -m "refactor(shefa): delete fat economic-event-factory.service

All consumers now use ECONOMIC_EVENT_FACTORY token
which defaults to EconomicEventsApiService (HTTP).
-394 lines."
```

---

### Task 5: Create IComputeEvent interface and COMPUTE_EVENT token

**Files:**
- Create: `elohim-app/src/app/shefa/interfaces/compute-event.interface.ts`
- Modify: `elohim-app/src/app/shefa/interfaces/index.ts`

**Step 1: Create the interface file**

Create `elohim-app/src/app/shefa/interfaces/compute-event.interface.ts`:

```typescript
/**
 * IComputeEvent -- Abstract interface for compute resource event emission.
 *
 * Handles the UX-layer concern of sampling compute metrics on an interval,
 * converting them to economic events, and persisting via the economic events API.
 *
 * Consumers inject the COMPUTE_EVENT token; the default factory resolves to
 * ComputeEventApiService.
 *
 * @example
 * ```typescript
 * @Component({...})
 * export class ShefaDashboardComponent {
 *   private readonly computeEvents = inject(COMPUTE_EVENT);
 *
 *   ngOnInit() {
 *     this.computeEvents
 *       .initializeEventEmission(this.operatorId, this.resourceId)
 *       .pipe(takeUntil(this.destroy$))
 *       .subscribe();
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';
import { Observable } from 'rxjs';

import type { ComputeEventApiService } from '../services/compute-event-api.service';

/**
 * Configuration for compute event generation
 */
export interface ComputeEventConfig {
  cpuHourRate: number;
  storageGBHourRate: number;
  bandwidthMbpsHourRate: number;
  eventEmissionInterval: number;
  aggregationStrategy: 'per-governance-level' | 'per-custodian' | 'aggregate';
}

/**
 * Compute usage snapshot for a single measurement period
 */
export interface ComputeUsageSnapshot {
  timestamp: string;
  cpuCoreHours: number;
  storageGBHours: number;
  bandwidthMbpsHours: number;
  governanceLevel?: 'individual' | 'household' | 'community' | 'network';
  custodianId?: string;
}

/**
 * Computed event payload emitted to subscribers
 */
export interface ComputeEventPayload {
  eventId: string;
  timestamp: string;
  operatorId: string;
  usage: ComputeUsageSnapshot;
  tokensEarned: number;
  economicEventId?: string;
}

/**
 * Abstract compute event emitter.
 */
export interface IComputeEvent {
  initializeEventEmission(
    operatorId: string,
    stewardedResourceId: string
  ): Observable<ComputeEventPayload>;

  getComputeEvents$(): Observable<ComputeEventPayload>;

  setConfig(config: Partial<ComputeEventConfig>): void;

  getConfig(): ComputeEventConfig;
}

/**
 * Injection token for compute event emission.
 *
 * Default factory resolves to ComputeEventApiService which:
 * - Samples ShefaComputeService metrics on interval
 * - Converts to EconomicEvent format
 * - Persists via ECONOMIC_EVENT_FACTORY (HTTP bulk endpoint)
 */
export const COMPUTE_EVENT = new InjectionToken<IComputeEvent>('ComputeEvent', {
  providedIn: 'root',
  factory: () => inject(ComputeEventApiService) as IComputeEvent,
});
```

**Step 2: Add to barrel exports**

In `elohim-app/src/app/shefa/interfaces/index.ts`, add:

```typescript
export type { IComputeEvent, ComputeEventConfig, ComputeUsageSnapshot, ComputeEventPayload } from './compute-event.interface';
export { COMPUTE_EVENT } from './compute-event.interface';
```

**Step 3: Verify file compiles (build will fail until Task 6 creates the service)**

This is expected — the token references `ComputeEventApiService` which doesn't exist yet. Continue to Task 6.

---

### Task 6: Create compute-event-api.service.ts (thin service)

**Files:**
- Create: `elohim-app/src/app/shefa/services/compute-event-api.service.ts`

**Step 1: Create the thin service**

Create `elohim-app/src/app/shefa/services/compute-event-api.service.ts`:

```typescript
/**
 * Compute Event API Service (Thin)
 *
 * UX-layer service that samples compute metrics on an interval,
 * converts them to economic events, and persists via the economic
 * events bulk API (elohim-storage /api/v1/economic-events/bulk).
 *
 * This replaces the fat ComputeEventService which called Holochain
 * zomes directly. Business logic stays here (metric sampling, pricing,
 * aggregation strategy) because it's UX-layer concern — the persistence
 * is delegated to ECONOMIC_EVENT_FACTORY.
 */

import { Injectable, inject } from '@angular/core';
import { BehaviorSubject, Observable, Subject, interval, of, from } from 'rxjs';
import { switchMap, tap, catchError, startWith, map } from 'rxjs/operators';

import { LamadEventType } from '@app/elohim/models/economic-event.model';

import { AllocationSnapshot, ComputeMetrics } from '../models/shefa-dashboard.model';
import type {
  ComputeEventConfig,
  ComputeEventPayload,
  ComputeUsageSnapshot,
  IComputeEvent,
} from '../interfaces/compute-event.interface';
import { ECONOMIC_EVENT_FACTORY } from '../interfaces';
import { CreateEconomicEventInput } from './economic.service';
import { ShefaComputeService } from './shefa-compute.service';

const DEFAULT_CONFIG: ComputeEventConfig = {
  cpuHourRate: 0.1,
  storageGBHourRate: 0.001,
  bandwidthMbpsHourRate: 0.01,
  eventEmissionInterval: 3600000,
  aggregationStrategy: 'per-governance-level',
};

@Injectable({
  providedIn: 'root',
})
export class ComputeEventApiService implements IComputeEvent {
  private config: ComputeEventConfig = { ...DEFAULT_CONFIG };
  private readonly lastMetrics$ = new BehaviorSubject<ComputeMetrics | null>(null);
  private readonly lastAllocations$ = new BehaviorSubject<AllocationSnapshot | null>(null);
  private readonly computeEvents$ = new Subject<ComputeEventPayload>();
  private lastEmissionTime = Date.now();

  private readonly eventFactory = inject(ECONOMIC_EVENT_FACTORY);
  private readonly shefaCompute = inject(ShefaComputeService);

  initializeEventEmission(
    operatorId: string,
    _stewardedResourceId: string
  ): Observable<ComputeEventPayload> {
    return interval(this.config.eventEmissionInterval).pipe(
      startWith(0),
      switchMap(() => {
        const state = this.shefaCompute.getDashboardState();
        if (!state) return of<ComputeEventPayload>(null as unknown as ComputeEventPayload);
        return this.generateComputeEvents(operatorId, state.computeMetrics, state.allocations);
      }),
      tap(event => {
        if (event) this.computeEvents$.next(event);
      }),
      catchError(() => of<ComputeEventPayload>(null as unknown as ComputeEventPayload))
    );
  }

  getComputeEvents$(): Observable<ComputeEventPayload> {
    return this.computeEvents$.asObservable();
  }

  setConfig(config: Partial<ComputeEventConfig>): void {
    this.config = { ...this.config, ...config };
  }

  getConfig(): ComputeEventConfig {
    return this.config;
  }

  // ---------------------------------------------------------------------------
  // Private — metric sampling, conversion, persistence
  // ---------------------------------------------------------------------------

  private generateComputeEvents(
    operatorId: string,
    metrics: ComputeMetrics,
    allocations: AllocationSnapshot
  ): Observable<ComputeEventPayload> {
    const lastMetrics = this.lastMetrics$.value;

    const cpuCoreHours = this.calculateCpuCoreHours(lastMetrics, metrics);
    const storageGBHours = this.calculateStorageGBHours(lastMetrics, metrics);
    const bandwidthMbpsHours = this.calculateBandwidthMbpsHours(lastMetrics, metrics);

    this.lastMetrics$.next(metrics);
    this.lastAllocations$.next(allocations);

    const events: ComputeEventPayload[] = [];

    switch (this.config.aggregationStrategy) {
      case 'per-governance-level':
        events.push(
          ...this.generatePerGovernanceLevelEvents(
            operatorId, cpuCoreHours, storageGBHours, bandwidthMbpsHours, allocations
          )
        );
        break;
      case 'per-custodian':
        events.push(
          ...this.generatePerCustodianEvents(
            operatorId, cpuCoreHours, storageGBHours, bandwidthMbpsHours, allocations
          )
        );
        break;
      case 'aggregate':
      default:
        events.push(
          this.generateAggregateEvent(
            operatorId, cpuCoreHours, storageGBHours, bandwidthMbpsHours
          )
        );
        break;
    }

    return this.persistComputeEvents(operatorId, events).pipe(
      map(persisted => {
        persisted.forEach(e => this.computeEvents$.next(e));
        return persisted[0] ?? ({} as ComputeEventPayload);
      })
    );
  }

  private generatePerGovernanceLevelEvents(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number,
    allocations: AllocationSnapshot
  ): ComputeEventPayload[] {
    const levels = ['individual', 'household', 'community', 'network'] as const;
    return levels.map(level => {
      const alloc = allocations.byGovernanceLevel[level];
      const cpu = (totalCpuHours * alloc.cpuPercent) / 100;
      const storage = (totalStorageGBHours * alloc.storagePercent) / 100;
      const bandwidth = (totalBandwidthHours * alloc.bandwidthPercent) / 100;
      return {
        eventId: this.generateEventId(),
        timestamp: new Date().toISOString(),
        operatorId,
        usage: { timestamp: new Date().toISOString(), cpuCoreHours: cpu, storageGBHours: storage, bandwidthMbpsHours: bandwidth, governanceLevel: level },
        tokensEarned: this.calculateTokensEarned(cpu, storage, bandwidth),
      };
    });
  }

  private generatePerCustodianEvents(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number,
    allocations: AllocationSnapshot
  ): ComputeEventPayload[] {
    const events: ComputeEventPayload[] = [];
    allocations.allocationBlocks.forEach(block => {
      if (block.relatedAgents?.length) {
        const cpu = (totalCpuHours * block.cpu.percent) / 100;
        const storage = (totalStorageGBHours * block.storage.percent) / 100;
        const bandwidth = (totalBandwidthHours * block.bandwidth.percent) / 100;
        block.relatedAgents.forEach(custodianId => {
          const count = block.relatedAgents!.length;
          events.push({
            eventId: this.generateEventId(),
            timestamp: new Date().toISOString(),
            operatorId,
            usage: { timestamp: new Date().toISOString(), cpuCoreHours: cpu / count, storageGBHours: storage / count, bandwidthMbpsHours: bandwidth / count, custodianId },
            tokensEarned: this.calculateTokensEarned(cpu / count, storage / count, bandwidth / count),
          });
        });
      }
    });
    return events;
  }

  private generateAggregateEvent(
    operatorId: string,
    totalCpuHours: number,
    totalStorageGBHours: number,
    totalBandwidthHours: number
  ): ComputeEventPayload {
    return {
      eventId: this.generateEventId(),
      timestamp: new Date().toISOString(),
      operatorId,
      usage: { timestamp: new Date().toISOString(), cpuCoreHours: totalCpuHours, storageGBHours: totalStorageGBHours, bandwidthMbpsHours: totalBandwidthHours },
      tokensEarned: this.calculateTokensEarned(totalCpuHours, totalStorageGBHours, totalBandwidthHours),
    };
  }

  private persistComputeEvents(
    operatorId: string,
    events: ComputeEventPayload[]
  ): Observable<ComputeEventPayload[]> {
    if (events.length === 0 || Date.now() - this.lastEmissionTime < this.config.eventEmissionInterval * 0.8) {
      return of([]);
    }
    this.lastEmissionTime = Date.now();

    const staged = events.map(e => this.convertToStagedFormat(operatorId, e));

    return from(this.eventFactory.createMultipleFromStaged(staged)).pipe(
      map(created => {
        return events.map((e, i) => ({
          ...e,
          economicEventId: created[i]?.id ?? undefined,
        }));
      }),
      catchError(() => of([]))
    );
  }

  private convertToStagedFormat(operatorId: string, payload: ComputeEventPayload) {
    const { cpuHours, storageHours, bandwidthHours } = {
      cpuHours: payload.usage.cpuCoreHours,
      storageHours: payload.usage.storageGBHours,
      bandwidthHours: payload.usage.bandwidthMbpsHours,
    };

    let quantity = cpuHours;
    let unit = 'cpu-hour';
    if (storageHours > cpuHours && storageHours > bandwidthHours) {
      quantity = storageHours;
      unit = 'gb-hour';
    } else if (bandwidthHours > cpuHours) {
      quantity = bandwidthHours;
      unit = 'mbps-hour';
    }

    // Return a shape compatible with StagedTransaction for the bulk API
    return {
      id: payload.eventId,
      type: 'credit' as const,
      amount: quantity,
      description: `Compute: ${cpuHours.toFixed(2)} CPU-h, ${storageHours.toFixed(2)} GB-h, ${bandwidthHours.toFixed(2)} Mbps-h`,
      merchantName: payload.usage.governanceLevel ?? payload.usage.custodianId ?? 'family-community',
      category: 'compute-infrastructure',
      date: payload.timestamp,
      reviewStatus: 'approved' as const,
      plaidTransactionId: payload.eventId,
      accountId: operatorId,
    } as any; // StagedTransaction shape — the API handles transformation
  }

  private calculateCpuCoreHours(last: ComputeMetrics | null, current: ComputeMetrics): number {
    if (!last) {
      return (current.cpu.usagePercent / 100) * current.cpu.totalCores * (this.config.eventEmissionInterval / 3600000);
    }
    const avg = (last.cpu.usagePercent + current.cpu.usagePercent) / 2 / 100;
    return avg * current.cpu.totalCores * (this.config.eventEmissionInterval / 3600000);
  }

  private calculateStorageGBHours(last: ComputeMetrics | null, current: ComputeMetrics): number {
    if (!last) return current.storage.usedGB * (this.config.eventEmissionInterval / 3600000);
    return ((last.storage.usedGB + current.storage.usedGB) / 2) * (this.config.eventEmissionInterval / 3600000);
  }

  private calculateBandwidthMbpsHours(last: ComputeMetrics | null, current: ComputeMetrics): number {
    if (!last) return current.network.bandwidth.usedUpstreamMbps * (this.config.eventEmissionInterval / 3600000);
    const avg = ((last.network.bandwidth.usedUpstreamMbps + current.network.bandwidth.usedUpstreamMbps) / 2
      + (last.network.bandwidth.usedDownstreamMbps + current.network.bandwidth.usedDownstreamMbps) / 2) / 2;
    return avg * (this.config.eventEmissionInterval / 3600000);
  }

  private calculateTokensEarned(cpu: number, storage: number, bandwidth: number): number {
    return cpu * this.config.cpuHourRate + storage * this.config.storageGBHourRate + bandwidth * this.config.bandwidthMbpsHourRate;
  }

  private generateEventId(): string {
    return `ce-${Date.now()}-${(crypto.getRandomValues(new Uint32Array(1))[0] / 2 ** 32).toString(36).substring(2, 11)}`;
  }
}
```

**Step 2: Verify build compiles (with interface from Task 5)**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Build succeeds.

---

### Task 7: Rewire ShefaDashboardComponent to COMPUTE_EVENT token

**Files:**
- Modify: `elohim-app/src/app/shefa/components/shefa-dashboard/shefa-dashboard.component.ts:26,108`
- Modify: `elohim-app/src/app/shefa/components/shefa-dashboard/shefa-dashboard.component.spec.ts`

**Step 1: Update the component import and injection**

In `shefa-dashboard.component.ts`:

Replace line 26:
```typescript
// Old:
import { ComputeEventService } from '../../services/compute-event.service';
// New:
import { COMPUTE_EVENT } from '../../interfaces';
```

Replace lines 105-109 (constructor):
```typescript
// Old:
constructor(
    private readonly shefaCompute: ShefaComputeService,
    private readonly familyProtection: FamilyCommunityProtectionService,
    private readonly computeEvents: ComputeEventService
  ) {
// New:
constructor(
    private readonly shefaCompute: ShefaComputeService,
    private readonly familyProtection: FamilyCommunityProtectionService,
    private readonly computeEvents = inject(COMPUTE_EVENT)
  ) {
```

Note: Since this component uses constructor DI for the other services, add the `inject()` import and use it inline for the token. Alternatively, convert all three to `inject()` for consistency:

```typescript
private readonly shefaCompute = inject(ShefaComputeService);
private readonly familyProtection = inject(FamilyCommunityProtectionService);
private readonly computeEvents = inject(COMPUTE_EVENT);
```

Remove the constructor parameters and keep just:
```typescript
constructor() {
    this.mergedConfig = { ...DEFAULT_CONFIG, ...this.config };
}
```

**Step 2: Update the spec to provide via token**

In `shefa-dashboard.component.spec.ts`, replace any `{ provide: ComputeEventService, ...}` with `{ provide: COMPUTE_EVENT, ...}` and update the import.

**Step 3: Verify build**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Build succeeds.

**Step 4: Run tests**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts "shefa-dashboard"`
Expected: All dashboard tests pass.

**Step 5: Commit**

```bash
git add elohim-app/src/app/shefa/components/shefa-dashboard/ \
  elohim-app/src/app/shefa/interfaces/ \
  elohim-app/src/app/shefa/services/compute-event-api.service.ts
git commit -m "refactor(shefa): create IComputeEvent interface + thin ComputeEventApiService

- IComputeEvent interface + COMPUTE_EVENT injection token
- ComputeEventApiService persists via ECONOMIC_EVENT_FACTORY (HTTP bulk)
- ShefaDashboardComponent rewired to inject(COMPUTE_EVENT)"
```

---

### Task 8: Delete fat compute-event.service

**Files:**
- Delete: `elohim-app/src/app/shefa/services/compute-event.service.ts` (531 lines)
- Delete: `elohim-app/src/app/shefa/services/compute-event.service.spec.ts`

**Step 1: Verify no remaining imports**

Run: `cd /projects/elohim && grep -r "ComputeEventService\b" elohim-app/src/ --include="*.ts" -l | grep -v compute-event.service | grep -v compute-event-api`

Expected: No results.

**Step 2: Delete**

```bash
rm elohim-app/src/app/shefa/services/compute-event.service.ts
rm elohim-app/src/app/shefa/services/compute-event.service.spec.ts
```

**Step 3: Verify build**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Build succeeds.

**Step 4: Full test run**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20`
Expected: No new failures.

**Step 5: Commit**

```bash
git add -A elohim-app/src/app/shefa/services/compute-event.service.ts \
  elohim-app/src/app/shefa/services/compute-event.service.spec.ts
git commit -m "refactor(shefa): delete fat compute-event.service

Replaced by ComputeEventApiService + COMPUTE_EVENT token.
Persistence via ECONOMIC_EVENT_FACTORY (HTTP bulk endpoint).
-531 lines."
```

---

### Task 9: Final verification and cleanup

**Step 1: Full build**

Run: `cd /projects/elohim/elohim-app && pnpm run build`
Expected: Clean build.

**Step 2: Full test suite**

Run: `cd /projects/elohim/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -30`
Expected: No regressions.

**Step 3: Lint**

Run: `cd /projects/elohim/elohim-app && pnpm run lint`
Expected: Clean or pre-existing warnings only.

**Step 4: Verify scorecard**

Count remaining fat services:
Run: `cd /projects/elohim && grep -rl "callZome\b" elohim-app/src/app/shefa/ --include="*.ts" | grep -v spec | grep -v node_modules`

Expected: shefa-compute.service, flow-planning.service, economic.service, appreciation.service (4 remaining fat shefa services).

**Step 5: Commit cleanup if needed, then use superpowers:finishing-a-development-branch**
