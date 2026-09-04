---
epr-habit-version: 1
id: dev-system-equilibrium
invariant: >
  Every stock in the development system drains at least as fast as it
  fills — measured as rates over a declared window, never as a level.
  Equilibrium is drain >= inflow per stock; a check that reads green
  because it measured nothing is an over-claim, not a green.
status: red
active: false
checks:
  - "epr flow stocks --window <START..END> --per week --stock commitments --check --root /projects/elohim (exit 0 = every declared stock draining; fail-closed: refusals exit non-zero, never green-on-nothing)"
  - "a2o @concern:dev-system-equilibrium (genesis/a2o/features/devflow/run-plane.feature — 4 scenarios: filling exits non-zero, drain>=inflow exits zero, an unmeasurable window refuses rather than reporting equilibrium, and the @regression guard that a discharge counts as outflow and never as inflow. @wip: no step definitions drive them yet, so this check runs in NO suite today — the CLI clause above is the only executing proof)"
guard: >
  Regression risks: (1) the outflow classifier keys on `fulfills` — a new
  discharge path that forgets the field silently under-drains (the level
  parity check with `epr flow status` is the tripwire); (2) discharge
  dedup is occurred_at-sorted, never append-order (the C2 channel,
  pinned by the_drain_is_dated_by_occurred_at_never_by_append_order);
  (3) greening this habit by widening the outflow arm instead of by
  actually draining commitments is the over-claim the invariant names.
refs:
  - "genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md"
  - "genesis/docs/superpowers/plans/2026-08-13-agentic-harness-borrows-implementation-plan.md"
  - "genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md — row 7"
  - "displaced: declarative-desired-state → genesis/data/timeline/backlog/declarative-desired-state-parked-habit.md (2026-08-13, operator-directed; returns when its brit/eprfs precondition greens)"
retire-when: >
  when every declared stock's drain>=inflow assertion runs inside the pre-push gate and has
  held a full quarter with no operator override. A system actually in equilibrium does not
  need a weekly reader.
---
DELTA 2026-09-05 (valueflow authoring surface closed through its own verbs; no status
flip): the plan's 11 tasks were claimed and fulfilled with `epr flow claim` / `fulfill --on`,
lifting commitments outflow 0.429 -> 2.000/day on the 2026-08-29..09-05 window (level 616,
inflow 5.571/day, still FILLING). Task-level fulfilment is now a real drain path; the rate
is still net positive, so RED stands. An event, not a rate.
DELTA 2026-09-05 (first task-level fulfilments via epr flow claim/fulfill;
event, not a rate; no status flip): valueflow-authoring Task 11 dogfooded
the new verbs against the Holochain Evolution Epic MVP plan — decompose
re-ran (20 gap-items), then three landed epic tasks were claimed and
fulfilled as agent:implementer@claude-opus-5 (Task 16 roster check,
825a090df + fix 4425bb6fb; Task 17 constitution_root, 10cb3dc00; Task 18
export_held_records, 4fe69b918); no tool:decompose-claim commitment held
any of the three intents, so --supersede was never exercised. Stock
reading over 2026-08-29..2026-09-05 --per day --stock commitments,
identical window/flags before and after: BEFORE level 614, inflow
3.286/day, outflow 0.000/day, FILLING; AFTER level 616, inflow 4.000/day,
outflow 0.429/day, FILLING. Outflow moved (0.000 -> 0.429/day, 3 consume
events newly witnessed in the window) — this is the event this habit
exists to witness becoming visible, not evidence of a sustained rate:
three fulfilments in one authoring session, not a drain the stock can
count on. Habit remains RED; equilibrium is unproven and the check stays
FILLING.
DELTA 2026-08-16 (developer CLI drain; no status flip): the committed
command stock fell 362→322 while its public surface converged to eight
root verbs; 32 local gates now have one manifest-declared detector and
executor, eliminating the pre-push two-map inflow. This is a local tooling
outflow, not evidence that the commitments stock itself is draining;
habit remains RED on its existing measured rate.
DELTA 2026-08-14 (leg 2 close, run #1349 banked): OUTFLOW MOVED A SECOND
TIME — 2.000/wk -> 3.000/wk on a genuinely NEW consume event (level
559->558, consumed 19->20, fulfill ledger 'fulfilled (new): 1'), not the
re-discharge that held the reading flat at #1348. Sequence since birth:
0.000/wk -> 2.000 (#1345, first ever) -> 3.000 (#1349). That is the
event-becoming-a-rate this habit exists to witness. Verdict stays
FILLING and the habit stays RED — inflow 23.000/wk against 3.000/wk
drain is still a filling stock, and the fail-open defect below is
unfixed in epr itself.
DELTA 2026-08-14 (leg 2): the check FAILS OPEN and its green cannot be
trusted yet — REPRODUCED: with git unreadable (the dubious-ownership
state a fresh container starts in), `epr flow stocks` reads inflow 0.0
while level+outflow survive, so outflow >= inflow holds trivially and
the verdict reads DRAINING. It asserts "drained" precisely when blind,
and did so in the wild (the 17:05Z SessionStart fold cached draining for
a stock filling at 23/wk). Control vs HOME=/nonexistent, identical
window/stock/binary and byte-identical flows.jsonl (2502991): inflow
23.0/filling vs 0.0/draining. This — not the window definition — is
what disqualifies equilibrium as a stasis TERMINATION criterion, since
this invariant's own words ("a check that reads green because it
measured nothing is an over-claim") name exactly this failure.
Mitigated fail-closed in run-projection.py (git_readable gate, honest
absence); the typed refusal still owed in stocks.rs, so any direct
caller of `epr flow stocks --check` remains exposed —
backlog/equilibrium-inflow-fails-open-to-false-draining.md. Outflow did
NOT move a second time at banked run #1347: still 2.000/wk from the same
two events (ch04's recovery was a re-discharge of an already-drained
commitment, correctly not counted as outflow). CHECK=1, FILLING.
DELTA 2026-08-14: FIRST OUTFLOW — banked run #1345 (validate-only,
quiesce-gated) fulfilled 2 commitments (ch11 pull-queue-retires
first-ever green + 1); 2026-08-08..2026-08-15 --per week reads
outflow 2.000/wk (was 0.000 since birth), level 561->559, verdict
still FILLING (inflow 23.000/wk, net +21), CHECK=1 — the drain arm
is witnessed live, the equilibrium target stands.
DELTA 2026-08-13: T8 close-the-loop — a2o scenarios landed
(@concern:run-plane-note, @concern:run-plane-projection,
@concern:dev-system-equilibrium; step-def wiring NOT written — all 11
scenarios dry-run undefined and the feature carries @wip, so the story
is authored and blind-read but not yet executing), all tree gates green
(a2o LINT/FMT/TSC/GHERKIN/UNIT=0, 180 unit tests; eprfs workspace
FMT/CLIPPY/TEST=0), final CHECK=1 (FILLING at 22.000/wk inflow vs
0.000/wk outflow, level 560, unchanged from the T2 red).
RED WRITTEN 2026-08-13 (T2, commit cae50c1, the unwired->red first_move
completed same day as admission): live reading over
2026-08-06..2026-08-13 --per week: commitments stock level 560,
inflow 22.000/wk, outflow 0.000/wk, net +22.000/wk, verdict FILLING,
CHECK=1. Level independently agrees with `epr flow status`
unfulfilled (total): 560 — same fulfills-keyed predicate, two readers.
Turnover NaN (honest absence, not +inf). Outflow arm v1 = fulfillment
discharge only (Dismiss is a regression marker here, NOT a dismissal
— counting it would double-drain; printed on every render). Rides
Stock{level,inflow,outflow} + MeasureKind::Rate{per} (measure-family
rows 12-14); 28 tests; fail-closed on MismatchedPeriods / empty
window / NaN rate.
