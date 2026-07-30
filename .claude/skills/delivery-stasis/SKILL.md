---
name: delivery-stasis
description: "Drives the whole development cycle to stasis against the developer docs in one loop — the operator-as-conveyor role, formalized. Each round reads the class-aware delivery scoreboard (per-job CI verdicts, env-red≠code-red, ledgers), dispatches the equipped station for the highest-leverage pressure (/converge, /deliver, /close-loop, scope-reconcile, triage, memory loops; pre-authors /shift Objectives), re-measures until only ceiling items remain — then presents the ceiling menu (the 2-5 decisions only the operator can make). Under explicit launch-time BUILD authorization (overnight /loop), each cycle adds a build leg: fires the top vision-ranked OPEN Objective as an agentic-developer arc under an evidence-gated push lease. Use when \"drive the dev cycle to stasis\", \"delivery stasis pass\", \"run the conveyor\", at session boundaries, or looped overnight with build mode granted. NOT for a single station's work (invoke it directly) or memory-only hygiene (use /memory-stasis-loop)."
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/delivery-stasis"
---

# Delivery Stasis Loop

Spec: `genesis/docs/superpowers/specs/2026-06-06-delivery-stasis-loop-design.md`.
The role inversion: the operator stops being the conveyor between stations
and becomes the ceiling. Each round's true deliverable is a **shrinking
scoreboard plus a short ceiling menu** — never raw work reports.

**Stasis** := every developer-doc claim is verified-delivered, in-flight with
a live trajectory, held-by-env, or on the ceiling menu. Nothing orphaned
between stations waiting to be noticed.

## The loop

Each round:

### 1. Measure (full scoreboard — deterministic, local)

```bash
python3 .claude/scripts/ci-harvest.py            # fresh CI evidence (network)
python3 .claude/scripts/delivery-scoreboard.py   # the whole board, pure-local
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | head -25   # per-file queue when needed
```

The scoreboard's CI floor is **failure-class-aware** — read the per-job
VERDICT, not the bare ratio. `code-red`/`attention` rows are this loop's
dispatchable CI surface; `env-gated` red rides the ceiling/held track;
`fix-landed`/`clearing` rows are WAIT states (the harvester's disappearance
sweep owns their closure — re-dispatching them is grinding). A bare pass_ratio
counts SUCCESS only and once steered a whole day's dispatch at substrate-gated
jobs (2026-06-09); the verdict ladder exists so that never recurs.

Every scoreboard section and headline gate self-reports liveness: a line
reading `⚠ gate-error (…)` means the INSTRUMENT died, not that the surface is
clean — repairing it is itself a station dispatch (highest leverage: a blind
conveyor mis-steers everything downstream).

### 2. Pick highest-leverage pressure, dispatch the equipped station

| Pressure | Station | Autonomy |
|---|---|---|
| ranking stale / next unclear | /converge | free |
| CLAIMED-unverified items | /deliver | free |
| dev-intent uncaptured, scenario drift | /close-loop, /story-harvest | free |
| scope drift | `scope-reconcile.py --apply` | free |
| open dep/sec findings | deprecation-triage dispatch (background) | free |
| `code-red`/`attention` CI verdicts | ci-failure-triage dispatch (background) | free |
| `⚠ gate-error` on any instrument | repair the instrument FIRST | free |
| memory gates firing | /memory-stasis-loop · /memory-ceremony | free |
| OPEN gap-items, READY verdict | **pre-author** the /shift Objective | ceiling fires it |

One station per round for coupled pressures; parallel background dispatches
when independent. Broad goals, not procedures — each station owns its HOW.

Effort (`low`/`medium`/`high`/`xhigh`) is the primary lever for a dispatch's token cost and latency — reach for `low`/`medium` liberally where quality holds, and reserve `xhigh` for the most demanding legs; both the `Agent` tool and workflow `agent()` accept it, complementing (not replacing) the model-tier discipline.

**/deliver economics — bring-up paths are a maintained asset.** Render
verification is the loop's most expensive station: the first proof of a
surface costs a full local-stack bring-up. Hand /deliver the verified ladder
in `hc-dev-orchestrator` (§"Verified bring-up ladder") instead of letting it
rediscover stale binaries, the anchor-gate 404s, and the two-bundle render
topology — and when a /deliver round learns a NEW bring-up fact, it must
write it back to that ladder before closing (the asset is the station's
real output alongside the verdict).

### 3. Re-measure and decide

Loop while the non-stasis count shrinks. Stop when a round makes no progress
OR only ceiling items remain. Then present the **ceiling menu**: 2–5
decisions only the operator can make — pre-authored shift Objectives ready to
fire, pushes awaiting the integrator, env flips, spend commitments,
brainstorm-shaped design questions — each with its evidence attached
(scoreboard line, gap-item id, finding fp). The menu IS the close; never bury
it under work narration.

Match the length of written artifacts — reports, journaled decisions, pre-authored Objectives — to what the task needs: cover the substance, don't pad with filler sections, redundant summaries, or boilerplate.

## Ambition mode — the build leg (explicit launch authorization)

The maintenance loop alone tends the board; it never builds. Under BUILD
authorization each cycle gains a build leg:

```
measure → drain (ONE bounded round over dispatchable verdicts) → BUILD ARC → re-measure → …
```

**The grant is a file, not a vibe.** BUILD mode and the push lease are
SEPARATE explicit grants recorded in `.claude/data/push-lease.json`
(`{"build": bool, "push": bool, "hook_bypass": bool, "granted": "<operator
words verbatim>", "expires": iso}`) — written at launch from the operator's
unambiguous literal grant (echo it back into the file), never inferred from
ambitious-sounding phrasing. **Re-read the file immediately before every
leg fire and every push**: absent/expired/`false` → that authority is
ceiling. The operator revokes at any hour by editing or deleting the file.
Lease conditions unverifiable → build legs SUSPEND (not just pushes), and
the revocation/suspension event leads the morning report's ceiling menu.

**Target selection — depth over breadth, finish over start.** *Arc
continuity first:* an in-flight feature with a live trajectory outranks any
new top rung until delivered (two-render) or stalled-out twice. Otherwise
take the top vision×readiness rung that is: OPEN gap-items (not CLAIMED —
those go to /deliver), env-satisfiable, path-bounded, judge-clean. The
ranking must PRE-DATE launch (or be pinned in the launch prompt) — an
overnight /converge refresh may steer maintenance but never arms a build
leg the operator hasn't seen. No eligible target = skip the leg, note it on
the menu. **Cap: 3 legs per night; two consecutive legs stalled on the same
blocker-class → stop firing legs and menu the blocker.** A post-stall hop
opens a NEW cycle (re-measure first) — one arc per cycle holds.

**The arc has real phases, fired as the agentic-developer discipline
(unattended-kickoff variant — see that skill):** explore (corpus +
`p2p-design-gate` when entities are touched) → **written plan**
(superpowers:writing-plans) → execute under the shift rails. Pre-push
iterations are local-verification-paced, not Jenkins-wait-paced. Composing
the Objective is the loop's job, bounded: its measure must be a **standard
CI-verdict pattern or one the operator saw pre-launch** — never a novel
measure invented mid-loop (the ceiling's judge rule covers editing any
EXISTING judge mid-flight). Budget: size to the night's remaining window
minus a maintenance reserve — never the supervised-shift default. Any
human-visible target ⇒ `Visual gate: on` + kickoff baseline render in the
composed Objective; "logged green" without a rendered witness re-enters the
board as CLAIMED-unverified, which is drift, not progress. The arc inherits
the shift rails **minus** destructive-parameter rebuilds (`RESET_STORAGE`
etc.), env flips, and spend — this loop's ceiling overrides inherited shift
authority. Overnight arcs run on the **durable palette only** (plus the
research toolset: git log, spec-coherence-index, ledger reads — declared
palette-default for build-authorized legs); palette gaps go to the wishlist
or end the leg, never into settings.local.json.

**Branch topology + push lease (delegated integrator, evidence-gated).**
The arc commits on its work branch; the leased push is the local ff-merge to
`dev` followed by literally `git push origin dev` — never `main`, never
tags, never refspecs/deletes/force/amend/rebase/branch-deletes (only dev is
orchestrator-indexed; a work-branch push spawns nothing). ALL of these hold
or the push goes to the menu:
- the push range (`origin/dev..HEAD`) contains ONLY commits authored by this
  loop's legs — any foreign/co-session commit in the range → ceiling;
- targeted local verification green, where "targeted" means AT MINIMUM the
  per-project gates the pre-push hook would select for the diff; hook bypass
  only via its own lease field;
- single-dispatcher verified by named reads: no orchestrator build
  RUNNING/QUEUED (`mcp__jenkins__getJob`), no foreign shift journal in
  `.claude/shifts/` with writer-relative freshness < 30 min;
- a commit embodying an *interpretive decision* (see research-before-bail)
  is lease-pushable only when its journaled evidence includes a
  directly-governing spec/plan sentence — otherwise commit-only, menu it;
- post-push the orchestrator SPAWN is confirmed; not spawned within ~10 min
  → ONE tagged empty-commit retrigger, then no further pushes this loop and
  the evidence goes on the menu;
- the wave is WATCHED (its window runs maintenance rounds, or sleeps with
  wakeup ≈ observed wave time — an idle watched wave is correct, not a
  stall; legs serialize on wave returns by design, expect ~⌊night/wave⌋);
  **default attribution**: a new code-red on any job your push touched is
  YOURS unless evidence excludes it — one fix attempt, then revert.
  "Fixed" = the fix pushed AND its own watched wave returned non-code-red
  (the disappearance sweep keeps final closure).

**Research-before-bail.** Overnight there is no operator to answer — a bail
on an answerable question wastes the night. The gate (full text:
agentic-developer principle 3) stands in front of EVERY bail shape —
question-, blocker-, or stall-shaped — and scopes to *leg-blocking*
questions: sub-questions discovered during a research pass inherit that
pass, and principle-8 residue capture is explicitly exempt (capturing is
cheap by design). One research pass per question per leg — re-encountering
it cites the prior pass. An evidence TIE counts as corpus-dry for that
question: take the more reversible reading and journal the tie.

**Stall + dawn rules.** Build-leg progress = arc-phase advancement (plan
written, scoped diff staged, targeted local gates green) OR
Objective-measure delta OR a journaled interpretive decision — the 2-count
starts only once EXECUTE begins, and while the arc is live the shift's own
judge table governs its iterations; this stall rule judges the arc as a
whole. Stall → capture the trajectory (journal + backlog/
Objective-candidate), fall back to maintenance, then next ranked target
(new cycle) or close. The final report leads with: what was DELIVERED
(rendered witness) · what was pushed + its CI verdicts · **every
interpretive decision taken on standing evidence (with its falsifier)** ·
THEN the ceiling menu — ambition never excuses a dump.

**Stop-rule under build authorization:** the §3 stop test applies only
AFTER the cycle's build leg — a ceiling-only maintenance board is the GO
condition for a leg, not a stop; arc advancement counts as round progress.

## Ceiling (never dispatched by this loop)

env flips (`scope-reconcile --set`) · spend commitments (remote routines,
ultra reviews) · editing any EXISTING judge (a live shift's measure,
fixtures, oracle files) · vision/theme choices and RE-ranking ·
brainstorm-shaped design questions (route to /brainstorm WITH the operator,
not around them) · destructive ops (DNA reinstall, data wipes, destructive
parameterized rebuilds, force-push, remote deletes, tag pushes) ·
instrument SEMANTICS (a `⚠ gate-error` repair restores liveness only —
crash → runs, before/after output journaled; changing thresholds, verdict
classes, or counting logic is ceiling). /shift kickoffs and `git push` are
ceiling **by default** — they move under the loop's authority ONLY through
the lease file's explicit `build`/`push` grants (above), and revert to
ceiling the moment the file or its conditions can't be verified.

## Wiring

- **Ceremony**: invoke at a boundary; rounds until ceiling-only.
- **/loop self-paced**: overnight-capable; the ceiling menu accumulates in
  the final report for the morning.
- **SessionStart advisory** (`.claude/hooks/delivery-gate.py`): a one-line
  planning feed only — it surfaces, it never redirects. The pilot's subject
  wins unless a listed item is a BLOCKER to it.

## Hard rules

- Commit-only; the integrator pushes.
- The ceiling menu is never empty-by-omission: if you couldn't drain
  something, it's either on the menu or in a station's live trajectory —
  an item dropped between is the dump this loop exists to prevent.
- Don't grind a station that didn't move: one re-dispatch with sharpened
  evidence, then escalate to the menu.
- This loop never edits judges, never re-ranks vision (that's /converge's
  craft under the cartographer), never bypasses a gate.
- **Instrument liveness**: a gate that errors must surface as `⚠ gate-error`;
  a gate line that silently VANISHES from the headline is itself a finding
  (the scope-gate went dark for days in 2026-06 and read as "nothing to
  report"). Trust no scoreboard whose instruments you haven't seen breathe.
- Dispatch by VERDICT class, never by bare ratio: env-gated red is ceiling
  work, fix-landed/clearing red is the harvester's wait state — grinding
  either burns rounds the dispatchable surface needed.
