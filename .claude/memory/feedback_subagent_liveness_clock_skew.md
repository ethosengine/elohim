---
name: subagent-liveness-clock-skew
description: "Never infer background-agent death from transcript mtime — container clocks are skewed (hours apart); a \"stale\" agent may still be writing and will race your edits"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 8f8685c8-7840-46a2-9dd2-f43936b1d36b
---

During the 2026-06-04 portal-handoff sprint, two background agents were presumed dead because their transcript JSONL mtimes read ~1h stale vs `date` — but the clocks were skewed (shell `date` said 06:15 UTC while vitest in the same env stamped 12:13). Both agents were alive; one later raced the main session's edits on the same spec file (mystery `Promise.resolve()`→`settle()` rewrites that looked like a linter).

**Why:** different processes in this environment read different clocks; mtime-vs-now comparisons across them are meaningless.

**How to apply:** to check a background agent's liveness, compare the transcript mtime against another file the SAME writer just touched, or look for fresh tree edits in its scoped path — never against `date`. If a shared-worktree file changes unexpectedly mid-edit ("modified by user or linter"), suspect a live concurrent agent and TaskStop it before rewriting; see [[feedback_concurrent_sessions_shared_worktree]].
