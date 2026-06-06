---
name: deprecation-stasis
description: Drive the deprecation/security-concern discipline to stasis in one loop — reconcile the deterministic ledger (.claude/data/deprecations.jsonl) against the canonical backlog (genesis/data/timeline/backlog/deprecation-*.md), dispatch deprecation-triage for the highest-leverage open item, re-check blocked items whose blockers may have cleared, re-measure, repeat until stasis. Sibling of /memory-stasis-loop (same measure→dispatch→re-measure shape). Use when "drain the deprecation ledger", "deprecation stasis pass", on a /loop or scheduled routine, or when the sentinel's nudges have accumulated past comfort.
---

# Deprecation Stasis Sweep

The capture side is deterministic and always-on (the `deprecation-sentinel`
PostToolUse hook: fingerprint dedupe → ledger; NEW → background
`deprecation-triage` dispatch; re-encounter → backlog citation, no re-fire).
This sweep is the **deliberate drain**: it walks both stores to stasis the way
`/memory-stasis-loop` walks the memory disciplines.

**Stasis** := the ledger is EMPTY, or contains only `blocked` entries with a
still-valid, documented blocker in their canonical backlog entry. Fixed items
DECOMPOSE at close (ledger line + backlog entry deleted; the verifying commit
is the record; rare chronicle graduation) — everything in the backlog has a
live trajectory or a status, or it's not there. Terminal tombstones in either
store are themselves an incoherence to repair.

## The loop

Each round:

### 1. Measure (scoreboard)

```bash
python3 - <<'EOF'
import json, collections
counts = collections.Counter()
entries = []
for l in open('.claude/data/deprecations.jsonl'):
    e = json.loads(l); counts[e.get('status','open')] += 1; entries.append(e)
print(dict(counts) or 'EMPTY — stasis (capture side)')
for e in entries:
    print(e['fp'], e.get('status'), e.get('backlog','(no backlog)'), '-', e['line'][:90])
EOF
ls genesis/data/timeline/backlog/deprecation-*.md 2>/dev/null
```

Cross-check coherence: every non-`open` ledger entry must point at a backlog
file whose frontmatter `fingerprints:` includes it and whose
`deprecation_status:` agrees with the ledger status (domain axis, live states
only). The entry's `status:` field carries the unified delivery gradient
(`backlog`/`wip` — captured-or-blocked / fix-in-flight) so the shared
`delivery-status-distribution.py` projection and /converge rank deprecation
concerns like every other backlog item. Any `fixed`/`stable` tombstone parked
in either store is a missed decomposition — close it out (delete; chronicle
first only if genuinely meaningful). Incoherence = the first thing to fix (it
breaks the deterministic citation).

### 2. Pick highest-leverage work

Priority order:
1. **Ledger↔backlog incoherence** (deterministic layer is lying) — repair
   directly.
2. **`open` entries** (captured, never triaged — a dispatch was lost or
   declined) — dispatch `deprecation-triage` with those fingerprints.
3. **`blocked` entries whose blocker may have cleared** — read the backlog
   entry's "Current decision"; check the blocker empirically (package
   version now available? substrate capability flipped in
   genesis/manifests/cluster-state.yaml? upstream issue closed — WebFetch
   it). Cleared → dispatch `deprecation-triage` to re-attempt; still
   blocked → refresh the decision line with today's evidence and move on.
4. **`triaged` entries older than a week** (fix in flight that never
   landed) — dispatch to finish or demote to `blocked` with the reason.

Dispatch one `deprecation-triage` (Opus) at a time for focused concerns, or
fan out in parallel when concerns are independent (different packages,
different projects).

### 3. Re-measure and decide

Re-run the scoreboard. Loop while the non-stasis count is shrinking. Stop at
stasis, or when a round makes no progress (then report exactly what is stuck
and why — a stuck item usually means the concern needs an operator-initiated
sprint, which is a `vision-readiness` roadmap conversation, not more agent
rounds).

## Wiring options

- **Manual**: invoke this skill at any boundary.
- **Looped**: `/loop /deprecation-stasis` self-paced until stasis.
- **Routine**: a `/schedule` weekly remote agent invoking this skill —
  skips cheaply when the scoreboard is already at stasis.

## Hard rules

- Never edit ledger statuses by hand here without also reconciling the
  backlog entry (and vice versa) — coherence of the two stores is what lets
  the sentinel answer re-encounters deterministically.
- Commit-only; the integrator pushes.
- A `blocked` verdict with a documented, verified blocker is SUCCESS, not
  failure — the goal is stasis, not zero entries.
