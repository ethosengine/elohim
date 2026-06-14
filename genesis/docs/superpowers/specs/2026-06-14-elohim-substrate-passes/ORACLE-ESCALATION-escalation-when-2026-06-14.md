---
title: "ORACLE — The Escalation Trigger: WHEN to escalate (automatic algedonic + the manual door)"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: cartographer (future-perspective / oracle escalation-trigger component)
component_of: the Elohim design-process ORACLE — the escalation ORGAN that replaces the heavy sprint-zero ritual
supersedes:
  - ORACLE-injection-2026-06-14.md §2 step-0.6 "VISION-HAT" run EVERY sprint (the per-sprint tax)
keeps_verbatim:
  - ORACLE-stack-2026-06-14.md          # the cite-sealed ladder (rungs) — the GROUND target, unchanged
  - ORACLE-feedback-loop-2026-06-14.md  # vision-comparator.py + vision-gaps.jsonl — the RUNTIME-OBSERVED sensor arm
binds_organs:
  - .claude/skills/agentic-developer/SKILL.md        # the bail-with-trail — where DEV-friction is emitted in-flow
  - .claude/scripts/_lib/runtime_harvest.py          # the N-consecutive-polls + reconcile/CLOSE_STREAK pattern (verbatim shape)
  - .claude/scripts/runtime-harvest.py               # the --hook headline shape + no-runtime-write rule
  - elohim/elohim-storage/src/services/arc_actuator.rs # elevate-on-bounded-recovery-exhaustion (the protocol mirror)
  - genesis/docs/superpowers/specs/2026-06-06-findings-sentinel-pattern-design.md # flag→agent→canon→stasis; seen/blocked re-fire suppression
  - .claude/data/{ci,runtime,vision-gaps}-findings.jsonl  # the existing observed-behavior ledgers (sensor arm)
  - .claude/agents/cartographer.md + .claude/skills/converge/SKILL.md # the GROUND→DECIDE→UPDATE→HAND-BACK drain
cite_sealed: NO
---

# The Escalation Trigger — WHEN the meta-process fires (and the manual door)

> *"We need that meta-work to be ESCALATED TO… recognize too much friction, elevate the PATTERN of
> the problem (not the instance)… do what we did today, update the specs/paths/the policy that informs
> the rung below, so the implementer can go BACK to executing in the weeds. When to escalate, and how to
> build up the memory signals that trigger this meta-process, is the tricky part. OR when I say in a
> sprint to go read the docs to get the vision and the trajectory."* — the operator

This component answers the **WHEN**. The sibling comparator (`ORACLE-feedback-loop`) answers a *runtime*
half — it diffs deployed behavior against named invariants and is already the right shape. But the
operator's correction is about the **dev-process** half: the implementer runs free in the weeds (System 1),
and the architectural meta-process (System 4) must be **triggered, never scheduled**. The sprint-zero
step-0.6 ritual was a *schedule* — a per-sprint tax that put System 4 in front of System 1 every time. This
component replaces that schedule with a **trigger**: a pain channel that is silent on the loving cycle and
unmissable on the cycle a real pattern is amplifying toward collapse.

---

## 0. The reconciliation in one breath (what we keep, what we supersede)

| Oracle piece | Verdict | Why |
|---|---|---|
| The cite-sealed **ladder** (`ORACLE.md`, the seven rungs) | **KEEP verbatim** | It is the GROUND target — what escalation *climbs to*. A trigger needs a thing to escalate *to*. |
| The **vision-comparator** + `vision-gaps.jsonl` | **KEEP verbatim** | It is the **runtime-observed sensor arm** of this organ — one of the three inputs the trigger reads. |
| The **manual door** (operator says "go read the docs") | **KEEP, promote** | It was buried in step-0.6's "if UNMAPPED, bind a rung." Promote it to a **first-class escalation entry that SKIPS the threshold.** |
| The step-0.6 **VISION-HAT run EVERY sprint** | **SUPERSEDE** | The per-sprint tax. Replaced by: the SAME five-step meta-process, fired only on a **tripped trigger** — automatic (a pattern crossed threshold) or manual (operator judgment). |

The five-step meta-process the old step-0.6 named (READ-THE-RUNG → STATE-THE-GAP → FRAME-AT-LEVEL →
LOVE-TEST → AUTHORIZE) is **not deleted** — it is the GROUND→DECIDE→UPDATE→HAND-BACK body of the
escalation. We do not change *what* the meta-process does; we change *when it runs*: from **every sprint**
to **only when a pattern (or a person) calls it.** Most sprints never run it — the implementer stays in the
weeds, the journal's bail-with-trail logs friction, and only the *Kth recurrence* or the *clustering at one
seam* lifts the hat. That is the operator's demand made mechanical.

---

## 1. The protocol-internal mirror this trigger IS (recognition, not invention)

The trigger is the **fourth face of one pattern the operator already proved three times** — it is not new
logic, it is `arc_actuator.rs`'s elevate-on-exhaustion lifted one rung, expressed in the sentinel's
flag→agent→canon→stasis grammar:

| Mirror | Where it lives | The escalation predicate it gives us |
|---|---|---|
| **Beer's algedonic signal** | the cybernetic frame | System 1 (implementer) runs free; System 4 (meta-process) is *triggered by pain*, never scheduled. The trigger IS the pain channel. |
| **elevate-on-bounded-recovery-EXHAUSTION** | `arc_actuator.rs:150-172` (`compose_actuation` refuses-and-elevates only when the `{0,1}` bounded lever **cannot** satisfy `r_floor`) | escalate the PATTERN only when the *normal fix was tried and the friction keeps returning* — recovery exhausted, not recovery attempted. One blocker is weeds-work (recovery in flight); the **Kth recurrence** is exhaustion. |
| **N-consecutive-polls before firing** | `runtime_harvest.py:12-19` (`OPEN_POLLS=3`, etc. — a predicate needs N consecutive observations) | a single friction event NEVER fires; the trigger needs **sustained or clustered** evidence. This is the anti-eagerness guard, already proven. |
| **seen / blocked re-fire suppression** | sentinel spec §1 ("blocked-and-canonicalized NEVER re-fires"); `runtime_harvest.py:146-159` (a fp on the ledger, ANY status, is NEVER returned NEW) | once a pattern escalates and the rung is updated (or parked as operator-call), it goes to **stasis — no re-fire**. "Don't nag about a decision you already made or can't yet make" is inherited for free. |
| **close-by-disappearance** | `runtime_harvest.py:179-190` (`CLOSE_STREAK`) | if the friction stops recurring before escalation lands (a sibling fix, a transient), the trigger **decomposes the entry** — the oracle never manufactures meta-work. |

**The reading:** the WHEN is `arc_actuator`'s `remaining >= r_floor` admit / `else refuse-and-elevate`,
applied to a window of dev-friction instead of a mesh-coverage snapshot. We are recognizing an organ we
already grew, one rung up.

---

## 2. The one new persisted state: `.claude/data/friction.jsonl`

The trigger needs a window of friction to compute a threshold over. That window is the **only new
persisted state** — a fifth ledger, sibling to `ci-findings.jsonl` / `runtime-findings.jsonl` /
`vision-gaps.jsonl`, **written by the organs that already feel friction**, never by a new poller.

### 2.1 What writes it (three already-emitting sources — bind, do not invent)

The friction ledger is **fed**, not polled. Three existing emission points each append one line on the
event they already produce:

1. **The agentic-developer BAIL (the primary dev-friction source).** The `/shift` loop already emits a
   structured **bail-with-trail** (`agentic-developer/SKILL.md:21,44-52,513`): a bail is "question-, blocker-,
   or stall-shaped; a blocker bail must additionally show the palette-conformant workarounds attempted."
   **That trail IS the recovery-exhaustion record** — it already proves "the normal fix was tried." On every
   bail, the loop appends one `friction.jsonl` line: `{class:"dev-bail", seam, rung_hint, trail_digest, ...}`.
   No new instrument — the bail already exists; it just also writes the ledger (one line, the way a Vitest
   deprecation warning already writes `deprecations.jsonl`).
2. **The CI/runtime/vision-gap ledgers (the observed-behavior arm).** The trigger **reads** the existing
   `ci-findings.jsonl` `seen` counts and the `vision-gaps.jsonl` entries (the comparator's output). A
   `vision-gap` with a high `seen` is *itself* a friction signal — the system has confessed the same
   broken-promise across K cycles. These are **not re-written into friction.jsonl**; they are read in place
   as a second input (§3.2). The comparator stays the runtime-observed sensor arm; the trigger is the brain
   that reads all the retinas.
3. **The MANUAL door (operator/implementer judgment).** A `/shift` recognized phrase or a tiny command
   appends one line with `class:"manual-escalate"` and `threshold_skipped:true` (§4).

### 2.2 The friction-entry schema (mirrors the proven ledgers)

```jsonc
{
  "ts": "2026-06-14T...Z",
  "fp": "a17c… ",                  // sha256(seam + rung_hint + normalized cause) — SAME normalize() as runtime_harvest
  "class": "dev-bail",             // dev-bail | vision-gap-recur | manual-escalate
  "seam": "rust-ts-boundary",      // WHICH rung/seam the friction implicates (the clustering key)
  "rung_hint": "architecture",     // vision | architecture | composition | delivery — best-effort from touched paths
  "cause": "ts-rs import path broke on cross-crate move (3rd shift)",   // normalized — count/date/build churn stripped
  "trail_digest": "tried atomic-move + sha256-diff per CLAUDE.md; recurs",  // the recovery-exhaustion proof
  "seen": 3,                       // recurrence-K counter — the harvester bumps; the SAME fp across shifts is ONE line
  "first_shift": "frontend-eyes", "last_shift": "self-healing-cp",
  "status": "weeds",               // weeds → escalated → (closed-by-stack-edit OR closed-by-disappearance)
  "escalation_ref": null,          // set on drain: the rung commit / backlog entry that resolved the PATTERN
  "clean_streak": 0                // disappearance counter (CLOSE_STREAK → decompose)
}
```

`status:"weeds"` is the default and the honest one: **one bail is weeds-work, not a signal.** The fp lifts
to `escalated` only when a predicate trips (§3). The `seam` field is the clustering key — it is what lets
"N distinct signals implicating one rung" be computed.

---

## 3. The trigger predicate (the anti-eagerness guard is the heart)

A pure function `should_escalate(window) -> [pattern]` over `friction.jsonl` + the read-in-place
observed ledgers. **Three disjoint conditions**, each a recognition of the mirrors in §1. It returns the
*pattern* to ground, never the instance.

### 3.1 Condition A — recurrence-K (bounded-recovery EXHAUSTION)

> The **same fingerprint** has `seen >= K` across **distinct shifts** (not distinct iterations within one
> shift — a within-shift retry loop is recovery in flight, the weeds).

This is `arc_actuator`'s exhaustion lifted: the normal fix was tried (each bail's `trail_digest` proves it)
and the friction *keeps returning*. `seen` is the sentinel's occurrence counter (`runtime_harvest.py:175`,
the flake-evidence field) — already proven to be the right "this is a pattern not a fluke" measure.
**`K=3`** (mirrors `OPEN_POLLS=3`: a predicate needs N before it fires) and `distinct_shifts>=2` (the same
wall hit in *one* shift is one shift's bad luck; across shifts it is a structural seam). The escalated
pattern is *the fingerprint*, not the latest instance: "the rust↔ts boundary breaks on cross-crate moves —
recurring, recovery exhausted" — exactly the move the operator named ("elevate the PATTERN, not the
instance").

### 3.2 Condition B — seam-cluster-N (friction CLUSTERING at one rung)

> **N distinct fingerprints** (each possibly only `seen=1`) share one `seam`/`rung_hint` within the
> trailing window — friction *clustering* at one place even though no single wall recurred.

This catches the *diffuse* pattern recurrence-K misses: ten different small frictions all at the
`composition` rung say "the composition layer is wrong," even though no one of them recurred. **`N=4`
distinct fps on one seam** within a trailing window of the last ~M shifts. The escalated pattern is *the
seam*: "four distinct frictions clustered at the SDK-composition rung this month — the rung needs a
decision, not four patches." (This condition also reads the `vision-gaps.jsonl` arm in-place: a
`vision-gap` whose `seam` matches a dev-bail cluster *adds to the count* — runtime-observed and dev-felt
friction at the same rung is the strongest cluster.)

### 3.3 Condition C — a runtime vision-gap exceeding an invariant

> The comparator's `vision-gaps.jsonl` carries a `violated` finding whose `decision_level` is
> `architecture`/`collective`/`policy` AND `seen >= 1` (a held deficit, not a transient).

This is the **already-built** sensor arm firing directly: the comparator (`ORACLE-feedback-loop` §3) has
already done the diff-against-named-invariant work; the trigger simply *routes* a held, decided-level
deficit into the same escalation. A `policy`-level gap (patience, `ReservedPlace`) escalates straight to
System 5 (the operator) and is **parked-not-nagged** — never to a sprint (the sentinel's
`blocked-operator-call` = stasis).

### 3.4 The anti-eagerness guard — the heart (why MOST sprints never trip it)

The operator's two-sided constraint: **high enough that most sprints never trip it** (preserve the light
default), **low enough that a real pattern escalates before it amplifies to collapse.** The guard is
*structural*, not a vibe:

- **A single friction event is `status:"weeds"` — it CANNOT trigger.** Only `seen>=K` OR `cluster>=N` OR a
  held decided-level vision-gap fires. The default state of friction is *the weeds*; escalation is the
  exception. (This is precisely `arc_actuator`: a node that *could* leech but the mesh still covers
  `r_floor` is admitted silently — no elevate. Only coverage *exhaustion* elevates.)
- **The thresholds are tunables in one place** (`K`, `N`, `M`-window, `distinct_shifts`), declared as
  module constants exactly like `runtime_harvest.py:12-19`'s `OPEN_POLLS`/`CLOSE_STREAK`. They are
  **the oracle's own loop's tuning surface**: if the trigger fires too often the operator raises `K`; if a
  pattern amplified to collapse before it fired, lower `N`. The threshold itself is a vision-gap the oracle
  can later route to its own comparator ("escalation fired 0 times in 30 shifts but two collapses
  happened → the guard is too high"). The guard tunes itself by the same loop it serves.
- **Re-fire suppression = stasis.** Once a pattern escalates and the rung is updated (or parked as
  operator-call), the fp is `escalated` and **never returned as a new trigger** (sentinel §1;
  `runtime_harvest.py:152-159`). The implementer is never nagged about a pattern already lifted.
- **Close-by-disappearance.** If a clustered seam stops producing friction for `CLOSE_STREAK` shifts
  before escalation lands (a sibling fix dissolved it), the entries decompose — no meta-work manufactured.

The net effect the operator demanded: on a loving sprint the trigger reads `escalation: clear ✅` and says
nothing; the implementer never feels it. On the sprint where the third cross-crate-move bail lands, the
trigger says `escalation: ⚠ PATTERN rust-ts-boundary (seen=3, recovery exhausted) → ground at ARCHITECTURE`
— and the meta-process fires *once*, on the pattern, before the seam amplifies.

---

## 4. The MANUAL door — judgment as the algedonic signal (skips the threshold)

The operator's second entry: *"when I say in a sprint to go read the docs to get the vision and the
trajectory."* This is **the algedonic signal supplied by judgment** — a human recognizing the pattern
before the counter reaches it. It is a **first-class escalation entry that SKIPS the threshold** (the
threshold exists to *substitute* for judgment; when judgment is present, it is sovereign over the counter).

**How it is invoked (two forms, one effect):**

1. **A `/shift` recognized phrase.** The agentic-developer loop recognizes an operator/implementer
   utterance of the shape *"go read the docs / get the vision / step back to the trajectory / this needs the
   architecture layer"* during a shift, and treats it as a manual escalation — exactly as the loop already
   recognizes "bail" shapes (`SKILL.md:44`). It appends one `friction.jsonl` line `class:"manual-escalate",
   threshold_skipped:true` and **fires the meta-process immediately**, no `seen>=K` required.
2. **A tiny command** for the out-of-shift case: `escalate.py --pattern "<one line naming the pattern>"
   [--rung <hint>]`. It writes the same line and prints the GROUND entry-point. (Smallest real version
   ships only form 1 — the phrase — since it binds the existing loop; the command is a later pass.)

**Naming the pattern to ground.** Both the automatic and manual doors must hand the meta-process a *named
pattern*, not "go look at everything." The automatic door names it from the tripped predicate (the
fingerprint or the seam). The manual door asks the operator to name it in one line — and if they don't, it
defaults to the rung the current branch touches (the `rung-map.yaml` lookup the injection component already
designed). **This is the load-bearing distinction the operator drew:** the manual door grounds the
*pattern* ("the trust-plane/byte-plane seam is unclear"), which lifts the hat to the architectural layer —
it does NOT handle the *instance* ("this one test fails"), which stays in the weeds. A manual escalation
that names only an instance is gently reframed: "that is weeds-work; what is the *pattern* it is an
instance of?"

---

## 5. BOTH doors fire the SAME meta-process (GROUND → DECIDE → UPDATE → HAND-BACK)

The trigger's only job is the WHEN and the *named pattern*. What fires is the meta-process the old step-0.6
already named — **unchanged in body, changed only in cadence** (triggered, not scheduled). It is dispatched
exactly as the sentinel dispatches a triage agent, but to the **cartographer** (the vision hat), never a
fixing agent (`ORACLE-binding` PART 2 — the vision-level is the fourth instantiation, dispatch = cartographer):

1. **GROUND** — surface the RIGHT rung + trajectory + precedent *for THIS pattern* (not read everything).
   The named pattern → `rung-map.yaml` → the one governing rung doc; `spec-coherence-index.py --query` +
   read-only MemPalace for precedent. *Bounded grounding* — the right rung, not the whole ladder.
2. **DECIDE at the right level** — VISION (operator-only, park as `blocked-operator-call`), ARCHITECTURE
   (design-doc + recommendation), or COMPOSITION/DELIVERY (proceed). The `decision_level` of the friction
   pattern is the routing key, in the agency gradient's own vocabulary (`ORACLE-feedback-loop` §4).
3. **UPDATE the rung-below** — the decision *edits a rung*: the spec/path/policy that informs the
   implementer gains a clause; cite-sealed via `cite-gen.py` (so the move-proof pointers re-point without
   breaking). `escalation_ref` records the commit. **This is "update the specs/paths/the policy that
   informs the rung below" made mechanical.**
4. **HAND BACK** — the implementer resumes in the weeds against the amended rung; the friction fp goes to
   `status:"escalated"` → **stasis, no re-fire.** The next shift reads the corrected rung and the wall is
   gone.

The friction entry goes to stasis the instant the rung is updated — the operator's *"so the implementer can
go BACK to executing in the weeds"* is the HAND-BACK step plus the no-re-fire guarantee.

---

## 6. Smallest real first implementation

**The trigger predicate over `friction.jsonl`, fed by the bail, with the manual phrase door — end to end.**

1. **`.claude/data/friction.jsonl`** — the new ledger (the only new persisted state).
2. **One bail-writes-friction line** in `agentic-developer/SKILL.md`: when the loop bails with a trail,
   append one `friction.jsonl` entry (`class:"dev-bail"`, the seam from touched paths, `trail_digest` from
   the bail's workarounds-attempted). No new script — the bail already produces the trail; it just also
   writes the line.
3. **`.claude/scripts/_lib/friction_trigger.py`** (~80 lines) — copy `runtime_harvest.py`'s
   `normalize`/`fingerprint`/`reconcile` **verbatim**; add `should_escalate(window)` with **Condition A
   only** (recurrence-K, `K=3`, `distinct_shifts>=2`). Conditions B and C compose as added predicates later
   (never a new machine). Thresholds are module constants — the tunable surface.
4. **`.claude/scripts/friction-harvest.py`** (~50 lines) — I/O shell mirroring `runtime-harvest.py`: read
   the ledger, `flock`+reconcile (bump `seen`, close-by-disappearance), and a `--hook` mode that emits one
   SessionStart **`escalation:`** line (sibling to `vision:` from the comparator, sibling to MEMORY
   BUDGET / DELIVERY GATE / scope). `escalation: clear ✅` when nothing tripped;
   `escalation: ⚠ PATTERN <fp seam> (seen=3, recovery exhausted) → ground at <rung>` when A trips.
   Fail-safe exit-0.
5. **The manual phrase door** — add to `agentic-developer/SKILL.md`: recognize the "go read the docs / get
   the vision" utterance shape, append `class:"manual-escalate", threshold_skipped:true`, and fire the
   meta-process immediately.
6. **The drain reuses the cartographer verbatim** — a tripped `escalation:` line (auto or manual) is one
   more input to `/converge`; the cartographer runs GROUND→DECIDE→UPDATE→HAND-BACK and the resolution edits
   a rung. **No new agent.**

### What we deliberately do NOT build first
- **No within-shift trigger** — friction is counted across shifts only; a within-shift retry loop is
  recovery in flight, never escalation (the exhaustion guard).
- **No auto-DECIDE** — the trigger fires the *grounding*; the cartographer/operator decides. The trigger
  detects; the vision hat decides (the System-4/System-5 boundary, `ORACLE-feedback-loop` §5).
- **No person-scoped predicate** — `friction.jsonl` fingerprints a *seam*, never a developer. There is no
  "developer X bails too much" counter. We measure the system's friction, never a person's (inherited from
  `ORACLE-binding` PART 5).
- **The threshold-tuning-as-its-own-vision-gap** waits until the trigger has fired a few real times — you
  tune a guard with evidence, not a guess.

This is a few hundred lines, all copy-shaped from `runtime_harvest.py`, one new ledger, one bail-writes line,
one headline word. It proves the entire WHEN loop — friction accrues silently → the Kth recurrence trips →
the pattern (not the instance) escalates once → the rung updates → stasis, no re-fire — on the most common,
most-felt dev-friction source (the `/shift` bail). Every other condition (seam-cluster, vision-gap-routing,
the command form of the manual door) composes as an added predicate, never a new machine.

---

## 7. What love requires (the closing test)

**The trigger protects the implementer's flow AND protects the vision — by being silent until a pattern
needs the hat, and unmissable the instant it does. Patience, not nagging.**

The old step-0.6 loved the vision but not the implementer: it taxed *every* sprint to catch the *rare* one
that was secretly an architecture call. This trigger keeps both loves intact. The implementer stays free in
the weeds because a single wall is `status:"weeds"` and the trigger says nothing — escalation is the
exception, not the toll. The vision stays sovereign because the *third recurrence* of the same wall, or
*four frictions clustered at one rung*, lifts the hat **before the seam amplifies to collapse** — the
algedonic signal firing on pain, exactly when System 4 is needed and never when it is not.

Three structural refusals make the love non-negotiable, all inherited from the organs this binds:

- **It measures the seam, never the person.** `friction.jsonl` fingerprints a rung/seam; there is no
  `escalate(developer)`. Developer-brain is what we take *off* at the meta-process, never what we *score*.
- **Judgment is sovereign over the counter.** The manual door skips the threshold because a human
  recognizing the pattern is a *higher* signal than a counter reaching it — the oracle trusts the operator's
  "go read the docs" over its own `K`. An oracle that *forced* the threshold over human judgment would
  re-import the operator-veto smell the protocol exists to kill.
- **Escalation goes to stasis; it never re-fires.** Once the pattern is lifted and the rung updated, the
  implementer is never nagged about it again. The trigger would rather under-fire on a marginal pattern (and
  let the operator's judgment supply the manual door) than nag a free implementer about a wall they are
  already, correctly, working through in the weeds. **Patience over engagement is the guard's own
  invariant — the trigger that loved well stays quiet, and speaks only when the pattern, or the operator,
  truly calls it up the ladder.**

---

*All SKILL-edits, the new ledger, and the `friction_trigger.py`/`friction-harvest.py` scripts named here are
operator-GATED. This is a proposal for operator blessing — not yet cite-sealed, not a decision, not code. It
SUPERSEDES the per-sprint step-0.6 ritual of `ORACLE-injection-2026-06-14.md` and KEEPS verbatim the
cite-sealed ladder (`ORACLE-stack`) and the vision-comparator sensor arm (`ORACLE-feedback-loop`).*
