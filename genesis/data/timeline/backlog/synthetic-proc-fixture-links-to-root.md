---
id: "backlog-synthetic-proc-fixture-links-to-root"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A cleanup-guard probe left fake /proc trees whose root links point at /, and the editor's search follows them"
slug: "synthetic-proc-fixture-links-to-root"
written: "2026-10-30"
author: "claude-opus-5-5"
status: "open"
priority: "medium"
area: "agentic evidence fixtures / workspace hygiene"
domain: "D5"
tags: [fixtures, symlinks, editor, wall-clock, cleanup]
---

# Synthetic /proc fixtures link to /

The 2026-10-23 habit-cleanup-guard probes (shift `autonomous-delivery-20261022`, evidence
dirs `habit-cleanup-guard-adaptation-20261023T122500Z` and
`habit-cleanup-guard-v2-20261023T124000Z`) built fake process trees, `synthetic-*/<pid>/root`,
pointing at `/` the way a real `/proc/<pid>/root` does. 148 such links remain under
`.worktrees/autonomous-delivery-20261022/.claude/shifts/…/evidence/`.

The editor's workspace `package.json` discovery runs
`rg --files --no-ignore --follow -g **/package.json` from `/projects/elohim`. `--no-ignore`
takes it into `.worktrees/`, and `--follow` sends each link back over the whole filesystem,
`/proc`, `/sys` and the cargo pools included. On 2026-10-30 one run held about 4.5 cores
for over 45 minutes during an integration pre-push.

| Row | What | State |
|---|---|---|
| 1 | The probe that builds a synthetic `/proc` points each `root` inside its own fixture dir (or at a missing path) and removes the tree when it finishes; evidence keeps the results, not the scaffold | open |
| 2 | Remove the 148 leftover `synthetic-*` trees from the two evidence dirs | open: awaiting the steward's word |
| 3 | `.vscode/settings.json` sets `search.followSymlinks: false` and excludes `.worktrees/**` and `.cargo-pool/**` from search | open: awaiting the steward's word |

## Done when

- No symlink under `/projects/elohim` resolves to `/`.
- The editor's file discovery cannot leave the repository through a symlink.
