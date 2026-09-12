---
name: feedback_tiered_agent_capabilities_destructive_git
title: Tiered agent capabilities — haiku/sonnet never git-reset without a team check
id: feedback-tiered-agent-capabilities-destructive-git
description: "Operator 2026-09-11 after a Haiku hard-reset of shared dev: destructive git needs a declared tier (opus+) or a team check; unknown = deny; gate = PreToolUse hook on the policies.yaml row."
metadata:
  type: feedback
---

**What happened (2026-09-11 22:34Z):** a Haiku subagent of another session ran `git reset --hard 466536cb4`
on the shared `/projects/elohim` checkout: three commits left the `dev` pointer (objects intact) and ~27
tracked files' uncommitted work across several lanes was wiped. Recovery took two sessions an hour.

**Operator direction:** "haiku and sonnet should not have git reset level capabilities to do that kind of
thing without checking with the team first." Tiered permissions for agent capabilities belong to the
harness, keyed by the same `agent:<role>@<model>` vocabulary the reader lens uses.

**How to apply:** the gate is `.claude/hooks/capability-tier-gate.py` (PreToolUse on Bash) reading the
declared table `destructive-git-requires-tier@1` in `.claude/epr-meta/policies.yaml` (patterns, tier
order, tier floor, unknown = deny, remedy text). When dispatching a haiku/sonnet subagent, never brief it
to run destructive git; when YOU are the controller, do destructive git yourself after checking the shared
tree's other lanes. Worktrees per plan (`.claude/worktrees/<plan>`) are the structural half of this
discipline — a hard reset in a worktree cannot wipe another lane. Related:
[[feedback_push_branch_discipline]], [[feedback_agent_fleet_and_harness]], [[project_recall_reaches_authority_habit]].
