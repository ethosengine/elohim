---
index: false
name: feedback-agents-commit-ledger-files
title: "Agent commits drag in hook-maintained ledgers"
description: "Worktree agents commit .claude/data/*.jsonl ledgers the hooks rewrite; the index patch lands but the working-tree sync fails — amend the ledger out, apply --exclude '.claude/*'"
metadata: 
  node_type: memory
  title: Agent commits drag in hook-maintained ledgers
  type: feedback
  originSessionId: dcddc033-024b-4dfa-8d13-39fa5f75b9ef
  modified: 2026-09-13T05:40:38.428Z
---

Twice on 2026-09-13 an agent's worktree commit (`git add -A`-style) carried `.claude/data/governance-findings.jsonl`, a hook-maintained ledger that is always dirty in the shared tree. The index-patch landing succeeds (`git apply --cached` sees a clean index), but the working-tree sync fails on that file, so the source files silently stay behind HEAD in the shared checkout — and a build from the shared tree ships the OLD code.

**Why:** the ledgers are rewritten by PostToolUse hooks in every worktree; agents see them as changes and commit them.

**How to apply:** tell agents to commit by pathspec (never `-A`); when landing, check `git show --stat` for `.claude/data/`, reverse-apply that hunk to the index (`git show HEAD -- <ledger> | git apply --cached -R && git commit --amend --no-edit`), then sync the working tree with `git apply --exclude='.claude/*' <patch>` and verify `git diff --quiet HEAD -- <crate>/src` before any build from the shared tree. Related: [[feedback_push_branch_discipline]], [[feedback_tiered_agent_capabilities_destructive_git]].
