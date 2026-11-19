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

- **shem's refs lane** (`sprint3/refs-shem`, six patches applied locally, tip `4a39e9dffa`, held until
  shem's fix round): `rev-parse`, `for-each-ref`, `ls-remote`, `check-ref-format`, global `-C`, quiet
  exit on a closed pipe. Its review found the closed-pipe patch too broad, and
  `rev-parse --verify HEAD -- x` printing three lines with exit 0; the fix round is pending. shem also
  takes `merge-base --is-ancestor` and `rev-list --count`.
- **Lane `sprint3/diff-show`** (tip `3d10b379cf`, in review): `brit diff` with revisions, ranges, merge
  bases and all output forms, and `brit show`.

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
