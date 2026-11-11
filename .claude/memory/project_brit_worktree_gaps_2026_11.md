---
name: project_brit_worktree_gaps_2026_11
title: "Brit gaps observed in the re-genesis 1 sprint (worktrees, shallow repo, push)"
description: "brit 0.1.12: commit refuses worktree hooksPath, status panics, pull refuses shallow, push closed-channel; add takes -- <paths>. Declared git fallbacks."
metadata:
  type: project
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-07T03:19:57.648Z
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

**Why:** CLAUDE.md requires a stated `# Brit fallback: <reason>` before any direct git; these are the
reasons that are real today, so a session does not re-discover them one panic at a time.

**How to apply:** try brit first (the add/status forms do work), quote the exact refusal in the
fallback line, keep commits pathspec-scoped, and never amend to repair the resulting `UNMEASURED`
attribution line (append-only). Re-check after the brit pin moves past 0.1.13 (T4 lands brit-cli
0.1.13); retire this note when `brit commit` succeeds in a worktree. Related:
[[feedback_push_branch_discipline]], [[project_regenesis1_landing_traps_receipt_and_epr_closure]].
