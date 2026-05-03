# Blob Custody Reconciliation — Phase 2 Redesign for Light Up the Topology

**Status:** Design (pre-implementation)
**Date:** 2026-05-02
**Predecessor:** [Light Up the Topology](2026-05-01-light-up-the-topology-design.md) — Phase 0 + Phase 1 complete; Phase 2 BLOCKED on substrate gaps surfaced during implementation.
**Successor:** Returns to the parent sprint after Phase 2 lands; Phases 3-11 of the parent plan resume on top of this work.

## Context

Phase 2 of the parent sprint (T12-T15) was a substrate-fix phase: peer-fallback helper, GET-time blob fallback, on-connect replication kick, filesystem-count parity regression. The implementer correctly BLOCKED before writing code, surfacing six substrate-vs-plan mismatches:

1. The plan's `SwarmClient` abstraction does not exist — substrate uses `mpsc::Sender<P2PCommand>` with oneshot reply channels.
2. No Kademlia provider track for blob hashes — only EPR atom CIDs are advertised via `KadStartProviding`.
3. No multi-peer test harness in `test_util` — only a stub `P2PHandle::for_testing()`.
4. EPR verification is not sha256-of-bytes — it is CBOR-canonical CID recompute via `verify_incoming_epr`. The plan's helper would silently downgrade verification.
5. `AppState` is HTTP-layer; P2P state lives on `P2PNode`. The plan's "add fields to AppState" targeted the wrong struct.
6. `P2PCommand::SendListContent { peer, limit }` does not exist; `ShardRequest::ListContent` exists only as an inline wire message in `run_replication_cycle`.

The plan's Phase 2 was discovery-mechanism-first (Kad lookup → fetch). This redesign is **manifest-vs-reality-first**: the protocol already declares custody contracts via `rea_commitments(action='custody-blob')` (established by T03d); the substrate needs an operational layer that observes who currently has what, plus a controller that reconciles the diff.

## Sprint Goal

**Resilience IS resilience; visibility of resilience is what builds trust, safety, and acceptance.** The demo lives or dies on the topology UI showing two things faithfully:

- **Replicas grow toward target after a peer connects.** Custody commitments that were under-honored become honored as the new peer pulls bytes; the count rises in real time.
- **Commitments unhonored beyond grace are visible.** When a custodian goes offline and stays offline, the topology UI surfaces a `placement-gap` badge — not as an alarm, but as the structured economic signal it is (`project_placement_signals_are_shefa_inputs`).

Both behaviors require a substrate that knows three things: who *should* host what (manifest), who *currently* hosts what (reality), and the diff between them.

## The Three-Surface Reconciliation Pattern

The architecture mirrors a Kubernetes control-plane pattern, with elohim's three-layer truth model (DHT / libp2p / doorway) supplying each surface:

| Surface | What it is | Storage | Authority | Established by |
|---|---|---|---|---|
| **Manifest** (desired state) | "Peer X commits to host blob Y for content steward Z, expected for N seconds." | `rea_commitments` rows: `action='custody-blob'`, `resource_classified_as=<blob_hash>`, `provider=<peer-steward-cid>`, `receiver=<content-steward-cid>`, `resource_quantity_value=<bytes>`. | DHT-notarized via `dht_anchor_hash`. Signed. Cannot be forged. | T03d (action conventions). The DHT entry that backs each commitment is integrity-bearing. |
| **Reality** (observed state) | "Peer X currently hosts blobs A, B, C — gossiped at time T with sequence S." | `peer_blob_inventory(peer_id, blob_hash, last_seen_at, source, sequence)` — populated by libp2p gossipsub on topic `elohim/inventory/blob`. | Operational. Eventually consistent. Falsifiable: peer can gossip-claim hosting they don't have, but the lie collapses on first failed fetch. | This sprint (T12-T15). |
| **Diff** | Reconciliation controller's input | Computed on demand; not stored. | Drives kicks (act on own commitments), placement-gap signals (observe on others'), and topology UI badges. | This sprint (T16). |

**Why three surfaces, not two.** A two-surface model (just manifest + reality) would force every observation to bake into either the contract or the operational state. The diff surface lets the controller stay stateless about *outcomes* — it computes drift fresh each pass — while still emitting durable artifacts (REA events, placement-gap signals) for the topology UI and downstream shefa flows.

**Why the manifest is signed but the reality isn't.** Custody commitments are economic contracts; forgery there would let a peer claim hosting authority they don't have. The reality layer is operational chatter — falsehoods collapse on first failed fetch (no `serve-blob` event lands), and `last_seen_at` ages out stale entries. Signing every gossip message would impose Holochain-DHT-level cost on libp2p-level operational data, exactly what `project_dht_vs_libp2p_scoping` warns against. Verifiable serves are the integrity floor; signed gossip is unnecessary.

**Door open for Good-Samaritan salvage.** When a custody commitment goes unhonored, the placement-gap signal is consumed by this sprint only as a topology UI badge. A future sprint may add an opt-in salvage path: a peer with spare capacity sees the gap and (per consent policy) commits as a new custodian, healing the network without centralized coordination. This is a feature, not a bug — but it is not in this sprint's scope.

## Components

### 1. `peer_blob_inventory` — Reality projection (Category C)

A new SQLite table parallel to `peer_identity_bindings`. The `source` field discriminates evidence quality: gossip claims are operational; fetch-success is the strongest evidence (we proved it).

```sql
CREATE TABLE peer_blob_inventory (
    peer_id      TEXT NOT NULL,
    blob_hash    TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,                  -- ISO 8601, refreshed on each gossip arrival
    source       TEXT NOT NULL CHECK(source IN ('gossip-snapshot', 'gossip-delta', 'fetch-success')),
    sequence     INTEGER NOT NULL,                -- per-peer monotonic; gap-detect on out-of-order delta arrivals
    PRIMARY KEY (peer_id, blob_hash)
);
CREATE INDEX idx_peer_blob_inventory_blob ON peer_blob_inventory(blob_hash);
CREATE INDEX idx_peer_blob_inventory_recent ON peer_blob_inventory(last_seen_at);
```

Source of truth: libp2p gossipsub messages on topic `elohim/inventory/blob`, plus local `serve-blob` event arrivals. Rebuildable from gossip replay. Eligible for TTL eviction (entries older than N minutes drop out — N is operator-tunable, default 10 minutes, longer than the slowest archetype's broadcast cadence).

### 2. Inventory gossip wire (libp2p Gossipsub)

Two wire messages on `elohim/inventory/blob`:

```rust
// Periodic full-state resync. Receivers replace their per-peer entries
// with this set; entries not in the snapshot are evicted.
pub struct BlobInventorySnapshot {
    pub peer_id: String,                 // multibase libp2p PeerId
    pub hashes: Vec<String>,             // blob hashes the peer currently hosts
    pub snapshot_at: i64,                // microseconds since epoch
    pub sequence: u64,                   // monotonic per-peer
    pub signature: Vec<u8>,              // structural non-empty (Stage 1); Ed25519 in Stage 2
}

// Event-driven delta. Receivers apply add/remove against existing entries.
pub struct BlobInventoryDelta {
    pub peer_id: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub emitted_at: i64,
    pub sequence: u64,                   // monotonic per-peer; gap → request snapshot
    pub signature: Vec<u8>,              // structural non-empty (Stage 1); Ed25519 in Stage 2
}
```

Encoding: MessagePack via `rmp_serde`. Topic: `elohim/inventory/blob`. Signing: structural-non-empty enforced at Stage 1 (single null byte sufficient), per the same gradient as `project_bootstrap_to_elohim_security_gradient`.

**Sequence semantics.** Each peer maintains a monotonic counter that increments on every snapshot or delta it emits. Receivers track the highest sequence seen per peer; if a delta arrives with `sequence > expected_next`, the receiver knows it missed messages — it requests an out-of-band snapshot from the source peer (or waits for the next periodic snapshot). Snapshots are self-contained and can replace any prior state.

**Cadence.** Archetype-tunable (`project_cadence_archetype_tunable_with_dev_overrides` 4-layer pattern):

| Archetype | Snapshot cadence | Delta on change |
|---|---|---|
| `node` (always-on household blade) | 60s | Yes |
| `desktop` (sometimes-on workstation) | 300s | Yes |
| `mobile` (battery-precious) | Disabled by default | Disabled by default |
| `steward` (collective infrastructure) | 60s | Yes |

Operator preset: `inventory-broadcast-seconds`. Mobile defaults to disabled because the device is rarely the source of truth on hosting; if a mobile is acting as a relay or content steward, the operator can enable the broadcast.

### 3. Inventory projection writer

Mirrors the T03b pattern: gossip arrival → upsert with the existing `upsert_preserving_*` semantics from T03b. New writer functions:

- `apply_snapshot(conn, peer_id, hashes, sequence, snapshot_at)` — replaces all `(peer_id, *)` rows with the snapshot set; deletes entries not in the new set. **Snapshots are accepted regardless of sequence** (any snapshot is authoritative; this is the recovery path from sequence-manipulation attacks). The receiver updates its sequence high-watermark to the snapshot's sequence.
- `apply_delta(conn, peer_id, added, removed, sequence, emitted_at)` — checks `sequence == stored_max_for_peer + 1`. If `sequence > stored_max + 1` (gap-detect), queues a snapshot-request and aborts (the delta is dropped; the snapshot will resync). If `sequence <= stored_max` (replay), drops silently.
- `record_fetch_success(conn, peer_id, blob_hash, observed_at)` — upserts with `source='fetch-success'`; this is the strongest-evidence path that overwrites prior gossip-only entries.

Per-peer sequence high-watermark lives in a small in-memory `DashMap<PeerId, u64>` for fast gap-detect, mirrored to a `peer_inventory_cursor` row on each commit (analogous to the existing `projector_cursor` table) so it survives restart.

### 4. Inventory broadcast scheduler

Owns the timer-driven snapshot emit and the change-driven delta emit:

- **Snapshot timer.** Per archetype config; on tick, computes the local set of hosted blob hashes (from filesystem walk or the existing local blob-store index) and publishes `BlobInventorySnapshot`.
- **Delta emitter.** Hooked into the local blob-store's add/remove paths. On a single-blob change, queues a delta (small batch window — e.g. 200ms — collapses bursts into a single message).
- **Sequence allocator.** Single source of truth for this peer's sequence number.

### 5. Custody reconciliation controller

The diff engine. New module under `elohim/elohim-storage/src/reconcile/` (parallel to the existing `controller.rs`). Multi-trigger pattern:

| Trigger | Source | What it does |
|---|---|---|
| **Gossip arrival** | A `BlobInventorySnapshot` or `BlobInventoryDelta` was processed for peer X | Reconcile owned commitments where peer X is provider OR receiver. |
| **Connection event** | `ConnectionEstablished` for peer X | Reconcile owned commitments where peer X is provider OR receiver. (Acts before the first gossip arrives — the peer just joined.) |
| **Periodic sweep** | Timer (default 120s, operator preset `custody-sweep-seconds`) | Full reconcile pass over all owned commitments. Catches anything missed by event-driven triggers. |

A **reconcile pass** is the same function called from any trigger. It is idempotent — safe to call repeatedly. For each `custody-blob` commitment where this peer is the provider:

```
if blob_hash NOT in local store:
    candidates = SELECT peer_id FROM peer_blob_inventory
                 WHERE blob_hash = ? AND last_seen_at > now - freshness_horizon
                 ORDER BY (source = 'fetch-success' DESC, last_seen_at DESC)
    if candidates is empty:
        no-op (waiting for inventory to populate)
    else:
        kick fetch via T17's path; on success emit serve-blob event.
```

For each `custody-blob` commitment where this peer is the *receiver* (the content steward, not the custodian) and the provider is some other peer:

```
last_seen = SELECT max(last_seen_at) FROM peer_blob_inventory
            WHERE peer_id = <provider> AND blob_hash = <commitment.blob_hash>
if last_seen is NULL or now - last_seen > placement_grace_seconds:
    if no placement-gap event has been emitted for this commitment within the cooldown:
        emit placement-gap REA event (action='placement-gap',
                                       provider=<custodian>, receiver=<content_steward>,
                                       resource_inventoried_as=<blob_hash>,
                                       output_of=<custody-blob commitment action_hash>)
```

**Connectivity API on `P2PNode`.** Expose `is_connected(peer_id) -> bool` and `connected_peers() -> Vec<PeerId>` accessors backed by the existing `peer_metrics: DashMap`. Both the reconciliation controller and T17's GET-time fallback consume these.

**Reconciliation metrics.** New struct `ReconciliationMetrics` on `P2PNode` with:
- `reconcile_passes_total` (counter): how many reconcile passes have run
- `kicks_fired_total` (counter): how many fetches the controller has initiated
- `placement_gaps_emitted_total` (counter): how many `placement-gap` events have been emitted (with cooldown applied)
- `last_pass_at`: timestamp of last reconcile pass
- `last_pass_duration_ms`: observability into pass cost

Per-peer counters (e.g., fetch attempts per peer) extend the existing `peer_metrics: DashMap`. Whole-controller counters live on `ReconciliationMetrics`. Separation by what they describe.

**Grace period.** `placement_grace_seconds` defaults to 300 (5 minutes). Tunable via the same 4-layer cadence pattern. Cooldown on placement-gap re-emission: `placement_gap_cooldown_seconds` defaults to 1800 (30 minutes) to avoid flooding the REA stream with duplicate gaps.

### 6. GET-time blob fallback (and the shared fetch helper)

The fetch logic (consult inventory → iterate candidates → verify hash → persist → emit `serve-blob`) is extracted into a shared helper module (`p2p/blob_fetch.rs` or similar) so both the HTTP handler and the reconciliation controller call the same code path. The original Phase 2 plan's T12 (peer-fallback helper extraction) is absorbed here — the helper's contract is what T12 was reaching for, but the helper now lives at the right layer (operating on `peer_blob_inventory` candidates rather than Kad providers).

The HTTP blob handler (`elohim/elohim-storage/src/http.rs`, the `GET /blob/{hash}` route) extends:

```
on local store miss:
    candidates = SELECT peer_id FROM peer_blob_inventory
                 WHERE blob_hash = ? AND last_seen_at > now - freshness_horizon
                 ORDER BY (source = 'fetch-success' DESC, last_seen_at DESC)
    filter candidates: only those is_connected(peer_id) (per the new accessor)
    while candidates is non-empty:
        batch = candidates.take(fetch_parallelism)        # default 3
        race the batch in parallel:
            for each peer in batch concurrently:
                send P2PCommand::FetchBlob { peer_id, hash, reply: oneshot }
                with per-peer timeout (default 5s)
        on first reply that returns Ok(bytes) AND verify_blob_hash(bytes, hash):
            cancel/drop pending replies in this batch
            persist locally to blob store
            record_fetch_success(winning_peer_id, hash)
            emit serve-blob REA event:
                action='serve-blob'
                provider=<winning_peer_id-steward-cid>
                receiver=<this-peer-cid>
                resource_inventoried_as=<blob_hash>
                output_of=<matching custody-blob commitment action_hash, if known; else NULL>
            return bytes to HTTP requester
        if all replies in batch failed (timeout / error / hash-mismatch):
            log per-peer failure (drives peer-quality metrics later)
            continue with next batch
    if all candidates exhausted:
        return 404 (existing behavior preserved)
```

`FetchShard` is extended from single-peer to peer-iteration (or a new sibling command `FetchBlob { peer_id, hash, reply }` if the existing `FetchShard` semantics are too tied to shard storage). Implementation choice deferred to the implementer with a clear contract: per-peer timeout, fail-fast on hash mismatch, emit `serve-blob` only on verified success.

**Verification.** The blob's content hash MUST match the requested hash before persisting locally. Mismatch is a protocol violation; log + drop + do not record fetch-success. (This is the symmetric protection to T03c's signature-non-empty fix: integrity is enforced at every layer that writes durable state.)

**Why race in parallel rather than iterate sequentially.** Sequential iteration with a 5s per-peer timeout means tail latency is `(slow_peer_count × 5s)` in the bad case — one wedged custodian gates every request behind it. Racing N=3 candidates in parallel means tail latency is `min(per-peer latency)` for the batch; a single fast hit returns immediately and pending replies are cancelled. Bandwidth cost is small (only the winner's bytes are kept; losing replies are short response messages); CPU cost is small (one oneshot per peer). The default `fetch_parallelism=3` balances opportunistic concurrency against load amplification on heavily-pinned blobs. Operator preset `fetch-blob-parallelism`.

### 7. Filesystem parity sweep

A periodic self-consistency check that runs on each peer:

- Walks the local blob store filesystem (or queries the local index) for the actual set of blob hashes hosted.
- Compares against the set of blob hashes the broadcast scheduler last gossiped.
- If they differ: emit an `inventory-parity-drift` operational signal (locally logged, exposed via `/api/v1/diagnostics/inventory-parity`); the next snapshot broadcast naturally corrects via the snapshot's authoritative replacement semantics.

This catches the failure mode from `feedback_storage_quilt` (and its predecessor memory `project_inventory_exchange_not_byte_replication`): inventory gossip can run cleanly while bytes never actually replicate. The parity sweep is the regression-defense; the test added in this task asserts that the broadcast scheduler's gossiped set matches the filesystem set after a sync round.

## Data flows

### Flow 1: Replica grows after peer connects (the primary demo)

1. Peer J (Jessica's mobile) connects to peer M (Matthew's doorway-blade).
2. Identity handshake completes (Phase 0 path).
3. **T16 connection-event trigger fires** on peer J. Reconcile pass runs over peer J's owned `custody-blob` commitments.
4. For each commitment where peer J is the custodian and the blob is missing locally:
   - Initially `peer_blob_inventory` has no entry for `(peer_M, blob_hash)` — pass exits with no-op.
5. Within 60s, peer M's snapshot arrives on `elohim/inventory/blob`.
6. **T14 projection writer** upserts `peer_blob_inventory` with peer M's hosted set, source='gossip-snapshot'.
7. **T16 gossip-arrival trigger fires.** Reconcile pass runs again.
8. For each owned commitment matching a hash now in `peer_blob_inventory` for peer M:
   - **T17 GET-time fallback path** kicks: `FetchShard { peer_id: M, hash }` — except this is a controller-driven kick, not a GET-driven one. (The controller calls into the same fetch helper that the GET handler uses; both paths converge on the same `serve-blob` emission.)
9. Bytes arrive; verified; persisted locally; `serve-blob` REA event written.
10. **Distribution-summary view aggregator** (Phase 4 / T23 — already specified) recomputes: `replicaCount = COUNT(DISTINCT provider WHERE action='custody-blob' AND blob_hash=?)` and `currentlyHonoredCount = COUNT(DISTINCT custodian WHERE peer_blob_inventory has fresh entry)`. The badge updates: 1/3 → 2/3.

### Flow 2: Placement gap surfaces when custodian goes offline

1. Peer C (a custodian for blob X) was online and gossiping; `peer_blob_inventory` shows fresh entry.
2. Peer C goes offline. Gossip stops arriving for peer C.
3. `last_seen_at` for `(peer_C, blob_X)` ages.
4. After `placement_grace_seconds` elapses, the next reconcile pass on peer C's commitment receivers detects the gap.
5. **T16 emits a `placement-gap` REA event** (with cooldown to prevent spam).
6. **Distribution-summary aggregator** queries `economic_events WHERE action='placement-gap' AND resource_inventoried_as=blob_X`; the badge surfaces "1 commitment unhonored 5m+".
7. Topology UI shows the gap. Trust signal: the commitment exists but is not currently being honored. Recovery flows (later sprint) can consume the same signal.

### Flow 3: GET-time fallback (the user-visible recovery path)

1. User in Angular app requests blob X via `GET /api/v1/blob/<hash>` through doorway → elohim-storage.
2. Local store misses.
3. **T17 fallback path** consults `peer_blob_inventory` for hosts of blob X.
4. Filtered to currently-connected peers; iterated with per-peer timeout.
5. First hit returns bytes; verified; stored locally; `serve-blob` event written; bytes returned to user.
6. Subsequent requests for blob X hit the local store directly.

## What's NOT in scope

- **Good-Samaritan salvage** (the door-open path B from the brainstorm): a peer adopting another peer's unhonored commitment without explicit instruction. Future sprint; the `placement-gap` signal feeds that future flow but this sprint emits, doesn't consume.
- **Signed gossip** (Ed25519 over canonical bytes): structural non-empty signature only at Stage 1, per the security gradient.
- **Kademlia-routed blob discovery**: explicitly rejected. `KadStartProviding` stays narrow to EPR atom CIDs; blob discovery is libp2p-mesh-internal via the inventory gossip topic.
- **Cross-DNA reconciliation** (e.g., lamad + mishpat content commitments): out of scope. The custody reconciliation controller works against any `rea_commitments(action='custody-blob')` row regardless of which DNA contributed the commitment; no schema or controller changes for cross-DNA.
- **Multi-peer integration tests in Eclipse Che**: deferred to Jenkins per `feedback_shift_measure_jenkins`. Local TDD uses unit-level tests against mocked P2P channels and a unit-mockable reconciliation pass.
- **Bandwidth optimization** at large blob counts: at alpha topology scale (~6 peers, hundreds of blobs) the snapshot bandwidth is comfortable. Graduation to delta-only with periodic snapshot-resync (which this design's wire format already supports) becomes worth measuring at 10K+ blobs; not measured in this sprint.

## Scale ceiling

This design targets the **alpha topology**: households (~6 peers per household, bootstrap-pair pattern per `project_alpha_topology_bootstrap_pair`) plus small collectives that look topologically like enlarged households. The architecture is honest about its ceiling: it is a *full-mesh inventory broadcast* with O(N²) gossip overhead in peer count and O(M) per-peer overhead in blob count. At alpha scale (N≈10², M≈10³) bandwidth is comfortable; at global UGC scale (N→10⁶, M→10⁹) it does not work as designed.

Three concrete extensions handle scale graduation when measurement makes them necessary. None of them are in this sprint's scope, but the substrate this sprint lands does not block any of them — each is a layer added on top of the trinity.

**Bloom-filter inventory** (bandwidth optimization). Replace `BlobInventorySnapshot.hashes: Vec<String>` (N hashes × 32 bytes each = ~32 KB at 1K blobs) with a Bloom filter (~1.5 KB for 1K blobs at 1% false-positive rate). The cost is occasional false-positive fetch attempts (fetch a blob the peer doesn't actually have, get an empty 404 back, learn nothing); the benefit is two orders of magnitude bandwidth reduction. The trinity is unchanged: gossip wire becomes Bloom-filtered; `peer_blob_inventory` becomes "peers whose Bloom said they probably have this blob" with the false-positive caveat handled by the existing `record_fetch_success` strongest-evidence path. Future sprint when alpha bandwidth measurement crosses a threshold.

**Household aggregation** (peer-count optimization). Within a household (memory pin: `project_household_is_resilience_unit` — resilience is household-to-household, not peer-to-peer), peers don't gossip individual inventory across the wider mesh; instead one *household-aggregate* node (probably the household's elohim-operator blade per `project_household_fabric`) speaks the household's combined inventory to outsiders. Inside the household, full-detail gossip continues. This collapses N peers to roughly N/household_size at the cross-household level. The trinity is unchanged: each household exposes a single aggregate `peer_blob_inventory` row per blob; the inner household's per-peer detail is a household-internal concern. Compatible with alpha topology because alpha households today already have a clear primary host; no schema changes required, just an additive aggregator.

**Hierarchical routing** (mesh-scale graduation). At global UGC scale, full-mesh broadcast doesn't work — a peer can't know about every blob on the network. The graduation is to a layered routing pattern: bloom-filtered gossip within a "zone" (collective, region, or some clustering); cross-zone discovery via a directory service (likely DHT-routed at that point — but for *zone summaries*, not individual blob hashes, so the DHT-narrow principle is still honored). T17's fetch helper consumes whichever discovery layer is active without changing its contract. Compatible with alpha because alpha is a single zone; cross-zone work begins when there is more than one zone.

**The substrate this sprint lands does not have to be re-done at scale.** The trinity (manifest / reality / diff), the reconciliation controller's multi-trigger pattern, the `serve-blob` event ledger, the `placement-gap` signal, the topology UI's view aggregators — all of these survive each scale extension. What changes is the gossip wire format and the discovery layer underneath `peer_blob_inventory`. The seam is the candidate-list query inside T17's fetch helper: today it is `SELECT peer_id FROM peer_blob_inventory WHERE blob_hash = ?`; tomorrow it consults a Bloom-filtered local cache; the day after, it routes through a zone directory. The fetch helper's contract — "give me a candidate list for this blob hash" — does not change.

## Migration Plan

Seven tasks, dependency-ordered:

| # | Task | Files |
|---|---|---|
| **T12** | `peer_blob_inventory` migration + diesel schema + models | New migration directory; `db/diesel_schema.rs`; `db/models.rs`; new `db/peer_blob_inventory.rs` with `upsert_*`/`apply_snapshot`/`apply_delta`/`record_fetch_success`. |
| **T13** | Inventory gossip wire types | New `p2p/inventory_gossip.rs`: `BlobInventorySnapshot`, `BlobInventoryDelta`, MessagePack codecs, structural-verification, topic constant `INVENTORY_TOPIC = "elohim/inventory/blob"`. |
| **T14** | Inventory projection writer + sequence cursor | Hook into the existing libp2p gossipsub event handler in `p2p/mod.rs`. Add `peer_inventory_cursor` table + migration if a fresh table is preferred over piggy-backing on existing cursors. |
| **T15** | Inventory broadcast scheduler | New `p2p/inventory_broadcaster.rs`: snapshot timer (archetype-tunable), delta emitter hooked into blob-store add/remove paths, sequence allocator. Operator preset `inventory-broadcast-seconds`. |
| **T16** | Custody reconciliation controller | New `reconcile/custody.rs`: multi-trigger reconcile function, connectivity API extension on `P2PNode`, `ReconciliationMetrics` struct, kick-replication for own commitments, `placement-gap` REA event emission for others'. New migration for `placement-gap` action convention (extend the T03d index pattern). |
| **T17** | GET-time blob fallback | Modify HTTP `GET /blob/{hash}` handler in `http.rs`. Extend or add `P2PCommand::FetchBlob { peer_id, hash, reply }` for peer-targeted fetch. Emit `serve-blob` REA event on success. |
| **T18** | Filesystem parity sweep | Periodic check function in `p2p/inventory_broadcaster.rs` (or a new `p2p/parity.rs` if it grows). Diagnostic endpoint `/api/v1/diagnostics/inventory-parity`. Regression test asserting gossiped set == filesystem set after sync round. |

**Test harness implications.** Each task's unit tests run locally; integration tests that require multi-peer setup live in `tests/` and run on Jenkins. New integration tests (deferred to Jenkins): replica-grows-after-connect (Flow 1), placement-gap-surfaces-on-offline (Flow 2), GET-time-fallback (Flow 3).

**REA action vocabulary additions.** `placement-gap` joins `project-blob` / `serve-blob` / `custody-blob` from T03d. Convention: emitted when a custody commitment goes unhonored beyond grace; carried in `economic_events` (it's an observation event, not a commitment). T16's commit body documents the convention; the existing T03d composite indexes already cover the `(action, resource_inventoried_as)` query pattern.

## Security and Threat Model

- **Stage 1 (this sprint):** Structural-non-empty signatures on gossip messages; verified content hashes on fetch (no signature verification on serve-blob — the hash check is the integrity floor); reconciliation controller acts only on its own peer's commitments; placement-gap emissions are observational, not punitive.
- **Lying gossip:** A peer can claim hosting in gossip and never serve. The lie costs them nothing in Stage 1 — but the failed fetch never produces a `serve-blob` event, so the topology UI's "currently honored" count never includes them. `last_seen_at` ages out gossip-only claims; the receiving peer's `peer_blob_inventory` eventually drops them. No durable harm.
- **Kicking flood:** A peer with a large number of commitments could in principle kick a flood of fetches at every connection event. Mitigated by per-peer rate limit on kicks (default 10 kicks/peer/minute, operator-tunable) tracked in `peer_metrics`.
- **Sequence-number manipulation:** A malicious peer could emit gossip with a high sequence number to cause receivers to drop subsequent legitimate deltas. Mitigated by snapshot fallback: receivers always accept any snapshot that arrives, regardless of sequence; gap-detection requests a fresh snapshot.
- **Stage 2 (future sprint):** Ed25519 signatures over canonical gossip bytes; signature verification on inventory ingestion; placement-gap escalation thresholds for qahal-level intervention.

## Operator presets

This sprint introduces (or formalizes) these tunable knobs (4-layer cadence pattern: archetype default → policy.toml → env/CLI → sync admin trigger):

| Preset | Default (per archetype) | What it controls |
|---|---|---|
| `inventory-broadcast-seconds` | node:60, desktop:300, mobile:disabled, steward:60 | Snapshot cadence on the inventory gossip topic |
| `inventory-freshness-seconds` | 600 | TTL for `peer_blob_inventory` entries before they're considered stale |
| `custody-sweep-seconds` | 120 | Periodic reconcile-pass cadence |
| `placement-grace-seconds` | 300 | How long a commitment can be unhonored before a `placement-gap` event fires |
| `placement-gap-cooldown-seconds` | 1800 | Minimum time between repeated `placement-gap` events for the same commitment |
| `kick-fetch-per-peer-per-minute` | 10 | Rate limit on reconciliation-driven fetches per peer |
| `fetch-blob-timeout-seconds` | 5 | Per-peer timeout within T17's racing batch |
| `fetch-blob-parallelism` | 3 | How many candidates T17 races concurrently per batch |

## Test plan

- [ ] T12: migration applies cleanly; up/down round-trip; diesel schema compiles; models construct correctly.
- [ ] T13: snapshot/delta wire types round-trip via MessagePack; structural-verify rejects empty fields.
- [ ] T14: snapshot apply replaces per-peer set; delta apply respects sequence; gap-detect queues a snapshot request.
- [ ] T15: scheduler emits at archetype cadence; delta emitter collapses bursts within window; sequence is monotonic.
- [ ] T16: reconcile pass is idempotent; multi-trigger fires on each event source; act-on-own commitments path emits `serve-blob` on simulated fetch-success; signal-on-others' path emits `placement-gap` after grace; cooldown suppresses re-emission.
- [ ] T17: HTTP blob handler returns local hit on local presence; on local miss, iterates candidates from `peer_blob_inventory`, returns first hit, persists locally, emits `serve-blob`; returns 404 on all-miss; rejects bytes whose hash doesn't match.
- [ ] T18: parity sweep detects mismatch; broadcast scheduler corrects on next snapshot; regression test asserts post-sync filesystem-count == gossiped-count.

Multi-peer integration scenarios run on Jenkins:
- Flow 1 (replica-grows): seed a custody commitment on peer J for a blob hosted on peer M; connect them; assert replica count rises.
- Flow 2 (placement-gap-surfaces): seed a commitment with custodian peer C; take peer C offline; assert placement-gap emits after grace and is visible via the distribution-summary view.
- Flow 3 (GET-time fallback): seed peer J without a blob, peer M with the blob; GET via doorway → peer J; assert the bytes are served, persisted on J, and a `serve-blob` event lands.

## Related

- [Light Up the Topology — Operational Visibility Sprint Design](2026-05-01-light-up-the-topology-design.md) — the parent sprint; Phases 0/1 complete, Phases 3-11 resume after this lands
- T03d action conventions (in the parent sprint plan) — `custody-blob` / `project-blob` / `serve-blob` are the manifest layer this design's reality + diff layers operate against
- Memory pin `project_principle_p1_reconciliation_controller` — the k8s-style controller pattern this design instantiates for blob custody
- Memory pin `project_three_layer_truth_model` — DHT (manifest) / libp2p (reality) / doorway (projection) — preserved exactly
- Memory pin `project_inventory_exchange_not_byte_replication` — the failure mode T18's parity sweep defends against
- Memory pin `project_placement_signals_are_shefa_inputs` — placement-gap is structured economic signal, not operational warning
- Memory pin `project_dht_vs_libp2p_scoping` — narrow DHT to integrity; operational state on libp2p; this design honors the boundary
