---
title: "ORACLE — The Escalation Organ: the meta-process, the light injection, and the reconcile that supersedes the sprint-zero ritual"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: cartographer (future-perspective; escalation-organ component design)
supersedes: ORACLE-injection-2026-06-14.md §1–§2 (the always-on step-0.6 sprint-zero ritual)
keeps: ORACLE-2026-06-14.md (the cite-sealed STACK/ladder) · ORACLE-feedback-loop-2026-06-14.md (vision-comparator.py — the RUNTIME-OBSERVED sensor arm)
binds_existing_organs:
  - .claude/scripts/runtime-harvest.py + _lib/runtime_harvest.py     # the harvester SHAPE the friction-sentinel copies
  - .claude/scripts/ci-harvest.py · .claude/data/{ci,runtime,deprecation}-findings.jsonl  # sibling ledgers
  - .claude/scripts/vision-comparator.py + .claude/data/vision-gaps.jsonl  # the runtime-observed sensor arm (KEEP)
  - .claude/skills/agentic-developer/SKILL.md (kickoff 0.5 · Ground 1 · Judge 6 · ceiling rail)  # where friction is emitted in-flow
  - .claude/agents/{runtime,ci-failure,deprecation}-triage.md         # the triage-agent template the meta-process forks from
  - .claude/agents/cartographer.md · .claude/skills/converge/SKILL.md  # the GROUND arm (projection, roadmap §4)
  - .claude/agents/historian.md                                       # the GROUND arm (trajectory + precedent)
  - .claude/scripts/memory-kit/{cite-gen.py,spec-coherence-index.py,placement-audit.py}  # the UPDATE arm + the rung index
  - .claude/hooks/load-project-context.py                             # the SessionStart injection channel
reuses_pattern: findings-sentinel-pattern-design (flag → agent → canon → stasis), this time escalating to the ARCHITECTURAL rung
cite_sealed: NO
---

# The Escalation Organ

> *"We need that meta-work to be ESCALATED TO… escalate a performance bottleneck or blocker, recognize too
> much friction, elevate the PATTERN of the problem (not the instance) to the architectural/compositional
> design layer, do what we did today, update the specs/paths/the policy that informs the rung below, so the
> implementer can go BACK to executing in the weeds toward reaching the vision."* — the operator

This component is the **organ that replaces the ritual.** The completed oracle put a heavy step-0.6
vision-hat in front of *every* sprint. The operator's correction: most sprints stay light — the implementer
runs free in the weeds toward the vision; the architectural meta-process is **triggered, not scheduled.**
This is Beer's **algedonic signal**: System 1 (the implementer) runs unsupervised; System 4 (the
architectural meta-process) fires only when something *hurts at the level of a pattern.* It is the
self-healing loop's `detect → recover → verify → ELEVATE` arc, one rung up — elevate on **bounded-recovery
exhaustion**, from runtime-recovery to architectural-redesign. And it is the **fourth-and-a-half
instantiation** of the operator's proven `flag → agent → canon → stasis` sentinel: a deterministic ledger
flag dispatches a background agent that writes cite-sealed canon and goes quiet on blocked — except the
canon it writes is **a rung update**, not a code fix.

---

## 0 · WHAT THIS REPLACES, AND WHAT IT KEEPS (reconcile with the completed ORACLE)

The completed `ORACLE-2026-06-14.md` is **three movements plus one organ.** This component touches exactly
two of them and leaves the rest sovereign:

| Completed-oracle piece | Verdict | Why |
|---|---|---|
| **Movement 1 — THE LADDER** (`ORACLE.md`, cite-sealed seven-rung index) | **KEEP, unchanged** | The ladder is *what GROUND climbs.* The escalation organ does not replace the stack — it is the thing that *consults* it on demand. The ladder is the rung-set the meta-process reads-the-docs-for-THIS-pattern against. |
| **Movement 3 / the new organ — `vision-comparator.py` + `vision-gaps.jsonl`** | **KEEP, re-framed** | This IS the **runtime-observed sensor arm** of the escalation organ — the automatic door that watches the *deployed* system drift from a vision invariant. It supplies algedonic signal #1 (runtime). This component adds the *missing sensor*: the **in-flight DEV-friction** signal the comparator structurally cannot see (it reads `/p2p/status`, not the implementer's stalled loop). Two sensors, one organ. |
| **Movement 2 — THE VISION HAT, the always-on `/shift` step-0.6 ritual** | **SUPERSEDE** | This is the per-sprint tax the operator rejected. Step 0.6 demanded *every* shift read a rung, state a gap, frame a decision-level, run the love-test, before authorizing code. Replaced by §3 (the light injection: step 0.6 becomes a one-line *check*, not a ritual) + §2 (the meta-process the implementer escalates *into* on demand). |
| **The SessionStart `ORACLE` headline** | **KEEP, re-scoped** | Still a fourth sibling line. But it no longer announces "today's rung · the decision pending · ⚖ what-love-requires" as a standing frame the developer must consume. It **shows open escalations** (friction patterns at/over threshold + pending vision-gaps) and is **silent when none.** §3 gives the amended line. |

**The one-line reconcile:** the completed oracle built the *stack* and the *runtime sensor*; it was wrong
only to make consulting them a per-sprint ritual. Keep the stack (it's the GROUND target). Keep the
comparator (it's sensor arm #1). Replace the ritual with an **organ that fires on a pattern** — supplied
by a second sensor (dev-friction) and by operator judgment (the manual door).

---

## 1 · THE TWO DOORS INTO ONE ORGAN

Both doors raise the *same* algedonic signal and fire the *same* meta-process. The difference is only how
the signal is supplied.

### 1.1 Door A — AUTOMATIC (the friction-sentinel; a pattern crosses threshold)

A new deterministic flag, **`friction-harvest.py`** + **`.claude/data/friction.jsonl`**, the fifth
sentinel arm (siblings: `deprecations.jsonl`, `ci-findings.jsonl`, `runtime-findings.jsonl`,
`vision-gaps.jsonl`). It does **not poll**; it reads what the dev loop already writes. **One blocker is
weeds-work; the PATTERN is the signal.** Two threshold shapes — both are *bounded-recovery exhaustion*:

- **SAME-WALL-K-TIMES** — the same friction fingerprint recurs `K` times across iterations/shifts (default
  `K=3`). This is `detect → recover → verify → ELEVATE` made literal: the normal fix was tried, the friction
  *kept returning*. (Mirrors `ci-harvest.py`'s flake `seen` count — recurrence is the leverage.)
- **CLUSTERING-AT-ONE-SEAM** — `M` *distinct* friction fingerprints land on the **same rung/seam** within a
  window (default `M=4`, window = the current shift + the prior two). Many different walls at one seam =
  the seam itself is wrong, not any one wall.

A fingerprint that crosses *either* threshold flips a friction entry to `status: escalate-ready` and the
SessionStart line shows it. **Below threshold, the organ is silent — the implementer never sees it.** This
is the whole anti-ritual guarantee: friction below a pattern is just the weeds, and the weeds are where the
implementer is *supposed* to be.

### 1.2 Door B — MANUAL (the operator/implementer supplies the signal by judgment)

When the operator says *"go read the docs, get the vision and the trajectory"* — or the implementer
recognizes it themselves mid-shift — that **IS the algedonic signal, supplied by judgment, skipping the
threshold.** No accumulator needs to fill. The manual door is a one-liner anyone can fire:

```bash
python3 .claude/scripts/friction-harvest.py --escalate "<one-sentence pattern statement>" [--rung <hint>]
```

It writes a `friction.jsonl` entry with `source: manual`, `status: escalate-ready` immediately, and the
next session-start (or the in-flight loop) runs the *same* meta-process §2 against it. The manual door is
the operator's hand on the algedonic cord; the automatic door is the cord pulled by accumulated pain. **One
cord, two hands.**

### 1.3 Where the friction signal is EMITTED (binding the /shift loop, not a new surface)

The friction sentinel does not invent a capture surface — it reads the dev loop's **existing in-flight
honesty signals** (`agentic-developer/SKILL.md`):

| Existing /shift signal | What it already records | Becomes a friction fingerprint when… |
|---|---|---|
| **Judge: `stall`** (step 6) | "no delta over 2+ iterations" | a stall fingerprint (normalized symptom + touched-rung) recurs `K` times → SAME-WALL |
| **Judge: `bail`** (step 6) | "stuck… out of ideas; close with question" | every bail is a candidate; bail's closing question IS the executive question seed |
| **Ceiling rail** (CI findings rails) | a finding whose resolution "needs a design decision, an architecture change, a substrate capability, or cross-cutting work beyond the Objective's path scope" → `ci_status: blocked` + routed to `/brainstorm` | the ceiling rail is *already* the in-loop "this is above my rung" detector — the friction sentinel reads its `blocked` emissions; **a single ceiling hit at one seam, repeated, IS the cluster** |
| **Journal: Observed anti-patterns** (step 7) | accumulated per-shift watch-outs | a watch-out matching a SUPERSEDED history silhouette (the 0.5 discovery hazard) recurring across shifts |
| **`dev-intent.jsonl`** (exploration fallback) | "what was built, learner impact, which a2o feature needs updating" | an intent whose summary repeatedly names the same workaround-around-an-architecture-wall |

The ceiling rail is the keystone binding: **it already exists to say "stop iterating, start designing."**
Today it routes to `/brainstorm` (a fresh design conversation). The escalation organ gives it a *better
terminal*: route the **pattern** (when it clusters/recurs) to the structured meta-process §2 that GROUNDs
in the existing rung + trajectory before designing — `do what we did today`, not brainstorm from zero.

---

## 2 · THE META-PROCESS (what fires on escalation — GROUND → DECIDE → UPDATE → HAND BACK)

This is `do what we did today` made into a repeatable subroutine. It is dispatched **exactly as a
sentinel triage agent is dispatched** (`run_in_background`, the dev's current task never derails —
identical to `runtime-harvest.py:166-176`'s dispatch directive), except the agent is an **architectural
design pass** and its canon is **a rung update.** It is a `/shift` subroutine — not a new top-level skill —
so it composes into the loop the implementer is already in.

### 2.1 GROUND — surface the RIGHT rung + trajectory + precedent for THIS pattern (not everything)

The defining discipline: **read-the-docs-for-THIS-pattern, not read everything.** This is the operator's
*"go read the docs, get the vision and the trajectory"* made surgical. Three existing arms, scoped to the
escalating fingerprint's seam:

1. **The RIGHT RUNG (the stack).** Resolve the pattern's seam → its governing rung via the kept
   `ORACLE.md` ladder + `spec-coherence-index.py --query "<pattern statement>"` (the same lexical prior-art
   index `/shift` step 0.5 already runs) + JIT MemPalace semantic recall (defeats vocabulary drift). Open
   the **governing doc, not its children** — the *why*, not the *how*. One rung read, chosen by the
   fingerprint, never the whole ladder.
2. **The TRAJECTORY + PRECEDENT (the historian).** Dispatch the **historian** for "has this seam been hit
   before, and how was it resolved or worked-around?" — the historian's standing job (precedent + chronicle
   surfacing). A SUPERSEDED match means *the pattern was already escalated once* — the meta-process extends
   that precedent instead of re-deciding from zero (born-linked, the compaction-loop FRONT-fire discipline).
3. **The PROJECTION (the cartographer).** Intersect the pattern with the live `placement-audit.py --ledger`
   × `--focus` (TESTABLE-now vs BLOCKED-BY-ENV) × the gospel-tier vision axis — so the decision is framed
   against what's *actually buildable now*, and never ranks BLOCKED-BY-ENV work. (This is the cartographer's
   existing ROADMAP-CURRENCY mandate, pointed at one pattern.)

GROUND's output is a **one-page brief**: the pattern in one sentence, the governing rung cite, the
trajectory line (prior precedent or "new ground"), and the buildable-now framing. *That* is the docs the
implementer needed — surfaced for the pattern, not ritually for the sprint.

### 2.2 DECIDE — at the right level (frame VISION / ARCHITECTURE / COMPOSITION / DELIVERY)

The brief frames the executive decision as a **level question** (the surviving load-bearing kernel of the
old step-0.6, now run only *on escalation*):

- **VISION** (values/theology — operator-only): park as `status: blocked-operator-call`. The six
  irreducible value-calls and the `ReservedPlace`/`patience` guardians route here; surfaced once, never
  re-fired (the sentinel's blocked-with-valid-blocker = stasis).
- **ARCHITECTURE** (primitive vs instantiation, coverage-domain): the meta-process **owns this** — it
  produces a design-doc recommendation and the rung edit. This is `do what we did today`.
- **COMPOSITION** (how existing primitives wire at the seam): the meta-process owns it — usually a
  spec/path/policy clarification, not a new primitive.
- **DELIVERY** (which gap-item first): **hand straight back** — this was never an escalation; the
  threshold fired on noise, or the manual door was pulled on weeds-work. Demote the friction entry, return
  the implementer to the loop. *Most* escalations that survive to DECIDE are ARCHITECTURE/COMPOSITION; a
  DELIVERY verdict is the organ honestly saying "this was weeds — go back."

### 2.3 UPDATE — the rung-below specs/paths/policy (cite-sealed; the canon arm of the sentinel)

The decision's artifact is **an edit to the rung that informs the layer below** — the operator's *"update
the specs/paths/the policy that informs the rung below."* This is the sentinel's **canon** layer, lifted
to the architectural level:

- The architecture/composition doc gains a clause; OR a path/policy (`cluster-state.yaml`, `rung-map`, a
  manifest, a managed surface) is corrected; OR a plan is authored with a pre-authored `shift_objective`.
- **Cite-sealed via `cite-gen.py`** (the move-proof pointer) — content-addressed, so the rung edit
  re-points inbound cites without breaking them; the stack stays coherent across the edit.
- A `backlog/friction-<slug>.md` (timeline-CONVENTIONS-conformant, the cartographer's existing schema)
  records `decision_ref` = the rung commit. One entry per **concern** (fingerprints N:1), exactly the
  sentinel's canon discipline.

### 2.4 HAND BACK — the implementer resumes in the weeds; the friction entry goes to stasis

The updated rung **propagates down** (content-addressed cites carry it; the next `/shift` step 0.5
discovery surfaces the now-amended seed). The implementer **resumes in the weeds against a better
substrate** — the whole point. The friction fingerprint is **DELETED from `friction.jsonl`** (the rung
commit + the backlog entry are the durable record; reintroduction reads as NEW → re-escalates = regression
handling for free). A VISION-level park stays as `blocked-operator-call` (correctly parked, not drift).
**Stasis = friction.jsonl empty or blocked-operator-call only** — the sentinel's stasis definition,
verbatim.

```
  [/shift Judge:stall/bail · ceiling-rail:blocked · dev-intent]   [operator/implementer says "read the docs"]
                    │  (Door A: pattern accumulates)                          │  (Door B: judgment supplies it)
                    ▼                                                         ▼
            friction-harvest.py  ──fingerprint, threshold (K-times | cluster)──►  friction.jsonl  status:escalate-ready
                    │
       SessionStart `escalation:` line  (silent when none)  ── shows it once
                    │
                    ▼   run_in_background dispatch (current task never derails)
        ┌──────────────────────── THE META-PROCESS (one /shift subroutine) ───────────────────────┐
        │  GROUND   right rung (ORACLE.md + spec-coherence-index + JIT MemPalace)                   │
        │           + trajectory/precedent (historian) + buildable-now (cartographer --ledger/focus)│
        │  DECIDE   level: VISION(park) · ARCHITECTURE(own) · COMPOSITION(own) · DELIVERY(hand back)│
        │  UPDATE   edit the rung-below spec/path/policy, cite-seal (cite-gen) → backlog/friction-* │
        │  HAND BACK rung propagates (content-addressed cites) → implementer resumes in the weeds   │
        └──────────────────────────────────────────────────────────────────────────────────────────┘
                    │
                    ▼
            friction.jsonl entry DELETED (stasis)  ·  no re-fire on the same pattern  ·  VISION-call parked
```

---

## 3 · THE LIGHT INJECTION (replace step-0.6 with a one-line check)

### 3.1 The amended SessionStart line (supersedes the heavy `ORACLE` frame)

The old `ORACLE` headline announced a standing frame the developer had to consume every session
(rung · decision-pending · ⚖ what-love-requires). The amended line **shows open escalations and is silent
otherwise** — it echoes two ledgers, nothing more:

```
escalation: ✅ none          # the common case — silent, the implementer stays in the weeds

escalation: ⚠ 2 ready        # the uncommon case — a pattern crossed threshold, OR a vision-gap awaits a call
  ⚠ friction: "warm-stream serial-loop wall hit 3× across 2 shifts" → seam: doorway upstream-warmup (ARCHITECTURE)
  ⚖ vision-gap: coverage-union-full @ alpha (laptop ships as leecher) → cartographer  [blocked-operator-call]
```

Resolution is pure echo of already-written facts (deterministic, no LLM): count `friction.jsonl` entries
where `status == escalate-ready` + `vision-gaps.jsonl` entries where `status ∈ {open, blocked-operator-call}`;
print the highest-`seen` friction line + the highest-leverage vision-gap. Wired through the **same**
`load-project-context.py` → `placement-audit.py --headline` / `_gate_subprocess` plumbing as the other
sibling lines, fail-safe exit-0 (a sentinel crash surfaces as `⚠ gate-error`, never silently vanishes).

**The discipline:** the line is a *doorbell, not a frame.* It does not put the vision hat on the
developer's head every session — it rings only when a pattern needs the meta-process, and the implementer
chooses whether to answer now or after the current loop. Silence is the loving default.

### 3.2 What step-0.6 becomes (the supersede, concretely)

`agentic-developer/SKILL.md` does **not** gain a heavy step-0.6 ritual. Instead:

- **Kickoff stays light.** Steps 0 (mode) → 0.5 (discovery) → 1 (Objective interview) run as today. The
  implementer goes into the weeds. No mandatory rung-read, no mandatory decision-level framing, no
  mandatory love-test per sprint.
- **Step 0.6 becomes a one-line CHECK** (not a ritual): *"If the `escalation:` line showed a ready pattern
  on a seam this Objective touches, run the meta-process §2 first (GROUND→DECIDE→UPDATE→HAND BACK), then
  resume kickoff. Otherwise proceed."* That's the entire injection — a conditional, not a tax.
- **The in-flight escalation hook** lives at the loop's **Judge step (6)** and the **ceiling rail**: when
  the loop itself produces a `stall`/`bail`/`blocked` that crosses threshold (or the implementer recognizes
  the pattern), it fires Door A/B and runs the meta-process *as a subroutine of the current shift*, then
  returns to the weeds. Escalation is **inside the work, on demand** — not a gate in front of it.

This is the operator's correction made mechanical: the meta-work is **escalated to**, by a pattern or a
word, not performed as a ritual before every sprint.

---

## 4 · BIND TO flag → agent → canon → stasis (the friction-sentinel is the new arm)

The escalation organ is the **fifth instantiation** of the operator's proven sentinel pattern
(`findings-sentinel-pattern-design`, §1) — inheriting every anti-dump property verbatim:

| Sentinel layer | Deprecation (A) | CI (B) | Runtime (D) | Vision-gap (oracle) | **Friction-escalation (NEW)** |
|---|---|---|---|---|---|
| **1. Deterministic flag** | `deprecation-sentinel.py` | `ci-harvest.py` | `runtime-harvest.py` | `vision-comparator.py` | **`friction-harvest.py`** → `friction.jsonl` |
| **2. Background dispatch** | `deprecation-triage` | `ci-failure-triage` | `runtime-triage` | cartographer | **the meta-process** (architectural design pass — GROUND→DECIDE→UPDATE; a `/shift` subroutine, run_in_background) |
| **3. Canonical backlog** | `backlog/deprecation-*` | museum + ledger | `backlog/runtime-*` | `backlog/vision-gap-*` | **`backlog/friction-*`** + **the rung edit itself** (canon = the spec/path/policy update, cite-sealed) |
| **4. Stasis sweep** | `/deprecation-stasis` | dev-loop rails | deterministic | `/converge` + roadmap | **`/converge`** (drains escalate-ready, re-checks blocked-operator-calls whose value may have been decided) |

The properties transfer for free: **fingerprint dedupe** (a recurring friction pattern is ONE entry, not
noise); **presence-suppresses-dispatch** (an escalated-and-blocked pattern NEVER re-fires the meta-process
— the operator is not nagged about an architecture call already made or a vision call not yet makeable);
**close-by-decomposition** (a resolved pattern is deleted; the rung commit is the record); **blocked =
stasis** (a VISION-level park is a first-class terminal state, not drift). The operator's words *"loop
that back… without re-firing on blocked"* are this pattern's defining property — already built four times.

---

## 5 · THE SMALLEST REAL FIRST IMPLEMENTATION

Prove the **manual door + the meta-process + the light line**, end to end, before any automatic-threshold
accumulator. The manual door needs no accumulator — it is the operator's *"go read the docs"* made
executable, and it exercises the entire loop on day one.

1. **`friction.jsonl` + the manual door (`friction-harvest.py --escalate`).** ~80 lines, copy-shaped from
   `runtime-harvest.py`'s shell (`load_jsonl`/`write_jsonl`/`flock`/fingerprint/fail-safe exit-0). v1
   supports **only** `--escalate "<sentence>" [--rung <hint>]` (writes one `escalate-ready` entry,
   `source: manual`) and `--hook` (emits the `escalation:` line). No auto-thresholds yet — Door B only.
2. **The `escalation:` SessionStart line.** Add `escalation_line()` to `placement-audit.py`, wired through
   the existing `--headline`/`_gate_subprocess` plumbing. v1 echoes: count of `escalate-ready` friction
   entries + `open`/`blocked-operator-call` vision-gaps; print the top one each; `✅ none` when empty,
   silent in the headline when clean. A stub that honestly echoes is a live instrument.
3. **The meta-process as a `/shift` subroutine (prose, no new script).** Add the §2 four-step subroutine
   (GROUND→DECIDE→UPDATE→HAND BACK) to `agentic-developer/SKILL.md` as a **named subroutine** the loop
   calls from Judge (step 6) and from the light step-0.6 check — exactly as step 0.5's discovery is prose
   the Opus orchestrator runs. GROUND reuses `spec-coherence-index.py` + the historian + the cartographer
   verbatim; UPDATE reuses `cite-gen.py` + the existing `backlog/` schema. **No new agent** — the
   cartographer (already Opus, already the vision-hat) wears it; the dispatch directive forks
   `runtime-triage.md`'s template, retargeted to a rung update.
4. **Supersede step-0.6.** Replace the heavy ritual block in `agentic-developer/SKILL.md` with the §3.2
   one-line conditional check. Net change to the kickoff: **−1 ritual, +1 conditional.**

This v1 delivers the entire load-bearing value — *the implementer escalates to the meta-process on a word,
GROUNDs in the right rung, decides at the right level, updates the rung, and returns to the weeds* — with
**one new script (~80 lines), one headline function, one prose subroutine, and the deletion of the
ritual.** The automatic door (the threshold accumulator reading `/shift` Judge/ceiling emissions) is a
*later pass that arms the same loop* — once the manual door has proven the meta-process is worth firing,
the SAME-WALL-K-times and CLUSTERING-at-a-seam predicates compose as new evaluators over `friction.jsonl`,
never a new machine. (This is the sentinel discipline: prove the loop on one real signal — here the manual
door — then widen by adding predicates.)

### What we deliberately do NOT build first
- **No auto-threshold accumulator** until the manual door proves the meta-process (Door B first; Door A is
  the widening).
- **No new agent** (the cartographer is the vision hat; the historian surfaces precedent — both exist).
- **No actuation** — the meta-process *updates a rung*; it never reaches into runtime (the
  no-runtime-write rule, inherited from every sensor arm).
- **No re-import of the heavy ritual** under any flag — the supersede is total, not toggled.

---

## 6 · WHAT LOVE REQUIRES (the closing test)

**The meta-process serves the implementer — it returns them to the weeds with a better substrate, and it
grounds the vision for the RIGHT pattern, not ritually.** The whole correction is an act of restraint: the
oracle that loved well would *not* put the vision hat on the developer's head every sprint. It would let
the implementer run free in the weeds toward the vision — where the implementer is *supposed* to be — and
ring the doorbell only when a **pattern** (not an instance) needed the architectural rung. Three structural
refusals, inherited from the architecture the organ measures:

- **The organ measures the work, never the worker.** There is no `escalate(person)` predicate. The friction
  sentinel flags *"this seam hit a wall 3 times"*; it never flags *"developer X stalls too often."*
  Developer-brain is what we take *off* on escalation, not what we *score* by it. An organ that *forced*
  escalation would re-import the per-sprint tax the operator just removed — so the doorbell *rings and
  surfaces*; it does not *compel.* A DELIVERY-level verdict hands straight back to the weeds.
- **Silence is the loving default.** Below threshold, the organ says nothing. Patience over engagement is
  structural: the organ does not optimize for the operator answering the doorbell; it waits, and most
  sessions print `escalation: ✅ none` and vanish. A VISION-level park is surfaced once and held — never
  re-fired — so the operator is never nagged about a call already made or not yet makeable.
- **Escalation serves the descent, never replaces it.** The meta-process exists to UPDATE the rung so the
  implementer can go *back down* and execute. It is not a place to live. HAND BACK is the load-bearing
  step: the organ's success is measured by how fast it returns the implementer to the weeds against an
  amended substrate — `do what we did today, then go build.`

The escalation organ hands the implementer the freedom to stay in the weeds, the doorbell that rings only
when a pattern needs the rung, and the meta-process that — on a word or on a wall hit three times — surfaces
the right vision for *this* pattern, decides at the right level, updates the rung below, and gets out of the
way. It does not climb the ladder for every sprint. It climbs only when the work hurts at the level of a
pattern, and then only to make the next descent better.

---

*All `git mv` / `--seal` / SKILL-edit / ledger-schema acts named here are operator-GATED. This is a
proposal for operator blessing — reconciled with the completed `ORACLE-2026-06-14.md` (keep the stack +
the comparator; supersede the heavy ritual) — not yet cite-sealed, not a decision, not code.*
