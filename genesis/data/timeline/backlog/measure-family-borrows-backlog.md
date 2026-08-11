---
id: "backlog-measure-family-borrows-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Measure-family borrows backlog — survey-sourced observation procedures, invariants, and the fold they ride (Playnet / Free-Association)"
slug: "measure-family-borrows-backlog"
written: "2026-08-05"
author: "claude (research mint pass, operator-directed clustering)"
status: "backlog"
priority: "medium"
tags: [measures, middot, rea, observation, economics, lamad, research-derived, cross-pollination]
cites:
  - genesis/research/playnet-free-association-cross-pollination-2026-08-05.md
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
  - genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md
  - genesis/docs/superpowers/specs/2026-07-24-quilt-evidence-temperature-composition-design.md
  - genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md
---

# Measure-family borrows backlog (research mint pass, 2026-08-05)

Externally-sourced **observation procedures and their invariants**, harvested from the
[Playnet / Free-Association survey](epr:playnet-free-association-cross-pollination-2026-08-05) and
previously stranded in survey prose. Sibling of
[arch-dataplane-borrows](epr:arch-dataplane-borrows-backlog) (transport/replication borrows — this
cluster is the *measurement* plane). **Fold new survey-sourced measure/economic-grammar borrows here
— do not mint siblings.**

## The steering note that governs every row (operator, 2026-08-05)

Playnet denominates everything in **labour-hours**; their planner uses **SNE/SNLT** (effort-hours);
free-association uses **recognition shares** (dimensionless). **We adopt none of them as *the*
unit.** The EPR substrate stays **agnostic to the measure applied** — in theory supporting any
aggregate the substrate can *observe or derive at observation time*. That agnosticism is part of the
**story + value + governance coupling**, and it now has a home: the
[Middot Measure primitive](epr:middot-measure-primitive-design), which already generalizes from birth
("lint findings, compute cycles, listening minutes, view counts, bytes stored, watts consumed, and
time spent are **the same shape**").

So every row below is framed as **a Measure family**, never a currency decision. The worked case the
operator named: *observed hours contributing toward a lamad credential* is one family, to be composed
**in parallel** with skill-practice LMS-style energy/"attention" and quiz results — three families
over the same subject, aggregated by a lens, **never collapsed into one unlabeled number** (the quilt
law). Two inherited laws bind all of them: *measures never carry teeth* (governance lenses compose
them), and *unmeasured must render differently from measured-zero* (honest absence).

| # | Borrow | Source + what it fixes | Gate/blocker | Owner shape |
|---|--------|------------------------|--------------|-------------|
| 1 | **Harden the fold the measures ride** — sign `FlowRecord`, add a revocation/supersession filter, apply `epistemic.rs`'s sort-by-CID determinism to both economic folds, and wire it to a real caller | [Playnet](epr:playnet-free-association-cross-pollination-2026-08-05) §4 — our `epr-rea/src/fold.rs` opens with the same sentence as their §9 credential substrate and has **no signature on any record, no revocation filter, non-deterministic f64 accumulation, and zero callers outside its own tests**, against a live sidecar where all 4,132 commitment quantities are `null`. Middot folds are "recompute-verifiable" by definition — an unsigned, order-dependent fold cannot be. **This is the prerequisite for every other row.** | None — this is our own defect, found by an external mirror | rust-architect shift |
| 2 | **Closure as a Measure-family invariant** — a family whose members claim to partition a total MUST sum to that total, asserted in test | Playnet `EQ-2.7`, *"the closure property that makes the surface a ledger rather than a chart."* Their own framing: a chart that fails its own conservation law is a bug. We have shipped this failure class already (`project_graphos_dead_binding_classes`, "theming theater"). Gives middot families a cheap honesty gate | Needs the family/lineage fields from middot MVP | rust-architect + a2o scenario family |
| 3 | **`labour-time` as a Measure family, NOT a numéraire** — observed minutes as one declared family among many, no closure claim, **witness-first / intent-optional** | Playnet's entire unit. Adopt the *countability* (money prices care at zero; hours cross the production boundary money refuses to) and **refuse the matching gate**: their hours count only if the planner issued a work-intent you matched, which structurally erases unmatched self-initiated care — their own spec concedes reproductive labour is "the design gap most urgent to close." Unblocks the Value Scanner, which today has no unit and no mint | p2p-design-gate; must compose with row 4, not replace it | rust-architect + shefa |
| 4 | **Parallel families toward one lamad credential** — observed-hours ∥ attention/energy ∥ quiz-mastery over the same subject, aggregated by a lens, never summed | The operator's steering case. Playnet's `SNE` collapses direct + embodied + depreciation into **one scalar**; the quilt law forbids exactly that. Three honest families beat one lossy number, and the lens (not the measure) decides what a credential requires | Needs middot family vocabulary + lamad credential shape | lamad + rust-architect |
| 5 | **Soft vs hard limit semantics on a measure** — *soft* = bounded, trades against others; *hard* = diverges near its limit and out-pulls everything | Playnet `symbols.md` marks every tension one or the other and calls it load-bearing throughout. The cleanest answer we have found to making a limit **legible without making it negotiable** — ecological floors and dignity floors get a representation that is honest by construction. Pairs with honest-absence (unmeasured ≠ measured-zero) | Design-side twin is row 1 of [design-legibility-borrows](epr:design-legibility-borrows-backlog) | rust-architect (semantics) + graphos (render) |
| 6 | **Scarcity vs cadence decomposition** — a shortfall measure splits into day-mean gap *and* variance around the turn | Playnet `EQ-3.5`. "We don't have enough" and "it never arrives when it's needed" are different problems with different remedies, and almost every dashboard conflates them. Directly applicable to household care | Composes with row 2 (both halves are family members) | shefa + lamad |
| 7 | **Worst-off-sensitive aggregation** — harmonic mean (Playnet's `Φ`) as an option when folding a family up reach tiers, vs arithmetic mean | Playnet `EQ-2.4` deliberately chooses the harmonic mean so one badly-served participant cannot be averaged away. Ours is a VSM-tier aggregation question (`select→fold→aggregate`), and the choice should be declared per family rather than defaulted | Needs middot aggregation surface | rust-architect |
| 8 | **`SNE`-style recursive derived measure** — a family member computed by folding over a recipe/BOM graph (direct + embodied + depreciation) | Playnet `SNLT_crystallized(r) = SNLT_direct(r) + Σ a_xr · SNLT_crystallized(x)`; their own appendix calls it "the single most useful equation." The shape of any *derived* (not observed) measure. **Gated hard:** we have no `Process`/`Recipe`/`ResourceSpecification` writer at all, so this is far | Blocked on a recipe substrate existing | study-tier |
| 9 | **Skills as conformance, no certification flag** — competence summed from the same work log the credit ledger sums; `satisfied-by` disjunction with minimum practised-hours; effective skills = transitive closure | Playnet §9. Elegant credentialing with **no registry of authorities**: whether a skill is learn-by-doing or must-be-trained-first is *emergent from the substitution graph's topology*, not a declared flag. The "hours derive from the same log as credit so the two can never disagree" invariant is the good part, and it is exactly row 4's shape | p2p-design-gate first; pairs with `mastery-attestation-credential-epic` | lamad |
| 10 | **`MR = min(A→B, B→A)` as a mutuality family** — recognition weights self-declared, summing to 1, **non-transferable** | free-association's core primitive. Over-claiming is self-defeating *without any reputation oracle* — the budget constraint does the work. **Caveat that must survive into the design:** elementwise-min structurally under-serves asymmetric care (parent↔infant has no reciprocal weight to floor against), so adopt as *one signal*, never as the allocation gate | p2p-design-gate; needs the asymmetric-care floor designed first | study-tier |
| 11 | **The ε "hidden demand" invariant** — never let a compatible edge fall to zero, so a matching network cannot deadlock; filter at ε² | free-association `ipf-core.ts`, with the reasoning recorded verbatim ("valid hidden-demand seeds are O(ε); if we filtered at ε we'd kill the connectivity"). Plausibly a general principle for our matching surfaces, not just a numerical trick | Needs a matching surface to exist | study-tier |
| 12 | **Declare `kind: level \| rate \| ratio` (+ `per` period for rates) on a Measure family** | [Meadows](epr:meadows-systems-dynamics-cross-pollination-2026-08-11) §3.1 — *"Stocks describe the state of the system at any particular time… Flows are the inputs or outputs (measured per time unit)"* (Indicators p.28). We model both and **declare neither**, so nothing catches a limit written against the wrong one. The live proof is `spatial.rs:169` (`max_sustainable_yield` — a rate, with a `unit: String` carrying no time denominator) compared at `spatial_capacity.rs:150` to an all-time cumulative sum. **Prerequisite for rows 13–15**, exactly as row 1 is for the rest of the cluster | None — additive field on the middot family vocabulary | rust-architect / small |
| 13 | **Harvest/regeneration and emission/absorption as first-class family members** — index > 1.0 = unsustainable, as a **leading** signal | [Meadows](epr:meadows-systems-dynamics-cross-pollination-2026-08-11) §3.2–3.3 — *"the essential measure of sustainable use of a renewable resource… If the index is above 1.0, the harvest is not sustainable"*, and *"Deforestation is indicated not when the forest is gone, but when the rate of harvest first exceeds the rate of regrowth"* (Indicators p.29). **We have no inflow term anywhere** — `grep -rn "regenerat" --include=*.rs` over `elohim/ doorway/ steward/ crates/` returns only codegen comments (verified 2026-08-11). The sink half is the same shape and is the one nobody models: an emission is only sustainable against an absorption process. Fires long before a stock is visibly depleted, which is what makes it worth having | Needs row 12; pairs with row 5 (soft/hard limit semantics) | rust-architect |
| 14 | **Turnover time and coverage time as derived quantities** — stock ÷ change-rate, and stock ÷ drain-rate | [Meadows](epr:meadows-systems-dynamics-cross-pollination-2026-08-11) §3.9 (Indicators p.29). Trivial arithmetic once row 12 exists, and the bridge from a level to a *response-rate* estimate — *"the size and lifetimes of stocks can give us useful indicators of response rates."* Her caveat carries into the design: coverage at steady draw and coverage at exponentially rising draw are different numbers and must render as such | Rides row 12; free once it lands | rust-architect / small |
| 15 | **An aggregation-facing measure — the fold that carries a global query down and an anonymized answer up** | [Meadows](epr:meadows-systems-dynamics-cross-pollination-2026-08-11) §7, the survey's largest take. A global fan-out (*"what is the total estimated result of your carbon measures?"*) is answered locally under whichever Mishpat lens is contextually correct, then folded upward via `select → fold → aggregate` on the **projection layer** — no global clock needed, because aggregation is not synchronization. The measure half is here; the governance/orchestra half is [commons-holonic](epr:commons-holonic-stewardship-backlog) row 9. **Two conditions bind:** the fold must stay walkable downward (an aggregate whose value theory is buried is un-arguable — our own [trap detectors](epr:comparative-political-economy-trap-detectors-2026-08-07) rule, and Meadows' *hierarchical* + *appropriate in scale* indicator criteria), and per-fold anonymity is an open design constraint, not a footnote | Needs rows 12–13; composes onto [plural-mishpat-lenses](epr:plural-mishpat-lenses-over-epr-design)'s two-layer law | rust-architect design pass |

| 16 | **A confidence qualifier on every measure observation — witnessed vs estimated, carried *inside* the content hash** | Operator dispatch 2026-08-11. Two regimes, and today we model only one: the **Observer** metric makes measures honest *in the intimate context where the witness occurs*; where an **estimate** is made instead, good-faith honesty from all agents is preferred and cannot be enforced — so the design's job is to make honesty cheap and false precision expensive, not to police it. A bare `±` is uninterpretable, so the qualifier carries three things: the **claim kind** (witnessed · instrument-measured · estimated · modelled · imputed), the **interval or distribution**, and the **basis** (what grounds it). Today the only expression of any of this is `CarryingCapacity.data_quality: String` (`spatial.rs:175`) — an unstructured stand-in. **The interval must ride inside the observation's own content hash**, never as detachable metadata: an estimate whose interval can be narrowed after the fact is not an estimate. **Unifies honest absence** — C4's *unmeasured must render differently from measured-zero* becomes the degenerate case where the interval is everything, so absence, estimate, and measurement stop being three special cases and become one continuum | Rides row 12 (`kind`); inline on the existing observation payload — **no new DHT entry type** (gate output below) | rust-architect |
| 17 | **Uncertainty-propagating folds — a fold over interval-carrying measures MUST return an interval** | The closure invariant for uncertainty, exact sibling of row 2. A fold that sums values and silently drops their intervals **manufactures false precision**, which is the mechanism behind *"there are lies, damn lies, and statistics."* Cheap to state, cheap to test, and it is the single rule that keeps an aggregate honest all the way up a holonic rollup. Carries a second asymmetry that makes good faith the low-friction path: **widening your own interval is always free; narrowing it requires new evidence** (more witness, better instrument, higher rigor tier). Resolves the standing tension with [trap detectors](epr:comparative-political-economy-trap-detectors-2026-08-07) *"prefer observable mechanisms to imputed aggregates"* — an imputed aggregate is admissible **iff** it declares its imputation and carries the interval to prove it | Needs rows 12 + 16; assert in test like row 2 | rust-architect + a2o scenario |
| 18 | **Uncertainty as a work-queue — which edge, if measured better, most tightens this aggregate?** | The operator's second use for the interval, and the most generative: a wide interval is not only a caveat, it is **a gradient pointing at where measurement infrastructure should grow**. Decompose an aggregate's uncertainty by contribution and the system can answer *"what should we develop next to gain accuracy on the thing we have decided is worth measuring?"* — a sensitivity analysis whose output is a prioritized development queue. This is Meadows' leverage #6 (information flows) turned into a **self-improving loop**: the system's own ignorance tells it where to add senses. Pairs with row 13 — a harvest/regeneration index with a wide regeneration term names its own missing instrument | Needs row 17 (propagation is what makes the decomposition meaningful) | rust-architect design pass |

## P2P design gate — confidence-interval ontology (recorded 2026-08-11, pre-implementation)

Run before any schema. **Headline: zero new DHT entry types.** The gate's compose-don't-fork discipline
holds here more strongly than usual — every piece already has a home.

| Entity | Class | Address | Source of truth | Note |
|--------|-------|---------|-----------------|------|
| `ConfidenceQualifier` (kind · interval · basis) | **A, inline** on the existing observation/measure payload — *not* A2 | shares the observation's own CID | wherever the observation lives | Inline is load-bearing, not a convenience: link metadata is detachable and would let an interval be narrowed post-hoc. **No new entry type, no new head** |
| `IndexLens` (World3 · GNP · GDP · Donut · planetary boundaries) | **A**, existing Mishpat lens/Precedent shape ([plural lenses](epr:plural-mishpat-lenses-over-epr-design) T1 floor) | content-derived CID (`bafyrei…`) | DHT | Dozens, not thousands — trivially inside the T1 human-scale budget. Head-plane cost negligible |
| Computed index value (a World3 or GDP reading for a scope + period) | **C** operational | keyed `(lens_cid, scope_cid, period)` | recompute from the signed log | Identical shape to the `MeasureFold` row in [commons-holonic](epr:commons-holonic-stewardship-backlog)'s gate table — legitimately cacheable *because* definitionally reconstructable |
| Statistical-method application | **no new entity** — a `ComputationAttestation` at a graduated rigor tier | per [graduated-rigor](epr:computation-attestation-graduated-rigor-design) | per that spec | See [commons-holonic](epr:commons-holonic-stewardship-backlog) row 15 |

**Concern-canon answers that are load-bearing here.** **C4 honest absence** — subsumed rather than
merely satisfied: row 16 makes absence the degenerate interval, so the render distinction falls out of
the ontology instead of being a special case. **C5 evidence-not-authority** — a narrow interval is a
*claim*, and narrowing requires evidence while widening never does; authority never substitutes for it.
**C10 contract-evolution honesty** — an estimate made under a 2026 basis stays valid when a 2030
instrument tightens the same quantity; version-pinned, never recency. **C8 observability-per-decision** —
row 18 is this concern paying a dividend: the decomposition *is* the per-decision observability.

**Status: spec landed (2026-08-11).** Rows 12–18 are spec'd by
[measure-dynamics-confidence-ontology-design](epr:measure-dynamics-confidence-ontology-design),
six laws each anchored to the construct that enforces it — `MeasureKind::Rate { per }` (row 12),
the dag-cbor canonical-bytes contract (row 16, proven at the serializer level, not yet wired to a
typed entry point), `Interval::unknown()` (row 16, honest absence), the `Confidence::widen` /
`NarrowingRefused` asymmetry (row 17, with a recorded field-privacy caveat), and named-but-not-yet-built
constructs for the uncertainty-propagating fold (row 17, Task 3's `fold::with_uncertainty`) and the
`.epr-meta` measure-tier gate (row 12's governance analog, Task 4). Rows 13–15 (harvest/regeneration
index, turnover/coverage time, the aggregation-facing fold) and row 18 (the uncertainty work-queue)
remain design-only, exactly as their own Gate/blocker columns already said — they ride L1 (row 12)
and L5 (row 17) respectively and are not built by this spec. The invariants (interval-inside-the-hash,
propagating folds, the widen-free/narrow-costly asymmetry) are the kind that must be pinned by
contract test at birth or they erode silently; this spec is that pin.

## Explicitly refused (recorded so they are not re-proposed)

- **Labour-hours as *the* numéraire.** Our canon already declares six currencies with decay rates
  (`shefa.md`), Time among them — a labour family is *additive*, never a replacement. See row 3.
- **`EQ-3.9` peer-attested capacity** — a labour-weighted vote of other people about how disabled you
  are, self-attestation barred, feeding your consumption share. Collides with Constitution Article II
  (dignity "prior to and independent of utility, productivity, or **social standing**") and with
  supported-decision-making-not-substituted-judgment. **Do not port.**
- **The global convex solve** — centralizes wherever RAM permits (their own figures: the world is
  plannable by whoever holds ~2,600 machines), inverting our hub-optional floor.
- **Credits lapsing on loss of standing** — no floor beneath membership; exit confiscatory by
  construction.
