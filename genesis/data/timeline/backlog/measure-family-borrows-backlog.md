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
  - genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md
  - genesis/docs/superpowers/specs/2026-07-24-quilt-evidence-temperature-composition-design.md
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
