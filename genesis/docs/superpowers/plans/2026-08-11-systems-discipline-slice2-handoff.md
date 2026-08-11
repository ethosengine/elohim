---
title: "Systems Discipline Slice 2 — handoff: build the stock-and-flow, and give the intervenors an exit"
id: systems-discipline-slice2-handoff
status: Draft
domain: D2
sprint: next
kind: handoff
cites:
  - genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md
  - measure-dynamics-confidence-ontology-design | the sealed canon this slice inherits — its six laws are the constructs slice 2 builds on, and its Q10-Q13 are the gaps that will bite a stock model on day one; its §4 network list is the OTHER slice 2 and explicitly not this sprint | sha256:2220560b22326a6e | path: genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md
  - measure-ontology-slice1-epr-local-first | the completed predecessor — what it established (kind/confidence vocabulary, closure law, both governance gates, one meter) and the landmines its run uncovered that this handoff carries forward | sha256:80c86e9af8f35b5e | path: genesis/docs/superpowers/plans/2026-08-11-measure-ontology-slice1-epr-local-first-plan.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - genesis/data/timeline/backlog/2026-08-11-carrying-capacity-cumulative-vs-rate-unit-error.md
---

# Systems Discipline Slice 2 — Handoff

**Read this first if you are picking up the systems-discipline work.** Slice 1 landed
2026-08-11 (`58d2238f0`). This document says what it established, what it deliberately
left, and what the next sprint should be — with the reasoning, so you can disagree with
it on evidence rather than re-derive it.

---

## 0. The scoping decision you must make first

**There are two different "slice 2"s, and merging them would wreck both.**

| | Slice 2 (network) | Slice 2 (systems discipline) — **THIS ONE** |
|---|---|---|
| Question | What happens when a measure crosses the network boundary? | What happens when we apply the ontology to our own development system? |
| Contract | Spec §4, six inherited-unsolved items | Meadows survey §4 leverage ladder, §5 traps, §6 worked application |
| Blocked by | Per-fold anonymity (blocks the network rung outright) | Nothing. Fully local, no capability dependency |
| Env | Needs the substrate | Runs on any laptop |

The spec's §4 list — per-fold anonymity, correlation-aware interval arithmetic, cross-peer
determinism at a band edge, statistical-method application at the ceiling, index lenses,
and the missing typed `measure::canonical_bytes(&Quantity)` entry point — **is not this
sprint.** It is a real contract and it is written down; leave it written down. If you find
yourself reaching for k-anonymity or DP noise budgets, you have drifted into the other
slice.

This sprint stays local and works on the development system itself, which the survey
argues (§6) is the correct first system to master.

---

## 1. What slice 1 actually established

Four things, all verifiable:

- **A vocabulary where the dishonest states are unrepresentable.** `MeasureKind::{Level,
  Rate{per}, Ratio}` in `elohim/epr/src/measure.rs` — a rate cannot forget its period.
  `Confidence{claim, interval, basis}` rides inside the canonical dag-cbor bytes, verified
  at the CID level.
- **A closure law.** `fold::with_uncertainty` in `elohim/epr-rea/src/fold.rs` returns an
  interval and takes the weakest claim among its inputs. Mixed kinds refuse to fold —
  including two rates with *different periods*, which the plan never tested for.
- **A governance gate on both paths.** `validate_meta` (manifest rules) and `load_policies`
  (registry policies) both refuse a `class: measure` declaration with no `kind`, sharing one
  `MEASURE_KIND_VOCAB`.
- **One meter on the front hall.** The session-start line, which is Meadows' own #6
  intervention applied to us.

**Sealed canon:** `2026-08-11-measure-dynamics-confidence-ontology-design.md`, six laws
L1-L6 each anchored to an enforcing construct, **Q1-Q14 open by design**.
**Habit:** `measure-honesty-local`, green, `active: false`.

### Where that left the leverage ladder

| # | Lever | Slice 1's effect |
|---|---|---|
| 10 | Stock-and-flow structure | **absent → expressible.** Vocabulary only; no stock is modelled anywhere |
| 6 | Information flows | one honest meter added |
| 3 | Goals | one evidence-bound habit minted |
| 12 | Numbers | **two new undeclared parameters** (28d window, `× [1,3]`) |
| 11, 9 | Buffers, delays | untouched; #9's ratio is now expressible, unbuilt |

The barbell the survey diagnosed is still a barbell. We gave the missing middle a
vocabulary, not a spine.

---

## 2. The work, ranked by leverage

### A. Give every intervenor an exit — `retire-when:` (survey §5, the escalating item)

The survey names *Shifting the Burden to the Intervenor* as **"the trap we are most
exposed to, and it is not close,"** and marks it WATCH **escalating to TAKE**. Measured
on disk 2026-08-11: **30 hooks in `.claude/hooks/`, 40 `.epr-meta` manifests, and exactly
zero removal conditions anywhere in the repo.** Slice 1 made this worse — it added two
more intervenors (the measure-tier `kind` gate, the scoreboard line) with no exit.

Meadows' Way Out is a design obligation on the intervenor: *restore the system's own
ability to solve its problems, **then remove yourself***.

- [ ] Add a `retire-when:` field to the `.epr-meta` rule shape and to hook/sentinel
      registration — a stated condition under which the intervenor is removed, not a date.
- [ ] Backfill it for the highest-traffic intervenors first. A rule whose retire-condition
      is genuinely "never" should say so explicitly and say why; that is an honest answer
      and a countable one.
- [ ] Surface the count of intervenors lacking a retire condition. **This is a `Level`,
      and it must declare itself as one** — it is the first real second consumer of the
      slice-1 vocabulary beyond `doc_dynamics`.

**The recursion is the test, and you must face it head-on.** A gate that counts
un-retired intervenors is itself an intervenor. It needs its own `retire-when:` — plausibly
"retire when the count holds at zero for N weeks." If you cannot write a retire condition
for this gate, that is strong evidence the whole mechanism is wrong, and you should say so
rather than ship it.

### B. Model one real stock — the doc corpus (lever #10, the structural gap)

Slice 1 made stocks *expressible*. Nothing is *modelled*. The survey (§3.1) calls this the
gap that puts levers #11 and #9 out of reach.

Model exactly one stock end to end, and pick the one we already have instruments for: the
live spec+plan corpus.

- [ ] A `Stock` shape: level, inflow rate, outflow rate — each carrying its own `kind` and
      `confidence`, per L1/L2.
- [ ] **Turnover time** (survey §3.9): stock ÷ outflow rate. This is the measure that
      distinguishes *dynamic equilibrium* from *silting* (§6.4), and the survey is explicit
      that stasis is the wrong target — throughput continuing while the stock stays bounded
      is the right one.
- [ ] Report it honestly. Current readings make this concrete: 28d → generated 63,
      absorbed 0; 90d → generated 319, absorbed 98; oldest live spec/plan dates to
      **2026-04-16**. With outflow at zero, turnover is unbounded — and the L3 honest-absence
      shape is the correct output, not a number.

Do **not** generalise to a stock-and-flow framework. One stock, modelled honestly, beats a
framework with nothing in it.

### C. Declare the window (lever #12, done properly this time)

The final review's sharpest live finding: **the 28-day window alone decides whether the
headline reads `unknown` or `3.2`.** A measure whose conclusion flips on an undeclared
parameter is not yet honest, and slice 1 shipped two such parameters (the window and the
`× [1,3]` multiplier).

- [ ] Make the window part of the measure's identity and its `basis`, not an argument
      default.
- [ ] Report more than one window, or justify the single one. Absorption here is **bursty**
      — a large sweep clustered 2026-06-01→06-11, then nothing since 2026-07-11 — so any
      single window is a claim about what counts as "now."

### D. Ratchet evidence to best-observed (survey §5, Drift to Low Performance)

Her fix is concrete and cheap, and we did not take it in slice 1: *"let standards be
enhanced by the **best** actual performances instead of being discouraged by the worst."*

`habits.yaml` evidence records **last**-observed. We have a documented eroding-goal channel
— PVC-deferral making "green" mean "deferred" — and last-observed evidence cannot
distinguish the two.

- [ ] Record a high-water mark alongside current evidence, so a later deferred-gate green
      reads as *visibly weaker* rather than equivalent.
- [ ] `measure-honesty-local`'s own evidence block is the natural first subject.

### E. The respite/response ratio (lever #9)

Now expressible as a `Ratio` and still unbuilt. Meadows' controllability index: problem
growth rate ÷ response rate. It is the measure that says whether *"try harder"* is even on
the menu — and §6.3 argues it is not, because effort-as-a-lever is the *confuse effort with
result* trap.

- [ ] Build it on top of B, once a stock has real inflow and outflow rates.

### F. Correct the survey's own §6.2 — the instrument now disagrees with it

**Do this early; it is small and it is the cleanest demonstration that the ontology works.**

§6.2 claims *"we are in overshoot, and we can prove it in her own index,"* citing
`MEMORY.md` bytes ÷ budget (1.38) and cleanup pressure ÷ threshold (1.73). Those are
**levels against ceilings**, not rate ratios. Meadows' harvest/regeneration index is a
*rate* ratio. §6.2 committed the same dimensional error the survey itself diagnoses in
`spatial_capacity.rs` — and it is exactly what L1 exists to make unrepresentable.

The conclusion survives; the proof does not. A proper rate ratio gives **319 / 98 ≈ 3.3 at
90 days** — genuine overshoot, by a different measure than the one claimed.

- [ ] Annotate or correct §6.2 with the rate-ratio reading and note which of its rows are
      levels. When a doc and a live probe disagree, the probe is the authority.

---

## 3. Ontology gaps that will bite you immediately

Q10-Q13 are not optional polish. **B and E will hit them on day one**, so close them first
or plan around them deliberately.

- **Q12 — `Interval::is_unknown()` has no sign check**, so `[+inf, +inf]` reports as honest
  absence. Build a stock whose outflow is zero and you will produce that exact shape; L3's
  own detector will not tell you it is wrong. **This is the one most likely to silently
  corrupt slice 2.**
- **Q13 — `value` is still `inf` at zero absorption**; only the interval and the display
  were fixed. Any second consumer reading the documented return shape sees
  `value > 1.0 → True` and concludes "confirmed overshoot."
- **Q11 — the fold destroys `basis`**, replacing every input's grounding with
  `"fold of N terms"`. Aggregate stocks over a corpus and the resulting basis names no
  observation, instrument, or window.
- **Q10 — multiplier-based widening cannot produce width from a zero base.** The general
  mechanism behind the bug slice 1 hit.

Q1-Q9 and Q14 remain open by design; Q6 in particular (should a ratio's floor be `0`
rather than `-inf`?) is a real question that slice 1 deliberately declined to decide inside
an implementation task. Keep that discipline: **a sealed open question is not closed by an
implementer who finds it inconvenient.**

---

## 4. Landmines — carried forward from slice 1's run

These cost real time. They are recorded nowhere else in git.

- **`cargo-pool key` prints a multi-line advisory banner, not a bare path.**
  `export CARGO_TARGET_DIR="$(cargo-pool key)"` sets a multi-line value, cargo dies with
  *"path segment contains separator `:`"*, and a mangled directory appears inside the crate.
  It hit **three of three** agents plus the controller. Use the literal path:
  `/projects/.cargo-target-pool/family/<family>/elohim/dev`. Note the repo's own
  CLAUDE.md documents the broken form.
- **Never pipe a cargo gate.** `cargo test | tail` reports `tail`'s exit status. A run in
  slice 1 reported `EXIT=0` while cargo had failed. Redirect to a file; echo `EXIT=$?` on
  its own line.
- **`.claude/scripts/_lib/__tests__/` is not pytest-collectable.** Most files there are
  self-running scripts asserting at import time, so `python3 -m pytest <dir>` aborts during
  collection on two pre-existing unrelated failures. The documented whole-suite gate is
  therefore **not a gate**. Run per-file.
- **Concurrent sessions commit into this worktree constantly** — four unrelated commits
  landed inside slice 1's review ranges. Always commit with an explicit file list, never a
  directory path, and build any review diff from the *task commit's real parent*, not from
  a pre-dispatch HEAD.

Filing these three as backlog items is itself worth doing; they are repo-tooling defects,
not project trivia.

---

## 5. Deferred minors inherited (triaged, none blocking)

- `FoldError::Empty` has no test — the only error branch with zero coverage.
- `Confidence::basis` non-emptiness documented but unenforced (Q5).
- `genesis/research/.epr-meta`'s `kind: level` on a rule that computes no quantity (Q14).
- `--diff-filter=DR` counts an in-place rename as absorption; structurally reachable, not
  currently live (3 rename events, all genuine `held/` moves).
- The live shape test in `doc_dynamics_test.py` asserts shape only, by design.
- L2's status is "serializer-proven but **unwired**" — accurate, and it under-sells: the
  guard asserts at the CID level via `compute_cid`, not mere byte-inequality.

---

## 6. Definition of done

Per the repo covenant, the deliverable is the delta, not a summary.

- [ ] `measure-honesty-local` gains checks for whichever of Q10-Q13 you closed, with
      evidence.
- [ ] A one-line delta in `genesis/manifests/habits.yaml` recording what this slice proved,
      with evidence — a real reading, not an intention.
- [ ] **If the dev-system stock model warrants its own habit, draft it — do not mint it.**
      The register is capped at 12 and currently holds 10. Minting is an operator
      governance act; slice 1 drafted `measure-honesty-local` and let the operator decide,
      and that is the pattern.

**The honest failure mode to watch for in this sprint specifically:** every item here adds
instrumentation, and the survey's sharpest finding is that we are addicted to adding
instrumentation without removal conditions. If this sprint ends with more intervenors and
no exits, it will have moved the wrong lever with great precision. Item A exists to prevent
that, and it should land first.
