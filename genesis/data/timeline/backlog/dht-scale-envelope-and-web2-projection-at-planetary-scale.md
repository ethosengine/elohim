---
id: "backlog-dht-scale-envelope-and-web2-projection-at-planetary-scale"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Design session: the DHT scale envelope — sharded-arc perf constraints, hot-EPR neighborhoods, and how notarized trust projects back to web 2.0 at planetary scale (p2p YouTube/Meta/Spotify with REA value flow)"
slug: "dht-scale-envelope-and-web2-projection-at-planetary-scale"
written: "2026-07-11"
author: "operator vision note (2026-07-11) + shift notary-scenario2-green close"
status: "open"
priority: "high"
area: "architecture/scale-envelope"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "memory:project_per_node_memory_is_conductor_authority_arc"
  - "memory:project_earned_reach_governance_pr_ceremony_vision"
  - "memory:project_hub_optional_floor"
cites:
  - genesis-pair-dht-unity-plan | Genesis-Pair DHT Unity | sha256:4740875c8434d6be | path: genesis/docs/superpowers/plans/2026-07-11-genesis-pair-dht-unity-plan.md
  - peer-discovery-fractal-federation | Peer Discovery as Fractal Federation | sha256:42ae0e67f9e9d4bc | path: genesis/docs/superpowers/specs/2026-07-09-peer-discovery-fractal-federation-design.md
  - iroh-libp2p-complementarity | iroh ↔ libp2p Complementarity | sha256:29235aeb35aff128 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
tags: [architecture, scale, dht, arc-factor, sharding, hot-content, patron-cdn, reach-earning, rea, web2-projection, brainstorm-needed]
---

# The DHT scale envelope — think it through before it thinks for us

## The operator's question (2026-07-11, verbatim intent)

Is "one DHT space" going to scale to 7 billion users? Does the pattern
create a natural limit for EPRs (e.g., one EPR replicated to <200 peers
or it becomes non-performant)? And how does notarized trust scale back
to web 2.0 — enabling socially-notarized, REA-backed p2p
YouTube/Meta/Spotify replacements where value flows to contributor and
commons pools?

## What the architecture already answers (collect, verify, quantify)

1. **"One DHT space" ≠ full replication.** The alpha runs
   target_arc_factor=1 (every conductor holds the whole corpus) — that is
   the ALPHA SCAFFOLD, already identified as the RAM driver
   (per-node-GB = full-arc working set) with arc-factor<1 named as the
   scale lever. At scale, kitsune2 shards by DHT location: each entry is
   validated+held by the R nearest authority peers (R = redundancy
   factor, dozens not billions); lookups hop O(log n). The natural
   per-EPR limit is therefore the INVERSE of the operator's worry: an
   EPR's notary neighborhood stays ~R peers regardless of network size.
   NEEDS QUANTIFYING: what R, what validation latency, what
   neighborhood churn tolerance at 10^6 / 10^9 agents.

2. **The real hot-spot is popularity, not population.** A viral EPR's
   authority neighborhood (the R peers at its hash location) absorbs
   planetary read pressure unless serving is decoupled from notarizing.
   The architecture already decouples: notary plane (sharded DHT) vs
   byte plane (reach-earned distribution: quilts/pantry/RS(N,K),
   patron-CDN) vs projection plane (doorways as CDN edges — "views are
   served THROUGH a doorway, never owned BY one"). The C3/REQ-F4
   pattern (verify-locally-then-serve, landed 2026-07-10/11) is the
   microcosm of the whole answer: any edge can serve any EPR iff its
   own conductor verified the head. DESIGN QUESTION: the cache/serve
   tier for hot EPRs — who earns the right to serve (reach-earning as
   CDN admission), how does a doorway's projection cache scale
   horizontally, and what does the notary neighborhood shed to it.

3. **Infrastructure federates fractally; integrity does not fragment.**
   Tonight's membrane lesson generalizes: one DNA = one integrity
   space, but transport infrastructure (bootstrap/signal/relay) is
   domain-scoped commons (fractal-federation Tier-A) with earned
   Tier-B cross-domain federation. 7B users = many domains
   (households → neighborhoods → collectives → councils), one
   verification grammar. The anti-datacenter mechanics (reach-earning
   cost asymmetry O(M) federation vs O(N²) absorption; stewardship
   contracts capping spoke counts) are the capture guards at that
   scale. NEEDS A NUMBER-SHAPED PASS: domain sizes, cross-domain
   query amplification, council attestation fan-in.

4. **Web2 scale-back = the trust-legibility projection.** The
   notarized/published/unconfirmed trust label (the "https padlock")
   is what a browser-facing p2p YouTube shows a viewer; REA events
   (attention, stewardship, patronage) ride the same substrate and
   settle to contributor + commons pools (shefa; earned-reach
   governance: sub-commons peers fork/merge/compete, high-stakes
   artifacts converge through council-affirmed PR-ceremony). DESIGN
   QUESTION: the read-path economics — what does one video view cost
   the network at 10^9 scale, where does the REA attention event get
   notarized (per-view on-DHT is absurd; aggregation windows /
   rollup attestations are the likely shape — VSM recursion / weave
   tier-capability waves are prior art), and what fraction flows to
   commons pools without a toll-booth chokepoint emerging.

## Deliverable

A brainstorm → design session (p2p-design-gate for any new entities)
producing a canonical architecture doc: **the scale envelope** — per-EPR
notary neighborhood math, hot-content serve tier, domain federation
sizing, and the REA read-path economics — folded into the seam-map atlas
(hyperscaler-parity crosswalk) rather than forked beside it. Fold into /
sequence with the dht-unity plan's T5 (federation-level transport
commons): T5 fixes today's two-doorway membrane; this item designs the
10^9 version of the same seam.
