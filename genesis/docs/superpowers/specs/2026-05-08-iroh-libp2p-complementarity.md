---
title: iroh ↔ libp2p Complementarity — Three Substrate Transport Tracks, Anti-Capture by Design
status: Draft (architecture spec — gates Phase 11 backend wiring)
created: 2026-05-08
related:
  - genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
  - genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md
  - genesis/docs/superpowers/specs/2026-05-02-elohim-hub-boundaries-design.md
  - genesis/plans/2026-04-13-device-archetypes-design.md
  - genesis/plans/2026-04-10-agency-phase-registration-design.md
  - genesis/docs/content/elohim-protocol/hardware-spec.md
  - elohim/elohim-storage/src/p2p_iroh/README.md
  - elohim/elohim-storage/tests/bench_blob_perf.rs
memory_anchors:
  - project_iroh_parallel_stack_phases3_7_landed
  - project_iroh_parallel_stack_phase0_blocker
  - project_three_layer_truth_model
  - project_dht_vs_libp2p_scoping
  - project_substrate_scale_ceiling
  - project_hub_archetype_abstraction
  - project_elohim_hub_elevation
  - project_household_is_resilience_unit
  - project_household_horizontal_scaling
  - project_multi_device_humans
  - project_multi_doorway_human_registration
  - project_shem_is_p2p_live_canvas
  - project_alpha_topology_bootstrap_pair
  - project_compute_and_model_independent_diversity_surfaces
  - project_no_sovereignty_stewardship_over_ownership
  - project_doorway_is_federation_surface_atproto
  - project_doorway_single_target_no_fanout
  - project_doorway_peer_registration
  - project_doorway_manifest_driven_routes
  - project_p2p_is_hosting
  - project_reach_earned_at_authoring
  - project_social_reach_nervous_system
  - project_trust_as_efficiency_signal
  - project_signal_kind_extensible_protocol_class
  - project_substrate_floor_elohim_ceiling
  - project_household_fabric
  - project_elohim_subagent_specialists
  - project_elohim_as_counsel
  - project_first_class_graph_pattern
  - project_intelligence_revolution_scales_to_humans
  - project_redeploy_the_substrate
  - project_subsume_g_f_a_via_it_just_works
  - project_innovators_dilemma_diagnosis
---

## Why this spec exists

The iroh parallel stack landed on `dev` (commit `c3708e77`). Phases 1–10 of `2026-05-07-iroh-parallel-stack.md` are complete: every wire plane (blob, gossip, sync, EPR, EPR-atom, shard, view-fed, identity-handshake, trust, reach, discovery) has parity-tested ALPNs over iroh QUIC, with a cross-stack peer-map bridging libp2p `PeerId` ↔ iroh `NodeId` via canonical `agent_cid`. Loopback bench (`tests/bench_blob_perf.rs`) shows iroh winning p50 at every blob size by 4×–290× — but that's loopback on a dev machine, not a phone, not a Chromebook, not a recycled laptop on cellular.

Phase 11's prerequisites all assume a cutover decision has been made. **It hasn't.** Naming the right end-state — full replacement, partial, or permanent dual-stack — is load-bearing because backend wiring, HTTP route graduation, seeder rewrite, gossip publish wiring, and rollback drill all branch on it.

The two surface-level pulls are simplification (one transport everywhere) and diversity (transport monoculture is a capture vector). The deeper question is what **simplification means** — is the unit "transport choice" or "each architectural layer has exactly one job"? This spec argues the second: **structural simplification of three transport tracks each doing one job is simpler in the way the protocol cares about** than reductive simplification to one transport. The cost is forever-dual-stack on Track 2; the value is consumer-grade-first-class agency, anti-capture posture, and a structural ceiling on hub-as-datacenter pressure.

## The three substrate transport tracks (and a fourth web2 projection)

The protocol's transport surface decomposes into three structurally distinct tracks that each do one job, plus the doorway projection layer. Each track has a different library, a different device-class, and a different anti-capture property. **Conflating them is the architecture mistake; keeping them distinct is the simplification.**

```
                                           ┌─────────────────────────────────────┐
                                           │ TRACK 4 — Doorway web2 projection   │
                                           │ HTTP, OAuth-RP, manifest-driven     │
                                           │ Browser visitors, federation        │
                                           │ projection (ATProto / future)       │
                                           │ NOT a P2P participant               │
                                           └─────────────────────────────────────┘
                                                          ▲
   ┌─────────────────────────────────────┐                │
   │ TRACK 1 — DHT notary layer          │                │
   │ kitsune2 / tx5 over WebRTC          │                │
   │ Identity, integrity, stewardship    │                │
   │ contracts, reach-earning, REA       │                │
   │ Source of truth                     │                │
   └─────────────────────────────────────┘                │
        ▲                ▲                ▲              ▲
        │                │                │              │
   ┌────┴──────┐    ┌────┴──────┐    ┌────┴──────┐  ┌────┴──────┐
   │ Wearable  │    │ Phone     │    │ Laptop    │  │ Tier-3    │
   │ IoT       │    │ Chromebook│    │ Recycled  │  │ DwellingHub│
   │ Sensor    │    │ Tablet    │    │ NUC, Pi   │  │ CollectiveHub│
   └────┬──────┘    └────┬──────┘    └────┬──────┘  └────┬──────┘
        │                │                │              │
        │                │                │              │
        ▼                ▼                ▼              ▼
   ┌─────────────────────────────────┐  ┌─────────────────────────┐
   │ TRACK 3 — Hub-spoke bridge      │  │ TRACK 2 — Substrate     │
   │ HTTP-over-WebSocket             │  │ data plane (dual-stack) │
   │ Doorway-shaped semantics        │  │                         │
   │ Stewardship-contract-bound      │  │ ┌─────────────────────┐ │
   │ Spoke identity stays the spoke's│  │ │ libp2p (consumer-   │ │
   │ Hub is carrier, not owner       │  │ │ grade direct +      │ │
   │                                 │  │ │ intermittent)       │ │
   │ Wearables, IoT, phone-as-spoke  │  │ │ TCP+yamux, QUIC,    │ │
   │ NO full elohim-storage          │  │ │ WebRTC, Circuit     │ │
   │                                 │  │ │ Relay v2            │ │
   └─────────────────────────────────┘  │ └─────────────────────┘ │
              ▲                          │            ↕            │
              │                          │  cross-stack peer-map   │
              │                          │  permanent structural   │
              ▼                          │  schema                 │
   ┌─────────────────────────────────┐  │            ↕            │
   │ DwellingHub / CollectiveHub     │  │ ┌─────────────────────┐ │
   │ (Track 3 endpoint AND Track 2   │  │ │ iroh (hub-to-hub    │ │
   │ federation participant)         │  │ │ federation)         │ │
   │                                 │←─┤ │ iroh-blobs, iroh-   │ │
   │ Constitutional: bounded by      │  │ │ gossip, custom ALPNs│ │
   │ humans inside that govern       │  │ │ over QUIC           │ │
   └─────────────────────────────────┘  │ └─────────────────────┘ │
                                         └─────────────────────────┘
```

### Track 1 — DHT notary layer (consumer-grade first-class for ALL device classes)

| Concern | Choice |
|---|---|
| Library | `kitsune2` + `tx5` over **WebRTC** |
| Bootstrap | `https://doorway.elohim.host/bootstrap` (operator-runnable; multi-doorway registration) |
| Signal | `wss://signal.doorway.elohim.host` (operator-runnable; multi-doorway registration) |
| ICE servers | `stun:stun.cloudflare.com:3478`, `stun:stun.l.google.com:19302` (replaceable; operator-set) |
| Device classes | All — wearable (capability-level 0–1), phone, browser, behind-CGN, hub |

**This track does not change in any iroh/libp2p decision.** WebRTC works on every device class that runs a browser-equivalent runtime. STUN+SBD signal punches through carrier-grade NAT. A wearable running a micro-conductor (`device-environmental-sensor`, `device-biometric-fob`) is a first-class DHT participant today. A phone running a lightweight messaging hApp is a first-class DHT participant today. The DHT layer is settled and the spec leaves it untouched — it is the consumer-grade-first-class story for identity, integrity, and stewardship contracts.

**The DHT is the source of truth.** Tracks 2 and 3 are operational projections / data-ops; they do not authorize, they execute on top of authorizations notarized here. Iroh's `discovery_n0()` and libp2p's Kademlia are *peer discovery* mechanisms for Track 2; they do not replace DHT-notarized identity.

### Track 2 — Substrate data plane (dual-stack, structurally permanent)

Two profiles, one shared wire format, runtime-selected per peer pair via the cross-stack peer-map.

| Profile | Library | Device classes | Job |
|---|---|---|---|
| Hub-to-hub federation | **`iroh = 0.92` + `iroh-blobs = 0.94` + `iroh-gossip = 0.92`** | DwellingHub, CollectiveHub, always-on Tier-3+, Pi/NUC running stage-4 operator | High-throughput federation, blob distribution, gossip, custom ALPNs over QUIC. Where iroh's bench wins are real and BLAKE3 chunked verified streaming is the natural primitive. |
| Consumer-grade direct | **`libp2p = 0.53`** (TCP+yamux, QUIC, WebRTC, Circuit Relay v2, Kademlia) | Laptop intermittent peer, gaming desktop when on, NUC behind weird NATs, browser-WebRTC for direct-participation, Stage-4-lightweight phone with own elohim-storage | Reliable substrate participation under intermittent connectivity, UDP-restricted networks, battery-constrained devices, and browser-direct-P2P scenarios. No n0 dependency in the auth path. |

**Both profiles share the wire format** (`crate::p2p::wire` MessagePack/CBOR frames). Adding a new plane = one wire spec + two ALPN registrations (one per transport), not two diverging schemas.

**Cross-stack peer-map is permanent structural schema, not transition hack.** Every peer's canonical identity (`agent_cid`, DHT-notarized) carries a transport-profile manifest; connections pick the highest-shared profile.

```rust
// In elohim-storage's view types and DHT-projected peer records:
struct PeerTransportManifest {
    agent_cid: AgentCid,
    iroh: Option<IrohTransportProfile>,     // None for non-iroh peers
    libp2p: Option<Libp2pTransportProfile>, // None for iroh-only peers
    discovery: Vec<DiscoveryMethod>,        // pkarr, kademlia, mdns, manifest-declared
    capability_level: u8,                    // 0–5 from device archetypes
}

struct IrohTransportProfile {
    node_id: NodeId,
    relays: Vec<RelayUrl>,                   // n0 default + operator-self-hosted
    supports: Vec<IrohPlane>,                // [Blob, Gossip, Sync, Epr, EprAtom, Shard, ViewFed]
}

struct Libp2pTransportProfile {
    peer_id: PeerId,
    listen_addrs: Vec<Multiaddr>,            // includes /webrtc, /tcp, /quic, circuit-relay
    supports: Vec<Libp2pPlane>,              // [Blob, Gossip, Sync, Epr, EprAtom, Shard, ViewFed, IdentityHandshake, Trust, ReachAuth]
}
```

The protocol negotiates per-call: a laptop with `{libp2p: full, iroh: None}` and a Tier-3 hub with `{iroh: full, libp2p: receive-only}` connect over libp2p. Two Tier-3 hubs connect over iroh. **Users don't see the choice.**

### Hub graduation gradient — hubs are a role, not a hardware tier

A hub is defined by **role**, not by hardware tier:

> A peer is acting as a hub when it (a) accepts Track 3 spoke connections under DHT-notarized stewardship contracts and serves them via HTTP/WS, AND (b) federates horizontally on Track 2 with peer hubs under continuously-negotiated REA contracts.

Both conditions can be satisfied on consumer-grade hardware. **The hub role is graduated, not gated.**

| Tier | Hardware | Examples | Track 2 transport posture | Track 3 spoke capacity | AI inference role |
|---|---|---|---|---|---|
| **Consumer-grade hub** | Repurposed laptop, gaming desktop intermittent, old NAS, composed thin-client-batch via coordinator | `recycled-laptop`, `gaming-desktop`, `thin-client-batch` (composed) | **libp2p primary** (intermittent, TCP fallback through residential NAT) + iroh receive when peer is iroh-only | small (a few household members + extended family custodial keys) | none → relay-to-peer-hub |
| **Tier-1 lightweight hub** | Always-on Pi 4, NUC, intentional mid-tier steward | `raspberry-pi-4`, `home-nuc` | **iroh primary** + libp2p receive for consumer-grade peers | medium (household + neighborhood spokes) | small CPU-bound models |
| **Tier-3 full hub (DwellingHub canonical)** | Family-node base/extended, dedicated server | `family-node-base`, `family-node-extended`, `dedicated-server` | **iroh primary** + libp2p receive | full (extended family + community + custodial-keyed relatives) | full local inference (70B-class) |

**Why this matters — the onboarding funnel for hubs:**

The substrate's Tier-3 federation paradigm (`project_substrate_scale_ceiling`) is a **target**, not a precondition. Consumer-grade hubs are how households reach the target:

1. **Meets people where they are.** Someone with an old laptop and a desire to host their family's photos can run a consumer-grade hub today, without a $3,000 Tier-3 investment. Someone with a gaming desktop already volunteers burst compute when not gaming. Someone with a closet Pi 4 already runs a 24/7 always-on hub. The protocol counts all of these as first-class hub stewards.
2. **Repurposes hardware that would otherwise be e-waste.** Recycled-laptop, NAS upgrades, lab thin-client batches, hand-me-down desktops — the protocol's substrate makes them meaningful again. Inclusion + circularity.
3. **Onboarding into full-capability DwellingHubs.** Consumer-grade hub is the entry point; Tier-1 lightweight is the intermediate; Tier-3 full is the destination. Each tier is first-class **within its capabilities**, with a clear graduation path. AI inferencing graduation lands at Tier-1+ (small models) and Tier-3 (full local 70B-class).
4. **Reaches edge capabilities earlier.** A neighborhood with three consumer-grade hubs federating is already FANG-equivalent at neighborhood scale long before any of those households upgrades to Tier-3. Federation density wins before hardware density.
5. **Constitutionally distinct from datacenters.** A consumer-grade hub on a recycled laptop is structurally bounded — it can't grow into a datacenter because the hardware doesn't support it, the steward count is small, and the stewardship contracts cap acceptance. **The graduation path goes through DwellingHub, never through datacenter.** Tier-3 is the ceiling for substrate-native hub composition; further scaling is via federation density (more hubs), never vertical hub scaling.

**Graduation preserves identity.** Per `hardware-spec.md` migration-preserves-everything: when a steward graduates from a consumer-grade hub to a Tier-3 hub, the agent identity moves with them. The substrate sees a transport-profile change (libp2p-primary → iroh-primary), not an identity change. The stewardship contracts the hub carries are reassigned to the new hardware via DHT-notarized REA events witnessed by the household's stewards. Spokes don't notice the upgrade beyond their hub's `node_addr` rotating.

**Track 2 transport implication:** the dual-stack posture is what makes consumer-grade hubs viable. If we went iroh-only, a consumer-grade hub on intermittent residential networking with carrier-grade NAT and battery-aware idle disconnect would struggle to participate. libp2p's TCP+yamux fallback + Circuit Relay v2 + WebRTC keeps consumer-grade hubs first-class. **Forfeiting consumer-grade hub viability to the simplification of one transport would forfeit the protocol's onboarding funnel for hubs themselves.**

### The hub's elohim-operator — runtime composition + capacity negotiation

Every hub runs an **elohim-operator** — a context-bound specialist agent (`project_elohim_subagent_specialists`, `project_household_fabric`) that fills the role a household's devops/IT person would fill if they had one. The operator treats the hub's hardware as a **cluster** (whether the cluster is one recycled laptop, three NUCs, five blades, or a Tier-3 family-node-extended with hot-swap modules) and continuously negotiates:

- **Internal cluster operations** — hot/cold blade migration, leader election, replica placement, PVC movement, blob tiering across NVMe/bulk/encrypted-shard storage. Kubernetes-class concerns absorbed *inside* the hub so household stewards don't have to think about them.
- **Stewardship-vs-capacity tradeoffs** — how much compute / bandwidth / storage / AI inference budget each Track 3 spoke commitment, each Track 2 federation contract, and each internal household need consumes. When to defer, when to spend, when to renegotiate.
- **External federation participation** — peer-hub gossip, sponsored compute contracts (sending and receiving), AbusePattern signal emission, defense-reach earning, federation-manifest declaration of which peer hubs are reachable.
- **Track 3 mediation** — when a wearable streams sensor data, the operator decides which signals get notarized; when a phone-as-spoke requests inference, the operator applies reach-earning; when a stewardship contract is renegotiated, the operator drafts the renegotiation for steward witness/sign.

The operator is bound by the **substrate-floor / elohim-ceiling pattern** (`project_substrate_floor_elohim_ceiling`):

- **Substrate floor** — deterministic gates from Track 1 stewardship contracts + Track 2 reach-earning rules. What the hub MUST / MUST NOT do.
- **Elohim ceiling** — the operator's discernment within the floor's permission. What the hub CHOOSES to do given context. **Discernment is signed, witnessable, reversible** by the household's stewards. The operator never overrides the floor; it adds context-aware judgment on top.

**The operator graduates with the hub.** A consumer-grade hub runs a smaller operator (modest context window, defers most heavy discernment to peer hubs); a Tier-1 lightweight hub runs a more capable operator (small CPU-bound inference, more local discernment); a Tier-3 full hub runs the canonical operator (full 70B-class local inference, federation-scale discernment, sponsors compute for smaller peer hubs). When a steward upgrades hardware, the operator's identity and decision history migrate intact via DHT-notarized REA events; the substrate sees a capability-grant evolution, not a re-installation. **This is what makes the consumer-grade-hub onboarding funnel real** — a household can start with a recycled laptop running a humble operator and grow into a Tier-3 family-node with a full operator without losing context.

**Serving the home AND the broader social resilient compute simultaneously.** The operator's job is two-way: it stewards the hub's capacity for the household's internal needs (family inference requests, photo backups, custodial keys, learning paths) AND for the household's outward federation commitments (sponsored compute for peer hubs, defense-reach earning, AbusePattern emission, content distribution). Both are first-class; the operator's discernment is what continuously balances them. **This is the household-fabric-manager role, concretely** — the entity that holds in tension "serve my family" and "serve the social fabric my family is part of."

**Stewardship is two-way:** the operator stewards the hub's capacity for the household's needs; the household stewards the operator's discernment shape via manifest configuration and witness-and-reverse mechanisms. Per `project_elohim_as_counsel`, the operator has standing to act in the household's defense even against the household's current-moment preferences when stewardship is at stake — but always within the substrate floor and always reversibly.

The operator's specific role-manifest, witness-UX, and renegotiation-flow are out of scope for this spec — they're seeded as stub epics in `2026-05-08-doorway-hub-edge-design.md` (Stub-epic seeds #2, #3, #6, #9). What this spec settles is **the operator is the entity that orchestrates the hub's use of Tracks 1, 2, and 3**, and the dual-stack Track 2 posture is what gives operators on consumer-grade hardware a first-class role from day one.

**Hubbiness is a dial, not a binary.** The boundary between "this device is just my device" and "this device is also a hub" is a continuous setting humans dial at the device level — not a hard architectural distinction. A laptop at 6pm is just-the-steward's-laptop and the operator's role on it is small (the steward gets the consistency of personal space — the operator doesn't intrude on what they're doing). The same laptop plugged in overnight, shared with the household, becomes a-hub-too — its operator picks up Track 3 spoke commitments and Track 2 federation participation. Humans dial up hubbiness as comfort and capacity allow; humans hand it off as Tier-3 hardware arrives and the household-fabric-manager role migrates to a more capable operator. **The steward+device layer gives humans the consistency of their own space; the hub layer gives the elohim-operator a dwelling to be opinionated and helpful at social-coordination scope without stepping on human toes.** The operator's discernment is at hub scope; what happens on the personal device is the human's, full stop. The dial is owned by the human, declared via standing manifests, signed/witnessable/reversible at every increment.

### Track 3 — Hub-spoke bridge (wearables, IoT, phone-as-spoke; no full storage)

| Concern | Choice |
|---|---|
| Library | **HTTP-over-WebSocket** with doorway-shaped semantics (existing `doorway/doorway-service` pattern; per-hub spoke-endpoint extension) |
| Authorization | **DHT-notarized stewardship contract** (Track 1) per spoke-hub relationship, requiring explicit steward consent. Spoke retains identity; hub is carrier, not owner. |
| Device classes | Wearable, IoT environmental sensor, phone-as-spoke, Chromebook spoke, tablet, biometric fob, observer mic-array, observer camera |
| Anti-capture mechanism | **Track 3 is HTTP-shaped, NOT a substrate transport.** Each spoke connection is a stateful WS session bounded by hub compute budget. Vertical absorption hits hub compute caps long before substrate-scale caps. |

**Why this is a separate track and not "iroh on phones":**

iroh requires UDP and works best on always-on hardware. A wearable running a single-zome micro-conductor doesn't have UDP keepalive budget — it streams sensor data when paired and sleeps. A phone on cellular faces UDP-blocked carriers and aggressive battery-saving idle disconnect. **Forcing these devices onto Track 2 (either iroh or libp2p) creates pressure to either run a full elohim-storage they don't need, or be excluded from substrate participation altogether.** Track 3 dissolves this: spokes get convenient HTTP/WS bridging through their dwelling hub, with explicit stewardship contracts that preserve their agency.

**The crucial property: spoke identity stays the spoke's.** The DHT-notarized stewardship contract says "DwellingHub X carries data on behalf of Spoke Y, under terms Z, witnessable by stewards [...]." When the spoke wants to migrate, they take their identity to a different dwelling hub, doorway, or direct substrate participation — no lock-in.

### Track 4 — Doorway web2 projection (visitors, hosted users, federation)

Per `2026-05-08-doorway-hub-edge-design.md` — kept unchanged by this spec for completeness. Doorway speaks HTTP, presents identity (OAuth-RP, never owns), routes via manifest, and **never swarms libp2p or iroh**. Doorway's co-located storage pod runs Track 2.

## Device-archetype × transport matrix

For each archetype in `genesis/data/devices/devices.json`, which tracks does it use?

| Archetype | Cap | Track 1 (DHT) | Track 2 (substrate) | Track 3 (spoke) | Track 4 (doorway) |
|---|---|---|---|---|---|
| `2019-android-phone` | 2 | tx5/WebRTC | libp2p (if direct, rare) | HTTP/WS via dwelling hub (default) | optional client |
| `chromebook-edu` | 2 | tx5/WebRTC | libp2p (if direct, optional) | HTTP/WS via dwelling hub (default) | optional client |
| `recycled-laptop` | 3 | tx5/WebRTC | **libp2p primary** (intermittent, TCP fallback) | optional Track 3 spoke; **Track 3 host as consumer-grade hub when plugged in** | optional client |
| `gaming-desktop` | 4 | tx5/WebRTC | **libp2p primary** when on (variable availability) | **optional Track 3 host (burst consumer-grade hub when not gaming)** | optional client |
| `raspberry-pi-4` | 3 | tx5/WebRTC | **iroh primary** (always-on, can also accept libp2p) | **Track 3 host (Tier-1 lightweight hub)** | n/a |
| `home-nuc` | 4 | tx5/WebRTC | **iroh primary** (always-on; libp2p receive-only) | **Track 3 host (Tier-1 lightweight hub)**; potentially Track 4 host | optional host |
| `family-node-base` | 5 | tx5/WebRTC | **iroh primary; libp2p receive for consumer-grade peers** | **Track 3 host (Tier-3 DwellingHub canonical)** | hub host (if doorway-enabled) |
| `family-node-extended` | 5 | tx5/WebRTC | **iroh primary; libp2p receive** | **Track 3 host (Tier-3 DwellingHub extended)** | hub host |
| `dedicated-server` | 5 | tx5/WebRTC | **iroh primary; libp2p receive** | **Track 3 host (CollectiveHub)** | hub host |
| `k8s-pod-256mb` | 5 | tx5/WebRTC | **iroh primary** (deployment convenience; not a real archetype) | n/a | hub host |
| `observer-mic-array` | 1 | tx5/WebRTC | n/a | **HTTP/WS to dwelling hub** | n/a |
| `observer-camera` | 1 | tx5/WebRTC | n/a | **HTTP/WS to dwelling hub** | n/a |
| `environmental-sensor` | 1 | tx5/WebRTC (via gateway) | n/a | **LoRaWAN → gateway → HTTP/WS to dwelling hub** | n/a |
| `biometric-fob` | 0 | tx5/WebRTC (paired) | n/a | **streams to paired device, hub-internal** | n/a |
| `thin-client-batch` | 1 individually / **3 composed** | tx5/WebRTC | n/a individually; **libp2p primary as composed hub via coordinator** | **HTTP/WS to dwelling hub** individually; **Track 3 host as composed consumer-grade hub** | n/a |

**Reading the matrix:**

- Every device class has Track 1 (DHT) participation. Identity is universal.
- Track 2 transport posture is **graduated by always-on-ness and hub role**: Tier-3+ always-on hubs → iroh primary with libp2p receive; Tier-1 lightweight always-on hubs (Pi 4, NUC) → iroh primary; consumer-grade hubs and consumer-grade direct peers (recycled laptop, gaming desktop, intermittent) → libp2p primary with iroh receive when peer is iroh-only. **Hub-capable status is a property of the role, not the hardware tier.**
- Track 3 (HTTP/WS spoke bridge) has two participant types: **spoke participants** (L0–L2 devices that don't run elohim-storage and bridge through a hub — wearables, IoT, phone-as-spoke), and **hub hosts** (any L3+ archetype acting as a hub, plus L1 thin-client-batch when composed via coordinator). The consumer-grade-hub onboarding funnel runs through this column.
- Track 4 is **opt-in projection** for any device class that wants HTTP-shaped access.

## Plane-by-plane verdict and decision rule

For each wire plane in elohim-storage, the verdict is one of:

- **iroh-canonical, libp2p-fallback** — iroh is the protocol; libp2p stays for peers that can't speak iroh
- **dual-stack permanent** — both run, selected by transport profile, parity-tested forever
- **libp2p-canonical, iroh-receive** — libp2p is the canonical path; iroh stays for hub-to-hub when convenient

| Plane | Verdict | Rationale |
|---|---|---|
| **Blob** | **iroh-canonical, libp2p-fallback** | iroh-blobs IS the protocol — BLAKE3 chunked verified streaming is the right primitive; loopback bench shows 4×–290× p50 wins. libp2p `BlobProtocol` stays as the fallback for consumer-grade peers that can't establish a QUIC connection (rare but kept). |
| **Gossip (inventory, identity-binding, attention, feedback, recovery topics)** | **dual-stack permanent** | Inventory gossip MUST reach consumer-grade peers — they're in the resilience graph. iroh-gossip for hub-to-hub; libp2p gossipsub for consumer-grade. Both publish to the same logical topic-id (BLAKE3-hashed); peer-map handles cross-stack delivery. |
| **Sync (`/elohim/sync/2.0.0`)** | **dual-stack permanent** | Wire format already shared. Pick by transport profile. |
| **EPR (`/elohim/epr/2.0.0`)** | **dual-stack permanent** | EPR codec is transport-agnostic by design. Pick by profile. |
| **EPR-atom (`/elohim/epr-atom/2.0.0`)** | **dual-stack permanent** | Same as EPR. CBOR codec on iroh; MessagePack on libp2p (both supported by harness). |
| **Shard (`/elohim/shard/2.0.0`)** | **dual-stack permanent** | Reed-Solomon coding stays in pure Rust; framing is per-transport. |
| **View-federation (`/elohim/view-federation/2.0.0`)** | **dual-stack permanent** | 256 KiB cap on responses applies on both. |
| **Identity-handshake (`/elohim/identity-handshake/2.0.0`)** | **dual-stack permanent** | The feature is canonical to the protocol. Integrity is preserved by Track 1 DHT-notarized agent identity (kitsune2/tx5) and the signed wire frames the handshake exchanges — **not** by any transport-level security property. Both transports preserve DHT-derived integrity equally. Current implementation is libp2p-shaped because that's where the code lived; iroh-side ALPN is parity-tested. Selection per transport-profile via cross-stack peer-map. |
| **Trust (`/elohim/trust/2.0.0`)** | **dual-stack permanent** | Trust attestations are DHT-notarized; the wire just carries them. Same integrity property as identity-handshake — DHT layer + signed frames, not transport. |
| **Reach-authorization** | **n/a — internal service, not a wire plane** | Per `p2p_iroh/README.md`, reach-authorization is an internal service that consumes identity context from the handshake plane and reads DHT-notarized stewardship contracts to make reach-gate decisions. **The feature itself is canonical to the protocol.** Its wire participation is via the planes that carry the data being authorized (sync, EPR, shard, etc.) — those planes' verdicts apply. How reach-authorization's full wire-composition is shaped while preserving DHT-derived integrity is a subject of in-progress feature design — see §What this spec deliberately does NOT settle. |
| **Discovery** | **dual-stack permanent: pkarr (iroh) + Kademlia (libp2p)** | Multi-bootstrap. n0 demoted to one-of-many defaults; operators self-host pkarr resolvers. Kademlia stays for cross-collective discovery on consumer-grade peers. mDNS available for local fabric. |

### Decision rule (Phase 11 backend wiring)

When wiring a backend in Phase 11 (replacing a stub backend so the daemon routes through iroh ALPNs in iroh mode), the rule is:

```
For each backend call site:
  1. Look up the plane's verdict in this spec.
  2. If verdict is "iroh-canonical, libp2p-fallback":
       - Implement iroh-side as the primary path
       - Keep libp2p-side as a fallback that's selected when peer's transport-profile lacks iroh
  3. If verdict is "dual-stack permanent":
       - Implement both sides
       - Selection: peer's transport-profile manifest (cross-stack peer-map)
       - Wire format MUST be identical; codec helpers in `super::codec::*` enforce this
  4. If verdict is "libp2p-canonical, iroh-receive":
       - libp2p is the primary path; iroh ALPN is registered but not preferred
       - Iroh-side participation is for hub-to-hub convenience, not user-facing flow
  5. NEVER:
       - Hard-code transport choice based on hostname or config flag
       - Bypass the cross-stack peer-map for transport selection
       - Add a third transport without going through this spec's update process
```

## Anti-capture: how dual-stack prevents transport monoculture

The 3-pillar coupling (`project_three_layer_truth_model`) prevents capture by keeping each pillar structurally distinct. Within Track 2 (the libp2p/data-ops pillar), **transport monoculture is itself a capture vector** — for two reasons.

### Reason 1: discovery centralization

Iroh's `discovery_n0()` defaults to n0's hosted DNS resolvers (`relay.iroh.network`, `dns.iroh.link`). The underlying mechanism (pkarr — signed records on a P2P DHT) is fully decentralized; n0 just runs the default public resolvers. **Defaulting is acceptable; *only*-ing is not.**

If the substrate's discovery becomes n0-only, n0 becomes a single point of operational compromise. An n0 outage halts hub federation. An n0 acquisition transfers a chokepoint. An n0 policy change becomes protocol policy.

**Mitigation (built into this spec):**

1. **pkarr resolvers are operator-self-hostable.** Every doorway operator can run a pkarr DNS resolver. The federation manifest declares which resolvers a given hub trusts; n0 is one of many.
2. **libp2p Kademlia stays as the alternative discovery mechanism.** Cross-collective discovery on consumer-grade peers uses Kademlia, which is fully P2P with no hosted dependency.
3. **Discovery method is per-peer-declared.** A peer's transport-profile manifest declares which discovery mechanisms it speaks. The protocol picks the highest-shared mechanism. A hub that distrusts n0 sets `discovery: [pkarr-self-hosted, kademlia]` and never queries n0.
4. **Cutover gate requires self-hostable pkarr in production.** Before libp2p-Kademlia can be retired (it can't, but if the question were ever raised), a self-hostable pkarr resolver running on a household-scale device must be production-soaked for one week.

### Reason 2: transport CVE / vendor-capture

A single transport stack is a single CVE target. iroh is younger (~7 months soak as of plan-write); libp2p is IPFS-soaked over years. Running both means a CVE in one isolates to that path; the other carries the substrate while the affected stack is patched.

This is **not the primary anti-capture argument** (defense-in-depth at the message layer is stronger — signed wire frames, content-addressed integrity, recovery via socially-derived security). But it is *a* property worth keeping, and it costs nothing extra given Track 2's dual-stack is already justified by consumer-grade-first-class.

## Anti-datacenter: how this spec prevents hub-as-datacenter pressure

The hub-as-datacenter risk: a DwellingHub that gets very good at iroh-only operation can absorb spokes faster than its stewards can govern, structurally pulling toward consolidation. Capture by aggregation, not by monoculture.

Five mechanisms in this spec actively prevent it:

### 1. DHT-notarized stewardship contracts cap spoke counts per hub

Per the agency-phase-registration design and `signal_kind`-extensible-protocol-class pattern: every spoke-hub relationship is a REA commitment (Track 1, DHT-notarized) requiring explicit steward consent. A hub with N stewards cannot unilaterally accept the 100,000th spoke — the stewardship-contract validator caps acceptance at what stewards can govern. **The cap is constitutional, enforced at the DNA validator level, not configurable.**

### 2. Federation reach-earning makes vertical growth structurally expensive

Per `project_reach_earned_at_authoring` and `project_social_reach_nervous_system`: every hop costs reach. A hub serving N spokes from M peer hubs pays N × M reach-earning at every distribution. **Federation density (more, smaller hubs) costs O(M); vertical absorption (one hub with 100M spokes) costs O(N²).** The cost asymmetry is built into the protocol's core economics; trust-as-efficiency-signal makes legitimate distribution near-zero-marginal-cost while unearned vertical growth pays exponentially.

### 3. Track 3 is HTTP-shaped, not a substrate transport

A hub absorbing spokes via Track 3 hits hub compute caps before substrate-scale caps. Each spoke session is stateful (HTTP/WS), bounded by the hub's compute budget, paid for by the stewardship contract. **There's no efficient protocol for "absorb 100M spokes" because Track 3 is deliberately not designed to scale that way.** The substrate (Track 2) is designed to scale via federation; Track 3 is designed to bridge wearables and IoT to dwelling-scale hubs. Different jobs, different scaling profiles, intentionally.

### 4. Track 2 (substrate) preserves consumer-grade direct participation

A laptop CAN choose direct libp2p substrate participation. A phone running its own conductor + lightweight elohim-storage CAN be a direct Track 2 peer. **The three paths (Track 2 direct, Track 3 spoke, Track 4 doorway-projected) are independent** — a hub has no structural absorption advantage. If a hub becomes oppressive, spokes leave for direct substrate participation. The exit option is structurally guaranteed.

### 5. Constitutional bifurcation: DwellingHub vs CollectiveHub vs nothing

Per `project_hub_archetype_abstraction` and `2026-05-08-doorway-hub-edge-design.md`: a dwelling hub is bounded by "humans inside can govern." Past that capacity, it must split (fork into two dwelling hubs) or convert to a CollectiveHub with delegated-stewardship governance. **There is no archetype for "datacenter."** Operators can run datacenter infrastructure as opt-in fronting (hyperscaler-fronting per the doorway-hub-edge spec), but the protocol does not natively model a hub at that scale.

**The corollary:** if someone wants to run "FANG-scale infrastructure," they have to run *thousands of federated hubs*, not one giant hub. Federation is the only scaling vector, and federation requires bilateral standing and continuously-negotiated REA contracts at every edge. The structural advantage federation has over centralization is built into the cost asymmetry (Reason 2 in §Anti-capture, mechanism 2 here). FANG-scale via federation is *possible*; FANG-scale via vertical consolidation is *expensive enough to be unattractive*.

## Subsuming Cloudflare and FANG (the federation answer)

Per `2026-05-08-doorway-hub-edge-design.md`'s thesis — **the federation IS the FANG-equivalent**, not any single hub. The four reach-earning surfaces (compute, distribution, defense, AI-coordination) absorb at federation aggregate.

This spec's transport-track decomposition makes the libraries concrete:

| FANG concern | Protocol mechanism | Track | Library |
|---|---|---|---|
| Aggregate compute (Google AI) | Mobile inference proxies at dwelling hubs; cross-hub compute sponsorship contracts | Track 3 (mobile→hub) + Track 2 (hub-to-hub) + Track 1 (REA contracts) | HTTP/WS + iroh + Holochain |
| Aggregate distribution (YouTube/Netflix CDN) | Single-target dispatch via doorway; peer-to-peer via Track 2; AbusePattern signals via Track 2 gossip | Track 4 (doorway projection) + Track 2 (substrate) | doorway HTTP + iroh-blobs / libp2p |
| Aggregate defense (Cloudflare) | Meet-and-protect contracts; cross-hub threat coordination via gossip; per-`/48`/`/64` rate limits at doorway | Track 1 (REA) + Track 2 (gossip) + Track 4 (doorway) | Holochain + iroh-gossip / libp2p-gossipsub + doorway |
| Aggregate algorithmic discernment (Facebook feed) | Elohim-operators per dwelling hub, federating discernment via Track 2 substrate gossip, signing actions on Track 1 DHT | Track 1 (DHT) + Track 2 (substrate) | Holochain + iroh-gossip / libp2p-gossipsub |

The substrate-floor / elohim-ceiling pattern (`project_substrate_floor_elohim_ceiling`) applies: substrate determines deterministic gates (allowed / blocked / pending); elohim-operator discernment escalates, sponsors, witnesses. Both layers are structurally bounded — neither is a unilateral authority.

## n0 centralization seam — full mitigation plan

The seam: iroh's `discovery_n0()` flag in `IrohConfig::use_n0_discovery` defaults to true; when true, the endpoint queries n0's hosted DNS resolvers for peer NodeAddr lookups.

Five-step mitigation, each with a concrete cutover-gate criterion:

### Step 1 (production-soaked): `discovery_n0()` becomes one-of-many defaults

**Change:** `IrohConfig` exposes `discovery_resolvers: Vec<DiscoveryResolver>` instead of a single boolean. Default list includes n0's resolvers AND any operator-configured resolvers. The `use_n0_discovery` boolean is deprecated in favor of explicit resolver configuration.

**Cutover gate:** the `IrohConfig` change is shipped, n0 is one of three+ default resolvers in production deployment, no production deployment uses `[n0]` as the sole resolver list.

### Step 2 (production-soaked): self-hostable pkarr resolver

**Change:** doorway operators run a pkarr DNS resolver alongside their bootstrap and signal services. Code: `doorway/doorway-service` adds a pkarr-resolver service module. Operator deployment config exposes the resolver via `https://<doorway>.elohim.host/pkarr`.

**Cutover gate:** pkarr resolver running on `doorway.elohim.host` for one week with zero unavailability beyond the doorway itself's uptime. A new hub joining the federation can configure `discovery_resolvers: [doorway.elohim.host/pkarr]` and never query n0.

### Step 3 (federation manifest): per-hub resolver trust declaration

**Change:** federation manifest entry per peer-hub includes `discovery_resolvers: [...]`. A hub that distrusts n0 publishes its manifest with self-hosted resolvers only. Peers connecting to it use only the resolvers it declares.

**Cutover gate:** federation-manifest schema extension lands; one production hub runs with `discovery_resolvers: [doorway.elohim.host/pkarr]` (no n0).

### Step 4 (resilience epic): pkarr resolver redundancy at federation scale

**Change:** dwelling hubs that host doorways automatically expose pkarr-resolver as part of the doorway role. As federation density grows, the substrate's pkarr-resolver count grows organically.

**Cutover gate:** at least three operator-self-hosted pkarr resolvers are reachable from the alpha cluster.

### Step 5 (distant — only if needed): protocol-level n0 deprecation

**Not a cutover gate.** Only contemplated if n0 takes an action incompatible with protocol governance (acquisition by a hostile party, policy changes that conflict with substrate values). At that point, the federation manifest mechanism (Step 3) lets every hub coordinate removing n0 from their resolver list — a federation-wide decision via gossip, not a protocol-level patch.

**Bottom line:** n0's hosted infrastructure is *acceptable as a default* during the soak period. It must be *replaceable per-deployment* before the cutover gate clears. It must *never become the only path*.

## Cross-stack peer-map as permanent structural schema

The existing `cross_stack_peer_map` Diesel migration (`2026-05-08-045024_cross_stack_peer_map`) was framed as a transition-bridge. **This spec graduates it to permanent structural schema.**

### Schema (Diesel + view-types)

```sql
CREATE TABLE peer_transport_manifest (
    agent_cid TEXT PRIMARY KEY,                  -- DHT-canonical identity
    libp2p_peer_id TEXT NULL,                    -- if peer speaks libp2p
    iroh_node_id TEXT NULL,                      -- if peer speaks iroh
    libp2p_addrs_json TEXT NULL,                 -- listen addrs, JSON array
    iroh_relays_json TEXT NULL,                  -- relay URLs, JSON array
    libp2p_supports_json TEXT NULL,              -- planes supported, JSON array
    iroh_supports_json TEXT NULL,                -- planes supported, JSON array
    discovery_methods_json TEXT NOT NULL,        -- ["pkarr", "kademlia", "mdns", ...]
    capability_level INTEGER NOT NULL,           -- 0-5 from device archetypes
    last_observed INTEGER NOT NULL,              -- unix timestamp
    CHECK (libp2p_peer_id IS NOT NULL OR iroh_node_id IS NOT NULL)
);
```

### View type (Rust → TypeScript)

```rust
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PeerTransportManifestView {
    pub agent_cid: String,
    pub libp2p: Option<Libp2pTransportProfileView>,
    pub iroh: Option<IrohTransportProfileView>,
    pub discovery: Vec<String>,
    pub capability_level: u8,
    pub last_observed: i64,
}
```

### Selection algorithm

```
fn select_transport(self_profile: &PeerTransportManifest,
                    peer_profile: &PeerTransportManifest,
                    plane: Plane) -> Result<TransportChoice> {
    // 1. Both peers must support the plane on at least one shared transport.
    // 2. Prefer iroh if both support iroh AND plane verdict allows iroh.
    // 3. Fall back to libp2p if either peer lacks iroh or plane verdict requires libp2p.
    // 4. If neither shares a transport, route via dwelling hub Track 3 if applicable.
    // 5. Else fail with NO_SHARED_TRANSPORT.
}
```

### What lives where

| Concern | Location |
|---|---|
| Agent identity (canonical) | Track 1 — DHT, kitsune2/tx5 |
| `libp2p_peer_id` derivation | DHT-attested via identity-binding gossip topic |
| `iroh_node_id` derivation | DHT-attested via identity-binding gossip topic |
| Transport-profile publication | iroh-gossip (`/elohim/transport-profile/2.0.0` topic) AND libp2p-gossipsub (same topic-id, BLAKE3-hashed) |
| Selection at call site | `peer_map` module in `elohim-storage` |

### Migration

The existing `2026-05-08-045024_cross_stack_peer_map` migration adds the bridge table. **Phase 11 graduates it** to the full schema above (more columns, capability_level), via a new migration `2026-05-09-XXXXXX_peer_transport_manifest`. Transition is mechanical (existing rows have `libp2p_peer_id` populated, `iroh_node_id` populated where known, `capability_level` defaulted to 5 for hubs / queried from device archetype for spokes).

## Cutover gate (revised)

Phase 11's existing prerequisites stand, with these spec-level additions:

1. **Backend wiring per the decision rule above** — each backend's iroh-side and libp2p-side implemented per plane verdict; cross-stack peer-map governs selection.
2. **HTTP route graduation** — `/api/v1/blob/{hash}` reads from `IrohBlobStore` for blobs registered as iroh-canonical; falls through to legacy `BlobStore` for libp2p-fallback peers.
3. **Genesis seeder rewrite** — writes to `IrohBlobStore` AND `BlobStore` during transition; canonical address is BLAKE3 post-cutover, SHA256 retained as alternate for libp2p fallback.
4. **Gossip topic broadcast wiring** — per-topic publish call sites route to **both** iroh-gossip and libp2p-gossipsub during transition; same topic-id (BLAKE3-hashed); same wire format. **Permanent post-cutover** for inventory + identity-binding + recovery topics (consumer-grade peers must receive).
5. **Recovery e2e** — full social-recovery flow runs over both stacks. The recovery-seed shares (per `project_socially_derived_security`) traverse whichever transport profile each peer supports.
6. **CI parity soak** — nightly run of every parity test for one week with zero divergences. **Permanent**, not transition-only.
7. **Alpha-cluster soak** — 6-peer cluster runs in dual-stack mode for one week. Both transports active; cross-stack peer-map governs selection.
8. **Latency stress** — 10k blob round-trips on iroh-canonical path; p99 ≤ libp2p baseline. (Already established at p50; revalidate post-Phase-11 wiring.)
9. **Consumer-grade soak** (NEW) — iroh on a real phone (Stage-4-lightweight) over cellular for one week; iroh on a Chromebook over school Wi-Fi; iroh through carrier-grade-NAT residential connection. **If iroh fails any of these, the affected plane stays libp2p-canonical for that device class permanently.**
10. **Self-hostable pkarr resolver in production** — see n0 mitigation Step 2.
11. **Rollback drill** — flip `TransportBackend` default to libp2p, run full alpha smoke, flip back. Document playbook.
12. **Column-drop migration** — `peer_blob_inventory.blob_hash` (SHA256) **stays** for libp2p-fallback peers. The column is NOT dropped post-cutover; both BLAKE3 and SHA256 remain populated for consumer-grade peers that fetch via libp2p-blob-protocol.

## What this spec deliberately does NOT settle

These are open questions explicitly outside this spec's scope:

- **Hub-internal protocol shape (Track 3) detailed wire format.** This spec names HTTP-over-WebSocket with doorway-shaped semantics; the concrete sub-protocol (frame format, auth handshake, stewardship-contract enforcement) is a sibling spec. Existing doorway HTTP routes are the starting point.
- **DwellingHub trait surface implementation.** Per `2026-05-02-elohim-hub-boundaries-design.md` and `2026-05-08-doorway-hub-edge-design.md`. This spec assumes the hub trait exists at Phase 11 wiring time; if it doesn't, the wiring wires through current `elohim-node` orchestration.
- **Wearable / IoT registration flow.** Track 3 endpoints exist; the registration UX (how a wearable announces itself to a dwelling hub, how stewards consent) is a separate epic.
- **CollectiveHub differentiation in transport defaults.** Sketched as "different attitude" in `2026-05-08-doorway-hub-edge-design.md`; concrete config defaults are a follow-up.
- **Browser-direct-WebRTC P2P participation surface.** libp2p-WebRTC is a real path; the user-facing surface for "advanced browser users want direct substrate" is undesigned. Most browsers will use Track 4 doorway.
- **Federation manifest schema for `discovery_resolvers`.** Sketched in §n0 mitigation Step 3; concrete schema is part of the federation manifest epic.
- **Reach-authorization wire-composition design.** Reach-authorization is canonical to the protocol — it's the elohim-mediated matchmaking surface that decides whether a peer is authorized to receive content under reach class (commons / regional / bioregional / municipal / neighborhood / local / invited / private). The integrity property the protocol commits to: **reach-gate decisions are anchored in DHT-notarized stewardship contracts and reach-class manifests**, never in transport-level claims. Today reach-authorization is an internal service consumed by the wire planes that carry data being authorized; how it composes across both transports while preserving DHT-derived integrity (especially as features like sponsorship contracts, witness-mediated upgrades, and reach-gate elohim discernment land) is in-progress feature design. Likely a sibling spec when the feature design completes. The transport question for reach-authorization is **NOT** "which wire is more secure" — both wires preserve DHT-derived integrity equally; the question is how the feature composes its wire participation cleanly across the two transports.

## Update to `elohim/elohim-storage/src/p2p_iroh/README.md`

A reference to this spec is added to the module README's "What works / What's next" section so future agents picking up Phase 11 backend wiring see the decision rule first.

## Decision rule summary (the one-liner)

> **iroh wins where it wins (hub-to-hub federation, BLAKE3-native blob); libp2p stays where consumer-grade-first-class agency lives (intermittent, UDP-restricted, browser-direct, transport-diversity hedge against discovery centralization); both selected at call-site by transport-profile manifest. Wearables and IoT bridge through dwelling hubs via HTTP/WS (Track 3), not substrate transport. Hub is a role, not a hardware tier — consumer-grade hardware (recycled laptops, gaming desktops, composed thin-client batches) acts as a hub when it's the only option available, with a graduation path to Tier-1-lightweight (Pi 4, NUC) and Tier-3 DwellingHub (full local AI inference) that preserves identity continuity. Every hub is orchestrated by an **elohim-operator** — a context-bound specialist agent that fills the household's devops/IT role, treating the hub's hardware as a cluster (whether that cluster is one recycled laptop or a Tier-3 family-node-extended) and continuously negotiating stewardship-vs-capacity tradeoffs across internal household needs and external federation commitments, bound by the substrate-floor / elohim-ceiling pattern and signed/witnessable/reversible by the household's stewards. Hubs federate horizontally; the protocol structurally prevents hub-as-datacenter via DHT-notarized stewardship contracts, federation reach-earning cost asymmetry, hardware-bounded consumer-grade-hub ceilings, and three independent paths for consumer-grade peers (Track 2 direct, Track 3 spoke, Track 4 doorway-projected). The protocol subsumes Cloudflare and FANG via federation density, not via a single hub at scale.**

## Memory anchors that load this spec

(See frontmatter `memory_anchors` for the full list. The load-bearing ones for this decision are:)

- `project_three_layer_truth_model` — DHT / libp2p-data-ops / doorway. Track 2 is the libp2p-data-ops pillar; this spec keeps it diverse.
- `project_substrate_scale_ceiling` — Tier-3 federation, but the substrate INCLUDES consumer-grade peers as first-class. This spec corrects the over-narrow reading.
- `project_hub_archetype_abstraction` — DwellingHub / CollectiveHub bounded by governance capacity. Anti-datacenter mechanism 5.
- `project_household_is_resilience_unit` — household-to-household resilience graph; consumer-grade peers participate via Track 2 OR Track 3.
- `project_reach_earned_at_authoring` + `project_social_reach_nervous_system` + `project_trust_as_efficiency_signal` — reach-earning cost asymmetry as anti-datacenter mechanism 2.
- `project_doorway_is_federation_surface_atproto` — Track 4's role; n0 dependency is structurally similar to ATProto's PDS dependency, mitigated the same way.
- `project_doorway_single_target_no_fanout` — Track 4 never fans out; doorway projects, substrate moves bytes.
- `project_iroh_parallel_stack_phases3_7_landed` — current state of the parallel stack; this spec is the cutover-decision input.
- `project_compute_and_model_independent_diversity_surfaces` — peer diversity has multiple axes; transport diversity is a similar property at this layer.
- `project_no_sovereignty_stewardship_over_ownership` — vocabulary; this spec uses steward / contributor / hub-carrier / agency.

## Sibling specs

- `2026-05-08-doorway-hub-edge-design.md` — companion spec on doorway/hub responsibilities; this spec extends it with the Track 1/2/3 transport decomposition.
- `2026-05-02-elohim-hub-boundaries-design.md` — Hub trait sketch.
- `2026-05-07-iroh-parallel-stack.md` — the executable plan that produced Phases 1–10; this spec is its Phase 11 architectural input.
- `2026-04-13-device-archetypes-design.md` — device archetype catalog; the matrix in this spec is keyed by it.
- `2026-04-10-agency-phase-registration-design.md` — graduated stewardship; consumer-grade direct participation requires the `device` and `node` agency phases.

---

## Status

**Draft, gating Phase 11 cutover work.** Bench expansion (`/deliver iroh-bench-expansion`) is the next downstream prompt; that work benches only planes this spec marks dual-stack or iroh-canonical, and skips libp2p-canonical planes.

**Next sessions:** Phase 11 backend wiring follows the decision rule in §Plane-by-plane verdict. Each backend's wiring commit references this spec. Any deviation (e.g., a backend the implementer believes should be iroh-only when this spec says dual-stack) MUST go through a spec amendment, not a silent backend choice.
