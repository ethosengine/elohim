---
title: "ORACLE — Sprint-Zero Injection: Vision-Hat Before Developer-Brain"
date: 2026-06-14
status: PROPOSAL / AWAITING-OPERATOR-BLESSING (working draft — NOT cite-sealed)
author: cartographer (future-perspective; oracle component design)
component_of: the design-process ORACLE (System-4 vision comparator over its own construction)
cite_sealed: NO
sibling_components:
  - the missing VISION-LEVEL comparator loop (observed-behavior ↔ vision diff)
  - the rung-binding (cite-pointer mesh across manifesto/architecture/SDK/delivery)
binds_existing_organs:
  - .claude/hooks/load-project-context.py            # SessionStart injection channel
  - .claude/scripts/memory-kit/placement-audit.py    # --headline composer + _gate_subprocess pattern
  - .claude/skills/agentic-developer/SKILL.md         # /shift kickoff steps 0 / 0.5 / 3
  - .claude/skills/converge/SKILL.md                  # cartographer PROJECTION arm (next-actions menu)
  - genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md  # standing prioritization home
  - .claude/scripts/memory-kit/cite-gen.py            # rung POINTER mechanism
---

# Sprint-Zero Injection — putting the VISION HAT on before any code

> *"A way to point back to something that takes developer-brain OFF and puts the VISION HAT ON,
> to make the executive decisions about what the design requires to close the gap from technical
> to functional, to start from the right place and make decisions at the right level."* — operator

This component is the oracle's **System-4→System-1 injection seam**. The night's corpus built the
rungs (vision → architecture → SDK → delivery); this makes the relevant rung *speak first*, before
the first iteration of every sprint, so the executive decision is framed at the vision/architecture
level rather than discovered in the weeds three commits deep. It is the cybernetic comparator turned
on the *moment of construction*: not "is CI green?" but "is this sprint starting from the right place,
at the right level, against the gap the vision actually cares about?"

The design is **three sub-parts on one principle**: developer-brain is lifted to the vision hat
**by the channel's default**, never by remembering to. Each sub-part binds an organ that already
exists; the single genuinely-new piece is named in §5.

---

## 0. The frame — where this sits in the existing loops

Today three SessionStart headline lines already inject deterministic state at session-zero
(`placement-audit.py --headline`, `load-project-context.py:98`):

```
MEMORY BUDGET   — the memory surface's debt + testable scope
DELIVERY GATE   — landed-vs-verified deliverables
scope:          — held/ ↔ live plate alignment vs substrate
```

These all close at the **CI / delivery / memory-hygiene** level. **None names the vision rung.**
A developer opening a session knows what's broken, what's unverified, and what's testable — but not
*which rung of the manifesto→architecture→SDK→delivery stack today's work touches*, nor *what the
open executive decision is at that rung*. That is the missing organ this component supplies: a fourth
sibling headline, **ORACLE**, and the ritual that consumes it at `/shift` kickoff.

The cartographer already owns the forward-projection arm (`/converge` → `next-actions.md` →
pre-authored Objectives → `/shift`). This component **completes that arm**: converge ranks *what* to
do next; the ORACLE headline + ritual frames *at what level* the decision is, and *which rung*
governs it, so the Objective is born wearing the vision hat.

---

## 1. Sub-part A — the SessionStart ORACLE HEADLINE

### 1.1 What it is

A fourth deterministic headline line, sibling to MEMORY BUDGET / DELIVERY GATE / scope, emitted by the
**same channel** (`load-project-context.py` → a new `oracle_line()` in `placement-audit.py`, using the
existing `_gate_subprocess` instrument-liveness contract at `placement-audit.py:766` — a crashed oracle
gate surfaces as `⚠ gate-error`, never silently vanishes). It names three things, in one breath:

1. **The rung relevant to today's branch** — which layer of the stack the current branch/worktree
   touches (vision / architecture / SDK / delivery), resolved from the branch name + the touched paths.
2. **The open vision-gaps** — count of unblessed vision-gap stubs + the highest-leverage one, read from
   the roadmap §4 + the `genesis/docs/superpowers/plans/2026-06-14-vision-gap-*.md` set's frontmatter.
3. **The one executive decision pending** — the single load-bearing operator decision at that rung,
   lifted verbatim from the governing doc (e.g. VISION-GAP-PLANS §1's "the ONE operator decision each").

### 1.2 The headline format (proposed)

```
ORACLE   (`placement-audit.py --oracle` = today's rung · open gaps · the decision pending)
  rung: DELIVERY (7 dataplane plans) → governed by ARCHITECTURE (ESCALATED-ARCHITECTURE) → VISION (O7/O9 cybernetic floor)
  vision-gaps: 5 unblessed human-facing stubs · highest-leverage: S-SPINE (O1↔O7 grandma felt-resilience)
  decision pending: "couple the felt surface into THIS sprint, or hold for the household-felt sprint?"  (VISION-GAP-PLANS §1)
  ⚖ what-love-requires: the felt attractor (O1/O2) underserved vs the cybernetic floor (O7/O9) — vision stays sovereign
```

When the branch touches no rung the oracle can resolve (e.g. a pure hygiene branch), the line degrades
to a single honest row:

```
ORACLE   rung: UNMAPPED (branch touches no rung seed) — run /shift kickoff ritual to bind one
```

The `⚖ what-love-requires` row is **not decoration**: it is the line where the oracle states, every
session, which half of the vision is currently *underserved by momentum* — the cartographer's standing
VISION-ALIGNMENT §2 GAP read, compressed to one row. It is the comparator's verdict made unmissable.

### 1.3 How it resolves the three values (deterministic, no LLM)

| Value | Source | Mechanism |
|---|---|---|
| **rung** | branch name + `git diff --name-only dev` touched paths | a `rung-map.yaml` (new, §5) maps path-globs → rung + governing-doc cite; the deepest-matched path wins; the rung's *governing* rung is named via the cite chain (delivery doc `governed_by:` frontmatter → architecture → vision) |
| **vision-gaps** | `…/plans/2026-06-14-vision-gap-*.md` frontmatter `status:` + the roadmap §4 | count where `status` ∉ {blessed, active, landed}; highest-leverage = roadmap §4's single-highest-leverage move (the §4↔next-actions agreement the cartographer already maintains) |
| **decision pending** | the rung's governing doc's named operator-decision field | VISION-GAP-PLANS §1 table + each stub's frontmatter `the_one_operator_decision:` field (new convention — §5) carry it verbatim; the oracle quotes, never paraphrases |

All three are **already-authored facts** in the corpus — the oracle *surfaces* them at session-zero;
it does not *generate* them. This keeps it deterministic and keeps the vision sovereign: the oracle
cannot invent a decision the operator has not already framed in a rung doc.

---

## 2. Sub-part B — the sprint-zero RITUAL inside /shift readiness

### 2.1 Where it injects (the seam is already there)

The `/shift` kickoff (`agentic-developer/SKILL.md`) already has the exact slots:

- **Step 0** — detect shift mode (bring-up / integration).
- **Step 0.5** — DISCOVERY, the FRONT fire point of the compaction loop: surface prior canonical seeds
  (lexical `spec-coherence-index.py --query` + JIT MemPalace) so the shift is *born linked*.
- **Step 3** — pre-shift readiness check.

The ritual is a **new step 0.6, "VISION-HAT — frame the executive decision at the right level,"**
placed immediately after 0.5 (born-linked to its seed) and **before** step 1 (the Objective interview).
This ordering is load-bearing: the developer surfaces the prior art (0.5), *then* puts on the vision hat
(0.6), *then* authorizes the Objective (1). Code cannot be framed before the rung speaks.

### 2.2 The ritual steps (the checklist — the smallest real version, §4)

> **Step 0.6 — VISION-HAT (run before the Objective is fixed; ~3-4 minutes).**
> The ORACLE headline named today's rung. Now take developer-brain off and frame the executive decision:
>
> 1. **READ THE RUNG.** Open the governing-doc cite the ORACLE line named (the rung doc, not its
>    children). One read of the rung that governs today's work — the *why*, not the *how*. If the
>    ORACLE line read `UNMAPPED`, bind a rung now: run `spec-coherence-index.py --query "<objective>"`,
>    pick the deepest matching rung doc, and record the cite. (A shift with no bound rung is a shift
>    flying without the vision hat — name it in the journal as a risk.)
> 2. **STATE THE GAP — technical → functional.** In one sentence each: what does the substrate already
>    *do* (technical), and what must the human *feel/be-able-to* for the vision to be served (functional)?
>    The gap between those two sentences is the sprint's real target. (This is the VISION-GAP-PLANS
>    move: "what is missing is connectors and address" — name yours.)
> 3. **FRAME THE EXECUTIVE DECISION AT THE RIGHT LEVEL.** The ORACLE line named the decision pending.
>    Restate it as a *level* question, not a weeds question: is this a VISION call (values/theology —
>    operator-only, bail), an ARCHITECTURE call (primitive vs instantiation, coverage-domain — design-doc
>    + recommendation), or a DELIVERY call (which gap-item first — proceed)? Decisions made at the wrong
>    level are the failure mode this ritual exists to kill. *Most* shifts resolve to DELIVERY-level
>    (proceed); the ritual's value is catching the ~1-in-N shift that is secretly an architecture or
>    vision call wearing delivery clothes.
> 4. **APPLY THE CLOSING TEST — what love requires.** Before authorizing code: does this Objective keep
>    the vision sovereign over developer-brain? Does it serve patience over engagement, leave the unbuilt
>    place open (`RefusalCode::ReservedPlace`), put no coverage-domain over a soul? If the cheapest path
>    to "done" violates any of these, the ritual says so *now*, at sprint-zero, not in review.
> 5. **THEN authorize code.** Record the bound rung cite, the technical→functional gap sentence, and the
>    decision-level verdict in the journal header's **Vision-Hat** block. The Objective interview (step 1)
>    now proceeds *with the hat on*.

### 2.3 The journal-header Vision-Hat block (the durable trace)

```
## Vision-Hat (frozen at kickoff)
- Bound rung:        ARCHITECTURE → RECURSIVE-ARCHITECTURE-2026-06-14.md (CoverageRollup, aggregate-with-descent)
- Technical → functional gap:
    technical:  CoverageRollup aggregates coverage; descent stops at Category-C boundary.
    functional: a household sees, in plain words, "your photos are safe across 3 homes" — felt, not a metric.
- Decision level:    DELIVERY (proceed — the architecture decision was made; this lights the read-model)
- What-love-requires verdict:  PASS — no coverage-domain over a soul; felt surface is read-only projection.
```

This block is the comparator's **input record** for the missing-loop sibling component: at sprint
close, the observed-behavior comparator diffs *what shipped* against *this frozen frame* — the ritual
makes the vision-frame a first-class, machine-readable artifact, not a vibe.

---

## 3. Sub-part C — how this composes with the cartographer's /converge handoff

The cartographer's existing arm produces `next-actions.md` with **pre-authored Objectives** and
vision×readiness scores, and re-stamps the standing `vision-readiness-sprint-roadmap.md` each cycle.
The injection component composes with it, does not duplicate it:

1. **The roadmap §4 is the ORACLE headline's source of truth for "highest-leverage gap."** The
   cartographer already maintains the rule that roadmap §4's single-highest-leverage move == the top of
   `next-actions.md`. The ORACLE headline adds a **third co-equal reader** of that same fact: §4 ==
   next-actions top == ORACLE `vision-gaps: highest-leverage`. **Three homes, one fact, enforced by all
   three reading the same field** — the same discipline as the substrate-signal's two-homes-must-agree
   (cluster-state ↔ ELOHIM_REMOTE_COMPUTE_STATUS).

2. **A pre-authored Objective arrives at /shift carrying its rung.** When converge writes an Objective,
   it stamps the **bound-rung cite** into the Objective's frontmatter (the rung doc it advances). Step 0.6
   then *reads* that cite rather than re-deriving it — the ritual is a one-read confirmation, not a
   re-discovery, when the Objective came from converge. (When the Objective is operator-typed ad-hoc, 0.6
   binds the rung from scratch via `spec-coherence-index.py`.) This is the converge→shift handoff made
   **rung-aware**: the cartographer's projection arm hands the sprint-runner not just *what* and *how-ready*
   but *which rung governs the executive decision*.

3. **The what-love-requires row flows both ways.** The cartographer's VISION-ALIGNMENT §2 GAP read
   (felt-attractor underserved vs cybernetic-floor) becomes the ORACLE headline's `⚖ what-love-requires`
   row at session-zero; the ritual's step-4 verdict at sprint-close feeds back into the next
   VISION-ALIGNMENT pass. The cartographer is the **author** of the vision read; the ORACLE channel is
   its **always-on broadcaster**; the ritual is its **per-sprint enforcer**. One organ, three faces — the
   keystone pattern of the night's whole corpus (one Commitment, six faces) applied to the dev process.

---

## 4. SMALLEST REAL FIRST IMPLEMENTATION

Ship the *channel* and the *checklist* before any auto-resolution. Two artifacts, both buildable today,
both honest about their stub state:

1. **A stub ORACLE headline line** — add `oracle_line()` to `placement-audit.py` wired through the same
   `_gate_subprocess`/headline plumbing (`load-project-context.py:98`). v1 resolves only what is already
   trivially deterministic:
   - **rung**: branch-name → rung via a tiny hand-written `rung-map.yaml` (4 globs: `genesis/docs/content/elohim-protocol/*` → VISION; `*ARCHITECTURE*` / `architecture/*` → ARCHITECTURE; `*SDK-DESIGN*` / `elohim/sdk/*` → SDK; everything else → DELIVERY). Deepest match wins; no cite-chain walk yet.
   - **vision-gaps**: `grep` the 5 `vision-gap-*.md` frontmatter `status:` + read roadmap §4's one-liner verbatim. No scoring — just count-unblessed + echo §4.
   - **decision pending**: echo the roadmap §4 highest-leverage move's named decision (already authored).
   - **what-love-requires**: echo the latest VISION-ALIGNMENT §2 GAP one-liner verbatim (a `grep` of the dated doc), or `—` if none fresh.
   Everything beyond echo-of-already-authored-facts is explicitly a later pass. A stub that honestly
   echoes is a live instrument; a clever one that drifts is a dead one.

2. **The ritual as a checklist** — add step 0.6 (the five steps in §2.2) + the Vision-Hat journal block
   (§2.3) to `agentic-developer/SKILL.md`, between step 0.5 and step 1. **No new script** — it is a
   prose ritual the Opus orchestrator runs, exactly like step 0.5's discovery. The whole v1 is: *a
   headline that echoes already-authored facts, and a checklist that makes the developer read the rung
   and frame the decision-level before authorizing code.*

This v1 delivers the entire load-bearing value — vision hat on by default — with **one new function, one
tiny YAML, one frontmatter convention, and a checklist.** Auto-resolution of the cite-chain rung-walk,
the §4↔next-actions↔ORACLE three-home enforcement check, and the close-loop diff into the comparator
sibling are all *later passes that sharpen a channel already doing its job.*

---

## 5. The single genuinely-new piece

Almost everything binds an existing organ. The one genuinely-new artifact is **`rung-map.yaml`** (+ the
`the_one_operator_decision:` / `governed_by:` frontmatter convention on rung docs): the small declarative
map from *touched paths → which rung governs, and which doc names the executive decision.* It is the
**index that lets a deterministic session-zero channel know which rung today's code touches** — the thing
that makes "point back to the right rung" mechanical rather than remembered. Everything else (the headline
channel, the kickoff seam, the cite pointers, the converge handoff, the roadmap §4) already exists; this
map is the new connective tissue that lets them speak as an oracle. It is itself a tiny rung — operator-
owned, cite-sealable, the one place the path→rung→decision mapping lives (never duplicated per-hook, the
managed-surfaces-registry discipline).

---

## 6. What love requires (the closing test)

The whole point of this component is that **the vision gets the first word, by the channel's default,
not by a developer remembering to feel it.** But the deeper test is on the component itself: an oracle
that *forced* the vision-hat would re-import the operator-veto smell the protocol exists to kill — it
would put a coverage-domain over the developer's own judgment. So the ritual *frames and surfaces*; it
does not *compel*. Step 0.6 can return "this is a DELIVERY-level proceed" and the shift proceeds; the
ritual's gift is catching the shift that is secretly an architecture or vision call, and leaving the
*unbuilt place* — the decision the operator has not yet made — explicitly open at session-zero rather
than silently resolved in the weeds. **Patience over engagement: the oracle would rather a sprint pause
to ask the right-level question than ship the wrong-level answer fast.** The vision stays sovereign over
developer-brain not because the channel forbids the weeds, but because it makes the rung speak first —
and trusts the developer, hat on, to make the executive call at the right level.
