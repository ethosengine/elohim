---
name: feedback_subagent_no_git_stash
description: "In subagent-driven sprints, forbid subagents from git stash / checkout <ref> -- entirely; they entangle with the operator's pre-existing stashes and pollute the shared working tree."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 8d8e1025-ab7f-4f41-a41f-7236abc36867
cites:
  - .claude/skills/agentic-developer/SKILL.md
---

During the 2026-05-28 mutual-storage-replication subagent-driven sprint, a Task-4 implementer subagent ran `git stash` / `git stash pop` / `git checkout stash@{N} -- <files>` to test whether a compile error pre-dated the sprint. `git stash pop` conflicted on a generated dist artifact and the recovery **materialized foreign content from the operator's pre-existing stashes into the shared working tree** (a parallel memory-coherence feature: `doorway/.../storage_events_subscriber.rs`, `lamad-spa.json`, `.claude/memory-kit/*`, etc.). No data was lost (all 13 stashes stayed intact; every task commit was verified to contain only its intended files), but the operator's working tree was disturbed and could only be safely cleaned by the operator.

**Why:** The dev environment shares ONE working tree across the orchestrator's subagents AND the operator's parallel sessions, which carry a deep stash stack. Any `git stash`/`git checkout <ref> --`/`reset --hard` by a subagent can pop/apply the wrong stash or strip uncommitted operator WIP.

**How to apply:**
- Every subagent dispatch prompt MUST forbid: `git stash`, `git stash pop`, `git checkout <ref> -- <path>`, `git reset --hard`, `git revert`, branch switching. Tell them: if they think they need any of these, STOP and report BLOCKED to the orchestrator.
- To answer "is this failure pre-existing?", the ORCHESTRATOR checks (`git show origin/dev:<file>`, `git log`, diffing against a ref) — never a subagent stashing the live tree. (This sprint resolved several "is dev already broken?" questions exactly this way.)
- Always use targeted `git add <explicit paths>` in subagents, never `-A`/`.`/`-u` (see [[feedback_commit_attribution_parallel_agent_leak]]). Verify each commit's `--stat` for foreign-file leak before accepting the task.
- Expect interleaved parallel-operator commits on the same branch; each side's commits stay atomic if everyone uses targeted adds. Surface the interleaving in close-out rather than trying to untangle it.

Related: [[feedback_subagent_scope_guardrails]] (forbid git revert/reset in subagents), [[feedback_subagent_dep_conflict_supervision]].
