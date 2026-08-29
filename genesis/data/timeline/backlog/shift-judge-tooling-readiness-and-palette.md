---
id: "backlog-shift-judge-tooling-readiness-and-palette"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Shift judge tooling trips on itself — readiness git-clean fails on hook-owned ledgers; palette matcher `*` cannot match path arguments"
slug: "shift-judge-tooling-readiness-and-palette"
written: "2026-08-29"
author: "shift 2026-08-29T22-16-edge-lands-bdd5f9ef (kickoff)"
status: "open"
priority: "medium"
jobs: []
cluster: "agentic-developer-loop"
relatedNodeIds:
  - "spec:2026-04-16-agentic-developer-loop-design"
tags: [agentic, shift, readiness, palette, tooling]
---

## Measured (2026-08-29 22:2xZ, live session, Che sandbox)

- `pnpm run agentic:readiness` → `ready:false` on `git status --porcelain --ignore-submodules=all` because of
  `.claude/data/ci-cursor.json` (written by `ci-harvest.py` — a tool the skill mandates at Ground),
  `.claude/data/{deprecations,governance-findings}.jsonl` (sentinel hooks) and two `.claude/memory/*.md`
  (memory hooks). All insertions-only, none committed by any shift. In a live session the check is unsatisfiable.
- Also first failed with `fatal: detected dubious ownership` — the sandbox shell is uid 0, the repo is owned by
  1234; readiness spawns `git` without `safe.directory`. Workaround: `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0=/projects/elohim`.
- `matchesPalette('python3 .claude/shifts/x.py', ['Bash(python3 *)'])` → false: `toGlob` hands picomatch `python3 *`,
  and `*` does not cross `/`. Every scripted command with a path argument reads GAP against an entry that allows it.

## Current decision

Fix both in `genesis/agentic/` (out of the landing shift's scope): (1) readiness ignores hook-owned ledger paths
(`.claude/data/**`, `.claude/memory/**`) and passes `safe.directory` through; (2) `toGlob` maps a bare trailing ` *`
to ` **` (or picomatch `{ bash: true }`) so path arguments match. Verify against the 119-entry local palette with
the commands a shift actually runs (measure script, ci-harvest, curl to Jenkins, git -c safe.directory).
