---
title: Doorway and Elohim-Hub — Edge Responsibilities and the Reach-Earning Throughline
status: Draft (exploratory backlog spec)
created: 2026-05-08
related:
  - genesis/docs/superpowers/specs/2026-05-02-elohim-hub-boundaries-design.md
  - genesis/docs/superpowers/specs/2026-05-01-atproto-lexicon-projection-doorway-design.md
  - genesis/docs/superpowers/plans/2026-04-28-doorway-blob-registry-routing.md
  - genesis/docs/superpowers/specs/2026-04-23-epr-phase-2c-libp2p-federation-design.md
  - genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md
  - doorway/doorway-service/ARCHITECTURE.md
  - doorway/doorway-service/FEDERATION.md
  - doorway/doorway-service/SCALING.md
  - doorway/doorway-service/REACH.md
memory_anchors:
  - project_reach_earned_at_authoring
  - project_social_reach_nervous_system
  - project_trust_as_efficiency_signal
  - project_substrate_scale_ceiling
  - project_dht_vs_libp2p_scoping
  - project_three_layer_truth_model
  - project_doorway_single_target_no_fanout
  - project_doorway_peer_registration
  - project_doorway_manifest_driven_routes
  - project_peer_native_account_canonical_surface
  - project_p2p_is_hosting
  - project_hub_archetype_abstraction
  - project_household_fabric
  - project_elohim_hub_elevation
  - project_substrate_floor_elohim_ceiling
  - project_signal_kind_extensible_protocol_class
  - project_social_compute_collective_is_stewardship_unit
  - project_intelligence_revolution_scales_to_humans
  - project_redeploy_the_substrate
  - project_household_is_resilience_unit
  - project_multi_doorway_human_registration
  - project_inventory_exchange_not_byte_replication
  - project_reach_gate_is_elohim_mediated_matchmaking
---

## Why this spec exists

Two things converged. The alpha cluster's SSR delivery wiring (recent commit `feat(ssr-delivery): wire alpha cluster end-to-end`) forced ingress to be first-class instead of a future hand-wave. At the same time a Gemini conversation about how Cloudflare and Kubernetes actually handle DDoS, anycast, IPv6 reachability, eBPF/XDP, and PoW challenges surfaced the question: when the protocol stops leaning on hyperscalers, *where* does the aggregate-scale work land?

The answer turned out to be larger than doorway. Doorway is the per-deployment web2 projection surface, and it should stay simple — self-hosted, human-operable, the kind of thing a household steward can run on a single blade. The aggregate-scale concerns — cross-deployment threat coordination, mobile inference processing, workload state migration, social-resilient compute contracts — belong at the **hub** layer: the home-node cluster that stewards a family or collective's compute, federates horizontally with peer hubs, and coordinates discernment via elohim-operators.

This spec maps the boundary between the two layers and surfaces the throughline that makes the boundary coherent: the protocol's existing **reach-earning** principle, already load-bearing at the per-message authoring layer (memory `project_reach_earned_at_authoring`, `project_social_reach_nervous_system`), extends at the hub layer to compute, distribution, defense, and AI-coordination at aggregate scale. A pattern shaped like a DDoS attack is structurally just unearned-reach compute or distribution. The hub fabric simply doesn't engage with it — defense is a side-effect of earning, not a bolt-on firewall.

The spec is exploratory. It locks in only what existing memory has already settled, names the open questions without picking answers, and seeds stub-epics for future sprints. The intent is for the next several sprints' design decisions to land in the right layer the first time.

## The thesis

The protocol already earns reach at message authoring. Authors earn the right to broadcast through provenance, standing, contracts, and trust gradients; receivers pre-authorize through their own values-forward filters; intermediate nodes either propagate or quarantine based on the same earning signals. This is the social-reach nervous system — a sense-and-respond loop at every node.

The hub layer extends the same pattern to four aggregate-scale surfaces:

1. **Compute reach** — does this inference request, observer stream, workload migration, or sponsored compute earn hub cycles? Mobile devices delegate inference to their dwelling hub; the hub spends its scarce GPU/CPU budget on requests that have earned standing through the family's relationships and contracts. Cross-hub sponsored compute (a neighboring hub donating cycles to an overwhelmed peer) is itself a continuously-negotiated REA contract.

2. **Distribution reach** — does this traffic propagate across federated hubs? Cross-hub gossip, load balancing, content fanout, and federation projection all consume the same earning signal. Anomalous distribution patterns — message floods, content amplification by unestablished sources — are exactly *unearned distribution reach*. They die at the first unconvinced hub. There is no central anti-spam classifier; there is the cumulative judgment of every hub in the federation.

3. **Defense reach** — does this hub get sponsored compute when it's overwhelmed? When a hub falls under attack, who answers the call? Defense is earned via the meet-and-protect contracts a hub continuously negotiates with peer hubs — through trust gradients, federation standing, and the resilience commitments that household-to-household reciprocation already encodes (memory `project_household_is_resilience_unit`).

4. **AI-coordination reach** (the discernment surface) — do elohim-operators read this signal, sign this action, escalate this pattern? Elohim-operators sit on the substrate-floor / elohim-ceiling pattern (memory `project_substrate_floor_elohim_ceiling`): the substrate's deterministic earning rules form the floor; elohim-operator discernment forms the ceiling. The ceiling extends what the floor permits — never gates below it, never overrides. Elohim-operators earn the right to act on a hub's behalf the same way humans do — through standing, witness, and accountability.

A DDoS-shaped pattern is unearned-reach compute and distribution. The hub fabric doesn't propagate it because no node along the way has reason to spend its budget on it. The "thousands of small targets" defense Cloudflare achieves through anycast is achieved here through federation: there is no single IP to overwhelm, because the network *is* the federation, and each hub's budget is a function of what it has earned and what it has been sponsored. Hyperscalers absorb aggregate-scale concerns via centralization. The protocol absorbs them via federated earning.

The scaling target this enables is **FANG-subsumption**: aggregate compute (Google AI), aggregate distribution (YouTube/Netflix CDN), aggregate defense (Cloudflare), and aggregate algorithmic discernment (Facebook's feed). The DHT can't carry this — Holochain DNAs strain past ~100 active participants per shard, and DHT operations are deliberately expensive to enforce notarization (memory `project_dht_vs_libp2p_scoping`). Hub federation is the only dimension where this scaling works, and it works by earning, not by enlarging any single hub past what the humans inside can govern.

## The duo: doorway and elohim-hub

```
┌──────────────────────────────────────────────────────────────────────┐
│  DOORWAY                              ELOHIM-HUB                     │
│  per-deployment                       home-node cluster              │
│  ────────────────                     ────────────────               │
│                                                                      │
│  • Self-hosted, simple                • Multi-blade cluster          │
│  • Web2 projection surface            • Stewards family/collective   │
│  • TLS / HTTP-3 / QUIC                  compute and presence         │
│  • OAuth-RP (presents identity)       • Federates horizontally       │
│  • Manifest-driven routes              with peer hubs                │
│  • Reach gating per request           • Reach-earning at the four    │
│  • SSR delivery                         aggregate surfaces           │
│  • Federation projection              • Elohim-operators as the      │
│    (ATProto, future ActivityPub)        AI fabric-managers           │
│  • Single-target dispatch             • Two archetypes:              │
│    (no fanout, ever)                    DwellingHub (primary,        │
│                                         physical-limit-grounded),    │
│  Some hubs have a doorway.              CollectiveHub (technical     │
│  Some don't.                            tier, different attitude)    │
│                                                                      │
│  Complexity stays simple              Complexity goes here           │
│  by design.                           by design.                     │
└──────────────────────────────────────────────────────────────────────┘
```

The duo replaces the previous "doorway and an unnamed third layer" framing. There is no global protector layer above hubs. Hubs federate horizontally with peer hubs; the network-shaped concerns Cloudflare-class services centralize live *inside* each hub, applied via the four reach-earning surfaces, coordinated by elohim-operators that themselves earn the right to act.

This duo is the natural extension of memory `project_three_layer_truth_model` (DHT / libp2p / doorway). Hub composes the libp2p-and-storage layer into a runtime primitive. Doorway projects that runtime out for web2 consumption. The DHT remains the notarization spine across the whole substrate.

### Doorway responsibilities (kept simple)

Doorway's surface is intentionally bounded. The list is short, well-trodden, and resistant to drift:

- **TLS termination** — cert-manager, ACME, wildcard certs as operator preference
- **HTTP/3 and QUIC** — the natural transport pairing with libp2p's QUIC underneath storage
- **Manifest-driven request routing** — `RouteRegistry` matches incoming URLs against the federation manifest; routes register themselves rather than being hand-coded (memory `project_doorway_manifest_driven_routes`, plan `2026-04-28-doorway-blob-registry-routing.md`)
- **OAuth-RP identity presentation** — doorway presents identity that lives elsewhere, never owns it (memory `project_peer_native_account_canonical_surface`)
- **Reach gating per request** — the existing REACH.md ladder: commons / regional / bioregional / municipal / neighborhood / local / invited / private. Per-request, deterministic.
- **SSR delivery** — projecting peer-native content into HTML for web2 browsers; the recent alpha-cluster wiring made this concrete
- **Federation projection** — translating peer-native EPRs into lexicon-flavored federation surfaces (ATProto today, possibly ActivityPub later) and ingesting their inverse (memory `project_doorway_is_federation_surface_atproto`)
- **Single-target dispatch** — substrate moves bytes peer-to-peer; doorway projects and caches; doorway never iterates peers for blob delivery (memory `project_doorway_single_target_no_fanout`)
- **Inside-out registration** — peers register interest in content with the doorway; the doorway makes content available; peers subscribe by manifest (memory `project_doorway_peer_registration`)

Doorway runs on a single blade comfortably. A household steward can deploy and operate one without thinking about kubernetes, BGP, or eBPF. The protocol's edge defenses must work without hyperscaler fronting — but hyperscaler fronting is allowed as an operator opt-in for high-traffic public deployments (see § Hyperscaler-fronting as operator opt-in).

What stays explicitly out of doorway:

- No libp2p swarm (storage runs the swarm; doorway speaks HTTP to its co-located storage)
- No DHT operations (storage and the conductor handle these)
- No identity ownership (OAuth-RP only)
- No blob fanout to peers (single-target dispatch)
- No P2P-shaped responsibilities that could equally live in storage or hub

### Elohim-hub responsibilities (where complexity lives)

The hub layer absorbs aggregate-scale concerns. Its responsibilities organize around the four reach-earning surfaces:

#### a. Compute reach

- **Mobile device inference proxy** — family devices delegate inference to their dwelling hub. The hub serves as edge-AI for family-shaped devices, applying reach-earning to decide which requests get cycles and which are deferred or denied.
- **Elohim_observer stream processing** — observer streams (the manifesto's `autonomous_entity/epic.md` describes converted surveillance cameras as observer-network inputs) get processed at hub scope. The hub is the natural point where observer streams meet REA pattern recognition.
- **Workload state migration (PVCs and equivalents)** — moving persistent workload state across cluster blades. When a blade goes down or a hub rebalances, the hub orchestrates the migration. This is the kubernetes-class concern that the hub absorbs internally so deployment operators don't have to think about it.
- **Node rebalancing** — cluster operations across the dwelling's blades. Hot/cold storage tiers, leader election, replica placement. Internal to the hub.
- **Sponsored compute via continuously-negotiated contracts** — when a peer hub is overwhelmed, neighboring hubs can sponsor compute under standing meet-and-protect contracts. Contracts are REA commitments; sponsorship is an economic event; the relationship is continuously negotiated.

Earning shape: who owns the device, what standing the family has, what contracts the hub has signed with peers, what the elohim-operator's discernment says about the request's pattern.

Unearned shape: anonymous high-volume inference requests; observer streams from devices the hub doesn't know; workload migrations that don't match the cluster's stewardship contract.

DDoS is exactly the unearned shape applied to compute. The hub doesn't engage with it because nothing in the reach-earning evaluation says it should.

#### b. Distribution reach

- **Cross-hub federation gossip** — metadata-only inventory exchange (memory `project_inventory_exchange_not_byte_replication`); gossip lists are signals, not byte streams.
- **Load balancing across federated hubs** — when a hub is overwhelmed but the request is legitimately earned, route to a sponsoring peer hub. Federation IS the load balancer, and the routing decisions are reach-earning judgments, not round-robin.
- **Traffic-shape recognition** — the hub observes its own traffic and emits structural patterns (`signal_kind: AbusePattern` candidate; memory `project_signal_kind_extensible_protocol_class`). Other hubs subscribe to neighboring patterns; elohim-operators synthesize.
- **AbusePattern emission** — the hub publishes evidence about unearned-reach attempts: prefix patterns, timing fingerprints, request-shape anomalies. Peer hubs decide whether to act on the signals based on the publishing hub's standing.
- **Federation manifest enforcement** — the federation manifest declares which doorways federate with which hubs and on what terms. Distribution reach respects the manifest.

Earning shape: provenance + standing + receiver pre-authorization at every hop.

Unearned shape: amplification without provenance, fanout without standing, mass distribution without earned authoring reach.

The "thousands of small targets" defense emerges naturally. There is no shared anycast IP, so an attacker has to convince each hub independently to propagate. Each hub's reach-earning evaluation is a small but correlated cost. Mass distribution attacks become quadratic against the number of hubs they must convince — which is the structural advantage federation has over centralization.

#### c. Defense reach

- **Meet-and-protect contracts** — REA-style commitments between peer hubs about defensive sponsorship. Continuously negotiated; relationship-shaped; standing-aware.
- **Prefix-rate-limit policy** — per-`/48` and per-`/64` rate limits in IPv6 (per-IP rate limits are obsolete in v6 because address space is too large). Policy is hub-level; enforcement may live at doorway, CNI, or kernel; coordination across federated hubs is hub-level.
- **Cross-hub threat coordination** — when one hub sees an attack pattern, peer hubs subscribe to the signal and pre-tighten their own defenses. Coordination is gossip-shaped and elohim-operator-mediated.
- **Defensive compute sponsorship** — when a hub is genuinely overwhelmed, neighboring hubs spend cycles on its behalf. The PoW challenge gateway and SSR cache replication can be sponsored across hubs.
- **Recovery hub coordination** — when defense fails and a hub goes dark, the multi-doorway human-registration pattern (memory `project_multi_doorway_human_registration`) means humans don't lose access. Recovery routing is hub-level.

Earning shape: standing of the hub being defended, history of reciprocation, contractual commitments, neighboring trust gradients.

Unearned shape: requests for defense from hubs without contracts, without standing, without evidence of legitimate operation.

This is where the household-as-resilience-unit memory becomes load-bearing. Resilience is hub-to-hub, not peer-to-peer. The reciprocation count between dwelling hubs is the measure of defense reach earned.

#### d. AI-coordination reach

- **Elohim-operators as the household-fabric-manager role** — memory `project_household_fabric` already establishes this; the hub is where elohim-operators do their actual work.
- **Hub-scope discernment** — substrate-floor / elohim-ceiling pattern (memory `project_substrate_floor_elohim_ceiling`). The substrate determines deterministic outcomes (allowed / blocked / pending); the elohim-operator adds discernment on top — escalating, sponsoring, recommending, witnessing.
- **Continuous contract negotiation** — meet-and-protect, compute-sponsorship, federation-standing contracts evolve continuously as the relationships between hubs change. Elohim-operators do the negotiation work that humans would otherwise have to manage themselves.
- **Observer stream synthesis** — observer streams + REA pattern recognition + cross-hub correlation produces network-level intelligence about what's happening. Elohim-operators read; humans witness; hubs respond.
- **Earning judgment** — when a request, contract, or pattern is borderline, the elohim-operator's discernment decides. Discernment is signed, witnessable, and accountable; it is NOT opaque.

Earning shape: the elohim-operator itself earned its role through the same standing/witness/accountability machinery as any other actor in the system. Memory `project_elohim_subagent_specialists` and `project_elohim_as_counsel` apply.

Unearned shape: discernment claims without provenance, signatures without standing, escalations without witnessability.

The four surfaces are coupled. A request that earns compute reach but not distribution reach is processed locally without propagation. A pattern that earns AI-coordination attention but not defense reach is logged but not sponsored. The coupling is the protocol's expressive vocabulary; elohim-operators do the work of evaluating couplings the same way the social-reach nervous system already does for messages.

## Hub archetypes: Dwelling and Collective

The hub is a runtime composition primitive (memory `project_elohim_hub_elevation`, spec `2026-05-02-elohim-hub-boundaries-design.md`). It has two archetypes that share the same trait surface but differ in **design attitude** about how humans and elohim-operators co-inhabit the fabric.

### DwellingHub (primary archetype)

**Naming intent.** "Dwelling" grounds the concept at the natural physical limit at which it was designed: the place where humans live. A dwelling has walls, a roof, an inside and an outside, a known set of inhabitants who steward it together. The DwellingHub mirrors that — a home-node cluster that stewards a family's compute and presence, sized to what humans inside the dwelling can govern together.

The previously-used `HouseholdHub` term (in `2026-05-02-elohim-hub-boundaries-design.md` and memory `project_hub_archetype_abstraction`) is a synonym. Going forward, **DwellingHub** is the preferred term in this spec and onward; existing code and docs may transition incrementally.

**Design attitude — co-presence.** Humans and their elohim-operators are co-present in the dwelling's fabric. The fabric is visible: family members can see what the hub is doing, can intervene, can steward. The elohim-operator is a fabric-helper alongside the humans, not a fabric-owner. When the operator makes a discernment call, it is signed, witnessable, and reversible by the human stewards.

The dwelling expects the humans inside to be aware of the fabric at some level. This is intentional. Memory `project_intelligence_revolution_scales_to_humans` insists that intelligence scales TO human complexity; the dwelling archetype is where this is most directly true.

**Scale.** Sized to one family / one dwelling. Multi-blade cluster (memory `project_household_horizontal_scaling`); horizontal scaling means more dwellings, not bigger dwellings.

**Doorway optionality.** A DwellingHub may or may not host a doorway. A dwelling that publishes content to the public web hosts a doorway; a dwelling that participates only privately does not. Memory `project_multi_doorway_human_registration` means humans inside the dwelling may register with multiple doorways — the dwelling's own (if any) and others — for resilience. The doorway is one role a dwelling can take, not a mandatory layer.

### CollectiveHub (secondary archetype)

**Naming intent.** A collective is one of many shapes (memory `project_social_compute_collective_is_stewardship_unit`): a church, a co-op, a patron circle, a DAO, a mutual aid network. Each has its own stewardship contract; the CollectiveHub is the runtime composition primitive that serves them all.

**Design attitude — delegated stewardship.** The collective expects that elohim-operators carry more of the day-to-day fabric work autonomously, with humans designating stewardship roles rather than every member operating fabric directly. This is a different attitude than the dwelling's co-presence — not opposed, just suited to scale and shape. A church with five hundred members cannot have every member auditing the fabric; the collective designates stewards, and the elohim-operators do most of the operational discernment within the contracts the stewards have approved.

This is the technical tier where the human-versus-elohim separation is more visible. Not as a hard taxonomic rule — both archetypes have humans and elohim-operators, both are accountable, both are bound by the substrate-floor / elohim-ceiling pattern. But the *attitude* of who is most active in operation differs: dwelling = co-presence; collective = delegated stewardship.

The attitude framing matters because it tells future agents (and future-Gemini-style brainstorms) what the design serves. The dwelling serves humans most directly inhabiting their fabric. The collective serves human stewardship of larger collective contracts via more-autonomous elohim-operators.

**Scale.** Sized to a collective's stewardship contract. May span multiple physical sites; may have many more blades and more federation surface than a dwelling. Doorways are typical (collectives usually have a public face) but not mandatory.

### Federation between archetypes

Both archetypes federate horizontally. A DwellingHub may federate with neighboring dwellings, with a CollectiveHub it belongs to (the family's church, the co-op the family contributes to), with strangers' hubs through standing-mediated discovery. A CollectiveHub may federate with member dwellings, with peer collectives, with public dwellings that have earned standing. Federation contracts are continuously negotiated; the federation manifest declares the surface.

The reach-earning surfaces are the same across archetypes. Compute, distribution, defense, AI-coordination — the same earning logic, the same discernment pattern. The archetypes differ in attitude, not in protocol surface.

## The doorway/hub split: placing each Gemini topic

The Gemini conversation surfaced a catalog of edge concerns. Each one places at doorway, hub, or operator-deployment, sometimes spanning two or three.

| Gemini topic | Where it lands | Notes |
|---|---|---|
| Anycast / BGP | Operator deployment (BGP-feasible infra only) + doorway (presents shared IP) | Most operators won't have BGP; default to GSLB. |
| GSLB via DNS | Hub (federation policy) + doorway (resolution) | Hub coordinates which doorways are healthy; doorway resolves. |
| MetalLB + BIRD (k8s anycast bridge) | Operator deployment | Lives in CNI/cluster setup, not in protocol code. |
| TLS termination | Doorway | cert-manager, ACME, wildcard. |
| HTTP/3 / QUIC | Doorway | Natural pairing with libp2p QUIC underneath storage. |
| SNI multi-tenancy | Doorway | One doorway can front multiple federation namespaces. |
| Wasm projection filters (Envoy-style) | Doorway, manifest-declared | Federation manifest declares projection logic per route. |
| eBPF/XDP / Cilium kernel-layer drop | Operator deployment (CNI) | Hub receives synthesized traffic-shape signals from doorway and CNI; doorway code does not implement eBPF. |
| Proof-of-Work challenge gateway | Doorway emits + Hub aggregates patterns | Probably a `signal_kind` extension; deferred. |
| IPv6 GUA per component | Operator deployment + protocol awareness | Doorway, storage, conductor all benefit from stable v6 GUAs. |
| NDP cache exhaustion hardening | Operator deployment (kernel sysctl) | Not protocol code. |
| Prefix-aware rate limiting (`/48`, `/64`) | Doorway implements + Hub coordinates | Policy distributed across federation; enforcement at doorway. |
| IPv6 Flow Label use | Doorway (advisory; future optimization) | Not a near-term concern. |
| SSR delivery | Doorway | The precipitating moment; recent alpha wiring made this real. |
| SSR projection attestation | Open question | Does rendered HTML carry a `ProjectionClaim` equivalent? Sibling to ATProto projection attestation. |
| Multi-doorway failover | Hub | The hub knows which doorways a given human registered with and which are healthy. |
| Mobile inference requests | Hub (DwellingHub primarily) | New responsibility class; family edge-AI. |
| Elohim_observer stream processing | Hub | REA pattern recognition meets observer streams here. |
| PVC movement / cluster rebalancing | Hub (internal) | Kubernetes-class fabric concern, absorbed by hub. |
| Continuously-negotiated compute contracts | Hub | Meet-and-protect, sponsorship, federation standing. |
| Cross-hub threat coordination | Hub | The Cloudflare-class concern at hub aggregate. |
| AbusePattern signal emission | Doorway emits, Hub aggregates, Federation propagates | `signal_kind` extension; cross-hub vocabulary. |

Items that explicitly do NOT land in doorway code, despite Gemini's framing:

- **"Doorway runs a P2P node (libp2p or Iroh)"** — no. Memory `project_three_layer_truth_model` is firm: doorway is web2 projection; it is not a libp2p participant. The libp2p swarm lives in `elohim-storage`, co-located with doorway but distinct. Doorway speaks HTTP to its storage.
- **"Doorway hosts the cache layer for P2P fetches"** — only as a transparent HTTP cache in front of its co-located storage. The cache is a doorway-internal optimization; it is not a P2P fan-out, not a peer iteration, not a swarm participant.
- **"Doorway absorbs the federation routing logic"** — no. Federation is hub-level. Doorway emits and ingests federation projection; the routing decision (which peer hub, what standing, what contract) is hub-level.

## Distributed defense via hub federation

The Cloudflare playbook is "thousands of small targets, one shared fabric, kernel-level packet drop." The protocol's analog is "thousands of federated dwellings, no shared fabric, reach-earning at every hop, elohim-operator discernment in the loop."

- **"Thousands of small targets"** ↔ **federated hubs.** No single anycast IP to target; an attacker must enumerate a moving topology. Each hub's federation surface is small but coordinated.
- **"One shared fabric"** ↔ **continuously-negotiated meet-and-protect contracts.** Defense is federation, not a shared 248Tbps pipe. When one hub is overwhelmed, contracts say which neighbors answer.
- **"Kernel-level packet drop"** ↔ **reach-earning at the protocol layer.** Unearned-reach traffic doesn't survive the first hop because no node along the way will spend cycles on it. Defense costs are amortized across federation.
- **"Cloudflare's Gatebot autonomous classifier"** ↔ **elohim-operator discernment with witnessable trajectories.** Same job, but observable, signed, and reversible.

The compensating advantage federation has over centralization:

- **No single point of capture.** A federated network cannot be acquired, throttled, or compelled in the way a hyperscaler can.
- **Socially-resilient peer compute.** When neighbors sponsor compute, the resilience is a function of human-scale relationships, which means the network gets more resilient as humans get more connected — not less.
- **Cost asymmetry against attackers.** Trust-as-efficiency-signal (memory `project_trust_as_efficiency_signal`): trustworthy traffic costs less to distribute. Untrustworthy traffic costs more at every hop. Attackers face quadratic-in-hops costs; legitimate traffic faces near-zero marginal cost.

The compensating disadvantages, honestly named:

- **No shared 248Tbps pipe.** Each hub has limited absorption capacity. Mitigated by sponsorship contracts but not eliminated.
- **Cross-hub coordination latency.** Hub-to-hub federation gossip is slower than intra-Cloudflare fabric coordination. Mitigated by elohim-operator pre-tightening based on neighboring signals.
- **Bootstrap problem.** A new hub with no standing has limited defense reach. Mitigated by graduated-recovery-authority pattern (memory `project_graduated_recovery_authority`) — community always extends initial standing even before crypto hardening solidifies it.

## Bad-actor topology — hub-emergent, not central

If elohim-operators are going to coordinate defense across hubs, they need a substrate-level concept of who is attacking. Memory `project_signal_kind_extensible_protocol_class` says new feedback vocabulary goes through schema + validator + manifest, not new entry types. The cleanest path:

- **`signal_kind: AbusePattern`** (working name) — schema-defined, doorway-emitted, hub-aggregated, federation-propagated.
- **Subscription is manifest-declared.** Hubs subscribe to neighboring hubs' AbusePattern streams via federation manifest entries.
- **Synthesis is elohim-operator work.** Patterns are evidence; conclusions are discernment. Discernment is signed, witnessable, reversible.
- **Response is contract-mediated.** When a pattern emerges that warrants action, the contract between hubs determines what action (rate-limit tightening, compute sponsorship, route shifting).

This is not a feature this spec implements. It is an open question planted as a seed for a future epic. The shape is sketched here so that when the epic arrives, the work lands in the right layer (signal_kind + manifest + elohim-operator discernment), not as a parallel attack-DNA or a centralized abuse classifier.

## Boundaries — what's settled

These are commitments anchored in existing memory. Future specs can build on them; future brainstorms should not relitigate them without naming why memory should change.

| Commitment | Memory anchor |
|---|---|
| Doorway never swarms libp2p; storage does | `project_three_layer_truth_model` |
| Doorway never fans out blob delivery to peers | `project_doorway_single_target_no_fanout` |
| Doorway presents identity, never owns it | `project_peer_native_account_canonical_surface` |
| The P2P mesh is the hosting layer; doorway is optional projection | `project_p2p_is_hosting` |
| Hub is peer-layer (libp2p-shaped); doorway is web2-shaped | `project_three_layer_truth_model` (extended here) |
| Elohim-operators are hub-scope, not doorway-scope | `project_household_fabric` |
| Hubs federate horizontally; substrate scales by federation, not hyperscale | `project_substrate_scale_ceiling`, `project_elohim_hub_elevation` |
| Reach is earned at every node, applied at every hop | `project_reach_earned_at_authoring`, `project_social_reach_nervous_system` |
| Substrate-floor / elohim-ceiling: substrate determines deterministic; elohim adds discernment | `project_substrate_floor_elohim_ceiling` |
| Inventory exchange is metadata-only; byte movement is single-target | `project_inventory_exchange_not_byte_replication` |
| Routes register themselves via manifest; doorway is a registry-driven proxy | `project_doorway_manifest_driven_routes` |

## Boundaries — what's in motion

These are open questions the spec deliberately does not settle. Each is a candidate for its own future brainstorm or epic.

- **Exact split of network-protection responsibilities between doorway, hub, and operator deployment.** Some concerns clearly belong to operator (CNI choice, kernel sysctls); some clearly belong to doorway (TLS, HTTP/3); some clearly belong to hub (cross-hub coordination, sponsorship contracts). The middle band — prefix rate-limit policy, PoW challenge issuance, traffic-shape recognition — is in motion. The spec places initial guesses; the answers solidify with implementation.

- **SSR projection attestation.** Does the SSR'd HTML carry a `ProjectionClaim` analogous to the ATProto outbound projection attestation? Rendering is computation; computation invites attestation. The shape is similar to the existing ATProto projection but the consumer surface (web browsers) doesn't have a natural verifier. Possibly the attestation lives in HTTP headers, possibly as a doorway-signed sidecar, possibly as a manifest-declared computation_attestation EPR. Sibling to spec `2026-05-01-computation-attestation-graduated-rigor-design.md`.

- **Federation-level doorway-to-doorway communication.** Today doorways don't talk to each other directly. Tomorrow, when federation projection grows (ATProto Firehose, ActivityPub, future flavors), doorways may need to coordinate. The constitutional question: does this violate "doorway never swarms"? Probably not — federation flavors are HTTP/web2-shaped, not libp2p-shaped. But the rule needs explicit treatment when the case arises.

- **First concrete elohim-operator manifest scope for the network role.** Elohim-operators exist conceptually (memory `project_elohim_subagent_specialists`); the household-fabric-manager role is named (memory `project_household_fabric`). What does the manifest actually declare for the network-protection scope — which signals it reads, which actions it can sign, what humans witness, what failure modes the substrate floor catches? An epic of its own.

- **Hub-with-doorway vs. hub-without-doorway architectural split.** Some dwellings host doorways; some don't. The boundary is clear in principle but the operational mechanics (how a no-doorway dwelling federates with the rest of the network, how it consumes federation projections, how its presence is announced) want their own treatment.

- **Continuously-negotiated compute contract schema.** Meet-and-protect, compute sponsorship, federation standing — all are REA-flavored contracts. The schema will need its own brainstorm. Coordinate with EPR Phase 3+ work.

- **DwellingHub vs CollectiveHub trait surface.** Today the `Hub` trait sketched in `2026-05-02-elohim-hub-boundaries-design.md` doesn't yet differentiate by archetype. The "attitude" framing in this spec suggests the differentiation will be more in *defaults and discernment shape* than in trait surface. Worth confirming when the trait moves from sketch to code.

- **Bad-actor DHT topology.** Sketched here as a `signal_kind: AbusePattern` candidate. Worth its own epic when the protocol has enough hub federation to make the signals load-bearing.

- **Mobile-device inference protocol shape.** The hub-as-edge-AI for mobile devices is a new responsibility class. The protocol shape (how devices request inference, how requests carry standing, how results stream back, how the family steward consents to which inferences happen) is undesigned. Likely sibling to existing storage-client protocols but with its own manifest scope.

## Hyperscaler-fronting as operator opt-in

Hyperscaler-fronting (Cloudflare, AWS Shield, GCP Armor) is **allowed** as an operator opt-in for doorways that face heavy public web traffic. A civic-tech operator running a federation surface for ten thousand contributors may legitimately choose to front its doorway with Cloudflare for the practical economics of v6-prefix rate-limiting and L7 DDoS absorption.

But hyperscaler-fronting is **not the protocol's answer to DDoS.** The protocol's answer is hub federation + reach-earning + elohim-operator coordination + socially-resilient compute contracts. The protocol's defenses must work without a hyperscaler in front of any doorway. A household-scale dwelling on a residential connection cannot afford or operate a hyperscaler partnership; the protocol must remain credible at that scale (memory `project_subsume_g_f_a_via_it_just_works`).

This means: every responsibility this spec places at hub or doorway must be implementable at household scale, on commodity hardware, by a non-expert steward. Hyperscaler-fronting is an operator's choice to add absorption capacity above and beyond what the protocol guarantees — it is never a protocol commitment.

Concretely:

- The federation manifest must not require hyperscaler-fronting to function.
- The cross-hub threat coordination signals must work over commodity internet connections.
- The continuously-negotiated compute contracts must be settleable without hyperscaler infrastructure.
- The elohim-operator discernment must run on commodity GPU/CPU.

If any of these break under household-scale operation, the protocol has slipped its constitutional commitment. Hyperscaler-fronting closes the door on "we'll just put Cloudflare in front of it" as the answer to a protocol-level question.

## Stub-epic seeds

These are candidate future-sprint epics. Each seed names a scope sketch, dependencies, and the open question that would unblock a brainstorm.

### 1. Hub network-protection MVP

**Scope.** First implementation of cross-hub threat coordination. Two dwellings federate; one observes a traffic pattern; the second pre-tightens its policy. End-to-end working signal flow.

**Dependencies.** `signal_kind: AbusePattern` schema (a sub-epic). Federation manifest schema extension. Elohim-operator discernment scaffold.

**Unblocking question.** What's the minimum viable abuse pattern vocabulary for v1? (Probably: prefix-rate spike, request-shape anomaly, path-pattern flood. Defer the rich vocabulary.)

### 2. Mobile inference proxy at hub

**Scope.** Family device delegates inference request to dwelling hub. Hub applies reach-earning, processes or defers, returns result. End-to-end on alpha cluster with one dwelling and one family device.

**Dependencies.** Hub trait surface implementation (currently sketched in `2026-05-02-elohim-hub-boundaries-design.md`). Elohim-operator scope for inference-mediation. Mobile client SDK.

**Unblocking question.** What's the minimum-viable consent surface for the human steward — i.e., how does the family decide which inferences happen automatically vs. require approval?

### 3. PVC movement / cluster rebalancing in elohim-node

**Scope.** Hot/cold blade migration, leader election, replica placement within a single dwelling cluster. Internal to elohim-node; observable from the dwelling's operator surface.

**Dependencies.** elohim-node graduates from "deployment wrapper" to "hub instance" (per `2026-05-02-elohim-hub-boundaries-design.md`). Cluster discovery via mDNS or libp2p mDNS analog.

**Unblocking question.** What's the failure-mode story for migration mid-flight? (Substrate floor catches; what's the elohim-operator's response shape?)

### 4. SSR projection attestation

**Scope.** Doorway-rendered HTML carries a verifiable claim that the rendering corresponds to the source EPR. Browser-consumable verification path.

**Dependencies.** Existing ATProto projection attestation (sibling pattern). Computation attestation graduated-rigor framework.

**Unblocking question.** Where does the attestation live in the HTTP response — header, sidecar resource, embedded `<meta>` tag, or something else? Optimize for browser verifiability without requiring browser plugins.

### 5. signal_kind: AbusePattern

**Scope.** Schema, validator, doorway emission path, hub aggregation, federation propagation.

**Dependencies.** Existing `signal_kind` extensible class. Federation manifest schema.

**Unblocking question.** What's the minimum vocabulary that survives schema evolution? (Probably extensible enum with `pattern_kind` discriminator and `evidence` payload; defer concrete kinds to first driver.)

### 6. Elohim-operator manifest for network role

**Scope.** First concrete manifest scope for the household-fabric-manager elohim role — which signals it reads, which actions it signs, what humans witness, what the substrate floor catches.

**Dependencies.** Elohim-agent sense-and-respond architecture (memory `project_elohim_agent_sense_respond_architecture`). Manifest framework existing for elohim-agent.

**Unblocking question.** What's the witness-and-reverse story for elohim-operator actions? (Per memory `project_substrate_floor_elohim_ceiling`: substrate floor catches deterministic gates; elohim adds discernment. Need concrete UX for human witness + reversal.)

### 7. Continuously-negotiated compute contract schema

**Scope.** REA-shaped commitments between hubs for meet-and-protect, compute sponsorship, federation standing.

**Dependencies.** EPR Phase 3 manifest resolver. REA commitment substrate.

**Unblocking question.** How do contracts evolve continuously without a central renegotiation point? (Probably: contracts are EPR variants; renegotiation is a new variant referencing the prior; standing is the path through variants.)

### 8. Hub-with-doorway vs hub-without-doorway architectural split

**Scope.** Concrete operational story for both shapes. How a no-doorway dwelling federates, announces presence, consumes federation projections.

**Dependencies.** Federation manifest schema. Hub trait surface.

**Unblocking question.** What's the multi-doorway-registration story (memory `project_multi_doorway_human_registration`) when the human's home dwelling has no doorway? Probably: humans register with peer dwellings' or collectives' doorways; their home dwelling federates with those doorways' hubs.

### 9. DwellingHub primary archetype kickoff

**Scope.** First concrete DwellingHub implementation pulling together storage + agent + bitswap + cluster + operator surface in one process. May be the same epic as PVC movement above; named separately because it could be staged.

**Dependencies.** Most of the above.

**Unblocking question.** Is the DwellingHub a new crate (`elohim-hub`) or a trait module inside `elohim-node` (per the boundaries spec's default)? Decide when a second consumer (operator UI, fixtures crate) needs the trait independently.

### 10. CollectiveHub differentiation

**Scope.** What CollectiveHub does that DwellingHub doesn't — concretely. The "attitude" difference made testable.

**Dependencies.** DwellingHub primary kickoff.

**Unblocking question.** Is the differentiation in defaults (config), discernment shape (operator manifests), trait methods (different surface), or operational policy? Probably config + manifests, not trait surface.

## Breadcrumbs

### Memory anchors that load this spec

- `project_reach_earned_at_authoring` — reach is earned at every node; coupled responsibilities at every edge.
- `project_social_reach_nervous_system` — full sense-and-respond contract: provenance + back-prop + quarantine + restitution. The hub-layer extension of this.
- `project_trust_as_efficiency_signal` — trust reduces compute burden; cost asymmetry mechanism. Foundational for "DDoS is unearned compute reach."
- `project_substrate_scale_ceiling` — Tier 3 federation paradigm; ~100M households × 1 node.
- `project_dht_vs_libp2p_scoping` — DHT is expensive/authoritative; push operational state to libp2p. Why hub federation must scale outside the DHT.
- `project_three_layer_truth_model` — DHT / libp2p / doorway. Hub composes the libp2p-and-storage layer.
- `project_doorway_single_target_no_fanout` — substrate moves bytes; doorway projects + caches. Constitutional.
- `project_doorway_peer_registration` — inside-out. Peers register with doorway, not the inverse.
- `project_doorway_manifest_driven_routes` — registry-driven proxy. Doorway routes self-register via manifest.
- `project_peer_native_account_canonical_surface` — doorway as OAuth-RP. Doorway presents identity; never owns it.
- `project_p2p_is_hosting` — mesh is hosting; doorway is optional projection.
- `project_hub_archetype_abstraction` — abstract Hub interface; HouseholdHub and CollectiveHub. This spec renames HouseholdHub → DwellingHub.
- `project_household_fabric` — elohim-operator as household-fabric-manager. The hub's AI-coordination surface.
- `project_elohim_hub_elevation` — elohim-node graduates into hub instances. Hub-to-hub federation as Tier 3 scaling story.
- `project_substrate_floor_elohim_ceiling` — substrate determines deterministic gates; elohim adds discernment. The pattern for AI-coordination reach.
- `project_signal_kind_extensible_protocol_class` — feedback vocabulary extension via schema + validator + manifest. Path for AbusePattern.
- `project_social_compute_collective_is_stewardship_unit` — household is one kind of collective; CollectiveHub generalizes.
- `project_intelligence_revolution_scales_to_humans` — protocol scales TO human complexity, not away.
- `project_redeploy_the_substrate` — same tools redeployed on commons hardware. Hubs are the redeployment.
- `project_household_is_resilience_unit` — resilience is hub-to-hub, not peer-to-peer. Defense reach lives here.
- `project_multi_doorway_human_registration` — humans register with multiple doorways for resilience.
- `project_inventory_exchange_not_byte_replication` — gossip is metadata-only; bytes single-target.
- `project_reach_gate_is_elohim_mediated_matchmaking` — substrate floor + elohim discernment for reach decisions.

### Sibling specs

- `2026-05-02-elohim-hub-boundaries-design.md` — the predecessor spec sketching the Hub trait. This spec extends it with the four reach surfaces and the DwellingHub renaming.
- `2026-05-01-atproto-lexicon-projection-doorway-design.md` — federation projection at doorway. Sibling for the SSR-attestation question.
- `2026-04-28-doorway-blob-registry-routing.md` — manifest-driven RouteRegistry plan. The pattern this spec builds doorway responsibilities on.
- `2026-04-23-epr-phase-2c-libp2p-federation-design.md` — libp2p federation substrate. Hub federation rides on top.
- `2026-05-01-light-up-the-topology-design.md` — predecessor for the elohim-hub-boundaries spec.
- `2026-05-01-computation-attestation-graduated-rigor-design.md` — computation attestation pattern; sibling for SSR projection attestation.

### Doorway crate docs

- `doorway/doorway-service/ARCHITECTURE.md` — current state of doorway as bootstrap + signal + gateway.
- `doorway/doorway-service/FEDERATION.md` — fediverse-pattern federation with DHT-as-truth.
- `doorway/doorway-service/SCALING.md` — two-axis scaling (Projection unbounded, Identity Hosting bounded). This spec adds the hub-layer coordinate.
- `doorway/doorway-service/REACH.md` — reach gating per-request. The local enforcement surface.

### Manifesto / epic content

- `genesis/docs/content/elohim-protocol/autonomous_entity/epic.md` — Maria's restaurant story. Tone reference for elohim-operator scope; not the same domain as network protection.
- `genesis/docs/content/elohim-protocol/autonomous_entity/public_observer/` — possibly relevant to elohim-operator network role; worth checking when that role's manifest is concretely scoped.

### Recent commits

- `feat(ssr-delivery): wire alpha cluster end-to-end for SSR observability` — the precipitating moment.
- `fix(elohim-site): handle Angular 19 SSR's index.csr.html output` — SSR delivery becoming first-class.
- `fix(doorway): install curl + ca-certificates for v8 build.rs` — doorway crate operational health.

## Open questions worth bookmarking

These are bigger than stub-epic seeds — they shape multiple future sprints' framing.

- **What's the noun for AI-coordinated network protection at hub federation scope?** Earlier in this brainstorm we considered "elohim-autonomous-entity," but the manifesto's autonomous-entity domain (Maria's restaurant exit-to-community) is different. Possibly "elohim-operator at network scope," possibly a specialization of `public_observer`, possibly its own thing. The spec uses "elohim-operator" for the per-hub role and leaves the cross-hub-aggregate noun open.

- **How does the dwelling/collective attitude difference manifest concretely in code?** Most likely in defaults, manifests, and discernment shape — not trait surface. Worth confirming when DwellingHub and CollectiveHub move from sketch to implementation.

- **How does the protocol perform a "graceful first-attack response" for a brand-new dwelling with no standing?** Memory `project_graduated_recovery_authority` insists community always extends initial standing; the protocol's bootstrap-attack-response story needs to honor that without becoming a free DDoS amplifier.

- **What's the human-witnessable surface for elohim-operator actions at hub scope?** Per memory `project_substrate_floor_elohim_ceiling`, discernment is observable. What's the dashboard, the notification, the audit trail? An epic of its own.

- **Does FANG-subsumption need a concrete user story to make the scaling target legible?** Memory `project_subsume_g_f_a_via_it_just_works` insists the standard is grandmother-credible, not internal-demo-good. The hub layer should have an early concrete story (mobile inference for one family, federation defense for one neighborhood) that proves the scaling shape without requiring 100M dwellings.

- **What does a federation manifest look like in v1?** The current `RouteRegistry` is doorway-shaped. The federation manifest is hub-shaped — declares peer hubs, contracts, signal subscriptions, projection postures. Likely a sibling schema to the route registry, with overlap.
