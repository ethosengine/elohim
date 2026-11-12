---
name: deprecation-stasis
description: Drive the deprecation/security-concern discipline to stasis in one loop — reconcile the deterministic ledger (.claude/data/deprecations.jsonl) against the canonical backlog (genesis/data/timeline/backlog/{deprecation,security}-*.md), dispatch deprecation-triage for the highest-leverage open item, re-check blocked items whose blockers may have cleared, re-measure, repeat until stasis. Sibling of /memory-stasis-loop (same measure→dispatch→re-measure shape). NOT for CI findings — that class closes deterministically in ci-harvest (disappearance/reopen) and drains under the agentic-developer loop's floor/ceiling rails. Use when "drain the deprecation ledger", "deprecation stasis pass", on a /loop or scheduled routine, or when the sentinel's nudges have accumulated past comfort.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/deprecation-stasis"
---

# Deprecation Stasis Sweep

The capture side is deterministic and always-on (the `deprecation-sentinel`
PostToolUse hook: fingerprint dedupe → ledger; NEW → background
`deprecation-triage` dispatch; re-encounter → backlog citation, no re-fire).
This sweep is the **deliberate drain**: it walks both stores to stasis the way
`/memory-stasis-loop` walks the memory disciplines.

**Stasis** additionally requires no actionable Dependabot package cluster;
unpatchable or blocked clusters retain their documented blockers and stay open
until the harvester observes closure. For the warning ledger, stasis := it is EMPTY, or contains only `blocked` entries with a
still-valid, documented blocker in their canonical backlog entry. Fixed items
DECOMPOSE at close (ledger line + backlog entry deleted; the verifying commit
is the record; rare chronicle graduation) — everything in the backlog has a
live trajectory or a status, or it's not there. Terminal tombstones in either
store are themselves an incoherence to repair.

## The loop

Each round:

### 1. Measure (scoreboard)

Read `python3 .claude/scripts/dependabot-harvest.py --clusters` FIRST. Its live
ledger is `.claude/data/dependabot-findings.jsonl`, separate from the warning
ledger below. Rank package clusters by critical alert count first, then total
alert count. Dispatch ONE remediation per ecosystem/package cluster across
ALL affected lockfiles, not one task per advisory or manifest. For Cargo use
`cargo update -p <package> --precise <patched-version>` in each affected
workspace; for pnpm mirror root `package.json` `pnpm.overrides` into
`pnpm-workspace.yaml` and regenerate the affected lockfiles. Test each owning
workspace. Unpatchable clusters need a documented mitigation/blocker.

Dependabot closure is measured by disappearance in the NEXT successful harvest,
never by a fix commit or a local lockfile check. Do not delete findings or mark
them closed yourself. The following warning-ledger coherence rules do not own
Dependabot's live-set reconciliation or append-only closure history.

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
1. **Dependabot package clusters** — critical count first, then alert count;
   one remediation spans all of the cluster’s lockfiles.
2. **Ledger↔backlog incoherence** (deterministic layer is lying) — repair
   directly.
3. **`open` entries** (captured, never triaged — a dispatch was lost or
   declined) — dispatch `deprecation-triage` with those fingerprints.
4. **`blocked` entries whose blocker may have cleared** — read the backlog
   entry's "Current decision"; check the blocker empirically (package
   version now available? substrate capability flipped in
   genesis/manifests/cluster-state.yaml? upstream issue closed — WebFetch
   it). Cleared → dispatch `deprecation-triage` to re-attempt; still
   blocked → refresh the decision line with today's evidence and move on.
5. **`triaged` entries older than a week** (fix in flight that never
   landed) — dispatch to finish or demote to `blocked` with the reason.

Dispatch one `deprecation-triage` (Opus) at a time for focused concerns, or
fan out in parallel when concerns are independent (different packages,
different projects).

> **CI findings are NOT this sweep's class.** That ledger
> (`.claude/data/ci-findings.jsonl`) closes deterministically inside
> `ci-harvest.py` (confirmation-by-disappearance via green streaks;
> recurrence-reopen) and its open items drain under the
> **agentic-developer loop's CI findings rails** (floor: pass/unstable/fail
> ratios; ceiling: brainstorming-confidence) — see
> `.claude/skills/agentic-developer/SKILL.md` and the findings-sentinel
> pattern spec §3.3.

### 3. Re-measure and decide

Re-run the Dependabot harvest and the warning scoreboard. A fix still present
in the next harvest remains open; only the harvester records its closure. Loop while the non-stasis count is shrinking. Stop at
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
