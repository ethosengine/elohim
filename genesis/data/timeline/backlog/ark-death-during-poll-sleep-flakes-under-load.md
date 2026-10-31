---
id: "backlog-ark-death-during-poll-sleep-flakes-under-load"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ark: a SIGKILL sent in the same poll interval as shutdown is recorded as a clean stop under gate load"
slug: "ark-death-during-poll-sleep-flakes-under-load"
written: "2026-10-31"
author: "claude-opus-5-5"
status: "open"
priority: "medium"
area: "elohim/ark supervisor / tests"
domain: "D5"
tags: [ark, supervisor, flake, death-witness, load]
---

# ark: death during the poll sleep, under load

`elohim/ark/supervisor/tests/supervise_death.rs`
`a_death_during_the_poll_sleep_is_witnessed_and_never_laundered_into_a_stop` failed in the
pre-push gate of train 2 (b075fa30a, 2026-10-31 ~02:10Z) with
`the SIGKILL was laundered into a clean stop: no witness reached the spool`. Train 2 changes
nothing under `elohim/ark`; the same suite passed in push 7 (9a5719006), and the test passes
10/10 run alone on the same tree.

The test calls `sigkill(pid)` and then sets `shutdown` at once, expecting the supervisor's
next wake to read the death first. Under load the killed child may not yet be reapable when
the supervisor wakes and sees `shutdown`, so the loop answers the stop and the child's exit
is never witnessed.

| Row | What | State |
|---|---|---|
| 1 | Decide whether this is the test assuming instant reaping, or a real window: on shutdown the supervisor could reap (blocking, bounded) before deciding the child stopped cleanly, so a death that raced the stop flag is still witnessed | open: ark owner's reading |
| 2 | Until then the test is a load-sensitive red on any push that selects elohim-ark | open |

## Done when

- The property holds under load: a SIGKILL that lands in the same poll interval as shutdown
  is witnessed, and the test passes in 20 consecutive full pre-push runs.
