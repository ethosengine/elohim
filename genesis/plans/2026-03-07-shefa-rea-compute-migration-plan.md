# Shefa / REA / Compute Migration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate 5 remaining shefa-pillar fat services (~4,870 lines of direct zome calls) to thin HTTP API boundaries, completing the shefa API surface.

**Architecture:** Each fat service is replaced by: (1) an interface + InjectionToken in `shefa/interfaces/`, (2) a thin API service calling doorway HTTP endpoints, (3) a doorway transparent proxy route, (4) an elohim-storage HTTP handler. UX/aggregation logic stays in Angular domain services; data access moves to Rust.

**Tech Stack:** Angular 19, Vitest, Rust (doorway: hyper, elohim-storage: custom HTTP + Diesel), TypeScript interfaces with DI tokens.

**Design Doc:** `genesis/plans/2026-03-07-shefa-rea-compute-migration-design.md`

---

## Reference Files

These files are the templates for all tasks below:

| Pattern | Reference File |
|---------|---------------|
| Thin API service | `elohim-app/src/app/shefa/services/economic-events-api.service.ts` (56 lines) |
| Interface + token | `elohim-app/src/app/shefa/interfaces/economic-event-factory.interface.ts` |
| Interface barrel | `elohim-app/src/app/shefa/interfaces/index.ts` |
| Doorway proxy route | `doorway/src/routes/economic_events.rs` |
| Doorway route registration | `doorway/src/routes/mod.rs` + `doorway/src/server/http.rs:1233-1252` |
| elohim-storage handler | `holochain/elohim-storage/src/api/economic_events.rs` |
| Test pattern | Vitest + TestBed, `vi.fn()` mocks, token injection |

---

## Phase 1: REA Consolidation (economic.service + appreciation.service → expand economic-events-api)

### Task 1: Extract appreciation and economic query types into the interface

**Files:**
- Modify: `elohim-app/src/app/shefa/interfaces/economic-event-factory.interface.ts`
- Read: `elohim-app/src/app/shefa/services/economic.service.ts` (for method signatures)
- Read: `elohim-app/src/app/shefa/services/appreciation.service.ts` (for AppreciationDisplay type)

**Step 1: Read fat services to extract public API signatures**

Read `economic.service.ts` and `appreciation.service.ts` fully. Identify every public method and its return type. These are the methods that must appear on the expanded interface.

From `economic.service.ts`:
- `getEventsByProvider(agentId: string): Observable<EconomicEvent[]>`
- `getEventsByReceiver(agentId: string): Observable<EconomicEvent[]>`
- `getEventsByAction(action: REAAction): Observable<EconomicEvent[]>`
- `getEventsByLamadType(lamadType: LamadEventType): Observable<EconomicEvent[]>`
- `createEconomicEvent(payload: CreateEconomicEventInput): Promise<EconomicEvent>`

From `appreciation.service.ts`:
- `getAppreciationsFor(appreciatedId: string): Observable<AppreciationDisplay[]>`
- `getAppreciationsBy(appreciatorId: string): Observable<AppreciationDisplay[]>`
- `createAppreciation(payload: CreateAppreciationInput): Promise<AppreciationDisplay>`

**Step 2: Add the new methods to IEconomicEventFactory**

Add the query and appreciation methods to the interface. Move `AppreciationDisplay` and `CreateAppreciationInput` type definitions into the interface file (locally defined, same pattern as `EconomicEvent`).

Add these to `IEconomicEventFactory`:
```typescript
// Query methods (from economic.service)
getEventsByProvider(agentId: string): Promise<EconomicEvent[]>;
getEventsByReceiver(agentId: string): Promise<EconomicEvent[]>;
getEventsByAction(action: string): Promise<EconomicEvent[]>;
getEventsByLamadType(lamadType: string): Promise<EconomicEvent[]>;

// Appreciation methods (from appreciation.service)
getAppreciationsFor(appreciatedId: string): Promise<AppreciationDisplay[]>;
getAppreciationsBy(appreciatorId: string): Promise<AppreciationDisplay[]>;
createAppreciation(payload: CreateAppreciationInput): Promise<AppreciationDisplay>;
```

Note: Changed from Observable to Promise — the API boundary uses `firstValueFrom()` pattern. Consumers that need Observables can wrap with `from()`.

**Step 3: Run lint to verify interface compiles**

Run: `cd elohim-app && pnpm exec tsc --noEmit --pretty 2>&1 | head -30`
Expected: Errors in `economic-events-api.service.ts` (doesn't implement new methods yet) — that's correct, we'll fix in Task 2.

**Step 4: Commit**

```bash
git add elohim-app/src/app/shefa/interfaces/economic-event-factory.interface.ts
git commit -m "feat(shefa): expand IEconomicEventFactory with query and appreciation methods"
```

---

### Task 2: Implement expanded methods in economic-events-api.service

**Files:**
- Modify: `elohim-app/src/app/shefa/services/economic-events-api.service.ts`

**Step 1: Write failing test**

Create test file `elohim-app/src/app/shefa/services/economic-events-api.service.spec.ts`:

```typescript
import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';
import { EconomicEventsApiService } from './economic-events-api.service';

describe('EconomicEventsApiService', () => {
  let service: EconomicEventsApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [EconomicEventsApiService, provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(EconomicEventsApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('getEventsByProvider calls GET with agent query param', async () => {
    const promise = service.getEventsByProvider('agent-1');
    const req = httpMock.expectOne('/api/v1/economic-events?provider=agent-1');
    expect(req.request.method).toBe('GET');
    req.flush([{ id: 'evt-1' }]);
    const result = await promise;
    expect(result).toEqual([{ id: 'evt-1' }]);
  });

  it('getAppreciationsFor calls GET with appreciated query param', async () => {
    const promise = service.getAppreciationsFor('content-1');
    const req = httpMock.expectOne('/api/v1/economic-events/appreciations?for=content-1');
    expect(req.request.method).toBe('GET');
    req.flush([{ id: 'apr-1' }]);
    const result = await promise;
    expect(result).toEqual([{ id: 'apr-1' }]);
  });

  it('createAppreciation calls POST', async () => {
    const input = { appreciationOf: 'evt-1', appreciatedBy: 'agent-1', quantityValue: 1, quantityUnit: 'kudos' };
    const promise = service.createAppreciation(input);
    const req = httpMock.expectOne('/api/v1/economic-events/appreciations');
    expect(req.request.method).toBe('POST');
    req.flush({ id: 'apr-new' });
    const result = await promise;
    expect(result).toEqual({ id: 'apr-new' });
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd elohim-app && pnpm exec vitest run --config vite.config.ts "economic-events-api.service" 2>&1 | tail -20`
Expected: FAIL — methods don't exist yet.

**Step 3: Implement the new methods**

Add to `economic-events-api.service.ts`:

```typescript
async getEventsByProvider(agentId: string): Promise<EconomicEvent[]> {
  return firstValueFrom(this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { provider: agentId } }));
}

async getEventsByReceiver(agentId: string): Promise<EconomicEvent[]> {
  return firstValueFrom(this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { receiver: agentId } }));
}

async getEventsByAction(action: string): Promise<EconomicEvent[]> {
  return firstValueFrom(this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { action } }));
}

async getEventsByLamadType(lamadType: string): Promise<EconomicEvent[]> {
  return firstValueFrom(this.http.get<EconomicEvent[]>('/api/v1/economic-events', { params: { lamadType } }));
}

async getAppreciationsFor(appreciatedId: string): Promise<AppreciationDisplay[]> {
  return firstValueFrom(this.http.get<AppreciationDisplay[]>('/api/v1/economic-events/appreciations', { params: { for: appreciatedId } }));
}

async getAppreciationsBy(appreciatorId: string): Promise<AppreciationDisplay[]> {
  return firstValueFrom(this.http.get<AppreciationDisplay[]>('/api/v1/economic-events/appreciations', { params: { by: appreciatorId } }));
}

async createAppreciation(payload: CreateAppreciationInput): Promise<AppreciationDisplay> {
  return firstValueFrom(this.http.post<AppreciationDisplay>('/api/v1/economic-events/appreciations', payload));
}
```

Import `AppreciationDisplay` and `CreateAppreciationInput` from the interface file.

**Step 4: Run tests to verify they pass**

Run: `cd elohim-app && pnpm exec vitest run --config vite.config.ts "economic-events-api.service" 2>&1 | tail -20`
Expected: PASS

**Step 5: Commit**

```bash
git add elohim-app/src/app/shefa/services/economic-events-api.service.ts elohim-app/src/app/shefa/services/economic-events-api.service.spec.ts
git commit -m "feat(shefa): implement query and appreciation methods in economic-events-api"
```

---

### Task 3: Migrate consumers from fat services to IEconomicEventFactory token

**Files:**
- Search: all files importing `EconomicService` or `AppreciationService` from shefa
- Modify: each consumer to inject `ECONOMIC_EVENT_FACTORY` token instead

**Step 1: Find all consumers**

Run: `cd elohim-app && grep -rn "EconomicService\|AppreciationService" src/app --include='*.ts' | grep -v '\.spec\.' | grep -v 'economic\.service\.ts' | grep -v 'appreciation\.service\.ts'`

**Step 2: For each consumer, update the injection**

Replace:
```typescript
import { EconomicService } from '@app/shefa/services/economic.service';
// ...
private readonly economic = inject(EconomicService);
```

With:
```typescript
import { ECONOMIC_EVENT_FACTORY, type IEconomicEventFactory } from '@app/shefa';
// ...
private readonly economic = inject(ECONOMIC_EVENT_FACTORY);
```

Note: method signatures changed from Observable to Promise. Consumers using `.subscribe()` need to switch to `await` or wrap with `from()`. Check each call site.

**Step 3: Update consumer tests**

For each consumer's `.spec.ts`, replace mock provider:
```typescript
// Before:
{ provide: EconomicService, useValue: mockEconomic }
// After:
{ provide: ECONOMIC_EVENT_FACTORY, useValue: mockEconomic }
```

**Step 4: Run full test suite**

Run: `cd elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -30`
Expected: PASS (or pre-existing failures only)

**Step 5: Commit**

```bash
git add -u elohim-app/src/app
git commit -m "refactor(shefa): migrate consumers from EconomicService/AppreciationService to ECONOMIC_EVENT_FACTORY"
```

---

### Task 4: Delete fat services

**Files:**
- Delete: `elohim-app/src/app/shefa/services/economic.service.ts`
- Delete: `elohim-app/src/app/shefa/services/economic.service.spec.ts`
- Delete: `elohim-app/src/app/shefa/services/appreciation.service.ts`
- Delete: `elohim-app/src/app/shefa/services/appreciation.service.spec.ts`
- Modify: `elohim-app/src/app/shefa/services/index.ts` (remove exports)

**Step 1: Delete fat services and their tests**

```bash
cd elohim-app
rm src/app/shefa/services/economic.service.ts src/app/shefa/services/economic.service.spec.ts
rm src/app/shefa/services/appreciation.service.ts src/app/shefa/services/appreciation.service.spec.ts
```

**Step 2: Remove from barrel exports**

Edit `elohim-app/src/app/shefa/services/index.ts` — remove any export lines for `EconomicService` or `AppreciationService`.

**Step 3: Run full test suite to verify nothing breaks**

Run: `cd elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -30`
Expected: PASS

**Step 4: Commit**

```bash
git add -u elohim-app/src/app/shefa
git commit -m "refactor(shefa): delete fat economic.service and appreciation.service (-783 lines)"
```

---

### Task 5: Add elohim-storage endpoints for appreciation and query operations

**Files:**
- Modify: `holochain/elohim-storage/src/api/economic_events.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs` (if new module needed)

**Step 1: Read the existing economic_events.rs handler**

Read `holochain/elohim-storage/src/api/economic_events.rs` to understand the dispatch pattern.

**Step 2: Add query-by-provider/receiver/action routes**

In the `handle()` match block, add:
```rust
(&Method::GET, "") => {
    // Check query params: provider, receiver, action, lamadType
    let query = parse_query(req.uri().query());
    if let Some(provider) = query.get("provider") {
        handle_events_by_provider(provider, pool, ctx).await
    } else if let Some(receiver) = query.get("receiver") {
        handle_events_by_receiver(receiver, pool, ctx).await
    } else if let Some(action) = query.get("action") {
        handle_events_by_action(action, pool, ctx).await
    } else if let Some(lamad_type) = query.get("lamadType") {
        handle_events_by_lamad_type(lamad_type, pool, ctx).await
    } else {
        handle_list(req, pool, ctx).await
    }
}
```

**Step 3: Add appreciation sub-routes**

```rust
(&Method::GET, p) if p.starts_with("appreciations") => {
    let query = parse_query(req.uri().query());
    if let Some(for_id) = query.get("for") {
        handle_appreciations_for(for_id, pool, ctx).await
    } else if let Some(by_id) = query.get("by") {
        handle_appreciations_by(by_id, pool, ctx).await
    } else {
        handle_list_appreciations(pool, ctx).await
    }
}
(&Method::POST, "appreciations") => handle_create_appreciation(req, pool, ctx).await,
```

**Step 4: Build and test**

Run: `cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20`
Expected: Compiles (handler implementations may need to be stubbed initially)

**Step 5: Commit**

```bash
git add holochain/elohim-storage/src/api/economic_events.rs
git commit -m "feat(storage): add appreciation and query-by-agent endpoints to economic-events API"
```

---

## Phase 2: Custodian Services

### Task 6: Define ICustodianMetrics interface and token

**Files:**
- Create: `elohim-app/src/app/shefa/interfaces/custodian-metrics.interface.ts`
- Modify: `elohim-app/src/app/shefa/interfaces/index.ts`
- Read: `elohim-app/src/app/elohim/services/shefa.service.ts` (for method signatures and types)

**Step 1: Read shefa.service.ts to extract public API**

Read the full service. Extract every public method signature and the `CustodianMetrics` type.

**Step 2: Create the interface file**

Follow the pattern from `economic-event-factory.interface.ts`:
- Define `CustodianMetrics` type locally in the interface file
- Define `ICustodianMetrics` interface with all public methods (using Promise, not Observable)
- Create `CUSTODIAN_METRICS` InjectionToken with factory pointing to future `CustodianMetricsApiService`

Key methods to include:
```typescript
export interface ICustodianMetrics {
  getMetrics(custodianId: string): Promise<CustodianMetrics>;
  listAllMetrics(): Promise<CustodianMetrics[]>;
  getRankedByHealth(): Promise<CustodianMetrics[]>;
  getRankedBySpeed(): Promise<CustodianMetrics[]>;
  getRankedByReputation(): Promise<CustodianMetrics[]>;
  getAvailableCustodians(): Promise<CustodianMetrics[]>;
  getAlerts(): Promise<CustodianMetrics[]>;
  getRecommendations(custodianId: string): Promise<CustodianRecommendation[]>;
  reportMetrics(metrics: ReportMetricsInput): Promise<void>;
}
```

**Step 3: Add to barrel exports**

Add to `elohim-app/src/app/shefa/interfaces/index.ts`:
```typescript
export type { ICustodianMetrics, CustodianMetrics } from './custodian-metrics.interface';
export { CUSTODIAN_METRICS } from './custodian-metrics.interface';
```

**Step 4: Commit**

```bash
git add elohim-app/src/app/shefa/interfaces/custodian-metrics.interface.ts elohim-app/src/app/shefa/interfaces/index.ts
git commit -m "feat(shefa): define ICustodianMetrics interface and CUSTODIAN_METRICS token"
```

---

### Task 7: Define IDataProtection interface and token

**Files:**
- Create: `elohim-app/src/app/shefa/interfaces/data-protection.interface.ts`
- Modify: `elohim-app/src/app/shefa/interfaces/index.ts`
- Read: `elohim-app/src/app/shefa/services/family-community-protection.service.ts` (for method signatures)

**Step 1: Read family-community-protection.service.ts**

Extract the public API: trust graph, geographic distribution, redundancy views.

**Step 2: Create the interface file**

```typescript
export interface IDataProtection {
  getTrustGraph(stewardId: string): Promise<TrustGraphView>;
  getGeographicDistribution(stewardId: string): Promise<GeographicDistributionView>;
  getRedundancyStatus(stewardId: string): Promise<RedundancyView>;
  getProtectionSummary(stewardId: string): Promise<ProtectionSummary>;
}
```

Define `TrustGraphView`, `GeographicDistributionView`, `RedundancyView`, `ProtectionSummary` locally.

Create `DATA_PROTECTION` InjectionToken.

**Step 3: Add to barrel exports**

**Step 4: Commit**

```bash
git add elohim-app/src/app/shefa/interfaces/data-protection.interface.ts elohim-app/src/app/shefa/interfaces/index.ts
git commit -m "feat(shefa): define IDataProtection interface and DATA_PROTECTION token"
```

---

### Task 8: Implement custodian-metrics-api.service

**Files:**
- Create: `elohim-app/src/app/shefa/services/custodian-metrics-api.service.ts`
- Create: `elohim-app/src/app/shefa/services/custodian-metrics-api.service.spec.ts`

**Step 1: Write failing test**

Same pattern as Task 2 — use `HttpTestingController`, test that each method calls the right endpoint.

Key test cases:
- `getMetrics('cust-1')` → GET `/api/v1/custodians/metrics/cust-1`
- `getRankedByHealth()` → GET `/api/v1/custodians/metrics?rank=health`
- `getAlerts()` → GET `/api/v1/custodians/metrics?alerts=true`
- `reportMetrics(input)` → POST `/api/v1/custodians/metrics`

**Step 2: Run test to verify it fails**

**Step 3: Implement service**

Follow `economic-events-api.service.ts` pattern exactly: `@Injectable({ providedIn: 'root' })`, `inject(HttpClient)`, `firstValueFrom()`.

**Step 4: Run tests to verify they pass**

**Step 5: Commit**

```bash
git add elohim-app/src/app/shefa/services/custodian-metrics-api.service.*
git commit -m "feat(shefa): implement custodian-metrics-api.service with tests"
```

---

### Task 9: Implement data-protection-api.service

**Files:**
- Create: `elohim-app/src/app/shefa/services/data-protection-api.service.ts`
- Create: `elohim-app/src/app/shefa/services/data-protection-api.service.spec.ts`

Same pattern as Task 8 but for data protection endpoints:
- `getTrustGraph('steward-1')` → GET `/api/v1/custodians/protection/steward-1/trust-graph`
- `getGeographicDistribution('steward-1')` → GET `/api/v1/custodians/protection/steward-1/geographic`
- `getRedundancyStatus('steward-1')` → GET `/api/v1/custodians/protection/steward-1/redundancy`
- `getProtectionSummary('steward-1')` → GET `/api/v1/custodians/protection/steward-1/summary`

**Step 1-5:** Same TDD flow as Task 8.

**Commit:**
```bash
git add elohim-app/src/app/shefa/services/data-protection-api.service.*
git commit -m "feat(shefa): implement data-protection-api.service with tests"
```

---

### Task 10: Migrate consumers and delete fat custodian services

**Files:**
- Search: all files importing `ShefarService` (from elohim pillar) or `FamilyCommunityProtectionService`
- Modify: each consumer
- Delete: `elohim-app/src/app/elohim/services/shefa.service.ts` + spec
- Delete: `elohim-app/src/app/shefa/services/family-community-protection.service.ts` + spec
- Modify: barrel exports

**Step 1: Find consumers**

```bash
cd elohim-app && grep -rn "ShefarService\|FamilyCommunityProtectionService\|ShefaService" src/app --include='*.ts' | grep -v '\.spec\.' | grep -v 'shefa\.service\.ts' | grep -v 'family-community-protection\.service\.ts'
```

Note: The service in elohim pillar may be named `ShefaService` — check the actual class name.

**Step 2: Update each consumer to inject the token**

Replace direct service injection with `CUSTODIAN_METRICS` or `DATA_PROTECTION` tokens.

**Step 3: Update consumer tests**

**Step 4: Delete fat services and update barrel exports**

**Step 5: Run full test suite**

**Step 6: Commit**

```bash
git add -u elohim-app/src/app
git commit -m "refactor(shefa): migrate consumers to custodian API boundaries, delete fat services (-1,081 lines)"
```

---

### Task 11: Add doorway proxy routes for custodian endpoints

**Files:**
- Create: `doorway/src/routes/custodians.rs`
- Modify: `doorway/src/routes/mod.rs`
- Modify: `doorway/src/server/http.rs`

**Step 1: Create custodians.rs**

Copy `economic_events.rs` verbatim, rename handler to `handle_custodians_api_request`. Same transparent proxy pattern.

**Step 2: Register in mod.rs**

Add `pub mod custodians;` and `pub use custodians::handle_custodians_api_request;`

**Step 3: Add route match in http.rs**

Add before the `// Not found` match arm (around line 1254):
```rust
// Custodian metrics and data protection
(_, p) if p.starts_with("/api/v1/custodians") => {
    return Ok(to_boxed(
        routes::handle_custodians_api_request(req, Arc::clone(&state), p).await,
    ));
}
```

**Step 4: Build doorway**

Run: `cd doorway && RUSTFLAGS="" cargo build --release 2>&1 | tail -20`
Expected: Compiles

**Step 5: Commit**

```bash
git add doorway/src/routes/custodians.rs doorway/src/routes/mod.rs doorway/src/server/http.rs
git commit -m "feat(doorway): add transparent proxy route for /api/v1/custodians/*"
```

---

### Task 12: Add elohim-storage custodian API handlers

**Files:**
- Create: `holochain/elohim-storage/src/api/custodians.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs`

**Step 1: Create custodians.rs**

Follow the dispatch pattern from `economic_events.rs`:
```rust
pub async fn handle(req, method, resource_path, pool, ctx) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');
    match (&method, path) {
        // Metrics endpoints
        (&Method::GET, "metrics") => handle_list_metrics(pool, ctx).await,
        (&Method::GET, p) if p.starts_with("metrics/") => {
            let id = p.trim_start_matches("metrics/");
            handle_get_metrics(id, pool, ctx).await
        }
        (&Method::POST, "metrics") => handle_report_metrics(req, pool, ctx).await,
        // Protection endpoints
        (&Method::GET, p) if p.starts_with("protection/") => handle_protection(p, pool, ctx).await,
        _ => Err(StorageError::NotFound),
    }
}
```

Initial implementation can return stub JSON responses — the full Diesel queries come later when the DB schema is in place.

**Step 2: Register in mod.rs**

**Step 3: Build**

Run: `cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20`

**Step 4: Commit**

```bash
git add holochain/elohim-storage/src/api/custodians.rs holochain/elohim-storage/src/api/mod.rs
git commit -m "feat(storage): add custodian metrics and data protection API handlers"
```

---

## Phase 3: Compute Dashboard

### Task 13: Define IComputeDashboard interface and token

**Files:**
- Create: `elohim-app/src/app/shefa/interfaces/compute-dashboard.interface.ts`
- Modify: `elohim-app/src/app/shefa/interfaces/index.ts`
- Read: `elohim-app/src/app/shefa/services/shefa-compute.service.ts` (for SheafaDashboardState type)

**Step 1: Read shefa-compute.service.ts**

Extract `SheafaDashboardState` and all its nested types. This is the response shape that Rust will assemble.

**Step 2: Create the interface**

```typescript
export interface IComputeDashboard {
  getDashboard(): Promise<SheafaDashboardState>;
  getDashboardForGovernanceLevel(level: GovernanceLevel): Promise<SheafaDashboardState>;
  refreshDashboard(): Promise<SheafaDashboardState>;
}
```

The key insight: the dashboard is a single pre-aggregated response from Rust. Angular just fetches and displays.

**Step 3: Add to barrel exports**

**Step 4: Commit**

```bash
git add elohim-app/src/app/shefa/interfaces/compute-dashboard.interface.ts elohim-app/src/app/shefa/interfaces/index.ts
git commit -m "feat(shefa): define IComputeDashboard interface and COMPUTE_DASHBOARD token"
```

---

### Task 14: Implement compute-dashboard-api.service

**Files:**
- Create: `elohim-app/src/app/shefa/services/compute-dashboard-api.service.ts`
- Create: `elohim-app/src/app/shefa/services/compute-dashboard-api.service.spec.ts`

**Step 1: Write failing test**

Test cases:
- `getDashboard()` → GET `/api/v1/compute/dashboard`
- `getDashboardForGovernanceLevel('community')` → GET `/api/v1/compute/dashboard?level=community`
- `refreshDashboard()` → POST `/api/v1/compute/dashboard/refresh`

**Step 2-5:** Standard TDD flow.

**Commit:**
```bash
git add elohim-app/src/app/shefa/services/compute-dashboard-api.service.*
git commit -m "feat(shefa): implement compute-dashboard-api.service with tests"
```

---

### Task 15: Migrate consumers and delete shefa-compute.service

**Files:**
- Search: all files importing `ShefaComputeService`
- Delete: `elohim-app/src/app/shefa/services/shefa-compute.service.ts` + spec
- Modify: barrel exports

**Step 1: Find consumers**
**Step 2: Update to inject COMPUTE_DASHBOARD token**
**Step 3: Run tests**
**Step 4: Delete fat service**
**Step 5: Commit**

```bash
git add -u elohim-app/src/app
git commit -m "refactor(shefa): migrate consumers to COMPUTE_DASHBOARD, delete shefa-compute.service (-2,056 lines)"
```

---

### Task 16: Add doorway and elohim-storage compute dashboard endpoints

**Files:**
- Create: `doorway/src/routes/compute.rs` (transparent proxy)
- Modify: `doorway/src/routes/mod.rs`, `doorway/src/server/http.rs`
- Create: `holochain/elohim-storage/src/api/compute.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs`

**Step 1: Doorway proxy**

Same pattern as `economic_events.rs`. Handler: `handle_compute_request`.
Route match: `p.starts_with("/api/v1/compute")`

**Step 2: elohim-storage handler**

```rust
pub async fn handle(req, method, resource_path, pool, ctx) -> Result<...> {
    match (&method, resource_path.trim_start_matches('/')) {
        (&Method::GET, "dashboard") => handle_dashboard(req, pool, ctx).await,
        (&Method::POST, "dashboard/refresh") => handle_refresh(pool, ctx).await,
        _ => Err(StorageError::NotFound),
    }
}
```

The `handle_dashboard` handler aggregates:
- Compute metrics from performance tables
- Allocation snapshots from resource tables
- Custodian health (can call custodian service internally)
- Token balances from economic event ledger
- Constitutional limit status

Returns a single `SheafaDashboardState` JSON response.

**Step 3: Build both**

```bash
cd doorway && RUSTFLAGS="" cargo build --release
cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

**Step 4: Commit**

```bash
git add doorway/src/routes/compute.rs doorway/src/routes/mod.rs doorway/src/server/http.rs holochain/elohim-storage/src/api/compute.rs holochain/elohim-storage/src/api/mod.rs
git commit -m "feat(doorway,storage): add compute dashboard proxy and aggregation endpoint"
```

---

## Phase 4: Flow Planning Shell

### Task 17: Define IFlowPlanning interface and token

**Files:**
- Create: `elohim-app/src/app/shefa/interfaces/flow-planning.interface.ts`
- Modify: `elohim-app/src/app/shefa/interfaces/index.ts`
- Read: `elohim-app/src/app/shefa/services/flow-planning.service.ts` (for method signatures)

**Step 1: Read flow-planning.service.ts**

All methods throw NOT_IMPLEMENTED_ERROR. Extract the complete interface specification — it's a design document.

**Step 2: Create the interface**

Group methods by domain:
```typescript
export interface IFlowPlanning {
  // Plan management
  createPlan(input: CreatePlanInput): Promise<FlowPlan>;
  updatePlan(planId: string, updates: Partial<FlowPlan>): Promise<FlowPlan>;
  getPlan(planId: string): Promise<FlowPlan>;
  archivePlan(planId: string): Promise<void>;

  // Budget management
  createBudget(planId: string, input: CreateBudgetInput): Promise<Budget>;
  getBudgetVsActual(budgetId: string): Promise<BudgetComparison>;

  // Goal tracking
  createGoal(planId: string, input: CreateGoalInput): Promise<Goal>;
  evaluateGoalProgress(goalId: string): Promise<GoalProgress>;

  // Projections
  getFinancialHealthProjection(planId: string): Promise<HealthProjection>;

  // Dashboard
  getPlanningOverview(agentId: string): Promise<PlanningOverview>;
}
```

**Step 3: Add to barrel exports**

**Step 4: Commit**

```bash
git add elohim-app/src/app/shefa/interfaces/flow-planning.interface.ts elohim-app/src/app/shefa/interfaces/index.ts
git commit -m "feat(shefa): define IFlowPlanning interface and FLOW_PLANNING token"
```

---

### Task 18: Implement flow-planning-api.service and delete fat service

**Files:**
- Create: `elohim-app/src/app/shefa/services/flow-planning-api.service.ts`
- Create: `elohim-app/src/app/shefa/services/flow-planning-api.service.spec.ts`
- Delete: `elohim-app/src/app/shefa/services/flow-planning.service.ts` + spec

**Step 1: Write failing test**

Test that each method calls the right endpoint. Since the backend will return 501, tests just verify HTTP calls are made correctly.

**Step 2: Implement thin API service**

Each method calls `/api/v1/flow-planning/*` via HTTP.

**Step 3: Find consumers of FlowPlanningService, update to inject FLOW_PLANNING token**

**Step 4: Delete fat service**

**Step 5: Run tests, commit**

```bash
git add -u elohim-app/src/app/shefa
git commit -m "refactor(shefa): migrate flow-planning to API boundary, delete fat service (-983 lines)"
```

---

### Task 19: Add doorway and elohim-storage flow-planning shell routes

**Files:**
- Create: `doorway/src/routes/flow_planning.rs`
- Modify: `doorway/src/routes/mod.rs`, `doorway/src/server/http.rs`
- Create: `holochain/elohim-storage/src/api/flow_planning.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs`

**Step 1: Doorway proxy**

Same transparent proxy pattern. Route: `/api/v1/flow-planning`

**Step 2: elohim-storage shell**

All handlers return 501 Not Implemented:
```rust
fn not_implemented(feature: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(
            r#"{{"error": "Not implemented", "feature": "{feature}"}}"#
        ))))
        .unwrap()
}
```

**Step 3: Build both, commit**

```bash
git add doorway/src/routes/flow_planning.rs doorway/src/routes/mod.rs doorway/src/server/http.rs holochain/elohim-storage/src/api/flow_planning.rs holochain/elohim-storage/src/api/mod.rs
git commit -m "feat(doorway,storage): add flow-planning shell routes (501 Not Implemented)"
```

---

## Final Task: Update barrel exports and verify

### Task 20: Clean up barrel exports and run full verification

**Files:**
- Modify: `elohim-app/src/app/shefa/services/index.ts`
- Modify: `elohim-app/src/app/shefa/index.ts`
- Modify: `elohim-app/src/app/elohim/services/index.ts` (remove shefa.service export)

**Step 1: Update all barrel files**

Remove exports for deleted services. Add exports for new API services.

**Step 2: Run full Angular test suite**

Run: `cd elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -30`

**Step 3: Run TypeScript compiler check**

Run: `cd elohim-app && npx tsc --noEmit 2>&1 | tail -30`

**Step 4: Run lint**

Run: `cd elohim-app && pnpm run lint 2>&1 | tail -30`

**Step 5: Build doorway**

Run: `cd doorway && RUSTFLAGS="" cargo build --release 2>&1 | tail -10`

**Step 6: Build elohim-storage**

Run: `cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -10`

**Step 7: Final commit**

```bash
git add -u
git commit -m "chore(shefa): clean up barrel exports after shefa API migration"
```

---

## Summary

| Phase | Tasks | Fat Lines Eliminated | New Thin Lines |
|-------|-------|---------------------|----------------|
| 1: REA Consolidation | 1-5 | ~750 | ~80 |
| 2: Custodian Services | 6-12 | ~1,080 | ~250 |
| 3: Compute Dashboard | 13-16 | ~2,056 | ~100 |
| 4: Flow Planning | 17-19 | ~983 | ~50 |
| Final | 20 | — | — |
| **Total** | **20 tasks** | **~4,870** | **~480** |

After completion: 16 fat services remain (~7,130 lines), 14 thin API services (was 10).
