---
name: commit-attribution-parallel-agent-leak
description: When multiple agents work the same repo in parallel, an untracked-file directory created by one agent can be swept into another agent's commit if the second agent uses `git add <dir>` while files are pending. Resilience-epic commit e44bd77c3 accidentally swept in 10 views_convert/ scaffold files from a parallel monolithic-code-decomposition agent; the follow-up commit 994a17d46 ("scaffold views_convert/...") landed only a Cargo.lock bump because the file creation had already been absorbed.
metadata:
  type: feedback
---

When parallel agents are working in the same repo, untracked files from one agent's work can be captured by another agent's commit if the second agent stages a parent directory.

**The incident (2026-05-18):** I ran `git add genesis/docs/content/elohim-protocol/resilience/` to stage the resilience epic, but somehow my commit `e44bd77c3` ALSO included 10 `elohim/elohim-storage/src/views_convert/{epr,imagodei,infrastructure,inputs,lamad,mod,qahal,shared,shefa}.rs` files plus a `lib.rs` edit — work from a parallel monolithic-code-decomposition agent. The parallel agent's follow-up commit `994a17d46` ("scaffold views_convert/ sibling module tree") landed only a `Cargo.lock` bump because the file creation it expected had already been captured by my commit.

**Why:** Most likely cause is that some intermediate `git add` (perhaps via a pre-commit hook, or via a hook in the `Bash` tool path, or via a transient `git add -A` somewhere) staged the untracked files. The dedicated path `git add genesis/docs/.../resilience/` should not have swept anything else in, but the index ended up with both. **Root cause not definitively confirmed**; treat this as a real-but-rare risk pattern under parallel-agent work.

**How to apply:**
- Before committing, do `git status --short` and confirm the staged set matches intent. If it doesn't, `git reset HEAD <unwanted-path>` before commit.
- When multiple agents are known to be working concurrently in the same worktree, prefer narrower `git add <specific-file>` over `git add <directory>`. Directories can pick up newly-created files atomically.
- When you DO discover the leak post-commit, options are: (a) `git reset --soft <bad-commit>^` and re-commit cleanly (safe when local-only), or (b) accept and leave a note in MEMORY.md / commit message of the next commit. Don't unilaterally rewrite history if the commit is already pushed.
- Memory-ceremony note: this is the kind of process-friction the librarian/cartographer should watch for — repeated occurrences indicate a hook or staging-default needs tightening.

**Open thread on commits e44bd77c3 + 994a17d46 (resilience epic + views_convert):** unresolved as of 2026-05-18. Local-only on `dev`. Both commits compile; the artifacts work. The only cost is attribution accuracy (my commit message describes the resilience epic and lists views_convert files; the parallel agent's commit describes views_convert and lists only Cargo.lock). Worth sorting before push to origin/dev; not urgent if not pushed.
