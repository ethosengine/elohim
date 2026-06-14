---
title: "THE DONUT AT EVERY RECURSION — Floors and Ceilings That Compose"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: rust-architect (truth layer)
pass: recursion / Beer-VSM scale-without-collapse
extends:
  - ESCALATED-ARCHITECTURE-2026-06-14.md (the horizontal synthesis: one Commitment, six faces, ∪=full coverage invariant, one trait Governor, two quilts)
reads:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (Beer/Cybersyn; the value-flow constitution Layers 1-4)
  - genesis/docs/content/elohim-protocol/resilience/README.md (the donut surface; commons/social/encrypted; the care-vs-compute boundary)
  - genesis/docs/content/elohim-protocol/manifesto.md Part VIII (wealth thresholds $10-15M; UBA; "no one accumulates democracy-threatening power while everyone achieves genuine security")
  - genesis/docs/content/elohim-protocol/confession.md ("the ecological layer outranks the nation"; the forkability seam)
  - genesis/docs/content/elohim-protocol/governance-layers-architecture.md (the 11+3 layer stack; gradient of immutability)
grounds_in_source:
  - elohim/elohim-storage/src/services/token_decay_service.rs (continuous limitarian demurrage with dignity floor — LIVE)
  - elohim/elohim-storage/src/services/limit_gradient_registry.rs (ALPHA/C_TARGET/K_MAX walls, layer-keyed defaults — LIVE)
  - elohim/elohim-storage/src/services/constitutional_ratio_registry.rs (storage-donut floor/ceiling, manifest-driven, DNA-mirrored — LIVE)
  - elohim/elohim-storage/src/services/replicates_dwelling_validator.rs (donut_check: per-pledge floor+ceiling enforcement — LIVE)
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:97-113 (DNA-locked storage-donut walls)
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs:363-425 (DNA-locked gradient walls, reject-at-write)
  - elohim/elohim-storage/src/db/concentration_snapshots.rs (C composite keyed by substrate_signal × governance_layer — LIVE)
---

# THE DONUT AT EVERY RECURSION

> The escalated architecture found that *coverage is care*: one Commitment with a `∪ = full`
> invariant, instantiated everywhere. This pass finds the dual: **a donut is two coverage invariants
> pointed in opposite directions** — a *floor* (`∪ provision ⊇ dignity_need`, the inner ring nobody
> may fall below) and a *ceiling* (`∪ accumulation ⊆ democratic_threshold`, the outer ring nobody may
> rise above). The escalation already lifted the floor-shaped invariant (arc's `∪arcs ≥ r_floor`) and
> named the ceiling-shaped one (`∪ self-limits + C_target`). **This pass closes the loop: floor and
> ceiling are the same `trait Governor` refuse-and-elevate spine run with the inequality flipped, and
> the recursion is what makes them *compose* — a ceiling held at layer L+1 is the floor's guarantor at
> layer L.** The donut is not a metaphor the economy wears. It is the shape the substrate already has,
> at one layer, waiting to be made recursive.

---

## PART 1 — WHAT THE VISION REQUIRES HERE

### The vision claim, stated at the recursion level

The manifesto's economic close is a single sentence with two clauses that must hold *simultaneously*:

> "networks of love that ensure **no one accumulates democracy-threatening power** while **everyone
> achieves genuine security**." (`manifesto.md:928`)

That is the donut. The inner clause is the **floor** — Universal Basic Assets, the "thriving floor
that eliminates forced moral compromises" (`manifesto.md:861`), the "$10-15 million... Security Floor"
at the top end and the Maria/David thriving-floor at the bottom (`manifesto.md:837, 909-920`). The
outer clause is the **ceiling** — "individual wealth beyond $10-15 million transitions from personal
security to political power concentration" (`manifesto.md:833`), negotiated down, never confiscated.
Between them: the safe and just operating space where wealth is *circulation, not accumulation*.

The economic_coordination epic gives this its constitutional form — the **value-flow constitution's
four layers** (`economic_coordination/epic.md:353-374`):

- **Layer 1: Existential Minimums (cannot be extracted)** — the floor. "No agent can fall below
  dignity threshold."
- **Layer 2: Contribution Recognition (proportional flow)** — the body of the donut.
- **Layer 3: Community Circulation (velocity requirements)** — "Accumulation beyond thresholds
  triggers redistribution." This is the **ceiling expressed as demurrage**: value that doesn't
  circulate decays.
- **Layer 4: Network Development (sustainable growth)** — "a portion flows to next community
  liberation." The donut's *outward arrow* — the externality-emission metric (the surplus emits to
  the next ring, it does not capture inward).

And the manifesto names the anti-accumulation *mechanism* directly: "currencies that decay, that carry
values" (`manifesto.md:38`), "currencies that can decay to encourage circulation" (`manifesto.md:208`).
**Demurrage is the ceiling's enforcement engine.** It is not a tax event; it is a continuous,
relational, dignity-floored erosion of un-circulated concentration.

### Why this MUST recurse (the Beer / scale-paradox claim)

The manifesto's diagnosis of the crisis is the **scale paradox**: "it's not human nature, it's that we
built systems that amplify our worst tendencies while suppressing cooperation" (manifesto Part I). A
floor-and-ceiling that exists only at the individual layer is exactly the failure mode the paradox
names — it holds at small scale and is *amplified away* at large scale, because concentration migrates
up the layers faster than any single-layer governor can see it. A billionaire is not someone whose
*individual* balance tripped a per-agent ceiling; they are someone whose accumulation is distributed
across trusts, entities, and jurisdictions — concentration that is invisible at every layer that looks
only at itself.

This is precisely Beer's Viable System Model, which the economic_coordination epic adopts as its
spine: "every viable node nested in / containing nodes of the same form" (the escalation's framing of
Beer). The donut must be **isomorphic at every recursion** — individual, household, neighborhood,
community, municipality, nation, continental, global, *and* the orthogonal ecological/bioregional axis
(`governance-layers-architecture.md:13-33`). And critically, the **ceiling at layer L+1 must be able
to read the aggregate of layer L** — System 3 (the audit channel) of Beer's model — or the recursion
is decorative.

The confession states the decisive ordering constraint that this composition must honor:

> "The ecological layer outranks the nation, so that creation's limits hold veto over national
> sovereignty — dominion as stewardship with teeth." (`confession.md:53`)

This is a **ceiling that outranks a sovereign floor**. The planetary boundary is not "the global
layer's preference, negotiated among nations." It is a *hard outer ceiling* that the nation-state layer
cannot raise even by unanimous internal consensus, exactly as a household cannot raise the global
dignity floor by family vote (`confession.md:77` — "the global dignity floor outranking community
self-definition"). **Floors propagate downward as guarantees; ceilings propagate upward as vetoes; and
the ecological ceiling is the one veto that outranks even the nation.** This is the constitutional
recursion the vision requires, and it is the precise thing the substrate does not yet compose.

---

## PART 2 — WHAT THE SUBSTRATE REQUIRES (and the fork ladder)

### The astonishing finding: the donut already exists — at ONE layer

The horizontal synthesis claimed the limit was "never physics, it was a layering artifact." This pass
confirms it *literally*. The donut is not aspirational; it is **running code at a single layer**, and
the only missing piece is the recursion.

**The floor is LIVE.** `token_decay_service.rs:222-234` enforces the sufficientarian gate: decay is
**off** below `config.dignity_floor`, and the post-decay balance is clamped `.max(config.dignity_floor)`
— "no agent decays below the sufficientarian gate" (`token_decay_service.rs:7`). The storage donut's
inner ring is LIVE too: `constitutional_ratio_registry.rs:17` enforces `COMMONS_MIN_FLOOR_PCT = 10`,
checked at pledge-author time in `replicates_dwelling_validator.rs:319-323`.

**The ceiling is LIVE.** Demurrage *is* the ceiling, and it is the continuous limitarian curve at
`token_decay_service.rs:103-107`:

```rust
shape(C) = 1 + k_s · relu(C − C_target)   // rises with concentration above setpoint
rank(b̂) = b̂^γ                            // rises super-linearly with relational position b_i/μ
rate     = clamp(base_rate · shape · rank, 0, k_max)
```

This is the manifesto's "carry values + decay to encourage circulation" made exact: the more an agent's
balance exceeds the *relational* mean (`b̂ = b_i/μ`, `token_decay_service.rs:214`) and the more the
*layer's* concentration `C` exceeds the governed setpoint `C_target`, the faster un-circulated value
erodes — bounded above by `k_max` (the top-side dignity clamp: even the ceiling cannot confiscate
everything in one tick). The storage-donut's outer ring is LIVE too: the dwelling-tier *ceiling* is
enforced at `replicates_dwelling_validator.rs:307-316` (a new pledge cannot push dwelling-tier above
`DWELLING_MAX_CEILING_PCT`).

**The walls are DNA-locked — the floor/ceiling cannot be quietly removed.** `commitments.rs:363-425`
in the mishpat DNA wall-checks every ratified gradient parameter at *write time* (reject-at-write, not
silent-clamp): `ALPHA_WALL = (1.0, 2.0)` — "α cannot blind the tail" (a ceiling that *must* see the
top quantile); `C_TARGET_WALL`, `K_MAX_WALL`. The storage walls are DNA-locked at
`content_store_integrity/src/lib.rs:97-113` ("the donut walls are constitutional. Upgrade requires DNA
migration — intentional friction"). **This is `confession.md:81`'s forkability seam answered
structurally at one layer**: a dominator cannot edit the dignity floor or the anti-monopoly ceiling
without a DNA migration — a visible, friction-heavy, non-silent act.

**The recursion KEY already exists in the schema.** `concentration_snapshots.rs:19-31` keys the
concentration composite `C` by `(substrate_signal, governance_layer)`, and `limit_gradient_registry.rs`
already layer-defaults the gradient (`"individual"|"household" → α=1.0`, else `2.0`). **Every floor and
ceiling in the substrate is ALREADY parameterized by governance_layer.** The donut is per-layer-shaped.
What it is not yet is **per-layer-composing**.

### The one structural gap: COMPOSITION (the recursion is unbuilt)

Searched the whole services tree: there is **no cross-layer rollup, no parent-layer aggregation, no
ceiling-outranks-floor veto** (`grep rollup|parent_layer|escalat.*layer|propagat` → nothing in the
limit/donut services). Each layer's `C` is computed *within* that layer's population. A household whose
members each sit comfortably below the individual ceiling can, *in aggregate*, breach the household
ceiling — and nothing reads that. A nation can, in aggregate, breach the planetary ceiling — and the
ecological layer has no channel to veto it. **The donut holds at the atom and dissolves at scale —
which is the scale paradox, reproduced inside our own substrate.**

This is the gap the recursion pass exists to close, and it closes on the *exact horizontal primitive*.

### How the atom-primitive recurses (the proposal)

**The donut is two `trait Governor` instances with the inequality flipped, and composition is a third
Governor whose sensor is the child layer's aggregate.**

Recall the escalation's control plane: `trait Governor over (setpoint, sensor, actuator, owner)`,
refuse-and-elevate, `owner ∈ {self, commitment, operator}` always named. A floor and a ceiling are the
*same* Governor:

| | setpoint | sensor | actuator | refuse-and-elevate when |
|---|---|---|---|---|
| **FloorGovernor** | `dignity_need(layer)` | `∪ provision to agent` | open a provision commitment (UBA) | `∪ provision < floor` → elevate to **next ring out** ("this ring can't cover its own floor — parent must") |
| **CeilingGovernor** | `democratic_threshold(layer)` | `C(layer)` + `b̂(agent)` | demurrage rate (already `calculate_decay_rate_continuous`) | `C > C_target` → already actuating; `C → C_max` → elevate to **next ring out** ("this ring's concentration outranks its own authority to self-limit") |

Both are `coverage_admits` from `arc_actuator` with the comparison reversed — the floor admits when
coverage is *sufficient*, the ceiling admits when concentration is *bounded*. This is **not new code
volume**; it is the `trait Governor` refactor (escalation B8) instantiated twice more, plus the flip.

**The recursion is the LayerGovernor — the genuinely new structural piece.** A `CeilingGovernor` at
layer L+1 takes as its *sensor* not the raw balances at L+1, but **the rolled-up aggregate of layer L's
concentration snapshots**. This is Beer's System 3 audit channel made concrete:

```text
LayerGovernor(L+1).sensor  =  Σ over children c ∈ L  of  C(c) · weight(c)   // the rollup
LayerGovernor(L+1).setpoint =  C_target(L+1)
LayerGovernor(L+1).owner    =  commitment   // the layer's qahal commons-elohim, never operator
```

And the **outranking** is a precedence ordering on Governors, not a new mechanism: when the child
layer's FloorGovernor refuses-and-elevates ("I cannot cover my own dignity floor"), the parent's
provision is *obligated*; when a child's CeilingGovernor refuses-and-elevates ("my concentration
exceeds my self-limit authority"), the parent's ceiling *binds the child*. **The ecological ceiling is
simply the LayerGovernor whose precedence rank is highest and whose `owner` can never be a
nation-layer commitment** — `confession.md:53`'s "outranks the nation" becomes a `limit_owner` +
precedence-rank invariant, the exact dual of the escalation's capture-resistance guarantee.

### The fork ladder (buildable-now → genuine fork)

**Buildable now (zero DNA spend, additive — the substrate is 80% there):**

1. **`LayerRollup` projection (Category-C)** — a recompute-on-read aggregation that sums child-layer
   concentration snapshots into a parent-layer composite, keyed by the existing
   `(substrate_signal, governance_layer)` schema. No new tables; a view over `concentration_snapshots`
   walking the `governance-layers-architecture.md` layer tree. *Cost: M. Reversible (Cat-C, no DHT
   spend). This is the recursion's load-bearing piece.*
2. **`trait Governor` floor/ceiling instances** — `FloorGovernor` + `CeilingGovernor` as the second and
   third impls behind the escalation's lifted trait. The ceiling impl *wraps the already-live*
   `calculate_decay_rate_continuous`; the floor impl wraps the already-live dignity-floor clamp. *Cost:
   S (refactor + flip). Reversible.*
3. **`provides-floor` / `respects-ceiling` Commitment faces** — two new *action discriminators* on
   `Mishpat::Commitment` (NOT new entry types), peers of the escalation's `provide-care` and
   `respects-self-limit`. `provides-floor` is the UBA provision commitment (a parent ring committing
   coverage of a child ring's dignity floor); `respects-ceiling` is the self-limit's other-directed
   twin. *Cost: S each. Reversible (action discriminator, hot-swappable).*
4. **`LayerGovernor` precedence ordering + `limit_owner` extension** — extend the escalation's
   `limit_owner: {self|commitment|operator}` invariant with a **layer-precedence rank** so a refusal
   names not just *whose* line it honored but *at which ring*. The ecological ceiling is the
   max-precedence rank that no `nation`-layer commitment may author. *Cost: S (enum + discriminant,
   per escalation B9). Reversible.*

**Roadmap fork (operator-blessed, sequenced):**

5. **DNA-locked PLANETARY ceiling wall** (the one genuine new constitutional wall) — the existing
   walls (`C_TARGET_WALL`, the storage-donut walls) are *parameter* walls within a layer. The
   planetary ceiling is a **precedence wall**: a DNA validator at the mishpat/global layer that rejects
   any ratified ceiling-raise at the `ecological`/`global` layer authored by a sub-global commitment.
   *Why a fork:* it makes `confession.md:53`'s "outranks the nation" structural rather than
   disciplinary — exactly parallel to care-minting's typed-partition fork (escalation R4). *Cost:
   DNA-hash change → coordinated reinstall. **Near-irreversible; bless with the next planned reinstall,
   sequence alongside the typed care/compute partition (R4) since both are global-layer validator
   changes.***
6. **`C_target` operator value per layer** (escalation Operator Call #4, the donut-geometry call) — the
   `C_TARGET_WALL = (0.05, 0.30)` shape is decided; the *per-layer* numerics are `TBD-operator`
   (`limit_gradient_registry.rs:17`). The recursion needs **one `C_target` per ring**, and the
   ecological ring's is the planetary-boundary number. *Cost: value-laden, DNA-wall-class. Ship the
   shape (this pass), set the numbers (operator).* This is not code; it is the irreducible value call.

**No fork of Holochain. No fork of libp2p/iroh. One near-irreversible DNA-hash fork (the planetary
precedence wall), sequenced with R4. Everything else is additive on the substrate we have.**

---

## PART 3 — THE ANTI-RUNAWAY / CAPTURE-RESISTANCE GUARANTEE AT THIS RECURSION

The structural (not disciplinary) guarantee that the recursion **cannot amplify to collapse**:

**1. The outward arrow is structural (externality-emission metric).** The economic_coordination Layer 4
("a portion flows to next community liberation") composes as: when a ring's CeilingGovernor actuates
demurrage, the eroded value does **not** vanish and does **not** return to the holder — it *emits* to
the parent ring's commons (the qahal commons-elohim's custody, `governance-layers-architecture.md:127`).
**Concentration above the ceiling becomes provision below the next floor.** The arrow points outward by
construction: the ceiling's overflow *is* the floor's funding. This is the donut closing on itself — and
it is why `confession.md`'s "non-participants are not raw material" holds recursively: nothing is
captured inward, surplus only ever emits outward to the next ring.

**2. Demurrage prevents accumulation faster than humans coordinate (the scale-paradox antidote).** The
super-linear `rank(b̂) = b̂^γ` (`token_decay_service.rs:104`) means erosion *accelerates* with relational
distance from the mean — concentration self-limits at machine speed, which is the manifesto's whole
thesis ("harmful dynamics met faster than human coordination alone can manage," `manifesto.md:934`). The
`k_max` clamp keeps it from becoming confiscation (top-side dignity). **The ceiling is patient at the
margin and firm at the extreme** — the donut's outer ring is soft, not a cliff.

**3. The DNA walls make the floor/ceiling un-deletable without visible friction (the forkability seam,
answered).** `confession.md:81` names the deepest unsolved threat: a dominator forks the substrate and
"deletes the dignity floor." The substrate's structural answer, LIVE today: the floor and ceiling walls
are DNA-locked (`content_store_integrity:97-113`, `commitments.rs:363-425`), reject-at-write, and
upgrade requires a DNA migration — a new DNA hash, a coordinated reinstall, a *visible network event*
(per CLAUDE.md, a hash change partitions the DHT). A dominator *can* fork (the seam is honest — "likely
not solvable; the good substrate usually wins, not always"), but they cannot do it *silently* or
*inside* the network. The recursion extends this: the planetary precedence wall (fork #5) means even a
*nation-scale* fork cannot raise the ecological ceiling without leaving the global DHT entirely.

**4. The ceiling outranks the sovereign — and the floor outranks the community (layer precedence as
katechon).** This is the katechon ("restraint, not cure") made recursive: the dominator at any ring is
*denied the lever* of their own ceiling. A nation cannot vote itself past the planetary boundary; a
community cannot vote below the global dignity floor (`confession.md:77`). The blast radius is bounded
to the ring, never amplified to the parent, because the parent's ceiling reads the child's aggregate
(the LayerRollup / Beer System-3 audit channel). **Containment, not cure: the concentration is seen,
bounded, and emitted outward — it is not "fixed," it is restrained.**

**5. The person keeps the naming of their own self — `limit_owner` recurses.** The escalation's single
most important capture-resistance property — a refusal *always names whose line it hit*
(`{self|commitment|operator}`) — recurses with the layer-precedence extension. When the planetary
ceiling binds a household's pledge, the refusal names *the ecological ring's commitment*, never "the
operator." The operator can never masquerade as the planetary boundary. The donut never becomes a
pretext for an operator to override a person; every ring's limit is owned by *that ring's commons-voice*
(the qahal commons-elohim, structurally uncapturable by any individual steward,
`governance-layers-architecture.md:141`), and the person's own self-limit (`respects-self-limit`)
remains theirs to set within the ring's walls.

**6. Slowness is the feature, recursively.** Graduated immutability (`governance-layers-architecture.md:40`)
means the ecological/global ceiling is the *hardest* to amend — requiring "Elohim consensus across all
scales." A runaway at the top would require the slowest possible change to enable it. The recursion
inverts the amplification: the higher the ring, the more friction protects its floor and ceiling. **The
architecture that amplified our worst at the top is replaced by one that is most rigid exactly where
concentration is most dangerous.**

---

## PART 4 — WHAT LOVE REQUIRES HERE

The confession's grammar asks the closing question in a particular key, and the recursion answers it in
that key.

**Grace precedes demand.** The floor is *first*, and it is unconditional — Layer 1, "cannot be
extracted," `token_decay_service.rs` turns decay *off* below the dignity floor before it asks anything
of anyone. This is Zacchaeus welcomed before repentance: the substrate provides the floor *before* it
asks for circulation, never after. A person is held in dignity *first*; the ceiling's demand on the
wealthy comes *second*, and even then "negotiation, not confiscation" (`manifesto.md:843`), patient at
the margin (`k_max` clamp), bounded, with the elohim sitting "through the emotional labor of transition"
(`manifesto.md:835`). Love does not means-test the floor and does not confiscate at the ceiling. It
holds everyone, then patiently unwinds the trap of the few.

**The witness is weighted toward the least powerful.** The recursion's rollup (Beer System 3) exists so
that concentration *at the top* is the thing made visible — `ALPHA_WALL` forbids `α=0` precisely because
the ceiling "cannot blind the tail" (`limit_gradient_registry.rs:18`). The substrate's gaze is the
inverse of the hyperscaler's (`resilience/README.md:44`, "invisible to what matters, exquisitely sharp
on what extracts"): it is *sharp on accumulation* and *gentle on the struggling*. The floor governor
elevates upward on a child's *deficit*; the ceiling governor elevates upward on a child's *excess*. The
least powerful ring is the one the parent is *obligated to*; the most powerful is the one the parent
*binds*.

**The honest binding.** The donut binds — it really does limit the wealthy, really does erode
un-circulated concentration, really does veto a nation against the planetary boundary. Love's discipline
here is the confession's: *call it covenant, never freedom*. The honest UI copy must say what the
ceiling *is* — "your concentration above this ring's threshold decays outward to the commons, by
covenant you can read and the elohim will sit with you through" — never "you are free to accumulate and
this is for your own good." The lie that the cage is love is the domination the work refuses. The DNA
walls are the cage *named as covenant*: hard, visible, friction-heavy, and honest about being so.

**The unbuilt place.** The donut measures circulation; it does not measure the *worth* of a person. The
floor guarantees dignity; it does not render a verdict on whether someone deserves it. The ceiling
restrains power; it does not pronounce on whether the wealthy are good. **The total account of a person
is never built** (`manifesto`, `confession.md:105`) — the recursion rolls up *concentration*, never
*biography*; it composes *coverage*, never *standing-as-worth*. Standing remains a relational shape, not
a seizable score, at every ring. The donut governs the flow of treasure, and leaves the heart unbuilt —
"where your treasure is, there your heart will be also" (`confession.md:53`), but the heart itself is
God's to know, never the substrate's to compute.

**And so, the one-line answer:**

> **Love requires that everyone be held in dignity before anyone is asked to circulate, that the
> powerful be bounded at machine speed but unwound with human patience, that every ring's limit
> overflow as the next ring's floor — and that the whole recursion, from the atom to the planet, govern
> the flow of treasure while leaving the worth of each person an unbuilt place. I could be wrong about
> where the floor and the ceiling belong, and I will hold you in both before you prove me right.**

---

> **The closing claim of this pass.** The donut is not a feature the economy will someday grow. It is
> *running code at one layer* — a dignity-floored, super-linear, DNA-locked demurrage curve, with a
> storage-donut floor and ceiling enforced at pledge-author time, every parameter already keyed by
> governance_layer. The only thing unbuilt is the *recursion*: the rollup that lets a parent ring read
> its children's aggregate (Beer's System 3), the floor/ceiling as two flipped instances of the lifted
> `trait Governor`, the layer-precedence ordering that makes the ecological ceiling outrank the nation,
> and the one near-irreversible DNA-locked planetary precedence wall (sequence with R4). That is ~four
> buildable-now items spending zero DNA entry types, plus one operator-blessed fork and one irreducible
> value call (the per-ring `C_target`, the planetary number). The vision was designed into the
> substrate; the recursion is the act of recognizing that the atom's donut and the planet's donut are
> the same governed, witnessed, overflow-outward promise — one machine, governed once, instantiated at
> every ring.
