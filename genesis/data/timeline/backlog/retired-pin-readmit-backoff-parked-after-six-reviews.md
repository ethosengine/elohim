---
id: "backlog-retired-pin-readmit-backoff-parked-after-six-reviews"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The retired-pin backoff is built on a branch and parked: six independent reviews, the last with three narrow blockers, under a bail rule set before the shift"
slug: "retired-pin-readmit-backoff-parked-after-six-reviews"
written: "2026-10-14"
author: "claude (overnight performance shift, operator-directed)"
status: "open"
priority: "medium"
severity: medium
nodes: [alpha, household]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [elohim-storage, acquisition, pins, caughtUp, quiesce, performance, perf-convergence, idle-is-free, plane-dataplane, unit-peer, phase-steady, lane-background]
cites:
  - elohim/elohim-storage/src/db/acquisition_pins.rs
  - elohim/elohim-storage/src/p2p/acquisition.rs
  - genesis/data/timeline/backlog/ci-edge-inventory-convergence-caughtup-sawtooth-flap.md
  - genesis/data/timeline/backlog/pure-iroh-pull-core-cannot-retire-unfillable-pins.md
---

**The problem it cures (measured on alpha, 2026-10-13).** A retired acquisition pin is re-admitted on a flat 6 h
cooldown forever. On alpha matthew 1,103 retired pins (1,063 of them `epr:e2e-*` scenario residue) came back about a
thousand at a time, and while they were re-probed `pull.caughtUp` read false, so the edge quiesce gate failed
(#2047 to #2049). The operator removed the 1,063 by hand; the cause is still in the code.

**What was built, not landed.** Branch `fix/retired-pin-readmit-backoff`, head `bed2eabe8`, worktree
`.worktrees/wt-pin-readmit`, one unshipped migration (`2026-10-13-143107_acquisition_pin_readmit_backoff`). An
unread, unfillable pin backs off 6 h, 12 h, then 24 h; reads, inventory and explicit re-pins behave as on dev, except
that the rung is kept, a removed pin stays removed on a read (operator decision, 2026-10-14), and a read the autopin
throttles caps the hold at 6 h. Computed, not measured: on a node with 1,100 such pins, one re-probe round a day
instead of four. Lib tests pass bar the two rakia tests; clippy clean with and without `p2p-iroh`.

**Why parked.** The shift set a bail rule before starting: a storage change that is not rated SAFE within two more
review rounds is parked and nothing of it lands. Codex's sixth review (`genesis/local-dev/idle-call-floor-2/
codex-review-pin-readmit-6.md`) resolved every round-5 finding and found three new blockers, each small:

1. The due check truncates `now` to whole seconds and compares strictly, so a pin due at `18:00:00Z` is missed by a
   pass at `18:00:00.250Z` and waits one more tick (60 s) where dev admits it. Fix shape: compare `<=`, or compare
   at full precision.
2. The hold pass bumps `round` and discards the tracker unconditionally when it re-admits a pin. If the exhausted
   tracker was never evicted (the process was suspended through the hold, or every reconcile failed before tracker
   maintenance), the pin gets three extra attempts where dev re-retires it without a fetch. Fix shape: bump the round
   only when no tracker for the pin survives.
3. A throttled read's mark computes the cap in a SELECT and writes it in an UPDATE that does not check the retirement
   it read, so a stalled request can stamp an old hold's deadline onto a later retirement. Fix shape: one conditional
   UPDATE that computes the cap from the row's current `retired_at`.

Plus two stale comments (`demand_autopin.rs:202`, `p2p/acquisition.rs:749`).

**Review history.** Six rounds. Rounds 1 to 3 added machinery (deferred demand, a per-tick bound, provider-coverage
tracking) that each review showed made a read or peer path worse than dev; rounds 4 to 6 removed it again. The
lesson for the next attempt: state "a read, a peer or a re-pin does exactly what dev does" as the design rule from
the first draft, and change only what an unread pin does.

**Decision for the operator.** Allow one more round on the three fixes above (each a few lines), or leave the branch
parked. Landing it changes nothing on alpha until a deploy carries it; the fleet roll is the operator's call either
way.

**Done when.** A node with unread, unfillable retired pins re-probes them on the ladder, measured on the household
or the fleet, and reads, peers and re-pins are shown unchanged.
