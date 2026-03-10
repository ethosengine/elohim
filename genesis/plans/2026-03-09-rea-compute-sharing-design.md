# REA Compute Sharing Design: Paired Commitments + Real-Time Settlement

**Date**: 2026-03-09
**Status**: Approved
**Depends on**: testnet-lifecycle (completed this sprint)
**Next**: Mutual credit balance tracking, ServiceRequest/Offer marketplace

## Context

The testnet lifecycle emits CoordinationEnvelopes as JSONL files that *look like* REA records but bypass the actual economic infrastructure. This design wires the lifecycle to real elohim-storage persistence — Commitments on spawn, EconomicEvents on settlement, paired give/take actions, CPU mutual credit denomination.

The mental model: AWS billing between peers. Same accounting granularity (cpu-seconds, megabytes), but multi-peer, mutual, and auditable on the DHT.

## Design Decisions

1. **Commitment + Event (not full marketplace)**: Skip ServiceRequest/Offer matching — Matthew is requesting from known community members. The Commitment→Event fulfillment chain is the core REA pattern.
2. **Real-time to elohim-storage**: POST Commitments and Events as they happen (not JSONL-then-seed). The protocol is real, the economics should be real.
3. **Assume storage is running**: Health check gates the scenario. Operator starts elohim-storage separately at localhost:8090.
4. **Paired Commitments (give/take)**: Matthew commits to `take` compute. Each persona commits to `give` compute. Balanced ledger from day one.
5. **CPU mutual credit**: New MediumOfExchange — `{ code: 'CPU', exchangeType: 'mutual-credit' }`. Denominated in cpu-seconds. No token abstraction, no price oracle.
6. **Dual measures**: `resourceQuantity` carries cpu-seconds (the billable resource), `effortQuantity` carries megabytes (capacity used). Both fields already exist on EconomicEvent and Commitment.
7. **Sense envelopes stay in JSONL**: High-frequency telemetry (10s samples) is coordination data, not economic records. Only Commitments and settlement Events hit storage.

## Lifecycle with REA

```
Matthew requests 5 peers for 1800 cpu-seconds
  │
  ├─ POST Commitment (matthew, action: 'take', 1800 cpu-s, medium: CPU mutual credit)
  ├─ POST Commitment (susan, action: 'give', 360 cpu-s, medium: CPU mutual credit)
  ├─ POST Commitment (pete, action: 'give', 360 cpu-s, medium: CPU mutual credit)
  ├─ POST Commitment (frank, action: 'give', 360 cpu-s, medium: CPU mutual credit)
  ├─ POST Commitment (nancy, action: 'give', 360 cpu-s, medium: CPU mutual credit)
  │
  ├─ spawn persona nodes (existing shell scripts)
  ├─ budget watcher tracks actual usage (existing JSONL)
  │
  ├─ On each sample: no HTTP call (sense envelopes stay in JSONL)
  │
  ├─ On settlement (per persona):
  │   POST EconomicEvent (susan, action: 'deliver-service',
  │     resourceQuantity: {1.5, cpu-second},
  │     effortQuantity: {33.4, megabyte},
  │     fulfills: [susan-commitment-id],
  │     resourceClassifiedAs: ['compute'],
  │     realizationOf: agreement-id)
  │
  ├─ POST EconomicEvent (matthew, action: 'take',
  │     resourceQuantity: {total, cpu-second},
  │     fulfills: [matthew-commitment-id])
  │
  └─ PATCH Commitment state → 'fulfilled' (or 'breached' if budget-exceeded)
```

## Rust Backend: Commitment Endpoint

New routes following existing EconomicEvent patterns:

```
POST   /api/v1/commitments              Create commitment
GET    /api/v1/commitments/{id}         Get by ID
GET    /api/v1/commitments              List (with query filters)
PATCH  /api/v1/commitments/{id}         Update state
GET    /api/v1/commitments/agent/{id}   Commitments for an agent
```

Database table:
- `id, action, provider, receiver`
- `resource_conforms_to, resource_classified_as`
- `resource_quantity_value, resource_quantity_unit`
- `effort_quantity_value, effort_quantity_unit`
- `has_beginning, has_end, due`
- `clause_of` (Agreement ID)
- `in_scope_of`
- `state` (proposed → accepted → in-progress → fulfilled → cancelled → breached)
- `finished` (boolean)
- `note, created_at`

CreateCommitmentInput follows CreateEconomicEventInput patterns — snake_case Rust, camelCase TypeScript via `#[serde(rename_all = "camelCase")]` + `#[derive(TS)]`.

## MediumOfExchange Seed Record

```json
{
  "id": "cpu-mutual-credit",
  "code": "CPU",
  "name": "Compute Mutual Credit",
  "exchangeType": "mutual-credit",
  "description": "Peer-to-peer compute sharing denominated in cpu-seconds"
}
```

## TypeScript Integration: Testnet Manager

```
startTestnet()
  ├─ Health check: GET /api/v1/health on localhost:8090
  ├─ Ensure MediumOfExchange exists (pre-seeded or idempotent create)
  ├─ Create paired Commitments:
  │   - Matthew 'take' commitment (total budget)
  │   - Per-persona 'give' commitments (per-node budget)
  ├─ Store commitment IDs in session for fulfillment linking
  ├─ Spawn persona nodes (unchanged)
  └─ Start budget watcher (unchanged)

stopTestnet()
  ├─ Kill budget watcher (unchanged)
  ├─ For each persona:
  │   POST EconomicEvent (deliver-service, fulfills commitment)
  ├─ POST EconomicEvent for Matthew's 'take' (aggregate)
  ├─ PATCH Commitment state → 'fulfilled' or 'breached'
  ├─ Write compute report (unchanged)
  └─ Stop persona nodes (unchanged)
```

Failure mode: If elohim-storage is unreachable during settlement, fall back to JSONL-only and log a warning. Process lifecycle is mandatory, economics are best-effort.

## A2O Scenarios

```gherkin
Background:
  Given human "Matthew" has a running steward node
  And elohim-storage is healthy at "http://localhost:8090"
  And compute mutual credit medium exists

@e2e
Scenario: Compute commitments are persisted as REA records
  Given Matthew has a simulation requiring 5 peer nodes
  When he submits a ServiceRequest with budget 1800 cpu-seconds
  Then a 'take' commitment exists for Matthew with 1800 cpu-seconds
  And a 'give' commitment exists for each of the 5 personas
  And all commitments reference the CPU mutual credit medium

@e2e
Scenario: Settlement produces paired EconomicEvents
  Given 5 conductors are running for Matthew's simulation
  When the simulation workload completes
  Then each persona has a 'deliver-service' EconomicEvent
  And Matthew has a 'take' EconomicEvent for the total
  And each event fulfills its corresponding commitment
  And each event has resourceQuantity in cpu-seconds
  And each event has effortQuantity in megabytes
  And all persona commitments are marked 'fulfilled'
```

Existing lifecycle scenarios (spawn, envelopes, budget tracking, cleanup) stay unchanged — ops vs economics separation.

## Implementation Scope

| Component | Work | Location |
|-----------|------|----------|
| Commitment table + migrations | New SQLite table | `holochain/elohim-storage/src/db/` |
| Commitment API routes | CRUD + state transitions | `holochain/elohim-storage/src/api/` |
| Commitment view type | Rust struct with `#[derive(TS)]` | `holochain/elohim-storage/src/views/` |
| MediumOfExchange seed | CPU mutual credit record | `genesis/seeder/` |
| Storage client in testnet manager | HTTP client for Commitments + Events | `genesis/a2o/src/framework/` |
| Testnet manager REA integration | Paired Commitments on spawn, Events on settle | `genesis/a2o/src/framework/testnet-manager.ts` |
| New step definitions | Commitment/Event assertions against storage API | `genesis/a2o/steps/` |
| Feature file updates | Background health check, 2 new scenarios | `genesis/a2o/features/elohim/` |
| TypeScript type generation | `cargo test export_bindings` | `holochain/sdk/storage-client-ts/` |

## Deferred

- Mutual credit balance tracking (debits/credits across sessions)
- ServiceRequest/Offer marketplace matching
- Shefa UX rendering of compute events
- Scale to 20 personas

## YAGNI

- No Agreement endpoint (reference by ID, no CRUD)
- No Process endpoint (inputOf/outputOf stays null)
- No real-time event streaming (poll after settlement)
