---
id: "backlog-steward-key-chain-fork-warrant-exile"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Steward keys are warranted for chain fork and cut off by every authority with no end date — adam and eve keep minting forks, the conductor cannot say at which seq, and nothing unblocks a key"
slug: "steward-key-chain-fork-warrant-exile"
written: "2026-10-10"
author: "steward-exile-probe shift, 2026-10-10 fleet read"
status: "open"
priority: "high"
jobs: [elohim-edge]
tags: [fleet, conductor, warrant, chain-fork, kitsune2, dataplane-convergence, alpha]
---

**What is measured (alpha, 2026-10-10).** Adam's agent key `uhCAkCUzKl3…` and eve's `uhCAkhsVVjkuw1D8…` are the
subject of chain-fork warrants authored by jessica, james, gertrude, eve and matthew. Every conductor that integrates
one blocks the key from now to `Timestamp::max()` (`integrate_dht_ops_workflow.rs`). kitsune2 0.5.0 then refuses the
blocked agent's info at the peer store (`mem_peer_store.rs`), so no access decision exists for its url, and a url with
no decision is treated as blocked (`core_space.rs is_any_agent_at_url_blocked`). Matthew has discarded every message
adam sent him (4,285,797 of 4,285,796); adam's `hc_conductor_workflow_integrated_ops_total` is 0. Six distinct forked
action pairs in Loki's four days of retention: adam four, eve two, new ones on three consecutive days, one each after
the 2026-10-09 conductor clear. Evidence and queries: `dataplane-convergence` DELTA 2026-10-10a and 10-10b.

**Why it matters.** A warranted steward cannot converge by waiting. The two doorways serve different app builds and
will keep doing so; `dataplane-convergence`'s "a peer that missed a deploy heals" is false for this peer under this pin.

**Open, in the order that unblocks the most.**

1. **Cause of the forks — unread.** A fork is two actions by one author at one seq. The conductor logs the two action
   hashes and not the seq, and the forked peer logs nothing about its own fork. Next read, once Loki is healthy and in
   windows of three hours or less (a four-day regexp query probably OOM-killed loki-0 at ~02:20Z on 2026-10-10): the
   forked peer's own conductor log in the ten minutes before each pair's first sighting, for write-lock and I/O
   errors, restarts and re-authoring. Candidates not yet separated: an action published, then lost locally and
   re-authored; a dataset restored behind its own published chain; one batch of stale ops replayed by a third peer
   (the two pairs at 2026-10-09T23:52:10Z reached jessica 0.5 ms apart).
2. **The fork's seq is not observable.** `make_fork_warrant_op_inner` receives `seq` and logs only the hashes. Logging
   it, and exposing integrated warrants (warrantee, kind, seq, when) through the admin surface storage already reads
   for `/db/p2p/conductor-diagnostics`, turns item 1 from inference into a read. Conductor-fork change; a pin move
   rolls the fleet, so it rides the next intended conductor roll.
3. **No way back for a warranted key.** The block has no end and no admin call lifts it. Whether a steward key returns
   by re-key with lineage (`identity-head-key-lineage`) or by an operator unblock is a design and operator decision,
   not a code default. Until it is made, adam and eve stay cut off with these keys.
4. **Matthew's key — unverified.** One Loki read reported matthew's own key warranted at 2026-10-09T23:52Z; a second
   did not reproduce it. Transport evidence only: adam blocks all of matthew's traffic in space `5a385…`. Re-read with
   `Warrant op is valid` grouped by warrantee in a three-hour window. If true, the node the quiesce gate reads is in
   this class.

**Standing detector.** `scripts/ci/substrate-seam-smoke.sh` seam 7 `peer-exile` (advisory) prints `EXILED` per doorway
when a connected peer's traffic is discarded whole or the peer answers nothing. Flip it into `--gate` when no steward
key is warranted.
