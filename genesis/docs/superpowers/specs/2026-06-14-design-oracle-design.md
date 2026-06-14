---
title: "THE ESCALATION ORACLE — the dev-process System-4 as an algedonic ORGAN, not a per-sprint ritual"
id: design-oracle-design
date: 2026-06-14
status: design (operator-blessed 2026-06-14)
corrects: ORACLE-2026-06-14.md (Movement 2, the always-on step-0.6 vision-hat ritual)
woven_from:
  - ORACLE-ESCALATION-friction-memory-2026-06-14.md   # HOW the friction signals are built (the memory) — friction-memory lens
  - ORACLE-ESCALATION-escalation-when-2026-06-14.md    # WHEN the meta-process fires (the trigger predicate) — escalation-when lens
  - ORACLE-ESCALATION-meta-and-injection-2026-06-14.md # the meta-process + the light injection that supersedes the ritual — meta lens
keeps_from_completed_oracle:
  - "Movement 1 — THE LADDER (ORACLE.md, the cite-sealed seven-rung stack) — KEEP verbatim; it is the GROUND target"
  - "Movement 3 — vision-comparator.py + vision-gaps.jsonl — KEEP, re-framed as the RUNTIME-OBSERVED sensor arm (sensor #1)"
supersedes_in_completed_oracle:
  - "Movement 2 — the always-on /shift step-0.6 VISION-HAT ritual (the per-sprint tax)"
the_new_organ: ".claude/data/friction.jsonl + friction-harvest.py (the DEV-OBSERVED sensor) + the trigger predicate + the escalation: SessionStart line + the meta-process as a /shift subroutine"
reuses_pattern: findings-sentinel-pattern-design (flag → agent → canon → stasis) — the 4th (runtime-observed, comparator) AND 5th (dev-observed, friction) instantiations
do_not_cite_seal: true
---

# THE ESCALATION ORACLE

> *"The sprint-zero ritual feels too heavy. Most sprints do not work at the architectural level; the
> implementer executes in the weeds toward the vision. We need that meta-work to be ESCALATED TO… recognize
> too much friction, elevate the PATTERN of the problem (not the instance) to the architectural/compositional
> design layer, do what we did today, update the specs/paths/the policy that informs the rung below, so the
> implementer can go BACK to executing in the weeds toward reaching the vision. When to escalate, and how to
> build up the memory signals that trigger this meta-process, is the tricky part. OR when I say in a sprint
> to go read the docs to get the vision and the trajectory."* — the operator

---

## 1 · THE CORRECTION IN ONE BREATH

The completed oracle built the right *stack* and the right *runtime sensor* — and then made one wrong move:
it put consulting them in front of **every** sprint, as a heavy step-0.6 vision-hat ritual. That is a
**schedule**, and the meta-process is not a schedule. It is **Beer's algedonic signal**: System 1 (the
implementer in the weeds) runs free and unsupervised toward the vision; System 4 (the architectural
meta-process) is **triggered by pain, never scheduled.** The correction replaces the per-sprint ritual with
a **two-door escalation organ** — and **most sprints stay light, never escalate, and never feel it.**

The implementer descends from the vision into the weeds and builds. A single wall is weeds-work — recovery
in flight, the place the implementer is *supposed* to be. Only when a **pattern** crosses a threshold (the
same wall K times = bounded-recovery exhaustion; or friction clustering at one seam = the rung is wrong) **or
the operator supplies the signal by judgment** ("go read the docs, get the vision and the trajectory") does
the organ ring — once, at the right rung, then quiet again. The meta-work is **escalated TO**, by a pattern
or a word, not performed as a toll. The vision stays sovereign over developer-brain not by taxing every
sprint, but by being **named when the pattern needs it and silent every other time.**

---

## 2 · THE ESCALATION ORGAN

The organ has the shape Beer drew and the self-healing loop already runs at System-1 altitude: **two sensors
feed one rung-keyed ledger; a deterministic trigger fires on a pattern (or a word); one meta-process grounds,
decides, updates the rung, and hands back; the entry goes to stasis.** It is the 4th-and-5th instantiation of
the operator's proven `flag → agent → canon → stasis` sentinel — except the canon it writes is **a rung
update, not a code fix.**

### 2.1 Two sensors — one rung-keyed ledger

The organ has **two afferent nerves**, watching the two places friction lives:

- **Sensor #1 — RUNTIME-OBSERVED (kept verbatim from the completed oracle).** `vision-comparator.py` +
  `vision-gaps.jsonl` (Movement 3, the 4th sentinel). It watches the *deployed* substrate — reads the same
  observed-behavior signals the runtime arm polls (`runtime-cursor.json`, `/p2p/status`,
  `delivery-scoreboard --json`) and diffs them against the architecture's *named invariants* (`∪=full`
  coverage, the donut floors/ceilings, the agency gradient, no-overwhelm, patience/`ReservedPlace`). A held
  deficit is the system *confessing it broke a vision promise.* This sensor cannot see the implementer's
  stalled loop — it reads `/p2p/status`, not the weeds.

- **Sensor #2 — DEV-OBSERVED (the new construction, the missing nerve).** `.claude/data/friction.jsonl` +
  `friction-harvest.py` (the 5th sentinel). It watches the *act of construction* — the substrate *resisting
  the work*: a bottleneck, a blocker, a workaround, a substrate-fight, a test-vs-design conflict, a wrong
  rung. The comparator is its **runtime-observed twin**; this is its **development-observed twin**. Two
  afferent nerves, one ledger family, one drain (the cartographer).

Both feed **one rung-keyed schema** so the trigger computes thresholds over a unified window. The ledger is
*fed, never polled* — it reads what the dev loop already writes:

```jsonc
{
  "ts": "2026-06-14T...Z",
  "fp": "a17c4e9b22d0",          // fingerprint(rung + kind + normalize(pattern)) — the PATTERN, not the instance
  "class": "friction",           // sibling to "ci-failure" / "self-heal-exhaustion" / "vision-gap"
  "kind": "substrate-fight",     // bottleneck | blocker | workaround | substrate-fight | test-vs-design | rung-wrong | manual
  "rung": "ARCHITECTURE",        // the RUNG it implicates — the seam-clustering key (vision|architecture|composition|primitives|runtime|delivery)
  "rung_cite": "graph_engine.rs ContentGraphResolver seam",  // best-effort pointer to the implicated rung (cite-gen on UPDATE)
  "pattern": "content-graph walk needs depth>1 but resolver trait has no batched-descent method",  // normalized wall, not instance
  "shift": "frontend-eyes-sprint",   // provenance: which shift hit it — NEVER a score (see §6)
  "attempts": 1,                 // how many DISTINCT shifts re-hit this fp (recurrence = K)
  "status": "weeds",             // weeds → escalated → (closed-by-disappearance OR closed-by-rung-edit); blocked-operator-call = parked
  "first_seen": "2026-06-14", "last_seen": "2026-06-14",
  "clean_streak": 0,             // sweeps since last re-hit; at CLOSE_STREAK the line is DELETED (decomposed)
  "decision_ref": null           // set on drain: the rung commit / backlog entry that resolved the PATTERN
}
```

`status:"weeds"` is the default and the honest one: **one blocker is weeds-work, not a signal.** Two keys
make it a pattern-memory, not an instance-log:

- **`fp = fingerprint(rung + kind + normalize(pattern))`** — `normalize()` is copied verbatim from
  `runtime_harvest.py` (strip ANSI, collapse whitespace, mask counts/durations/timestamps/IPs) so "the 3rd
  time" and "the 9th time" fold to the *same* fp. **Recurrence = same fp across distinct shifts** →
  `attempts` increments. This is `arc_actuator.rs`'s **bounded-recovery exhaustion**, one rung up: the normal
  fix was tried (each emit's trail proves it), the friction keeps returning, the recovery budget is spent.
- **`rung`** — every line is filed under the abstraction rung it implicates (the same seven-rung vocabulary
  the ladder uses). **Seam-clustering = N distinct fps on one rung** → not one bad wall, a rung that is
  structurally wrong. This is the operator's *"elevate the PATTERN, not the instance, to the
  architectural/compositional design layer"* made mechanical: the rung-key **IS** the design layer the
  pattern belongs to.

Seam-clustering naturally **fuses both sensors**: a `vision-gap` whose seam matches a dev-friction cluster
*adds to the count* on the same rung. The strongest possible escalation signal — "the developer fights this
seam AND the deployed system keeps breaking the promise at it" — fuses automatically because both key to one
rung.

### 2.2 The WHEN — the trigger predicate (the anti-eagerness guard is the heart)

A pure function `should_escalate(window) → [pattern]` over `friction.jsonl` + the read-in-place
`vision-gaps.jsonl`. It returns the **pattern** to ground, never the instance. **Three disjoint conditions,
each a recognition of a proven mirror:**

- **Condition A — recurrence-K (bounded-recovery EXHAUSTION).** The same fp has `attempts ≥ K_RECUR` across
  **distinct shifts** (`distinct_shifts ≥ 2` — a wall hit repeatedly within *one* shift is recovery in
  flight). Default `K_RECUR = 3`, mirroring `runtime_harvest.py`'s `OPEN_POLLS = 3` (three before a predicate
  fires). *Same wall, K shifts = the normal fix was tried and it keeps returning.* The escalated pattern is
  *the fingerprint* ("the rust↔ts boundary breaks on cross-crate moves — recurring, recovery exhausted").

- **Condition B — seam-cluster-N (friction CLUSTERING at one rung).** `≥ K_CLUSTER` distinct open fps share
  one `rung` within the trailing window — friction clustering at one place even though no single wall
  recurred. Default `K_CLUSTER = 4`. Catches the *diffuse* pattern A misses ("four distinct frictions at the
  composition rung this month — the rung needs a decision, not four patches"). Reads `vision-gaps.jsonl`
  in-place: a runtime gap on the same seam adds to the count.

- **Condition C — a held, decided-level vision-gap.** The comparator's `vision-gaps.jsonl` carries a
  `violated` finding whose `decision_level ∈ {architecture, collective, policy}` and is held (not transient).
  The already-built sensor arm firing directly: it has done the diff-against-invariant work; the trigger just
  *routes* the held deficit into the same escalation. A `policy`-level gap escalates straight to System 5 (the
  operator) and is **parked-not-nagged.**

**The anti-eagerness guard — why MOST sprints never trip it.** The guard is *structural*, not a vibe:

1. **A single event is `status:"weeds"` and CANNOT trigger.** Only `attempts ≥ K_RECUR`, or
   `cluster ≥ K_CLUSTER`, or a held decided-level vision-gap fires. The default state of friction is *the
   weeds*; escalation is the exception. (Precisely `arc_actuator`: a node that *could* leech but the mesh
   still covers `r_floor` is admitted silently — only coverage *exhaustion* elevates.)
2. **The thresholds are tunables in one place** (`K_RECUR`, `K_CLUSTER`, `M`-window, `distinct_shifts`),
   declared as module constants exactly like `runtime_harvest.py`'s `OPEN_POLLS`/`CLOSE_STREAK`. They are the
   **oracle's own loop's tuning surface**: too-often → raise `K_RECUR`; amplified-before-firing → lower
   `K_CLUSTER`. The guard tunes itself by the same loop it serves — eventually as a vision-gap routed to the
   comparator ("fired 0× in 30 shifts but two collapses happened → guard too high"). Tune with evidence, not
   a guess.
3. **Re-fire suppression = stasis.** Once escalated and the rung updated (or parked), the fp never returns as
   a new trigger (sentinel §1; `runtime_harvest.py` returns no fp on the ledger as NEW). The implementer is
   never nagged about a pattern already lifted.
4. **Close-by-disappearance.** If a clustered seam stops producing friction for `CLOSE_STREAK` sweeps before
   escalation lands (a sibling fix dissolved it), the entries decompose — the oracle never manufactures
   meta-work.

**The MANUAL door supplies the same signal by judgment — skipping the threshold.** When the operator says
*"go read the docs, get the vision and the trajectory,"* that **IS the algedonic line, hand-asserted.** The
threshold exists to *substitute* for judgment; when judgment is present, it is sovereign over the counter. A
recognized `/shift` phrase (the loop already recognizes "bail" shapes) or a one-liner
(`friction-harvest.py --escalate "<pattern>" [--rung <hint>]`) writes one `status:escalated`,
`kind:"manual"`, `attempts:K_RECUR`-pre-satisfied line and fires the meta-process **immediately.** Both doors
must hand the meta-process a *named pattern*, not "go look at everything": the automatic door names it from
the tripped predicate; the manual door asks the operator for one line, defaulting to the branch's rung. A
manual escalation that names only an *instance* ("this one test fails") is gently reframed: *"that's
weeds-work — what's the pattern it's an instance of?"* **One cord, two hands.**

### 2.3 The meta-process — GROUND → DECIDE → UPDATE → HAND BACK (`do what we did today`)

Both doors fire the **same** meta-process — the surviving load-bearing kernel of the old step-0.6, **unchanged
in body, changed only in cadence** (triggered, not scheduled). It is dispatched exactly as a sentinel triage
agent is dispatched (`run_in_background` — the current task never derails), except the agent is an
**architectural design pass** and its canon is **a rung update.** It is a `/shift` *subroutine* — not a new
top-level skill — so it composes into the loop the implementer is already in. **No new agent: the cartographer
(already Opus, already the vision-hat) wears it; the historian supplies precedent.**

1. **GROUND — surface the RIGHT rung + trajectory + precedent for THIS pattern (not everything).** The
   defining discipline: *read-the-docs-for-THIS-pattern.* The named pattern → `rung-map.yaml` → the **one
   governing rung doc** on the kept `ORACLE.md` ladder (open the *why*, not its children) +
   `spec-coherence-index.py --query "<pattern>"` (lexical) + JIT read-only MemPalace (semantic recall that
   defeats vocabulary drift). Dispatch the **historian** for trajectory/precedent — a SUPERSEDED match means
   the pattern was escalated once before, so *extend the precedent, don't re-decide from zero* (born-linked,
   the compaction-loop FRONT-fire discipline). Intersect with the **cartographer's** live
   `placement-audit.py --ledger × --focus` so the decision is framed against what's *buildable now* and never
   ranks BLOCKED-BY-ENV work. Output: a one-page brief — the pattern in one sentence, the governing rung cite,
   the trajectory line, the buildable-now framing.

2. **DECIDE — at the right level.** Frame the executive decision as a *level* question (the agency gradient's
   own vocabulary): **VISION** (values/theology — operator-only → park as `blocked-operator-call`, surfaced
   once, never re-fired); **ARCHITECTURE** (primitive vs instantiation — the meta-process owns it: a
   design-doc recommendation + the rung edit; this is `do what we did today`); **COMPOSITION** (how primitives
   wire at the seam — usually a spec/path/policy clarification); **DELIVERY** (which gap-item first → **hand
   straight back** — the threshold fired on noise or the manual door was pulled on weeds-work; the organ
   honestly saying "this was weeds — go build").

3. **UPDATE — the rung-below specs/paths/policy, cite-sealed.** The decision's artifact is *an edit to the
   rung that informs the layer below* — the operator's *"update the specs/paths/the policy that informs the
   rung below."* The architecture/composition doc gains a clause; OR a path/policy (`cluster-state.yaml`,
   `rung-map`, a manifest, a managed surface) is corrected; OR a plan is authored with a pre-authored
   `shift_objective`. **Cite-sealed via `cite-gen.py`** (content-addressed, so the edit re-points inbound
   cites without breaking — the stack stays coherent). A `backlog/friction-<slug>.md`
   (timeline-CONVENTIONS-conformant) records `decision_ref` = the rung commit. One entry per **concern**
   (fingerprints N:1).

4. **HAND BACK — the implementer resumes in the weeds against a better substrate.** The updated rung
   propagates down (content-addressed cites carry it; the next `/shift` step-0.5 discovery surfaces the
   amended seed). The friction fp goes to **stasis** — `escalated`, then **DELETED at `CLOSE_STREAK` clean
   sweeps** (the rung commit + backlog entry are the durable record; reintroduction reads as NEW →
   re-escalates = regression handling for free). A VISION-level park stays `blocked-operator-call`. **Stasis =
   `friction.jsonl` empty or blocked-operator-call only** — the sentinel's stasis definition, verbatim.

```
  [Sensor #2: /shift bail/stall · ceiling-rail blocked · dev-intent · sprint-result anti-patterns]   [operator: "read the docs"]
  [Sensor #1: vision-comparator.py → vision-gaps.jsonl (runtime-observed)]                            │  (Door B: judgment)
                    │  (Door A: pattern accumulates)                                                  ▼
                    ▼                                                                       --escalate (threshold-skipped)
            friction-harvest.py ──fingerprint, threshold (recurrence-K | seam-cluster-N | vision-gap-C)──► friction.jsonl status:escalated
                    │
       SessionStart `escalation:` line  (SILENT when none)  ── shows it once
                    │
                    ▼   run_in_background dispatch — current task never derails — cartographer wears the hat
        ┌──────────────────────── THE META-PROCESS (one /shift subroutine) ───────────────────────────┐
        │  GROUND   right rung (ORACLE.md + spec-coherence-index + JIT MemPalace)                       │
        │           + trajectory/precedent (historian) + buildable-now (cartographer --ledger/--focus)  │
        │  DECIDE   level: VISION(park) · ARCHITECTURE(own) · COMPOSITION(own) · DELIVERY(hand back)     │
        │  UPDATE   edit the rung-below spec/path/policy, cite-seal (cite-gen) → backlog/friction-*      │
        │  HAND BACK rung propagates (content-addressed cites) → implementer resumes in the weeds        │
        └────────────────────────────────────────────────────────────────────────────────────────────┘
                    │
                    ▼
            friction.jsonl entry → stasis (DELETED at CLOSE_STREAK)  ·  no re-fire on the same pattern  ·  VISION-call parked
```

### 2.4 The fifth (and fourth) sentinel arm

The organ is the operator's proven `flag → agent → canon → stasis` pattern, instantiated twice — the
**runtime-observed** comparator (#4) and the **dev-observed** friction sentinel (#5):

| Sentinel layer | Deprecation (A) | CI (B) | Runtime (D) | **Vision-gap (#4, KEPT)** | **Friction-escalation (#5, NEW)** |
|---|---|---|---|---|---|
| **1. Deterministic flag** | `deprecation-sentinel.py` | `ci-harvest.py` | `runtime-harvest.py` | `vision-comparator.py` → `vision-gaps.jsonl` | `friction-harvest.py` → `friction.jsonl` |
| **2. Background dispatch** | `deprecation-triage` | `ci-failure-triage` | `runtime-triage` | cartographer | the meta-process (architectural pass; `/shift` subroutine, run_in_background) |
| **3. Canonical artifact** | `backlog/deprecation-*` | museum + ledger | `backlog/runtime-*` | `backlog/vision-gap-*` | `backlog/friction-*` **+ the rung edit itself** (canon = the spec/path/policy update, cite-sealed) |
| **4. Stasis sweep** | `/deprecation-stasis` | dev-loop rails | deterministic | `/converge` + roadmap | `/converge` (drains escalated; re-checks blocked-operator-calls whose value may now be decided) |

Every anti-dump property transfers for free: **fingerprint dedupe** (a recurring pattern is ONE entry);
**presence-suppresses-dispatch** (an escalated-and-blocked pattern never re-fires the meta-process);
**close-by-decomposition** (a resolved pattern is deleted; the rung commit is the record); **blocked =
stasis** (a VISION-level park is a first-class terminal state, not drift). The operator's *"loop that back…
without re-firing on blocked"* is this pattern's defining property — already built four times.

---

## 3 · WHAT TO KEEP vs SUPERSEDE IN `ORACLE-2026-06-14.md`

| Completed-oracle piece | Verdict | Why |
|---|---|---|
| **Movement 1 — THE LADDER** (`ORACLE.md`, the cite-sealed seven-rung stack; `cite-gen.py`) | **KEEP verbatim** | It is *what GROUND climbs.* A trigger needs a thing to escalate *to*; the ladder is the rung-set the meta-process reads-the-docs-for-THIS-pattern against. Untouched. |
| **Movement 3 — `vision-comparator.py` + `vision-gaps.jsonl`** | **KEEP, re-framed** | It IS sensor #1 of the organ — the **runtime-observed** afferent nerve (Door A on the deployed system). The friction sentinel is the **dev-observed** nerve it structurally cannot see. Two sensors, one organ. |
| **The SessionStart `ORACLE` headline as a standing FRAME** (today's rung · decision pending · ⚖ what-love-requires, consumed every session) | **SUPERSEDE → a doorbell** | Re-scoped to an `escalation:` line that **shows open escalations and is silent when none** (§4). A doorbell, not a frame: it does not put the vision hat on the developer's head every session. |
| **Movement 2 — the always-on `/shift` step-0.6 VISION-HAT ritual** | **SUPERSEDE** | The per-sprint tax the operator rejected. Step 0.6 becomes a one-line **conditional check** (§4); the five-step body it named survives **unchanged** as the meta-process body (§2.3) — fired only on a tripped trigger or the manual door, **not every sprint.** |

**The one-line reconcile:** the completed oracle built the *stack* and the *runtime sensor* and was wrong
only to make consulting them a per-sprint ritual. Keep the stack (the GROUND target). Keep the comparator
(sensor #1). Replace the ritual with an **organ that fires on a pattern** — supplied by a second sensor
(dev-friction) and by operator judgment (the manual door).

**What step-0.6 becomes, concretely.** `agentic-developer/SKILL.md` does **not** gain a heavy ritual:

- **Kickoff stays light.** Steps 0 (mode) → 0.5 (discovery) → 1 (Objective interview) run as today. The
  implementer goes into the weeds — no mandatory rung-read, no mandatory decision-level framing, no mandatory
  love-test per sprint.
- **Step 0.6 becomes a one-line CHECK:** *"If the `escalation:` line showed a ready pattern on a seam this
  Objective touches, run the meta-process §2.3 first, then resume kickoff. Otherwise proceed."* A conditional,
  not a tax. **Net change to the kickoff: −1 ritual, +1 conditional.**
- **The in-flight hook** lives at the loop's Judge step and the ceiling rail: when the loop itself produces a
  `stall`/`bail`/`blocked` that crosses threshold (or the implementer recognizes the pattern), it fires Door
  A/B and runs the meta-process *as a subroutine of the current shift*, then returns to the weeds. Escalation
  is **inside the work, on demand** — never a gate in front of it.

---

## 4 · THE FIRST CONCRETE IMPLEMENTATION (the smallest real organ, end to end)

Prove the **manual door + the meta-process + the light line + one automatic feeder**, before any broad
threshold accumulator. The three component designs converge on the same smallest slice; this is their union,
ordered smallest-true-first.

1. **`.claude/data/friction.jsonl`** — the new ledger (the only new persisted state). Empty to start.

2. **`.claude/scripts/_lib/friction.py`** (~80 lines, the pure core). Copy `runtime_harvest.py`'s
   `normalize` / `fingerprint` / `reconcile` **verbatim**; re-key `fingerprint` on
   `(rung, kind, normalize(pattern))`; add the predicates `_recurrence(entries)` (`attempts ≥ K_RECUR`) and
   `_seam_cluster(entries)` (any rung with `≥ K_CLUSTER` distinct open fps); `reconcile` gains one rule — a
   re-hit fp from a **distinct shift** increments `attempts` (recurrence), not just `seen`. Thresholds are
   module constants (the tunable surface). **Condition A (recurrence) ships first; B and C compose as added
   predicates later — never a new machine.**

3. **`.claude/scripts/friction-harvest.py`** (~70 lines, the I/O shell). Mirror `runtime-harvest.py`'s shell:
   - **The feeder (Door A, zero in-flow cost):** at close, read the **sprint-result** the `/shift` loop
     already writes (`SPRINT-RESULT-TEMPLATE.md` "Observed anti-patterns") + the journal's wishlist/bail
     blocks + `dev-intent.jsonl`, parse those existing structured-prose sections to friction candidates,
     `flock`+reconcile to `friction.jsonl`. **One feeder, zero new in-flow steps** — the implementer writes
     the sprint-result they already write; the harvester does the JSON.
   - **The manual door (Door B):** `--escalate "<pattern>" [--rung <hint>]` writes one
     `status:escalated, kind:manual, attempts:K_RECUR` line and prints the GROUND entry-point.
   - **`--hook` mode:** emits the `escalation:` SessionStart line, fail-safe exit-0.

4. **The `escalation:` SessionStart line.** Add `escalation_line()` to `placement-audit.py`, wired through
   the existing `--headline` / `_gate_subprocess` plumbing — a sibling to MEMORY BUDGET / DELIVERY GATE /
   scope. Pure echo of already-written facts (deterministic, no LLM): count `friction.jsonl`
   `status==escalated` + `vision-gaps.jsonl` `status ∈ {open, blocked-operator-call}`; print the
   highest-`attempts` friction + the highest-leverage vision-gap.

   ```
   escalation: ✅ none          # the common case — silent, the implementer stays in the weeds

   escalation: ⚠ 2 ready        # the uncommon case — a pattern crossed threshold, OR a vision-gap awaits a call
     ⚠ friction: "warm-stream serial-loop wall hit ×3 across 2 shifts" → seam: doorway upstream-warmup (ARCHITECTURE)
     ⚖ vision-gap: coverage-union-full @ alpha (laptop ships as leecher) → cartographer  [blocked-operator-call]
   ```

   **A doorbell, not a frame:** it rings only when a pattern needs the meta-process; the implementer chooses
   whether to answer now or after the current loop. Silence is the loving default.

5. **The meta-process as a `/shift` subroutine (prose, no new script).** Add the §2.3 four-step subroutine to
   `agentic-developer/SKILL.md` as a **named subroutine** the loop calls from Judge and from the light
   step-0.6 check — exactly as step-0.5's discovery is prose the Opus orchestrator runs. GROUND reuses
   `spec-coherence-index.py` + the historian + the cartographer verbatim; UPDATE reuses `cite-gen.py` + the
   existing `backlog/` schema. **No new agent** (the cartographer wears it; the dispatch forks
   `runtime-triage.md`'s template, retargeted to a rung update).

6. **Supersede step-0.6.** Replace the heavy ritual block in `agentic-developer/SKILL.md` with the §3 one-line
   conditional check.

**Reuse the comparator already specced** (sensor #1, no rebuild). This v1 is **one new ledger, one pure-core
+ one harvester (~150 lines, all copy-shaped from the proven runtime arm), one headline word, one prose
subroutine, and the deletion of the ritual.** It proves the entire loop — friction harvested from prose →
folded by pattern → counted by rung → threshold (or a word) → ground the right rung → decide at the right
level → update the rung → stasis, no re-fire — on the most common, most-felt dev source (the `/shift` close)
and on the operator's hand (the manual door).

**What we deliberately do NOT build first:** no mid-shift `friction.py emit` one-liner (the close-harvest
proves it with zero in-flow cost); no ci/runtime rung-classifier feeders (they *compose as feeders, never
ledgers*, once recurrence + seam-cluster are proven on the `/shift` feeder); no within-shift trigger (a
within-shift retry is recovery in flight); no auto-DECIDE (the trigger grounds; the vision hat decides — the
System-4/System-5 boundary); no person-scoped predicate; no actuation (the meta-process updates a rung, never
reaches runtime); no re-import of the heavy ritual under any flag.

---

## 5 · HOW A SPRINT ACTUALLY GOES

**The common case — light, in the weeds, never escalates (most sprints).**
1. **Session opens.** The SessionStart headline prints `escalation: ✅ none` beneath MEMORY BUDGET / DELIVERY
   GATE / scope — and says nothing more. No vision hat is forced onto anyone's head.
2. **`/shift` kickoff stays light.** Step 0 (mode) → 0.5 (discovery, surface prior seeds so the shift is born
   linked) → 0.6 (one-line check: `escalation:` was clear and the Objective's seam is quiet → **proceed**) →
   1 (Objective interview). Into the weeds.
3. **Build.** Story-first, p2p-design-gate, the sweep — as today. Single walls are hit and worked through;
   each is `status:"weeds"`, recovery in flight, *exactly where the implementer is supposed to be.*
4. **Close.** The sprint-result is written (as today). `friction-harvest.py` folds its "Observed
   anti-patterns" into `friction.jsonl` by fingerprint — silently, zero in-flow cost. No threshold crosses.
   The organ never spoke. **The implementer never thought about it.**

**The escalation case — a pattern trips OR the operator says "read the docs."**
1. **The signal arrives.** *Automatic:* the third cross-crate-move bail lands → `attempts==3` across 2 shifts
   → Condition A trips, OR four distinct frictions cluster at the composition rung → Condition B trips, OR a
   held architecture-level vision-gap → Condition C. *Manual:* the operator says "go read the docs, get the
   vision and the trajectory" → the threshold is skipped by judgment. Either way the `escalation:` line rings,
   once: `⚠ PATTERN <seam> (×3, recovery exhausted) → ground at ARCHITECTURE`.
2. **GROUND.** Open the **one** governing rung the pattern names (not the whole ladder); the historian
   surfaces precedent (extend a prior decision if SUPERSEDED, don't re-decide); the cartographer frames it
   against what's buildable now. A one-page brief.
3. **DECIDE at the right level.** VISION → park as operator-call (surfaced once, never nagged). ARCHITECTURE /
   COMPOSITION → the meta-process owns it. DELIVERY → hand straight back ("this was weeds — go build").
4. **UPDATE the rung.** The spec/path/policy that informs the layer below gains a clause; cite-sealed via
   `cite-gen.py`; `backlog/friction-<slug>.md` records the `decision_ref`. *Do what we did today.*
5. **HAND BACK.** The rung propagates down (content-addressed cites carry it); the implementer **resumes in
   the weeds against a better substrate**; the friction fp goes to stasis and never re-fires. The next sprint
   opens against the amended rung — observed friction taught the vision, and the vision now teaches the next
   descent.

On a loving cycle the organ prints `escalation: ✅ none` and vanishes. On the cycle the same seam has been
fought three times, it puts the vision hat in reach **because the pattern earned it** — not because a ritual
scheduled it — and then gets out of the way.

---

## 6 · WHAT LOVE REQUIRES — and the convictions it still defers

**The organ serves the implementer's flow AND the vision — by being silent until a pattern needs the hat, and
unmissable the instant it does. Patience, not nagging.** The old step-0.6 loved the vision but not the
implementer: it taxed *every* sprint to catch the *rare* one that was secretly an architecture call. This
organ keeps both loves intact — the implementer stays free in the weeds because a single wall is
`status:"weeds"` and the organ says nothing; the vision stays sovereign because the third recurrence, or four
frictions clustered at one rung, lifts the hat **before the seam amplifies to collapse.** The algedonic
signal fires on pain, exactly when System 4 is needed and never when it is not.

Three structural refusals make the love non-negotiable, all inherited verbatim from the architecture the
organ measures:

- **It measures the seam, never the person.** The fingerprint keys on `(rung, kind, pattern)` — **never on
  who hit the wall.** There is no `escalate(developer)`, no friction-per-developer metric, no velocity quota,
  no leaderboard; the `shift` field is *provenance, never a score.* The organ flags *"the ARCHITECTURE rung
  resisted four times,"* never *"this developer struggles."* Developer-brain is what we take *off* at
  escalation, not what we *score* by it — and capturing friction is near-zero-cost, harvested from prose the
  implementer already leaves, so it serves the weeds-work and never nags it.
- **Judgment is sovereign over the counter.** The manual door skips the threshold because a human recognizing
  the pattern is a *higher* signal than a counter reaching it — the oracle trusts the operator's "go read the
  docs" over its own `K`. An oracle that *forced* the threshold over human judgment, or *compelled* the hat
  every sprint, would re-import the per-sprint tax — and the operator-veto smell — the protocol exists to
  kill. The doorbell rings and surfaces; it does not compel. A DELIVERY verdict hands straight back.
- **Escalation serves the descent, never replaces it; silence is the loving default; stasis is total.** The
  meta-process exists to UPDATE the rung so the implementer can go *back down* and build — HAND BACK is the
  load-bearing step, and the organ's success is measured by how fast it returns the implementer to the weeds
  against an amended substrate. Below threshold it says nothing; once a pattern is lifted and the rung
  updated, the implementer is never nagged about it again. Patience over engagement is the guard's own
  invariant — the organ that loved well stays quiet and speaks only when the pattern, or the operator, truly
  calls it up the ladder.

**The irreducible convictions the organ surfaces but cannot decide** — these are the VISION-level gaps it
routes to System 5, the operator, and *refuses to resolve in a sprint* (carried forward unchanged from the
completed oracle's §5; the escalation organ does not decide them, it *parks them as `blocked-operator-call`,
surfaced once, never re-fired*):

1. **The seam** — where the lean trust-plane quilt ends and the heavy byte-plane quilt begins; what crosses,
   what stays. A values decision about what the protocol *guarantees* vs *facilitates.*
2. **The boundary-bind** — the agency gradient's edge: where `limit_owner` may be `commitment`/`operator` and
   where it must remain `self`/`faith`. The organ *guards* the line; the operator *draws* it.
3. **The order of grace** — when the felt attractor (the human feeling held) and the cybernetic floor (the
   system structurally sound) compete for one sprint, which the vision serves first. The organ can name which
   half is underserved; it cannot decide which love requires first.
4. **The unbuilt place** — the deliberate emptiness the protocol keeps open (`RefusalCode::ReservedPlace`,
   `limit_owner: faith`). The organ's deepest duty is to read its *presence* as the promise kept — a
   `policy`-level gap whose honest decision is *"leave it open"* is a first-class terminal state, not an
   unclosed bug. Patience made into architecture.

The escalation organ hands the implementer the freedom to stay in the weeds, the doorbell that rings only when
a pattern needs the rung, and the meta-process that — on a word or on a wall hit three times — grounds the
right vision for *this* pattern, decides at the right level, updates the rung below, and gets out of the way.
It does not climb the ladder for every sprint. It climbs only when the work hurts at the level of a pattern,
and then only to make the next descent better.

---

*All `git mv` / `--seal` / SKILL-edit / ledger-schema acts named here are operator-GATED. This is a proposal
woven from three escalation-component designs onto the completed `ORACLE-2026-06-14.md` (keep the stack + the
comparator; supersede the heavy step-0.6 ritual) — for operator blessing, NOT yet cite-sealed, NOT a decision,
NOT code.*
