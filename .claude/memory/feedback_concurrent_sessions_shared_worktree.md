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
- **`git add <files>` then `git commit` is NOT atomic against a shared index** (observed 2026-06-08): a concurrent storage session reset+restaged the index between my `git add` (6 docs) and `git commit`, so my commit captured THEIR 3 staged files under MY message while my 6 docs stayed uncommitted. Fix: **always commit with path-limited `git commit -m "…" -- <explicit paths>`** — it ignores index state, commits only the named paths from the working tree, and leaves anyone else's staged files intact. (Watch the arg order: `-m` BEFORE `--`, else the message is read as a pathspec.)
- **`git commit --amend` is the SHARPEST trap — never amend in a shared worktree without re-checking HEAD** (observed 2026-06-20): I committed my spec (became HEAD), a concurrent facings session committed on top (HEAD moved to THEIR commit), then my `git commit --amend --no-edit` rewrote *their* commit — bundling my file into their tree under their message. `--amend` always targets whatever HEAD is *now*, which in a shared worktree may not be your commit. Fix: before any amend, `git rev-parse --short HEAD` must equal YOUR last commit; if unsure, **make a NEW commit instead of amending.** Recovery: find their original sha in `git reflog` (the `commit:` entry, not your `commit (amend):`), `git reset --soft <their-sha>` (restores their commit pristine as an ancestor, leaves your delta staged), then commit your delta on top with your own message — this un-pollutes their commit AND preserves their hash (never rewrite their commit object).
- **Recovery when your commit ate someone's staged work:** `git reset --soft HEAD~1` un-commits but KEEPS their files staged (zero content loss; safe iff the bad commit is local/unpushed — check `git status -sb`), then re-commit yours path-limited. Never `git revert` (that would delete their tree changes).
- **The race hits dispatched subagents too** (observed 2026-06-09, reverse direction: a rust-architect agent's 3 staged doorway files were swept into a concurrent session's commit between its `add` and `commit`; the agent's own commit then had nothing to commit). When dispatching any agent that commits in the shared worktree, put the path-limited commit form (`git commit -m "…" -- <explicit paths>`) in its prompt verbatim — agents default to `add`+`commit`. If swept: verify content integrity in HEAD (`git grep`/`git log -S`), report the host commit sha, do NOT rewrite the shared history.
