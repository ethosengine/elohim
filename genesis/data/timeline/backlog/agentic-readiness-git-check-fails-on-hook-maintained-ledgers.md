---
id: "backlog-agentic-readiness-git-check-fails-on-hook-maintained-ledgers"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "agentic:readiness git check cannot pass — the skill's own instructions dirty the files it gates on"
slug: "agentic-readiness-git-check-fails-on-hook-maintained-ledgers"
written: "2026-08-20"
author: "claude (land-latency-instrument-ratchet-saga shift)"
status: "backlog"
priority: "medium"
jobs: []
tags: [agentic-developer, readiness, shift, hooks, ci-harvest, tooling-contradiction]
cites:
  - .claude/skills/agentic-developer/SKILL.md
  - genesis/agentic/readiness.mjs
  - .claude/scripts/ci-harvest.py
---

# The readiness gate fails by construction

`pnpm run agentic:readiness` refuses to start a shift when the working tree has
uncommitted changes. In this repo that condition is **permanently true**, because
several tracked files are runtime state written by the hook layer on every
session:

```
M .claude/data/ci-cursor.json            # ci-harvest.py
M .claude/data/ci-findings.jsonl         # ci-harvest.py
M .claude/data/deprecations.jsonl        # deprecation sentinel
M .claude/data/governance-findings.jsonl # governance sentinel
M .claude/subject-focus.md               # session hook
```

The contradiction is internal to the discipline, not incidental: the
agentic-developer skill's own "CI findings rails" section instructs the shift to
run `ci-harvest.py` **at Ground (step 1) and after each Measure (step 5)** — and
that command writes `ci-findings.jsonl` and `ci-cursor.json`. Following the skill
as written guarantees failing its own precondition. A gate that cannot pass
teaches agents to bypass gates, which is worse than not having it.

Observed 2026-08-20 on the `land-latency-instrument-ratchet-saga` shift: measure
check green (baseline 8), git check red on exactly the five files above plus two
other sessions' in-flight memory edits. Nothing dirty was in the objective's
scope.

## Fix direction

The check wants to prevent a shift from starting on top of *unrelated
uncommitted source work* it might clobber or mis-attribute. That intent is right;
the implementation is too broad. Narrow it to the question actually being asked:

- Ignore a declared set of hook-maintained runtime paths (`.claude/data/**`,
  `.claude/subject-focus.md`, the generated `.claude/memory/MEMORY.md`), the same
  way `.gitignore` would if these were not tracked for other reasons.
- Better: gate on **scope intersection** rather than tree cleanliness — fail only
  when a dirty path matches `objective.scope.paths`, which is the case that can
  actually cause clobbering or false attribution. A dirty file the shift may not
  edit is not a hazard.
- Consider whether these ledgers should be tracked at all. They are per-checkout
  runtime state; `flows.jsonl` and the reports tree are already gitignored for
  the same reason.

## Why this is filed rather than fixed here

`genesis/agentic/**` is outside this shift's scope, and changing a readiness gate
mid-shift edits the machinery that judges the shift. Captured per principle 8 so
it has an owned home instead of being worked around silently a third time.
