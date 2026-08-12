---
title: "Holonic capacity convergence — pivot plan: capacity is a Measure family on a holon, not a struct on a Place"
id: holonic-capacity-measure-convergence-pivot
status: Draft
domain: D2
sprint: next
kind: handoff
cites:
  - systems-discipline-slice2-handoff | the predecessor whose slice this pivot interrupts — its stock-and-flow spine (Stock, Window, stock_over_window) is the fold this convergence targets, and its scoping table is why the network slice stays out of scope here too | sha256:17ab320e2383ebba | path: genesis/docs/superpowers/plans/2026-08-11-systems-discipline-slice2-handoff.md
  - measure-dynamics-confidence-ontology-design | the sealed canon this pivot extends rather than amends — L1 (kind) and L5 (propagating folds) are what a converged capacity would inherit for free, and its Q-series is where this plan's Q1-Q4 belong if they graduate from open questions to design decisions | sha256:52d601baa6117450 | path: genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md
  - genesis/data/timeline/backlog/2026-08-11-carrying-capacity-cumulative-vs-rate-unit-error.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
---

# Holonic capacity convergence — pivot plan

**Read this first if you are picking up the carrying-capacity work.** A defect fix is
sitting **uncommitted and verified** in the tree. It is not committed because the
operator's steering note on 2026-08-12 re-framed what it is a fix *of*. This document
records the state, the re-framing, and the convergence — with the reasoning, so the next
session can disagree on evidence rather than re-derive it.

**The pivot in one line:** we hardened a scaffold when we should have converged it. The
fix is correct and closes a live fail-open; it is also a second implementation of a fold
`epr-rea` already models better.

---

## 0. State of the tree — READ BEFORE TOUCHING ANYTHING

Uncommitted, gates green, **deliberately not committed**:

| Path | What | Verified |
|---|---|---|
| `elohim/elohim-storage/src/services/spatial_capacity.rs` | window the harvest; `YieldDenominator` level/rate/density split; live dimensional guard; instant-not-string comparison; prefilter removed | `cargo test --lib spatial_capacity` 14 passed; clippy `-D warnings` 0; fmt 0 |
| `elohim/elohim-storage/src/services/spatial.rs` | doc comments only on `CarryingCapacity` — `unit` as load-bearing, `current_utilization` as non-authoritative | `cargo test --lib export_bindings` 77 passed; generated-TS churn is comment-only, no shape moved |
| `elohim/elohim-storage/src/services/vulnerability.rs` | error path no longer reads as "no stress"; `1.0` documented as a floor, not a maximum | same run |
| `elohim/sdk/schemas/v1/objects/carrying-capacity.schema.json` | descriptions only, no field added/removed | see G7 — this file is wired to nothing |
| `elohim/epr/src/measure.rs`, `elohim/epr-rea/src/{fold,stock}.rs` + tests | `UnknownReason`, `unknown_because`, total-ordered `reduce`, operand threading | `epr` + `epr-rea` + `eprfs` all `cargo test` 0; clippy 0; fmt 0 |
| `.claude/scripts/_lib/cluster_state.py` + tests + 3 call sites | one cluster-state parser, true-wins merge | 65 assertions; 7 consumer outputs byte-identical to HEAD |
| backlog + spec docs | grooming, Q17, the dated disclosure section | — |

**Do not commit any of it before deciding G1.** If `CarryingCapacity` does not survive as
a struct, part of the `spatial_capacity.rs` work is scaffold that should be deleted rather
than shipped, and committing it first makes the deletion look like a regression.

**Two files in that list are ambient from other sessions and are NOT this work:**
`elohim/elohim-storage/Dockerfile`, `elohim/sdk/domains/lamad/CLAUDE.md`. Do not sweep
them into a commit.

---

## 1. The finding: three things wear the word "capacity"

`CarryingCapacity` fuses them into one struct, which is why "is this a judgement or a
calculation?" has no clean answer today. It is both, plus a third thing.

| | What it is | Where it lives now | Where it belongs |
|---|---|---|---|
| **Declared limit** | a *claim* — "this aquifer sustainably yields 500 ML/year", with a claimant and a basis | `max_sustainable_yield: f64` + `unit: String` + `data_quality: String` + `source` + `measured_at` | a `Quantity` whose `Confidence{claim, interval, basis}` says who claims it and how well |
| **Computed utilization** | a *fold* over witnessed events — pure arithmetic, no judgement | `compute_usage` → `current_utilization: f64` | `epr_rea::stock::stock_over_window`, which already returns intervals |
| **The limit verdict** | a *judgement* — `is_allowed`, `trigger_governance > 0.8` | baked into `check_carrying_capacity` | a Mishpat lens the caller composes — **but see Q1, this is NOT decided** |

`data_quality: String` is an unstructured `Confidence` in disguise; measure-family row 16
already says so and names this exact field as the stand-in.

**The cost of the fusion, stated concretely:** `current_utilization` is a bare `f64` with
no interval. The `0.8` governance trigger therefore fires on a point estimate with no
honesty about its basis — the precise failure the measure ontology was built to prevent.
The measure plane got interval discipline this week; the one place in the tree that
actually gates an allocation against a limit did not.

---

## 2. The duplication, precisely located

Two folds over economic events, same shape, different fidelity:

| | `spatial_capacity::compute_usage` | `epr_rea::stock::stock_over_window` |
|---|---|---|
| Input | diesel rows at a Place | `&[FlowEvent]` |
| Window | bespoke `UsageWindow` + `parse_event_instant` | `Window { start, end, per, periods }` |
| Unit → kind | bespoke `YieldDenominator` parser over `unit: String` | `MeasureKind::{Level, Rate{per}, Ratio}` declared |
| Returns | `f64` | `Stock` of `Quantity` with `Confidence`, `Interval`, `UnknownReason` |
| Unknown handling | none — an absent reading is `0.0` | honest absence; `unmeasured ≠ measured-zero` |

The 2026-08-12 fix borrowed the **algebra** (`MeasureKind::divide` in the guard) but not
the **fold**. That is the half-convergence to finish.

**This is the trajectory violation to name plainly.**
`feedback-cleanup-toward-p2p-dataplane-trajectory` says clean up drift, never extend the
scaffold. The fix hand-rolled a window, a unit parser, and a denominator vocabulary inside
`elohim-storage` — reimplementing what `epr-rea` models with intervals. It was the right
*stop-the-bleed* (the defect was a permanent-refusal fail-open reachable by any Place with
history) and the wrong *destination*.

---

## 3. The holonic distinction that actually matters

The operator's question — a person has capacities, an ecosystem has *carrying* capacity;
are we duplicating REA aggregation? The answer is that **the aggregation rule is not
uniform across the holarchy**, which is exactly why one fold cannot cover it and why
"carrying capacity" is not merely "capacity, aggregated."

| Holon | Rule | Why that rule |
|---|---|---|
| Person, agent, machine | *(none — it is a leaf)* | A limit on flow through one agent. Nothing beneath it to aggregate. |
| Factory, workshop, team | **min** (Liebig) | *"the input that is most important is the one that is most limiting."* Summing machine throughputs overstates the whole. Same rule the agentic queue's item 17 invokes for model capacity — different subject, identical shape. |
| Community, cohort, commons | **harmonic mean** | measure-family row 7 (Playnet's `Φ`), chosen so one badly-served member cannot be averaged away. |
| Ecosystem, watershed, biosphere | **declared at the holon — not folded from parts** | Emergent. And *erodible*: sustained overshoot decrements the capacity itself. |

That last row is the crux. A Place's `max_sustainable_yield` is a **declaration at the
holon level**, not a roll-up of anything beneath it. A factory's capacity *would* be a
roll-up. Same word, opposite provenance — so they cannot share one `aggregate()`, and a
generic REA sum-fold is wrong for three of the four rows.

**Erodibility stops being a special case** under this reading: an erodible capacity is one
whose *declaration* is itself a `Stock` with an inflow and an outflow. That is `stock.rs`
again, one level up. (The governance question of *whether* to erode stays held at STUDY in
commons-holonic row 12 — this only says where it would live if decided.)

---

## 4. The convergence target

**No new entity.** The inclusive ontology is the Measure family it already is, carrying
two declarations it does not yet have:

| Element | Status |
|---|---|
| `kind: level \| rate \| ratio` + `per` | **landed** (`elohim/epr/src/measure.rs`) |
| `Confidence{claim, interval, basis}` inside the content hash | **landed** |
| `UnknownReason` — why an interval is unknown | **landed** 2026-08-12, uncommitted |
| **aggregation rule** — `sum \| min \| harmonic \| declared-at-holon` | measure-family row 7, **not landed**, never yet connected to capacity |
| **limit semantics** — `soft` (trades against others) vs `hard` (diverges near the limit) | measure-family row 5, **not landed** |
| **verdict as a lens**, not a field on the measure | the teeth law — **Q1, undecided** |

With those, a person's capacity, a machine's throughput, a factory's binding constraint,
and a watershed's carrying capacity are **one shape at four holon levels**, differing only
in (a) whether the value is declared or folded, and (b) which rule composes it upward.

---

## 5. The p2p design gate this pivot forces

The escalation recorded on 2026-08-12 in the backlog entry — *"adding a `per` field to
`CarryingCapacity` needs a gate pass"* — **is the wrong question**, and the next session
should not answer it as written. Adding a field to a struct that may not survive is
deepening the scaffold.

**The gate question is: does `CarryingCapacity` survive as a struct at all, or is it a
Measure family declared on a Place?** Run the gate on that, not on the field. Inputs the
gate will need:

- `CarryingCapacity` rides on `Place.carrying_capacity_json` — a DHT-projected shape, so
  any answer moves a wire format. Classify (A / A2 / B / B2 / C) before designing.
- A Measure family declared on a Place is plausibly **A2** (an attribute of an
  already-notarized Place) rather than a standalone **A** — which would cost no new entry
  type and no new head. Verify against `#[hdk_entry_types]`; do not trust this sentence.
- Head-plane cost: how many capacity declarations at 1 year, across how many Places.
- Content address: a declared limit is a claim by an agent about a Place — check whether
  that is agent-scoped composite (Option 2) rather than content-derived.

---

## 6. Open questions — record, do not silently resolve

**Q1 — Does the limit verdict belong in the measure? UNDECIDED (operator, 2026-08-12: "I'm
not sure, if it's a violation").** `check_carrying_capacity` returns `is_allowed` and
`trigger_governance > 0.8` alongside the utilization. The repo law says *measures never
carry teeth; governance lenses compose them.* Two readings, both defensible:

- **It is a violation.** The measure should return a `Quantity` and a lens should decide.
  Evidence for: three different thresholds already exist across two homes (0.8 enforce,
  0.85 dashboard, 0.8/1.0 map) — precisely what happens when a threshold is not a declared,
  composable object.
- **It is not.** `check_carrying_capacity` is a purpose-built *enforcement service*, not a
  Measure family; the law constrains measures, and an enforcement service is allowed to
  enforce.

Do not resolve this as a side effect of the convergence. It changes *who decides* whether
an allocation is refused, which is a governance move, not a refactor.

**Q2 — What is the aggregation rule's default when a family declares none?** Silently
summing is how the original unit error happened. Refusing is honest but makes every family
declare something on day one. (Related: measure-family row 7 asks for per-family
declaration but does not name a default.)

**Q3 — Is `data_quality: String` migrated or dual-written?** It is the unstructured
ancestor of `Confidence`. Live projected data carries it; a hard cutover breaks reads.

**Q4 — Does a declared-at-holon capacity need to be distinguishable, in the type, from a
folded one?** A reader cannot currently tell whether a number was claimed or computed —
and §3 says that distinction is what separates an ecosystem from a factory. `ClaimKind`
may already cover it (`Estimated`/`Modelled` vs `InstrumentMeasured`), or may not.

---

## 6a. G1 RESOLVED — the recorded p2p-design-gate (2026-08-12)

**The gate question as §5 framed it was still too narrow.** It asked whether
`CarryingCapacity` survives *as a struct*. Grounding against disk answered a prior
question: it is not load-bearing as a struct at all. It is a **transient decode** of a
`TEXT` column, materialised at exactly two `serde_json::from_str` sites; no HTTP route
returns it, no TypeScript imports either generated type, and all production values arrive
as JSON text. **10 sites the compiler catches, 13 it does not.**

### The finding that decides it: three homes already exist

`CarryingCapacity` is not one fused struct needing a split. It is a **triple
re-implementation of three types the crates already have**, each by a different author.

| Wears "capacity" | Already is | Where | Provenance |
|---|---|---|---|
| declared limit | `Bound { limit, unit, threshold_pct }` on a `Commitment` | `epr-rea/model.rs:100-166` | Beer — a bound on a promise |
| computed utilization | `Stock { level, inflow, outflow }` via `stock_over_window` | `epr-rea/stock.rs:114-409` | Meadows — the plant |
| the verdict | `Verdict{ Permit \| Refuse \| Refer{layer, reason} }` | `epr/verdict.rs:76-142` | Beer — requisite-variety routing |

The crate's own doctrine already dictates the first row, verbatim at `model.rs:106`:
*"A limit is only ever a bound on a promise — which is why the algedonic `bound_ref` is the
bounding commitment's own CID rather than a separate atom's."* A bound-on-a-promise supplies
the **claimant** §1 asks for, and `Place.governing_collective_id`
(`mishpat_integrity/src/lib.rs`) already exists to be it.

**Decision: `CarryingCapacity` does not survive as the model.** It is retired to a decode
shim over the existing `carrying_capacity_json` and stops being the shape anything reasons
about. No new entity is minted — three existing ones are used.

### Entity: DeclaredLimit (capacity as `Bound`)
- **Classification**: Notarized (A) — **inline on an existing entry, no new entry type.**
- **Justification**: A sustainable-yield claim is a promise someone is accountable to; the
  protocol would be lying if it changed silently. It is not a thing in its own right — it is
  a field on a `Commitment` that already exists (`Commitment` is declared on **both**
  `mishpat_integrity` and `content_store_integrity`; `epr-rea/model.rs` names
  `Mishpat::Commitment` as the graduated home, cid = entry_hash, never action_hash).
- **Head-Plane Cost Budget**: **zero new heads.** The bound rides inside a commitment's own
  canonical bytes. Count at 1yr = the count of bounded commitments, which is bounded by the
  number of stewardship promises, not by observations. No bundling justification needed.
- **Network Stakes**: all four stages. The band-edge comparison is **floor-protected**
  (`Constitutional`) — an ecological limit must not cheapen at Simulacra.
- **Content Address Strategy**: Content-Derived (CID) — the commitment's own `atom_cid`.
- **Address Justification**: Declaring a bound already moves the commitment CID by design,
  pinned by `declaring_a_bound_changes_the_commitment_cid` (`model.rs:530`). Not Option 2:
  the limit is not an agent's stance toward a target, it is a term of the promise itself.
- **Source of Truth**: Holochain DHT (the Commitment entry).
- **Integrity Zome + DNA-hash class**: `mishpat_integrity` — **DNA-hash-NEUTRAL for the
  epr-rea change** (it touches no integrity zome; `Bound` is a payload field on an atom).
  Whether the DHT `Commitment` entry needs a matching field is a **separate gate**, deliberately
  deferred — see Design Constraints below.
- **Coordinator Zome**: `mishpat::create_commitment` (exists; `commitments.rs`) → EntryHash.
- **Projections**: SQLite — existing commitment table. Automerge sync — n-a (not
  broadcast-tier content).
- **HTTP Route**: none new.
- **Anti-Pattern Check**: caught and corrected — *"is there DNA headroom?"* was never asked;
  the deciding tests were "is it an attribute of something already notarized" (yes) and "does
  a type already exist" (yes). Also caught: minting a fresh type when the concept has a
  consolidating home.

### Entity: ComputedUtilization (capacity as `Stock`)
- **Classification**: **Ephemeral (C)**.
- **Justification**: A pure fold over notarized `EconomicEvent`s. `stock.rs:13-20` states the
  reason it must never be stored: *"A stored level would be a second home for a number that
  already has one, which is the shape that lets `CarryingCapacity.current_utilization` drift
  against the events that supposedly produce it."* The defect this plan exists to fix is
  named, in the source, as the anti-pattern.
- **Reconstruction strategy**: re-fold the event log over the declared `Window`. No state.
- **Head-Plane Cost**: none.
- **Source of Truth**: SQLite (operational projection of DHT `EconomicEvent`s).
- **Anti-Pattern Check**: this entity IS the correction of the "two homes for one number"
  anti-pattern; `Place.current_utilization` is the second home being retired.

### Entity: Holarchy (the containment index)
- **Classification**: **Ephemeral (C)** — and this is the load-bearing result.
- **Justification**: Containment is **already notarized twice** — `Place.parent_place_id` +
  the `ParentToChildPlace` link type (mishpat), and `Collective` + `Membership` (imagodei).
  A holarchy is a read-side index over links that already exist. **Zero new entry types,
  zero new heads** — which satisfies rather than fights the standing constraint from the
  2026-08-11 measure-family gate ("headline: zero new DHT entry types").
- **Reconstruction strategy**: walk `ParentToChildPlace` / `Membership` links.
- **Anti-Pattern Check**: caught — the reflex here is to mint a `Holon` entry type. Refused:
  an authored containment edge is already notarized; a computed index over it is C.

### Entity: CapacityVerdict (the decision)
- **Classification**: **Ephemeral (C)** — `verdict.rs:14-16` already declares Category C,
  *"never persisted, reconstructed by re-evaluation over notarized inputs."*
- **This is a decision predicate**, so it additionally owes the Step 4 concern-canon answer
  and a `seam-registry.yaml` registration. Neither `epr` nor `epr-rea` has a seam registry
  today — see G13.
- **Anti-Pattern Check**: caught — `is_allowed: bool` is a `Decision` that has **lost
  `Refer`**. `verdict.rs:60-68` carries a ceiling law (*no conversion may collapse `Refer`
  into `Refuse`*) and a bool cannot express `Refer` at all, so the one gate in the tree that
  refuses an ecological allocation is structurally incapable of routing a novel situation to
  a council. See Q1 below — resolved.

### Back-fill detector (the three questions that cannot be answered in reverse)
1. **What does the coordinator function return, and is it the hash the route's `{id}` accepts?**
   `mishpat::create_commitment` → EntryHash; the bound has no route of its own and no `{id}`.
   The capacity read path takes a *place id*, which is a different identifier from the
   commitment entry hash — so the two must not be conflated at any future route.
2. **Which integrity zome, and does the change move the DNA hash?** `mishpat_integrity`;
   the epr-rea change is DNA-hash-NEUTRAL. The DHT-entry question is deferred, not assumed.
3. **Item count at 1 year, and what it does to quiesce?** Zero new heads on all four
   entities. Quiesce is unmoved. This is the whole point of the classification.

### Design Constraints Discovered
- **`Place` is a declared-but-unwired entry type.** It is validated (`validate_place`,
  `mishpat_integrity/src/lib.rs:586-631`) with six Place-specific link types, and the hApp
  role is installed — but `grep "EntryTypes::Place"` returns exactly one hit (the validation
  arm). **No `create_place` coordinator function exists anywhere in the repo**, no post-commit
  signal projects it, and `api/places.rs:61-68` writes a hardcoded `dev-placeholder-{id}` into
  a `dht_anchor_hash TEXT NOT NULL` column. The migration comment `-- Source of truth: DHT
  (Mishpat DNA). Classification: A.` (`up.sql:1102`) **overstates current reality**. In the
  habits register's vocabulary, Place is `unwired`.
  → **A2-on-a-notarized-Place is therefore not available today.** The bound-on-a-Commitment
  path is, because commitments *are* created. The design degrades gracefully: the bound's
  `in_scope_of` is a collective CID now and a Place CID when Place is wired.
- **The fold reads content_store while the bound lives on mishpat.** `EconomicEvent` is
  declared on `content_store_integrity`; `Commitment` is declared on *both* DNAs. A capacity
  check therefore spans two DNAs. That cross-DNA read is a real constraint on any future
  in-zome enforcement and an argument for keeping enforcement in the storage/fold layer.
- **Adding a field to `Bound` re-addresses every bounded commitment.** `Bound` is
  `#[serde(rename_all = "camelCase")]` inside `Commitment.bound`. New fields MUST follow the
  Q17 precedent — `Option<T>` + `skip_serializing_if`, `None` ≡ v1 semantics — with a
  pre-change golden vector, or existing CIDs move.
- **`epr-rea`'s folds are scope-blind.** `stock_over_window` filters on **resource CID only**;
  `in_scope_of`, `provider`, `receiver`, and `process` are all ignored by every fold. A
  per-Place or per-holon stock is not computable today without the caller pre-partitioning
  the event slice.

---

## 6b. Q1 RESOLVED — by operator direction, 2026-08-12

Neither recorded reading survived contact with the evidence, and the operator's direction
supersedes both.

**The third reading neither option covered:** `is_allowed: bool` is not "a measure carrying
teeth." It is a `Decision` **with `Refer` deleted**. The repo law that bites is not *measures
never carry teeth* — it is `verdict.rs`'s **ceiling law**: `Refer` must never collapse into
`Refuse`. A novel ecological situation must be routable to a council, and a bool cannot
route. That is the violation.

**The operator's direction goes further, and it is the better answer:**

> *"verdicts should be able to be revisited — when it's novel or unknown that should be the
> initial verdict, but we need something around it so deliberation can continuously revisit
> the verdict as often as the domain needs (a p2p network communication → reactive streams →
> continuously, vs an ecosystem → per season, or when measurements indicate a trend)."*

A verdict is a **homeostat, not a gate**. `Verdict` is *already* Category C — never
persisted, reconstructed by re-evaluation. Re-evaluation is already the architecture. What is
missing is that **nothing declares when**: today a verdict is recomputed whenever a caller
happens to call. The cadence is the domain's to declare.

```rust
ReferQuestion { layer, reason, note, revisit: Cadence }

/// When a held Refer must be re-evaluated. Declared by the domain, never defaulted.
pub enum Cadence {
    PerObservation,      // reactive-stream domains — p2p admission, backpressure
    PerWindow(Window),   // seasonal domains — a watershed, a harvest
    OnBandEdge,          // trend-triggered — reuses the algedonic hysteresis
}
```

`Refer` becomes a **held state with a declared revisit cadence** — the same discipline
`algedonic.rs:354` already applies to pain (`should_emit`: *"pain is a held state, not a
stream"*). It also unifies two seams that looked unrelated: the `hc_client` reactive-stream
pacing work is `PerObservation`; a watershed is `PerWindow`. Same homeostat, different
cadence.

**Consequence for G8**: unblocked. The three divergent thresholds become one declared `Bound`,
and the verdict they were each re-deriving becomes one `Verdict` with a declared revisit.

---

## 7. Gaps

Ordered so that each is landable alone. G1 gates the rest.

- [x] **G1 — Gate `CarryingCapacity`'s survival as a struct** (§5) — **RESOLVED 2026-08-12,
      see §6a.** It does not survive as the model; three existing types absorb it. Zero new
      DHT entry types, zero new heads.
- [ ] **G2 — Decide the commit disposition of the uncommitted tree** (§0). If the struct
      survives, commit the defect fix as a stop-gap citing this plan. If it does not,
      commit only the parts that survive the convergence (the `epr`/`epr-rea`
      `UnknownReason` work and the `_lib` consolidation are independent of G1 and are
      committable either way).
- [ ] **G3 — Converge `compute_usage` onto `epr_rea::stock::stock_over_window`**, so
      utilization is a `Quantity` carrying kind, interval, and unknown-reason instead of a
      bare `f64`. Delete the bespoke `UsageWindow` and the hand-rolled window arithmetic.
- [ ] **G4 — Move the `unit` → kind classification off a string parser.** `YieldDenominator`
      is a parser for a declaration that should be structured. Keep the *vocabulary*
      (tempo / extent / unreadable, and the refusal on ambiguity — that part is good and
      was adversarially tested); move the *declaration* to the measure family.
- [ ] **G5 — Land the aggregation rule as a declared field on a Measure family**
      (`sum | min | harmonic | declared-at-holon`), with the four §3 cases as tests.
      Resolves measure-family row 7 and connects it to capacity for the first time.
- [ ] **G6 — Land soft/hard limit semantics** (measure-family row 5), so an ecological floor
      is legible without being negotiable, and pairs with honest absence.
- [ ] **G7 — Wire or retire `elohim/sdk/schemas/v1/objects/carrying-capacity.schema.json`.**
      It has no `$ref` from any schema, no `codegen-ts.mjs` entry, and no `schema_contract`
      case — an unvalidated document, so descriptions added to it are inert. Per repo
      CLAUDE.md these schemas are the source of truth for this boundary; this one is not
      wired to be. Either wire it into the harness or delete it and let ts-rs be the truth.
- [ ] **G8 — Reconcile the three thresholds to one declared object.** `spatial_dashboard.rs`
      `> 0.85` on the stored value; `spatial-map.component.ts` `0.8`/`1.0` on the stored
      value; enforcement `0.8` on the derived value. Blocked on Q1 — the shape of the fix
      depends on whether a verdict is a lens.
- [ ] **G9 — Replace the cumulative-sum stand-in for a level capacity with a real stock.**
      For a level unit, "usage" is still the all-time sum of `consume`/`use`, an upper bound
      on the current level rather than the level. It fails closed, so it is not urgent —
      but a level capacity wants inflow and outflow, which is `Stock` (Meadows' actual
      shape).
- [ ] **G10 — Normalize the timestamp column so the window can be pushed back into SQL.**
      `compute_usage` now materializes rows because the window predicate is an instant
      comparison SQL cannot express against a free-form text column. On the level path
      there is no window at all, so every historical row for the Place is loaded.
      Correct, and unbounded. Performance, not correctness.

---

## 8. Landmines this session hit — do not re-pay for them

- **A subagent died mid-task on an API error** (`Now the Defect 2 tests`) leaving partial
  edits on disk that *compiled and passed*. A dead agent does not mean a broken tree, and a
  green tree does not mean the task finished — check the scope list item by item. Here,
  defects 1 and 2 had landed and defect 3 had not.
- **Adversarial verification earned its keep three times.** Every one of the three work
  items was reported green by its author; two were REFUTED on re-read, and the third
  carried two fail-opens the author had introduced. The verifiers re-ran the gates
  themselves and mutation-tested rather than trusting reported exit codes. Budget for this;
  it is not optional overhead.
- **The first fix for a fail-open introduced two more.** Windowing changed the question
  from "how much" to "how much *recently*", and every defect since has been a way a
  timestamp or a unit can fail to answer it. Expect the convergence to have the same
  property and verify accordingly.
- **A lexical prefilter on a free-form timestamp column cannot be "just an optimization."**
  A 24h-slack lexical bound still silently dropped epoch-seconds and empty-string stamps
  before the fail-closed check could run. If a prefilter can decide, it is not a prefilter.
- **`cargo test export_bindings` at the crate root filters to 0 tests in this crate**; use
  `cargo test --lib export_bindings` (77 tests). A green run that filtered everything out
  proves nothing.
- **The worktree is shared.** HEAD moved under this session more than once. Commit
  path-limited with explicit file lists; never `git add -A`.
