---
id: "backlog-prepush-memory-ceremony-reruns-eprfs-recall-tests"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The memory-ceremony pre-push gate re-runs the epr-cli recall tests the eprfs gate already ran"
slug: "prepush-memory-ceremony-reruns-eprfs-recall-tests"
written: "2026-10-30"
author: "claude-opus-5-5"
status: "open"
priority: "medium"
area: "pre-push / gate selection / wall-clock"
domain: "D5"
tags: [pre-push, wall-clock, gradient-reading, eprfs, memory-ceremony]
---

# memory-ceremony re-runs the eprfs recall tests

Push 7 of the 2026-10-27..30 overnight integration (9a5719006) ran the `eprfs` gate, which
tests the whole eprfs workspace including every `epr-cli/tests/flow_memory_recall*` binary,
and passed it at 21:38Z (5114 s). At 22:51Z the `memory-ceremony` gate started
`cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli` and was still running
the same recall binaries 35 minutes later, in the same push, on the same tree.

| Row | What | State |
|---|---|---|
| 1 | When `eprfs` is selected and green in the same run, `memory-ceremony` should not pay for `elohim-epr-cli` tests again: either its step names only what `eprfs` does not cover, or the gate runner reuses the run's own `eprfs` verdict for the shared closure | open |
| 2 | If the two gates must stay separate, `memory-ceremony` should share `eprfs`'s target directory so it does not recompile `elohim-epr-cli` | open |

## Done when

- A push that selects both gates runs the `elohim-epr-cli` recall binaries once.
