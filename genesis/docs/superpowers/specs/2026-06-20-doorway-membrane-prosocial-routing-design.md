---
title: "Doorway Membrane & Pro-Social Routing — the thin federated edge that absorbs web2 concerns, shields fragile peers, and lets the dataplane self-heal (a sibling arc consuming the Weave Epic substrate)"
id: doorway-membrane-prosocial-routing-design
status: Draft
class: protocol-canonical
domain: D8
topic: [doorway, membrane, edge, federation, multi-wan, cdn, ddos, dns, pkarr, peer-selection, serve-routing, self-heal, pro-social, mutual-aid, defense-in-depth, fail2ban, peer-fabric, crate-extraction, origin-cloaking, dataplane]
cites:
  - weave-epic-arc-design | the D5 substrate epic this arc CONSUMES not forks (#0 lens, #1 CoverageRollup, #2 tier-capability, #3 compute-contracts/recognition); sibling-arc placement | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - doorway-ssr-runtime | the canonical doorway-as-compute seed this refines; the capability-advertisement substrate the membrane extends | sha256:7f75b3027ae4f9d4 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
  - doorway/doorway-service/EDGE-DESIGN.md
  - landing-page-epr-dual-doorway | the unexecuted dual-federated-doorway plan this supersedes with the membrane + capability-routing + self-heal model | sha256:161bb449d50df804 | path: genesis/docs/superpowers/plans/2026-05-23-landing-page-epr-dual-doorway.md
  - pillar-epr-decomposition-design | the EPR projection the membrane caches/serves; projection-as-disposable (P1) is why a doorway can be untrusted/replaceable | sha256:3db7d2c205a0d7d6 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - che-network-agency-arc-design | hosted-session / sovereign-peer / delegated-agency + the delegates-compute bounds-gate = the identity-axis authority that travels when routing to a peer | sha256:d73e30ea0a205c13 | path: genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md
  - doorway-dispatch-registry-fallback-and-vocabulary | landmine: never reintroduce a prefix guard in the wildcard arm; new routing surfaces must be registry/manifest-driven classification, not prefix enumeration | sha256:8adde339010ac508 | path: genesis/docs/content/elohim-protocol/history/2026-06-02-doorway-dispatch-registry-fallback-and-vocabulary.md
  - conductor-agent-info-substrate-gossip | landmine: cross-WAN gossip propagation != remote projection; the deferred multi-bootstrap/signal-federation dependency that gates cross-WAN conductor routing | sha256:7ee98c749aadb58d | path: genesis/docs/content/elohim-protocol/history/2026-06-02-conductor-agent-info-substrate-gossip.md
  - dht-is-a-notary-not-a-byte-store | binding constraint: ban-table, score cache, acute backpressure, capacity = gossip+projection (Operational-C), NEVER a DHT entry | sha256:a1d408ef2478b288 | path: genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - elohim-facings-crate-extraction-plan | the pure-crate extraction pattern elohim-peer-fabric follows (deps-graph boundary enforcement, no-diesel, byte-identical gate) | sha256:d301f34b3b7e66d4 | path: genesis/docs/superpowers/plans/2026-06-19-elohim-facings-crate-extraction-plan.md
  - vision-gap-limit-governor-stub | home of the limit-governor that bounds the emergent market layer (limitarian ceilings + donut floor/ceiling); v1 ships mutual-aid recognition only | sha256:14ea8f3e81cd87c8 | path: genesis/docs/superpowers/plans/2026-06-14-vision-gap-limit-governor-stub.md
  - epr-reachability-economics | KEYSTONE — Role-2 finance-bridge, the immutable-bytes/mutable-heads CDN model, the participation/reachability gradient, the anycast-CDN endgame, and toll-settlement open questions; grounds the §3 CDN two-planes correction AND the §7.1 toll layer | sha256:19e359867f22af5a | path: genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md
  - mutual-storage-replication-dwelling-hub-design | commons floor (COMMONS_MIN_FLOOR_PCT), free→dwelling→collective→commons graduation, proposed→active on first ProvideAnnounce — the commons-pool + opt-in-hosting backing for §7.1 | sha256:1acbeeec8b7a3956 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - non-commons-provide-commitments-design | the replicates-content/commons capacity-variant rationale — the commons-pool membership / opt-in hosting backing (§4·7) | sha256:936b660644fde390 | path: genesis/docs/superpowers/specs/2026-06-13-non-commons-provide-commitments-design.md
  - genesis/data/timeline/backlog/witnessed-records-reach-flywheel.md
  - doorway-two-axis-scaling | doorway DNS/TLS-gateway self-description + "the K8s expression is the developer test-bench, not the architecture" — grounds the doorway-as-P2P-ingress reframe (§1) | sha256:36fb15e24ceaf8b2 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-doorway-two-axis-scaling.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
  - doorway/doorway-service/EDGE-DESIGN.md
  - genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
refines: genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
# Mixed-env arc (CLAUDE.md scope convention): NO doc-level requires_env so the household-nodes-testable
# spine (the elohim-peer-fabric crate, the membrane policy stage, serve-routing on the local mesh,
# the self-heal loop) stays fair-game. The cross-WAN legs are tagged inline (@requires:shem /
# @requires:alpha-cluster-6peer) and are DESIGN-ONLY: cross-WAN reroute, origin-cloaking under real
# WAN hostility, anycast/scrubbing, :53/DNS-delegation, cross-region council rollup.
---

# Doorway Membrane & Pro-Social Routing — arc design

> **One line:** doorway becomes the **thin web2-facing membrane** that absorbs the hostile/legacy
> internet (TLS, CDN read-mass, L7 abuse, name-resolution, origin-cloaking) so the **fragile sovereign
> peers** behind it never face WAN hostility directly — and behind the membrane the peers form a
> **self-healing pro-social dataplane** that advertises capability + headroom, routes load to peers
> that can help, and recognizes the help as mutual aid. This is a **sibling arc** to the Weave Epic:
> it *consumes* Weave's capacity/capability/compute-contract substrate and adds only the edge membrane,
> a shared defense-in-depth crate, the routing/self-heal consumer, and the WAN/DNS/cloaking edge.

## Provenance & binding context

Surfaced 2026-06-20 from the operator's question "how do we get to multi-WAN federated host-routing —
doorways acting as CDN, failover, loadbalance-vs-DDoS over WAN, and acting as a DNS server themselves?"
The brainstorm reframed the web2 vocabulary onto the protocol grain. The operator's thesis, captured
verbatim as binding context:

- **Doorway is a thin federated membrane.** The fragile sovereign core is the peers/conductors (the
  content-addressed DHT/EPR truth). Doorway is the hardened web2 edge that **absorbs web2.0 concerns**
  — DNS, TLS, CDN load, volumetric DDoS, bot/abuse traffic, loadbalancing — so fragile peers never
  face that hostility directly. "Mixed by design": web2 *mechanisms* live AT the membrane; the
  substrate-native grain lives BEHIND it. The seam IS the doorway.
- **Defense-in-depth is a node capability, not a doorway feature.** Because the protocol rule is
  "served THROUGH not BY" (a peer *can* be hit directly), the protective muscle — fail2ban-style
  ban/rate-shape, abuse detection, admission, challenge, reputation — must be **written once as a
  shared crate** consumed by both `doorway-service` and `elohim-storage`, driven feature-gated per
  node role. Shared, reusable defense is what makes a helping peer *safe* to take redirected traffic.
- **Thin clients (Chromebooks) are first-class.** "Thin" describes the *consumer device*. The doorway
  is the muscle that lets a Chromebook participate without running the stack; the hub-optional floor
  still holds (a laptop *can* be a full peer/doorway). Capability advertisement is how a thin client
  finds capable peers; the dataplane **self-negotiates how traffic flows** and **quickly surfaces a
  peer that needs help**, encouraging other peers to absorb load. This is a **pro-social compute
  network**.
- **Mutual aid first; markets emergent and bounded.** Recognition defaults to non-transactional
  commons-standing (you help because it sustains a rich robust commons; reach/standing is *earned*,
  not invoiced). A market *may* emerge on top via REA/mutual-credit, but it is **subordinate to and
  capped by** human flourishing within the limits of the whole — **limitarianism** (ceilings on
  accumulation), **donut economics** (between a social floor and a whole-system ceiling),
  **consequentialist libertarianism** (voluntary means, judged by flourishing outcomes). v1 ships
  mutual-aid recognition only; the market layer is specified as a limit-governed extension.

This arc passed the **P2P design gate** (§4 below): across its new entities the only DHT writes reuse
existing entry types — **zero new DHT entry types**, mirroring the Weave Epic's verdict.

## Map — what is new vs what is consumed

| Piece | Verdict | Home / composes-with |
|---|---|---|
| `elohim-peer-fabric` crate (`guard` + `score`) | **NEW** | new `elohim/elohim-peer-fabric`, the `elohim-facings` pure-crate pattern |
| Doorway membrane policy stage | **NEW** | `doorway/doorway-service` `handle_request`; D8; EDGE-DESIGN seam |
| Acute backpressure signal (1b) | **NEW** | transport-plane (libp2p/SSE); Operational-C, never DHT |
| Serve-routing consumer (`score.rank` at serve time) | **NEW** | `elohim-storage` `services/distribution_view.rs`; D1 |
| WAN / DNS / origin-cloaking edge | **NEW (design-only)** | EDGE-DESIGN three-altitude seam; held until shem |
| Recognition / accounting | **CONSUME** | Weave **#3** `compute-fulfilled` EconomicEvent + `appreciation` |
| `score` capability input | **CONSUME** | Weave **#2** `TierCapabilityView` |
| `score` headroom/load input | **CONSUME** | Weave **#0** operational-weave lens + `NodeHeartbeat` |
| `score` hub capacity rollup | **CONSUME** | Weave **#1** `CoverageRollup` |
| Byte movement behind the membrane | **REUSE** | `sharding.rs` / `shard_service.rs` / `blob_store.rs` |

## 1. The membrane model & the seam

Doorway is a **thin, replaceable, web2-facing edge** that projects content-addressed truth to the
traditional internet and shields the fragile peers from WAN hostility. The tightest analog is the
**IPFS trustless gateway** (a gateway returns only verifiable bytes, so the client needs zero trust in
it — exactly why a doorway can be untrusted, replaceable, and *many-not-one*); the secondary analog is
**Bluesky's Relay/AppView** (the sovereign PDS is never the read surface; a projection tier absorbs
read fan-out). Cross-cutting lesson (Matrix's *abandoned* sliding-sync proxy): a separate edge tier
survives only if it is operationally cheap — EDGE-DESIGN's "runs on a single blade" is the right
instinct. **Design for many doorways from day one.**

The seam (reconciles the binding thesis with EDGE-DESIGN's three altitudes — doorway-resident / hub /
operator-deployment). "Doorway absorbs DDoS" means *terminate the application flood and escalate the
volumetric concern upward*, not *implement anti-DDoS*:

| Web2 concern | Where it lives | What passes to the substrate |
|---|---|---|
| DNS / name-resolution | **Delegated upstream** (managed anycast) for `elohim.host`; **pkarr behind the membrane** for key→records. Doorway does NOT answer `:53`. | A *doorway* address, never a peer address. The signature (pkarr) is the truth anchor, not the DNS record. |
| TLS | **AT doorway** (self-terminated, cert-manager/ACME — under sovereign control, unlike Cloudflare Tunnel). | Plaintext stops at the edge. |
| CDN — bytes | **AT doorway** (`TieredBlobCache` pantry + `DeliveryRelay` coalescing + write-on-fetch). CID-authoritative + immutable ⇒ cached forever, no invalidation. | Only cold/uncached, single-target fetches reach a peer; coalescing kills the thundering-herd. |
| CDN — projection / head index | **AT doorway, REPLICATED doorway↔doorway** — the mutable name→latest-CID head + the "which doorway projects this EPR" index (reach-earned; informs an EPR/client *which* doorway to reach). | Heads visible at the edge **even for peers without a doorway contract**; only *serving load* is conductor-gated. Bytes still come from the CID — projection replication never makes doorway a byte target-chooser. |
| Volumetric DDoS (L3/L4) | **UPSTREAM** (anycast scrubbing / BGP / CDN headroom). By the time a packet hits doorway's socket the uplink is already full. | Nothing volumetric reaches the peer; the doorway is the first casualty, not the peer. |
| Application abuse (L7) | **AT doorway** (`guard`: admission shed, circuit breakers, PoW/challenge, reputation rate-shape). | Only admitted, cost-paid, reputation-cleared requests pass. |
| Loadbalance | **Split**: L7 + health-aware conductor routing AT doorway (identity axis); GSLB/anycast upstream. | For *bytes*, no balancing crosses the seam — single-target dispatch (no fan-out). |
| Origin-cloaking of peer addresses | **AT doorway** — the highest-leverage shield. Peers connect outbound; the proxy is the only egress; backend targets are private cluster DNS. | A WAN client never receives a dialable peer address. |

**Doorway IS the P2P-native ingress controller.** The L7/control plane — ingress, L7 load-balancing,
service discovery, health-aware routing, self-heal — *is* doorway: the operator's **"k8s-like powers
over a true P2P substrate."** The k8s `Ingress`/ingress-nginx currently in front of it
(`doorway/prod.yaml`) is the **developer test-bench, not the architecture**
(`2026-06-11-doorway-two-axis-scaling`: "the K8s expression … is the developer test-bench, not the
architecture … the model must hold for three blades in a closet"; `feedback_k8s_is_not_the_architecture`:
"at full protocol maturity it goes away completely, subsumed into peer-native modeling"). This **extends
P1** (storage = reconciliation-controller over the DHT manifest) from the *data* plane to the
*ingress/control* plane — the new claim of this arc. **The reframe is the L7/control plane ONLY** — the
L3/L4 volumetric / anycast / `:53`-delegation seam above stays exactly as stated (upstream /
operator-altitude; EDGE-DESIGN keeps eBPF/XDP/BGP-anycast at the hub + operator-deployment tier).

## 2. The shared spine — `elohim/elohim-peer-fabric`

The operational spine the operator chose to build first. It follows the **`elohim-facings` extraction
pattern** (landed 2026-06-19): a **pure-logic crate**, deps minimal (`std`/`serde`), **no diesel**.
The boundary is enforced *by the dependency graph* — impure (DB-touching) code won't compile against
it, the same mechanism facings proved. Both runtimes implement its traits; the impure adapters stay in
the runtimes.

- **`guard` module** — defense-in-depth. `guard.assess(source, ctx) -> Verdict` where
  `Verdict = Allow | Shape(delay) | Challenge(pow) | Deny`. Owns ban-table, rate counters, local
  reputation. *Depends on:* `GuardStore`, `Clock`, `MetricsSink` traits. `source = agent_cid` when
  authenticated, else a transport id (never raw-compared across namespaces — see §5).
- **`score` module** — peer ranking for diversity/selection. `score.rank(need, candidates) ->
  Vec<ScoredPeer>`, a **pure composer** over Weave outputs: capability (Weave #2 `TierCapabilityView`)
  × live headroom (Weave #0 + `NodeHeartbeat`) × **attested RTT/reachability** (`HealthAttestation`,
  BFT) × bonded capacity (`Mishpat` commitments) × household fault-domain diversity × **decaying
  delivery-success** (derived from Weave #3 `compute-fulfilled` events). *Depends on:*
  `OperationalView`, `Clock` traits. Degrades gracefully: missing Weave #2 ⇒ rank on
  `NodeHeartbeat`+`HealthAttestation` only, gaining the tier signal when #2 lands.

**Cargo features = node role.** `doorway-service` enables `edge-defense`; `elohim-storage` enables
`serve-routing` + `peer-defense`. A node running both shares one implementation and one ban/score
semantics — no fork, no drift. The crate is the write-once home the operator specified.

**Crate boundaries (the five units):**
1. `elohim-peer-fabric` (above) — pure logic.
2. **Doorway integration (D8 edge):** a membrane policy stage in `handle_request`, between the wisdom
   gate and the admission semaphore, calling `guard.assess` and applying the `Verdict` *before any
   peer is touched*; reuses `admission_exempt` for the bypass set. Conductor identity-axis routing
   stays in `conductor/registry.rs` (left as-is for v1; "unify its scoring onto `score`" is a
   fast-follow).
3. **Storage integration (D1 peer):** applies `guard.assess` on directly-exposed surfaces (peers need
   the muscle too); extends `services/distribution_view.rs` placement selector to use `score.rank` at
   **serve** time. Backs `OperationalView`/`GuardStore` with SQLite.
4. **Self-heal loop driver:** watches local `NodeHeartbeat` (`current_load`/`status`); on distress sets
   the *existing* `degraded` status + raises `current_load` (no new vocab — see §4·1a) and emits the
   acute backpressure signal (1b, transport-scoped, *not* broadcast); the `score` layer routes *new*
   load to headroom peers; the helping peer's serve is metered.
5. **Recognition** = Weave #3 (consume, not re-spec): per-serve metering (Operational-C) → epoch
   rollup → `compute-fulfilled` EconomicEvent + `appreciation`, joined on `agent_cid`.

## 3. Per-capability verdict (the operator's four + the meta-cap)

- **CDN / projection cache — two planes, split by mutability** (refinement, not a reversal). **(a)
  Bytes:** CID-authoritative + immutable ⇒ cached at the edge forever, no invalidation; serving is
  single-target / no-fan-out (gospel unchanged); *residual:* trustless-gateway CID re-verify **on
  cache-fill** (not per-serve). **(b) Projection / head / addressing layer:** the mutable
  name→latest-CID head + the "which doorway projects this EPR" index — **this** replicates
  cache-to-cache (doorway↔doorway), is **reach-earned**, and informs an EPR/client which doorway to
  reach. Authoritative bytes still come only from the CID; projection replication never makes doorway a
  byte target-chooser. Canon: `epr-reachability-economics` §6–§7 ("immutable bytes cached eternally;
  mutable heads kept fresh cheaply"; "origin = the replicated P2P substrate"). **One principle (this
  spec's synthesizing contribution — assembled from canon, not yet stated as one anywhere):** *EPR heads
  from peers without a doorway contract are visible at the edge (head-visibility filter `epr_head.rs`);
  only serving load is conductor-gated (`epr_service.rs`/`http.rs` reach-gate + universal `/epr/{id}`
  resolver); projection replication is earned by reach.* **GAP:** the doorway↔doorway projection-index
  *replication mechanism* is designed-for, not built (`compute_projection_tier` only *describes* observed
  coverage).
- **Failover** — doorway-level edge failover on the **identity-hosting axis** (which conductor is
  healthy for a human), backed by peers registering with multiple doorways. For **bytes**, failover is
  substrate re-resolution (Kademlia / `epr-cross-peer-resolution`), **never** a doorway target-chooser
  (that is forbidden fan-out).
- **Loadbalance vs DDoS** — doorway owns L7 entirely (admission shed + `Retry-After`, circuit
  breakers, origin-cloaking, PoW/challenge, reputation-keyed rate-shape *per agent-key, not per-IP*).
  Volumetric L3/L4 is **upstream**. *Irreducible residual:* a flood exceeding the doorway's own uplink
  takes the doorway down (shared-fate); a lone self-hosted node with no upstream cannot close
  volumetric resistance in-app — the mesh redistributes *application* load, not line-saturation.
- **Doorway-as-DNS** — **delegate, do not run `:53`** (a self-run authoritative/recursive server is a
  UDP amplification/reflection vector — the exact volumetric surface the thesis keeps off fragile
  peers). Managed anycast + GeoDNS + health-check failover answers `elohim.host`; split-horizon returns
  the local doorway on-LAN. Run **pkarr behind the membrane** as the substrate-native, censorship-
  resistant peer-addressing layer (already implemented). *Residual:* a managed zone is a centralized
  takedown lever — pkarr is the sovereign fallback; name truth anchors in the CID/signature, never in
  the DNS record.
- **Route a conductor-agent request to a capable peer (meta-cap)** — the seam-honest split:
  capability→peer **selection logic is substrate-native (D1)** (extend `distribution_view.rs` from
  placement to serve-routing); doorway carries capability routing only on the **identity-hosting axis
  (D8)** (the existing `conductor/registry.rs`). Conflating the two — a doorway-resident *byte*
  target-chooser — is the one move that breaks the constitution.
- **Reach-earned projection (the gate)** — projection replication is gated by **stewardship-backed
  reach**: broader *earned* reach ⇒ more doorways project the EPR, where "earned" = backed by
  `replicates-content`/`replicates-commons` Mishpat commitments accumulated from distributed stewards
  (doctrine: `witnessed-records-reach-flywheel` — "each steward connection elevates earned reach →
  blobs shard onto that reach-level"). The **byte analog exists** (`replica_target_for(reach)`:
  Private=2 … Public=16, `distribution_view.rs:58`). **GAP / the named seam:** there is no
  `projection_target_for(reach)` yet — `compute_projection_tier` is *descriptive* (counts observed
  projectors), and the reach-aware target is an explicit `TODO(reach-aware-targeting)`
  (`distribution_view.rs:501`). `ProjectionTier` is the correct home.

## 4. P2P Design Gate output

**Zero new DHT entry types.** Verdicts:

- **1a · Sustained headroom/load advertisement** — Operational/A, **reuse `NodeHeartbeat`**
  (`current_load`, `active_connections`, `status`, 30s). **No vocab change, no DNA-hash event:** the
  distress state reuses the *existing* `degraded` status ("Running but with reduced capacity") + a
  `current_load` threshold, derived Operational-C as `(status == degraded ∨ current_load > θ)`.
  (Verified: `NODE_STATUS` is an advisory `const [&str; 4]= [online, maintenance, degraded, offline]`;
  `status` is a free `String` **not** enforced in the integrity zome — so even a future value would not
  move the DNA hash, but we need none.) Address: agent-scoped composite (node↔agent). Source of truth:
  DHT. Projection: heartbeat tables (`dht_anchor_hash`: yes).
- **1b · Acute backpressure** — Operational-C, **transport-plane** (libp2p gossip / SSE), **never DHT**
  (200–2000ms gossip too slow; ephemeral; a public "kick-me" beacon is a privacy/targeting leak).
  Reconstructable from heartbeat + local metrics. It is an *optimization over the 30s heartbeat floor*,
  never a correctness dependency.
- **2 · Capability-aware serve-routing** — *not a data entity*; selection **logic** consuming existing
  entities. Only persisted artifact = a score/RTT-rollup cache → Operational-C, reconstructable.
- **3 · Help-given recognition** — raw per-serve metering → Operational-C; aggregated recognition →
  **REA `EconomicEvent`** (Weave #3, existing type), minted per epoch/threshold. **Never notarize
  per-serve** (granular-data-on-DHT anti-pattern); provider keyed on `agent_cid`.
- **4 · Shared defense-in-depth state** — Operational-C (ban-table, counters, local reputation);
  ephemeral, local, reconstructable. Cross-peer threat-intel deferred (opt-in Operational-C gossip if
  added; not a new DHT type in v1).
- **5 · Membrane policy layer** — *not a data entity*; a request stage reusing existing admission /
  circuit-breaker operational state.
- **6 · Toll receipt** (design-only) — **REUSE** REA `EconomicEvent` (fiat is another resource flow);
  per-toll detail stays Operational-C, **epoch-aggregated** (never notarize per-toll/per-serve). Joined
  on `agent_cid`.
- **7 · Commons-pool membership / opt-in hosting offer** (design-only) — **REUSE** `Mishpat::Commitment`
  `replicates-content`/`replicates-commons` *capacity* variant (verified `commitments.rs:294`);
  graduates `proposed → active` on first `ProvideAnnounce`. No new entry type.

**Design constraints discovered.** (a) Reuse over create — `NodeRegistration`, `NodeHeartbeat`,
`HealthAttestation`, `ShardAssignment`, `Mishpat::Commitment` (`delegates-compute`,
`replicates-content`/capacity), REA `EconomicEvent` all exist; gaps are consumers + Operational-C
layers + epoch aggregation. (b) Sybil-resistance partly exists: health/RTT are **peer-attested (BFT)**
via `HealthAttestation`; capacity is **bonded** via Mishpat. Only `NodeRegistration.region` is
self-declared → treat as a hint, weight on attested RTT. (c) **Economic attribution joins on
`agent_cid` directly — NOT the `AgentPeerBinding`** (it is `STAGE1_SIGNATURE_SENTINEL`,
self-asserted/unsigned; forbidden for economic attribution until a cross-signed proof lands).
(d) DHT-is-a-notary: ban-table, score cache, acute backpressure stay operational (gossip+projection),
never DHT.

## 5. Data flow

- **Read (happy path):** DNS → ingress *(k8s test-bench; at maturity doorway IS the ingress controller)* → **doorway:8080** (client never sees a peer address) → wisdom
  gate → **membrane policy stage** (`guard.assess`; anything but `Allow` ⇒ no peer touched) →
  admission semaphore (503 shed) → EPR router (cache hit ⇒ served at the edge, peer untouched; miss ⇒
  single-target fetch) → trustless-gateway CID re-verify **on cache-fill**.
- **Conductor-agent (identity axis, D8):** `conductor/registry.rs` selects a healthy conductor for the
  agent; authority travels as the agent credential; the `delegates-compute` bounds-gate enforces scope.
- **Serve-routing (byte axis, D1):** storage lacking bytes calls `score.rank(need = content:{reach},
  candidates)` → capable × live × low-attested-RTT × bonded × diverse peer; **storage chooses, not
  doorway**; content-addressed fetch verifies the CID.
- **Distress / self-heal:** peer P's load climbs → P sets `NodeHeartbeat.status → degraded` (existing
  value) + raises `current_load` + emits acute backpressure (scoped to current routers) → doorway +
  siblings' `score` route *new* load to headroom peers Q/R (in-flight on P untouched) → Q's serve
  metered → epoch rollup → `compute-fulfilled` EconomicEvent + `appreciation` recognizes Q (mutual aid);
  Q's decaying-score rises ⇒ preferred next time; advertise-then-drop ⇒ score decays +
  `HealthAttestation{success:false}` from attesters (the commons self-polices without a central judge) →
  recovery: P's load falls → `status → online`, re-enters the candidate pool.

## 6. Error handling & degradation

- **`guard` infra failure** → fail toward availability for legit users, never toward unbounded peer
  exposure: lose fine-grained banning, keep the admission semaphore (503) as the coarse floor.
- **`score` degradation (layered):** missing Weave #2 ⇒ heartbeat+health only; **all candidates
  saturated** ⇒ no fan-out — cache (stale-but-CID-verified, marked) → single-target → honest 503
  `Retry-After` (the membrane is the casualty, not a peer stampede).
- **Selected peer dies mid-fetch** → substrate re-resolution, not a doorway retry-fan-out.
- **Partition behavior, split by axis:** the content/projection axis is stateless + content-addressed ⇒
  **partition-safe** (minority-partition doorway serves stale-but-verified; heal-on-read on reconnect).
  The conductor *custody* axis is **stateful ⇒ the dangerous case** (the single-pin `signal_url`
  landmine) — exactly why cross-WAN conductor routing is design-only; v1 keeps it inside the household
  mesh, cross-WAN identity failover deferred with the multi-bootstrap dependency named.

**Residual-risk boundaries (explicit, not hand-waved):** volumetric L3/L4 (above); **key
rotation/revocation across federated doorways+peers** (JWKS/DID/pkarr revocation-propagation latency —
named open problem, flagged follow-on); **membrane-as-chokepoint** mitigated by many-doorways + the
"served THROUGH not BY" escape hatch (a sovereign node may run its own doorway or be reached directly;
doorway is convenience, never a mandatory hop — the hub-optional floor); **privacy** (the
capability/heartbeat discovery map is public-DHT by accepted design; the acute distress signal stays
scoped, not broadcast).

## 7. Economic / incentive layer

Recognition is **mutual-aid-first**: helping is how reach/standing is earned (commons-standing), not
invoiced — routed through Weave #3's `appreciation` EconomicEvent and the `delegates-compute`
bounds-gate. A **market may emerge** on top via REA/mutual-credit, but it is **subordinate to and
governed by** the limits of the whole: limitarian ceilings, donut floor-and-ceiling, consequentialist
evaluation by flourishing. The market layer is **specified, not built in v1**, and is gated by the
`limit-governor` (the `2026-06-14-vision-gap-limit-governor-stub` is its home). v1 ships recognition
only; the decaying delivery-success score closes the pro-social loop (help → standing → preferred
selection) without any market.

### 7.1 Fiat-interop, the commons toll, and traffic-reward commons pools (design-only; eventual; limit-governed; a `bridges/` crate)

The doorway's web2 surface eventually hosts a **finance-bridge** (Role 2 of `epr-reachability-economics`).
Non-stewarding clients — search engines, bots, browsers with no standing and no projection contract —
reach content lacking an explicit projection contract by paying a **toll**: a micro-transaction that
internalizes the externality web2 hides ("visitors pay rent; stewards grow the commons" — *not* a
paywall, "honest pricing of an externality"). Tolls flow as **traffic rewards** to small peers who
**opt-in to host cached EPRs**, building **commons pools** (the `replicates-commons` capacity pledge +
the `COMMONS_MIN_FLOOR_PCT` floor; graduates `proposed → active` on first `ProvideAnnounce`). This is the
*bounded, emergent* market — **subordinate to and capped by** the `limit-governor` (limitarian ceiling +
donut floor/ceiling). **Heads visible / bytes metered:** commons heads stay visible to anonymous
visitors; the toll meters only *bytes*; the economic frontier is enforced **peer-side** (the serving
peer's standing check), never as a doorway-resident gatekeeper.

**It is a `bridges/` crate, not core doorway** — canon is explicit (`epr-reachability-economics`: "the
finance-bridge belongs at the doorway web2 surface, as a bridge"). `bridges/fiat` plugs into the membrane
exactly as `atproto`/`activitypub`/`valueflows` do, keeping the membrane thin (this is the operator's
"bridge crates keep core doorway clean"). The positive complement of EDGE-DESIGN's "a pattern shaped like
a DDoS is structurally *unearned-reach* compute": non-stewarding traffic lacks standing, so it pays the
externality to earn the cycles it consumes. **v1 ships nothing here** — mutual-aid recognition (§7) is
the only economic layer that ships. **Gate:** toll receipt = REA `EconomicEvent` (§4·6); commons-pool /
opt-in hosting = `replicates-*` capacity Commitment (§4·7) — zero new DHT entry types. **Named
fast-follow:** a `Toll`/`Pay` variant of `guard.assess`'s `Verdict` is the natural enforcement extension
(not yet specified). **GAP:** no `bridges/fiat` crate exists today (`bridges/` holds `valueflows` + the
web2 bridges); the chain *toll → traffic-reward → opt-in hosting → commons pool* is a coherent
composition of existing primitives, not yet written as one flow.

## 8. Scope — D8 / D1 / D5 and buildable-now vs design-only

- **Domain center:** D8 (Web2 Projection & Doorway — the membrane). The serve-routing brain is D1
  (EPR cross-peer travel). The capacity/capability/recognition substrate is **consumed from D5** (the
  Weave Epic).
- **Buildable-now (household-nodes, M/J/J mesh):** the `elohim-peer-fabric` crate (`guard` + `score`
  logic), the membrane policy stage, serve-routing on local peers, the self-heal loop, recognition
  epoch rollups joined on `agent_cid`. Existing a2o (`doorway-pool-degrade`, `peer-loss-failover`,
  `peer-recovery`, capacity-heartbeat) already make several of these acceptance contracts.
- **Design-only (held until shem / operator-owned):** cross-WAN reroute, origin-cloaking under real
  WAN hostility, anycast/scrubbing, `:53`/DNS-delegation, cross-region council rollup (Weave #1 needs
  shem). `@requires:shem` / `@requires:alpha-cluster-6peer` tagged at those gaps.

## 9. Testing

- **DB-free pure-crate proofs (Slice-0, the facings pattern):** `guard` verdict matrix + ban-decay +
  reputation over hand-built histories; `score` ranking order + degradation + all-saturated→shed;
  self-heal heartbeat-status→reroute logic.
- **Integration (M/J/J):** membrane policy stage in `handle_request` (+ an `is_service_path`-style unit
  test so the stage can't be shadowed — the `/auth/portal` incident shape); serve-routing reroute on a
  downed local peer; an epoch rollup emitting a `compute-fulfilled` EconomicEvent joined on `agent_cid`.
- **New a2o scenarios (story-first):** *pro-social self-heal* (distress → redistribute → recognize);
  *membrane shields fragile peer* (abusive source banned at the edge → peer request count stays 0);
  *thin client leans on a capable peer* (Chromebook-shaped client served via a peer it never picked).

## 10. Forks (each re-confirmed at `/plan`)

| Fork | Recommendation | Locked? |
|---|---|---|
| Arc placement: sibling vs Weave #5 | sibling arc citing Weave (distinct seed: membrane/edge vs operational-weave lens) | locked (operator) |
| Spine first-deliverable | shared operational spine (`elohim-peer-fabric`) first | locked (operator) |
| `elohim-peer-fabric` shape | one crate, two modules (`guard`+`score`) | locked (operator) |
| Conductor router refactor onto `score` | leave as-is for v1; unify as fast-follow | locked (operator) |
| `score` as pure composer over Weave #2/#0/#1 | yes, with graceful degradation | locked (operator) |
| Cross-peer threat-intel sharing | deferred (local-first v1) | per-plan |
| Market layer / limit-governor coupling | specify only; build deferred behind the limit-governor | per-plan |

## 11. Non-goals

- This arc does not re-spec Weave's capacity/capability/compute-contract/recognition substrate — it
  consumes #0/#1/#2/#3 and only adds the membrane + `guard` + routing-consumer + edge.
- **No new DHT entry type.** Any proposal to notarize a distress signal, a routing decision, a
  ban-table, a per-serve event, or a capability/score on the DHT is an anti-pattern this arc
  forecloses (Operational-C + epoch-aggregated recognition only).
- Doorway must never become a **byte** target-chooser (single-target/no-fanout gospel); peer selection
  for content lives in storage (D1).
- Live-WAN behavior, authoritative `:53`, anycast/scrubbing, and origin-cloaking under real hostility
  are design-only here; they are operator-owned and held until shem.
- This index does not implement anything — the spine gets its own `/plan` → sprint cycle.
