# Shefa Pillar — Economic Experience Layer

Shefa is the human experience of the Elohim Protocol's economic
infrastructure. It renders stewardship, banking, resource flows, and
compute sharing in ways humans can understand and interact with.

## Shefa is UX, Not Truth

The protocol primitives (economic events, commitments, agreements,
mutual credit) live on the Holochain DHT — distributed infrastructure
that no one can capture. Shefa services in this directory are the
**experience layer** that makes those primitives legible to humans.

The distinction matters: if an economic event is only recorded in an
Angular service's state, it can be lost, forged, or silently modified.
If it's notarized on the DHT and projected to storage with a
`dht_anchor_hash`, it's cryptographically provable. Shefa reads from
storage (fast), but writes should go through the conductor (truthful).

## Service Categories

### API Services (thin HTTP clients to storage projections)
Services like `EconomicEventsApiService`, `ExchangeApiService`,
`FlowPlanningApiService` read from elohim-storage's HTTP API. These
are reading the **projection** of DHT truth — fast and queryable,
but not the source of truth.

For writes, these services should call through to the Holochain
conductor (via HolochainClientService zome calls), which writes to
the DHT and projects back to storage via post-commit signals. Direct
storage writes bypass the notary and create un-notarized records
(dht_anchor_hash = null).

### Composition Services (app-level logic)
Services like `InsuranceMutualService`, `BudgetReconciliationService`
compose multiple protocol primitives into domain-specific workflows.
These belong in the app, not the SDK — they're how this particular
app interprets the protocol, not the protocol itself.

### Transition State
Some services currently POST directly to elohim-storage. As the
conductor-first pattern is wired up, these should migrate to:
1. Write via conductor zome call
2. Post-commit signal projects to storage
3. Read from storage HTTP API (unchanged)
