# Trust as Efficiency Signal — The Compute-Burden Gradient

**Architectural brainstorm — foundation for EPR-native graph enablement, prerequisite to Phase 3**

**Date:** 2026-04-30
**Status:** Brainstorm artifact (output of `superpowers:brainstorming` session)
**Author:** Matthew Dowell + Claude
**Downstream:** Phase 2B spec amendments (§6.4 + §3.7/§7 O2 reconciliation); Phase 3 kickoff prompt refresh; Phase 3.5 plan (new substrate); social-reach epic narrative addition

---

## §1 Thesis: Our Attention Is Sacred

Across the deepest currents of human wisdom — the Torah's command to *till and keep* the garden (Gen 2:15), the Sh'ma's call to love with all one's heart, soul, and might (Deut 6:5), the contemplative traditions of attention as the seat of formation, Simone Weil's claim that "attention is the rarest and purest form of generosity," and the Center for Humane Technology's modern critique of attention as an extracted commons — the same conviction emerges:

> **Human attention is sacred, and how a system treats it reveals what that system actually believes about human dignity.**

Extractive platforms treat attention as a renewable resource to be strip-mined. Engagement metrics, dopamine traps, infinite scroll, opaque algorithmic curation, frictionless amplification of outrage — each is a confession that the underlying architecture does not believe attention is sacred. The harms catalogued in CHT's *Ledger of Harms* (children's brain development, fake news traveling six times faster than truth, anger as the most viral emotion, civil discourse decaying, democratic processes eroded) are not bugs of the extractive web. They are the *predictable effects* of building an architecture that treats attention as a commodity rather than a commons.

The Elohim Protocol's foundational design commitment is the inverse: **attention is treated with dignity, reverence, and care at every layer of the substrate.** This is not a feature; it is the architecture's moral spine.

The technical primitives specified in this document — standing as accountability for stewarded power, tending as the human's sacred act in their attention garden, carrot-before-stick at compose time, constitutional protections against weaponization, the elohim as advocate and tender — all derive from this single commitment. Every future protocol decision must answer to it: *does this treat attention as sacred?*

---

## §2 Foundational Principles

Nine load-bearing claims emerged in the brainstorm. Each is a constitutional commitment for everything downstream.

### §2.1 Trust as efficiency signal — the compute-economic frame

Trustworthy, accurate, good-faith content reduces the computational cost of distribution, discovery, validation, and verification at every edge of the network. Untrustworthy, inaccurate, bad-faith content imposes structural cost. Distribution cost **scales with trust** at every hop, which is the protocol's anti-spam-megalith mechanism: there is no central verification authority because there is no need for one. Cost-asymmetry is enforced edge-by-edge by the substrate itself.

This is the compute-economic frame on a moral category. Trust is not just a virtue we want to encourage; it is a compute-resource property that reduces network burden when present. The protocol architecture must make this property visible and operative at every layer.

(See memory pin `project_trust_as_efficiency_signal`.)

### §2.2 Standing on agents, reach on content (the disambiguation)

Three signals that the casual word "trust" conflates, with sharp architectural distinctions:

| Property | Lives on | Set when | Mutability | Source of truth |
|---|---|---|---|---|
| **Reach** | content envelope | at authoring | immutable post-publish (envelope is signed) | author's earned reach at publish moment |
| **Reach-earning** | agent (author-side) | at authoring | derived per publish | graph walk against delegation/membership edges |
| **Standing** | agent | continuously, via sense-respond | mutates with attestations / corrections / restitution | derived from attestation/citation/correction subgraph |
| **Provenance** | envelope chain | at each forward | append-only; constitutionally revealable | per-hop signed predecessor records |

**Reach is about what content can flow.** It is bounded at authoring time by the author's earned standing in the relevant scope. Standing is about what stewards can stake when they participate in propagation, and what cost-asymmetry they consequently impose on the network. The two are orthogonal but coupled: reach-earning is gated by standing; standing accrues through participation across all reach scopes.

Critically, **standing is a graph-derived property, not a stored score.** The architecture refuses the social-credit-system shape: there is no central tabulation, no authoritative number, no scoreboard to game. Standing is a view rendered by walking the attestation/citation/correction subgraph from an agent through whatever constitutional lenses (manifests) the evaluator subscribes to. Different evaluators see different views; that is a feature of pluralism, not a bug.

### §2.3 Power coupled to responsibility — Dunbar-by-design

> The opportunity cost of reach is borne by the stewarded compute that affords it. Distribution power confers stewardship responsibility at every edge. Both are visible, accountable, and bounded. Humans who bear network burden recognize that burden in their compute; the network's design respects human cognitive limits (Dunbar's number) by making popularity-amplification cost something at every hop, paid by humans whose attention and compute and standing power it.

This is the architectural commitment behind the whole brainstorm. Email and social-media collapse occurred because distribution was free at the marginal cost layer (sender pays nothing per recipient; recipient pays everything in attention) — so megaliths had to absorb the asymmetry centrally, becoming both arbiters and exploiters.

In the Elohim substrate, distribution costs are distributed, edge by edge, and proportional to the standing being staked. **Trust-bubble boundaries (where churn breaks the back-prop walk, where Dunbar-scale relationships exhaust) are not robustness limitations — they are humane architectural properties.** Reach beyond Dunbar costs proportional standing to sustain.

### §2.4 Constitutional revealability of provenance — Genesis 3:11

> *"Who told you that you were naked?"* — Genesis 3:11

When Adam and Eve eat of the fruit, the first divine question is one of provenance. The question must be answerable. Disclosure is not punishment but the mechanism by which responsibility is taken.

Architectures that permanently hide provenance protect bad actors. Architectures that always reveal provenance weaponize social graph. The protocol's commitment is between:

> **Provenance is private by default. It is recoverable through governance, never by free read. Recovery is itself an accountability event — recorded, attested, traceable. Imagodei constitutional protections (elohim-as-counsel) are active during recovery; the agent is represented, not silenced. No absolute opacity. No default transparency.**

Cryptographically, predecessor links are recorded *sealed-against-the-self* (e.g., encrypted with a constitutional-disclosure key derivable only through threshold cooperation of mishpat-quorum + the subject's imagodei). Subpoena pressure is structurally infeasible: no single peer can disclose unilaterally; the constitutional handshake is required.

### §2.5 Paced reconciliation accountable to stewarded compute

When a peer goes offline and returns, content reconciliation does not flood. It processes through a batch queue paced by the device's available compute. New authorings drain at a sustainable rate; received forwards reconcile without blocking; pending standing-impact signals absorb without thundering-herd.

This is `principle_p1_reconciliation_controller` applied at peer scale: observe drift, reconcile eagerly *but at a pace*, index-bounded, observable via reconciliation-lag metrics, never blocking. The same controller pattern that governs DHT manifest reconciliation governs back-prop signal absorption and offline-online catchup.

### §2.6 Carrot before stick — author-time tender conversation

Aggregate/advisory sensemaking is the **safety net**. The primary vehicle of the standing/filter/discernment system is the elohim's **author-time conversation** with the human, before the post leaves the device.

```
COMPOSE TIME                          POST TIME             RECEIVE / AGGREGATE
────────────                          ─────────             ───────────────────
elohim (tender specialist)            post leaves           safety net only —
holds the conversation:               the device.           if author-time
"This might land hard with                                  missed it, this
the household — last week's takes                           catches it.
on this topic met fatigue. Want                             Stick rare,
to narrow scope, soften framing,                            recovery-oriented.
or post anyway with awareness?"

← carrot, agency-preserving           agency exercised      ← stick, recovery
```

The architecture pushes the burden of accountability **left** in the flow — into the moment of authorial agency, where the human can act on it most cheaply and humanely. The substrate must support this: the aggregate filter signals + standing computation must be queryable cheaply from the author-side compose flow, not just the receive-side discernment.

(Pairs with `project_elohim_as_counsel` in tender mode, and `feedback_less_pushy_notifications`.)

### §2.7 Substrate-thin / manifest-medium / agent-thick-at-scale

The architecture has three layers, fixed; the *weight* each layer carries shifts as the network matures.

| Phase | Substrate | Manifests | Agents |
|---|---|---|---|
| Today (thousands of agents) | EPR primitives + constitutional floor | bootstrap defaults + early collective rules | supplementary discernment |
| Tomorrow (millions) | unchanged | richer per-collective variation | primary nuance carrier |
| At scale (billions) | unchanged | constraint frame only | the discernment substrate |

**Substrate** provides only graph operations: signed EPR kinds (attestation, correction, restitution, vouch, feedback-signal, tending), chain provenance, constitutional floor schemas. Substrate does NOT define what counts as a debit, weighting algorithms, statute of limitations.

**Manifests** declare rules (mishpat) and variation (qahal) and epistemic frames (lamad). Each collective publishes a `standing-policy` manifest as a Manifest-EPR; agents see standing through whichever manifests they subscribe to.

**Elohim agents** execute case-by-case discernment. Specialist subagents (defender, advocate, tender, gate-discerner) apply manifest rules to local context. As the agent layer matures, manifests become *less prescriptive*, not more.

This is bullish architecture: we don't have to perfectly specify every rule today. We specify the constitutional floor + bootstrap defaults, and let agent-think absorb increasing nuance load. Quality of elohim agents becomes the primary scaling variable; architecture must be agent-permissive (offline/minimalist elohims still participate) but not agent-dependent.

### §2.8 Constitutional floors — protections at every layer

Two distinct floors hold across the trust→compute gradient: the **standing-immune floor** (cannot be eroded by standing impact) and the **tending-immune floor** (cannot be silenced by attention-tending filters). Both are mishpat-DNA-notarized, immutable to community override (collectives may be *more* protective, never *less*).

**Standing-immune floor** (against gradient-driven erosion):

| Floor | Protection |
|---|---|
| Local relationship reach | unconditional — family/household/closest-trust-bubble propagation never standing-gated |
| CID-targeted lookup | unconditional — anyone with a CID can fetch the content; standing modulates discovery, not retrievability |
| Constitutional-floor signatures | bypass amortization — mishpat decisions, imagodei-as-counsel signatures always per-message verified |
| New-voice baseline | nonzero — a first-time author has a standing-floor derived from their household/collective sponsor; never starts at zero |
| Vulnerable-class elevation | children, refugees, persecution-targets, displaced persons have elevated floors; cannot be silenced by standing-gradient alone |

**Tending-immune floor** (against filter-driven silencing — the broccoli floor):

| Floor | Protection |
|---|---|
| Accountability information | un-filterable — corrections, peer-review, restitution requests targeting the agent always reach them |
| Community facts | un-filterable — civic emergencies, public-health, governance decisions affecting the agent's collectives |
| Custodial communications | un-filterable — under stewardship-graduated authority, ward-targeting messages bypass the steward's filter |
| Constitutional updates | un-filterable — mishpat rule changes, qahal ratifications, charter amendments |
| Elohim-as-counsel notifications | non-binding to filter — agent's own elohim has standing to override filters when judgment requires |

The §6.3 broccoli floor in the tending subsystem refers specifically to the second table. The §3.2 gradient table's "floor protection" column draws from the first.

### §2.9 Honesty at onboarding

The protocol is up-front about its virtue commitments at the moment of joining. There are no surprise rules, no hidden gates. New peers acknowledge an onboarding manifest (a signed EPR) declaring the network's commitments before participation.

> "Welcome. This is a virtue-centric network. It values truth, accountability, good-faith stewardship, paced attention, and the dignity of every participant — not as features but as architectural commitments. If you are bringing baggage from extractive platforms — thinking you can use this as a sewage dump for the bullying, grift, propaganda, or spectacle you got away with elsewhere — you will find friction here. That is not punishment. That is the structure. Growth is mutually beneficial but can be painful. Welcome to the network."

Bootstrap-light onboarding paths are available for compute-poor peers (early-network, low-bandwidth, minimal-elohim devices). These do not exempt the peer from standing accountability — **standing back-prop fires at any elohim inference edge, not at the peer's own compute**. A new peer with no local elohim still accrues standing impact when their content reaches a peer with discernment capacity. The system is *discernment-gated*, not *compute-gated*: standing accountability participates equally regardless of the peer's own resources.

Pairs with `project_ungrudging_service` (no gating on acknowledgment after the fact; healing flows quietly) and `project_graduated_recovery_authority` (community recoverability is structural; users cannot opt out).

---

## §3 The Trust→Compute Gradient

### §3.1 Today's substrate is binary; the gradient is absent

Phase 2B's substrate encodes a binary trust signal at one layer: reach is earned at authoring, and pre-authorization is a topology decision at receive. That is the floor. Between the floor and the ceiling, trust is flat. There is no continuous signal modulating compute cost based on author standing, content history, or chain provenance.

This brainstorm specifies the gradient that fills that gap. It is implemented as standing-aware code paths in Phase 3 (with placeholders) and lit up by Phase 3.5 substrate (feedback-signal, tending, constitutional floor).

### §3.2 The seven-layer gradient

| Layer | Today (flat) | With standing-gradient | Floor protection |
|---|---|---|---|
| **DHT notarization** | per-entry signing (constant cost) | unchanged at write — DHT stays narrow per `project_dht_vs_libp2p_scoping`. Standing signals are NOT DHT-notarized. | n/a |
| **libp2p fanout (gossipsub)** | tier-by-reach (Phase 2B Batch D) | author-standing modulates initial hop budget; chain-standing modulates forward hop budget at each peer; low-standing = clipped budget but never zero within trust-bubble | every peer reaches direct relationships at full fidelity, regardless of standing |
| **Kad provider records** | flat advertising | high-standing peers' provider records refreshed eagerly; low-standing records expire faster; query results biased by standing | low-standing content remains *findable* by anyone seeking it (CID lookup never standing-gated) |
| **schemaRef walks** (Phase 3) | flat resolution | walks bias toward locally-cached + high-standing peer providers; depth limit shorter on low-standing chains; cycle detection cheap on high-trust paths | manifests are always resolvable — schemaRef cannot be standing-gated for protocol-load-bearing types |
| **Projection caching** | flat eviction | cache priority weighted by author-standing × content-citation-density; low-standing evicted first; back-prop signals invalidate cache | recent local-relationship content never evicted purely by standing |
| **Validation (verify_incoming)** | flat per-message | bulk-verify amortization for known-good signer streams; low-standing signers re-verified per message | constitutional-floor signatures always per-message verified |
| **Cold-fetch peer selection** (Phase 3 P3.4) | first-available | high-standing providers queried first; low-standing as fallback; timeout shorter on low-standing | if no high-standing provider exists, low-standing fallback is mandatory |

### §3.3 Persona stress-tests for the gradient

| Persona | Stress-test | Floor that holds |
|---|---|---|
| **Child** | reach-floor + graduated expansion via demonstrated good judgment | child reach unconditional within family; vulnerable-class elevation; custodial communication un-filterable |
| **Activist** | brigade-resistant; trust-curve cannot weaponize into silencing dissent | new-voice baseline floor; constitutional-floor signatures bypass amortization for evidence-bearing dissent |
| **Content creator** | quality + verification earn reach (not viral gaming) | counter-evidence floor — corrections always reach the creator (un-filterable) |
| **Community moderator** | restorative not punitive at edge; can't centralize moderation power | aggregate filter visibility at advisory level only; agent-load-shifting to elohims; no central scoreboard |
| **Displaced person** | safety from those fled from + dual-community belonging | vulnerable-class elevation; constitutional revealability requires high governance bar |
| **Elder** | scam-resistant reach + autonomy preserved (not infantilized) | trust-bubble floor; elohim-as-tender protects without disempowering |
| **Refugee** | cross-border reach + cultural preservation + persecution protection | vulnerable-class elevation; mishpat sanctuary protocols at full rigor |

---

## §4 Standing as Agent-Property

### §4.1 Reach vs Standing vs Provenance — the load-bearing disambiguation

See §2.2 table. This section restates what each property *is* in graph terms, because the entire architecture hinges on the distinction.

**Reach** is a field on the content envelope, signed at authoring time. It declares the scope the content may travel within. Reach is bounded by the author's earned reach at publish moment — the author cannot declare a reach beyond their standing supports.

**Reach-earning** is a graph property of an agent computed at publish time: walk delegation/membership edges from the agent; what scopes have credentialed paths? The author can earn reach=district if there is a path through their delegations + memberships to the district scope.

**Standing** is a continuous graph property of an agent computed by walking the attestation/citation/correction subgraph through whichever constitutional manifests the evaluator subscribes to. Standing is what an agent stakes when they participate in propagation; the back-prop nervous system settles those stakes when corrections arrive.

**Provenance** is the chain of signed predecessor records, threading through propagation. Constitutionally revealable; private by default.

### §4.2 Standing is a graph-derived view, not a stored score

The architecture refuses the social-credit-system shape. There is:
- no central tabulation
- no authoritative numeric standing
- no scoreboard
- no "score" stored on an agent record

Standing is *derived*, on demand, by the evaluator (a peer, a peer's elohim) walking the relevant subgraph through the constitutional lens of whichever manifests apply. Different evaluators see different views; this is pluralism, not inconsistency. An agent participating in multiple collectives has multiple standing views, one per collective's constitutional lens.

This is what makes standing different from social credit:
- Social credit centralizes nuance in opaque algorithms imposed on everyone
- Elohim standing distributes nuance across billions of personal agents, each with their own context and constitutional commitments
- Manifests are revisable by the collectives that author them; algorithms are not
- Agents are accountable to their humans; algorithms are not

### §4.3 Aggregation, decay, visibility — design resolutions

**Aggregation across collectives** (4a): an evaluator computes the subject's standing through the *evaluator's* constitutional lens (Option (i) from the brainstorm). Each peer evaluates standing through *their* manifest subscriptions. Forum-shopping by the subject is moot because the evaluator doesn't have to honor the subject's choice of permissive collective.

**Decay** (4b): no automatic time-based decay. Debits don't fade; they are discharged through restitution events — corrections published, vouches from corrected-by peers, time spent in good-faith stewardship attestable by graph mutation. Restitution is graph mutation, not a clock tick. This forces the network to design *redemption paths*, which is far more humane than "wait long enough and your past disappears."

**Visibility** (4c): app-manifest declared, evolves with agent maturity.
- Today / gamified contexts: numeric/credit-score visibility appropriate (game progress, learning mastery)
- Tomorrow / relational contexts: subsumes into agent-mediated contextual storytelling
- The mode is per-domain UX, declared in app-manifest, revisable

The substrate doesn't change with mode. Only manifest declarations and agent capabilities shift.

### §4.4 Restitution as graph mutation

Standing recovers when the subgraph rewires. The restitution paths are graph operations:

- **Correction-published**: the agent authors a correction EPR acknowledging an earlier error
- **Vouch-from-corrected-by**: the peer who issued a correction signs a vouch attesting the agent's good-faith repair
- **Time-in-good-faith**: stewardship of others' content without further squelches; attested by the network's continued engagement
- **Mishpat ratification**: governance bodies can ratify a restitution and propagate the signal

Standing recovery is **active**, not passive. It requires the agent to *do something* and other agents to *witness* it. This is exactly the human-scale shape: redemption requires repair, not just time.

### §4.5 Standing visibility evolves with agent maturity

App manifests declare which mode applies. Substrate doesn't change. As elohim agents mature, more domains shift toward contextual storytelling; the numeric mode persists where it serves (games, learning), fades where it doesn't (relational, professional, civic).

---

## §5 Sense-Respond Nervous System

### §5.1 Wire format — feedback-signal EPR kind

```
FeedbackSignal {
  target_cid:        Cid,         // content being acted on
  signal_kind:       enum,        // squelch | correction | retraction | quarantine
  evidence_cid:      Option<Cid>, // pointer to a Correction EPR with claims/citations
  standing_impact:   enum,        // graduated (advisory / debit-soft / debit-firm)
  signed_by:         AgentKey,
  signature:         Sig,
}
```

Travels via the existing `/elohim/epr-atom/1.0.0` libp2p protocol. No new wire crate needed; the EPR substrate carries the nervous-system signals as first-class atoms.

Four signal kinds, graduated:

- **squelch**: a steward's discretion — "I don't think this should propagate further." Private to that peer's forwarding decisions.
- **correction**: epistemic — "Here's what's actually true; the previous claim was wrong." References evidence.
- **retraction**: author-side — "I, the original author, withdraw this." Terminates propagation chain.
- **quarantine**: governance — "The collective has determined this is harmful; structural cost imposed." Requires mishpat/qahal authorization.

### §5.2 Edge-local back-prop with implicit chain (Primitive 2)

Each peer maintains *locally* (private): a predecessor map for content they've forwarded — "for content X, I received it from peer Y at time T." The chain is **never on the wire**; it is reconstructed hop-by-hop, one peer at a time, walking backward through local memory.

When a correction signal for content X arrives at peer P:

1. P verifies the corrector's standing-to-correct (local graph walk; elohim specialist does the work)
2. If validated, P records *local* standing impact (debit self for forwarding; credit corrector)
3. P stops propagating X
4. P forwards the correction signal to its own predecessor only (one hop back, using local predecessor map)
5. Predecessor does the same on receipt

Properties:
- **Privacy-preserving**: each peer knows only immediate predecessor
- **Bounded wire metadata**: no chain on the EPR itself
- **Cost-distributed**: each peer pays its own back-prop work
- **Composable with elohim specialists**: validation gate at every hop, contextual to that peer's constitutional lens
- **Trust-bubble-bounded**: walk breaks at peer-offline / peer-out-of-relationship boundaries — a humane property, not a bug

### §5.3 Constitutional revealability — sealed predecessor records

Predecessor links are recorded in two forms locally:
1. **In-the-clear** for the peer's own use (their elohim's discernment, their tender's authoring help)
2. **Sealed-against-the-self** — encrypted with a constitutional-disclosure key derivable only through threshold cooperation of mishpat-quorum + the subject's imagodei

Recovery is a governance act:
- Mishpat rule applies (which kinds of inquiries authorize disclosure)
- The subject's imagodei elohim acts as counsel during the unwinding
- Qahal of the relevant collective ratifies
- Each disclosed hop emits its own attestation EPR; the disclosure is itself recorded
- Subject's elohim observes the disclosure and may file objections

(Cryptographic detail of the threshold/sealing scheme is deferred to Phase 5/6; this brainstorm canonicalizes the property.)

### §5.4 Gossip-flood notification (Primitive 3) as complement

A *separate* feedback-signal published to the content's reach gossipsub topic for general epistemic awareness — "everyone with a copy should know this is corrected." Layers on top of Primitive 2; does not replace it.

The two primitives are orthogonal:
- Primitive 2 distributes **standing impact** along the chain of stewards (back-prop on participation)
- Primitive 3 distributes **epistemic notification** to all current holders (so they know not to act on stale content)

Both live in the same `/elohim/epr-atom/1.0.0` protocol; both emit `FeedbackSignal` EPRs; the gossip-flood path is a separate publish call.

---

## §6 Tending as Values-Forward Filter Subsystem

### §6.1 The AttentionTending EPR kind

The user-side primitive is `AttentionTending`. The Genesis 2:15 vocabulary — placed in the garden to till and keep — anchors the metaphor: the human tends the shape of their attention. This is not a perimeter filter; it is a discernment signal feeding the elohim's tender conversation and the collective's wisdom layer.

```
AttentionTending {
  filter_subject:    FilterSubject,    // pattern (content kind, topic, author scope, etc.)
  classification:    FilterClass,      // values-forward | fatigue | scope-mismatch | safety
  reason:            Option<String>,   // human-readable, optional
  ttl:               Duration,         // expires unless re-tended
  tended_at:         Vec<Timestamp>,   // re-tending events
  context:           ContextScope,     // when this applies (collective, mode, time-of-day)
  signed_by:         AgentKey,
  signature:         Sig,
}
```

Peer-private by default. The peer's elohim consults it during discernment; the collective sees aggregate, anonymous patterns. Bytes still arrive subject to upstream standing/reach gating; tending is post-arrival, peer-private discernment.

### §6.2 Five constraints → five graph operations

| Constraint | Implementation | Distinction from email-collapse |
|---|---|---|
| Human-set | signed by the agent's key; no algorithmic auto-creation | the human authored it; auditable; not silently imposed |
| Time-limited | TTL on the EPR; expires unless re-tended | filters can't accumulate into a permanent moat |
| Tended | tended_at events extend TTL; un-tended expires | mindless filter-everything is structurally costly |
| Anti-filter-bubble | mishpat constitutional floor declares un-filterable classes; elohim has standing to override | weaponizing filters to hide from accountability is structurally impossible |
| Collective-wisdom feedback | filters export aggregate, anonymous signals to the collective's manifest registry | filter becomes input to collective sense-making; not a private cone of silence |

### §6.3 The broccoli floor — un-filterable classes

Per §2.8, the constitutional floor declares classes that cannot be filtered:

1. Accountability info about self or close associates
2. Community facts (civic emergencies, public-health, governance)
3. Child-safety and custodial communications
4. Constitutional updates (mishpat / qahal / charter)
5. Elohim-as-counsel notifications

The elohim is the agent's advocate AND tender — it honors filters as discernment signals AND surfaces broccoli when judgment requires. This dual role is the elohim-specialist pattern at its most concrete (advocate + tender + gate-discerner subagents composed).

### §6.4 Collective-wisdom aggregation

```
CollectiveFilterPattern {
  collective:        CollectiveId,
  filter_subject:    FilterSubject,
  classification:    FilterClass,
  participating_pct: u8,              // % of collective members tending against this
  trend:             Trend,           // rising | stable | falling
  context_window:    Duration,
  // NO peer identities — pure aggregate
}
```

Aggregate is necessarily privacy-preserving — individual peers are never identified. Differential-privacy noise added if k-anonymity threshold isn't met. The collective's manifest layer consumes these patterns to:

- Adjust standing-policy debit weights
- Spawn community-moderator roles (qahal governance trigger)
- Surface to mishpat (constitutional review) when crossing thresholds suggesting coordinated bad-faith
- Inform the elohim layer's discernment

### §6.5 Trust-bubble visibility — aggregate/advisory only, never per-peer-callout

Filters are peer-private. Aggregate filter patterns within trust-bubbles are visible to that collective's governance roles only (qahal moderators, mishpat adjudicators), never as per-peer callouts to other peers. Aunt does not see "Cousin Bob is filtering my content." She might see (via her elohim's tender conversation) "your last three shares to the household had below-average tending engagement; consider whether the topic is fatiguing the room."

The information is surfaced at the **aggregate + advisory** level only. **Author-time elohim conversation is the primary vehicle (carrot before stick). Aggregate visibility is the safety net.**

Tending defaults (bootstrap manifest, per-collective override allowed):
- safety classification: no expiry
- fatigue: 7-day TTL
- values-forward: 30-day TTL
- scope-mismatch: 90-day TTL

Override authority for the elohim to surface broccoli over a tending: constitutional-floor overrides are mandatory; non-floor overrides are elohim-discretionary, must be recorded as attestation EPRs the human can later see and contest. Override frequency contributes to the agent's elohim-quality signal.

---

## §7 Pillar Partition of Authority

### §7.1 Substrate

EPR primitives + the constitutional floor. DNA-notarized in elohim-core. Includes:
- `Manifest` EPR kind (Phase 3)
- `Attestation`, `Correction`, `Restitution`, `Vouch` EPR kinds
- `FeedbackSignal` EPR kind (Phase 3.5)
- `AttentionTending` EPR kind (Phase 3.5)
- Constitutional floor manifest schema (Phase 3.5; mishpat-DNA-notarized)
- Edge-local predecessor map + sealed records format (Phase 3.5)

Substrate does NOT define: debit weights, statute of limitations, what counts as bad-faith. Those are manifest territory.

### §7.2 Manifests

Constitutional rules and variation. Manifest-EPRs (Phase 3 lands these as first-class) declare:

- **mishpat manifests**: rules — what counts as a debit, due-process minimums, statute of limitations, immutable principles, vulnerable-class protections
- **qahal manifests**: variation — each collective declares its own evidence standards, governance roles, restitution rituals
- **lamad manifests**: epistemic frames — peer-review, citation, falsifiability standards for content domains
- **shefa manifests**: cross-coupling rules — how standing influences shefa flows and vice versa
- **app manifests**: per-domain UX (compose-time conversation surfacing, visibility mode for standing)
- **standing-policy manifests**: per-collective declarations of debit weights, restitution paths, decay (none by default), aggregation rules

A bootstrap default standing-policy ships with the protocol so Stage 1 onboarding works without manifest authoring. Communities fork-and-modify; the default is a starting point, not a law.

### §7.3 Elohim agents — specialist subagents

Local discernment at decision time. Specialist subagent roles:

- **defender**: reactive — when an agent is attacked, brigaded, or wrongly debited, defends
- **advocate**: representational — speaks for the human in governance proceedings, restitution, recovery
- **tender**: proactive — author-time conversation (carrot-before-stick)
- **gate-discerner**: validation — evaluates whether incoming signals meet the agent's constitutional standard for action; audits other gate-discerners (peer-elohim audit)

Agents execute *whatever combination* of constitutional floor + collective manifests + their own local rules apply. They can be more strict than their collective requires (defender on high alert) but never less.

### §7.4 Agent-load-shifting as scale property

Per §2.7. The architecture is permissive but not dependent. A peer with no local elohim still participates in standing accountability; their content is evaluated by other peers' agents. Bad-elohim populations are detected by gate-discerner audit + behavioral fingerprinting + peer-elohim-attestation; model diversity is the operational defense against coordinated drift.

---

## §8 Phase 3 Compute-Burden Refinements

The original kickoff prompt at `genesis/docs/plans/2026-04-26-epr-phase-3-manifest-resolver-kickoff-prompt.md` lists 7 tasks. This brainstorm refines each with compute-burden constraints and standing-aware code paths.

| Task | Original framing | Compute-burden refinement |
|---|---|---|
| **P3.1 ManifestRegistry** | replaces `pillar_for_kind_provisional` | high-trust manifests cached eagerly; experimental lazy-loaded; refresh schedule modulated by manifest's standing; **author-side lookups are fast-path** for carrot-before-stick |
| **P3.2 Manifest-as-EPR** | `kind: Manifest` variant + DNA entry + projection | constitutional category — full per-message validation, never amortized; eager projection; HDI-validation deterministic per `project_hdi_no_get_links_in_validators` |
| **P3.3 schemaRef walks** | recursive walks, cycle detection, depth limit | depth limit shorter on low-standing chains (3-5 hops vs 8 high-standing); cache-first; walk highest-standing peer first; **floor: protocol-load-bearing schemaRef always resolvable** |
| **P3.4 Cold-fetch via swarm** | `swarm_handle.resolve_epr(cid)` on local miss | high-standing providers queried first parallel; low-standing serial-fallback shorter timeout; **floor: low-standing fallback mandatory if no high-standing** |
| **P3.5 Manifest write-through SoT** | replaces `HashMap::new()` stub | per-manifest absorption rate (paced reconciliation); manifest mutations constitutional — full validation, never amortized; reconciliation-lag observable per P1 |
| **P3.6 Dedup wiring on read routes** | 5 TODO(phase-3) markers in epr.rs | dedup window shorter for high-standing peers; longer for low-standing; PeerId threading prepped at Z.1 |
| **P3.7 Integration test extension** | cold-fetch + schemaRef walk scenarios | add: floor-protection assertions, compute-burden assertions (high-standing peers see X% lower validation cost), persona-stress-test scenarios |

**Standing-aware code paths in Phase 3:** functions take a `Standing` argument that returns `Standing::Unknown` until Phase 3.5 lights up the signal. Wiring is in place; the gradient *architecture* is testable; live signal flow follows.

---

## §9 Phase 3.5 Proposal — New Substrate

A new phase between Phase 3 and Phase 4 (VF-GraphQL) that adds the brainstorm's new substrate. Estimated 3-4 weeks; separate plan to be brainstormed.

| Task | One-liner | Priority |
|---|---|---|
| P3.5.1 | `FeedbackSignal` EPR kind + libp2p protocol extension (squelch / correction / retraction / quarantine variants) | P0 |
| P3.5.2 | Edge-local predecessor map + sealed-against-self record format | P0 |
| P3.5.3 | Hop-by-hop back-prop walk impl (Primitive 2) | P0 |
| P3.5.4 | Gossip-flood notification (Primitive 3) layered on top | P1 |
| P3.5.5 | `AttentionTending` EPR kind + tending TTL/lifecycle | P0 |
| P3.5.6 | Collective-wisdom aggregator (anonymous, k-anonymous, differential-privacy when needed) | P1 |
| P3.5.7 | Constitutional floor manifest schema (mishpat-DNA) + 10 floor classes from §2.8 | P0 |
| P3.5.8 | Bootstrap default standing-policy manifest | P0 |
| P3.5.9 | Author-side compose-time query API (cheap; for elohim tender conversation) | P0 |
| P3.5.10 | Integration test: end-to-end aunt-and-rage-bait scenario (Appendix B) | P0 |

### §9.2 Sequencing relative to Phase 3 / Phase 4

```
Phase 3 (current kickoff, refined)  →  Phase 3.5 (new substrate)  →  Phase 4 (VF-GraphQL surface)
        │                                     │                              │
        ├ Manifest-EPR resolver               ├ Standing/Tending/Floor       ├ hREA / VF semantics
        ├ schemaRef walks                     ├ Back-prop primitive          ├ Apollo / GraphQL endpoint
        ├ Cold-fetch via swarm                ├ Constitutional floor         ├ Resolver implementations
        ├ Standing-aware code paths            ├ Author-side query API
        │  (signals = Unknown placeholder)    └ Lights up Phase 3 placeholders
        └ Integration tests
```

### §9.3 Ties to other epics

- **Recovery M-series**: M5's auth-portal convergence + revocation UX shares the DNA signal stream contract with Phase 3.5's FeedbackSignal pipe. Coordinate schemas at `elohim/sdk/schemas/v1/dna-signal-stream.schema.json`.
- **Defender stub** (M5): the defender specialist subagent currently stubbed — Phase 3.5 lights up the constitutional-floor manifest the defender consults.
- **Social-reach epic**: the protocol's narrative spine. Phase 3.5 is the substrate that makes Maria's morning context (epic.md §6:15 AM) literally true.
- **Avodah pillar (reference impl)**: as the protocol-as-process reference, avodah will be among the first to author a `standing-policy` manifest demonstrating the pattern.

---

## §10 Open Questions and Stage Gates

### §10.1 Cryptographic detail of constitutional disclosure scheme

The threshold-encryption / Shamir-split scheme for sealed-against-self predecessor records is **deferred to Phase 5/6** as its own design. This brainstorm canonicalizes the property (private by default, governance-recoverable, disclosure is itself accountability event); the cryptographic implementation is its own design.

**Phase 3.5 interim sealing mechanism:** predecessor records are encrypted at rest with a per-collective `constitutional-disclosure-key` derived deterministically from the collective's mishpat-quorum public key + the subject agent's imagodei key. Recovery requires both keys cooperating (a 2-of-2 not a t-of-n). This is sufficient for the property to be testable end-to-end while the full t-of-n threshold scheme is designed. Replaced (not removed) when Phase 5/6 lands the threshold scheme.

### §10.2 Standing aggregation across collectives in heterogeneous agents

What happens when a peer subscribes to multiple collectives whose standing-policy manifests disagree about the same agent? Today's resolution: each evaluator computes the agent's standing through *their* lens (per §4.3). But agents that need a single answer (e.g., a UI displaying a numeric mode) need a tie-breaker. Manifest-declared aggregation rule (mean, max, min, voted)? Open.

### §10.3 Bad-elohim detection at scale

Quality of elohim agents is the primary scaling variable. Defenses:
- Gate-discerner specialist audits other gate-discerners
- Behavioral fingerprinting (statistical drift detection)
- Peer-elohim-attestation (elohims attest to other elohims' good-faith)
- Model diversity (per `project_compute_and_model_independent_diversity_surfaces`)

The detection mechanisms themselves are Phase 6+ territory; the *substrate* must support them (signed elohim-attestation EPRs, behavioral signal fanout). Brainstorm canonicalizes the requirement.

### §10.4 Stage-gate: Stage 1 bootstrap → Stage 3 enforcement transition

Per `project_bootstrap_to_elohim_security_gradient`, the protocol operates at three stages:
- Stage 1: structural validators (Phase 2B/3 substrate)
- Stage 2: elohim-coordinated trust (Phase 3.5 lights up)
- Stage 3: full elohim enforcement (matures over years)

The transition between stages is gradual and per-collective. Each collective's manifest declares which stage it operates at; the substrate supports all three simultaneously. Documentation must clearly mark which stage a given protection operates at, so we don't force Stage 3 rigor at Stage 1.

---

## §11 Cross-References

### §11.1 Memory pins consulted

- `project_elohim_vision_fruit_back_on_tree` — protocol's purpose
- `project_reach_earned_at_authoring` — author-side floor
- `project_social_reach_nervous_system` — sense/respond primitives
- `project_trust_as_efficiency_signal` — compute-economic frame
- `project_values_forward_preference_guards` — tending constraints
- `project_first_class_graph_pattern` — substrate is graph
- `project_three_layer_truth_model` — DHT/libp2p/doorway separation
- `project_dht_vs_libp2p_scoping` — DHT stays narrow
- `project_elohim_subagent_specialists` — agent-layer architecture
- `project_principle_p1_reconciliation_controller` — paced controller pattern
- `project_elohim_as_counsel` — defender/tender duality
- `project_ungrudging_service` — no gating on acknowledgment
- `feedback_less_pushy_notifications` — ambient over interruptive
- `project_household_is_resilience_unit` — Dunbar-by-design
- `project_graduated_recovery_authority` — community recoverability
- `project_bootstrap_to_elohim_security_gradient` — three-stage authority
- `project_hdi_no_get_links_in_validators` — HDI validator constraint

### §11.2 Prior specs extended/contradicted

**Extended:**
- `2026-04-21-elohim-core-graph-substrate-design.md` — graph substrate; this brainstorm adds standing as derived view
- `2026-04-19-p2p-dataplane-visibility-design.md` — household-scale resilience; standing-gated layers map onto L2-L4
- `2026-04-21-bootstrap-steward-authority-frame-design.md` — bootstrap social vs Stage 3 gradient

**Contradicted (must reconcile):**
- `2026-04-24-epr-phase-2b-design.md` §3.7 ("reach-gated subscription") vs §7 O2 (graph-grounded reach with two faces) — reconciliation in Appendix A
- `2026-04-22-reach-backfill-policy.md` — reach default is "community"; this brainstorm validates and extends

### §11.3 Persona stress-test docs

- `genesis/docs/content/elohim-protocol/social_medium/child/README.md`
- `genesis/docs/content/elohim-protocol/social_medium/activist/README.md`
- `genesis/docs/content/elohim-protocol/social_medium/content_creator/README.md`
- `genesis/docs/content/elohim-protocol/social_medium/community_moderator/README.md`
- `genesis/docs/content/elohim-protocol/social_medium/displaced_person/README.md`
- `genesis/docs/content/elohim-protocol/social_medium/elder/README.md`
- `genesis/docs/content/elohim-protocol/social_medium/refugee/README.md`

### §11.4 Related epics

- **Social-reach epic** (`genesis/docs/content/elohim-protocol/social_medium/epic.md`) — narrative addition: "Designing for Human Vulnerability" section between Part III and Part IV; thesis-line addition: "Our Attention Is Sacred"
- **Recovery epic** (M-series at `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`) — DNA signal stream convergence
- **Phase 4 / VF-GraphQL** — depends on Phase 3.5 standing/tending substrate being live before VF semantics layer adds nuance

---

## Appendix A — Phase 2B §3.7 vs §7 O2 reconciliation

**Current state:** §3.7 (lines 345-346) frames reach authorization as "reach-gated subscription"; §7 O2 (lines 576-578) reframes it as graph-grounded reach with two faces (author-side earning + receiver-side pre-authorization). The §7 O2 framing is the corrected one; §3.7 is stale.

**File reference error:** §7 O2 references `subscription_auth.rs`. The actual file is `reach_authorization.rs` at `elohim/elohim-storage/src/p2p/reach_authorization.rs`. Confirmed: `subscription_auth.rs` does not exist.

**Reconciliation actions for `2026-04-24-epr-phase-2b-design.md`:**

1. **§3.7 rewrite:** Replace the two-line "reach-gated subscription" framing with:
   > Reach authorization with two faces: (1) author-side earning at publish time — the signer must have earned the declared reach; refused puts never enter local storage or hit the wire. (2) Receiver-side pre-authorization classification — a topology decision: which scopes does this node have standing in? Receive-side per-message filtering is rejected as the email-collapse anti-pattern (memory pin `project_reach_earned_at_authoring`); putting filter cost on receivers breaks the human-scale contract. Reach is coupled to embodied responsibilities at every node — author-side earning + receiver-side pre-authorization both derive from graph topology. See §7 O2 for Stage 1 implementation pointer (Batch D.4).

2. **§7 O2 file path:** `subscription_auth.rs` → `reach_authorization.rs`

3. **New §6.4 — Trust as efficiency signal — the compute-burden gradient:**
   - Forward to this brainstorm artifact for full design rationale
   - Inline summary: 9 foundational principles (§2 here)
   - Trust→compute gradient table (§3.2 here)
   - Floor protections (§2.8 here)
   - Phase 3 compute-burden refinements (§8 here)
   - Phase 3.5 substrate proposal (§9 here)

4. **§6.2 trust-state additions:** Add cache-priority + peer-selection rows for Phase 3+ consumers — high-standing-author content cached eagerly; low-standing evicted first; cold-fetch peer selection biased by standing.

---

## Appendix B — Aunt-and-rage-bait, end-to-end worked example

This narrative shows how the primitives compose. It is the integration-test scenario specified at P3.5.10.

### B.0 Bob joins the network

Bob signs up via a bootstrap-light onboarding path (his old phone, minimal local elohim, low-bandwidth). The onboarding manifest he signs declares the protocol's virtue commitments: truth, accountability, good-faith stewardship, paced attention, dignity. His acknowledgment is recorded as an `OnboardingAck` EPR.

> "Welcome, Bob. This is a virtue-centric network. It values X, Y, Z in its architecture. If you are bringing baggage from extractive platforms, you'll find friction here. That's the structure, not punishment. Welcome."

Bob's standing is initialized at the new-voice baseline floor (per §2.8). He participates in standing accountability from the first message he authors, regardless of his device's compute capacity. **Standing is discernment-gated, not compute-gated.**

### B.1 Bob authors rage-bait

Bob authors content with reach=district. The content is racist commentary disguised as cultural commentary. The reach-earning gate at his device's elohim allows the publish (he has district-scope membership through a community he joined yesterday). The envelope is signed; it enters local storage and the wire.

Bob's standing at this moment: new-voice baseline. He has no prior history. The post does not yet face any standing-asymmetry penalty.

### B.2 Aunt receives and re-shares

Aunt subscribes to Bob's district scope. The post arrives at her node (within reach, within her trust-bubble subscription). Aunt's elohim's tender specialist notices it during her morning catch-up.

The tender's author-time conversation does not fire here — Aunt is *receiving*, not authoring. The aggregate filter signals at this collective indicate moderate fatigue on takes about this topic, but Aunt's threshold is high; her elohim surfaces the post.

Aunt re-shares to her family group. Her elohim's tender *does* fire at this compose moment:

> "Aunt, this take from Bob is one you might want to consider. It's been propagating in the district for 12 hours and three peers have already squelched it (no public reasons given). The household had a rough conversation last week about a similar topic. Want to add context, narrow scope, or share with awareness?"

Aunt clicks through. She wants to share. Her elohim records her informed agency and the share proceeds. Her standing is staked on the share.

### B.3 Cousin Sarah receives, recognizes harm, corrects

Sarah is a graduate student in cultural studies. Her elohim's gate-discerner flags the content as potentially racist on first surface. Sarah confirms, opens her elohim's tender to compose a correction.

She authors a `Correction` EPR with citations from peer-reviewed literature. She publishes it with reach=district (her standing supports it; she has scholarly reach in this domain through her graduate program's collective).

She then issues a `FeedbackSignal {kind: correction, target_cid: <Bob's content>, evidence_cid: <her correction>}`. This signal:

- Travels backward via Primitive 2 — Sarah's elohim records that she received the content from Aunt; the signal forwards to Aunt (one hop)
- Floods forward via Primitive 3 — published on the gossipsub topic for Bob's content's reach scope so all current holders see the correction

### B.4 Back-prop walks the chain

**At Aunt's node:**
- Aunt's gate-discerner specialist verifies Sarah's standing-to-correct (graph walk: yes, scholarly reach, peer-reviewed evidence cited)
- Aunt's standing is debited per her household's standing-policy manifest (debit-soft for re-share with informed-agency record; debit-firm if the elohim's tender record showed she ignored a strong fatigue signal — it didn't)
- Sarah's standing is credited (correction-published)
- The `FeedbackSignal` forwards to Aunt's predecessor: Bob (one hop)
- Aunt stops propagating Bob's content

**At Bob's node:**
- Bob's gate-discerner verifies Sarah's standing-to-correct
- Bob's standing debited (origin-author position, debit-firm — racism is in the constitutional-floor un-filterable categories per the bootstrap manifest)
- The walk terminates (Bob is origin)

### B.5 Cost-asymmetry kicks in

Bob's next authoring attempt at reach=district fails the reach-earning gate. His standing has dropped below the threshold for that scope per the district's standing-policy manifest. He can still author at reach=community (his local trust-bubble); his speech is not silenced, only its amplification is now structurally costly to him.

Aunt's next compose-time tender conversation surfaces the recent debit. The elohim does not lecture; it offers context: "You shared Bob's content yesterday; Sarah's correction propagated. Want to acknowledge the correction, or proceed?" Aunt decides whether to publish a brief acknowledgment EPR (a restitution path) or move on. The choice is hers.

### B.6 Restitution paths

For Bob:
- He can publish a `Correction` EPR acknowledging the content was harmful and citing Sarah's correction
- Sarah may sign a `Vouch` EPR if she finds the acknowledgment good-faith
- Time-in-good-faith stewardship of others' content (squelch-free, attested by network engagement) gradually rewires his subgraph
- Mishpat does not need to intervene; the graph self-heals through Bob's own actions

For Aunt:
- A brief `Correction` referencing Sarah's evidence + apology to the household
- Cousin Sarah can sign a `Vouch` if appropriate
- Aunt's tender records this in her ongoing reflection log (private to her, not the network)

### B.7 Constitutional disclosure (only if invoked)

Suppose a hate-speech investigation is opened by mishpat (not local moderation; constitutional inquiry into a coordinated rage-bait campaign). The investigator wishes to walk the chain back to identify all participants in propagation.

Mishpat-quorum + Bob's imagodei (acting as Bob's counsel) + qahal of the investigating collective ratify the disclosure. Each peer in the chain (Aunt, Sarah, others) receives a request to disclose their predecessor link. Each disclosure is itself attested as an EPR. Aunt's elohim observes the disclosure, may file objections, and represents Aunt during the inquiry.

The disclosure is governance-mediated, recorded, traceable. No subpoena pressure on individual peers — the constitutional handshake is required.

### B.8 What this scenario demonstrates

- **Standing accountability fires at the discernment edge**, not at the peer's own compute (Bob's bootstrap-light onboarding does not exempt him)
- **Carrot before stick**: Aunt's tender conversation gave her informed agency at compose time; the receive-side feedback only fired because she chose to proceed
- **Edge-local back-prop**: the chain walks backward through local memory, never on the wire; privacy preserved
- **Constitutional revealability**: chain is recoverable but only through governance act; private by default
- **Restitution as graph mutation**: Bob and Aunt repair through *their own actions*, not by waiting for time to pass
- **Standing-immune floor**: Bob's speech is not silenced (he can still author at reach=community); his amplification is structurally costly to him
- **Anti-megalith mechanism**: cost-asymmetry distributed at every edge; no central moderator decided anything; the network's collective discernment self-heals

---

*"And now faith, hope, and love remain, these three, and the greatest of these is love."*  — 1 Cor 13:13

*Love rejoices in the truth (1 Cor 13:6). The protocol that enables this is the protocol that makes truth less expensive than lies — at every edge of the network, by design.*
