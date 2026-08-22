---
index: false
name: feedback_work_stays_in_operator_visible_tree
title: Work stays in the operator-visible tree
description: "All work lands in /projects/elohim (the operator's VS Code mount); never create sibling worktrees like /projects/elohim-wt-land — invisible work is unreviewable."
metadata: 
  node_type: memory
  title: "Work stays in /projects/elohim — the operator's visible tree"
  type: feedback
  originSessionId: d3b11772-7c9f-4ea6-85dc-f9e1c2d68fc8
  modified: 2026-07-25T16:02:56.398Z
---

Operator directive 2026-07-25: **everything stays in `/projects/elohim`.** A prior session created a
sibling worktree `/projects/elohim-wt-land` (checked out on `dev`) and integrated + pushed from there.
The operator's VS Code mounts `/projects/elohim` only, so that work — commits, gate output, the whole
integration — was **invisible to them** without mounting another directory, which they do not want to do.

**Why:** review is a first-class part of delivery here, not a formality. Work the operator cannot see is
work they cannot review, and an agent silently relocating the integration surface removes them from the
loop without ever saying so. The cost isn't tidiness — it's that the human stops being able to check the
machine.

**How to apply:**
- Do **not** `git worktree add` a sibling of the repo. If isolation is genuinely needed, worktrees belong
  under `/projects/elohim/.claude/worktrees/` (where `cargo-pool steward`/`orphans` already tracks and
  reclaims them) — inside the visible mount.
- Integrate and push from `/projects/elohim` itself. When the shared tree is mid-edit by a concurrent
  session, the answer is **path-limited commits** ([[feedback_concurrent_sessions_shared_worktree]]),
  not a second working tree.
- `git push origin HEAD:dev` pushes commits, not the working tree — a dirty tree from co-resident
  sessions is never a reason to relocate.

**This cannot be enforced by `.epr-meta`** (checked 2026-07-25 against `.claude/scripts/_lib/epr_meta.py`):
`_matches_when` does `name = Path(write["path"]).name` — the `when:` glob matches the **basename only**,
so no rule can test an absolute path. A worktree also carries its own identical copies of every
`.epr-meta`, so the cascade cannot tell one tree from the other. Authoring a predicate-less rule to fake
it would be *malformed*, and strict-but-recoverable then downgrades every write in the subtree to `ask` —
strictly worse than no rule. The guard is this memory plus removing the stray worktree, not a gate.

Related: [[feedback_commit_only_integrator_pushes]] (when NOT holding push authority),
[[feedback_concurrent_sessions_shared_worktree]] (the real tool for a shared dirty tree).
