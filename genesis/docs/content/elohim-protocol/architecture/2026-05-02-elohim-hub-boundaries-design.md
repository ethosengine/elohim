---
title: elohim-hub / elohim-node / elohim-storage — Boundary Design
id: elohim-hub-boundaries-design
tier: architecture
status: Design (pre-refactor scaffold; no code moves this sprint)
created: 2026-05-02
pillar coupling: elohim (substrate), infrastructure (runtime composition)
informed-by:
  - 2026-05-01-light-up-the-topology-design.md (Light Up the Topology)
  - 2026-05-02-blob-custody-reconciliation-design.md (Blob Custody Reconciliation)
# Compacted into the "Doorway / hub edge" section below (raw bodies retire to git):
compacted_from:
  - genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md
  - genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
informs:
  - All future hub-archetype implementations (HouseholdHub, CollectiveHub)
  - All Phase 3+ work that needs to land in the right crate the first time (hub vs node vs storage)
  - All future runtime composition specs (operator UI, fixtures crate) that consume the Hub trait
  - The doorway-vs-hub responsibility split and the four reach-earning surfaces at hub scale
memory_anchors:
  - project_elohim_hub_elevation
  - project_hub_archetype_abstraction
  - project_substrate_scale_ceiling
  - project_three_layer_truth_model
  - project_household_horizontal_scaling
  - project_elohim_node_role
  # Carried from the hub-edge spec (B.4 amendment — memory↔doc edges preserved):
  - project_reach_earned_at_authoring
  - project_social_reach_nervous_system
  - project_trust_as_efficiency_signal
  - project_dht_vs_libp2p_scoping
  - project_doorway_single_target_no_fanout
  - project_doorway_peer_registration
  - project_doorway_manifest_driven_routes
  - project_peer_native_account_canonical_surface
  - project_p2p_is_hosting
  - project_household_fabric
  - project_substrate_floor_elohim_ceiling
  - project_signal_kind_extensible_protocol_class
  - project_social_compute_collective_is_stewardship_unit
  - project_intelligence_revolution_scales_to_humans
  - project_redeploy_the_substrate
  - project_household_is_resilience_unit
  - project_multi_doorway_human_registration
  - project_inventory_exchange_not_byte_replication
  - project_reach_gate_is_elohim_mediated_matchmaking
  - project_doorway_hub_sister_brother
  - project_hub_compute_aggregate_primary
# Bidirectional history edge (PLACEMENT.md): settled node/household/doorway decisions distilled here.
history:
  - ../history/2026-04-19-d1-through-d5-node-and-household-canon.md
---

# elohim-hub / elohim-node / elohim-storage — Boundary Design

**Status:** Design (pre-refactor scaffold; no code moves this sprint)
**Date:** 2026-05-02
**Predecessor specs:** [Light Up the Topology](2026-05-01-light-up-the-topology-design.md), [Blob Custody Reconciliation](2026-05-02-blob-custody-reconciliation-design.md)
**Memory anchors:** `project_elohim_hub_elevation`, `project_hub_archetype_abstraction`, `project_substrate_scale_ceiling`, `project_three_layer_truth_model`, `project_household_horizontal_scaling`, `project_elohim_node_role`

## Why this exists

The light-up-topology sprint surfaced a vocabulary gap. We are wiring substrate primitives — blob custody reconciliation, view federation, peer topology — but the architecture doc still describes elohim-node as "a deployment wrapper that packages elohim-storage." The wrapper framing is no longer load-bearing. The thing we are actually building is the **runtime composition primitive that scales the protocol while keeping it human-scale**.

This document names the three crates' responsibilities, sketches the `Hub` trait that elohim-node will graduate into, and identifies what stays where. **No code moves in this sprint.** The intent is to make Phase 3+ decisions land in the right crate the first time, and to flag the refactor that the next sprint should pick up.

### What "scales the protocol" means — the reach math

The substrate is **not** built for FB/YT-shape hyperscale; it is built for a federated topology where Tier 3 family nodes (`hardware-spec.md`) are the substrate participants — closest analogy email/Mastodon federation, except each "instance" is a household, church basement, or community-center serving its trust-network members *deeply*, not one operator serving thousands shallowly. The reach is **~100M Tier 3 nodes × tens-to-hundreds of humans carried each = billions of participants, most without owning hardware**: a single Tier 3 carries its household, its spokes (laptops/phones syncing to the hub), custodial key hosting for less-technical relatives, and relational backup for the trust network. **Count humans carried (billions), not nodes (100M).** Doorway absorbs the web2 mass-readership at CDN scale — the substrate never sees Stage 1/2 visitors. Per-node load is therefore bounded by *trust-network membership* (realistic per-Tier-3 connection count 100–500), which is why most apparent "scale work" (bloom inventory, hierarchical aggregation, tiered storage) is **topology-expression, not new architecture** — it just makes substrate routing match what is already socially and hardware-true. The substrate-level care (narrow integrity layer, content-addressed identity, migration-preserves-everything) is precisely the load-bearing layer for *inclusion*: it is what makes entry-tier participation honest rather than extractive. **Never frame the protocol as "for the rich who can afford Tier 3"** — Tier 3 nodes are the substrate participants who carry billions through trusted hosting and hub-and-spoke; Stage 1/2 users are first-class, their substrate rights guaranteed by the same constitutional contracts the operators run on.

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

### Horizontal scaling is the operator's placement job

A hub grows by adding blades, never by growing one process — `storage_participants()` lengthens, the hub identity does not change. The unit of deployment is the lightweight elohim-node binary; *design it to be cheap to run many of, not to scale up internally.* When a family outgrows one node the operator distributes **purpose** across the fleet — primary anchor (conductor + storage + doorway), content-stewardship nodes, inference/model-serving nodes, per-person nodes (each member's conductor + source chain on dedicated hardware), guest nodes (grandma slides her blades into the rack). The operator's job is placement (which blade runs what), local optimization (what's replicated for LAN speed vs. what rides the DHT), and lifecycle (blade joins/leaves). The substrate sees each node as a peer; the hub sees them collectively.

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
2. **How does a household with two blades present as one substrate peer?** Today each blade runs its own elohim-storage and shows up as a separate libp2p peer. The hub-internal swarm could let one blade speak for the household, but that is a federation-aggregation question, not a substrate-identity question. Defer until needed. *Direction for the human-facing answer:* the surface a human sees is the **hub aggregate**, not a per-device breakdown — a hub is a storage pool rolling up its members' capacities (sliding a blade in jumps "5GB / 15GB" to "5GB / 100GB" without changing the human's sense of "my hub"). Substrate truth stays per-device (system_metrics probes + rea_commitments); the projection layer rolls it up with progressive disclosure by capability (kids/grandma see the two-tuple; power users see the stewarded/self triptych and drill down to per-device tiles). Build the per-device substrate without hub-aggregate coupling, but design the projection to roll up cleanly.
3. **Does CollectiveHub federate via the same protocol as HouseholdHub?** Likely yes for view federation; possibly no for blob custody (collectives may have institutional storage with different commitment shapes). Out of scope until a CollectiveHub exists.

---

**This document's job is done when** every implementer touching Phase 3+ federation work in this sprint can answer: "Where does this code live: substrate (elohim-storage), hub (elohim-node), or hub-abstraction (elohim-hub trait, future)?" — and pick the right answer without a vocabulary collision.

---

## Doorway / hub edge (amended 2026-06-02 — compacts the hub-edge + stewardship-chain cluster)

> This section folds two docs that named the same topic at two layers: the **architecture boundary**
> (`2026-05-08-doorway-hub-edge-design.md`) and the **landed standing-succession slice**
> (`2026-05-19-doorway-stewardship-chain-design.md`). Their raw bodies retire to git history.

### Doorway-vs-hub responsibility split

Doorway is the **per-deployment web2 projection surface** and should stay simple — self-hosted, human-operable, the kind of thing a household steward runs on a single blade (per `project_doorway_single_target_no_fanout`: it moves bytes to one target and caches; it does not fan out). The **aggregate-scale** concerns — cross-deployment threat coordination, mobile inference processing, workload state migration, social-resilient compute contracts — belong at the **hub** layer (the home-node cluster that stewards a family or collective's compute, federates horizontally with peer hubs, coordinates discernment via elohim-operators). The arrows of this seed's three-layer diagram do not reverse here either: doorway projects, the hub composes.

**Doorway and hub are symmetric projection edges, not the same thing.** Both project the *same* canonical DHT/libp2p truth; they differ only in audience and reach contract — which way the truth faces:

- **Doorway projects outward to web2** — CDN/DNS/TLS, OAuth-relying-party for browsers, federation to other doorways (DNS bonding, federation registry), AT Proto / ActivityPub interop. Doorway is **not** a P2P participant.
- **Hub projects inward to nearby peers** — aggregates substrate truth for peers in the same household / school / village. The teacher-laptop hosting a Khan library that student devices sync from the moment they walk in. Hub **is** a P2P participant; it federates hub-to-hub, peer-native.

A village's hub may *peer with* a doorway when it wants a public web2 face, but the hub stands alone without one. The design test for any new projection feature: **is this serving browsers + other doorways, or nearby peers, or both?** A view contract (cluster-view, peer-topology, reciprocity, distribution, doorway-dashboard) that is valid on one edge should not bake the other edge's assumptions in — serve it from both where it makes sense.

### The four reach-earning surfaces at hub scale

The protocol already earns reach at **message authoring** (`project_reach_earned_at_authoring`, `project_social_reach_nervous_system`). The hub layer extends the *same* earning signal to four aggregate-scale surfaces:

1. **Compute reach** — does this inference request / observer stream / workload migration / sponsored-compute call earn hub cycles? Mobile devices delegate inference to their dwelling hub; cross-hub sponsored compute is itself a continuously-negotiated REA contract.
2. **Distribution reach** — does this traffic propagate across federated hubs? Cross-hub gossip, load balancing, content fanout, federation projection all spend the same signal. **A pattern shaped like a DDoS attack is structurally just unearned distribution reach** — it dies at the first unconvinced hub. There is no central anti-spam classifier; there is the cumulative judgment of every hub.
3. **Defense reach** — defense is a **side-effect of earning, not a bolt-on firewall**. The hub fabric simply doesn't engage with unearned reach.
4. **AI-coordination reach** — elohim-operator discernment at hub scale spends the same earning signal.

**"DDoS = unearned reach"** is the throughline that makes the doorway/hub boundary coherent.

### Vocabulary: DwellingHub / CollectiveHub

`HouseholdHub` is a **retired synonym for `DwellingHub`** — use DwellingHub. CollectiveHub carries a different *attitude* (institutional/community-scale governance) but the same Hub trait. Keep the substrate hub-kind-agnostic (`dwelling | collective | computed` resolve in UI labels only, per `project_hub_archetype_abstraction`).

**Why intentionally separate implementations, not one parametric `Hub` with a `realm:` flag.** Governance considerations do not degrade gracefully. A dwelling's "we sync everything because we trust each other and live together" does not translate to a congregation hub where new spokes need consent to join, content visibility has institutional defaults, and removing a spoke is a community-governance event. A `realm` parameter would push governance into config flags and lose realm character. This is the **same pattern as elohim agents** (`human-elohim`, `household-elohim`, `collective-elohim` are separate specializations precisely because their contracts are different *shapes*, not different *settings*) — mirror it when adding future hub types (enterprise, civic): two implementations behind a narrow shared interface, never a parameter. Use **stewards**, never "members," for the humans with authority over a hub — membership is a passive category; stewardship carries agency, accountability, and the constitutional power below.

**Constitutional rule — hub hardware MUST be steward-accessible.** A hub's access path must be inspectable, modifiable, retrievable, and must not depend on a third party who is not a steward. If access is denied or revoked by anyone other than a steward, the stewards retain the power to **quarantine** (mark unusable, reassign duties, treat its data as unrecoverable until access restored) or **evict** (remove from hub composition, halt routing through the device, notarize the eviction). Inaccessibility is a *violation*, not a normal failure mode — this is a constitutional power, not an operational override.

**The encryption boundary terminates at the hub↔spoke edge.** A hub is an always-on, physically-present, centralized theft target: if it is stolen you lose data for every steward and every spoke that syncs through it. This makes hub at-rest encryption + key custody load-bearing in a way device-level OS custody on phones/laptops is not. The model: end-to-end-style encryption runs **peer↔hub**, terminates at the **hub boundary** on the way down, and **device-level OS custody** takes over on the spoke (the unencrypted-at-rest terminus we already accept — the phone/laptop OS does that work). Hubs sit in the middle ground that needs explicit design: at-rest encryption with steward-held keys, hardware-key-bound (TPM/secure-enclave on Tier 3) so theft yields ciphertext, key recovery via the same steward-quorum that handles eviction/quarantine (ties to `project_socially_derived_security`). CollectiveHub may warrant a different encryption profile than DwellingHub (institutional vs intimate trust — open). Do not accidentally re-introduce always-encrypted-at-rest on spokes (that's the OS layer's job) or unencrypted-at-rest on hubs (the theft target).

**Hyperscaler-fronting is a constitutional constraint**, not the protocol's DDoS answer: it is operator opt-in, never required, and the defense story must be **household-scale-implementable** (a family on one blade can run it). Note the two source docs used *different* reach ladders — both are preserved here; do not silently merge them into one.

### The three-tier stewardship chain (the "who has standing to operate the doorway" instance)

The stewardship chain IS the standing/custody-succession instance of this boundary: it answers **"who has standing to operate the doorway"** as a chain of attestations, not a config flag. **Substrate LANDED:** the `operate-doorway` action, the schemas, `auth/operator.rs`, and the `DoorwayOperatorBindingView` projection.

> **HELD — not done:** the **wiring** (task-#16) is NOT in the DNA. The imagodei `Attestation` kinds, the `verify_custody_chain` traversal, and the `request_kind` for custody transitions are **absent**. Read this as "substrate landed, chain not yet wired" — do not assert the succession chain operates end-to-end. The plan body for this wiring is HELD in the pile (not retired) until it lands or moves to a live successor.

---

## Settled decisions (history)

The node / household / doorway / shem topology decisions this boundary design rests on were settled earlier and distilled into a history record: [D1–D5 Node / Household / Doorway / Shem canonical decisions](../history/2026-04-19-d1-through-d5-node-and-household-canon.md).
