---
title: "History/ADR: Light Up the Topology / Graph — operational-visibility arc, landed & evolved"
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [topology, graph-native, view-federation, graphql, viewer, hub, reach-gate]
# DISTILLS the 2026-05-01 → 2026-05-29 operational-visibility arc. The bespoke
# view-federation codec + per-view HTTP routes were superseded by GraphQL Viewer
# resolvers over the graph-native substrate. Raw plan/spec bodies retire to git.
# NOTE: a2o @wip topology scenarios remain env-blocked browser-tier — NOT asserted green.
distills:
  - genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md
  - genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md
  - genesis/docs/superpowers/plans/2026-05-07-topology-substrate-completion-m1-plan.md
  - genesis/docs/plans/2026-05-20-light-up-the-topology.md
canonical:
  - ../../../superpowers/specs/2026-05-29-durability-topology-felt-resilience.md         # the live successor vision
  - ../../../superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md    # the substrate it reprojected onto
memory_anchors:
  - project_node_metrics_vs_hub_aggregation_boundary
  - project_hub_archetype_abstraction
  - project_social_reach_nervous_system
  - project_qahal_collective_view_slide45_reference
---

# History/ADR: Light Up the Topology / Graph (2026-05-01 → 2026-05-29) — operational-visibility arc, landed & evolved

> **One-sentence lesson:** Building a one-off libp2p federation codec for read-views was premature
> substrate when a general graph-projection layer was already landing — collapse the six surfaces into
> one GraphQL subgraph rather than six endpoints. And always run the pre-flight table+writer+proxy-identity
> grep before any "live-aggregate over existing tables" plan: "projection module exists" ≠ "projection
> table exists."

We set out to make the invisible P2P substrate *felt*: prove "this network actually works" from the
surface. The arc began as two sibling sprint designs on 2026-05-01 — **Light Up the Graph** (substrate
orchestration: wire the trust-compute primitives into the live runtime — lift the two `aunt_and_rage_bait`
mocks by landing the real reach-earning gate + Vouch primitive + LibP2P sink/gossip adapters) and
**Light Up the Topology** (six operational views — distribution badge, my-cluster, peer-topology,
reciprocity, doorway-dashboard — over one substrate query layer, plus two demo-critical substrate fixes:
on-connect replication kick and GET-time blob peer-fallback). The original topology design proposed a
bespoke `cluster_view`/`peer_topology_view`/`reciprocity_view` service trio fronted by a hand-rolled
`/elohim/view-federation/1.0.0` libp2p request-response codec.

**The turn.** On 2026-05-16 the graph-native projection substrate (CozoDB + async-graphql
Apollo-Federation-v2 subgraph) landed in tree, which made the bespoke federation codec and per-view HTTP
endpoints *the wrong shape*. The **2026-05-19 synthesis** re-baselined the whole effort onto GraphQL
`Viewer.{hub,peers,reciprocity}` resolvers wrapping the existing Diesel-backed services (decision: wrap,
don't reproject to Cozo yet — preserves the M1 substrate work, lands in days), retired the now-obsolete
2026-05-07 M1 plan, and added the qahal social-lens reframe (slide-45 "After the Feed"). The
**2026-05-20 plan** landed three concrete surfaces (compute triptych free/used/stewarded,
doorway-dashboard app-shell wiring, resilience tooltip + hub-abstract placement-gaps). By the
**2026-05-29 kickoff** a 3-scout file-level sweep confirmed Epic A topology is LIVE (`/shefa/cluster`,
`/shefa/peers`, `device-tile`, `ResilienceView`, progressive `●◐○` glyphs) and reframed the *remaining*
gap as connective tissue + committed-accounting readers (PeerCapacity stubs returning 0, prioritizer
dead-code) — work that then proceeded on later sprints (commit `e6300665c` lands the readers).

**What superseded what.** GraphQL Viewer resolvers superseded the bespoke view-federation codec +
per-view HTTP routes (codec retained as transitional fallback, never retired); the graph-native
substrate superseded the hand-rolled service-federation shape; the 2026-05-19 synthesis superseded the
2026-05-07 M1 plan.

**Why we turned.** Building a one-off libp2p federation codec for read-views was premature substrate
when a general graph-projection layer was already landing — collapse the six surfaces into one subgraph
rather than six endpoints.

**Watch-out for future planners.**
1. The original topology design's three pre-flight gaps were real and load-bearing — no
   AgentPeerBinding *seeder* existed, the `peer_identity_bindings` projection table dropped
   `device_archetype`/`superseded_by` columns the wire view exposed, and the `rea_projection` table the
   badge/distribution view queried *did not exist at all* (the module of that name is a signal-handler,
   not a backing store). Any "live-aggregate over existing tables" plan MUST run the pre-flight
   table-and-writer grep first — **"projection module exists" ≠ "projection table exists."**
2. Doorway did NOT forward agent identity through the registry-routed proxy
   (`storage_proxy::forward_to_storage` forwards only Content-Type/Authorization/X-Observation-Id;
   `extract_agent_key` reads an `X-Agent-Id` header nothing injects) — and `agent_pub_key` (Holochain
   pubkey, in the JWT) is a *different identifier* than the imagodei `agent_cid` on AgentPeerBinding.
   Steward `/me` federation can't resolve bindings until that proxy-inject + identifier-kind decision
   lands.
3. "Hub is the abstraction, not household" — keep the substrate hub-kind-agnostic
   (`dwelling|collective|computed` resolve in UI labels only).
4. The reach-earning gate gates author-side compose ONLY; gating the receive path is meaningless once
   an EPR has arrived (project standing as evidence, don't block).

> **Landed-by-evidence, not verified-stable:** the topology a2o `@wip` scenarios are env-blocked
> browser-tier and are flagged HELD, not green. The surfaces are confirmed present by file-level sweep;
> in-cluster green is not asserted here.

## Bidirectional links

- **This record → canonical:** [durability/topology felt-resilience](../../../superpowers/specs/2026-05-29-durability-topology-felt-resilience.md) (the live successor vision) + [graph-native substrate](../../../superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md) (the substrate it reprojected onto).
- **Distilled-from (raw bodies in git history):** light-up-the-graph/topology designs, the M1 plan, the 2026-05-20 plan (linked in frontmatter).
