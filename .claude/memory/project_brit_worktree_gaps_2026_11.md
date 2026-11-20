---
name: project_brit_worktree_gaps_2026_11
title: "Brit gaps observed in the re-genesis 1 sprint (worktrees, shallow repo, push)"
description: "brit 0.1.12–0.2.6: commit refuses worktree hooksPath, status panics, pull refuses shallow, push closed-channel, fetch fails on dubious ownership, worktree add fails on large blobs; declared git fallbacks."
metadata:
  type: project
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-19T13:14:33.277Z
---

Observed on brit-cli 0.1.12 (monorepo pin `1c6ae5568`) during re-genesis 1 (2026-11-05/06),
on ethosengine and on shem:

- `brit commit` in any `/projects/elohim/.worktrees/*` worktree: `Error: refusing to ignore an
  untrusted core.hooksPath` (the repo sets an absolute `core.hooksPath=.husky`). `brit add -- <paths>`
  stages fine; the commit is then `git commit -F <file> -- <paths>` (pathspec, message from a file).
- `brit status` / `brit log` in a worktree: panic `too few index slots` (gix 32-slot store vs 49 packs)
  or print nothing. Read-only git is the fallback.
- `brit pull --ff-only` on the shallow main checkout: `shallow repositories require Git`.
- `brit push` after a green pre-push hook: `send-pack: sending on a closed channel` (seen on both
  devices); `git push` with `GATE_EVIDENCE=reuse` lands the same gated head.
- `brit push` from the brit submodule checkout to ethosengine/brit (no heavy hook): `unexpected end of
  stream while reading the push report` — the ref did not land; `git push origin <branch>:run-ci/<name>` did.
- `brit add` takes literal paths only after `--`; pathspec patterns are unsupported.
- `brit commit` under a refusing pre-commit hook (the formatting ratchet, or a malformed `.epr-meta`
  manifest making every file's policy unresolvable) exits 1 and prints a bare backtrace
  (`__libc_start_call_main`, `<unknown>` frames) after the hook's own lines (2026-11-07, 0.1.12).
  The refusal is real and the index is untouched: nothing was committed. Read the hook lines above
  the backtrace, fix, `brit add` again, re-run; never read the backtrace as a brit bug in the commit
  itself, and never filter the output to its tail.
- `brit add -- <submodule path>` panics on a gitlink (observed moving the brit pin); `brit submodule` has only
  `list`, no fetch/update verb — `git -C <sub> fetch/checkout` + `git add <sub>` is the fallback for a pin move.

- **brit 0.1.13 (2026-11-09, ethosengine, after a workspace restart wiped `$HOME`):** git refuses the
  main checkout as `dubious ownership` (owner UNKNOWN uid 1234 vs root; `~/.gitconfig` → `tooling/.gitconfig`
  holds no `safe.directory`). `epr` still passes its "repository trusted" check because it injects the
  trust itself, but `brit fetch -r origin` fails `Could not verify that "origin" is a valid git
  directory`, even with `brit -c safe.directory=/projects/elohim`. Fallback: `git -c safe.directory='*' …`
  per command (no global config write). `brit worktree add -d <path> <rev>` fails before writing
  anything: `object for checkout at …/.claude/data/governance-findings.jsonl could not be retrieved
  … Entry too large to fit in memory`. Fallback: `git worktree add`. `brit log <rev>` fails `File-based
  lookup isn't yet implemented in a way that is competitively fast`.

**Why:** CLAUDE.md requires a stated `# Brit fallback: <reason>` before any direct git; these are the
reasons that are real today, so a session does not re-discover them one panic at a time.

**How to apply:** try brit first (the add/status forms do work), quote the exact refusal in the
fallback line, keep commits pathspec-scoped, and never amend to repair the resulting `UNMEASURED`
attribution line (append-only). Pin is now 0.2.6; each 0.1.x bullet is unverified on 0.2.x — re-probe (brit commit in a
worktree, brit status in a worktree, brit push after green hook) and delete bullets that no
longer reproduce. The CLAUDE.md landing exception still names brit 0.1.12 send-pack and needs the
same recheck. Retire this note when `brit commit` succeeds in a worktree. Related:
[[feedback_push_branch_discipline]], [[project_regenesis1_landing_traps_receipt_and_epr_closure]].

**2026-11-13 ungated landing trap:** `-c core.hooksPath=<worktree>/.husky/_` on a worktree that has no `.husky/_` (husky generates it only on `pnpm install`) makes git run NO pre-push hook, silently; shem's landing 21 reached dev ungated this way. Before every landing push: `test -x <worktree>/.husky/_/pre-push` or refuse.

**2026-11-11 landing from a worktree:** the build-notes push (`refs/notes/brit/build/*`) failed twice. (1) The main checkout's hook predates 2bba47154 and gates notes refs as code: push with `-c core.hooksPath=<worktree>/.husky/_`. (2) The other device writes the same notes refs, so they reject `fetch first`: fetch `+refs/notes/brit/build/*:refs/notes-remote/brit/build/*`, then `git notes --ref=<ref> merge -s cat_sort_uniq <remote ref>` per diverged ref (27 merged, 0 conflicts), then push.

**2026-11-13 brit 0.1.14 (prerelease brit-build-a4db877811c1, checksums OK, probed from scratch before the pin moved):** `brit log -n N` now works. Still open: `brit rebase <upstream>` in a worktree refuses `shallow repositories require Git` even as a no-op; `brit diff -- .claude/data/governance-findings.jsonl` (18 MB) fails `Entry too large to fit in memory` (same cap as worktree add, so any 18 MB+ tracked blob trips diff/checkout). Push and worktree commit were not probed (need a real push / commit).

**2026-11-11 `brit worktree add` on a large blob:** `brit worktree add -b <branch> <path> origin/dev` fails with `Entry too large to fit in memory` on `.claude/data/governance-findings.jsonl`. The branch is kept but no worktree record exists. Recovery: `git worktree add <path> <branch>`, the git fallback, which attaches to the kept branch.

**2026-11-16 brit 0.2.0 (e70fc8ad):** `brit worktree add -b <branch> <path> <sha>` and `brit commit -F` in a worktree both worked (lane E, commit 7c8cbb94e). Still open: `brit log -n N <rev>` and `brit merge-base`-then-`log` with a revision argument fail `File-based lookup isn't yet implemented in a way that is competitively fast`; read a commit's files with `brit cat <rev>:<path>` instead. `brit fetch -r origin` works.

**2026-11-17 brit 0.2.0, overnight train 2 (ethosengine):** worked: `brit branch create <name> <sha>` + `brit switch <name>` (a new local branch in an existing worktree keeps its cargo slot warm), `brit add` + `brit commit -F`, `brit fetch origin dev`. Still open: `brit rebase <sha>` refuses `shallow repositories require Git` before writing; `brit cherry-pick` answers "a Git bridge operation; run: git cherry-pick". After a conflicted `git cherry-pick` resolved with `--ours` + `--continue`, git leaves a stale `AUTO_MERGE` pseudo-ref and the next `brit commit` refuses `merge metadata exists without MERGE_HEAD`; `git update-ref -d AUTO_MERGE` clears it. Never pass `-c core.hooksPath=/dev/null` to a fallback rebase: it silently skips every replayed commit's hooks (done once and aborted, 2026-11-17).

**2026-11-17 brit 0.2.0 `fetch origin dev` leaves origin/dev stale:** it negotiated and fetched, but `refs/remotes/origin/dev` stayed at the old sha. `epr bin follow` then refused "HEAD is not published on dev" (publishedRefs = refs/heads/dev, refs/remotes/origin/dev). `brit fetch -r origin` (configured refspecs) moved it. Before a follow or land, use `brit fetch -r origin` and check `brit rev parse origin/dev`.

**2026-11-19 brit 0.2.6, the AUTO_MERGE trap again, in a worktree:** after a conflicted fallback `git rebase` in a linked worktree, every later `brit commit` refused `merge metadata exists without MERGE_HEAD` (exit 1 + backtrace) while `brit add` kept staging, so four "commits" never happened. The stale file is in the worktree's OWN gitdir, `/projects/elohim/.git/worktrees/<name>/AUTO_MERGE`; `rm -f .git/AUTO_MERGE` in a worktree does nothing (`.git` is a file there). Cure: remove it there (or `git update-ref -d AUTO_MERGE` from the worktree). Never pipe `brit commit` into `grep -c` or `tail -1`: read its full output and confirm the new subject with `brit log -n 1` before treating a commit as made.

**2026-11-17 brit 0.2.0 cleanup gaps:** `brit worktree remove <path>` refuses "contains an initialized submodule" (exits 1 with a backtrace); after a read-only per-submodule `git status --porcelain` shows clean, `brit worktree remove -f` works. `brit branch` has only create and list, so deleting a branch is a git fallback. `git branch -d` judges merges against the main checkout's HEAD (a stale local dev), so check `git merge-base --is-ancestor <b> origin/dev` and use -D only when contained. Brit has no bundle verb (git bundle to keep a retired branch).

**2026-11-19/20 brit 0.2.6, two traps from brit sprint 3 (session 0e0924d5), both observed:**
- `brit remote -n origin refs | grep ' refs/heads/dev$'` prints TWO lines (the `HEAD symref-target:refs/heads/dev` line and the branch line). I (Claude) put that in an unquoted shell variable and ran the fallback `git rebase $devtip`: git read it as `<upstream> <branch>` and detached HEAD at dev. The branch was untouched and nothing was lost, but always take `| head -1` and pass one explicit commit id.
- brit's reach gate (`.githooks/pre-push` in the brit repo) refuses a push whose source is a commit id: "Brit reach refused: only a named local branch is supported". Push `refs/heads/<branch>:refs/heads/<dst>` and check the branch's head first.
