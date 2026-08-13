---
title: "Dev-system equilibrium stocks — drain rate against inflow rate, per stock, weekly"
id: dev-system-equilibrium-stocks
tier: spec
status: Draft
class: process-meta
context-tier: disclosed
steward: cartographer
graduation-trigger: "`epr flow stocks --check` runs inside the eprfs pre-push gate and the elohim-eprfs pipeline, red-or-green from evidence, AND habits.yaml carries the operator-ratified displace/admit delta — OR the operator declines admission, in which case the leg graduates as a read-only instrument and this spec's habit section is struck rather than left aspirational"
created: 2026-08-13
domain: D2
topic: [meadows, stock, flow, rate, equilibrium, habits, epr-rea, epr-cli, measure, harness, commitment]
informed-by:
  - genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md
cites:
  - genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md
  - genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md
  - genesis/manifests/habits.yaml
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
  - measure-dynamics-confidence-ontology-design | Measure Dynamics + Confidence Ontology | sha256:52d601baa6117450 | path: genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md
  - epr-rea-valueflow-fabric | EPR-REA ValueFlow Fabric | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md
  - elohim/epr-rea/src/stock.rs
  - elohim/epr/src/measure.rs
  - elohim/epr-rea/src/fold.rs
  - elohim/eprfs/epr-cli/src/flow/mod.rs
  - elohim/eprfs/epr-cli/src/flow/project.rs
---

# Dev-system equilibrium stocks

> **One-line:** every stock in the development system gets one runnable check — **drain rate ≥
> inflow rate, weekly** — computed as a pure fold over the flow log we already project, with no
> new storage and no new measure primitive. First stock: the commitment stock. **This spec
> carries the habit admission**, and the admission is the operator's to grant or refuse.

## 1. Provenance and the seam this fills

Cluster row 7 of the agentic-harness borrows backlog — the one row marked **synthesis-born**: a
Meadows lens laid over the context-engineering survey rather than a claim lifted from a primary
(`genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md:45`). Its finding is short and
uncomfortable. Our gates read **stocks**: unfulfilled commitments, cleanup pressure, `MEMORY.md`
over cap. A stock level says nothing about controllability. The honest measure is **rates against
rates**, and equilibrium is then a check anyone can run.

The measure family this rides was built for exactly this and is already landed. `stock.rs`'s own
header states the discipline: *"a **stock** is a level, accumulated by past history; **flows** are
rates that fill and drain it; and the diagnostic quantities are ratios *of rates*, never levels
against ceilings"* (`elohim/epr-rea/src/stock.rs:5-7`), and the leading-indicator argument that
makes a rate check worth having at all — *"deforestation is indicated not when the forest is gone,
but when the rate of harvest first exceeds the rate of regrowth"* (`stock.rs:31-33`). The survey's
own warning applies to this instrument as much as to the harness parts it grades: scaffolding
depreciates, and anything we build must be load-bearing *now*
(`genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md:306-308`,
§4.6). The backlog row states the composition plainly: the primitives already landed, so **the
measure wiring needs no new primitive**; what is gated is the covenant decision
(`agentic-harness-borrows-backlog.md:45`).

This spec composes and does not fork: the flow projection, its recipes, and the sidecar are the
valueflow fabric's (`genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md`),
and the measure vocabulary is the measure-dynamics spec's six laws
(`genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md`).

## 2. The measure

**Equilibrium, per stock, weekly: `outflow ≥ inflow`.** Not a level under a ceiling — a drain rate
at or above a fill rate over a declared window. Zero net change is the target state and is
emphatically not stillness: *"A corpus with high inflow and equally high outflow is healthy; one
with zero of both is not stable, it is dead"* (`stock.rs:153-155`).

Three properties make this a check rather than a dashboard number:

- **The window is part of the measure's identity, not an argument default.** `Window { start, end,
  per, periods }` (`stock.rs:80-90`) declares `periods` rather than computing it from `start`/`end`
  — *"it forces the author to state the denominator their rate is actually per, instead of
  inheriting whatever a subtraction happened to produce"* (`stock.rs:70-73`). The doc comment
  records why: on the first live run of the doc-corpus index the 28-day window **alone** decided
  whether the headline read `unknown` or `3.2` (`stock.rs:64-67`). A weekly equilibrium claim that
  hides its window is not a claim.
- **Dimensional safety is by construction.** `Stock::new` refuses any incoherent combination before
  arithmetic happens: the level must be a `Level`, both flows must be `Rate { per }`, and the two
  periods must match (`stock.rs:121-148`, `StockError::MismatchedPeriods` at `stock.rs:137-141`),
  over `MeasureKind::{Level, Rate{per}, Ratio}` (`elohim/epr/src/measure.rs:30-34`).
- **Absence stays absence.** A zero denominator yields `Interval::unknown()` and a `NaN` value, never
  `+∞` — *"`+∞` is a claim and absence is not"* (`stock.rs:200`; the NaN rule at `stock.rs:315-319`).
  A stock with no counted drain must read *unknown*, and `--check` must not read unknown as green.

**Two comparisons, both safe.** `outflow / inflow` is `Stock::harvest_regeneration` (`stock.rs:226`);
`inflow / outflow` is `Stock::emission_absorption` (`stock.rs:215`). They are exact reciprocals and
the direction is where the reasoning error lives (`stock.rs:219-228`) — our development stocks are
**sinks** (work is emitted, draining absorbs it), the same orientation the doc corpus already uses
(`stock.rs:28-29`). The comparison is two `Rate`s of **equal** `Period`, which `MeasureKind::divide`
resolves to a `Ratio` (`measure.rs:67-85`); different periods are refused outright, never silently
converted (`measure.rs:78-80`).

## 3. First stock — commitments over the projected flow log

**Level** = active unfulfilled commitments. **Inflow** = commitments minted per week. **Outflow** =
(fulfilled + dismissed + revoked) per week.

> **Amended at implementation (T2, commit cae50c1) — the outflow arm is narrower than this
> paragraph stated, and the render says so on every readout.** Read against `fulfill.rs:360-393`,
> `ReaVerb::Dismiss` in this tree is a **regression marker**, not a commitment dismissal: it is
> minted only when an already-discharged commitment goes red, with `fulfills: []`, unit `red-run`.
> Counting it as outflow would drain the same promise twice and push the level below the
> `unfulfilled_total` every other reader quotes — so it is `NotCounted::DischargesNothing`, pinned
> by `a_re_fulfillment_does_not_drain_the_same_promise_twice`. The `revoked` arm is structurally
> unreachable today (577/577 live commitments `active` — Q-C confirmed) and reads as absence, not a
> counted zero. **Outflow v1 = fulfillment discharge only**, and the leg prints
> `outflow arm: fulfillment only …` on every render rather than hiding the narrowing. Two further
> implementation corrections: refusals on this path are `StockError` variants (not
> `FoldError::{Empty, MixedKinds}` — that enum belongs to `fold::with_uncertainty`, which a
> single-stock fold has no reason to call), and `--window`/`--per` are **required** (refused when
> absent — a default would be a wall-clock claim on a history-derived path), with
> `--per month|year` refused rather than approximated. Run-notes (`ReaVerb::Cite`) move this stock
> in neither direction — Q2's answer, pinned by `a_run_note_moves_no_number_in_this_stock`.

The level predicate already exists and is not re-derived: `unfulfilled_in_scope` keeps a commitment
that is `Proposed | Active` and whose CID appears in no event's `fulfills`
(`elohim/epr-rea/src/store.rs:131-146`), which is the same set `epr flow status` already counts
(`elohim/eprfs/epr-cli/src/flow/walk.rs:405-471`). The backlog's row 4 names precisely the gap this
row is the measure half of: `epr flow status` **counts but does not dispatch**
(`agentic-harness-borrows-backlog.md:42`).

**The fold is `stock_over_window` unchanged** (`stock.rs:343-468`): level is cumulative
`produced_all − consumed_all` through `window.end` and is deliberately **not** windowed, while
inflow/outflow are the in-window events divided by `window.periods` (`stock.rs:334-337`). Claims come
back `Witnessed` because the events are the observation (`stock.rs:339-342`).

**What the leg must build is the derived event view, and that is the whole risk.** The raw sidecar
records do not fold correctly as they stand, for three grounded reasons:

1. **A fulfillment is a `Produce`.** `epr flow fulfill` mints the discharging event as
   `ReaVerb::Produce` with `fulfills: vec![*commit_cid]` and unit `green-run`
   (`elohim/eprfs/epr-cli/src/flow/fulfill.rs:336-352`). Fed to the fold raw, every fulfillment
   would count as **inflow** — the exact sign inversion `stock.rs:219-228` warns about, arriving
   through the data rather than through the caller's choice of index.
2. **A dismissal is invisible.** Regressions mint `ReaVerb::Dismiss` with unit `red-run`
   (`fulfill.rs:366-373`), and the fold's match arm reads only `Produce`/`Consume`, dropping
   everything else (`stock.rs:423-437`). Unmapped, the outflow silently under-counts.
3. **Unit matching is exact-string.** `count_in` admits only `Magnitude::Count { unit }` whose unit
   equals the requested one (`stock.rs:470-475`); a mismatch is silently excluded, not converted and
   not errored. The view therefore mints one unit — `"commitment"` — for every arm.

So the leg projects a `Vec<FlowEvent>` view on a synthetic commitment-stock resource: **mint →
`Produce`**, **fulfilled/dismissed/revoked → `Consume`**, `Magnitude::Count { value: 1.0, unit:
"commitment" }`. Zero new storage, zero new primitives, one new CLI leg plus call-site window
construction — and a projection function whose unit tests are the actual deliverable.

**Timestamps stay git-derived; a commitment's mint instant needs one hop.** `FlowEvent.occurred_at`
is history-derived by construction on the projection path — *"Deterministic and history-derived like
every other timestamp on this path — never `now()`"* (`elohim/eprfs/epr-cli/src/flow/project.rs:353`,
the `git log … %aI` format string at `project.rs:364`). A `Commitment`, however, carries
`valid_from: Option<String>` that **both** mint sites leave `None` — the gap-item claim at
`project.rs:548-560` and the a2o scenario at `project.rs:585-598` — and the sidecar itself is
gitignored and explicitly not source-of-truth (`.gitignore:112-114`: *"reconstruction = re-run `epr
flow project`. Never source-of-truth."*), so there is no appending commit to read a mint instant
from. The honest source is the artifact's own history through the existing helper
`producing_commit(root, rel) -> (author, RFC3339 oldest-add)` (`elohim/eprfs/epr-cli/src/flow/mod.rs:521-530`),
which `derive_process_doc` already uses (`project.rs:484`): a scenario commitment carries its
repo-relative feature path at `classified_as[1]` (`project.rs:585-591`), so its mint instant resolves
directly. Gap-item claim commitments carry `["gap:claimed", item.id]` instead (`project.rs:522-530`)
and resolve only as coarse as their gap doc — see Q-B in §9.

Where confidence intervals apply — a coarse or partially-resolved mint instant is exactly such a case
— the leg composes them through `fold::with_uncertainty` (`elohim/epr-rea/src/fold.rs:240-293`),
which refuses mixed `MeasureKind`s, carries the weakest claim, and attaches the least-tightenable
`UnknownReason` when the fold comes back unknown (`fold.rs:258-283`). It does not invent a narrower
number than the inputs support.

## 4. Mechanism — `epr flow stocks`

A new, **read-only** leg on the existing `epr flow` dispatcher (`elohim/eprfs/epr-cli/src/flow/mod.rs:77-148`),
sitting beside `walk` and `status`, the two legs that already open the sidecar and write nothing:

```
epr flow stocks [--window START..END] [--per week] [--stock commitments] [--check] [--json] [--root DIR]
```

- `--root` / `--json` are the existing global options, stripped by `parse_global` before leg parsing
  (`mod.rs:63-69`, `mod.rs:263-291`).
- `--window` / `--per` construct the `Window` at the call site. Two windows are the intended default
  reading — the same shape the doc-corpus instrument already renders, where the 28d/90d/365d spread
  is what distinguishes an honestly unbounded ratio from a converged one
  (`genesis/manifests/habits.yaml:640-644`).
- `--stock` names which stock to fold; `commitments` is the only one at birth, and §6 is how the
  others arrive.
- **`--check` exits nonzero when `outflow < inflow`.** That is the runnable habit check. An
  *unknown* index is not a pass: a stock whose counted drain is zero reports unknown by construction
  (`stock.rs:200`, `stock.rs:315-319`) and `--check` must fail it, because "we cannot see the drain"
  and "the drain is adequate" are the two states this whole vocabulary exists to keep apart.

**No new register.** This is a second reader for the flow log and `habits.yaml`, in the same spirit
the survey's TAKE-1 declines a `current.md` (`context-engineering-primary-sources-cross-pollination-2026-08-13.md:326`,
LEAVE-11). Nothing is persisted; re-running re-derives.

## 5. Habit admission delta — **operator-gated at review**

`habits.yaml` is full on both fences: **max 12 habits** and **max 2 `active`**
(`genesis/manifests/habits.yaml:40`, `:45`), and the file currently holds exactly 12. So admission
**displaces or waits**, and the backlog row states whose call that is: *"that is an operator
decision, not an agent's"* (`agentic-harness-borrows-backlog.md:45`). What follows is a **proposal
for review**, not an edit to be applied by an agent.

**Displace: `declarative-desired-state`** (`habits.yaml:521-537`). The evidence is in its own entry:
`status: unwired`, `active: false`, no `evidence:` field at all, and a `first_move` that ends *"park
until this node is green"* (`habits.yaml:534`) — naming a different habit's completion as its stated
precondition. The file itself marks it deliberately parked. **Its commitment is not cancelled**: the
invariant and its in-protocol precedent survive in its `refs:` (`habits.yaml:535-537`) and in the
backlog, and it re-enters the register when its precondition greens. Displacement here is the WIP
fence doing its job, not a retraction.

**Admit: `dev-system-equilibrium`.**

- **invariant** — every stock in the development system drains at least as fast as it fills, measured
  as two rates of one declared period over a declared window, never as a level against a ceiling.
- **status: `unwired`**, `active: false`. Per covenant rule 2 the only legal first move for an unwired
  habit is writing the red (`habits.yaml:43-44`); prose does not advance it.
- **first_move** — land `epr flow stocks --check` over the commitment stock and record whatever it
  says. If it exits nonzero on first run, that is the red, and the red is the deliverable.
- **checks** — (1) `epr flow stocks --check --stock commitments` (the CLI check); (2) *later* one
  weekly rate line in the SessionStart headline, which is `checks:`-eligible only once it is rendered
  from the same fold rather than a parallel implementation.
- **guard** — two named regression channels, both already documented on the sibling habit: (a) the
  **two-implementation hash-exclusion invariant**, where adding a key on one side only produces a
  *silent un-enforcement* (`habits.yaml:657-664`), and its generalization recorded in the same guard
  — *"The Python doc-corpus instrument and the Rust `Stock` are parallel implementations of one
  shape, held together by review and by mirrored tests, not by codegen"* (`habits.yaml:665-667`); a
  headline rate line rendered by Python beside a Rust fold would be a third such pair. (b) the
  **`gate_honesty` lesson from `measure-honesty-local`**: until 2026-08-12 `elohim/epr-rea` had *no*
  build-manifest entry, no pipeline and no pre-push case, so an epr-rea-only change matched no glob
  and shipped un-gated — *"Every green this habit recorded before 2026-08-12 therefore rests on two
  checks that CI never executed"* (`habits.yaml:600-613`).

**Because of (b), this check names its CI home at birth.** The leg lands in `elohim/eprfs/epr-cli`,
a member of the eprfs workspace (`elohim/eprfs/Cargo.toml:3-13`), which has both a pre-push block
firing on `^elohim/eprfs/` (`.husky/pre-push.bash:330-336`, `cargo test --workspace`) and a pipeline
— `elohim-eprfs`, `gate.projects.eprfs → elohim/eprfs` (`elohim/eprfs/build-manifest.json:3`, `:33-35`),
whose stage runs `cargo nextest run --workspace --all-targets` (`elohim/eprfs/Jenkinsfile:128`). One
seam stays honest only by declaration: `epr-cli` path-deps `elohim-epr-rea` across a workspace
boundary (`elohim/eprfs/epr-cli/Cargo.toml:22`), while the `elohim-epr` gate fires on
`^elohim/epr/|^elohim/epr-rea/|^elohim/sdk/epr-ts/` and compiles neither CLI
(`.husky/pre-push.bash:474-476`). An `epr-rea` change can therefore still break this leg without any
gate compiling it — recorded here as a guard rather than discovered later.

## 6. Later stocks — one leg, many authored observations

The commitment stock folds because its events are already projected from git. The other stocks named
in row 7 — memory-index bytes, cleanup pressure — have no such log, and the answer is **not** a
second measurement path. They arrive as **authored `run:observation` events** on the run plane
(cross-cite: `genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md`),
each observation an event in the same flow log, folded by the same `--stock` leg over the same
`Window`. That keeps the honest-absence property intact end to end: a stock nobody has observed reads
*unknown*, not zero, which is the distinction `UnknownReason` exists to preserve (`measure.rs:8-11`).

## 7. P2P design gate

**Entity classification: (d) — derived, no persistence.** A `Stock` is never stored: *"A `Stock` is
never stored. It is a pure fold over `FlowEvent` history … A stored level would be a second home for
a number that already has one"* (`stock.rs:15-20`). Nothing here mints a DHT entry type, a coordinator
function, a signal, an HTTP surface, or a table; the identity questions are therefore moot by
construction, and the only address in play is the content-derived CID the flow records already carry
(`elohim/epr-rea/src/store.rs:34-46` per the fabric spec's §6). The sidecar is gitignored, derived,
and reconstructable by re-running `epr flow project` (`.gitignore:112-114`) — class **C** in the
fabric's vocabulary (`2026-07-18-epr-rea-valueflow-fabric-design.md:220`). Non-git stocks do not
change this: they arrive as §6's authored observation events, which are the run plane's mint, not
this leg's. Head-plane cost: **zero rows added** by this spec.

## 8. Decomposition (gap-items)

- [ ] The `stocks` leg — dispatcher arm beside `walk`/`status`, the derived commitment-event view
      (mint→`Produce`, fulfilled/dismissed/revoked→`Consume`, unit `"commitment"`), and window
      construction from `--window`/`--per` (§3, §4).
- [ ] Unit tests: a two-window fold over a fixture `flows.jsonl` — one window where the index is
      bounded and one where zero counted drain must return *unknown*, plus a regression asserting a
      `Produce`-shaped fulfillment does **not** land in inflow (§3).
- [ ] `--check` semantics and exit codes: nonzero on `outflow < inflow`, nonzero on *unknown*, zero
      only on a bounded index at or below 1.0 (§4).
- [ ] `habits.yaml` delta — displace `declarative-desired-state`, admit `dev-system-equilibrium`
      (`unwired`, `active: false`) with the checks and guard of §5. **Operator-gated: present as a
      checkbox for ratification; an agent does not apply it.**
- [ ] Headline integration — one weekly rate line in the SessionStart memory-budget block, rendered
      from this fold rather than a second implementation, and honoring the standing consolidation
      concern: the headline is already a 3-deep python-spawning-python chain (~7 interpreters) marked
      *flatten to library calls* (`genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md:31-33`).
      Adding a subprocess to that chain is a regression against a filed item; the line arrives as a
      read of an already-computed artifact or not at all.
- [ ] Seam-registry row for the equilibrium verdict predicate. It is an **instantiation** of a shape
      `epr-rea` already owns, not a new cross-cutting shape, so by the birth rule it registers in the
      consuming crate (`crates/seam-contracts/seam-registry.yaml:3-6`) — `elohim/eprfs` has no
      registry yet, so this creates its first row-bearing one against
      `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json`.
- [ ] a2o scenario under `genesis/a2o/features/devflow/` (sibling of
      `developer-valueflow-projection.feature`, which already carries `@concern:epr-rea-valueflow-fabric`)
      with its own `@concern:` tag, asserting the two-window verdict and the unknown-is-not-green arm.

## 9. Open questions

- **Q15 (inherited, non-blocking).** `Level ÷ Rate` returns a bare `Level`, so the period a turnover
  time is denominated in survives only in the `basis` string — *"`3.0` here is three days or three
  years depending on a divisor the type no longer holds"* (`stock.rs:194-204`; the same gap named at
  `measure.rs:61-66`, deferred because minting a `Duration { per }` variant *"is not an implementer's
  call"*). **Non-blocking here**: `--check` compares two `Rate`s of equal `Period`, the arm
  `MeasureKind::divide` resolves to `Ratio` (`measure.rs:73-75`) — the safe side of the algebra.
  `turnover_time` may be *reported* but must not be a check predicate until Q15 closes.
- **Q-A. Which stock is authoritative for the habit's verdict when several are folded?** Row 7 states
  the invariant as *every* stock draining at least as fast as it fills, but the aggregation rule
  across stocks (all-must-pass, worst-index, or per-stock rows) is not settled here.
- **Q-B. The gap-item claim commitment's mint instant.** Scenario commitments resolve exactly through
  `producing_commit` on the path at `classified_as[1]`; gap-item claims carry `["gap:claimed",
  item.id]` (`project.rs:522-530`) and resolve only as coarse as their gap doc's own history, which
  biases inflow toward the doc's add date. Whether that coarseness is acceptable, widened via
  `Confidence::widen`, or cured at the projection is open.
- **Q-C. Whether `revoked` is observable at all today.** `unfulfilled_in_scope` excludes
  `CommitmentState::Revoked` from the open set (`store.rs:140-144`), but no projection path in
  `project.rs` mints a commitment in any state other than `Active` (`project.rs:557`, `:594`), so the
  revoke arm of the outflow may be structurally empty at birth. If so it must read as absence, not as
  a counted zero.
