---
id: "backlog-observation-seq-race-under-concurrent-posts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Observation seq can repeat when one observer's POSTs race"
slug: "observation-seq-race-under-concurrent-posts"
written: "2026-09-24"
author: "agent:implementer@claude-opus-5-5 (Lane A, task A9)"
status: "backlog"
priority: "low"
domain: "D2"
area: "elohim-storage observation write path"
tags: [observation, seq, concurrency, race, lane-a]
relatedNodeIds:
  - attention-witnessed-privately
cites:
  - genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md
  - elohim/elohim-storage/src/api/observations.rs
  - elohim/elohim-storage/src/observation/manager.rs
shift_objective: |
  Make an observer's seq strictly increasing under concurrent POST /api/v1/observations, by
  assigning it under the same lock that assigns log_offset (or deriving it from log_offset).
  Done when a test that fires N concurrent posts for one observer finds N distinct seq values
  in 1..=N, and the doc comment on accept_observation no longer carries the caveat.
---

# Observation seq can repeat under concurrent posts

**The limitation, as A3 documented it** (`elohim/elohim-storage/src/api/observations.rs`,
doc comment on `accept_observation`):

> `seq` is read before the manager's append lock is taken, so two writes by the same observer
> racing each other can share a `seq`; `log_offset` (under the lock) stays the observer's total
> order.

Step 6 of the write computes `seq` as the observer's `max(seq) + 1` from the `observations`
table. Then it appends through `ObservationManager`, which stamps `log_offset` and the log head
under its lock. Two POSTs from the same observer that arrive together can read the same max
and both write `seq = k + 1`.

**Why it is low today.** The only producer is the content viewer's
`ObservationEmitterService`. It posts once per leave per ref, so one person produces well
under one post per second. `log_offset` stays correct, and the stream ranks by
`observed_at` and dwell, never by `seq`.

**Why fix it anyway.** `seq` is on the wire (`ObservationAcceptedView.seq`) and in the row. Any
reader that takes it as a per-observer sequence (dedupe, "last seen" cursors, the signing
graduation's canonical bytes) will see a duplicate as corruption. A second producer (the
native shell, or a batch import) makes the race realistic.

**Direction.** Assign `seq` inside the manager's append critical section (the manager already
resumes per-observer state from `observation_logs`), or define `seq := log_offset + 1` and
drop the separate read. Add a concurrency test beside the A3 write tests
(`tests/api/api_observations_write_test.rs`): N tasks, one observer, distinct seqs.
