---
title: "Habits as REA compositions — progress, red mass, and the time a habit costs and returns"
id: habit-progress-scalar-design
tier: spec
status: active
created: 2026-10-22
revised: 2026-10-22
maintainers: Matthew Dowell + Claude
steward: human:matthew
class: architecture
context-tier: disclosed
graduation-trigger: the first sprint window read with `epr flow stocks --stock red-mass` is cited in that sprint's record, and its drained mass, hours spent and hours returned match the atoms and the flow log
habits: [dev-system-equilibrium]
topic: [habits, middot, measures, stocks, register, epr-rea, time-banking]
informed-by:
  - genesis/docs/superpowers/sprints/2026-10-18-marathon-retrospective.md
  - genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - elohim/epr-rea/src/lib.rs
  - elohim/eprfs/epr-cli/src/flow/stocks.rs
cites:
  - marathon-retrospective-20261018 | Retrospective | sha256:cbef898ec7bda733 | path: genesis/docs/superpowers/sprints/2026-10-18-marathon-retrospective.md
  - middot-measure-primitive-design | Middot | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - .epr-meta/dev-system-equilibrium.habit.md
---

# Habits as REA compositions

## 1. Problem

The register says whether a habit is kept: `green`, `red` or `unwired`. That status is human-owned and moves only on evidence. That is right for a promise. It is the wrong grain for reading a sprint.

The 2026-10-12..18 marathon scored 15 → 14 green and 0 of 0 in the ledger. Its atoms recorded measured progress that neither number could show. Two examples: app-delivery-refuses-fast went from 1/4 to 3/4 scenarios, and a recurring 43 s readiness window became 10 s.

Three things were invisible:
- **how far a red habit had moved toward green;**
- **what the work cost;**
- **what it returned.** That 33 s is saved on every run, every week.

The retro's root cause applies too (§7 there): the ledger's unit and the work's unit were never joined. A sprint that moves reds discharges nothing the ledger can see.

This spec makes each habit a **declared composition of the epr-rea primitives** the rest of the substrate already uses, so all three become readable from one fabric:
- **progress:** the fraction of the way to green;
- **cost:** hours consumed, judged against a declared budget;
- **return:** hours per week saved, with payback = cost ÷ return.

It is forward-looking. Readings start when they are recorded, and nothing is backdated. Status flips stay the operator's, on evidence.

## 2. Design rule: compose the substrate, don't beside it

Every concept below is an existing primitive or a domain-neutral extension of one. Habit-specific code is glue: it reads atoms and git history and calls the fabric.

| habit concept | primitive | crate |
|---|---|---|
| a habit's identity | `PinnedRef { id: "habit:<id>", version: 1 }` → its `atom_cid` is the **habit scope**. It is stable across edits; it is the identity the atom's filename already enforces. | `elohim-epr-rea` (existing) |
| "make this habit green", one per red episode | `Commitment` (Produce, one `habit-green`), `classified_as: ["register:habit", <id>]`, `in_scope_of: repository scope`, `valid_from: episode start`, `state: Active` (red and unwired alike — both carry red mass). It carries the budget declared when the episode opened as a `Bound` in unit `hour`. | existing |
| the habit flips green | `FlowEvent` with `fulfills: [episode]` → a discharge in the commitments stock (retro S2) | existing |
| how far toward green | `progress::Part` readings composed by a declared `Composition` (`sum` = equal shares, `min` = weakest component, `harmonic` = worst-off-sensitive) | **new, domain-neutral**: `elohim_epr_rea::progress` |
| red mass | the habit resource's STATE: `Produce`/`Consume` of unit `habit-equivalent` on the habit's own resource. The register's stock re-addresses every habit's events to one synthetic resource in memory and folds them with `stock_over_window_within`, the move the commitments stock already makes | existing fold |
| time spent | `FlowEvent` `Consume` of the time resource, unit `hour`, `in_scope_of` the habit scope, `fulfills: []` | existing |
| budget pain | `Bound::breached_by` / `approached_by` on hours spent → `AlgedonicEvidence` whose `bound_ref` is the episode | **promoted to public**: `fold::level_evidence` (was private `bound_evidence`) |
| time returned | a gauge part with a time unit and a declared frequency → a `Quantity` `Rate { per: Week }` in hours | `progress` + `elohim_epr::measure` |
| payback | hours spent (Level) ÷ hours returned (Rate) → a Level in weeks, carrying the weaker claim and NaN-on-unknown | **promoted to public**: `stock::coverage_time` (the private `divide` that `turnover_time` already uses) |
| evidence tier | `elohim_epr::measure::ClaimKind` (`witnessed`, `instrument-measured`, `estimated`, …) | existing |

**Two rules from the fabric carry through unchanged:**
1. **Annotations never discharge.** Progress and cost events carry an empty `fulfills`, so they cannot retire a promise. Only the flip event names its episode.
2. **Time is history-derived.** Nothing reads `now()`. Habit events are dated when the change ARRIVED on the line: the committer date of the first-parent commit that changed the atom, clamped non-decreasing along the walk (real first-parent lines carry backwards dates; the walk order is the causal order). Spend events are dated at HEAD's author date, as `epr flow note` is.

## 3. The declaration (the habit atom's `rea:` block)

The habit atom (`<dir>/.epr-meta/<id>.habit.md`) stays the home: CLAUDE.md artifact 1, git-tracked and dated. A habit that declares nothing still works: its components are its `@concern:` checks, composed by `sum`. The optional `rea:` block configures how the habit composes the primitives:

```yaml
rea:
  composition: sum            # sum | min | harmonic — how parts compose into the habit's progress
  budget:                     # a Bound on each red episode's commitment, in hours
    hours: 40
    threshold_pct: 85         # band edge, as Bound declares it (percent, 1..100)
  parts:
    - part: app-delivery-refuses-fast    # same name as an @concern check ⇒ the same part
      reading: 3/4                       # passed / declared scenarios (skipped and pending count as not passed)
      at: 2026-10-18
      evidence: sprint-report-household-20261018T114938Z-80db9208
      claim: instrument-measured         # ClaimKind; default instrument-measured
    - part: station-2-window             # a gauge
      unit: s
      baseline: 43
      target: 12
      reading: 10
      at: 2026-10-18
      evidence: household A/B 11:36Z
      per_week: 40                       # occurrences per week ⇒ this part RETURNS time
      per_week_evidence: gate-cycle rows 2026-10-11..18
    - part: deploy-window-stations       # declared, not yet measured
```

**Part rules** (`progress::Part`):

| reading | progress |
|---|---|
| `n/d`, with 0 ≤ n ≤ d and d > 0 | `n / d` |
| a number, with `baseline` ≠ `target` | `clamp((reading − baseline) / (target − baseline), 0, 1)`. Direction comes from the sign. A reading past its baseline in the wrong direction is reported as **deteriorated**, and is not hidden by the clamp. |
| `pass` / `fail` | `1` / `0` |
| absent | **unmeasured**: the part is declared but has no reading. It is never `0` or `1`. |

**Validation.** An invalid part counts as unmeasured and is reported by name with its reason. It never counts as zero. A part is invalid when:
- a reading has no `at` or no `evidence`;
- `at` is later than the author date of the commit that records it;
- `d = 0`, or `n > d`;
- `baseline` equals `target`;
- the name is a duplicate;
- any field contains `@concern:`. That keeps CI-harvest routing, which scans the whole habit block, from gaining routes.

**Comparability.** A part's denominator `d`, or its `baseline`/`target` pair, is part of its identity. A change to either is a **rescope**, recorded as its own event kind and never as progress or regression.

**Composition.** Each rule gives a *floor*, which is the headline number, and a *measured* value with its coverage `m/n`. The floor counts unmeasured parts as `0`. Measuring one easy part of eleven therefore cannot inflate the habit.
- **sum:** floor = Σp/n; measured = Σp/m.
- **min:** floor = min(p), or `0` when any part is unmeasured; measured = the min over measured parts.
- **harmonic:** the harmonic mean, by the same rule.

With `n = 0`, progress is `0` and the habit is reported as **no parts**.

**Time returned.** A lower-is-better gauge part with a time `unit` (`s`, `min`, `h`) and a declared `per_week` returns `(baseline − reading) × per_week` hours per week, converted from its unit. A gauge that has not moved below its baseline, or a higher-is-better gauge, returns NOTHING (`None`), never zero and never a negative: deterioration is reported as deterioration. The claim is the weaker of the reading's claim and the frequency's: `instrument-measured` when `per_week_evidence` is given, else `estimated`. A part may also return time when the habit is green. A return is a rate and never a level, so it is never added to progress.

## 4. Episodes, red mass and the history walk

`epr flow project` gains `derive_habits`, and `epr flow stocks --stock red-mass` calls the same derivation directly, so it is never stale. Both walk the habit atoms' **first-parent** history from HEAD with one `git log --first-parent` and one `git cat-file --batch`. Per changed atom per commit, the walk computes the habit's state:
- status;
- parts;
- progress;
- red mass, which is `0` when green and `1 − progress floor` otherwise.

Against the previous state it emits, dated at the commit's author date:

| change | event(s) |
|---|---|
| habit appears, not green | `Commitment` episode (valid_from = this date) + `Produce` red mass |
| habit appears, green | none (no red mass, no episode) |
| a reading moves progress up / down | `Consume` / `Produce` of the red-mass difference, classified `habit:progress` / `habit:regression` |
| a part added, removed or rescoped | the red-mass difference, classified `habit:rescope` |
| status red → green | `Consume` of the remaining mass (`habit:flip-green`) + a discharge event `fulfills: [episode]` |
| status green → red | a new episode `Commitment` + `Produce` red mass (`habit:flip-red`) |
| atom deleted (and not re-added under the same id in that commit) | `Consume` of the remaining mass (`habit:retired`) + a discharge of the open episode classified `habit:retired`. The commitments stock has no withdrawn arm; §8 records that gap. |

Each event's `resource` is the red-mass resource (`PinnedRef { id: "register:red-mass" }`). Its `classified_as` is `[kind, <id>, "commit:<sha12>"]`. Every event is deterministic from git, so `project` dedupes it by CID and two clones derive the same events.

**Integrity of the walk.** Merges are diffed against their first parent (`--diff-merges=first-parent`). Renames are read as a removal plus an addition and reconciled by id within the commit, so a move is not a retirement. A revision whose frontmatter cannot be read is skipped and listed as unreadable; it never implies a retirement or a zero. A shallow clone is refused: its oldest commit would read as every habit's birth. Each standing habit carries the date it was born, so a window counts only habits that existed in it. Records are derived from the line being read; a rewritten line leaves its old records orphaned in the append-only sidecar while the red-mass reading, derived fresh, is unaffected. `project` dedupes events by `(verb, provider, resource, process, occurred_at)`; because each habit is its own resource, two habits never collide, and the residual case (the same habit, the same verb, two commits in the same second) can only lose a ledger copy, never a reading.

**Readings come only from the atoms.** A lane report becomes a reading when its result is recorded in the atom with the run id as `evidence`, by the agent at its DELTA or by the `just measure` listener. Live, gitignored, power-loss-prone reports are never read as state. A historical window is therefore reproducible from git alone.

## 5. Spending time

`epr flow habit spend --habit <id> --hours <h> --meter <pipeline|gate|session|other> --evidence <ref> [--as …] [--session …]` appends one event:

> `Consume` of the time resource (`PinnedRef { id: "resource:time" }`), `Magnitude::Count { value: h, unit: "hour" }`, `in_scope_of: habit scope`, `fulfills: []`, `classified_as: ["cost:<meter>", <id>, "claim:<kind>", "evidence:<ref>", steward slot]`, dated at HEAD's author date.

The attribution arms are `epr flow note`'s, reused rather than copied. `--habit` is checked against the register, as `claim --serves` is.

**Denominate in the witnessable** (middot Commitment II):
- `pipeline` and `gate` meters with evidence are `instrument-measured`.
- `session` is `estimated`: advisory forever, and shown as such.

**Witness, don't pay** (Commitment I): the time bank is an account of hours, never a credit.

**Budget.** The hours spent in the current episode (`occurred_at ≥ valid_from`) are judged against the episode's bound by `fold::level_evidence`, which gives an approach or a breach. A budget change in the atom mid-episode does not move the open promise. The reading shows the declared budget beside the promised one.

## 6. The readout

`epr flow stocks --window A..B --per week --stock red-mass [--json]`:

- **The stock.** `stock_over_window_within(red-mass, Within::Anywhere, events, window, "habit-equivalent")`:
  - level = the red mass at B;
  - inflow = regressions, new reds and rescope-up over the window, per period;
  - outflow = progress, flips and retirements, per period;
  - the verdict is the unchanged `equilibrium_verdict`: `DRAINING` when outflow ≥ inflow.
  - `observed_in_window` counts the **events** admitted, not the quantity, so a 0.0625 drain is one observation and not a rounded zero.
- **Per habit,** a row with:
  - status and composition;
  - progress floor, measured value and coverage;
  - red mass at A and B, and the in-window drained/filled split by kind (progress, regression, rescope, flip, retire);
  - the open episode and its bound;
  - hours spent in the window and in the episode, by meter and claim;
  - hours per week returned;
  - payback in weeks (`coverage_time`);
  - every part, with its reading, claim, `at`, evidence, and invalid or deteriorated flags.

`habits-status.py` reads that JSON. It never computes the scalar itself, so there is one evaluator.
- **The headline** gets one segment over the last 7 days: `red mass 31.40 of 34 not green · drained 1.20 · filled 0.40 · 5 partly won · 6.5 h spent · 0.37 h/wk returned (estimated) (7d: DRAINING)`.
- **`--full`** gets `progress`, `spent` and `returned` columns.
- When the binary is older than the measure, the segment reads `not read`, never zero.

The measure records (`habit-progress@1`, `red-mass@1`, `habit-hours@1`) are added to `.claude/epr-meta/measures.yaml`. Their procedures name these commands.

## 7. API surface

**`elohim-epr-rea` (pure, domain-neutral):**
- **`progress` module (new; admitted under the guidestar's §3a — Meadows' goal gap, Beer's achievement indices and VF partial fulfillment all need it):**
  - `Part`: `Ratio { passed, declared }`, `Gauge { baseline, target, reading }` or `Check { holds }`;
  - `Part::validate()`, `Part::progress()` and `Part::deterioration()`;
  - `compose(rule: Composition, readings: &[Option<f64>]) -> Composed { rule, declared, measured, floor, over_measured }` — `declared` is the readings' length, `None` is unmeasured;
  - `recurring_return(part, scale, occurrences, per, claim, basis) -> Option<Quantity>`, a Rate; the caller passes the weaker of the reading's and the frequency's claims.

  None of it mentions habits. It is "how far a promise's declared parts have moved toward their targets", reusable by any consumer: a pool's charter goals, a learning path's mastery parts, a device's arrival conditions.
- **`fold::level_evidence(commitment_cid, bound, level)`:** public. It is the same function `fulfillment` uses, now reusable by any reader that judges a level it measured some other way. The WIP fence in `stocks.rs` currently re-implements it inline.
- **`stock::coverage_time(level, rate)`:** public. It is the `divide` that `turnover_time` already uses, typed for Level ÷ Rate, and it keeps the weaker-claim and NaN-on-unknown discipline.

- **`ClaimKind::rank()` / `ClaimKind::weaker()` in `elohim-epr`:** the claim ordering every fold reduces by, promoted from two private copies (`stock.rs`, `fold.rs`) so a third consumer does not write a third.

**`epr-cli` (habit glue):** `flow/habits.rs`, which holds:
- the typed `HabitRea` declaration, as the schema of record;
- the history walk;
- the red-mass report;
- `spend`.

`project.rs` calls `derive_habits`; `stocks.rs` gains `StockName::RedMass`; `mod.rs` wires `habit spend`.

## 8. Open frontier

- **A withdrawn arm for retired promises.** VF has no "withdrawn" discharge in the commitments view (`no path mints a Revoked commitment`), so a retirement is classified `habit:retired` on its discharge.
- **Metered attribution.** Folds of `delivery-cost@1` and `gate-cycle-seconds@1` could become spend events on the habit their bound names. Today `spend` is explicit.
- **Independent recomputation.** No reading here is `witnessed` in the middot sense until a peer recomputes it; it waits on the fold-attestation path.
- **Two limit models.** `epr flow report`'s lenses (`flow/measures.rs`) carry their own `Bound` (`soft`/`hard`) and `Compare`, importing nothing from epr-rea's `Bound` (`limit`/`threshold_pct`/`Sense`). This slice chose epr-rea's `Bound` for the habit budget rather than adding a third; joining the two is the first candidate of the substrate-bypass census (2026-10-22), not this slice. The census is now carried by habit `sensing-rides-the-measure-plane` (`elohim/eprfs/.epr-meta/`, red; detector `epr flow sense`, measures `limit-models@1` and `concern-joins-unresolved@1`), with `ledgers-fold-once` for the private ledgers; design `genesis/docs/superpowers/specs/2026-10-22-sense-organ-design.md`. Its ruling on signature 2 (false positive) agrees with this slice's reading.
- **First declarations (2026-10-22).** Red-habit budgets follow one rule: 4 h per part short of target, at an 85 % band. One time gauge carries a frequency: push-delivers' `station-2-window`, `per_week: 4`, from fleet rolls on dev. That is a floor, so its time returned (~0.04 h/week) understates the household restarts it also saves. Each budget binds a habit's next red episode; an open episode keeps the bound it was minted with.
- **Weights.** Parts count equally under `sum`. A habit that needs weights should first ask whether it is two habits.
