---
id: "backlog-brit-porcelain-gaps-census-2026-11"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Which git commands agents still fall back to: a census of shem's transcripts since 2026-11-06, and what is still missing from brit"
slug: "brit-porcelain-gaps-census-2026-11"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "backlog"
priority: "medium"
relatedNodeIds: []
tags: [brit, daily-driver, porcelain, git-fallback, census]
cites:
  - CLAUDE.md
  - genesis/data/timeline/backlog/brit-sprint-3-handoff-2026-11-19.md
---

# brit porcelain gaps: census, November 2026

## Census

From shem's agent transcripts since 2026-11-06: 3,767 git invocations, 1,340 of them declared
fallbacks.

| Family | Uses |
|---|---|
| diff | 255 |
| push | 150 |
| log | 123 |
| worktree and merge | 116 |
| commit | 84 |
| show | 69 |
| ls-remote | 59 |
| rev-parse | 57 |
| fetch | 45 |
| submodule | 41 |
| ls-tree | 23 |
| merge-base | 14 |
| for-each-ref | 3 |

The largest single form was `git -C <dir> …`, about 1,700 uses.

The brit-first hook's own ledger keeps no command text, and its `epr feedback capture` call fails with
"unknown command feedback".

## In hand

- **shem's refs lane** (`sprint3/refs-shem`): `rev-parse`, `for-each-ref`, `ls-remote`,
  `check-ref-format`, global `-C`, quiet exit on a closed pipe, and now also `merge-base --is-ancestor`
  and `rev-list --count`. Its first review found the closed-pipe patch too broad, and
  `rev-parse --verify HEAD -- x` printing three lines with exit 0. shem delivered two fix rounds;
  sixteen patches were applied as tip `806ea8114b` (shem's `b8264afc4`) and merged into the
  integration branch (`b9b4a294fc`). A deliberate difference is kept: `rev-parse --verify missing` with
  a closed stderr exits 128 where git dies with 141.
- **Lane `sprint3/diff-show`** (tip `5a422ceb51`, two review passes and a delta review, merge in
  progress): `brit diff` with revisions, ranges, merge bases and all output forms, and `brit show`.
  Final state:
  - commit encodings re-encoded as git does (the whole commit, including author and committer names);
    unknown labels, and non-ASCII text under a non-UTF-8 output encoding, refuse by name; ASCII-only
    commits print;
  - tree forms identical to git;
  - a name that is both a revision and a file is refused as ambiguous in `show`, as in `diff`;
    `diff <name> --` reads a revision;
  - `brit diff -- <path>` selecting nothing exits 0 (was 1);
  - git's first-line wording for unknown names; the exit code stays 1 where git exits 128;
  - measured on the monorepo `origin/main...origin/dev` (debug build): `--name-status` identical;
    `--numstat` differs on 3 of 26,673 files by a few lines on both sides (brit uses imara-diff's
    Myers with gix's indent heuristic, git xdiff's Myers with its cost heuristic; neither is minimal;
    the patch text differs in hunk placement).

  Measured limits from its review:
  - any file with a `diff=<driver>` attribute makes diff and show refuse, even a built-in driver with
    nothing configured; `--name-only` still works; no driver command or textconv is ever executed;
  - intent-to-add entries and a conflicted index refuse the worktree forms;
  - multiple merge bases refuse `--merge-base` (git refuses too, with a different exit code).

## Still missing after those

- A porcelain merge. brit cannot record a two-parent commit; every merge on the integration branch
  used git.
- cherry-pick.
- `submodule` beyond list.
- Worktree-side `brit diff` when the index contains a gitlink (it refuses; the monorepo has five).
- Combined diffs for merges in `show`.
- `ls-tree` with a path filter and `--name-only`.
- config get/set.
- Removing a path from the index (`rm --cached`).
- Applying a patch series (`am` / `apply`).
- `check-ignore`, and the `ls-files` spelling with its options. brit can list tracked paths
  (`brit index entries`); an agent used plain `git ls-files` this sprint without checking, a recorded
  slip.
- `status --porcelain`.
- Pickaxe (`log -S`).
- Per-hunk staging and pathspec patterns in `add`. It stages whole literal paths, so interleaved
  changes cannot be split into commits.
- `restore --source` other than HEAD.
- Shallow clone.
- `brit log` range output with a format or body.

Global: `brit … | head` prints "Error: Broken pipe (os error 32)" on 0.2.6.
