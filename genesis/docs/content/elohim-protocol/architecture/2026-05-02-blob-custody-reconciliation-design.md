---
title: Blob Custody Reconciliation — Design
id: blob-custody-reconciliation-design
tier: architecture
status: Design (canonical)
created: 2026-05-02
pillar coupling: elohim (substrate primitive), shefa (placement signals are economic inputs)
informed-by:
  - 2026-05-01-light-up-the-topology-design.md (Light Up the Topology — parent sprint)
informs:
  - All future custody / replication / recovery specs that reconcile commitments against observed reality
  - The placement-gap → shefa economic-planning loop (subsidies, peer recruitment, recovery routing)
  - Scale graduation of the inventory plane (bloom-filter / household-aggregation / hierarchical routing)
memory_anchors:
  - project_principle_p1_reconciliation_controller
  - project_three_layer_truth_model
  - project_dht_vs_libp2p_scoping
  - project_inventory_exchange_not_byte_replication
  - project_placement_signals_are_shefa_inputs
  - project_substrate_scale_ceiling
  - project_cadence_archetype_tunable_with_dev_overrides
# Bidirectional: the ADR that distills the implementation lessons points back here.
history:
  - ../history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
---

# Blob Custody Reconciliation — Design

**Predecessor:** [Light Up the Topology](2026-05-01-light-up-the-topology-design.md)
**Sibling:** [elohim-hub / elohim-node / elohim-storage Boundary Design](2026-05-02-elohim-hub-boundaries-design.md)
**Memory anchors:** `project_principle_p1_reconciliation_controller`, `project_three_layer_truth_model`, `project_dht_vs_libp2p_scoping`, `project_placement_signals_are_shefa_inputs`

## Why this exists

Resilience IS resilience; *visibility* of resilience is what builds trust, safety, and acceptance. The
topology must show two things faithfully: replicas growing toward target after a peer connects, and
commitments left unhonored beyond grace surfacing as a structured signal — not an alarm. Both require a
substrate that knows three things: who *should* host what, who *currently* hosts what, and the diff.

The protocol already declares custody contracts via `rea_commitments(action='custody-blob')`. The design
this document names is **manifest-vs-reality-first**, not discovery-first: it observes who currently has
what, and runs a controller that reconciles the diff. (The discovery-first alternative — Kademlia
provider-lookup for blob hashes — was explicitly rejected; `KadStartProviding` stays narrow to EPR *atom*
CIDs. See the history ADR for why that path was not taken.)

## The three-surface reconciliation pattern

The architecture mirrors a Kubernetes control-plane, with elohim's three-layer truth model
(`project_three_layer_truth_model`) supplying each surface:

| Surface | What it is | Storage | Authority |
|---|---|---|---|
| **Manifest** (desired) | "Peer X commits to host blob Y for steward Z for N seconds." | `rea_commitments` rows: `action='custody-blob'`, `resource_classified_as=<blob_hash>`, provider/receiver as steward CIDs. | **DHT-notarized. Signed. Cannot be forged.** |
| **Reality** (observed) | "Peer X currently hosts blobs A, B, C — gossiped at time T, sequence S." | `peer_blob_inventory(peer_id, blob_hash, last_seen_at, source, sequence)`, fed by libp2p gossipsub on `elohim/inventory/blob`. | **Operational. Eventually consistent. Falsifiable** — but a lie collapses on first failed fetch. |
| **Diff** | The controller's input | Computed on demand; not stored. | Drives kicks (own commitments), placement-gap signals (others'), and topology badges. |

**Why three surfaces, not two.** A two-surface model forces every observation to bake into either the
contract or the operational state. The diff surface lets the controller stay stateless about *outcomes* —
it computes drift fresh each pass — while still emitting durable artifacts (REA events, placement-gap
signals) for the topology UI and downstream shefa flows.

**Why the manifest is signed but reality isn't** (`project_dht_vs_libp2p_scoping`). Custody commitments are
economic contracts; forgery there would let a peer claim hosting authority it lacks. Reality is operational
chatter — falsehoods collapse on the first failed fetch (no `serve-blob` event lands), and `last_seen_at`
ages out stale entries. Signing every gossip message would impose DHT-level cost on libp2p-level data.
**Verifiable serves are the integrity floor; signed gossip is unnecessary.** Symmetrically, the fetch path
enforces a content-hash check before persisting — integrity is enforced at every layer that writes durable
state, never deferred to a signature the operational layer doesn't carry.

## Placement signals are shefa economic inputs

This is the load-bearing teaching, not an aside. A placement gap, a missed verification, a reconstruction
event, an over-extended commitment — these are **not operational warnings**. They are structured economic
signals that flow up into shefa to drive planning: where new peer support is needed (more nodes), where
subsidies should flow, who needs recovery or repair, who is over-extended
(`project_placement_signals_are_shefa_inputs`).

The dataplane's imperfect reality is the shefa layer's *input surface* — resilience degradation is how the
elohim learns where to act economically. Design consequences:

- Every dataplane anomaly is a **structured, queryable REA record** (`placement-gap` carried in
  `economic_events`), never just a log line or a toast.
- The same record feeds two consumers at once: the topology UI badge ("1 commitment unhonored 5m+") *and*
  the economic-planning loops that subscribe to subsidy / recruitment / recovery decisions. Signal surfaces
  belong in shefa views, not buried in storage logs.
- This is the *positive* case of sense-and-respond: the mesh's breathing — replicas growing, not just
  crashing — is data for the guardian loop, not only its failures.
- Litmus when adding any dataplane flag: **"what shefa decision would this inform?"** If none, it is noise;
  if some, it deserves a structured record.

A future **Good-Samaritan salvage** path consumes the same `placement-gap` signal: a peer with spare
capacity sees the gap and (per consent policy) commits as a new custodian, healing the network without
centralized coordination. The signal emits here; that flow consumes it later. This is a feature of treating
the gap as an economic signal rather than an error.

## The reconciliation controller

The diff engine instantiates the storage-as-reconciliation-controller principle
(`project_principle_p1_reconciliation_controller`): DHT is the manifest, libp2p is the controller. A
**reconcile pass** is one idempotent function called from any of three triggers, so it is safe to call
repeatedly:

- **Gossip arrival** — a snapshot/delta processed for peer X → reconcile commitments where X is provider or
  receiver.
- **Connection event** — `ConnectionEstablished` for peer X → reconcile before the first gossip arrives.
- **Periodic sweep** — a timer catches anything the event-driven triggers missed.

For each `custody-blob` commitment where **this peer is the provider** and the blob is missing locally, the
pass queries `peer_blob_inventory` for fresh candidates and kicks a fetch (sharing the GET-time fetch
helper). For each commitment where **this peer is the receiver** (the content steward) and the provider has
aged past `placement_grace_seconds`, the pass emits a `placement-gap` REA event — with a cooldown so the
stream isn't flooded with duplicates.

The reality plane is fed by archetype-tunable gossip (`project_cadence_archetype_tunable_with_dev_overrides`
— the 4-layer archetype→policy→env→admin cadence): always-on blades broadcast frequently, battery-precious
mobiles default to disabled. Snapshots replace per-peer state authoritatively (also the recovery path from
any sequence-manipulation); deltas apply incrementally and gap-detect via a monotonic per-peer sequence,
requesting a fresh snapshot on a gap.

**Inventory gossip is not byte replication** (`project_inventory_exchange_not_byte_replication`). Gossip can
run cleanly while bytes never actually replicate. A filesystem-parity sweep defends the regression: it
compares the locally-hosted hash set against the last-gossiped set, and the next authoritative snapshot
naturally corrects any drift.

## Scale ceiling and the graduation seam

This design targets the **alpha topology** (`project_substrate_scale_ceiling`): households (~6 peers, the
bootstrap-pair pattern) plus small collectives that look topologically like enlarged households. It is
honest about its ceiling — a full-mesh inventory broadcast, O(N²) in peer count and O(M) per-peer in blob
count. At alpha scale (N≈10², M≈10³) bandwidth is comfortable; at global UGC scale it is not.

Three extensions handle graduation **without re-doing the substrate** — the trinity (manifest / reality /
diff), the multi-trigger controller, the `serve-blob` ledger, the `placement-gap` signal, and the topology
view aggregators all survive each one:

- **Bloom-filter inventory** (bandwidth): the gossip wire carries a Bloom filter instead of a hash list
  (~32 KB → ~1.5 KB at 1K blobs), trading occasional false-positive fetches for two orders of magnitude
  less bandwidth; the `record_fetch_success` strongest-evidence path absorbs the false positives.
- **Household aggregation** (peer count): one household-aggregate node speaks the household's combined
  inventory to outsiders, collapsing N peers to ~N/household_size at the cross-household level, while
  full-detail gossip continues inside the household.
- **Hierarchical routing** (mesh scale): bloom-filtered gossip within a zone; cross-zone discovery via a
  directory of *zone summaries* (DHT-routed at that point — but for summaries, not individual blob hashes,
  so the DHT-narrow principle still holds).

**The graduation seam is one query:** the candidate-list lookup inside the fetch helper. Today it is
`SELECT peer_id FROM peer_blob_inventory WHERE blob_hash = ?`; tomorrow a Bloom-filtered cache; the day
after, a zone directory. The fetch helper's contract — "give me a candidate list for this blob hash" —
never changes.

## Related

- [Light Up the Topology](2026-05-01-light-up-the-topology-design.md) — the parent sprint
- [elohim-hub Boundary Design](2026-05-02-elohim-hub-boundaries-design.md) — names which crate this lands in
- [Tiered Quilt Stewardship](2026-05-11-tiered-quilt-stewardship-design.md) — where the bytes actually live
- [History/ADR: The DHT is a notary, not a byte store](../history/2026-06-01-dht-is-a-notary-not-a-byte-store.md) — distills why discovery-first was the wrong path
- Implementation detail (task breakdown, wire types, SQL, operator presets) lives in the sprint spec:
  `genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md`
