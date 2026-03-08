# Shefa / REA / Compute Migration Design

**Date:** 2026-03-07
**Scope:** Migrate 5 remaining shefa-pillar fat services (~4,870 lines) to thin HTTP API boundaries

## Context

The shefa pillar has 5 fat services making direct Holochain zome calls. Four thin API services already exist (compute-event-api, exchange-api, economic-events-api, stewarded-resources-api) proving the migration pattern. This design completes the shefa API boundary.

## Layering Model

```
Component -> Domain Service (can cross-inject) -> API Service (strict boundary) -> Doorway -> elohim-storage
```

- **API services** are strict HTTP boundaries. One interface, one injection token, no cross-dependencies.
- **Domain services** consume API services and MAY cross-inject each other for cross-cutting concerns (e.g., data-protection needs custodian health status).
- Components never call API services directly — domain services mediate.

## New API Services (Angular)

| New Service | Replaces | Interface | Doorway Route | Est. Lines |
|-------------|----------|-----------|---------------|------------|
| compute-dashboard-api.service | shefa-compute (2,056) | IComputeDashboard | /api/v1/compute/dashboard | ~150 |
| custodian-metrics-api.service | shefa (480) | ICustodianMetrics | /api/v1/custodians/metrics | ~120 |
| data-protection-api.service | family-community-protection (601) | IDataProtection | /api/v1/custodians/protection | ~130 |
| flow-planning-api.service | flow-planning (983) | IFlowPlanning | /api/v1/flow-planning/* | ~80 |

## Expanded API Service

| Service | Change |
|---------|--------|
| economic-events-api.service (56) | Absorb appreciation methods + economic.service queries (by-provider, by-receiver, by-action) |

## Rust Endpoints (elohim-storage + doorway proxy)

| Endpoint | Responsibility |
|----------|---------------|
| /api/v1/compute/dashboard | Pre-aggregated SheafaDashboardState — metrics, allocations, token balances, constitutional limits |
| /api/v1/custodians/metrics | Custodian health, reputation, ranked queries, alerts |
| /api/v1/custodians/protection | Trust graph, geographic distribution, redundancy/recovery |
| /api/v1/flow-planning/* | Shell routes (501 until implemented) |
| /api/v1/economic-events/* | Extend with appreciation + query-by-provider/receiver/action |

## Custodian Cross-Cutting Pattern

custodian-metrics-api and data-protection-api are separate API boundaries but closely related. At the domain service layer:

- Both share generated types from @elohim/storage-client (Rust view types)
- Domain services that consume them MAY inject both for cross-cutting queries
- They do NOT merge — different audiences (operator vs. user), different refresh cadences

## Migration Phases

### Phase 1: REA Consolidation
**Target:** economic.service (453) + appreciation.service (~300) -> expand economic-events-api
**Why first:** Smallest scope, proves the merge pattern.
**Angular work:**
- Add appreciation methods (getAppreciationsFor, getAppreciationsBy, createAppreciation) to IEconomicEventFactory
- Add query methods (getByProvider, getByReceiver, getByAction) to IEconomicEventFactory
- Expand economic-events-api.service to call new doorway endpoints
- Update all consumers to inject IEconomicEventFactory instead of EconomicService/AppreciationService
- Delete economic.service and appreciation.service
**Rust work:**
- Add appreciation endpoints to elohim-storage economic-events controller
- Add query-by-provider/receiver/action endpoints
- Add doorway proxy routes
**Lines eliminated:** ~750

### Phase 2: Custodian Services
**Target:** shefa.service (480) + family-community-protection.service (601)
**Why second:** Related services, share custodian types. Builds infrastructure for Phase 3.
**Angular work:**
- Define ICustodianMetrics interface (health, reputation, ranked queries, alerts, recommendations)
- Define IDataProtection interface (trust graph, geographic distribution, redundancy/recovery)
- Create custodian-metrics-api.service calling /api/v1/custodians/metrics
- Create data-protection-api.service calling /api/v1/custodians/protection
- Update consumers, delete fat services
**Rust work:**
- Define CustodianMetricsView and DataProtectionView in elohim-storage
- Build custodian metrics controller (aggregate from metrics zome)
- Build data protection controller (trust graph, geographic, redundancy)
- Add doorway proxy routes for /api/v1/custodians/*
**Lines eliminated:** ~1,080

### Phase 3: Compute Dashboard
**Target:** shefa-compute.service (2,056)
**Why third:** Largest service. Depends on custodian endpoints from Phase 2.
**Angular work:**
- Define IComputeDashboard interface returning SheafaDashboardState
- Create compute-dashboard-api.service — single call to /api/v1/compute/dashboard
- UX logic (filtering, sorting, display transforms) stays in Angular domain service
- Delete shefa-compute.service
**Rust work:**
- Build /api/v1/compute/dashboard in elohim-storage
- Aggregation logic moves to Rust: compute metrics + allocations + custodian health + token balances + constitutional limits
- Single pre-assembled response (read-heavy, assemble once in Rust vs. N calls from Angular)
- Add doorway proxy route
**Lines eliminated:** ~2,056

### Phase 4: Flow Planning Shell
**Target:** flow-planning.service (983)
**Why last:** All methods are NOT_IMPLEMENTED_ERROR. Low risk stub migration.
**Angular work:**
- Define IFlowPlanning interface (plan CRUD, budgets, goals, projections, scenarios, cadence, dashboard)
- Create flow-planning-api.service with HTTP calls to /api/v1/flow-planning/*
- Delete flow-planning.service
**Rust work:**
- Shell routes returning 501 Not Implemented
- Interface contract ready for future implementation
**Lines eliminated:** ~983

## Net Impact

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Fat services | 21 | 16 | -5 |
| Thin API services | 10 | 14 | +4 (plus 1 expanded) |
| Fat lines | ~12,000 | ~7,130 | -4,870 |
| New thin lines | — | ~480 | +480 |
| Shefa API coverage | partial | complete | All shefa zome calls behind HTTP |

## Decision Log

1. flow-planning: migrate interface to API shell (option c) — establish boundary now, fill later
2. appreciation: fold into economic-events-api (option c) — appreciation is a specialized economic event
3. shefa-compute: push aggregation to Rust (option c) — read-heavy dashboard, assemble once in Rust
4. shefa + family-community-protection: separate API surfaces (option b) — different audiences, but domain services may cross-inject for cross-cutting concerns
