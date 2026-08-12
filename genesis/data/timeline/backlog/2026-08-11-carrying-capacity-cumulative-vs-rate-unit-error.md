---
id: "backlog-carrying-capacity-cumulative-vs-rate-unit-error"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "spatial_capacity.rs compares an all-time cumulative sum to a sustainable yield (a rate) — monotonic false overshoot, dimensionally invalid, and two homes for utilization"
slug: "carrying-capacity-cumulative-vs-rate-unit-error"
written: "2026-08-11"
author: "claude (Meadows survey mint pass, operator-directed)"
status: "open"
priority: "medium"
tags: [elohim-storage, spatial, carrying-capacity, unit-error, research-derived, bounded-code-fix]
cites:
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
---

# Carrying capacity compares a cumulative stock to a rate

Found while adjudicating the [Meadows survey](epr:meadows-systems-dynamics-cross-pollination-2026-08-11)
§3.2 against build state. Verified on disk 2026-08-11, branch `feat/angular22-node24`.

`elohim/elohim-storage/src/services/spatial_capacity.rs`:

- `compute_current_usage` (`:81`) sums `resource_quantity_value` over **every** `consume`/`use`
  event at a Place, for all time — no window, no period, no decay (`:96` filters on action only).
- `check_carrying_capacity` (`:111`) divides that total by `CarryingCapacity.max_sustainable_yield`
  (`:139`, `:144`) and sets `is_allowed: util_after <= 1.0` (`:150`) and
  `trigger_governance: util_after > 0.8` (`:156`).
- `max_sustainable_yield` (`spatial.rs:169`) is a **rate** by its own name — sustainable *yield* is
  per period — and its sibling `unit: String` (`:171`) carries no time denominator, so nothing
  downstream can detect the mismatch.

Three consequences follow mechanically, and the first two are user-visible:

1. **Monotonic false overshoot.** Utilization is non-decreasing forever. Any Place with consumption
   history eventually reads >100% and every allocation is refused, permanently. There is no state
   the system can reach where it recovers.
2. **Dimensionally invalid comparison.** A cumulative quantity over a rate has no correct
   interpretation, so the 0.8 governance trigger is firing on a meaningless number.
3. **Two homes for utilization.** `CarryingCapacity.current_utilization` (`spatial.rs:173`) is a
   stored DHT-projected field *and* the value is recomputed here — they cannot agree, and the
   stored one drifts by construction.

## Fix shape

The cure is Meadows' `harvest ÷ regeneration` directly: window the harvest to the yield's own
period, and model the regeneration inflow so the stock is computed rather than assumed. Concretely:

1. Window `compute_current_usage` to the period `max_sustainable_yield` is denominated in — this
   alone kills the monotonic refusal and is the minimum viable fix.
2. Give the capacity declaration an explicit period (or adopt the
   [measure-family](epr:measure-family-borrows-backlog) row 12 `kind: level | rate | ratio` + `per`
   declaration, which is the general cure and prevents the whole class).
3. Decide which home for `current_utilization` is authoritative and make the other derived —
   projection-derived is the trajectory-correct answer.
4. Regression test proven red on old code: a Place whose consumption is steady and *within* the
   sustainable yield must not drift toward refusal as history accumulates.

## Why this matters more than an arithmetic bug looks

Per the survey's §8 (tier-1 standing, added on the operator's steering note), the biophysical floor
is held as an **empirical non-negotiable** — physics is not a paradigm — and the anthropocentric
focus of the protocol is a *control-point* claim: we and the AI acting on our values are the
requisite-variety attenuator at biospheric scale, sitting **inside** the system we regulate. On that
reading, an apparent conflict between human and ecological thriving is the signature of a broken
control loop rather than a contest of interests — **which makes the honest limit signal, not the
declaration, the ethically decisive artifact.**

So this is not only a unit error. `CarryingCapacity` is the substrate's operational form of an
ecological-standing commitment, and today it is an instrument that cannot report a true reading in
any state the system can reach. Declaring reverence for the living world while shipping a capacity
check that compares a cumulative sum to a rate states a value and breaks the instrument that would
hold it. That argument, not the severity of the symptom, is the case for scheduling it.

## Notes

This is Sprint 7/8 geospatial scaffold (2026-03), and per
`feedback-cleanup-toward-p2p-dataplane-trajectory` the trajectory move is to fix the model rather
than extend the scalar — so prefer step 2 over step 1 alone if the measure-family row is in flight.

**Related but distinct:** the *erodible* capacity question (should sustained overshoot decrement
`max_sustainable_yield` itself?) is a governance decision held at STUDY in
[commons-holonic](epr:commons-holonic-stewardship-backlog) row 12. Do not fold it into this fix —
this entry is arithmetic, that one is a judgment reserved for the human sortition floor.

**Also related:** the global-orchestra epic §10 claims planetary-boundary overshoot is *"impossible —
system won't process transactions beyond limits."* This defect is the counter-example: enforcing a
limit at transaction time against a wrong capacity estimate produces false refusal. See the survey's
§7.2 canon-edit recommendation (operator's call, via cite tooling).

## 2026-08-12 — step 1 landed; step 2 deliberately NOT taken (open escalation)

**What landed** in `elohim/elohim-storage/src/services/spatial_capacity.rs`:

- **Step 1 (windowing).** When the capacity's `unit` carries a time denominator, usage is summed
  over exactly one period of it, so the comparison is `Rate ÷ Rate = Ratio`. The monotonic false
  overshoot and the unreachable recovery state are gone, with a regression test proven red on the
  old code.
- **A level/rate split** the original fix shape did not name. `unit` with **no** denominator
  (`"dwellings"`, `"hectares"`, `"people"`) declares a **level**, and windowing a level would have
  been a fresh fail-open in the opposite direction — occupancy older than the window would vanish
  and over-allocation would be permitted. Level units are therefore compared level-to-level and are
  not windowed. A denominator naming an **extent** rather than a tempo (`"people/km2"`, `"kg/ha"`)
  is a density — still a level, still not windowed — read from a whitelist, never as a fallback: a
  "not a time unit ⇒ density" rule would have swallowed `"liters/fortnight"`, a real tempo outside
  the vocabulary, and read it as a level. A denominator that is present but unreadable
  (`"liters/fortnight"`, a bare `"m"` that is minute or month or metre) is refused outright rather
  than guessed at.
- **Instant, not string, comparison.** `has_point_in_time` permits a UTC offset
  (`create-economic-event-input.schema.json` says only "ISO 8601"), and an offset-bearing stamp
  sorts lexically below a UTC cutoff. **The SQL prefilter was removed entirely**, not merely
  slackened: an intermediate revision kept a 24h-slack lexical bound for index locality, and an
  adversarial pass proved it still silently dropped any stamp that does not sort like
  `%Y-%m-%dT%H:%M:%S` — epoch seconds, an empty string — *before* the fail-closed Rust check could
  run. A prefilter that can decide is not a prefilter. The window is now applied only after
  parsing, and unparseable stamps are counted, not dropped: on a limit gate, over-counting refuses
  and under-counting allows.
- **Descriptions** on `elohim/sdk/schemas/v1/objects/carrying-capacity.schema.json` recording the
  `unit` parse contract and demoting `currentUtilization` to non-authoritative. Descriptions only —
  no field added or removed, no generated shape moved.

**Step 2 was NOT taken, and this is the open escalation.** Giving the capacity declaration an
explicit period (or adopting the [measure-family](epr:measure-family-borrows-backlog) row 12
`kind` + `per` declaration) changes a **DHT-projected data shape** — `CarryingCapacity` rides on
`Place.carrying_capacity_json` — so it needs a `p2p-design-gate` pass before a field is added, not
an implementer's call inside a defect fix. The Notes above prefer step 2 over step 1 alone *when the
measure-family row is in flight*, and **row 12 is landed, not merely in flight** — so this
deviation is deliberate and is disclosed here rather than left implicit. Until step 2 lands, `unit`
string-parsing is the whole period contract, which is why the schema description now states it
precisely.

**Step 3 (which home for `current_utilization`) is partially done.** The derived value is declared
authoritative in code and schema, but the stored field still has **two live consumers**, with three
different thresholds across two homes:

- `elohim/elohim-storage/src/services/spatial_dashboard.rs:394-403` — `> 0.85` on the stored value.
- `app/elohim-app/src/app/elohim/components/spatial-map/spatial-map.component.ts:101-106` — renders
  `cap.currentUtilization` from the stored blob with its own `0.8` / `1.0` thresholds.

Neither was changed here (the Angular one is a different gate). Making both read the derived value —
and reconciling 0.8 / 0.85 / 1.0 to one threshold — is the remaining work on step 3.

**Residual modeling gap (not a regression).** For a level unit, "usage" is still the cumulative sum
of `consume`/`use` events, which is an upper bound on the current level rather than the level
itself — it errs toward refusal, so it fails closed, but a level capacity wants a stock computed
from inflow and outflow (Meadows' actual shape), which is step 2 territory.

**Residual: performance, not correctness.** `compute_usage` replaced the SQL `SUM()` with a row
materialization, because the window predicate is now an instant comparison that SQL cannot express
against a free-form timestamp column. On the level path there is no window at all, so every
historical `consume`/`use` row for the Place lands in a `Vec`. Correct, and unbounded. The fix is
either a normalized instant column on the projection (which SQL *can* filter) or a paged fold —
both larger than this defect.

**Residual: the JSON schema mirror reaches nobody.** `elohim/sdk/schemas/v1/objects/
carrying-capacity.schema.json` has no `$ref` from any schema, no entry in `codegen-ts.mjs`, and no
`schema_contract` case — it is an unvalidated document, so descriptions added there are inert. The
surface that actually reaches consumers is the ts-rs export of
`elohim/elohim-storage/src/services/spatial.rs`, and the same three descriptions now live on those
Rust doc comments so they propagate through `cargo test export_bindings` into
`CarryingCapacity.ts` — which is the type the Angular consumer above imports. Wiring the JSON
schema into the validation harness is separate work; per repo CLAUDE.md these schemas are supposed
to be the source of truth for this boundary, and this one is not wired to be.
