---
id: "backlog-prepush-epr-meta-gate-pays-one-process-per-file"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The pre-push epr-meta gate spawns one epr govern per changed file, serially, before any project gate starts"
slug: "prepush-epr-meta-gate-pays-one-process-per-file"
written: "2026-10-30"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "pre-push / epr-meta governance / wall-clock"
domain: "D5"
tags: [pre-push, wall-clock, gradient-reading, epr-meta, governance]
---

# The pre-push epr-meta gate pays one process per file

Push 5 of the 2026-10-27..30 overnight integration (4dc561c4b, an integration train of 109
commits over 653 changed files) sat in `.claude/scripts/epr-meta-git-gate.py --range` for
about an hour before the first project gate opened. `main()` loops `changed_files()` and
calls `epr_client.govern()` once per path, each a fresh `epr govern` subprocess. One call
measured 2 s at load 30; 653 of them in series is the hour.

This is the cheap-check-first rule inverted: a check whose verdict is seconds per file
blocks every project gate behind it, and its cost scales with the train, not with risk.

| Row | What | State |
|---|---|---|
| 1 | Evaluate the range in one `epr govern` invocation (a batch of paths and contents on stdin, one verdict per path), or a worker pool bounded by the host's cores | open |
| 2 | Run the epr-meta gate beside the project gates instead of before them; its verdict joins the run like any other gate's | open |
| 3 | Each pushed commit already passed the same rules at pre-commit; a range whose commits all carry a pre-commit witness for the current rule CID could reuse those verdicts | open: needs the governance owner's reading of whether a pre-commit witness is admissible at push |

## Done when

- A 600-file integration push spends under 2 minutes in the epr-meta gate, measured from the
  pre-push log.
