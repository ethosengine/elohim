---
name: project_shem_workspace_runs_long_sprints
title: "Shem runs the long sprints; this workspace lags — pull first"
description: "The shem workspace runs the 40 h orchestrated sprints and this Che workspace lags origin/dev by days; pull first, resolve toward shem, back up local hook ledgers."
id: project-shem-workspace-runs-long-sprints
metadata:
  node_type: memory
  title: Shem runs the long sprints; this workspace lags — pull first
  type: project
  originSessionId: 09892ed7-62ff-451d-a4cc-7871ccfa5ccb
  modified: 2026-10-18T02:10:01.318Z
cites:
  - .claude/scripts/habits-project.py
---

Two workspaces work the same `dev`: **shem** (the operator's T7610 box, see
[[reference_node_hardware_ethosengine_shem]]) runs the long orchestrated sprints
(the 2026-10-12..17 performance-backlog sprint: 242 commits, ~40 h; it crashed with
~15 h left), while this Che workspace carries only small local arcs and falls days
behind.

**Why:** on 2026-10-18 the pull brought 242 commits against 11 local ones; the
operator said this workspace "might be a bit stale because shem has been working on a
lot of performance oriented backlog". Conflicts were all shared registers
(CLUSTERS.md, a backlog table, habits.yaml) plus hook-written ledgers under
`.claude/data/`.

**How to apply:** after a workspace restart run `git config --global --add
safe.directory '*'` first (HOME is wiped). Stash local dirt, merge origin/dev,
take shem's side on shared registers, re-project `habits.yaml` from the atoms
(`habits-project.py`; its `--check` reports FLIP-WITHOUT-RULING mid-merge and clears
after the commit), then restore the stash and take the merged version of
`.claude/data/*.jsonl` and cursors, backing this workspace's copies up under
`genesis/local-dev/pre-pull-<date>/`. `git stash drop` is denied to agents by the
destructive-git hook; leave the stash for the operator. Shem's conductors ran on
`hc-fork-96bcd2bee215-proof`, which this workspace's tools dir does not have.
Related: [[feedback_push_branch_discipline]], [[project_devspace_recovery]].
