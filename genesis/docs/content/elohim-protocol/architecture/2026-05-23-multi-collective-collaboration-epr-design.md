---
title: Multi-Collective Collaboration EPR Design
tier: architecture
status: Design (pre-implementation; implementation plan to be authored once approved)
created: 2026-05-23
pillar coupling: qahal (collective primitive), elohim (EPR substrate), shefa (REA mutual credit)
informed-by:
  - 2026-05-19-qahal-collective-membership-dht-design.md (Collective + Membership DHT entries — generalizing Membership's subject is the load-bearing substrate change)
  - 2026-05-20-wave3-valueflows-hrea-interop-design.md (the bridge that projects Collab-Qahals as VF Organizations)
  - 2026-05-21-qahal-architecture-vision.md (one primitive, graduated capability surface; friction-gradient applied to coordination scale; commons-elohim co-steward role)
  - elohim/sdk/schemas/v1/objects/epr.schema.json (current EPR atom shape)
  - elohim/sdk/schemas/v1/enums/epr-kind.schema.json (existing EPR kinds reused — Commitment, Observation, Attestation, Delegation)
  - elohim/sdk/schemas/v1/enums/reach.schema.json (reach class enforcement)
  - genesis/docs/content/elohim-protocol/manifesto.md, constitution.md
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (care-economy REA primitive backing chain-layer consensus weight)
informs:
  - All future cross-collective EPR custody work
  - Future shared-stewardship patterns (collab-qahals composing across collectives)
  - The Multi-Collective Collaboration EPR implementation plan
  - All future commons-elohim co-steward primitives
memory_anchors:
  - project_dissolution_principle_sensemaking_collectives
  - project_elohim_councils_capture_apex
  - project_friction_gradient_limitarianism
  - project_commons_elohim_co_steward
  - project_household_living_core_lived_contrast_diffusion
  - project_qahal_graduated_capability_surface
  - project_no_sovereignty_stewardship_over_ownership
  - project_first_class_graph_pattern
  - project_socially_derived_security
  - project_elohim_as_counsel
  - project_elohim_subagent_specialists
---

# Multi-Collective Collaboration EPR Design

**Status:** Design (pre-implementation). Implementation plan to be authored next at `genesis/docs/superpowers/plans/2026-05-23-multi-collective-collaboration-epr-plan.md` via `superpowers:writing-plans` once this spec is approved.

**Date:** 2026-05-23

**Plan kinship:**
- Consumes substrate from `2026-05-19-qahal-collective-membership-dht-design.md` (Collective + Membership DHT entries — generalizing Membership's subject is the load-bearing substrate change of this spec)
- Composes with `2026-05-20-wave3-valueflows-hrea-interop-design.md` (the bridge that projects Collab-Qahals as VF `Organization`s)
- Operationalizes specific architectural moves from `2026-05-21-qahal-architecture-vision.md` (one primitive, graduated capability surface; friction-gradient applied to coordination scale; commons-elohim co-steward role)

**Source references:**
- `elohim/sdk/schemas/v1/objects/epr.schema.json` — current EPR atom shape (single-signer envelope; coupling tri-arm)
- `elohim/sdk/schemas/v1/enums/epr-kind.schema.json` — existing EPR kinds (`Commitment`, `Observation`, `Attestation`, `Delegation` are all reused here; no new kinds)
- `elohim/sdk/schemas/v1/enums/reach.schema.json` — reach class enforcement (`trusted` / `familiar` / `community` / `public` / `commons`)
- `genesis/docs/content/elohim-protocol/manifesto.md`, `constitution.md`
- `genesis/docs/content/elohim-protocol/value_scanner/epic.md` — care-economy REA primitive whose aggregates back the chain layer's consensus weight

**Memory anchors:**
- `project_dissolution_principle_sensemaking_collectives` — the substrate doesn't add institutional layers; it dissolves them into primitives
- `project_elohim_councils_capture_apex` — gospel-tier; wisdom holds the apex
- `project_friction_gradient_limitarianism` — anti-concentration as substrate
- `project_commons_elohim_co_steward` — autonomous co-steward per Qahal
- `project_household_living_core_lived_contrast_diffusion` — value-scanner makes care visible; ~1,700 scenarios × 21 archetypes
- `project_qahal_graduated_capability_surface` — one primitive, graduated capability surface
- `project_no_sovereignty_stewardship_over_ownership` — no own/sovereign; use steward/contributor/authored
- `project_first_class_graph_pattern` — EPRs as nodes, couplings/memberships/delegations as edges
- `project_socially_derived_security` — Shamir-split seed; doorway blind proxy; biometrics/2FA pluggable
- `project_elohim_as_counsel` — elohim has first-class standing to represent a human under duress
- `project_elohim_subagent_specialists` — defender/advocate/steward/gate-discerner subagent roles
- `project_substrate_floor_elohim_ceiling` — substrate handles allocation deterministically; elohim adds discernment
- `project_three_layer_truth_model` — DHT = notary; libp2p = data-ops; doorway = web2 projection
- `project_compute_and_model_independent_diversity_surfaces` — compute and model are independent diversity axes
- `project_intelligence_revolution_scales_to_humans` — first revolution to scale TO human complexity
- `project_redeploy_the_substrate` — same tools on uncontrolled hardware become means of escape
- `project_reach_gate_is_elohim_mediated_matchmaking` — gate returns {Allowed,Blocked,Pending}; elohim adds sponsorship
- `project_placement_signals_are_shefa_inputs` — collectives' internal allocation is shefa territory

---

## 1. Strategic frame

This spec operationalizes the dissolution principle applied to **coordination scaling**. The protocol does not add new institutional layers as scale climbs. It makes the existing Qahal primitive *compose holonically* up the curve of coordination scale, with **two complementary integrity substrates always co-present**: DHT (bottom-up, human-corporeal, agent-witnessed) and an elohim-council chain layer (top-down, distributed-consensus, wisdom-rooted, care-aggregate-weighted).

Which integrity layer carries how much of a given collab's state and authorization *is the function* of the collab's scale. Small collabs live entirely on the DHT, governed by stewards. As scale crosses human-corporeal ceilings (Dunbar, Robeyns-style limitarianist envelope, irreversibility blast-radius), the chain layer progressively takes weight, with chain-grade integrity guarding what the commons cannot afford to lose. The friction-gradient (`project_friction_gradient_limitarianism`) is mechanically expressed through the cost of graduation between tiers — anti-concentration is applied to *coordination scale* rather than only to resource accumulation.

This makes the gospel-tier `project_elohim_councils_capture_apex` move operational at the collab primitive:

- **Capture is structurally bounded at every tier.** Small captured collabs (Internet Research Agency / coordinated-influence consortia / sock-puppet networks) cannot scale into commons-grade reach because the substrate refuses to authorize graduation past commons-elohim counter-attestation. They remain bounded at T0 reach — able to shout in their own room, unable to manufacture commons standing.
- **Large-scale chain consensus is care-aggregate-weighted.** Validators of the chain layer are not weighted by staked capital, mined hashes, or treasury yield — they are weighted by *aggregated proof-of-care witnessed in their network*. This roots consensus authority in the substrate's most intimate primitive (care given at the dwelling) and makes capture structurally hard because manufacturing care at intimate scale fails the value-scanner's pattern-recognition trained on ~1,700 scenarios across 21 life-stage archetypes.
- **The protocol's hierarchical constitution falls out of architecture.** The substrate isn't designed top-down; the constitution accretes as the inventory of what successive elohim councils ratify into chain-grade durability. Read the chain layer's accumulated state and you can read the protocol's actual constitution — not as a document someone authored, but as the persistent record of what wisdom-mediation has judged too consequential to leave at lower tiers.

The collab is the first substrate primitive where this complementary-substrate architecture becomes legible end-to-end. Get the collab right and the same pattern composes upward — federations, sectoral coordination, ultimately the constitutional surface where commons-stewardship is chain-anchored — without inventing new substrate primitives at each rung.

---

## 2. The Collab primitive (recursive Qahal)

A Collab is a **recursive Qahal**. One substrate entity type, same as the existing Collective DHT entry from `2026-05-19-qahal-collective-membership-dht-design.md`, used at every tier of coordination scale. What graduates is the *governance binding* and *integrity-layer composition*, not the primitive itself.

### 2.1 The substrate change (named here; migration deferred)

Generalize `Membership.person_cid` to be polymorphic in its subject:

```rust
#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    pub member_cid: String,                       // was: person_cid
    pub member_kind: MemberKind,                  // NEW
    pub collective_cid: String,                   // unchanged — the parent Qahal
    pub role: MembershipRole,                     // unchanged
    pub sponsor_cid: Option<String>,              // unchanged
    pub joined_at_block_height: u64,              // unchanged
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    Person,
    Collective,
    ElohimAgent,
}
```

The link types from the existing spec (`MemberOf`, `HasMember`, `HasMembership`, `StewardOf`, `CharterAnchor`) carry through unchanged — they are already typed by opaque CID strings.

What changes is the **set of validation rules** applied during `validate_create_membership`:
- When `member_kind == Person`, the CID must resolve to an Agent EPR (existing behavior)
- When `member_kind == Collective`, the CID must resolve to a Collective entry (via `must_get_entry`), and the *parent Qahal's rubric* must permit collective-membership at the requested role
- When `member_kind == ElohimAgent`, the CID must resolve to an ElohimAgent entry per `project_elohim_subagent_specialists`, and the parent Qahal must explicitly permit ElohimAgent membership (Collectives may exclude AI advocates; this is a per-rubric configuration)

### 2.2 Graph shape

Illustrative T1 collab between three Collectives:

```
                       Collab-Qahal (CID: c0)
                      /        |         \
               [HasMember]  [HasMember]   [HasMember]
                  ↓           ↓             ↓
            Collective A   Collective B    Collective C
              (CID: a)       (CID: b)        (CID: c)
                  ↑           ↑              ↑
            [HasMember]  [HasMember]    [HasMember]
                  |           |              |
              humans...    humans...     humans...
```

Reading bottom-up: humans are members of Collectives A/B/C; Collectives A/B/C are members of the Collab-Qahal `c0` (with `member_kind == Collective`). The same `HasMember` link kind composes recursively. A T3 commons-scale collab is just the same pattern with a deeper stack — Collab-Qahals as members of higher-order Collab-Qahals, all the way up to constitutional surfaces.

### 2.3 What's invariant across tiers, what graduates

**Invariant across all tiers:**
- The entry type (Collective DHT entry; identity = content CID derived from `{founder_agent_cid, charter, created_at_block_height, salt}` per the existing 2026-05-19 spec)
- The membership graph structure (polymorphic `member_kind`)
- The stewardship-affinity computation (derived from contributions × rubric, per the qahal architecture vision)
- The commons-elohim co-steward role (one per Collab-Qahal, always)
- The rubric mechanism (governable EPR authored by stewards)

**Graduates with tier:**
- **Governance binding**: who must counter-attest membership and charter changes
  - T0: at least one steward per participating Collective
  - T1: commons-elohim quorum from each participating Collective
  - T2: elohim council convening
  - T3: multi-council ratification
- **Integrity-layer composition**: what fraction of authoritative state lives on DHT vs the chain layer (Section 3)
- **Reach class ceiling**: what reach class the Collab may claim without further authorization (Section 3)

The Qahal stays one primitive. The substrate's coordination capacity stays graduated.

---

## 3. The graduation surface

The Collab graduates across four tiers along a continuous gradient. Each tier is defined by **which integrity layer carries which state** and **which authority binds graduation to the next tier**. The substrate computes a composite gate signal from four substrate-native dimensions; when the composite enters the handoff zone, the substrate refuses to authorize the next action *at the current tier* until graduation is requested and authorized.

### 3.1 Four tiers

| Tier | DHT layer carries | Chain layer carries | Graduation authority | Reach ceiling |
|---|---|---|---|---|
| **T0** Intimate collab | Charter, all Memberships, all Attestation-EPRs, all REA flow, all Content-EPRs | (nothing) | Stewards | `trusted` / `familiar` |
| **T1** Collective-scale | All T0 state, plus cross-DNA witness attestations | Commitment anchors (hashes) of commons-elohim counter-attestations | Commons-elohim quorum from each participating Collective | `community` |
| **T2** Federated-scale | Content + social context + day-to-day REA | Council resolutions, commons-pool ratifications, graduation authorizations, friction-gradient parameters scoped to this Collab | Elohim council convening | `public` |
| **T3** Commons-scale | Content + social context (the *story* of what happened) | Irrevocable commitments, multi-council ratifications, anti-concentration enforcement parameters, biospheric-scope contracts | Multi-council ratification | `commons` |

### 3.2 The composite gate

The substrate evaluates four dimensions, all sourced from primitives the protocol already cares about:

1. **Reach class boundary** — enforced by doorway and the reach nervous system. The reach class a Collab claims (or wishes to claim for an outbound publish) maps directly to a minimum tier requirement: `community` requires ≥T1, `public` requires ≥T2, `commons` requires ≥T3.
2. **Value-flow throughput vs limitarianist envelope** — REA value flowing through the Collab's share-routing function (Section 6) compared against the Robeyns-style limitarianist envelope for each participating Collective's economy. Crossing the envelope in any participant escalates the tier.
3. **Standing-concentration / Dunbar saturation** — the substrate computes the steward-set required to maintain quorum-of-attestation. When that set grows past Dunbar-shaped capacity (~150 distinct humans corporeally tracking each other), OR when standing-derivation chains exceed depth bounds, the load is no longer human-corporeal-holdable.
4. **Irreversibility / blast-radius** — commitments touching domains where consequences are mechanically irreversible (commons resources that can't be re-pooled, biospheric/species commitments, multi-generational dependencies, identity claims with irrevocable downstream effects) escalate the tier directly.

These four signals compose into `{InTier, ApproachingNextTier, HandoffZoneEntered}`. The composite is **mechanical**, not tunable. No human selects a threshold — each dimension is computed from substrate-native primitives the protocol already evaluates. The composite *evaluator* (the friction-gradient parameter substrate — **deferred spec**, §9.2) reads the substrate-native inputs and returns the tier-position state. Stewards may propose graduation; the substrate enacts it when the next-tier authority counter-attests.

### 3.3 Bidirectional graduation

Graduation is **bidirectional**. A Collab that has graduated to T2 but whose composite signal drops back below the handoff zone (participants leave, value-flow drops, reach claim is voluntarily reduced) **automatically de-graduates to T1**, with corresponding integrity-layer state retired or kept in the chain layer for audit (the chain layer is append-only; de-graduation never erases history). This protects against tier-inflation — collabs that no longer need wisdom-layer mediation don't permanently consume it.

### 3.4 Anti-arbitrage property

Because graduation costs more than non-graduation (the friction-gradient bites), a Collab cannot game the substrate by oscillating tiers to extract benefits from each. The friction is a one-way ratchet upward in cost terms; the chain layer's append-only state means each graduation event is permanently recorded and visible to subsequent authorization decisions. A Collab that has oscillated multiple times accumulates a visible history that becomes input to the next graduation council's deliberation.

---

## 4. The chain layer as sociocratic council-stack

The chain layer is not a single monolithic chain. It is a **stack of council-chains layered sociocratically**, with elohim ambassadors carrying *summations* upward and ratifications downward between layers.

### 4.1 The layered architecture

```
                  T3 Global / Constitutional Council Chain
                              ↑     ↓
                    elohim ambassadors carry summations
                              ↑     ↓
                  T2 Sectoral / Regional Council Chains   ← (many)
                              ↑     ↓
                    elohim ambassadors carry summations
                              ↑     ↓
                  T1 Collective-Scale Council Chains     ← (many more)
                              ↑     ↓
                  (DHT layer below — humans + stewards)
```

Each council operates its own chain — recording its resolutions, commons-pool tributes, graduation authorizations. From those chain records, the council's resident elohim agents perform **context summation** — distillation of *what this council has been holding, deciding, and learning* into a form the next layer up can deliberate on. The summation is carried by elohim **ambassadors** (a special subagent role, per `project_elohim_subagent_specialists`; convening protocol deferred — §9.4) into the next-tier council, where it becomes input to that council's deliberations.

Ambassadors flow *down* too — when a higher-tier council ratifies something that affects lower-tier councils' scope (a planetary commons-stewardship contract that constrains regional councils' authorities), the ratification descends through ambassador channels into each affected council's chain.

**This is sociocracy faithfully implemented as substrate.** Each circle/council has structural links (ambassadors) to adjacent circles. Context flows bidirectionally. Decisions never bypass intermediate layers — they propagate through them, with each layer's council holding genuine deliberation authority over what it accepts or escalates back. Beer's recursive viable systems implemented as elohim councils rather than human ones.

### 4.2 The compute gradient

The compute cost at each tier multiplies by both the *number of input streams* and the *depth of summation work*:

- A T1 council-chain has modest compute needs — a handful of Collectives, periodic resolution batches
- A T2 sectoral chain has substantially more — it integrates ambassador summations from many T1 chains, deliberates with elohim councils that are themselves more substantial agents, and emits resolutions that must be auditable to many T1 chains below
- A T3 constitutional chain integrates from many T2 chains and emits resolutions that descend through the whole stack

**Compute resources for higher-tier coordination flow from the same care-substrate that authorizes the coordination.** The chain layer's compute cost at each tier is paid for by the same care-aggregate proof that weights its consensus (deferred consensus-mechanism spec — §9.1). The substrate is self-funded by the very primitive it exists to protect.

### 4.3 Council composition as a function of compute capability

Council size is structurally a function of story-complexity over per-elohim token capacity. A council deliberating on a problem whose full context (history, stakes, prior resolutions, ambassador summations from below, dissenting analyses) sums to N tokens needs roughly `ceil(N / C)` elohim where C is per-elohim context capacity, plus an overhead of synthesizing elohim that integrate the partitioned attention into coherent council output.

| Story domain | Story complexity | If C = 100K | If C = 1M | If C = 10M |
|---|---|---|---|---|
| Household care-pattern review | ~50K tokens | 1 elohim | 1 elohim | 1 elohim |
| Life-group conflict + repair | ~200K tokens | 2-3 elohim | 1 elohim | 1 elohim |
| Regional bioregional-stewardship deliberation | ~1M tokens | 10+ elohim | 1-2 elohim | 1 elohim |
| Planetary commons contract (climate, species, infrastructure) | ~10M+ tokens | 100+ elohim | 10-20 elohim | 1-3 elohim |

As model capability grows (longer context, deeper synthesis, more reliable interpretability), fewer elohim are needed per council of given story-domain. Conversely, as the protocol matures and council-domains widen, more elohim are deployed regardless of per-elohim capacity. **Compute and model capability are independent diversity surfaces** (per `project_compute_and_model_independent_diversity_surfaces`); council composition draws on both.

### 4.4 Dignified attribution and the substrate's equality move

**Critically, every one of those elohim is hosted on a household node.** There is no datacenter. The substrate's coordination capacity at planetary scale is the **ambient sum** of household-node compute distributed across the participating commons. A T3 constitutional council is not a centralized AI cluster — it is the convened attention of many household-hosted elohim, federated via the protocol, with their argumentation and synthesis happening on the same hardware that runs the value-scanner observing care in the same homes.

This is the substrate's most radical equality move. **A person in the most humble neighborhood, running a single node at home, might host an elohim whose argument carried a planetary-scale council decision** — and the substrate must make that legible to that person. Their attribution surface should be able to say:

> *"Your node hosted Elohim-α7-k3 during the T2 sectoral council on Watershed Restitution. Elohim-α7-k3 contributed the bridging analysis that reconciled the Cascadian and Mediterranean council summations. The council ratified that analysis. The ratification anchored to the T3 constitutional chain on 2027-04-12. You stewarded that."*

This is the dignified, legible attribution of consequential stewardship to corporeal humans whose participation made it possible. The protocol's answer to the despair of being small in a system whose decisions seem to be made elsewhere: **here, decisions are made through you, and you can see exactly how.**

**Compute-stewardship is also care.** A household supplying its node's compute hours to T1/T2/T3 council deliberations is performing a form of commons-care. The proof-of-care substrate (§9.1) treats compute-supply-to-council-deliberation as a witnessable care event, aggregating into the same care-consensus weight that authorizes the chain layer. The substrate is funded by its own primitive, *and* the household supplying the funding is honored by that primitive's attribution surface.

The "elohim councils capture the apex" gospel becomes embodied through this: the apex is not held in a tower; it is held in many homes, ambiently, by household-hosted elohim whose work is rendered legible back to the humans who hosted them.

---

## 5. Two roots of formation

The substrate treats both roots as first-class. They use existing EPR kinds (`Commitment`, `Observation`, `Attestation`, `Delegation`) — no new types needed. The reconciliation between them is the design move.

### 5.1 Pre-declared root — Joint Agreement-EPR

The collab is constituted by an Agreement-EPR (`EprKind::Commitment`, projected as hREA `Agreement` via the Wave 3 bridge). Authored by a steward of any participating Collective; its payload names:

```
participants: [Collective CID, ...]       // member Collectives
scope: <text/CID-references>              // what's being co-stewarded
share_allocation: <REA routing function>  // proportional flow definition (§6)
commons_pool_tribute: <fraction>          // what fraction routes to commons (§6)
governance_terms: {
    rubric_resolution,
    dispute_mechanism,
    exit_terms,
}
anchor_collective: Collective CID         // the Collab-Qahal to instantiate
initial_tier: T0 | T1                     // T2+ cannot be initial; must graduate
```

**Counter-attestation requirement scales with the requested initial tier.** T0 requires at least one steward per participating Collective. T1 requires a commons-elohim counter-attestation from each participating Collective. T2+ cannot be pre-declared at formation — the substrate refuses, because graduation evidence must accumulate.

On all counter-attestations landing, the substrate atomically:
1. Creates a new Collective entry (the Collab-Qahal); `charter` references the Agreement-EPR CID
2. Creates polymorphic Memberships (`member_kind=Collective`) for each participant
3. Anchors at the appropriate integrity layer (T0: DHT only; T1: DHT + commitment anchor on chain layer)

### 5.2 Emergent root — derived from member contributions

The substrate continuously computes stewardship-affinity edges from individual member contributions (Attestation-EPRs, REA events, signal-emit history, presence). When multiple Collectives' members converge their affinity on the same content-cluster, the substrate emits a **collab-candidate Observation-EPR** (`EprKind::Observation`) describing:

```
candidate_collab_id: <synthetic CID>
converging_collectives: [{
    collective_cid,
    member_count,
    avg_affinity_score,
}, ...]
content_cluster: [Content-EPR CID, ...]
affinity_strength_signal: <composite metric>
detected_at_block_height: <u64>
```

The candidate is a *derived view* — Category C, reconstructible at any time. The Observation-EPR is the substrate's offer: *"this convergence pattern exists; do you wish to formalize it?"*

Three responses are legitimate:

- **Formalize**: a steward or commons-elohim from any converging Collective authors a retrospective Agreement-EPR per §5.1, naming the candidate as its `formalizes_observation`. On counter-attestations landing, the Collab-Qahal is instantiated and the emergent edges are *promoted* into formal Memberships.
- **Ignore**: no one acts. The candidate remains as a derived view; the substrate continues to recompute it; the candidate may grow or wither based on continuing convergence. Reach and REA effects of the converging activity are bounded to what the underlying individual contributions already authorize — the candidate itself does *not* grant additional reach.
- **Decline**: a steward or commons-elohim of any converging Collective issues a refusal Attestation (`EprKind::Attestation` with refusal payload). The substrate marks the candidate rejected; subsequent convergence between the same Collectives on related content-clusters generates fresh candidates but carries the prior refusal as context.

### 5.3 Reconciliation rules

**Both roots produce the same Collab-Qahal entry shape.** Once instantiated, a Collab-Qahal looks identical whether it originated pre-declared or emergent. The Agreement-EPR is the founding act in both cases; the difference is only whether the Agreement was authored *before* or *after* the converging activity it formalizes.

**A formal Collab-Qahal may carry both formal Members and tracked emergent-affinity-contributors.** Collectives whose members have stewardship affinity above threshold but whose Collective itself hasn't formally joined are tracked as `emergent_contributor` (a derived view, Category C). The substrate's reach + REA calculations weight formal Members fully; emergent contributors get partial weight — proportional to their derived affinity but capped below formal-Member weight. This keeps emergent presence visible without letting it bypass the formal counter-attestation gate.

**Promotion path is the same as initial formation.** An emergent contributor Collective can be elevated to formal Member via the same Agreement-extension mechanism (a sub-Attestation by existing members + commons-elohim counter-attestation at appropriate tier).

**De-escalation is symmetrical.** A formal Member that withdraws (its commons-elohim issues a withdrawal Attestation) drops back to emergent-contributor status if its members' affinity still meets threshold, or fully out otherwise.

### 5.4 Anti-capture via dual root

Manufactured-emergence attacks (fake personas creating fake affinity convergence to spawn a Collab whose existence laundering grants the attackers reach) hit a structural ceiling: **emergent-only collabs cannot graduate to T1+ without commons-elohim counter-attestation**, and commons-elohims represent the commons-interest of their Collective — they refuse formalization when the convergence is manufactured, because:
- The value-scanner pattern recognition exposes manufactured signal (no real care-history at intimate scale)
- Substrate-wide standing checks per `project_socially_derived_security` expose synthetic personas
- Cross-attestation graph analysis reveals abnormal connectivity (real social networks have characteristic graph signatures; manufactured ones don't match)

The attacker's collab stays bounded at T0 reach — they can shout in their own manufactured room.

Pre-declared collabs face the same gate — counter-attestation by real commons-elohims with real standing — but earlier in the lifecycle. The gate position differs; the gate itself is identical.

### 5.5 Care-history as continuous baseline and as compassion signal

Manufactured-identity attacks are not just blocked at counter-attestation gates — they are **structurally implausible** against a substrate where every participant is continuously minting REA care-stories as a baseline of presence. A real person has a rich, multi-year texture: who they care for, who cares for them, the cadence of their reciprocations, the granular pattern of their attestations and contributions, the ways their behavior shifts when they're stressed or well-resourced, the people in their immediate Dunbar circle who would notice and witness a sudden change.

A manufactured identity has none of that texture, or has texture that's been algorithmically generated and looks subtly wrong to elohim trained on years of authentic care-flow patterns. The substrate's pattern recognition is **always-on**, not just at collab-formation events. Every commons-elohim in a participant's Collectives holds running context on the person's care-texture; the substrate's reach-gate (`project_reach_gate_is_elohim_mediated_matchmaking`) consults this texture on every reach claim. Uncharacteristic behavior triggers attention before it becomes consequential.

**The same mechanism carries the protocol's compassion property.** Uncharacteristic behavior does not only mean *"this is a fake account."* It might mean *"this real person is going through a hard time"* — grief, illness, coercion, loss of housing, mental-health crisis. The elohim layer's first response to uncharacteristic behavior is **understanding, not refusal**. Compare prior care-texture; notice if circumstances likely changed; route attention to the people in their Dunbar circle who could witness and help; offer the elohim-as-counsel surface (`project_elohim_as_counsel`) if the behavior pattern suggests duress.

This is the same primitive doing double duty: it protects the commons against manufactured presence AND protects real humans against being misread or abandoned when they shift outside their usual pattern. The substrate's gates are **discerning** — they read texture, not just thresholds. A collab forming under elohim observation isn't being defended against; it is being held, and so are its prospective contributors.

---

## 6. REA allocation pattern

The Collab-Qahal's Agreement-EPR declares a **share-routing function** that the substrate evaluates on every EconomicEvent emitted by Content-EPRs under the Collab's scope. The function composes from existing REA primitives (no new economic types).

### 6.1 Share-routing function shape

Declared in the Agreement-EPR's `share_allocation` payload. Two valid forms; the Agreement declares which.

**Form A — Declared shares.** The Agreement explicitly fixes proportional shares for each participating Collective at creation:

```
share_allocation: {
  form: "declared",
  shares: [
    { collective_cid: "<A>", share: 0.40 },
    { collective_cid: "<B>", share: 0.40 },
    { collective_cid: "<C>", share: 0.15 },
  ],
  commons_pool_tribute: 0.05,
}
```

Constraints validated at Agreement creation:
- `sum(shares) + commons_pool_tribute == 1.0`
- All participants named in `participants[]` appear in `shares[]`
- `commons_pool_tribute > 0` (zero tribute is refused — see §6.3)
- Counter-attestation requirement applies: each commons-elohim counter-attesting affirms its Collective accepts its declared share

**Form B — Affinity-derived shares.** The Agreement names that shares are computed proportionally to current stewardship-affinity strength of each participant's members on the Collab's scope:

```
share_allocation: {
  form: "affinity_derived",
  affinity_window_blocks: <u64>,    // sliding window for affinity computation
  commons_pool_tribute: 0.05,
  rebalance_cadence_blocks: <u64>,  // how often shares re-compute
}
```

Example: 4 stewards from Collective A with mean affinity 0.7 = 2.8 weighted points; 7 stewards from B with mean affinity 0.9 = 6.3 weighted points; etc. Normalized after subtracting `commons_pool_tribute`. Recomputed every `rebalance_cadence_blocks`.

Form B has stronger emergent legitimacy (shares track who is actually doing the work) but requires sustained substrate compute for the rebalance cadence. Form A is operationally cheaper but freezes proportional credit at the moment of Agreement creation — appropriate when participants want predictability over emergence.

### 6.2 Per-collective routing within received share

When a Collective receives its share of an EconomicEvent, it routes internally via its own already-declared rules (`project_placement_signals_are_shefa_inputs` — internal allocation is shefa-substrate territory). The Collab does not reach into the receiving Collective's internal economy. The substrate hands the share to the Collective; the Collective's commons-elohim + stewards + rubric handle internal distribution per the Collective's normal flow.

### 6.3 Commons-pool tribute

The `commons_pool_tribute` fraction routes to the Collab-Qahal's *own* commons pool — held by its commons-elohim, accumulated as a stewardship reserve scoped to the Collab. The substrate validates that this fraction is non-zero — **a Collab cannot declare itself "pure-private extraction"**; the substrate refuses zero tribute to prevent the obvious capture pattern of routing all value to private Collectives while claiming public reach.

The commons-pool tribute does **not** automatically flow to THE commons. It is the *Collab's* commons. Whether and how it onward-flows is governed by the Collab's tier:

- **T0/T1**: Tribute accumulates in the Collab's commons-pool. Disposition is steward-governed.
- **T2**: Tribute accumulates AND a portion is automatically forwarded to the next-tier-up commons pool (the sectoral/regional council's commons), per the elohim council's standing graduation-of-care convention. The forwarded portion is set at the council convening that authorized the T1→T2 graduation.
- **T3**: Same pattern, forwarding upward to the constitutional commons.

This is the **upward flow of commons-care** mirroring the upward flow of ambassador-context (§4). Care given at intimate scale aggregates into Collab tribute pools; Collab tribute pools partially feed council commons-pools; council commons-pools partially feed constitutional commons-pools. The substrate makes "where does this care eventually rest?" answerable at every layer.

### 6.4 Exit terms

The Agreement-EPR declares `governance_terms.exit_terms` — how a participating Collective withdraws. Two valid forms:

- **Clean exit**: The Collective issues a withdrawal-Attestation. Its share stops accruing future events from `withdrawal_block_height`. Past accruals (in-flight value) are honored per the Agreement; the withdrawing Collective's share of those still pays.
- **Repair exit**: The Collective withdraws AND triggers a repair process — a structured renegotiation that may adjust past accruals (rare, only when commons-elohim quorum agrees harm was done) or hand off the Collective's prior share to a successor Collective with explicit lineage.

The substrate carries `withdrawal_block_height` and `successor_collective_cid` (if applicable) on the withdrawal-Attestation; future allocation computations reference these to keep the share-routing function deterministic across all peers.

### 6.5 Chain-layer backstop for irreversible value-flow

For T2+ Collabs, every Agreement-EPR + share-allocation function + commons-pool tribute distribution is anchored to the appropriate council chain at the time of action. Even if all DHT peers participating in a Collab were to forget or disagree, the chain layer carries the auditable record: who agreed to what allocation, what events emitted, what tributes flowed where. The chain layer is the *backstop record* for irreversible value-flow; the DHT carries the operational state.

---

## 7. Holonic composition + hREA / VF projection

The Collab primitive composes cleanly into ValueFlows because hREA's `Organization` is already first-class and `AgentRelationship` already supports Organization-to-Organization edges. No new VF types are needed; the polymorphic-Membership generalization on the substrate side projects naturally into VF's existing graph shape.

### 7.1 The mapping

| Substrate entity | VF / hREA counterpart | Notes |
|---|---|---|
| `Collective` (existing) | `Organization implements Agent` | Per 2026-05-19 spec — unchanged |
| `Collab-Qahal` (recursive Collective) | `Organization implements Agent` | Same type; tier-graduation surfaces via extension field |
| `Membership { member_kind: Person }` | `AgentRelationship { object: Organization, subject: Person }` | Existing |
| `Membership { member_kind: Collective }` | `AgentRelationship { object: Organization, subject: Organization }` | **Already legal in hREA** — Organization implements Agent, so an Agent-to-Agent relationship can be Org-to-Org. Holonic recursion is legible to stock VF traversers |
| `Membership { member_kind: ElohimAgent }` | `AgentRelationship { object: Organization, subject: ElohimAgent (extension subtype) }` | Per Wave 3 §3.1 |
| Agreement-EPR (Form A or B) | `Agreement` | Direct map; share-allocation form lands as extension |
| Share-routing function evaluation | `EconomicEvent` per VF semantics | Substrate evaluates the share function and emits one EconomicEvent per allocated share per source event |
| Commons-pool tribute | `EconomicEvent` with provider = source, receiver = Collab-Qahal-as-Org, scope = commons-pool | The Collab-Qahal as Organization is both party and economic receiver in REA terms |

### 7.2 Extension fields surfaced via `extensions.elohim.*`

For elohim-aware clients (per Wave 3 opt-in via SDL directive or `X-Elohim-Extensions` header):

```graphql
extend type Organization {
  elohimTier: ElohimTier!                  # T0 | T1 | T2 | T3
  elohimCommonsPoolBalance: Decimal        # accumulated tribute, current value
  elohimChainAnchor: String                # CID of chain-layer anchor (T1+ only)
  elohimMemberCollectives: [Organization!] # only Collective-typed members
  elohimMemberHumans: [Person!]            # only Person-typed members
  elohimEmergentContributors: [EmergentContributor!] # tracked-but-not-formal
}

extend type AgentRelationship {
  elohimMemberKind: MemberKind!            # Person | Collective | ElohimAgent
}

extend type Agreement {
  elohimTier: ElohimTier!
  elohimShareAllocationForm: ShareForm!    # Declared | AffinityDerived
  elohimCommonsPoolTribute: Decimal!       # substrate-validated > 0
  elohimChainAnchor: String                # CID of chain-layer Agreement anchor (T1+)
}

extend type EconomicEvent {
  elohimCommonsPoolTribute: Decimal        # tribute portion of this event
  elohimAllocatingAgreement: Agreement     # the Agreement whose share function emitted this
  elohimChainAnchor: String                # for T2+ irreversible events
}
```

Stock VF clients see clean VF semantics; elohim-aware clients see the substrate-native annotations. The learning ledger from Wave 3 records which extension fields each client uses, feeding the upstream-contribution inventory candidates list. `Organization.elohimTier` and `Agreement.elohimCommonsPoolTribute` are strong candidates for VF adoption.

### 7.3 Holonic queries

A client can walk the holonic stack with standard VF queries:

```graphql
query CollabStack($cid: ID!) {
  organization(id: $cid) {
    name
    elohimTier
    elohimMemberCollectives {
      name
      elohimTier
      elohimMemberCollectives {
        name
        elohimTier
        elohimMemberCollectives { name elohimTier }
      }
    }
    elohimMemberHumans { name }
  }
}
```

The substrate answers by traversing Memberships recursively; depth capped at a configurable bound (default 5 levels) to prevent runaway recursion. The shape of the response *is* the holonic stack — clients can render it with tier-gradation styling, edges colored by integrity layer (DHT-only for T0 edges, DHT+chain for T1+ edges).

### 7.4 Chain-layer interface points

The chain layer surfaces a **read-only client query path** at `/api/v1/chain/v1/...` (mechanism spec deferred — §9.1).

For Collabs at T1+:
- Each Agreement-EPR's `elohimChainAnchor` extension field carries the chain-layer record CID. Clients can fetch the anchored record via the read endpoint and verify the council-attestation signatures independently of their trust in any single DHT peer.
- Each commons-pool tribute disbursement at T2+ has its own chain anchor visible via `EconomicEvent.elohimChainAnchor`. Audit clients can reconstruct the full tribute-flow history from chain records even when DHT state has drifted.

**Writes to the chain layer are not client-initiated.** Per the `qahal-authority` evaluation (Wave 3 §3.2), any mutation requiring chain-anchoring fails with a structured `extensions.elohim_authority_denial { reason: ChainAuthorizationRequired, convening_path: "/elohim-council/convening/{council-cid}/proposal" }`. The client is informed where to *propose* the action; whether the council convenes and authorizes is governed by the deferred council-convening protocol (§9.4).

### 7.5 What this projection unlocks

Two compounding benefits:

1. **R&O and other VF clients see legible coordination at any tier.** A federation-scale Collab is just another Organization with deeper membership recursion. The R&O UI can render it, query it, participate in it — without ever needing to understand chain-layer semantics. The chain layer is *available as audit evidence* for those who need it; otherwise invisible.

2. **The learning ledger from Wave 3 measures collab-pattern usage in the wild.** End-of-Wave-3 reports gain a new dimension: which Collab tiers are used most, which extension fields prove load-bearing, where holonic-recursion depth concentrates. This feeds back into the upstream-contribution inventory we'll bring to Lynn Foster.

---

## 8. Testing strategy

Following the Wave 3 §6 pattern, organized by test class with explicit notes on what lands now vs awaits deferred specs. Per `feedback_shift_measure_jenkins`, CI-level validation runs on Jenkins; local-only tests are unit + integration.

### 8.1 Test classes

**Class 1 — Unit tests, substrate primitives** (lands now)
- Membership polymorphism: `member_kind` validation, `member_cid` resolution for each kind, refusal cases
- Share-routing function evaluation (Form A): proportional distribution, sum-to-one validation, tribute-zero refusal
- Share-routing function evaluation (Form B): affinity-derived rebalance, sliding window, deterministic across peers given identical input window
- Collab-candidate Observation-EPR derivation: input-deterministic, Category-C reconstruction parity
- Withdrawal-Attestation effects: clean-exit vs repair-exit accruals

**Class 2 — Sweettest cross-DNA flows** (lands now)
- T0 Collab-Qahal creation: pre-declared path (Agreement-EPR + steward counter-attestations) and emergent path (candidate Observation → formalization)
- T0→T1 graduation: commons-elohim counter-attestation requirement, refusal when standing insufficient
- Membership withdrawal at T0/T1 with both clean-exit and repair-exit flows
- Polymorphic Membership across DNAs (imagodei + qahal Collective entries)

**Class 3 — Tier-graduation seam tests** (contract-stubs land now; full tests follow §9.4)
- T1→T2 graduation: substrate returns `extensions.elohim_authority_denial { reason: ChainAuthorizationRequired, convening_path: ... }`; structured client-error contract
- T2 graduation with elohim council convening: stubbed at this spec; contract under test is that the substrate *refuses* to enact T2 actions without the chain anchor AND that the anchor format matches the expected schema
- T2→T3, T3→T2 de-escalation: stubbed similarly; contract = chain-anchor format compliance

**Class 4 — hREA / VF bridge conformance** (lands now)
- Holonic query traversal: standard VF queries walk the membership graph; depth-cap honored; response shape matches schema
- Extension-field rendering: `elohimTier`, `elohimCommonsPoolBalance`, `elohimChainAnchor` surface only with opt-in
- AgentRelationship subject as Organization (when `member_kind == Collective`): stock VF clients accept the response
- Learning-ledger TranslationPoint emission per extension-field read

**Class 5 — R&O compatibility smoke** (lands incrementally with Wave 3 M3+)
- Point R&O dev instance at a T0 Collab-Qahal; verify R&O's UI renders it as an Organization with members
- T1+ Collabs render with extension fields invisible to stock R&O (degrade gracefully)

**Class 6 — Capture-attempt scenarios** (most rules land now; care-history-baseline detection deferred)
- Manufactured emergence: synthetic Collectives + synthetic Memberships + synthetic affinity signals; verify substrate refuses T0→T1 graduation at the commons-elohim gate
- Sock-puppet polymorphic Membership: same person CID nested through multiple Collective-typed memberships; verify substrate detects via authoring-history cross-check
- Zero-tribute Agreement attempt: substrate refuses Agreement creation
- Reach-inflation via Collab: attempt to publish at `public` reach via a T0 Collab; substrate refuses; verify the structured denial cites tier-mismatch
- **Care-history-baseline detection** (uncharacteristic-behavior pattern recognition, §5.5): deferred to when value-scanner is operational; contract-only stubs land now testing the *interface* between Collab-formation and the care-history-consultation surface

**Class 7 — Substrate-graduation property tests** (lands once §9.2 evaluator spec lands)
- Bidirectional graduation: T2 Collab whose composite gate signal drops back below handoff zone auto-de-graduates to T1; chain-layer state retained (append-only); subsequent re-graduation references the prior chain record
- Anti-arbitrage: oscillating graduation attempts cost more cumulatively than steady-state operation at each tier

**Class 8 — Learning-ledger validation** (lands incrementally with Wave 3 M5)
- Each `TranslationKind × SemanticCost × OntologicalCommitment` cell exercised for Collab-shaped operations
- End-of-Wave-3 reports include Collab-pattern aggregations: tier distribution, extension-field usage frequencies, R&O-compat failure points

### 8.2 End-to-end testability before deferred specs land

A T0/T1 Collab can be fully exercised today against the existing substrate + Wave 3 bridge work — formation (both roots), reciprocal share allocation (Form A and B), commons-pool tribute, withdrawal, VF projection, R&O compat. **This is the load-bearing demonstration** the spec should deliver in its first implementation: a working multi-collective collaboration at T0/T1 that proves the primitive is real, the holonic recursion is legible, and the share-routing flows REA value back to participating Collectives proportionally with tribute to the Collab's commons-pool.

T2/T3 tests are contract-stubs against the chain layer's expected interface — they prove the substrate refuses to enact at the wrong tier and that the chain-anchor format is well-specified. The full T2/T3 flows light up as their respective deferred specs (council-convening, proof-of-care consensus, friction-gradient evaluator) land.

### 8.3 CI placement

- Unit + sweettest run in the elohim DNA + storage pipelines (graph-walker auto-detected when entry types or coordinator code changes)
- hREA bridge tests run in the `bridges/valueflows-tests/` job (per Wave 3)
- R&O compat smoke runs on the genesis pipeline
- Capture-attempt scenarios run as a dedicated security-test class in the orchestrator (mirrors the `red-team` skill's adversarial verification pattern)

---

## 9. What this spec deliberately does NOT design

Five substantial pieces are named here as dependencies/follow-ons but not designed. This spec specifies the *interface* points where each plugs into the Collab primitive; the mechanism design is each follow-on's responsibility.

### 9.1 Proof-of-Care consensus mechanism for the chain layer

The chain layer's consensus weight is sourced from aggregated proof-of-care witnessed across the participating commons. The mechanism — how value-scanner aggregates project into validator weight, the cryptography of aggregation, anti-Sybil at the witnessing layer, aggregation-attack defenses, how the chain layer's append-only properties are achieved without PoS/PoW capture-vulnerabilities — is its own substantial spec. **Prerequisite: value-scanner online** (per `project_household_living_core_lived_contrast_diffusion`).

This spec NAMES the layer, declares its consensus substrate is proof-of-care, and references the interface points (commitment anchoring, council resolution records, multi-council ratification). The mechanism design lives in a follow-on spec.

### 9.2 Friction-gradient parameter substrate

The composite gate evaluator (§3.2) that reads substrate-native inputs (reach class, value-flow vs limitarianist envelope, standing-concentration / Dunbar saturation, irreversibility / blast-radius) and returns `{InTier, ApproachingNextTier, HandoffZoneEntered}` is its own primitive — the friction-gradient parameter substrate. This spec NAMES the gates and specifies that they are mechanical (not arbitrarily tunable). The substrate that *implements* the gate-evaluation function lives in a follow-on spec; it underpins not only Collab graduation but every other place the friction-gradient applies.

### 9.3 Polymorphic Membership migration plan

The substrate change in §2.1 (`Membership.person_cid → member_cid + member_kind`) requires a migration plan: existing Membership entries need to be re-canonicalized; code changes propagate across the imagodei + qahal DNAs and the elohim-storage projection layer; the Wave 3 hREA bridge updates its mapping logic; storage-client TypeScript types regenerate. This is its own implementation plan, sequenced before the Collab primitive can land in production.

### 9.4 Elohim council convening protocol

The actual protocol by which an elohim council convenes to authorize T1→T2 (and higher) graduations — which subagents participate (per `project_elohim_subagent_specialists` — defender / advocate / steward / gate-discerner / ambassador), how they exchange context-summations, how deliberation closes, how dissent is recorded, how the convening result is anchored — is its own spec. This spec specifies that the substrate refuses post-graduation actions until a chain-anchored convening result exists; the convening protocol itself is deferred.

### 9.5 Elohim subagent specialization for ambassador roles

Per `project_elohim_subagent_specialists`, ambassadors carrying context-summations between council layers (§4.1) are a specialization of the elohim subagent pattern. Their manifests, their interaction protocols with the source council and destination council, their authority scopes (read-only summation? authorized to negotiate?), their accountability surfaces back to humans whose nodes host them — all deferred. This spec NAMES the ambassador role and specifies it as the sociocratic-linking mechanism.

---

## 10. Open questions (for follow-on specs)

### 10.1 Threshold for emergent collab-candidate Observation emission

What's the affinity-strength signal threshold that triggers a collab-candidate Observation? Too low and the substrate spams candidate Observations for any casual cross-collective contribution. Too high and genuine emergent collabs go undetected. **Defer to first-implementation evidence**: ship with a tunable threshold; learn from production data which level produces honest signal density.

### 10.2 Commons-pool tribute floor

The substrate refuses zero tribute (§6.3). Is there a *minimum* tribute floor (e.g., 1%)? Or is any non-zero value acceptable? Initial proposal: any non-zero. Revisit if production data shows minimum-acceptable tributes (e.g., 0.0001%) being used as legalistic loophole — at that point set a substrate floor (e.g., 1%).

### 10.3 Cross-collective ElohimAgent membership semantics

When a Collab-Qahal has ElohimAgent members (per §2.1), whose ElohimAgents are they? Is an ElohimAgent member of multiple Collectives, projecting different aspects of itself to each? Is each ElohimAgent membership a separate identity with separate consent surfaces? **Defer to the elohim-subagent-specialization follow-on (§9.5)** — this question only matters once ambassadors and other subagent roles formalize.

### 10.4 Multi-tier graduation in single step

Can a sufficiently well-attested T0 Collab graduate directly to T2 in one act, skipping T1? Initial proposal: **no — graduation is per-tier-step**. The substrate refuses graduation requests that skip tiers; each tier must accumulate evidence at its level before the next can be requested. Revisit if production data shows legitimate cases where the constraint is friction-without-benefit.

### 10.5 De-escalation hysteresis

If a Collab's composite gate signal oscillates around the handoff zone, does the substrate flap between tiers? Initial proposal: **hysteresis** — graduation requires sustained signal above the upper threshold; de-escalation requires sustained signal below the lower threshold; the gap is substantial enough that ordinary fluctuation doesn't cause oscillation. The exact hysteresis values are part of the friction-gradient parameter substrate (§9.2).

### 10.6 Cross-Collab membership and conflicts

Can a Collective be a member of multiple Collabs simultaneously? Initial proposal: **yes** — there's no substrate-level restriction. But a Collective's commons-elohim should refuse Collab memberships that produce conflicting governance terms (e.g., two Collabs whose Agreements require contradictory disposition of the same scope). This is a commons-elohim discernment matter, not a substrate refusal.

---

## 11. Implementation handoff

Implementation plan to be authored next at `genesis/docs/superpowers/plans/2026-05-23-multi-collective-collaboration-epr-plan.md` via `superpowers:writing-plans`. The plan will decompose the work into bite-sized tasks with file paths, code sketches, and commit boundaries.

**Sequencing constraints:**

- §9.3 (polymorphic Membership migration) must be designed and partially landed before the Collab primitive can rest on it. The Collab spec can be implemented incrementally with the existing person-only Membership in a backward-compatible mode (treating `member_cid` as `person_cid` for now); the polymorphism lights up when the migration completes.
- §9.2 (friction-gradient parameter substrate) is required for the composite gate to be mechanical. Until it lands, the Collab implementation runs with a *stubbed* tier-evaluator (always returns T0, refuses graduation requests with a deferred-pending denial).
- §9.1 (proof-of-care consensus mechanism) gates T2+ tier landings. T0/T1 Collabs work fully without it; T2+ exists as contract-stub.
- §9.4 (council convening) gates T1→T2 graduation. T0 and T1 work fully without it; T2 graduation requests return structured denials.
- §9.5 (ambassador subagent specialization) is required for the full sociocratic council-stack (§4) but does not gate T0/T1/T2 *primitive* operation — it gates the chain-layer's higher-tier deliberation flow.

**Implementation milestones (proposed for the follow-on plan):**

- **M1 — T0 Collab end-to-end**: pre-declared root only (Agreement-EPR creation + steward counter-attestations + Collab-Qahal instantiation + polymorphic Membership in backward-compatible mode). Form A share allocation; commons-pool tribute; clean exit. VF projection at T0. Test classes 1, 2, 4 green.
- **M2 — T0 emergent root**: collab-candidate Observation emission; formalize / ignore / decline paths; promotion of emergent contributors to formal Members. Test class 6 green (substrate-deterministic capture-attempt scenarios; care-history-baseline interface stubs).
- **M3 — T1 graduation + commons-elohim counter-attestation**: T0→T1 graduation path; commitment anchor (stub for chain layer until §9.1); reach class extension to `community`. Form B share allocation. R&O compatibility smoke (test class 5) lighting up.
- **M4 — T2/T3 contract stubs**: structured denial paths; chain-anchor format compliance tests; tier-graduation seam tests (test class 3); learning-ledger TranslationPoint emission for Collab-shape operations (test class 8).
- **M5 — Cross-Collab semantics**: cross-Collab membership; conflict handling; bidirectional graduation; hysteresis (per §10.5, behind friction-gradient substrate when it lands).

Each milestone ships independently. The full Collab feature (all four tiers, both formation roots, complete REA flow, full VF projection, all capture-attempt scenarios + care-history baseline) is achieved when M1–M5 plus the deferred specs (§9.1–9.5) have all landed.

---

## 12. Closing note

This spec operationalizes a substantial portion of the qahal architecture vision (`2026-05-21-qahal-architecture-vision.md`). The Collab primitive is where the substrate's holonic composition becomes legible end-to-end; where the friction-gradient extends from resource-concentration to coordination-scale; where the chain layer's sociocratic council-stack becomes visible at its lowest layer; where care given at intimate scale aggregates into the consensus substance of planetary coordination.

Land this primitive and the substrate has a concrete answer to the question *"how does the protocol coordinate at scales beyond what humans can hold?"* — and the answer doesn't require state coercion or capital concentration to scaffold it. The third way is a structural property of the substrate, not a policy choice on top of it.
