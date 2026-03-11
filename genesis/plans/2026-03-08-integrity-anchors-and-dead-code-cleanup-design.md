# Integrity Anchors & Dead Code Cleanup — Design

## Context

The fat service migration (28→13 services) revealed that the remaining 13 services fall into three categories:
1. **Dead code** — services with zero consumers or orphaned infrastructure
2. **Correctly designed hybrids** — services with one zome call (integrity anchor) + HTTP orchestration
3. **Integrity-heavy services** — services making many zome calls (future work)

This batch addresses categories 1 and 2.

## Principle

Holochain's role is **integrity**: identity, proofs, attestations, addressing, and currency/crypto. The DHT's performance ceiling aligns with human-scale compute — it's a feature, not a bug. Services that touch Holochain should make their integrity role visible.

## Tier 1: Dead Code Deletion

### CircuitBreakerService (333 lines + spec)
- Zero production consumers in elohim-app
- Used only in doorway-app (separate Angular app)
- Not barrel-exported
- **Action**: Delete service + spec

### OfflineOperationQueueService (511 lines + spec)
- `enqueue()` has zero callers — the queue is never populated
- Only consumer (`HolochainAvailabilityUiComponent`) reads queue size (always 0) and calls `syncAll()` (no-ops on empty queue)
- **Action**: Delete service + spec, remove from availability UI component

### ComputeEventApiService — CORRECTION
- Initially flagged as "zero consumers" but **shefa-dashboard.component.ts injects COMPUTE_EVENT token** and calls `initializeEventEmission()`
- Already fully HTTP (no zome calls)
- **Action**: Remove from fat list. Already migrated. No work needed.

**Tier 1 net deletion: ~1,355 lines (2 services + 2 specs + component cleanup)**

## Tier 2: Integrity Anchor Extraction

Three services have a single zome call each — the cryptographic proof that anchors the service's data to the DHT's integrity guarantees. Extract these into a dedicated `integrity/` module.

### Architecture

```
elohim/integrity/
  integrity-anchor.interface.ts   # IIntegrityAnchor<TInput, TOutput>
  blob-metadata.anchor.ts         # content_store.get_blobs_by_content_id
  federation-registry.anchor.ts   # infrastructure.get_doorways_by_region
  node-registry.anchor.ts         # node_registry_coordinator.get_my_nodes
  index.ts                        # barrel exports
```

Each anchor is ~20-30 lines: typed input, one zome call, typed output. Nothing else.

The existing services (BlobManager, DoorwayRegistry, RunningContext) then inject the anchor instead of calling zomes directly. This makes the architecture self-documenting: when you open `integrity/`, you see exactly what Holochain is responsible for.

### IIntegrityAnchor Interface

```typescript
export interface IIntegrityAnchor<TInput, TOutput> {
  readonly zomeName: string;
  readonly fnName: string;
  verify(input: TInput): Promise<TOutput>;
}
```

`verify()` not `fetch()` or `get()` — the naming reinforces that this is a cryptographic verification, not a data fetch.

### Anchor: blob-metadata

- **Zome**: `content_store`
- **Function**: `get_blobs_by_content_id`
- **Input**: `{ content_id: string }`
- **Output**: `BlobsForContentOutput`
- **Consumer**: BlobManagerService (line 632)

### Anchor: federation-registry

- **Zome**: `infrastructure`
- **Function**: `get_doorways_by_region`
- **Input**: `string` (region, currently always `'global'`)
- **Output**: `DoorwayInfo[]`
- **Consumer**: DoorwayRegistryService (line 381)

### Anchor: node-registry

- **Zome**: `node_registry_coordinator`
- **Function**: `get_my_nodes`
- **Input**: `null`
- **Output**: `RawRegisteredNode[]`
- **Consumer**: RunningContextService (line 225)

## Scorecard After This Batch

| Metric | Before | After |
|--------|--------|-------|
| Fat services | 13 | 10 (−3: CircuitBreaker, OfflineQueue deleted; ComputeEventApi reclassified) |
| Dead code eliminated | ~10,000 | ~11,355 (+1,355) |
| Integrity anchors | 0 | 3 (new module) |
| Services with visible integrity role | 0 | 3 |
