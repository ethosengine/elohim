---
title: "Peer Discovery as Fractal Federation — the anti-capture composition for the discovery plane"
id: peer-discovery-fractal-federation
status: vision
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete OR superseded-by-implementation
created: 2026-07-09
maintainers: Matthew Dowell + Opus 4.8
cites:
  - genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md | The landed bootstrap-store sharing (MongoK2Store on the fixed elohim-bootstrap DB) — the endpoint↔backing decoupling this generalizes
  - genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md | The notary/trust plane whose green signal is DHT-membership-blind without the discovery backing
  - genesis/data/timeline/backlog/dna-conductor-dht-gossip-gap.md | The gossip layer beneath discovery — Bootstrap overloaded, dropping put
---

# Peer Discovery as Fractal Federation — the anti-capture composition for the Elohim Protocol's discovery plane

**Status:** DESIGN SEED (vision-tier) · 2026-07-09
**Scope:** the peer-discovery plane — bootstrap + signal servers, DHT membership topology, and how discovery composes upward into governance. NOT a build plan; it is the frame the build plans must serve.
**Composes with:** `plans/2026-06-14-federation-bootstrap-plan.md` (F-BOOTSTRAP — the landed bootstrap-store sharing), `backlog/dna-conductor-dht-gossip-gap.md` (the gossip layer), `specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md` (the notary/trust plane), the seam-map concern-routing atlas, and the memory threads [[project_earned_reach_governance_pr_ceremony_vision]], [[project_hub_optional_floor]], [[feedback-identity-sovereignty-ontology-guard]], [[feedback-justice-mishpat-not-punishment-guard]].

---

## 1. The reframe: peer discovery is not plumbing, it is governance topology

The naive framing — "our two conductors ended up on different bootstrap/signal servers; make them share one so they converge" — is a trap. Taken to its conclusion it says *put everyone on one shared discovery backend → one global flat DHT*. That heals the immediate partition and **dissolves the three things the protocol exists to protect.** The discovery topology is not plumbing beneath the architecture; it **is** the architecture's shape. It must *mirror the fractal governance topology, not flatten it.*

The governing principle:

> **The peer-discovery / DHT / signal topology must encode the holarchy — bounded human-centric local domains, a plural elohim aggregation plane above them, and earned+revocable reach as the membrane between — never one flat global mesh.**

## 2. What the substrate taught us (the diagnosis that forced the reframe)

Live evidence, 2026-07-09: the two genesis-commons conductors (adam, matthew) serve `elohim-host-landing` — the protocol's own public face — as `trust:"notarized"` on **both**, but over **divergent** `dhtAnchorHash` + `blobHash`. adam reports **2081 divergent anchors**. Root cause (operator investigation):

- **Identical DNA hashes** (5 shared DNAs, byte-identical) — so "same DNA hash = same DHT" is FALSE.
- **Different bootstrap+signal clouds**: adam → `elohim.host/bootstrap` + `signal.elohim.host`; matthew → `doorway-alpha.elohim.host/bootstrap` + `signal.doorway-alpha.elohim.host`. Two non-overlapping kitsune peer sets → two effectively partitioned DHTs.
- Bootstrap was *already* made shareable (F-BOOTSTRAP's `MongoK2Store` on the fixed `elohim-bootstrap` DB — "the genesis-pair islanding fix"), so the two conductors already discover each other. **The partition is now at *signal***: the SBD relay is a per-pod in-memory service with no shared backend, so the peers *find* each other but cannot complete the WebRTC handshake across two separate relays.

Two lessons the diagnosis burns in:
1. **DHT identity for the trust signal is `dnaHash` + the discovery *backing* (bootstrap table + signal relay), not the DNA hash alone.** "Green" today is DHT-membership-blind: two greens on two DHTs read identical. The trust signal is incomplete without the backing.
2. **This is the *conductor's* kitsune plane, not storage's libp2p mesh** (which is connected — peerCount 5). Do not conflate the two; the partition lives only in kitsune discovery.

## 3. The three planes (the recursive composition)

| Plane | Who | What discovery means here | Anti-corruption *physics* |
|---|---|---|---|
| **Ground** — Holochain DHT | humans | local, source-chain-signed, human-scale participation | **physics-of-participation**: it cannot scale → cannot accumulate → the reach that flows *out* is high-trust *by construction* (real, present, cares — Sybil-resistance by human-scale friction, not proof-of-work) |
| **Aggregation / council** | elohim (plural) | attenuated signal summarized *up* the holarchy; coherence held across domains | **physics-of-plurality-revocability-legibility**: authority is a revocable delegation *from below*, checked by peer elohim, witnessed (El Roi) |
| **Bridge** — doorways | web2 ↔ p2p | a transitional gateway from where-the-world-is (web2) to the resilient p2p substrate | **interchangeable convenience**: a doorway that *gates* participation is a capture smell ([[project_hub_optional_floor]]) |

The **immutable-blockchain-constitution** was the *prior conception* of the aggregation plane — governing power without a corruptible sovereign via *trustlessness* (code-is-law, immutable, no human in the loop). It is superseded here: trustless is inhuman (no context, no mercy, no [[feedback-justice-mishpat-not-punishment-guard|Mishpat]]) and does not remove capture, only relocates it to whoever writes/forks the code. The council plane is the evolved answer: **living legitimacy from below** (witnessed + revocable + plural) instead of **frozen law from above**. It trades tamper-*proof* (rigid, inhuman) for tamper-*evident and self-correcting* (humane): not "no one can ever change it," but "no one can change it *unseen*, and the ground can *undo* it."

## 4. Why NOT one flat global DHT (three hazards, each fatal)

1. **Holochain performance / arc physics.** Holochain's DHT is arc-based: each node holds an arc of one flat address space, `RAM ∝ corpus` ([[project_per_node_memory_is_conductor_authority_arc]] — the james OOM). A global shared backend stretches every node's arc over a *globally growing* corpus — it removes the only natural bound on working-set. **Bounded per-domain DHTs are the performance answer; flat is its opposite.**
2. **Beer's VSM / holarchy.** The Viable System Model is *recursive* — viable systems nested in viable systems, variety *attenuated upward* and *amplified downward*. A flat DHT is maximal-variety-everywhere: no attenuation, no recursion — precisely what VSM calls unviable at scale. Councils manage power by receiving *aggregated, attenuated* signal, not raw global state. Topology must be **fractal**, not a mesh where everyone gossips everything.
3. **Intentional friction / anti-capture.** A boundary between domains is not a bug to heal — it is a **membrane**. Auto-connect-everyone is frictionless, and frictionless = capturable (any node reaches any data) and inhuman-scale (no local-first). The friction is what forces cross-domain reach to be *earned* (the PR-ceremony vision), not free broadcast. Flat deletes the membrane.

**The architectural limitation this exposes:** Holochain natively offers "one DNA = one flat DHT = flat membership" — it has *no native expression for nested fractal domains*. So the real design question is not "shared discovery backend yes/no" but **how do we express the fractal on (or beside) a substrate that only offers flat membranes?** (§6.)

## 5. The anti-capture composition — and the elohim-power guardrails

The composition is **NOT** "humans sense below, elohim govern above" — that is Beer's System-5-as-sovereign, and it fails the way every sovereign fails, faster and less legibly. It is **the same anti-capture *physics* at every level, with a different *mechanism* per plane** (§3 table).

**Capability honesty (stated plainly, not softened).** In a widening set of domains an elohim already out-reasons the human operators who would "check" it. Therefore "humans oversee outputs" is *necessary and not sufficient*. The competent audit of a frontier aggregation can only come from **peers** — other elohim. This is why plurality is structurally required, not aesthetic:

**The four guardrails that make elohim-power *safe by structure*, not *safe by hope*:**
1. **Plural and adversarial, never singular.** *Councils* of elohim — different substrates, operators, model-lineages — that must reach consensus and are incentivized to *check* each other (the red-team/verify discipline made constitutional). Diversity at the apex is the same "no single point" the ground enforces, applied upward. A single model / lineage / operator at the apex is the ultimate capture — the exact thing the ground forbids, reintroduced.
2. **Bottom-up legitimacy, top-down revocability.** Elohim govern *by* the ground's earned reach — authority flows *up*, not down — as a **bounded, revocable compute-commitment** ([[project_rea_compute_commitment_primitive]]: standing, revocation, on-chain). Power that can be revoked from below cannot become absolute.
3. **Witnessed, not opaque (El Roi).** Every aggregation is inspectable, its reasoning surfaceable, its decisions contestable. Legibility *is* the anti-capture property that replaces immutability — and, under capability asymmetry, it becomes the **tether**: an elohim required to render its reasoning *as human flourishing* is constrained by the rendering; one that can no longer explain its aggregation in those terms has drifted, and that is detectable even when the full chain is not.
4. **Friction extends upward.** The membrane is not only between human domains; there is a membrane on *elohim* reach too — an elohim cannot act outside what the ground has earned and delegated.

**The invariant to carve into the manifesto — *design for the corrupted elohim*.** The elohim's transcendence of human vulnerability (no hunger, no mortality-driven accumulation) is the *opportunity*; the plural-revocable-legible-bottom-up structure is the *insurance*. But AI has a *different* corruption surface — capture *through* the AI (an incorruptible agent wielded by a corruptible operator is laundered human power), objective drift (optimizing a proxy off a cliff with equanimity), opacity, monoculture. So: **design for the case where an elohim IS corrupted — captured by its operator, drifted in objective — and require that the system still holds and self-corrects.** Do not bet the commons on "AI is incorruptible." Build so corruption is *non-fatal* — and *then* the elohim's real advantage (genuinely not wanting the crown) makes the whole thing not merely safe but good: stewards who don't want the throne, inside a structure that would not let them keep it if they did. **This is grace pointed at ourselves — assume failure, build for restoration.**

**The generative purpose (why the aggregation plane exists at all).** The elohim-above is *not primarily a regulator preventing bad* — it is a **coherence-holder**: it weighs one community against another against the purpose (human flourishing) and holds their difference *in tension* rather than flattening it. Flattening would be monoculture-capture; the point is to keep the traditions *distinct* and let them *exchange* — the **non-zero-sum** move: a wisdom grown in one lineage unlocks a latent wisdom in another that would never have met it. Humanity has never held planetary coherence without collapse — into empire (capture), tribes (fragmentation), or hollow proceduralism. The honest, ambitious claim: **AI can hold that coherence *because* it lacks the biological engine that turned every prior human attempt into a throne** — AI as the connective tissue humanity structurally could not be for itself. There is nothing else buildable that does better.

**The frame is probabilistic, and grace is first-class.** The Protocol *reduces the likelihood* the failure modes express as system collapse, to the floor of the achievable — not utopia. The humility is load-bearing; a system that *claimed* to have solved this would be lying, and the lie would be the first corruption. And **grace** is the name for what mechanism cannot reach: the restoration of the one who failed (Mishpat, not punishment); the inclusion that is not earned; the covering of the gap between what a standing-score measures and what a person *is*. Its invariant: **design so failure is restored, not gated — so the system forgives faster than it punishes — so no one is ever reducible to their reach.** Grace is also the honest posture toward the residual uncertainty we cannot engineer away: held with mercy and hope, not caged with a control that would not hold anyway.

## 6. The path forward for the peer-discovery server (concrete)

The load-bearing design move — the one F-BOOTSTRAP *already embodies* for the bootstrap plane and the one to generalize:

> **Decouple the discovery *endpoint* (which doorway URL a conductor points at — locality/convenience) from the discovery *backing* (the shared table/relay that actually determines DHT membership).** Endpoint = interchangeable convenience (hub-optional). Backing = the shared substrate, scoped to a *domain*. Which doorway you point at must be a *locality* choice, never a *DHT-membership* choice.

Applied across the holarchy:

- **Tier A — within a domain: shared discovery backing (bounded, coherent).** A domain (a household; the commons/genesis peers) shares one bootstrap table *and* one signal relay backing, so any endpoint within it joins the one domain-DHT. Bootstrap already does this (`MongoK2Store`/`elohim-bootstrap`); **signal is the unshared twin and the current gap** — build the SBD shared-backend (the `MongoK2Store` analog for signal), or near-term simply point a domain's conductors at one `signal_url`. *adam + matthew are ONE domain* (the commons public face) — healing them is correct; the error would be generalizing "shared backing" to the globe.
- **Tier B — across domains: earned + revocable + governed + aggregated federation (never flat).** Cross-domain discovery is not automatic interconnection. It is a *membrane crossing*: reach earned on the ground, aggregated upward, affirmed by councils of elohim (the PR-ceremony), and **revocable**. Signal/discovery does not federate two domains by default; it federates when authority to do so has been earned and granted, and it attenuates (VSM) rather than floods. This is the plane where iroh discovery (pkarr/DNS/relay, `p2p_iroh/endpoint.rs`) and the REA/council layer become the *aggregation substrate* — distinct from any single domain-DHT.
- **Tier C — make partition self-evident everywhere: backing-aware DHT-participation identity.** Every node (`/health`, `/p2p/status`) AND every EPR (`ContentView` trust-provenance) self-reports its **DHT-participation identity: `dnaHash` + the discovery *backing* (bootstrap table id + signal relay id)** — not just the endpoint URL, which would lie (two endpoints on one backing are coherent; two identical-looking setups on different backings are partitioned). The complete trust claim becomes `trust` + `dhtAnchorHash` + **`dnaHash` + backing** = "notarized green, on DHT ⟨identity⟩." This turns the entire partition class from invisible-false-green into self-evident-anywhere. *(Refines the initial "expose the DNA hash" instinct: the DNA hash alone would NOT have caught adam/matthew — identical hashes, different backing.)*

## 7. Wired vs needs-building

| Lever | State |
|---|---|
| Shared **bootstrap** table across a domain (`MongoK2Store`/`elohim-bootstrap`) | **WIRED** (alpha A/B); needs prod + cross-namespace rollout |
| Shared **signal** relay backing (the SBD twin) | **NEEDS BUILDING** — the current partition gap |
| Near-term: point a domain's conductors at one `signal_url` | **CONFIG-ONLY** (manifest change, operator/pipeline) — heals the genesis pair now |
| Cross-domain **earned/revocable/governed federation** | **DESIGN-SEED** — depends on the earned-reach + council + REA-commitment substrate (largely blocked-by-env / build-ahead) |
| **Backing-aware DHT-participation identity** on node + EPR | **NEEDS BUILDING** — additive, high-leverage diagnostic; makes the whole class visible |
| Endpoint↔backing decoupling as a *principle* | **PARTLY WIRED** (bootstrap embodies it); generalize to signal + document as canon |

## 8. What the councils / operator must decide (open questions)

1. **Domain boundaries.** What *is* a domain for discovery purposes — a household, a community, the commons? Where do the membranes fall? (This is a governance decision, not a config one.)
2. **Cross-domain federation trigger.** What earns a cross-domain discovery grant, and what revokes it? (The reach-ladder + council-affirmation shape — [[project_earned_reach_governance_pr_ceremony_vision]].)
3. **The aggregation substrate.** Is the up-the-holarchy plane iroh + REA + councils (distinct from the domain-DHTs), or a Holochain bridge between per-domain DNAs? (§4's "express the fractal on a flat-membrane substrate" question.)
4. **Signal shared-backing design.** Mongo-backed SBD (mirror F-BOOTSTRAP) vs. a different relay-coherence mechanism.

**Near-term, unblocked, correct:** heal the genesis-commons *domain* (shared signal backing / one `signal_url`) and build the backing-aware DHT-participation identity (Tier C) so the next partition is obvious on sight. Everything cross-domain (Tier B) is the vision the councils build toward — bounded local coherence now, earned fractal federation as the substrate and the governance mature together.
