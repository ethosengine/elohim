# elohim-hub / elohim-node / elohim-storage — Boundary Design

**Status:** Design (pre-refactor scaffold; no code moves this sprint)
**Date:** 2026-05-02
**Predecessor specs:** [Light Up the Topology](2026-05-01-light-up-the-topology-design.md), [Blob Custody Reconciliation](2026-05-02-blob-custody-reconciliation-design.md)
**Memory anchors:** `project_elohim_hub_elevation`, `project_hub_archetype_abstraction`, `project_substrate_scale_ceiling`, `project_three_layer_truth_model`, `project_household_horizontal_scaling`, `project_elohim_node_role`

## Why this exists

The light-up-topology sprint surfaced a vocabulary gap. We are wiring substrate primitives — blob custody reconciliation, view federation, peer topology — but the architecture doc still describes elohim-node as "a deployment wrapper that packages elohim-storage." The wrapper framing is no longer load-bearing. The thing we are actually building is the **runtime composition primitive that scales the protocol while keeping it human-scale**.

This document names the three crates' responsibilities, sketches the `Hub` trait that elohim-node will graduate into, and identifies what stays where. **No code moves in this sprint.** The intent is to make Phase 3+ decisions land in the right crate the first time, and to flag the refactor that the next sprint should pick up.

## The three layers

```
┌─────────────────────────────────────────────────────────────────┐
│ elohim-hub   — composition primitive (Hub trait)                │
│   trait Hub { id, archetype, governance, storage_budget,        │
│                operator_surface, federation_contract, ... }     │
│   impl HouseholdHub                                             │
│   impl CollectiveHub                                            │
│                                                                 │
│   Question this sprint: new crate, or trait inside elohim-node? │
│   Default: trait module inside elohim-node until a second       │
│   consumer (operator UI, fixtures crate) needs it independently.│
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ embeds
                              │
┌─────────────────────────────────────────────────────────────────┐
│ elohim-node  — hub instance (one running process)               │
│   • Composes elohim-storage + elohim-agent + elohim-bitswap     │
│   • Runs cluster discovery (mDNS), leader election, pod monitor │
│   • Owns operator-side surfaces: dashboard router, registration │
│   • Today: cluster/, network/, pod/, dashboard/, sync/, storage/│
│   • Becomes: HouseholdHub or CollectiveHub instance, depending  │
│              on archetype declared at boot                      │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ depends on (with `p2p` feature)
                              │
┌─────────────────────────────────────────────────────────────────┐
│ elohim-storage — substrate participant (state + projections)    │
│   • Canonical libp2p participant per project_three_layer_truth  │
│   • DHT-projecting state: REA commitments, peer_blob_inventory, │
│     bindings, FeedbackSignal projections                        │
│   • HTTP API surface (camelCase boundary)                       │
│   • view-federation/1.0.0 (Phase 3, just landed)                │
│   • blob/1.0.0 (Phase 2, just landed)                           │
│   • Knows nothing about hubs — speaks peer-to-peer              │
└─────────────────────────────────────────────────────────────────┘
```

The arrows do not reverse. elohim-storage stays oblivious to hub composition; what it sees are peers and contracts. elohim-node assembles hub-shaped behavior on top of multiple storage instances (when the household has multiple blades) and wires in the operator surface. elohim-hub names the abstraction.

## Hub trait — sketch

```rust
// elohim-node/src/hub/mod.rs (or future elohim-hub crate)

/// A Hub is the runtime composition primitive that keeps a piece of the
/// substrate human-scale. Hubs scale by federation between hubs, not by
/// growing any single hub past what the humans inside it can govern.
///
/// Per project_substrate_scale_ceiling: substrate participants are Tier 3
/// nodes. A hub IS the composition that makes a Tier 3 node coherent.
pub trait Hub: Send + Sync {
    /// Stable identity for this hub. Content-derived where possible
    /// (per project_first_class_graph_pattern).
    fn id(&self) -> &HubId;

    /// Which kind of hub this is. Drives governance shape, federation
    /// behavior, and operator surface.
    fn archetype(&self) -> HubArchetype;

    /// What human-scale governance applies to this hub. For HouseholdHub:
    /// household members (qahal-bounded). For CollectiveHub: the collective's
    /// stewardship contract (per project_social_compute_collective_is_stewardship_unit).
    fn governance(&self) -> &dyn Governance;

    /// How much this hub commits to host. Bounded by what humans inside
    /// can steward; growth = more hubs, never bigger hubs.
    fn storage_budget(&self) -> StorageBudget;

    /// The operator UX endpoint set. HouseholdHub: family dashboard.
    /// CollectiveHub: stewardship admin (varies by collective shape).
    fn operator_surface(&self) -> &dyn OperatorSurface;

    /// Outbound federation policy: which other hubs we federate with,
    /// and on which protocols. Per project_inventory_exchange_not_byte_replication
    /// gossip is metadata-only; byte movement is single-target dispatch.
    fn federation_contract(&self) -> &FederationContract;

    /// All elohim-storage participants composing this hub. A HouseholdHub
    /// with two blades has two; a single-process hub has one. The substrate
    /// sees each one as a peer; the hub sees them collectively.
    fn storage_participants(&self) -> &[StorageHandle];
}

pub enum HubArchetype {
    Household,    // intimate-circle scale, family/extended-family governance
    Collective,   // community/institutional scale, varies in shape
}
```

## What is hub-shape today vs not

| Module / file | Layer it belongs in | Today | Direction |
|---|---|---|---|
| `elohim-storage/src/p2p/*` | substrate | substrate participant; oblivious to hubs | Stays. Hub-aware behavior never leaks here. |
| `elohim-storage/src/services/*` | substrate | view aggregation over local + federated state | Stays. Federation queries other peers, not other hubs. |
| `elohim-storage/src/views.rs` | substrate | wire shapes; ts-rs export | Stays. View kinds may grow (e.g. `HubTopology`) but the file itself is substrate. |
| `steward/node/src/cluster/*` | hub-internal | mDNS discovery, leader election, membership | Becomes `HouseholdHub::cluster()` — already hub-shape, just unnamed. |
| `steward/node/src/network/operator.rs` | hub | operator identity + permissions | Becomes part of `OperatorSurface`. (Pre-existing `OperatorRelationship::Owner` should be renamed per `project_no_sovereignty_stewardship_over_ownership` — flagged as cleanup, not blocking.) |
| `steward/node/src/network/registration.rs` | hub | how a node joins a hub | Becomes `Hub::register_participant()` flow. |
| `steward/node/src/network/sync_state.rs` | hub-internal | inter-blade sync within a household | Stays inside HouseholdHub; CollectiveHub will not have this shape. |
| `steward/node/src/pod/*` | hub-internal | k8s-style pod monitor / consensus | Stays inside HouseholdHub. Maps to `project_household_fabric`. |
| `steward/node/src/dashboard/*` | hub-presentation | operator-facing UI server | Becomes `HouseholdHub::operator_surface()`'s router. |
| `steward/node/src/p2p/*` | hub libp2p | secondary swarm for cluster ops | This is interesting — see "two libp2p swarms" below. |
| `steward/node/src/elohim_service.rs` | hub | embeds elohim-storage in the runtime | Becomes `HouseholdHub::storage_participants()` provisioning. |

## Two libp2p swarms: substrate vs hub-internal

elohim-storage runs a libp2p swarm (the substrate). steward/node also has a `p2p/` module that runs its own libp2p — but for a different purpose: cluster orchestration *within* the hub (between blades on the same household network). These should not collapse into one swarm; they are different scales:

| Swarm | Purpose | Membership | Cadence |
|---|---|---|---|
| **Substrate** (in elohim-storage) | Hub-to-hub federation; protocol participation; the alpha cluster topology per `project_alpha_topology_bootstrap_pair` | Every Tier 3 node in the protocol | Bounded by deliberate federation policy |
| **Hub-internal** (in elohim-node) | Blade-to-blade within a household; pod consensus; failover | Only this hub's blades | Tight, local, mDNS-first |

The Hub trait owns the substrate-side handle (the storage participants); the hub-internal swarm is private to the HouseholdHub implementation. CollectiveHub may not need a hub-internal swarm at all — that decision is per-archetype.

## Migration story (later sprint, not now)

A reasonable sequence when this gets picked up:

1. **Crate decision.** Inside elohim-node first (`mod hub`). Promote to `elohim-hub` only if a second consumer (operator UI compiled separately, fixtures crate, simulation harness per `project_hub_archetype_abstraction`'s `@wip` note) needs it.
2. **Add `Hub` trait + `HouseholdHub` skeleton** that re-exports today's modules behind it. No behavior change; just naming.
3. **Move `cluster::`, `pod::`, `dashboard::`** into `HouseholdHub` impl methods.
4. **Add `CollectiveHub` skeleton** with empty impls; wire one behavior (e.g. `governance()`) end to end.
5. **Reframe `network::operator`** as `OperatorSurface` trait; rename `Owner → Steward` per stewardship vocabulary; archetype-vary the permission shape.
6. **Federation contract reads cross-hub topology** — at this point Phase 3+ federation work in elohim-storage has a typed Hub-to-Hub call site, not just peer-to-peer.

This sprint: items 0 and 1 only — write this doc, name the boundary. The rest is its own sprint.

## What this changes for the current sprint

Phase 3+ of `2026-05-01-light-up-the-topology-plan.md` writes federation primitives in elohim-storage that today communicate peer-to-peer. With the hub framing in place we can ask, for each new piece of federation work, **"is this expressing a hub-to-hub relationship, or a peer-to-peer relationship?"** Most of the topology views (`MyClusterView`, `PeerTopologyView`) already implicitly assume a hub-shape: the bindings expand to *this human's peers*, which is hub-internal. The federation protocol itself is hub-to-hub when the requester and responder are on different hubs.

Concretely:

- **Federation request authentication** (Phase 3 finish) should be designed so a future `HubId` could be carried alongside the requester's agent_cid without breaking the wire — even though only `agent_cid` is meaningful today.
- **`MyClusterView`** is already a hub-internal aggregation in disguise (one human's bindings → one human's hub's storage participants). Naming this in the doc will help when CollectiveHub appears.
- **No DNA changes.** The DHT remains the manifest layer; hubs compose runtime behavior on top.

## Non-goals for this sprint

- Moving any code.
- Promoting `elohim-hub` to its own crate.
- Defining the full `OperatorSurface` trait.
- Specifying the federation contract beyond what Phase 3 already implements.
- Touching CollectiveHub at all — it remains a paper concept until the hub fixture harness exists (`project_hub_archetype_abstraction`).

## Plan touch-up (this doc's only artifact in code)

The companion plan (`2026-05-01-light-up-the-topology-plan.md`) gets a short framing block added near Phase 3 referencing this design, plus a Phase 2 reality update — Phase 2 actually shipped T12-T23 (blob custody reconciliation per `2026-05-02-blob-custody-reconciliation-design.md`) rather than the originally scoped T12-T15. The plan's task numbering is otherwise preserved.

## Open questions (deferred)

1. **Where does the hub identity come from?** Content-derived from genesis configuration? Notarized via DHT? Self-signed at first boot? Probably starts self-signed and graduates per `project_bootstrap_to_elohim_security_gradient`.
2. **How does a household with two blades present as one substrate peer?** Today each blade runs its own elohim-storage and shows up as a separate libp2p peer. The hub-internal swarm could let one blade speak for the household, but that is a federation-aggregation question, not a substrate-identity question. Defer until needed.
3. **Does CollectiveHub federate via the same protocol as HouseholdHub?** Likely yes for view federation; possibly no for blob custody (collectives may have institutional storage with different commitment shapes). Out of scope until a CollectiveHub exists.

---

**This document's job is done when** every implementer touching Phase 3+ federation work in this sprint can answer: "Where does this code live: substrate (elohim-storage), hub (elohim-node), or hub-abstraction (elohim-hub trait, future)?" — and pick the right answer without a vocabulary collision.
