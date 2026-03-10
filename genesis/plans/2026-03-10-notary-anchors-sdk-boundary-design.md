# Notary Anchors + SDK Boundary Design

**Date**: 2026-03-10
**Status**: Approved
**Depends on**: REA compute sharing (completed), testnet lifecycle (completed)
**Next**: Mutual credit balance tracking (EconomicResource), governance notary anchors

## Context

The service audit revealed 15 "dead" Angular services — but they aren't dead. They're a map of where the integrity coupling is missing. Services like GovernanceApiService and ContentAttestationApiService talk to elohim-storage directly, bypassing the DHT. This means the capabilities they expose could theoretically be centralized and used to extract rent at scale.

The Holochain DHT already defines 8 REA entry types (EconomicEvent, Commitment, EconomicResource, Process, Intent, Appreciation, Claim, Settlement) in the content_store integrity zome, but **no coordinator function ever writes to them**. All REA operations go straight to SQLite. The entry types are shells waiting to be activated.

This design activates the three entry types needed for compute sharing (Agreement, Commitment, EconomicEvent), adds a Fulfillment link type, establishes the DHT-first/storage-projection pattern, and plants CLAUDE.md markers that explain the SDK boundary principle for future development.

## The SDK Boundary Principle

**If a capability could be centralized and used by a handful of people to extract rent from the planetary deployment of humans in this space, it must live on distributed P2P infrastructure within the SDK boundary.**

The test isn't "is this a thin HTTP wrapper" — it's "what happens if someone captures this at scale?"

- Economic events/commitments → captured = you become the bank
- Attestations/credentials → captured = you become the credential authority
- Governance/consent → captured = you become the platform governance board
- Identity/presence → captured = you become the identity provider
- Content addressing → captured = you become the content landlord

These capabilities MUST be notarized on the DHT. The DHT has limits (100 entry types per DNA, slow/expensive writes), so it acts as the **notary**, not the database. Storage is the fast queryable projection.

Doorway is NOT in the SDK — it's a web2 bridge for hosted humans progressing toward stewardship. Projection API is doorway's attested-commons-reach web2 service of the internal protocol-validated network. Neither is a protocol primitive.

## Architecture: DHT-First with Storage Projection

```
Client (testnet manager / Angular app)
    │
    ├─ Zome call: create_agreement / create_commitment / create_economic_event
    │
    ▼
Holochain Conductor (DHT write — the notary)
    │
    ├─ Integrity validation (entry type rules)
    ├─ Commit to source chain + DHT gossip
    │
    ▼
Post-commit signal
    │
    ├─ Emits: { entry_type, id, action_hash, full_entry }
    │
    ▼
elohim-storage (projection — the index)
    │
    ├─ Upsert by id
    ├─ Store action_hash as dht_anchor_hash column
    └─ Available for fast queries via HTTP API
```

**Call paths:**
- **Steward (default)**: client → conductor → DHT → post-commit → storage projection
- **Hosted (transitional)**: client → doorway → conductor → DHT → post-commit → storage projection

Doorway validates reach/agency in the hosted path but doesn't own the truth. Once a human is a steward, they talk to their conductor directly.

**Dual IDs:** Every record carries both a String `id` (application-level, same on both sides) and an `action_hash` (cryptographic proof of notarization). The client generates the UUID, passes it to the zome, the post-commit signal carries both to storage.

## Entry Types + Link

### Agreement (new integrity entry)

```rust
pub struct Agreement {
    pub id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}
```

Deliberately thin. The Agreement is the anchor that paired Commitments point to via `clause_of`. It doesn't carry the terms — the Commitments carry the terms. The Agreement proves "these commitments belong together as a bilateral contract."

### Commitment (existing entry, add coordinator)

Already defined in content_store_integrity with `clause_of: Option<String>` (Agreement.id) and full REA fields.

Coordinator functions: `create_commitment`, `get_commitment`, `get_commitments_by_agreement`

### EconomicEvent (existing entry, add coordinator)

Already defined with `fulfills_json: String` and `realization_of: Option<String>` (Agreement.id).

Coordinator functions: `create_economic_event`, `get_economic_event`, `get_events_by_agreement`

### Fulfillment (new link type)

Link from EconomicEvent ActionHash → Commitment ActionHash. Created during `create_economic_event` when `fulfills` IDs are provided. Traversable on the DHT without parsing JSON arrays.

## Post-Commit Signals

Following the existing pattern (DoorwayCommitted, ContentServerCommitted):

```
AgreementCommitted { id, action_hash, entry }
CommitmentCommitted { id, action_hash, entry }
EconomicEventCommitted { id, action_hash, entry, fulfillment_links }
```

**Projection receiver:** elohim-storage already listens for conductor signals. The same listener upserts into `agreements`, `rea_commitments`, and `economic_events` tables, setting `dht_anchor_hash` on insert.

**Conflict resolution:** If a record already exists in storage from a direct HTTP POST (transitional period), the projection upserts by `id` and fills in `dht_anchor_hash`. Records with non-null `dht_anchor_hash` are notarized. Records with null are storage-only (legacy/transitional).

**Graceful degradation:** If storage is unreachable during post-commit, the signal is lost but the DHT has the truth. A reconciliation query can backfill later (deferred).

## Storage Changes

**New migration: dht_anchor_hash columns**
- Add `dht_anchor_hash TEXT` to `economic_events`
- Add `dht_anchor_hash TEXT` to `rea_commitments`

**New migration: agreements table**
```sql
CREATE TABLE agreements (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    name TEXT,
    note TEXT,
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Agreement API routes** matching existing commitment/event patterns.

## CLAUDE.md SDK Boundary Markers

Three files explaining the principle (not just listing capabilities):

- `holochain/dna/CLAUDE.md` — the integrity layer as SDK primitives
- `holochain/sdk/CLAUDE.md` — the TypeScript SDK surface
- `elohim-app/src/app/shefa/CLAUDE.md` — shefa as UX layer, not source of truth

Each explains WHY the boundary exists (rent extraction prevention), not just WHAT goes where.

## Testnet Manager Update

The testnet manager currently POSTs directly to elohim-storage. After this work:

1. Manager calls conductor: `create_agreement` → gets agreement_id + action_hash
2. Manager calls conductor: `create_commitment` (per persona, clause_of: agreement_id)
3. Post-commit signals project to storage automatically
4. On settlement: `create_economic_event` (with fulfills links) → post-commit projects
5. Storage queries for reporting unchanged (same HTTP API, now with dht_anchor_hash)

Fallback: if conductor is unreachable, fall back to storage-only POST with null dht_anchor_hash and log a warning.

## Implementation Scope

| Component | Work | Location |
|---|---|---|
| Agreement integrity entry | New entry type | content_store_integrity |
| Fulfillment link type | New link definition | content_store_integrity |
| Coordinator functions | create/get for Agreement, Commitment, EconomicEvent + Fulfillment links | content_store coordinator |
| Post-commit signals | 3 new signal types | content_store coordinator |
| Storage projection receiver | Signal listener → upsert with dht_anchor_hash | elohim-storage |
| Migration: dht_anchor_hash | Add column to economic_events + rea_commitments | elohim-storage/migrations/ |
| Migration: agreements table | New table | elohim-storage/migrations/ |
| Agreement API routes | CRUD | elohim-storage/src/api/ |
| Testnet manager update | Zome calls instead of direct storage POST | genesis/a2o/src/framework/ |
| CLAUDE.md markers | 3 files explaining SDK boundary principle | holochain/dna/, holochain/sdk/, elohim-app/src/app/shefa/ |

## Deferred

- EconomicResource coordinator (mutual credit balance tracking)
- Intent coordinator (marketplace matching)
- Appreciation, Claim, Settlement coordinators (no use case yet)
- Reconciliation query (DHT → storage backfill)
- Governance notary anchors (separate design)

## YAGNI

- No ResourceSpecification entry (reference by string ID)
- No Process coordinator (compute sharing is bilateral, not a transformation pipeline)
- No real-time event streaming from DHT (poll storage after projection)
- No reconciliation daemon (manual backfill if needed)
