---
name: deprecation-stasis
description: Drive the findings-sentinel disciplines to stasis in one loop — BOTH ledgers (.claude/data/deprecations.jsonl for deprecation/security, .claude/data/ci-findings.jsonl for CI failures) reconciled against the canonical backlog (genesis/data/timeline/backlog/{deprecation,security,ci}-*.md), dispatching deprecation-triage / ci-failure-triage for the highest-leverage open items, re-checking blocked items whose blockers may have cleared, confirming CI closures by fingerprint disappearance, re-measuring until stasis. Sibling of /memory-stasis-loop (same measure→dispatch→re-measure shape). Use when "drain the deprecation ledger", "drain the CI findings", "deprecation stasis pass", "findings stasis", on a /loop or scheduled routine, or when sentinel/harvest nudges have accumulated past comfort.
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
ls genesis/data/timeline/backlog/deprecation-*.md genesis/data/timeline/backlog/security-*.md 2>/dev/null
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

### 1b. CI findings ledger (instantiation B — same loop, three asymmetries)

Run `python3 .claude/scripts/ci-harvest.py` first (fresh evidence), then
scoreboard `.claude/data/ci-findings.jsonl` the same way. The CI class
differs in three ways:

1. **Dispatch target** is `ci-failure-triage` (it composes the read-only
   ci-observer/ci-investigator analysts; museum gate before novel root
   causes).
2. **Closure is by disappearance, and the SWEEP owns it** — the computable
   rule (build-number arithmetic lies across aborted-build gaps): an entry
   with `status: triaged` is a CONFIRMED fix when
   `cursor.green_streak.<job> >= 3` (the harvester's consecutive-green
   counter) AND `last_build <= triaged_at_build` (no recurrence since the
   fix — `triaged_at_build` is stamped by ci-failure-triage when it sets
   triaged). Confirmed → decompose here (delete ledger line + backlog entry;
   graduate genuinely-recurring lessons to the anti-patterns museum record,
   not chronicle). A `triaged` entry with `last_build > triaged_at_build`
   RECURRED — the fix didn't take: set it back to `open` and re-dispatch.
3. **Flake evidence is deterministic**: `seen` count + `first_build..
   last_build` spread (harvester-owned) + `getFlakyFailures` cross-check —
   read it before any verdict; never re-derive what the ledger already
   counted.

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
