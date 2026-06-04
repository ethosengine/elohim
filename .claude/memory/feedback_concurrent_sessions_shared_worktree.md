---
name: concurrent-sessions-shared-worktree
description: "Multiple Claude sessions co-commit on shift/* branches in the SAME worktree — never bulk-revert \"ambient\" modifications, selectively stage your own hunks"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 52e75242-0c09-464a-9a0d-613667fc14e0
---

Multiple Claude sessions can be active in /projects/elohim simultaneously, co-committing on the same shift/* branch (observed 2026-06-04: this session + a URL-Routing-Contract session interleaving commits on shift/a2o-greenup; their in-flight edits appeared as unexplained working-tree modifications, including a 146-file cosmetic lint sweep and `/lamad/path`→`/path` route renames).

**Why:** "Ambient" modified files are NOT necessarily leaked noise from your own subagents — they may be another session's in-progress work. I bulk-reverted 146 cosmetics that were plausibly theirs (grazed, recoverable, but wrong move).

**How to apply:**
- Before reverting/committing anything you didn't explicitly edit: `pgrep -af claude` (other sessions), `git log --all --since="6 hours ago"` (their commit themes), and stat mtimes vs your timeline.
- Commit by explicit file list only; for files with MIXED workstreams, selectively stage your hunks (filter the unified diff by a discriminating pattern, `git apply --cached --recount`).
- Their throwaway sweeps can capture YOUR temp files (a probe script of mine landed in their commit) — keep scratch out of the repo tree or clean immediately.
