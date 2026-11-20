---
name: feedback-haiku-wraps-codex-in-workflows
title: "Haiku wraps Codex in workflows"
description: "Operator 2026-10-28: in workflows a Haiku agent runs codex exec for reading/implementation legs and returns its result file; it never runs git."
id: feedback-haiku-wraps-codex-in-workflows
metadata:
  node_type: memory
  title: Haiku wraps Codex in workflows
  type: feedback
  originSessionId: a60b1c22-d9df-413d-8848-0321540e049e
  modified: 2026-10-28T17:52:37.627Z
cites:
  - .claude/workflows/
---

In Workflow scripts, a cheap Haiku agent (`model: 'haiku'`) can wrap a `codex exec` call: the
Haiku agent assembles the prompt file, runs codex with the right flags, waits, and returns the
`-o` result file verbatim. Codex (gpt-5.6-sol for implementation, gpt-6-astra for hard reviews)
does the heavy reading or writing; the top model keeps judgment, synthesis and review.

**Why:** operator reminder 2026-10-28 ("remember you can use haiku to wrap codex in your
workflows"), on top of the standing ruling to use Codex wherever possible to cut token spend.
Default-model agents doing their own file reading cost far more for the same legwork.

**How to apply:**
- The wrapper's brief names the exact command to run and says: run no git commands, edit no
  files yourself, return the result file's contents. A Haiku agent once executed a hypothetical
  `git cherry-pick` + `reset --hard` in the shared tree ([[feedback_agent_fleet_and_harness]]).
- Write legs run in their own worktree (`-s workspace-write -C <worktree>`), with the /tmp
  guards from [[reference-codex-model-ids]]; codex never runs cargo, the orchestrator builds.
- Keep judgment stages (decision records, adversarial review, planning) on the session model.
- **Outside a Workflow, skip the wrapper** (operator 2026-10-29: "ultraC isn't on, so you probably
  can just dispatch codex directly"): run `codex exec` yourself from Bash (background, `-o` result
  file) for reading legs such as cite checks, prior-art reads and readability edits, instead of
  spawning default-model agents to do the reading.
