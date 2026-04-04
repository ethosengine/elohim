# P2P Resilience Proof — End-to-End Design

**Date**: 2026-04-04
**Goal**: Prove that content seeded into the network is automatically RS-encoded, distributed across peers based on stewardship relationships, and reconstructable when peers are absent. Make this entire pipeline visible in the UI through the shefa/network lens.

## P2P Design Gate (Completed Upfront)

All entities in this design have been classified per the P2P design gate:

| Entity | Category | DHT Entry? | dht_anchor_hash? | Identity | Justification |
|--------|----------|-----------|-------------------|----------|---------------|
| `shard_manifests` | **C (operational)** | No | No — local projection | content_id (FK) | Per-peer encoding state. Rebuilt from local blob store. Not shared via DHT. |
| `shard_locations` | **C (operational)** | No | No — local projection | (shard_hash, peer_id) | Ephemeral peer tracking. Rebuilt from shard protocol ack events. Not shared. |
| REA commitment (storage) | **A (notarized)** | Yes (existing) | Yes (existing table) | CID of commitment | Uses existing `rea_commitments` table — already DHT-designed. |
| Resilience projection | **Computed** | No | N/A | N/A | Read-time aggregation from manifests + locations + allocations. No storage. |

**Why the HTTP routes don't need DHT entry types**: Routes like `/api/v1/resilience/{content_id}` and `/api/v1/resilience/{content_id}/verify` serve *computed projections* from Category C local tables, not DHT reads. They aggregate shard_manifests + shard_locations + stewardship_allocations on each read. This is the same pattern as `/p2p/status` — operational visibility, not protocol state.

**No new DHT entry types.** Lamad DNA stays at current capacity (~83/~100). All new tables are SQLite-local projections of P2P protocol events.

## Architectural Principle

**Seeding IS distribution.** The act of storing content triggers the full pipeline:

```
Seed → POST /db/content/bulk
  → blob stored locally
  → RS-encode (if above threshold)
  → lookup stewardship allocations + trust topology
  → push shards to peers via /elohim/shard/1.0.0
  → peers acknowledge → shard_locations updated
  → EPR Head auto-published with shard metadata
```

There is no separate distribution worker. The relationships (stewardship allocations, storage commitments, trust topology) ARE the distribution policy. Content creation IS content distribution.

## Three Sprints

### Sprint A: Wire Existing Data

**Outcome**: Stewardship shows in tooltip. RS math proven correct. Bugs fixed.

#### A1. Stewardship Join on Content Response

**File**: `elohim/elohim-storage/src/views.rs` + `http.rs`

Add `stewarded_by` field to `ContentView` response:

```rust
pub struct ContentStewardView {
    pub human_id: String,
    pub allocation_ratio: f32,
    pub contribution_type: String,    // "steward" | "authored" | "inherited"
    pub governance_state: String,     // "active" | "disputed"
}
```

Join from `stewardship_allocations` table when fetching content. This is a view join — always live, never denormalized.

**Wire format**: `stewarded_by: Vec<ContentStewardView>` serialized as camelCase per existing convention. The TS `ContentSteward` interface already has the right shape (`humanId`, `affinity` maps to `allocationRatio`, `role` maps to `contributionType`).

#### A2. Fix Resilience Tooltip

**File**: `content-viewer.component.ts`

- `getResilienceIcon()`: Use `stewardship.allocations.length` (already loaded via `StewardshipAllocationService`) instead of `node.stewardedBy?.length`
- `getResilienceTooltip()`: Build tooltip from `stewardship.allocations` — show real steward names, roles, allocation percentages
- Fallback: If stewardship hasn't loaded yet, show "Loading stewardship..." not "No stewards assigned"
- Click behavior: Already wired to `setActiveTab('trust')` — change to `setActiveTab('network')` since that's where the deeper view will live

#### A3. Fix Context Menu Z-Index

**File**: `qahal/components/context-menu-only/context-menu-only.component.ts`

- Bump `.menu-backdrop` to `z-index: 9998`
- Bump `.menu-dropdown` to `z-index: 9999`
- Add `z-index: 9997` to `.context-menu-wrapper` to establish stacking context

#### A4. RS Integration Tests

**File**: `elohim/elohim-storage/src/sharding.rs` (new test module)

Three test cases:
1. **Single shard**: Encode small blob (<16MB), verify hash matches, roundtrip
2. **Chunked**: Encode medium blob, split into 1MB chunks, reassemble, verify identical
3. **Reed-Solomon**: Encode blob into 4 data + 3 parity shards. Drop 1 shard — reconstruct. Drop 2 shards — reconstruct. Drop 3 shards — reconstruct. Drop 4 shards — verify failure. Assert reconstructed bytes match original.

#### A5. Signal Empty State

**File**: `content-viewer.component.html`

Show the signal summary section even when `reactionCounts.total === 0`:
- Display "0 signals from 0 participants" with a muted style
- Remove the `*ngIf="aggregatedSignals.reactionCounts.total > 0"` guard
- Keep the section but show empty state instead of hiding it

---

### Sprint B: Resilience Projection

**Outcome**: Backend tracks shard distribution. Network tab shows resilience data. Storage commitments modeled as REA acts.

#### B1. Shard Manifest Table

**File**: `elohim/elohim-storage/src/db/` (new module: `shard_manifests.rs`)

```sql
CREATE TABLE shard_manifests (
    content_id TEXT PRIMARY KEY,
    encoding TEXT NOT NULL,           -- "none" | "chunked" | "rs-4-7"
    shard_hashes TEXT NOT NULL,       -- JSON array of hex hashes, ordered
    data_shard_count INTEGER NOT NULL,
    parity_shard_count INTEGER NOT NULL,
    total_size_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**P2P Design Gate classification**: Category C (operational). This is a projection of local encoding state, not DHT-notarized. Each peer maintains its own manifest for content it has encoded.

Populated on blob ingest in `blob_store.rs` — after RS encoding, write manifest before returning hash.

#### B2. Shard Location Tracking

**File**: `elohim/elohim-storage/src/db/` (new module: `shard_locations.rs`)

```sql
CREATE TABLE shard_locations (
    shard_hash TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'announced',  -- "announced" | "verified" | "lost"
    last_verified TEXT,
    first_seen TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (shard_hash, peer_id)
);
```

**Category C (operational)**. Updated by:
- Shard protocol `Push` acknowledgments → status = "announced"
- Shard protocol `Have` verification → status = "verified"
- Connection closed / timeout → status = "lost" (after grace period)

#### B3. Resilience Endpoint

**File**: `elohim/elohim-storage/src/http.rs` (new route group)

`GET /api/v1/resilience/{content_id}` returns:

```json
{
  "contentId": "manifesto-foundations",
  "encoding": {
    "strategy": "rs-4-7",
    "dataShards": 4,
    "parityShards": 3,
    "totalSizeBytes": 42000
  },
  "distribution": {
    "totalPeers": 5,
    "peersHoldingShards": 3,
    "shards": [
      { "hash": "sha256-abc...", "type": "data", "peers": ["peer1", "peer3"] },
      { "hash": "sha256-def...", "type": "parity", "peers": ["peer2"] }
    ]
  },
  "stewardship": {
    "allocations": [
      { "humanId": "matthew", "ratio": 0.35, "type": "steward" },
      { "humanId": "nancy", "ratio": 0.40, "type": "steward" }
    ]
  },
  "commitments": {
    "activePeers": 5,
    "totalCommittedBytes": 21474836480,
    "totalUsedBytes": 8589934592
  },
  "health": {
    "score": 0.85,
    "canSurviveFailures": 3,
    "status": "healthy"
  }
}
```

Health score computed from: shard redundancy + steward count + commitment headroom.

#### B4. Seed REA Storage Commitments

**File**: `genesis/seeder/src/seed-commitments.ts` (new)

For each of the 5 test peers, seed a REA commitment:

```json
{
  "action": "provide",
  "provider": "peer-matthew-storage",
  "receiver": "network",
  "resourceConformsTo": "storage-capacity",
  "resourceQuantityValue": 4000,
  "resourceQuantityUnit": "GB",
  "state": "accepted",
  "note": "Home lab storage node — 4TB committed"
}
```

Wire into seeder main flow so commitments are created alongside content and allocations.

#### B5. Network Tab — Resilience Section

**File**: `content-viewer.component.html` (network tab section)

Add above the knowledge graph:

```
┌─────────────────────────────────────────┐
│ Resilience                    🟢 Healthy │
├─────────────────────────────────────────┤
│ Encoding: RS 4+3 (can lose 3 peers)    │
│                                         │
│ Shards:  ██████░ 5/7 distributed        │
│ Peers:   ●●●●○  4/5 online             │
│                                         │
│ Stewards:                               │
│   Matthew (35%) · Nancy (40%) · Eve(25%)│
│                                         │
│ Storage Committed: 20TB                 │
│ Storage Used:      8TB (40%)            │
└─────────────────────────────────────────┘
```

New service: `ResilienceService` fetches from `/api/v1/resilience/{contentId}`.

#### B6. Doorway Proxy Routes

**File**: `doorway/doorway-service/src/routes/` or `http.rs`

Proxy:
- `/api/v1/resilience/*` → elohim-storage
- `/api/v1/commitments/*` → elohim-storage (if not already proxied)

---

### Sprint C: End-to-End Distribution Proof

**Outcome**: Seeding content automatically distributes RS shards across peers. Reconstruction verified when a peer is absent. Full topology visible and proven.

#### C1. Auto-Distribution on Ingest

**File**: `elohim/elohim-storage/src/http.rs` (content creation handlers) + `p2p/mod.rs`

On `POST /db/content` and `POST /db/content/bulk`:

1. After blob is stored and RS-encoded (existing code in `blob_store.rs`)
2. Write shard manifest (Sprint B table)
3. Determine placement targets:
   - Query stewardship allocations for this content
   - Map stewards to peer IDs via trust topology (`cluster.rs`)
   - Fall back to `select_replication_targets()` for unallocated content
4. For each target peer, for each shard:
   - `P2PHandle::push_shard(peer_id, shard_hash, shard_data)` via `/elohim/shard/1.0.0`
   - On acknowledgment, insert into `shard_locations` with status "announced"
5. This is async (tokio::spawn) — content creation returns immediately, distribution happens in background

**Key**: The distribution is fire-and-forget from the HTTP handler's perspective. The shard protocol handles retries and the location table tracks state. If a peer is offline, the shard stays queued.

#### C2. Shard Protocol Enhancement — Push + Ack

**File**: `elohim/elohim-storage/src/p2p/shard_protocol.rs`

Current `ShardRequest::Push` exists but needs:
- Receiving peer: store shard in local blob store
- Receiving peer: update own shard manifest (it now holds this shard)
- Receiving peer: respond with `ShardResponse::Accepted { hash }`
- Sending peer: on `Accepted`, update `shard_locations`

#### C3. Periodic Shard Verification

**File**: `elohim/elohim-storage/src/p2p/mod.rs` (new periodic task in event loop)

Every 5 minutes (configurable):
- For each content this peer stewards:
  - For each shard in manifest:
    - For each peer in `shard_locations`:
      - Send `ShardRequest::Have { hash }`
      - On `ShardResponse::Yes`, update `last_verified`
      - On timeout/`No`, mark status = "lost"
- If any content drops below minimum redundancy:
  - Select new replication target
  - Push shard to new peer

This is the self-healing loop. Content automatically re-replicates when peers disappear.

#### C4. Reconstruction Verification Endpoint

**File**: `elohim/elohim-storage/src/http.rs`

`POST /api/v1/resilience/{content_id}/verify`:

1. Fetch shard manifest for this content
2. For each shard, attempt to fetch from a peer (not local — prove network works)
3. Intentionally skip up to `parity_shard_count` shards (simulate failures)
4. RS-decode remaining shards
5. Compare reconstructed hash with original blob hash
6. Return verification result:

```json
{
  "contentId": "manifesto-foundations",
  "verified": true,
  "shardsAvailable": 5,
  "shardsUsedForReconstruction": 4,
  "shardsSkipped": 3,
  "reconstructionTimeMs": 45,
  "originalHash": "sha256-abc...",
  "reconstructedHash": "sha256-abc...",
  "hashMatch": true
}
```

#### C5. Frontend — Verify Button + Live Topology

**File**: `content-viewer.component.html` (network tab)

Add to resilience section:
- "Verify Resilience" button → calls `POST /api/v1/resilience/{contentId}/verify`
- Shows verification result inline (pass/fail, shards used, reconstruction time)
- Peer map: for each peer, show online/offline status and which shards they hold
- If degraded: show which shards are at risk and re-replication status

#### C6. Integration Test — Full Pipeline

**File**: `elohim/elohim-storage/tests/` (new integration test)

Test scenario:
1. Start 5 storage instances (in-process or via test harness)
2. Seed content to peer 1 via HTTP
3. Wait for auto-distribution (poll shard_locations until all shards placed)
4. Verify all 5 peers hold shards (via `/api/v1/resilience/{id}`)
5. Shut down 2 peers
6. Call `POST /api/v1/resilience/{id}/verify` on a remaining peer
7. Assert reconstruction succeeds
8. Assert health score reflects degraded state
9. Restart peers → verify re-verification updates locations

#### C7. Seed Data Updates

**File**: `genesis/seeder/src/seed.ts` and related

Ensure the 5-peer seed scenario includes:
- Stewardship allocations with varied ratios across peers
- REA storage commitments per peer (from Sprint B)
- Content with varied reach levels (commons, community, trusted)
- At least one large content item that triggers RS encoding (>10MB)
- At least one content item per reach level to test access gating

The seeder itself doesn't change behavior — it still calls `POST /db/content/bulk`. The auto-distribution (C1) handles the rest.

---

## What Doesn't Change

- **Seeder code**: Same API calls, same seed data format (plus commitments from B4)
- **EPR protocol**: Already auto-publishes heads on content creation
- **Shard protocol wire format**: 4-byte BE + MessagePack, no changes
- **Trust topology**: `cluster.rs` already has `select_replication_targets()` — we use it, not replace it
- **Automerge sync**: CRDT sync for metadata is separate from shard distribution — both continue independently

## P2P Design Gate Answers

| Entity | Category | Identity | Justification |
|--------|----------|----------|---------------|
| shard_manifest | C (operational) | content_id (FK) | Projection of local encoding state, not shared |
| shard_location | C (operational) | (shard_hash, peer_id) composite | Ephemeral tracking, rebuilt from protocol events |
| REA commitment (storage) | A (notarized) | CID of commitment | Economic act — must be verifiable by third parties |
| Resilience projection | N/A (computed) | N/A | Derived on read from manifests + locations + allocations |

No new DHT entry types. Lamad DNA stays at current capacity. Storage commitments use existing `rea_commitments` table.
