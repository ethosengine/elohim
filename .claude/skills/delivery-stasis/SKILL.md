---
name: delivery-stasis
description: Drive the WHOLE development cycle to stasis against the developer docs in one loop — the operator-as-conveyor role, formalized. Each round reads the full delivery scoreboard (placement ledger orphan-OPEN/CLAIMED-unverified, delivery-status distribution, CI ratio windows + findings ledgers, scope/coverage/roadmap currency), dispatches the equipped station for the highest-leverage pressure (/converge ranking, /deliver verification, /close-loop capture, scope-reconcile, memory loops; pre-authors /shift Objectives), re-measures, repeats until only ceiling items remain — then presents the ceiling menu (the 2-5 decisions only the operator can make: shift kickoffs, pushes, env flips, spend, judge changes). Third instantiation of the stasis-loop shape (spec: delivery-stasis-loop-design). Use when "what should we do", "drive the dev cycle to stasis", "delivery stasis pass", "run the conveyor", at session boundaries, or /loop'd overnight. NOT for a single station's work (invoke that station directly) or memory-only hygiene (use /memory-stasis-loop).
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
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | head -25
python3 .claude/scripts/memory-kit/delivery-status-distribution.py >/dev/null && \
  python3 -c "import json; d=json.load(open('.claude/memory-kit/delivery-status-distribution.json')); print(json.dumps({k:v for k,v in d.items() if k in ('summary','anomalies','counts')} or d, default=str)[:600])"
python3 .claude/scripts/ci-harvest.py            # fresh CI evidence + ratios
python3 -c "
import json, os
for led in ('.claude/data/ci-findings.jsonl', '.claude/data/deprecations.jsonl'):
    n = sum(1 for _ in open(led)) if os.path.exists(led) else 0
    print(led.split('/')[-1], n)
cur = json.load(open('.claude/data/ci-cursor.json'))
for j, w in sorted(cur.get('recent', {}).items()):
    if w: print(j, 'pass_ratio', round(w.count('SUCCESS')/len(w), 2), f'({len(w)}w)')"
```

Plus the SessionStart headline gates you already have in context (cleanup /
scope / decompose / review / path / roadmap).

### 2. Pick highest-leverage pressure, dispatch the equipped station

| Pressure | Station | Autonomy |
|---|---|---|
| ranking stale / next unclear | /converge | free |
| CLAIMED-unverified items | /deliver | free |
| dev-intent uncaptured, scenario drift | /close-loop, /story-harvest | free |
| scope drift | `scope-reconcile.py --apply` | free |
| open dep/sec findings | deprecation-triage dispatch (background) | free |
| open CI findings | ci-failure-triage dispatch (background) | free |
| memory gates firing | /memory-stasis-loop · /memory-ceremony | free |
| OPEN gap-items, READY verdict | **pre-author** the /shift Objective | ceiling fires it |

One station per round for coupled pressures; parallel background dispatches
when independent. Broad goals, not procedures — each station owns its HOW.

### 3. Re-measure and decide

Loop while the non-stasis count shrinks. Stop when a round makes no progress
OR only ceiling items remain. Then present the **ceiling menu**: 2–5
decisions only the operator can make — pre-authored shift Objectives ready to
fire, pushes awaiting the integrator, env flips, spend commitments,
brainstorm-shaped design questions — each with its evidence attached
(scoreboard line, gap-item id, finding fp). The menu IS the close; never bury
it under work narration.

## Ceiling (never dispatched by this loop)

/shift kickoffs (pre-author only) · any `git push` · env flips
(`scope-reconcile --set`) · spend commitments (remote routines, ultra
reviews) · anything touching a judge (Objectives, measure commands, fixtures)
· vision/theme choices · brainstorm-shaped design questions (route to
/brainstorm WITH the operator, not around them).

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
