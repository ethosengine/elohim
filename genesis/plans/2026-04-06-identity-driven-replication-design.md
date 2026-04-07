# Identity-Driven Replication — Design Spec

**Date**: 2026-04-06
**Goal**: Replace push-based seeding with protocol-native pull-based replication. Every empty elohim-storage node discovers its identity, queries the network for content it's responsible for, and pulls it at its own pace. Genesis bootstrap, new device onboarding, and account recovery all use the same code path.

## Problem

The current seeder (`seed-sqlite.ts`) pushes content to each conductor's SQLite database via HTTP bulk writes. With 5 peers and ~1,364 content items each, this causes:

- **SQLite "database is locked" errors** — parallel bulk writes from an external process contend with the conductor's own operations
- **No P2P path tested** — seeding bypasses the entire shard/sync/EPR infrastructure, so we never prove sync works during bootstrap
- **Genesis-specific code** — the seeder is bespoke pipeline tooling, not reusable for production scenarios like device recovery or new node onboarding

## Design Principle

**An empty node knows who it is. It asks the network for what it needs.**

There is no "seeder." There is no "bootstrap mode." There is only: I have keys, I have peers, I discover my responsibilities, I pull content. The node doesn't know *why* it's empty — genesis, recovery, new device — it just knows it has a gap between what it holds and what it should hold.

## P2P Design Gate

| Entity | Category | DHT Entry? | Identity | Justification |
|--------|----------|-----------|----------|---------------|
| Replication state | **C (operational)** | No | N/A | Per-node tracking of replication progress. Local only. |
| EPR Head | **A (notarized)** | Yes (existing) | CID | Already published to Kademlia DHT. No new entry type. |
| Stewardship allocation | **A (notarized)** | Yes (existing) | UUID | Already in `stewardship_allocations` table. |
| Content | **A (notarized)** | Yes (existing) | Content ID (slug) | Already in `content` table. |

**No new DHT entry types.** All new code is operational logic using existing protocol primitives.

### Genesis Peer: Adam

Adam (`human-adam-firstman`) already exists as a persona at the data layer — account package, presence, conductor-groups membership ("Eden Household"). What's missing is his K8s deployment. This sprint adds Adam as the 6th deployed conductor in the genesis topology, with elevated CPU/memory/storage so he can handle the initial seed write.

The deployed topology becomes: **Adam** (genesis, elevated resources), **Matthew**, **Jessica**, **Pete**, **Timothy**, **Frank**. Adam is the first and only peer to receive the direct content seed; the other five replicate from him via P2P.

Adam's role as genesis peer establishes a reusable pattern: any peer that lends compute for network bootstrap (or recovery, or migration) earns credit on the network as a shefa contribution.

## Architecture

### What Already Exists

| Component | Status | Location |
|-----------|--------|----------|
| EPR Head publication | Working | `p2p/mod.rs:publish_all_epr_heads()` — publishes all content to Kademlia on startup |
| EPR Head structure | Working | `epr_codec.rs:EprHead` — carries `shefa.stewards`, `qahal.reach`, content CID |
| Shard fetch | Working | `P2PHandle::fetch_shard()` — fetches blob by hash from peers |
| Resolve + fetch | Working | `P2PHandle::resolve_and_fetch()` — full pipeline: DHT → EPR Head → shard fetch → integrity check |
| Sync rounds | Working | `initiate_sync_round()` — 60s interval, ListDocuments from all peers |
| mDNS discovery | Working | Peers auto-discover on LAN, register in Kademlia |
| Bootstrap nodes | Working | `config.bootstrap_nodes` dialed on startup |
| Stewardship allocations | Working | `db/stewardship_allocations.rs` — content-to-steward mapping |
| Bulk content write | Working | `db/content_diesel.rs:bulk_create_content()` — transactional insert |

### What's New

One new component: **the replication loop**. A periodic task inside elohim-storage that:

1. Compares local content against what the network offers
2. Filters by identity (stewardship allocations + commons reach)
3. Pulls missing content via existing EPR → shard protocols
4. Self-paces based on local write capacity

## Replication Loop Design

### Trigger Conditions

The replication loop runs:

- **On startup** — after peer connections are established (2-5 second delay for mDNS/bootstrap)
- **On new peer connection** — a new peer means potential new content sources
- **Periodically** — every 60 seconds (aligned with existing sync round cadence), checks for gaps

### Phase 1: Discover Available Content

Query connected peers for their EPR Head inventories. Two complementary discovery paths:

**Path A — Peer inventory query**: Extend `SyncRequest::ListDocuments` (already exists) to return EPR Head summaries. Each summary includes `id`, `content` (CID), `shefa.stewards`, `qahal.reach`. This gives a paginated view of what a specific peer holds.

**Path B — DHT scan**: Walk the Kademlia DHT for `epr:*` records. This discovers content from any peer, not just directly connected ones. More complete but slower.

**This sprint: Path A only.** For genesis bootstrap, Adam is directly connected. Path B (DHT scan) is a future enhancement for WAN-scale account recovery where the source peer may not be directly connected.

### Phase 2: Filter by Identity

The node learns "who am I?" through the normal registration/auth path — the same way a human logs into a new device or an operator adds a blade to their rack. elohim-storage doesn't know or care about K8s, env vars, or deployment context. Someone presents credentials, the node has an identity, and the replication loop uses it.

The registration path (`POST /auth/register`) already creates a local session with `human_id` and `agent_pub_key`. The replication loop reads this from the local session store. If no identity is registered, the node only pulls commons content (anonymous bootstrap).

For each discovered EPR Head, determine if this node should hold it:

```
should_replicate(head: EprHead, my_human_id: Option<&str>) -> bool {
    // Commons content: every peer holds it
    if head.qahal.reach == Some("commons") { return true; }

    // --- Below is the full filter, activated after the encryption sprint ---

    // No identity registered — commons only
    let human_id = match my_human_id {
        Some(id) => id,
        None => return false,
    };

    // Stewarded content: I'm in the stewards list
    if head.shefa.stewards.contains(&human_id.to_string()) { return true; }

    // Not my responsibility
    false
}
```

**This sprint**: only the commons check is active. Stewardship-filtered replication is wired but gated behind a feature flag until the encryption sprint delivers private content key exchange.

Compare against local content table. If an EPR Head passes the filter but the content isn't in local SQLite, it's a replication gap.

### Phase 3: Fetch Missing Content

For each gap, use the existing `resolve_and_fetch()` pipeline:

1. Resolve EPR Head from DHT (or use the already-decoded head from discovery)
2. Fetch content bytes via shard protocol (`ShardRequest::Get`)
3. Verify integrity (SHA-256 hash check, already implemented)
4. Write to local SQLite via `bulk_create_content()` (from within the Rust process — no HTTP, no external contention)

Self-pacing: fetch in configurable batches (default: 50 items), with a configurable inter-batch delay (default: 100ms). The node controls its own write rate.

### Phase 4: Republish

After ingesting new content, publish EPR Heads for what this node now holds. This makes the node a source for other peers — the network bootstraps in a cascade:

```
Adam ingests → publishes EPR Heads
Jessica pulls from Adam → publishes her EPR Heads
Pete pulls from Adam OR Jessica → publishes...
(cascade until all peers have their stewardship allocation)
```

### Replication State Tracking

Track replication progress in memory (not persisted — rebuilt on restart):

```rust
struct ReplicationState {
    /// EPR Heads discovered but not yet fetched
    pending: HashSet<String>,
    /// Successfully replicated content IDs
    completed: HashSet<String>,
    /// Content IDs that failed fetch (with retry count)
    failed: HashMap<String, u32>,
    /// Whether initial replication is complete
    caught_up: bool,
}
```

Expose via existing `/p2p/status` endpoint so Jenkins (or any orchestrator) can poll for completion:

```json
{
    "replication": {
        "pending": 42,
        "completed": 1322,
        "failed": 0,
        "caughtUp": false
    }
}
```

## Content Metadata Transport

The EPR Head carries enough metadata to create a content record (`id`, `title`, `contentType`, `contentFormat`, `tags`, `reach`, `stewards`, `relationships`). The shard protocol carries the content bytes (blob).

However, the EPR Head doesn't carry the full `contentBody` (the inline JSON for non-blob content like learning paths). Two options:

**Option A — Extend EPR Head**: Add an optional `body` field to `EprHead` for small inline content. This keeps everything in one round-trip but inflates the ~500B gossip envelope.

**Option B — Two-phase fetch**: EPR Head carries the CID. For content with `blob_hash`, fetch via shard protocol. For content without `blob_hash` (inline `contentBody`), add a new `ShardRequest::GetContent { id }` that returns the full content record from the source peer's SQLite.

**Recommendation: Option B.** EPR Heads should stay small (gossip-friendly). A new `GetContent` request type in the shard protocol is clean and keeps the separation between addressing (EPR Head) and content (shard/content fetch). This also naturally handles metadata fields beyond what EPR Head carries (description, metadata JSON, etc.).

## Jenkins Pipeline Changes

The genesis Jenkinsfile seeding stage changes from:

```groovy
// OLD: Push content to all 5 conductors
for (human in humans) {
    SEED_CMD="npx tsx src/seed-sqlite.ts --conductor-for=${humanId}"
    STORAGE_URL="http://${storageUrl}" $SEED_CMD
}
```

To:

```groovy
// NEW: Seed genesis peer, register others, let P2P handle the rest

// Step 1: Direct write to Adam's conductor only (genesis peer)
STORAGE_URL="http://${adamStorageUrl}" npx tsx src/seed-sqlite.ts

// Step 2: Register the other humans on their conductors (identity only, no content)
// Same as someone punching in their key on a new device
for (human in [jessica, pete, timothy, frank]) {
    curl -X POST "http://${human.doorwayUrl}/auth/register" \
        -H "Content-Type: application/json" \
        -d '{"identifier":"${human.email}","displayName":"${human.name}",...}'
}

// Step 3: Wait for EPR Heads to publish on genesis peer
waitForEprPublication(adamStorageUrl, expectedCount: 1364)

// Step 4: Wait for other peers to replicate via P2P
for (human in [jessica, pete, timothy, frank]) {
    waitForReplication(human.storageUrl, expectedCount: human.expectedContentCount)
}
```

The `waitForReplication` helper polls `/p2p/status` for `replication.caughtUp == true`, with a timeout. If replication doesn't complete within the timeout, the pipeline fails with diagnostics about what's stuck.

`seed-sqlite.ts` is retained but simplified — no `--conductor-for` filtering needed. It writes all content to one conductor. The stewardship filtering happens naturally via identity-driven replication. The only external action for non-genesis peers is registration — the same thing any new node in the network does.

## Private Content & Encryption (Future Sprint)

Not in scope for this sprint. This sprint replicates **commons content only** — content where `qahal.reach == "commons"`. The replication loop skips non-commons content.

The private content problem is real and interesting. During genesis, Adam's conductor holds private content for other peers in cleartext. In production, content would be encrypted by the creator before publication. The genesis moment is special — content is defined from seed files, not from a human's device.

The protocol-native solution is a **trust substrate for key exchange**:

1. Adam's conductor notices "I have unencrypted content that isn't mine"
2. Adam signals the rightful owner (e.g., Jessica)
3. Jessica sends her public key
4. Adam encrypts the content locally and deletes the cleartext
5. The exposure is recorded as an attestation: "Adam held Jessica's cleartext from T1 to T2"
6. Jessica rotates her keys (changes the locks on the house)
7. Jessica re-encrypts under new keys
8. An acknowledgment flows back — a shefa event: trust earned through correct handling

This pattern generalizes beyond genesis to any exposure event: network partition recovery, emergency failover, device migration. The trust substrate doesn't pretend exposure didn't happen — it makes it legible and resolvable.

## Settle Phase (Future Sprint)

After bootstrap replication and the encryption sprint, Adam holds all content but only stewards a subset. Once resilience proofs (Sprint B's `shard_manifests` + `shard_locations`) confirm that stewards have their content:

1. Adam identifies non-stewarded cleartext content
2. Verifies steward copies via shard location tracking
3. Converts non-stewarded content to encrypted reciprocal shards (shefa compute)
4. Steady state: each peer holds stewarded content (cleartext) + reciprocal shards (encrypted)

## Scope & Sprint Boundaries

**This design covers:**
- Identity-driven replication loop in elohim-storage (the permanent capability)
- `ShardRequest::GetContent` extension for full content record fetch
- Replication state tracking and status endpoint
- Jenkins pipeline change to single-genesis-peer seeding
- Commons content replication across all 5 peers via P2P
- Adam as the genesis peer (first human, carries all content initially)

**This design does NOT cover:**
- Private content encryption and key exchange (next sprint — trust substrate)
- Settle phase (cleartext → encrypted shard transition)
- Stewardship-filtered replication for non-commons content (depends on encryption sprint)
- Stewardship allocation changes triggering mid-life replication
- Cross-WAN replication (relay-mediated fetch) — works in theory via existing relay infrastructure, but not tested
- Content deletion/eviction when stewardship is revoked

## Success Criteria

1. Adam's conductor is the only one that receives a direct SQLite write
2. The other four conductors receive all commons content via P2P replication
3. Each conductor holds all commons content (private content replication deferred to encryption sprint)
4. The Jenkins pipeline completes without "database is locked" errors
5. `/p2p/status` on each conductor shows `replication.caughtUp: true`
6. Content integrity verified — CIDs match between genesis peer and pulling peers
7. No elohim-storage code references K8s, env vars, or deployment context — identity comes through the registration path
