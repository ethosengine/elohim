# Shefa API Boundary Cleanup — Design

**Date:** 2026-03-08
**Pillar:** shefa
**Goal:** Eliminate 4 fat services (~2,580 lines), completing shefa's data boundary migration to thin API pattern.

## Context

8 of 30 data-boundary services have been migrated to the thin API + interface + injection token pattern (stewardship, presence, exchange-api, economic-events-api, stewarded-resources-api, collective, storage-api, projection-api). Two fat/thin duplicate pairs remain where the thin API exists but the fat service hasn't been deleted. Two more fat services need new thin pairs.

## Targets

| Service | Lines | Consumers | Action |
|---------|-------|-----------|--------|
| exchange.service | 1,341 | 0 | Delete (EXCHANGE token already defaults to thin) |
| economic-event-bridge.service | 314 | 0 | Delete (orphaned, superseded by HTTP API) |
| economic-event-factory.service | 394 | 1 | Rewire TransactionImportService to token, delete |
| compute-event.service | 531 | 1 | Extract UX logic to thin service, delete fat |

## Phase 1: Trivial Deletes (~1,655 lines)

**exchange.service** — 0 consumers. `EXCHANGE` injection token already defaults to `ExchangeApiService`. Delete service + spec + clean barrel exports.

**economic-event-bridge.service** — 0 consumers. Orphaned pre-API bridge in `banking-bridge/`. Superseded by `EconomicEventsApiService`. Delete service + spec + clean barrel exports.

## Phase 2: Rewire + Delete (~394 lines)

**economic-event-factory.service** — 1 consumer (`TransactionImportService`) directly injects the fat class instead of using the `ECONOMIC_EVENT_FACTORY` token. The token already defaults to `EconomicEventsApiService`. Rewire the single consumer to use the token, update its spec, delete fat service + spec.

## Phase 3: Compute-Event Refactor (~531 lines restructured)

**compute-event.service** calls `create_economic_events_batch` zome directly. The UX logic (interval sampling, metric conversion, pricing config) stays in Angular. The persistence call moves to the existing `/api/v1/economic-events/bulk` endpoint via `ECONOMIC_EVENT_FACTORY`.

Steps:
1. Create `IComputeEvent` interface in `shefa/interfaces/`
2. Create `COMPUTE_EVENT` injection token
3. Create `compute-event-api.service.ts` — keeps UX logic, delegates persistence to `ECONOMIC_EVENT_FACTORY`
4. Rewire `ShefaDashboardComponent` to use `COMPUTE_EVENT` token
5. Delete fat service + spec

## Scorecard After Completion

| Metric | Before | After |
|--------|--------|-------|
| Thin API services | 8 | 12 |
| Fat services | 22 | 18 |
| Lines removed | — | ~2,580 |
| Shefa fat remaining | ~8 | ~4 |

## P2P-First Note

No new Rust endpoints needed. Compute events ARE economic events — the existing bulk endpoint in elohim-storage handles persistence. The Angular thin service is UX-layer only: interval sampling, pricing config, metric conversion. The DHT remains the truth layer.
